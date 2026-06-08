use std::collections::BTreeSet;

use crate::content::cards::CardId;
use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::rewards::RewardCard;

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
    semantic_class: String,
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
    let surface = build_decision_surface(session);
    let options = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let input = candidate.action.executable_input()?;
            let ClientInput::CampfireOption(choice) = input else {
                return None;
            };
            let metadata = campfire_option_metadata(session, choice);
            Some(CampfireBranchOption {
                label: candidate.label.clone(),
                command: candidate.action.command_hint(),
                card: metadata.card,
                upgrades: metadata.upgrades,
                effect_kind: metadata.effect_kind,
                semantic_class: metadata.semantic_class,
                equivalence_key: metadata.equivalence_key,
                representative_count: 1,
                suppressed_count: 0,
            })
        })
        .collect::<Vec<_>>();
    let options = compressed_campfire_branch_options(options);
    (!options.is_empty()).then_some(options)
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

    let mut annotated = options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            (
                index,
                campfire_option_priority(option),
                option.semantic_class.clone(),
            )
        })
        .collect::<Vec<_>>();
    annotated.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));

    let mut selected = Vec::new();
    let mut selected_classes = BTreeSet::new();
    for (index, _, class_key) in &annotated {
        if selected.len() >= capped_limit {
            break;
        }
        if selected_classes.insert(class_key.clone()) {
            selected.push(*index);
        }
    }
    for index in 0..options.len() {
        if selected.len() >= capped_limit {
            break;
        }
        if !selected.contains(&index) {
            selected.push(index);
        }
    }

    selected.sort_unstable();
    let selected_indices = selected.iter().copied().collect::<BTreeSet<_>>();
    let options = options
        .into_iter()
        .enumerate()
        .filter_map(|(index, option)| selected_indices.contains(&index).then_some(option))
        .collect();
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
            counts[index] += 1;
        } else {
            groups.push(option);
            counts.push(1);
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
    semantic_class: String,
    equivalence_key: String,
}

fn campfire_option_metadata(
    session: &RunControlSession,
    choice: CampfireChoice,
) -> CampfireOptionMetadata {
    match choice {
        CampfireChoice::Rest => {
            let semantic_class = if session.run_state.current_hp < session.run_state.max_hp {
                "rest:wounded"
            } else {
                "rest:full_hp"
            };
            campfire_metadata_without_card(semantic_class, "rest")
        }
        CampfireChoice::Smith(idx) => {
            let Some(card) = session.run_state.master_deck.get(idx) else {
                return campfire_metadata_without_card("smith:unknown", "upgrade_card");
            };
            let upgraded = card.upgrades.saturating_add(1);
            let profile = crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1(
                &RewardCard::new(card.id, upgraded),
            );
            let (_, class_key) = super::reward_option_semantic_class(&profile);
            CampfireOptionMetadata {
                card: Some(card.id),
                upgrades: Some(card.upgrades),
                effect_kind: "upgrade_card".to_string(),
                semantic_class: format!("smith:{class_key}"),
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
                semantic_class: "toke".to_string(),
                equivalence_key: card
                    .map(|card| format!("toke:{}", super::card_stat_identity_key(card)))
                    .unwrap_or_else(|| "toke:unknown".to_string()),
            }
        }
        CampfireChoice::Recall => campfire_metadata_without_card("recall", "recall"),
    }
}

fn campfire_metadata_without_card(
    semantic_class: &str,
    effect_kind: &str,
) -> CampfireOptionMetadata {
    CampfireOptionMetadata {
        card: None,
        upgrades: None,
        effect_kind: effect_kind.to_string(),
        semantic_class: semantic_class.to_string(),
        equivalence_key: semantic_class.to_string(),
    }
}

fn campfire_option_priority(option: &CampfireBranchOption) -> usize {
    match option.semantic_class.as_str() {
        "rest:wounded" => 0,
        class if class.starts_with("smith:") => 1,
        "rest:full_hp" => 2,
        "dig" | "lift" | "toke" => 2,
        "recall" => 3,
        _ => 4,
    }
}
