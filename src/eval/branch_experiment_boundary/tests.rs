use std::collections::BTreeSet;

use super::card_reward::select_card_reward_branch_options_with_limit;
use super::*;
use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::content::cards::CardId;
use crate::content::potions::{Potion, PotionId};
use crate::content::relics::{RelicId, RelicState};
use crate::eval::run_control::{RunControlConfig, RunControlSession};
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{BossRelicChoiceState, RewardCard, RewardItem, RewardState};
use crate::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

#[test]
fn card_reward_option_portfolio_keeps_semantic_variety() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let options = card_reward_branch_options(&session).expect("card reward options");
    let selected = select_card_reward_branch_options_with_limit(options, 2, None).options;
    let picked_labels = selected
        .iter()
        .map(|option| option.label.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(selected.len(), 2);
    assert!(
        picked_labels.contains("Shrug It Off"),
        "non-transition defense/draw candidate should not be crowded out"
    );
    assert_eq!(
        picked_labels
            .iter()
            .filter(|label| **label == "Twin Strike" || **label == "Cleave")
            .count(),
        1,
        "pure transition options should be represented, not exhaustively expanded"
    );
}

#[test]
fn current_boundary_wraps_card_reward_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: Some(1),
            max_campfire_options_per_branch: None,
            include_skip: false,
            include_event_reward_skip: false,
        },
        Some(CardRewardPortfolioContext {
            depth: 0,
            frontier_key: "frontier".to_string(),
            boundary_title: "Card Reward".to_string(),
        }),
    )
    .expect("card reward boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::CardReward);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "card_reward");
    assert!(boundary.reward_option_portfolio.is_some());
}

#[test]
fn current_boundary_does_not_treat_opened_card_reward_back_as_skip_option() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("card reward boundary");

    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.kind == "card_reward_skip"),
        "opened card reward back/cancel does not consume the reward; skip branching belongs to the unopened reward screen"
    );
}

#[test]
fn current_boundary_can_include_skip_for_unopened_card_reward_item() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("visible card reward boundary");

    let skip = boundary
        .options
        .iter()
        .find(|option| option.kind == "card_reward_skip")
        .expect("skip branch should be present on unopened reward screen");

    assert_eq!(skip.command, "skip");
    assert_eq!(skip.effect_kind, "skip_card_reward");
    assert!(skip.selected_cards.is_empty());
}

#[test]
fn current_boundary_suppresses_completed_event_reward_skip_by_default() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::Neow,
        current_screen: 2,
        internal_state: 0,
        completed: true,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::Panache, 0),
            RewardCard::new(CardId::Metamorphosis, 0),
            RewardCard::new(CardId::ThinkingAhead, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("visible Neow card reward boundary");

    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.kind == "card_reward_skip"),
        "completed event rewards have already committed their event choice, so skip is not a default exploration branch"
    );
}

#[test]
fn current_boundary_can_opt_into_completed_event_reward_skip() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::Neow,
        current_screen: 2,
        internal_state: 0,
        completed: true,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::Panache, 0),
            RewardCard::new(CardId::Metamorphosis, 0),
            RewardCard::new(CardId::ThinkingAhead, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: true,
        },
        None,
    )
    .expect("visible Neow card reward boundary");

    assert!(boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_skip" && option.command == "skip"));
}

#[test]
fn current_boundary_can_include_singing_bowl_card_reward_option() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::SingingBowl));
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("card reward boundary");

    let bowl = boundary
        .options
        .iter()
        .find(|option| option.kind == "card_reward_bowl")
        .expect("Singing Bowl branch should be present");

    assert_eq!(bowl.command, "bowl");
    assert_eq!(bowl.effect_kind, "singing_bowl");
    assert_eq!(bowl.effect_label, "Singing Bowl | gain 2 max HP");
    assert!(bowl.selected_cards.is_empty());
}

#[test]
fn current_boundary_expands_certified_shop_purge() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 100;
    session
        .run_state
        .add_card_to_deck_without_interception_from(
            CardId::Doubt,
            0,
            crate::state::selection::DomainEventSource::DeckMutation,
        );
    session.engine_state = EngineState::Shop(ShopState::new());

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("shop purge boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "shop_policy_purge");
    assert_eq!(boundary.options[0].command, "purge 10");
    assert_eq!(boundary.options[0].effect_kind, "shop_purge");
    assert_eq!(boundary.options[0].card, Some(CardId::Doubt));
}

