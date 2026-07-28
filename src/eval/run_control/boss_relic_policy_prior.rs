use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::ai::action_supply_v1::{action_supply_profile_v1, ActionSupplyProfileV1};
use crate::ai::card_semantics_v1::{relic_acquisition_traits_v1, RelicAcquisitionTraitV1};
use crate::ai::deck_startup_profile_v1::{deck_startup_profile_v1, DeckStartupProfileV1};
use crate::ai::strategic::{
    run_debt_projection_for_relic_v1, RunDebtContractKindV1, RunDebtProjectionV1,
};
use crate::content::relics::{energy_master_delta, RelicId};
use crate::state::core::EngineState;
use crate::state::RunPendingChoiceReason;

use super::{
    exact_run_policy_decision_v1, positive_ranked_run_policy_prior_v1, DecisionCandidateKey,
    ExactRunPolicyActionSuccessorV1, ExactRunPolicyDecisionV1, RunControlSession,
    RunPolicyCandidateV1, RunPolicyPriorV1,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BossRelicPolicyBandV1 {
    SupportedEnergy,
    DeckReconstruction,
    StrategicAsset,
    StarterUpgrade,
    ConditionalGrowth,
    PreserveState,
    ConstrainedEnergy,
    Liability,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BossRelicPolicyFollowupV1 {
    ActAdvanced,
    Selection(RunPendingChoiceReason),
    Reward,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BossRelicPolicyActionV1 {
    Pick {
        relic: RelicId,
        energy_gain: u8,
        replaces_existing_relic: bool,
        changed_deck_cards: usize,
        followup: BossRelicPolicyFollowupV1,
        traits: Vec<RelicAcquisitionTraitV1>,
        added_debts: Vec<RunDebtContractKindV1>,
        unresolved_debt_terms: usize,
        compounding_debt_count: usize,
        startup: BossRelicStartupDeltaV1,
        action_supply: BossRelicActionSupplyDeltaV1,
    },
    Skip,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BossRelicStartupDeltaV1 {
    pub setup_debt: i16,
    pub setup_payment: i16,
    pub immediate_survival: i16,
    pub payoff_engine: i16,
    pub combat_shape_risk: i16,
    pub strong_draw: i16,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BossRelicActionSupplyDeltaV1 {
    pub opening_once_options: i16,
    pub delayed_per_turn_sources: i16,
    pub same_turn_burst_sources: i16,
    pub triggered_repeatable_sources: i16,
    pub additional_play_sources: i16,
    pub cost_or_resource_compression_sources: i16,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossRelicPolicyActionEvidenceV1 {
    pub candidate_id: String,
    pub candidate_key: DecisionCandidateKey,
    pub action: BossRelicPolicyActionV1,
    pub band: BossRelicPolicyBandV1,
    surface_index: usize,
}

#[derive(Clone, Debug)]
pub struct ExactBossRelicPolicyDecisionV1 {
    pub exact: ExactRunPolicyDecisionV1,
    pub evidence: Vec<BossRelicPolicyActionEvidenceV1>,
    pub prior: RunPolicyPriorV1,
}

struct BossRelicPolicyContextV1 {
    startup_before: DeckStartupProfileV1,
    action_supply_before: ActionSupplyProfileV1,
    debt: Vec<(RelicId, RunDebtProjectionV1)>,
}

pub fn exact_boss_relic_policy_prior_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<RunPolicyPriorV1, String> {
    Ok(exact_boss_relic_policy_decision_v1(session, legal)?.prior)
}

pub fn exact_boss_relic_policy_decision_v1(
    session: &RunControlSession,
    legal: &[RunPolicyCandidateV1<'_>],
) -> Result<ExactBossRelicPolicyDecisionV1, String> {
    if !matches!(session.engine_state, EngineState::BossRelicSelect(_)) {
        return Err("exact boss relic policy requires a BossRelicSelect boundary".to_string());
    }
    let exact = exact_run_policy_decision_v1(session)?;
    validate_same_candidate_surface(&exact, legal)?;
    let context = boss_relic_policy_context_v1(session, &exact);
    let mut evidence = exact
        .actions
        .iter()
        .enumerate()
        .map(|(surface_index, successor)| {
            boss_relic_action_evidence_v1(session, successor, surface_index, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;
    evidence.sort_by(compare_boss_relic_evidence);
    let prior = positive_ranked_run_policy_prior_v1(
        legal,
        evidence
            .iter()
            .map(|candidate| candidate.candidate_id.clone()),
    )?;
    Ok(ExactBossRelicPolicyDecisionV1 {
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
        .filter(|action| action.candidate_key.as_ref().is_some_and(is_boss_relic_key))
        .map(|action| action.candidate_id.as_str())
        .collect::<BTreeSet<_>>();
    let legal_ids = legal
        .iter()
        .map(|candidate| candidate.candidate_id)
        .collect::<BTreeSet<_>>();
    if exact_ids != legal_ids || exact_ids.len() != legal.len() {
        return Err(format!(
            "boss relic policy surface differs from exact typed surface: exact={} policy={}",
            exact_ids.len(),
            legal.len()
        ));
    }
    Ok(())
}

fn is_boss_relic_key(key: &DecisionCandidateKey) -> bool {
    matches!(
        key,
        DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip
    )
}

fn boss_relic_policy_context_v1(
    parent: &RunControlSession,
    exact: &ExactRunPolicyDecisionV1,
) -> BossRelicPolicyContextV1 {
    let debt = exact
        .actions
        .iter()
        .filter_map(|action| match action.candidate_key {
            Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => Some((
                relic,
                run_debt_projection_for_relic_v1(&parent.run_state, relic),
            )),
            _ => None,
        })
        .collect();
    BossRelicPolicyContextV1 {
        startup_before: deck_startup_profile_v1(&parent.run_state),
        action_supply_before: action_supply_profile_v1(&parent.run_state),
        debt,
    }
}

fn boss_relic_action_evidence_v1(
    parent: &RunControlSession,
    successor: &ExactRunPolicyActionSuccessorV1,
    surface_index: usize,
    context: &BossRelicPolicyContextV1,
) -> Result<BossRelicPolicyActionEvidenceV1, String> {
    let candidate_key = successor.candidate_key.clone().ok_or_else(|| {
        format!(
            "boss relic candidate '{}' has no typed key",
            successor.candidate_id
        )
    })?;
    let action = match candidate_key {
        DecisionCandidateKey::BossRelicPick { relic, .. } => {
            let child = &successor.exact.session;
            let debt = context
                .debt
                .iter()
                .find(|(candidate, _)| *candidate == relic)
                .map(|(_, projection)| projection)
                .expect("boss relic context covers every exact relic candidate");
            let startup_after = deck_startup_profile_v1(&child.run_state);
            let supply_after = action_supply_profile_v1(&child.run_state);
            BossRelicPolicyActionV1::Pick {
                relic,
                energy_gain: energy_master_delta(relic),
                replaces_existing_relic: child.run_state.relics.len()
                    <= parent.run_state.relics.len(),
                changed_deck_cards: changed_deck_cards(
                    &parent.run_state.master_deck,
                    &child.run_state.master_deck,
                ),
                followup: followup_v1(parent, child),
                traits: relic_acquisition_traits_v1(relic),
                added_debts: debt
                    .added_contracts
                    .iter()
                    .map(|contract| contract.kind)
                    .collect(),
                unresolved_debt_terms: debt
                    .added_contracts
                    .iter()
                    .map(|contract| contract.unresolved.len())
                    .sum(),
                compounding_debt_count: debt.compounding_tags.len(),
                startup: startup_delta_v1(&context.startup_before, &startup_after),
                action_supply: action_supply_delta_v1(&context.action_supply_before, &supply_after),
            }
        }
        DecisionCandidateKey::BossRelicSkip => BossRelicPolicyActionV1::Skip,
        ref other => {
            return Err(format!(
                "exact boss relic policy received non-boss-relic candidate key {other:?}"
            ))
        }
    };
    let band = boss_relic_policy_band_v1(&action);
    Ok(BossRelicPolicyActionEvidenceV1 {
        candidate_id: successor.candidate_id.clone(),
        candidate_key,
        action,
        band,
        surface_index,
    })
}

fn followup_v1(parent: &RunControlSession, child: &RunControlSession) -> BossRelicPolicyFollowupV1 {
    match &child.engine_state {
        EngineState::RunPendingChoice(choice) => {
            BossRelicPolicyFollowupV1::Selection(choice.reason)
        }
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            BossRelicPolicyFollowupV1::Reward
        }
        EngineState::MapNavigation if child.run_state.act_num > parent.run_state.act_num => {
            BossRelicPolicyFollowupV1::ActAdvanced
        }
        _ => BossRelicPolicyFollowupV1::Other,
    }
}

fn changed_deck_cards(
    before: &[crate::runtime::combat::CombatCard],
    after: &[crate::runtime::combat::CombatCard],
) -> usize {
    let before_by_uuid = before
        .iter()
        .map(|card| (card.uuid, (card.id, card.upgrades)))
        .collect::<BTreeMap<_, _>>();
    let after_by_uuid = after
        .iter()
        .map(|card| (card.uuid, (card.id, card.upgrades)))
        .collect::<BTreeMap<_, _>>();
    before_by_uuid
        .iter()
        .filter(|(uuid, card)| after_by_uuid.get(uuid) != Some(card))
        .count()
        .saturating_add(
            after_by_uuid
                .keys()
                .filter(|uuid| !before_by_uuid.contains_key(uuid))
                .count(),
        )
}

fn startup_delta_v1(
    before: &DeckStartupProfileV1,
    after: &DeckStartupProfileV1,
) -> BossRelicStartupDeltaV1 {
    BossRelicStartupDeltaV1 {
        setup_debt: delta_u8(after.setup_debt, before.setup_debt),
        setup_payment: delta_u8(
            after.effective_setup_payment,
            before.effective_setup_payment,
        ),
        immediate_survival: delta_u8(after.immediate_survival, before.immediate_survival),
        payoff_engine: delta_u8(after.payoff_engine, before.payoff_engine),
        combat_shape_risk: delta_u8(after.combat_shape_risk, before.combat_shape_risk),
        strong_draw: delta_u8(
            after.effective_strong_draw_count,
            before.effective_strong_draw_count,
        ),
    }
}

fn action_supply_delta_v1(
    before: &ActionSupplyProfileV1,
    after: &ActionSupplyProfileV1,
) -> BossRelicActionSupplyDeltaV1 {
    BossRelicActionSupplyDeltaV1 {
        opening_once_options: delta_u8(after.opening_once_options, before.opening_once_options),
        delayed_per_turn_sources: delta_u8(
            after.delayed_per_turn_sources,
            before.delayed_per_turn_sources,
        ),
        same_turn_burst_sources: delta_u8(
            after.same_turn_burst_sources,
            before.same_turn_burst_sources,
        ),
        triggered_repeatable_sources: delta_u8(
            after.triggered_repeatable_sources,
            before.triggered_repeatable_sources,
        ),
        additional_play_sources: delta_u8(
            after.additional_play_sources,
            before.additional_play_sources,
        ),
        cost_or_resource_compression_sources: delta_u8(
            after.cost_or_resource_compression_sources,
            before.cost_or_resource_compression_sources,
        ),
    }
}

fn delta_u8(after: u8, before: u8) -> i16 {
    i16::from(after) - i16::from(before)
}

fn boss_relic_policy_band_v1(action: &BossRelicPolicyActionV1) -> BossRelicPolicyBandV1 {
    let BossRelicPolicyActionV1::Pick {
        energy_gain,
        replaces_existing_relic,
        changed_deck_cards,
        followup,
        traits,
        added_debts,
        unresolved_debt_terms,
        compounding_debt_count,
        startup,
        action_supply,
        ..
    } = action
    else {
        return BossRelicPolicyBandV1::PreserveState;
    };

    if *energy_gain > 0
        && added_debts.is_empty()
        && *unresolved_debt_terms == 0
        && *compounding_debt_count == 0
    {
        return BossRelicPolicyBandV1::SupportedEnergy;
    }
    if *changed_deck_cards > 0
        || matches!(
            followup,
            BossRelicPolicyFollowupV1::Selection(
                RunPendingChoiceReason::Purge | RunPendingChoiceReason::TransformUpgraded
            )
        )
    {
        return BossRelicPolicyBandV1::DeckReconstruction;
    }
    if startup.setup_payment > 0
        || startup.immediate_survival > 0
        || startup.payoff_engine > 0
        || startup.strong_draw > 0
        || action_supply.opening_once_options > 0
        || action_supply.delayed_per_turn_sources > 0
        || action_supply.same_turn_burst_sources > 0
        || action_supply.triggered_repeatable_sources > 0
        || action_supply.cost_or_resource_compression_sources > 0
        || !traits.is_empty()
    {
        return BossRelicPolicyBandV1::StrategicAsset;
    }
    if *replaces_existing_relic {
        return BossRelicPolicyBandV1::StarterUpgrade;
    }
    if *energy_gain > 0 {
        return BossRelicPolicyBandV1::ConstrainedEnergy;
    }
    if matches!(followup, BossRelicPolicyFollowupV1::Reward) {
        return BossRelicPolicyBandV1::ConditionalGrowth;
    }
    if !added_debts.is_empty() || startup.combat_shape_risk > 0 || startup.setup_debt > 0 {
        return BossRelicPolicyBandV1::Liability;
    }
    BossRelicPolicyBandV1::ConditionalGrowth
}

fn compare_boss_relic_evidence(
    left: &BossRelicPolicyActionEvidenceV1,
    right: &BossRelicPolicyActionEvidenceV1,
) -> Ordering {
    left.band
        .cmp(&right.band)
        .then_with(|| compare_action_evidence(&left.action, &right.action))
        .then_with(|| left.surface_index.cmp(&right.surface_index))
}

fn compare_action_evidence(
    left: &BossRelicPolicyActionV1,
    right: &BossRelicPolicyActionV1,
) -> Ordering {
    match (left, right) {
        (
            BossRelicPolicyActionV1::Pick {
                energy_gain: left_energy,
                unresolved_debt_terms: left_unresolved,
                compounding_debt_count: left_compounding,
                startup: left_startup,
                ..
            },
            BossRelicPolicyActionV1::Pick {
                energy_gain: right_energy,
                unresolved_debt_terms: right_unresolved,
                compounding_debt_count: right_compounding,
                startup: right_startup,
                ..
            },
        ) => left_compounding
            .cmp(right_compounding)
            .then_with(|| left_unresolved.cmp(right_unresolved))
            .then_with(|| right_energy.cmp(left_energy))
            .then_with(|| right_startup.setup_payment.cmp(&left_startup.setup_payment))
            .then_with(|| {
                right_startup
                    .immediate_survival
                    .cmp(&left_startup.immediate_survival)
            })
            .then_with(|| {
                left_startup
                    .combat_shape_risk
                    .cmp(&right_startup.combat_shape_risk)
            }),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::eval::run_control::{build_decision_surface, RunControlConfig};
    use crate::runtime::combat::CombatCard;
    use crate::state::rewards::BossRelicChoiceState;

    fn boss_relic_session(relics: Vec<RelicId>) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::BossRelicSelect(BossRelicChoiceState::new(relics));
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

    #[test]
    fn every_exact_boss_relic_action_including_skip_has_positive_support() {
        let session = boss_relic_session(vec![
            RelicId::CoffeeDripper,
            RelicId::RunicPyramid,
            RelicId::PandorasBox,
        ]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_boss_relic_policy_decision_v1(&session, &legal)
            .expect("exact boss relic decision");

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
            .any(|entry| matches!(entry.action, BossRelicPolicyActionV1::Skip)));
    }

    #[test]
    fn nested_relic_effects_remain_real_followup_boundaries() {
        let session = boss_relic_session(vec![RelicId::Astrolabe, RelicId::EmptyCage]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_boss_relic_policy_decision_v1(&session, &legal)
            .expect("nested boss relic decision");

        assert!(decision.evidence.iter().any(|entry| matches!(
            entry.action,
            BossRelicPolicyActionV1::Pick {
                relic: RelicId::Astrolabe,
                followup: BossRelicPolicyFollowupV1::Selection(
                    RunPendingChoiceReason::TransformUpgraded
                ),
                ..
            }
        )));
        assert!(decision.evidence.iter().any(|entry| matches!(
            entry.action,
            BossRelicPolicyActionV1::Pick {
                relic: RelicId::EmptyCage,
                followup: BossRelicPolicyFollowupV1::Selection(RunPendingChoiceReason::Purge),
                ..
            }
        )));
    }

    #[test]
    fn pandora_uses_the_exact_transformed_deck_not_a_starter_count_prediction() {
        let mut session = boss_relic_session(vec![RelicId::PandorasBox]);
        session.run_state.master_deck = [
            CardId::Strike,
            CardId::Strike,
            CardId::Defend,
            CardId::Defend,
            CardId::Bash,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, card)| CombatCard::new(card, 20_000 + index as u32))
        .collect();
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_boss_relic_policy_decision_v1(&session, &legal).expect("Pandora decision");
        let pandora = decision
            .evidence
            .iter()
            .find(|entry| {
                matches!(
                    entry.action,
                    BossRelicPolicyActionV1::Pick {
                        relic: RelicId::PandorasBox,
                        ..
                    }
                )
            })
            .expect("Pandora evidence");

        assert!(matches!(
            pandora.action,
            BossRelicPolicyActionV1::Pick {
                changed_deck_cards: 4..,
                ..
            }
        ));
        assert_eq!(pandora.band, BossRelicPolicyBandV1::DeckReconstruction);
    }

    #[test]
    fn energy_constraints_are_typed_without_removing_the_action() {
        let session = boss_relic_session(vec![RelicId::CoffeeDripper, RelicId::FusionHammer]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision =
            exact_boss_relic_policy_decision_v1(&session, &legal).expect("energy relic decision");

        for relic in [RelicId::CoffeeDripper, RelicId::FusionHammer] {
            let evidence = decision
                .evidence
                .iter()
                .find(|entry| {
                    matches!(
                        entry.action,
                        BossRelicPolicyActionV1::Pick {
                            relic: found,
                            ..
                        } if found == relic
                    )
                })
                .expect("energy relic evidence");
            assert!(matches!(
                evidence.action,
                BossRelicPolicyActionV1::Pick { energy_gain: 1, .. }
            ));
        }
        assert_eq!(decision.prior.entries.len(), 3);
    }

    #[test]
    fn typed_energy_debt_is_not_mistaken_for_debt_free_energy() {
        let session = boss_relic_session(vec![
            RelicId::BustedCrown,
            RelicId::Astrolabe,
            RelicId::RunicPyramid,
        ]);
        let surface = build_decision_surface(&session);
        let legal = policy_candidates(&surface);
        let decision = exact_boss_relic_policy_decision_v1(&session, &legal)
            .expect("boss relic decision with reward-width debt");
        let busted_crown = decision
            .evidence
            .iter()
            .find(|entry| {
                matches!(
                    entry.action,
                    BossRelicPolicyActionV1::Pick {
                        relic: RelicId::BustedCrown,
                        ..
                    }
                )
            })
            .expect("Busted Crown evidence");

        assert!(matches!(
            busted_crown.action,
            BossRelicPolicyActionV1::Pick {
                ref added_debts,
                ..
            } if added_debts.contains(&RunDebtContractKindV1::RewardWidthDebt)
        ));
        assert_eq!(busted_crown.band, BossRelicPolicyBandV1::ConstrainedEnergy);

        let rank = |relic| {
            decision
                .prior
                .entries
                .iter()
                .position(|entry| {
                    decision.evidence.iter().any(|evidence| {
                        evidence.candidate_id == entry.candidate_id
                            && matches!(
                                evidence.action,
                                BossRelicPolicyActionV1::Pick { relic: found, .. }
                                    if found == relic
                            )
                    })
                })
                .expect("relic remains present in the positive prior")
        };
        assert!(rank(RelicId::Astrolabe) < rank(RelicId::BustedCrown));
        assert!(rank(RelicId::RunicPyramid) < rank(RelicId::BustedCrown));
    }
}
