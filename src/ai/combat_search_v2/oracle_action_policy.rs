use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades as strategic_card_definition, CombatEvent, PlayEffect,
};
use crate::sim::combat::{CombatPosition, CombatStepper, EngineCombatStepper};
use crate::state::core::ClientInput;
use crate::{
    content::{
        cards::{exhausts_when_played, get_card_definition, CardType},
        powers::{store, PowerId},
    },
    runtime::combat::{CombatCard, CombatState},
};
use serde::Serialize;

use super::action_ordering::{order_indexed_action_choices_with_plugins, IndexedActionChoice};
use super::frontier::SearchNode;
use super::value::combat_search_state_value_for_state;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleCombatRolloutGuideV1 {
    pub components: Vec<i32>,
    pub actions_simulated: usize,
}

/// One shared evaluation of the typed combat-state knowledge consumed by the
/// planner's independent guide lanes.
///
/// Building the guides separately used to recompute the same phase profile,
/// hand/draw facts, incoming pressure, and setup summary once per lane.  The
/// planner normally asks for four lanes at a time, so expose the shared result
/// without changing any lane's lexicographic coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleCombatGuideBundleV1 {
    pub progress: Vec<i32>,
    pub survival: Vec<i32>,
    pub horizon: Vec<i32>,
    pub turn_generation: Vec<i32>,
    pub setup: Vec<i32>,
}

#[derive(Clone, Debug, Serialize)]
pub struct OracleAtomicActionPriorityDiagnosticV1 {
    pub role: &'static str,
    pub role_rank: i32,
    pub mitigation: i32,
    pub action_supply: i32,
    pub reactive_risk: i32,
    pub target_progress: i32,
    pub block: i32,
    pub damage: i32,
    pub phase_setup: i32,
    pub phase_survival: i32,
    pub phase_transition_safety: i32,
    pub resource_timing: i32,
    pub policy_log2_bias: i32,
    pub direct_persistent_enemy_strength_down: i32,
    pub direct_temporary_enemy_strength_down: i32,
    pub direct_visible_attack_mitigation_hint: i32,
    pub direct_player_block: i32,
    pub reactive_player_block: i32,
    pub reactive_enemy_damage: i32,
    pub reactive_bad_draw_cards: i32,
}

/// Runs one bounded, non-authoritative tactical rollout and returns only its
/// ordering evidence. The simulated actions are deliberately not exposed:
/// callers must generate and replay their own exact witness.
pub fn oracle_combat_rollout_guide_v1(
    position: &CombatPosition,
    max_actions: usize,
    deadline: Option<Instant>,
) -> OracleCombatRolloutGuideV1 {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let config = super::CombatSearchV2Config::default();
    let mut performance = super::rollout_profile::RolloutPerformanceCounters::default();
    let estimate = super::rollout::phase_aware_no_potion_rollout(
        &node,
        &EngineCombatStepper,
        &config,
        max_actions.max(1),
        deadline,
        &mut performance,
    );
    let (outcome, evidence) = if estimate.terminal == super::SearchTerminalLabel::Win {
        // A simulated win is positive existence evidence. The caller still
        // has to generate and replay its own exact witness.
        (2, 2)
    } else if estimate.evaluated {
        // One bounded policy rollout that loses or runs out of horizon is not
        // a refutation of the exact state. Keep it live and let the remaining
        // typed coordinates order this heuristic sample.
        (1, 1)
    } else {
        (1, 0)
    };
    let eval = super::value::combat_eval_from_rollout_estimate(&estimate);
    let survival = match eval.survival_bucket() {
        super::value::CombatEvalSurvivalBucket::DeadOrForcedLoss => 0,
        super::value::CombatEvalSurvivalBucket::LethalVisible => 1,
        super::value::CombatEvalSurvivalBucket::Critical => 2,
        super::value::CombatEvalSurvivalBucket::Stabilizing => 3,
        super::value::CombatEvalSurvivalBucket::Stable => 4,
    };
    let progress = match eval.progress_bucket() {
        super::value::CombatEvalProgressBucket::Regression => 0,
        super::value::CombatEvalProgressBucket::Stalled => 1,
        super::value::CombatEvalProgressBucket::AttritionFavored => 2,
        super::value::CombatEvalProgressBucket::RaceFavored => 3,
        super::value::CombatEvalProgressBucket::LethalNextTurnLikely => 4,
        super::value::CombatEvalProgressBucket::LethalNow => 5,
    };
    let phase_pressure = (estimate.special_enemy_phase_count
        + estimate.guardian_mode_shift_pending_count
        + estimate.lagavulin_waking_count
        + estimate.sentry_dazed_pressure_count
        + estimate.hexaghost_opening_pressure_count
        + usize::from(estimate.high_fanout_pending_choice)) as i32
        + estimate.gremlin_nob_anger_amount_total.max(0)
        + estimate.pending_choice_estimated_action_fanout as i32;
    OracleCombatRolloutGuideV1 {
        // Positive existence evidence leads. Non-winning bounded rollouts
        // remain live heuristic estimates rather than false refutations.
        // Remaining coordinates are intentionally typed and lexicographic
        // rather than collapsed into one score.
        components: vec![
            outcome,
            evidence,
            survival,
            progress,
            estimate.survival_margin,
            estimate
                .final_hp
                .saturating_add(estimate.persistent_run_value),
            estimate.final_hp,
            estimate.persistent_run_value,
            -estimate.phase_adjusted_enemy_effort,
            -phase_pressure,
            -((estimate.potions_used + estimate.potions_discarded) as i32),
            -(estimate.turns as i32),
            -(estimate.cards_played as i32),
        ],
        actions_simulated: estimate.actions_simulated,
    }
}

/// Converts the existing typed action-ordering knowledge into positive,
/// relative policy weights. The caller owns normalization and the uniform
/// exploration floor.
///
/// This function does not evaluate successor states and cannot remove a legal
/// action. An input the exact stepper cannot address receives neutral weight.
pub fn oracle_atomic_action_policy_weights(
    position: &CombatPosition,
    inputs: &[ClientInput],
) -> Vec<f64> {
    oracle_atomic_action_policy_weights_from_iter(position, inputs.iter())
}

