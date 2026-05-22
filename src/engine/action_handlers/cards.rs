// action_handlers/cards.rs — Card pile management domain
//
// Handles: DrawCards, EmptyDeckShuffle, DiscardCard, ExhaustCard, MoveCard, PutOnDeck,
//          MakeTempCard*, MakeCopy*, MakeRandom*, PlayCardDirect, PlayTopCard,
//          UseCardDone, UpgradeCard, UpgradeRandomCard, UpgradeAllInHand, UpgradeAllBurns,
//          ReduceAllHandCosts, RandomizeHandCosts, ModifyCardMisc,
//          UsePotion, DiscardPotion, ObtainPotion, ObtainSpecificPotion, Scry,
//          EndTurnTrigger, StartTurnTrigger, PostDrawTrigger, BattleStartTrigger, ClearCardQueue,
//          AddCardToMasterDeck, MakeTempCardInDiscardAndDeck, SuspendForCardReward

use crate::content::cards::CardId;
use crate::runtime::action::Action;

mod discard;
mod draw;
mod exhaust;
mod generated;
mod movement;
mod mutation;
mod pile_ops;
mod play_queue;
mod potions;
mod turn_triggers;
pub use discard::{
    handle_calculated_gamble, handle_discard_card, handle_discard_card_with_order,
    handle_discard_from_hand, handle_discard_to_hand, handle_scrape_follow_up, DiscardHookOrder,
};
pub use draw::{
    handle_draw_cards, handle_draw_cards_with_history, handle_draw_for_unique_orb_types,
};
pub use exhaust::{
    handle_exhaust_card, handle_exhaust_from_hand, handle_recycle, handle_recycle_selected_card,
    move_card_to_exhaust_pile,
};
#[cfg(test)]
pub(crate) use generated::class_card_pool_for_type;
pub use generated::{
    handle_conjure_blade, handle_make_constructed_copy_in_hand, handle_make_copy_in_discard,
    handle_make_copy_in_draw_pile, handle_make_copy_in_hand, handle_make_random_card_in_draw_pile,
    handle_make_random_card_in_hand, handle_make_random_colorless_card_in_hand,
    handle_make_temp_card_in_discard, handle_make_temp_card_in_discard_and_deck,
    handle_make_temp_card_in_draw_pile, handle_make_temp_card_in_hand, handle_nightmare,
    handle_return_stasis_card, handle_transmutation, queue_nightmare_power_front,
};
pub use movement::{
    handle_all_cost_to_hand, handle_discard_pile_to_top_of_deck, handle_draw_pile_to_hand_by_type,
    handle_exhume_card, handle_meditate, handle_move_card, handle_remove_card_from_pile,
};
pub use mutation::{
    handle_apply_bullet_time, handle_enlightenment, handle_gash, handle_madness,
    handle_modify_card_block, handle_modify_card_damage, handle_modify_card_misc,
    handle_randomize_hand_costs, handle_reduce_all_hand_costs, handle_reduce_card_cost_for_combat,
    handle_reduce_retained_hand_costs, handle_upgrade_all_burns,
    handle_upgrade_all_cards_in_combat, handle_upgrade_all_in_hand, handle_upgrade_card,
    handle_upgrade_random_card,
};
pub use pile_ops::{
    handle_empty_deck_shuffle, handle_forethought, handle_put_on_deck,
    handle_shuffle_all_into_draw, handle_shuffle_discard_into_draw, handle_shuffle_draw_pile,
};
pub use play_queue::{
    handle_enqueue_card_play, handle_flush_next_queued_card, handle_play_card_direct,
    handle_play_card_from_hand, handle_play_top_card, handle_queue_early_end_turn,
    handle_queue_play_top_card_to_bottom, handle_retain_non_ethereal_hand_cards,
    handle_skip_enemies_turn, handle_use_card_after_use_hooks, handle_use_card_done,
};
pub use potions::{handle_obtain_potion, handle_use_potion, obtain_specific_potion_if_allowed};
pub use turn_triggers::{
    handle_add_card_to_master_deck, handle_battle_start_pre_draw_trigger,
    handle_battle_start_trigger, handle_clear_card_queue, handle_end_turn_trigger,
    handle_post_draw_trigger, handle_pre_battle_trigger,
};

use crate::runtime::combat::CombatState;

pub fn handle_barrage(damage: crate::runtime::action::DamageInfo, state: &mut CombatState) {
    let count = state
        .entities
        .player
        .orbs
        .iter()
        .filter(|orb| orb.id != crate::runtime::combat::OrbId::Empty)
        .count();

    for _ in 0..count {
        state.queue_action_front(Action::Damage(damage.clone()));
    }
}

pub fn handle_escape_plan_block_if_skill(block: i32, state: &mut CombatState) {
    if state.runtime.last_drawn_cards.iter().any(|record| {
        crate::content::cards::get_card_definition(record.card_id).card_type
            == crate::content::cards::CardType::Skill
    }) {
        state.queue_action_front(Action::GainBlock {
            target: 0,
            amount: block,
        });
    }
}

pub fn handle_blade_fury(upgraded: bool, state: &mut CombatState) {
    let count = state.zones.hand.len() as u8;
    state.queue_action_front(
        crate::content::cards::make_constructed_temp_card_in_hand_action(
            CardId::Shiv,
            count,
            upgraded,
            state,
        ),
    );
    state.queue_action_front(Action::DiscardFromHand {
        amount: count as i32,
        random: false,
        end_turn: false,
    });
}

pub fn handle_unload_non_attack(state: &mut CombatState) {
    let non_attacks: Vec<u32> = state
        .zones
        .hand
        .iter()
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id).card_type
                != crate::content::cards::CardType::Attack
        })
        .map(|card| card.uuid)
        .collect();

    for uuid in non_attacks {
        state.queue_action_front(Action::DiscardCard { card_uuid: uuid });
    }
}

pub fn handle_expertise_draw(target_hand_size: i32, state: &mut CombatState) {
    let to_draw = target_hand_size - state.zones.hand.len() as i32;
    if to_draw > 0 {
        state.queue_action_front(Action::DrawCards(to_draw as u32));
    }
}

pub fn handle_halt(block: i32, additional: i32, state: &mut CombatState) {
    let amount = if state.entities.player.stance == crate::runtime::combat::StanceId::Wrath {
        block + additional
    } else {
        block
    };
    state.queue_action_front(Action::GainBlock { target: 0, amount });
}

pub fn handle_aggregate_energy(divide_amount: i32, state: &mut CombatState) {
    if divide_amount <= 0 {
        return;
    }
    let amount = state.zones.draw_pile.len() as i32 / divide_amount;
    if amount > 0 {
        state.turn.adjust_energy(amount);
    }
}

