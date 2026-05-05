use crate::bot::card_disposition::{deck_cut_score, duplicate_score, DeckDispositionMode};
use crate::bot::deck_profile::deck_profile;
use crate::bot::deck_scoring::{curse_remove_severity, score_card_offer, score_owned_card};
use crate::bot::shared::{analyze_run_needs, RunNeedSnapshot};
use crate::bot::upgrade_facts::{dominant_upgrade_semantic_key, upgrade_facts};
use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckOperationKind {
    Add(CardId),
    Remove,
    Upgrade,
    Duplicate,
    Transform {
        count: usize,
        upgraded_context: bool,
    },
    VampiresExchange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckOpsCandidate {
    pub target_index: Option<usize>,
    pub target_uuid: Option<u32>,
    pub score: i32,
    pub rationale_key: &'static str,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckOpsAssessment {
    pub operation: DeckOperationKind,
    pub best_candidate: Option<DeckOpsCandidate>,
    pub total_score: i32,
    pub rationale_key: &'static str,
}

pub fn assess(run_state: &RunState, operation: DeckOperationKind) -> DeckOpsAssessment {
    let profile = deck_profile(run_state);
    let need = analyze_run_needs(run_state);
    match operation {
        DeckOperationKind::Add(card_id) => assess_add(run_state, card_id, need),
        DeckOperationKind::Remove => best_ranked(
            operation,
            "remove_best_target",
            run_state
                .master_deck
                .iter()
                .enumerate()
                .map(|(idx, card)| rank_remove_candidate(run_state, &profile, card, idx)),
        ),
        DeckOperationKind::Upgrade => best_ranked(
            operation,
            "upgrade_best_target",
            run_state
                .master_deck
                .iter()
                .enumerate()
                .filter(|(_, card)| is_upgradable(card))
                .map(|(idx, card)| rank_upgrade_candidate(run_state, &profile, &need, card, idx)),
        ),
        DeckOperationKind::Duplicate => best_ranked(
            operation,
            "duplicate_best_target",
            run_state
                .master_deck
                .iter()
                .enumerate()
                .map(|(idx, card)| rank_duplicate_candidate(run_state, &profile, card, idx)),
        ),
        DeckOperationKind::Transform {
            upgraded_context, ..
        } => best_ranked(
            operation,
            "transform_best_target",
            run_state.master_deck.iter().enumerate().map(|(idx, card)| {
                rank_transform_candidate(run_state, &profile, card, idx, upgraded_context)
            }),
        ),
        DeckOperationKind::VampiresExchange => assess_vampires_exchange(run_state),
    }
}

pub fn best_purge_indices(run_state: &RunState, count: usize) -> Vec<usize> {
    let profile = deck_profile(run_state);
    let mut ranked = run_state
        .master_deck
        .iter()
        .enumerate()
        .map(|(idx, card)| {
            let score = deck_cut_score(run_state, &profile, card, DeckDispositionMode::Purge, 0);
            (idx, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|lhs, rhs| lhs.1.cmp(&rhs.1).then_with(|| lhs.0.cmp(&rhs.0)));
    ranked
        .into_iter()
        .take(count.min(run_state.master_deck.len()))
        .map(|(idx, _)| idx)
        .collect()
}

pub fn best_upgrade_index(run_state: &RunState) -> Option<usize> {
    assess(run_state, DeckOperationKind::Upgrade)
        .best_candidate
        .and_then(|candidate| candidate.target_index)
}

pub fn best_duplicate_index(run_state: &RunState) -> Option<usize> {
    assess(run_state, DeckOperationKind::Duplicate)
        .best_candidate
        .and_then(|candidate| candidate.target_index)
}

pub fn best_transform_indices(
    run_state: &RunState,
    count: usize,
    upgraded_context: bool,
) -> Vec<usize> {
    let profile = deck_profile(run_state);
    let mut ranked = run_state
        .master_deck
        .iter()
        .enumerate()
        .map(|(idx, card)| {
            let score =
                rank_transform_candidate(run_state, &profile, card, idx, upgraded_context).score;
            (idx, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|lhs, rhs| rhs.1.cmp(&lhs.1).then_with(|| lhs.0.cmp(&rhs.0)));
    ranked
        .into_iter()
        .take(count.min(run_state.master_deck.len()))
        .map(|(idx, _)| idx)
        .collect()
}

pub fn deck_ops_assessment_json(assessment: &DeckOpsAssessment) -> Value {
    json!({
        "operation": format!("{:?}", assessment.operation),
        "total_score": assessment.total_score,
        "rationale_key": assessment.rationale_key,
        "best_candidate": assessment.best_candidate.as_ref().map(|candidate| json!({
            "target_index": candidate.target_index,
            "target_uuid": candidate.target_uuid,
            "score": candidate.score,
            "rationale_key": candidate.rationale_key,
            "label": candidate.label,
        })),
    })
}

pub fn focus_summary(assessment: &DeckOpsAssessment) -> String {
    let target = assessment
        .best_candidate
        .as_ref()
        .map(|candidate| candidate.label.as_str())
        .unwrap_or("none");
    format!(
        "op={:?} target={} total={} rationale={}",
        assessment.operation, target, assessment.total_score, assessment.rationale_key
    )
}

fn assess_add(run_state: &RunState, card_id: CardId, need: RunNeedSnapshot) -> DeckOpsAssessment {
    let total_score = score_card_offer(card_id, run_state)
        + i32::from(need.damage_gap > 0) * 6
        + i32::from(need.block_gap > 0) * 6;
    DeckOpsAssessment {
        operation: DeckOperationKind::Add(card_id),
        best_candidate: Some(DeckOpsCandidate {
            target_index: None,
            target_uuid: None,
            score: total_score,
            rationale_key: "add_card_offer_value",
            label: cards::get_card_definition(card_id).name.to_string(),
        }),
        total_score,
        rationale_key: "add_card_offer_value",
    }
}

fn assess_vampires_exchange(run_state: &RunState) -> DeckOpsAssessment {
    let strike_count = run_state
        .master_deck
        .iter()
        .filter(|card| cards::is_starter_strike(card.id))
        .count() as i32;
    let hp_ratio = run_state.current_hp as f32 / run_state.max_hp.max(1) as f32;
    let total_score = strike_count * 35 + if hp_ratio < 0.60 { 30 } else { 5 };
    DeckOpsAssessment {
        operation: DeckOperationKind::VampiresExchange,
        best_candidate: None,
        total_score,
        rationale_key: "vampires_exchange",
    }
}

fn best_ranked<I>(
    operation: DeckOperationKind,
    fallback_key: &'static str,
    candidates: I,
) -> DeckOpsAssessment
where
    I: Iterator<Item = DeckOpsCandidate>,
{
    let best = candidates.max_by(|lhs, rhs| {
        lhs.score
            .cmp(&rhs.score)
            .then_with(|| rhs.target_index.cmp(&lhs.target_index))
    });
    let total_score = best.as_ref().map(|candidate| candidate.score).unwrap_or(0);
    let rationale_key = best
        .as_ref()
        .map(|candidate| candidate.rationale_key)
        .unwrap_or(fallback_key);
    DeckOpsAssessment {
        operation,
        best_candidate: best,
        total_score,
        rationale_key,
    }
}

fn rank_remove_candidate(
    run_state: &RunState,
    profile: &crate::bot::deck_profile::DeckProfile,
    card: &CombatCard,
    idx: usize,
) -> DeckOpsCandidate {
    let cut_score = deck_cut_score(run_state, profile, card, DeckDispositionMode::Purge, 0);
    let score = (-cut_score / 10).clamp(-200, 400)
        + curse_remove_severity(card.id) * 18
        + i32::from(cards::is_starter_basic(card.id)) * 16;
    DeckOpsCandidate {
        target_index: Some(idx),
        target_uuid: Some(card.uuid),
        score,
        rationale_key: if curse_remove_severity(card.id) > 0 {
            "remove_curse_burden"
        } else if cards::is_starter_basic(card.id) {
            "remove_starter_density"
        } else {
            "remove_low_value_slot"
        },
        label: label_for_card(card),
    }
}

fn rank_upgrade_candidate(
    run_state: &RunState,
    profile: &crate::bot::deck_profile::DeckProfile,
    need: &RunNeedSnapshot,
    card: &CombatCard,
    idx: usize,
) -> DeckOpsCandidate {
    let facts = upgrade_facts(card.id);
    let mut score = score_owned_card(card.id, run_state);
    if facts.changes_cost {
        score += 16;
    }
    if facts.improves_draw_consistency {
        score += 14;
    }
    if facts.improves_scaling {
        score += 14;
    }
    if facts.improves_exhaust_control {
        score += 10;
    }
    if facts.improves_frontload && need.survival_pressure >= 120 {
        score += 10;
    }
    if facts.improves_block_efficiency && need.block_gap > 0 {
        score += 12;
    }
    if facts.repeatable_upgrade {
        score += 18 + card.upgrades as i32 * 10;
    }
    if card.id == CardId::SearingBlow {
        score += 45 + profile.searing_blow_upgrades * 10;
    }
    if cards::is_starter_basic(card.id) {
        score -= 24;
    }
    DeckOpsCandidate {
        target_index: Some(idx),
        target_uuid: Some(card.uuid),
        score,
        rationale_key: dominant_upgrade_semantic_key(card.id),
        label: label_for_card(card),
    }
}

fn rank_duplicate_candidate(
    run_state: &RunState,
    profile: &crate::bot::deck_profile::DeckProfile,
    card: &CombatCard,
    idx: usize,
) -> DeckOpsCandidate {
    let score = (duplicate_score(run_state, profile, card) / 10).clamp(-200, 300);
    DeckOpsCandidate {
        target_index: Some(idx),
        target_uuid: Some(card.uuid),
        score,
        rationale_key: "duplicate_best_core",
        label: label_for_card(card),
    }
}

fn rank_transform_candidate(
    run_state: &RunState,
    profile: &crate::bot::deck_profile::DeckProfile,
    card: &CombatCard,
    idx: usize,
    upgraded_context: bool,
) -> DeckOpsCandidate {
    let cut_score = deck_cut_score(
        run_state,
        profile,
        card,
        if upgraded_context {
            DeckDispositionMode::TransformUpgraded
        } else {
            DeckDispositionMode::Transform
        },
        0,
    );
    let mut score = (-cut_score / 10).clamp(-200, 400)
        + curse_remove_severity(card.id) * 18
        + i32::from(cards::is_starter_basic(card.id)) * 18;
    if upgraded_context {
        score += 18;
    }
    DeckOpsCandidate {
        target_index: Some(idx),
        target_uuid: Some(card.uuid),
        score,
        rationale_key: if upgraded_context {
            "transform_upgrade_window"
        } else {
            "transform_replace_burden"
        },
        label: label_for_card(card),
    }
}

fn label_for_card(card: &CombatCard) -> String {
    let mut label = cards::get_card_definition(card.id).name.to_string();
    if card.upgrades > 0 {
        label.push_str(&"+".repeat(card.upgrades as usize));
    }
    label
}

fn is_upgradable(card: &CombatCard) -> bool {
    let def = cards::get_card_definition(card.id);
    card.id == CardId::SearingBlow
        || (card.upgrades == 0 && !matches!(def.card_type, CardType::Status | CardType::Curse))
}