/// Borrowing counterpart used by planner adapters whose choices already hold
/// references to exact inputs.  Keeping this separate from the owned-slice
/// compatibility API avoids cloning every `ClientInput` merely to rank it.
pub fn oracle_atomic_action_policy_weights_for_refs(
    position: &CombatPosition,
    inputs: &[&ClientInput],
) -> Vec<f64> {
    oracle_atomic_action_policy_weights_from_iter(position, inputs.iter().copied())
}

/// Fast counterpart for inputs already obtained from the exact legal-action
/// surface of `position`.
///
/// Unlike the compatibility APIs above, this does not ask the stepper to
/// rediscover and validate every action or build the legacy ordering summary.
/// It uses the same typed priority and comparison rules and returns the same
/// ordinal weights. Callers must not pass speculative inputs.
pub fn oracle_legal_atomic_action_policy_weights_for_refs(
    position: &CombatPosition,
    inputs: &[&ClientInput],
) -> Vec<f64> {
    oracle_legal_atomic_action_policy_weights_from_iter(position, inputs.iter().copied())
}

fn oracle_legal_atomic_action_policy_weights_from_iter<'a, I>(
    position: &CombatPosition,
    inputs: I,
) -> Vec<f64>
where
    I: Clone + ExactSizeIterator<Item = &'a ClientInput>,
{
    let plugins = oracle_action_ordering_plugins(position);
    let mut ranked = inputs
        .clone()
        .enumerate()
        .map(|(original_action_id, input)| {
            (
                original_action_id,
                super::action_priority::priority_for_input_with_plugins(
                    &position.engine,
                    &position.combat,
                    input,
                    plugins,
                ),
            )
        })
        .collect::<Vec<_>>();
    if super::action_ordering::action_ordering_enabled(&position.engine) {
        ranked.sort_by(|(left_id, left), (right_id, right)| {
            super::action_ordering::compare_action_ordering_priorities(left, None, right, None)
                .then_with(|| left_id.cmp(right_id))
        });
    }
    let mut rank_by_input = vec![None; inputs.len()];
    for (rank, (original_action_id, priority)) in ranked.into_iter().enumerate() {
        rank_by_input[original_action_id] = Some((rank, priority.policy_log2_bias));
    }
    rank_by_input
        .into_iter()
        .zip(inputs)
        .map(|(rank, input)| {
            if matches!(input, ClientInput::UsePotion { .. })
                && !super::potions::semantic_potion_action_allowed(&position.combat, input)
            {
                return 1.0e-6;
            }
            rank.map_or(1.0, |(rank, policy_log2_bias)| {
                oracle_rank_weight_with_semantic_bias(rank, policy_log2_bias)
            })
        })
        .collect()
}

fn oracle_atomic_action_policy_weights_from_iter<'a, I>(
    position: &CombatPosition,
    inputs: I,
) -> Vec<f64>
where
    I: Clone + ExactSizeIterator<Item = &'a ClientInput>,
{
    let stepper = EngineCombatStepper;
    let choices = inputs
        .clone()
        .enumerate()
        .filter_map(|(original_action_id, input)| {
            stepper
                .choice_for_legal_input(position, input)
                .map(|choice| IndexedActionChoice {
                    original_action_id,
                    choice,
                })
        })
        .collect::<Vec<_>>();
    let mut rank_by_input = vec![None; inputs.len()];
    for (rank, choice) in order_indexed_action_choices_with_plugins(
        &position.engine,
        &position.combat,
        choices,
        oracle_action_ordering_plugins(position),
    )
    .choices
    .into_iter()
    .enumerate()
    {
        rank_by_input[choice.original_action_id] = Some(rank);
    }
    rank_by_input
        .into_iter()
        .zip(inputs)
        // The source is an ordinal action ordering, not a calibrated action
        // probability.  Reciprocal rank keeps that ordering useful while
        // preventing two locally non-greedy actions from acquiring an
        // exponential path penalty before their turn-boundary successor can
        // be evaluated.
        .map(|(rank, input)| {
            if matches!(input, ClientInput::UsePotion { .. })
                && !super::potions::semantic_potion_action_allowed(&position.combat, input)
            {
                return 1.0e-6;
            }
            rank.map_or(1.0, |rank| {
                let priority = super::action_priority::priority_for_input_with_plugins(
                    &position.engine,
                    &position.combat,
                    input,
                    oracle_action_ordering_plugins(position),
                );
                oracle_rank_weight_with_semantic_bias(rank, priority.policy_log2_bias)
            })
        })
        .collect()
}

/// Exposes the typed ordering coordinates used by the oracle action policy.
/// This is diagnostic provenance only: it neither changes the ordering nor
/// turns a heuristic preference into outcome evidence.
pub fn oracle_atomic_action_policy_priority_diagnostics_v1(
    position: &CombatPosition,
    inputs: &[ClientInput],
) -> Vec<Option<OracleAtomicActionPriorityDiagnosticV1>> {
    let stepper = EngineCombatStepper;
    inputs
        .iter()
        .map(|input| {
            stepper.choice_for_legal_input(position, input)?;
            let priority = super::action_priority::priority_for_input_with_plugins(
                &position.engine,
                &position.combat,
                input,
                oracle_action_ordering_plugins(position),
            );
            Some(OracleAtomicActionPriorityDiagnosticV1 {
                role: priority.role.label(),
                role_rank: priority.role_rank,
                mitigation: priority.mitigation,
                action_supply: priority.action_supply,
                reactive_risk: priority.reactive_risk,
                target_progress: priority.target_progress,
                block: priority.block,
                damage: priority.damage,
                phase_setup: priority.phase_setup,
                phase_survival: priority.phase_survival,
                phase_transition_safety: priority.phase_transition_safety,
                resource_timing: priority.resource_timing,
                policy_log2_bias: priority.policy_log2_bias,
                direct_persistent_enemy_strength_down: priority
                    .effects
                    .direct
                    .persistent_enemy_strength_down,
                direct_temporary_enemy_strength_down: priority
                    .effects
                    .direct
                    .temporary_enemy_strength_down,
                direct_visible_attack_mitigation_hint: priority
                    .effects
                    .direct
                    .visible_attack_mitigation_hint,
                direct_player_block: priority.effects.direct.player_block,
                reactive_player_block: priority.effects.reactive.player_block,
                reactive_enemy_damage: priority.effects.reactive.enemy_damage,
                reactive_bad_draw_cards: priority.effects.reactive.bad_draw_cards,
            })
        })
        .collect()
}

