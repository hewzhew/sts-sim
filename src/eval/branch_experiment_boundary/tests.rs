use std::collections::BTreeSet;

use super::card_reward::select_card_reward_branch_options_with_limit;
use super::*;
use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::content::cards::CardId;
use crate::content::potions::{Potion, PotionId};
use crate::content::relics::{RelicId, RelicState};
use crate::eval::run_control::{RunControlConfig, RunControlSession};
use crate::runtime::action::CardDestination;
use crate::runtime::combat::CombatCard;
use crate::state::core::{
    EngineState, PendingChoice, RunPendingChoiceReason, RunPendingChoiceState,
};
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
fn card_reward_portfolio_without_strategy_preserves_input_order_across_classes() {
    let options = vec![
        super::card_reward::CardRewardBranchOption {
            label: "Twin Strike".to_string(),
            command: "rp 0".to_string(),
            card: Some(CardId::TwinStrike),
            upgrades: Some(0),
            source: super::card_reward::CardRewardBranchOptionSource::PermanentReward,
            decision_signal: None,
        },
        super::card_reward::CardRewardBranchOption {
            label: "Body Slam".to_string(),
            command: "rp 1".to_string(),
            card: Some(CardId::BodySlam),
            upgrades: Some(0),
            source: super::card_reward::CardRewardBranchOptionSource::PermanentReward,
            decision_signal: None,
        },
        super::card_reward::CardRewardBranchOption {
            label: "Shrug It Off".to_string(),
            command: "rp 2".to_string(),
            card: Some(CardId::ShrugItOff),
            upgrades: Some(0),
            source: super::card_reward::CardRewardBranchOptionSource::PermanentReward,
            decision_signal: None,
        },
    ];

    let selected = select_card_reward_branch_options_with_limit(options, 2, None).options;
    let labels = selected
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        labels,
        vec!["Twin Strike", "Body Slam"],
        "without strategic context, semantic classes should diversify the input order rather than impose package/stabilizer value priority"
    );
}

#[test]
fn card_reward_option_portfolio_includes_decline_candidates() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::SingingBowl));
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: Some(2),
            max_campfire_options_per_branch: None,
            include_skip: true,
            include_event_reward_skip: false,
        },
        Some(CardRewardPortfolioContext {
            depth: 0,
            frontier_key: "frontier".to_string(),
            boundary_title: "Card Reward".to_string(),
        }),
    )
    .expect("card reward boundary");

    let portfolio = boundary
        .reward_option_portfolio
        .expect("capped card reward boundary should emit a portfolio report");
    let labels = portfolio
        .selected_options
        .iter()
        .chain(portfolio.pruned_options.iter())
        .map(|entry| entry.label.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(portfolio.original_count, 4);
    assert!(labels.contains("Singing Bowl | gain 2 max HP"));
    assert!(!labels.contains("Skip card reward"));
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
fn current_boundary_expands_combat_card_reward_select_with_choose_commands() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::PendingChoice(PendingChoice::CardRewardSelect {
        cards: vec![
            CardId::Transmutation,
            CardId::DarkShackles,
            CardId::JackOfAllTrades,
        ],
        destination: CardDestination::Hand,
        can_skip: false,
    });

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
            frontier_key: "combat-card-reward".to_string(),
            boundary_title: "Combat card reward".to_string(),
        }),
    )
    .expect("combat card reward boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::CardReward);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| option.command.as_str())
            .collect::<Vec<_>>(),
        vec!["choose 0", "choose 1", "choose 2"],
        "Toolbox-style combat reward choices are tactical combat branches, not rp card picks"
    );
    assert!(
        boundary.reward_option_portfolio.is_none(),
        "combat reward choices should not be capped by the macro card reward portfolio limit"
    );
    assert!(boundary
        .options
        .iter()
        .all(|option| option.kind == "combat_card_reward"));
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
        "opened card reward back/cancel does not consume the reward; branch experiment must not model it as a consumed skip"
    );
}

#[test]
fn current_boundary_does_not_include_map_preview_skip_for_unopened_card_reward_item() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.skippable = true;
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

    assert!(!boundary
        .options
        .iter()
        .any(|option| option.command == "skip"));
    let skip = boundary
        .options
        .iter()
        .find(|option| option.kind == "card_reward_skip")
        .expect("branch experiment may synthesize a true card-reward skip branch");
    assert_eq!(skip.command, "branch-skip-card-reward 0");
}

