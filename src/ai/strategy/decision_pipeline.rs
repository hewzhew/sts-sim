use crate::ai::analysis::card_semantics::{card_definition_with_upgrades, CardBurden, Mechanic};
use crate::ai::strategy::acquisition::{
    assess_card_acquisition, evaluate_deck_construction_contract, AcquisitionContext,
    AcquisitionPolicyVerdict,
};
use crate::ai::strategy::boss_relic_admission::{
    boss_relic_admission_order_rank, skip_boss_relic_admission, BossRelicAdmission,
};
use crate::ai::strategy::boss_scaling_evidence::assess_boss_scaling_evidence;
use crate::ai::strategy::deck_admission::DeckAdmission;
use crate::ai::strategy::deck_construction_pressure::ConstructionLaneAdjustment;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::deck_strategic_deficit::{StrategicBurdenLevel, StrategicDeficitLevel};
use crate::ai::strategy::reward_admission::{
    RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
};
use crate::ai::strategy::reward_quality::RewardDuplicateConcern;
use crate::ai::strategy::role_saturation::{
    assess_role_saturation, marginal_reason_label, LaneCap, RoleSaturationCandidate,
};
use crate::ai::strategy::shop_purchase_bundle::{
    evaluate_shop_purchase_bundle, ShopGoldOpportunity, ShopPurchaseBundleVerdict,
};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionPipelineContext {
    pub deck_plan: DeckPlanSnapshot,
    pub gold: Option<i32>,
    pub shop_investment: Option<ShopInvestmentEvidence>,
    pub shop_gold_opportunity: Option<ShopGoldOpportunity>,
}

impl DecisionPipelineContext {
    pub fn reward(deck_plan: DeckPlanSnapshot) -> Self {
        Self {
            deck_plan,
            gold: None,
            shop_investment: None,
            shop_gold_opportunity: None,
        }
    }

    pub fn shop(deck_plan: DeckPlanSnapshot, gold: i32) -> Self {
        Self {
            deck_plan,
            gold: Some(gold),
            shop_investment: None,
            shop_gold_opportunity: None,
        }
    }

    pub fn with_shop_investment(self, shop_investment: ShopInvestmentEvidence) -> Self {
        Self {
            shop_investment: Some(shop_investment),
            ..self
        }
    }

