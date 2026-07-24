use crate::content::powers::{store, PowerId};
use crate::state::core::CombatContext;
use crate::state::map::node::RoomType;

use super::{RunControlHpLossLimit, RunControlSession};

/// Returns the largest HP loss that is already good enough for strategic run
/// progression to stop refining an exact combat witness.
///
/// This is deliberately distinct from the much looser execution reserve. A
/// line must both preserve the run's generic survival reserve and lose no more
/// than one fifth of max HP. Finite-duration survival encounters additionally
/// preserve half of entry HP. Bosses that immediately heal at the next act or
/// end the requested run do not need a combat-local quality target.
pub fn strategic_combat_quality_hp_loss_limit_v1(
    session: &RunControlSession,
) -> RunControlHpLossLimit {
    let RunControlHpLossLimit::Limit(reserve_limit) =
        strategic_combat_survival_hp_loss_limit_v1(session)
    else {
        return RunControlHpLossLimit::Unlimited;
    };
    let (_, max_hp) = session.visible_player_hp();
    let max_hp = max_hp.max(1);
    let quality_limit = (max_hp / 5).max(1) as u32;
    RunControlHpLossLimit::Limit(reserve_limit.min(quality_limit))
}

/// Returns the broad survival reserve used to reject combat lines that would
/// leave the run without a meaningful floor-to-floor buffer.
pub fn strategic_combat_survival_hp_loss_limit_v1(
    session: &RunControlSession,
) -> RunControlHpLossLimit {
    if room_boss_win_reaches_recovery_or_run_end(session) {
        return RunControlHpLossLimit::Unlimited;
    }
    let (current_hp, max_hp) = session.visible_player_hp();
    let max_hp = max_hp.max(1);
    let generic_reserve_hp = (max_hp / 4).max(1);
    let finite_survival_reserve_hp = finite_survival_damage_mitigation_active(session)
        .then(|| current_hp.max(0).saturating_add(1) / 2)
        .unwrap_or_default();
    let reserve_hp = generic_reserve_hp.max(finite_survival_reserve_hp);
    let max_hp_loss = current_hp.saturating_sub(reserve_hp).max(0) as u32;
    RunControlHpLossLimit::Limit(max_hp_loss)
}

fn finite_survival_damage_mitigation_active(session: &RunControlSession) -> bool {
    session.active_combat.as_ref().is_some_and(|active| {
        active
            .combat_state
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .any(|monster| {
                store::power_amount(&active.combat_state, monster.id, PowerId::Fading) > 0
                    && store::has_power(&active.combat_state, monster.id, PowerId::Shifting)
            })
    })
}

fn room_boss_win_reaches_recovery_or_run_end(session: &RunControlSession) -> bool {
    session.active_combat.as_ref().is_some_and(|active| {
        matches!(
            active.context,
            CombatContext::Room(ref room) if room.room_type == RoomType::MonsterRoomBoss
        )
    }) && !session.run_state.should_start_act3_double_boss()
}
