use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::combat::CombatState;
use crate::content::cards::{self, CardTarget};
use crate::content::potions;
use crate::content::powers::PowerId;
use crate::diff::parser::{parse_replay, ReplayAction};
use crate::diff::replay_support::{continue_deferred_pending_choice, tick_until_stable};
use crate::diff::state_sync::{build_combat_state, sync_state};
use crate::state::core::{ClientInput, EngineState, PendingChoice};

const KEY_POWER_TAGS: &[(PowerId, &str)] = &[
    (PowerId::Vulnerable, "Vulnerable"),
    (PowerId::Weak, "Weak"),
    (PowerId::Artifact, "Artifact"),
    (PowerId::NoDraw, "NoDraw"),
    (PowerId::Unawakened, "Unawakened"),
    (PowerId::Regrow, "Regrow"),
    (PowerId::Shackled, "Shackled"),
    (PowerId::Malleable, "Malleable"),
    (PowerId::CurlUp, "CurlUp"),
    (PowerId::DarkEmbrace, "DarkEmbrace"),
    (PowerId::FeelNoPain, "FeelNoPain"),
];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InteractionSignature {
    pub source_kind: String,
    pub source_id: String,
    pub target_shape: String,
    pub pending_choice: String,
    pub archetype_tags: Vec<String>,
    pub hook_tags: Vec<String>,
    pub pile_tags: Vec<String>,
    pub power_tags: Vec<String>,
    pub outcome_tags: Vec<String>,
}

impl InteractionSignature {
    pub fn canonicalize(&mut self) {
        sort_dedup(&mut self.archetype_tags);
        sort_dedup(&mut self.hook_tags);
        sort_dedup(&mut self.pile_tags);
        sort_dedup(&mut self.power_tags);
        sort_dedup(&mut self.outcome_tags);
    }