#[test]
fn current_boundary_uses_admission_to_keep_boss_answer_over_bloated_goodstuff() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 46;
    for _ in 0..34 {
        session.run_state.add_card_to_deck(CardId::Strike);
    }
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::PommelStrike, 0),
            RewardCard::new(CardId::Shockwave, 0),
            RewardCard::new(CardId::TwinStrike, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: Some(1),
            max_campfire_options_per_branch: None,
            include_skip: false,
            include_event_reward_skip: false,
        },
        None,
    )
    .expect("visible card reward boundary");

    assert!(
        boundary.options.iter().any(|option| {
            option.kind == "card_reward" && option.card == Some(CardId::Shockwave)
        }),
        "admission pressure should preserve a clear boss/elite answer over draw-one goodstuff"
    );
    assert!(
        !boundary.options.iter().any(|option| {
            option.kind == "card_reward" && option.card == Some(CardId::PommelStrike)
        }),
        "rejected goodstuff should not occupy the only card reward branch slot"
    );
    assert!(!boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_skip"));
}

#[test]
fn current_boundary_does_not_offer_unexecutable_card_skip_in_reward_overlay() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.skippable = true;
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::BattleTrance, 0),
            RewardCard::new(CardId::Armaments, 1),
            RewardCard::new(CardId::GhostlyArmor, 0),
        ],
    });
    session.engine_state = EngineState::RewardOverlay {
        reward_state: reward,
        return_state: Box::new(EngineState::Shop(ShopState::new())),
    };

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
    .expect("overlay card reward boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::CardReward);
    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.kind == "card_reward_skip"),
        "RewardOverlay has no single visible command that consumes one unopened card reward as a skip"
    );
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

    let skip = boundary
        .options
        .iter()
        .find(|option| option.kind == "card_reward_skip")
        .expect("explicit opt-in should expose the synthetic card reward skip branch");
    assert_eq!(skip.command, "branch-skip-card-reward 0");
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
    assert!(boundary.options.iter().any(|option| {
        option.kind == "shop_policy_purge"
            && option.command == "purge 10"
            && option.effect_kind == "shop_purge"
            && option.card == Some(CardId::Doubt)
    }));
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
        relic_id: RelicId::FrozenEye,
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

    let mut commands = boundary
        .options
        .iter()
        .map(|option| option.command.as_str())
        .collect::<Vec<_>>();
    commands.sort_unstable();

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert!(!commands.is_empty());
    assert!(commands.len() <= 4);
    assert!(commands
        .iter()
        .all(|command| command.starts_with("buy ") || command.starts_with("purge ")));
    assert!(commands.contains(&"buy potion 0") || commands.contains(&"buy relic 0"));
}

#[test]
fn current_boundary_caps_high_fanout_shop_purchase_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.floor_num = 6;
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
        relic_id: RelicId::FrozenEye,
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
    assert!(!boundary.options.is_empty());
    assert!(boundary.options.len() <= 4);
    assert!(effect_kinds.contains("shop_buy_relic"));
    assert!(effect_kinds.contains("shop_buy_potion"));
    assert!(!effect_kinds.contains("shop_leave"));
}

#[test]
fn current_boundary_includes_combo_purchase_for_high_pressure_shop() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.floor_num = 6;
    session.run_state.gold = 631;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Shockwave,
        upgrades: 0,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::FlameBarrier,
        upgrades: 0,
        price: 90,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::FrozenEye,
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
        .expect("high-pressure shop should expose a purchase portfolio");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Shop);
    assert!(
        boundary
            .options
            .iter()
            .any(|option| option.effect_kind == "shop_buy_combo"
                && option.command.contains(" && ")),
        "high-pressure shops should expose compact multi-purchase portfolio branches"
    );
}

#[test]
fn current_boundary_includes_three_purchase_combo_for_high_gold_shop_pressure() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 46;
    session.run_state.gold = 430;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Shockwave,
        upgrades: 0,
        price: 90,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::FrozenEye,
        price: 146,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::DuplicationPotion,
        price: 60,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("high-gold shop should expose a purchase portfolio");

    assert!(
        boundary.options.iter().any(|option| {
            option.effect_kind == "shop_buy_combo" && option.command.matches(" && ").count() >= 2
        }),
        "high-gold boss-prep shops should expose a compact three-purchase branch"
    );
}

