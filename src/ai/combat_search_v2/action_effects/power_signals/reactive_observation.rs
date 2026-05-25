use crate::content::powers::store::powers_snapshot_for;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState};

use super::monster_signals::is_living_monster_id;
use super::observation::{observe_power_action, RawPowerEffects};

pub(super) fn observe_card_play_reactive_power_actions(
    combat: &CombatState,
    card: &CombatCard,
    raw: &mut RawPowerEffects,
) {
    let trigger_owners = std::iter::once(0usize)
        .chain(combat.entities.monsters.iter().map(|monster| monster.id))
        .collect::<Vec<_>>();
    for owner in trigger_owners {
        for power in powers_snapshot_for(combat, owner) {
            let actions = crate::content::powers::resolve_power_on_card_played(
                power.power_type,
                combat,
                owner,
                card,
                power.amount,
            );
            for action in actions {
                observe_reactive_action(combat, raw, action);
            }
        }
    }
}

fn observe_reactive_action(combat: &CombatState, raw: &mut RawPowerEffects, action: Action) {
    observe_power_action(raw, action.clone());
    match action {
        Action::Damage(info)
        | Action::PummelDamage(info)
        | Action::BaneDamage(info)
        | Action::WallopDamage(info)
        | Action::DamagePerAttackPlayed(info)
        | Action::DropkickDamageAndEffect {
            damage_info: info, ..
        }
        | Action::Ftl {
            damage_info: info, ..
        }
        | Action::Skewer {
            damage_info: info, ..
        }
        | Action::VampireDamage(info)
        | Action::Barrage { damage: info }
        | Action::Sunder {
            damage_info: info, ..
        }
        | Action::FearNoEvil {
            damage_info: info, ..
        }
        | Action::FiendFire {
            damage_info: info, ..
        }
        | Action::Feed {
            damage_info: info, ..
        }
        | Action::LessonLearned {
            damage_info: info, ..
        }
        | Action::HandOfGreed {
            damage_info: info, ..
        }
        | Action::RitualDagger {
            damage_info: info, ..
        } => observe_reactive_damage_info(combat, raw, info.target, info.output.max(info.base)),
        Action::LoseHp { target, amount, .. } | Action::PoisonLoseHp { target, amount } => {
            observe_reactive_hp_loss(combat, raw, target, amount)
        }
        Action::DamageAllEnemies { damages, .. }
        | Action::VampireDamageAllEnemies { damages, .. } => {
            for (slot, damage) in damages.into_iter().enumerate() {
                if let Some(monster) = combat.entities.monsters.get(slot) {
                    observe_reactive_hp_loss(combat, raw, monster.id, damage);
                }
            }
        }
        Action::GainBlock { target, amount } if target == 0 => {
            raw.reactive_player_block = raw.reactive_player_block.saturating_add(amount.max(0));
        }
        Action::MakeTempCardInDrawPile {
            card_id, amount, ..
        } => {
            if generated_card_is_bad_draw(card_id) {
                raw.reactive_bad_draw_cards = raw
                    .reactive_bad_draw_cards
                    .saturating_add(i32::from(amount));
            }
        }
        Action::TriggerTimeWarpEndTurn { .. } => {
            raw.reactive_forced_turn_end = true;
        }
        _ => {}
    }
}

fn observe_reactive_damage_info(
    combat: &CombatState,
    raw: &mut RawPowerEffects,
    target: usize,
    amount: i32,
) {
    observe_reactive_hp_loss(combat, raw, target, amount);
}

fn observe_reactive_hp_loss(
    combat: &CombatState,
    raw: &mut RawPowerEffects,
    target: usize,
    amount: i32,
) {
    let amount = amount.max(0);
    if amount == 0 {
        return;
    }
    if target == 0 {
        raw.reactive_player_hp_loss = raw.reactive_player_hp_loss.saturating_add(amount);
    } else if is_living_monster_id(combat, target) {
        raw.reactive_enemy_damage = raw.reactive_enemy_damage.saturating_add(amount);
    }
}

fn generated_card_is_bad_draw(card_id: crate::content::cards::CardId) -> bool {
    let def = crate::content::cards::get_card_definition(card_id);
    matches!(
        def.card_type,
        crate::content::cards::CardType::Status | crate::content::cards::CardType::Curse
    )
}
