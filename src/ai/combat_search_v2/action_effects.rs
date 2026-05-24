use super::*;
use crate::content::powers::store::powers_snapshot_for;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatCard;
use crate::sim::combat_projection::project_monster_move_preview_in_combat;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PlayCardEffectSummary {
    pub(super) persistent_enemy_strength_down: i32,
    pub(super) temporary_enemy_strength_down: i32,
    pub(super) visible_attack_mitigation_hint: i32,
    pub(super) enemy_strength_gain: i32,
    pub(super) visible_attack_pressure_hint: i32,
    pub(super) reactive_player_hp_loss: i32,
    pub(super) reactive_player_block: i32,
    pub(super) reactive_enemy_damage: i32,
    pub(super) reactive_bad_draw_cards: i32,
    pub(super) reactive_forced_turn_end: bool,
    pub(super) enemy_weak: i32,
    pub(super) enemy_vulnerable: i32,
}

impl PlayCardEffectSummary {
    pub(super) fn mitigation_ordering_score(self) -> i32 {
        self.persistent_enemy_strength_down
            .saturating_add(self.temporary_enemy_strength_down)
            .saturating_add(self.visible_attack_mitigation_hint)
    }

    pub(super) fn enemy_scaling_risk_score(self) -> i32 {
        self.enemy_strength_gain
            .saturating_add(self.visible_attack_pressure_hint)
    }

    pub(super) fn reactive_risk_score(self) -> i32 {
        self.enemy_scaling_risk_score()
            .saturating_add(self.reactive_player_hp_loss)
            .saturating_add(self.reactive_bad_draw_cards)
            .saturating_add(i32::from(self.reactive_forced_turn_end))
    }

    pub(super) fn net_mitigation_ordering_score(self) -> i32 {
        self.mitigation_ordering_score()
            .saturating_sub(self.reactive_risk_score())
    }
}

pub(super) fn summarize_play_card_effects(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> PlayCardEffectSummary {
    let actions = crate::content::cards::resolve_card_play_with_context(
        card.id,
        combat,
        card,
        target,
        crate::content::cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut raw = RawPowerEffects::default();

    for info in actions {
        observe_power_action(&mut raw, info.action);
    }
    observe_card_play_reactive_power_actions(combat, card, &mut raw);

    summarize_power_effects(combat, raw)
}

pub(super) fn state_sustained_mitigation_score(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            let strength = combat.get_power(monster.id, PowerId::Strength);
            if strength >= 0 {
                return 0;
            }
            (-strength).saturating_mul(monster_attack_relevance(combat, monster.id))
        })
        .sum()
}

#[derive(Default)]
struct RawPowerEffects {
    enemy_strength_down_by_target: BTreeMap<usize, i32>,
    enemy_strength_gain_by_target: BTreeMap<usize, i32>,
    shackled_targets: BTreeSet<usize>,
    reactive_player_hp_loss: i32,
    reactive_player_block: i32,
    reactive_enemy_damage: i32,
    reactive_bad_draw_cards: i32,
    reactive_forced_turn_end: bool,
    enemy_weak: i32,
    enemy_vulnerable: i32,
}