fn oracle_action_ordering_plugins(
    position: &CombatPosition,
) -> super::CombatSearchActionOrderingPlugins<'static> {
    use crate::content::monsters::EnemyId;

    let awakened_one_curiosity = position.combat.entities.monsters.iter().any(|monster| {
        monster.is_alive_for_action()
            && EnemyId::from_id(monster.monster_type) == Some(EnemyId::AwakenedOne)
            && store::has_power(&position.combat, monster.id, PowerId::Curiosity)
    });
    let phase_guard = if position.combat.entities.monsters.iter().any(|monster| {
        monster.is_alive_for_action()
            && EnemyId::from_id(monster.monster_type) == Some(EnemyId::TimeEater)
    }) {
        super::CombatSearchPhaseGuardPluginId::TimeEaterClockHint
    } else if position.combat.entities.monsters.iter().any(|monster| {
        monster.is_alive_for_action()
            && EnemyId::from_id(monster.monster_type) == Some(EnemyId::Champ)
    }) {
        super::CombatSearchPhaseGuardPluginId::ChampSplitGuard
    } else {
        super::CombatSearchPhaseGuardPluginId::Default
    };
    super::CombatSearchActionOrderingPlugins {
        phase_guard,
        // Curiosity is a real cost, but the ordinary phase rule assigns it
        // equally to every Power.  Preserve the existing semantic distinction
        // between a key engine/scaling Power and a low-impact Power so the
        // oracle policy does not bury every viable long-fight setup behind
        // ordinary nonlethal attacks.  The Curiosity penalty still applies,
        // and lethal/survival roles remain ahead of this prior.
        action_prior: if awakened_one_curiosity {
            super::CombatSearchActionPriorPluginId::KeyCardOnline
        } else {
            super::CombatSearchActionPriorPluginId::Default
        },
        ..super::CombatSearchActionOrderingPlugins::default()
    }
}

fn oracle_combat_state_value(position: &CombatPosition) -> super::value::CombatSearchStateValueV1 {
    combat_search_state_value_for_state(&position.engine, &position.combat)
}

fn combat_state_guide_components(
    position: &CombatPosition,
    value: &super::value::CombatSearchStateValueV1,
) -> Vec<i32> {
    vec![
        value.fewer_living_enemies,
        encounter_priority_owner_progress(&position.combat),
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        value.split_debt_hp,
        value.guardian_defensive_block,
        value.guardian_mode_shift_pending,
        value.lagavulin_waking_pressure,
        value.gremlin_nob_enrage_pressure,
        value.sentry_dazed_pressure,
        value.hexaghost_opening_pressure,
        value.high_fanout_pending_choice,
        value.pending_choice_estimated_action_fanout,
        value.survival_margin,
        value.sustained_mitigation,
        value.player_hp,
        value.player_block,
        value.hand_damage,
        value.hand_block,
        value.hand_playable_cards,
        value.hand_low_cost,
        value.next_draw_damage,
        value.next_draw_block,
        value.next_draw_playable_cards,
        value.next_draw_low_cost,
    ]
}

fn combat_survival_guide_components(
    position: &CombatPosition,
    value: &super::value::CombatSearchStateValueV1,
) -> Vec<i32> {
    vec![
        value.survival_margin,
        value.player_hp,
        value.fewer_living_enemies,
        encounter_priority_owner_progress(&position.combat),
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        value.sustained_mitigation,
        value.player_block,
        value.guardian_defensive_block,
        value.guardian_mode_shift_pending,
        value.split_debt_hp,
        value.hand_block,
        value.next_draw_block,
        value.hand_damage,
        value.next_draw_damage,
    ]
}

fn combat_horizon_guide_components(
    position: &CombatPosition,
    value: &super::value::CombatSearchStateValueV1,
) -> Vec<i32> {
    vec![
        i32::try_from(position.combat.turn.turn_count).unwrap_or(i32::MAX),
        value.fewer_living_enemies,
        encounter_priority_owner_progress(&position.combat),
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        value.survival_margin,
        value.player_hp,
    ]
}

fn combat_turn_generation_guide_components(
    position: &CombatPosition,
    value: &super::value::CombatSearchStateValueV1,
    setup: PlayerSetupSummary,
) -> Vec<i32> {
    vec![
        i32::from(position.combat.turn.counters.cards_played_this_turn),
        setup.exhaust_engine_connected,
        setup.status_access_engine_connected,
        setup.active_power_count,
        setup.active_power_mass,
        value.sustained_mitigation,
        value.fewer_living_enemies,
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        i32::from(position.combat.turn.energy),
        value.hand_playable_cards,
        value.player_hp,
        value.player_block,
    ]
}

fn combat_setup_guide_components(
    position: &CombatPosition,
    value: &super::value::CombatSearchStateValueV1,
    setup: PlayerSetupSummary,
) -> Vec<i32> {
    vec![
        setup.exhaust_engine_connected,
        setup.status_access_engine_connected,
        setup.exhaust_payoff_engine_count,
        setup.recurring_output_count,
        setup.recurring_output_mass,
        value.fewer_living_enemies,
        encounter_priority_owner_progress(&position.combat),
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        setup.exhaust_engine_fuel,
        setup.exhaust_payoff_engine_fuel,
        value.player_hp,
        value.survival_margin,
        setup.active_power_count,
        setup.active_power_mass,
        value.hand_block,
        value.hand_damage,
        i32::try_from(position.combat.turn.turn_count).unwrap_or(i32::MAX),
    ]
}