#[test]
fn current_boundary_includes_combo_purchase_for_capped_affordable_shop() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 2;
    session.run_state.floor_num = 25;
    session.run_state.gold = 220;
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
        relic_id: RelicId::FrozenEye,
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
        .expect("capped affordable shop should expose a compact purchase portfolio");

    assert!(
        boundary
            .options
            .iter()
            .any(|option| option.effect_kind == "shop_buy_combo"
                && option.command.contains(" && ")),
        "deep affordable shops should expose a compact two-purchase branch even below the high-gold threshold"
    );
}

#[test]
fn current_boundary_suppresses_shop_leave_for_high_impact_affordable_relic() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 200;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Orrery,
        price: 180,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("high-impact shop relic should create purchase pressure");
    let commands = boundary
        .options
        .iter()
        .map(|option| option.command.as_str())
        .collect::<Vec<_>>();

    assert!(commands.contains(&"buy relic 0"));
    assert!(!commands.contains(&"leave"));
}

#[test]
fn current_boundary_prefers_boss_potion_over_smoke_bomb_in_capped_shop_portfolio() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.gold = 500;
    let mut shop = ShopState::new();
    for card_id in [
        CardId::TwinStrike,
        CardId::Cleave,
        CardId::IronWave,
        CardId::WildStrike,
        CardId::Clothesline,
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
        relic_id: RelicId::FrozenEye,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::SmokeBomb,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::DuplicationPotion,
        price: 60,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("high-fanout shop should keep a capped purchase portfolio");

    assert!(
        boundary
            .options
            .iter()
            .any(|option| option.effect_kind == "shop_buy_potion"
                && option.command == "buy potion 1"),
        "capped shop potion representative should be the combat-relevant boss potion"
    );
    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.effect_kind == "shop_buy_potion"
                && option.command == "buy potion 0"),
        "Smoke Bomb should not crowd out a boss-fight potion representative"
    );
}