fn observe_power_action(raw: &mut RawPowerEffects, action: Action) {
    match action {
        Action::ApplyPower {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerDetailed {
            target,
            power_id,
            amount,
            ..
        }
        | Action::ApplyPowerWithPayload {
            target,
            power_id,
            amount,
            ..
        } => observe_apply_power(raw, target, power_id, amount),
        _ => {}
    }
}

fn observe_apply_power(raw: &mut RawPowerEffects, target: usize, power_id: PowerId, amount: i32) {
    match power_id {
        PowerId::Strength if amount < 0 => {
            *raw.enemy_strength_down_by_target.entry(target).or_default() += -amount;
        }
        PowerId::Strength if amount > 0 => {
            *raw.enemy_strength_gain_by_target.entry(target).or_default() += amount;
        }
        PowerId::Shackled if amount > 0 => {
            raw.shackled_targets.insert(target);
        }
        PowerId::Weak if amount > 0 => {
            raw.enemy_weak = raw.enemy_weak.saturating_add(amount);
        }
        PowerId::Vulnerable if amount > 0 => {
            raw.enemy_vulnerable = raw.enemy_vulnerable.saturating_add(amount);
        }
        _ => {}
    }
}

fn observe_card_play_reactive_power_actions(
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

fn summarize_power_effects(combat: &CombatState, raw: RawPowerEffects) -> PlayCardEffectSummary {
    let mut summary = PlayCardEffectSummary {
        reactive_player_hp_loss: raw.reactive_player_hp_loss,
        reactive_player_block: raw.reactive_player_block,
        reactive_enemy_damage: raw.reactive_enemy_damage,
        reactive_bad_draw_cards: raw.reactive_bad_draw_cards,
        reactive_forced_turn_end: raw.reactive_forced_turn_end,
        enemy_weak: raw.enemy_weak,
        enemy_vulnerable: raw.enemy_vulnerable,
        ..PlayCardEffectSummary::default()
    };

    for (target, amount) in raw.enemy_strength_down_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        if raw.shackled_targets.contains(&target) {
            summary.temporary_enemy_strength_down = summary
                .temporary_enemy_strength_down
                .saturating_add(weighted_amount);
        } else {
            summary.persistent_enemy_strength_down = summary
                .persistent_enemy_strength_down
                .saturating_add(weighted_amount);
        }
        summary.visible_attack_mitigation_hint =
            summary.visible_attack_mitigation_hint.saturating_add(
                visible_strength_down_mitigation_hint(combat, target, amount),
            );
    }
    for (target, amount) in raw.enemy_strength_gain_by_target {
        if !is_living_monster_id(combat, target) {
            continue;
        }
        let weighted_amount = amount.saturating_mul(monster_attack_relevance(combat, target));
        summary.enemy_strength_gain = summary.enemy_strength_gain.saturating_add(weighted_amount);
        summary.visible_attack_pressure_hint = summary
            .visible_attack_pressure_hint
            .saturating_add(visible_strength_gain_pressure_hint(combat, target, amount));
    }

    summary
}

fn visible_strength_down_mitigation_hint(
    combat: &CombatState,
    target: usize,
    strength_down: i32,
) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    let Some(damage_per_hit) = preview.damage_per_hit else {
        return 0;
    };
    let per_hit = strength_down.min(damage_per_hit).max(0);
    per_hit.saturating_mul(preview.hits.max(1) as i32)
}

fn visible_strength_gain_pressure_hint(
    combat: &CombatState,
    target: usize,
    strength_gain: i32,
) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    if preview.damage_per_hit.is_none() {
        return 0;
    }
    strength_gain
        .max(0)
        .saturating_mul(preview.hits.max(1) as i32)
}

fn monster_attack_relevance(combat: &CombatState, target: usize) -> i32 {
    let Some(monster) = combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target && monster.is_alive_for_action())
    else {
        return 0;
    };
    let preview = project_monster_move_preview_in_combat(combat, monster);
    if preview.hits > 0 {
        preview.hits as i32
    } else {
        1
    }
}

