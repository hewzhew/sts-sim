use std::cmp::Ordering;
use std::collections::BTreeSet;

use serde::Serialize;

use crate::ai::block_plan_profile_v1::{block_plan_profile_v1, BlockPlanProfileV1};
use crate::ai::boss_mechanics_v1::{
    boss_mechanic_pressure_profile_v1, BossEncounterTargetTopologyV1, BossMechanicPressurePointV1,
};
use crate::ai::card_analysis_v1::{
    card_analysis_profile_v1, CardAnalysisAoeSupportV1, CardAnalysisAttackChunkV1,
};
use crate::ai::card_component_signal_v1::{
    evaluate_card_component_signals_v1, is_concrete_package_support_signal_v1,
    is_unresolved_package_payoff_debt_signal_v1, CardComponentSignalContextV1,
    CardComponentSignalKindV1, CardComponentSignalReportV1,
};
use crate::ai::card_semantics_v1::{
    card_access_evidence_v1, card_reward_facts_v1, card_reward_semantic_profile_v1,
    CardAccessEvidenceV1, CardAccessLeverageV1, CardRewardPickDependencyV1,
    CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::deck_shape_v1::{
    deck_shape_candidate_delta_v1, deck_shape_profile_v1,
    persistent_draw_pile_status_assessment_v1, DeckShapeProfileV1, DeckShapeRiskV1,
    PersistentDrawPileStatusAssessmentV1, PersistentDrawPileStatusHandlingV1,
};
use crate::ai::deck_startup_profile_v1::{deck_startup_profile_v1, DeckStartupProfileV1};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, threat_relevant_capability_improvements_v1,
    StrategyCapabilityCoverageV1, StrategyCapabilityKindV1, StrategyPackageIdV2,
    StrategyPlanSupportV1, StrategyThreatSourceV1,
};
use crate::ai::strategy::boss_damage_plan::{
    assess_boss_damage_plan_v1, BossDamagePlanEngineReliabilityV1, BossDamagePlanReadinessV1,
};
use crate::ai::strategy::power_tempo::{mummified_hand_power_tempo_v1, MummifiedHandPowerTempoV1};
use crate::ai::strength_profile_v1::card_unlocks_convertible_strength_payoff_v1;
use crate::content::cards::{get_card_definition, CardId};
use crate::runtime::combat::CombatCard;
use crate::state::rewards::RewardCard;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, run_policy_state_delta_v1,
    DecisionCandidateKey, ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1,
    RunControlSession, RunPolicyCandidateV1, RunPolicyPriorV1, RunPolicyStateDeltaV1,
};