#[test]
fn current_boundary_expands_shop_leave_when_no_purchase_competes() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 10;
    session.engine_state = EngineState::Shop(ShopState::new());

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("empty shop should be a branchable leave boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "shop_leave");
    assert_eq!(boundary.options[0].command, "leave");
    assert_eq!(boundary.options[0].effect_kind, "shop_leave");
}

#[test]
fn current_boundary_expands_low_fanout_shop_purchase_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 250;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::PommelStrike,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("low-fanout shop purchase boundary");

    let commands = boundary
        .options
        .iter()
        .map(|option| option.command.as_str())
        .collect::<Vec<_>>();

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert_eq!(
        commands,
        vec!["buy card 0", "buy relic 0", "buy potion 0", "leave"]
    );
    assert!(boundary
        .options
        .iter()
        .any(|option| option.effect_kind == "shop_buy_card"
            && option.card == Some(CardId::PommelStrike)));
}

#[test]
fn current_boundary_caps_high_fanout_shop_purchase_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 500;
    let mut shop = ShopState::new();
    for card_id in [
        CardId::PommelStrike,
        CardId::TwinStrike,
        CardId::ShrugItOff,
        CardId::Cleave,
        CardId::IronWave,
    ] {
        shop.cards.push(ShopCard {
            card_id,
            upgrades: 0,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
    }
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Anchor,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("high-fanout shop should keep a capped purchase portfolio");
    let effect_kinds = boundary
        .options
        .iter()
        .map(|option| option.effect_kind.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert_eq!(boundary.options.len(), 5);
    assert!(effect_kinds.contains("shop_buy_card"));
    assert!(effect_kinds.contains("shop_buy_relic"));
    assert!(effect_kinds.contains("shop_buy_potion"));
    assert!(effect_kinds.contains("shop_leave"));
    assert!(
        boundary
            .options
            .iter()
            .any(|option| option.suppressed_count > 0),
        "capped shop portfolios should expose suppressed purchase count"
    );
}

#[test]
fn current_boundary_expands_sapphire_relic_reward_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Relic {
        relic_id: RelicId::Anchor,
    });
    reward.items.push(RewardItem::SapphireKey);
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("reward key/relic boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Reward);
    assert_eq!(boundary.options.len(), 2);
    assert_eq!(boundary.options[0].kind, "reward_claim");
    assert_eq!(boundary.options[0].command, "claim 0");
    assert_eq!(boundary.options[0].effect_kind, "reward_claim");
    assert!(boundary.options[0].effect_label.contains("Relic Anchor"));
    assert_eq!(boundary.options[1].command, "claim 1");
    assert!(boundary.options[1].effect_label.contains("Sapphire key"));
}

#[test]
fn current_boundary_does_not_branch_safe_relic_reward() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Relic {
        relic_id: RelicId::Anchor,
    });
    session.engine_state = EngineState::RewardScreen(reward);

    assert!(
        current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None).is_none(),
        "safe relic rewards should stay with low-agency reward automation, not branch experiment"
    );
}

#[test]
fn current_boundary_expands_emerald_key_reward() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::EmeraldKey);
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("emerald key reward boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Reward);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "reward_claim");
    assert_eq!(boundary.options[0].command, "claim 0");
    assert!(boundary.options[0].effect_label.contains("Emerald key"));
}

#[test]
fn current_boundary_can_skip_full_slot_potion_reward_after_low_agency_claims() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.potions = vec![
        Some(Potion::new(PotionId::FirePotion, 1)),
        Some(Potion::new(PotionId::DexterityPotion, 2)),
        Some(Potion::new(PotionId::StrengthPotion, 3)),
    ];
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Potion {
        potion_id: PotionId::HeartOfIron,
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("full potion reward should expose a skip branch");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Reward);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "reward_skip");
    assert_eq!(boundary.options[0].command, "skip");
    assert_eq!(boundary.options[0].effect_kind, "reward_skip_full_potion");
    assert!(boundary.options[0].effect_label.contains("Heart"));
    assert!(boundary.options[0]
        .effect_label
        .contains("full potion slots"));
    assert!(!boundary.options[0]
        .effect_label
        .contains("replacement policy not modeled"));
}

