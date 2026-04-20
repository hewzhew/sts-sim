use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;
use std::time::Instant;

use super::legal_moves::get_legal_moves;
use super::posture::posture_features;
use super::pressure::StatePressureFeatures;
use super::profile::SearchProfileBreakdown;
use super::stepping::project_turn_close_state_bounded;
use super::terminal::{survives, terminal_outcome, TerminalKind, TerminalOutcome};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NonTerminalValue {
    pub(super) survives: bool,
    pub(super) projected_unblocked: i32,
    pub(super) future_position_score: i32,
    pub(super) projected_enemy_total: i32,
    pub(super) projected_hp: i32,
    pub(super) projected_block: i32,
    pub(super) player_buff_score: i32,
    pub(super) player_debuff_score: i32,
    pub(super) enemy_buff_score: i32,
    pub(super) enemy_debuff_score: i32,
    pub(super) next_non_endturn_options: i32,
    pub(super) setup_window_score: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CombatValue {
    Terminal(TerminalOutcome),
    NonTerminal(NonTerminalValue),
}

pub(super) fn project_turn_close_state(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> (EngineState, CombatState, bool) {
    let (projected_engine, projected_combat, outcome) =
        project_turn_close_state_bounded(engine, combat, max_engine_steps, deadline, profile);
    (
        projected_engine,
        projected_combat,
        outcome.truncated || outcome.timed_out,
    )
}

pub(super) fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

pub(super) fn projected_unblocked(combat: &CombatState) -> i32 {
    (StatePressureFeatures::from_combat(combat).value_incoming - combat.entities.player.block)
        .max(0)
}

pub(super) fn incoming_damage(combat: &CombatState) -> i32 {
    StatePressureFeatures::from_combat(combat).value_incoming
}

pub(super) fn non_terminal_value(
    survives: bool,
    projected_unblocked: i32,
    future_position_score: i32,
    projected_enemy_total: i32,
    projected_hp: i32,
    projected_block: i32,
    player_buff_score: i32,
    player_debuff_score: i32,
    enemy_buff_score: i32,
    enemy_debuff_score: i32,
    next_non_endturn_options: i32,
    setup_window_score: i32,
) -> NonTerminalValue {
    NonTerminalValue {
        survives,
        projected_unblocked,
        future_position_score,
        projected_enemy_total,
        projected_hp,
        projected_block,
        player_buff_score,
        player_debuff_score,
        enemy_buff_score,
        enemy_debuff_score,
        next_non_endturn_options,
        setup_window_score,
    }
}

pub(super) fn projected_frontier(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> (EngineState, CombatState, CombatValue, bool) {
    let (projected_engine, projected_combat, truncated) =
        project_turn_close_state(engine, combat, max_engine_steps, deadline, profile);
    let value = frontier_value_at_state(&projected_engine, &projected_combat);
    (projected_engine, projected_combat, value, truncated)
}

pub(super) fn frontier_value_at_state(engine: &EngineState, combat: &CombatState) -> CombatValue {
    let value = if let Some(outcome) = terminal_outcome(engine, combat) {
        CombatValue::Terminal(outcome)
    } else {
        CombatValue::NonTerminal(non_terminal_value(
            survives(
                super::terminal::terminal_kind(engine, combat),
                combat.entities.player.current_hp,
            ),
            projected_unblocked(combat),
            future_position_score(
                engine,
                combat,
                player_buff_score(combat),
                player_debuff_score(combat),
                enemy_buff_score(combat),
                enemy_debuff_score(combat),
                setup_window_score(combat),
            ),
            total_enemy_hp(combat),
            combat.entities.player.current_hp,
            combat.entities.player.block,
            player_buff_score(combat),
            player_debuff_score(combat),
            enemy_buff_score(combat),
            enemy_debuff_score(combat),
            next_non_endturn_options(engine, combat),
            setup_window_score(combat),
        ))
    };
    value
}

pub(super) fn compare_values(left: &CombatValue, right: &CombatValue) -> std::cmp::Ordering {
    value_bucket(right)
        .cmp(&value_bucket(left))
        .then_with(|| match (left, right) {
            (CombatValue::Terminal(left), CombatValue::Terminal(right)) => right
                .kind
                .cmp(&left.kind)
                .then_with(|| right.final_hp.cmp(&left.final_hp))
                .then_with(|| right.final_block.cmp(&left.final_block)),
            (CombatValue::NonTerminal(left), CombatValue::NonTerminal(right)) => right
                .survives
                .cmp(&left.survives)
                .then_with(|| left.projected_unblocked.cmp(&right.projected_unblocked))
                .then_with(|| right.future_position_score.cmp(&left.future_position_score))
                .then_with(|| left.projected_enemy_total.cmp(&right.projected_enemy_total))
                .then_with(|| right.projected_hp.cmp(&left.projected_hp))
                .then_with(|| right.player_buff_score.cmp(&left.player_buff_score))
                .then_with(|| right.enemy_debuff_score.cmp(&left.enemy_debuff_score))
                .then_with(|| left.player_debuff_score.cmp(&right.player_debuff_score))
                .then_with(|| left.enemy_buff_score.cmp(&right.enemy_buff_score))
                .then_with(|| {
                    right
                        .next_non_endturn_options
                        .cmp(&left.next_non_endturn_options)
                })
                .then_with(|| right.setup_window_score.cmp(&left.setup_window_score))
                .then_with(|| right.projected_block.cmp(&left.projected_block)),
            _ => std::cmp::Ordering::Equal,
        })
}

pub(super) fn diagnostic_score(value: CombatValue, input: &ClientInput) -> f32 {
    match value {
        CombatValue::Terminal(outcome) => {
            let mut score = outcome.final_hp as f32 * 0.1 + outcome.final_block as f32 * 0.05;
            score += match outcome.kind {
                TerminalKind::Defeat => -20.0,
                TerminalKind::Ongoing => 0.0,
                TerminalKind::CombatCleared => 20.0,
                TerminalKind::Victory => 25.0,
            };
            if !matches!(input, ClientInput::EndTurn) {
                score += 0.1;
            }
            score
        }
        CombatValue::NonTerminal(value) => {
            let mut score = value.projected_hp as f32 * 0.1 + value.projected_block as f32 * 0.05;
            score -= value.projected_unblocked as f32 * 2.0;
            score -= value.projected_enemy_total as f32 * 0.02;
            score += value.future_position_score as f32 * 0.18;
            score += value.player_buff_score as f32 * 0.14;
            score += value.enemy_debuff_score as f32 * 0.10;
            score -= value.player_debuff_score as f32 * 0.08;
            score -= value.enemy_buff_score as f32 * 0.06;
            score += value.next_non_endturn_options as f32 * 0.18;
            score += value.setup_window_score as f32 * 0.12;
            if value.survives {
                score += 10.0;
            }
            if !matches!(input, ClientInput::EndTurn) {
                score += 0.1;
            }
            score
        }
    }
}

fn value_bucket(value: &CombatValue) -> i32 {
    match value {
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Victory,
            ..
        }) => 3,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::CombatCleared,
            ..
        }) => 2,
        CombatValue::NonTerminal(_) => 1,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Defeat,
            ..
        }) => 0,
        CombatValue::Terminal(TerminalOutcome {
            kind: TerminalKind::Ongoing,
            ..
        }) => 1,
    }
}

