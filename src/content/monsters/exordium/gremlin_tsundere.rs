use crate::combat::{CombatState, MonsterEntity, Intent};
use crate::core::EntityId;
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct GremlinTsundere;

impl MonsterBehavior for GremlinTsundere {
    fn roll_move(_rng: &mut crate::rng::StsRng, _entity: &MonsterEntity, _ascension_level: u8, _num: i32) -> (u8, Intent) {
        // First move is always Protect.
        (1, Intent::Defend)
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.ascension_level;
        let block_amt = if asc >= 17 { 11 } else if asc >= 7 { 8 } else { 7 };
        let bash_dmg = if asc >= 2 { 8 } else { 6 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => { // PROTECT
                // Java uses GainBlockRandomMonsterAction(this, blockAmt)
                // which excludes: m == source, m.intent == ESCAPE, m.isDying
                // It does NOT exclude m.current_hp == 0 or is_escaped
                let alive_monsters: Vec<EntityId> = state.monsters.iter()
                    .filter(|m| m.id != entity.id  // exclude self (source)
                        && m.current_intent != Intent::Escape
                        && !m.is_dying)
                    .map(|m| m.id)
                    .collect();
                
                let target_id = if alive_monsters.is_empty() {
                    entity.id  // fallback to self if no valid targets
                } else {
                    let idx = state.rng.ai_rng.random(alive_monsters.len() as i32 - 1) as usize;
                    alive_monsters[idx]
                };

                actions.push(Action::GainBlock {
                    target: target_id,
                    amount: block_amt,
                });
                
                // Java: checks aliveCount (all non-dying, non-escaping monsters)
                // then decides next move based on whether >1 alive
                let alive_count = state.monsters.iter()
                    .filter(|m| !m.is_dying && !m.is_escaped)
                    .count();
                
                if alive_count > 1 {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 1,
                        intent: Intent::Defend,
                    });
                } else {
                    actions.push(Action::SetMonsterMove {
                        monster_id: entity.id,
                        next_move_byte: 2,
                        intent: Intent::Attack { damage: bash_dmg, hits: 1 },
                    });
                }
            }
            2 => { // BASH
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bash_dmg,
                    output: bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // BASH repeats forever
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 2,
                    intent: Intent::Attack { damage: bash_dmg, hits: 1 },
                });
            }
            99 => { // ESCAPE
                actions.push(Action::Escape { target: entity.id });
            }
            _ => { }
        }

        // NOTE: No RollMonsterMove here due to Java's direct SetMove bypassing RollMove actions.
        // Bypassing RollMonsterMove retains strict RNG alignment syncing.

        actions
    }
}
