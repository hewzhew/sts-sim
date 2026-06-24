use crate::content::powers::{store, PowerId};
use crate::runtime::combat::{CombatState, Power, PowerPayload};
use crate::EntityId;

fn monster_is_left_of_player(state: &CombatState, monster_id: EntityId) -> Option<bool> {
    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == monster_id)
        .map(|monster| {
            match crate::content::monsters::EnemyId::from_id(monster.monster_type) {
                // Java positions Shield slightly left of the player and Spear
                // far to the right. Protocol imports may store absolute drawX
                // in logical_position, so keep the Act 4 pair semantic by type.
                Some(crate::content::monsters::EnemyId::SpireShield) => true,
                Some(crate::content::monsters::EnemyId::SpireSpear) => false,
                _ => monster.logical_position < 0,
            }
        })
}

fn monster_is_behind_player(monster_is_left: bool, player_facing_left: bool) -> bool {
    if player_facing_left {
        !monster_is_left
    } else {
        monster_is_left
    }
}

fn back_attack_power() -> Power {
    Power {
        power_type: PowerId::BackAttack,
        instance_id: None,
        amount: -1,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: true,
    }
}

/// Java `AbstractPlayer.playCard()` flips the player toward the hovered target
/// only while Surrounded is active. In headless Rust this is the mechanical
/// substitute for that UI field mutation.
pub fn face_target_for_surrounded_if_needed(state: &mut CombatState, target: Option<EntityId>) {
    if !store::has_power(state, 0, PowerId::Surrounded) {
        return;
    }

    let Some(target_id) = target else {
        return;
    };
    let Some(target_is_left) = monster_is_left_of_player(state, target_id) else {
        return;
    };

    state.entities.player.facing_left = target_is_left;
    sync_back_attack_markers(state);
}

/// Keep Rust's BackAttack marker aligned with Java's Surrounded facing rule.
///
/// Java calculates the actual 1.5x damage from player facing and monster drawX
/// in `AbstractMonster.applyBackAttack()`. It also queues BackAttackPower from
/// `applyPowers()` / hand-layout refresh as the visible marker. Rust's damage
/// pipeline uses the marker as the multiplier source, so this synchronization is
/// mechanical, not cosmetic UI simulation.
pub fn sync_back_attack_markers(state: &mut CombatState) {
    let surrounded = store::has_power(state, 0, PowerId::Surrounded);
    let facing_left = state.entities.player.facing_left;
    let updates = state
        .entities
        .monsters
        .iter()
        .map(|monster| {
            let monster_is_left = monster_is_left_of_player(state, monster.id).unwrap_or(false);
            let should_have = surrounded
                && !monster.is_dying
                && !monster.is_escaped
                && !monster.half_dead
                && monster_is_behind_player(monster_is_left, facing_left);
            (monster.id, should_have)
        })
        .collect::<Vec<_>>();

    for (monster_id, should_have) in updates {
        let has_marker = store::has_power(state, monster_id, PowerId::BackAttack);
        match (should_have, has_marker) {
            (true, false) => {
                store::ensure_powers_for_mut(state, monster_id).push(back_attack_power());
                store::sort_powers_for_java(state, monster_id);
            }
            (false, true) => {
                store::remove_power_type(state, monster_id, PowerId::BackAttack);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{face_target_for_surrounded_if_needed, sync_back_attack_markers};
    use crate::content::monsters::EnemyId;
    use crate::content::powers::{store, PowerId};
    use crate::runtime::combat::{Power, PowerPayload};

    fn sentinel_power(power_type: PowerId) -> Power {
        Power {
            power_type,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }
    }

    fn shield_and_spear_state() -> crate::runtime::combat::CombatState {
        let mut shield = crate::test_support::test_monster(EnemyId::SpireShield);
        shield.id = 1;
        shield.logical_position = -1;
        let mut spear = crate::test_support::test_monster(EnemyId::SpireSpear);
        spear.id = 2;
        spear.logical_position = 1;
        let mut state = crate::test_support::combat_with_monsters(vec![shield, spear]);
        store::set_powers_for(&mut state, 0, vec![sentinel_power(PowerId::Surrounded)]);
        state
    }

    #[test]
    fn sync_marks_initial_right_facing_left_side_back_attack_like_java() {
        let mut state = shield_and_spear_state();
        state.entities.player.facing_left = false;

        sync_back_attack_markers(&mut state);

        assert!(store::has_power(&state, 1, PowerId::BackAttack));
        assert!(!store::has_power(&state, 2, PowerId::BackAttack));
    }

    #[test]
    fn targeting_left_monster_flips_player_and_moves_back_attack_to_right_monster() {
        let mut state = shield_and_spear_state();
        sync_back_attack_markers(&mut state);

        face_target_for_surrounded_if_needed(&mut state, Some(1));

        assert!(state.entities.player.facing_left);
        assert!(!store::has_power(&state, 1, PowerId::BackAttack));
        assert!(store::has_power(&state, 2, PowerId::BackAttack));
    }

    #[test]
    fn removing_surrounded_clears_back_attack_markers() {
        let mut state = shield_and_spear_state();
        sync_back_attack_markers(&mut state);

        store::remove_power_type(&mut state, 0, PowerId::Surrounded);
        sync_back_attack_markers(&mut state);

        assert!(!store::has_power(&state, 1, PowerId::BackAttack));
        assert!(!store::has_power(&state, 2, PowerId::BackAttack));
    }

    #[test]
    fn protocol_absolute_draw_x_keeps_shield_left_and_spear_right_by_type() {
        let mut state = shield_and_spear_state();
        state.entities.monsters[0].logical_position = 440;
        state.entities.monsters[1].logical_position = 1510;
        state.entities.player.facing_left = false;

        sync_back_attack_markers(&mut state);

        assert!(store::has_power(&state, 1, PowerId::BackAttack));
        assert!(!store::has_power(&state, 2, PowerId::BackAttack));
    }
}
