use crate::ai::strategy::reward_semantic_probe::{
    RewardCandidateSemanticExplanationV1, RewardSemanticCoverageStatusV1,
};
use crate::content::cards::CardId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCandidateSemanticReviewV1 {
    pub card: CardId,
    pub contributions: Vec<RewardSemanticContributionV1>,
    pub cautions: Vec<RewardSemanticCautionV1>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardSemanticContributionV1 {
    PackageChanged(String),
    ClosesRequirement(String),
    OpensRequirement(String),
    ProvidesMechanic(String),
    ProvidesFrontloadDamage,
    ProvidesAreaDamage,
    ProvidesCombatUpgrade,
    DamageUses(String),
    EmitsEvent(String),
    InstallsRule(String),
    EventHandler(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardSemanticCautionV1 {
    Burden(String),
    DuplicateBehavior(String),
    EmptyExplanation,
    DeferredSequenceTactical,
}

pub fn review_reward_candidate_semantics_v1(
    explanation: &RewardCandidateSemanticExplanationV1,
) -> RewardCandidateSemanticReviewV1 {
    let mut contributions = Vec::new();
    let mut cautions = Vec::new();
    for item in &explanation.package_changes {
        contributions.push(RewardSemanticContributionV1::PackageChanged(item.clone()));
    }
    for item in &explanation.closes {
        contributions.push(RewardSemanticContributionV1::ClosesRequirement(
            item.clone(),
        ));
    }
    for item in &explanation.opens {
        contributions.push(RewardSemanticContributionV1::OpensRequirement(item.clone()));
    }
    for item in &explanation.provides {
        if item == "CombatUpgradeSingle" || item == "CombatUpgradeAll" {
            contributions.push(RewardSemanticContributionV1::ProvidesCombatUpgrade);
        } else {
            contributions.push(RewardSemanticContributionV1::ProvidesMechanic(item.clone()));
        }
    }
    for item in &explanation.damage {
        if item == "Frontload" {
            contributions.push(RewardSemanticContributionV1::ProvidesFrontloadDamage);
        } else if item == "AoE" {
            contributions.push(RewardSemanticContributionV1::ProvidesAreaDamage);
        }
    }
    for item in &explanation.damage_uses {
        contributions.push(RewardSemanticContributionV1::DamageUses(item.clone()));
    }
    for item in &explanation.emits {
        contributions.push(RewardSemanticContributionV1::EmitsEvent(item.clone()));
    }
    for item in &explanation.rules {
        contributions.push(RewardSemanticContributionV1::InstallsRule(item.clone()));
    }
    for item in &explanation.handlers {
        contributions.push(RewardSemanticContributionV1::EventHandler(item.clone()));
    }
    for item in &explanation.burdens {
        cautions.push(RewardSemanticCautionV1::Burden(item.clone()));
    }
    for item in &explanation.duplicates {
        cautions.push(RewardSemanticCautionV1::DuplicateBehavior(item.clone()));
    }
    match explanation.coverage.status {
        RewardSemanticCoverageStatusV1::Explained => {}
        RewardSemanticCoverageStatusV1::Empty => {
            cautions.push(RewardSemanticCautionV1::EmptyExplanation);
        }
        RewardSemanticCoverageStatusV1::DeferredSequenceTactical => {
            cautions.push(RewardSemanticCautionV1::DeferredSequenceTactical);
        }
    }
    RewardCandidateSemanticReviewV1 {
        card: explanation.card,
        contributions,
        cautions,
    }
}

pub fn render_reward_candidate_semantic_review_v1(
    review: &RewardCandidateSemanticReviewV1,
) -> String {
    format!(
        "contributions=[{}] cautions=[{}]",
        render_contributions_v1(&review.contributions),
        render_cautions_v1(&review.cautions)
    )
}

fn render_contributions_v1(items: &[RewardSemanticContributionV1]) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|item| match item {
            RewardSemanticContributionV1::PackageChanged(value) => {
                format!("PackageChanged({value})")
            }
            RewardSemanticContributionV1::ClosesRequirement(value) => {
                format!("ClosesRequirement({value})")
            }
            RewardSemanticContributionV1::OpensRequirement(value) => {
                format!("OpensRequirement({value})")
            }
            RewardSemanticContributionV1::ProvidesMechanic(value) => {
                format!("ProvidesMechanic({value})")
            }
            RewardSemanticContributionV1::ProvidesFrontloadDamage => {
                "ProvidesFrontloadDamage".to_string()
            }
            RewardSemanticContributionV1::ProvidesAreaDamage => "ProvidesAreaDamage".to_string(),
            RewardSemanticContributionV1::ProvidesCombatUpgrade => {
                "ProvidesCombatUpgrade".to_string()
            }
            RewardSemanticContributionV1::DamageUses(value) => format!("DamageUses({value})"),
            RewardSemanticContributionV1::EmitsEvent(value) => format!("EmitsEvent({value})"),
            RewardSemanticContributionV1::InstallsRule(value) => format!("InstallsRule({value})"),
            RewardSemanticContributionV1::EventHandler(value) => format!("EventHandler({value})"),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn render_cautions_v1(items: &[RewardSemanticCautionV1]) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|item| match item {
            RewardSemanticCautionV1::Burden(value) => format!("Burden({value})"),
            RewardSemanticCautionV1::DuplicateBehavior(value) => {
                format!("DuplicateBehavior({value})")
            }
            RewardSemanticCautionV1::EmptyExplanation => "EmptyExplanation".to_string(),
            RewardSemanticCautionV1::DeferredSequenceTactical => {
                "DeferredSequenceTactical".to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}
