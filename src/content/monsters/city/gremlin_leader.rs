use crate::combat::{CombatState, MonsterEntity, Intent};
use crate::action::{Action, DamageType, DamageInfo};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct GremlinLeader;

impl GremlinLeader {
    pub fn roll_move_custom(rng: &mut crate::rng::StsRng, entity: &MonsterEntity, _ascension_level: u8, _num: i32, monsters: &[MonsterEntity]) -> (u8, Intent) {
        let alive_gremlins = monsters.iter().filter(|m| m.id != entity.id && !m.is_dying).count();
        let last_move = entity.move_history.back().copied().unwrap_or(0);
        
        let num = rng.random_range(0, 99);
        
        let move_byte = if alive_gremlins == 0 {
            if num < 75 {
                if last_move != 2 { 2 } else { 4 }
            } else if last_move != 4 {
                4
            } else {
                2
            }
        } else if alive_gremlins < 2 {
            if num < 50 {
                if last_move != 2 { 2 } else {
                    // Re-roll to 50-99
                    let reroll = rng.random_range(50, 99);
                    if reroll < 80 {
                        if last_move != 3 { 3 } else { 4 }
                    } else if last_move != 4 {
                        4
                    } else {
                        // In Java, rerolling to 0-80 would happen if all these checks failed.
                        // For simplicity since STS RNG rerolls can be deeply recursive, we just map it out:
                        let reroll2 = rng.random_range(0, 79);
                        if reroll2 < 50 { 2 } else { 3 }
                    }
                }
            } else if num < 80 {
                if last_move != 3 { 3 } else { 4 }
            } else if last_move != 4 {
                4
            } else {
                let reroll = rng.random_range(0, 79);
                if reroll < 50 {
                    if last_move != 2 { 2 } else { 3 } // Approx
                } else if last_move != 3 { 3 } else { 4 }
            }
        } else {
            if num < 66 {
                if last_move != 3 { 3 } else { 4 }
            } else if last_move != 4 {
                4
            } else {
                3
            }
        };

        match move_byte {
            2 => (2, Intent::Unknown),
            3 => (3, Intent::DefendBuff),
            4 | _ => (4, Intent::Attack { damage: 6, hits: 3 }),
        }
    }
}

impl MonsterBehavior for GremlinLeader {
    fn roll_move(_rng: &mut crate::rng::StsRng, _entity: &MonsterEntity, _ascension_level: u8, _num: i32) -> (u8, Intent) {
        // Will never be called because engine overrides this.
        (2, Intent::Unknown)
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        match entity.next_move_byte {
            2 => { // RALLY
                let mut minion_slots = vec![];
                if !state.monsters.iter().any(|m| m.slot == 1 && !m.is_dying) { minion_slots.push(1); }
                if !state.monsters.iter().any(|m| m.slot == 2 && !m.is_dying) { minion_slots.push(2); }

                let variants = [
                    crate::content::monsters::EnemyId::GremlinTsundere,
                    crate::content::monsters::EnemyId::GremlinFat,
                    crate::content::monsters::EnemyId::GremlinWarrior,
                    crate::content::monsters::EnemyId::GremlinThief,
                    crate::content::monsters::EnemyId::GremlinWizard,
                ];

                let mut mock_max_id = state.monsters.iter().map(|m| m.id).max().unwrap_or(0);

                for slot in minion_slots {
                    mock_max_id += 1;
                    let minion_id = variants[state.rng.ai_rng.random_range(0, 4) as usize];
                    actions.push(Action::SpawnMonsterSmart {
                        monster_id: minion_id,
                        current_hp: 0,
                        max_hp: 0,
                        logical_position: slot as i32,
                    });

                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: mock_max_id,
                        power_id: PowerId::Minion,
                        amount: 1,
                    });
                }
            }
            3 => { // ENCOURAGE
                let str_amt = if state.ascension_level >= 18 { 5 } else if state.ascension_level >= 3 { 4 } else { 3 };
                let block_amt = if state.ascension_level >= 18 { 10 } else { 6 };

                actions.push(Action::ApplyPower { source: entity.id, target: entity.id, power_id: PowerId::Strength, amount: str_amt });

                for m in state.monsters.iter() {
                    if m.id != entity.id && !m.is_dying {
                        actions.push(Action::ApplyPower { source: entity.id, target: m.id, power_id: PowerId::Strength, amount: str_amt });
                        actions.push(Action::GainBlock { target: m.id, amount: block_amt });
                    }
                }
            }
            4 => { // STAB
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: 6,
                        output: 6,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            _ => {}
        }
        actions.push(Action::RollMonsterMove { monster_id: entity.id });

        actions
    }
}
