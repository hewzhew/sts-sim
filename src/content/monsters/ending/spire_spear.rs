use crate::combat::{MonsterEntity, Intent};
use crate::action::{Action, DamageType, DamageInfo};
use crate::content::monsters::MonsterBehavior;

pub struct SpireSpear;

impl MonsterBehavior for SpireSpear {
    fn roll_move(rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        let history = &entity.move_history;
        let _last_move = if history.len() > 0 { history[history.len() - 1] } else { 0 };
        let count = entity.move_history.len();
        let last_move = entity.move_history.back().copied().unwrap_or(0);
        
        let burn_strike_dmg = if ascension_level >= 18 { 6 } else { 5 };
        let _piercing_dmg = if ascension_level >= 18 { 10 } else { 5 };
        let skewer_dmg = if ascension_level >= 18 { 10 } else { 10 };
        let skewer_hits = if ascension_level >= 18 { 4 } else { 3 }; 

        match count % 3 {
            0 => {
                if last_move != 1 {
                    (1, Intent::AttackDebuff { damage: burn_strike_dmg, hits: 2 })
                } else {
                    (2, Intent::Buff)
                }
            }
            1 => {
                (3, Intent::Attack { damage: skewer_dmg, hits: skewer_hits })
            }
            _ => {
                if rng.random_boolean() {
                    (2, Intent::Buff)
                } else {
                    (1, Intent::AttackDebuff { damage: burn_strike_dmg, hits: 2 })
                }
            }
        }
    }

    fn use_pre_battle_action(_entity: &MonsterEntity, _hp_rng: &mut crate::rng::StsRng, ascension_level: u8) -> Vec<Action> {
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
        let move_byte = entity.next_move_byte;
        
        let burn_strike_dmg = if asc >= 18 { 6 } else { 5 };
        let skewer_dmg = if asc >= 18 { 10 } else { 10 };
        let skewer_hits = if asc >= 18 { 4 } else { 3 };

        match move_byte {
            1 => { // Burn Strike
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id, target: 0,
                        base: burn_strike_dmg, output: burn_strike_dmg,
                        damage_type: DamageType::Normal, is_modified: false,
                    }));
                }
                if asc >= 18 {
                    actions.push(Action::MakeTempCardInDrawPile {
                        card_id: crate::content::cards::CardId::Burn,
                        amount: 2,
                        random_spot: true,
                        upgraded: false,
                    });
                } else {
                    actions.push(Action::MakeTempCardInDiscard {
                        card_id: crate::content::cards::CardId::Burn,
                        amount: 2,
                        upgraded: false,
                    });
                }
            },
            2 => { // Buff (Strength +2 to all monsters)
                for m in &state.monsters {
                    if m.current_hp > 0 && !m.is_dying {
                        actions.push(Action::ApplyPower {
                            source: entity.id, target: m.id,
                            power_id: crate::content::powers::PowerId::Strength,
                            amount: 2,
                        });
                    }
                }
            },
            3 => { // Skewer
                for _ in 0..skewer_hits {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id, target: 0,
                        base: skewer_dmg, output: skewer_dmg,
                        damage_type: DamageType::Normal, is_modified: false,
                    }));
                }
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
