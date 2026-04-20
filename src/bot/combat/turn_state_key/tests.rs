use super::{
    diagnostic_outcome_key, pending_choice::pending_choice_key, stable_dominance_bucket_key,
    stable_outcome_key, turn_state_key,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::runtime::combat::{CombatCard, QueuedCardPlay, QueuedCardSource};
use crate::state::core::PendingChoice;
use crate::state::EngineState;
use crate::test_support::{blank_test_combat, planned_monster};

#[test]
fn stable_outcome_key_ignores_player_resources_potions_and_runtime_noise() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline.zones.hand.push(CombatCard::new(CardId::Strike, 1));

    let mut variant = baseline.clone();
    variant.entities.player.current_hp -= 7;
    variant.entities.player.block = 12;
    variant.entities.potions = vec![Some(Potion::new(PotionId::SteroidPotion, 1)), None, None];
    variant.zones.queued_cards.push_back(QueuedCardPlay {
        card: CombatCard::new(CardId::Strike, 77),
        target: None,
        energy_on_use: 1,
        ignore_energy_total: false,
        autoplay: false,
        random_target: false,
        is_end_turn_autoplay: false,
        purge_on_use: false,
        source: QueuedCardSource::Normal,
    });
    variant.zones.card_uuid_counter = 99;
    variant.queue_action_back(crate::runtime::action::Action::GainEnergy { amount: 1 });

    assert_eq!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &variant),
    );
    assert_ne!(
        turn_state_key(&EngineState::CombatPlayerTurn, &baseline),
        turn_state_key(&EngineState::CombatPlayerTurn, &variant),
    );
}

#[test]
fn stable_outcome_key_keeps_future_relevant_monster_card_and_turn_state() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .hand
        .push(CombatCard::new(CardId::Rampage, 1));

    let mut monster_variant = baseline.clone();
    monster_variant.entities.monsters[0].current_hp -= 5;
    assert_ne!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &monster_variant),
    );

    let mut card_variant = baseline.clone();
    card_variant.zones.hand[0].base_damage_mut = 13;
    assert_ne!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &card_variant),
    );

    let mut turn_variant = baseline.clone();
    turn_variant.turn.energy = 1;
    assert_ne!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &turn_variant),
    );
}

#[test]
fn stable_outcome_key_ignores_card_instance_ids_but_keeps_rng() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .hand
        .push(CombatCard::new(CardId::Rampage, 11));

    let mut uuid_variant = baseline.clone();
    uuid_variant.zones.hand[0].uuid = 999;
    uuid_variant.zones.card_uuid_counter = 1001;
    assert_eq!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &uuid_variant),
    );

    let mut rng_variant = baseline.clone();
    rng_variant.rng.ai_rng.counter += 1;
    assert_ne!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &rng_variant),
    );
}

#[test]
fn stable_outcome_key_normalizes_hand_and_discovery_choice_order() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline.zones.hand.extend([
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 12),
    ]);
    let hand_uuids = baseline
        .zones
        .hand
        .iter()
        .map(|card| card.uuid)
        .collect::<Vec<_>>();

    assert_eq!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: hand_uuids.clone(),
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: hand_uuids.into_iter().rev().collect(),
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &baseline,
        ),
    );

    assert_eq!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::DiscoverySelect(vec![
                CardId::Strike,
                CardId::Defend,
                CardId::Bash,
            ])),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::DiscoverySelect(vec![
                CardId::Bash,
                CardId::Strike,
                CardId::Defend,
            ])),
            &baseline,
        ),
    );
}

#[test]
fn stable_outcome_key_keeps_scry_order_and_postcombat_gold_meta() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    assert_ne!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![1, 2],
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Defend, CardId::Strike],
                card_uuids: vec![2, 1],
            }),
            &baseline,
        ),
    );

    let mut postcombat_variant = baseline.clone();
    postcombat_variant
        .zones
        .hand
        .push(CombatCard::new(CardId::Strike, 99));
    postcombat_variant.entities.monsters[0].current_hp -= 5;
    assert_eq!(
        stable_outcome_key(&EngineState::MapNavigation, &baseline),
        stable_outcome_key(&EngineState::MapNavigation, &postcombat_variant),
    );

    let mut gold_variant = baseline.clone();
    gold_variant.entities.player.gold += 25;
    assert_ne!(
        stable_outcome_key(&EngineState::MapNavigation, &baseline),
        stable_outcome_key(&EngineState::MapNavigation, &gold_variant),
    );

    let mut meta_variant = baseline.clone();
    meta_variant
        .meta
        .meta_changes
        .push(crate::runtime::combat::MetaChange::AddCardToMasterDeck(
            CardId::Strike,
        ));
    assert_ne!(
        stable_outcome_key(&EngineState::MapNavigation, &baseline),
        stable_outcome_key(&EngineState::MapNavigation, &meta_variant),
    );
}

