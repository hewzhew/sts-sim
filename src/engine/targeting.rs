use crate::combat::CombatState;
use crate::content::cards::CardTarget;
use crate::core::EntityId;
use crate::state::TargetValidation;

pub fn validation_for_card_target(target: CardTarget) -> Option<TargetValidation> {
    match target {
        CardTarget::Enemy => Some(TargetValidation::AnyEnemy),
        _ => None,
    }
}

pub fn validation_for_potion_target(target_required: bool) -> Option<TargetValidation> {
    if target_required {
        Some(TargetValidation::AnyMonster)
    } else {
        None
    }
}

pub fn candidate_targets(state: &CombatState, validation: TargetValidation) -> Vec<EntityId> {
    match validation {
        TargetValidation::AnyEnemy | TargetValidation::AnyMonster => state
            .monsters
            .iter()
            .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
            .map(|m| m.id)
            .collect(),
    }
}

pub fn resolve_target_request(
    state: &CombatState,
    validation: Option<TargetValidation>,
    requested: Option<EntityId>,
) -> Result<Option<EntityId>, &'static str> {
    let Some(validation) = validation else {
        return Ok(None);
    };

    let targetable = candidate_targets(state, validation);
    match requested {
        Some(target) if targetable.contains(&target) => Ok(Some(target)),
        Some(_) => Err("Invalid or untargetable monster selected."),
        None if targetable.is_empty() => Err("No valid targets available."),
        None if targetable.len() == 1 => Ok(Some(targetable[0])),
        None => Err("Multiple targets available. Must specify a target."),
    }
}

pub fn pick_random_target(
    state: &mut CombatState,
    validation: TargetValidation,
) -> Option<EntityId> {
    let targetable = candidate_targets(state, validation);
    if targetable.is_empty() {
        return None;
    }
    let idx = state
        .rng
        .card_random_rng
        .random(targetable.len() as i32 - 1) as usize;
    targetable.get(idx).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::combat::{
        CombatPhase, CombatState, Intent, MonsterEntity, PlayerEntity, RelicBuses, StanceId,
    };
    use crate::content::monsters::EnemyId;
    use std::collections::{HashMap, VecDeque};

    fn combat_with_monsters(monsters: Vec<MonsterEntity>) -> CombatState {
        CombatState {
            ascension_level: 0,
            turn_count: 1,
            current_phase: CombatPhase::PlayerTurn,
            energy: 3,
            draw_pile: Vec::new(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            player: PlayerEntity {
                id: 0,
                current_hp: 80,
                max_hp: 80,
                block: 0,
                gold_delta_this_combat: 0,
                gold: 99,
                max_orbs: 0,
                orbs: Vec::new(),
                stance: StanceId::Neutral,
                relics: Vec::new(),
                relic_buses: RelicBuses::default(),
                energy_master: 3,
            },
            monsters,
            potions: vec![None, None, None],
            power_db: HashMap::new(),
            action_queue: VecDeque::<Action>::new(),
            counters: Default::default(),
            card_uuid_counter: 1,
            rng: crate::rng::RngPool::new(123),
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        }
    }

    fn monster(id: usize) -> MonsterEntity {
        MonsterEntity {
            id,
            monster_type: EnemyId::JawWorm as usize,
            current_hp: 40,
            max_hp: 40,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            next_move_byte: 0,
            current_intent: Intent::Unknown,
            move_history: VecDeque::new(),
            intent_dmg: 0,
            logical_position: 0,
        }
    }

    #[test]
    fn resolves_single_target_implicitly() {
        let state = combat_with_monsters(vec![monster(7)]);
        let resolved =
            resolve_target_request(&state, Some(TargetValidation::AnyEnemy), None).unwrap();
        assert_eq!(resolved, Some(7));
    }

    #[test]
    fn rejects_missing_target_when_multiple_exist() {
        let state = combat_with_monsters(vec![monster(1), monster(2)]);
        let err =
            resolve_target_request(&state, Some(TargetValidation::AnyEnemy), None).unwrap_err();
        assert_eq!(err, "Multiple targets available. Must specify a target.");
    }

    #[test]
    fn rejects_dead_targets() {
        let mut dead = monster(9);
        dead.is_dying = true;
        let state = combat_with_monsters(vec![dead]);
        let err =
            resolve_target_request(&state, Some(TargetValidation::AnyEnemy), Some(9)).unwrap_err();
        assert_eq!(err, "Invalid or untargetable monster selected.");
    }
}
