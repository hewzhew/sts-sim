use crate::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfireDecisionV1,
    CampfirePlanCandidateV1, CampfirePlanRoleV1, CampfirePolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentCampfirePlanCandidateEntryV1, BranchExperimentCampfirePlanCandidatePoolV1,
    BranchExperimentChoiceDecisionSignalV1,
};
use crate::eval::run_control::RunControlSession;
use crate::state::core::{CampfireChoice, EngineState};
use std::collections::BTreeSet;

const MIN_INSPECT_ONLY_SMITH_BRANCH_SCORE: i32 = 300;

#[derive(Clone, Debug)]
pub(super) struct CampfireBranchOption {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) card: Option<CardId>,
    pub(super) upgrades: Option<u8>,
    pub(super) effect_kind: String,
    pub(super) equivalence_key: String,
    pub(super) strategy_tag: Option<String>,
    pub(super) representative_count: usize,
    pub(super) suppressed_count: usize,
    pub(super) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
    pub(super) plan_role: CampfirePlanRoleV1,
    pub(super) score_hint: i32,
}

#[derive(Clone, Debug)]
pub(super) struct CampfireBranchOptionSelection {
    pub(super) options: Vec<CampfireBranchOption>,
    pub(super) candidate_pool: BranchExperimentCampfirePlanCandidatePoolV1,
}

pub(super) fn campfire_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CampfireBranchOption>> {
    campfire_branch_selection(session, None).map(|selection| selection.options)
}

pub(super) fn campfire_branch_selection(
    session: &RunControlSession,
    max_campfire_options_per_branch: Option<usize>,
) -> Option<CampfireBranchOptionSelection> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return None;
    }
    let choices = crate::engine::campfire_handler::get_available_options(&session.run_state);
    let context = build_campfire_decision_context_v1(&session.run_state, choices);
    let decision = plan_campfire_decision_v1(&context, &CampfirePolicyConfigV1::default());
    let options = decision
        .candidate_plans
        .iter()
        .filter_map(|plan| campfire_branch_option_from_plan(session, plan))
        .collect();
    let options = compressed_campfire_branch_options(options);
    let selected = select_campfire_branch_options(options, max_campfire_options_per_branch);
    if selected.options.is_empty() {
        return None;
    }
    Some(CampfireBranchOptionSelection {
        candidate_pool: campfire_candidate_pool_from_decision(
            session,
            &decision,
            &selected.options,
        ),
        options: selected.options,
    })
}

fn campfire_branch_option_from_plan(
    session: &RunControlSession,
    plan: &CampfirePlanCandidateV1,
) -> Option<CampfireBranchOption> {
    if !plan.branch_active {
        return None;
    }
    let choice = plan.choice?;
    if matches!(choice, CampfireChoice::Smith(_))
        && plan.role == CampfirePlanRoleV1::InspectOnly
        && plan.score_hint < MIN_INSPECT_ONLY_SMITH_BRANCH_SCORE
        && plan.suppressed_count > 0
        && !campfire_smith_tag_is_concrete_branch(plan.strategy_tag.as_deref())
    {
        return None;
    }
    let metadata = campfire_option_metadata(session, choice);
    Some(CampfireBranchOption {
        label: campfire_option_label(session, choice).unwrap_or_else(|| plan.plan_id.clone()),
        command: campfire_option_command(choice),
        card: metadata.card,
        upgrades: metadata.upgrades,
        effect_kind: metadata.effect_kind,
        equivalence_key: metadata.equivalence_key,
        strategy_tag: plan.strategy_tag.clone(),
        representative_count: plan.representative_count,
        suppressed_count: plan.suppressed_count,
        decision_signal: Some(super::campfire_plan_decision_signal_v1(plan)),
        plan_role: plan.role,
        score_hint: plan.score_hint,
    })
}

fn campfire_smith_tag_is_concrete_branch(tag: Option<&str>) -> bool {
    matches!(
        tag,
        Some(
            "upgrade_role:defensive_survival"
                | "upgrade_role:phase_burst"
                | "upgrade_role:debuff_coverage"
                | "upgrade_debt:stasis_recovery"
                | "upgrade_debt:hyperbeam_block"
                | "upgrade_debt:phase_burst"
                | "upgrade_debt:execute_block"
                | "upgrade_debt:debuff_coverage"
                | "upgrade_debt:transitional_frontload"
        )
    )
}

pub(super) fn select_campfire_branch_options(
    options: Vec<CampfireBranchOption>,
    max_campfire_options_per_branch: Option<usize>,
) -> CampfireBranchOptionSelection {
    let Some(limit) = max_campfire_options_per_branch else {
        return CampfireBranchOptionSelection {
            candidate_pool: empty_campfire_candidate_pool(options.len()),
            options,
        };
    };
    select_campfire_branch_options_with_limit(options, limit)
}