#[test]
fn stable_outcome_key_treats_combat_processing_as_unstable_not_game_over() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    let unstable = diagnostic_outcome_key(&EngineState::CombatProcessing, &baseline);
    let game_over = stable_outcome_key(
        &EngineState::GameOver(crate::state::core::RunResult::Defeat),
        &baseline,
    );

    assert!(unstable.diagnostic_string().contains("scope=Unstable"));
    assert_ne!(unstable, game_over);
    assert_eq!(
        stable_dominance_bucket_key(&EngineState::CombatProcessing, &baseline),
        None,
    );
}

#[test]
fn stable_outcome_key_prefers_visible_card_resolution_over_uuid_fallback() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].uuid = 99;

    assert_eq!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: crate::state::core::PileType::Draw,
                candidate_uuids: vec![42],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::GridSelectReason::MoveToDrawPile,
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: crate::state::core::PileType::Draw,
                candidate_uuids: vec![99],
                min_cards: 0,
                max_cards: 1,
                can_cancel: true,
                reason: crate::state::GridSelectReason::MoveToDrawPile,
            }),
            &variant,
        ),
    );
}

#[test]
fn stable_outcome_key_master_deck_grid_select_uses_explicit_master_refs() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    let forward = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::MasterDeck,
            candidate_uuids: vec![42, 7],
            min_cards: 0,
            max_cards: 1,
            can_cancel: true,
            reason: crate::state::GridSelectReason::MoveToDrawPile,
        }),
        &baseline,
    );
    let reversed = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::MasterDeck,
            candidate_uuids: vec![7, 42],
            min_cards: 0,
            max_cards: 1,
            can_cancel: true,
            reason: crate::state::GridSelectReason::MoveToDrawPile,
        }),
        &baseline,
    );

    assert_eq!(forward, reversed);
    let diagnostic = forward.diagnostic_string();
    assert!(diagnostic.contains("master_ref:7") && diagnostic.contains("master_ref:42"));
    assert!(!diagnostic.contains("opaque_uuid"));
}

#[test]
fn stable_outcome_key_master_deck_does_not_resolve_visible_uuid_collisions() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].base_damage_mut = 99;

    let choice = PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::MasterDeck,
        candidate_uuids: vec![42],
        min_cards: 0,
        max_cards: 1,
        can_cancel: true,
        reason: crate::state::GridSelectReason::MoveToDrawPile,
    };

    let baseline_key = pending_choice_key(&choice, &baseline);
    let variant_key = pending_choice_key(&choice, &variant);

    assert_eq!(baseline_key, variant_key);
}

#[test]
fn stable_hand_select_does_not_resolve_cards_outside_hand() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .discard_pile
        .push(CombatCard::new(CardId::Strike, 42));

    let mut variant = baseline.clone();
    variant.zones.discard_pile[0].base_damage_mut = 99;

    let choice = PendingChoice::HandSelect {
        candidate_uuids: vec![42],
        min_cards: 0,
        max_cards: 1,
        can_cancel: true,
        reason: crate::state::core::HandSelectReason::Discard,
    };

    assert_eq!(
        pending_choice_key(&choice, &baseline),
        pending_choice_key(&choice, &variant)
    );
}

#[test]
fn stable_scry_select_uses_card_state_and_uuid_fallback() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    baseline
        .zones
        .draw_pile
        .push(CombatCard::new(CardId::Strike, 1));
    baseline
        .zones
        .draw_pile
        .push(CombatCard::new(CardId::Strike, 2));

    let mut variant = baseline.clone();
    variant.zones.draw_pile[1].base_damage_mut = 13;

    assert_ne!(
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![1, 2],
            }),
            &baseline,
        ),
        stable_outcome_key(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![1, 2],
            }),
            &variant,
        ),
    );

    let missing = stable_outcome_key(
        &EngineState::PendingChoice(PendingChoice::ScrySelect {
            cards: vec![CardId::Strike],
            card_uuids: vec![99],
        }),
        &baseline,
    );
    assert!(missing.diagnostic_string().contains("scry_ref:99"));
}

