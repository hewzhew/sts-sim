use sts_simulator::eval::run_control::{RunControlHpLossLimit, RunControlSession};

pub(super) fn owner_audit_hp_loss_limit(session: &RunControlSession) -> RunControlHpLossLimit {
    let (current_hp, max_hp) = session.visible_player_hp();
    let max_hp = max_hp.max(1);
    let reserve_hp = max_hp / 4 + i32::from(max_hp % 4 != 0);
    let max_hp_loss = current_hp.saturating_sub(reserve_hp).max(0) as u32;
    RunControlHpLossLimit::Limit(max_hp_loss)
}
