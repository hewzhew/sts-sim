use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::ai::deck_mutation_compiler_v1::{
    deck_removal_target_snapshots_v1, DeckMutationTargetLossTierV1,
};
use crate::ai::deck_repair_profile_v1::{
    deck_repair_profile_from_upgrade_plan_v1, DeckRepairUpgradePriorityV1,
};
use crate::ai::upgrade_planner_v1::{
    plan_upgrades_v1, UpgradeDebtKindV1, UpgradeDebtSeverityV1, UpgradeRoleV1, UpgradeVerdictV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, run_policy_state_delta_v1,
    DecisionCandidateKey, ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1,
    RunControlSession, RunPolicyCandidateV1, RunPolicyPriorV1, RunPolicyStateDeltaV1,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CampfirePolicyBandV1 {
    ImmediateSurvival,
    PayRequiredUpgradeDebt,
    RepairDeck,
    ImproveReliability,
    PreserveSurvival,
    PersistentGrowth,
    KeyProgress,
    Speculative,
    Liability,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CampfirePolicyActionV1 {
    Rest {
        hp_gain: i32,
    },
    Smith {
        deck_index: usize,
        card_uuid: u32,
        card: CardId,
        upgrades_before: u8,
        repair_priority: Option<DeckRepairUpgradePriorityV1>,
        urgency: Option<UpgradeDebtSeverityV1>,
        verdict: Option<UpgradeVerdictV1>,
        roles: Vec<UpgradeRoleV1>,
        pays_debts: Vec<UpgradeDebtKindV1>,
    },
    Dig {
        gained_relic: Option<RelicId>,
    },
    Lift {
        girya_counter_gain: i32,
    },
    Toke {
        deck_index: usize,
        card_uuid: u32,
        card: CardId,
        upgrades: u8,
        target_loss: Option<DeckMutationTargetLossTierV1>,
        shared_repair_target: bool,
    },
    Recall {
        ruby_key_gained: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfirePolicyActionEvidenceV1 {
    pub candidate_id: String,
    pub candidate_key: DecisionCandidateKey,
    pub action: CampfirePolicyActionV1,
    pub band: CampfirePolicyBandV1,
    pub delta: RunPolicyStateDeltaV1,
    surface_index: usize,
}

#[derive(Clone, Debug)]
pub struct ExactCampfirePolicyDecisionV1 {
    pub exact: ExactRunPolicyDecisionV1,
    pub evidence: Vec<CampfirePolicyActionEvidenceV1>,
    pub prior: RunPolicyPriorV1,
}

struct CampfirePolicyContextV1 {
    repair_upgrades: BTreeMap<(usize, u32), DeckRepairUpgradePriorityV1>,
    upgrade_evidence: BTreeMap<(usize, u32), CampfireSmithEvidenceV1>,
    target_losses: BTreeMap<(usize, u32), DeckMutationTargetLossTierV1>,
    low_loss_removals: BTreeSet<(usize, u32)>,
}

struct CampfireSmithEvidenceV1 {
    urgency: UpgradeDebtSeverityV1,
    verdict: UpgradeVerdictV1,
    roles: Vec<UpgradeRoleV1>,
    pays_debts: Vec<UpgradeDebtKindV1>,
}

pub fn exact_campfire_policy_prior_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<RunPolicyPriorV1, String> {
    Ok(exact_campfire_policy_decision_v1(session, legal)?.prior)
}

pub fn exact_campfire_policy_decision_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactCampfirePolicyDecisionV1, String> {
    let exact = exact_run_policy_decision_v1(session)?;
    validate_same_candidate_surface(&exact, legal)?;
    let context = campfire_policy_context_v1(session);
    let mut evidence = exact
        .actions
        .iter()
        .filter(|action| action.candidate_key.as_ref().is_some_and(is_campfire_key))
        .enumerate()
        .map(|(surface_index, action)| {
            campfire_action_evidence_v1(session, &exact, action, surface_index, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;
    evidence.sort_by(compare_campfire_evidence);

    let ranked_ids = evidence
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
                    .filter(|candidate| !ranked_ids.contains(candidate.candidate_id))
                    .map(|candidate| candidate.candidate_id.to_string()),
            ),
    )?;

    Ok(ExactCampfirePolicyDecisionV1 {
        exact,
        evidence,
        prior,
    })
}

fn campfire_policy_context_v1(parent: &RunControlSession) -> CampfirePolicyContextV1 {
    let upgrade_plan = plan_upgrades_v1(&parent.run_state);
    let repair = deck_repair_profile_from_upgrade_plan_v1(&parent.run_state, &upgrade_plan);
    let repair_upgrades = repair
        .reliability_upgrades
        .iter()
        .map(|candidate| ((candidate.deck_index, candidate.uuid), candidate.priority))
        .collect();
    let low_loss_removals = repair
        .low_loss_removals
        .iter()
        .map(|candidate| (candidate.deck_index, candidate.uuid))
        .collect();
    let upgrade_evidence = upgrade_plan
        .candidates
        .into_iter()
        .map(|candidate| {
            (
                (candidate.deck_index, candidate.card_uuid),
                CampfireSmithEvidenceV1 {
                    urgency: candidate.urgency,
                    verdict: candidate.verdict,
                    roles: candidate.roles,
                    pays_debts: candidate.pays_debts,
                },
            )
        })
        .collect();
    let target_losses = deck_removal_target_snapshots_v1(&parent.run_state)
        .into_iter()
        .map(|snapshot| {
            (
                (snapshot.deck_index, snapshot.uuid),
                snapshot.target_loss.tier,
            )
        })
        .collect();
    CampfirePolicyContextV1 {
        repair_upgrades,
        upgrade_evidence,
        target_losses,
        low_loss_removals,
    }
}

fn validate_same_candidate_surface(
    exact: &ExactRunPolicyDecisionV1,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<(), String> {
    let exact_ids = exact
        .actions
        .iter()
        .filter(|action| action.candidate_key.as_ref().is_some_and(is_campfire_key))
        .map(|action| action.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let legal_ids = legal
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    if exact_ids.is_empty() || exact_ids != legal_ids {
        return Err(format!(
            "campfire policy surface differs from the exact typed surface: exact={} policy={}",
            exact_ids.len(),
            legal_ids.len()
        ));
    }
    Ok(())
}

fn is_campfire_key(key: &DecisionCandidateKey) -> bool {
    matches!(
        key,
        DecisionCandidateKey::CampfireRest
            | DecisionCandidateKey::CampfireSmith { .. }
            | DecisionCandidateKey::CampfireDig
            | DecisionCandidateKey::CampfireLift
            | DecisionCandidateKey::CampfireToke { .. }
            | DecisionCandidateKey::CampfireRecall
    )
}

fn campfire_action_evidence_v1(
    parent: &RunControlSession,
    decision: &ExactRunPolicyDecisionV1,
    successor: &ExactRunPolicyActionSuccessorV1,
    surface_index: usize,
    context: &CampfirePolicyContextV1,
) -> Result<CampfirePolicyActionEvidenceV1, String> {
    let candidate_key = successor.candidate_key.clone().ok_or_else(|| {
        format!(
            "campfire candidate '{}' has no typed key",
            successor.candidate_id
        )
    })?;
    let delta = run_policy_state_delta_v1(&decision.before, &successor.after);
    let action = campfire_action_v1(parent, successor, &candidate_key, &delta, context)?;
    let band = campfire_policy_band_v1(parent, &action);

    Ok(CampfirePolicyActionEvidenceV1 {
        candidate_id: successor.candidate_id.clone(),
        candidate_key,
        action,
        band,
        delta,
        surface_index,
    })
}

fn campfire_action_v1(
    parent: &RunControlSession,
    successor: &ExactRunPolicyActionSuccessorV1,
    key: &DecisionCandidateKey,
    delta: &RunPolicyStateDeltaV1,
    context: &CampfirePolicyContextV1,
) -> Result<CampfirePolicyActionV1, String> {
    Ok(match key {
        DecisionCandidateKey::CampfireRest => CampfirePolicyActionV1::Rest {
            hp_gain: delta.hp_gain,
        },
        DecisionCandidateKey::CampfireSmith {
            deck_index,
            card_uuid,
            card,
            upgrades,
        } => {
            let repair_priority = context
                .repair_upgrades
                .get(&(*deck_index, *card_uuid))
                .copied();
            let upgrade = context.upgrade_evidence.get(&(*deck_index, *card_uuid));
            CampfirePolicyActionV1::Smith {
                deck_index: *deck_index,
                card_uuid: *card_uuid,
                card: *card,
                upgrades_before: *upgrades,
                repair_priority,
                urgency: upgrade.map(|candidate| candidate.urgency),
                verdict: upgrade.map(|candidate| candidate.verdict),
                roles: upgrade
                    .map(|candidate| candidate.roles.clone())
                    .unwrap_or_default(),
                pays_debts: upgrade
                    .map(|candidate| candidate.pays_debts.clone())
                    .unwrap_or_default(),
            }
        }
        DecisionCandidateKey::CampfireDig => {
            let before = parent
                .run_state
                .relics
                .iter()
                .map(|relic| relic.id)
                .collect::<Vec<_>>();
            let gained_relic = successor
                .exact
                .session
                .run_state
                .relics
                .iter()
                .map(|relic| relic.id)
                .find(|relic| !before.contains(relic));
            CampfirePolicyActionV1::Dig { gained_relic }
        }
        DecisionCandidateKey::CampfireLift => {
            let before = relic_counter(parent, RelicId::Girya);
            let after = relic_counter(&successor.exact.session, RelicId::Girya);
            CampfirePolicyActionV1::Lift {
                girya_counter_gain: after.saturating_sub(before),
            }
        }
        DecisionCandidateKey::CampfireToke {
            deck_index,
            card_uuid,
            card,
            upgrades,
        } => {
            let target_loss = context
                .target_losses
                .get(&(*deck_index, *card_uuid))
                .copied();
            let shared_repair_target = context
                .low_loss_removals
                .contains(&(*deck_index, *card_uuid));
            CampfirePolicyActionV1::Toke {
                deck_index: *deck_index,
                card_uuid: *card_uuid,
                card: *card,
                upgrades: *upgrades,
                target_loss,
                shared_repair_target,
            }
        }
        DecisionCandidateKey::CampfireRecall => CampfirePolicyActionV1::Recall {
            ruby_key_gained: !parent.run_state.keys[0] && successor.exact.session.run_state.keys[0],
        },
        other => {
            return Err(format!(
                "exact campfire policy received non-campfire candidate key {other:?}"
            ))
        }
    })
}

fn relic_counter(session: &RunControlSession, relic_id: RelicId) -> i32 {
    session
        .run_state
        .relics
        .iter()
        .find(|relic| relic.id == relic_id)
        .map(|relic| relic.counter)
        .unwrap_or(0)
}

fn campfire_policy_band_v1(
    parent: &RunControlSession,
    action: &CampfirePolicyActionV1,
) -> CampfirePolicyBandV1 {
    match action {
        CampfirePolicyActionV1::Rest { hp_gain } if *hp_gain <= 0 => {
            CampfirePolicyBandV1::Liability
        }
        CampfirePolicyActionV1::Rest { hp_gain } if parent.run_state.current_hp <= *hp_gain => {
            CampfirePolicyBandV1::ImmediateSurvival
        }
        CampfirePolicyActionV1::Rest { .. } => CampfirePolicyBandV1::PreserveSurvival,
        CampfirePolicyActionV1::Smith {
            repair_priority: Some(DeckRepairUpgradePriorityV1::NeededFunction),
            ..
        }
        | CampfirePolicyActionV1::Smith {
            urgency:
                Some(
                    UpgradeDebtSeverityV1::ImportantBeforeBoss
                    | UpgradeDebtSeverityV1::CriticalBeforeBoss,
                ),
            ..
        }
        | CampfirePolicyActionV1::Smith {
            verdict: Some(UpgradeVerdictV1::CoreDebtPayment | UpgradeVerdictV1::Important),
            ..
        } => CampfirePolicyBandV1::PayRequiredUpgradeDebt,
        CampfirePolicyActionV1::Smith {
            repair_priority: Some(DeckRepairUpgradePriorityV1::Reliability),
            ..
        }
        | CampfirePolicyActionV1::Smith {
            verdict: Some(UpgradeVerdictV1::Useful),
            ..
        } => CampfirePolicyBandV1::ImproveReliability,
        CampfirePolicyActionV1::Smith {
            verdict: Some(UpgradeVerdictV1::Avoid),
            ..
        } => CampfirePolicyBandV1::Liability,
        CampfirePolicyActionV1::Smith { .. } => CampfirePolicyBandV1::Speculative,
        CampfirePolicyActionV1::Toke {
            target_loss: Some(DeckMutationTargetLossTierV1::LowValue),
            ..
        }
        | CampfirePolicyActionV1::Toke {
            shared_repair_target: true,
            ..
        } => CampfirePolicyBandV1::RepairDeck,
        CampfirePolicyActionV1::Toke {
            target_loss:
                Some(
                    DeckMutationTargetLossTierV1::CoreFunctional
                    | DeckMutationTargetLossTierV1::Unsupported,
                ),
            ..
        } => CampfirePolicyBandV1::Liability,
        CampfirePolicyActionV1::Toke { .. } => CampfirePolicyBandV1::Speculative,
        CampfirePolicyActionV1::Dig { gained_relic } if gained_relic.is_some() => {
            CampfirePolicyBandV1::PersistentGrowth
        }
        CampfirePolicyActionV1::Lift { girya_counter_gain } if *girya_counter_gain > 0 => {
            CampfirePolicyBandV1::PersistentGrowth
        }
        CampfirePolicyActionV1::Recall {
            ruby_key_gained: true,
        } => CampfirePolicyBandV1::KeyProgress,
        CampfirePolicyActionV1::Dig { .. }
        | CampfirePolicyActionV1::Lift { .. }
        | CampfirePolicyActionV1::Recall { .. } => CampfirePolicyBandV1::Speculative,
    }
}

fn compare_campfire_evidence(
    left: &CampfirePolicyActionEvidenceV1,
    right: &CampfirePolicyActionEvidenceV1,
) -> Ordering {
    left.band
        .cmp(&right.band)
        .then_with(|| compare_action_evidence(&left.action, &right.action))
        .then_with(|| right.delta.hp_gain.cmp(&left.delta.hp_gain))
        .then_with(|| {
            right
                .delta
                .closed_threat_gaps
                .len()
                .cmp(&left.delta.closed_threat_gaps.len())
        })
        .then_with(|| {
            right
                .delta
                .capability_improvements
                .len()
                .cmp(&left.delta.capability_improvements.len())
        })
        .then_with(|| left.surface_index.cmp(&right.surface_index))
}

fn compare_action_evidence(
    left: &CampfirePolicyActionV1,
    right: &CampfirePolicyActionV1,
) -> Ordering {
    match (left, right) {
        (
            CampfirePolicyActionV1::Smith {
                repair_priority: left_repair,
                urgency: left_urgency,
                verdict: left_verdict,
                pays_debts: left_debts,
                ..
            },
            CampfirePolicyActionV1::Smith {
                repair_priority: right_repair,
                urgency: right_urgency,
                verdict: right_verdict,
                pays_debts: right_debts,
                ..
            },
        ) => repair_priority_rank(*left_repair)
            .cmp(&repair_priority_rank(*right_repair))
            .then_with(|| right_urgency.cmp(left_urgency))
            .then_with(|| {
                upgrade_verdict_rank(*right_verdict).cmp(&upgrade_verdict_rank(*left_verdict))
            })
            .then_with(|| right_debts.len().cmp(&left_debts.len())),
        (
            CampfirePolicyActionV1::Toke {
                target_loss: left_loss,
                ..
            },
            CampfirePolicyActionV1::Toke {
                target_loss: right_loss,
                ..
            },
        ) => left_loss.cmp(right_loss),
        _ => Ordering::Equal,
    }
}

fn repair_priority_rank(priority: Option<DeckRepairUpgradePriorityV1>) -> u8 {
    match priority {
        Some(DeckRepairUpgradePriorityV1::NeededFunction) => 0,
        Some(DeckRepairUpgradePriorityV1::Reliability) => 1,
        None => 2,
    }
}

fn upgrade_verdict_rank(verdict: Option<UpgradeVerdictV1>) -> u8 {
    match verdict {
        None => 0,
        Some(UpgradeVerdictV1::Avoid) => 1,
        Some(UpgradeVerdictV1::Defer) => 2,
        Some(UpgradeVerdictV1::Opportunistic) => 3,
        Some(UpgradeVerdictV1::Useful) => 4,
        Some(UpgradeVerdictV1::Important) => 5,
        Some(UpgradeVerdictV1::CoreDebtPayment) => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;
    use crate::eval::run_control::{build_decision_surface, RunControlConfig};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::EngineState;

    fn policy_candidates<'a>(
        surface: &'a super::super::DecisionSurface,
    ) -> Vec<RunPolicyCandidateV1<'a>> {
        surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
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

    fn campfire_session(cards: &[CardId]) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Campfire;
        session.run_state.master_deck = cards
            .iter()
            .enumerate()
            .map(|(index, card)| CombatCard::new(*card, 10_000 + index as u32))
            .collect();
        session
    }

    #[test]
    fn exact_campfire_surface_has_typed_positive_support_for_every_action() {
        let mut session = campfire_session(&[CardId::Strike, CardId::Defend]);
        session.run_state.is_final_act_available = true;
        session.run_state.relics = [RelicId::Shovel, RelicId::Girya, RelicId::PeacePipe]
            .into_iter()
            .map(RelicState::new)
            .collect();
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_campfire_policy_decision_v1(&session, &legal).expect("exact campfire decision");

        assert_eq!(decision.evidence.len(), legal.len());
        assert_eq!(decision.prior.entries.len(), legal.len());
        assert!(decision
            .prior
            .entries
            .iter()
            .all(|entry| entry.probability.is_finite() && entry.probability > 0.0));
        assert!(decision
            .evidence
            .iter()
            .all(|entry| is_campfire_key(&entry.candidate_key)));
    }

    #[test]
    fn low_hp_rest_is_urgent_but_full_hp_rest_is_not_universally_preferred() {
        let mut low_hp = campfire_session(&[CardId::Apparition, CardId::Cleave]);
        low_hp.run_state.current_hp = 1;
        low_hp.run_state.max_hp = 80;
        let surface = build_decision_surface(&low_hp);
        let legal = policy_candidates(&surface);
        let decision = exact_campfire_policy_decision_v1(&low_hp, &legal).expect("low hp campfire");
        assert_eq!(decision.prior.entries[0].candidate_id, "rest");

        let mut full_hp = low_hp.clone();
        full_hp.run_state.current_hp = full_hp.run_state.max_hp;
        let surface = build_decision_surface(&full_hp);
        let legal = policy_candidates(&surface);
        let decision =
            exact_campfire_policy_decision_v1(&full_hp, &legal).expect("full hp campfire");
        assert_ne!(decision.prior.entries[0].candidate_id, "rest");
    }

    #[test]
    fn reliability_upgrade_uses_shared_deck_repair_evidence() {
        let session = campfire_session(&[CardId::Apparition, CardId::Cleave]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_campfire_policy_decision_v1(&session, &legal).expect("upgrade decision");
        let apparition = decision
            .evidence
            .iter()
            .find(|entry| {
                matches!(
                    entry.action,
                    CampfirePolicyActionV1::Smith {
                        card: CardId::Apparition,
                        ..
                    }
                )
            })
            .expect("Apparition smith");

        assert_eq!(apparition.band, CampfirePolicyBandV1::ImproveReliability);
        assert!(matches!(
            apparition.action,
            CampfirePolicyActionV1::Smith {
                repair_priority: Some(DeckRepairUpgradePriorityV1::Reliability),
                ..
            }
        ));
    }

    #[test]
    fn toke_uses_shared_target_loss_instead_of_card_name_fallbacks() {
        let mut session = campfire_session(&[CardId::Flex, CardId::Flex, CardId::Barricade]);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::PeacePipe));
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_campfire_policy_decision_v1(&session, &legal).expect("toke decision");
        let best_toke = decision
            .evidence
            .iter()
            .find(|entry| matches!(entry.action, CampfirePolicyActionV1::Toke { .. }))
            .expect("toke action");

        assert!(matches!(
            best_toke.action,
            CampfirePolicyActionV1::Toke {
                card: CardId::Flex,
                shared_repair_target: true,
                ..
            }
        ));
        assert_eq!(best_toke.band, CampfirePolicyBandV1::RepairDeck);
    }
}