#[test]
fn stable_player_signature_in_combat_scope_ignores_gold_deltas() {
    let mut baseline = blank_test_combat();
    baseline
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    let mut variant = baseline.clone();
    variant.entities.player.gold += 99;
    variant.entities.player.gold_delta_this_combat += 99;

    assert_eq!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &baseline),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &variant),
    );
}

#[test]
fn stable_monster_signature_ignores_irrelevant_runtime_fields_but_keeps_relevant_ones() {
    let mut cultist = blank_test_combat();
    cultist
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));

    let mut irrelevant_runtime = cultist.clone();
    irrelevant_runtime.entities.monsters[0].hexaghost.activated = true;
    assert_eq!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &cultist),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &irrelevant_runtime),
    );

    let mut hexaghost = blank_test_combat();
    hexaghost
        .entities
        .monsters
        .push(planned_monster(EnemyId::Hexaghost, 3));
    let mut relevant_runtime = hexaghost.clone();
    relevant_runtime.entities.monsters[0].hexaghost.activated = true;
    assert_ne!(
        stable_outcome_key(&EngineState::CombatPlayerTurn, &hexaghost),
        stable_outcome_key(&EngineState::CombatPlayerTurn, &relevant_runtime),
    );
}

#[test]
fn stable_run_pending_choice_keeps_return_state_payloads_distinct() {
    let baseline = blank_test_combat();
    let reward_a = crate::rewards::state::RewardState {
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        items: vec![crate::rewards::state::RewardItem::Gold { amount: 10 }],
        pending_card_choice: None,
        skippable: true,
    };
    let reward_b = crate::rewards::state::RewardState {
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        items: vec![crate::rewards::state::RewardItem::Gold { amount: 20 }],
        pending_card_choice: None,
        skippable: true,
    };

    let a = stable_outcome_key(
        &EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::Purge,
            return_state: Box::new(EngineState::RewardScreen(reward_a)),
        }),
        &baseline,
    );
    let b = stable_outcome_key(
        &EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::Purge,
            return_state: Box::new(EngineState::RewardScreen(reward_b)),
        }),
        &baseline,
    );

    assert_ne!(a, b);
}

