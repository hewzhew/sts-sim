use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::ai::card_semantics_v1::{
    card_reward_semantic_profile_v1, potion_acquisition_requirements_v1,
    potion_acquisition_traits_v1, relic_acquisition_requirements_v1, relic_acquisition_traits_v1,
    AcquisitionRequirementV1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
    PotionAcquisitionTraitV1, RelicAcquisitionTraitV1,
};
use crate::ai::combat_upgrade_coverage_v1::CombatUpgradeScopeV1;
use crate::ai::deck_mutation_compiler_v1::{
    deck_removal_target_snapshots_v1, DeckMutationTargetLossTierV1,
};
use crate::ai::noncombat_strategy_v1::{
    StrategyCapabilityKindV1, StrategyDeckFormationNeedV1, StrategyPackageIdV2,
};
use crate::ai::route_window_facts::{
    build_route_path_family_from_target, route_window_targets, RouteWindowFactsConfig,
};
use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::rewards::RewardCard;

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

#[derive(Clone, Debug, PartialEq)]
pub enum ShopPolicyAcquisitionV1 {
    Card {
        card: CardId,
        upgrades: u8,
        copies_before: usize,
        semantics: CardRewardSemanticProfileV1,
    },
    Relic {
        relic: RelicId,
        traits: Vec<RelicAcquisitionTraitV1>,
        requirements: Vec<AcquisitionRequirementV1>,
        requirements_satisfied: bool,
    },
    Potion {
        potion: PotionId,
        traits: Vec<PotionAcquisitionTraitV1>,
        requirements: Vec<AcquisitionRequirementV1>,
        requirements_satisfied: bool,
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
    pub resolved_formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub added_formation_strengths: Vec<StrategyPackageIdV2>,
    pub matched_consumable_capabilities: Vec<StrategyCapabilityKindV1>,
    pub upgrade_scope_before: Option<CombatUpgradeScopeV1>,
    pub upgrade_scope_after: Option<CombatUpgradeScopeV1>,
    pub introduces_status_burden: bool,
    pub redundant_upgrade_access: bool,
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
    pub resolved_formation_needs: Vec<String>,
    pub added_formation_strengths: Vec<String>,
    pub matched_consumable_capabilities: Vec<String>,
    pub upgrade_scope_before: Option<String>,
    pub upgrade_scope_after: Option<String>,
    pub introduces_status_burden: bool,
    pub redundant_upgrade_access: bool,
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
                    redundant_upgrade_access: evidence.redundant_upgrade_access,
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
    let mut evidence = exact
        .actions
        .iter()
        .enumerate()
        .map(|(surface_index, action)| {
            shop_action_evidence_v1(session, &exact, action, surface_index, &purge_target_losses)
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
) -> Result<ShopPolicyActionEvidenceV1, String> {
    let candidate_key = action
        .candidate_key
        .clone()
        .ok_or_else(|| format!("shop candidate '{}' has no typed key", action.candidate_id))?;
    let acquisition = acquisition_v1(parent, &candidate_key)?;
    let delta = run_policy_state_delta_v1(&decision.before, &action.after);
    let closed_threat_gaps = delta.closed_threat_gaps;
    let capability_improvements = delta.capability_improvements;
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
        hp_gain,
        max_hp_gain,
        deck_size_delta,
        &closed_threat_gaps,
        &capability_improvements,
        &resolved_formation_needs,
        &added_formation_strengths,
        &matched_consumable_capabilities,
        upgrade_scope_before,
        upgrade_scope_after,
        introduces_status_burden,
        redundant_upgrade_access,
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
        resolved_formation_needs,
        added_formation_strengths,
        matched_consumable_capabilities,
        upgrade_scope_before,
        upgrade_scope_after,
        introduces_status_burden,
        redundant_upgrade_access,
        purge_target_loss,
        surface_index,
    })
}

fn acquisition_v1(
    parent: &RunControlSession,
    key: &DecisionCandidateKey,
) -> Result<ShopPolicyAcquisitionV1, String> {
    Ok(match key {
        DecisionCandidateKey::ShopBuyCard { card, upgrades, .. } => {
            let copies_before = parent
                .run_state
                .master_deck
                .iter()
                .filter(|owned| owned.id == *card)
                .count();
            ShopPolicyAcquisitionV1::Card {
                card: *card,
                upgrades: *upgrades,
                copies_before,
                semantics: card_reward_semantic_profile_v1(&RewardCard::new(*card, *upgrades)),
            }
        }
        DecisionCandidateKey::ShopBuyRelic { relic, .. } => {
            let requirements = relic_acquisition_requirements_v1(*relic);
            let requirements_satisfied = requirements
                .iter()
                .all(|requirement| acquisition_requirement_satisfied(parent, *requirement));
            ShopPolicyAcquisitionV1::Relic {
                relic: *relic,
                traits: relic_acquisition_traits_v1(*relic),
                requirements,
                requirements_satisfied,
            }
        }
        DecisionCandidateKey::ShopBuyPotion { potion, .. } => {
            let requirements = potion_acquisition_requirements_v1(*potion);
            let requirements_satisfied = requirements
                .iter()
                .all(|requirement| acquisition_requirement_satisfied(parent, *requirement));
            ShopPolicyAcquisitionV1::Potion {
                potion: *potion,
                traits: potion_acquisition_traits_v1(*potion),
                requirements,
                requirements_satisfied,
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

fn acquisition_requirement_satisfied(
    parent: &RunControlSession,
    requirement: AcquisitionRequirementV1,
) -> bool {
    match requirement {
        AcquisitionRequirementV1::XCostPayoff => parent
            .run_state
            .master_deck
            .iter()
            .any(|card| get_card_definition(card.id).cost == -1),
        AcquisitionRequirementV1::DuplicateTarget => !parent.run_state.master_deck.is_empty(),
        AcquisitionRequirementV1::LowHpDeathInsurance => {
            parent.run_state.current_hp.saturating_mul(2) <= parent.run_state.max_hp
        }
        AcquisitionRequirementV1::RouteEscapeValue => route_escape_value_v1(parent),
    }
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
        Trait::DeathInsurance | Trait::EscapeTool => vec![Capability::SustainedDefense],
        Trait::DebuffControl => vec![Capability::DebuffResilience],
    }
}

#[allow(clippy::too_many_arguments)]
fn shop_policy_band_v1(
    parent: &RunControlSession,
    acquisition: &ShopPolicyAcquisitionV1,
    followup: ShopPolicyFollowupV1,
    hp_gain: i32,
    max_hp_gain: i32,
    deck_size_delta: isize,
    closed_threat_gaps: &[ShopPolicyThreatGapKeyV1],
    capability_improvements: &[ShopPolicyCapabilityChangeV1],
    resolved_formation_needs: &[StrategyDeckFormationNeedV1],
    added_formation_strengths: &[StrategyPackageIdV2],
    matched_consumable_capabilities: &[StrategyCapabilityKindV1],
    upgrade_scope_before: Option<CombatUpgradeScopeV1>,
    upgrade_scope_after: Option<CombatUpgradeScopeV1>,
    introduces_status_burden: bool,
    redundant_upgrade_access: bool,
    purge_target_loss: Option<DeckMutationTargetLossTierV1>,
) -> ShopPolicyBandV1 {
    if matches!(acquisition, ShopPolicyAcquisitionV1::OpenRewards) {
        return ShopPolicyBandV1::ResolvePendingBoundary;
    }
    if hp_gain > 0 || max_hp_gain > 0 {
        return ShopPolicyBandV1::ImmediateSurvival;
    }
    if !closed_threat_gaps.is_empty() {
        return ShopPolicyBandV1::CloseThreatGap;
    }
    if !capability_improvements.is_empty() || !matched_consumable_capabilities.is_empty() {
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
    if introduces_status_burden || redundant_upgrade_access {
        return ShopPolicyBandV1::Liability;
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
    ShopPolicyBandV1::SpeculativePurchase
}

fn strategic_acquisition_supported(
    parent: &RunControlSession,
    acquisition: &ShopPolicyAcquisitionV1,
    followup: ShopPolicyFollowupV1,
) -> bool {
    match acquisition {
        ShopPolicyAcquisitionV1::Card {
            semantics,
            copies_before,
            ..
        } => {
            *copies_before == 0
                && semantics.roles.iter().any(|role| {
                    matches!(
                        role,
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
            requirements_satisfied,
            ..
        } => {
            *requirements_satisfied
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
            requirements_satisfied,
            ..
        } => !requirements.is_empty() && *requirements_satisfied,
        ShopPolicyAcquisitionV1::Purge { .. }
        | ShopPolicyAcquisitionV1::OpenRewards
        | ShopPolicyAcquisitionV1::Leave => false,
    }
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

const fn shop_band_priority(band: ShopPolicyBandV1) -> u8 {
    match band {
        ShopPolicyBandV1::ResolvePendingBoundary => 0,
        ShopPolicyBandV1::ImmediateSurvival => 1,
        ShopPolicyBandV1::CloseThreatGap => 2,
        ShopPolicyBandV1::ImproveRequiredCapability => 3,
        // Both are durable deck improvements. Their exact evidence and
        // opportunity cost must compare before the owner commits all gold to
        // one category merely because it was represented by a different verb.
        ShopPolicyBandV1::DeckRepair | ShopPolicyBandV1::EstablishStrategicAsset => 4,
        ShopPolicyBandV1::PreserveResources => 5,
        ShopPolicyBandV1::SpeculativePurchase => 6,
        ShopPolicyBandV1::Liability => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run_control::{build_decision_surface, RunControlConfig};
    use crate::runtime::combat::CombatCard;
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
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
                requirements_satisfied: true,
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
                requirements_satisfied: false,
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
}