/// Computes every independent guide lane from one shared state evaluation.
pub fn oracle_combat_guide_bundle_v1(position: &CombatPosition) -> OracleCombatGuideBundleV1 {
    let value = oracle_combat_state_value(position);
    let setup = player_setup_summary(&position.combat);
    OracleCombatGuideBundleV1 {
        progress: combat_state_guide_components(position, &value),
        survival: combat_survival_guide_components(position, &value),
        horizon: combat_horizon_guide_components(position, &value),
        turn_generation: combat_turn_generation_guide_components(position, &value, setup),
        setup: combat_setup_guide_components(position, &value, setup),
    }
}

/// Reuses the mature search's typed, lexicographic state knowledge without
/// transferring ownership of its frontier or terminal claims. Components are
/// ordered exactly as `CombatSearchStateValueV1::cmp` orders them.
pub fn oracle_combat_state_guide_components(position: &CombatPosition) -> Vec<i32> {
    let value = oracle_combat_state_value(position);
    combat_state_guide_components(position, &value)
}

/// A separate non-authoritative view of the same typed state knowledge.
/// Keeping survival independent from progress lets multi-heuristic search
/// retain healthy setup lines without inventing a conversion rate between
/// enemy progress and player HP.
pub fn oracle_combat_survival_guide_components(position: &CombatPosition) -> Vec<i32> {
    let value = oracle_combat_state_value(position);
    combat_survival_guide_components(position, &value)
}

/// A non-authoritative long-horizon view for setup-heavy combats. Progress
/// and survival guides can both prefer an earlier turn forever: the former
/// because setup has not dealt damage yet, and the latter because later turns
/// have usually paid some HP. Keeping horizon in its own queue gives those
/// states service without calibrating turn depth against HP or enemy damage.
pub fn oracle_combat_horizon_guide_components(position: &CombatPosition) -> Vec<i32> {
    let value = oracle_combat_state_value(position);
    combat_horizon_guide_components(position, &value)
}

/// A horizon view specifically for partial states inside one player turn.
/// The ordinary horizon guide starts with `turn_count`, which is constant
/// until EndTurn and therefore cannot help a lazy complete-turn generator
/// expose longer setup sequences.  This view rewards realized action depth
/// first, then persistent assets and concrete combat progress.  It owns only
/// one guide lane; the anchor, progress, survival, and setup lanes remain
/// independent.
pub fn oracle_combat_turn_generation_guide_components(position: &CombatPosition) -> Vec<i32> {
    let value = oracle_combat_state_value(position);
    combat_turn_generation_guide_components(
        position,
        &value,
        player_setup_summary(&position.combat),
    )
}

/// An independent view of persistent player setup. Damage-first and
/// survival-first guides both undervalue a turn which spends energy putting
/// powers in play: the enemy is still healthy and the immediate block may
/// already have expired by the next player boundary. This lane recognizes
/// the resulting exact state, rather than assigning bonuses to the actions
/// which happened to create it.
pub fn oracle_combat_setup_guide_components(position: &CombatPosition) -> Vec<i32> {
    let value = oracle_combat_state_value(position);
    combat_setup_guide_components(position, &value, player_setup_summary(&position.combat))
}

/// Progress against an encounter member whose death removes a persistent
/// team-wide growth source. Total enemy HP alone treats symmetric-looking
/// targets as interchangeable, even when their future combat semantics are
/// not. Donu is the concrete owner in the Donu/Deca encounter: its alternating
/// buff grants Strength to every living monster, while Deca's death does not
/// stop that clock.
///
/// This is deliberately a guide coordinate rather than a forced target rule.
/// Exact search may still prefer Deca when Dazed pressure or a lethal window
/// makes that line better.
fn encounter_priority_owner_progress(combat: &CombatState) -> i32 {
    use crate::content::monsters::EnemyId;

    let has_deca = combat
        .entities
        .monsters
        .iter()
        .any(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::Deca));
    if !has_deca {
        return 0;
    }
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::Donu))
        .map(|donu| donu.max_hp.saturating_sub(donu.current_hp.max(0)).max(0))
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct PlayerSetupSummary {
    exhaust_engine_connected: i32,
    status_access_engine_connected: i32,
    exhaust_engine_fuel: i32,
    exhaust_payoff_engine_count: i32,
    exhaust_payoff_engine_fuel: i32,
    recurring_output_count: i32,
    recurring_output_mass: i32,
    active_power_count: i32,
    active_power_mass: i32,
}