    pub fn canonical_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|archetypes={}|hooks={}|piles={}|powers={}|outcomes={}",
            self.source_kind,
            self.source_id,
            self.target_shape,
            self.pending_choice,
            self.archetype_tags.join(","),
            self.hook_tags.join(","),
            self.pile_tags.join(","),
            self.power_tags.join(","),
            self.outcome_tags.join(",")
        )
    }

    pub fn source_combo_key(&self) -> String {
        format!(
            "{}|{}|{}|hooks={}|piles={}",
            self.source_kind,
            self.source_id,
            self.pending_choice,
            self.hook_tags.join(","),
            self.pile_tags.join(",")
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservedInteractionRecord {
    pub observed_from: String,
    pub source_file: String,
    pub combat_idx: Option<usize>,
    pub action_idx: Option<usize>,
    pub command: String,
    pub signature_key: String,
    pub source_combo_key: String,
    pub signature: InteractionSignature,
}

#[derive(Default, Debug, Serialize)]
pub struct InteractionCoverageReport {
    pub generated_from: Vec<String>,
    pub total_records: usize,
    pub unique_signatures: usize,
    pub unique_sources: usize,
    pub observed_from_counts: BTreeMap<String, usize>,
    pub source_signature_counts: BTreeMap<String, usize>,
    pub archetype_signature_counts: BTreeMap<String, usize>,
    pub archetype_source_counts: BTreeMap<String, usize>,
    pub archetype_sources: BTreeMap<String, Vec<String>>,
    pub low_diversity_sources: Vec<String>,
    pub low_order_sources: Vec<String>,
    pub high_value_signature_counts: BTreeMap<String, usize>,
    pub high_value_undercovered_sources: Vec<String>,
    pub uncovered_cards: Vec<String>,
    pub uncovered_potions: Vec<String>,
    pub notes: Vec<String>,
}

pub fn signature_from_transition(
    before_engine: &EngineState,
    before: &CombatState,
    input: &ClientInput,
    after_engine: &EngineState,
    after: &CombatState,
) -> InteractionSignature {
    let (source_kind, source_id, target_shape) = source_descriptor(before, input);
    let mut hook_tags = BTreeSet::new();
    let mut pile_tags = BTreeSet::new();
    let mut outcome_tags = BTreeSet::new();

    match input {
        ClientInput::PlayCard { .. } => {
            hook_tags.insert("on_use_card".to_string());
        }
        ClientInput::UsePotion { .. } => {
            hook_tags.insert("on_use_potion".to_string());
        }
        ClientInput::EndTurn => {
            hook_tags.insert("at_end_of_turn".to_string());
            hook_tags.insert("at_end_of_round".to_string());
        }
        _ => {}
    }

    if monsters_took_damage(before, after) {
        hook_tags.insert("on_attacked".to_string());
    }
    if any_new_monster_death(before, after) {
        hook_tags.insert("on_death".to_string());
        outcome_tags.insert("kills".to_string());
    }
    if any_new_half_dead(before, after) {
        outcome_tags.insert("half_dead".to_string());
    }
    if any_revive(before, after) {
        outcome_tags.insert("revives".to_string());
    }
    if spawned_monsters(before, after) {
        outcome_tags.insert("spawns".to_string());
    }
    if after.zones.hand.len() > before.zones.hand.len() {
        pile_tags.insert("draw".to_string());
        outcome_tags.insert("draws_cards".to_string());
    }
    if after.zones.discard_pile.len() > before.zones.discard_pile.len() {
        pile_tags.insert("discard".to_string());
    }
    if after.zones.exhaust_pile.len() > before.zones.exhaust_pile.len() {
        pile_tags.insert("exhaust".to_string());
    }
    if before.zones.draw_pile.is_empty()
        && !before.zones.discard_pile.is_empty()
        && !after.zones.draw_pile.is_empty()
    {
        pile_tags.insert("shuffle".to_string());
    }
    if after.zones.limbo.len() != before.zones.limbo.len() {
        pile_tags.insert("limbo".to_string());
    }
    if after.turn.energy > before.turn.energy && !matches!(input, ClientInput::EndTurn) {
        outcome_tags.insert("gains_energy".to_string());
    }
    if matches!(after_engine, EngineState::PendingChoice(_)) {
        outcome_tags.insert("opens_pending_choice".to_string());
    }

    let mut signature = InteractionSignature {
        source_kind,
        source_id,
        target_shape,
        pending_choice: pending_choice_descriptor(before_engine, after_engine),
        archetype_tags: crate::bot::evaluator::CardEvaluator::archetype_tags(
            &crate::bot::evaluator::CardEvaluator::combat_profile(before),
        ),
        hook_tags: hook_tags.into_iter().collect(),
        pile_tags: pile_tags.into_iter().collect(),
        power_tags: collect_key_power_tags(before, after),
        outcome_tags: outcome_tags.into_iter().collect(),
    };
    signature.canonicalize();
    signature
}

pub fn replay_records_from_path(path: &Path) -> Vec<ObservedInteractionRecord> {
    let replay = parse_replay(path.to_string_lossy().as_ref());
    let mut records = Vec::new();

    for combat in replay.combats {
        let mut combat_state = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for (action_idx, action) in combat.actions.iter().enumerate() {
            sync_state(&mut combat_state, &prev_snapshot);

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let _ = continue_deferred_pending_choice(
                        &pending,
                        &mut combat_state,
                        &action.result,
                    );
                }
            }

            if action.action_type == "sync" {
                prev_snapshot = action.result.clone();
                continue;
            }

            if let Some(input) = replay_action_to_input(action, &combat_state) {
                let before_engine = EngineState::CombatPlayerTurn;
                let before_state = combat_state.clone();
                let mut after_engine = EngineState::CombatPlayerTurn;
                let _alive = tick_until_stable(&mut after_engine, &mut combat_state, input.clone());
                let signature = signature_from_transition(
                    &before_engine,
                    &before_state,
                    &input,
                    &after_engine,
                    &combat_state,
                );
                records.push(ObservedInteractionRecord {
                    observed_from: "replay".to_string(),
                    source_file: path.to_string_lossy().into_owned(),
                    combat_idx: Some(combat.combat_idx),
                    action_idx: Some(action_idx + 1),
                    command: command_string(&input),
                    signature_key: signature.canonical_key(),
                    source_combo_key: signature.source_combo_key(),
                    signature,
                });
                carried_pending = match &after_engine {
                    EngineState::PendingChoice(choice) => Some(choice.clone()),
                    _ => None,
                };
            }

            prev_snapshot = action.result.clone();
        }
    }

    records
}

