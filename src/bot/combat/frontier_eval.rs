use crate::runtime::combat::{CombatState, Power};
use crate::state::EngineState;

use super::legal_moves::get_legal_moves;
use super::pressure::StatePressureFeatures;
use super::terminal::{survives, terminal_outcome, TerminalKind, TerminalOutcome};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrontierNonTerminalEval {
    pub(crate) survives: bool,
    pub(crate) projected_unblocked: i32,
    pub(crate) projected_enemy_total: i32,
    pub(crate) projected_hp: i32,
    pub(crate) projected_block: i32,
    pub(crate) player_buff_score: i32,
    pub(crate) player_debuff_score: i32,
    pub(crate) enemy_buff_score: i32,
    pub(crate) enemy_debuff_score: i32,
    pub(crate) next_non_endturn_options: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FrontierEval {
    Terminal(TerminalOutcome),
    NonTerminal(FrontierNonTerminalEval),
}

pub(crate) fn eval_frontier_state(engine: &EngineState, combat: &CombatState) -> FrontierEval {
    if let Some(outcome) = terminal_outcome(engine, combat) {
        FrontierEval::Terminal(outcome)
    } else {
        FrontierEval::NonTerminal(FrontierNonTerminalEval {
            survives: survives(
                super::terminal::terminal_kind(engine, combat),
                combat.entities.player.current_hp,
            ),
            projected_unblocked: projected_unblocked(combat),
            projected_enemy_total: total_enemy_hp(combat),
            projected_hp: combat.entities.player.current_hp,
            projected_block: combat.entities.player.block,
            player_buff_score: player_buff_score(combat),
            player_debuff_score: player_debuff_score(combat),
            enemy_buff_score: enemy_buff_score(combat),
            enemy_debuff_score: enemy_debuff_score(combat),
            next_non_endturn_options: next_non_endturn_options(engine, combat),
        })
    }
}

pub(crate) fn compare_frontier_eval(
    left: &FrontierEval,
    right: &FrontierEval,
) -> std::cmp::Ordering {
    frontier_bucket(right)
        .cmp(&frontier_bucket(left))
        .then_with(|| match (left, right) {
            (FrontierEval::Terminal(left), FrontierEval::Terminal(right)) => right
                .kind
                .cmp(&left.kind)
                .then_with(|| right.final_hp.cmp(&left.final_hp))
                .then_with(|| right.final_block.cmp(&left.final_block)),
            (FrontierEval::NonTerminal(left), FrontierEval::NonTerminal(right)) => right
                .survives
                .cmp(&left.survives)
                .then_with(|| left.projected_unblocked.cmp(&right.projected_unblocked))
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
                .then_with(|| right.projected_block.cmp(&left.projected_block)),
            _ => std::cmp::Ordering::Equal,
        })
}

fn frontier_bucket(value: &FrontierEval) -> i32 {
    match value {
        FrontierEval::Terminal(TerminalOutcome {
            kind: TerminalKind::Victory,
            ..
        }) => 3,
        FrontierEval::Terminal(TerminalOutcome {
            kind: TerminalKind::CombatCleared,
            ..
        }) => 2,
        FrontierEval::NonTerminal(_) => 1,
        FrontierEval::Terminal(TerminalOutcome {
            kind: TerminalKind::Defeat,
            ..
        }) => 0,
        FrontierEval::Terminal(TerminalOutcome {
            kind: TerminalKind::Ongoing,
            ..
        }) => 1,
    }
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn projected_unblocked(combat: &CombatState) -> i32 {
    (StatePressureFeatures::from_combat(combat).value_incoming - combat.entities.player.block)
        .max(0)
}

fn next_non_endturn_options(engine: &EngineState, combat: &CombatState) -> i32 {
    get_legal_moves(engine, combat)
        .into_iter()
        .filter(|input| !matches!(input, crate::state::core::ClientInput::EndTurn))
        .count() as i32
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

fn generic_power_magnitude(power: &Power) -> i32 {
    power.amount.abs().clamp(1, 8)
}

#[cfg(test)]
mod tests {
    use super::{compare_frontier_eval, eval_frontier_state};
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatCard, Power};
    use crate::state::EngineState;
    use crate::test_support::{blank_test_combat, planned_monster};

    #[test]
    fn frontier_eval_prefers_lower_unblocked_over_same_enemy_total() {
        let mut unsafe_state = blank_test_combat();
        unsafe_state.entities.player.current_hp = 20;
        unsafe_state
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 1));

        let mut safe_state = unsafe_state.clone();
        safe_state.entities.player.block = 6;

        let unsafe_eval = eval_frontier_state(&EngineState::CombatPlayerTurn, &unsafe_state);
        let safe_eval = eval_frontier_state(&EngineState::CombatPlayerTurn, &safe_state);

        assert!(
            compare_frontier_eval(&unsafe_eval, &safe_eval).is_gt(),
            "frontier eval should strongly prefer the frontier with less projected incoming leakage"
        );
    }

    #[test]
    fn frontier_eval_prefers_structural_buffs_without_setup_window_aggregate() {
        let mut baseline = blank_test_combat();
        baseline
            .entities
            .monsters
            .push(planned_monster(EnemyId::Cultist, 3));
        baseline.zones.hand.push(CombatCard::new(CardId::Strike, 1));

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

        let baseline_eval = eval_frontier_state(&EngineState::CombatPlayerTurn, &baseline);
        let buffed_eval = eval_frontier_state(&EngineState::CombatPlayerTurn, &buffed);

        assert!(
            compare_frontier_eval(&baseline_eval, &buffed_eval).is_gt(),
            "frontier eval should still reflect persistent buff quality without depending on the old future_position_score aggregate"
        );
    }
}
