use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::content::cards::{self, CardTarget};
use crate::content::potions;
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatState;
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

pub fn signature_from_transition_with_archetypes(
    before_engine: &EngineState,
    before: &CombatState,
    input: &ClientInput,
    after_engine: &EngineState,
    after: &CombatState,
    archetype_tags: Vec<String>,
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
        archetype_tags,
        hook_tags: hook_tags.into_iter().collect(),
        pile_tags: pile_tags.into_iter().collect(),
        power_tags: collect_key_power_tags(before, after),
        outcome_tags: outcome_tags.into_iter().collect(),
    };
    signature.canonicalize();
    signature
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

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
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

fn monster_unavailable(monster: &crate::runtime::combat::MonsterEntity) -> bool {
    monster.is_dying || monster.is_escaped || monster.current_hp <= 0
}
