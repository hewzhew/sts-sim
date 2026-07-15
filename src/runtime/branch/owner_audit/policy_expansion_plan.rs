use std::collections::BTreeSet;

use sts_simulator::ai::strategy::candidate_pressure_response::{
    assess_candidate_pressure_response, CandidatePressureResponse, StrategyCommitmentKind,
};
use sts_simulator::ai::strategy::challenger_choice_policy::{
    seed_challenger_repair_policy, select_challenger_repair_choice, PolicyChoiceSelection,
    PolicyRepairCandidateView, PolicySelectionClass,
};
use sts_simulator::ai::strategy::challenger_decision_context::ChallengerDecisionContext;
use sts_simulator::ai::strategy::challenger_policy_state::{
    ChallengerPolicyState, CommitmentStatus, PolicyProgress,
};
use sts_simulator::ai::strategy::decision_pipeline::{CandidateLane, DecisionCandidateKind};
use sts_simulator::ai::strategy::pressure_assessment::PressureAxis;
use sts_simulator::ai::strategy::role_saturation::LaneCap;
use sts_simulator::eval::run_control::DecisionCandidateKey;

use super::branch_policy_lane::BranchPolicyLane;
use super::owner_model::OwnerChoice;

#[derive(Clone, Debug)]
pub(super) struct PolicyExpansion {
    pub(super) choice_index: usize,
    pub(super) child_lane: BranchPolicyLane,
    pub(super) selection_evidence: PolicyExpansionEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PolicyExpansionClass {
    Production,
    OrdinaryChallenger,
    PressureRepair,
    CommitmentRepair,
}

#[derive(Clone, Debug)]
pub(super) struct PolicyExpansionEvidence {
    pub(super) class: PolicyExpansionClass,
    pub(super) matched_pressure_axes: Vec<PressureAxis>,
    pub(super) matched_commitments: Vec<StrategyCommitmentKind>,
    pub(super) original_lane: CandidateLane,
    pub(super) original_inspect_only: Option<String>,
    pub(super) overrode_reject: bool,
    pub(super) checkpoint_ref: String,
}

impl PolicyExpansionEvidence {
    pub(super) fn production(checkpoint_ref: impl Into<String>) -> Self {
        Self {
            class: PolicyExpansionClass::Production,
            matched_pressure_axes: Vec::new(),
            matched_commitments: Vec::new(),
            original_lane: CandidateLane::Mainline,
            original_inspect_only: None,
            overrode_reject: false,
            checkpoint_ref: checkpoint_ref.into(),
        }
    }

    fn ordinary_challenger(checkpoint_ref: impl Into<String>) -> Self {
        Self {
            class: PolicyExpansionClass::OrdinaryChallenger,
            matched_pressure_axes: Vec::new(),
            matched_commitments: Vec::new(),
            original_lane: CandidateLane::Mainline,
            original_inspect_only: None,
            overrode_reject: false,
            checkpoint_ref: checkpoint_ref.into(),
        }
    }
}

pub(super) fn plan_policy_expansions(
    lane: &BranchPolicyLane,
    context: &ChallengerDecisionContext,
    choices: &[OwnerChoice],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    if lane_budget == 0 {
        return Vec::new();
    }
    if matches!(lane, BranchPolicyLane::Baseline { .. })
        && choices
            .iter()
            .any(|choice| matches!(choice.key, Some(DecisionCandidateKey::BossRelicPick { .. })))
    {
        return plan_boss_relic_expansions(lane, choices, lane_budget, checkpoint_ref);
    }
    let production_index = choices.iter().position(OwnerChoice::auto_expand_allowed);
    let candidate_views = policy_candidate_views(choices);
    match lane {
        BranchPolicyLane::Baseline { .. } => plan_baseline_expansions(
            lane,
            context,
            production_index,
            &candidate_views,
            lane_budget,
            checkpoint_ref,
        ),
        BranchPolicyLane::Challenger { policy } => plan_existing_challenger(
            lane,
            policy,
            context,
            production_index,
            &candidate_views,
            checkpoint_ref,
        ),
    }
}

fn plan_boss_relic_expansions(
    lane: &BranchPolicyLane,
    choices: &[OwnerChoice],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    let Some(production_index) = choices.iter().position(|choice| {
        choice.auto_expand_allowed()
            && matches!(choice.key, Some(DecisionCandidateKey::BossRelicPick { .. }))
    }) else {
        return Vec::new();
    };

    let mut baseline_lane = lane.clone();
    let mut expansions = vec![PolicyExpansion {
        choice_index: production_index,
        child_lane: baseline_lane.clone(),
        selection_evidence: PolicyExpansionEvidence::production(checkpoint_ref),
    }];
    for (choice_index, choice) in choices.iter().enumerate() {
        if expansions.len() >= lane_budget || choice_index == production_index {
            continue;
        }
        if !choice.auto_expand_allowed()
            || !matches!(choice.key, Some(DecisionCandidateKey::BossRelicPick { .. }))
        {
            continue;
        }
        let mut issued_lane = baseline_lane.clone();
        let Some(lane_id) = issued_lane.issue_challenger_id() else {
            break;
        };
        baseline_lane = issued_lane;
        expansions.push(PolicyExpansion {
            choice_index,
            child_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(lane_id)),
            selection_evidence: PolicyExpansionEvidence::ordinary_challenger(checkpoint_ref),
        });
    }
    expansions[0].child_lane = baseline_lane;
    expansions
}

