use sts_simulator::eval::run_control::{
    strategic_combat_quality_hp_loss_limit_v1, strategic_combat_survival_hp_loss_limit_v1,
    RunControlHpLossLimit, RunControlSession,
};

pub(super) fn owner_audit_hp_loss_limit(session: &RunControlSession) -> RunControlHpLossLimit {
    strategic_combat_survival_hp_loss_limit_v1(session)
}

/// A route reserve is the largest loss the run can survive, not a quality
/// target for ending combat search.  Requiring a materially cleaner incumbent
/// before early exit prevents high-HP hallways from spending only a few dozen
/// nodes and handing the resulting attrition to every later floor.
pub(super) fn owner_audit_search_quality_loss_target(
    session: &RunControlSession,
) -> RunControlHpLossLimit {
    strategic_combat_quality_hp_loss_limit_v1(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::monsters::{factory::EncounterId, EnemyId};
    use sts_simulator::content::powers::PowerId;
    use sts_simulator::eval::run_control::RunControlConfig;
    use sts_simulator::runtime::combat::{Power, PowerPayload};
    use sts_simulator::state::core::{
        ActiveCombat, CombatContext, EngineState, EventCombatContext, PostCombatReturn,
        RoomCombatContext,
    };
    use sts_simulator::state::map::node::RoomType;
    use sts_simulator::state::rewards::RewardState;

    fn session_with_context(context: CombatContext) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        combat.entities.player.current_hp = 40;
        combat.entities.player.max_hp = 79;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            context,
        ));
        session
    }

    fn room_session(room_type: RoomType) -> RunControlSession {
        session_with_context(CombatContext::Room(RoomCombatContext { room_type }))
    }

    #[test]
    fn boss_victory_hp_boundary_allows_act2_room_boss_recovery() {
        let mut session = room_session(RoomType::MonsterRoomBoss);
        session.run_state.act_num = 2;

        assert_eq!(
            owner_audit_hp_loss_limit(&session),
            RunControlHpLossLimit::Unlimited
        );
    }

    #[test]
    fn boss_victory_hp_boundary_keeps_floor_reserve_for_nontransition_combats() {
        let hallway = room_session(RoomType::MonsterRoom);
        let event_boss = session_with_context(CombatContext::Event(EventCombatContext {
            rewards: RewardState::new(),
            reward_allowed: true,
            no_cards_in_rewards: false,
            elite_trigger: false,
            post_combat_return: PostCombatReturn::EventRoom,
        }));

        for session in [hallway, event_boss] {
            assert_eq!(
                owner_audit_hp_loss_limit(&session),
                RunControlHpLossLimit::Limit(21)
            );
            assert_eq!(
                owner_audit_search_quality_loss_target(&session),
                RunControlHpLossLimit::Limit(15)
            );
        }
    }

    #[test]
    fn finite_survival_mechanic_requires_half_of_entry_hp_before_accepting_a_line() {
        let mut session = room_session(RoomType::MonsterRoom);
        let combat = &mut session
            .active_combat
            .as_mut()
            .expect("active combat")
            .combat_state;
        combat.entities.player.current_hp = 74;
        combat.entities.player.max_hp = 80;
        combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::Cultist)];
        let owner = combat.entities.monsters.first_mut().expect("monster");
        owner.id = 7;
        combat.entities.power_db.insert(
            owner.id,
            vec![
                Power {
                    power_type: PowerId::Fading,
                    instance_id: None,
                    amount: 5,
                    extra_data: 0,
                    payload: PowerPayload::None,
                    just_applied: false,
                },
                Power {
                    power_type: PowerId::Shifting,
                    instance_id: None,
                    amount: -1,
                    extra_data: 0,
                    payload: PowerPayload::None,
                    just_applied: false,
                },
            ],
        );

        assert_eq!(
            owner_audit_hp_loss_limit(&session),
            RunControlHpLossLimit::Limit(37)
        );
    }

    #[test]
    fn boss_victory_hp_boundary_keeps_only_first_a20_double_boss_reserve() {
        let mut first_boss = room_session(RoomType::MonsterRoomBoss);
        first_boss.run_state.act_num = 3;
        first_boss.run_state.ascension_level = 20;
        first_boss.run_state.boss_list = vec![EncounterId::AwakenedOne, EncounterId::TimeEater];

        let mut second_boss = first_boss.clone();
        second_boss.run_state.boss_list = vec![EncounterId::TimeEater];

        assert_eq!(
            owner_audit_hp_loss_limit(&first_boss),
            RunControlHpLossLimit::Limit(21)
        );
        assert_eq!(
            owner_audit_hp_loss_limit(&second_boss),
            RunControlHpLossLimit::Unlimited
        );
    }
}