fn player_setup_summary(combat: &CombatState) -> PlayerSetupSummary {
    let player = combat.entities.player.id;
    let (active_power_count, active_power_mass, recurring_output_count, recurring_output_mass) =
        store::powers_for(combat, player)
            .into_iter()
            .flatten()
            .filter_map(|power| {
                realized_player_setup_power_amount(combat, player, power)
                    .map(|amount| (power.power_type, amount))
            })
            .fold(
                (0_i32, 0_i32, 0_i32, 0_i32),
                |(active_count, active_mass, recurring_count, recurring_mass), (power, amount)| {
                    let recurring =
                        i32::from(player_power_has_deterministic_recurring_output(power));
                    (
                        active_count.saturating_add(1),
                        active_mass.saturating_add(amount),
                        recurring_count.saturating_add(recurring),
                        recurring_mass.saturating_add(amount.saturating_mul(recurring)),
                    )
                },
            );
    let unexhausted_cards = combat
        .zones
        .hand
        .iter()
        .chain(&combat.zones.draw_pile)
        .chain(&combat.zones.discard_pile)
        .collect::<Vec<_>>();
    let (remaining_skills, remaining_statuses) =
        unexhausted_cards
            .iter()
            .fold(
                (0_i32, 0_i32),
                |(skills, statuses), card| match get_card_definition(card.id).card_type {
                    CardType::Skill => (skills.saturating_add(1), statuses),
                    CardType::Status => (skills, statuses.saturating_add(1)),
                    _ => (skills, statuses),
                },
            );
    let exhaust_engine_connected = i32::from(
        store::has_power(combat, player, PowerId::Corruption)
            && store::has_power(combat, player, PowerId::DarkEmbrace),
    );
    let status_access_engine_connected =
        i32::from(remaining_statuses > 0 && store::has_power(combat, player, PowerId::Evolve));
    let corruption_active = store::has_power(combat, player, PowerId::Corruption);
    let remaining_exhaust_event_sources = unexhausted_cards
        .iter()
        .filter(|card| {
            corruption_active && get_card_definition(card.id).card_type == CardType::Skill
                || card_can_emit_exhaust_event(card)
        })
        .count()
        .try_into()
        .unwrap_or(i32::MAX);
    let exhaust_payoff_engine_count = if remaining_exhaust_event_sources > 0 {
        i32::from(store::has_power(combat, player, PowerId::FeelNoPain)).saturating_add(i32::from(
            store::has_power(combat, player, PowerId::DarkEmbrace),
        ))
    } else {
        0
    };
    PlayerSetupSummary {
        exhaust_engine_connected,
        status_access_engine_connected,
        exhaust_engine_fuel: remaining_skills.saturating_mul(exhaust_engine_connected),
        exhaust_payoff_engine_count,
        exhaust_payoff_engine_fuel: remaining_exhaust_event_sources
            .saturating_mul(exhaust_payoff_engine_count),
        recurring_output_count,
        recurring_output_mass,
        active_power_count,
        active_power_mass,
    }
}

fn card_can_emit_exhaust_event(card: &CombatCard) -> bool {
    if exhausts_when_played(card) {
        return true;
    }
    strategic_card_definition(card.id, card.upgrades)
        .play_effects
        .iter()
        .any(|effect| {
            matches!(
                effect,
                PlayEffect::EmitEvent(CombatEvent::CardExhausted)
                    | PlayEffect::PlayTopCardAndExhaust
            )
        })
}

/// Powers in this set have already paid their setup cost and deterministically
/// produce a useful resource or combat effect on later turns.  Conditional
/// engines (Rupture, Juggernaut, Fire Breathing), one-shot effects, and
/// recurring effects with an HP cost (Combust, Brutality) deliberately stay
/// out: merely having those icons in play does not prove future value.
///
/// This is a state semantic used by one independent setup guide, not a card
/// play bonus.  Other lanes continue to own immediate survival and progress.
fn player_power_has_deterministic_recurring_output(power: PowerId) -> bool {
    matches!(
        power,
        PowerId::Ritual
            | PowerId::DemonForm
            | PowerId::Metallicize
            | PowerId::BattleHymnPower
            | PowerId::DevotionPower
            | PowerId::DevaForm
            | PowerId::OmegaPower
            | PowerId::NoxiousFumes
            | PowerId::InfiniteBladesPower
            | PowerId::ToolsOfTheTrade
    )
}

fn realized_player_setup_power_amount(
    combat: &CombatState,
    player: crate::EntityId,
    power: &crate::runtime::combat::Power,
) -> Option<i32> {
    if !player_power_is_positive_setup(power.power_type, power.amount) {
        return None;
    }
    let amount = if crate::content::powers::uses_sentinel_amount(power.power_type) {
        1
    } else {
        power.amount.clamp(1, 12)
    };
    let scheduled_rollback = match power.power_type {
        PowerId::Strength => store::power_amount(combat, player, PowerId::LoseStrength),
        PowerId::Dexterity => store::power_amount(combat, player, PowerId::DexterityDown),
        _ => 0,
    }
    .max(0);
    let realized = amount.saturating_sub(scheduled_rollback);
    (realized > 0).then_some(realized)
}

fn player_power_is_positive_setup(power: PowerId, amount: i32) -> bool {
    if amount <= 0 && !crate::content::powers::uses_sentinel_amount(power) {
        return false;
    }
    !matches!(
        power,
        PowerId::Vulnerable
            | PowerId::Weak
            | PowerId::Frail
            | PowerId::LoseStrength
            | PowerId::Entangle
            | PowerId::Hex
            | PowerId::NoDraw
            | PowerId::NoBlock
            | PowerId::Confusion
            | PowerId::Constricted
            | PowerId::Shackled
            | PowerId::DrawReduction
            | PowerId::Surrounded
            | PowerId::BackAttack
            | PowerId::EnergyDownPower
            | PowerId::DexterityDown
            | PowerId::CannotChangeStance
    )
}

fn oracle_ordinal_rank_weight(rank: usize) -> f64 {
    1.0 / rank.saturating_add(1) as f64
}

