use crate::combat::{MonsterEntity, Intent};
use crate::action::{Action, DamageType, DamageInfo};
use crate::content::monsters::MonsterBehavior;

pub struct SpireShield;

impl MonsterBehavior for SpireShield {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        // Java: SpireShield.getMove uses moveCount % 3 deterministic cycle.
        // moveCount increments AFTER each selection (post-increment).
        // Since roll_move is called once per turn, moveCount == move_history.len()
        let move_count = entity.move_history.len();

        let bash_dmg = if ascension_level >= 3 { 14 } else { 12 };
        let smash_dmg = if ascension_level >= 3 { 38 } else { 34 };

        let last_move = entity.move_history.back().copied().unwrap_or(0);

        match move_count % 3 {
            0 => {
                // Java: aiRng.randomBoolean() → 50% Fortify, 50% Bash
                if _rng.random_boolean() {
                    (2, Intent::Defend) // FORTIFY
                } else {
                    (1, Intent::AttackDebuff { damage: bash_dmg, hits: 1 }) // BASH
                }
            }
            1 => {
                // Java: if (!lastMove(BASH)) → Bash; else → Fortify
                if last_move != 1 {
                    (1, Intent::AttackDebuff { damage: bash_dmg, hits: 1 }) // BASH
                } else {
                    (2, Intent::Defend) // FORTIFY
                }
            }
            _ => {
                // Java: always SMASH (byte 3) with ATTACK_DEFEND intent
                (3, Intent::AttackDefend { damage: smash_dmg, hits: 1 }) // SMASH
            }
        }
    }

    fn use_pre_battle_action(_entity: &MonsterEntity, _hp_rng: &mut crate::rng::StsRng, ascension_level: u8) -> Vec<Action> {
        // Java: Apply Surrounded to player (positional, not modeled) + Artifact to self
        let artifact_amt = if ascension_level >= 18 { 2 } else { 1 };
        vec![
            Action::ApplyPower {
                source: _entity.id,
                target: _entity.id,
                power_id: crate::content::powers::PowerId::Artifact,
                amount: artifact_amt,
            }
        ]
    }

    fn take_turn(
        state: &mut crate::combat::CombatState,
        entity: &MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.ascension_level;

        let bash_dmg = if asc >= 3 { 14 } else { 12 };
        let smash_dmg = if asc >= 3 { 38 } else { 34 };

        match entity.next_move_byte {
            1 => { // BASH — attack + debuff (-1 Str or -1 Focus)
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: bash_dmg,
                    output: bash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // Java: if player has orbs && aiRng.randomBoolean() → -1 Focus
                // else → -1 Str. 
                if state.player.max_orbs > 0 && state.rng.ai_rng.random_boolean() {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: 0,
                        power_id: crate::content::powers::PowerId::Focus,
                        amount: -1,
                    });
                } else {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: 0,
                        power_id: crate::content::powers::PowerId::Strength,
                        amount: -1,
                    });
                }
            },
            2 => { // FORTIFY — block all monsters (Java uses flat 30, not Asc-scaled)
                for m in &state.monsters {
                    if !m.is_dying {
                        actions.push(Action::GainBlock { target: m.id, amount: 30 });
                    }
                }
            },
            3 => { // SMASH — attack + gain block
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: smash_dmg,
                    output: smash_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                // Java: Asc 18 → 99 block, else → damage output as block
                let block = if asc >= 18 { 99 } else { smash_dmg };
                actions.push(Action::GainBlock { target: entity.id, amount: block });
            },
            _ => {},
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });

        actions
    }

    fn on_death(state: &mut crate::combat::CombatState, _entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        // Java: if player has "Surrounded" power, remove it and adjust player facing
        if state.power_db.get(&0).map_or(false, |powers| powers.iter().any(|p| p.power_type == crate::content::powers::PowerId::Surrounded)) {
            actions.push(Action::RemovePower { target: 0, power_id: crate::content::powers::PowerId::Surrounded });
        }
        
        // Java: Remove "BackAttack" power from surviving monsters
        for m in &state.monsters {
            if m.current_hp > 0 && !m.is_dying {
                if state.power_db.get(&m.id).map_or(false, |powers| powers.iter().any(|p| p.power_type == crate::content::powers::PowerId::BackAttack)) {
                    actions.push(Action::RemovePower { target: m.id, power_id: crate::content::powers::PowerId::BackAttack });
                }
            }
        }
        actions
    }
}
