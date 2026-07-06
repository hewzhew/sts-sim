use super::super::*;

pub(super) fn accepted_complete_win(node: &SearchNode, config: &CombatSearchV2Config) -> bool {
    if terminal_label(&node.engine, &node.combat) != SearchTerminalLabel::Win {
        return false;
    }
    let hp_loss = (node.initial_hp - node.combat.entities.player.current_hp).max(0) as u32;
    if hp_loss == 0 && !super::super::external_payoff::has_external_payoff_opportunity(&node.combat)
    {
        return true;
    }
    let Some(limit) = config.stop_on_win_hp_loss_at_most else {
        return false;
    };
    hp_loss <= limit
}
