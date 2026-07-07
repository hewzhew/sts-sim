use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::run_control::RunControlSession;

use super::combat_search_lanes::CombatSearchStakes;

#[derive(Clone, Copy)]
pub(super) struct CombatSearchPortfolioContext {
    pub(super) stakes: CombatSearchStakes,
    pub(super) time_eater_boss: bool,
    pub(super) nonboss_potion_rescue_signal: bool,
}

impl CombatSearchPortfolioContext {
    pub(super) fn from_session(session: &RunControlSession) -> Self {
        Self {
            stakes: combat_search_stakes(session),
            time_eater_boss: is_time_eater_boss(session),
            nonboss_potion_rescue_signal: should_try_nonboss_potion_rescue(session),
        }
    }
}

fn combat_search_stakes(session: &RunControlSession) -> CombatSearchStakes {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            if active.combat_state.meta.is_boss_fight {
                CombatSearchStakes::Boss
            } else if active.combat_state.meta.is_elite_fight {
                CombatSearchStakes::Elite
            } else {
                CombatSearchStakes::Hallway
            }
        })
        .unwrap_or(CombatSearchStakes::Hallway)
}

fn is_time_eater_boss(session: &RunControlSession) -> bool {
    session.active_combat.as_ref().is_some_and(|active| {
        active.combat_state.meta.is_boss_fight
            && active
                .combat_state
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .any(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::TimeEater))
    })
}

fn should_try_nonboss_potion_rescue(session: &RunControlSession) -> bool {
    let Some(active) = session.active_combat.as_ref() else {
        return false;
    };
    let meta = &active.combat_state.meta;
    let player = &active.combat_state.entities.player;
    let has_usable_potion = active
        .combat_state
        .entities
        .potions
        .iter()
        .flatten()
        .any(|potion| potion.can_use);
    !meta.is_boss_fight
        && has_usable_potion
        && (meta.is_elite_fight
            || session.run_state.act_num >= 3
            || player.current_hp * 2 <= player.max_hp)
}
