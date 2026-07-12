use serde::Serialize;

use crate::content::powers::{store, PowerId};
use crate::runtime::combat::CombatState;
use crate::EntityId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum TimedEnemyThreatKind {
    ForcedPlayerDamage,
}

impl TimedEnemyThreatKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::ForcedPlayerDamage => "forced_player_damage",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub(super) struct TimedEnemyThreatV1 {
    pub(super) source_entity_id: EntityId,
    pub(super) kind: TimedEnemyThreatKind,
    pub(super) owner_turns_until_trigger: u32,
    pub(super) raw_player_damage: i32,
    pub(super) canceled_by_owner_death: bool,
}

pub(super) fn timed_enemy_threats(combat: &CombatState) -> Vec<TimedEnemyThreatV1> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .filter_map(|monster| timed_enemy_threat_for_target(combat, monster.id))
        .collect()
}

pub(super) fn timed_enemy_threat_for_target(
    combat: &CombatState,
    entity_id: EntityId,
) -> Option<TimedEnemyThreatV1> {
    let owner = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())?;
    let amount = store::power_amount(combat, owner.id, PowerId::Explosive);
    (amount > 0).then_some(TimedEnemyThreatV1 {
        source_entity_id: owner.id,
        kind: TimedEnemyThreatKind::ForcedPlayerDamage,
        owner_turns_until_trigger: amount as u32,
        raw_player_damage: crate::content::powers::core::explosive::EXPLOSION_DAMAGE,
        canceled_by_owner_death: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatState, Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    fn combat_with_explosive(amount: i32) -> CombatState {
        let mut combat = blank_test_combat();
        let mut owner = test_monster(EnemyId::Exploder);
        owner.id = 7;
        combat.entities.monsters = vec![owner];
        combat.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::Explosive,
                instance_id: None,
                amount,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat
    }

    #[test]
    fn timed_enemy_threat_reads_positive_explosive_power() {
        let combat = combat_with_explosive(3);

        assert_eq!(
            timed_enemy_threats(&combat),
            vec![TimedEnemyThreatV1 {
                source_entity_id: 7,
                kind: TimedEnemyThreatKind::ForcedPlayerDamage,
                owner_turns_until_trigger: 3,
                raw_player_damage: crate::content::powers::core::explosive::EXPLOSION_DAMAGE,
                canceled_by_owner_death: true,
            }]
        );
    }

    #[test]
    fn timed_enemy_threat_tracks_urgency_and_ignores_inactive_owners() {
        let amount_one = combat_with_explosive(1);
        assert_eq!(
            timed_enemy_threat_for_target(&amount_one, 7)
                .map(|fact| fact.owner_turns_until_trigger),
            Some(1)
        );

        let powerless = combat_with_explosive(0);
        assert!(timed_enemy_threats(&powerless).is_empty());

        let mut dead = combat_with_explosive(3);
        dead.entities.monsters[0].is_dying = true;
        assert!(timed_enemy_threats(&dead).is_empty());
    }
}