    pub fn with_shop_gold_opportunity(self, shop_gold_opportunity: ShopGoldOpportunity) -> Self {
        Self {
            shop_gold_opportunity: Some(shop_gold_opportunity),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopInvestmentEvidence {
    pub membership_card: MembershipCardInvestmentEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipCardInvestmentEvidence {
    SameShopAmortized,
    SameShopUnamortized,
    FutureShop,
    NoPayoff,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionCandidateIr {
    pub kind: DecisionCandidateKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionCandidateKind {
    CardRewardPick {
        card: CardId,
        upgrades: u8,
    },
    CardRewardSkip,
    BossRelicPick {
        relic: RelicId,
    },
    BossRelicSkip,
    ShopBuyCard {
        card: CardId,
        upgrades: u8,
        price: i32,
    },
    ShopBuyRelic {
        relic: RelicId,
        price: i32,
    },
    ShopBuyPotion {
        potion: PotionId,
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
pub enum FilterDecision {
    Pass,
    InspectOnly(&'static str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScoreComponent {
    pub by: &'static str,
    pub value: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateLaneCapSource {
    RoleSaturation,
    Strategic,
    Acquisition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateLaneCap {
    pub source: CandidateLaneCapSource,
    pub cap: LaneCap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateLaneAdjudication {
    pub raw_lane: CandidateLane,
    pub final_lane: CandidateLane,
    pub caps: Vec<CandidateLaneCap>,
}

impl CandidateLaneAdjudication {
    pub fn uncapped(lane: CandidateLane) -> Self {
        Self {
            raw_lane: lane,
            final_lane: lane,
            caps: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CandidateOrderKey {
    pub lane_rank: u8,
    pub score_rank: i32,
    pub tiebreak_rank: u8,
}

impl CandidateOrderKey {
    pub fn fallback() -> Self {
        Self {
            lane_rank: 3,
            score_rank: 0,
            tiebreak_rank: 9,
        }
    }

    pub fn with_auto_rank(self, auto_rank: u8) -> (u8, Self) {
        (auto_rank, self)
    }

    pub fn optional_skip(has_mainline_take: bool) -> Self {
        Self {
            lane_rank: candidate_lane_rank(CandidateLane::Skip, has_mainline_take),
            score_rank: 0,
            tiebreak_rank: candidate_tiebreak_rank(DecisionCandidateKind::CardRewardSkip),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateEvaluation {
    pub candidate: DecisionCandidateIr,
    pub lane: CandidateLane,
    pub adjudication: CandidateLaneAdjudication,
    pub expansion: ExpansionPlan,
    pub scores: Vec<ScoreComponent>,
}

impl CandidateEvaluation {
    pub fn inspect_only(candidate: DecisionCandidateIr, reason: &'static str) -> Self {
        Self {
            candidate,
            lane: CandidateLane::Reject,
            adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Reject),
            expansion: ExpansionPlan::InspectOnly(reason),
            scores: Vec::new(),
        }
    }

    pub fn total_score(&self) -> i32 {
        self.scores.iter().map(|score| score.value).sum()
    }

    pub fn is_mainline(&self) -> bool {
        self.lane == CandidateLane::Mainline
    }

    pub fn auto_expands(&self) -> bool {
        self.expansion == ExpansionPlan::Auto
    }

    pub fn inspect_only_reason(&self) -> Option<&'static str> {
        match self.expansion {
            ExpansionPlan::Auto => None,
            ExpansionPlan::InspectOnly(reason) => Some(reason),
        }
    }

    pub fn order_key(&self, has_mainline_take: bool) -> CandidateOrderKey {
        CandidateOrderKey {
            lane_rank: candidate_lane_rank(self.lane, has_mainline_take),
            score_rank: -self.total_score(),
            tiebreak_rank: candidate_tiebreak_rank(self.candidate.kind),
        }
    }

    pub fn auto_order_key(&self, has_mainline_take: bool) -> (u8, CandidateOrderKey) {
        self.order_key(has_mainline_take)
            .with_auto_rank(u8::from(!self.auto_expands()))
    }
}

type FilterPass =
    fn(DecisionPipelineContext, DecisionCandidateIr, Option<&RewardAdmission>) -> FilterDecision;
type ScorePass = fn(
    DecisionPipelineContext,
    DecisionCandidateIr,
    Option<&RewardAdmission>,
    &mut Vec<ScoreComponent>,
);

pub fn evaluate_decision_candidate(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<&RewardAdmission>,
) -> CandidateEvaluation {
    let candidate = DecisionCandidateIr { kind };
    for pass in filter_passes() {
        if let FilterDecision::InspectOnly(reason) = pass(context, candidate, admission) {
            return CandidateEvaluation::inspect_only(candidate, reason);
        }
    }

    let mut scores = Vec::new();
    for pass in score_passes() {
        pass(context, candidate, admission, &mut scores);
    }
    let saturation = assess_role_saturation(
        context.deck_plan,
        role_saturation_candidate(candidate.kind),
        admission,
    );
    for penalty in &saturation.penalties {
        scores.push(score(
            marginal_reason_label(penalty.reason),
            penalty.score_delta,
        ));
    }
    let adjudication = adjudicate_candidate_lane(
        candidate.kind,
        scores.iter().map(|score| score.value).sum(),
        saturation.lane_cap,
        strategic_lane_cap(context, candidate.kind, admission),
        acquisition_lane_cap(context, candidate.kind, admission),
    );
    let lane = adjudication.final_lane;
    let expansion = expansion_for_candidate(candidate.kind, lane);
    CandidateEvaluation {
        candidate,
        lane,
        adjudication,
        expansion,
        scores,
    }
}

pub fn boss_relic_order_key(
    kind: DecisionCandidateKind,
    admission: Option<&BossRelicAdmission>,
) -> CandidateOrderKey {
    match kind {
        DecisionCandidateKind::BossRelicPick { .. } | DecisionCandidateKind::BossRelicSkip => {
            CandidateOrderKey {
                lane_rank: admission
                    .map(boss_relic_admission_order_rank)
                    .unwrap_or_else(|| {
                        boss_relic_admission_order_rank(&skip_boss_relic_admission())
                    }),
                score_rank: 0,
                tiebreak_rank: candidate_tiebreak_rank(kind),
            }
        }
        _ => CandidateOrderKey::fallback(),
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

pub fn candidate_lane_rank(lane: CandidateLane, has_mainline_take: bool) -> u8 {
    match lane {
        CandidateLane::Mainline => 0,
        CandidateLane::Skip => {
            if has_mainline_take {
                1
            } else {
                0
            }
        }
        CandidateLane::Probe => {
            if has_mainline_take {
                2
            } else {
                1
            }
        }
        CandidateLane::Reject => 3,
    }
}

pub fn candidate_tiebreak_rank(kind: DecisionCandidateKind) -> u8 {
    match kind {
        DecisionCandidateKind::ShopPurge { target } => match target {
            CleanupTarget::Curse => 0,
            CleanupTarget::Status => 1,
            CleanupTarget::StarterStrike => 2,
            CleanupTarget::StarterDefend => 3,
            CleanupTarget::OtherStarter => 4,
            CleanupTarget::Other => 5,
        },
        DecisionCandidateKind::ShopOpenRewards => 1,
        DecisionCandidateKind::ShopBuyRelic { .. } => 2,
        DecisionCandidateKind::ShopBuyPotion { .. } => 3,
        DecisionCandidateKind::ShopBuyCard { .. } => 4,
        DecisionCandidateKind::ShopLeave => 5,
        DecisionCandidateKind::CardRewardPick { .. } => 6,
        DecisionCandidateKind::CardRewardSkip => 7,
        DecisionCandidateKind::BossRelicPick { .. } => 8,
        DecisionCandidateKind::BossRelicSkip => 9,
        DecisionCandidateKind::Unsupported => 9,
    }
}

fn filter_passes() -> &'static [FilterPass] {
    &[
        unsupported_candidate_filter,
        missing_card_admission_filter,
        shop_affordability_filter,
        shop_investment_filter,
        shop_purchase_bundle_filter,
        shop_followup_required_filter,
        cleanup_target_filter,
        unmodeled_card_filter,
        thin_support_filter,
        duplicate_marginal_filter,
        unsupported_payoff_filter,
        risky_shop_card_filter,
        shop_card_acquisition_filter,
    ]
}

fn score_passes() -> &'static [ScorePass] {
    &[
        static_candidate_score,
        cleanup_score,
        admission_class_score,
        strategic_deficit_score,
        deck_admission_score,
        construction_pressure_score,
        latent_quality_score,
        reward_reason_score,
        payoff_support_quality_score,
        shop_investment_score,
        shop_purchase_bundle_score,
        shop_relic_score,
        shop_potion_score,
        survival_pressure_score,
    ]
}

fn unsupported_candidate_filter(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    if candidate.kind == DecisionCandidateKind::Unsupported {
        FilterDecision::InspectOnly("unsupported decision candidate")
    } else {
        FilterDecision::Pass
    }
}

fn missing_card_admission_filter(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    if candidate_requires_card_admission(candidate.kind) && admission.is_none() {
        FilterDecision::InspectOnly("card candidate missing admission")
    } else {
        FilterDecision::Pass
    }
}

fn shop_affordability_filter(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    match (candidate.kind, context.gold) {
        (
            DecisionCandidateKind::ShopBuyCard { price, .. }
            | DecisionCandidateKind::ShopBuyRelic { price, .. }
            | DecisionCandidateKind::ShopBuyPotion { price, .. },
            Some(gold),
        ) if price > gold => FilterDecision::InspectOnly("shop item is unaffordable"),
        _ => FilterDecision::Pass,
    }
}

fn shop_investment_filter(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    match membership_card_investment(context, candidate.kind) {
        ShopInvestmentDecision::Reject(reason) => FilterDecision::InspectOnly(reason),
        ShopInvestmentDecision::Mainline
        | ShopInvestmentDecision::Probe
        | ShopInvestmentDecision::NotInvestment => FilterDecision::Pass,
    }
}

fn shop_purchase_bundle_filter(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let Some(opportunity) = context.shop_gold_opportunity else {
        return FilterDecision::Pass;
    };
    if !is_shop_bundle_candidate(candidate.kind) {
        return FilterDecision::Pass;
    }
    let evaluation = CandidateEvaluation {
        candidate,
        lane: CandidateLane::Mainline,
        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
        expansion: ExpansionPlan::Auto,
        scores: Vec::new(),
    };
    let bundle = evaluate_shop_purchase_bundle(opportunity, &evaluation);
    if bundle.verdict == ShopPurchaseBundleVerdict::Reject {
        FilterDecision::InspectOnly(bundle.reason)
    } else {
        FilterDecision::Pass
    }
}

fn shop_followup_required_filter(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    match candidate.kind {
        DecisionCandidateKind::ShopBuyRelic { relic, .. }
            if shop_relic_purchase_needs_followup(relic) =>
        {
            FilterDecision::InspectOnly("shop relic purchase opens a follow-up choice")
        }
        _ => FilterDecision::Pass,
    }
}

fn cleanup_target_filter(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
) -> FilterDecision {
    match candidate.kind {
        DecisionCandidateKind::ShopPurge {
            target: CleanupTarget::Curse | CleanupTarget::Status | CleanupTarget::StarterStrike,
        } => FilterDecision::Pass,
        DecisionCandidateKind::ShopPurge { .. } => {
            FilterDecision::InspectOnly("shop purge target is not safe for tiny owner")
        }
        _ => FilterDecision::Pass,
    }
}

fn unmodeled_card_filter(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    if admission.is_some_and(|admission| admission.class == RewardAdmissionClass::EmptyOrDeferred) {
        FilterDecision::InspectOnly("unmodeled or deferred card candidate")
    } else {
        FilterDecision::Pass
    }
}

fn thin_support_filter(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let thin = admission.is_some_and(|admission| {
        admission
            .reasons
            .iter()
            .any(|reason| matches!(reason, RewardAdmissionReason::ThinSupport(_)))
    });
    if thin {
        FilterDecision::InspectOnly("payoff support is too thin")
    } else {
        FilterDecision::Pass
    }
}

fn duplicate_marginal_filter(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let low_marginal = admission.is_some_and(|admission| {
        admission.reasons.iter().any(|reason| {
            matches!(reason, RewardAdmissionReason::DuplicateBurden(_))
                || matches!(
                    reason,
                    RewardAdmissionReason::DuplicateConcern(concern)
                        if concern.is_hard_penalty()
                )
        })
    });
    if low_marginal {
        FilterDecision::InspectOnly("duplicate marginal value is too low")
    } else {
        FilterDecision::Pass
    }
}

fn unsupported_payoff_filter(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    if admission
        .is_some_and(|admission| admission.class == RewardAdmissionClass::OpensUnsupportedPayoff)
    {
        FilterDecision::InspectOnly("unsupported payoff candidate")
    } else {
        FilterDecision::Pass
    }
}

fn risky_shop_card_filter(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = candidate.kind else {
        return FilterDecision::Pass;
    };
    if is_hard_boss_answer_shop_bundle(context, candidate) {
        return FilterDecision::Pass;
    }
    let definition = card_definition_with_upgrades(card, upgrades);
    let card_risk = definition.burdens.iter().any(|burden| {
        matches!(
            burden,
            CardBurden::RandomExhaust
                | CardBurden::AddsCombatDeckClutter
                | CardBurden::HpCost
                | CardBurden::DrawLockout
                | CardBurden::ExhaustsHand
        )
    });
    let duplicate_risk = admission.is_some_and(|admission| {
        admission.reasons.iter().any(|reason| {
            matches!(
                reason,
                RewardAdmissionReason::DuplicateBurden(_)
                    | RewardAdmissionReason::DuplicateConcern(_)
            )
        })
    });
    if card_risk || duplicate_risk {
        FilterDecision::InspectOnly("shop card buy carries unresolved risk")
    } else {
        FilterDecision::Pass
    }
}

fn is_hard_boss_answer_shop_bundle(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
) -> bool {
    let Some(opportunity) = context.shop_gold_opportunity else {
        return false;
    };
    if !is_shop_bundle_candidate(candidate.kind) {
        return false;
    }
    let evaluation = CandidateEvaluation {
        candidate,
        lane: CandidateLane::Mainline,
        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
        expansion: ExpansionPlan::Auto,
        scores: Vec::new(),
    };
    evaluate_shop_purchase_bundle(opportunity, &evaluation).verdict
        == ShopPurchaseBundleVerdict::HardBossAnswerBuy
}

fn shop_card_acquisition_filter(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let DecisionCandidateKind::ShopBuyCard {
        card,
        upgrades,
        price,
    } = candidate.kind
    else {
        return FilterDecision::Pass;
    };
    let (Some(admission), Some(gold)) = (admission, context.gold) else {
        return FilterDecision::Pass;
    };
    if is_hard_boss_answer_shop_bundle(context, candidate) {
        return FilterDecision::Pass;
    }
    let report = assess_card_acquisition(
        AcquisitionContext::shop(context.deck_plan, gold, price),
        card,
        upgrades,
        admission,
    );
    let policy = evaluate_deck_construction_contract(&report);
    match policy.inspect_only_reason() {
        None => FilterDecision::Pass,
        Some(reason) => FilterDecision::InspectOnly(reason),
    }
}

fn static_candidate_score(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    match candidate.kind {
        DecisionCandidateKind::CardRewardSkip => scores.push(score("skip", 0)),
        DecisionCandidateKind::ShopOpenRewards => scores.push(score("open-rewards", 300)),
        DecisionCandidateKind::ShopLeave => scores.push(score("leave", 0)),
        _ => {}
    }
}

fn cleanup_score(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let DecisionCandidateKind::ShopPurge { target } = candidate.kind else {
        return;
    };
    scores.push(score(
        "cleanup-target",
        match target {
            CleanupTarget::Curse => 320,
            CleanupTarget::Status => 260,
            CleanupTarget::StarterStrike => 180,
            CleanupTarget::StarterDefend | CleanupTarget::OtherStarter | CleanupTarget::Other => 0,
        },
    ));
}

fn admission_class_score(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(admission) = admission else {
        return;
    };
    scores.push(score(
        "admission-class",
        match admission.class {
            RewardAdmissionClass::ClosesRequirement => 130,
            RewardAdmissionClass::BuildsSupportedPackage => 105,
            RewardAdmissionClass::EngineSeed => 65,
            RewardAdmissionClass::ImmediateWork => 55,
            RewardAdmissionClass::BurdenedImmediateWork => 25,
            RewardAdmissionClass::OpensUnsupportedPayoff
            | RewardAdmissionClass::EmptyOrDeferred
            | RewardAdmissionClass::Skip => 0,
        },
    ));
}

fn strategic_deficit_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    if !matches!(
        candidate.kind,
        DecisionCandidateKind::CardRewardPick { .. } | DecisionCandidateKind::ShopBuyCard { .. }
    ) {
        return;
    }
    let Some(admission) = admission else {
        return;
    };
    let deficit = context.deck_plan.strategic_deficit;
    let mut improves = false;

    if needs(deficit.deck_access)
        && (admission_provides(admission, Mechanic::CardDraw)
            || admission
                .reasons
                .contains(&RewardAdmissionReason::CombatUpgrade))
    {
        improves = true;
        scores.push(score("strategic-access-gap", 55));
    }
    if needs(deficit.energy_or_playability) && admission_provides(admission, Mechanic::Energy) {
        improves = true;
        scores.push(score("strategic-energy-gap", 50));
    }
    if needs(deficit.aoe_or_minion_control) && admission_aoe(admission) {
        improves = true;
        scores.push(score("strategic-aoe-gap", 50));
    }
    if needs(deficit.block_or_mitigation) && admission_survival_tool(admission) {
        improves = true;
        scores.push(score("strategic-survival-gap", 40));
    }
    if needs(deficit.boss_scaling_plan) {
        let evidence = assess_boss_scaling_evidence(
            context.deck_plan,
            candidate_card(candidate.kind),
            admission,
        );
        if evidence.score_delta != 0 {
            scores.push(score(evidence.label, evidence.score_delta));
        }
        if evidence.relevant_to_boss_plan && !fragile_supported_payoff(context, admission) {
            improves = true;
        }
    }
    if needs(deficit.frontload_damage) && admission_frontloads(admission) {
        improves = true;
        scores.push(score("strategic-frontload-gap", 35));
    }

    if !improves && heavy_burden_penalty_applies(context, admission) {
        scores.push(score("strategic-burden-no-gap", -85));
    }
    if deficit.too_many_low_impact_attacks
        && deficit.frontload_damage == StrategicDeficitLevel::Surplus
        && admission_frontloads(admission)
        && !admission_aoe(admission)
        && !admission_scaling_or_engine(admission)
        && !admission_provides(admission, Mechanic::CardDraw)
    {
        scores.push(score("strategic-frontload-saturated", -65));
    }
}

fn deck_admission_score(
    context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(admission) = admission else {
        return;
    };
    scores.push(score(
        "deck-admission",
        match context.deck_plan.deck_admission(admission) {
            DeckAdmission::Welcome => 0,
            DeckAdmission::Conditional => -30,
            DeckAdmission::Discouraged => -90,
        },
    ));
}

fn construction_pressure_score(
    context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(admission) = admission else {
        return;
    };
    scores.push(score(
        "construction-pressure",
        match context.deck_plan.reward_lane_adjustment(admission) {
            ConstructionLaneAdjustment::None => 0,
            ConstructionLaneAdjustment::PromoteOneStep => 35,
            ConstructionLaneAdjustment::PromoteToMainline => 70,
            ConstructionLaneAdjustment::SoftDemote => -45,
            ConstructionLaneAdjustment::HardDemote => -130,
        },
    ));
}

fn latent_quality_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    if !matches!(
        candidate.kind,
        DecisionCandidateKind::CardRewardPick { .. } | DecisionCandidateKind::ShopBuyCard { .. }
    ) {
        return;
    }
    let Some(admission) = admission else {
        return;
    };
    if has_combat_upgrade(admission) && context.deck_plan.roles.upgrade_access_units == 0 {
        let value = if context.deck_plan.context.act <= 1 {
            55
        } else {
            35
        };
        scores.push(score("latent-upgrade-leverage", value));
    }
    if has_duplicate_access_copy(admission) {
        scores.push(score("marginal-access-copy", -25));
    }
}

fn reward_reason_score(
    _context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(admission) = admission else {
        return;
    };
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
}

fn payoff_support_quality_score(
    context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(admission) = admission else {
        return;
    };
    if fragile_supported_payoff(context, admission) {
        scores.push(score("payoff-support-fragile", -80));
    }
}

fn shop_investment_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    match membership_card_investment(context, candidate.kind) {
        ShopInvestmentDecision::Mainline => scores.push(score("shop-investment", 150)),
        ShopInvestmentDecision::Probe => scores.push(score("shop-investment", 55)),
        ShopInvestmentDecision::Reject(_) | ShopInvestmentDecision::NotInvestment => {}
    }
}

fn shop_purchase_bundle_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let Some(opportunity) = context.shop_gold_opportunity else {
        return;
    };
    if !is_shop_bundle_candidate(candidate.kind) {
        return;
    }
    let evaluation = CandidateEvaluation {
        candidate,
        lane: CandidateLane::Mainline,
        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
        expansion: ExpansionPlan::Auto,
        scores: Vec::new(),
    };
    let bundle = evaluate_shop_purchase_bundle(opportunity, &evaluation);
    let label = match bundle.verdict {
        ShopPurchaseBundleVerdict::HardSurvivalBuy => "shop-bundle-hard-survival",
        ShopPurchaseBundleVerdict::HardBossAnswerBuy => "shop-bundle-boss-answer",
        ShopPurchaseBundleVerdict::EfficientBundleBuy => "shop-bundle-efficient",
        ShopPurchaseBundleVerdict::ContextBuy => "shop-bundle-context",
        ShopPurchaseBundleVerdict::PreserveGoldPreferred => "shop-bundle-preserve-gold",
        ShopPurchaseBundleVerdict::Reject => "shop-bundle-reject",
    };
    scores.push(score(label, shop_purchase_bundle_score_value(&bundle)));
}

fn shop_purchase_bundle_score_value(
    bundle: &crate::ai::strategy::shop_purchase_bundle::ShopPurchaseBundleDecision,
) -> i32 {
    match bundle.verdict {
        ShopPurchaseBundleVerdict::HardSurvivalBuy => 180,
        ShopPurchaseBundleVerdict::HardBossAnswerBuy => match bundle.facts.kind {
            crate::ai::strategy::shop_purchase_bundle::ShopPurchaseBundleKind::BuyOneCard => 240,
            _ => 160,
        },
        ShopPurchaseBundleVerdict::EfficientBundleBuy => 120,
        ShopPurchaseBundleVerdict::ContextBuy => 70,
        ShopPurchaseBundleVerdict::PreserveGoldPreferred => 50,
        ShopPurchaseBundleVerdict::Reject => 0,
    }
}

fn shop_relic_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let DecisionCandidateKind::ShopBuyRelic { relic, .. } = candidate.kind else {
        return;
    };
    for component in shop_relic_score_components(context.deck_plan, relic) {
        scores.push(component);
    }
}

fn shop_potion_score(
    context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let DecisionCandidateKind::ShopBuyPotion { potion, .. } = candidate.kind else {
        return;
    };
    let mut value = shop_potion_score_value(potion);
    if context.deck_plan.survival_pressure() && value > 0 {
        value += 40;
    }
    scores.push(score("shop-potion", value));
}

fn survival_pressure_score(
    context: DecisionPipelineContext,
    _candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    if !context.deck_plan.survival_pressure() {
        return;
    }
    let Some(admission) = admission else {
        return;
    };
    let provides_block = admission_provides(admission, Mechanic::Block);
    let provides_draw = admission_provides(admission, Mechanic::CardDraw);
    let mitigates = admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::EnemyStrengthDown);
    scores.push(score(
        "survival-pressure",
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
        },
    ));
}

fn lane_for_candidate(kind: DecisionCandidateKind, score: i32) -> CandidateLane {
    match kind {
        DecisionCandidateKind::CardRewardSkip
        | DecisionCandidateKind::BossRelicSkip
        | DecisionCandidateKind::ShopLeave => CandidateLane::Skip,
        _ if score >= 110 => CandidateLane::Mainline,
        _ if score >= 45 => CandidateLane::Probe,
        _ => CandidateLane::Reject,
    }
}

fn adjudicate_candidate_lane(
    kind: DecisionCandidateKind,
    score: i32,
    role_saturation: Option<LaneCap>,
    strategic: Option<LaneCap>,
    acquisition: Option<LaneCap>,
) -> CandidateLaneAdjudication {
    let raw_lane = lane_for_candidate(kind, score);
    let caps = lane_caps(role_saturation, strategic, acquisition);
    let cap = caps
        .iter()
        .fold(None, |cap, next| stricter_lane_cap(cap, Some(next.cap)));
    CandidateLaneAdjudication {
        raw_lane,
        final_lane: capped_lane(raw_lane, cap),
        caps,
    }
}

fn lane_caps(
    role_saturation: Option<LaneCap>,
    strategic: Option<LaneCap>,
    acquisition: Option<LaneCap>,
) -> Vec<CandidateLaneCap> {
    [
        (CandidateLaneCapSource::RoleSaturation, role_saturation),
        (CandidateLaneCapSource::Strategic, strategic),
        (CandidateLaneCapSource::Acquisition, acquisition),
    ]
    .into_iter()
    .filter_map(|(source, cap)| cap.map(|cap| CandidateLaneCap { source, cap }))
    .collect()
}

fn capped_lane(lane: CandidateLane, cap: Option<LaneCap>) -> CandidateLane {
    match (lane, cap) {
        (CandidateLane::Mainline, Some(LaneCap::ProbeOnly)) => CandidateLane::Probe,
        (CandidateLane::Mainline | CandidateLane::Probe, Some(LaneCap::Reject)) => {
            CandidateLane::Reject
        }
        _ => lane,
    }
}

fn stricter_lane_cap(left: Option<LaneCap>, right: Option<LaneCap>) -> Option<LaneCap> {
    match (left, right) {
        (Some(LaneCap::Reject), _) | (_, Some(LaneCap::Reject)) => Some(LaneCap::Reject),
        (Some(LaneCap::ProbeOnly), _) | (_, Some(LaneCap::ProbeOnly)) => Some(LaneCap::ProbeOnly),
        _ => None,
    }
}

fn strategic_lane_cap(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<&RewardAdmission>,
) -> Option<LaneCap> {
    if !matches!(
        kind,
        DecisionCandidateKind::CardRewardPick { .. } | DecisionCandidateKind::ShopBuyCard { .. }
    ) {
        return None;
    }
    let admission = admission?;
    if !heavy_burden_penalty_applies(context, admission) {
        return None;
    }
    Some(LaneCap::ProbeOnly)
}

fn acquisition_lane_cap(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<&RewardAdmission>,
) -> Option<LaneCap> {
    if is_hard_boss_answer_shop_bundle(context, DecisionCandidateIr { kind }) {
        return None;
    }
    let admission = admission?;
    let (card, upgrades, source) = match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades } => (
            card,
            upgrades,
            AcquisitionContext::reward(context.deck_plan),
        ),
        DecisionCandidateKind::ShopBuyCard {
            card,
            upgrades,
            price,
        } => {
            let gold = context.gold?;
            (
                card,
                upgrades,
                AcquisitionContext::shop(context.deck_plan, gold, price),
            )
        }
        _ => return None,
    };
    let report = assess_card_acquisition(source, card, upgrades, admission);
    let policy = evaluate_deck_construction_contract(&report);
    match policy.verdict {
        AcquisitionPolicyVerdict::AutoAcquire | AcquisitionPolicyVerdict::ContextTake => None,
        AcquisitionPolicyVerdict::Speculative | AcquisitionPolicyVerdict::SkipPreferred => {
            Some(LaneCap::ProbeOnly)
        }
        AcquisitionPolicyVerdict::Reject => Some(LaneCap::Reject),
    }
}

fn role_saturation_candidate(kind: DecisionCandidateKind) -> Option<RoleSaturationCandidate> {
    match kind {
        DecisionCandidateKind::CardRewardPick { upgrades, .. } => Some(RoleSaturationCandidate {
            upgrades,
            is_shop_card: false,
        }),
        DecisionCandidateKind::ShopBuyCard { upgrades, .. } => Some(RoleSaturationCandidate {
            upgrades,
            is_shop_card: true,
        }),
        _ => None,
    }
}

fn expansion_for_candidate(kind: DecisionCandidateKind, lane: CandidateLane) -> ExpansionPlan {
    match (kind, lane) {
        (_, CandidateLane::Reject) => ExpansionPlan::InspectOnly("candidate score rejected"),
        (DecisionCandidateKind::CardRewardPick { .. }, CandidateLane::Probe) => {
            ExpansionPlan::InspectOnly("card reward pick is below mainline")
        }
        (DecisionCandidateKind::ShopBuyCard { .. }, CandidateLane::Probe) => {
            ExpansionPlan::InspectOnly("shop card buy is below mainline")
        }
        (DecisionCandidateKind::ShopBuyRelic { .. }, CandidateLane::Probe) => {
            ExpansionPlan::InspectOnly("shop relic buy is below mainline")
        }
        (DecisionCandidateKind::ShopBuyPotion { .. }, CandidateLane::Probe) => {
            ExpansionPlan::InspectOnly("shop potion buy is below mainline")
        }
        _ => ExpansionPlan::Auto,
    }
}

fn candidate_requires_card_admission(kind: DecisionCandidateKind) -> bool {
    matches!(
        kind,
        DecisionCandidateKind::CardRewardPick { .. } | DecisionCandidateKind::ShopBuyCard { .. }
    )
}

fn is_shop_bundle_candidate(kind: DecisionCandidateKind) -> bool {
    matches!(
        kind,
        DecisionCandidateKind::ShopBuyCard { .. }
            | DecisionCandidateKind::ShopBuyRelic { .. }
            | DecisionCandidateKind::ShopBuyPotion { .. }
            | DecisionCandidateKind::ShopPurge { .. }
            | DecisionCandidateKind::ShopLeave
    )
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

fn needs(level: StrategicDeficitLevel) -> bool {
    matches!(
        level,
        StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
    )
}

fn ordinary_reward_addition(admission: &RewardAdmission) -> bool {
    matches!(
        admission.class,
        RewardAdmissionClass::ImmediateWork | RewardAdmissionClass::BurdenedImmediateWork
    )
}

fn heavy_burden_penalty_applies(
    context: DecisionPipelineContext,
    admission: &RewardAdmission,
) -> bool {
    context.deck_plan.strategic_deficit.deck_burden == StrategicBurdenLevel::Heavy
        && ordinary_reward_addition(admission)
        && !improves_strategic_gap(context, admission)
        && !heavy_burden_exception(context, admission)
}

fn heavy_burden_exception(context: DecisionPipelineContext, admission: &RewardAdmission) -> bool {
    survival_pressure_exception(context, admission)
        || (admission_provides(admission, Mechanic::CardDraw)
            && admission_provides(admission, Mechanic::Energy))
        || admission
            .reasons
            .iter()
            .any(|reason| matches!(reason, RewardAdmissionReason::RunReward(_)))
}

fn improves_strategic_gap(context: DecisionPipelineContext, admission: &RewardAdmission) -> bool {
    let deficit = context.deck_plan.strategic_deficit;
    (needs(deficit.deck_access)
        && (admission_provides(admission, Mechanic::CardDraw)
            || admission
                .reasons
                .contains(&RewardAdmissionReason::CombatUpgrade)))
        || (needs(deficit.energy_or_playability) && admission_provides(admission, Mechanic::Energy))
        || (needs(deficit.aoe_or_minion_control) && admission_aoe(admission))
        || (needs(deficit.block_or_mitigation) && admission_survival_tool(admission))
        || (needs(deficit.boss_scaling_plan)
            && assess_boss_scaling_evidence(context.deck_plan, None, admission)
                .relevant_to_boss_plan
            && !fragile_supported_payoff(context, admission))
        || (needs(deficit.frontload_damage) && admission_frontloads(admission))
}

fn survival_pressure_exception(
    context: DecisionPipelineContext,
    admission: &RewardAdmission,
) -> bool {
    context.deck_plan.survival_pressure()
        && (admission_provides(admission, Mechanic::EnemyStrengthDown)
            || admission_provides(admission, Mechanic::Weak)
            || (admission_provides(admission, Mechanic::Block)
                && admission_provides(admission, Mechanic::CardDraw)))
}

fn admission_provides(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(mechanic))
}

fn candidate_card(kind: DecisionCandidateKind) -> Option<(CardId, u8)> {
    match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades }
        | DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } => Some((card, upgrades)),
        _ => None,
    }
}

fn admission_frontloads(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::FrontloadDamage)
}

fn admission_aoe(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::AreaDamage)
}

fn admission_damage_uses(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::DamageUses(mechanic))
}

fn has_combat_upgrade(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::CombatUpgrade)
}

