use crate::ai::campfire_policy_v1::{
    build_campfire_decision_context_v1, plan_campfire_decision_v1, CampfirePlanCandidateV1,
    CampfirePolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::eval::run_control::RunControlSession;
use crate::state::core::{CampfireChoice, EngineState};

#[derive(Clone, Debug)]
pub(super) struct CampfireBranchOption {
    pub(super) label: String,
    pub(super) command: String,
    pub(super) card: Option<CardId>,
    pub(super) upgrades: Option<u8>,
    pub(super) effect_kind: String,
    pub(super) equivalence_key: String,
    pub(super) representative_count: usize,
    pub(super) suppressed_count: usize,
}

#[derive(Clone, Debug)]
pub(super) struct CampfireBranchOptionSelection {
    pub(super) options: Vec<CampfireBranchOption>,
}

pub(super) fn campfire_branch_options(
    session: &RunControlSession,
) -> Option<Vec<CampfireBranchOption>> {
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
    (!options.is_empty()).then_some(options)
}

fn campfire_branch_option_from_plan(
    session: &RunControlSession,
    plan: &CampfirePlanCandidateV1,
) -> Option<CampfireBranchOption> {
    if !plan.branch_active {
        return None;
    }
    let choice = plan.choice?;
    let metadata = campfire_option_metadata(session, choice);
    Some(CampfireBranchOption {
        label: campfire_option_label(session, choice).unwrap_or_else(|| plan.plan_id.clone()),
        command: campfire_option_command(choice),
        card: metadata.card,
        upgrades: metadata.upgrades,
        effect_kind: metadata.effect_kind,
        equivalence_key: metadata.equivalence_key,
        representative_count: plan.representative_count,
        suppressed_count: plan.suppressed_count,
    })
}

pub(super) fn select_campfire_branch_options(
    options: Vec<CampfireBranchOption>,
    max_campfire_options_per_branch: Option<usize>,
) -> CampfireBranchOptionSelection {
    let Some(limit) = max_campfire_options_per_branch else {
        return CampfireBranchOptionSelection { options };
    };
    select_campfire_branch_options_with_limit(options, limit)
}

fn select_campfire_branch_options_with_limit(
    options: Vec<CampfireBranchOption>,
    limit: usize,
) -> CampfireBranchOptionSelection {
    let capped_limit = limit.min(options.len());
    if options.len() <= capped_limit {
        return CampfireBranchOptionSelection { options };
    }

    let options = options.into_iter().take(capped_limit).collect();
    CampfireBranchOptionSelection { options }
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