#[test]
fn current_boundary_waits_for_low_agency_reward_before_full_potion_skip() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.potions = vec![
        Some(Potion::new(PotionId::FirePotion, 1)),
        Some(Potion::new(PotionId::DexterityPotion, 2)),
        Some(Potion::new(PotionId::StrengthPotion, 3)),
    ];
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Gold { amount: 25 });
    reward.items.push(RewardItem::Potion {
        potion_id: PotionId::HeartOfIron,
    });
    session.engine_state = EngineState::RewardScreen(reward);

    assert!(
        current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None).is_none(),
        "branch experiment should let reward automation claim deterministic gold before considering full-potion skip"
    );
}

#[test]
fn current_boundary_can_include_singing_bowl_for_unopened_card_reward_item() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::SingingBowl));
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("visible card reward boundary");

    assert!(boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_bowl" && option.command == "bowl"));
    assert!(boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_skip" && option.command == "skip"));
}

#[test]
fn current_boundary_wraps_campfire_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: None,
            max_campfire_options_per_branch: Some(2),
            include_skip: false,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("campfire boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
    assert_eq!(boundary.options.len(), 2);
    assert!(boundary
        .options
        .iter()
        .all(|option| option.kind == "campfire"));
}

#[test]
fn current_boundary_compresses_duplicate_campfire_smith_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("campfire boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
    assert_eq!(
        boundary
            .options
            .iter()
            .filter(|option| option.effect_kind == "upgrade_card")
            .map(|option| {
                (
                    option.command.as_str(),
                    option.effect_label.as_str(),
                    option.card,
                    option.upgrades,
                    option.representative_count,
                    option.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "smith 0",
                "Smith Strike",
                Some(CardId::Strike),
                Some(0),
                5,
                4
            ),
            (
                "smith 5",
                "Smith Defend",
                Some(CardId::Defend),
                Some(0),
                4,
                3
            ),
            ("smith 9", "Smith Bash", Some(CardId::Bash), Some(0), 1, 0),
        ]
    );
}

#[test]
fn current_boundary_keeps_distinct_campfire_smith_card_state_separate() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut first = CombatCard::new(CardId::RitualDagger, 10);
    first.misc_value = 17;
    let mut second = CombatCard::new(CardId::RitualDagger, 11);
    second.misc_value = 23;
    session.run_state.master_deck = vec![first, second];
    session.engine_state = EngineState::Campfire;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("campfire boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Campfire);
    assert_eq!(
        boundary
            .options
            .iter()
            .filter(|option| option.effect_kind == "upgrade_card")
            .map(|option| {
                (
                    option.command.as_str(),
                    option.effect_label.as_str(),
                    option.representative_count,
                    option.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("smith 0", "Smith Ritual Dagger", 1, 0),
            ("smith 1", "Smith Ritual Dagger", 1, 0),
        ]
    );
}

#[test]
fn current_boundary_wraps_boss_relic_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
        RelicId::BlackStar,
        RelicId::EmptyCage,
        RelicId::TinyHouse,
    ]));

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("boss relic boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::BossRelic);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (option.kind, option.command.as_str(), option.card))
            .collect::<Vec<_>>(),
        vec![
            ("boss_relic", "relic 0", None),
            ("boss_relic", "relic 1", None),
            ("boss_relic", "relic 2", None),
        ]
    );
}

#[test]
fn current_boundary_wraps_low_fanout_event_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState::new(EventId::BigFish));
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("event boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (option.kind, option.command.as_str(), option.card))
            .collect::<Vec<_>>(),
        vec![
            ("event", "event 0", None),
            ("event", "event 1", None),
            ("event", "event 2", None),
        ]
    );
}

#[test]
fn current_boundary_wraps_neow_bonus_four_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .apply_command(crate::eval::run_control::RunControlCommand::Candidate(
            "0".to_string(),
        ))
        .expect("advance to Neow bonus");

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Neow bonus should be a low-fanout event boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(boundary.options.len(), 4);
}

#[test]
fn current_boundary_allows_event_options_that_open_single_card_selection() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState::new(EventId::UpgradeShrine));
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("event boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (option.kind, option.command.as_str()))
            .collect::<Vec<_>>(),
        vec![("event", "event 0"), ("event", "event 1")]
    );
}