pub fn load_live_comm_records(path: &Path) -> Vec<ObservedInteractionRecord> {
    if !path.exists() {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<ObservedInteractionRecord>(line).ok())
        .collect()
}

pub fn write_coverage_outputs(
    records: &[ObservedInteractionRecord],
    generated_from: Vec<String>,
    coverage_path: &Path,
    report_path: &Path,
    notes: Vec<String>,
) -> std::io::Result<()> {
    let mut unique_signatures = HashSet::new();
    let mut source_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut archetype_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut archetype_source_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut observed_from_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut high_value_signature_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut high_value_source_flags: HashMap<String, BTreeSet<String>> = HashMap::new();
    let mut source_unique_signature_sets: HashMap<String, BTreeSet<String>> = HashMap::new();

    for record in records {
        unique_signatures.insert(record.signature_key.clone());
        *observed_from_counts
            .entry(record.observed_from.clone())
            .or_insert(0) += 1;
        *source_signature_counts
            .entry(record.signature.source_id.clone())
            .or_insert(0) += 1;
        for tag in &record.signature.archetype_tags {
            *archetype_signature_counts.entry(tag.clone()).or_insert(0) += 1;
            archetype_source_sets
                .entry(tag.clone())
                .or_default()
                .insert(record.signature.source_id.clone());
        }
        source_unique_signature_sets
            .entry(record.signature.source_id.clone())
            .or_default()
            .insert(record.signature_key.clone());

        let flags = high_value_flags(&record.signature);
        for flag in &flags {
            *high_value_signature_counts.entry(flag.clone()).or_insert(0) += 1;
        }
        high_value_source_flags
            .entry(record.signature.source_id.clone())
            .or_default()
            .extend(flags);
    }

    let low_diversity_sources: Vec<String> = source_signature_counts
        .iter()
        .filter(|(_, count)| **count <= 1)
        .map(|(source, _)| source.clone())
        .collect();

    let low_order_sources: Vec<String> = source_unique_signature_sets
        .iter()
        .filter(|(source, signatures)| {
            signatures.len() <= 1
                && high_value_source_flags
                    .get(*source)
                    .map_or(true, |flags| flags.is_empty())
        })
        .map(|(source, _)| source.clone())
        .collect();

    let high_value_undercovered_sources: Vec<String> = source_signature_counts
        .keys()
        .filter(|source| {
            let flags = high_value_source_flags.get(*source);
            flags.map_or(true, |f| f.is_empty())
        })
        .cloned()
        .collect();

    let observed_sources: HashSet<&str> =
        source_signature_counts.keys().map(String::as_str).collect();
    let uncovered_cards: Vec<String> = known_card_sources()
        .into_iter()
        .filter(|name| !observed_sources.contains(name.as_str()))
        .collect();
    let uncovered_potions: Vec<String> = known_potion_sources()
        .into_iter()
        .filter(|name| !observed_sources.contains(name.as_str()))
        .collect();

    let report = InteractionCoverageReport {
        generated_from,
        total_records: records.len(),
        unique_signatures: unique_signatures.len(),
        unique_sources: source_signature_counts.len(),
        observed_from_counts,
        source_signature_counts,
        archetype_signature_counts,
        archetype_source_counts: archetype_source_sets
            .iter()
            .map(|(tag, sources)| (tag.clone(), sources.len()))
            .collect(),
        archetype_sources: archetype_source_sets
            .into_iter()
            .map(|(tag, sources)| (tag, sources.into_iter().collect()))
            .collect(),
        low_diversity_sources,
        low_order_sources,
        high_value_signature_counts,
        high_value_undercovered_sources,
        uncovered_cards,
        uncovered_potions,
        notes,
    };

    if let Some(parent) = coverage_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(
        coverage_path,
        serde_json::to_string_pretty(records).unwrap(),
    )?;
    std::fs::write(report_path, serde_json::to_string_pretty(&report).unwrap())?;
    Ok(())
}