#[test]
fn current_boundary_does_not_branch_on_shop_potion_when_slots_are_full() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 200;
    session.run_state.potions = vec![
        Some(Potion::new(PotionId::BlockPotion, 0)),
        Some(Potion::new(PotionId::StrengthPotion, 1)),
        Some(Potion::new(PotionId::DexterityPotion, 2)),
    ];
    let mut shop = ShopState::new();
    shop.potions.push(ShopPotion {
        potion_id: PotionId::StrengthPotion,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("full potion slots should still allow leaving the shop");
    let commands = boundary
        .options
        .iter()
        .map(|option| option.command.as_str())
        .collect::<Vec<_>>();

    assert!(!commands
        .iter()
        .any(|command| command.starts_with("buy potion")));
}

#[test]
fn current_boundary_does_not_branch_late_act3_bloated_shop_goodstuff_cards() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 39;
    session.run_state.gold = 999;
    for _ in 0..35 {
        session.run_state.add_card_to_deck(CardId::Strike);
    }
    let mut shop = ShopState::new();
    for card_id in [
        CardId::BodySlam,
        CardId::HeavyBlade,
        CardId::Havoc,
        CardId::Metallicize,
    ] {
        shop.cards.push(ShopCard {
            card_id,
            upgrades: 0,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
    }
    session.engine_state = EngineState::Shop(shop);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("late bloated shop should still expose leave");

    assert!(
        boundary
            .options
            .iter()
            .all(|option| option.effect_kind != "shop_buy_card"),
        "late Act3 shop should not keep low-impact goodstuff card buys for a bloated deck"
    );
    assert!(boundary.options.iter().any(|option| {
        option.effect_kind == "shop_leave" || option.effect_kind == "shop_purge"
    }));
}

#[test]
fn current_boundary_suppresses_nloth_energy_relic_trade() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::PhilosopherStone));
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::OldCoin));
    session.run_state.event_state = Some(EventState {
        id: EventId::Nloth,
        current_screen: 0,
        internal_state: 1 | (2 << 8),
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("N'loth should still expose safe event branches");
    let labels = boundary
        .options
        .iter()
        .map(|option| option.label.as_str())
        .collect::<Vec<_>>();

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert!(
        !labels
            .iter()
            .any(|label| label.contains("PhilosopherStone")),
        "N'loth must not branch through a protected energy relic trade"
    );
    assert!(labels.iter().any(|label| label.contains("OldCoin")));
    assert!(labels.iter().any(|label| label.contains("Leave")));
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
fn current_boundary_can_skip_sozu_blocked_potion_reward() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::Sozu));
    session.run_state.potions = vec![
        Some(Potion::new(PotionId::FirePotion, 1)),
        None,
        Some(Potion::new(PotionId::StrengthPotion, 3)),
    ];
    let mut reward = RewardState::new();
    reward.items.push(RewardItem::Potion {
        potion_id: PotionId::SpeedPotion,
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Sozu-blocked potion reward should expose a skip branch");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Reward);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].kind, "reward_skip");
    assert_eq!(boundary.options[0].command, "claim 0");
    assert_eq!(
        boundary.options[0].effect_kind,
        "reward_skip_blocked_potion"
    );
    assert_eq!(
        boundary.options[0].effect_key,
        "reward:skip_sozu_blocked_potion"
    );
    assert!(boundary.options[0].effect_label.contains("Speed"));
    assert!(boundary.options[0].effect_label.contains("Sozu blocks"));
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
fn current_boundary_can_include_singing_bowl_but_not_skip_for_unopened_card_reward_item() {
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
    assert!(!boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_skip"));
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
    assert!(!boundary.options.is_empty());
    assert!(boundary.options.len() <= 2);
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
    let mut upgrade_options = boundary
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
        .collect::<Vec<_>>();
    upgrade_options.sort_by(|left, right| left.0.cmp(right.0));
    assert_eq!(
        upgrade_options,
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
fn campfire_toke_branch_options_use_deck_mutation_compiler_roles() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::PeacePipe));
    session
        .run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 99));
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let toke_cards = options
        .iter()
        .filter(|option| option.effect_kind == "remove_card")
        .map(|option| option.card)
        .collect::<Vec<_>>();

    assert!(toke_cards.contains(&Some(CardId::Strike)));
    assert!(toke_cards.contains(&Some(CardId::Defend)));
    assert!(
        !toke_cards.contains(&Some(CardId::TrueGrit)),
        "functional toke targets should not consume active campfire branch slots while low-value targets exist"
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
fn current_boundary_routes_direct_event_removes_through_deck_mutation_compiler() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck = vec![
        CombatCard::new(CardId::SecondWind, 100),
        CombatCard::new(CardId::Evolve, 101),
        CombatCard::new(CardId::Cleave, 102),
    ];
    session.run_state.master_deck[0].upgrades = 1;
    session.run_state.master_deck[1].upgrades = 1;
    session.run_state.event_state = Some(EventState {
        id: EventId::Falling,
        current_screen: 1,
        internal_state: (0 & 0x3FF) | ((1 & 0x3FF) << 10) | ((2 & 0x3FF) << 20),
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Falling should expose direct remove options");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (
                option.command.as_str(),
                option.effect_kind.as_str(),
                option.card,
                option.upgrades,
            ))
            .collect::<Vec<_>>(),
        vec![
            ("event 2", "remove_card", Some(CardId::Cleave), Some(0)),
            ("event 0", "remove_card", Some(CardId::SecondWind), Some(1)),
            ("event 1", "remove_card", Some(CardId::Evolve), Some(1)),
        ],
        "direct event removals should be sorted by the deck mutation compiler, not raw event UI order"
    );
    assert!(boundary
        .options
        .iter()
        .all(|option| option.effect_label.contains("deck mutation role=")));
}

#[test]
fn current_boundary_caps_library_card_offer_with_card_semantics() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::TheLibrary,
        current_screen: 1,
        internal_state: 0,
        completed: false,
        combat_pending: false,
        extra_data: [
            CardId::Havoc,
            CardId::ShrugItOff,
            CardId::PommelStrike,
            CardId::DemonForm,
            CardId::FlameBarrier,
            CardId::Clash,
        ]
        .into_iter()
        .flat_map(|card| [card as i32, 0])
        .collect(),
    });
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(
        &session,
        BranchBoundaryConfigV1 {
            max_reward_options_per_branch: Some(4),
            ..BranchBoundaryConfigV1::default()
        },
        None,
    )
    .expect("The Library card offer should use a capped event card portfolio");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(boundary.options.len(), 4);
    assert!(boundary
        .options
        .iter()
        .all(|option| option.effect_kind == "event_card_reward"));
    assert!(boundary
        .options
        .iter()
        .all(|option| option.selected_cards.len() == 1));
}