fn select_campfire_branch_options_with_limit(
    options: Vec<CampfireBranchOption>,
    limit: usize,
) -> CampfireBranchOptionSelection {
    let filtered = options
        .iter()
        .cloned()
        .filter(|option| !is_full_hp_rest_branch_option(option))
        .collect::<Vec<_>>();
    let options = if filtered.is_empty() {
        options
            .into_iter()
            .filter(is_full_hp_rest_branch_option)
            .collect::<Vec<_>>()
    } else {
        filtered
    };
    let filtered = options
        .iter()
        .cloned()
        .filter(|option| !is_unprotected_rest_branch_option(option))
        .collect::<Vec<_>>();
    let options = if filtered.is_empty() {
        options
    } else {
        filtered
    };
    let capped_limit = limit.min(options.len());
    if capped_limit == 0 || options.len() <= capped_limit {
        return CampfireBranchOptionSelection {
            candidate_pool: empty_campfire_candidate_pool(options.len()),
            options,
        };
    }

    let mut selected_indices = Vec::<usize>::new();
    let mut used_indices = BTreeSet::<usize>::new();
    let mut used_smith_tags = BTreeSet::<String>::new();
    let mut used_untagged_smith = false;

    if let Some(index) = options.iter().position(is_recovery_rest_branch_option) {
        push_campfire_option_index(
            index,
            &options,
            &mut selected_indices,
            &mut used_indices,
            &mut used_smith_tags,
            &mut used_untagged_smith,
        );
    }

    while selected_indices.len() < capped_limit {
        let Some(index) = options.iter().enumerate().position(|(index, option)| {
            !used_indices.contains(&index) && is_concrete_smith_branch_option(option)
        }) else {
            break;
        };
        push_campfire_option_index(
            index,
            &options,
            &mut selected_indices,
            &mut used_indices,
            &mut used_smith_tags,
            &mut used_untagged_smith,
        );
    }

    if selected_indices.len() < capped_limit {
        if let Some(index) = options.iter().position(is_smith_branch_option) {
            push_campfire_option_index(
                index,
                &options,
                &mut selected_indices,
                &mut used_indices,
                &mut used_smith_tags,
                &mut used_untagged_smith,
            );
        }
    }

    while selected_indices.len() < capped_limit {
        let Some(index) = options.iter().enumerate().position(|(index, option)| {
            !used_indices.contains(&index)
                && is_smith_branch_option(option)
                && campfire_branch_option_adds_new_smith_coverage(
                    option,
                    &used_smith_tags,
                    used_untagged_smith,
                )
        }) else {
            break;
        };
        push_campfire_option_index(
            index,
            &options,
            &mut selected_indices,
            &mut used_indices,
            &mut used_smith_tags,
            &mut used_untagged_smith,
        );
    }

    while selected_indices.len() < capped_limit {
        let Some(index) = options.iter().enumerate().position(|(index, option)| {
            !used_indices.contains(&index)
                && !is_smith_branch_option(option)
                && !is_full_hp_rest_branch_option(option)
        }) else {
            break;
        };
        push_campfire_option_index(
            index,
            &options,
            &mut selected_indices,
            &mut used_indices,
            &mut used_smith_tags,
            &mut used_untagged_smith,
        );
    }

    while selected_indices.len() < capped_limit {
        let Some(index) = options.iter().enumerate().position(|(index, option)| {
            !used_indices.contains(&index)
                && campfire_branch_option_adds_new_smith_coverage(
                    option,
                    &used_smith_tags,
                    used_untagged_smith,
                )
        }) else {
            break;
        };
        push_campfire_option_index(
            index,
            &options,
            &mut selected_indices,
            &mut used_indices,
            &mut used_smith_tags,
            &mut used_untagged_smith,
        );
    }

    let options: Vec<CampfireBranchOption> = selected_indices
        .into_iter()
        .filter_map(|index| options.get(index).cloned())
        .collect();
    CampfireBranchOptionSelection {
        candidate_pool: empty_campfire_candidate_pool(options.len()),
        options,
    }
}

fn empty_campfire_candidate_pool(
    branch_option_count: usize,
) -> BranchExperimentCampfirePlanCandidatePoolV1 {
    BranchExperimentCampfirePlanCandidatePoolV1 {
        branch_id: String::new(),
        branch_choices: Vec::new(),
        branch_commands: Vec::new(),
        depth: 0,
        frontier_key: String::new(),
        boundary_title: String::new(),
        candidate_count: 0,
        branch_option_count,
        selected_plan_id: None,
        candidates: Vec::new(),
    }
}

