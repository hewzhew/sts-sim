use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::ai::block_plan_profile_v1::{block_plan_profile_v1, BlockPlanProfileV1};
use crate::ai::card_component_signal_v1::{
    evaluate_card_component_signals_v1, CardComponentSignalContextV1, CardComponentSignalKindV1,
    CardComponentSignalReportV1,
};
use crate::ai::card_semantics_v1::{
    card_access_evidence_v1, card_reward_semantic_profile_v1, CardAccessEvidenceV1,
    CardAccessLeverageV1, CardRewardPickDependencyV1, CardRewardSemanticProfileV1,
    CardRewardSemanticRoleV1,
};
use crate::ai::deck_startup_profile_v1::{deck_startup_profile_v1, DeckStartupProfileV1};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, threat_relevant_capability_improvements_v1,
    StrategyCapabilityCoverageV1, StrategyCapabilityKindV1, StrategyPackageIdV2,
    StrategyPlanSupportV1, StrategyThreatSourceV1,
};
use crate::content::cards::CardId;
use crate::state::rewards::RewardCard;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, run_policy_state_delta_v1,
    DecisionCandidateKey, ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1,
    RunControlSession, RunPolicyCandidateV1, RunPolicyPriorV1, RunPolicyStateDeltaV1,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug, PartialEq)]
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
    pub access_conflict_or_redundancy: bool,
    pub improves_threat_relevant_capability: bool,
    pub amplifies_existing_answers: bool,
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
    let block_plan = block_plan_profile_v1(&session.run_state);
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
                &block_plan,
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
    block_plan: &BlockPlanProfileV1,
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
    let duplicate_low_marginal = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card {
            copies_before,
            ..
        } if *copies_before > 0
            && delta.closed_threat_gaps.is_empty()
            && delta.capability_improvements.is_empty()
            && delta.resolved_formation_needs.is_empty()
            && delta.added_formation_strengths.is_empty()
    );
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
    let improves_threat_relevant_capability = !threat_relevant_capability_improvements_v1(
        &decision.before.threats,
        &decision.before.threat_coverage,
        &action.after.threat_coverage,
    )
    .is_empty();
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
    let base_band = card_reward_band_v1(
        &acquisition,
        &delta,
        introduces_unsupported_mechanics,
        introduces_undigested_status_burden,
        duplicate_low_marginal,
        access_conflict_or_redundancy,
        improves_threat_relevant_capability,
        amplifies_existing_answers,
    );
    let band = apply_upgrade_investment_gate_v1(base_band, upgrade_investment_support);

    Ok(CardRewardPolicyActionEvidenceV1 {
        candidate_id: action.candidate_id.clone(),
        candidate_key,
        acquisition,
        band,
        delta,
        introduces_unsupported_mechanics,
        introduces_undigested_status_burden,
        duplicate_low_marginal,
        access_conflict_or_redundancy,
        improves_threat_relevant_capability,
        amplifies_existing_answers,
        upgrade_investment_support,
        surface_index,
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
    access_conflict_or_redundancy: bool,
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
        CardRewardPolicyAcquisitionV1::Card { semantics, .. } => {
            if status_burden
                || unsupported
                || duplicate_low_marginal
                || access_conflict_or_redundancy
            {
                CardRewardPolicyBandV1::Liability
            } else if !delta.closed_threat_gaps.is_empty() {
                CardRewardPolicyBandV1::CloseThreatGap
            } else if amplifies_existing_answers {
                CardRewardPolicyBandV1::AmplifyStrategicAccess
            } else if improves_threat_relevant_capability {
                CardRewardPolicyBandV1::ImproveRequiredCapability
            } else if !delta.resolved_formation_needs.is_empty()
                || !delta.added_formation_strengths.is_empty()
                || semantics
                    .roles
                    .iter()
                    .any(|role| is_independent_role(*role))
            {
                CardRewardPolicyBandV1::EstablishStrategicAsset
            } else {
                CardRewardPolicyBandV1::SpeculativeAddition
            }
        }
    }
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

fn is_independent_role(role: CardRewardSemanticRoleV1) -> bool {
    matches!(
        role,
        CardRewardSemanticRoleV1::AoeDamage
            | CardRewardSemanticRoleV1::Block
            | CardRewardSemanticRoleV1::CardDraw
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
    use crate::runtime::combat::CombatCard;
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

    fn decision(session: &RunControlSession) -> ExactCardRewardPolicyDecisionV1 {
        let surface = build_decision_surface(session);
        let legal = surface
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
            .collect::<Vec<_>>();
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
    fn urgent_frontload_gap_still_precedes_access_amplification() {
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
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, index as u32))
        .collect();

        let decision = decision(&session);
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
