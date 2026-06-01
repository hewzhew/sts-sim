use crate::runtime::combat::CombatState;
pub fn handle_gain_gold(amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        return;
    }

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::Ectoplasm)
    {
        return;
    }

    state.entities.player.gold += amount;
    state.entities.player.gold_delta_this_combat += amount;

    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::BloodyIdol)
    {
        let actions = crate::content::relics::bloody_idol::BloodyIdol::on_gain_gold();
        state.queue_actions(actions);
    }
}

pub fn handle_steal_player_gold(thief_id: usize, amount: i32, state: &mut CombatState) {
    if amount <= 0 {
        if let Some(thief) = state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == thief_id)
        {
            thief.thief.protocol_seeded = true;
            thief.thief.slash_count = thief.thief.slash_count.saturating_add(1);
        }
        return;
    }

    let actual = amount.min(state.entities.player.gold).max(0);
    state.entities.player.gold = (state.entities.player.gold - actual).max(0);
    state.entities.player.gold_delta_this_combat -= actual;

    if let Some(thief) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == thief_id)
    {
        thief.thief.protocol_seeded = true;
        thief.thief.slash_count = thief.thief.slash_count.saturating_add(1);
        thief.thief.stolen_gold += actual;
    }
}