fn campfire_candidate_pool_from_decision(
    session: &RunControlSession,
    decision: &CampfireDecisionV1,
    selected_options: &[CampfireBranchOption],
) -> BranchExperimentCampfirePlanCandidatePoolV1 {
    BranchExperimentCampfirePlanCandidatePoolV1 {
        branch_id: String::new(),
        branch_choices: Vec::new(),
        branch_commands: Vec::new(),
        depth: 0,
        frontier_key: String::new(),
        boundary_title: String::new(),
        candidate_count: decision.candidate_plans.len(),
        branch_option_count: selected_options.len(),
        selected_plan_id: Some(decision.selected_plan.plan_id.clone()),
        candidates: decision
            .candidate_plans
            .iter()
            .map(|plan| campfire_candidate_entry_from_plan(session, plan, selected_options))
            .collect(),
    }
}

fn campfire_candidate_entry_from_plan(
    session: &RunControlSession,
    plan: &CampfirePlanCandidateV1,
    selected_options: &[CampfireBranchOption],
) -> BranchExperimentCampfirePlanCandidateEntryV1 {
    let (command, label, effect_kind) = match plan.choice {
        Some(choice) => (
            campfire_option_command(choice),
            campfire_option_label(session, choice).unwrap_or_else(|| plan.plan_id.clone()),
            campfire_option_metadata(session, choice).effect_kind,
        ),
        None => (
            "stop".to_string(),
            "Stop campfire automation".to_string(),
            "stop".to_string(),
        ),
    };
    BranchExperimentCampfirePlanCandidateEntryV1 {
        plan_id: plan.plan_id.clone(),
        command: command.clone(),
        label,
        role: format!("{:?}", plan.role),
        effect_kind,
        strategy_tag: plan.strategy_tag.clone(),
        score_hint: plan.score_hint,
        confidence_milli: (plan.confidence * 1000.0).round() as i32,
        execute_autopilot: plan.execute_autopilot,
        branch_active: plan.branch_active,
        branch_admission: campfire_candidate_branch_admission_v1(plan, &command, selected_options),
        representative_count: plan.representative_count,
        suppressed_count: plan.suppressed_count,
        reasons: plan.reasons.clone(),
    }
}

fn campfire_candidate_branch_admission_v1(
    plan: &CampfirePlanCandidateV1,
    command: &str,
    selected_options: &[CampfireBranchOption],
) -> String {
    if selected_options
        .iter()
        .any(|option| option.command == command)
    {
        return "selected".to_string();
    }
    if plan.branch_active {
        "branch_active_unselected".to_string()
    } else {
        "not_branch_active".to_string()
    }
}

fn push_campfire_option_index(
    index: usize,
    options: &[CampfireBranchOption],
    selected_indices: &mut Vec<usize>,
    used_indices: &mut BTreeSet<usize>,
    used_smith_tags: &mut BTreeSet<String>,
    used_untagged_smith: &mut bool,
) {
    if !used_indices.insert(index) {
        return;
    }
    if let Some(option) = options.get(index) {
        if is_smith_branch_option(option) {
            if let Some(tag) = &option.strategy_tag {
                used_smith_tags.insert(tag.clone());
            } else {
                *used_untagged_smith = true;
            }
        }
    }
    selected_indices.push(index);
}

fn is_smith_branch_option(option: &CampfireBranchOption) -> bool {
    option.effect_kind == "upgrade_card"
}

fn is_concrete_smith_branch_option(option: &CampfireBranchOption) -> bool {
    is_smith_branch_option(option)
        && campfire_smith_tag_is_concrete_branch(option.strategy_tag.as_deref())
}

fn is_recovery_rest_branch_option(option: &CampfireBranchOption) -> bool {
    option.command == "rest"
        && option.equivalence_key == "rest:wounded"
        && (option.plan_role == CampfirePlanRoleV1::PolicyPreferred
            || option.score_hint >= severe_rest_score_hint_threshold())
}

fn is_full_hp_rest_branch_option(option: &CampfireBranchOption) -> bool {
    option.command == "rest" && option.equivalence_key == "rest:full_hp"
}

fn is_unprotected_rest_branch_option(option: &CampfireBranchOption) -> bool {
    option.command == "rest" && !is_recovery_rest_branch_option(option)
}

fn severe_rest_score_hint_threshold() -> i32 {
    2_000
}