#[test]
fn current_boundary_wraps_single_card_run_selection_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("run selection boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (
                option.kind,
                option.command.as_str(),
                option.card,
                option.upgrades,
                option.effect_key.as_str(),
                option.effect_label.as_str(),
                option.representative_count,
                option.suppressed_count,
            ))
            .collect::<Vec<_>>(),
        vec![
            (
                "run_selection",
                "select 0",
                Some(CardId::Strike),
                Some(0),
                "run_selection:remove_card:Strike:0",
                "remove Strike",
                5,
                4,
            ),
            (
                "run_selection",
                "select 5",
                Some(CardId::Defend),
                Some(0),
                "run_selection:remove_card:Defend:0",
                "remove Defend",
                4,
                3,
            ),
            (
                "run_selection",
                "select 9",
                Some(CardId::Bash),
                Some(0),
                "run_selection:remove_card:Bash:0",
                "remove Bash",
                1,
                0,
            ),
        ]
    );
}

#[test]
fn current_boundary_keeps_distinct_run_selection_card_state_separate() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut first = CombatCard::new(CardId::RitualDagger, 10);
    first.misc_value = 17;
    let mut second = CombatCard::new(CardId::RitualDagger, 11);
    second.misc_value = 23;
    session.run_state.master_deck = vec![first, second];
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("run selection boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| {
                (
                    option.command.as_str(),
                    option.effect_label.as_str(),
                    option.representative_count,
                    option.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("select 0", "remove Ritual Dagger", 1, 0),
            ("select 1", "remove Ritual Dagger", 1, 0),
        ]
    );
}

#[test]
fn current_boundary_wraps_small_multi_card_run_selection_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck.truncate(3);
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 2,
        max_choices: 2,
        reason: RunPendingChoiceReason::Transform,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("small multi-card run selection boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| {
                (
                    option.command.as_str(),
                    option.card,
                    option.selected_cards.len(),
                    option.representative_count,
                    option.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![("select 0 1", None, 2, 3, 2)]
    );
}

#[test]
fn current_boundary_compresses_multi_card_run_selection_by_effect_key() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 2,
        max_choices: 2,
        reason: RunPendingChoiceReason::Transform,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("compressed multi-card run selection boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| {
                (
                    option.command.as_str(),
                    option.effect_label.as_str(),
                    option.representative_count,
                    option.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("select 0 1", "transform Strike x2", 10, 9),
            ("select 0 5", "transform Strike, Defend", 20, 19),
            ("select 0 9", "transform Strike, Bash", 5, 4),
            ("select 5 6", "transform Defend x2", 6, 5),
            ("select 5 9", "transform Defend, Bash", 4, 3),
        ]
    );
}

#[test]
fn current_boundary_uses_policy_representative_for_high_fanout_run_selection_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
        CombatCard::new(CardId::Bash, 12),
        CombatCard::new(CardId::TwinStrike, 13),
        CombatCard::new(CardId::PommelStrike, 14),
        CombatCard::new(CardId::Shockwave, 15),
    ];
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 2,
        max_choices: 2,
        reason: RunPendingChoiceReason::Transform,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("policy representative should keep high-fanout run selection moving");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].command, "select 0 1");
    assert!(boundary.options[0]
        .effect_label
        .contains("transform Strike, Defend"));
}

#[test]
fn campfire_branch_option_portfolio_keeps_rest_when_wounded() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.current_hp = session.run_state.max_hp - 20;
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(2)).options;

    assert!(
        selected.iter().any(|option| option.command == "rest"),
        "rest should remain represented when campfire branching is capped"
    );
    assert!(
        selected
            .iter()
            .any(|option| option.command.starts_with("smith ")),
        "at least one smith option should remain represented when campfire branching is capped"
    );
}

#[test]
fn campfire_branch_option_portfolio_does_not_spend_cap_on_full_hp_rest() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(2)).options;

    assert!(
        selected.iter().all(|option| option.command != "rest"),
        "full-hp rest should not consume capped campaign/campfire branch slots"
    );
    assert!(
        selected
            .iter()
            .any(|option| option.command.starts_with("smith ")),
        "smith options should be preferred over no-op full-hp rest"
    );
}

#[test]
fn reward_option_semantic_class_distinguishes_stabilizer_roles() {
    let shockwave = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Shockwave, 0));
    let armaments = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Armaments, 0));

    let (_, shockwave_class) = reward_option_semantic_class(&shockwave);
    let (_, armaments_class) = reward_option_semantic_class(&armaments);

    assert_ne!(
        shockwave_class, armaments_class,
        "control/debuff stabilizers and plain block/upgrade stabilizers should not collapse into one option class"
    );
}