pub const EXACT_CARD_REWARD_POLICY_AUDIT_SCHEMA_NAME: &str = "ExactCardRewardPolicyAudit";
pub const EXACT_CARD_REWARD_POLICY_AUDIT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardPolicyBandV1 {
    ResolvePendingBoundary,
    ImmediateResource,
    CloseThreatGap,
    AmplifyStrategicAccess,
    ImproveRequiredCapability,
    EstablishStrategicAsset,
    PreserveDeckQuality,
    SpeculativeAddition,
    Liability,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CardRewardPolicyAcquisitionV1 {
    Card {
        card: CardId,
        upgrades: u8,
        copies_before: usize,
        semantics: CardRewardSemanticProfileV1,
        access: Option<CardAccessEvidenceV1>,
        component_signals: CardComponentSignalReportV1,
    },
    SingingBowl,
    Skip,
    OpenReward,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardBossDamagePlanImprovementV1 {
    pub before_readiness: BossDamagePlanReadinessV1,
    pub after_readiness: BossDamagePlanReadinessV1,
    pub before_reliability: BossDamagePlanEngineReliabilityV1,
    pub after_reliability: BossDamagePlanEngineReliabilityV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardPolicyActionEvidenceV1 {
    pub candidate_id: String,
    pub candidate_key: DecisionCandidateKey,
    pub acquisition: CardRewardPolicyAcquisitionV1,
    pub band: CardRewardPolicyBandV1,
    pub delta: RunPolicyStateDeltaV1,
    pub introduces_unsupported_mechanics: bool,
    pub introduces_undigested_status_burden: bool,
    pub duplicate_low_marginal: bool,
    /// A costly tactical card only adds one-turn debuff coverage already
    /// present in the owned deck.
    pub expensive_short_debuff_overlap: bool,
    pub access_conflict_or_redundancy: bool,
    pub mummified_hand_power_tempo: Option<MummifiedHandPowerTempoV1>,
    /// The known act boss punishes playing powers and this candidate is a
    /// shared-analysis minor power without an exact strategic improvement.
    pub boss_power_tax_conflict: bool,
    pub random_target_frontload_reliable: bool,
    /// Exact deck-shape liabilities introduced by this candidate.
    ///
    /// This consumes the shared deck-shape model instead of allowing the
    /// reward owner to rediscover card-specific playability and saturation
    /// rules through coarse threat thresholds.
    pub added_deck_shape_risks: Vec<DeckShapeRiskV1>,
    /// Candidate-local assessment of persistent statuses shuffled directly
    /// into the draw pile and the owned deck's typed ways to absorb them.
    pub persistent_draw_pile_status: Option<PersistentDrawPileStatusAssessmentV1>,
    pub improves_threat_relevant_capability: bool,
    pub amplifies_existing_answers: bool,
    /// A shared boss-damage-plan improvement introduced by this exact card.
    /// This is a same-band tie-break, not a card-specific priority override.
    pub boss_damage_plan_improvement: Option<CardRewardBossDamagePlanImprovementV1>,
    /// Support for the route resource required by an upgrade-investment card.
    ///
    /// `None` means the candidate has no such dependency.  Keeping this typed
    /// and visible prevents an immediate-damage delta from silently erasing an
    /// unfunded long-term commitment.
    pub upgrade_investment_support: Option<StrategyPlanSupportV1>,
    surface_index: usize,
}

#[derive(Clone, Debug)]
pub struct ExactCardRewardPolicyDecisionV1 {
    pub exact: ExactRunPolicyDecisionV1,
    pub evidence: Vec<CardRewardPolicyActionEvidenceV1>,
    pub prior: RunPolicyPriorV1,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardPolicyAuditCandidateV1 {
    pub owner_rank: usize,
    pub candidate_id: String,
    pub label: String,
    pub candidate_key: DecisionCandidateKey,
    pub acquisition: CardRewardPolicyAcquisitionV1,
    pub band: CardRewardPolicyBandV1,
    pub closed_threat_gaps: Vec<super::RunPolicyThreatGapKeyV1>,
    pub capability_improvements: Vec<super::RunPolicyCapabilityChangeV1>,
    pub resolved_formation_needs:
        Vec<crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1>,
    pub added_formation_strengths: Vec<StrategyPackageIdV2>,
    pub introduces_unsupported_mechanics: bool,
    pub introduces_undigested_status_burden: bool,
    pub duplicate_low_marginal: bool,
    pub expensive_short_debuff_overlap: bool,
    pub access_conflict_or_redundancy: bool,
    pub mummified_hand_power_tempo: Option<MummifiedHandPowerTempoV1>,
    pub boss_power_tax_conflict: bool,
    pub random_target_frontload_reliable: bool,
    pub added_deck_shape_risks: Vec<DeckShapeRiskV1>,
    pub persistent_draw_pile_status: Option<PersistentDrawPileStatusAssessmentV1>,
    pub improves_threat_relevant_capability: bool,
    pub amplifies_existing_answers: bool,
    pub boss_damage_plan_improvement: Option<CardRewardBossDamagePlanImprovementV1>,
    pub upgrade_investment_support: Option<StrategyPlanSupportV1>,
    pub surface_index: usize,
    pub prior_probability: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactCardRewardPolicyAuditV1 {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub candidates: Vec<CardRewardPolicyAuditCandidateV1>,
}

impl ExactCardRewardPolicyDecisionV1 {
    pub fn audit(
        &self,
        legal: &[RunPolicyCandidateV1<'_>],
    ) -> Result<ExactCardRewardPolicyAuditV1, String> {
        let candidates = self
            .evidence
            .iter()
            .enumerate()
            .map(|(owner_rank, evidence)| {
                let legal_candidate = legal
                    .iter()
                    .find(|candidate| candidate.candidate_id == evidence.candidate_id)
                    .ok_or_else(|| {
                        format!(
                            "card reward policy audit could not find legal candidate '{}'",
                            evidence.candidate_id
                        )
                    })?;
                let prior_probability = self
                    .prior
                    .entries
                    .iter()
                    .find(|entry| entry.candidate_id == evidence.candidate_id)
                    .map(|entry| entry.probability)
                    .ok_or_else(|| {
                        format!(
                            "card reward policy audit could not find prior for candidate '{}'",
                            evidence.candidate_id
                        )
                    })?;
                Ok(CardRewardPolicyAuditCandidateV1 {
                    owner_rank,
                    candidate_id: evidence.candidate_id.clone(),
                    label: legal_candidate.label.to_string(),
                    candidate_key: evidence.candidate_key.clone(),
                    acquisition: evidence.acquisition.clone(),
                    band: evidence.band,
                    closed_threat_gaps: evidence.delta.closed_threat_gaps.clone(),
                    capability_improvements: evidence.delta.capability_improvements.clone(),
                    resolved_formation_needs: evidence.delta.resolved_formation_needs.clone(),
                    added_formation_strengths: evidence.delta.added_formation_strengths.clone(),
                    introduces_unsupported_mechanics: evidence.introduces_unsupported_mechanics,
                    introduces_undigested_status_burden: evidence
                        .introduces_undigested_status_burden,
                    duplicate_low_marginal: evidence.duplicate_low_marginal,
                    expensive_short_debuff_overlap: evidence.expensive_short_debuff_overlap,
                    access_conflict_or_redundancy: evidence.access_conflict_or_redundancy,
                    mummified_hand_power_tempo: evidence.mummified_hand_power_tempo,
                    boss_power_tax_conflict: evidence.boss_power_tax_conflict,
                    random_target_frontload_reliable: evidence.random_target_frontload_reliable,
                    added_deck_shape_risks: evidence.added_deck_shape_risks.clone(),
                    persistent_draw_pile_status: evidence.persistent_draw_pile_status.clone(),
                    improves_threat_relevant_capability: evidence
                        .improves_threat_relevant_capability,
                    amplifies_existing_answers: evidence.amplifies_existing_answers,
                    boss_damage_plan_improvement: evidence.boss_damage_plan_improvement,
                    upgrade_investment_support: evidence.upgrade_investment_support,
                    surface_index: evidence.surface_index,
                    prior_probability,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(ExactCardRewardPolicyAuditV1 {
            schema_name: EXACT_CARD_REWARD_POLICY_AUDIT_SCHEMA_NAME,
            schema_version: EXACT_CARD_REWARD_POLICY_AUDIT_SCHEMA_VERSION,
            current_hp: self.exact.before.resources.current_hp,
            max_hp: self.exact.before.resources.max_hp,
            gold: self.exact.before.resources.gold,
            candidates,
        })
    }
}

pub fn exact_card_reward_policy_audit_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactCardRewardPolicyAuditV1, String> {
    exact_card_reward_policy_decision_v1(session, legal)?.audit(legal)
}

pub fn exact_card_reward_policy_prior_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<RunPolicyPriorV1, String> {
    Ok(exact_card_reward_policy_decision_v1(session, legal)?.prior)
}

pub fn exact_card_reward_policy_decision_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactCardRewardPolicyDecisionV1, String> {
    let exact = exact_run_policy_decision_v1(session)?;
    validate_same_candidate_surface(&exact, legal)?;
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let upgrade_sink_support = strategy.support(StrategyPackageIdV2::UpgradeSink);
    let formation_needs = strategy.formation_summary().needs;
    let startup = deck_startup_profile_v1(&session.run_state);
    let deck_shape = deck_shape_profile_v1(&session.run_state);
    let block_plan = block_plan_profile_v1(&session.run_state);
    let boss_profile = session
        .run_state
        .boss_key
        .map(|boss| boss_mechanic_pressure_profile_v1(&session.run_state, boss));
    let boss_power_tax_active = boss_profile
        .as_ref()
        .is_some_and(|profile| profile.has_pressure(BossMechanicPressurePointV1::PowerPlayPenalty));
    let boss_target_topology = boss_profile
        .as_ref()
        .map(|profile| profile.target_topology)
        .unwrap_or(BossEncounterTargetTopologyV1::Unknown);
    let mut evidence = exact
        .actions
        .iter()
        .filter(|action| {
            action
                .candidate_key
                .as_ref()
                .is_some_and(is_card_reward_key)
        })
        .enumerate()
        .map(|(surface_index, action)| {
            card_reward_action_evidence_v1(
                session,
                &exact,
                action,
                surface_index,
                upgrade_sink_support,
                &formation_needs,
                &startup,
                &deck_shape,
                &block_plan,
                boss_power_tax_active,
                boss_target_topology,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    evidence.sort_by(compare_card_reward_evidence);
    let ranked_card_ids = evidence
        .iter()
        .map(|candidate| candidate.candidate_id.clone())
        .collect::<BTreeSet<_>>();
    let prior = positive_ranked_run_policy_prior_v1(
        legal,
        evidence
            .iter()
            .map(|candidate| candidate.candidate_id.clone())
            .chain(
                legal
                    .iter()
                    .filter(|candidate| !ranked_card_ids.contains(candidate.candidate_id))
                    .map(|candidate| candidate.candidate_id.to_string()),
            ),
    )?;

    Ok(ExactCardRewardPolicyDecisionV1 {
        exact,
        evidence,
        prior,
    })
}

fn validate_same_candidate_surface(
    exact: &ExactRunPolicyDecisionV1,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<(), String> {
    let exact_ids = exact
        .actions
        .iter()
        .filter(|action| {
            action
                .candidate_key
                .as_ref()
                .is_some_and(is_card_reward_key)
        })
        .map(|action| action.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let legal_ids = legal
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    if exact_ids.is_empty() || !exact_ids.iter().all(|id| legal_ids.contains(id)) {
        return Err(format!(
            "card reward policy surface omits exact typed actions: exact={} policy={}",
            exact_ids.len(),
            legal.len()
        ));
    }
    Ok(())
}

fn is_card_reward_key(key: &DecisionCandidateKey) -> bool {
    matches!(
        key,
        DecisionCandidateKey::CardRewardPick { .. }
            | DecisionCandidateKey::CardRewardOpen { .. }
            | DecisionCandidateKey::CardRewardSingingBowl { .. }
            | DecisionCandidateKey::CardRewardSkip { .. }
    )
}

fn card_reward_action_evidence_v1(
    parent: &RunControlSession,
    decision: &ExactRunPolicyDecisionV1,
    action: &ExactRunPolicyActionSuccessorV1,
    surface_index: usize,
    upgrade_sink_support: StrategyPlanSupportV1,
    formation_needs: &[crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1],
    startup: &DeckStartupProfileV1,
    deck_shape: &DeckShapeProfileV1,
    block_plan: &BlockPlanProfileV1,
    boss_power_tax_active: bool,
    boss_target_topology: BossEncounterTargetTopologyV1,
) -> Result<CardRewardPolicyActionEvidenceV1, String> {
    let candidate_key = action.candidate_key.clone().ok_or_else(|| {
        format!(
            "card reward candidate '{}' has no typed key",
            action.candidate_id
        )
    })?;
    let acquisition = acquisition_v1(parent, &candidate_key, formation_needs, startup, block_plan)?;
    let delta = run_policy_state_delta_v1(&decision.before, &action.after);
    let introduces_unsupported_mechanics = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card { semantics, .. }
            if !semantics.unsupported_mechanics.is_empty()
                || semantics.roles.contains(&CardRewardSemanticRoleV1::UnsupportedMechanics)
    );
    let introduces_undigested_status_burden = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card { semantics, .. }
            if semantics.roles.contains(&CardRewardSemanticRoleV1::StatusGenerator)
                && action.after.deck.status_payoffs == 0
                && parent.run_state.relics.iter().any(|relic| {
                    relic.id == crate::content::relics::RelicId::RunicPyramid
                })
    );
    let duplicate_low_marginal = duplicate_low_marginal_v1(&acquisition, &delta);
    let expensive_short_debuff_overlap = expensive_short_debuff_overlap_v1(parent, &acquisition);
    let mummified_hand_power_tempo = match &acquisition {
        CardRewardPolicyAcquisitionV1::Card { card, upgrades, .. } => {
            mummified_hand_power_tempo_v1(&parent.run_state, *card, *upgrades)
        }
        _ => None,
    };
    let access_conflict_or_redundancy = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card {
            semantics,
            access,
            component_signals,
            ..
        }
            if !semantics.roles.is_empty()
                && semantics.roles.iter().all(|role| is_access_role(*role))
                && (component_signals
                    .debt_signals
                    .contains(&CardComponentSignalKindV1::DuplicateNoDrawAccessDebt)
                    || (access.is_none()
                        && decision.before.deck.draw_sources
                            .saturating_add(decision.before.deck.energy_sources)
                            >= 3
                        && delta.capability_improvements.is_empty()
                        && delta.resolved_formation_needs.is_empty())
                    || (access.is_some_and(|access| {
                        access.leverage == CardAccessLeverageV1::Incremental
                    })
                        && decision.before.deck.draw_sources
                            .saturating_add(decision.before.deck.energy_sources)
                            >= 3
                        && delta.capability_improvements.is_empty()
                        && delta.resolved_formation_needs.is_empty()))
    );
    let added_deck_shape_risks = match &acquisition {
        CardRewardPolicyAcquisitionV1::Card { card, .. } => {
            deck_shape_candidate_delta_v1(deck_shape, *card).risks
        }
        CardRewardPolicyAcquisitionV1::SingingBowl
        | CardRewardPolicyAcquisitionV1::Skip
        | CardRewardPolicyAcquisitionV1::OpenReward => Vec::new(),
    };
    let persistent_draw_pile_status = match &acquisition {
        CardRewardPolicyAcquisitionV1::Card { card, upgrades, .. } => {
            let facts = card_reward_facts_v1(&RewardCard::new(*card, *upgrades));
            persistent_draw_pile_status_assessment_v1(&parent.run_state, &facts.status_injections)
        }
        CardRewardPolicyAcquisitionV1::SingingBowl
        | CardRewardPolicyAcquisitionV1::Skip
        | CardRewardPolicyAcquisitionV1::OpenReward => None,
    };
    let improves_threat_relevant_capability = !threat_relevant_capability_improvements_v1(
        &decision.before.threats,
        &decision.before.threat_coverage,
        &action.after.threat_coverage,
    )
    .is_empty();
    let boss_power_tax_conflict =
        boss_power_tax_conflict_v1(boss_power_tax_active, &acquisition, &delta);
    let random_target_frontload_reliable =
        random_target_frontload_reliable_v1(boss_target_topology, &acquisition);
    let upgrade_investment_support = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card { semantics, .. }
            if semantics
                .dependencies
                .contains(&CardRewardPickDependencyV1::RouteUpgradeDensity)
    )
    .then_some(upgrade_sink_support);
    let amplifies_existing_answers =
        access_amplifies_existing_answers(&decision.before, &acquisition);
    let boss_damage_plan_improvement = boss_damage_plan_improvement_v1(parent, &acquisition);
    let base_band = card_reward_band_v1(
        &acquisition,
        &delta,
        introduces_unsupported_mechanics,
        introduces_undigested_status_burden,
        duplicate_low_marginal,
        expensive_short_debuff_overlap,
        access_conflict_or_redundancy,
        boss_power_tax_conflict,
        random_target_frontload_reliable,
        !added_deck_shape_risks.is_empty(),
        improves_threat_relevant_capability,
        amplifies_existing_answers,
    );
    let band = apply_upgrade_investment_gate_v1(
        apply_persistent_draw_pile_status_gate_v1(base_band, persistent_draw_pile_status.as_ref()),
        upgrade_investment_support,
    );

    Ok(CardRewardPolicyActionEvidenceV1 {
        candidate_id: action.candidate_id.clone(),
        candidate_key,
        acquisition,
        band,
        delta,
        introduces_unsupported_mechanics,
        introduces_undigested_status_burden,
        duplicate_low_marginal,
        expensive_short_debuff_overlap,
        access_conflict_or_redundancy,
        mummified_hand_power_tempo,
        boss_power_tax_conflict,
        random_target_frontload_reliable,
        added_deck_shape_risks,
        persistent_draw_pile_status,
        improves_threat_relevant_capability,
        amplifies_existing_answers,
        boss_damage_plan_improvement,
        upgrade_investment_support,
        surface_index,
    })
}

fn duplicate_low_marginal_v1(
    acquisition: &CardRewardPolicyAcquisitionV1,
    delta: &RunPolicyStateDeltaV1,
) -> bool {
    let CardRewardPolicyAcquisitionV1::Card {
        card,
        upgrades,
        copies_before,
        semantics,
        ..
    } = acquisition
    else {
        return false;
    };
    if *copies_before == 0 {
        return false;
    }

    let no_new_strategic_shape = delta.closed_threat_gaps.is_empty()
        && delta.resolved_formation_needs.is_empty()
        && delta.added_formation_strengths.is_empty();
    if no_new_strategic_shape && delta.capability_improvements.is_empty() {
        return true;
    }

    let tactical_coverage_only = !semantics.roles.is_empty()
        && semantics.roles.iter().all(|role| {
            matches!(
                role,
                CardRewardSemanticRoleV1::FrontloadDamage
                    | CardRewardSemanticRoleV1::AoeDamage
                    | CardRewardSemanticRoleV1::Vulnerable
                    | CardRewardSemanticRoleV1::Weak
            )
        });
    let only_reinforces_supported_capabilities = !delta.capability_improvements.is_empty()
        && delta.capability_improvements.iter().all(|change| {
            matches!(
                change.before,
                StrategyCapabilityCoverageV1::Supported | StrategyCapabilityCoverageV1::Strong
            ) && change.after == StrategyCapabilityCoverageV1::Strong
        });

    let has_aoe_role = semantics
        .roles
        .contains(&CardRewardSemanticRoleV1::AoeDamage);
    let expensive_tactical_duplicate = get_card_definition(*card).cost >= 2 && !has_aoe_role;
    let light_aoe_duplicate = has_aoe_role
        && card_analysis_profile_v1(*card, *upgrades).aoe_support
            == CardAnalysisAoeSupportV1::Present;

    (expensive_tactical_duplicate || light_aoe_duplicate)
        && tactical_coverage_only
        && no_new_strategic_shape
        && only_reinforces_supported_capabilities
}

fn expensive_short_debuff_overlap_v1(
    parent: &RunControlSession,
    acquisition: &CardRewardPolicyAcquisitionV1,
) -> bool {
    let CardRewardPolicyAcquisitionV1::Card {
        card,
        upgrades,
        semantics,
        ..
    } = acquisition
    else {
        return false;
    };
    let candidate = card_reward_facts_v1(&RewardCard::new(*card, *upgrades));
    let tactical_debuff_only = !semantics.roles.is_empty()
        && semantics.roles.iter().all(|role| {
            matches!(
                role,
                CardRewardSemanticRoleV1::FrontloadDamage
                    | CardRewardSemanticRoleV1::Vulnerable
                    | CardRewardSemanticRoleV1::Weak
            )
        });
    if candidate.cost < 2 || !tactical_debuff_only {
        return false;
    }

    parent.run_state.master_deck.iter().any(|owned| {
        let owned = card_reward_facts_v1(&RewardCard::new(owned.id, owned.upgrades));
        (candidate.vulnerable == 1 && owned.vulnerable > 0)
            || (candidate.weak == 1 && owned.weak > 0)
    })
}

fn boss_damage_plan_improvement_v1(
    parent: &RunControlSession,
    acquisition: &CardRewardPolicyAcquisitionV1,
) -> Option<CardRewardBossDamagePlanImprovementV1> {
    let CardRewardPolicyAcquisitionV1::Card { card, upgrades, .. } = acquisition else {
        return None;
    };

    let before = assess_boss_damage_plan_v1(&parent.run_state.master_deck);
    let mut after_deck = parent.run_state.master_deck.clone();
    let mut added = CombatCard::new(*card, u32::MAX);
    added.upgrades = *upgrades;
    after_deck.push(added);
    let after = assess_boss_damage_plan_v1(&after_deck);

    ((after.readiness, after.engine_reliability) > (before.readiness, before.engine_reliability))
        .then_some(CardRewardBossDamagePlanImprovementV1 {
            before_readiness: before.readiness,
            after_readiness: after.readiness,
            before_reliability: before.engine_reliability,
            after_reliability: after.engine_reliability,
        })
}

fn apply_upgrade_investment_gate_v1(
    base_band: CardRewardPolicyBandV1,
    support: Option<StrategyPlanSupportV1>,
) -> CardRewardPolicyBandV1 {
    match support {
        None | Some(StrategyPlanSupportV1::Strong) => base_band,
        Some(StrategyPlanSupportV1::Plausible) => {
            base_band.max(CardRewardPolicyBandV1::SpeculativeAddition)
        }
        Some(StrategyPlanSupportV1::Weak | StrategyPlanSupportV1::Blocked) => {
            CardRewardPolicyBandV1::Liability
        }
    }
}

fn apply_persistent_draw_pile_status_gate_v1(
    base_band: CardRewardPolicyBandV1,
    assessment: Option<&PersistentDrawPileStatusAssessmentV1>,
) -> CardRewardPolicyBandV1 {
    match assessment.map(|assessment| assessment.handling) {
        None | Some(PersistentDrawPileStatusHandlingV1::Covered) => base_band,
        Some(
            PersistentDrawPileStatusHandlingV1::Unsupported
            | PersistentDrawPileStatusHandlingV1::Conditional,
        ) => base_band.max(CardRewardPolicyBandV1::SpeculativeAddition),
    }
}

fn acquisition_v1(
    parent: &RunControlSession,
    key: &DecisionCandidateKey,
    formation_needs: &[crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1],
    startup: &DeckStartupProfileV1,
    block_plan: &BlockPlanProfileV1,
) -> Result<CardRewardPolicyAcquisitionV1, String> {
    Ok(match key {
        DecisionCandidateKey::CardRewardPick { card, upgrades, .. } => {
            let reward = RewardCard::new(*card, *upgrades);
            let semantics = card_reward_semantic_profile_v1(&reward);
            let copies_before = parent
                .run_state
                .master_deck
                .iter()
                .filter(|owned| owned.id == *card)
                .count();
            CardRewardPolicyAcquisitionV1::Card {
                card: *card,
                upgrades: *upgrades,
                copies_before,
                access: card_access_evidence_v1(&reward),
                component_signals: evaluate_card_component_signals_v1(
                    &CardComponentSignalContextV1 {
                        same_card_count: copies_before,
                        formation_needs: formation_needs.to_vec(),
                        startup: startup.clone(),
                        block_plan: block_plan.clone(),
                        candidate_unlocks_convertible_strength_payoff:
                            card_unlocks_convertible_strength_payoff_v1(
                                &parent.run_state,
                                *card,
                                *upgrades,
                            ),
                    },
                    &semantics,
                ),
                semantics,
            }
        }
        DecisionCandidateKey::CardRewardSingingBowl { .. } => {
            CardRewardPolicyAcquisitionV1::SingingBowl
        }
        DecisionCandidateKey::CardRewardSkip { .. } => CardRewardPolicyAcquisitionV1::Skip,
        DecisionCandidateKey::CardRewardOpen { .. } => CardRewardPolicyAcquisitionV1::OpenReward,
        other => {
            return Err(format!(
                "exact card reward policy received non-card-reward candidate key {other:?}"
            ))
        }
    })
}

fn card_reward_band_v1(
    acquisition: &CardRewardPolicyAcquisitionV1,
    delta: &RunPolicyStateDeltaV1,
    unsupported: bool,
    status_burden: bool,
    duplicate_low_marginal: bool,
    expensive_short_debuff_overlap: bool,
    access_conflict_or_redundancy: bool,
    boss_power_tax_conflict: bool,
    random_target_frontload_reliable: bool,
    introduces_deck_shape_risk: bool,
    improves_threat_relevant_capability: bool,
    amplifies_existing_answers: bool,
) -> CardRewardPolicyBandV1 {
    match acquisition {
        CardRewardPolicyAcquisitionV1::OpenReward => CardRewardPolicyBandV1::ResolvePendingBoundary,
        CardRewardPolicyAcquisitionV1::SingingBowl
            if delta.max_hp_gain > 0 || delta.hp_gain > 0 =>
        {
            CardRewardPolicyBandV1::ImmediateResource
        }
        CardRewardPolicyAcquisitionV1::SingingBowl => CardRewardPolicyBandV1::PreserveDeckQuality,
        CardRewardPolicyAcquisitionV1::Skip => CardRewardPolicyBandV1::PreserveDeckQuality,
        CardRewardPolicyAcquisitionV1::Card {
            card,
            upgrades,
            semantics,
            component_signals,
            ..
        } => {
            let has_concrete_package_support = component_signals
                .positive_signals
                .iter()
                .any(|signal| is_concrete_package_support_signal_v1(*signal));
            let has_intrinsic_asset_role = semantics
                .roles
                .iter()
                .any(|role| is_self_sufficient_strategic_role(*role, *card, *upgrades))
                || random_target_frontload_reliable;
            let has_blocking_package_debt = component_signals
                .debt_signals
                .iter()
                .any(|signal| is_unresolved_package_payoff_debt_signal_v1(*signal))
                && !has_concrete_package_support
                && !has_intrinsic_asset_role;
            if status_burden
                || (unsupported && !has_concrete_package_support)
                || duplicate_low_marginal
                || access_conflict_or_redundancy
                || introduces_deck_shape_risk
                || has_blocking_package_debt
            {
                CardRewardPolicyBandV1::Liability
            } else if boss_power_tax_conflict {
                CardRewardPolicyBandV1::SpeculativeAddition
            } else if expensive_short_debuff_overlap {
                CardRewardPolicyBandV1::SpeculativeAddition
            } else if !delta.closed_threat_gaps.is_empty() {
                CardRewardPolicyBandV1::CloseThreatGap
            } else if amplifies_existing_answers {
                CardRewardPolicyBandV1::AmplifyStrategicAccess
            } else if improves_threat_relevant_capability {
                CardRewardPolicyBandV1::ImproveRequiredCapability
            } else if !delta.resolved_formation_needs.is_empty()
                || !delta.added_formation_strengths.is_empty()
                || has_concrete_package_support
                || has_intrinsic_asset_role
            {
                CardRewardPolicyBandV1::EstablishStrategicAsset
            } else {
                CardRewardPolicyBandV1::SpeculativeAddition
            }
        }
    }
}

fn boss_power_tax_conflict_v1(
    boss_power_tax_active: bool,
    acquisition: &CardRewardPolicyAcquisitionV1,
    delta: &RunPolicyStateDeltaV1,
) -> bool {
    let CardRewardPolicyAcquisitionV1::Card { card, upgrades, .. } = acquisition else {
        return false;
    };

    boss_power_tax_active
        && card_analysis_profile_v1(*card, *upgrades).is_boss_minor_power
        && delta.closed_threat_gaps.is_empty()
        && delta.capability_improvements.is_empty()
        && delta.resolved_formation_needs.is_empty()
        && delta.added_formation_strengths.is_empty()
}

fn random_target_frontload_reliable_v1(
    boss_target_topology: BossEncounterTargetTopologyV1,
    acquisition: &CardRewardPolicyAcquisitionV1,
) -> bool {
    let CardRewardPolicyAcquisitionV1::Card {
        card,
        upgrades,
        semantics,
        ..
    } = acquisition
    else {
        return false;
    };

    boss_target_topology == BossEncounterTargetTopologyV1::SingleOpponent
        && semantics
            .roles
            .contains(&CardRewardSemanticRoleV1::RandomOutput)
        && semantics
            .roles
            .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
        && card_analysis_profile_v1(*card, *upgrades).attack_chunk
            != CardAnalysisAttackChunkV1::None
}

fn access_amplifies_existing_answers(
    before: &super::RunPolicyStateEvidenceV1,
    acquisition: &CardRewardPolicyAcquisitionV1,
) -> bool {
    let CardRewardPolicyAcquisitionV1::Card {
        copies_before,
        access:
            Some(CardAccessEvidenceV1 {
                leverage: CardAccessLeverageV1::EfficientBurst,
                ..
            }),
        component_signals,
        ..
    } = acquisition
    else {
        return false;
    };
    if *copies_before > 0
        || !component_signals
            .positive_signals
            .contains(&CardComponentSignalKindV1::DrawEnergyAccess)
        || !component_signals.debt_signals.is_empty()
    {
        return false;
    }

    !before.formation.strengths.is_empty()
        || before
            .threat_coverage
            .capabilities
            .iter()
            .any(|capability| {
                capability.capability != StrategyCapabilityKindV1::DrawEnergyConsistency
                    && matches!(
                        capability.coverage,
                        StrategyCapabilityCoverageV1::Supported
                            | StrategyCapabilityCoverageV1::Strong
                    )
            })
}

fn is_access_role(role: CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::CardDraw
            | CardRewardSemanticRoleV1::CycleAccess
            | CardRewardSemanticRoleV1::DiscardPileTopdeckAccess
            | CardRewardSemanticRoleV1::HandTopdeckSelection
            | CardRewardSemanticRoleV1::EnergySource
    )
}

/// Roles whose strategic value is intrinsic enough to justify adding a card
/// without a current threat, formation, or package-support delta.
///
/// Plain block is deliberately absent. Block is an amount and timing claim,
/// not an asset by itself: it must fill a current need or carry another
/// self-sufficient role before it can outrank preserving deck quality. Light
/// AoE follows the same rule; only a shared-analysis `Strong` AoE profile is
/// intrinsically valuable without an exact capability or threat delta.
fn is_self_sufficient_strategic_role(
    role: CardRewardSemanticRoleV1,
    card: CardId,
    upgrades: u8,
) -> bool {
    if role == CardRewardSemanticRoleV1::AoeDamage {
        return card_analysis_profile_v1(card, upgrades).aoe_support
            == CardAnalysisAoeSupportV1::Strong;
    }

    matches!(
        role,
        CardRewardSemanticRoleV1::CardDraw
            | CardRewardSemanticRoleV1::CycleAccess
            | CardRewardSemanticRoleV1::DiscardPileTopdeckAccess
            | CardRewardSemanticRoleV1::HandTopdeckSelection
            | CardRewardSemanticRoleV1::EnergySource
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::ScalingSource
            | CardRewardSemanticRoleV1::CombatExternalPayoff
            | CardRewardSemanticRoleV1::CombatSustain
            | CardRewardSemanticRoleV1::ExhaustGenerator
            | CardRewardSemanticRoleV1::ExhaustReuse
    )
}

fn compare_card_reward_evidence(
    left: &CardRewardPolicyActionEvidenceV1,
    right: &CardRewardPolicyActionEvidenceV1,
) -> Ordering {
    left.band
        .cmp(&right.band)
        .then_with(|| {
            compare_gap_source_counts(
                &left.delta.closed_threat_gaps,
                &right.delta.closed_threat_gaps,
            )
        })
        .then_with(|| {
            right
                .boss_damage_plan_improvement
                .is_some()
                .cmp(&left.boss_damage_plan_improvement.is_some())
        })
        .then_with(|| {
            right
                .delta
                .capability_improvements
                .len()
                .cmp(&left.delta.capability_improvements.len())
        })
        .then_with(|| {
            right
                .delta
                .resolved_formation_needs
                .len()
                .cmp(&left.delta.resolved_formation_needs.len())
        })
        .then_with(|| {
            right
                .delta
                .added_formation_strengths
                .len()
                .cmp(&left.delta.added_formation_strengths.len())
        })
        .then_with(|| {
            right
                .random_target_frontload_reliable
                .cmp(&left.random_target_frontload_reliable)
        })
        .then_with(|| right.delta.max_hp_gain.cmp(&left.delta.max_hp_gain))
        .then_with(|| left.surface_index.cmp(&right.surface_index))
}

fn compare_gap_source_counts(
    left: &[super::RunPolicyThreatGapKeyV1],
    right: &[super::RunPolicyThreatGapKeyV1],
) -> Ordering {
    for source in [
        StrategyThreatSourceV1::ActBoss,
        StrategyThreatSourceV1::ActEliteEncounter,
        StrategyThreatSourceV1::ActElitePool,
        StrategyThreatSourceV1::ActHallwayPool,
    ] {
        let left_count = left.iter().filter(|gap| gap.source == source).count();
        let right_count = right.iter().filter(|gap| gap.source == source).count();
        match right_count.cmp(&left_count) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::eval::run_control::{build_decision_surface, RunControlConfig};
    use crate::state::core::EngineState;
    use crate::state::rewards::{RewardItem, RewardState};

    fn reward_session(cards: &[(CardId, u8)]) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let reward_cards = cards
            .iter()
            .map(|(card, upgrades)| RewardCard::new(*card, *upgrades))
            .collect::<Vec<_>>();
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: reward_cards.clone(),
        }];
        reward.pending_card_choice = Some(reward_cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);
        session
    }

    fn owned_deck(cards: &[(CardId, u8)]) -> Vec<CombatCard> {
        cards
            .iter()
            .enumerate()
            .map(|(index, (card, upgrades))| {
                let mut owned = CombatCard::new(*card, index as u32);
                owned.upgrades = *upgrades;
                owned
            })
            .collect()
    }

    fn a1f10_mummified_hand_session(cards: &[(CardId, u8)]) -> RunControlSession {
        let mut session = reward_session(cards);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 10;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session.run_state.current_hp = 45;
        session.run_state.max_hp = 85;
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::MummifiedHand));
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::PowerThrough, 0),
            (CardId::SecondWind, 0),
            (CardId::ThunderClap, 0),
            (CardId::Anger, 0),
            (CardId::ShrugItOff, 0),
            (CardId::DarkEmbrace, 0),
        ]);
        session
    }

    fn a1f11_mummified_hand_session(cards: &[(CardId, u8)]) -> RunControlSession {
        let mut session = a1f10_mummified_hand_session(cards);
        session.run_state.floor_num = 11;
        session.run_state.current_hp = 37;
        session.run_state.gold = 159;
        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::RecklessCharge, 10_001));
        session
    }

    fn a2f20_champ_without_damage_engine_session(cards: &[(CardId, u8)]) -> RunControlSession {
        let mut session = reward_session(cards);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.current_hp = 60;
        session.run_state.max_hp = 85;
        session.run_state.gold = 105;
        session.run_state.relics = vec![
            RelicState::new(RelicId::BurningBlood),
            RelicState::new(RelicId::OrnamentalFan),
            RelicState::new(RelicId::MummifiedHand),
            RelicState::new(RelicId::PenNib),
            RelicState::new(RelicId::RunicCube),
            RelicState::new(RelicId::OrangePellets),
        ];
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::PowerThrough, 0),
            (CardId::SecondWind, 0),
            (CardId::ThunderClap, 0),
            (CardId::Anger, 0),
            (CardId::ShrugItOff, 0),
            (CardId::DarkEmbrace, 0),
            (CardId::RecklessCharge, 0),
            (CardId::DarkEmbrace, 0),
            (CardId::FiendFire, 0),
            (CardId::Cleave, 0),
        ]);
        session
    }

    fn policy_candidates<'a>(
        surface: &'a super::super::DecisionSurface,
    ) -> Vec<RunPolicyCandidateV1<'a>> {
        surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
                if !candidate.key.as_ref().is_some_and(is_card_reward_key) {
                    return None;
                }
                candidate
                    .action
                    .executable_action_ref()
                    .map(|action| RunPolicyCandidateV1 {
                        candidate_id: &candidate.id,
                        label: &candidate.label,
                        action,
                    })
            })
            .collect()
    }

    fn decision(session: &RunControlSession) -> ExactCardRewardPolicyDecisionV1 {
        let surface = build_decision_surface(session);
        let legal = policy_candidates(&surface);
        exact_card_reward_policy_decision_v1(session, &legal).expect("exact card reward policy")
    }

    fn position(
        decision: &ExactCardRewardPolicyDecisionV1,
        predicate: impl Fn(&DecisionCandidateKey) -> bool,
    ) -> usize {
        decision
            .evidence
            .iter()
            .position(|candidate| predicate(&candidate.candidate_key))
            .expect("candidate position")
    }

    fn card_evidence(
        decision: &ExactCardRewardPolicyDecisionV1,
        card: CardId,
    ) -> &CardRewardPolicyActionEvidenceV1 {
        decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::CardRewardPick {
                        card: candidate_card,
                        ..
                    } if candidate_card == card
                )
            })
            .expect("card evidence")
    }

    #[test]
    fn second_expensive_tactical_coverage_loses_to_preserving_deck_quality() {
        // Preserve the exact A1F10 public deck shape: the first Uppercut has
        // already supplied its tactical roles before the duplicate is offered.
        let mut session = reward_session(&[(CardId::Uppercut, 0)]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 10;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session.run_state.current_hp = 53;
        session.run_state.max_hp = 85;
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::MummifiedHand));
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::PowerThrough, 0),
            (CardId::Uppercut, 0),
            (CardId::SecondWind, 0),
            (CardId::Cleave, 0),
            (CardId::ShrugItOff, 0),
            (CardId::DarkEmbrace, 0),
        ]);

        let decision = decision(&session);
        let uppercut = card_evidence(&decision, CardId::Uppercut);
        assert!(uppercut.duplicate_low_marginal, "uppercut={uppercut:#?}");
        assert_eq!(uppercut.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Uppercut,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn expensive_short_debuff_overlap_loses_to_preserving_deck_quality() {
        let session = a1f10_mummified_hand_session(&[
            (CardId::Flex, 0),
            (CardId::Uppercut, 0),
            (CardId::RecklessCharge, 0),
        ]);

        let decision = decision(&session);
        let uppercut = card_evidence(&decision, CardId::Uppercut);
        assert!(uppercut.expensive_short_debuff_overlap);
        assert_eq!(uppercut.band, CardRewardPolicyBandV1::SpeculativeAddition);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Uppercut,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn upgraded_debuff_duration_does_not_trigger_short_overlap_gate() {
        let session = a1f10_mummified_hand_session(&[(CardId::Uppercut, 1)]);

        let decision = decision(&session);
        let uppercut = card_evidence(&decision, CardId::Uppercut);
        assert!(!uppercut.expensive_short_debuff_overlap);
        assert_eq!(uppercut.band, CardRewardPolicyBandV1::CloseThreatGap);
    }

    #[test]
    fn expensive_short_debuff_without_owned_source_does_not_trigger_overlap_gate() {
        let mut session = a1f10_mummified_hand_session(&[(CardId::Uppercut, 0)]);
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
        ]);

        let decision = decision(&session);
        let uppercut = card_evidence(&decision, CardId::Uppercut);
        assert!(!uppercut.expensive_short_debuff_overlap);
        assert_eq!(uppercut.band, CardRewardPolicyBandV1::CloseThreatGap);
    }

    #[test]
    fn supported_second_exhaust_power_keeps_typed_mummified_hand_tempo() {
        let mut session = reward_session(&[(CardId::DarkEmbrace, 0)]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 13;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::MummifiedHand));
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
            (CardId::PowerThrough, 0),
            (CardId::SecondWind, 0),
            (CardId::DarkEmbrace, 0),
        ]);

        let decision = decision(&session);
        let dark_embrace = card_evidence(&decision, CardId::DarkEmbrace);
        let tempo = dark_embrace
            .mummified_hand_power_tempo
            .expect("Power candidate with Mummified Hand should expose typed tempo");
        assert_eq!(tempo.card, CardId::DarkEmbrace);
        assert_eq!(tempo.paid_cost, 2);
        assert!(tempo.eligible_positive_cost_cards > 0);
        assert!(!dark_embrace.duplicate_low_marginal);
        assert_ne!(dark_embrace.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::DarkEmbrace,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn efficient_access_amplifies_existing_answers_before_marginal_frontload() {
        let mut session = reward_session(&[(CardId::RecklessCharge, 0), (CardId::BattleTrance, 0)]);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::Automaton);
        session.run_state.current_hp = 76;
        session.run_state.max_hp = 80;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::TrueGrit, 1),
            (CardId::SwordBoomerang, 0),
            (CardId::Clash, 0),
            (CardId::Uppercut, 1),
            (CardId::Corruption, 0),
            (CardId::BurningPact, 0),
            (CardId::SeverSoul, 0),
            (CardId::Offering, 0),
            (CardId::Intimidate, 0),
            (CardId::Headbutt, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let battle_trance = card_evidence(&decision, CardId::BattleTrance);
        assert!(battle_trance.amplifies_existing_answers);
        assert_eq!(
            battle_trance.band,
            CardRewardPolicyBandV1::AmplifyStrategicAccess
        );
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::BattleTrance,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::RecklessCharge,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn covered_urgent_frontload_gap_still_precedes_access_amplification() {
        let mut session = reward_session(&[(CardId::WildStrike, 0), (CardId::BattleTrance, 0)]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 1;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.current_hp = 80;
        session.run_state.max_hp = 80;
        session.run_state.master_deck = [
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Evolve,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();

        let decision = decision(&session);
        assert_eq!(
            card_evidence(&decision, CardId::WildStrike)
                .persistent_draw_pile_status
                .as_ref()
                .expect("Wild Strike persistent status assessment")
                .handling,
            PersistentDrawPileStatusHandlingV1::Covered
        );
        assert_eq!(
            card_evidence(&decision, CardId::WildStrike).band,
            CardRewardPolicyBandV1::CloseThreatGap
        );
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::WildStrike,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::BattleTrance,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn f11_persistent_wound_stays_speculative_with_only_conditional_hand_exhaust() {
        let session = a1f11_mummified_hand_session(&[
            (CardId::WildStrike, 0),
            (CardId::ThunderClap, 0),
            (CardId::Warcry, 0),
        ]);

        let decision = decision(&session);
        let wild = card_evidence(&decision, CardId::WildStrike);
        let assessment = wild
            .persistent_draw_pile_status
            .as_ref()
            .expect("Wild Strike persistent status assessment");

        assert_eq!(
            assessment.handling,
            PersistentDrawPileStatusHandlingV1::Conditional
        );
        assert_eq!(assessment.conditional_hand_exhaust_count, 1);
        assert!(
            !wild.delta.closed_threat_gaps.is_empty()
                || wild.improves_threat_relevant_capability,
            "the gate should remain meaningful even when coarse capability deltas favor Wild Strike"
        );
        assert_eq!(wild.band, CardRewardPolicyBandV1::SpeculativeAddition);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::WildStrike,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn f11_evolve_covers_persistent_wound_without_status_gate_demotion() {
        let mut session = a1f11_mummified_hand_session(&[
            (CardId::WildStrike, 0),
            (CardId::ThunderClap, 0),
            (CardId::Warcry, 0),
        ]);
        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::Evolve, 10_002));

        let decision = decision(&session);
        let wild = card_evidence(&decision, CardId::WildStrike);
        let assessment = wild
            .persistent_draw_pile_status
            .as_ref()
            .expect("Wild Strike persistent status assessment");

        assert_eq!(
            assessment.handling,
            PersistentDrawPileStatusHandlingV1::Covered
        );
        assert_eq!(assessment.draw_recovery_count, 1);
        assert!(
            wild.band < CardRewardPolicyBandV1::SpeculativeAddition,
            "covered status must preserve the underlying supported band: {wild:#?}"
        );
    }

    #[test]
    fn f11_second_light_aoe_does_not_turn_source_count_into_strategic_strength() {
        let session = a1f11_mummified_hand_session(&[
            (CardId::WildStrike, 0),
            (CardId::ThunderClap, 0),
            (CardId::Warcry, 0),
        ]);

        let decision = decision(&session);
        let thunder_clap = card_evidence(&decision, CardId::ThunderClap);

        assert!(thunder_clap.duplicate_low_marginal);
        assert!(thunder_clap
            .delta
            .capability_improvements
            .iter()
            .all(|change| {
                change.before == StrategyCapabilityCoverageV1::Supported
                    && change.after == StrategyCapabilityCoverageV1::Strong
            }));
        assert_eq!(thunder_clap.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::ThunderClap,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn first_light_aoe_source_can_still_close_an_act_one_gap() {
        let mut session = reward_session(&[(CardId::ThunderClap, 0)]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 4;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session.run_state.master_deck = owned_deck(&[
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 0),
        ]);

        let decision = decision(&session);
        let thunder_clap = card_evidence(&decision, CardId::ThunderClap);

        assert!(!thunder_clap.duplicate_low_marginal);
        assert!(thunder_clap.band < CardRewardPolicyBandV1::PreserveDeckQuality);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::ThunderClap,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn ethereal_dazed_does_not_enter_persistent_draw_pile_gate() {
        let session = reward_session(&[(CardId::RecklessCharge, 0)]);
        let decision = decision(&session);

        assert_eq!(
            card_evidence(&decision, CardId::RecklessCharge).persistent_draw_pile_status,
            None
        );
    }

    #[test]
    fn duplicate_no_draw_and_snecko_debt_block_clean_access_amplification() {
        let mut duplicate = reward_session(&[(CardId::BattleTrance, 0)]);
        duplicate.run_state.master_deck.extend(
            [CardId::Uppercut, CardId::BattleTrance]
                .into_iter()
                .enumerate()
                .map(|(index, card)| CombatCard::new(card, index as u32)),
        );
        let duplicate_decision = decision(&duplicate);
        let duplicate_access = card_evidence(&duplicate_decision, CardId::BattleTrance);
        assert!(!duplicate_access.amplifies_existing_answers);
        assert!(matches!(
            &duplicate_access.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .debt_signals
                .contains(&CardComponentSignalKindV1::DuplicateNoDrawAccessDebt)
        ));

        let mut snecko = reward_session(&[(CardId::Offering, 0)]);
        snecko
            .run_state
            .relics
            .push(RelicState::new(RelicId::SneckoEye));
        snecko.run_state.master_deck = [
            CardId::BattleTrance,
            CardId::ShrugItOff,
            CardId::PommelStrike,
            CardId::SpotWeakness,
            CardId::Inflame,
            CardId::FeelNoPain,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();
        let snecko_decision = decision(&snecko);
        let offering = card_evidence(&snecko_decision, CardId::Offering);
        assert!(!offering.amplifies_existing_answers);
        assert!(matches!(
            &offering.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .debt_signals
                .contains(&CardComponentSignalKindV1::SneckoEnergyDiscountDebt)
        ));
    }

    #[test]
    fn first_battle_trance_is_access_not_empty_or_deferred() {
        let session = reward_session(&[(CardId::BattleTrance, 1)]);
        let decision = decision(&session);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::BattleTrance,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            ))
        );
    }

    #[test]
    fn seed006_f8_clash_debt_does_not_masquerade_as_new_frontload() {
        let mut session = reward_session(&[
            (CardId::Clash, 0),
            (CardId::Feed, 0),
            (CardId::PommelStrike, 0),
        ]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 8;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.current_hp = 72;
        session.run_state.max_hp = 80;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Berserk, 0),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 0),
            (CardId::HeavyBlade, 0),
            (CardId::Clothesline, 0),
            (CardId::TwinStrike, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let clash = card_evidence(&decision, CardId::Clash);
        assert!(clash
            .added_deck_shape_risks
            .iter()
            .any(|risk| matches!(risk, DeckShapeRiskV1::ClashPlayabilityDebt { .. })));
        assert_eq!(clash.band, CardRewardPolicyBandV1::Liability);
        for better in [CardId::Feed, CardId::PommelStrike] {
            assert!(
                position(&decision, |key| matches!(
                    key,
                    DecisionCandidateKey::CardRewardPick { card, .. } if *card == better
                )) < position(&decision, |key| matches!(
                    key,
                    DecisionCandidateKey::CardRewardPick {
                        card: CardId::Clash,
                        ..
                    }
                )),
                "{better:?} should precede the newly unplayable Clash; evidence={:#?}",
                decision.evidence
            );
        }
    }

    #[test]
    fn clash_without_deck_shape_debt_is_not_rejected_by_the_shared_gate() {
        let session = reward_session(&[(CardId::Clash, 0)]);
        let decision = decision(&session);
        let clash = card_evidence(&decision, CardId::Clash);
        assert!(clash.added_deck_shape_risks.is_empty());
        assert_ne!(clash.band, CardRewardPolicyBandV1::Liability);
    }

    #[test]
    fn established_boss_scaling_repair_precedes_external_payoff_in_same_band() {
        let mut session = reward_session(&[
            (CardId::Feed, 0),
            (CardId::DemonForm, 0),
            (CardId::FiendFire, 0),
        ]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 16;
        session.run_state.boss_key = Some(EncounterId::Hexaghost);
        session.run_state.current_hp = 33;
        session.run_state.max_hp = 80;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::GhostlyArmor, 0),
            (CardId::SeeingRed, 1),
            (CardId::ShrugItOff, 0),
            (CardId::PommelStrike, 0),
            (CardId::SwordBoomerang, 0),
            (CardId::SpotWeakness, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let demon_form = card_evidence(&decision, CardId::DemonForm);
        let feed = card_evidence(&decision, CardId::Feed);

        assert_eq!(
            demon_form.band,
            CardRewardPolicyBandV1::EstablishStrategicAsset
        );
        assert_eq!(feed.band, CardRewardPolicyBandV1::EstablishStrategicAsset);
        assert_eq!(
            demon_form.boss_damage_plan_improvement,
            Some(CardRewardBossDamagePlanImprovementV1 {
                before_readiness: BossDamagePlanReadinessV1::Engine,
                after_readiness: BossDamagePlanReadinessV1::Engine,
                before_reliability: BossDamagePlanEngineReliabilityV1::Fragile,
                after_reliability: BossDamagePlanEngineReliabilityV1::Established,
            })
        );
        assert_eq!(feed.boss_damage_plan_improvement, None);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::DemonForm,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Feed,
                    ..
                }
            )),
            "the shared reliability repair must break the same-band surface tie; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn known_champ_engine_repair_precedes_generic_capability_count() {
        let session = a2f20_champ_without_damage_engine_session(&[
            (CardId::Clothesline, 0),
            (CardId::DemonForm, 0),
            (CardId::Havoc, 0),
        ]);

        let decision = decision(&session);
        let clothesline = card_evidence(&decision, CardId::Clothesline);
        let demon_form = card_evidence(&decision, CardId::DemonForm);

        assert_eq!(
            clothesline.band,
            CardRewardPolicyBandV1::EstablishStrategicAsset
        );
        assert_eq!(
            demon_form.band,
            CardRewardPolicyBandV1::EstablishStrategicAsset
        );
        assert!(
            clothesline.delta.capability_improvements.len()
                > demon_form.delta.capability_improvements.len(),
            "the fixture must preserve the misleading generic-count advantage"
        );
        assert_eq!(
            demon_form.boss_damage_plan_improvement,
            Some(CardRewardBossDamagePlanImprovementV1 {
                before_readiness: BossDamagePlanReadinessV1::Support,
                after_readiness: BossDamagePlanReadinessV1::Engine,
                before_reliability: BossDamagePlanEngineReliabilityV1::None,
                after_reliability: BossDamagePlanEngineReliabilityV1::Established,
            })
        );
        assert_eq!(clothesline.boss_damage_plan_improvement, None);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::DemonForm,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Clothesline,
                    ..
                }
            )),
            "the exact known-Boss engine repair should beat a larger count of generic capability changes; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn supported_exhaust_payoff_precedes_skip_before_awakened_one() {
        let mut session = reward_session(&[
            (CardId::ThunderClap, 0),
            (CardId::FeelNoPain, 1),
            (CardId::Evolve, 1),
        ]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 42;
        session.run_state.boss_key = Some(EncounterId::AwakenedOne);
        session.run_state.current_hp = 87;
        session.run_state.max_hp = 87;
        session.run_state.master_deck = [
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 1),
            (CardId::HeavyBlade, 0),
            (CardId::Clothesline, 0),
            (CardId::TwinStrike, 0),
            (CardId::Feed, 0),
            (CardId::Intimidate, 0),
            (CardId::Evolve, 1),
            (CardId::BattleTrance, 0),
            (CardId::Shockwave, 0),
            (CardId::Barricade, 1),
            (CardId::FiendFire, 1),
            (CardId::IronWave, 1),
            (CardId::Disarm, 0),
            (CardId::SwordBoomerang, 1),
            (CardId::PommelStrike, 0),
            (CardId::Immolate, 1),
            (CardId::Cleave, 0),
            (CardId::Inflame, 1),
            (CardId::BodySlam, 0),
            (CardId::SecondWind, 1),
            (CardId::Sentinel, 1),
            (CardId::DarkEmbrace, 1),
            (CardId::Carnage, 0),
            (CardId::Bloodletting, 1),
            (CardId::FireBreathing, 1),
            (CardId::Whirlwind, 1),
            (CardId::Parasite, 0),
            (CardId::ThunderClap, 1),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let fnp = card_evidence(&decision, CardId::FeelNoPain);
        assert!(matches!(
            &fnp.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .positive_signals
                .contains(&CardComponentSignalKindV1::ExhaustPayoffSupported)
        ));
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::FeelNoPain,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "supported exhaust payoff should precede skip; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn light_aoe_without_exact_delta_does_not_outrank_skip_in_a_mature_deck() {
        let mut session = reward_session(&[
            (CardId::Anger, 1),
            (CardId::Bloodletting, 0),
            (CardId::ThunderClap, 1),
        ]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 38;
        session.run_state.boss_key = Some(EncounterId::AwakenedOne);
        session.run_state.current_hp = 72;
        session.run_state.max_hp = 90;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::ShrugItOff, 1),
            (CardId::Clash, 0),
            (CardId::SearingBlow, 0),
            (CardId::IronWave, 0),
            (CardId::SeverSoul, 1),
            (CardId::Whirlwind, 1),
            (CardId::Cleave, 1),
            (CardId::FiendFire, 1),
            (CardId::DarkEmbrace, 0),
            (CardId::Armaments, 1),
            (CardId::Shockwave, 1),
            (CardId::PommelStrike, 1),
            (CardId::Offering, 1),
            (CardId::DemonForm, 0),
            (CardId::TrueGrit, 1),
            (CardId::DarkShackles, 0),
            (CardId::Inflame, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let thunder_clap = card_evidence(&decision, CardId::ThunderClap);
        assert!(matches!(
            &thunder_clap.acquisition,
            CardRewardPolicyAcquisitionV1::Card { semantics, .. }
                if semantics.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
        ));
        assert_eq!(
            card_analysis_profile_v1(CardId::ThunderClap, 1).aoe_support,
            CardAnalysisAoeSupportV1::Present
        );
        assert!(thunder_clap.delta.closed_threat_gaps.is_empty());
        assert!(thunder_clap.delta.capability_improvements.is_empty());
        assert!(thunder_clap.delta.resolved_formation_needs.is_empty());
        assert!(thunder_clap.delta.added_formation_strengths.is_empty());
        assert_eq!(
            thunder_clap.band,
            CardRewardPolicyBandV1::SpeculativeAddition
        );
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::ThunderClap,
                    ..
                }
            )),
            "light AoE that adds no exact capability must not outrank preserving deck quality; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn supported_exhaust_converter_survives_unmodeled_magnitude_and_secondary_axis_debt() {
        let mut session = reward_session(&[
            (CardId::FiendFire, 0),
            (CardId::SwordBoomerang, 1),
            (CardId::Clash, 0),
        ]);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 30;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.current_hp = 64;
        session.run_state.max_hp = 93;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 1),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Clothesline, 1),
            (CardId::IronWave, 0),
            (CardId::ThunderClap, 0),
            (CardId::BurningPact, 0),
            (CardId::Clash, 0),
            (CardId::ShrugItOff, 1),
            (CardId::DarkShackles, 0),
            (CardId::BattleTrance, 0),
            (CardId::FeelNoPain, 1),
            (CardId::FeelNoPain, 1),
            (CardId::TrueGrit, 1),
            (CardId::Disarm, 0),
            (CardId::BodySlam, 1),
            (CardId::PommelStrike, 0),
            (CardId::Entrench, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let fiend_fire = card_evidence(&decision, CardId::FiendFire);
        assert!(fiend_fire.introduces_unsupported_mechanics);
        assert!(matches!(
            &fiend_fire.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .positive_signals
                .contains(&CardComponentSignalKindV1::ExhaustConversionSupported)
                && component_signals
                    .debt_signals
                    .contains(&CardComponentSignalKindV1::StrengthPayoffUnsupported)
        ));
        assert_eq!(
            fiend_fire.band,
            CardRewardPolicyBandV1::EstablishStrategicAsset
        );
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::FiendFire,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "a visible exhaust conversion supported by the existing engine must not be erased by an unmodeled magnitude or optional strength axis; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn intrinsic_asset_axis_survives_an_unfunded_secondary_package_axis() {
        let mut session = reward_session(&[(CardId::Whirlwind, 0), (CardId::HeavyBlade, 0)]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 46;
        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);

        let decision = decision(&session);
        let whirlwind = card_evidence(&decision, CardId::Whirlwind);
        let heavy_blade = card_evidence(&decision, CardId::HeavyBlade);
        assert!(matches!(
            &whirlwind.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                semantics,
                component_signals,
                ..
            } if semantics
                .roles
                .contains(&CardRewardSemanticRoleV1::AoeDamage)
                && component_signals
                    .debt_signals
                    .contains(&CardComponentSignalKindV1::StrengthPayoffUnsupported)
        ));
        assert_ne!(whirlwind.band, CardRewardPolicyBandV1::Liability);
        assert_eq!(heavy_blade.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Whirlwind,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            ))
        );
    }

    #[test]
    fn random_multi_hit_uses_target_topology_without_claiming_multi_target_control() {
        let mut session = reward_session(&[(CardId::SwordBoomerang, 0), (CardId::Entrench, 0)]);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 28;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        session.run_state.master_deck = [
            (CardId::Defend, 1),
            (CardId::Defend, 1),
            (CardId::Defend, 1),
            (CardId::Defend, 1),
            (CardId::FeelNoPain, 1),
            (CardId::TrueGrit, 1),
            (CardId::BodySlam, 1),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let champ_decision = decision(&session);
        let sword_boomerang = card_evidence(&champ_decision, CardId::SwordBoomerang);
        assert!(sword_boomerang.random_target_frontload_reliable);
        assert!(matches!(
            &sword_boomerang.acquisition,
            CardRewardPolicyAcquisitionV1::Card { semantics, .. }
                if !semantics.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
        ));
        assert!(!sword_boomerang
            .delta
            .capability_improvements
            .iter()
            .any(|change| change.capability == StrategyCapabilityKindV1::MultiTargetControl));
        assert!(
            position(&champ_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::SwordBoomerang,
                    ..
                }
            )) < position(&champ_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Entrench,
                    ..
                }
            )),
            "a random-target attack becomes reliable frontload against a known single opponent without claiming deterministic all-enemy damage; evidence={:#?}",
            champ_decision.evidence
        );

        session.run_state.act_num = 3;
        session.run_state.floor_num = 46;
        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);
        let multi_target_decision = decision(&session);
        let sword_boomerang = card_evidence(&multi_target_decision, CardId::SwordBoomerang);
        assert!(!sword_boomerang.random_target_frontload_reliable);
        assert!(
            position(&multi_target_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Entrench,
                    ..
                }
            )) < position(&multi_target_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::SwordBoomerang,
                    ..
                }
            )),
            "random-target damage must not receive the single-opponent reliability credit in a multi-opponent boss fight; evidence={:#?}",
            multi_target_decision.evidence
        );
    }

    #[test]
    fn candidate_strength_burst_closes_uncontested_converter_and_payoff_package() {
        let mut session =
            reward_session(&[(CardId::Havoc, 1), (CardId::Flex, 1), (CardId::Pummel, 0)]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 45;
        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::ClockworkSouvenir));
        session.run_state.master_deck = [
            (CardId::FiendFire, 1),
            (CardId::SwordBoomerang, 0),
            (CardId::Reaper, 0),
            (CardId::BurningPact, 1),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let flex = card_evidence(&decision, CardId::Flex);
        assert!(matches!(
            &flex.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .positive_signals
                .contains(&CardComponentSignalKindV1::StrengthConvertiblePackageUnlock)
                && component_signals
                    .note_signals
                    .contains(&CardComponentSignalKindV1::ConvertibleStrengthRequiresDrawTiming)
        ));
        assert_eq!(flex.band, CardRewardPolicyBandV1::EstablishStrategicAsset);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Flex,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "the exact candidate delta closes a convertible strength package and must outrank preserving deck quality; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn candidate_strength_burst_does_not_double_book_artifact_needed_by_existing_access() {
        let mut session =
            reward_session(&[(CardId::Havoc, 1), (CardId::Flex, 1), (CardId::Pummel, 0)]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 45;
        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::ClockworkSouvenir));
        session.run_state.master_deck = [
            (CardId::FiendFire, 1),
            (CardId::SwordBoomerang, 0),
            (CardId::Reaper, 0),
            (CardId::BattleTrance, 1),
            (CardId::BurningPact, 1),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let flex = card_evidence(&decision, CardId::Flex);
        assert!(matches!(
            &flex.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if !component_signals
                .positive_signals
                .contains(&CardComponentSignalKindV1::StrengthConvertiblePackageUnlock)
        ));
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Flex,
                    ..
                }
            )),
            "one Artifact charge cannot simultaneously fund Battle Trance and permanent Flex; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn unsupported_exhaust_payoff_does_not_receive_shared_asset_status() {
        let session = reward_session(&[(CardId::FeelNoPain, 0)]);
        let decision = decision(&session);
        let fnp = card_evidence(&decision, CardId::FeelNoPain);
        assert!(matches!(
            &fnp.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .debt_signals
                .contains(&CardComponentSignalKindV1::ExhaustPayoffUnsupported)
        ));
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::FeelNoPain,
                    ..
                }
            )),
            "unsupported exhaust payoff must not be promoted above skip; evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn champ_obligation_prefers_persistent_scaling_to_transient_vulnerable() {
        let mut session = reward_session(&[(CardId::ThunderClap, 0), (CardId::Inflame, 1)]);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 30;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        let decision = decision(&session);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Inflame,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::ThunderClap,
                    ..
                }
            )),
            "evidence={:#?}",
            decision.evidence
        );
    }

    #[test]
    fn persistent_strength_down_precedes_an_already_supported_damage_payoff() {
        let mut session = reward_session(&[
            (CardId::SwordBoomerang, 0),
            (CardId::Disarm, 0),
            (CardId::PerfectedStrike, 0),
        ]);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 21;
        session.run_state.boss_key = Some(EncounterId::TheChamp);
        // Keep enough established offense that Sword Boomerang remains a
        // supported payoff; a smaller deck would test two missing capabilities.
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::PowerThrough, 0),
            (CardId::Uppercut, 0),
            (CardId::SecondWind, 0),
            (CardId::Cleave, 0),
            (CardId::ShrugItOff, 0),
            (CardId::DarkEmbrace, 0),
            (CardId::Uppercut, 0),
            (CardId::WildStrike, 0),
            (CardId::DarkEmbrace, 0),
            (CardId::FiendFire, 0),
            (CardId::Cleave, 0),
            (CardId::DemonForm, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let decision = decision(&session);
        let sword_boomerang = card_evidence(&decision, CardId::SwordBoomerang);
        let disarm = card_evidence(&decision, CardId::Disarm);

        assert!(sword_boomerang.random_target_frontload_reliable);
        assert_eq!(
            sword_boomerang.band,
            CardRewardPolicyBandV1::EstablishStrategicAsset
        );
        assert!(disarm.improves_threat_relevant_capability);
        assert_eq!(
            disarm.band,
            CardRewardPolicyBandV1::ImproveRequiredCapability
        );
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Disarm,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::SwordBoomerang,
                    ..
                }
            )),
            "persistent mitigation should close the defense quality gap before adding another supported payoff"
        );
    }

    #[test]
    fn pyramid_without_status_digest_exposes_wild_strike_liability() {
        let mut session = reward_session(&[(CardId::WildStrike, 0)]);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::RunicPyramid));
        let decision = decision(&session);
        let wild = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::CardRewardPick {
                        card: CardId::WildStrike,
                        ..
                    }
                )
            })
            .expect("Wild Strike evidence");
        assert!(wild.introduces_undigested_status_burden);
        assert_eq!(wild.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::WildStrike,
                    ..
                }
            ))
        );
    }

    #[test]
    fn every_exact_card_reward_action_keeps_positive_support() {
        let mut session = reward_session(&[(CardId::WildStrike, 0), (CardId::BattleTrance, 1)]);
        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::BattleTrance, 10_001));
        let decision = decision(&session);
        assert_eq!(decision.prior.entries.len(), decision.evidence.len());
        assert!(decision
            .prior
            .entries
            .iter()
            .all(|entry| entry.probability.is_finite() && entry.probability > 0.0));
    }

    #[test]
    fn audit_exposes_every_ranked_candidate_without_changing_support() {
        let session = reward_session(&[
            (CardId::WildStrike, 0),
            (CardId::BattleTrance, 1),
            (CardId::DualWield, 0),
        ]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let audit = exact_card_reward_policy_audit_v1(&session, &legal).expect("card reward audit");

        assert_eq!(audit.candidates.len(), legal.len());
        assert!(audit
            .candidates
            .iter()
            .enumerate()
            .all(|(rank, candidate)| {
                candidate.owner_rank == rank
                    && !candidate.label.is_empty()
                    && candidate.prior_probability.is_finite()
                    && candidate.prior_probability > 0.0
            }));
    }

    #[test]
    fn plain_block_needs_actionable_evidence_but_intrinsic_payoff_remains_an_asset() {
        let mut session = reward_session(&[(CardId::Sentinel, 0), (CardId::Feed, 0)]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 11;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.current_hp = 83;
        session.run_state.max_hp = 91;
        session.run_state.master_deck = [
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Strike, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Berserk, 0),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 0),
            (CardId::Evolve, 0),
            (CardId::Clothesline, 0),
            (CardId::Intimidate, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();
        let decision = decision(&session);
        let sentinel = card_evidence(&decision, CardId::Sentinel);
        let feed = card_evidence(&decision, CardId::Feed);
        let skip = position(&decision, |key| {
            matches!(key, DecisionCandidateKey::CardRewardSkip { .. })
        });

        assert!(matches!(
            &sentinel.acquisition,
            CardRewardPolicyAcquisitionV1::Card { semantics, .. }
                if semantics.roles == [CardRewardSemanticRoleV1::Block]
        ));
        assert!(sentinel.delta.closed_threat_gaps.is_empty());
        assert!(sentinel.delta.capability_improvements.is_empty());
        assert!(sentinel.delta.resolved_formation_needs.is_empty());
        assert_eq!(sentinel.band, CardRewardPolicyBandV1::SpeculativeAddition);
        assert!(
            skip < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Sentinel,
                    ..
                }
            )),
            "plain block without actionable evidence must not outrank preserving deck quality"
        );

        assert!(matches!(
            &feed.acquisition,
            CardRewardPolicyAcquisitionV1::Card { semantics, .. }
                if semantics
                    .roles
                    .contains(&CardRewardSemanticRoleV1::CombatExternalPayoff)
        ));
        assert_eq!(feed.band, CardRewardPolicyBandV1::EstablishStrategicAsset);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Feed,
                    ..
                }
            )) < skip,
            "an intrinsic persistent payoff must remain independently admissible"
        );
    }

    #[test]
    fn known_power_tax_demotes_supported_minor_power_without_exact_improvement() {
        let mut session = reward_session(&[(CardId::Rupture, 1)]);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 45;
        session.run_state.boss_key = Some(EncounterId::AwakenedOne);
        session.run_state.current_hp = 87;
        session.run_state.max_hp = 103;
        session.run_state.master_deck = [
            (CardId::Defend, 0),
            (CardId::Defend, 0),
            (CardId::Bash, 1),
            (CardId::Berserk, 0),
            (CardId::WildStrike, 0),
            (CardId::ShrugItOff, 1),
            (CardId::Evolve, 0),
            (CardId::Clothesline, 1),
            (CardId::Feed, 0),
            (CardId::Intimidate, 0),
            (CardId::BattleTrance, 0),
            (CardId::Shockwave, 0),
            (CardId::Barricade, 1),
            (CardId::FiendFire, 1),
            (CardId::IronWave, 1),
            (CardId::Disarm, 0),
            (CardId::PommelStrike, 0),
            (CardId::Immolate, 1),
            (CardId::Cleave, 0),
            (CardId::Inflame, 1),
            (CardId::DarkEmbrace, 1),
            (CardId::SecondWind, 1),
            (CardId::BodySlam, 0),
            (CardId::Bloodletting, 1),
            (CardId::DarkShackles, 1),
            (CardId::SeverSoul, 0),
            (CardId::Parasite, 0),
            (CardId::ThunderClap, 1),
            (CardId::FeelNoPain, 1),
            (CardId::Parasite, 0),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (card, upgrades))| {
            let mut owned = CombatCard::new(card, index as u32);
            owned.upgrades = upgrades;
            owned
        })
        .collect();

        let awakened_decision = decision(&session);
        let rupture = card_evidence(&awakened_decision, CardId::Rupture);
        assert!(matches!(
            &rupture.acquisition,
            CardRewardPolicyAcquisitionV1::Card {
                component_signals,
                ..
            } if component_signals
                .positive_signals
                .contains(&CardComponentSignalKindV1::SelfDamagePayoffSupported)
        ));
        assert!(rupture.delta.closed_threat_gaps.is_empty());
        assert!(rupture.delta.capability_improvements.is_empty());
        assert!(rupture.delta.resolved_formation_needs.is_empty());
        assert!(rupture.delta.added_formation_strengths.is_empty());
        assert!(rupture.boss_power_tax_conflict);
        assert_eq!(rupture.band, CardRewardPolicyBandV1::SpeculativeAddition);
        let mut exact_improvement = rupture.delta.clone();
        exact_improvement.capability_improvements.push(
            crate::eval::run_control::RunPolicyCapabilityChangeV1 {
                capability: StrategyCapabilityKindV1::LongFightScaling,
                before: StrategyCapabilityCoverageV1::Thin,
                after: StrategyCapabilityCoverageV1::Supported,
            },
        );
        assert!(
            !boss_power_tax_conflict_v1(true, &rupture.acquisition, &exact_improvement),
            "an exact capability improvement must override the generic minor-power conflict"
        );
        assert!(
            position(&awakened_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )) < position(&awakened_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Rupture,
                    ..
                }
            )),
            "a supported but non-improving minor power must not outrank preserving deck quality against the known power-tax boss"
        );

        session.run_state.boss_key = Some(EncounterId::DonuAndDeca);
        assert!(
            !card_evidence(&decision(&session), CardId::Rupture).boss_power_tax_conflict,
            "the conflict must come from the shared boss pressure, not from a Rupture-specific rejection"
        );
    }

    #[test]
    fn repeated_ordinary_frontload_needs_real_marginal_strategic_evidence() {
        let mut session = reward_session(&[
            (CardId::IronWave, 0),
            (CardId::WildStrike, 0),
            (CardId::DualWield, 0),
        ]);
        session.run_state.act_num = 1;
        session.run_state.floor_num = 1;
        session.run_state.current_hp = 80;
        session.run_state.max_hp = 80;
        session.run_state.master_deck = [
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::Berserk,
            // Isolate the ordinary-frontload contract from the independent
            // persistent draw-pile status gate.
            CardId::Evolve,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();

        let first_decision = decision(&session);
        assert!(
            position(&first_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::WildStrike,
                    ..
                }
            )) < position(&first_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "a first frontload card that closes exact current gaps remains admissible"
        );

        let mut second = reward_session(&[
            (CardId::PerfectedStrike, 0),
            (CardId::Clash, 0),
            (CardId::Warcry, 0),
        ]);
        second.run_state.act_num = 1;
        second.run_state.floor_num = 2;
        second.run_state.current_hp = 75;
        second.run_state.max_hp = 80;
        second.run_state.master_deck = session.run_state.master_deck.clone();
        second
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::WildStrike, 11));
        let second_decision = decision(&second);
        let skip = position(&second_decision, |key| {
            matches!(key, DecisionCandidateKey::CardRewardSkip { .. })
        });
        for card in [CardId::Clash, CardId::PerfectedStrike] {
            assert!(
                skip < position(&second_decision, |key| matches!(
                    key,
                    DecisionCandidateKey::CardRewardPick {
                        card: candidate,
                        ..
                    } if *candidate == card
                )),
                "{card:?} must not outrank skip merely by increasing an irrelevant capability or repeating an ordinary answer role; evidence={:#?}",
                second_decision.evidence
            );
        }

        let mut mitigation = reward_session(&[(CardId::Shockwave, 1)]);
        mitigation.run_state.act_num = 2;
        mitigation.run_state.floor_num = 17;
        mitigation.run_state.current_hp = 74;
        mitigation.run_state.max_hp = 80;
        mitigation.run_state.boss_key = Some(EncounterId::TheChamp);
        mitigation.run_state.master_deck = second.run_state.master_deck;
        let mitigation_decision = decision(&mitigation);
        assert!(
            position(&mitigation_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Shockwave,
                    ..
                }
            )) < position(&mitigation_decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardSkip { .. }
            )),
            "a durable mitigation answer for the known Champ must remain independently admissible"
        );
    }

    #[test]
    fn unfunded_upgrade_investment_does_not_masquerade_as_immediate_work() {
        let session = reward_session(&[(CardId::SearingBlow, 0), (CardId::Armaments, 0)]);
        let decision = decision(&session);
        let searing = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::CardRewardPick {
                        card: CardId::SearingBlow,
                        ..
                    }
                )
            })
            .expect("Searing Blow evidence");

        assert_eq!(
            searing.upgrade_investment_support,
            Some(StrategyPlanSupportV1::Blocked)
        );
        assert_eq!(searing.band, CardRewardPolicyBandV1::Liability);
        assert!(
            position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::Armaments,
                    ..
                }
            )) < position(&decision, |key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::SearingBlow,
                    ..
                }
            ))
        );
    }

    #[test]
    fn funded_upgrade_investment_keeps_its_underlying_policy_band() {
        assert_eq!(
            apply_upgrade_investment_gate_v1(
                CardRewardPolicyBandV1::CloseThreatGap,
                Some(StrategyPlanSupportV1::Strong),
            ),
            CardRewardPolicyBandV1::CloseThreatGap
        );
        assert_eq!(
            apply_upgrade_investment_gate_v1(
                CardRewardPolicyBandV1::CloseThreatGap,
                Some(StrategyPlanSupportV1::Plausible),
            ),
            CardRewardPolicyBandV1::SpeculativeAddition
        );
    }
}
