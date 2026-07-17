use super::super::*;

pub(super) fn accepted_complete_win(node: &SearchNode, config: &CombatSearchV2Config) -> bool {
    if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Win {
        return false;
    }
    let hp_loss = (node.initial_hp - node.combat.entities.player.current_hp).max(0) as u32;
    match config.satisfaction {
        CombatSearchV2Satisfaction::BudgetOrExhaustion => false,
        CombatSearchV2Satisfaction::ZeroLossOrBudget => {
            hp_loss == 0
                && !super::super::external_payoff::has_external_payoff_opportunity(&node.combat)
        }
        CombatSearchV2Satisfaction::FirstCompleteWin => true,
        CombatSearchV2Satisfaction::HpLossAtMost(limit) => hp_loss <= limit,
    }
}