fn campfire_branch_option_adds_new_smith_coverage(
    option: &CampfireBranchOption,
    used_smith_tags: &BTreeSet<String>,
    used_untagged_smith: bool,
) -> bool {
    if !is_smith_branch_option(option) {
        return true;
    }
    match &option.strategy_tag {
        Some(tag) if campfire_smith_tag_is_concrete_branch(Some(tag)) => true,
        Some(tag) => !used_smith_tags.contains(tag),
        None => !used_untagged_smith,
    }
}

fn compressed_campfire_branch_options(
    options: Vec<CampfireBranchOption>,
) -> Vec<CampfireBranchOption> {
    let mut groups = Vec::<CampfireBranchOption>::new();
    let mut counts = Vec::<usize>::new();
    for option in options {
        let key = option.equivalence_key.clone();
        if let Some((index, _)) = groups
            .iter()
            .enumerate()
            .find(|(_, representative)| representative.equivalence_key == key)
        {
            counts[index] += option.representative_count.max(1);
        } else {
            let count = option.representative_count.max(1);
            groups.push(option);
            counts.push(count);
        }
    }
    groups
        .into_iter()
        .zip(counts)
        .map(|(mut option, count)| {
            option.representative_count = count;
            option.suppressed_count = count.saturating_sub(1);
            option
        })
        .collect()
}

#[derive(Clone, Debug)]
struct CampfireOptionMetadata {
    card: Option<CardId>,
    upgrades: Option<u8>,
    effect_kind: String,
    equivalence_key: String,
}

fn campfire_option_metadata(
    session: &RunControlSession,
    choice: CampfireChoice,
) -> CampfireOptionMetadata {
    match choice {
        CampfireChoice::Rest => {
            let equivalence_key = if session.run_state.current_hp < session.run_state.max_hp {
                "rest:wounded"
            } else {
                "rest:full_hp"
            };
            campfire_metadata_without_card(equivalence_key, "rest")
        }
        CampfireChoice::Smith(idx) => {
            let Some(card) = session.run_state.master_deck.get(idx) else {
                return campfire_metadata_without_card("smith:unknown", "upgrade_card");
            };
            CampfireOptionMetadata {
                card: Some(card.id),
                upgrades: Some(card.upgrades),
                effect_kind: "upgrade_card".to_string(),
                equivalence_key: format!("smith:{}", super::card_stat_identity_key(card)),
            }
        }
        CampfireChoice::Dig => campfire_metadata_without_card("dig", "dig"),
        CampfireChoice::Lift => campfire_metadata_without_card("lift", "lift"),
        CampfireChoice::Toke(idx) => {
            let card = session.run_state.master_deck.get(idx);
            CampfireOptionMetadata {
                card: card.map(|card| card.id),
                upgrades: card.map(|card| card.upgrades),
                effect_kind: "remove_card".to_string(),
                equivalence_key: card
                    .map(|card| format!("toke:{}", super::card_stat_identity_key(card)))
                    .unwrap_or_else(|| "toke:unknown".to_string()),
            }
        }
        CampfireChoice::Recall => campfire_metadata_without_card("recall", "recall"),
    }
}

fn campfire_metadata_without_card(
    equivalence_key: &str,
    effect_kind: &str,
) -> CampfireOptionMetadata {
    CampfireOptionMetadata {
        card: None,
        upgrades: None,
        effect_kind: effect_kind.to_string(),
        equivalence_key: equivalence_key.to_string(),
    }
}

fn campfire_option_command(choice: CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "rest".to_string(),
        CampfireChoice::Smith(idx) => format!("smith {idx}"),
        CampfireChoice::Dig => "dig".to_string(),
        CampfireChoice::Lift => "lift".to_string(),
        CampfireChoice::Toke(idx) => format!("toke {idx}"),
        CampfireChoice::Recall => "recall".to_string(),
    }
}

fn campfire_option_label(session: &RunControlSession, choice: CampfireChoice) -> Option<String> {
    match choice {
        CampfireChoice::Rest => Some("Rest".to_string()),
        CampfireChoice::Smith(idx) => session.run_state.master_deck.get(idx).map(|card| {
            format!(
                "Smith {}",
                crate::content::cards::get_card_definition(card.id).name
            )
        }),
        CampfireChoice::Dig => Some("Dig".to_string()),
        CampfireChoice::Lift => Some("Lift".to_string()),
        CampfireChoice::Toke(idx) => session.run_state.master_deck.get(idx).map(|card| {
            format!(
                "Toke {}",
                crate::content::cards::get_card_definition(card.id).name
            )
        }),
        CampfireChoice::Recall => Some("Recall ruby key".to_string()),
    }
}