fn is_living_monster_id(combat: &CombatState, target: usize) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .any(|monster| monster.id == target && monster.is_alive_for_action())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn disarm_reports_persistent_enemy_strength_down_without_card_id_special_case() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.id = 1;
        guardian.set_planned_move_id(6);
        combat.entities.monsters = vec![guardian];
        let disarm = CombatCard::new(CardId::Disarm, 10);

        let summary = summarize_play_card_effects(&combat, &disarm, Some(1));

        assert!(summary.persistent_enemy_strength_down > 0);
        assert_eq!(summary.temporary_enemy_strength_down, 0);
    }

    #[test]
    fn state_mitigation_score_counts_negative_enemy_strength() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::Cultist);
        monster.id = 1;
        combat.entities.monsters = vec![monster];
        combat.entities.power_db.insert(
            1,
            vec![Power {
                power_type: PowerId::Strength,
                instance_id: None,
                amount: -3,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        assert_eq!(state_sustained_mitigation_score(&combat), 3);
    }

    #[test]
    fn anger_power_reports_enemy_strength_gain_for_skill_without_monster_special_case() {
        let mut combat = blank_test_combat();
        let mut nob = test_monster(EnemyId::GremlinNob);
        nob.id = 1;
        combat.entities.monsters = vec![nob];
        insert_power(&mut combat, 1, PowerId::Anger, 2);

        let defend = CombatCard::new(CardId::Defend, 10);
        let strike = CombatCard::new(CardId::Strike, 11);

        let defend_summary = summarize_play_card_effects(&combat, &defend, None);
        let strike_summary = summarize_play_card_effects(&combat, &strike, Some(1));

        assert!(defend_summary.enemy_strength_gain > 0);
        assert!(defend_summary.enemy_scaling_risk_score() > 0);
        assert_eq!(strike_summary.enemy_strength_gain, 0);
    }

    #[test]
    fn sharp_hide_reports_reactive_player_hp_loss_for_attack() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.id = 1;
        combat.entities.monsters = vec![guardian];
        insert_power(&mut combat, 1, PowerId::SharpHide, 3);

        let strike = CombatCard::new(CardId::Strike, 10);
        let defend = CombatCard::new(CardId::Defend, 11);

        let strike_summary = summarize_play_card_effects(&combat, &strike, Some(1));
        let defend_summary = summarize_play_card_effects(&combat, &defend, None);

        assert_eq!(strike_summary.reactive_player_hp_loss, 3);
        assert_eq!(defend_summary.reactive_player_hp_loss, 0);
    }

    #[test]
    fn after_image_reports_reactive_player_block() {
        let mut combat = blank_test_combat();
        insert_power(&mut combat, 0, PowerId::AfterImage, 1);
        let strike = CombatCard::new(CardId::Strike, 10);

        let summary = summarize_play_card_effects(&combat, &strike, Some(1));

        assert_eq!(summary.reactive_player_block, 1);
    }

    #[test]
    fn hex_reports_bad_draw_cards_for_non_attack() {
        let mut combat = blank_test_combat();
        let mut chosen = test_monster(EnemyId::Chosen);
        chosen.id = 1;
        combat.entities.monsters = vec![chosen];
        insert_power(&mut combat, 1, PowerId::Hex, 1);

        let defend = CombatCard::new(CardId::Defend, 10);
        let strike = CombatCard::new(CardId::Strike, 11);

        let defend_summary = summarize_play_card_effects(&combat, &defend, None);
        let strike_summary = summarize_play_card_effects(&combat, &strike, Some(1));

        assert_eq!(defend_summary.reactive_bad_draw_cards, 1);
        assert_eq!(strike_summary.reactive_bad_draw_cards, 0);
    }

    #[test]
    fn time_warp_reports_forced_turn_end() {
        let mut combat = blank_test_combat();
        let mut eater = test_monster(EnemyId::TimeEater);
        eater.id = 1;
        combat.entities.monsters = vec![eater];
        insert_power(&mut combat, 1, PowerId::TimeWarp, 11);
        let strike = CombatCard::new(CardId::Strike, 10);

        let summary = summarize_play_card_effects(&combat, &strike, Some(1));

        assert!(summary.reactive_forced_turn_end);
    }

    fn insert_power(combat: &mut CombatState, owner: usize, power_type: PowerId, amount: i32) {
        combat.entities.power_db.insert(
            owner,
            vec![Power {
                power_type,
                instance_id: None,
                amount,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
    }
}
