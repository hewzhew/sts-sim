use super::*;
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
fn early_shop_exposes_gold_convertibility_term() {
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

    let gold = trace
        .candidates
        .iter()
        .find(|candidate| candidate.index == 0)
        .expect("gold candidate should be present");
    assert!(gold.terms.shop_convertibility > 0.0);
}

#[test]
fn lament_elite_snipe_exposes_first_elite_security_term() {
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

    let lament = trace
        .candidates
        .iter()
        .find(|candidate| candidate.index == 0)
        .expect("lament candidate should be present");
    assert!(lament.terms.first_elite_security > 0.0);
}

#[test]
fn boss_swap_candidate_records_starter_relic_downside() {
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

    let boss_swap = trace
        .candidates
        .iter()
        .find(|candidate| candidate.index == 1)
        .expect("boss swap candidate should be present");
    assert!(boss_swap.terms.downside_cost < 0.0);
}

#[test]
fn neow_remove_one_followup_emits_one_legal_selection_command() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Purge, 1),
        "Ironclad",
    )
    .expect("remove one should be handled");

    assert!(decision.command.starts_with("select "));
    assert_eq!(decision.selected_cards.len(), 1);
}

#[test]
fn neow_followup_selection_is_sourced_from_deck_mutation_compiler() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Purge, 1),
        "Ironclad",
    )
    .expect("remove one should be handled");

    assert_eq!(decision.selection_mode, "deck_mutation_compiler_v1");
}

#[test]
fn neow_transform_two_followup_emits_two_legal_selection_commands() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Transform, 2),
        "Ironclad",
    )
    .expect("transform two should be handled");

    assert!(decision.command.starts_with("select "));
    assert_eq!(decision.selected_cards.len(), 2);
}

#[test]
fn neow_upgrade_one_followup_emits_one_legal_selection_command() {
    let run_state = RunState::new(1, 0, false, "Ironclad");
    let decision = neow_followup_selection_v1(
        &run_state,
        &pending_choice(RunPendingChoiceReason::Upgrade, 1),
        "Ironclad",
    )
    .expect("upgrade one should be handled");

    assert!(decision.command.starts_with("select "));
    assert_eq!(decision.selected_cards.len(), 1);
}
