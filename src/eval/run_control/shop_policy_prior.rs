use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ai::block_plan_profile_v1::{block_plan_profile_v1, BlockPlanProfileV1};
use crate::ai::card_component_signal_v1::{
    evaluate_card_component_signals_v1, is_unresolved_package_payoff_debt_signal_v1,
    CardComponentSignalContextV1, CardComponentSignalReportV1,
};
use crate::ai::card_semantics_v1::{
    card_access_evidence_v1, card_reward_facts_v1, card_reward_semantic_profile_v1,
    potion_acquisition_requirements_v1, potion_acquisition_traits_v1,
    relic_acquisition_requirements_v1, relic_acquisition_traits_v1, AcquisitionRequirementV1,
    CardAccessEvidenceV1, CardAccessLeverageV1, CardRewardSemanticProfileV1,
    CardRewardSemanticRoleV1, PotionAcquisitionTraitV1, RelicAcquisitionTraitV1,
};
use crate::ai::combat_upgrade_coverage_v1::CombatUpgradeScopeV1;
use crate::ai::deck_mutation_compiler_v1::{
    deck_removal_target_snapshots_v1, DeckMutationTargetLossTierV1,
};
use crate::ai::deck_shape_v1::{
    deck_shape_candidate_delta_v1, deck_shape_profile_v1, DeckShapeProfileV1, DeckShapeRiskV1,
};
use crate::ai::deck_startup_profile_v1::{deck_startup_profile_v1, DeckStartupProfileV1};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, threat_relevant_capability_improvements_v1,
    StrategyCapabilityCoverageV1, StrategyCapabilityKindV1, StrategyDeckFormationNeedV1,
    StrategyPackageIdV2,
};
use crate::ai::route_window_facts::{
    build_route_path_family_from_target, route_window_targets, RouteWindowFactsConfig,
};
use crate::ai::strategy::acquisition::{
    assess_card_acquisition, evaluate_deck_construction_contract, AcquisitionContext,
    AcquisitionOpportunityCost, AcquisitionPolicyDecision, AcquisitionPolicyReason,
    AcquisitionPolicyVerdict,
};
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::reward_admission::assess_reward_admission_from_master_deck;
use crate::ai::strength_profile_v1::card_unlocks_convertible_strength_payoff_v1;
use crate::content::cards::{get_card_definition, upgrade_card_once_java, CardId, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::{energy_master_delta, RelicId};
use crate::runtime::combat::CombatCard;
use crate::state::core::EngineState;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, run_policy_state_delta_v1,
    DecisionCandidateKey, ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1,
    RunControlSession, RunPolicyCandidateV1, RunPolicyCapabilityChangeV1, RunPolicyPriorV1,
    RunPolicyThreatGapKeyV1,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopPolicyBandV1 {
    ResolvePendingBoundary,
    ImmediateSurvival,
    CloseThreatGap,
    AmplifyStrategicAccess,
    ImproveRequiredCapability,
    DeckRepair,
    EstablishStrategicAsset,
    PreserveResources,
    SpeculativePurchase,
    Liability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ShopPolicyFollowupV1 {
    Shop,
    Reward,
    Selection,
    Map,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionRequirementSupportV1 {
    Current,
    OwnedUpgradePath {
        card: CardId,
        uuid: u32,
        upgrades_before: u8,
        energy_cost_before: i32,
        energy_cost_after: i32,
        energy_gain_before: i32,
        energy_gain_after: i32,
    },
    Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShopPolicyAcquisitionV1 {
    Card {
        card: CardId,
        upgrades: u8,
        copies_before: usize,
        semantics: CardRewardSemanticProfileV1,
        access: Option<CardAccessEvidenceV1>,
        component_signals: CardComponentSignalReportV1,
    },
    Relic {
        relic: RelicId,
        traits: Vec<RelicAcquisitionTraitV1>,
        requirements: Vec<AcquisitionRequirementV1>,
        requirement_support: AcquisitionRequirementSupportV1,
    },
    Potion {
        potion: PotionId,
        traits: Vec<PotionAcquisitionTraitV1>,
        requirements: Vec<AcquisitionRequirementV1>,
        requirement_support: AcquisitionRequirementSupportV1,
    },
    Purge {
        card: CardId,
        upgrades: u8,
    },
    OpenRewards,
    Leave,
}

pub type ShopPolicyCapabilityChangeV1 = RunPolicyCapabilityChangeV1;
pub type ShopPolicyThreatGapKeyV1 = RunPolicyThreatGapKeyV1;

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPolicyActionEvidenceV1 {
    pub candidate_id: String,
    pub candidate_key: DecisionCandidateKey,
    pub acquisition: ShopPolicyAcquisitionV1,
    pub band: ShopPolicyBandV1,
    pub followup: ShopPolicyFollowupV1,
    pub gold_spent: i32,
    pub hp_gain: i32,
    pub max_hp_gain: i32,
    pub deck_size_delta: isize,
    pub closed_threat_gaps: Vec<ShopPolicyThreatGapKeyV1>,
    pub capability_improvements: Vec<ShopPolicyCapabilityChangeV1>,
    pub reinforced_threat_capabilities: Vec<StrategyCapabilityKindV1>,
    pub resolved_formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub added_formation_strengths: Vec<StrategyPackageIdV2>,
    pub matched_consumable_capabilities: Vec<StrategyCapabilityKindV1>,
    pub upgrade_scope_before: Option<CombatUpgradeScopeV1>,
    pub upgrade_scope_after: Option<CombatUpgradeScopeV1>,
    pub introduces_status_burden: bool,
    pub added_deck_shape_risks: Vec<DeckShapeRiskV1>,
    pub introduces_package_debt: bool,
    pub redundant_upgrade_access: bool,
    pub card_acquisition_policy: Option<AcquisitionPolicyDecision>,
    pub card_acquisition_opportunity_cost: Option<AcquisitionOpportunityCost>,
    pub spent_gold_before_action: bool,
    pub durable_asset_support: bool,
    pub purge_target_loss: Option<DeckMutationTargetLossTierV1>,
    surface_index: usize,
}

#[derive(Clone, Debug)]
pub struct ExactShopPolicyDecisionV1 {
    pub exact: ExactRunPolicyDecisionV1,
    pub evidence: Vec<ShopPolicyActionEvidenceV1>,
    pub prior: RunPolicyPriorV1,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShopPolicyAuditCandidateV1 {
    pub owner_rank: usize,
    pub candidate_id: String,
    pub label: String,
    pub candidate_key: DecisionCandidateKey,
    pub acquisition: String,
    pub band: ShopPolicyBandV1,
    pub followup: ShopPolicyFollowupV1,
    pub gold_spent: i32,
    pub hp_gain: i32,
    pub max_hp_gain: i32,
    pub deck_size_delta: isize,
    pub closed_threat_gaps: Vec<String>,
    pub capability_improvements: Vec<String>,
    pub reinforced_threat_capabilities: Vec<String>,
    pub resolved_formation_needs: Vec<String>,
    pub added_formation_strengths: Vec<String>,
    pub matched_consumable_capabilities: Vec<String>,
    pub upgrade_scope_before: Option<String>,
    pub upgrade_scope_after: Option<String>,
    pub introduces_status_burden: bool,
    pub added_deck_shape_risks: Vec<String>,
    pub introduces_package_debt: bool,
    pub redundant_upgrade_access: bool,
    pub card_acquisition_verdict: Option<AcquisitionPolicyVerdict>,
    pub card_acquisition_reason: Option<AcquisitionPolicyReason>,
    pub card_acquisition_opportunity_cost: Option<AcquisitionOpportunityCost>,
    pub spent_gold_before_action: bool,
    pub durable_asset_support: bool,
    pub purge_target_loss: Option<String>,
    pub surface_index: usize,
    pub prior_probability: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExactShopPolicyAuditV1 {
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub candidates: Vec<ShopPolicyAuditCandidateV1>,
}

impl ExactShopPolicyDecisionV1 {
    pub fn audit(
        &self,
        legal: &[RunPolicyCandidateV1<'_>],
    ) -> Result<ExactShopPolicyAuditV1, String> {
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
                            "shop policy audit could not find legal candidate '{}'",
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
                            "shop policy audit could not find prior for candidate '{}'",
                            evidence.candidate_id
                        )
                    })?;
                Ok(ShopPolicyAuditCandidateV1 {
                    owner_rank,
                    candidate_id: evidence.candidate_id.clone(),
                    label: legal_candidate.label.to_string(),
                    candidate_key: evidence.candidate_key.clone(),
                    acquisition: format!("{:?}", evidence.acquisition),
                    band: evidence.band,
                    followup: evidence.followup,
                    gold_spent: evidence.gold_spent,
                    hp_gain: evidence.hp_gain,
                    max_hp_gain: evidence.max_hp_gain,
                    deck_size_delta: evidence.deck_size_delta,
                    closed_threat_gaps: evidence
                        .closed_threat_gaps
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    capability_improvements: evidence
                        .capability_improvements
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    reinforced_threat_capabilities: evidence
                        .reinforced_threat_capabilities
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    resolved_formation_needs: evidence
                        .resolved_formation_needs
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    added_formation_strengths: evidence
                        .added_formation_strengths
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    matched_consumable_capabilities: evidence
                        .matched_consumable_capabilities
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    upgrade_scope_before: evidence
                        .upgrade_scope_before
                        .map(|value| format!("{value:?}")),
                    upgrade_scope_after: evidence
                        .upgrade_scope_after
                        .map(|value| format!("{value:?}")),
                    introduces_status_burden: evidence.introduces_status_burden,
                    added_deck_shape_risks: evidence
                        .added_deck_shape_risks
                        .iter()
                        .map(|value| format!("{value:?}"))
                        .collect(),
                    introduces_package_debt: evidence.introduces_package_debt,
                    redundant_upgrade_access: evidence.redundant_upgrade_access,
                    card_acquisition_verdict: evidence
                        .card_acquisition_policy
                        .map(|policy| policy.verdict),
                    card_acquisition_reason: evidence
                        .card_acquisition_policy
                        .map(|policy| policy.reason),
                    card_acquisition_opportunity_cost: evidence.card_acquisition_opportunity_cost,
                    spent_gold_before_action: evidence.spent_gold_before_action,
                    durable_asset_support: evidence.durable_asset_support,
                    purge_target_loss: evidence.purge_target_loss.map(|value| format!("{value:?}")),
                    surface_index: evidence.surface_index,
                    prior_probability,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(ExactShopPolicyAuditV1 {
            current_hp: self.exact.before.resources.current_hp,
            max_hp: self.exact.before.resources.max_hp,
            gold: self.exact.before.resources.gold,
            candidates,
        })
    }
}

pub fn exact_shop_policy_audit_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactShopPolicyAuditV1, String> {
    exact_shop_policy_decision_v1(session, legal)?.audit(legal)
}

pub fn exact_shop_policy_prior_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<RunPolicyPriorV1, String> {
    Ok(exact_shop_policy_decision_v1(session, legal)?.prior)
}

pub fn exact_shop_policy_decision_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactShopPolicyDecisionV1, String> {
    if !matches!(session.engine_state, EngineState::Shop(_)) {
        return Err("exact shop policy requires a Shop decision boundary".to_string());
    }

    let exact = exact_run_policy_decision_v1(session)?;
    validate_same_candidate_surface(&exact, legal)?;
    let purge_target_losses = deck_removal_target_snapshots_v1(&session.run_state)
        .into_iter()
        .map(|snapshot| (snapshot.deck_index, snapshot.target_loss.tier))
        .collect::<BTreeMap<_, _>>();
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let formation_needs = strategy.formation_summary().needs;
    let startup = deck_startup_profile_v1(&session.run_state);
    let deck_shape = deck_shape_profile_v1(&session.run_state);
    let block_plan = block_plan_profile_v1(&session.run_state);
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let mut evidence = exact
        .actions
        .iter()
        .enumerate()
        .map(|(surface_index, action)| {
            shop_action_evidence_v1(
                session,
                &exact,
                action,
                surface_index,
                &purge_target_losses,
                &formation_needs,
                &startup,
                &deck_shape,
                &block_plan,
                deck_plan,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    evidence.sort_by(compare_shop_evidence);
    let prior = positive_ranked_run_policy_prior_v1(
        legal,
        evidence
            .iter()
            .map(|candidate| candidate.candidate_id.clone()),
    )?;

    Ok(ExactShopPolicyDecisionV1 {
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
        .map(|action| action.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let legal_ids = legal
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    if exact_ids != legal_ids || exact.actions.len() != legal.len() {
        return Err(format!(
            "shop policy surface disagrees with exact model: exact={} policy={}",
            exact.actions.len(),
            legal.len()
        ));
    }
    Ok(())
}

fn shop_action_evidence_v1(
    parent: &RunControlSession,
    decision: &ExactRunPolicyDecisionV1,
    action: &ExactRunPolicyActionSuccessorV1,
    surface_index: usize,
    purge_target_losses: &BTreeMap<usize, DeckMutationTargetLossTierV1>,
    formation_needs: &[StrategyDeckFormationNeedV1],
    startup: &DeckStartupProfileV1,
    deck_shape: &DeckShapeProfileV1,
    block_plan: &BlockPlanProfileV1,
    deck_plan: DeckPlanSnapshot,
) -> Result<ShopPolicyActionEvidenceV1, String> {
    let candidate_key = action
        .candidate_key
        .clone()
        .ok_or_else(|| format!("shop candidate '{}' has no typed key", action.candidate_id))?;
    let acquisition = acquisition_v1(parent, &candidate_key, formation_needs, startup, block_plan)?;
    let (card_acquisition_policy, card_acquisition_opportunity_cost) =
        card_acquisition_policy_v1(parent, &candidate_key, &acquisition, deck_plan);
    let delta = run_policy_state_delta_v1(&decision.before, &action.after);
    let closed_threat_gaps = delta.closed_threat_gaps;
    let capability_improvements = delta.capability_improvements;
    let threat_relevant_improvements = threat_relevant_capability_improvements_v1(
        &decision.before.threats,
        &decision.before.threat_coverage,
        &action.after.threat_coverage,
    );
    let reinforced_threat_capabilities = capability_improvements
        .iter()
        .filter(|change| {
            change.before == StrategyCapabilityCoverageV1::Supported
                && change.after == StrategyCapabilityCoverageV1::Strong
                && threat_relevant_improvements.contains(&change.capability)
        })
        .map(|change| change.capability)
        .collect::<Vec<_>>();
    let resolved_formation_needs = delta.resolved_formation_needs;
    let added_formation_strengths = delta.added_formation_strengths;
    let matched_consumable_capabilities =
        matched_consumable_capabilities_v1(&decision.before, &acquisition);
    let upgrade_scope_before = decision.before.combat_upgrade_coverage.strongest_scope();
    let upgrade_scope_after = action.after.combat_upgrade_coverage.strongest_scope();
    let introduces_status_burden = action.after.deck.status_generators
        > decision.before.deck.status_generators
        && action.after.deck.status_payoffs == 0
        && parent
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::RunicPyramid);
    let added_deck_shape_risks = match &acquisition {
        ShopPolicyAcquisitionV1::Card { card, .. } => {
            deck_shape_candidate_delta_v1(deck_shape, *card).risks
        }
        _ => Vec::new(),
    };
    let introduces_package_debt = matches!(
        &acquisition,
        ShopPolicyAcquisitionV1::Card {
            component_signals,
            ..
        } if component_signals
            .debt_signals
            .iter()
            .any(|signal| is_unresolved_package_payoff_debt_signal_v1(*signal))
    );
    let redundant_upgrade_access = matches!(
        acquisition,
        ShopPolicyAcquisitionV1::Card {
            card: CardId::Armaments | CardId::Apotheosis,
            ..
        }
    ) && upgrade_scope_before.is_some()
        && upgrade_scope_after == upgrade_scope_before;
    let purge_target_loss = match &candidate_key {
        DecisionCandidateKey::ShopPurgeCard { deck_index, .. } => {
            purge_target_losses.get(deck_index).copied()
        }
        _ => None,
    };
    let followup = followup_v1(&action.exact.session.engine_state);
    let spent_gold_before_action = parent
        .shop_visit_context()
        .is_some_and(|context| context.spent_gold_in_visit);
    let durable_asset_support = first_copy_efficient_access(&acquisition)
        || !reinforced_threat_capabilities.is_empty()
        || !matched_consumable_capabilities.is_empty()
        || !added_formation_strengths.is_empty()
        || upgrade_scope_after > upgrade_scope_before
        || strategic_acquisition_supported(parent, &acquisition, followup);
    let opens_only_unclaimable_potion_rewards =
        super::reward_auto::reward_surface_has_only_unclaimable_potions(&action.exact.session);
    let gold_spent = decision
        .before
        .resources
        .gold
        .saturating_sub(action.after.resources.gold);
    let hp_gain = action
        .after
        .resources
        .current_hp
        .saturating_sub(decision.before.resources.current_hp);
    let max_hp_gain = action
        .after
        .resources
        .max_hp
        .saturating_sub(decision.before.resources.max_hp);
    let deck_size_delta =
        action.after.deck.deck_size as isize - decision.before.deck.deck_size as isize;
    let band = shop_policy_band_v1(
        parent,
        &acquisition,
        followup,
        opens_only_unclaimable_potion_rewards,
        hp_gain,
        max_hp_gain,
        deck_size_delta,
        &closed_threat_gaps,
        &reinforced_threat_capabilities,
        &resolved_formation_needs,
        &added_formation_strengths,
        &matched_consumable_capabilities,
        upgrade_scope_before,
        upgrade_scope_after,
        introduces_status_burden,
        !added_deck_shape_risks.is_empty(),
        introduces_package_debt,
        redundant_upgrade_access,
        card_acquisition_policy,
        card_acquisition_opportunity_cost,
        spent_gold_before_action,
        durable_asset_support,
        purge_target_loss,
    );

    Ok(ShopPolicyActionEvidenceV1 {
        candidate_id: action.candidate_id.clone(),
        candidate_key,
        acquisition,
        band,
        followup,
        gold_spent,
        hp_gain,
        max_hp_gain,
        deck_size_delta,
        closed_threat_gaps,
        capability_improvements,
        reinforced_threat_capabilities,
        resolved_formation_needs,
        added_formation_strengths,
        matched_consumable_capabilities,
        upgrade_scope_before,
        upgrade_scope_after,
        introduces_status_burden,
        added_deck_shape_risks,
        introduces_package_debt,
        redundant_upgrade_access,
        card_acquisition_policy,
        card_acquisition_opportunity_cost,
        spent_gold_before_action,
        durable_asset_support,
        purge_target_loss,
        surface_index,
    })
}

fn card_acquisition_policy_v1(
    parent: &RunControlSession,
    candidate_key: &DecisionCandidateKey,
    acquisition: &ShopPolicyAcquisitionV1,
    deck_plan: DeckPlanSnapshot,
) -> (
    Option<AcquisitionPolicyDecision>,
    Option<AcquisitionOpportunityCost>,
) {
    let (
        DecisionCandidateKey::ShopBuyCard { price, .. },
        ShopPolicyAcquisitionV1::Card { card, upgrades, .. },
    ) = (candidate_key, acquisition)
    else {
        return (None, None);
    };
    let purge_reserve = match &parent.engine_state {
        EngineState::Shop(shop)
            if shop.purge_available && parent.run_state.gold >= shop.purge_cost =>
        {
            Some(shop.purge_cost)
        }
        _ => None,
    };
    let admission =
        assess_reward_admission_from_master_deck(&parent.run_state.master_deck, *card, *upgrades);
    let report = assess_card_acquisition(
        AcquisitionContext::shop_with_purge_reserve(
            deck_plan,
            parent.run_state.gold,
            *price,
            purge_reserve,
        ),
        *card,
        *upgrades,
        &admission,
    );
    (
        Some(evaluate_deck_construction_contract(&report)),
        Some(report.opportunity_cost),
    )
}

fn acquisition_v1(
    parent: &RunControlSession,
    key: &DecisionCandidateKey,
    formation_needs: &[StrategyDeckFormationNeedV1],
    startup: &DeckStartupProfileV1,
    block_plan: &BlockPlanProfileV1,
) -> Result<ShopPolicyAcquisitionV1, String> {
    Ok(match key {
        DecisionCandidateKey::ShopBuyCard { card, upgrades, .. } => {
            let copies_before = parent
                .run_state
                .master_deck
                .iter()
                .filter(|owned| owned.id == *card)
                .count();
            let reward = RewardCard::new(*card, *upgrades);
            let semantics = card_reward_semantic_profile_v1(&reward);
            ShopPolicyAcquisitionV1::Card {
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
        DecisionCandidateKey::ShopBuyRelic { relic, .. } => {
            let requirements = relic_acquisition_requirements_v1(*relic);
            let requirement_support =
                acquisition_requirements_support_v1(parent, requirements.as_slice());
            ShopPolicyAcquisitionV1::Relic {
                relic: *relic,
                traits: relic_acquisition_traits_v1(*relic),
                requirements,
                requirement_support,
            }
        }
        DecisionCandidateKey::ShopBuyPotion { potion, .. } => {
            let requirements = potion_acquisition_requirements_v1(*potion);
            let requirement_support =
                acquisition_requirements_support_v1(parent, requirements.as_slice());
            ShopPolicyAcquisitionV1::Potion {
                potion: *potion,
                traits: potion_acquisition_traits_v1(*potion),
                requirements,
                requirement_support,
            }
        }
        DecisionCandidateKey::ShopPurgeCard { card, upgrades, .. } => {
            ShopPolicyAcquisitionV1::Purge {
                card: *card,
                upgrades: *upgrades,
            }
        }
        DecisionCandidateKey::ShopOpenRewards => ShopPolicyAcquisitionV1::OpenRewards,
        DecisionCandidateKey::ShopLeave => ShopPolicyAcquisitionV1::Leave,
        other => {
            return Err(format!(
                "exact shop policy received non-shop candidate key {other:?}"
            ))
        }
    })
}

fn acquisition_requirements_support_v1(
    parent: &RunControlSession,
    requirements: &[AcquisitionRequirementV1],
) -> AcquisitionRequirementSupportV1 {
    let mut aggregate = AcquisitionRequirementSupportV1::Current;
    for requirement in requirements {
        match acquisition_requirement_support_v1(parent, *requirement) {
            AcquisitionRequirementSupportV1::Unavailable => {
                return AcquisitionRequirementSupportV1::Unavailable;
            }
            support @ AcquisitionRequirementSupportV1::OwnedUpgradePath { .. } => {
                if aggregate == AcquisitionRequirementSupportV1::Current {
                    aggregate = support;
                }
            }
            AcquisitionRequirementSupportV1::Current => {}
        }
    }
    aggregate
}

fn acquisition_requirement_support_v1(
    parent: &RunControlSession,
    requirement: AcquisitionRequirementV1,
) -> AcquisitionRequirementSupportV1 {
    let satisfied = match requirement {
        AcquisitionRequirementV1::XCostPayoff => parent
            .run_state
            .master_deck
            .iter()
            .any(|card| get_card_definition(card.id).cost == -1),
        AcquisitionRequirementV1::DuplicateTarget => !parent.run_state.master_deck.is_empty(),
        AcquisitionRequirementV1::AttackSkillPowerSameTurn => {
            return attack_skill_power_activation_support_v1(&parent.run_state);
        }
        AcquisitionRequirementV1::LowHpDeathInsurance => {
            parent.run_state.current_hp.saturating_mul(2) <= parent.run_state.max_hp
        }
        AcquisitionRequirementV1::RouteEscapeValue => route_escape_value_v1(parent),
    };
    if satisfied {
        AcquisitionRequirementSupportV1::Current
    } else {
        AcquisitionRequirementSupportV1::Unavailable
    }
}

fn attack_skill_power_activation_support_v1(
    run_state: &RunState,
) -> AcquisitionRequirementSupportV1 {
    let energy = 3_i32.saturating_add(
        run_state
            .relics
            .iter()
            .map(|relic| i32::from(energy_master_delta(relic.id)))
            .sum(),
    );
    if attack_skill_power_sequence_exists_v1(&run_state.master_deck, energy) {
        return AcquisitionRequirementSupportV1::Current;
    }

    for index in 0..run_state.master_deck.len() {
        let mut upgraded = run_state.master_deck.clone();
        let energy_cost_before = upgraded[index].combat_cost_without_turn_override_java();
        let energy_gain_before = activation_card_energy_gain_v1(&upgraded[index]);
        if !upgrade_card_once_java(&mut upgraded[index]) {
            continue;
        }
        let energy_cost_after = upgraded[index].combat_cost_without_turn_override_java();
        let energy_gain_after = activation_card_energy_gain_v1(&upgraded[index]);
        if energy_cost_before == energy_cost_after && energy_gain_before == energy_gain_after {
            continue;
        }
        if attack_skill_power_sequence_exists_v1(&upgraded, energy) {
            return AcquisitionRequirementSupportV1::OwnedUpgradePath {
                card: upgraded[index].id,
                uuid: upgraded[index].uuid,
                upgrades_before: upgraded[index].upgrades.saturating_sub(1),
                energy_cost_before,
                energy_cost_after,
                energy_gain_before,
                energy_gain_after,
            };
        }
    }
    AcquisitionRequirementSupportV1::Unavailable
}

#[derive(Clone, Copy)]
struct ActivationCardV1 {
    card_type: CardType,
    energy_cost: i32,
    energy_gain: i32,
}

fn attack_skill_power_sequence_exists_v1(deck: &[CombatCard], starting_energy: i32) -> bool {
    let cards = deck
        .iter()
        .filter_map(|card| {
            let card_type = get_card_definition(card.id).card_type;
            if !matches!(
                card_type,
                CardType::Attack | CardType::Skill | CardType::Power
            ) {
                return None;
            }
            Some(ActivationCardV1 {
                card_type,
                energy_cost: card.combat_cost_without_turn_override_java(),
                energy_gain: activation_card_energy_gain_v1(card),
            })
        })
        .collect::<Vec<_>>();
    let attacks = cards
        .iter()
        .copied()
        .filter(|card| card.card_type == CardType::Attack)
        .collect::<Vec<_>>();
    let skills = cards
        .iter()
        .copied()
        .filter(|card| card.card_type == CardType::Skill)
        .collect::<Vec<_>>();
    let powers = cards
        .iter()
        .copied()
        .filter(|card| card.card_type == CardType::Power)
        .collect::<Vec<_>>();

    attacks.iter().copied().any(|attack| {
        skills.iter().copied().any(|skill| {
            powers.iter().copied().any(|power| {
                let cards = [attack, skill, power];
                [
                    [0, 1, 2],
                    [0, 2, 1],
                    [1, 0, 2],
                    [1, 2, 0],
                    [2, 0, 1],
                    [2, 1, 0],
                ]
                .into_iter()
                .any(|order| activation_sequence_is_payable_v1(cards, order, starting_energy))
            })
        })
    })
}

fn activation_card_energy_gain_v1(card: &CombatCard) -> i32 {
    card_reward_facts_v1(&RewardCard::new(card.id, card.upgrades)).energy_gain
}

fn activation_sequence_is_payable_v1(
    cards: [ActivationCardV1; 3],
    order: [usize; 3],
    starting_energy: i32,
) -> bool {
    let mut energy = starting_energy;
    for index in order {
        let card = cards[index];
        if card.energy_cost < 0 {
            energy = 0;
        } else {
            if card.energy_cost > energy {
                return false;
            }
            energy = energy
                .saturating_sub(card.energy_cost)
                .saturating_add(card.energy_gain);
        }
    }
    true
}

fn route_escape_value_v1(parent: &RunControlSession) -> bool {
    route_window_targets(&parent.run_state)
        .into_iter()
        .any(|target| {
            let family = build_route_path_family_from_target(
                &parent.run_state,
                target.x,
                target.y,
                RouteWindowFactsConfig {
                    horizon_nodes: 5,
                    path_budget: 2_000,
                },
            );
            family.paths.iter().any(|path| {
                path.nodes.iter().any(|node| {
                    matches!(
                        node.room_type,
                        Some(
                            crate::state::map::node::RoomType::MonsterRoom
                                | crate::state::map::node::RoomType::MonsterRoomElite
                        )
                    )
                })
            })
        })
}

fn matched_consumable_capabilities_v1(
    before: &super::RunPolicyStateEvidenceV1,
    acquisition: &ShopPolicyAcquisitionV1,
) -> Vec<StrategyCapabilityKindV1> {
    let ShopPolicyAcquisitionV1::Potion { traits, .. } = acquisition else {
        return Vec::new();
    };
    let required = before
        .threat_coverage
        .gaps
        .iter()
        .flat_map(|gap| gap.required_capabilities.iter().copied())
        .collect::<BTreeSet<_>>();
    let supplied = traits
        .iter()
        .flat_map(|trait_| potion_capabilities(*trait_))
        .collect::<BTreeSet<_>>();
    required.intersection(&supplied).copied().collect()
}

fn potion_capabilities(trait_: PotionAcquisitionTraitV1) -> Vec<StrategyCapabilityKindV1> {
    use PotionAcquisitionTraitV1 as Trait;
    use StrategyCapabilityKindV1 as Capability;
    match trait_ {
        Trait::CombatDamage => vec![Capability::SingleTargetFrontload],
        Trait::AoeDamage => vec![Capability::MultiTargetControl],
        Trait::CombatBlock | Trait::WeakControl => vec![Capability::SustainedDefense],
        Trait::VulnerableSetup | Trait::StrengthGain => {
            vec![
                Capability::SingleTargetFrontload,
                Capability::LongFightScaling,
            ]
        }
        Trait::EnergyBurst | Trait::CardAccess | Trait::ActionAmplifier => {
            vec![Capability::DrawEnergyConsistency]
        }
        Trait::CardDiscovery => Vec::new(),
        Trait::DeathInsurance | Trait::EscapeTool => vec![Capability::SustainedDefense],
        Trait::DebuffControl => vec![Capability::DebuffResilience],
    }
}

#[allow(clippy::too_many_arguments)]
fn shop_policy_band_v1(
    parent: &RunControlSession,
    acquisition: &ShopPolicyAcquisitionV1,
    followup: ShopPolicyFollowupV1,
    opens_only_unclaimable_potion_rewards: bool,
    hp_gain: i32,
    max_hp_gain: i32,
    deck_size_delta: isize,
    closed_threat_gaps: &[ShopPolicyThreatGapKeyV1],
    reinforced_threat_capabilities: &[StrategyCapabilityKindV1],
    resolved_formation_needs: &[StrategyDeckFormationNeedV1],
    added_formation_strengths: &[StrategyPackageIdV2],
    matched_consumable_capabilities: &[StrategyCapabilityKindV1],
    upgrade_scope_before: Option<CombatUpgradeScopeV1>,
    upgrade_scope_after: Option<CombatUpgradeScopeV1>,
    introduces_status_burden: bool,
    introduces_deck_shape_risk: bool,
    introduces_package_debt: bool,
    redundant_upgrade_access: bool,
    card_acquisition_policy: Option<AcquisitionPolicyDecision>,
    card_acquisition_opportunity_cost: Option<AcquisitionOpportunityCost>,
    spent_gold_before_action: bool,
    durable_asset_support: bool,
    purge_target_loss: Option<DeckMutationTargetLossTierV1>,
) -> ShopPolicyBandV1 {
    if opens_only_unclaimable_potion_rewards {
        return ShopPolicyBandV1::Liability;
    }
    if matches!(acquisition, ShopPolicyAcquisitionV1::OpenRewards) {
        return if pending_shop_rewards_are_actionable(parent) {
            ShopPolicyBandV1::ResolvePendingBoundary
        } else {
            ShopPolicyBandV1::Liability
        };
    }
    if hp_gain > 0 || max_hp_gain > 0 {
        return ShopPolicyBandV1::ImmediateSurvival;
    }
    if introduces_status_burden
        || introduces_deck_shape_risk
        || introduces_package_debt
        || redundant_upgrade_access
        || matches!(
            card_acquisition_policy,
            Some(AcquisitionPolicyDecision {
                verdict: AcquisitionPolicyVerdict::Reject,
                reason,
            }) if reason != AcquisitionPolicyReason::NoPolicySupport
        )
    {
        return ShopPolicyBandV1::Liability;
    }
    if matches!(
        card_acquisition_policy.map(|policy| policy.verdict),
        Some(AcquisitionPolicyVerdict::SkipPreferred)
    ) {
        return ShopPolicyBandV1::SpeculativePurchase;
    }
    if card_acquisition_opportunity_cost == Some(AcquisitionOpportunityCost::SpendsPurgeReserve)
        && card_acquisition_policy.is_some_and(|policy| !policy.allows_acquisition())
        && !durable_asset_support
    {
        return ShopPolicyBandV1::SpeculativePurchase;
    }
    if spent_gold_before_action
        && matches!(
            acquisition,
            ShopPolicyAcquisitionV1::Card { .. } | ShopPolicyAcquisitionV1::Potion { .. }
        )
        && !card_acquisition_policy.is_some_and(AcquisitionPolicyDecision::allows_acquisition)
        && !durable_asset_support
    {
        return ShopPolicyBandV1::SpeculativePurchase;
    }
    if !closed_threat_gaps.is_empty() {
        return ShopPolicyBandV1::CloseThreatGap;
    }
    if first_copy_efficient_access(acquisition) {
        return ShopPolicyBandV1::AmplifyStrategicAccess;
    }
    if !reinforced_threat_capabilities.is_empty() || !matched_consumable_capabilities.is_empty() {
        return ShopPolicyBandV1::ImproveRequiredCapability;
    }
    if matches!(acquisition, ShopPolicyAcquisitionV1::Purge { .. }) {
        return if deck_size_delta < 0
            && matches!(
                purge_target_loss,
                Some(
                    DeckMutationTargetLossTierV1::LowValue
                        | DeckMutationTargetLossTierV1::RedundantFunctional
                )
            ) {
            ShopPolicyBandV1::DeckRepair
        } else {
            ShopPolicyBandV1::Liability
        };
    }
    if !resolved_formation_needs.is_empty()
        || !added_formation_strengths.is_empty()
        || upgrade_scope_after > upgrade_scope_before
        || strategic_acquisition_supported(parent, acquisition, followup)
    {
        return ShopPolicyBandV1::EstablishStrategicAsset;
    }
    if matches!(acquisition, ShopPolicyAcquisitionV1::Leave) {
        return ShopPolicyBandV1::PreserveResources;
    }
    if matches!(
        card_acquisition_policy.map(|policy| policy.verdict),
        Some(AcquisitionPolicyVerdict::Reject)
    ) {
        return ShopPolicyBandV1::Liability;
    }
    ShopPolicyBandV1::SpeculativePurchase
}

fn pending_shop_rewards_are_actionable(parent: &RunControlSession) -> bool {
    let EngineState::Shop(shop) = &parent.engine_state else {
        return false;
    };
    let Some(reward) = shop.pending_reward_overlay.as_ref() else {
        return false;
    };
    !super::reward_auto::reward_state_has_only_unclaimable_potions(&parent.run_state, reward)
}

fn first_copy_efficient_access(acquisition: &ShopPolicyAcquisitionV1) -> bool {
    matches!(
        acquisition,
        ShopPolicyAcquisitionV1::Card {
            copies_before: 0,
            access: Some(CardAccessEvidenceV1 {
                leverage: CardAccessLeverageV1::EfficientBurst,
                ..
            }),
            ..
        }
    )
}

fn strategic_acquisition_supported(
    parent: &RunControlSession,
    acquisition: &ShopPolicyAcquisitionV1,
    followup: ShopPolicyFollowupV1,
) -> bool {
    match acquisition {
        ShopPolicyAcquisitionV1::Card {
            card,
            semantics,
            copies_before,
            ..
        } => {
            *copies_before == 0
                && semantics.roles.iter().any(|role| {
                    if *role == CardRewardSemanticRoleV1::CombatExternalPayoff
                        && matches!(card, CardId::HandOfGreed | CardId::Wish)
                    {
                        return gold_payoff_has_conversion_window(parent);
                    }
                    matches!(
                        *role,
                        CardRewardSemanticRoleV1::CardDraw
                            | CardRewardSemanticRoleV1::CycleAccess
                            | CardRewardSemanticRoleV1::EnergySource
                            | CardRewardSemanticRoleV1::ScalingSource
                            | CardRewardSemanticRoleV1::EnemyStrengthDown
                            | CardRewardSemanticRoleV1::ExhaustGenerator
                            | CardRewardSemanticRoleV1::ExhaustPayoff
                            | CardRewardSemanticRoleV1::StatusPayoff
                            | CardRewardSemanticRoleV1::BlockRetention
                            | CardRewardSemanticRoleV1::BlockMultiplier
                            | CardRewardSemanticRoleV1::CombatExternalPayoff
                            | CardRewardSemanticRoleV1::CombatSustain
                    )
                })
        }
        ShopPolicyAcquisitionV1::Relic {
            traits,
            requirement_support,
            ..
        } => {
            *requirement_support == AcquisitionRequirementSupportV1::Current
                && traits.iter().any(|trait_| match trait_ {
                    RelicAcquisitionTraitV1::EliteFightLeverage => {
                        parent.run_state.peek_next_elite().is_some()
                    }
                    _ => true,
                })
                || matches!(
                    followup,
                    ShopPolicyFollowupV1::Reward | ShopPolicyFollowupV1::Selection
                )
        }
        ShopPolicyAcquisitionV1::Potion {
            requirements,
            requirement_support,
            ..
        } => {
            !requirements.is_empty()
                && *requirement_support == AcquisitionRequirementSupportV1::Current
        }
        ShopPolicyAcquisitionV1::Purge { .. }
        | ShopPolicyAcquisitionV1::OpenRewards
        | ShopPolicyAcquisitionV1::Leave => false,
    }
}

fn gold_payoff_has_conversion_window(parent: &RunControlSession) -> bool {
    // Gold survives the first two acts, so a later act can still convert it.
    // In Act 3 and beyond, require a visible non-Boss payoff combat followed
    // by another shop; gold earned on the Boss kill cannot improve this run.
    if parent.run_state.act_num < 3 {
        return true;
    }

    route_window_targets(&parent.run_state)
        .into_iter()
        .any(|target| {
            let family = build_route_path_family_from_target(
                &parent.run_state,
                target.x,
                target.y,
                RouteWindowFactsConfig {
                    horizon_nodes: 15,
                    path_budget: 2_000,
                },
            );
            family.paths.iter().any(|path| {
                let mut payoff_combat_seen = false;
                path.nodes.iter().any(|node| match node.room_type {
                    Some(
                        crate::state::map::node::RoomType::MonsterRoom
                        | crate::state::map::node::RoomType::MonsterRoomElite,
                    ) => {
                        payoff_combat_seen = true;
                        false
                    }
                    Some(crate::state::map::node::RoomType::ShopRoom) => payoff_combat_seen,
                    _ => false,
                })
            })
        })
}

fn followup_v1(engine_state: &EngineState) -> ShopPolicyFollowupV1 {
    match engine_state {
        EngineState::Shop(_) => ShopPolicyFollowupV1::Shop,
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            ShopPolicyFollowupV1::Reward
        }
        EngineState::RunPendingChoice(_) => ShopPolicyFollowupV1::Selection,
        EngineState::MapNavigation | EngineState::MapOverlay { .. } => ShopPolicyFollowupV1::Map,
        _ => ShopPolicyFollowupV1::Other,
    }
}

fn compare_shop_evidence(
    left: &ShopPolicyActionEvidenceV1,
    right: &ShopPolicyActionEvidenceV1,
) -> Ordering {
    shop_band_priority(left.band)
        .cmp(&shop_band_priority(right.band))
        .then_with(|| compare_blocked_card_acquisition_policy(left, right))
        .then_with(|| {
            right
                .closed_threat_gaps
                .len()
                .cmp(&left.closed_threat_gaps.len())
        })
        .then_with(|| {
            right
                .capability_improvements
                .len()
                .cmp(&left.capability_improvements.len())
        })
        .then_with(|| right.hp_gain.cmp(&left.hp_gain))
        .then_with(|| right.max_hp_gain.cmp(&left.max_hp_gain))
        .then_with(|| {
            right
                .resolved_formation_needs
                .len()
                .cmp(&left.resolved_formation_needs.len())
        })
        .then_with(|| {
            right
                .added_formation_strengths
                .len()
                .cmp(&left.added_formation_strengths.len())
        })
        .then_with(|| match (left.purge_target_loss, right.purge_target_loss) {
            (Some(left), Some(right)) => left.cmp(&right),
            _ => Ordering::Equal,
        })
        .then_with(|| left.gold_spent.cmp(&right.gold_spent))
        .then_with(|| left.surface_index.cmp(&right.surface_index))
}

fn compare_blocked_card_acquisition_policy(
    left: &ShopPolicyActionEvidenceV1,
    right: &ShopPolicyActionEvidenceV1,
) -> Ordering {
    match (
        left.card_acquisition_policy.map(|policy| policy.verdict),
        right.card_acquisition_policy.map(|policy| policy.verdict),
    ) {
        (Some(AcquisitionPolicyVerdict::SkipPreferred), Some(AcquisitionPolicyVerdict::Reject)) => {
            Ordering::Less
        }
        (Some(AcquisitionPolicyVerdict::Reject), Some(AcquisitionPolicyVerdict::SkipPreferred)) => {
            Ordering::Greater
        }
        _ => Ordering::Equal,
    }
}

const fn shop_band_priority(band: ShopPolicyBandV1) -> u8 {
    match band {
        ShopPolicyBandV1::ResolvePendingBoundary => 0,
        ShopPolicyBandV1::ImmediateSurvival => 1,
        ShopPolicyBandV1::CloseThreatGap => 2,
        ShopPolicyBandV1::AmplifyStrategicAccess => 3,
        ShopPolicyBandV1::ImproveRequiredCapability => 4,
        // Both are durable deck improvements. Their exact evidence and
        // opportunity cost must compare before the owner commits all gold to
        // one category merely because it was represented by a different verb.
        ShopPolicyBandV1::DeckRepair | ShopPolicyBandV1::EstablishStrategicAsset => 5,
        ShopPolicyBandV1::PreserveResources => 6,
        ShopPolicyBandV1::SpeculativePurchase => 7,
        ShopPolicyBandV1::Liability => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::factory::EncounterId;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::relics::RelicState;
    use crate::eval::run_control::{build_decision_surface, RunControlConfig, ShopVisitContextV1};
    use crate::runtime::combat::CombatCard;
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use crate::state::rewards::{RewardItem, RewardState};
    use crate::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};

    fn policy_candidates<'a>(
        surface: &'a super::super::DecisionSurface,
    ) -> Vec<RunPolicyCandidateV1<'a>> {
        surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
                Some(RunPolicyCandidateV1 {
                    candidate_id: &candidate.id,
                    label: &candidate.label,
                    action: candidate.action.executable_action_ref()?,
                })
            })
            .collect()
    }

    fn candidate_band(decision: &ExactShopPolicyDecisionV1, card: CardId) -> ShopPolicyBandV1 {
        decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: candidate_card,
                        ..
                    } if candidate_card == card
                )
            })
            .expect("card evidence")
            .band
    }

    fn candidate_position(decision: &ExactShopPolicyDecisionV1, card: CardId) -> usize {
        decision
            .evidence
            .iter()
            .position(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: candidate_card,
                        ..
                    } if candidate_card == card
                )
            })
            .expect("card position")
    }

    fn relic_candidate(
        decision: &ExactShopPolicyDecisionV1,
        relic: RelicId,
    ) -> &ShopPolicyActionEvidenceV1 {
        decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyRelic {
                        relic: candidate_relic,
                        ..
                    } if candidate_relic == relic
                )
            })
            .expect("relic evidence")
    }

    fn orange_pellets_decision(dark_embrace_upgrades: u8) -> ExactShopPolicyDecisionV1 {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 18;
        session.run_state.current_hp = 71;
        session.run_state.max_hp = 85;
        session.run_state.gold = 175;
        session.run_state.relics = vec![
            RelicState::new(RelicId::BurningBlood),
            RelicState::new(RelicId::PenNib),
            RelicState::new(RelicId::RunicCube),
        ];
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
            (CardId::PowerThrough, 1),
            (CardId::HeavyBlade, 0),
            (CardId::SecondWind, 1),
            (CardId::DarkEmbrace, dark_embrace_upgrades),
            (CardId::ThunderClap, 0),
            (CardId::ShrugItOff, 0),
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
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::OrangePellets,
            price: 145,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        exact_shop_policy_decision_v1(&session, &legal).expect("Orange Pellets policy")
    }

    fn cauldron_decision_with_potions(potions: Vec<Option<Potion>>) -> ExactShopPolicyDecisionV1 {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 300;
        session.run_state.potions = potions;
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Cauldron,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        exact_shop_policy_decision_v1(&session, &legal).expect("Cauldron policy")
    }

    #[test]
    fn orange_pellets_upgrade_path_is_visible_but_does_not_spend_gold_as_current_support() {
        let decision = orange_pellets_decision(0);
        let pellets = relic_candidate(&decision, RelicId::OrangePellets);

        assert!(matches!(
            pellets.acquisition,
            ShopPolicyAcquisitionV1::Relic {
                requirement_support: AcquisitionRequirementSupportV1::OwnedUpgradePath {
                    card: CardId::DarkEmbrace,
                    upgrades_before: 0,
                    energy_cost_before: 2,
                    energy_cost_after: 1,
                    energy_gain_before: 0,
                    energy_gain_after: 0,
                    ..
                },
                ..
            }
        ));
        assert_eq!(pellets.band, ShopPolicyBandV1::SpeculativePurchase);
        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopLeave
        ));
    }

    #[test]
    fn orange_pellets_current_three_card_line_is_a_supported_asset() {
        let decision = orange_pellets_decision(1);
        let pellets = relic_candidate(&decision, RelicId::OrangePellets);

        assert!(matches!(
            pellets.acquisition,
            ShopPolicyAcquisitionV1::Relic {
                requirement_support: AcquisitionRequirementSupportV1::Current,
                ..
            }
        ));
        assert_eq!(pellets.band, ShopPolicyBandV1::EstablishStrategicAsset);
        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopBuyRelic {
                relic: RelicId::OrangePellets,
                ..
            }
        ));
    }

    #[test]
    fn orange_pellets_without_an_owned_power_reports_unavailable() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];

        assert_eq!(
            attack_skill_power_activation_support_v1(&run_state),
            AcquisitionRequirementSupportV1::Unavailable
        );
    }

    #[test]
    fn orange_pellets_accepts_an_unconditional_energy_gain_sequence() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::SeeingRed, 2),
            CombatCard::new(CardId::DemonForm, 3),
        ];

        assert_eq!(
            attack_skill_power_activation_support_v1(&run_state),
            AcquisitionRequirementSupportV1::Current
        );
    }

    #[test]
    fn full_inventory_pending_potions_do_not_reopen_before_leaving_shop() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 16;
        session.run_state.potions = vec![
            Some(Potion::new(PotionId::AttackPotion, 1)),
            Some(Potion::new(PotionId::SpeedPotion, 2)),
            Some(Potion::new(PotionId::EssenceOfSteel, 3)),
        ];
        let mut pending = RewardState::new();
        pending.items.push(RewardItem::Potion {
            potion_id: PotionId::LiquidBronze,
        });
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.pending_reward_overlay = Some(pending);
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_shop_policy_decision_v1(&session, &legal).expect("pending potion shop policy");

        assert!(
            matches!(
                decision.evidence[0].candidate_key,
                DecisionCandidateKey::ShopLeave
            ),
            "evidence={:#?}",
            decision.evidence
        );
        assert_eq!(
            decision
                .evidence
                .iter()
                .find(|candidate| {
                    matches!(
                        candidate.candidate_key,
                        DecisionCandidateKey::ShopOpenRewards
                    )
                })
                .expect("open rewards evidence")
                .band,
            ShopPolicyBandV1::Liability
        );
    }

    #[test]
    fn pending_potion_with_empty_slot_reopens_before_leaving_shop() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 16;
        let mut pending = RewardState::new();
        pending.items.push(RewardItem::Potion {
            potion_id: PotionId::LiquidBronze,
        });
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.pending_reward_overlay = Some(pending);
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_shop_policy_decision_v1(&session, &legal).expect("pending potion shop policy");

        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopOpenRewards
        ));
        assert_eq!(
            decision.evidence[0].band,
            ShopPolicyBandV1::ResolvePendingBoundary
        );
    }

    #[test]
    fn full_inventory_cauldron_is_a_liability_without_a_replacement_policy() {
        let decision = cauldron_decision_with_potions(vec![
            Some(Potion::new(PotionId::AttackPotion, 1)),
            Some(Potion::new(PotionId::SpeedPotion, 2)),
            Some(Potion::new(PotionId::EssenceOfSteel, 3)),
        ]);
        let cauldron = relic_candidate(&decision, RelicId::Cauldron);

        assert_eq!(cauldron.followup, ShopPolicyFollowupV1::Reward);
        assert_eq!(cauldron.band, ShopPolicyBandV1::Liability);
        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopLeave
        ));
    }

    #[test]
    fn cauldron_remains_strategic_when_a_potion_slot_is_available() {
        let decision = cauldron_decision_with_potions(vec![
            Some(Potion::new(PotionId::AttackPotion, 1)),
            Some(Potion::new(PotionId::SpeedPotion, 2)),
            None,
        ]);
        let cauldron = relic_candidate(&decision, RelicId::Cauldron);

        assert_eq!(cauldron.band, ShopPolicyBandV1::EstablishStrategicAsset);
        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopBuyRelic {
                relic: RelicId::Cauldron,
                ..
            }
        ));
    }

    #[test]
    fn efficient_first_copy_access_precedes_marginal_frontload_improvement() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 2;
        session.run_state.floor_num = 20;
        session.run_state.boss_key = Some(EncounterId::Automaton);
        session.run_state.current_hp = 76;
        session.run_state.max_hp = 80;
        session.run_state.gold = 86;
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
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.cards.extend([
            ShopCard {
                card_id: CardId::RecklessCharge,
                upgrades: 0,
                price: 33,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::BattleTrance,
                upgrades: 0,
                price: 78,
                can_buy: true,
                blocked_reason: None,
            },
        ]);
        session.engine_state = EngineState::Shop(shop.clone());

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&session, &legal).expect("access shop policy");
        assert_eq!(
            candidate_band(&decision, CardId::BattleTrance),
            ShopPolicyBandV1::AmplifyStrategicAccess
        );
        assert!(
            candidate_position(&decision, CardId::BattleTrance)
                < candidate_position(&decision, CardId::RecklessCharge),
            "evidence={:#?}",
            decision.evidence
        );

        session
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::BattleTrance, 30_000));
        session.engine_state = EngineState::Shop(shop);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let duplicate =
            exact_shop_policy_decision_v1(&session, &legal).expect("duplicate access policy");
        assert_ne!(
            candidate_band(&duplicate, CardId::BattleTrance),
            ShopPolicyBandV1::AmplifyStrategicAccess
        );
    }

    #[test]
    fn purge_reserve_demotes_expensive_hard_gap_without_erasing_supported_shop_assets() {
        // Keep the complete A1F4 deck: starter density drives both the typed
        // strategic gaps and the still-available low-loss purge alternative.
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 4;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session.run_state.current_hp = 85;
        session.run_state.max_hp = 85;
        session.run_state.gold = 135;
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
            CardId::PowerThrough,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.cards.extend([
            ShopCard {
                card_id: CardId::Uppercut,
                upgrades: 0,
                price: 69,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::SecondWind,
                upgrades: 0,
                price: 35,
                can_buy: true,
                blocked_reason: None,
            },
        ]);
        shop.potions.push(ShopPotion {
            potion_id: PotionId::AttackPotion,
            price: 52,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&session, &legal).expect("A1F4 shop policy");
        let uppercut = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: CardId::Uppercut,
                        ..
                    }
                )
            })
            .expect("Uppercut evidence");

        assert_eq!(
            uppercut.card_acquisition_policy,
            Some(AcquisitionPolicyDecision {
                verdict: AcquisitionPolicyVerdict::Reject,
                reason: AcquisitionPolicyReason::NoPolicySupport,
            })
        );
        assert_eq!(
            uppercut.card_acquisition_opportunity_cost,
            Some(AcquisitionOpportunityCost::SpendsPurgeReserve)
        );
        assert_eq!(uppercut.band, ShopPolicyBandV1::SpeculativePurchase);
        assert_eq!(
            candidate_band(&decision, CardId::SecondWind),
            ShopPolicyBandV1::EstablishStrategicAsset
        );
        let attack_potion_position = decision
            .evidence
            .iter()
            .position(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyPotion {
                        potion: PotionId::AttackPotion,
                        ..
                    }
                )
            })
            .expect("Attack Potion position");
        let attack_potion = &decision.evidence[attack_potion_position];
        assert!(matches!(
            &attack_potion.acquisition,
            ShopPolicyAcquisitionV1::Potion { traits, .. }
                if traits.contains(&PotionAcquisitionTraitV1::CardDiscovery)
                    && !traits.contains(&PotionAcquisitionTraitV1::CardAccess)
        ));
        assert!(attack_potion.matched_consumable_capabilities.is_empty());
        assert_eq!(attack_potion.band, ShopPolicyBandV1::SpeculativePurchase);
        assert!(
            candidate_position(&decision, CardId::SecondWind)
                < candidate_position(&decision, CardId::Uppercut),
            "evidence={:#?}",
            decision.evidence
        );
        assert!(candidate_position(&decision, CardId::SecondWind) < attack_potion_position);
        assert!(
            decision.evidence.iter().position(|candidate| matches!(
                candidate.candidate_key,
                DecisionCandidateKey::ShopLeave
            )) < Some(candidate_position(&decision, CardId::Uppercut))
        );
    }

    #[test]
    fn skip_preferred_hard_gap_precedes_rejected_speculative_cards() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 4;
        session.run_state.boss_key = Some(EncounterId::SlimeBoss);
        session.run_state.current_hp = 85;
        session.run_state.max_hp = 85;
        session.run_state.gold = 100;
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
            CardId::PowerThrough,
            CardId::SecondWind,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.cards.extend([
            ShopCard {
                card_id: CardId::Uppercut,
                upgrades: 0,
                price: 69,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::WildStrike,
                upgrades: 0,
                price: 47,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::Armaments,
                upgrades: 0,
                price: 50,
                can_buy: true,
                blocked_reason: None,
            },
        ]);
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&session, &legal)
            .expect("post-Second Wind A1F4 shop policy");
        let armaments = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: CardId::Armaments,
                        ..
                    }
                )
            })
            .expect("Armaments evidence");

        assert_eq!(
            armaments.card_acquisition_policy,
            Some(AcquisitionPolicyDecision {
                verdict: AcquisitionPolicyVerdict::SkipPreferred,
                reason: AcquisitionPolicyReason::PurgeReserveBlocksHardGap,
            })
        );
        assert_eq!(armaments.band, ShopPolicyBandV1::SpeculativePurchase);
        assert!(
            decision.evidence.iter().position(|candidate| matches!(
                candidate.candidate_key,
                DecisionCandidateKey::ShopLeave
            )) < Some(candidate_position(&decision, CardId::Armaments))
        );
        assert!(
            candidate_position(&decision, CardId::Armaments)
                < candidate_position(&decision, CardId::Uppercut)
        );
        assert!(
            candidate_position(&decision, CardId::Armaments)
                < candidate_position(&decision, CardId::WildStrike)
        );
    }

    #[test]
    fn additional_marginal_purchase_cannot_empty_a_shop_visit_after_prior_spend() {
        // Model the exact post-bundle shape: the remaining card looks locally
        // affordable only because the visit already consumed 87 of 135 gold.
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 4;
        session.run_state.gold = 48;
        session.run_state.master_deck = [
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
            CardId::PowerThrough,
            CardId::SecondWind,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.cards.push(ShopCard {
            card_id: CardId::WildStrike,
            upgrades: 0,
            price: 47,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);
        session.shop_visit_context = Some(ShopVisitContextV1 {
            entry_act: 1,
            entry_floor: 4,
            entry_gold: 135,
            maw_bank_live_at_entry: false,
            membership_card_owned_at_entry: false,
            spent_gold_in_visit: true,
        });

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&session, &legal)
            .expect("post-purchase marginal shop policy");
        let wild_strike = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: CardId::WildStrike,
                        ..
                    }
                )
            })
            .expect("Wild Strike evidence");

        assert!(wild_strike.spent_gold_before_action);
        assert!(!wild_strike.durable_asset_support);
        assert_eq!(wild_strike.band, ShopPolicyBandV1::SpeculativePurchase);
        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopLeave
        ));
    }

    #[test]
    fn partial_capability_gain_cannot_hide_card_debt_or_spend_over_leave() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.act_num = 1;
        session.run_state.floor_num = 2;
        session.run_state.boss_key = Some(EncounterId::TheGuardian);
        session.run_state.current_hp = 80;
        session.run_state.max_hp = 80;
        session.run_state.gold = 93;
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
            CardId::WildStrike,
            CardId::ShrugItOff,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();

        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.cards.extend([
            ShopCard {
                card_id: CardId::HeavyBlade,
                upgrades: 0,
                price: 49,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::Clash,
                upgrades: 0,
                price: 52,
                can_buy: true,
                blocked_reason: None,
            },
        ]);
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_shop_policy_decision_v1(&session, &legal).expect("seed006 F2 shop policy");
        let heavy = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: CardId::HeavyBlade,
                        ..
                    }
                )
            })
            .expect("Heavy Blade evidence");
        let clash = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyCard {
                        card: CardId::Clash,
                        ..
                    }
                )
            })
            .expect("Clash evidence");
        let leave = decision
            .evidence
            .iter()
            .position(|candidate| {
                matches!(candidate.candidate_key, DecisionCandidateKey::ShopLeave)
            })
            .expect("leave evidence");

        assert!(heavy.capability_improvements.iter().any(|change| {
            change.capability == StrategyCapabilityKindV1::LongFightScaling
                && change.before == StrategyCapabilityCoverageV1::Missing
                && change.after == StrategyCapabilityCoverageV1::Thin
        }));
        assert!(heavy.reinforced_threat_capabilities.is_empty());
        assert!(
            matches!(
                &heavy.acquisition,
                ShopPolicyAcquisitionV1::Card {
                    component_signals,
                    ..
                } if component_signals
                    .debt_signals
                    .iter()
                    .any(|signal| is_unresolved_package_payoff_debt_signal_v1(*signal))
            ),
            "acquisition={:#?}",
            heavy.acquisition
        );
        assert_eq!(heavy.band, ShopPolicyBandV1::Liability);
        assert!(clash
            .added_deck_shape_risks
            .iter()
            .any(|risk| matches!(risk, DeckShapeRiskV1::ClashPlayabilityDebt { .. })));
        assert_eq!(clash.band, ShopPolicyBandV1::Liability);
        assert!(leave < candidate_position(&decision, CardId::HeavyBlade));
        assert!(leave < candidate_position(&decision, CardId::Clash));
        assert!(decision
            .prior
            .entries
            .iter()
            .all(|entry| entry.probability.is_finite() && entry.probability > 0.0));
    }

    #[test]
    fn first_armaments_is_an_asset_but_redundant_scope_is_not() {
        let mut first = RunControlSession::new(RunControlConfig::default());
        first.run_state.gold = 100;
        for uuid in 20_000..20_003 {
            first
                .run_state
                .master_deck
                .push(CombatCard::new(CardId::Impervious, uuid));
        }
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.cards.push(ShopCard {
            card_id: CardId::Armaments,
            upgrades: 0,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
        first.engine_state = EngineState::Shop(shop.clone());
        let first_surface = build_decision_surface(&first);
        let first_legal = policy_candidates(&first_surface);
        let first_decision =
            exact_shop_policy_decision_v1(&first, &first_legal).expect("first Armaments policy");

        let mut duplicate = first.clone();
        duplicate
            .run_state
            .master_deck
            .push(CombatCard::new(CardId::Armaments, 10_001));
        duplicate.engine_state = EngineState::Shop(shop);
        let duplicate_surface = build_decision_surface(&duplicate);
        let duplicate_legal = policy_candidates(&duplicate_surface);
        let duplicate_decision = exact_shop_policy_decision_v1(&duplicate, &duplicate_legal)
            .expect("duplicate Armaments policy");

        assert_eq!(
            candidate_band(&first_decision, CardId::Armaments),
            ShopPolicyBandV1::EstablishStrategicAsset
        );
        assert_eq!(
            candidate_band(&duplicate_decision, CardId::Armaments),
            ShopPolicyBandV1::Liability
        );
        assert_eq!(duplicate_decision.prior.entries[0].candidate_id, "leave");
        assert!(duplicate_decision
            .prior
            .entries
            .iter()
            .all(|entry| entry.probability > 0.0));
    }

    #[test]
    fn exact_waffle_heal_precedes_generic_cleanup() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.current_hp = 30;
        session.run_state.max_hp = 80;
        session.run_state.gold = 300;
        let mut shop = ShopState::new();
        shop.purge_cost = 75;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Waffle,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);

        let decision = exact_shop_policy_decision_v1(&session, &legal).expect("Waffle shop policy");

        assert!(matches!(
            decision.evidence[0].candidate_key,
            DecisionCandidateKey::ShopBuyRelic {
                relic: RelicId::Waffle,
                ..
            }
        ));
        assert_eq!(
            decision.evidence[0].band,
            ShopPolicyBandV1::ImmediateSurvival
        );
        assert!(decision.evidence[0].hp_gain > 0);
    }

    #[test]
    fn smoke_bomb_is_a_strategic_asset_only_when_the_visible_route_has_an_escape_edge() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 100;
        for uuid in 30_000..30_003 {
            session
                .run_state
                .master_deck
                .push(CombatCard::new(CardId::Impervious, uuid));
        }
        let mut combat = MapRoomNode::new(0, 0);
        combat.class = Some(RoomType::MonsterRoom);
        combat.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut rest = MapRoomNode::new(0, 1);
        rest.class = Some(RoomType::RestRoom);
        session.run_state.map = MapState::new(vec![vec![combat], vec![rest]]);
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.potions.push(ShopPotion {
            potion_id: PotionId::SmokeBomb,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop.clone());
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_shop_policy_decision_v1(&session, &legal).expect("route escape shop policy");
        let smoke = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyPotion {
                        potion: PotionId::SmokeBomb,
                        ..
                    }
                )
            })
            .expect("Smoke Bomb evidence");
        assert!(matches!(
            smoke.band,
            ShopPolicyBandV1::ImproveRequiredCapability | ShopPolicyBandV1::EstablishStrategicAsset
        ));
        assert!(matches!(
            smoke.acquisition,
            ShopPolicyAcquisitionV1::Potion {
                requirement_support: AcquisitionRequirementSupportV1::Current,
                ..
            }
        ));

        let mut no_route = session;
        no_route.run_state.map = MapState::new(Vec::new());
        no_route.engine_state = EngineState::Shop(shop);
        let surface = build_decision_surface(&no_route);
        let legal = policy_candidates(&surface);
        let decision =
            exact_shop_policy_decision_v1(&no_route, &legal).expect("no-route shop policy");
        let smoke = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyPotion {
                        potion: PotionId::SmokeBomb,
                        ..
                    }
                )
            })
            .expect("Smoke Bomb evidence");
        assert_eq!(smoke.band, ShopPolicyBandV1::SpeculativePurchase);
        assert!(matches!(
            smoke.acquisition,
            ShopPolicyAcquisitionV1::Potion {
                requirement_support: AcquisitionRequirementSupportV1::Unavailable,
                ..
            }
        ));
    }

    #[test]
    fn orrery_is_ranked_from_its_real_nested_reward_successor() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 300;
        let mut shop = ShopState::new();
        shop.purge_available = false;
        shop.relics.push(ShopRelic {
            relic_id: RelicId::Orrery,
            price: 150,
            can_buy: true,
            blocked_reason: None,
        });
        session.engine_state = EngineState::Shop(shop);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);

        let decision = exact_shop_policy_decision_v1(&session, &legal).expect("Orrery shop policy");
        let orrery = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopBuyRelic {
                        relic: RelicId::Orrery,
                        ..
                    }
                )
            })
            .expect("Orrery evidence");

        assert_eq!(orrery.followup, ShopPolicyFollowupV1::Reward);
        assert_eq!(orrery.band, ShopPolicyBandV1::EstablishStrategicAsset);
        assert_eq!(decision.prior.entries[0].candidate_id, orrery.candidate_id);
    }

    #[test]
    fn strategic_shop_pair_competes_with_only_genuine_low_loss_repair() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.gold = 128;
        let upgraded = |card, uuid| {
            let mut card = CombatCard::new(card, uuid);
            card.upgrades = 1;
            card
        };
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Strike, 2),
            CombatCard::new(CardId::Defend, 3),
            CombatCard::new(CardId::Defend, 4),
            CombatCard::new(CardId::Bash, 5),
            upgraded(CardId::FeelNoPain, 6),
            upgraded(CardId::FeelNoPain, 7),
            upgraded(CardId::BurningPact, 8),
            upgraded(CardId::TrueGrit, 9),
            upgraded(CardId::FireBreathing, 10),
            upgraded(CardId::WildStrike, 11),
            upgraded(CardId::Whirlwind, 12),
        ];
        let mut shop = ShopState::new();
        shop.purge_cost = 125;
        shop.cards.extend([
            ShopCard {
                card_id: CardId::DarkEmbrace,
                upgrades: 1,
                price: 37,
                can_buy: true,
                blocked_reason: None,
            },
            ShopCard {
                card_id: CardId::Disarm,
                upgrades: 0,
                price: 79,
                can_buy: true,
                blocked_reason: None,
            },
        ]);
        session.engine_state = EngineState::Shop(shop);

        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&session, &legal).expect("paired shop policy");

        assert!(matches!(
            decision.evidence[0].acquisition,
            ShopPolicyAcquisitionV1::Card {
                card: CardId::Disarm,
                ..
            }
        ));
        assert!(matches!(
            decision.evidence[1].acquisition,
            ShopPolicyAcquisitionV1::Card {
                card: CardId::DarkEmbrace,
                ..
            }
        ));
        let strike_purge = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopPurgeCard { deck_index: 0, .. }
                )
            })
            .expect("starter Strike purge");
        assert_eq!(strike_purge.band, ShopPolicyBandV1::DeckRepair);
        assert_eq!(
            strike_purge.purge_target_loss,
            Some(DeckMutationTargetLossTierV1::LowValue)
        );
        let feel_no_pain_purge = decision
            .evidence
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.candidate_key,
                    DecisionCandidateKey::ShopPurgeCard {
                        card: CardId::FeelNoPain,
                        ..
                    }
                )
            })
            .expect("Feel No Pain purge");
        assert_eq!(feel_no_pain_purge.band, ShopPolicyBandV1::Liability);

        let after_disarm = decision
            .exact
            .actions
            .iter()
            .find(|action| {
                matches!(
                    action.candidate_key,
                    Some(DecisionCandidateKey::ShopBuyCard {
                        card: CardId::Disarm,
                        ..
                    })
                )
            })
            .expect("Disarm successor")
            .exact
            .session
            .clone();
        let next_surface = build_decision_surface(&after_disarm);
        let next_legal = policy_candidates(&next_surface);
        let next_decision =
            exact_shop_policy_decision_v1(&after_disarm, &next_legal).expect("post-Disarm policy");
        assert!(matches!(
            next_decision.evidence[0].acquisition,
            ShopPolicyAcquisitionV1::Card {
                card: CardId::DarkEmbrace,
                ..
            }
        ));
    }

    #[test]
    fn late_act_three_gold_payoff_requires_a_later_shop_conversion_window() {
        fn session_with_route(route: &[RoomType]) -> RunControlSession {
            let mut session = RunControlSession::new(RunControlConfig::default());
            session.run_state.act_num = 3;
            session.run_state.floor_num = 46;
            session.run_state.boss_key = Some(EncounterId::AwakenedOne);
            session.run_state.gold = 428;
            // Exact F46 production deck: Hand of Greed improves a generic
            // efficiency axis here but closes no remaining threat gap.
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
                (CardId::DemonForm, 1),
                (CardId::TrueGrit, 1),
                (CardId::DarkShackles, 0),
                (CardId::Inflame, 0),
                (CardId::FeelNoPain, 0),
            ]
            .into_iter()
            .enumerate()
            .map(|(index, (card, upgrades))| {
                let mut owned = CombatCard::new(card, index as u32);
                owned.upgrades = upgrades;
                owned
            })
            .collect();

            let rows = route
                .iter()
                .enumerate()
                .map(|(y, room_type)| {
                    let mut node = MapRoomNode::new(0, y as i32);
                    node.class = Some(*room_type);
                    if y + 1 < route.len() {
                        node.edges
                            .insert(MapEdge::new(0, y as i32, 0, y as i32 + 1));
                    }
                    vec![node]
                })
                .collect::<Vec<_>>();
            let mut map = MapState::new(rows);
            map.current_x = 0;
            map.current_y = 0;
            session.run_state.map = map;

            let mut shop = ShopState::new();
            shop.purge_available = false;
            shop.cards.push(ShopCard {
                card_id: CardId::HandOfGreed,
                upgrades: 1,
                price: 192,
                can_buy: true,
                blocked_reason: None,
            });
            session.engine_state = EngineState::Shop(shop);
            session
        }

        let no_conversion = session_with_route(&[
            RoomType::ShopRoom,
            RoomType::RestRoom,
            RoomType::MonsterRoomBoss,
        ]);
        let surface = build_decision_surface(&no_conversion);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&no_conversion, &legal)
            .expect("late Act 3 shop policy without conversion");
        assert_eq!(
            candidate_band(&decision, CardId::HandOfGreed),
            ShopPolicyBandV1::SpeculativePurchase
        );
        assert!(
            decision
                .evidence
                .iter()
                .position(|candidate| matches!(
                    candidate.acquisition,
                    ShopPolicyAcquisitionV1::Leave
                ))
                .expect("leave position")
                < candidate_position(&decision, CardId::HandOfGreed)
        );

        let convertible = session_with_route(&[
            RoomType::ShopRoom,
            RoomType::MonsterRoom,
            RoomType::ShopRoom,
            RoomType::RestRoom,
        ]);
        let surface = build_decision_surface(&convertible);
        let legal = policy_candidates(&surface);
        let decision = exact_shop_policy_decision_v1(&convertible, &legal)
            .expect("late Act 3 shop policy with conversion");
        assert_eq!(
            candidate_band(&decision, CardId::HandOfGreed),
            ShopPolicyBandV1::EstablishStrategicAsset
        );
    }
}