pub fn novelty_bonus(
    signature_key: Option<&str>,
    source_combo_key: Option<&str>,
    db: &crate::bot::coverage::CoverageDb,
    mode: crate::bot::coverage::CoverageMode,
) -> f32 {
    match mode {
        crate::bot::coverage::CoverageMode::Off => 0.0,
        crate::bot::coverage::CoverageMode::PreferNovel => {
            let mut bonus = 0.0;
            if let Some(sig) = signature_key {
                if !db.tested_signatures.contains(sig) {
                    bonus += 120_000.0;
                }
            }
            if let Some(source_combo) = source_combo_key {
                if !db.source_signature_counts.contains_key(source_combo) {
                    bonus += 45_000.0;
                }
            }
            bonus
        }
        crate::bot::coverage::CoverageMode::AggressiveNovel => {
            let mut bonus = 0.0;
            if let Some(sig) = signature_key {
                if !db.tested_signatures.contains(sig) {
                    bonus += 180_000.0;
                }
            }
            if let Some(source_combo) = source_combo_key {
                if !db.source_signature_counts.contains_key(source_combo) {
                    bonus += 70_000.0;
                }
            }
            bonus
        }
    }
}

pub fn curiosity_bonus(
    signature: Option<&InteractionSignature>,
    target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> f32 {
    if let (Some(signature), Some(target)) = (signature, target) {
        if curiosity_target_matches(signature, target) {
            return 95_000.0;
        }
    }
    0.0
}

pub fn curiosity_target_matches(
    signature: &InteractionSignature,
    target: &crate::bot::coverage::CuriosityTarget,
) -> bool {
    match target {
        crate::bot::coverage::CuriosityTarget::Card(name) => {
            signature.source_kind == "card" && equals_ignore_ascii_case(&signature.source_id, name)
        }
        crate::bot::coverage::CuriosityTarget::Relic(name) => {
            signature.source_kind == "relic" && equals_ignore_ascii_case(&signature.source_id, name)
        }
        crate::bot::coverage::CuriosityTarget::Potion(name) => {
            signature.source_kind == "potion"
                && equals_ignore_ascii_case(&signature.source_id, name)
        }
        crate::bot::coverage::CuriosityTarget::Archetype(tag) => signature
            .archetype_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        crate::bot::coverage::CuriosityTarget::PowerTag(tag) => signature
            .power_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        crate::bot::coverage::CuriosityTarget::PileTag(tag) => signature
            .pile_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        crate::bot::coverage::CuriosityTarget::PendingChoice(tag) => {
            signature.pending_choice != "none"
                && signature
                    .pending_choice
                    .to_ascii_lowercase()
                    .contains(&tag.to_ascii_lowercase())
        }
        crate::bot::coverage::CuriosityTarget::Source(name) => {
            equals_ignore_ascii_case(&signature.source_id, name)
        }
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn replay_action_to_input(action: &ReplayAction, combat: &CombatState) -> Option<ClientInput> {
    match action.action_type.as_str() {
        "play" => Some(ClientInput::PlayCard {
            card_index: action.card_index?,
            target: action
                .target
                .and_then(|idx| combat.entities.monsters.get(idx).map(|m| m.id)),
        }),
        "potion" => {
            let cmd = action.command.as_deref()?;
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "potion" && parts[1] == "use" {
                let slot = parts[2].parse::<usize>().ok()?;
                let target = parts
                    .get(3)
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|idx| combat.entities.monsters.get(idx).map(|m| m.id));
                Some(ClientInput::UsePotion {
                    potion_index: slot,
                    target,
                })
            } else {
                None
            }
        }
        "end_turn" => Some(ClientInput::EndTurn),
        _ => None,
    }
}

fn source_descriptor(combat: &CombatState, input: &ClientInput) -> (String, String, String) {
    match input {
        ClientInput::PlayCard { card_index, .. } => {
            if let Some(card) = combat.zones.hand.get(*card_index) {
                let def = cards::get_card_definition(card.id);
                (
                    "card".to_string(),
                    def.name.to_string(),
                    card_target_shape(def.target).to_string(),
                )
            } else {
                (
                    "card".to_string(),
                    "unknown_card".to_string(),
                    "none".to_string(),
                )
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let source_id = combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .map(|p| potions::get_potion_definition(p.id).name.to_string())
                .unwrap_or_else(|| "unknown_potion".to_string());
            let target_shape = if target.is_some() {
                "single_enemy"
            } else {
                "none"
            };
            ("potion".to_string(), source_id, target_shape.to_string())
        }
        ClientInput::EndTurn => (
            "monster_turn".to_string(),
            "end_turn".to_string(),
            "none".to_string(),
        ),
        ClientInput::SubmitHandSelect(_) => (
            "pending_choice".to_string(),
            "hand_select".to_string(),
            "none".to_string(),
        ),
        ClientInput::SubmitGridSelect(_) => (
            "pending_choice".to_string(),
            "grid_select".to_string(),
            "none".to_string(),
        ),
        ClientInput::SubmitDiscoverChoice(_) => (
            "pending_choice".to_string(),
            "discover_choice".to_string(),
            "none".to_string(),
        ),
        ClientInput::Proceed => (
            "pending_choice".to_string(),
            "proceed".to_string(),
            "none".to_string(),
        ),
        ClientInput::Cancel => (
            "pending_choice".to_string(),
            "cancel".to_string(),
            "none".to_string(),
        ),
        _ => (
            "pending_choice".to_string(),
            "other".to_string(),
            "none".to_string(),
        ),
    }
}

fn card_target_shape(target: CardTarget) -> &'static str {
    match target {
        CardTarget::Enemy => "single_enemy",
        CardTarget::AllEnemy => "aoe",
        CardTarget::SelfTarget => "self",
        CardTarget::None => "none",
    }
}

fn pending_choice_descriptor(before_engine: &EngineState, after_engine: &EngineState) -> String {
    let engine = if matches!(after_engine, EngineState::PendingChoice(_)) {
        after_engine
    } else {
        before_engine
    };
    match engine {
        EngineState::PendingChoice(PendingChoice::HandSelect { reason, .. }) => {
            format!("hand_select:{reason:?}")
        }
        EngineState::PendingChoice(PendingChoice::GridSelect { reason, .. }) => {
            format!("grid_select:{reason:?}")
        }
        EngineState::PendingChoice(PendingChoice::DiscoverySelect(_)) => "discovery".to_string(),
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => "reward".to_string(),
        EngineState::PendingChoice(PendingChoice::StanceChoice) => "stance".to_string(),
        _ => "none".to_string(),
    }
}

fn collect_key_power_tags(before: &CombatState, after: &CombatState) -> Vec<String> {
    let mut tags = BTreeSet::new();
    for &(power_id, tag) in KEY_POWER_TAGS {
        if state_has_power(before, power_id) || state_has_power(after, power_id) {
            tags.insert(tag.to_string());
        }
    }
    tags.into_iter().collect()
}

fn state_has_power(state: &CombatState, power_id: PowerId) -> bool {
    state
        .entities
        .power_db
        .values()
        .any(|powers| powers.iter().any(|p| p.power_type == power_id))
}

fn monsters_took_damage(before: &CombatState, after: &CombatState) -> bool {
    before
        .entities
        .monsters
        .iter()
        .zip(after.entities.monsters.iter())
        .any(|(b, a)| {
            a.current_hp < b.current_hp || (a.block < b.block && b.current_hp == a.current_hp)
        })
}

fn any_new_monster_death(before: &CombatState, after: &CombatState) -> bool {
    before
        .entities
        .monsters
        .iter()
        .zip(after.entities.monsters.iter())
        .any(|(b, a)| !monster_unavailable(b) && monster_unavailable(a) && !a.half_dead)
}

fn any_new_half_dead(before: &CombatState, after: &CombatState) -> bool {
    before
        .entities
        .monsters
        .iter()
        .zip(after.entities.monsters.iter())
        .any(|(b, a)| !b.half_dead && a.half_dead)
}

fn any_revive(before: &CombatState, after: &CombatState) -> bool {
    before
        .entities
        .monsters
        .iter()
        .zip(after.entities.monsters.iter())
        .any(|(b, a)| {
            (b.half_dead || monster_unavailable(b)) && !monster_unavailable(a) && a.current_hp > 0
        })
}

fn spawned_monsters(before: &CombatState, after: &CombatState) -> bool {
    after.entities.monsters.len() > before.entities.monsters.len()
        || before
            .entities
            .monsters
            .iter()
            .zip(after.entities.monsters.iter())
            .any(|(b, a)| monster_unavailable(b) && !monster_unavailable(a) && a.current_hp > 0)
}

fn monster_unavailable(monster: &crate::combat::MonsterEntity) -> bool {
    monster.is_dying || monster.is_escaped || monster.current_hp <= 0
}

fn high_value_flags(signature: &InteractionSignature) -> Vec<String> {
    let mut flags = Vec::new();
    if signature.pending_choice != "none" {
        flags.push("pending_choice".to_string());
    }
    if signature
        .pile_tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "exhaust" | "draw" | "shuffle"))
    {
        flags.push("pile_chain".to_string());
    }
    if signature
        .outcome_tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "half_dead" | "revives" | "spawns"))
    {
        flags.push("rebirth_or_spawn".to_string());
    }
    if signature.power_tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "Artifact" | "NoDraw" | "Shackled" | "Malleable" | "CurlUp"
        )
    }) {
        flags.push("reactive_power".to_string());
    }
    flags
}

