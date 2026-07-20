use crate::sim::combat::{CombatPosition, CombatStepper, EngineCombatStepper};
use crate::state::core::ClientInput;
use crate::{content::powers::PowerId, runtime::combat::CombatState};

use super::action_ordering::{order_indexed_action_choices, IndexedActionChoice};
use super::frontier::SearchNode;
use super::value::combat_search_state_value;

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
        value.phase_adjusted_enemy_effort_progress,
        value.enemy_effort_progress,
        value.enemy_hp_progress,
        value.survival_margin,
        value.player_hp,
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
    let (active_setup_powers, setup_power_mass) = player_setup_power_summary(&position.combat);
    vec![
        active_setup_powers,
        setup_power_mass,
        value.hand_damage,
        value.hand_block,
        i32::try_from(position.combat.turn.turn_count).unwrap_or(i32::MAX),
        value.survival_margin,
        value.player_hp,
    ]
}

fn player_setup_power_summary(combat: &CombatState) -> (i32, i32) {
    crate::content::powers::store::powers_for(combat, combat.entities.player.id)
        .into_iter()
        .flatten()
        .filter(|power| player_power_is_positive_setup(power.power_type, power.amount))
        .fold((0_i32, 0_i32), |(count, mass), power| {
            let amount = if crate::content::powers::uses_sentinel_amount(power.power_type) {
                1
            } else {
                power.amount.clamp(1, 12)
            };
            (count.saturating_add(1), mass.saturating_add(amount))
        })
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
    use crate::runtime::combat::CombatCard;
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
}
