use sts_simulator::eval::combat_case::CombatCase;

use super::key_card_lifecycle::key_card_lifecycle;
use super::strategic_feedback::combat_strategic_feedback;

#[path = "case_payload/derived.rs"]
mod derived;
#[path = "case_payload/types.rs"]
mod types;

use derived::derived_payload_from_case;
pub(super) use types::{CombatCaseReview, CombatCaseReviewArtifacts};

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
        boss_setup_lane,
        frozen_panel_lanes,
        key_card_counterfactual,
        key_card_decision_microscope,
        root_action_role_duel,
        champ_phase_audit,
    } = artifacts;
    let derived = derived_payload_from_case(&case);
    let combat_strategic_feedback = combat_strategic_feedback(
        &case,
        &derived.static_strategic_deficit,
        &classification,
        review_focus.as_ref(),
        &ladder,
    );
    let key_card_lifecycle = key_card_lifecycle(&case.position, review_focus.as_ref());
    CombatCaseReview {
        schema: "combat_case_review",
        case_path,
        static_strategic_deficit: derived.static_strategic_deficit,
        deck: derived.deck,
        relics: derived.relics,
        potions: derived.potions,
        path_tail: derived.path_tail,
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
        boss_setup_lane,
        frozen_panel_lanes,
        key_card_counterfactual,
        key_card_decision_microscope,
        root_action_role_duel,
        champ_phase_audit,
        key_card_lifecycle,
    }
}
