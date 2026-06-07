use std::collections::BTreeSet;

use super::card_reward::select_card_reward_branch_options_with_limit;
use super::*;
use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::content::cards::CardId;
use crate::content::relics::{RelicId, RelicState};
use crate::eval::run_control::{RunControlConfig, RunControlSession};
use crate::runtime::combat::CombatCard;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{BossRelicChoiceState, RewardCard, RewardItem, RewardState};

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
fn current_boundary_can_include_card_reward_skip_option() {
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
        },
        None,
    )
    .expect("card reward boundary");

    let skip = boundary
        .options
        .iter()
        .find(|option| option.kind == "card_reward_skip")
        .expect("skip branch should be present");

    assert_eq!(skip.command, "skip");
    assert_eq!(skip.effect_kind, "skip_card_reward");
    assert!(skip.selected_cards.is_empty());
}

#[test]
fn current_boundary_does_not_skip_unopened_card_reward_item() {
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
        },
        None,
    )
    .expect("visible card reward boundary");

    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.kind == "card_reward_skip"),
        "skip is only a card-reward branch after the card reward is opened"
    );
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
        },
        None,
    )
    .expect("visible card reward boundary");

    assert!(boundary
        .options
        .iter()
        .any(|option| option.kind == "card_reward_bowl" && option.command == "bowl"));
    assert!(
        !boundary
            .options
            .iter()
            .any(|option| option.kind == "card_reward_skip"),
        "plain skip is still not a direct unopened-card-reward branch"
    );
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
fn current_boundary_still_rejects_high_fanout_distinct_multi_card_run_selection_options() {
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

    assert!(
        current_branch_boundary(&session, BranchBoundaryConfigV1::default(), None).is_none(),
        "multi-card run selection should still stop when semantic combinations exceed the branch cap"
    );
}

#[test]
fn campfire_branch_option_portfolio_keeps_rest_and_smith_classes() {
    let mut session = RunControlSession::new(RunControlConfig::default());
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
