use std::collections::BTreeMap;

use crate::bot::card_taxonomy::taxonomy;
use crate::runtime::combat::{CombatCard, CombatState, Power, PowerId};
use crate::content::cards::{get_card_definition, CardType};
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::profile::{SearchProfileCollector, SearchProfilePhase};
use super::root_policy::action_semantic_tags;
use super::root_rollout::{advance_to_decision_point_profiled, total_enemy_hp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceMode {
    Off,
    Safe,
    Experimental,
}

impl SearchEquivalenceMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Safe => "safe",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchEquivalenceKind {
    Exact,
    Heuristic,
}

impl SearchEquivalenceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Heuristic => "heuristic",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ReducedSearchMove {
    pub input: ClientInput,
    pub next_engine: EngineState,
    pub next_combat: CombatState,
    pub cluster_id: String,
    pub cluster_size: usize,
    pub collapsed_inputs: Vec<ClientInput>,
    pub equivalence_kind: Option<SearchEquivalenceKind>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum TransitionFamily {
    EndTurn,
    PureBlock,
    VanillaAttack,
    OtherCard,
    Potion,
    OtherInput,
}

impl TransitionFamily {
    const fn as_str(self) -> &'static str {
        match self {
            Self::EndTurn => "end_turn",
            Self::PureBlock => "pure_block",
            Self::VanillaAttack => "vanilla_attack",
            Self::OtherCard => "other_card",
            Self::Potion => "potion",
            Self::OtherInput => "other_input",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CardFingerprint {
    id_label: String,
    upgrades: u8,
    misc_value: i32,
    base_damage_override: Option<i32>,
    cost_modifier: i8,
    cost_for_turn: Option<u8>,
    base_damage_mut: i32,
    base_block_mut: i32,
    base_magic_num_mut: i32,
    multi_damage: Vec<i32>,
    exhaust_override: Option<bool>,
    retain_override: Option<bool>,
    free_to_play_once: bool,
    energy_on_use: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MonsterFingerprint {
    current_hp: i32,
    max_hp: i32,
    block: i32,
    is_dying: bool,
    is_escaped: bool,
    half_dead: bool,
    intent_dmg: i32,
    intent_label: String,
    power_signature: Vec<(u32, i32, i32, bool)>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SemanticStateFingerprint {
    engine_label: String,
    player_hp: i32,
    player_block: i32,
    energy: u8,
    stance_label: String,
    player_power_signature: Vec<(u32, i32, i32, bool)>,
    monsters: Vec<MonsterFingerprint>,
    hand: Vec<(CardFingerprint, usize)>,
    draw: Vec<(CardFingerprint, usize)>,
    discard: Vec<(CardFingerprint, usize)>,
    exhaust: Vec<(CardFingerprint, usize)>,
    potion_ids: Vec<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ActionEquivalenceProfile {
    family: TransitionFamily,
    target: Option<usize>,
    cost: Option<i8>,
    energy_delta: i32,
    block_delta: i32,
    player_hp_delta: i32,
    enemy_total_delta: i32,
    target_hp_delta: i32,
    hand_len_delta: i32,
    draw_len_delta: i32,
    discard_len_delta: i32,
    exhaust_len_delta: i32,
    target_space_changed: bool,
    kill_likely: bool,
    changes_damage_multiplier: bool,
    randomness_sensitive: bool,
    engine_trigger_sensitive: bool,
    changes_hand_shape: bool,
    changes_energy: bool,
    card_label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExperimentalFingerprint {
    family: TransitionFamily,
    target: Option<usize>,
    cost: Option<i8>,
    energy_delta: i32,
    block_delta: i32,
    player_hp_delta: i32,
    enemy_total_delta: i32,
    target_hp_delta: i32,
    hand_len_delta: i32,
    draw_len_delta: i32,
    discard_len_delta: i32,
    exhaust_len_delta: i32,
}

#[derive(Clone)]
struct RawTransition {
    input: ClientInput,
    next_engine: EngineState,
    next_combat: CombatState,
    profile: ActionEquivalenceProfile,
    exact_fingerprint: Option<SemanticStateFingerprint>,
    experimental_fingerprint: ExperimentalFingerprint,
    canonical_key: String,
}

struct TransitionCluster {
    mode: SearchEquivalenceMode,
    family: TransitionFamily,
    members: Vec<RawTransition>,
    kind: Option<SearchEquivalenceKind>,
}

pub(crate) fn default_equivalence_mode() -> SearchEquivalenceMode {
    match std::env::var("STS_SEARCH_EQUIVALENCE_MODE")
        .unwrap_or_else(|_| "safe".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "off" => SearchEquivalenceMode::Off,
        "experimental" | "exp" => SearchEquivalenceMode::Experimental,
        _ => SearchEquivalenceMode::Safe,
    }
}

pub(crate) fn reduce_search_moves(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
    max_engine_steps: usize,
    mode: SearchEquivalenceMode,
    profiler: &mut SearchProfileCollector,
    phase: SearchProfilePhase,
) -> Vec<ReducedSearchMove> {
    let equivalence_enabled = should_attempt_equivalence(engine, legal_moves, mode);
    let mut raw_transitions = legal_moves
        .iter()
        .cloned()
        .map(|input| {
            profiler.record_clone_calls(phase, 2);
            let mut next_engine = engine.clone();
            let mut next_combat = combat.clone();
            advance_to_decision_point_profiled(
                &mut next_engine,
                &mut next_combat,
                input.clone(),
                max_engine_steps,
                Some(profiler),
            );
            let profile = action_equivalence_profile(combat, &input, &next_combat);
            let exact_fingerprint = if equivalence_enabled {
                Some(semantic_state_fingerprint(&next_engine, &next_combat))
            } else {
                None
            };
            let experimental_fingerprint = ExperimentalFingerprint {
                family: profile.family,
                target: profile.target,
                cost: profile.cost,
                energy_delta: profile.energy_delta,
                block_delta: profile.block_delta,
                player_hp_delta: profile.player_hp_delta,
                enemy_total_delta: profile.enemy_total_delta,
                target_hp_delta: profile.target_hp_delta,
                hand_len_delta: profile.hand_len_delta,
                draw_len_delta: profile.draw_len_delta,
                discard_len_delta: profile.discard_len_delta,
                exhaust_len_delta: profile.exhaust_len_delta,
            };
            let canonical_key = canonical_transition_key(combat, &input);
            RawTransition {
                input,
                next_engine,
                next_combat,
                profile,
                exact_fingerprint,
                experimental_fingerprint,
                canonical_key,
            }
        })
        .collect::<Vec<_>>();

    raw_transitions.sort_by(|left, right| left.canonical_key.cmp(&right.canonical_key));

    if !equivalence_enabled {
        return raw_transitions
            .into_iter()
            .map(|transition| ReducedSearchMove {
                input: transition.input,
                next_engine: transition.next_engine,
                next_combat: transition.next_combat,
                cluster_id: format!("off:{}", transition.canonical_key),
                cluster_size: 1,
                collapsed_inputs: Vec::new(),
                equivalence_kind: None,
            })
            .collect();
    }

    let mut clusters = Vec::<TransitionCluster>::new();
    for transition in raw_transitions {
        let mut placed = false;
        for cluster in &mut clusters {
            if let Some(kind) = cluster_equivalence_kind(cluster, &transition, mode) {
                cluster.members.push(transition.clone());
                cluster.kind = Some(match (cluster.kind, kind) {
                    (Some(SearchEquivalenceKind::Heuristic), _)
                    | (_, SearchEquivalenceKind::Heuristic) => SearchEquivalenceKind::Heuristic,
                    _ => SearchEquivalenceKind::Exact,
                });
                placed = true;
                break;
            }
        }
        if !placed {
            clusters.push(TransitionCluster {
                mode,
                family: transition.profile.family,
                members: vec![transition],
                kind: None,
            });
        }
    }

    clusters
        .into_iter()
        .enumerate()
        .map(|(cluster_index, mut cluster)| {
            cluster
                .members
                .sort_by(|left, right| left.canonical_key.cmp(&right.canonical_key));
            let representative = cluster.members.remove(0);
            let collapsed_inputs = cluster
                .members
                .iter()
                .map(|member| member.input.clone())
                .collect::<Vec<_>>();
            let equivalence_kind = if collapsed_inputs.is_empty() {
                None
            } else {
                cluster.kind.or(Some(SearchEquivalenceKind::Exact))
            };

            ReducedSearchMove {
                input: representative.input,
                next_engine: representative.next_engine,
                next_combat: representative.next_combat,
                cluster_id: format!(
                    "{}:{}:{}:{}",
                    cluster.mode.as_str(),
                    equivalence_kind
                        .unwrap_or(SearchEquivalenceKind::Exact)
                        .as_str(),
                    cluster.family.as_str(),
                    cluster_index + 1
                ),
                cluster_size: collapsed_inputs.len() + 1,
                collapsed_inputs,
                equivalence_kind,
            }
        })
        .collect()
}

fn cluster_equivalence_kind(
    cluster: &TransitionCluster,
    candidate: &RawTransition,
    mode: SearchEquivalenceMode,
) -> Option<SearchEquivalenceKind> {
    let representative = cluster.members.first()?;
    equivalent_transition_kind(representative, candidate, mode)
}

fn equivalent_transition_kind(
    left: &RawTransition,
    right: &RawTransition,
    mode: SearchEquivalenceMode,
) -> Option<SearchEquivalenceKind> {
    if !is_reduction_candidate(&left.profile) || !is_reduction_candidate(&right.profile) {
        return None;
    }
    if left.profile.family != right.profile.family {
        return None;
    }
    if left.profile.target_space_changed || right.profile.target_space_changed {
        return None;
    }
    if left.profile.kill_likely || right.profile.kill_likely {
        return None;
    }
    if left.profile.changes_damage_multiplier || right.profile.changes_damage_multiplier {
        return None;
    }
    if left.profile.randomness_sensitive || right.profile.randomness_sensitive {
        return None;
    }
    if left.profile.engine_trigger_sensitive || right.profile.engine_trigger_sensitive {
        return None;
    }
    if left.profile.changes_hand_shape || right.profile.changes_hand_shape {
        return None;
    }
    if left.profile.changes_energy || right.profile.changes_energy {
        return None;
    }

    if left.exact_fingerprint.is_some() && left.exact_fingerprint == right.exact_fingerprint {
        return Some(SearchEquivalenceKind::Exact);
    }
    if mode == SearchEquivalenceMode::Experimental
        && left.experimental_fingerprint == right.experimental_fingerprint
    {
        return Some(SearchEquivalenceKind::Heuristic);
    }
    None
}

fn should_attempt_equivalence(
    engine: &EngineState,
    legal_moves: &[ClientInput],
    mode: SearchEquivalenceMode,
) -> bool {
    if mode == SearchEquivalenceMode::Off {
        return false;
    }
    if legal_moves.len() <= 2 {
        return false;
    }
    matches!(engine, EngineState::CombatPlayerTurn)
}

fn is_reduction_candidate(profile: &ActionEquivalenceProfile) -> bool {
    matches!(
        profile.family,
        TransitionFamily::PureBlock | TransitionFamily::VanillaAttack
    )
}

fn action_equivalence_profile(
    combat: &CombatState,
    input: &ClientInput,
    next_combat: &CombatState,
) -> ActionEquivalenceProfile {
    let tags = action_semantic_tags(combat, input);
    let before_enemy_count = live_enemy_count(combat);
    let after_enemy_count = live_enemy_count(next_combat);
    let target = target_from_input(input);
    let target_hp_delta = target
        .and_then(|entity_id| {
            let before = combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id)?;
            let after = next_combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id)?;
            Some(before.current_hp + before.block - after.current_hp - after.block)
        })
        .unwrap_or(0);

    let changes_damage_multiplier =
        multiplier_signature(combat) != multiplier_signature(next_combat);

    let (family, cost, card_label, randomness_sensitive, engine_trigger_sensitive) = match input {
        ClientInput::PlayCard { card_index, target } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return default_profile_for_input(input);
            };
            let definition = get_card_definition(card.id);
            let tax = taxonomy(card.id);
            let enemy_total_delta = total_enemy_hp(combat) - total_enemy_hp(next_combat);
            let block_delta = next_combat.entities.player.block - combat.entities.player.block;
            let hp_delta =
                next_combat.entities.player.current_hp - combat.entities.player.current_hp;
            let pure_block_only = definition.card_type == CardType::Skill
                && block_delta > 0
                && enemy_total_delta == 0
                && hp_delta == 0
                && target.is_none();
            let vanilla_attack = definition.card_type == CardType::Attack
                && target.is_some()
                && enemy_total_delta > 0
                && block_delta == 0
                && hp_delta == 0
                && !tax.is_aoe()
                && !tax.is_multi_hit()
                && !tax.is_vuln_enabler()
                && !tax.is_weak_enabler()
                && !tax.is_status_producer()
                && !tax.is_status_engine()
                && !tax.is_self_damage_source()
                && !tax.is_self_damage_payoff();
            let family = if pure_block_only {
                TransitionFamily::PureBlock
            } else if vanilla_attack {
                TransitionFamily::VanillaAttack
            } else {
                TransitionFamily::OtherCard
            };
            let randomness_sensitive = tax.is_multi_hit()
                || matches!(
                    card.id,
                    crate::content::cards::CardId::SwordBoomerang
                        | crate::content::cards::CardId::InfernalBlade
                        | crate::content::cards::CardId::Discovery
                        | crate::content::cards::CardId::RecklessCharge
                );
            let engine_trigger_sensitive = tags.persistent_setup
                || tags.exhaust_engine
                || tags.exhaust_trigger
                || tags.draw_core
                || tags.resource_bridge
                || tax.is_scaling_power()
                || tax.is_engine_piece()
                || tax.is_status_engine()
                || tax.is_exhaust_recovery()
                || tax.is_discard_cycle()
                || tax.is_discard_retrieval()
                || tax.is_vuln_enabler()
                || tax.is_weak_enabler()
                || tax.is_status_producer()
                || tax.is_self_damage_source()
                || tax.is_self_damage_payoff()
                || tax.is_aoe()
                || tax.is_multi_hit();

            (
                family,
                Some(card.get_cost()),
                describe_card(card),
                randomness_sensitive,
                engine_trigger_sensitive,
            )
        }
        ClientInput::EndTurn => (
            TransitionFamily::EndTurn,
            None,
            "EndTurn".to_string(),
            false,
            true,
        ),
        ClientInput::UsePotion { potion_index, .. } => (
            TransitionFamily::Potion,
            None,
            format!("UsePotion#{potion_index}"),
            true,
            true,
        ),
        _ => (
            TransitionFamily::OtherInput,
            None,
            format!("{input:?}"),
            true,
            true,
        ),
    };

    ActionEquivalenceProfile {
        family,
        target,
        cost,
        energy_delta: next_combat.turn.energy as i32 - combat.turn.energy as i32,
        block_delta: next_combat.entities.player.block - combat.entities.player.block,
        player_hp_delta: next_combat.entities.player.current_hp - combat.entities.player.current_hp,
        enemy_total_delta: total_enemy_hp(combat) - total_enemy_hp(next_combat),
        target_hp_delta,
        hand_len_delta: next_combat.zones.hand.len() as i32 - combat.zones.hand.len() as i32,
        draw_len_delta: next_combat.zones.draw_pile.len() as i32
            - combat.zones.draw_pile.len() as i32,
        discard_len_delta: next_combat.zones.discard_pile.len() as i32
            - combat.zones.discard_pile.len() as i32,
        exhaust_len_delta: next_combat.zones.exhaust_pile.len() as i32
            - combat.zones.exhaust_pile.len() as i32,
        target_space_changed: before_enemy_count != after_enemy_count,
        kill_likely: after_enemy_count < before_enemy_count,
        changes_damage_multiplier,
        randomness_sensitive,
        engine_trigger_sensitive,
        changes_hand_shape: hand_shape_changed(combat, next_combat),
        changes_energy: energy_shape_changed(combat, next_combat, cost),
        card_label,
    }
}

fn default_profile_for_input(input: &ClientInput) -> ActionEquivalenceProfile {
    ActionEquivalenceProfile {
        family: TransitionFamily::OtherInput,
        target: None,
        cost: None,
        energy_delta: 0,
        block_delta: 0,
        player_hp_delta: 0,
        enemy_total_delta: 0,
        target_hp_delta: 0,
        hand_len_delta: 0,
        draw_len_delta: 0,
        discard_len_delta: 0,
        exhaust_len_delta: 0,
        target_space_changed: true,
        kill_likely: false,
        changes_damage_multiplier: true,
        randomness_sensitive: true,
        engine_trigger_sensitive: true,
        changes_hand_shape: true,
        changes_energy: true,
        card_label: format!("{input:?}"),
    }
}

fn hand_shape_changed(before: &CombatState, after: &CombatState) -> bool {
    card_zone_shape(&before.zones.hand) != card_zone_shape(&after.zones.hand)
        && after.zones.hand.len() + 1 != before.zones.hand.len()
}

fn energy_shape_changed(before: &CombatState, after: &CombatState, cost: Option<i8>) -> bool {
    let Some(cost) = cost else {
        return true;
    };
    if cost < 0 {
        return true;
    }
    before.turn.energy as i32 - after.turn.energy as i32 != cost as i32
}

fn canonical_transition_key(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card_label = combat
                .zones
                .hand
                .get(*card_index)
                .map(describe_card)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            format!("card:{card_label}:idx={card_index}:target={target:?}")
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => format!("potion:{potion_index}:target={target:?}"),
        ClientInput::EndTurn => "end_turn".to_string(),
        other => format!("{other:?}"),
    }
}

fn describe_card(card: &CombatCard) -> String {
    format!(
        "{}+{}:cost={}:uuid={}",
        get_card_definition(card.id).name,
        card.upgrades,
        card.get_cost(),
        card.uuid
    )
}

fn target_from_input(input: &ClientInput) -> Option<usize> {
    match input {
        ClientInput::PlayCard { target, .. } | ClientInput::UsePotion { target, .. } => *target,
        _ => None,
    }
}

fn live_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .filter(|monster| monster.current_hp > 0)
        .count()
}

fn multiplier_signature(combat: &CombatState) -> Vec<(i32, i32, i32, i32)> {
    let relevant = [
        PowerId::Strength,
        PowerId::Dexterity,
        PowerId::Weak,
        PowerId::Vulnerable,
    ];
    let mut signature = Vec::new();
    signature.push(power_row(combat, 0, &relevant));
    for monster in &combat.entities.monsters {
        signature.push(power_row(combat, monster.id, &relevant));
    }
    signature
}

fn power_row(
    combat: &CombatState,
    entity_id: usize,
    relevant: &[PowerId; 4],
) -> (i32, i32, i32, i32) {
    let mut row = [0; 4];
    for (index, power_id) in relevant.iter().enumerate() {
        row[index] = combat.get_power(entity_id, *power_id);
    }
    (row[0], row[1], row[2], row[3])
}

fn semantic_state_fingerprint(
    engine: &EngineState,
    combat: &CombatState,
) -> SemanticStateFingerprint {
    SemanticStateFingerprint {
        engine_label: format!("{engine:?}"),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        stance_label: combat.entities.player.stance.as_str().to_string(),
        player_power_signature: power_fingerprint(
            combat
                .entities
                .power_db
                .get(&0)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
        ),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| MonsterFingerprint {
                current_hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                is_dying: monster.is_dying,
                is_escaped: monster.is_escaped,
                half_dead: monster.half_dead,
                intent_dmg: monster.intent_dmg,
                intent_label: format!("{:?}", monster.current_intent),
                power_signature: power_fingerprint(
                    combat
                        .entities
                        .power_db
                        .get(&monster.id)
                        .map(Vec::as_slice)
                        .unwrap_or(&[]),
                ),
            })
            .collect(),
        hand: card_zone_shape(&combat.zones.hand),
        draw: card_zone_shape(&combat.zones.draw_pile),
        discard: card_zone_shape(&combat.zones.discard_pile),
        exhaust: card_zone_shape(&combat.zones.exhaust_pile),
        potion_ids: combat
            .entities
            .potions
            .iter()
            .map(|slot| {
                slot.as_ref().map(|potion| {
                    crate::content::potions::get_potion_definition(potion.id)
                        .name
                        .to_string()
                })
            })
            .collect(),
    }
}

fn card_zone_shape(cards: &[CombatCard]) -> Vec<(CardFingerprint, usize)> {
    let mut counts = BTreeMap::<CardFingerprint, usize>::new();
    for card in cards {
        *counts.entry(card_fingerprint(card)).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

fn card_fingerprint(card: &CombatCard) -> CardFingerprint {
    CardFingerprint {
        id_label: get_card_definition(card.id).name.to_string(),
        upgrades: card.upgrades,
        misc_value: card.misc_value,
        base_damage_override: card.base_damage_override,
        cost_modifier: card.cost_modifier,
        cost_for_turn: card.cost_for_turn,
        base_damage_mut: card.base_damage_mut,
        base_block_mut: card.base_block_mut,
        base_magic_num_mut: card.base_magic_num_mut,
        multi_damage: card.multi_damage.iter().copied().collect(),
        exhaust_override: card.exhaust_override,
        retain_override: card.retain_override,
        free_to_play_once: card.free_to_play_once,
        energy_on_use: card.energy_on_use,
    }
}

fn power_fingerprint(powers: &[Power]) -> Vec<(u32, i32, i32, bool)> {
    let mut fingerprint = powers
        .iter()
        .map(|power| {
            (
                power.power_type as u32,
                power.amount,
                power.extra_data,
                power.just_applied,
            )
        })
        .collect::<Vec<_>>();
    fingerprint.sort();
    fingerprint
}