fn known_card_sources() -> Vec<String> {
    let mut ids: Vec<_> = crate::content::cards::build_java_id_map()
        .values()
        .copied()
        .collect();
    ids.sort_by_key(|id| crate::content::cards::get_card_definition(*id).name);
    ids.dedup();
    ids.into_iter()
        .filter_map(|id| {
            let def = crate::content::cards::get_card_definition(id);
            match def.card_type {
                crate::content::cards::CardType::Attack
                | crate::content::cards::CardType::Skill
                | crate::content::cards::CardType::Power => Some(def.name.to_string()),
                _ => None,
            }
        })
        .collect()
}

fn known_potion_sources() -> Vec<String> {
    all_potion_ids()
        .into_iter()
        .map(|id| {
            crate::content::potions::get_potion_definition(id)
                .name
                .to_string()
        })
        .collect()
}

fn all_potion_ids() -> Vec<crate::content::potions::PotionId> {
    use crate::content::potions::PotionId::*;
    vec![
        FirePotion,
        ExplosivePotion,
        PoisonPotion,
        WeakenPotion,
        FearPotion,
        BlockPotion,
        BloodPotion,
        EnergyPotion,
        StrengthPotion,
        DexterityPotion,
        SpeedPotion,
        SteroidPotion,
        SwiftPotion,
        FocusPotion,
        AttackPotion,
        SkillPotion,
        PowerPotion,
        ColorlessPotion,
        BottledMiracle,
        BlessingOfTheForge,
        AncientPotion,
        RegenPotion,
        EssenceOfSteel,
        LiquidBronze,
        DistilledChaosPotion,
        DuplicationPotion,
        CunningPotion,
        PotionOfCapacity,
        LiquidMemories,
        GamblersBrew,
        Elixir,
        StancePotion,
        FairyPotion,
        SmokeBomb,
        FruitJuice,
        EntropicBrew,
        SneckoOil,
        GhostInAJar,
        HeartOfIron,
        CultistPotion,
        Ambrosia,
        EssenceOfDarkness,
    ]
}