fn plan_baseline_expansions(
    lane: &BranchPolicyLane,
    context: &ChallengerDecisionContext,
    production_index: Option<usize>,
    candidates: &[PolicyRepairCandidateView],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    let Some(production_index) = production_index else {
        return Vec::new();
    };
    let mut baseline_lane = lane.clone();
    let production_candidate = candidates
        .iter()
        .find(|candidate| candidate.choice_index == production_index);
    let mut expansions = vec![PolicyExpansion {
        choice_index: production_index,
        child_lane: baseline_lane.clone(),
        selection_evidence: production_evidence(production_candidate, checkpoint_ref),
    }];
    let mut signatures = BTreeSet::new();
    for candidate in candidates {
        if expansions.len() >= lane_budget || candidate.choice_index == production_index {
            continue;
        }
        let mut issued_lane = baseline_lane.clone();
        let Some(lane_id) = issued_lane.issue_challenger_id() else {
            break;
        };
        let Some((policy, selection)) =
            seed_challenger_repair_policy(lane_id, checkpoint_ref, context, candidate)
        else {
            continue;
        };
        if !signatures.insert(policy_signature(&policy)) {
            continue;
        }
        baseline_lane = issued_lane;
        expansions.push(PolicyExpansion {
            choice_index: candidate.choice_index,
            child_lane: BranchPolicyLane::challenger(policy),
            selection_evidence: selection_evidence(candidate, &selection, checkpoint_ref),
        });
    }
    expansions[0].child_lane = baseline_lane;
    expansions
}

fn plan_existing_challenger(
    _lane: &BranchPolicyLane,
    policy: &ChallengerPolicyState,
    context: &ChallengerDecisionContext,
    production_index: Option<usize>,
    candidates: &[PolicyRepairCandidateView],
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion> {
    let mut contextual_policy = policy.clone();
    contextual_policy.reconcile_context(context);
    let selection = select_challenger_repair_choice(&contextual_policy, context, candidates);
    let selected_index = selection
        .as_ref()
        .map(|selection| selection.choice_index)
        .or(production_index);
    let Some(selected_index) = selected_index else {
        return Vec::new();
    };
    let selected_candidate = candidates
        .iter()
        .find(|candidate| candidate.choice_index == selected_index);
    if let (Some(selection), Some(candidate)) = (&selection, selected_candidate) {
        contextual_policy.merge_matched_pressure(&selection.matched_pressure);
        if production_index != Some(selected_index) {
            contextual_policy.record_divergence(checkpoint_ref, &candidate.response);
        }
        contextual_policy.satisfy_supported_requirements(&candidate.response);
    }
    contextual_policy.advance(PolicyProgress::DecisionBoundary);
    let evidence = match (&selection, selected_candidate) {
        (Some(selection), Some(candidate)) if production_index != Some(selected_index) => {
            selection_evidence(candidate, selection, checkpoint_ref)
        }
        _ => production_evidence(selected_candidate, checkpoint_ref),
    };
    vec![PolicyExpansion {
        choice_index: selected_index,
        child_lane: BranchPolicyLane::challenger(contextual_policy),
        selection_evidence: evidence,
    }]
}

fn policy_candidate_views(choices: &[OwnerChoice]) -> Vec<PolicyRepairCandidateView> {
    choices
        .iter()
        .enumerate()
        .filter_map(|(choice_index, choice)| {
            let decision = choice.annotation.candidate()?;
            let kind = decision.evaluation.candidate.kind;
            let response = candidate_card_identity(kind)
                .zip(decision.admission.as_ref())
                .map(|(card, admission)| assess_candidate_pressure_response(Some(card), admission))
                .unwrap_or_else(CandidatePressureResponse::default);
            let evaluation = &decision.evaluation;
            let inspect_only_reason = evaluation.inspect_only_reason().map(str::to_string);
            let hard_filtered = evaluation.lane == CandidateLane::Reject
                && inspect_only_reason.as_deref() != Some("candidate score rejected");
            let has_reject_cap = evaluation
                .adjudication
                .caps
                .iter()
                .any(|cap| cap.cap == LaneCap::Reject);
            Some(PolicyRepairCandidateView {
                choice_index,
                lane: evaluation.lane,
                raw_lane: evaluation.adjudication.raw_lane,
                auto_allowed: choice.auto_expand_allowed(),
                hard_filtered,
                has_reject_cap,
                inspect_only_reason,
                response,
            })
        })
        .collect()
}

fn candidate_card_identity(
    kind: DecisionCandidateKind,
) -> Option<(sts_simulator::content::cards::CardId, u8)> {
    match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades }
        | DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } => Some((card, upgrades)),
        _ => None,
    }
}

