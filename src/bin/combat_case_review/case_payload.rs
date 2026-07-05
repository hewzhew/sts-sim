use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatDeficitEvidenceReport, CombatLineLabReport, CombatSearchV2WitnessReplay,
};
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit, DeckStrategicDeficit,
};
use sts_simulator::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_simulator::content::cards::{get_card_definition, is_starter_basic, CardType};
use sts_simulator::content::relics::energy_master_delta;
use sts_simulator::eval::combat_case::{
    card_summary, CombatCase, CombatCaseCardSummary, CombatCasePathStep,
};

use super::boss_pressure_lens::BossPressureLensReport;
use super::champ_phase::ChampPhaseAudit;
use super::classification::CombatGapReviewClassification;
use super::counterfactual_hp::CounterfactualHpProbe;
use super::focus::{CombatReviewFocus, CombatReviewFocusPriorRerun};
use super::key_card_lifecycle::{key_card_lifecycle, KeyCardLifecycleReport};
use super::quality_lanes::CombatQualityLaneReview;
use super::search_types::SearchReview;
use super::strategic_feedback::{combat_strategic_feedback, CombatStrategicFeedbackReport};

#[derive(Serialize)]
pub(super) struct CombatCaseReview {
    schema: &'static str,
    case_path: String,
    source: sts_simulator::eval::combat_case::CombatCaseSource,
    gap: sts_simulator::eval::combat_case::CombatCaseGap,
    run: sts_simulator::eval::combat_case::CombatCaseRunSummary,
    combat: sts_simulator::eval::combat_case::CombatCaseCombatSummary,
    deck: Vec<CombatCaseCardSummary>,
    static_strategic_deficit: DeckStrategicDeficit,
    relics: Vec<String>,
    potions: Vec<Option<String>>,
    path_tail: Vec<CombatCasePathStep>,
    saved_search: Option<sts_simulator::eval::run_control::CombatSearchTraceSummary>,
    ladder: Vec<SearchReview>,
    classification: CombatGapReviewClassification,
    review_focus: Option<CombatReviewFocus>,
    review_focus_replay: Option<CombatSearchV2WitnessReplay>,
    review_focus_prior_rerun: Option<CombatReviewFocusPriorRerun>,
    line_lab: Option<CombatLineLabReport>,
    quality_lanes: Option<CombatQualityLaneReview>,
    counterfactual_hp_probe: Option<CounterfactualHpProbe>,
    combat_deficit_evidence: Option<CombatDeficitEvidenceReport>,
    combat_strategic_feedback: Option<CombatStrategicFeedbackReport>,
    boss_pressure_lens: Option<BossPressureLensReport>,
    champ_phase_audit: Option<ChampPhaseAudit>,
    key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

pub(super) struct CombatCaseReviewArtifacts {
    pub(super) ladder: Vec<SearchReview>,
    pub(super) classification: CombatGapReviewClassification,
    pub(super) review_focus: Option<CombatReviewFocus>,
    pub(super) review_focus_replay: Option<CombatSearchV2WitnessReplay>,
    pub(super) review_focus_prior_rerun: Option<CombatReviewFocusPriorRerun>,
    pub(super) line_lab: Option<CombatLineLabReport>,
    pub(super) quality_lanes: Option<CombatQualityLaneReview>,
    pub(super) counterfactual_hp_probe: Option<CounterfactualHpProbe>,
    pub(super) combat_deficit_evidence: Option<CombatDeficitEvidenceReport>,
    pub(super) boss_pressure_lens: Option<BossPressureLensReport>,
    pub(super) champ_phase_audit: Option<ChampPhaseAudit>,
}

pub(super) fn assemble_combat_case_review(
    case_path: String,
    case: CombatCase,
    artifacts: CombatCaseReviewArtifacts,
) -> CombatCaseReview {
    let CombatCaseReviewArtifacts {
        ladder,
        classification,
        review_focus,
        review_focus_replay,
        review_focus_prior_rerun,
        line_lab,
        quality_lanes,
        counterfactual_hp_probe,
        combat_deficit_evidence,
        boss_pressure_lens,
        champ_phase_audit,
    } = artifacts;
    let static_strategic_deficit = assess_deck_strategic_deficit(
        &case.position.combat.meta.master_deck_snapshot,
        strategic_facts_from_case(&case),
    );
    let combat_strategic_feedback = combat_strategic_feedback(
        &case,
        &static_strategic_deficit,
        &classification,
        review_focus.as_ref(),
        &ladder,
    );
    let key_card_lifecycle = key_card_lifecycle(&case.position, review_focus.as_ref());
    let deck = case
        .position
        .combat
        .meta
        .master_deck_snapshot
        .iter()
        .map(card_summary)
        .collect();
    let relics = case
        .position
        .combat
        .entities
        .player
        .relics
        .iter()
        .map(|relic| format!("{:?}", relic.id))
        .collect();
    let potions = case
        .position
        .combat
        .entities
        .potions
        .iter()
        .map(|potion| potion.as_ref().map(|potion| format!("{:?}", potion.id)))
        .collect();
    let path_tail = case
        .path
        .iter()
        .skip(case.path.len().saturating_sub(12))
        .cloned()
        .collect();
    CombatCaseReview {
        schema: "combat_case_review",
        case_path,
        static_strategic_deficit,
        deck,
        relics,
        potions,
        path_tail,
        saved_search: case.failed_search.clone(),
        source: case.source,
        gap: case.gap,
        run: case.run,
        combat: case.combat,
        ladder,
        classification,
        review_focus,
        review_focus_replay,
        review_focus_prior_rerun,
        line_lab,
        quality_lanes,
        counterfactual_hp_probe,
        combat_deficit_evidence,
        combat_strategic_feedback,
        boss_pressure_lens,
        champ_phase_audit,
        key_card_lifecycle,
    }
}

fn strategic_facts_from_case(case: &CombatCase) -> RunStrategicFacts {
    let deck = &case.position.combat.meta.master_deck_snapshot;
    RunStrategicFacts {
        entering_act: case.run.act,
        starter_basic_count: deck.iter().filter(|card| is_starter_basic(card.id)).count(),
        curse_count: deck
            .iter()
            .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
            .count(),
        has_energy_relic: case
            .position
            .combat
            .entities
            .player
            .relics
            .iter()
            .any(|relic| energy_master_delta(relic.id) > 0),
    }
}