fn equals_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}

pub fn command_string(input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            format!("play:{}:{:?}", card_index, target)
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => format!("potion:{}:{:?}", potion_index, target),
        ClientInput::EndTurn => "end_turn".to_string(),
        ClientInput::SubmitHandSelect(cards) => format!("hand_select:{}", cards.len()),
        ClientInput::SubmitGridSelect(cards) => format!("grid_select:{}", cards.len()),
        ClientInput::SubmitDiscoverChoice(idx) => format!("discover:{idx}"),
        ClientInput::Proceed => "proceed".to_string(),
        ClientInput::Cancel => "cancel".to_string(),
        _ => "other".to_string(),
    }
}

pub fn default_replay_inputs(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let replay_short = manifest_dir.join("tools/replay_short.jsonl");
    if replay_short.exists() {
        inputs.push(replay_short);
    }
    let replays_dir = manifest_dir.join("tools/replays");
    if replays_dir.exists() {
        let mut replay_files: Vec<_> = std::fs::read_dir(replays_dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
            .collect();
        replay_files.sort();
        inputs.extend(replay_files);
    }
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::Intent;
    use crate::combat::{
        CombatCard, CombatPhase, MonsterEntity, PlayerEntity, RelicBuses, StanceId,
    };
    use crate::content::cards::CardId;
    use std::collections::VecDeque;

    fn simple_combat(card_id: CardId, target: CardTarget) -> CombatState {
        let mut card = CombatCard::new(card_id, 111);
        card.upgrades = 0;
        let state = CombatState {
            meta: crate::combat::CombatMeta {
                ascension_level: 0,
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: crate::combat::TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 3,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: crate::combat::CardZones {
                draw_pile: Vec::new(),
                hand: vec![card],
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 222,
            },
            entities: crate::combat::EntityState {
                player: PlayerEntity {
                    id: 0,
                    current_hp: 80,
                    max_hp: 80,
                    block: 0,
                    gold_delta_this_combat: 0,
                    gold: 99,
                    max_orbs: 0,
                    orbs: Vec::new(),
                    stance: StanceId::Neutral,
                    relics: Vec::new(),
                    relic_buses: RelicBuses::default(),
                    energy_master: 3,
                },
                monsters: vec![MonsterEntity {
                    id: 1,
                    monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
                    current_hp: 30,
                    max_hp: 30,
                    block: 0,
                    slot: 0,
                    is_dying: false,
                    is_escaped: false,
                    half_dead: false,
                    next_move_byte: 0,
                    current_intent: Intent::Unknown,
                    move_history: VecDeque::new(),
                    intent_dmg: 0,
                    logical_position: 0,
                    hexaghost: Default::default(),
                    darkling: Default::default(),
                }],
                potions: vec![None, None, None],
                power_db: HashMap::new(),
            },
            engine: crate::combat::EngineRuntime {
                action_queue: VecDeque::new(),
            },
            rng: crate::combat::CombatRng::new(crate::rng::RngPool::new(123)),
        };
        let def = cards::get_card_definition(card_id);
        assert_eq!(def.target, target);
        state
    }

    #[test]
    fn canonical_key_ignores_runtime_hp_and_uuid_noise() {
        let before = simple_combat(CardId::Strike, CardTarget::Enemy);
        let mut after_a = before.clone();
        after_a.zones.hand.clear();
        after_a
            .zones
            .discard_pile
            .push(CombatCard::new(CardId::Strike, 9001));
        after_a.entities.monsters[0].current_hp = 24;

        let mut after_b = before.clone();
        after_b.zones.hand.clear();
        after_b
            .zones
            .discard_pile
            .push(CombatCard::new(CardId::Strike, 42));
        after_b.entities.monsters[0].current_hp = 19;

        let sig_a = signature_from_transition(
            &EngineState::CombatPlayerTurn,
            &before,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            &EngineState::CombatPlayerTurn,
            &after_a,
        );
        let sig_b = signature_from_transition(
            &EngineState::CombatPlayerTurn,
            &before,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            &EngineState::CombatPlayerTurn,
            &after_b,
        );

        assert_eq!(sig_a.canonical_key(), sig_b.canonical_key());
    }

    #[test]
    fn canonicalization_sorts_tag_order() {
        let mut sig = InteractionSignature {
            source_kind: "card".to_string(),
            source_id: "Second Wind".to_string(),
            target_shape: "none".to_string(),
            pending_choice: "none".to_string(),
            archetype_tags: vec!["hybrid".into(), "exhaust".into()],
            hook_tags: vec!["on_death".into(), "on_use_card".into()],
            pile_tags: vec!["shuffle".into(), "draw".into(), "shuffle".into()],
            power_tags: vec!["FeelNoPain".into(), "DarkEmbrace".into()],
            outcome_tags: vec!["draws_cards".into(), "kills".into()],
        };
        sig.canonicalize();
        assert_eq!(
            sig.pile_tags,
            vec!["draw".to_string(), "shuffle".to_string()]
        );
        assert_eq!(
            sig.canonical_key(),
            "card|Second Wind|none|none|archetypes=exhaust,hybrid|hooks=on_death,on_use_card|piles=draw,shuffle|powers=DarkEmbrace,FeelNoPain|outcomes=draws_cards,kills"
        );
    }

    #[test]
    fn replay_extracts_gamblers_brew_hand_select_signature() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl");
        let records = replay_records_from_path(&path);
        assert!(records.iter().any(|record| {
            record.signature.source_id == "Gambler's Brew"
                && record
                    .signature
                    .pending_choice
                    .contains("hand_select:GamblingChip")
        }));
    }

    #[test]
    fn replay_extracts_awakened_one_half_dead_and_revival_signatures() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl");
        let records = replay_records_from_path(&path);
        assert!(records.iter().any(|record| {
            record.signature.source_id == "Perfected Strike"
                && record
                    .signature
                    .outcome_tags
                    .contains(&"half_dead".to_string())
        }));
        assert!(records.iter().any(|record| {
            record.signature.source_id == "end_turn"
                && record
                    .signature
                    .outcome_tags
                    .contains(&"revives".to_string())
        }));
    }

    #[test]
    fn replay_extracts_transient_shackled_power_tag() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tools/replays/2026-03-28-20-17-13--4B45WXW03YIF.jsonl");
        let records = replay_records_from_path(&path);
        assert!(records.iter().any(|record| {
            record.signature.source_id == "Shrug It Off"
                && record
                    .signature
                    .power_tags
                    .contains(&"Shackled".to_string())
        }));
    }

    #[test]
    fn novelty_bonus_prefers_unseen_signatures_over_seen_ones() {
        let mut db = crate::bot::coverage::CoverageDb::default();
        db.tested_signatures.insert("known_signature".to_string());
        db.source_signature_counts
            .insert("known_source".to_string(), 2);

        let unseen = novelty_bonus(
            Some("unseen_signature"),
            Some("unseen_source"),
            &db,
            crate::bot::coverage::CoverageMode::PreferNovel,
        );
        let seen = novelty_bonus(
            Some("known_signature"),
            Some("known_source"),
            &db,
            crate::bot::coverage::CoverageMode::PreferNovel,
        );

        assert!(unseen > seen);
        assert_eq!(
            novelty_bonus(
                Some("unseen_signature"),
                Some("unseen_source"),
                &db,
                crate::bot::coverage::CoverageMode::Off,
            ),
            0.0
        );
    }

    #[test]
    fn curiosity_target_matches_card_and_power_tags() {
        let mut sig = InteractionSignature {
            source_kind: "card".to_string(),
            source_id: "Second Wind".to_string(),
            target_shape: "none".to_string(),
            pending_choice: "hand_select:Exhaust".to_string(),
            archetype_tags: vec!["exhaust".into()],
            hook_tags: vec!["on_use_card".into()],
            pile_tags: vec!["exhaust".into()],
            power_tags: vec!["DarkEmbrace".into()],
            outcome_tags: vec!["draws_cards".into()],
        };
        sig.canonicalize();

        assert!(curiosity_target_matches(
            &sig,
            &crate::bot::coverage::CuriosityTarget::card("Second Wind")
        ));
        assert!(curiosity_target_matches(
            &sig,
            &crate::bot::coverage::CuriosityTarget::power_tag("DarkEmbrace")
        ));
        assert!(curiosity_target_matches(
            &sig,
            &crate::bot::coverage::CuriosityTarget::archetype("exhaust")
        ));
        assert!(!curiosity_target_matches(
            &sig,
            &crate::bot::coverage::CuriosityTarget::potion("Gamblers Brew")
        ));
    }
}