fn has_duplicate_access_copy(admission: &RewardAdmission) -> bool {
    admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::DuplicateConcern(RewardDuplicateConcern::DiminishingAccessCopy)
        )
    })
}

fn admission_survival_tool(admission: &RewardAdmission) -> bool {
    admission_provides(admission, Mechanic::Block)
        || admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::EnemyStrengthDown)
}

fn admission_scaling_or_engine(admission: &RewardAdmission) -> bool {
    admission_provides(admission, Mechanic::Strength)
        || admission_provides(admission, Mechanic::StrengthMultiplier)
        || admission.reasons.iter().any(|reason| {
            matches!(
                reason,
                RewardAdmissionReason::Closes(_)
                    | RewardAdmissionReason::Supports(_)
                    | RewardAdmissionReason::Installs(_)
                    | RewardAdmissionReason::DamageScalesWith(_)
                    | RewardAdmissionReason::RunReward(_)
            )
        })
}

fn fragile_supported_payoff(context: DecisionPipelineContext, admission: &RewardAdmission) -> bool {
    if !admission
        .reasons
        .iter()
        .any(|reason| matches!(reason, RewardAdmissionReason::Supports(_)))
    {
        return false;
    }
    if admission_damage_uses(admission, Mechanic::Strength) {
        return context.deck_plan.roles.strength_source_units < 2;
    }
    if admission_damage_uses(admission, Mechanic::Block) {
        let roles = context.deck_plan.roles;
        return roles.block_units < 4 && roles.cycle_block_units < 2;
    }
    false
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShopInvestmentDecision {
    Mainline,
    Probe,
    Reject(&'static str),
    NotInvestment,
}

fn membership_card_investment(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
) -> ShopInvestmentDecision {
    let DecisionCandidateKind::ShopBuyRelic {
        relic: RelicId::MembershipCard,
        price: _,
    } = kind
    else {
        return ShopInvestmentDecision::NotInvestment;
    };
    let Some(evidence) = context.shop_investment else {
        return ShopInvestmentDecision::Reject("membership card missing shop investment evidence");
    };
    match evidence.membership_card {
        MembershipCardInvestmentEvidence::SameShopAmortized => ShopInvestmentDecision::Mainline,
        MembershipCardInvestmentEvidence::SameShopUnamortized
        | MembershipCardInvestmentEvidence::FutureShop => ShopInvestmentDecision::Probe,
        MembershipCardInvestmentEvidence::NoPayoff => {
            ShopInvestmentDecision::Reject("membership card has no shop payoff evidence")
        }
    }
}

fn shop_relic_purchase_needs_followup(relic: RelicId) -> bool {
    matches!(
        relic,
        RelicId::BottledFlame
            | RelicId::BottledLightning
            | RelicId::BottledTornado
            | RelicId::Cauldron
            | RelicId::DollysMirror
            | RelicId::Orrery
    )
}

fn shop_relic_score_components(
    deck_plan: DeckPlanSnapshot,
    relic: RelicId,
) -> impl Iterator<Item = ScoreComponent> {
    let mut scores = vec![score("shop-relic", shop_relic_score_value(relic))];
    match relic {
        RelicId::ChemicalX if deck_plan.roles.x_cost_payoff_units > 0 => {
            scores.push(score("shop-relic-x-cost-payoff", 75));
        }
        RelicId::ChemicalX => scores.push(score("shop-relic-x-cost-missing", -40)),
        RelicId::PaperFrog => {
            if deck_plan.roles.vulnerable_units >= 2 {
                scores.push(score("shop-relic-vulnerable-density", 70));
            } else if deck_plan.roles.vulnerable_units == 1 {
                scores.push(score("shop-relic-vulnerable-source", 35));
            }
            if deck_plan.roles.vulnerable_units > 0 && deck_plan.roles.aoe_units > 0 {
                scores.push(score("shop-relic-vulnerable-aoe", 25));
            }
        }
        _ => {}
    }
    scores.into_iter()
}

fn shop_relic_score_value(relic: RelicId) -> i32 {
    match relic {
        RelicId::Waffle => 220,
        RelicId::MedicalKit | RelicId::OrangePellets => 150,
        RelicId::MembershipCard => 0,
        RelicId::ClockworkSouvenir | RelicId::Toolbox => 115,
        RelicId::ChemicalX => 45,
        RelicId::FrozenEye => 45,
        _ => 45,
    }
}

fn shop_potion_score_value(potion: PotionId) -> i32 {
    match potion {
        PotionId::FairyPotion | PotionId::FruitJuice | PotionId::EntropicBrew => 90,
        PotionId::FirePotion
        | PotionId::ExplosivePotion
        | PotionId::FearPotion
        | PotionId::WeakenPotion
        | PotionId::EnergyPotion
        | PotionId::StrengthPotion
        | PotionId::SteroidPotion
        | PotionId::SwiftPotion
        | PotionId::BlessingOfTheForge
        | PotionId::LiquidMemories
        | PotionId::GamblersBrew
        | PotionId::DuplicationPotion => 70,
        PotionId::BlockPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::AncientPotion
        | PotionId::RegenPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::BloodPotion => 55,
        _ => 35,
    }
}

fn score(by: &'static str, value: i32) -> ScoreComponent {
    ScoreComponent { by, value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::deck_admission::DeckAdmissionContext;
    use crate::ai::strategy::reward_admission::assess_reward_admission_from_master_deck;
    use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
    use crate::runtime::combat::CombatCard;

    fn shop_context(cards: &[CardId]) -> DecisionPipelineContext {
        shop_context_with_gold_and_hp(cards, 999, 70, 80)
    }

    fn shop_context_with_hp(
        cards: &[CardId],
        current_hp: i32,
        max_hp: i32,
    ) -> DecisionPipelineContext {
        shop_context_with_gold_and_hp(cards, 999, current_hp, max_hp)
    }

    fn shop_context_with_gold_and_hp(
        cards: &[CardId],
        gold: i32,
        current_hp: i32,
        max_hp: i32,
    ) -> DecisionPipelineContext {
        let deck: Vec<_> = cards
            .iter()
            .enumerate()
            .map(|(index, card)| CombatCard::new(*card, index as u32 + 1))
            .collect();
        DecisionPipelineContext::shop(
            DeckPlanSnapshot::from_deck(
                &deck,
                DeckAdmissionContext {
                    act: 2,
                    current_hp,
                    max_hp,
                },
                RunStrategicFacts {
                    entering_act: 3,
                    starter_basic_count: 0,
                    curse_count: 0,
                    has_energy_relic: false,
                },
            ),
            gold,
        )
    }

    fn shop_context_with_maw_bank(cards: &[CardId], gold: i32) -> DecisionPipelineContext {
        shop_context_with_gold_and_hp(cards, gold, 70, 80).with_shop_gold_opportunity(
            crate::ai::strategy::shop_purchase_bundle::ShopGoldOpportunity {
                current_gold: gold,
                active_maw_bank: true,
                future_rooms_before_next_shop: 5,
                hard_checkpoint_imminent: false,
                survival_purchase_needed: false,
                boss_answer_needed: false,
            },
        )
    }

    fn shop_context_with_maw_bank_boss_gap(cards: &[CardId], gold: i32) -> DecisionPipelineContext {
        shop_context_with_gold_and_hp(cards, gold, 70, 80).with_shop_gold_opportunity(
            crate::ai::strategy::shop_purchase_bundle::ShopGoldOpportunity {
                current_gold: gold,
                active_maw_bank: true,
                future_rooms_before_next_shop: 5,
                hard_checkpoint_imminent: true,
                survival_purchase_needed: false,
                boss_answer_needed: true,
            },
        )
    }

    fn reward_context_with_act(cards: &[CardId], act: u8) -> DecisionPipelineContext {
        let deck = test_deck(cards);
        DecisionPipelineContext::reward(DeckPlanSnapshot::from_deck(
            &deck,
            DeckAdmissionContext {
                act,
                current_hp: 70,
                max_hp: 80,
            },
            RunStrategicFacts {
                entering_act: act,
                starter_basic_count: deck
                    .iter()
                    .filter(|card| matches!(card.id, CardId::Strike | CardId::Defend))
                    .count(),
                curse_count: 0,
                has_energy_relic: false,
            },
        ))
    }

    fn reward_card_with_act(
        cards: &[CardId],
        candidate: CardId,
        upgrades: u8,
        act: u8,
    ) -> CandidateEvaluation {
        let deck = test_deck(cards);
        let context = reward_context_with_act(cards, act);
        let admission = assess_reward_admission_from_master_deck(&deck, candidate, upgrades);
        evaluate_decision_candidate(
            context,
            DecisionCandidateKind::CardRewardPick {
                card: candidate,
                upgrades,
            },
            Some(&admission),
        )
    }

    fn shop_relic(context: DecisionPipelineContext, relic: RelicId) -> CandidateEvaluation {
        evaluate_decision_candidate(
            context,
            DecisionCandidateKind::ShopBuyRelic { relic, price: 150 },
            None,
        )
    }

    fn shop_leave(context: DecisionPipelineContext) -> CandidateEvaluation {
        evaluate_decision_candidate(context, DecisionCandidateKind::ShopLeave, None)
    }

    fn shop_card(cards: &[CardId], candidate: CardId) -> CandidateEvaluation {
        shop_card_with_upgrades(cards, candidate, 0)
    }

    fn shop_card_with_upgrades(
        cards: &[CardId],
        candidate: CardId,
        upgrades: u8,
    ) -> CandidateEvaluation {
        let deck = test_deck(cards);
        let context = shop_context(cards);
        shop_card_in_context(context, &deck, candidate, upgrades)
    }

    fn shop_card_in_context(
        context: DecisionPipelineContext,
        deck: &[CombatCard],
        candidate: CardId,
        upgrades: u8,
    ) -> CandidateEvaluation {
        shop_card_in_context_with_price(context, deck, candidate, upgrades, 80)
    }

    fn shop_card_in_context_with_price(
        context: DecisionPipelineContext,
        deck: &[CombatCard],
        candidate: CardId,
        upgrades: u8,
        price: i32,
    ) -> CandidateEvaluation {
        let admission = assess_reward_admission_from_master_deck(&deck, candidate, upgrades);
        evaluate_decision_candidate(
            context,
            DecisionCandidateKind::ShopBuyCard {
                card: candidate,
                upgrades,
                price,
            },
            Some(&admission),
        )
    }

    fn test_deck(cards: &[CardId]) -> Vec<CombatCard> {
        cards
            .iter()
            .enumerate()
            .map(|(index, card)| CombatCard::new(*card, index as u32 + 1))
            .collect()
    }

    fn act2_collector_pressure_deck() -> Vec<CardId> {
        vec![
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Armaments,
            CardId::Cleave,
            CardId::IronWave,
            CardId::Shockwave,
            CardId::BattleTrance,
            CardId::Whirlwind,
            CardId::ShrugItOff,
            CardId::Inflame,
            CardId::ShrugItOff,
        ]
    }

    fn act1_low_margin_reward_deck() -> Vec<CardId> {
        vec![
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Immolate,
            CardId::IronWave,
            CardId::Cleave,
            CardId::ShrugItOff,
            CardId::PommelStrike,
            CardId::Bloodletting,
        ]
    }

    #[test]
    fn reward_low_margin_filler_cannot_enter_mainline_after_basic_roles_exist() {
        let deck = act1_low_margin_reward_deck();

        let iron_wave = reward_card_with_act(&deck, CardId::IronWave, 0, 1);

        assert_eq!(iron_wave.adjudication.raw_lane, CandidateLane::Mainline);
        assert_eq!(iron_wave.adjudication.final_lane, CandidateLane::Probe);
        assert!(iron_wave
            .adjudication
            .caps
            .iter()
            .any(|cap| cap.source == CandidateLaneCapSource::Acquisition));
        assert_ne!(
            iron_wave.lane,
            CandidateLane::Mainline,
            "low-margin filler should not be promoted by stacked weak evidence: {:?}",
            iron_wave.scores
        );
    }

    #[test]
    fn reward_skip_sorts_before_probe_when_no_mainline_take_exists() {
        let deck = act1_low_margin_reward_deck();
        let iron_wave = reward_card_with_act(&deck, CardId::IronWave, 0, 1);
        let thunderclap = reward_card_with_act(&deck, CardId::ThunderClap, 0, 1);
        let skip = evaluate_decision_candidate(
            reward_context_with_act(&deck, 1),
            DecisionCandidateKind::CardRewardSkip,
            Some(&crate::ai::strategy::reward_admission::skip_reward_admission()),
        );

        assert_eq!(iron_wave.lane, CandidateLane::Probe);
        assert_eq!(thunderclap.lane, CandidateLane::Probe);
        assert_eq!(skip.lane, CandidateLane::Skip);
        assert!(
            skip.order_key(false) < iron_wave.order_key(false),
            "skip should outrank probe when there is no mainline take"
        );
        assert!(
            skip.order_key(false) < thunderclap.order_key(false),
            "skip should outrank every probe filler when there is no mainline take"
        );
    }

    #[test]
    fn reward_probe_filler_is_inspect_only_not_auto_expandable() {
        let deck = act1_low_margin_reward_deck();
        let iron_wave = reward_card_with_act(&deck, CardId::IronWave, 0, 1);

        assert_eq!(iron_wave.lane, CandidateLane::Probe);
        assert_eq!(
            iron_wave.inspect_only_reason(),
            Some("card reward pick is below mainline"),
            "probe reward picks may be visible for review, but must not auto-expand"
        );
    }

    #[test]
    fn reward_low_margin_filler_does_not_mainline_for_only_soft_gap_contact() {
        let deck = vec![
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Bash,
            CardId::Immolate,
            CardId::Cleave,
            CardId::PommelStrike,
        ];

        let iron_wave = reward_card_with_act(&deck, CardId::IronWave, 0, 1);

        assert_ne!(
            iron_wave.lane,
            CandidateLane::Mainline,
            "low-margin filler should not mainline just because it touches a soft gap: {:?}",
            iron_wave.scores
        );
    }

    #[test]
    fn shop_maw_bank_prefers_leave_over_generic_relic_purchase() {
        let context = shop_context_with_maw_bank(&act1_low_margin_reward_deck(), 224);
        let leave = shop_leave(context);
        let clockwork = evaluate_decision_candidate(
            context,
            DecisionCandidateKind::ShopBuyRelic {
                relic: RelicId::ClockworkSouvenir,
                price: 149,
            },
            None,
        );

        assert_eq!(leave.lane, CandidateLane::Skip);
        assert_ne!(
            clockwork.lane,
            CandidateLane::Mainline,
            "generic relic should not mainline through active Maw Bank: {:?}",
            clockwork.scores
        );
        assert!(
            leave.order_key(false) < clockwork.order_key(false),
            "leave-with-gold should outrank ordinary Maw Bank-breaking purchase: leave={:?} clockwork={:?}",
            leave,
            clockwork
        );
    }

    #[test]
    fn shop_boss_repair_card_can_break_maw_bank_when_boss_gap_is_open() {
        let deck_cards = act1_low_margin_reward_deck();
        let deck = test_deck(&deck_cards);
        let context = shop_context_with_maw_bank_boss_gap(&deck_cards, 288);

        let fiend_fire = shop_card_in_context_with_price(context, &deck, CardId::FiendFire, 0, 152);
        let leave = shop_leave(context);

        assert_ne!(
            fiend_fire.inspect_only_reason(),
            Some("BreaksMawBankWithoutHardNeed"),
            "deterministic boss repair should not be hidden behind Maw Bank protection"
        );
        assert!(
            fiend_fire.order_key(false) < leave.order_key(false),
            "boss-repair purchase should be eligible ahead of pure gold preservation: fiend_fire={:?} leave={:?}",
            fiend_fire,
            leave
        );
    }

    #[test]
    fn shop_boss_repair_card_orders_before_single_damage_potion() {
        let deck_cards = act1_low_margin_reward_deck();
        let deck = test_deck(&deck_cards);
        let context = shop_context_with_maw_bank_boss_gap(&deck_cards, 288);

        let fiend_fire = shop_card_in_context_with_price(context, &deck, CardId::FiendFire, 0, 152);
        let fire_potion = evaluate_decision_candidate(
            context,
            DecisionCandidateKind::ShopBuyPotion {
                potion: PotionId::FirePotion,
                price: 51,
            },
            None,
        );

        assert!(
            fiend_fire.order_key(false) < fire_potion.order_key(false),
            "boss repair payload should outrank one-shot potion when both can break Maw Bank: fiend_fire={:?} fire_potion={:?}",
            fiend_fire,
            fire_potion
        );
    }

    #[test]
    fn shop_paper_frog_beats_chemical_x_without_x_cost_payoff() {
        let context = shop_context(&[
            CardId::Shockwave,
            CardId::Uppercut,
            CardId::Cleave,
            CardId::Cleave,
        ]);

        let chemical_x = shop_relic(context, RelicId::ChemicalX);
        let paper_frog = shop_relic(context, RelicId::PaperFrog);

        assert!(
            paper_frog.total_score() > chemical_x.total_score(),
            "Paper Frog should beat dead Chemical X with vulnerable/AoE support: Paper Frog={:?}, Chemical X={:?}",
            paper_frog.scores,
            chemical_x.scores
        );
        assert_ne!(chemical_x.lane, CandidateLane::Mainline);
    }

    #[test]
    fn shop_chemical_x_stays_mainline_with_x_cost_payoff() {
        let context = shop_context(&[
            CardId::Shockwave,
            CardId::Uppercut,
            CardId::Cleave,
            CardId::Whirlwind,
        ]);

        let chemical_x = shop_relic(context, RelicId::ChemicalX);

        assert_eq!(chemical_x.lane, CandidateLane::Mainline);
    }

    #[test]
    fn shop_rejects_ordinary_unupgraded_transition_card_without_gap() {
        let deck = act2_collector_pressure_deck();

        let clothesline = shop_card(&deck, CardId::Clothesline);
        let iron_wave = shop_card(&deck, CardId::IronWave);

        assert_eq!(
            clothesline.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
        assert_eq!(
            iron_wave.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
    }

    #[test]
    fn shop_keeps_premium_access_card_eligible() {
        let deck = act2_collector_pressure_deck();

        let master_of_strategy = shop_card(&deck, CardId::MasterOfStrategy);

        assert_ne!(
            master_of_strategy.inspect_only_reason(),
            Some("shop card fails acquisition discipline")
        );
    }

    #[test]
    fn shop_rejects_act2_ordinary_cards_that_only_pad_adequate_roles() {
        let deck = act2_collector_pressure_deck();

        let clothesline = shop_card(&deck, CardId::Clothesline);
        let spot_weakness = shop_card(&deck, CardId::SpotWeakness);

        assert_eq!(
            clothesline.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
        assert_eq!(
            spot_weakness.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
    }

    #[test]
    fn shop_low_hp_does_not_turn_ordinary_cards_into_emergency_buys() {
        let cards = act2_collector_pressure_deck();
        let deck = test_deck(&cards);
        let context = shop_context_with_hp(&cards, 24, 90);

        let shrug = shop_card_in_context(context, &deck, CardId::ShrugItOff, 0);
        let clothesline = shop_card_in_context(context, &deck, CardId::Clothesline, 0);

        assert_eq!(
            shrug.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
        assert_eq!(
            clothesline.inspect_only_reason(),
            Some("shop card has no acquisition policy support")
        );
    }

    #[test]
    fn shop_rejects_common_access_card_when_it_spends_purge_reserve() {
        let cards = vec![
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Armaments,
            CardId::Cleave,
            CardId::Inflame,
            CardId::Uppercut,
            CardId::Whirlwind,
        ];
        let deck = test_deck(&cards);
        let context = shop_context_with_gold_and_hp(&cards, 72, 74, 85);

        let shrug = shop_card_in_context_with_price(context, &deck, CardId::ShrugItOff, 0, 51);

        assert_eq!(
            shrug.inspect_only_reason(),
            Some("shop card would spend purge reserve despite hard gap")
        );
    }
}