fn next_non_endturn_options(engine: &EngineState, combat: &CombatState) -> i32 {
    get_legal_moves(engine, combat)
        .into_iter()
        .filter(|input| !matches!(input, ClientInput::EndTurn))
        .count() as i32
}

fn setup_window_score(combat: &CombatState) -> i32 {
    let posture = posture_features(combat);
    posture.setup_payoff_density + posture.expected_fight_length_bucket
        - posture.immediate_survival_pressure / 4
}

fn player_buff_score(combat: &CombatState) -> i32 {
    combat.entities.power_db.get(&0).map_or(0, |powers| {
        powers
            .iter()
            .filter(|power| !crate::content::powers::is_debuff(power.power_type, power.amount))
            .map(generic_power_magnitude)
            .sum()
    })
}

fn player_debuff_score(combat: &CombatState) -> i32 {
    combat.entities.power_db.get(&0).map_or(0, |powers| {
        powers
            .iter()
            .filter(|power| crate::content::powers::is_debuff(power.power_type, power.amount))
            .map(generic_power_magnitude)
            .sum()
    })
}

fn enemy_buff_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            combat
                .entities
                .power_db
                .get(&monster.id)
                .map_or(0, |powers| {
                    powers
                        .iter()
                        .filter(|power| {
                            !crate::content::powers::is_debuff(power.power_type, power.amount)
                        })
                        .map(generic_power_magnitude)
                        .sum()
                })
        })
        .sum()
}

