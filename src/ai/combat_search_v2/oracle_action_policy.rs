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

use super::action_ordering::{order_indexed_action_choices, IndexedActionChoice};
use super::frontier::SearchNode;
use super::value::combat_search_state_value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleCombatRolloutGuideV1 {
    pub components: Vec<i32>,
    pub actions_simulated: usize,
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
    let stepper = EngineCombatStepper;
    let choices = inputs
        .iter()
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
    for (rank, choice) in order_indexed_action_choices(&position.engine, &position.combat, choices)
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
            rank.map_or(1.0, oracle_ordinal_rank_weight)
        })
        .collect()
}

/// Reuses the mature search's typed, lexicographic state knowledge without
/// transferring ownership of its frontier or terminal claims. Components are
/// ordered exactly as `CombatSearchStateValueV1::cmp` orders them.
pub fn oracle_combat_state_guide_components(position: &CombatPosition) -> Vec<i32> {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let value = combat_search_state_value(&node);
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

/// A separate non-authoritative view of the same typed state knowledge.
/// Keeping survival independent from progress lets multi-heuristic search
/// retain healthy setup lines without inventing a conversion rate between
/// enemy progress and player HP.
pub fn oracle_combat_survival_guide_components(position: &CombatPosition) -> Vec<i32> {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let value = combat_search_state_value(&node);
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

/// A non-authoritative long-horizon view for setup-heavy combats. Progress
/// and survival guides can both prefer an earlier turn forever: the former
/// because setup has not dealt damage yet, and the latter because later turns
/// have usually paid some HP. Keeping horizon in its own queue gives those
/// states service without calibrating turn depth against HP or enemy damage.
pub fn oracle_combat_horizon_guide_components(position: &CombatPosition) -> Vec<i32> {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let value = combat_search_state_value(&node);
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

/// A horizon view specifically for partial states inside one player turn.
/// The ordinary horizon guide starts with `turn_count`, which is constant
/// until EndTurn and therefore cannot help a lazy complete-turn generator
/// expose longer setup sequences.  This view rewards realized action depth
/// first, then persistent assets and concrete combat progress.  It owns only
/// one guide lane; the anchor, progress, survival, and setup lanes remain
/// independent.
pub fn oracle_combat_turn_generation_guide_components(position: &CombatPosition) -> Vec<i32> {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let value = combat_search_state_value(&node);
    let setup = player_setup_summary(&position.combat);
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

/// An independent view of persistent player setup. Damage-first and
/// survival-first guides both undervalue a turn which spends energy putting
/// powers in play: the enemy is still healthy and the immediate block may
/// already have expired by the next player boundary. This lane recognizes
/// the resulting exact state, rather than assigning bonuses to the actions
/// which happened to create it.
pub fn oracle_combat_setup_guide_components(position: &CombatPosition) -> Vec<i32> {
    let node = SearchNode::root(position.engine.clone(), position.combat.clone());
    let value = combat_search_state_value(&node);
    let setup = player_setup_summary(&position.combat);
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
        combat.zones.draw_pile = vec![CombatCard::new(CardId::TrueGrit, 23)];

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
        stockpiled.zones.draw_pile = (0..12)
            .map(|index| CombatCard::new(CardId::Feed, 100 + index))
            .collect();

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