pub fn handle_tempest(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }

    if effect > 0 {
        for _ in 0..effect {
            state.queue_action_back(Action::ChannelOrb(crate::runtime::combat::OrbId::Lightning));
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_multicast(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    if !state
        .entities
        .player
        .orbs
        .first()
        .is_some_and(|orb| orb.id != crate::runtime::combat::OrbId::Empty)
    {
        return;
    }

    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }

    if effect > 0 {
        for _ in 0..effect - 1 {
            state.queue_action_back(Action::EvokeOrbWithoutRemoving);
        }
        state.queue_action_back(Action::EvokeOrb);
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_reinforced_body(
    block_amount: i32,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);

    if effect > 0 {
        for _ in 0..effect {
            state.queue_action_back(Action::GainBlock {
                target: 0,
                amount: block_amount,
            });
        }
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        class_card_pool_for_type, handle_discard_pile_to_top_of_deck, handle_draw_cards,
        handle_draw_pile_to_hand_by_type, handle_end_turn_trigger,
        handle_make_constructed_copy_in_hand, handle_make_copy_in_discard,
        handle_make_random_card_in_draw_pile, handle_make_random_card_in_hand,
        handle_make_temp_card_in_discard, handle_make_temp_card_in_discard_and_deck,
        handle_make_temp_card_in_draw_pile, handle_make_temp_card_in_hand, handle_play_card_direct,
        handle_pre_battle_trigger, handle_queue_early_end_turn, handle_randomize_hand_costs,
        handle_return_stasis_card, handle_upgrade_all_burns, handle_upgrade_all_cards_in_combat,
        handle_upgrade_all_in_hand, handle_use_card_done, handle_use_potion,
        obtain_specific_potion_if_allowed,
    };
    use crate::content::cards::{CardId, CardType};
    use crate::content::monsters::EnemyId;
    use crate::content::potions::PotionId;
    use crate::content::powers::store;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::action::Action;
    use crate::runtime::combat::{CombatCard, Power, QueuedCardPlay, QueuedCardSource};
    use crate::runtime::rng::StsRng;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn draw_cards_splits_shuffle_like_java_draw_card_action() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
        ];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 12)];
        state.queue_action_back(Action::GainEnergy { amount: 1 });

        handle_draw_cards(3, &mut state);

        assert!(
            state.zones.hand.is_empty(),
            "Java DrawCardAction does not draw immediately when amount exceeds draw pile; it splits into top-queued actions"
        );
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(2)));
        assert_eq!(state.pop_next_action(), Some(Action::EmptyDeckShuffle));
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(1)));
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainEnergy { amount: 1 }),
            "split draw actions are addToTop-style and must run before previously queued actions"
        );
    }

    #[test]
    fn monster_group_pre_battle_uses_monster_hp_rng_for_louse_curl_up_like_java() {
        let mut state = blank_test_combat();
        state.rng.monster_hp_rng = StsRng::new(41);
        state.rng.misc_rng = StsRng::new(41);
        state.entities.monsters = vec![test_monster(EnemyId::LouseNormal)];

        let mut expected_hp_rng = state.rng.monster_hp_rng.clone();
        let expected_curl_up = expected_hp_rng.random_range(3, 7);

        handle_pre_battle_trigger(&mut state);

        assert_eq!(state.rng.monster_hp_rng, expected_hp_rng);
        assert_eq!(
            state.rng.misc_rng.counter, 0,
            "Java Louse.usePreBattleAction consumes AbstractDungeon.monsterHpRng, not miscRng"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::ApplyPower {
                source: 1,
                target: 1,
                power_id: PowerId::CurlUp,
                amount: expected_curl_up,
            })
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::BattleStartPreDrawTrigger)
        );
    }

    #[test]
    fn draw_cards_caps_amount_to_available_hand_space_before_split() {
        let mut state = blank_test_combat();
        state.zones.hand = (0..9)
            .map(|idx| CombatCard::new(CardId::Defend, 100 + idx))
            .collect();
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 200),
            CombatCard::new(CardId::Strike, 201),
        ];

        handle_draw_cards(5, &mut state);

        assert_eq!(state.zones.hand.len(), 9);
        assert_eq!(state.pop_next_action(), Some(Action::EmptyDeckShuffle));
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(1)));
        assert_eq!(state.pop_next_action(), None);
    }

    #[test]
    fn snecko_oil_randomize_updates_combat_cost_and_turn_cost_like_java() {
        let mut state = blank_test_combat();
        let mut modified = CombatCard::new(CardId::Strike, 10);
        modified.set_combat_and_turn_cost_java(3);
        modified.set_cost_for_turn_java(0);
        while {
            let mut probe = state.rng.card_random_rng.clone();
            probe.random(3) == modified.combat_cost_without_turn_override_java()
        } {
            state.rng.card_random_rng.random(3);
        }
        let mut expected_rng = state.rng.card_random_rng.clone();
        let expected_cost = expected_rng.random(3);
        assert_ne!(
            expected_cost,
            modified.combat_cost_without_turn_override_java()
        );
        state.zones.hand = vec![modified, CombatCard::new(CardId::Whirlwind, 11)];

        handle_randomize_hand_costs(&mut state);

        assert_eq!(
            state.zones.hand[0].combat_cost_without_turn_override_java(),
            expected_cost,
            "Java RandomizeHandCostAction mutates AbstractCard.cost, not only costForTurn"
        );
        assert_eq!(state.zones.hand[0].cost_for_turn_java(), expected_cost);
        assert_eq!(
            state.zones.hand[1].combat_cost_without_turn_override_java(),
            -1,
            "X-cost cards short-circuit before consuming a random cost roll"
        );
        assert_eq!(state.rng.card_random_rng.counter, expected_rng.counter);
    }

    #[test]
    fn obtain_specific_potion_fills_first_empty_slot() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                1,
            )),
            None,
            None,
        ];

        assert!(obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert_eq!(
            state.entities.potions[1].as_ref().map(|p| p.id),
            Some(PotionId::EnergyPotion)
        );
        assert!(state.entities.potions[2].is_none());
    }

    #[test]
    fn obtain_specific_potion_is_blocked_by_sozu() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![None, None, None];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::Sozu));

        assert!(!obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert!(state.entities.potions.iter().all(Option::is_none));
    }

    #[test]
    fn obtain_specific_potion_does_nothing_when_slots_are_full() {
        let mut state = blank_test_combat();
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::BlockPotion,
                2,
            )),
        ];
        let before = state.entities.potions.clone();

        assert!(!obtain_specific_potion_if_allowed(
            &mut state,
            PotionId::EnergyPotion
        ));

        assert_eq!(state.entities.potions, before);
    }

    #[test]
    fn fire_potion_applies_enemy_final_receive_before_damage_action_like_java() {
        let mut state = blank_test_combat();
        let mut nemesis_like = test_monster(EnemyId::JawWorm);
        nemesis_like.id = 1;
        nemesis_like.current_hp = 40;
        state.entities.monsters = vec![nemesis_like];
        crate::content::powers::store::set_powers_for(
            &mut state,
            1,
            vec![Power {
                power_type: PowerId::Intangible,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::FirePotion,
            1,
        ))];

        handle_use_potion(0, Some(1), &mut state);

        let Some(Action::Damage(info)) = state.pop_next_action() else {
            panic!("Fire Potion should queue one DamageAction");
        };
        assert_eq!(info.base, 20);
        assert_eq!(
            info.output, 1,
            "Java FirePotion.use calls DamageInfo.applyEnemyPowersOnly(target), so target IntangiblePower caps the queued THORNS damage before DamageAction runs"
        );
        assert!(info.is_modified);
        assert_eq!(state.entities.potions[0], None);
    }

    #[test]
    fn blood_potion_queues_fixed_use_time_heal_amount_without_minimum_one() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_hp = 1;
        state.entities.player.current_hp = 1;
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::BloodPotion,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        let Some(Action::Heal { target, amount }) = state.pop_next_action() else {
            panic!("Blood Potion should queue a fixed HealAction");
        };
        assert_eq!(target, 0);
        assert_eq!(
            amount, 0,
            "Java BloodPotion computes (int)(maxHealth * potencyPercent) directly and does not apply Fairy Potion's minimum-one revive rule"
        );
        assert_eq!(state.entities.potions[0], None);
    }

    #[test]
    fn blood_potion_heal_amount_is_computed_when_used_not_when_heal_executes() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_hp = 10;
        state.entities.player.current_hp = 1;
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::BloodPotion,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        state.entities.player.max_hp = 100;
        let Some(action) = state.pop_next_action() else {
            panic!("Blood Potion should queue a HealAction");
        };
        crate::engine::action_handlers::execute_action(action, &mut state);

        assert_eq!(
            state.entities.player.current_hp, 3,
            "Java BloodPotion.use computes the HealAction amount before it is queued; later max HP changes do not recalculate the potion heal"
        );
    }

    #[test]
    fn combat_fruit_juice_increases_max_hp_immediately_before_toy_heal_queue() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_hp = 80;
        state.entities.player.current_hp = 10;
        state
            .entities
            .player
            .add_relic(RelicState::new(RelicId::MagicFlower));
        state
            .entities
            .player
            .add_relic(RelicState::new(RelicId::ToyOrnithopter));
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::FruitJuice,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert_eq!(
            state.entities.player.max_hp, 85,
            "Java FruitJuice.use calls increaseMaxHp immediately, not via a later action"
        );
        assert_eq!(
            state.entities.player.current_hp, 18,
            "Java increaseMaxHp heals through combat onPlayerHeal hooks before PotionPopUp calls relic onUsePotion"
        );
        assert!(state.entities.potions[0].is_none());

        let Some(Action::Heal { target, amount }) = state.pop_next_action() else {
            panic!("Toy Ornithopter should queue its Java HealAction after Fruit Juice is used");
        };
        assert_eq!(target, 0);
        assert_eq!(
            amount, 5,
            "Toy Ornithopter queues a fixed HealAction(5); Magic Flower modifies it when that action resolves"
        );
        assert_eq!(
            state.action_queue_len(),
            0,
            "Fruit Juice itself should not leave a queued GainMaxHp action"
        );
    }

    #[test]
    fn entropic_brew_generates_concrete_limited_potions_before_obtain_actions() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::EntropicBrew,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                2,
            )),
            None,
        ];
        let potion_rng_before = state.rng.potion_rng.counter;

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(
            state.action_queue_len(),
            3,
            "Java Entropic Brew queues one ObtainPotionAction per potion slot"
        );
        assert!(
            state.rng.potion_rng.counter >= potion_rng_before + 9,
            "Java Entropic Brew calls returnRandomPotion(true) once per potion slot while the potion is used; each call consumes one rarity roll, discards one initial flat potion roll, then consumes at least one accepted/rejected flat roll"
        );
        while let Some(action) = state.pop_next_action() {
            crate::engine::action_handlers::execute_action(action, &mut state);
        }

        let filled = state
            .entities
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count();
        assert_eq!(
            filled, 3,
            "after Entropic Brew is destroyed, queued concrete potion obtains fill the newly empty slot and existing empty slots"
        );
        assert!(
            state
                .entities
                .potions
                .iter()
                .flatten()
                .all(|potion| potion.id != PotionId::FruitJuice),
            "Java returnRandomPotion(true) excludes Fruit Juice for Entropic Brew"
        );
    }

    #[test]
    fn distilled_chaos_rolls_random_targets_when_potion_is_used() {
        let mut state = blank_test_combat();
        let mut first = test_monster(EnemyId::JawWorm);
        first.id = 11;
        let mut second = test_monster(EnemyId::Cultist);
        second.id = 12;
        state.entities.monsters = vec![first, second];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::DistilledChaosPotion,
            1,
        ))];
        let card_random_before = state.rng.card_random_rng.counter;

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(state.action_queue_len(), 3);
        assert!(
            state.rng.card_random_rng.counter >= card_random_before + 3,
            "Java DistilledChaosPotion calls getRandomMonster once per PlayTopCardAction while the potion is used"
        );
        for _ in 0..3 {
            let Some(Action::PlayTopCard {
                target: Some(target),
                exhaust: false,
            }) = state.pop_next_action()
            else {
                panic!("Distilled Chaos should queue targeted PlayTopCard actions");
            };
            assert!(
                target == 11 || target == 12,
                "queued Java target should be one of the use-time random monster choices"
            );
        }
    }

    #[test]
    fn essence_of_darkness_channels_for_each_orb_slot_and_sacred_bark_potency() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.player.max_orbs = 3;
        state.entities.player.orbs = vec![
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Empty),
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Lightning),
            crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Empty),
        ];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::EssenceOfDarkness,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_none());
        assert_eq!(
            state.action_queue_len(),
            6,
            "Java EssenceOfDarknessAction channels potency Dark orbs for each orb slot"
        );
        while let Some(action) = state.pop_next_action() {
            assert_eq!(
                action,
                Action::ChannelOrb(crate::runtime::combat::OrbId::Dark)
            );
        }
    }

    #[test]
    fn smoke_bomb_is_blocked_by_spire_shield_back_attack_power() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::SpireShield);
        monster.id = 7;
        state.entities.monsters = vec![monster];
        state.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::BackAttack,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::SmokeBomb,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_some());
        assert_eq!(
            state.action_queue_len(),
            0,
            "Java SmokeBomb.canUse returns false when any monster has BackAttack"
        );
    }

    #[test]
    fn smoke_bomb_is_blocked_by_boss_monster_type_even_without_room_flag() {
        let mut state = blank_test_combat();
        state.meta.is_boss_fight = false;
        state.entities.monsters = vec![test_monster(EnemyId::SlimeBoss)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::SmokeBomb,
            1,
        ))];

        handle_use_potion(0, None, &mut state);

        assert!(state.entities.potions[0].is_some());
        assert_eq!(
            state.action_queue_len(),
            0,
            "Java SmokeBomb.canUse walks monsters and blocks EnemyType.BOSS, not only room boss flags"
        );
    }

    #[test]
    fn combat_potion_execution_respects_java_can_use_gate() {
        let mut disabled = blank_test_combat();
        disabled.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        disabled.entities.potions = vec![Some(
            crate::content::potions::Potion::with_affordance_truth(
                PotionId::FirePotion,
                1,
                false,
                true,
                true,
            ),
        )];
        handle_use_potion(0, Some(disabled.entities.monsters[0].id), &mut disabled);
        assert!(disabled.entities.potions[0].is_some());
        assert_eq!(
            disabled.action_queue_len(),
            0,
            "Java PotionPopUp checks potion.canUse before calling use()"
        );

        let mut dead_monsters = blank_test_combat();
        dead_monsters.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        dead_monsters.entities.monsters[0].current_hp = 0;
        dead_monsters.entities.monsters[0].is_dying = true;
        dead_monsters.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::FirePotion,
            2,
        ))];
        handle_use_potion(
            0,
            Some(dead_monsters.entities.monsters[0].id),
            &mut dead_monsters,
        );
        assert!(dead_monsters.entities.potions[0].is_some());
        assert_eq!(
            dead_monsters.action_queue_len(),
            0,
            "Java AbstractPotion.canUse blocks when the room monsters are basically dead"
        );
    }

    #[test]
    fn liquid_memories_auto_move_does_not_drop_cards_when_hand_fills() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::LiquidMemories,
            1,
        ))];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.zones.hand = (0..9)
            .map(|idx| CombatCard::new(CardId::Defend, 100 + idx))
            .collect();
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 201),
            CombatCard::new(CardId::Bash, 202),
        ];

        handle_use_potion(0, None, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(
            state.zones.discard_pile[0].id,
            CardId::Bash,
            "Java BetterDiscardPileToHandAction leaves remaining discard cards in place once hand is full"
        );
        assert_eq!(state.zones.hand[9].id, CardId::Strike);
        assert_eq!(state.zones.hand[9].cost_for_turn_java(), 0);
    }

    #[test]
    fn liquid_memories_sacred_bark_grid_select_requires_exact_potency() {
        let mut state = blank_test_combat();
        state.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        state.entities.potions = vec![Some(crate::content::potions::Potion::new(
            PotionId::LiquidMemories,
            1,
        ))];
        state
            .entities
            .player
            .relics
            .push(RelicState::new(RelicId::SacredBark));
        state.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 201),
            CombatCard::new(CardId::Bash, 202),
            CombatCard::new(CardId::Defend, 203),
        ];

        handle_use_potion(0, None, &mut state);

        let Some(Action::SuspendForGridSelect {
            source_pile,
            min,
            max,
            can_cancel,
            reason,
            ..
        }) = state.pop_next_action()
        else {
            panic!("Liquid Memories should queue a discard grid select when discard has more cards than potency");
        };
        assert_eq!(source_pile, crate::state::PileType::Discard);
        assert_eq!(min, 2);
        assert_eq!(max, 2);
        assert!(!can_cancel);
        assert_eq!(reason, crate::state::GridSelectReason::DiscardToHand);
    }

    #[test]
    fn random_class_card_in_combat_pool_excludes_healing_cards_like_java() {
        let all = class_card_pool_for_type("Ironclad", None);
        assert!(!all.contains(&CardId::Feed));
        assert!(!all.contains(&CardId::Reaper));

        let attacks = class_card_pool_for_type("Ironclad", Some(CardType::Attack));
        assert!(!attacks.contains(&CardId::Feed));
        assert!(!attacks.contains(&CardId::Reaper));
        assert!(!attacks.contains(&CardId::InfernalBlade));
        assert!(attacks.contains(&CardId::Pummel));
    }

    #[test]
    fn discard_pile_to_top_uses_java_basically_dead_guard() {
        let mut state = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.id = 900;
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        state.entities.monsters = vec![monster];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Strike, 901)];

        handle_discard_pile_to_top_of_deck(&mut state);

        assert!(state.zones.discard_pile.is_empty());
        assert_eq!(state.zones.draw_pile[0].uuid, 901);
    }

    #[test]
    fn generated_skill_entering_hand_obeys_corruption_cost_override() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Corruption,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_hand(CardId::Defend, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].id, CardId::Defend);
        assert_eq!(state.zones.hand[0].cost_for_turn, Some(0));
    }

    #[test]
    fn generated_skill_overflowing_to_discard_does_not_apply_hand_only_corruption_hook() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Corruption,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            state.zones.hand.push(CombatCard::new(CardId::Strike, uuid));
        }
        state.zones.card_uuid_counter = 10;

        handle_make_temp_card_in_hand(CardId::Defend, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Defend);
        assert_eq!(state.zones.discard_pile[0].cost_for_turn, None);
    }

    #[test]
    fn generated_cards_apply_master_reality_before_entering_zones() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_hand(CardId::Anger, 1, false, &mut state);
        handle_make_temp_card_in_discard(CardId::Anger, 1, false, &mut state);
        handle_make_temp_card_in_draw_pile(CardId::Anger, 1, false, false, false, &mut state);
        handle_make_temp_card_in_hand(CardId::Wound, 1, false, &mut state);

        assert_eq!(state.zones.hand[0].id, CardId::Anger);
        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Anger);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(state.zones.draw_pile[0].id, CardId::Anger);
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.hand[1].id, CardId::Wound);
        assert_eq!(state.zones.hand[1].upgrades, 0);
    }

    #[test]
    fn searing_blow_preserves_java_master_reality_effect_call_counts() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.zones.card_uuid_counter = 30;

        handle_make_temp_card_in_hand(CardId::SearingBlow, 1, false, &mut state);
        handle_make_temp_card_in_discard(CardId::SearingBlow, 1, false, &mut state);
        handle_make_temp_card_in_draw_pile(CardId::SearingBlow, 1, false, false, false, &mut state);

        assert_eq!(
            state.zones.hand[0].upgrades, 2,
            "Java MakeTempCardInHandAction plus ShowCardAndAddToHandEffect both call Master Reality"
        );
        assert_eq!(
            state.zones.discard_pile[0].upgrades, 1,
            "Java MakeTempCardInDiscardAction(card, amount) only upgrades through the discard effect"
        );
        assert_eq!(
            state.zones.draw_pile[0].upgrades, 2,
            "Java MakeTempCardInDrawPileAction amount<6 and the draw-pile effect both call Master Reality"
        );
    }

    #[test]
    fn make_temp_card_in_discard_large_amount_matches_java_no_effect() {
        let mut temp_state = blank_test_combat();
        handle_make_temp_card_in_discard(CardId::Burn, 6, false, &mut temp_state);
        assert!(
            temp_state.zones.discard_pile.is_empty(),
            "Java MakeTempCardInDiscardAction only adds effects when numCards < 6"
        );

        let mut copy_state = blank_test_combat();
        handle_make_copy_in_discard(
            Box::new(CombatCard::new(CardId::Anger, 20)),
            6,
            &mut copy_state,
        );
        assert!(
            copy_state.zones.discard_pile.is_empty(),
            "MakeCopyInDiscard mirrors Java MakeTempCardInDiscardAction(card, amount)"
        );
    }

    #[test]
    fn make_temp_card_in_draw_pile_large_amount_uses_java_src_card_path() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_draw_pile(CardId::SearingBlow, 6, true, false, false, &mut state);

        assert_eq!(state.zones.draw_pile.len(), 6);
        assert!(state
            .zones
            .draw_pile
            .iter()
            .all(|card| card.id == CardId::SearingBlow && card.upgrades == 1));
    }

    #[test]
    fn make_temp_card_in_hand_overflow_uses_java_discard_effect_upgrade_count() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            state.zones.hand.push(CombatCard::new(CardId::Strike, uuid));
        }
        state.zones.card_uuid_counter = 10;

        handle_make_temp_card_in_hand(CardId::SearingBlow, 1, false, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::SearingBlow);
        assert_eq!(
            state.zones.discard_pile[0].upgrades, 1,
            "Java MakeTempCardInHandAction overflow adds srcCard to discard, so only the action constructor Master Reality call affects the actual card"
        );
    }

    #[test]
    fn constructed_make_copy_in_hand_separates_constructor_and_effect_reality_calls() {
        let mut hand_state = blank_test_combat();
        hand_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut constructed = CombatCard::new(CardId::SearingBlow, 200);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut constructed,
            &hand_state,
            1,
        );
        handle_make_constructed_copy_in_hand(Box::new(constructed.clone()), 1, &mut hand_state);
        assert_eq!(
            hand_state.zones.hand[0].upgrades, 2,
            "hand path gets Java constructor and ShowCardAndAddToHandEffect Master Reality calls"
        );

        let mut delayed_state = blank_test_combat();
        delayed_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut delayed_constructed = CombatCard::new(CardId::SearingBlow, 201);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut delayed_constructed,
            &delayed_state,
            1,
        );
        store::set_powers_for(&mut delayed_state, 0, vec![]);
        handle_make_constructed_copy_in_hand(Box::new(delayed_constructed), 1, &mut delayed_state);
        assert_eq!(
            delayed_state.zones.hand[0].upgrades, 1,
            "constructor-time Master Reality persists even if the power is gone when the queued action executes"
        );

        let mut overflow_state = blank_test_combat();
        overflow_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            overflow_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Strike, uuid));
        }
        let mut overflow_constructed = CombatCard::new(CardId::SearingBlow, 202);
        crate::content::cards::apply_master_reality_to_generated_card(
            &mut overflow_constructed,
            &overflow_state,
            1,
        );
        handle_make_constructed_copy_in_hand(
            Box::new(overflow_constructed),
            1,
            &mut overflow_state,
        );
        assert_eq!(
            overflow_state.zones.discard_pile[0].upgrades, 1,
            "Java overflow discard receives the constructor-upgraded srcCard, not the visually upgraded discard-effect copy"
        );
    }

    #[test]
    fn stasis_return_preserves_same_uuid_and_java_master_reality_counts() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state
            .zones
            .limbo
            .push(CombatCard::new(CardId::SearingBlow, 77));

        handle_return_stasis_card(77, true, &mut state);

        assert!(state.zones.limbo.is_empty());
        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].uuid, 77);
        assert_eq!(
            state.zones.hand[0].upgrades, 2,
            "Java Stasis hand path uses MakeTempCardInHandAction(card, false, true), so sameUUID still receives constructor + hand-effect Master Reality calls"
        );

        let mut overflow_state = blank_test_combat();
        overflow_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            overflow_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Strike, uuid));
        }
        overflow_state
            .zones
            .limbo
            .push(CombatCard::new(CardId::SearingBlow, 88));

        handle_return_stasis_card(88, false, &mut overflow_state);

        assert!(overflow_state.zones.limbo.is_empty());
        assert_eq!(overflow_state.zones.discard_pile.len(), 1);
        assert_eq!(overflow_state.zones.discard_pile[0].uuid, 88);
        assert_eq!(
            overflow_state.zones.discard_pile[0].upgrades, 0,
            "Java Stasis full-hand path uses MakeTempCardInDiscardAction(card, true), whose sameUUID constructor skips Master Reality"
        );

        let mut execution_overflow_state = blank_test_combat();
        execution_overflow_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        for uuid in 1..=10 {
            execution_overflow_state
                .zones
                .hand
                .push(CombatCard::new(CardId::Strike, uuid));
        }
        execution_overflow_state
            .zones
            .limbo
            .push(CombatCard::new(CardId::SearingBlow, 99));

        handle_return_stasis_card(99, true, &mut execution_overflow_state);

        assert!(execution_overflow_state.zones.limbo.is_empty());
        assert_eq!(execution_overflow_state.zones.discard_pile.len(), 1);
        assert_eq!(execution_overflow_state.zones.discard_pile[0].uuid, 99);
        assert_eq!(
            execution_overflow_state.zones.discard_pile[0].upgrades, 1,
            "If Stasis queued the hand action but hand is full at execution, Java keeps the constructor Master Reality upgrade and the discard visual copy upgrade does not affect the srcCard"
        );
    }

    #[test]
    fn random_pool_blood_for_blood_copy_uses_java_make_copy_damage_discount() {
        let mut state = blank_test_combat();
        state.turn.counters.times_damaged_this_combat = 3;

        let card = crate::content::cards::make_fresh_card_copy_for_combat(
            CardId::BloodForBlood,
            90,
            &state,
        );

        assert_eq!(card.cost_modifier, -3);
        assert_eq!(
            card.get_cost(),
            1,
            "Java BloodForBlood.makeCopy() applies damagedThisCombat before random generated copies enter combat"
        );
    }

    #[test]
    fn make_copy_in_discard_uses_java_stat_equivalent_copy_not_transient_evaluation() {
        let mut state = blank_test_combat();
        state.zones.card_uuid_counter = 20;
        let mut original = CombatCard::new(CardId::Anger, 10);
        original.upgrades = 1;
        original.misc_value = 3;
        original.base_damage_override = Some(17);
        original.cost_modifier = -1;
        original.cost_for_turn = Some(0);
        original.free_to_play_once = true;
        original.base_damage_mut = 99;
        original.base_block_mut = 88;
        original.base_magic_num_mut = 77;
        original.multi_damage = smallvec::smallvec![1, 2, 3];
        original.exhaust_override = Some(true);
        original.retain_override = Some(true);
        original.energy_on_use = 5;

        handle_make_copy_in_discard(Box::new(original), 1, &mut state);

        let copied = &state.zones.discard_pile[0];
        assert_eq!(copied.uuid, 21);
        assert_eq!(copied.id, CardId::Anger);
        assert_eq!(copied.upgrades, 1);
        assert_eq!(copied.misc_value, 3);
        assert_eq!(copied.base_damage_override, Some(17));
        assert_eq!(copied.cost_modifier, -1);
        assert_eq!(copied.cost_for_turn, Some(0));
        assert!(copied.free_to_play_once);
        assert_eq!(copied.base_damage_mut, 0);
        assert_eq!(copied.base_block_mut, 0);
        assert_eq!(copied.base_magic_num_mut, 0);
        assert!(copied.multi_damage.is_empty());
        assert_eq!(copied.exhaust_override, None);
        assert_eq!(copied.retain_override, None);
        assert_eq!(copied.energy_on_use, 0);
    }

    #[test]
    fn make_temp_card_in_discard_and_deck_creates_distinct_instances() {
        let mut state = blank_test_combat();
        state.zones.card_uuid_counter = 30;

        handle_make_temp_card_in_discard_and_deck(CardId::Burn, 1, &mut state);

        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.draw_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].id, CardId::Burn);
        assert_eq!(state.zones.draw_pile[0].id, CardId::Burn);
        assert_eq!(
            state.zones.draw_pile[0].uuid, 31,
            "Java creates the draw-pile copy before the discard copy"
        );
        assert_eq!(state.zones.discard_pile[0].uuid, 32);
        assert_ne!(
            state.zones.discard_pile[0].uuid, state.zones.draw_pile[0].uuid,
            "Java MakeTempCardInDiscardAndDeckAction uses separate stat-equivalent copies"
        );
    }

    #[test]
    fn make_temp_card_in_discard_and_deck_applies_one_master_reality_per_destination() {
        let mut state = blank_test_combat();
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_make_temp_card_in_discard_and_deck(CardId::SearingBlow, 1, &mut state);

        assert_eq!(state.zones.draw_pile.len(), 1);
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(
            state.zones.draw_pile[0].upgrades, 1,
            "Java draw-pile effect upgrades its inserted stat-equivalent copy once"
        );
        assert_eq!(
            state.zones.discard_pile[0].upgrades, 1,
            "Java discard effect upgrades its separate stat-equivalent copy once"
        );
    }

    #[test]
    fn burn_increase_upgrades_only_draw_and_discard_like_java() {
        let mut state = blank_test_combat();
        state.zones.hand = vec![CombatCard::new(CardId::Burn, 1)];
        state.zones.draw_pile = vec![CombatCard::new(CardId::Burn, 2)];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Burn, 3)];
        state.zones.exhaust_pile = vec![CombatCard::new(CardId::Burn, 4)];

        handle_upgrade_all_burns(&mut state);

        assert_eq!(
            state.zones.hand[0].upgrades, 0,
            "Java BurnIncreaseAction does not iterate the hand"
        );
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(
            state.zones.exhaust_pile[0].upgrades, 0,
            "Java BurnIncreaseAction does not iterate the exhaust pile"
        );
    }

    #[test]
    fn make_random_card_in_hand_uses_current_player_class_pool() {
        let mut state = blank_test_combat();
        state.meta.player_class = "Silent";

        handle_make_random_card_in_hand(Some(CardType::Attack), Some(0), &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        let generated = &state.zones.hand[0];
        assert_eq!(generated.cost_for_turn, Some(0));
        assert!(
            crate::content::cards::silent_pool_for_type(CardType::Attack).contains(&generated.id),
            "random generated combat cards must come from the current character pool"
        );
        assert!(
            !crate::content::cards::ironclad_pool_for_type(CardType::Attack)
                .contains(&generated.id),
            "Silent random generated combat cards must not leak Ironclad cards"
        );
    }

    #[test]
    fn make_random_card_in_draw_pile_uses_current_player_class_pool() {
        let mut state = blank_test_combat();
        state.meta.player_class = "Silent";
        state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 1)];
        state.zones.card_uuid_counter = 1;

        handle_make_random_card_in_draw_pile(Some(CardType::Skill), Some(0), false, &mut state);

        assert_eq!(state.zones.draw_pile.len(), 2);
        let generated = &state.zones.draw_pile[0];
        let generated_def = crate::content::cards::get_card_definition(generated.id);
        if generated_def.cost >= 0 {
            assert_eq!(generated.cost_for_turn, Some(0));
        } else {
            assert_eq!(
                generated.cost_for_turn, None,
                "Java setCostForTurn(0) does not make unplayable cards playable"
            );
        }
        assert!(
            crate::content::cards::silent_pool_for_type(CardType::Skill).contains(&generated.id),
            "random generated draw-pile cards must come from the current character pool"
        );
        assert!(
            !crate::content::cards::ironclad_pool_for_type(CardType::Skill).contains(&generated.id),
            "Silent random generated draw-pile cards must not leak Ironclad cards"
        );
        assert_eq!(state.zones.draw_pile[1].id, CardId::Strike);
    }

    #[test]
    fn make_temp_card_in_draw_pile_non_random_goes_to_top() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        state.zones.card_uuid_counter = 2;

        handle_make_temp_card_in_draw_pile(CardId::Wound, 1, false, false, false, &mut state);

        assert_eq!(state.zones.draw_pile[0].id, CardId::Wound);
        assert_eq!(state.zones.draw_pile[1].id, CardId::Strike);
        assert_eq!(state.zones.draw_pile[2].id, CardId::Defend);
    }

    #[test]
    fn make_temp_card_in_draw_pile_to_bottom_goes_under_existing_cards() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        state.zones.card_uuid_counter = 2;

        handle_make_temp_card_in_draw_pile(CardId::Wound, 1, false, true, false, &mut state);

        assert_eq!(state.zones.draw_pile[0].id, CardId::Strike);
        assert_eq!(state.zones.draw_pile[1].id, CardId::Defend);
        assert_eq!(state.zones.draw_pile[2].id, CardId::Wound);
    }

    #[test]
    fn random_draw_pile_insert_does_not_put_card_on_top_when_pile_is_nonempty() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];

        state.add_card_to_draw_pile_random_spot(CombatCard::new(CardId::Wound, 3));

        assert_eq!(state.zones.draw_pile[0].id, CardId::Strike);
        assert!(state
            .zones
            .draw_pile
            .iter()
            .any(|card| card.id == CardId::Wound));
    }

    #[test]
    fn random_draw_pile_insert_maps_java_bottom_to_top_order() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
        ];
        let java_insert_index = state
            .rng
            .card_random_rng
            .clone()
            .random(state.zones.draw_pile.len() as i32 - 1)
            as usize;
        let expected_rust_index = state.zones.draw_pile.len() - java_insert_index;

        state.add_card_to_draw_pile_random_spot(CombatCard::new(CardId::Wound, 4));

        assert_eq!(state.zones.draw_pile[expected_rust_index].id, CardId::Wound);
    }

    #[test]
    fn draw_pile_to_hand_by_type_matches_java_temp_group_rng_sequence() {
        let mut state = blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
            CombatCard::new(CardId::Bash, 3),
            CombatCard::new(CardId::Strike, 4),
        ];
        let mut expected_rng = state.rng.clone();
        let mut expected_candidates = Vec::new();
        for uuid in [4_u32, 3, 1] {
            if expected_candidates.is_empty() {
                expected_candidates.push(uuid);
            } else {
                let index = expected_rng
                    .card_random_rng
                    .random(expected_candidates.len() as i32 - 1)
                    as usize;
                expected_candidates.insert(index, uuid);
            }
        }
        crate::runtime::rng::shuffle_with_random_long(
            &mut expected_candidates,
            &mut expected_rng.shuffle_rng,
        );
        let expected_uuid = expected_candidates[0];

        handle_draw_pile_to_hand_by_type(1, CardType::Attack, &mut state);

        assert_eq!(state.zones.hand.len(), 1);
        assert_eq!(state.zones.hand[0].uuid, expected_uuid);
        assert!(!state
            .zones
            .draw_pile
            .iter()
            .any(|card| card.uuid == expected_uuid));
        assert_eq!(
            state.rng.card_random_rng.counter,
            expected_rng.card_random_rng.counter
        );
        assert_eq!(
            state.rng.shuffle_rng.counter,
            expected_rng.shuffle_rng.counter
        );
    }

    #[test]
    fn draw_pile_to_hand_by_type_overflow_discards_selected_card() {
        let mut state = blank_test_combat();
        for uuid in 10..20 {
            state.zones.hand.push(CombatCard::new(CardId::Defend, uuid));
        }
        state.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 1)];

        handle_draw_pile_to_hand_by_type(1, CardType::Attack, &mut state);

        assert_eq!(state.zones.hand.len(), 10);
        assert!(state.zones.draw_pile.is_empty());
        assert_eq!(state.zones.discard_pile.len(), 1);
        assert_eq!(state.zones.discard_pile[0].uuid, 1);
    }

    fn seed_with_next_card_random_boolean(desired: bool) -> u64 {
        for seed in 1..10_000 {
            let mut rng = crate::runtime::rng::StsRng::new(seed);
            if rng.random_boolean() == desired {
                return seed;
            }
        }
        panic!("failed to find cardRandomRng seed with next randomBoolean={desired}");
    }

    #[test]
    fn use_card_done_resets_free_to_play_once_before_zone_move() {
        let mut discarded = blank_test_combat();
        let mut free_strike = CombatCard::new(CardId::Strike, 90);
        free_strike.free_to_play_once = true;
        discarded.zones.limbo = vec![free_strike];

        handle_use_card_done(false, true, &mut discarded);

        assert_eq!(discarded.zones.discard_pile.len(), 1);
        assert!(
            !discarded.zones.discard_pile[0].free_to_play_once,
            "Java UseCardAction clears freeToPlayOnce before discarding the used card"
        );

        let mut exhausted = blank_test_combat();
        let mut free_havoc_target = CombatCard::new(CardId::Strike, 91);
        free_havoc_target.free_to_play_once = true;
        free_havoc_target.exhaust_override = Some(true);
        exhausted.zones.limbo = vec![free_havoc_target];

        handle_use_card_done(true, true, &mut exhausted);

        assert_eq!(exhausted.zones.exhaust_pile.len(), 1);
        assert!(
            !exhausted.zones.exhaust_pile[0].free_to_play_once,
            "Java UseCardAction clears freeToPlayOnce before exhausting the used card"
        );
    }

    #[test]
    fn use_card_done_applies_strange_spoon_to_exhaust_on_use_once_cards() {
        for expected_saved in [true, false] {
            let mut state = blank_test_combat();
            state
                .entities
                .player
                .add_relic(RelicState::new(RelicId::StrangeSpoon));
            state.rng.card_random_rng = crate::runtime::rng::StsRng::new(
                seed_with_next_card_random_boolean(expected_saved),
            );

            let mut havoc_target = CombatCard::new(CardId::Strike, 92);
            havoc_target.free_to_play_once = true;
            havoc_target.exhaust_override = Some(true);
            state.zones.limbo = vec![havoc_target];

            handle_use_card_done(true, true, &mut state);

            assert_eq!(
                state.rng.card_random_rng.counter, 1,
                "Java Strange Spoon uses cardRandomRng.randomBoolean() when exhaustCard is true"
            );
            if expected_saved {
                assert!(state.zones.exhaust_pile.is_empty());
                assert_eq!(state.zones.discard_pile.len(), 1);
                assert_eq!(state.zones.discard_pile[0].id, CardId::Strike);
                assert_eq!(
                    state.zones.discard_pile[0].exhaust_override, None,
                    "Java clears exhaustOnUseOnce after UseCardAction resolves"
                );
                assert!(!state.zones.discard_pile[0].free_to_play_once);
            } else {
                assert!(state.zones.discard_pile.is_empty());
                assert_eq!(state.zones.exhaust_pile.len(), 1);
                assert_eq!(state.zones.exhaust_pile[0].id, CardId::Strike);
                assert!(!state.zones.exhaust_pile[0].free_to_play_once);
            }
        }
    }

    #[test]
    fn use_card_done_does_not_consume_spoon_rng_without_spoon_or_exhaust() {
        let mut no_spoon = blank_test_combat();
        no_spoon.rng.card_random_rng = crate::runtime::rng::StsRng::new(7);
        let before_counter = no_spoon.rng.card_random_rng.counter;
        no_spoon.zones.limbo = vec![CombatCard::new(CardId::Strike, 93)];

        handle_use_card_done(true, true, &mut no_spoon);

        assert_eq!(no_spoon.rng.card_random_rng.counter, before_counter);
        assert_eq!(no_spoon.zones.exhaust_pile.len(), 1);

        let mut not_exhausting = blank_test_combat();
        not_exhausting
            .entities
            .player
            .add_relic(RelicState::new(RelicId::StrangeSpoon));
        not_exhausting.rng.card_random_rng = crate::runtime::rng::StsRng::new(7);
        let before_counter = not_exhausting.rng.card_random_rng.counter;
        not_exhausting.zones.limbo = vec![CombatCard::new(CardId::Strike, 94)];

        handle_use_card_done(false, true, &mut not_exhausting);

        assert_eq!(not_exhausting.rng.card_random_rng.counter, before_counter);
        assert_eq!(not_exhausting.zones.discard_pile.len(), 1);
    }

    #[test]
    fn queued_direct_card_accepts_zero_hp_target_if_java_dead_flags_are_clear() {
        let mut state = blank_test_combat();
        let mut zero_hp = test_monster(EnemyId::JawWorm);
        zero_hp.id = 700;
        zero_hp.current_hp = 0;
        zero_hp.is_dying = false;
        zero_hp.half_dead = false;
        zero_hp.is_escaped = false;
        state.entities.monsters = vec![zero_hp];

        handle_play_card_direct(
            Box::new(CombatCard::new(CardId::Strike, 701)),
            Some(700),
            false,
            &mut state,
        );

        assert_eq!(
            state.zones.limbo.len(),
            1,
            "Java GameActionManager checks isDeadOrEscaped(), not currentHealth, before useCard"
        );
        assert!(matches!(
            state.pop_next_action(),
            Some(Action::Damage(crate::runtime::action::DamageInfo {
                target: 700,
                ..
            }))
        ));
    }

    #[test]
    fn failed_autoplay_target_cleanup_matches_java_can_use_path() {
        let mut state = blank_test_combat();
        let mut dying = test_monster(EnemyId::JawWorm);
        dying.id = 710;
        dying.current_hp = 0;
        dying.is_dying = true;
        state.entities.monsters = vec![dying];
        state.enqueue_card_play(
            crate::runtime::combat::QueuedCardPlay {
                card: CombatCard::new(CardId::Strike, 711),
                target: Some(710),
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: crate::runtime::combat::QueuedCardSource::Normal,
            },
            false,
        );

        let flush = state
            .pop_next_action()
            .expect("queued autoplay card should schedule flush");
        crate::engine::action_handlers::execute_action(flush, &mut state);

        assert!(matches!(
            state.pop_next_action(),
            Some(Action::UseCardDone {
                should_exhaust: false,
                trigger_after_use_hooks: false,
            }),
        ));
        assert!(
            state.zones.limbo.iter().any(|card| card.uuid == 711),
            "Java failed autoplay canUse path still routes the card through UseCardAction"
        );
    }

    #[test]
    fn early_end_turn_clears_card_queue_and_only_cleans_up_autoplay_cards_like_java() {
        let mut state = blank_test_combat();
        crate::content::powers::store::set_powers_for(
            &mut state,
            0,
            vec![Power {
                power_type: PowerId::Rebound,
                instance_id: None,
                amount: 1,
                extra_data: 1,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        state.zones.queued_cards = std::collections::VecDeque::from([
            QueuedCardPlay {
                card: CombatCard::new(CardId::Strike, 712),
                target: Some(7),
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: false,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
            QueuedCardPlay {
                card: CombatCard::new(CardId::Defend, 713),
                target: None,
                energy_on_use: 0,
                ignore_energy_total: true,
                autoplay: true,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            },
        ]);

        handle_queue_early_end_turn(&mut state);

        assert!(state.zones.queued_cards.is_empty());
        assert_eq!(
            state
                .zones
                .limbo
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![713]
        );
        let cleanup = state
            .pop_next_action()
            .expect("Java early-end sequence adds UseCardAction only for autoplay queued cards");
        assert_eq!(
            cleanup,
            Action::UseCardDone {
                should_exhaust: false,
                trigger_after_use_hooks: false,
            }
        );

        crate::engine::action_handlers::execute_action(cleanup, &mut state);
        assert_eq!(
            state
                .zones
                .discard_pile
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![713],
            "dontTriggerOnUseCard cleanup must skip Rebound-style after-use hooks"
        );
        assert!(state.zones.draw_pile.is_empty());
        assert!(state.zones.hand.iter().all(|card| card.uuid != 712));
        assert!(state.zones.discard_pile.iter().all(|card| card.uuid != 712));
        assert!(state.zones.exhaust_pile.iter().all(|card| card.uuid != 712));
    }

    #[test]
    fn upgrade_all_in_hand_matches_armaments_plus_can_upgrade_filter() {
        let mut state = blank_test_combat();
        let mut upgraded_defend = CombatCard::new(CardId::Defend, 801);
        upgraded_defend.upgrades = 1;
        let mut searing = CombatCard::new(CardId::SearingBlow, 804);
        searing.upgrades = 2;
        state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 800),
            upgraded_defend,
            CombatCard::new(CardId::Wound, 802),
            CombatCard::new(CardId::Injury, 803),
            searing,
        ];

        handle_upgrade_all_in_hand(&mut state);

        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(
            state.zones.hand[1].upgrades, 1,
            "Java canUpgrade() rejects already-upgraded normal cards"
        );
        assert_eq!(
            state.zones.hand[2].upgrades, 0,
            "Java canUpgrade() rejects Status cards"
        );
        assert_eq!(
            state.zones.hand[3].upgrades, 0,
            "Java canUpgrade() rejects Curse cards"
        );
        assert_eq!(
            state.zones.hand[4].upgrades, 3,
            "Searing Blow remains repeatedly upgradeable through its override"
        );
    }

    #[test]
    fn upgrade_all_cards_in_combat_matches_apotheosis_groups() {
        let mut state = blank_test_combat();
        state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 810),
            CombatCard::new(CardId::Wound, 811),
        ];
        state.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 812)];
        state.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 813)];
        state.zones.exhaust_pile = vec![CombatCard::new(CardId::ShrugItOff, 814)];
        state.zones.limbo = vec![CombatCard::new(CardId::Strike, 815)];

        handle_upgrade_all_cards_in_combat(&mut state);

        assert_eq!(state.zones.hand[0].upgrades, 1);
        assert_eq!(state.zones.hand[1].upgrades, 0);
        assert_eq!(state.zones.draw_pile[0].upgrades, 1);
        assert_eq!(state.zones.discard_pile[0].upgrades, 1);
        assert_eq!(state.zones.exhaust_pile[0].upgrades, 1);
        assert_eq!(
            state.zones.limbo[0].upgrades, 0,
            "Java ApotheosisAction upgrades hand/draw/discard/exhaust, not limbo/cardInUse"
        );
    }

    #[test]
    fn end_turn_ethereal_exhaust_respects_explicit_retain_like_java_discard_at_end() {
        let mut state = blank_test_combat();
        let mut retained_ethereal = CombatCard::new(CardId::GhostlyArmor, 830);
        retained_ethereal.retain_override = Some(true);
        state.zones.hand = vec![
            retained_ethereal,
            CombatCard::new(CardId::Carnage, 831),
            CombatCard::new(CardId::Strike, 832),
        ];

        handle_end_turn_trigger(&mut state);
        let queued: Vec<_> = std::iter::from_fn(|| state.pop_next_action()).collect();

        assert!(
            !queued.iter().any(|action| matches!(
                action,
                Action::ExhaustCard {
                    card_uuid: 830,
                    source_pile: crate::state::PileType::Hand
                }
            )),
            "Java removes retained/selfRetain cards from hand before triggerOnEndOfPlayerTurn, so explicit retain prevents ethereal exhaust"
        );
        assert!(
            queued.iter().any(|action| matches!(
                action,
                Action::ExhaustCard {
                    card_uuid: 831,
                    source_pile: crate::state::PileType::Hand
                }
            )),
            "non-retained ethereal cards still exhaust at end of turn"
        );
    }
}
