use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::powers::PowerId;
use crate::core::EntityId;
use smallvec::{smallvec, SmallVec};

pub fn play_colorless(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let dmg = card.base_damage_mut;
    let mag = card.base_magic_num_mut;
    let mut acts: SmallVec<[Action; 4]> = smallvec![];

    match card.id {
        CardId::Bite => {
            let target = target.expect("Bite requires a valid target!");
            let evaluated =
                crate::content::cards::evaluate_card_for_play(card, state, Some(target));
            let def = get_card_definition(card.id);
            return smallvec![
                ActionInfo {
                    action: Action::Damage(DamageInfo {
                        source: 0,
                        target,
                        base: evaluated.base_damage_mut,
                        output: evaluated.base_damage_mut,
                        damage_type: DamageType::Normal,
                        is_modified: evaluated.base_damage_mut != def.base_damage,
                    }),
                    insertion_mode: AddTo::Bottom,
                },
                ActionInfo {
                    action: Action::Heal {
                        target: 0,
                        amount: evaluated.base_magic_num_mut,
                    },
                    insertion_mode: AddTo::Bottom,
                }
            ];
        }
        CardId::BandageUp => {
            acts.push(Action::Heal {
                target: 0,
                amount: mag,
            });
        }
        CardId::Finesse => {
            acts.push(Action::GainBlock {
                target: 0,
                amount: card.base_block_mut,
            });
            acts.push(Action::DrawCards(1));
        }
        CardId::Blind => {
            if card.upgrades > 0 {
                for monster in
                    state.entities.monsters.iter().filter(|m| {
                        m.current_hp > 0 && !m.is_dying && !m.is_escaped && !m.half_dead
                    })
                {
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: monster.id,
                        power_id: PowerId::Weak,
                        amount: mag,
                    });
                }
            } else {
                let target_id = target.expect("Blind requires a target!");
                acts.push(Action::ApplyPower {
                    source: 0,
                    target: target_id,
                    power_id: PowerId::Weak,
                    amount: mag,
                });
            }
        }
        CardId::DarkShackles => {
            let target_id = target.expect("Dark Shackles requires a target!");
            acts.push(Action::ApplyPower {
                source: 0,
                target: target_id,
                power_id: PowerId::Strength,
                amount: -mag,
            });
            if !crate::content::powers::store::has_power(state, target_id, PowerId::Artifact) {
                acts.push(Action::ApplyPower {
                    source: 0,
                    target: target_id,
                    power_id: PowerId::Shackled,
                    amount: mag,
                });
            }
        }
        CardId::DeepBreath => {
            if !state.zones.discard_pile.is_empty() {
                acts.push(Action::ShuffleDiscardIntoDraw);
            }
            acts.push(Action::DrawCards(mag as u32));
        }
        CardId::Discovery => {
            acts.push(Action::SuspendForDiscovery {
                colorless: false,
                card_type: None,
                cost_for_turn: Some(0),
            });
        }
        CardId::DramaticEntrance => {
            let damages: smallvec::SmallVec<[i32; 5]> =
                state.entities.monsters.iter().map(|_| dmg).collect();
            acts.push(Action::DamageAllEnemies {
                source: 0,
                damages,
                damage_type: DamageType::Normal,
                is_modified: false,
            });
        }
        CardId::Enlightenment => {
            acts.push(Action::Enlightenment {
                permanent: card.upgrades > 0,
            });
        }
        CardId::FlashOfSteel => {
            let target_id = target.expect("Flash of Steel requires a target!");
            acts.push(Action::Damage(DamageInfo {
                source: 0,
                target: target_id,
                base: dmg,
                output: dmg,
                damage_type: DamageType::Normal,
                is_modified: false,
            }));
            acts.push(Action::DrawCards(1));
        }
        CardId::Forethought => {
            if !state.zones.hand.is_empty() {
                acts.push(Action::SuspendForHandSelect {
                    min: if card.upgrades > 0 { 0 } else { 1 },
                    max: if card.upgrades > 0 { 99 } else { 1 },
                    can_cancel: card.upgrades > 0,
                    filter: crate::state::HandSelectFilter::Any,
                    reason: crate::state::HandSelectReason::PutToBottomOfDraw,
                });
            }
        }
        CardId::GoodInstincts => {
            acts.push(Action::GainBlock {
                target: 0,
                amount: card.base_block_mut,
            });
        }
        CardId::Impatience => {
            let has_attack = state
                .zones
                .hand
                .iter()
                .any(|c| get_card_definition(c.id).card_type == CardType::Attack);
            if !has_attack {
                acts.push(Action::DrawCards(mag as u32));
            }
        }
        CardId::JackOfAllTrades => {
            for _ in 0..card.base_magic_num_mut.max(1) {
                acts.push(Action::MakeRandomColorlessCardInHand {
                    cost_for_turn: None,
                    upgraded: false,
                });
            }
        }
        CardId::MindBlast => {
            let target_id = target.expect("Mind Blast requires a target!");
            acts.push(Action::Damage(DamageInfo {
                source: 0,
                target: target_id,
                base: state.zones.draw_pile.len() as i32,
                output: dmg,
                damage_type: DamageType::Normal,
                is_modified: dmg != state.zones.draw_pile.len() as i32,
            }));
        }
        CardId::Panacea => {
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Artifact,
                amount: mag,
            });
        }
        CardId::PanicButton => {
            acts.push(Action::GainBlock {
                target: 0,
                amount: card.base_block_mut,
            });
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::NoBlock,
                amount: card.base_magic_num_mut,
            });
        }
        CardId::Purity => {
            if !state.zones.hand.is_empty() {
                acts.push(Action::SuspendForHandSelect {
                    min: 0,
                    max: mag as u8,
                    can_cancel: true,
                    filter: crate::state::HandSelectFilter::Any,
                    reason: crate::state::HandSelectReason::Exhaust,
                });
            }
        }
        CardId::SwiftStrike => {
            let target_id = target.expect("Swift Strike requires a target!");
            acts.push(Action::Damage(DamageInfo {
                source: 0,
                target: target_id,
                base: dmg,
                output: dmg,
                damage_type: DamageType::Normal,
                is_modified: false,
            }));
        }
        CardId::Trip => {
            if card.upgrades > 0 {
                for monster in
                    state.entities.monsters.iter().filter(|m| {
                        m.current_hp > 0 && !m.is_dying && !m.is_escaped && !m.half_dead
                    })
                {
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: monster.id,
                        power_id: PowerId::Vulnerable,
                        amount: mag,
                    });
                }
            } else {
                let target_id = target.expect("Trip requires a target!");
                acts.push(Action::ApplyPower {
                    source: 0,
                    target: target_id,
                    power_id: PowerId::Vulnerable,
                    amount: mag,
                });
            }
        }
        CardId::JAX => {
            acts.push(Action::LoseHp {
                target: 0,
                amount: 3,
                triggers_rupture: true,
            });
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Strength,
                amount: mag,
            });
        }
        CardId::Apotheosis => acts.push(Action::UpgradeAllInHand),
        CardId::Chrysalis => {
            for _ in 0..mag {
                acts.push(Action::MakeRandomCardInDrawPile {
                    card_type: Some(CardType::Skill),
                    cost_for_turn: Some(0),
                    random_spot: true,
                });
            }
        }
        CardId::HandOfGreed => {
            let target_id = target.expect("Hand of Greed requires a target!");
            acts.push(Action::HandOfGreed {
                target: target_id,
                damage_info: DamageInfo {
                    source: 0,
                    target: target_id,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                },
                gold_amount: mag,
            });
        }
        CardId::RitualDagger => {
            let target_id = target.expect("Ritual Dagger requires a target!");
            acts.push(Action::RitualDagger {
                target: target_id,
                damage_info: DamageInfo {
                    source: 0,
                    target: target_id,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                },
                misc_amount: mag,
                card_uuid: card.uuid,
            });
        }
        CardId::Magnetism => {
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::MagnetismPower,
                amount: 1,
            });
        }
        CardId::MasterOfStrategy => acts.push(Action::DrawCards(mag as u32)),
        CardId::Mayhem => {
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::MayhemPower,
                amount: 1,
            });
        }
        CardId::Metamorphosis => {
            for _ in 0..mag {
                acts.push(Action::MakeRandomCardInDrawPile {
                    card_type: Some(CardType::Attack),
                    cost_for_turn: Some(0),
                    random_spot: true,
                });
            }
        }
        CardId::Panache => {
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::PanachePower,
                amount: mag,
            });
        }
        CardId::SadisticNature => {
            acts.push(Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::SadisticPower,
                amount: mag,
            });
        }
        CardId::SecretTechnique => {
            let skills: Vec<_> = state
                .zones
                .draw_pile
                .iter()
                .filter(|c| get_card_definition(c.id).card_type == CardType::Skill)
                .map(|c| c.uuid)
                .collect();
            if skills.len() == 1 {
                acts.push(Action::MoveCard {
                    card_uuid: skills[0],
                    from: crate::state::PileType::Draw,
                    to: crate::state::PileType::Hand,
                });
            } else if !skills.is_empty() {
                acts.push(Action::SuspendForGridSelect {
                    source_pile: crate::state::PileType::Draw,
                    min: 1,
                    max: 1,
                    can_cancel: false,
                    filter: crate::state::GridSelectFilter::Skill,
                    reason: crate::state::GridSelectReason::SkillFromDeckToHand,
                });
            }
        }
        CardId::SecretWeapon => {
            let attacks: Vec<_> = state
                .zones
                .draw_pile
                .iter()
                .filter(|c| get_card_definition(c.id).card_type == CardType::Attack)
                .map(|c| c.uuid)
                .collect();
            if attacks.len() == 1 {
                acts.push(Action::MoveCard {
                    card_uuid: attacks[0],
                    from: crate::state::PileType::Draw,
                    to: crate::state::PileType::Hand,
                });
            } else if !attacks.is_empty() {
                acts.push(Action::SuspendForGridSelect {
                    source_pile: crate::state::PileType::Draw,
                    min: 1,
                    max: 1,
                    can_cancel: false,
                    filter: crate::state::GridSelectFilter::Attack,
                    reason: crate::state::GridSelectReason::AttackFromDeckToHand,
                });
            }
        }
        CardId::TheBomb => {
            acts.push(Action::ApplyPowerDetailed {
                source: 0,
                target: 0,
                power_id: PowerId::TheBombPower,
                amount: 3,
                instance_id: Some(card.uuid),
                extra_data: Some(mag),
            });
        }
        CardId::ThinkingAhead => {
            acts.push(Action::DrawCards(2));
            if !state.zones.hand.is_empty()
                || !state.zones.draw_pile.is_empty()
                || !state.zones.discard_pile.is_empty()
            {
                acts.push(Action::SuspendForHandSelect {
                    min: 1,
                    max: 1,
                    can_cancel: false,
                    filter: crate::state::HandSelectFilter::Any,
                    reason: crate::state::HandSelectReason::PutOnDrawPile,
                });
            }
        }
        CardId::Transmutation => {
            for _ in 0..card.energy_on_use.max(0) {
                acts.push(Action::MakeRandomColorlessCardInHand {
                    cost_for_turn: Some(0),
                    upgraded: card.upgrades > 0,
                });
            }
        }
        CardId::Violence => {
            acts.push(Action::DrawPileToHandByType {
                amount: mag as u8,
                card_type: CardType::Attack,
            });
        }
        _ => {}
    }

    acts.into_iter()
        .map(|action| ActionInfo {
            action,
            insertion_mode: AddTo::Bottom,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn finesse_grants_block_and_draws() {
        let mut card = CombatCard::new(CardId::Finesse, 1);
        let combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        crate::content::cards::evaluate_card(&mut card, &combat, None);

        let actions = play_colorless(&combat, &card, None);

        assert_eq!(actions.len(), 2);
        assert!(matches!(
            actions[0].action,
            Action::GainBlock {
                target: 0,
                amount: 2
            }
        ));
        assert!(matches!(actions[1].action, Action::DrawCards(1)));
    }

    #[test]
    fn jax_loses_hp_and_gains_strength() {
        let mut card = CombatCard::new(CardId::JAX, 2);
        let combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        crate::content::cards::evaluate_card(&mut card, &combat, None);

        let actions = play_colorless(&combat, &card, None);

        assert_eq!(actions.len(), 2);
        assert!(matches!(
            actions[0].action,
            Action::LoseHp {
                target: 0,
                amount: 3,
                triggers_rupture: true
            }
        ));
        assert!(matches!(
            actions[1].action,
            Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Strength,
                amount: 2
            }
        ));
    }

    #[test]
    fn ritual_dagger_kill_increases_misc() {
        let mut combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        let mut card = CombatCard::new(CardId::RitualDagger, 42);
        card.misc_value = 15;
        crate::content::cards::evaluate_card(&mut card, &combat, Some(1));
        combat.zones.limbo.push(card.clone());

        let actions = play_colorless(&combat, &card, Some(1));
        assert_eq!(actions.len(), 1);
        let action = actions.into_iter().next().unwrap().action;
        crate::engine::action_handlers::execute_action(action, &mut combat);
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        let ritual = combat
            .zones
            .limbo
            .iter()
            .find(|c| c.uuid == 42)
            .expect("ritual dagger should still be in limbo before use cleanup");
        assert_eq!(ritual.misc_value, 18);
    }

    #[test]
    fn hand_of_greed_kill_grants_gold() {
        let mut combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        let mut card = CombatCard::new(CardId::HandOfGreed, 7);
        crate::content::cards::evaluate_card(&mut card, &combat, Some(1));

        let actions = play_colorless(&combat, &card, Some(1));
        assert_eq!(actions.len(), 1);
        let action = actions.into_iter().next().unwrap().action;
        crate::engine::action_handlers::execute_action(action, &mut combat);
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(combat.entities.player.gold_delta_this_combat, 20);
        assert_eq!(combat.entities.player.gold, 119);
    }

    #[test]
    fn hand_of_greed_gold_triggers_bloody_idol_heal() {
        let mut combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        combat.entities.player.current_hp = 60;
        combat
            .entities
            .player
            .add_relic(crate::content::relics::RelicState::new(
                crate::content::relics::RelicId::BloodyIdol,
            ));
        let mut card = CombatCard::new(CardId::HandOfGreed, 8);
        crate::content::cards::evaluate_card(&mut card, &combat, Some(1));

        let action = play_colorless(&combat, &card, Some(1))
            .into_iter()
            .next()
            .unwrap()
            .action;
        crate::engine::action_handlers::execute_action(action, &mut combat);
        while let Some(action) = combat.engine.action_queue.pop_front() {
            crate::engine::action_handlers::execute_action(action, &mut combat);
        }

        assert_eq!(combat.entities.player.gold_delta_this_combat, 20);
        assert_eq!(combat.entities.player.current_hp, 65);
    }

    #[test]
    fn bite_uses_evaluated_player_damage_against_vulnerable_target() {
        let mut combat = basic_combat()
            .with_monster_max_hp(1, 10)
            .with_monster_hp(1, 10);
        combat.entities.power_db.insert(
            0,
            vec![crate::combat::Power {
                power_type: PowerId::Weak,
                instance_id: None,
                amount: 3,
                extra_data: 0,
                just_applied: false,
            }],
        );
        combat.entities.power_db.insert(
            1,
            vec![crate::combat::Power {
                power_type: PowerId::Vulnerable,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                just_applied: false,
            }],
        );

        let mut card = CombatCard::new(CardId::Bite, 1);
        crate::content::cards::evaluate_card(&mut card, &combat, Some(1));
        assert_eq!(card.base_damage_mut, 7);

        let actions = play_colorless(&combat, &card, Some(1));
        assert_eq!(actions.len(), 2);
        match &actions[0].action {
            Action::Damage(info) => {
                assert_eq!(info.source, 0);
                assert_eq!(info.base, 7);
                assert_eq!(info.output, 7);
            }
            other => panic!("expected Bite damage action, got {other:?}"),
        }
        assert!(matches!(
            actions[1].action,
            Action::Heal {
                target: 0,
                amount: 2
            }
        ));
    }
}