fn oracle_rank_weight_with_semantic_bias(rank: usize, policy_log2_bias: i32) -> f64 {
    const MAX_ABS_POLICY_LOG2_BIAS: i32 = 20;
    oracle_ordinal_rank_weight(rank)
        * 2.0_f64.powi(policy_log2_bias.clamp(-MAX_ABS_POLICY_LOG2_BIAS, MAX_ABS_POLICY_LOG2_BIAS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::state::core::EngineState;

    #[test]
    fn policy_weights_preserve_every_legal_input() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 11)];
        combat.turn.energy = 1;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = EngineCombatStepper.atomic_actions(&position);

        let weights = oracle_atomic_action_policy_weights(&position, &inputs);

        assert_eq!(weights.len(), inputs.len());
        assert!(weights
            .iter()
            .all(|weight| weight.is_finite() && *weight > 0.0));
    }

    #[test]
    fn trusted_legal_policy_path_matches_validated_ordering() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Defend, 12),
        ];
        combat.turn.energy = 1;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = EngineCombatStepper.atomic_actions(&position);
        let input_refs = inputs.iter().collect::<Vec<_>>();

        assert_eq!(
            oracle_legal_atomic_action_policy_weights_for_refs(&position, &input_refs),
            oracle_atomic_action_policy_weights(&position, &inputs)
        );
    }

    #[test]
    fn trusted_legal_policy_path_matches_validated_semantic_bias() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 80;
        let mut attacker = crate::test_support::planned_monster(EnemyId::TimeEater, 2);
        attacker.id = 2;
        combat.entities.monsters = vec![attacker];
        combat.zones.hand = vec![
            CombatCard::new(CardId::BurningPact, 11),
            CombatCard::new(CardId::Defend, 12),
        ];
        combat.turn.energy = 3;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = EngineCombatStepper.atomic_actions(&position);
        let input_refs = inputs.iter().collect::<Vec<_>>();

        let validated = oracle_atomic_action_policy_weights(&position, &inputs);
        let trusted = oracle_legal_atomic_action_policy_weights_for_refs(&position, &input_refs);
        assert_eq!(trusted, validated);

        let burning_pact_index = inputs
            .iter()
            .position(|input| {
                matches!(
                    input,
                    ClientInput::PlayCard {
                        card_index: 0,
                        target: None
                    }
                )
            })
            .expect("Burning Pact should be a legal atomic action");
        let diagnostics = oracle_atomic_action_policy_priority_diagnostics_v1(&position, &inputs);
        assert_eq!(
            diagnostics[burning_pact_index]
                .as_ref()
                .expect("Burning Pact should have priority diagnostics")
                .policy_log2_bias,
            6
        );
        assert!(trusted[burning_pact_index] > 1.0);
    }

    #[test]
    fn connected_second_wind_wound_bias_reaches_both_policy_paths() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Wound, 10),
            CombatCard::new(CardId::Cleave, 20),
        ];
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::SecondWind, 30),
            CombatCard::new(CardId::PowerThrough, 40),
        ]
        .into();
        combat.entities.power_db.insert(
            combat.entities.player.id,
            vec![Power {
                power_type: PowerId::DarkEmbrace,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        let position = CombatPosition::new(
            EngineState::PendingChoice(crate::state::core::PendingChoice::HandSelect {
                candidate_uuids: vec![10, 20],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Exhaust,
            }),
            combat,
        );
        let inputs = vec![
            ClientInput::SubmitSelection(crate::state::selection::SelectionResolution::card_uuids(
                crate::state::selection::SelectionScope::Hand,
                [10],
            )),
            ClientInput::SubmitSelection(crate::state::selection::SelectionResolution::card_uuids(
                crate::state::selection::SelectionScope::Hand,
                [20],
            )),
        ];
        let input_refs = inputs.iter().collect::<Vec<_>>();

        let validated = oracle_atomic_action_policy_weights(&position, &inputs);
        let trusted = oracle_legal_atomic_action_policy_weights_for_refs(&position, &input_refs);
        assert_eq!(trusted, validated);
        assert!(
            trusted[1] > trusted[0] * 1_000.0,
            "Wound should be suppressed by a semantic scale: {trusted:?}"
        );
        let diagnostics = oracle_atomic_action_policy_priority_diagnostics_v1(&position, &inputs);
        assert_eq!(
            diagnostics[0]
                .as_ref()
                .expect("Wound choice should have priority diagnostics")
                .policy_log2_bias,
            -10
        );
    }

    #[test]
    fn oracle_policy_uses_time_eater_haste_window_knowledge_automatically() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut eater = crate::test_support::test_monster(EnemyId::TimeEater);
        eater.id = 1;
        eater.current_hp = 200;
        eater.max_hp = 456;
        eater.time_eater.used_haste = true;
        eater.set_planned_move_id(5);
        combat.entities.monsters = vec![eater];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Cleave, 11),
            CombatCard::new(CardId::Defend, 12),
        ];
        combat.turn.energy = 3;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
        ];

        let weights = oracle_atomic_action_policy_weights(&position, &inputs);

        assert!(
            weights[1] > weights[0],
            "setup/mitigation must outrank nonlethal damage that Haste will erase"
        );
    }

    #[test]
    fn awakened_policy_preserves_key_setup_without_promoting_every_power() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut awakened = crate::test_support::test_monster(EnemyId::AwakenedOne);
        awakened.id = 1;
        awakened.current_hp = 300;
        awakened.max_hp = 300;
        combat.entities.monsters = vec![awakened];
        combat
            .entities
            .power_db
            .insert(1, vec![test_power_amount(PowerId::Curiosity, 1)]);
        combat.zones.hand = vec![
            CombatCard::new(CardId::DemonForm, 11),
            CombatCard::new(CardId::Evolve, 12),
            CombatCard::new(CardId::Strike, 13),
        ];
        combat.turn.energy = 6;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 2,
                target: Some(1),
            },
        ];

        let weights = oracle_atomic_action_policy_weights(&position, &inputs);

        assert!(
            weights[0] > weights[2],
            "Demon Form must retain long-fight setup relevance under Curiosity"
        );
        assert!(
            weights[2] > weights[1],
            "a low-impact Power must still pay the ordinary Curiosity penalty"
        );
    }

    #[test]
    fn awakened_key_setup_prior_does_not_override_immediate_lethal() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut awakened = crate::test_support::test_monster(EnemyId::AwakenedOne);
        awakened.id = 1;
        awakened.current_hp = 6;
        awakened.max_hp = 300;
        combat.entities.monsters = vec![awakened];
        combat
            .entities
            .power_db
            .insert(1, vec![test_power_amount(PowerId::Curiosity, 1)]);
        combat.zones.hand = vec![
            CombatCard::new(CardId::DemonForm, 11),
            CombatCard::new(CardId::Strike, 12),
        ];
        combat.turn.energy = 6;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ];

        let weights = oracle_atomic_action_policy_weights(&position, &inputs);

        assert!(weights[1] > weights[0]);
    }

    #[test]
    fn disarm_policy_prefers_a_target_without_artifact() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut blocked = crate::test_support::test_monster(EnemyId::Cultist);
        blocked.id = 1;
        let mut exposed = crate::test_support::test_monster(EnemyId::Cultist);
        exposed.id = 2;
        combat.entities.monsters = vec![blocked, exposed];
        combat
            .entities
            .power_db
            .insert(1, vec![test_power_amount(PowerId::Artifact, 1)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Disarm, 11)];
        combat.turn.energy = 1;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
        ];

        let weights = oracle_atomic_action_policy_weights(&position, &inputs);

        assert!(weights[1] > weights[0]);
    }

    #[test]
    fn dexterity_does_not_turn_artifact_blocked_disarm_into_a_block_card() {
        use crate::runtime::monster_move::{AttackSpec, DamageKind, MonsterMoveSpec};

        let mut combat = crate::test_support::blank_test_combat();
        let mut attacker = crate::test_support::test_monster(EnemyId::Deca);
        attacker.id = 1;
        let attack = MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 16,
            hits: 2,
            damage_kind: DamageKind::Normal,
        });
        attacker.set_planned_steps(attack.to_steps());
        attacker.set_planned_visible_spec(Some(attack));
        combat.entities.monsters = vec![attacker];
        combat
            .entities
            .power_db
            .insert(1, vec![test_power_amount(PowerId::Artifact, 1)]);
        let player = combat.entities.player.id;
        combat.entities.power_db.insert(
            player,
            vec![
                test_power_amount(PowerId::FeelNoPain, 4),
                test_power_amount(PowerId::Dexterity, 6),
            ],
        );
        let mut bash = CombatCard::new(CardId::Bash, 10);
        bash.upgrades = 1;
        combat.zones.hand = vec![bash, CombatCard::new(CardId::Disarm, 11)];
        combat.turn.energy = 3;
        let inputs = vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ];

        let exposed = CombatPosition::new(EngineState::CombatPlayerTurn, combat.clone());
        let exposed_weights = oracle_atomic_action_policy_weights(&exposed, &inputs);
        assert!(
            exposed_weights[0] > exposed_weights[1],
            "four incidental block must not outrank Bash while most damage remains"
        );

        assert!(
            exposed_weights[1].is_finite() && exposed_weights[1] > 0.0,
            "the blocked Disarm remains a legal searchable action"
        );
    }

    #[test]
    fn donu_damage_outranks_equal_deca_damage_without_forcing_a_target() {
        let mut base = crate::test_support::blank_test_combat();
        let mut deca = crate::test_support::test_monster(EnemyId::Deca);
        deca.id = 1;
        deca.current_hp = 250;
        deca.max_hp = 250;
        let mut donu = crate::test_support::test_monster(EnemyId::Donu);
        donu.id = 2;
        donu.current_hp = 250;
        donu.max_hp = 250;
        base.entities.monsters = vec![deca, donu];

        let mut damaged_deca = base.clone();
        damaged_deca.entities.monsters[0].current_hp = 230;
        let mut damaged_donu = base;
        damaged_donu.entities.monsters[1].current_hp = 230;

        let deca_rank = oracle_combat_state_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            damaged_deca,
        ));
        let donu_rank = oracle_combat_state_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            damaged_donu,
        ));

        assert!(donu_rank > deca_rank);
    }

    #[test]
    fn ordinal_rank_guidance_is_weak_rather_than_exponential() {
        assert_eq!(oracle_ordinal_rank_weight(0), 1.0);
        assert_eq!(oracle_ordinal_rank_weight(1), 0.5);
        assert_eq!(oracle_ordinal_rank_weight(2), 1.0 / 3.0);
        assert_eq!(oracle_ordinal_rank_weight(15), 1.0 / 16.0);
    }

    #[test]
    fn explicit_semantic_bias_uses_bounded_base_two_scale() {
        assert_eq!(oracle_rank_weight_with_semantic_bias(0, 6), 64.0);
        assert_eq!(oracle_rank_weight_with_semantic_bias(0, -10), 1.0 / 1_024.0);
        assert_eq!(
            oracle_rank_weight_with_semantic_bias(0, 21),
            oracle_rank_weight_with_semantic_bias(0, 20)
        );
        assert_eq!(
            oracle_rank_weight_with_semantic_bias(0, -21),
            oracle_rank_weight_with_semantic_bias(0, -20)
        );
    }

    #[test]
    fn horizon_guide_exposes_turn_depth_as_its_primary_independent_rank() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
        combat.turn.turn_count = 7;
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let rank = oracle_combat_horizon_guide_components(&position);

        assert_eq!(rank.first(), Some(&7));
    }

    #[test]
    fn turn_generation_horizon_uses_realized_in_turn_depth() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
        let shallow = CombatPosition::new(EngineState::CombatPlayerTurn, combat.clone());
        combat.turn.counters.cards_played_this_turn = 3;
        let deep = CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let shallow_rank = oracle_combat_turn_generation_guide_components(&shallow);
        let deep_rank = oracle_combat_turn_generation_guide_components(&deep);

        assert_eq!(shallow_rank.first(), Some(&0));
        assert_eq!(deep_rank.first(), Some(&3));
        assert!(deep_rank > shallow_rank);
    }

    #[test]
    fn setup_summary_recognizes_connected_engines_and_remaining_fuel() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat.entities.power_db.insert(
            player,
            vec![
                test_power(PowerId::DarkEmbrace),
                test_power(PowerId::Corruption),
                test_power(PowerId::Evolve),
            ],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::ShrugItOff, 21),
            CombatCard::new(CardId::Wound, 22),
        ];
        combat.zones.draw_pile = (vec![CombatCard::new(CardId::TrueGrit, 23)]).into();

        let summary = player_setup_summary(&combat);

        assert_eq!(summary.exhaust_engine_connected, 1);
        assert_eq!(summary.status_access_engine_connected, 1);
        assert_eq!(summary.exhaust_engine_fuel, 2);
    }

    #[test]
    fn evolve_without_status_burden_is_not_a_connected_status_engine() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat
            .entities
            .power_db
            .insert(player, vec![test_power(PowerId::Evolve)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 31)];

        let summary = player_setup_summary(&combat);

        assert_eq!(summary.status_access_engine_connected, 0);
        assert_eq!(summary.exhaust_engine_connected, 0);
        assert_eq!(summary.exhaust_engine_fuel, 0);
    }

    #[test]
    fn setup_recognizes_realized_exhaust_payoff_only_with_remaining_sources() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat.entities.power_db.insert(
            player,
            vec![
                test_power_amount(PowerId::FeelNoPain, 4),
                test_power_amount(PowerId::DarkEmbrace, 1),
            ],
        );
        combat.zones.hand = vec![
            CombatCard::new(CardId::BurningPact, 41),
            CombatCard::new(CardId::Feed, 42),
        ];

        let connected = player_setup_summary(&combat);
        assert_eq!(connected.exhaust_payoff_engine_count, 2);
        assert_eq!(connected.exhaust_payoff_engine_fuel, 4);

        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 43)];
        let disconnected = player_setup_summary(&combat);
        assert_eq!(disconnected.exhaust_payoff_engine_count, 0);
        assert_eq!(disconnected.exhaust_payoff_engine_fuel, 0);
    }

    #[test]
    fn realized_exhaust_payoff_gets_an_independent_setup_view() {
        let mut safer = crate::test_support::blank_test_combat();
        safer.entities.monsters = vec![crate::test_support::test_monster(EnemyId::AwakenedOne)];
        safer.entities.player.current_hp = 72;
        safer.zones.hand = vec![CombatCard::new(CardId::BurningPact, 51)];

        let mut connected = safer.clone();
        connected.entities.player.current_hp = 54;
        let player = connected.entities.player.id;
        connected
            .entities
            .power_db
            .insert(player, vec![test_power_amount(PowerId::FeelNoPain, 4)]);

        let safer_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            safer,
        ));
        let connected_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            connected,
        ));

        assert!(connected_rank > safer_rank);
    }

    #[test]
    fn abundant_exhaust_fuel_does_not_override_realized_combat_progress() {
        let mut stockpiled = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.current_hp = 100;
        monster.max_hp = 100;
        stockpiled.entities.monsters = vec![monster];
        let player = stockpiled.entities.player.id;
        stockpiled
            .entities
            .power_db
            .insert(player, vec![test_power_amount(PowerId::FeelNoPain, 4)]);
        stockpiled.zones.draw_pile = ((0..12)
            .map(|index| CombatCard::new(CardId::Feed, 100 + index))
            .collect::<Vec<_>>())
        .into();

        let mut progressed = stockpiled.clone();
        progressed.zones.draw_pile.pop();
        progressed.entities.monsters[0].current_hp = 80;

        let stockpiled_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            stockpiled,
        ));
        let progressed_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            progressed,
        ));

        assert!(progressed_rank > stockpiled_rank);
    }

    #[test]
    fn setup_prefers_realized_recurring_output_over_an_unset_safer_state() {
        let mut safer = crate::test_support::blank_test_combat();
        safer.entities.monsters = vec![crate::test_support::test_monster(EnemyId::TimeEater)];
        safer.entities.player.current_hp = 52;

        let mut scaled = safer.clone();
        scaled.entities.player.current_hp = 31;
        let player = scaled.entities.player.id;
        scaled
            .entities
            .power_db
            .insert(player, vec![test_power_amount(PowerId::DemonForm, 3)]);

        let safer_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            safer,
        ));
        let scaled_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
            EngineState::CombatPlayerTurn,
            scaled,
        ));

        assert!(scaled_rank > safer_rank);
    }

    #[test]
    fn conditional_or_static_power_does_not_override_setup_survival() {
        let mut safer = crate::test_support::blank_test_combat();
        safer.entities.monsters = vec![crate::test_support::test_monster(EnemyId::TimeEater)];
        safer.entities.player.current_hp = 52;

        for power in [PowerId::Strength, PowerId::Rupture, PowerId::Combust] {
            let mut lower_hp = safer.clone();
            lower_hp.entities.player.current_hp = 31;
            let player = lower_hp.entities.player.id;
            lower_hp
                .entities
                .power_db
                .insert(player, vec![test_power_amount(power, 3)]);

            let safer_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
                EngineState::CombatPlayerTurn,
                safer.clone(),
            ));
            let lower_hp_rank = oracle_combat_setup_guide_components(&CombatPosition::new(
                EngineState::CombatPlayerTurn,
                lower_hp,
            ));

            assert!(
                safer_rank > lower_hp_rank,
                "{power:?} must not claim recurring output"
            );
        }
    }

    #[test]
    fn scheduled_strength_rollback_is_not_persistent_setup() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat.entities.power_db.insert(
            player,
            vec![
                test_power_amount(PowerId::Strength, 5),
                test_power_amount(PowerId::LoseStrength, 5),
            ],
        );

        let summary = player_setup_summary(&combat);

        assert_eq!(summary.active_power_count, 0);
        assert_eq!(summary.active_power_mass, 0);
    }

    #[test]
    fn setup_counts_only_strength_that_survives_a_scheduled_rollback() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat.entities.power_db.insert(
            player,
            vec![
                test_power_amount(PowerId::Strength, 9),
                test_power_amount(PowerId::LoseStrength, 5),
            ],
        );

        let summary = player_setup_summary(&combat);

        assert_eq!(summary.active_power_count, 1);
        assert_eq!(summary.active_power_mass, 4);
    }

    #[test]
    fn cleansed_strength_rollback_becomes_realized_setup() {
        let mut combat = crate::test_support::blank_test_combat();
        let player = combat.entities.player.id;
        combat
            .entities
            .power_db
            .insert(player, vec![test_power_amount(PowerId::Strength, 5)]);

        let summary = player_setup_summary(&combat);

        assert_eq!(summary.active_power_count, 1);
        assert_eq!(summary.active_power_mass, 5);
    }

    fn test_power(power_type: PowerId) -> Power {
        test_power_amount(power_type, -1)
    }

    fn test_power_amount(power_type: PowerId, amount: i32) -> Power {
        Power {
            power_type,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }
    }
}
use std::time::Instant;