#[test]
fn current_boundary_classifies_gold_plus_curse_event_as_curse_debt() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 38;
    session.run_state.event_state = Some(EventState::new(EventId::MindBloom));
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Mind Bloom should be a branchable event");
    let desire = boundary
        .options
        .iter()
        .find(|option| option.label.contains("Normality"))
        .expect("low-floor Mind Bloom Desire should be visible");

    assert_eq!(desire.effect_kind, "event_gain_curse");
    assert!(desire.effect_key.contains("Normality"));
}

#[test]
fn current_boundary_does_not_branch_terminal_single_event_leave_screen() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::Beggar,
        current_screen: 2,
        internal_state: 0,
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    session.engine_state = EngineState::EventRoom;

    assert!(
        current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None).is_none(),
        "single terminal no-effect event leave screens should remain auto-advanceable, not become branch points"
    );
}

#[test]
fn current_boundary_uses_event_policy_safe_exit_for_optional_combat_event() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 45;
    session.run_state.current_hp = 71;
    session.run_state.max_hp = 90;
    session.run_state.event_state = Some(EventState::new(EventId::MysteriousSphere));
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Mysterious Sphere should expose a resolved event boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(
        boundary
            .options
            .iter()
            .map(|option| (option.command.as_str(), option.effect_kind.as_str()))
            .collect::<Vec<_>>(),
        vec![("event 1", "event_leave")],
        "branch boundaries should consume the central event policy safe-exit autopilot pick instead of preserving the optional high-risk fight as an equal branch"
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
    let mut options = boundary
        .options
        .iter()
        .map(|option| (option.kind, option.command.as_str()))
        .collect::<Vec<_>>();
    options.sort();
    assert_eq!(options, vec![("event", "event 0"), ("event", "event 1")]);
}

#[test]
fn current_boundary_reads_event_card_upgrade_state_from_event_data() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut event_state = EventState::new(EventId::TheLibrary);
    event_state.current_screen = 1;
    event_state.extra_data = vec![CardId::ShrugItOff as i32, 2];
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("Library card choice should be an event boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::Event);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].card, Some(CardId::ShrugItOff));
    assert_eq!(
        boundary.options[0].upgrades,
        Some(2),
        "event card branch metadata should come from structured event state, not the UI '+' suffix"
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
fn current_boundary_does_not_expand_nonbasic_purge_when_low_value_targets_exist() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .master_deck
        .push(CombatCard::new(CardId::TrueGrit, 99));
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::EventRoom),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("run selection boundary");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    let commands = boundary
        .options
        .iter()
        .map(|option| (option.command.as_str(), option.card))
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        vec![
            ("select 0", Some(CardId::Strike)),
            ("select 5", Some(CardId::Defend)),
            ("select 9", Some(CardId::Bash)),
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
    let mut options = boundary
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
        .collect::<Vec<_>>();
    options.sort_by(|left, right| left.0.cmp(right.0));
    assert_eq!(
        options,
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
fn current_boundary_uses_compiler_representative_for_high_fanout_run_selection_options() {
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
        .expect("compiler representative should keep high-fanout run selection moving");

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(boundary.options.len(), 1);
    assert_eq!(boundary.options[0].command, "select 0 1");
    assert!(boundary.options[0]
        .effect_label
        .contains("transform Strike, Defend"));
}

#[test]
fn current_boundary_keeps_high_value_duplicate_targets_when_fanout_is_high() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
        CombatCard::new(CardId::Bash, 12),
        CombatCard::new(CardId::TwinStrike, 13),
        CombatCard::new(CardId::PommelStrike, 14),
        CombatCard::new(CardId::Shockwave, 15),
        CombatCard::new(CardId::Offering, 16),
        CombatCard::new(CardId::Corruption, 17),
        CombatCard::new(CardId::ShrugItOff, 18),
        CombatCard::new(CardId::TrueGrit, 19),
        CombatCard::new(CardId::Uppercut, 20),
        CombatCard::new(CardId::Disarm, 21),
        CombatCard::new(CardId::FlameBarrier, 22),
        CombatCard::new(CardId::Cleave, 23),
        CombatCard::new(CardId::Armaments, 24),
    ];
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Duplicate,
        return_state: Box::new(EngineState::Shop(ShopState::new())),
    });

    let boundary = current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None)
        .expect("duplicate portfolio should keep high-fanout duplicate choice moving");
    let cards = boundary
        .options
        .iter()
        .map(|option| option.card)
        .collect::<Vec<_>>();

    assert_eq!(boundary.id, BranchBoundaryIdV1::RunSelection);
    assert_eq!(boundary.options.len(), 4);
    assert!(cards.contains(&Some(CardId::Offering)));
    assert!(cards.contains(&Some(CardId::Corruption)));
    assert!(cards.contains(&Some(CardId::Shockwave)));
    assert!(!cards.contains(&Some(CardId::Strike)));
    assert!(!cards.contains(&Some(CardId::Defend)));
}

