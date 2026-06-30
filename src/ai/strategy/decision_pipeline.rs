use crate::ai::analysis::card_semantics::{card_definition_with_upgrades, CardBurden, Mechanic};
use crate::ai::strategy::deck_admission::DeckAdmission;
use crate::ai::strategy::deck_construction_pressure::ConstructionLaneAdjustment;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::reward_admission::{
    RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionPipelineContext {
    pub deck_plan: DeckPlanSnapshot,
    pub gold: Option<i32>,
}

impl DecisionPipelineContext {
    pub fn reward(deck_plan: DeckPlanSnapshot) -> Self {
        Self {
            deck_plan,
            gold: None,
        }
    }

    pub fn shop(deck_plan: DeckPlanSnapshot, gold: i32) -> Self {
        Self {
            deck_plan,
            gold: Some(gold),
        }
    }
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
    pub expansion: ExpansionPlan,
    pub scores: Vec<ScoreComponent>,
}

impl CandidateEvaluation {
    pub fn inspect_only(candidate: DecisionCandidateIr, reason: &'static str) -> Self {
        Self {
            candidate,
            lane: CandidateLane::Reject,
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
    let lane = lane_for_candidate(candidate.kind, scores.iter().map(|score| score.value).sum());
    let expansion = expansion_for_candidate(candidate.kind, lane);
    CandidateEvaluation {
        candidate,
        lane,
        expansion,
        scores,
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
        DecisionCandidateKind::Unsupported => 9,
    }
}

fn filter_passes() -> &'static [FilterPass] {
    &[
        unsupported_candidate_filter,
        missing_card_admission_filter,
        shop_affordability_filter,
        shop_followup_required_filter,
        cleanup_target_filter,
        unmodeled_card_filter,
        thin_support_filter,
        duplicate_marginal_filter,
        unsupported_payoff_filter,
        risky_shop_card_filter,
    ]
}

fn score_passes() -> &'static [ScorePass] {
    &[
        static_candidate_score,
        cleanup_score,
        admission_class_score,
        deck_admission_score,
        construction_pressure_score,
        reward_reason_score,
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
            matches!(
                reason,
                RewardAdmissionReason::DuplicateBurden(_)
                    | RewardAdmissionReason::DuplicateConcern(_)
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
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    admission: Option<&RewardAdmission>,
) -> FilterDecision {
    let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = candidate.kind else {
        return FilterDecision::Pass;
    };
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

fn shop_relic_score(
    _context: DecisionPipelineContext,
    candidate: DecisionCandidateIr,
    _admission: Option<&RewardAdmission>,
    scores: &mut Vec<ScoreComponent>,
) {
    let DecisionCandidateKind::ShopBuyRelic { relic, .. } = candidate.kind else {
        return;
    };
    scores.push(score("shop-relic", shop_relic_score_value(relic)));
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
        DecisionCandidateKind::CardRewardSkip | DecisionCandidateKind::ShopLeave => {
            CandidateLane::Skip
        }
        _ if score >= 110 => CandidateLane::Mainline,
        _ if score >= 45 => CandidateLane::Probe,
        _ => CandidateLane::Reject,
    }
}

fn expansion_for_candidate(kind: DecisionCandidateKind, lane: CandidateLane) -> ExpansionPlan {
    match (kind, lane) {
        (_, CandidateLane::Reject) => ExpansionPlan::InspectOnly("candidate score rejected"),
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

fn shop_relic_score_value(relic: RelicId) -> i32 {
    match relic {
        RelicId::Waffle => 220,
        RelicId::MedicalKit | RelicId::OrangePellets | RelicId::MembershipCard => 150,
        RelicId::ClockworkSouvenir | RelicId::ChemicalX | RelicId::Toolbox => 115,
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
