use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::ai::card_semantics_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::noncombat_strategy_v1::{
    threat_relevant_capability_improvements_v1, StrategyThreatSourceV1,
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
    pub access_saturated: bool,
    pub improves_threat_relevant_capability: bool,
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
            card_reward_action_evidence_v1(session, &exact, action, surface_index)
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
) -> Result<CardRewardPolicyActionEvidenceV1, String> {
    let candidate_key = action.candidate_key.clone().ok_or_else(|| {
        format!(
            "card reward candidate '{}' has no typed key",
            action.candidate_id
        )
    })?;
    let acquisition = acquisition_v1(parent, &candidate_key)?;
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
    let access_saturated = matches!(
        &acquisition,
        CardRewardPolicyAcquisitionV1::Card { semantics, .. }
            if !semantics.roles.is_empty()
                && semantics.roles.iter().all(|role| is_access_role(*role))
                && decision.before.deck.draw_sources
                    .saturating_add(decision.before.deck.energy_sources) >= 3
                && delta.capability_improvements.is_empty()
                && delta.resolved_formation_needs.is_empty()
    );
    let improves_threat_relevant_capability = !threat_relevant_capability_improvements_v1(
        &decision.before.threats,
        &decision.before.threat_coverage,
        &action.after.threat_coverage,
    )
    .is_empty();
    let band = card_reward_band_v1(
        &acquisition,
        &delta,
        introduces_unsupported_mechanics,
        introduces_undigested_status_burden,
        duplicate_low_marginal,
        access_saturated,
        improves_threat_relevant_capability,
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
        access_saturated,
        improves_threat_relevant_capability,
        surface_index,
    })
}

fn acquisition_v1(
    parent: &RunControlSession,
    key: &DecisionCandidateKey,
) -> Result<CardRewardPolicyAcquisitionV1, String> {
    Ok(match key {
        DecisionCandidateKey::CardRewardPick { card, upgrades, .. } => {
            CardRewardPolicyAcquisitionV1::Card {
                card: *card,
                upgrades: *upgrades,
                copies_before: parent
                    .run_state
                    .master_deck
                    .iter()
                    .filter(|owned| owned.id == *card)
                    .count(),
                semantics: card_reward_semantic_profile_v1(&RewardCard::new(*card, *upgrades)),
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
    access_saturated: bool,
    improves_threat_relevant_capability: bool,
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
            if status_burden || unsupported || duplicate_low_marginal || access_saturated {
                CardRewardPolicyBandV1::Liability
            } else if !delta.closed_threat_gaps.is_empty() {
                CardRewardPolicyBandV1::CloseThreatGap
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
}