#[test]
fn campfire_branch_option_portfolio_does_not_spend_cap_on_minor_rest() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.current_hp = session.run_state.max_hp - 4;
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(2)).options;

    assert!(
        selected.iter().all(|option| option.command != "rest"),
        "minor HP recovery should not consume capped campaign/campfire branch slots when smith targets exist"
    );
    assert!(
        selected
            .iter()
            .any(|option| option.command.starts_with("smith ")),
        "smith options should remain represented when campfire branching is capped"
    );
}

#[test]
fn campfire_branch_option_portfolio_keeps_rest_under_recovery_pressure() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.current_hp = 20;
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(2)).options;

    assert!(
        selected.iter().any(|option| option.command == "rest"),
        "low-HP recovery should remain represented when campfire branching is capped"
    );
    assert!(
        selected
            .iter()
            .any(|option| option.command.starts_with("smith ")),
        "at least one smith option should remain represented alongside recovery"
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
fn campfire_branch_option_portfolio_keeps_full_hp_rest_when_it_is_the_only_exit() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::FusionHammer));
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(2)).options;

    assert_eq!(
        selected
            .iter()
            .map(|option| option.command.as_str())
            .collect::<Vec<_>>(),
        vec!["rest"],
        "full-hp rest is normally filtered, but it must remain when it is the only campfire exit"
    );
}

#[test]
fn campfire_branch_option_portfolio_prefers_bash_over_starter_filler_when_tightly_capped() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(1)).options;

    assert_eq!(
        selected
            .iter()
            .map(|option| option.command.as_str())
            .collect::<Vec<_>>(),
        vec!["smith 9"]
    );
    assert_eq!(selected[0].card, Some(CardId::Bash));
}

#[test]
fn campfire_branch_option_portfolio_prefers_automaton_boss_answers() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 2;
    session.run_state.floor_num = 31;
    session.run_state.boss_key = Some(crate::content::monsters::factory::EncounterId::Automaton);
    session.run_state.current_hp = 17;
    session.run_state.max_hp = 40;
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::PeacePipe));
    session.run_state.master_deck = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Bash, 11),
        CombatCard::new(CardId::Apparition, 12),
        CombatCard::new(CardId::Impervious, 13),
        CombatCard::new(CardId::PommelStrike, 14),
    ];
    session.engine_state = EngineState::Campfire;

    let options = campfire_branch_options(&session).expect("campfire options");
    let selected = select_campfire_branch_options(options, Some(3)).options;
    let selected_cards = selected
        .iter()
        .map(|option| option.card)
        .collect::<Vec<_>>();

    assert!(
        selected.iter().any(|option| option.command == "rest"),
        "low-hp pre-Automaton campfire should keep the recovery branch"
    );
    assert!(
        selected_cards.contains(&Some(CardId::Apparition)),
        "Apparition upgrade is a concrete Automaton survival branch"
    );
    assert!(
        selected_cards.contains(&Some(CardId::Impervious)),
        "big block upgrade is a concrete Hyperbeam branch"
    );
    assert!(
        selected
            .iter()
            .all(|option| !option.command.starts_with("toke ")),
        "tight pre-boss budget should not spend a slot on Peace Pipe before core boss answers"
    );
}

#[test]
fn reward_option_semantic_class_distinguishes_stabilizer_roles() {
    let shockwave = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Shockwave, 0));
    let armaments = card_reward_semantic_profile_v1(&RewardCard::new(CardId::Armaments, 0));

    let shockwave_class = reward_option_semantic_class(&shockwave);
    let armaments_class = reward_option_semantic_class(&armaments);

    assert_ne!(
        shockwave_class, armaments_class,
        "control/debuff stabilizers and plain block/upgrade stabilizers should not collapse into one option class"
    );
}