fn production_evidence(
    candidate: Option<&PolicyRepairCandidateView>,
    checkpoint_ref: &str,
) -> PolicyExpansionEvidence {
    let mut evidence = PolicyExpansionEvidence::production(checkpoint_ref);
    if let Some(candidate) = candidate {
        evidence.original_lane = candidate.lane;
        evidence.original_inspect_only = candidate.inspect_only_reason.clone();
    }
    evidence
}

fn selection_evidence(
    candidate: &PolicyRepairCandidateView,
    selection: &PolicyChoiceSelection,
    checkpoint_ref: &str,
) -> PolicyExpansionEvidence {
    let class = match selection.class {
        PolicySelectionClass::OrdinaryChallenger => PolicyExpansionClass::OrdinaryChallenger,
        PolicySelectionClass::PressureRepair => PolicyExpansionClass::PressureRepair,
        PolicySelectionClass::CommitmentRepair => PolicyExpansionClass::CommitmentRepair,
    };
    PolicyExpansionEvidence {
        class,
        matched_pressure_axes: selection
            .matched_pressure
            .iter()
            .map(|hypothesis| hypothesis.axis)
            .collect(),
        matched_commitments: selection.matched_commitments.clone(),
        original_lane: candidate.lane,
        original_inspect_only: candidate.inspect_only_reason.clone(),
        overrode_reject: selection.overrode_reject,
        checkpoint_ref: checkpoint_ref.to_string(),
    }
}

