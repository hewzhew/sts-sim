use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    explain_combat_search_v2_initial_decision, CombatSearchV2Config,
    CombatSearchV2DecisionMicroscopeReport, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};
use sts_simulator::content::cards::java_id;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::ClientInput;

use super::key_card_counterfactual::{move_key_card, KeyCardCounterfactualPlacement};
use super::key_card_lifecycle::{key_card_targets, KeyCardReason};
use super::options::ReviewOptions;

#[derive(Serialize)]
pub(super) struct KeyCardDecisionMicroscopeProbe {
    schema: &'static str,
    contract: &'static str,
    skipped_reason: Option<&'static str>,
    variants: Vec<KeyCardDecisionMicroscopeVariant>,
}

#[derive(Serialize)]
struct KeyCardDecisionMicroscopeVariant {
    card: String,
    uuid: u32,
    reason: &'static str,
    placement: &'static str,
    skipped_reason: Option<&'static str>,
    target_candidate: Option<KeyCardDecisionTargetCandidate>,
    candidates_before_target: Vec<KeyCardDecisionCandidateDigest>,
    selected_candidate: Option<KeyCardDecisionCandidateDigest>,
    microscope: Option<CombatSearchV2DecisionMicroscopeReport>,
}

#[derive(Serialize)]
struct KeyCardDecisionTargetCandidate {
    ordered_index: usize,
    action_key: String,
    action_role: &'static str,
    selected_by_best_complete: bool,
    one_step_status: &'static str,
    one_step_terminal: String,
    visible_hp_loss_if_turn_ends: i32,
    survival_margin: i32,
    total_enemy_hp: i32,
}

#[derive(Serialize)]
struct KeyCardDecisionCandidateDigest {
    ordered_index: usize,
    action_key: String,
    action_role: &'static str,
    selected_by_best_complete: bool,
    one_step_status: &'static str,
    visible_hp_loss_if_turn_ends: i32,
    survival_margin: i32,
    total_enemy_hp: i32,
}

pub(super) fn run_key_card_decision_microscope_probe(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<KeyCardDecisionMicroscopeProbe> {
    if !options.key_card_decision_microscope {
        return None;
    }
    let targets = key_card_targets(&case.position.combat);
    if targets.is_empty() {
        return Some(KeyCardDecisionMicroscopeProbe {
            schema: "key_card_decision_microscope_probe_v0",
            contract: "diagnostic_only_move_key_card_to_opening_hand_then_explain_root_decision",
            skipped_reason: Some("no_key_cards"),
            variants: Vec::new(),
        });
    }

    let variants = targets
        .into_iter()
        .map(|target| run_variant(options, case, &target.card, target.reason))
        .collect();

    Some(KeyCardDecisionMicroscopeProbe {
        schema: "key_card_decision_microscope_probe_v0",
        contract: "diagnostic_only_move_key_card_to_opening_hand_then_explain_root_decision",
        skipped_reason: None,
        variants,
    })
}

fn run_variant(
    options: &ReviewOptions,
    original_case: &CombatCase,
    card: &CombatCard,
    reason: KeyCardReason,
) -> KeyCardDecisionMicroscopeVariant {
    let mut case = original_case.clone();
    let placement = KeyCardCounterfactualPlacement::OpeningHand;
    if move_key_card(&mut case.position.combat, card.uuid, placement).is_none() {
        return skipped_variant(card, reason, placement, "card_not_in_active_combat_zones");
    }
    let Some(card_index) = case
        .position
        .combat
        .zones
        .hand
        .iter()
        .position(|hand_card| hand_card.uuid == card.uuid)
    else {
        return skipped_variant(card, reason, placement, "card_not_in_opening_hand");
    };

    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let microscope = explain_combat_search_v2_initial_decision(
        &case.position.engine,
        &case.position.combat,
        CombatSearchV2Config {
            max_nodes: options.slow_nodes,
            wall_time: Some(Duration::from_millis(options.slow_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::All,
            max_potions_used: Some(options.diagnostic_potion_max),
            rollout_policy,
            child_rollout_policy: options.child_rollout_policy(),
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
            input_label: Some(format!(
                "key_card_decision_microscope:{}#{}",
                java_id(card.id),
                card.uuid
            )),
            ..CombatSearchV2Config::default()
        },
    );
    let target_candidate = target_candidate(&microscope, card_index);
    let candidates_before_target = candidates_before_target(&microscope, &target_candidate);
    let selected_candidate = selected_candidate(&microscope);

    KeyCardDecisionMicroscopeVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: None,
        target_candidate,
        candidates_before_target,
        selected_candidate,
        microscope: Some(microscope),
    }
}

fn skipped_variant(
    card: &CombatCard,
    reason: KeyCardReason,
    placement: KeyCardCounterfactualPlacement,
    skipped_reason: &'static str,
) -> KeyCardDecisionMicroscopeVariant {
    KeyCardDecisionMicroscopeVariant {
        card: format!("{}+{}", java_id(card.id), card.upgrades),
        uuid: card.uuid,
        reason: reason.label(),
        placement: placement.label(),
        skipped_reason: Some(skipped_reason),
        target_candidate: None,
        candidates_before_target: Vec::new(),
        selected_candidate: None,
        microscope: None,
    }
}

fn target_candidate(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    card_index: usize,
) -> Option<KeyCardDecisionTargetCandidate> {
    microscope
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.input,
                ClientInput::PlayCard {
                    card_index: input_card_index,
                    target: _
                } if input_card_index == card_index
            )
        })
        .map(|candidate| KeyCardDecisionTargetCandidate {
            ordered_index: candidate.ordered_index,
            action_key: candidate.action_key.clone(),
            action_role: candidate.action_role,
            selected_by_best_complete: candidate.selected_by_best_complete,
            one_step_status: candidate.one_step.status,
            one_step_terminal: format!("{:?}", candidate.one_step.terminal),
            visible_hp_loss_if_turn_ends: candidate.one_step.visible_hp_loss_if_turn_ends,
            survival_margin: candidate.one_step.survival_margin,
            total_enemy_hp: candidate.one_step.total_enemy_hp,
        })
}

fn candidates_before_target(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
    target: &Option<KeyCardDecisionTargetCandidate>,
) -> Vec<KeyCardDecisionCandidateDigest> {
    let Some(target) = target else {
        return Vec::new();
    };
    microscope
        .candidates
        .iter()
        .filter(|candidate| candidate.ordered_index < target.ordered_index)
        .map(candidate_digest)
        .collect()
}

fn selected_candidate(
    microscope: &CombatSearchV2DecisionMicroscopeReport,
) -> Option<KeyCardDecisionCandidateDigest> {
    microscope
        .candidates
        .iter()
        .find(|candidate| candidate.selected_by_best_complete)
        .map(candidate_digest)
}

fn candidate_digest(
    candidate: &sts_simulator::ai::combat_search_v2::CombatSearchV2DecisionCandidateReport,
) -> KeyCardDecisionCandidateDigest {
    KeyCardDecisionCandidateDigest {
        ordered_index: candidate.ordered_index,
        action_key: candidate.action_key.clone(),
        action_role: candidate.action_role,
        selected_by_best_complete: candidate.selected_by_best_complete,
        one_step_status: candidate.one_step.status,
        visible_hp_loss_if_turn_ends: candidate.one_step.visible_hp_loss_if_turn_ends,
        survival_margin: candidate.one_step.survival_margin,
        total_enemy_hp: candidate.one_step.total_enemy_hp,
    }
}
