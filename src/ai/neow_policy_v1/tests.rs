use super::*;
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventEffect, EventRelicKind};
use crate::state::run::RunState;

fn choice(index: usize, label: &str, effects: Vec<EventEffect>) -> NeowChoiceInputV1 {
    NeowChoiceInputV1::from_effects(index, label, effects)
}

fn pending_choice(reason: RunPendingChoiceReason, count: usize) -> RunPendingChoiceState {
    RunPendingChoiceState {
        min_choices: count,
        max_choices: count,
        reason,
        return_state: Box::new(EngineState::EventRoom),
    }
}

#[test]
fn early_shop_makes_gold_convertible() {
    let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
        player_class: "Ironclad".to_string(),
        map: NeowMapFeaturesV1 {
            early_shop_available: true,
            shop_before_first_elite: true,
            ..NeowMapFeaturesV1::default()
        },
        choices: vec![
            choice(0, "Obtain 100 Gold.", vec![EventEffect::GainGold(100)]),
            choice(
                1,
                "Obtain a random common relic.",
                vec![EventEffect::ObtainRelic {
                    count: 1,
                    kind: EventRelicKind::RandomCommonRelic,
                }],
            ),
        ],
        config: NeowGuidanceConfigV1::default(),
    });

    assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
    assert!(trace.selected().unwrap().terms.shop_convertibility > 0.0);
}

#[test]
fn lament_is_prioritized_when_it_can_snipe_an_elite() {
    let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
        player_class: "Ironclad".to_string(),
        map: NeowMapFeaturesV1 {
            lament_elite_snipe_candidate: true,
            early_elite_available: true,
            ..NeowMapFeaturesV1::default()
        },
        choices: vec![
            choice(
                0,
                "Enemies in your next three combats have 1 HP.",
                vec![EventEffect::ObtainRelic {
                    count: 1,
                    kind: EventRelicKind::Specific(RelicId::NeowsLament),
                }],
            ),
            choice(1, "Max HP +8", vec![EventEffect::GainMaxHp(8)]),
        ],
        config: NeowGuidanceConfigV1::default(),
    });

    assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
    assert!(trace.selected().unwrap().terms.first_elite_security > 0.0);
}

#[test]
fn ironclad_does_not_default_to_boss_swap_when_stable_options_exist() {
    let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
        player_class: "Ironclad".to_string(),
        map: NeowMapFeaturesV1 {
            path_flexibility: 0.25,
            ..NeowMapFeaturesV1::default()
        },
        choices: vec![
            choice(
                0,
                "Obtain a random common relic.",
                vec![EventEffect::ObtainRelic {
                    count: 1,
                    kind: EventRelicKind::RandomCommonRelic,
                }],
            ),
            choice(
                1,
                "Obtain a random Boss Relic. Lose your starter Relic.",
                vec![
                    EventEffect::LoseStarterRelic { specific: None },
                    EventEffect::ObtainRelic {
                        count: 1,
                        kind: EventRelicKind::RandomBossRelic,
                    },
                ],
            ),
        ],
        config: NeowGuidanceConfigV1::default(),
    });

    assert_eq!(trace.selected().map(|choice| choice.index), Some(0));
    assert!(trace.candidates[1].terms.downside_cost < 0.0);
}

#[test]
fn ironclad_neow_remove_one_selects_strike() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Purge, 1),
        "Ironclad",
    )
    .expect("remove one should be handled");

    assert_eq!(decision.command, "select 0");
    assert_eq!(decision.selected_cards, vec![(CardId::Strike, 0)]);
}

#[test]
fn ironclad_neow_transform_two_selects_strike_and_defend() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Transform, 2),
        "Ironclad",
    )
    .expect("transform two should be handled");

    assert_eq!(decision.command, "select 0 5");
    assert_eq!(
        decision.selected_cards,
        vec![(CardId::Strike, 0), (CardId::Defend, 0)]
    );
}

#[test]
fn ironclad_neow_upgrade_one_selects_bash() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Upgrade, 1),
        "Ironclad",
    )
    .expect("upgrade one should be handled");

    assert_eq!(decision.command, "select 9");
    assert_eq!(decision.selected_cards, vec![(CardId::Bash, 0)]);
}