fn policy_signature(
    policy: &ChallengerPolicyState,
) -> (Vec<PressureAxis>, Vec<StrategyCommitmentKind>) {
    let mut axes = policy
        .active_pressure
        .iter()
        .map(|hypothesis| hypothesis.axis)
        .collect::<Vec<_>>();
    axes.sort();
    axes.dedup();
    let mut commitments = policy
        .commitments
        .iter()
        .filter(|commitment| commitment.status == CommitmentStatus::Active)
        .map(|commitment| commitment.kind)
        .collect::<Vec<_>>();
    commitments.sort();
    commitments.dedup();
    (axes, commitments)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::analysis::card_semantics::PayoffRequirement;
    use sts_simulator::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use sts_simulator::ai::strategy::challenger_decision_context::{
        challenger_decision_context, open_inventory_pressure, ChallengerDecisionContext,
    };
    use sts_simulator::ai::strategy::challenger_policy_state::{
        ChallengerPolicyState, CommitmentHorizon, CommitmentRequirement, CommitmentStatus,
        StrategyCommitment,
    };
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, CandidateLaneCap,
        CandidateLaneCapSource, DecisionCandidateIr, DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
    use sts_simulator::ai::strategy::deck_strategic_deficit::{
        DeckStrategicDeficitSummary, StrategicBurdenLevel, StrategicDeficitLevel,
    };
    use sts_simulator::ai::strategy::package_transition::PackageKind;
    use sts_simulator::ai::strategy::reward_admission::{
        skip_reward_admission, RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
    };
    use sts_simulator::ai::strategy::role_saturation::LaneCap;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::content::relics::RelicId;
    use sts_simulator::eval::run_control::{DecisionCandidateKey, RunDecisionAction};
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::state::run::RunState;

    use super::super::branch_policy_lane::BranchPolicyLane;
    use super::super::owner_model::{
        ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion,
    };

    fn candidate_choice(
        kind: DecisionCandidateKind,
        lane: CandidateLane,
        admission: RewardAdmission,
    ) -> OwnerChoice {
        let auto = lane != CandidateLane::Probe;
        OwnerChoice {
            key: None,
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: format!("{kind:?}"),
            annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                evaluation: CandidateEvaluation {
                    candidate: DecisionCandidateIr { kind },
                    lane,
                    adjudication: CandidateLaneAdjudication::uncapped(lane),
                    expansion: if auto {
                        ExpansionPlan::Auto
                    } else {
                        ExpansionPlan::InspectOnly("probe fixture")
                    },
                    scores: Vec::new(),
                },
                admission: Some(admission),
            }),
            expansion: if auto {
                OwnerChoiceExpansion::AutoAllowed
            } else {
                OwnerChoiceExpansion::InspectOnly("probe fixture")
            },
        }
    }

    fn skip_choice() -> OwnerChoice {
        candidate_choice(
            DecisionCandidateKind::CardRewardSkip,
            CandidateLane::Skip,
            skip_reward_admission(),
        )
    }

    fn boss_relic_choice(option_index: usize, relic: RelicId) -> OwnerChoice {
        OwnerChoice {
            key: Some(DecisionCandidateKey::BossRelicPick {
                option_index,
                relic,
            }),
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: format!("Take {relic:?}"),
            annotation: ChoiceAnnotation::None,
            expansion: OwnerChoiceExpansion::AutoAllowed,
        }
    }

    fn boss_relic_skip_choice() -> OwnerChoice {
        OwnerChoice {
            key: Some(DecisionCandidateKey::BossRelicSkip),
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: "Skip boss relic".to_string(),
            annotation: ChoiceAnnotation::None,
            expansion: OwnerChoiceExpansion::AutoAllowed,
        }
    }

    fn probe_card_choice(card: CardId, reasons: Vec<RewardAdmissionReason>) -> OwnerChoice {
        candidate_choice(
            DecisionCandidateKind::CardRewardPick { card, upgrades: 0 },
            CandidateLane::Probe,
            RewardAdmission {
                card: Some(card),
                class: RewardAdmissionClass::EngineSeed,
                reasons,
            },
        )
    }

    fn shop_leave_choice() -> OwnerChoice {
        OwnerChoice {
            key: None,
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: "Leave".to_string(),
            annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                evaluation: CandidateEvaluation {
                    candidate: DecisionCandidateIr {
                        kind: DecisionCandidateKind::ShopLeave,
                    },
                    lane: CandidateLane::Mainline,
                    adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                    expansion: ExpansionPlan::Auto,
                    scores: Vec::new(),
                },
                admission: None,
            }),
            expansion: OwnerChoiceExpansion::AutoAllowed,
        }
    }

    fn rejected_shop_card_choice(
        card: CardId,
        price: i32,
        reasons: Vec<RewardAdmissionReason>,
    ) -> OwnerChoice {
        OwnerChoice {
            key: None,
            action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
            label: format!("Buy {card:?}"),
            annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                evaluation: CandidateEvaluation {
                    candidate: DecisionCandidateIr {
                        kind: DecisionCandidateKind::ShopBuyCard {
                            card,
                            upgrades: 0,
                            price,
                        },
                    },
                    lane: CandidateLane::Reject,
                    adjudication: CandidateLaneAdjudication {
                        raw_lane: CandidateLane::Probe,
                        final_lane: CandidateLane::Reject,
                        caps: vec![CandidateLaneCap {
                            source: CandidateLaneCapSource::Acquisition,
                            cap: LaneCap::Reject,
                        }],
                    },
                    expansion: ExpansionPlan::InspectOnly("candidate score rejected"),
                    scores: Vec::new(),
                },
                admission: Some(RewardAdmission {
                    card: Some(card),
                    class: RewardAdmissionClass::BuildsSupportedPackage,
                    reasons,
                }),
            }),
            expansion: OwnerChoiceExpansion::InspectOnly("candidate score rejected"),
        }
    }

    fn adequate_facts() -> DeckStrategicDeficitSummary {
        DeckStrategicDeficitSummary {
            frontload_damage: StrategicDeficitLevel::Adequate,
            aoe_or_minion_control: StrategicDeficitLevel::Adequate,
            block_or_mitigation: StrategicDeficitLevel::Adequate,
            boss_scaling_plan: StrategicDeficitLevel::Adequate,
            deck_access: StrategicDeficitLevel::Adequate,
            energy_or_playability: StrategicDeficitLevel::Adequate,
            deck_burden: StrategicBurdenLevel::Clean,
            too_many_low_impact_attacks: false,
            opening_hand_pollution: false,
            severe_curse_burden: false,
        }
    }

    fn context_from_facts(facts: DeckStrategicDeficitSummary) -> ChallengerDecisionContext {
        let run = RunState::new(21, 0, false, "Ironclad");
        ChallengerDecisionContext {
            deck_plan: DeckPlanSnapshot::from_run_state(&run),
            gold: run.gold,
            current_pressure: open_inventory_pressure(facts),
            automatic_commitments: Vec::new(),
        }
    }

    #[test]
    fn baseline_keeps_production_choice_and_seeds_distinct_probe_challenger() {
        let mut facts = adequate_facts();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        let context = context_from_facts(facts);
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::PommelStrike,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            &context,
            &choices,
            3,
            "branch-0/step-0",
        );

        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].choice_index, 0);
        assert_eq!(plan[0].child_lane.label(), "baseline");
        assert_eq!(plan[1].choice_index, 1);
        assert_eq!(plan[1].child_lane.label(), "challenger-1");
    }

    #[test]
    fn boss_relic_picks_fill_three_policy_lanes_before_skip() {
        let choices = vec![
            boss_relic_choice(0, RelicId::BlackBlood),
            boss_relic_choice(1, RelicId::CoffeeDripper),
            boss_relic_choice(2, RelicId::PhilosopherStone),
            boss_relic_skip_choice(),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            &context_from_facts(adequate_facts()),
            &choices,
            4,
            "branch-29/step-29",
        );

        assert_eq!(
            plan.iter()
                .map(|item| item.choice_index)
                .collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(plan[0].child_lane.label(), "baseline");
        assert_eq!(plan[1].child_lane.label(), "challenger-1");
        assert_eq!(plan[2].child_lane.label(), "challenger-2");
        assert_eq!(
            plan[1].selection_evidence.class,
            PolicyExpansionClass::OrdinaryChallenger
        );
        assert_eq!(
            plan[2].selection_evidence.class,
            PolicyExpansionClass::OrdinaryChallenger
        );
    }

    #[test]
    fn existing_challenger_emits_only_one_later_choice() {
        let mut policy = ChallengerPolicyState::new(1);
        policy.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::DarkEmbrace,
                vec![
                    RewardAdmissionReason::Supports(PackageKind::Exhaust),
                    RewardAdmissionReason::Closes(PayoffRequirement::WantsEventStream(
                        sts_simulator::ai::analysis::card_semantics::CombatEvent::CardExhausted,
                    )),
                ],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::challenger(policy),
            &context_from_facts(adequate_facts()),
            &choices,
            3,
            "branch-2/step-4",
        );

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].choice_index, 1);
        assert_eq!(plan[0].child_lane.label(), "challenger-1");
    }

    #[test]
    fn semantically_equivalent_seed_candidates_do_not_consume_both_lanes() {
        let mut facts = adequate_facts();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        let context = context_from_facts(facts);
        let choices = vec![
            skip_choice(),
            probe_card_choice(
                CardId::PommelStrike,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
            probe_card_choice(
                CardId::Headbutt,
                vec![RewardAdmissionReason::FrontloadDamage],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            &context,
            &choices,
            3,
            "branch-0/step-0",
        );

        assert_eq!(
            plan.iter()
                .filter(|item| item.child_lane.challenger_policy().is_some())
                .count(),
            1
        );
    }

    #[test]
    fn baseline_leaves_while_challenger_takes_rejected_exhaust_payoff() {
        let mut run = RunState::new(22, 0, false, "Ironclad");
        run.act_num = 2;
        run.gold = 200;
        run.master_deck = vec![
            CombatCard::new(CardId::TrueGrit, 80_001),
            CombatCard::new(CardId::BurningPact, 80_002),
        ];
        let context = challenger_decision_context(&run);
        let choices = vec![
            shop_leave_choice(),
            rejected_shop_card_choice(
                CardId::FeelNoPain,
                75,
                vec![RewardAdmissionReason::Supports(PackageKind::Exhaust)],
            ),
        ];

        let plan = plan_policy_expansions(
            &BranchPolicyLane::default(),
            &context,
            &choices,
            3,
            "branch-0/step-0",
        );

        assert_eq!(plan[0].choice_index, 0);
        assert_eq!(plan[0].child_lane.label(), "baseline");
        assert_eq!(
            plan[0].selection_evidence.class,
            PolicyExpansionClass::Production
        );
        assert_eq!(plan[1].choice_index, 1);
        assert_eq!(plan[1].child_lane.label(), "challenger-1");
        assert_eq!(
            plan[1].selection_evidence.class,
            PolicyExpansionClass::CommitmentRepair
        );
        assert!(plan[1].selection_evidence.overrode_reject);
    }
}