#[test]
fn stable_postcombat_keys_normalize_display_only_order() {
    let baseline = blank_test_combat();

    let reward_a = crate::rewards::state::RewardState {
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        items: vec![
            crate::rewards::state::RewardItem::Gold { amount: 10 },
            crate::rewards::state::RewardItem::EmeraldKey,
        ],
        pending_card_choice: Some(vec![
            crate::rewards::state::RewardCard::new(CardId::Strike, 0),
            crate::rewards::state::RewardCard::new(CardId::Defend, 0),
        ]),
        skippable: true,
    };
    let reward_b = crate::rewards::state::RewardState {
        screen_context: crate::rewards::state::RewardScreenContext::Standard,
        items: vec![
            crate::rewards::state::RewardItem::EmeraldKey,
            crate::rewards::state::RewardItem::Gold { amount: 10 },
        ],
        pending_card_choice: Some(vec![
            crate::rewards::state::RewardCard::new(CardId::Defend, 0),
            crate::rewards::state::RewardCard::new(CardId::Strike, 0),
        ]),
        skippable: true,
    };
    assert_eq!(
        stable_outcome_key(&EngineState::RewardScreen(reward_a), &baseline),
        stable_outcome_key(&EngineState::RewardScreen(reward_b), &baseline),
    );

    let shop_a = crate::shop::ShopState {
        cards: vec![
            crate::shop::ShopCard {
                card_id: CardId::Strike,
                price: 50,
                can_buy: true,
                blocked_reason: None,
            },
            crate::shop::ShopCard {
                card_id: CardId::Defend,
                price: 60,
                can_buy: true,
                blocked_reason: None,
            },
        ],
        relics: vec![crate::shop::ShopRelic {
            relic_id: crate::content::relics::RelicId::BurningBlood,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        }],
        potions: vec![crate::shop::ShopPotion {
            potion_id: PotionId::SteroidPotion,
            price: 55,
            can_buy: true,
            blocked_reason: None,
        }],
        purge_cost: 75,
        purge_available: true,
    };
    let shop_b = crate::shop::ShopState {
        cards: vec![
            crate::shop::ShopCard {
                card_id: CardId::Defend,
                price: 60,
                can_buy: true,
                blocked_reason: None,
            },
            crate::shop::ShopCard {
                card_id: CardId::Strike,
                price: 50,
                can_buy: true,
                blocked_reason: None,
            },
        ],
        relics: vec![crate::shop::ShopRelic {
            relic_id: crate::content::relics::RelicId::BurningBlood,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        }],
        potions: vec![crate::shop::ShopPotion {
            potion_id: PotionId::SteroidPotion,
            price: 55,
            can_buy: true,
            blocked_reason: None,
        }],
        purge_cost: 75,
        purge_available: true,
    };
    assert_eq!(
        stable_outcome_key(&EngineState::Shop(shop_a), &baseline),
        stable_outcome_key(&EngineState::Shop(shop_b), &baseline),
    );

    let boss_a = crate::rewards::state::BossRelicChoiceState::new(vec![
        crate::content::relics::RelicId::BlackBlood,
        crate::content::relics::RelicId::RunicDome,
    ]);
    let boss_b = crate::rewards::state::BossRelicChoiceState::new(vec![
        crate::content::relics::RelicId::RunicDome,
        crate::content::relics::RelicId::BlackBlood,
    ]);
    assert_eq!(
        stable_outcome_key(&EngineState::BossRelicSelect(boss_a), &baseline),
        stable_outcome_key(&EngineState::BossRelicSelect(boss_b), &baseline),
    );

    let event_a = crate::state::core::EventCombatState {
        rewards: crate::rewards::state::RewardState {
            screen_context: crate::rewards::state::RewardScreenContext::Standard,
            items: vec![
                crate::rewards::state::RewardItem::Gold { amount: 10 },
                crate::rewards::state::RewardItem::EmeraldKey,
            ],
            pending_card_choice: None,
            skippable: true,
        },
        reward_allowed: true,
        no_cards_in_rewards: false,
        post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
        encounter_key: "test",
    };
    let event_b = crate::state::core::EventCombatState {
        rewards: crate::rewards::state::RewardState {
            screen_context: crate::rewards::state::RewardScreenContext::Standard,
            items: vec![
                crate::rewards::state::RewardItem::EmeraldKey,
                crate::rewards::state::RewardItem::Gold { amount: 10 },
            ],
            pending_card_choice: None,
            skippable: true,
        },
        reward_allowed: true,
        no_cards_in_rewards: false,
        post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
        encounter_key: "test",
    };
    assert_eq!(
        stable_outcome_key(&EngineState::EventCombat(event_a), &baseline),
        stable_outcome_key(&EngineState::EventCombat(event_b), &baseline),
    );

    let run_a = crate::state::core::RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: crate::state::core::RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::RewardScreen(
            crate::rewards::state::RewardState {
                screen_context: crate::rewards::state::RewardScreenContext::Standard,
                items: vec![
                    crate::rewards::state::RewardItem::Gold { amount: 10 },
                    crate::rewards::state::RewardItem::EmeraldKey,
                ],
                pending_card_choice: Some(vec![
                    crate::rewards::state::RewardCard::new(CardId::Strike, 0),
                    crate::rewards::state::RewardCard::new(CardId::Defend, 0),
                ]),
                skippable: true,
            },
        )),
    };
    let run_b = crate::state::core::RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: crate::state::core::RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::RewardScreen(
            crate::rewards::state::RewardState {
                screen_context: crate::rewards::state::RewardScreenContext::Standard,
                items: vec![
                    crate::rewards::state::RewardItem::EmeraldKey,
                    crate::rewards::state::RewardItem::Gold { amount: 10 },
                ],
                pending_card_choice: Some(vec![
                    crate::rewards::state::RewardCard::new(CardId::Defend, 0),
                    crate::rewards::state::RewardCard::new(CardId::Strike, 0),
                ]),
                skippable: true,
            },
        )),
    };
    assert_eq!(
        stable_outcome_key(&EngineState::RunPendingChoice(run_a), &baseline),
        stable_outcome_key(&EngineState::RunPendingChoice(run_b), &baseline),
    );
}
