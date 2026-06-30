use crate::ai::analysis::card_semantics::{card_definition_with_upgrades, CardBurden, Mechanic};
use crate::ai::strategy::deck_admission::DeckAdmission;
use crate::ai::strategy::deck_construction_pressure::ConstructionLaneAdjustment;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::reward_admission::{
    RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
};
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionCandidateKind {
    CardRewardPick {
        card: CardId,
        upgrades: u8,
    },
    CardRewardSkip,
    ShopBuyCard {
        card: CardId,
        upgrades: u8,
        price: i32,
    },
    ShopPurge {
        target: CleanupTarget,
    },
    ShopOpenRewards,
    ShopLeave,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CleanupTarget {
    Curse,
    Status,
    StarterStrike,
    StarterDefend,
    OtherStarter,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLane {
    Mainline,
    Probe,
    Skip,
    Reject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpansionPlan {
    Auto,
    InspectOnly(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreComponent {
    pub by: &'static str,
    pub value: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvaluation {
    pub kind: DecisionCandidateKind,
    pub lane: CandidateLane,
    pub expansion: ExpansionPlan,
    pub scores: Vec<ScoreComponent>,
}

impl CandidateEvaluation {
    pub fn inspect_only(kind: DecisionCandidateKind, reason: &'static str) -> Self {
        Self {
            kind,
            lane: CandidateLane::Reject,
            expansion: ExpansionPlan::InspectOnly(reason),
            scores: Vec::new(),
        }
    }

    pub fn total_score(&self) -> i32 {
        self.scores.iter().map(|score| score.value).sum()
    }
}

pub fn evaluate_decision_candidate(
    deck_plan: DeckPlanSnapshot,
    kind: DecisionCandidateKind,
    admission: Option<&RewardAdmission>,
) -> CandidateEvaluation {
    match kind {
        DecisionCandidateKind::CardRewardPick { .. }
        | DecisionCandidateKind::ShopBuyCard { .. } => {
            let Some(admission) = admission else {
                return CandidateEvaluation::inspect_only(kind, "card candidate missing admission");
            };
            evaluate_card_candidate(deck_plan, kind, admission)
        }
        DecisionCandidateKind::CardRewardSkip => CandidateEvaluation {
            kind,
            lane: CandidateLane::Skip,
            expansion: ExpansionPlan::Auto,
            scores: vec![score("skip", 0)],
        },
        DecisionCandidateKind::ShopPurge { target } => evaluate_cleanup_candidate(kind, target),
        DecisionCandidateKind::ShopOpenRewards => CandidateEvaluation {
            kind,
            lane: CandidateLane::Mainline,
            expansion: ExpansionPlan::Auto,
            scores: vec![score("open-rewards", 300)],
        },
        DecisionCandidateKind::ShopLeave => CandidateEvaluation {
            kind,
            lane: CandidateLane::Skip,
            expansion: ExpansionPlan::Auto,
            scores: vec![score("leave", 0)],
        },
        DecisionCandidateKind::Unsupported => {
            CandidateEvaluation::inspect_only(kind, "unsupported decision candidate")
        }
    }
}

pub fn candidate_lane_label(lane: CandidateLane) -> &'static str {
    match lane {
        CandidateLane::Mainline => "mainline",
        CandidateLane::Probe => "probe",
        CandidateLane::Skip => "skip",
        CandidateLane::Reject => "reject",
    }
}

fn evaluate_card_candidate(
    deck_plan: DeckPlanSnapshot,
    kind: DecisionCandidateKind,
    admission: &RewardAdmission,
) -> CandidateEvaluation {
    if admission.class == RewardAdmissionClass::EmptyOrDeferred {
        return CandidateEvaluation::inspect_only(kind, "unmodeled or deferred card candidate");
    }
    if admission
        .reasons
        .iter()
        .any(|reason| matches!(reason, RewardAdmissionReason::ThinSupport(_)))
    {
        return CandidateEvaluation::inspect_only(kind, "payoff support is too thin");
    }
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::DuplicateBurden(_) | RewardAdmissionReason::DuplicateConcern(_)
        )
    }) {
        return CandidateEvaluation::inspect_only(kind, "duplicate marginal value is too low");
    }
    if admission.class == RewardAdmissionClass::OpensUnsupportedPayoff {
        return CandidateEvaluation::inspect_only(kind, "unsupported payoff candidate");
    }
    if matches!(kind, DecisionCandidateKind::ShopBuyCard { .. })
        && shop_card_buy_is_risky(kind, admission)
    {
        return CandidateEvaluation::inspect_only(kind, "shop card buy carries unresolved risk");
    }

    let mut scores = Vec::new();
    scores.push(score(
        "admission-class",
        admission_class_score(admission.class),
    ));
    scores.push(score(
        "deck-admission",
        deck_admission_score(deck_plan.deck_admission(admission)),
    ));
    scores.push(score(
        "construction-pressure",
        construction_adjustment_score(deck_plan.reward_lane_adjustment(admission)),
    ));
    scores.extend(reason_scores(admission));
    if deck_plan.survival_pressure() {
        scores.push(score(
            "survival-pressure",
            survival_pressure_score(admission),
        ));
    }

    let lane = lane_from_score(scores.iter().map(|score| score.value).sum());
    let expansion = match (kind, lane) {
        (_, CandidateLane::Reject) => ExpansionPlan::InspectOnly("candidate score rejected"),
        (DecisionCandidateKind::ShopBuyCard { .. }, CandidateLane::Probe) => {
            ExpansionPlan::InspectOnly("shop card buy is below mainline")
        }
        _ => ExpansionPlan::Auto,
    };
    CandidateEvaluation {
        kind,
        lane,
        expansion,
        scores,
    }
}

fn evaluate_cleanup_candidate(
    kind: DecisionCandidateKind,
    target: CleanupTarget,
) -> CandidateEvaluation {
    let (lane, expansion, value) = match target {
        CleanupTarget::Curse => (CandidateLane::Mainline, ExpansionPlan::Auto, 320),
        CleanupTarget::Status => (CandidateLane::Mainline, ExpansionPlan::Auto, 260),
        CleanupTarget::StarterStrike => (CandidateLane::Mainline, ExpansionPlan::Auto, 180),
        CleanupTarget::StarterDefend | CleanupTarget::OtherStarter | CleanupTarget::Other => (
            CandidateLane::Reject,
            ExpansionPlan::InspectOnly("shop purge target is not safe for tiny owner"),
            0,
        ),
    };
    CandidateEvaluation {
        kind,
        lane,
        expansion,
        scores: vec![score("cleanup-target", value)],
    }
}

fn lane_from_score(score: i32) -> CandidateLane {
    if score >= 110 {
        CandidateLane::Mainline
    } else if score >= 45 {
        CandidateLane::Probe
    } else {
        CandidateLane::Reject
    }
}

fn admission_class_score(class: RewardAdmissionClass) -> i32 {
    match class {
        RewardAdmissionClass::ClosesRequirement => 130,
        RewardAdmissionClass::BuildsSupportedPackage => 105,
        RewardAdmissionClass::EngineSeed => 65,
        RewardAdmissionClass::ImmediateWork => 55,
        RewardAdmissionClass::BurdenedImmediateWork => 25,
        RewardAdmissionClass::OpensUnsupportedPayoff
        | RewardAdmissionClass::EmptyOrDeferred
        | RewardAdmissionClass::Skip => 0,
    }
}

fn deck_admission_score(admission: DeckAdmission) -> i32 {
    match admission {
        DeckAdmission::Welcome => 0,
        DeckAdmission::Conditional => -30,
        DeckAdmission::Discouraged => -90,
    }
}

fn construction_adjustment_score(adjustment: ConstructionLaneAdjustment) -> i32 {
    match adjustment {
        ConstructionLaneAdjustment::None => 0,
        ConstructionLaneAdjustment::PromoteOneStep => 35,
        ConstructionLaneAdjustment::PromoteToMainline => 70,
        ConstructionLaneAdjustment::SoftDemote => -45,
        ConstructionLaneAdjustment::HardDemote => -130,
    }
}

fn reason_scores(admission: &RewardAdmission) -> Vec<ScoreComponent> {
    let mut scores = Vec::new();
    for reason in &admission.reasons {
        match *reason {
            RewardAdmissionReason::Closes(_) => scores.push(score("closes-requirement", 85)),
            RewardAdmissionReason::Supports(_) => scores.push(score("supports-package", 65)),
            RewardAdmissionReason::Provides(Mechanic::CardDraw | Mechanic::Energy) => {
                scores.push(score("access", 45))
            }
            RewardAdmissionReason::Provides(
                Mechanic::Block | Mechanic::Weak | Mechanic::EnemyStrengthDown,
            ) => scores.push(score("survival-tool", 35)),
            RewardAdmissionReason::Provides(Mechanic::Strength | Mechanic::StrengthMultiplier) => {
                scores.push(score("scaling-tool", 30))
            }
            RewardAdmissionReason::FrontloadDamage => scores.push(score("frontload", 25)),
            RewardAdmissionReason::AreaDamage => scores.push(score("aoe", 45)),
            RewardAdmissionReason::CombatUpgrade => scores.push(score("combat-upgrade", 45)),
            RewardAdmissionReason::RunReward(_) => scores.push(score("run-reward", 40)),
            RewardAdmissionReason::Installs(_) => scores.push(score("installed-rule", 50)),
            RewardAdmissionReason::Burden(burden) => {
                scores.push(score("burden", burden_score(burden)))
            }
            _ => {}
        }
    }
    scores
}

fn survival_pressure_score(admission: &RewardAdmission) -> i32 {
    let provides_block = admission_provides(admission, Mechanic::Block);
    let provides_draw = admission_provides(admission, Mechanic::CardDraw);
    let mitigates = admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::EnemyStrengthDown);
    if provides_block && provides_draw {
        65
    } else if mitigates {
        55
    } else if provides_block {
        40
    } else if provides_draw {
        25
    } else if admission_provides(admission, Mechanic::Energy) {
        15
    } else {
        0
    }
}

fn shop_card_buy_is_risky(kind: DecisionCandidateKind, admission: &RewardAdmission) -> bool {
    let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind else {
        return false;
    };
    let definition = card_definition_with_upgrades(card, upgrades);
    definition.burdens.iter().any(|burden| {
        matches!(
            burden,
            CardBurden::RandomExhaust
                | CardBurden::AddsCombatDeckClutter
                | CardBurden::HpCost
                | CardBurden::DrawLockout
                | CardBurden::ExhaustsHand
        )
    }) || admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::DuplicateBurden(_) | RewardAdmissionReason::DuplicateConcern(_)
        )
    })
}

fn burden_score(burden: CardBurden) -> i32 {
    match burden {
        CardBurden::PowerSetup => -10,
        CardBurden::HpCost => -35,
        CardBurden::DrawLockout => -30,
        CardBurden::AddsCombatDeckClutter => -35,
        CardBurden::RandomExhaust => -30,
        CardBurden::ExhaustsHand => -45,
        CardBurden::RequiresEnemyAttackIntent => -15,
        CardBurden::CardBlockLockoutUntilNextTurn => -25,
    }
}

fn admission_provides(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(mechanic))
}

fn score(by: &'static str, value: i32) -> ScoreComponent {
    ScoreComponent { by, value }
}