fn enemy_debuff_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .map(|monster| {
            combat
                .entities
                .power_db
                .get(&monster.id)
                .map_or(0, |powers| {
                    powers
                        .iter()
                        .filter(|power| {
                            crate::content::powers::is_debuff(power.power_type, power.amount)
                        })
                        .map(generic_power_magnitude)
                        .sum()
                })
        })
        .sum()
}

fn generic_power_magnitude(power: &crate::runtime::combat::Power) -> i32 {
    power.amount.abs().clamp(1, 8)
}

fn future_position_score(
    engine: &EngineState,
    combat: &CombatState,
    player_buff_score: i32,
    player_debuff_score: i32,
    enemy_buff_score: i32,
    enemy_debuff_score: i32,
    setup_window_score: i32,
) -> i32 {
    player_buff_score * 4
        + enemy_debuff_score * 3
        + next_non_endturn_options(engine, combat) * 2
        + setup_window_score * 2
        - player_debuff_score * 2
        - enemy_buff_score * 2
}

#[cfg(test)]
mod tests {
    use super::{compare_values, projected_frontier, CombatValue};
    use crate::bot::combat::profile::SearchProfileBreakdown;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatCard, Power};
    use crate::state::core::ClientInput;
    use crate::state::EngineState;
    use crate::test_support::{blank_test_combat, planned_monster};

    #[test]
    fn projected_frontier_values_generic_player_buffs() {
        let mut baseline = blank_test_combat();
        baseline
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 3));

        let mut buffed = baseline.clone();
        buffed.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Strength,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let mut baseline_profile = SearchProfileBreakdown::default();
        let mut buffed_profile = SearchProfileBreakdown::default();
        let baseline_value = projected_frontier(
            &EngineState::CombatPlayerTurn,
            &baseline,
            80,
            None,
            &mut baseline_profile,
        )
        .2;
        let buffed_value = projected_frontier(
            &EngineState::CombatPlayerTurn,
            &buffed,
            80,
            None,
            &mut buffed_profile,
        )
        .2;

        assert!(
            compare_values(&baseline_value, &buffed_value).is_gt(),
            "generic player buffs should improve frontier ordering"
        );
    }

    #[test]
    fn projected_frontier_rewards_next_turn_options() {
        let mut empty_hand = blank_test_combat();
        empty_hand
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 3));

        let mut live_hand = empty_hand.clone();
        live_hand
            .zones
            .hand
            .push(CombatCard::new(CardId::Strike, 11));

        let mut empty_profile = SearchProfileBreakdown::default();
        let mut live_profile = SearchProfileBreakdown::default();
        let empty_value = projected_frontier(
            &EngineState::CombatPlayerTurn,
            &empty_hand,
            80,
            None,
            &mut empty_profile,
        )
        .2;
        let live_value = projected_frontier(
            &EngineState::CombatPlayerTurn,
            &live_hand,
            80,
            None,
            &mut live_profile,
        )
        .2;

        assert!(
            compare_values(&empty_value, &live_value).is_gt(),
            "states with an available next action should outrank empty frontiers"
        );
        assert!(
            matches!(live_value, CombatValue::NonTerminal(_)),
            "test setup should remain non-terminal"
        );
    }

    #[test]
    fn diagnostic_score_rewards_non_endturn_setup_windows() {
        let value = CombatValue::NonTerminal(super::non_terminal_value(
            true, 0, 18, 30, 80, 0, 2, 0, 0, 0, 3, 4,
        ));

        assert!(
            super::diagnostic_score(
                value,
                &ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                }
            ) > super::diagnostic_score(value, &ClientInput::EndTurn),
            "non-endturn actions should still get a slight preference in identical states"
        );
    }
}
