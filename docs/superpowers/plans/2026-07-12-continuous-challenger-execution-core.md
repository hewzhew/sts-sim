# Continuous Challenger Execution Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the existing evidence-only challenger primitives into at most two persistent challenger lanes that fork from the exact baseline state, make repeated non-combat decisions under their own policy memory, and resume from the ordinary frontier checkpoint without prefix replay.

**Architecture:** Keep legal boundary discovery and command application in the existing owner runtime. Add pure strategy functions that seed open low-confidence pressure hypotheses only from `Missing`/`Thin` inventory facts and rank already-legal candidate views. A runtime lane adapter assigns one baseline choice and at most two semantically distinct challenger choices, stores the lane on `Branch`, and serializes it through `frontier.json`; every lane then advances normally from its own cloned `RunControlSession`.

**Tech Stack:** Rust, serde/serde_json, existing owner-audit frontier/checkpoint runtime, existing pressure and challenger-policy primitives.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Do not use subagents for this repository session.
- Never run `cargo clean`.
- Follow red-green TDD with focused library tests and frequent local commits.
- Keep run-control free of strategy rules.
- Do not infer a damage or defense cause from HP loss.
- `Adequate` and `Surplus` static inventory labels must never certify a pressure axis as covered.
- The baseline must retain the current production ordering and legality behavior.
- A challenger may override only a final `Probe` disposition; it may not execute a `Reject` candidate or bypass an owner/legal-input check.
- A lane chooses one action per non-combat boundary; it does not branch again for every candidate.
- Keep at most one baseline and two challenger identities for the entire evaluation trajectory.
- This plan does not compare or promote trajectories; paired comparison is the next reviewed slice.
- Run the full library and `architecture_runtime_boundaries` suites only at the completion checkpoint.

---

### Task 1: Pure Pressure Seeding And Challenger Choice Policy

**Files:**
- Create: `src/ai/strategy/challenger_choice_policy.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Consumes: `DeckPlanSnapshot`, `CandidateLane`, `CandidatePressureResponse`, and `ChallengerPolicyState`.
- Produces: `PolicyCandidateView`, `open_inventory_pressure`, `seed_challenger_policy`, and `select_challenger_choice`.
- Boundary: static inventory may open a low-confidence hypothesis only for `Missing`/`Thin`; it never closes pressure and never consumes HP loss as causal evidence.

- [ ] **Step 1: Write failing policy tests**

Create the module with these public API tests first:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use crate::ai::strategy::challenger_policy_state::{
        CommitmentHorizon, CommitmentRequirement, CommitmentStatus, StrategyCommitment,
    };
    use crate::ai::strategy::deck_strategic_deficit::{
        DeckStrategicDeficitSummary, StrategicBurdenLevel, StrategicDeficitLevel,
    };

    fn summary() -> DeckStrategicDeficitSummary {
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

    #[test]
    fn static_adequacy_never_emits_covered_pressure() {
        let hypotheses = open_inventory_pressure(summary());
        assert!(hypotheses.is_empty());
    }

    #[test]
    fn missing_tempo_and_delay_remain_distinct_open_hypotheses() {
        let mut facts = summary();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        facts.block_or_mitigation = StrategicDeficitLevel::Thin;

        let hypotheses = open_inventory_pressure(facts);

        assert!(hypotheses.iter().any(|item| item.axis == PressureAxis::ResolutionTempo));
        assert!(hypotheses.iter().any(|item| item.axis == PressureAxis::DelayCapacity));
        assert!(hypotheses.iter().all(|item| item.coverage == PressureCoverage::Open));
    }

    #[test]
    fn seed_keeps_only_pressure_axes_the_candidate_can_answer() {
        let mut facts = summary();
        facts.frontload_damage = StrategicDeficitLevel::Missing;
        facts.block_or_mitigation = StrategicDeficitLevel::Thin;
        let response = CandidatePressureResponse {
            axes: vec![PressureAxis::DelayCapacity],
            ..CandidatePressureResponse::default()
        };

        let state = seed_challenger_policy(1, "branch-0/step-0", facts, &response)
            .expect("delay response should seed a challenger");

        assert_eq!(state.active_pressure.len(), 1);
        assert_eq!(state.active_pressure[0].axis, PressureAxis::DelayCapacity);
        assert_eq!(state.divergence_count, 1);
    }

    #[test]
    fn active_commitment_support_beats_baseline_order_on_later_boundary() {
        let mut policy = ChallengerPolicyState::new(1);
        policy.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });
        let candidates = vec![
            PolicyCandidateView {
                choice_index: 0,
                lane: CandidateLane::Skip,
                auto_allowed: true,
                response: CandidatePressureResponse::default(),
            },
            PolicyCandidateView {
                choice_index: 1,
                lane: CandidateLane::Probe,
                auto_allowed: false,
                response: CandidatePressureResponse {
                    supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
                    ..CandidatePressureResponse::default()
                },
            },
        ];

        assert_eq!(select_challenger_choice(&policy, &candidates), Some(1));
    }

    #[test]
    fn reject_candidate_cannot_become_a_challenger_action() {
        let policy = ChallengerPolicyState::new(1);
        let candidates = vec![PolicyCandidateView {
            choice_index: 3,
            lane: CandidateLane::Reject,
            auto_allowed: false,
            response: CandidatePressureResponse {
                axes: vec![PressureAxis::ResolutionTempo],
                ..CandidatePressureResponse::default()
            },
        }];

        assert_eq!(select_challenger_choice(&policy, &candidates), None);
    }
}
```

- [ ] **Step 2: Register the module and verify red**

Add `pub mod challenger_choice_policy;` to `src/ai/strategy/mod.rs` and run:

```powershell
cargo test --lib challenger_choice_policy::tests
```

Expected: compilation fails because the policy types and functions do not exist.

- [ ] **Step 3: Implement the pure policy**

Define the view and inventory mapping:

```rust
use crate::ai::strategy::candidate_pressure_response::CandidatePressureResponse;
use crate::ai::strategy::challenger_policy_state::ChallengerPolicyState;
use crate::ai::strategy::decision_pipeline::CandidateLane;
use crate::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficitSummary, StrategicDeficitLevel,
};
use crate::ai::strategy::pressure_assessment::{
    EvidenceConfidence, PressureAxis, PressureCoverage, PressureEvidence,
    PressureEvidenceSource, PressureHypothesis,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCandidateView {
    pub choice_index: usize,
    pub lane: CandidateLane,
    pub auto_allowed: bool,
    pub response: CandidatePressureResponse,
}

pub fn open_inventory_pressure(
    facts: DeckStrategicDeficitSummary,
) -> Vec<PressureHypothesis> {
    let mut axes = Vec::new();
    push_if_open(&mut axes, facts.frontload_damage, PressureAxis::ResolutionTempo, "frontload inventory is missing or thin");
    push_if_open(&mut axes, facts.aoe_or_minion_control, PressureAxis::MultiTargetControl, "multi-target inventory is missing or thin");
    push_if_open(&mut axes, facts.block_or_mitigation, PressureAxis::DelayCapacity, "delay inventory is missing or thin");
    push_if_open(&mut axes, facts.boss_scaling_plan, PressureAxis::GrowthHorizon, "scaling inventory is missing or thin");
    if is_open(facts.deck_access) || is_open(facts.energy_or_playability) {
        axes.push(open_hypothesis(PressureAxis::Deployability, "access or playability inventory is missing or thin"));
    }
    axes.sort_by_key(|item| item.axis);
    axes.dedup_by_key(|item| item.axis);
    axes
}

fn is_open(level: StrategicDeficitLevel) -> bool {
    matches!(level, StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin)
}

fn push_if_open(
    hypotheses: &mut Vec<PressureHypothesis>,
    level: StrategicDeficitLevel,
    axis: PressureAxis,
    label: &'static str,
) {
    if is_open(level) {
        hypotheses.push(open_hypothesis(axis, label));
    }
}

fn open_hypothesis(axis: PressureAxis, label: &'static str) -> PressureHypothesis {
    PressureHypothesis {
        axis,
        coverage: PressureCoverage::Open,
        confidence: EvidenceConfidence::Low,
        supporting_evidence: vec![PressureEvidence {
            source: PressureEvidenceSource::DeckCapability,
            label: label.to_string(),
        }],
        contradicting_evidence: Vec::new(),
    }
}
```

Implement seeding and selection. The selector admits an ordinary production-safe action or a `Probe` with non-empty semantic response, but never a `Reject`:

```rust
pub fn seed_challenger_policy(
    lane_id: u8,
    checkpoint_ref: impl Into<String>,
    facts: DeckStrategicDeficitSummary,
    response: &CandidatePressureResponse,
) -> Option<ChallengerPolicyState> {
    let active_pressure = open_inventory_pressure(facts)
        .into_iter()
        .filter(|hypothesis| response.axes.contains(&hypothesis.axis))
        .collect::<Vec<_>>();
    if active_pressure.is_empty() {
        return None;
    }
    let mut policy = ChallengerPolicyState::new(lane_id);
    policy.active_pressure = active_pressure;
    policy.record_divergence(checkpoint_ref, response);
    Some(policy)
}

pub fn select_challenger_choice(
    policy: &ChallengerPolicyState,
    candidates: &[PolicyCandidateView],
) -> Option<usize> {
    candidates
        .iter()
        .filter(|candidate| candidate_is_eligible(candidate))
        .min_by_key(|candidate| {
            let supports_commitment =
                policy.candidate_supports_active_commitment(&candidate.response);
            let answers_pressure = policy.active_pressure.iter().any(|hypothesis| {
                candidate.response.axes.contains(&hypothesis.axis)
            });
            (
                u8::from(!supports_commitment),
                u8::from(!answers_pressure),
                candidate.choice_index,
            )
        })
        .map(|candidate| candidate.choice_index)
}

fn candidate_is_eligible(candidate: &PolicyCandidateView) -> bool {
    candidate.auto_allowed
        || (candidate.lane == CandidateLane::Probe
            && (!candidate.response.axes.is_empty()
                || !candidate.response.supports_commitments.is_empty()
                || !candidate.response.opens_commitments.is_empty()))
}
```

- [ ] **Step 4: Run focused tests and commit**

```powershell
cargo test --lib challenger_choice_policy::tests
cargo fmt --all
git add src/ai/strategy/mod.rs src/ai/strategy/challenger_choice_policy.rs
git commit -m "feat: add challenger choice policy"
```

Expected: five tests pass.

---

### Task 2: Serializable Runtime Lane Identity

**Files:**
- Create: `src/runtime/branch/owner_audit/branch_policy_lane.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/branch_model.rs`
- Modify: `src/runtime/branch/owner_audit/branch_runtime.rs`

**Interfaces:**
- Produces: `BranchPolicyLane`, baseline issuance accounting, stable lane labels, and challenger accessors.
- Contract: at most two challenger identities are issued over the baseline trajectory.

- [ ] **Step 1: Write failing lane-state tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_issues_at_most_two_challenger_identities() {
        let mut lane = BranchPolicyLane::default();
        assert_eq!(lane.issue_challenger_id(), Some(1));
        assert_eq!(lane.issue_challenger_id(), Some(2));
        assert_eq!(lane.issue_challenger_id(), None);
    }

    #[test]
    fn lane_identity_round_trips_through_json() {
        let lane = BranchPolicyLane::challenger(ChallengerPolicyState::new(2));
        let json = serde_json::to_string(&lane).unwrap();
        let restored: BranchPolicyLane = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, lane);
        assert_eq!(restored.label(), "challenger-2");
    }
}
```

- [ ] **Step 2: Register the module and verify red**

Run `cargo test --lib branch_policy_lane::tests` and expect missing-type failures.

- [ ] **Step 3: Implement lane identity and attach it to `Branch`**

```rust
use serde::{Deserialize, Serialize};
use sts_simulator::ai::strategy::challenger_policy_state::ChallengerPolicyState;

const MAX_CHALLENGER_IDENTITIES: u8 = 2;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum BranchPolicyLane {
    Baseline { issued_challengers: u8 },
    Challenger { policy: ChallengerPolicyState },
}

impl Default for BranchPolicyLane {
    fn default() -> Self {
        Self::Baseline { issued_challengers: 0 }
    }
}

impl BranchPolicyLane {
    pub(super) fn challenger(policy: ChallengerPolicyState) -> Self {
        Self::Challenger { policy }
    }

    pub(super) fn issue_challenger_id(&mut self) -> Option<u8> {
        let Self::Baseline { issued_challengers } = self else { return None; };
        if *issued_challengers >= MAX_CHALLENGER_IDENTITIES { return None; }
        *issued_challengers += 1;
        Some(*issued_challengers)
    }

    pub(super) fn challenger_policy(&self) -> Option<&ChallengerPolicyState> {
        match self { Self::Challenger { policy } => Some(policy), _ => None }
    }

    pub(super) fn label(&self) -> String {
        match self {
            Self::Baseline { .. } => "baseline".to_string(),
            Self::Challenger { policy } => format!("challenger-{}", policy.lane_id),
        }
    }
}
```

Add `pub(super) policy_lane: BranchPolicyLane` to `Branch` and initialize the root with `BranchPolicyLane::default()`.

- [ ] **Step 4: Fix all test branch constructors and commit**

Use `policy_lane: BranchPolicyLane::default()` in existing test fixtures. Run:

```powershell
cargo test --lib branch_policy_lane::tests
cargo test --lib runtime_initial_frontier_starts_one_root_branch
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/branch_policy_lane.rs src/runtime/branch/owner_audit/branch_model.rs src/runtime/branch/owner_audit/branch_runtime.rs
git commit -m "feat: attach policy lanes to branches"
```

---

### Task 3: Lane-Aware Expansion Planning

**Files:**
- Create: `src/runtime/branch/owner_audit/policy_expansion_plan.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/branch_frontier.rs`

**Interfaces:**
- Produces: `PolicyExpansion { choice_index, child_lane }` and `plan_policy_expansions`.
- Consumes: sorted owner choices, current branch lane, and current deck plan.
- Contract: baseline chooses the first production-auto choice; challengers choose exactly one action; a baseline may additionally seed distinct challengers until its lifetime issuance limit reaches two.

- [ ] **Step 1: Write failing planner tests**

Tests use synthetic `OwnerChoice` values with real `CandidateEvaluation` and `RewardAdmission` annotations. Put this exact helper above the tests:

```rust
fn candidate_choice(
    kind: DecisionCandidateKind,
    lane: CandidateLane,
    admission: Option<RewardAdmission>,
) -> OwnerChoice {
    let auto = lane != CandidateLane::Probe;
    OwnerChoice {
        key: None,
        action: RunControlCommand::Noop,
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
            admission,
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
        Some(skip_reward_admission()),
    )
}

fn probe_card_choice(card: CardId) -> OwnerChoice {
    candidate_choice(
        DecisionCandidateKind::CardRewardPick { card, upgrades: 0 },
        CandidateLane::Probe,
        Some(assess_reward_admission(&[], card)),
    )
}

fn exhaust_support_choice() -> OwnerChoice {
    candidate_choice(
        DecisionCandidateKind::CardRewardPick {
            card: CardId::DarkEmbrace,
            upgrades: 0,
        },
        CandidateLane::Probe,
        Some(RewardAdmission {
            card: Some(CardId::DarkEmbrace),
            class: RewardAdmissionClass::BuildsSupportedPackage,
            reasons: vec![RewardAdmissionReason::Supports(PackageKind::Exhaust)],
        }),
    )
}

fn open_tempo_facts() -> DeckStrategicDeficitSummary {
    let mut facts = fully_adequate_summary_for_test();
    facts.frontload_damage = StrategicDeficitLevel::Missing;
    facts
}

fn fully_adequate_summary_for_test() -> DeckStrategicDeficitSummary {
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
```

Then assert:

```rust
#[test]
fn baseline_keeps_production_choice_and_seeds_distinct_probe_challenger() {
    let lane = BranchPolicyLane::default();
    let choices = vec![skip_choice(), probe_card_choice(CardId::PommelStrike)];

    let plan = plan_policy_expansions(
        &lane,
        open_tempo_facts(),
        &choices,
        3,
        "branch-0/step-0",
    );

    assert_eq!(plan.len(), 2);
    assert_eq!(plan[0].choice_index, 0);
    assert!(matches!(plan[0].child_lane, BranchPolicyLane::Baseline { .. }));
    assert_eq!(plan[1].choice_index, 1);
    assert_eq!(plan[1].child_lane.label(), "challenger-1");
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
    let lane = BranchPolicyLane::challenger(policy);
    let choices = vec![skip_choice(), exhaust_support_choice()];

    let plan = plan_policy_expansions(
        &lane,
        fully_adequate_summary_for_test(),
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
    let lane = BranchPolicyLane::default();
    let choices = vec![
        skip_choice(),
        probe_card_choice(CardId::PommelStrike),
        probe_card_choice(CardId::Headbutt),
    ];

    let plan = plan_policy_expansions(
        &lane,
        open_tempo_facts(),
        &choices,
        3,
        "branch-0/step-0",
    );

    assert_eq!(plan.iter().filter(|item| item.child_lane.challenger_policy().is_some()).count(), 1);
}
```

- [ ] **Step 2: Verify red**

Run `cargo test --lib policy_expansion_plan::tests` and expect missing planner failures.

- [ ] **Step 3: Implement choice adaptation and planning**

Define:

```rust
#[derive(Clone, Debug)]
pub(super) struct PolicyExpansion {
    pub(super) choice_index: usize,
    pub(super) child_lane: BranchPolicyLane,
}

pub(super) fn plan_policy_expansions(
    lane: &BranchPolicyLane,
    facts: DeckStrategicDeficitSummary,
    choices: &[OwnerChoice],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion>
```

Implementation rules:

1. `production_index` is the first `choice.auto_expand_allowed()` index.
2. A challenger candidate view is built only from `ChoiceAnnotation::Candidate`; card identity is `CardRewardPick` or `ShopBuyCard`, and response comes from `assess_candidate_pressure_response`.
3. `Reject` remains ineligible. `Probe` may be considered only when response is non-empty.
4. Baseline output starts with its production choice and a cloned baseline lane whose issuance count is updated for every emitted challenger.
5. Seed candidates exclude `production_index`, call `seed_challenger_policy(lane_id, checkpoint_ref, facts, response)`, and are deduplicated by the existing `ChallengerSignature` components: pressure axes plus active commitments. Stop at `lane_budget` and two lifetime identities.
6. Existing challenger output calls `select_challenger_choice`; if it returns `None`, fall back to `production_index`. Clone policy state, call `record_divergence` only when the selected index differs from production, and advance `PolicyProgress::DecisionBoundary`.

- [ ] **Step 4: Replace generic masks with policy plans**

Change `branch_frontier::expansion_masks` to accept a plan per branch and derive the boolean masks used only for trace rendering:

```rust
pub(super) fn expansion_masks(
    work: &[(Branch, bool, Vec<OwnerChoice>)],
    max_branches: usize,
) -> (Vec<Vec<PolicyExpansion>>, Vec<Vec<bool>>)
```

The total number of emitted expansions across all branches must not exceed `max_branches`. Do not use `recent_expanded_keys`; persistent lane identity replaces the old fairness heuristic.

- [ ] **Step 5: Run focused tests and commit**

```powershell
cargo test --lib policy_expansion_plan::tests
cargo test --lib branch_frontier::tests
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/policy_expansion_plan.rs src/runtime/branch/owner_audit/branch_frontier.rs
git commit -m "feat: plan persistent policy lane expansions"
```

---

### Task 4: Execute One Choice Per Policy Lane

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_generation.rs`
- Modify: `src/runtime/branch/owner_audit/branch_generation_step.rs`
- Modify: `src/runtime/branch/owner_audit/owner_choice_expander.rs`
- Modify: `src/runtime/branch/owner_audit/branch_path.rs`

**Interfaces:**
- Consumes: `Vec<PolicyExpansion>` instead of a behavior-defining boolean mask.
- Produces: child branches with the planned `BranchPolicyLane` and path evidence naming the lane used for the decision.
- Boundary: every child still clones the exact parent session before applying the typed owner command and still advances through the ordinary runner.

- [ ] **Step 1: Write failing execution tests**

Add a focused test around a parent branch and two `Noop` choices:

```rust
fn test_branch() -> Branch {
    Branch {
        id: 0,
        parent_id: None,
        path: Vec::new(),
        session: RunControlSession::new(RunControlConfig::default()),
        status: BranchStatus::AwaitingAuto {
            boundary: "test".to_string(),
            reason: "test fixture".to_string(),
        },
        policy_lane: BranchPolicyLane::default(),
        combat_portfolio: None,
        auto_steps: Vec::new(),
        combat_search: Vec::new(),
        combat_search_history: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    }
}

fn plain_choice(label: &str) -> OwnerChoice {
    OwnerChoice {
        key: None,
        action: RunControlCommand::Noop,
        label: label.to_string(),
        annotation: ChoiceAnnotation::None,
        expansion: OwnerChoiceExpansion::AutoAllowed,
    }
}

fn sample_args() -> Args {
    Args {
        seed: 1,
        ascension: 0,
        objective: RunObjective::FirstVictory,
        generations: 2,
        max_branches: 3,
        auto_ops: 1,
        search_nodes: 1,
        search_ms: 1,
        rescue_search_nodes: 1,
        rescue_search_ms: 1,
        boss_search_nodes: 1,
        boss_search_ms: 1,
        wall_ms: None,
        checkpoint_before_combat_portfolio: false,
        shop_boss_preview_bundle_limit: 0,
        shop_boss_preview_target_floor: None,
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    }
}

#[test]
fn planned_children_clone_one_exact_parent_and_keep_distinct_lane_identity() {
    let parent = test_branch();
    let choices = vec![plain_choice("baseline"), plain_choice("challenger")];
    let plans = vec![
        PolicyExpansion { choice_index: 0, child_lane: BranchPolicyLane::default() },
        PolicyExpansion { choice_index: 1, child_lane: BranchPolicyLane::challenger(ChallengerPolicyState::new(1)) },
    ];
    let parent_checkpoint = serde_json::to_value(
        RunControlSessionCheckpointV1::from_session(&parent.session)
    ).unwrap();

    let mut next_branch_id = 1;
    let children = expand_registered_owner(
        &parent,
        sample_args(),
        RunDeadline::new(Instant::now(), None),
        &choices,
        &plans,
        &mut next_branch_id,
    );

    assert_eq!(children.len(), 2);
    assert_eq!(children[0].policy_lane.label(), "baseline");
    assert_eq!(children[1].policy_lane.label(), "challenger-1");
    assert_eq!(parent_checkpoint, serde_json::to_value(
        RunControlSessionCheckpointV1::from_session(&parent.session)
    ).unwrap());
}
```

The assertion guards the common origin against mutation; existing command application tests continue to guard child semantics.

- [ ] **Step 2: Verify red**

Run `cargo test --lib planned_children_clone_one_exact_parent_and_keep_distinct_lane_identity` and expect signature/field failures.

- [ ] **Step 3: Wire the expansion plan through generation**

- `PreparedGeneration` stores `Vec<Vec<PolicyExpansion>>` and a separately derived trace mask.
- `advance_branch_work` receives both the trace mask and policy expansions.
- `expand_registered_owner` iterates policy expansions, looks up the referenced choice, clones the parent session, applies the command, and assigns `policy_lane: expansion.child_lane` to the child.
- Remove the unconditional shop-preview bundle child expansion from challenger mode. Preview bundles remain available only when `args.shop_boss_preview_bundle_limit > 0` and the parent lane is baseline; they receive the baseline lane identity.

- [ ] **Step 4: Record lane identity on each path step**

Add to `BranchPathStep`:

```rust
#[serde(default)]
pub(super) policy_lane: String,
```

Populate it from the planned child lane label. Legacy path JSON uses the empty default without failing to load.

- [ ] **Step 5: Run focused tests and commit**

```powershell
cargo test --lib owner_choice_expander::tests
cargo test --lib branch_path::tests
git add src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/branch_generation_step.rs src/runtime/branch/owner_audit/owner_choice_expander.rs src/runtime/branch/owner_audit/branch_path.rs
git commit -m "feat: execute persistent policy lanes"
```

---

### Task 5: Lane-Preserving Frontier Retention

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_frontier.rs`

**Interfaces:**
- Contract: retain the baseline whenever present; merge semantically equivalent challengers; retain at most two distinct challenger signatures; use progress/HP only to choose between equivalent lanes, never to discard a distinct live hypothesis.

- [ ] **Step 1: Write failing retention tests**

```rust
fn branch_with_lane(policy_lane: BranchPolicyLane, hp: i32) -> Branch {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.current_hp = hp;
    session.run_state.max_hp = 80;
    Branch {
        id: hp.max(0) as usize,
        parent_id: None,
        path: Vec::new(),
        session,
        status: BranchStatus::AwaitingAuto {
            boundary: "test".to_string(),
            reason: "retention fixture".to_string(),
        },
        policy_lane,
        combat_portfolio: None,
        auto_steps: Vec::new(),
        combat_search: Vec::new(),
        combat_search_history: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    }
}

fn baseline_branch(hp: i32) -> Branch {
    branch_with_lane(BranchPolicyLane::default(), hp)
}

fn challenger_branch(lane_id: u8, axis: PressureAxis, hp: i32) -> Branch {
    let mut policy = ChallengerPolicyState::new(lane_id);
    policy.active_pressure.push(PressureHypothesis {
        axis,
        coverage: PressureCoverage::Open,
        confidence: EvidenceConfidence::Low,
        supporting_evidence: Vec::new(),
        contradicting_evidence: Vec::new(),
    });
    branch_with_lane(BranchPolicyLane::challenger(policy), hp)
}

#[test]
fn lower_hp_baseline_is_not_dropped_for_healthier_challenger() {
    let mut frontier = VecDeque::from([
        challenger_branch(1, PressureAxis::DelayCapacity, 70),
        baseline_branch(20),
    ]);

    retain_frontier(&mut frontier, 2);

    assert!(frontier.iter().any(|branch| branch.policy_lane.label() == "baseline"));
}

#[test]
fn equivalent_challengers_merge_but_distinct_axes_survive() {
    let mut frontier = VecDeque::from([
        baseline_branch(50),
        challenger_branch(1, PressureAxis::ResolutionTempo, 30),
        challenger_branch(2, PressureAxis::ResolutionTempo, 45),
        challenger_branch(3, PressureAxis::DelayCapacity, 25),
    ]);

    retain_frontier(&mut frontier, 3);

    assert_eq!(frontier.len(), 3);
    assert!(frontier.iter().any(|branch| branch.policy_lane.label() == "baseline"));
    assert!(frontier.iter().any(|branch| branch.session.run_state.current_hp == 45));
    assert!(frontier.iter().any(|branch| branch.policy_lane.challenger_policy().is_some_and(|policy| {
        policy.active_pressure.iter().any(|item| item.axis == PressureAxis::DelayCapacity)
    })));
}
```

- [ ] **Step 2: Verify red**

Run `cargo test --lib branch_frontier::tests` and confirm the old HP-first retention fails the baseline assertion.

- [ ] **Step 3: Implement lane-aware retention**

Partition baseline and challenger branches. Keep the strongest branch only within an equal challenger signature, using the existing status/act/floor/HP key. Retain the baseline first, then up to `min(limit - 1, 2)` distinct challengers. If `limit == 1`, baseline-only behavior exactly matches the current single-mainline contract.

- [ ] **Step 4: Run focused tests and commit**

```powershell
cargo test --lib branch_frontier::tests
git add src/runtime/branch/owner_audit/branch_frontier.rs
git commit -m "fix: preserve distinct policy lanes in frontier"
```

---

### Task 6: Checkpoint And Artifact Persistence

**Files:**
- Modify: `src/runtime/branch/owner_audit/frontier_checkpoint.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`

**Interfaces:**
- Persists: branch policy lane, challenger memory, divergence count, and last checkpoint reference.
- Compatibility: a legacy checkpoint without `policy_lane` restores as baseline.

- [ ] **Step 1: Write failing checkpoint tests**

```rust
fn checkpoint_args() -> Args {
    Args {
        seed: 9,
        ascension: 0,
        objective: RunObjective::FirstVictory,
        generations: 2,
        max_branches: 3,
        auto_ops: 1,
        search_nodes: 1,
        search_ms: 1,
        rescue_search_nodes: 1,
        rescue_search_ms: 1,
        boss_search_nodes: 1,
        boss_search_ms: 1,
        wall_ms: None,
        checkpoint_before_combat_portfolio: false,
        shop_boss_preview_bundle_limit: 0,
        shop_boss_preview_target_floor: None,
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    }
}

fn checkpoint_branch(policy_lane: BranchPolicyLane) -> Branch {
    Branch {
        id: 1,
        parent_id: Some(0),
        path: Vec::new(),
        session: RunControlSession::new(RunControlConfig {
            seed: 9,
            ..RunControlConfig::default()
        }),
        status: BranchStatus::AwaitingAuto {
            boundary: "test".to_string(),
            reason: "checkpoint fixture".to_string(),
        },
        policy_lane,
        combat_portfolio: None,
        auto_steps: Vec::new(),
        combat_search: Vec::new(),
        combat_search_history: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    }
}

#[test]
fn challenger_policy_survives_frontier_checkpoint_round_trip() {
    let path = std::env::temp_dir().join("branch_tiny_challenger_policy_round_trip.json");
    let mut policy = ChallengerPolicyState::new(1);
    policy.record_divergence("a1f5", &CandidatePressureResponse::default());
    policy.record_divergence("a1f7", &CandidatePressureResponse::default());
    let branch = checkpoint_branch(BranchPolicyLane::challenger(policy));
    let frontier = VecDeque::from([branch]);

    save(&path, checkpoint_args(), 2, 3, &frontier).unwrap();
    let (restored, _) = load(&path).unwrap().into_frontier().unwrap();

    let restored_policy = restored.front().unwrap().policy_lane.challenger_policy().unwrap();
    assert_eq!(restored_policy.divergence_count, 2);
    assert_eq!(restored_policy.last_checkpoint_ref.as_deref(), Some("a1f7"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn legacy_checkpoint_branch_without_policy_lane_defaults_to_baseline() {
    let path = std::env::temp_dir().join("branch_tiny_legacy_policy_lane_default.json");
    let frontier = VecDeque::from([checkpoint_branch(BranchPolicyLane::default())]);
    save(&path, checkpoint_args(), 2, 3, &frontier).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    value["frontier"][0]
        .as_object_mut()
        .unwrap()
        .remove("policy_lane");
    std::fs::write(&path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    let (frontier, _) = load(&path).unwrap().into_frontier().unwrap();
    assert_eq!(frontier.front().unwrap().policy_lane.label(), "baseline");
    let _ = std::fs::remove_file(path);
}
```

- [ ] **Step 2: Verify red**

Run `cargo test --lib frontier_checkpoint::tests` and expect missing persistence failures.

- [ ] **Step 3: Persist and expose lane state**

Add to `BranchCheckpoint`:

```rust
#[serde(default)]
policy_lane: BranchPolicyLane,
```

Copy it in `from_branch` and `into_branch`. Add `"policy_lane": &branch.policy_lane` to `branch_summary_value`, `result_value`, and `path_value` so durable capsule evidence can distinguish trajectories without reopening the binary checkpoint.

- [ ] **Step 4: Run focused tests and commit**

```powershell
cargo test --lib frontier_checkpoint::tests
cargo test --lib run_capsule_format::tests
git add src/runtime/branch/owner_audit/frontier_checkpoint.rs src/runtime/branch/owner_audit/run_capsule_format.rs
git commit -m "feat: persist challenger policy lanes"
```

---

### Task 7: Two-Decision Resume Smoke Contract

**Files:**
- Create: `src/runtime/branch/owner_audit/challenger_execution_smoke.rs`
- Modify: `src/runtime/branch/owner_audit.rs`

**Interfaces:**
- Verifies: one challenger can diverge twice, retain its own deck/session identity, persist to a checkpoint, and resume without replay from run start.

- [ ] **Step 1: Write the smoke test**

Use this deterministic reward fixture; it preserves the same session and only installs the next visible boundary, so it never replays from Neow:

```rust
fn install_card_reward(session: &mut RunControlSession, cards: Vec<RewardCard>) {
    let mut reward = RewardState::new();
    reward.items = vec![RewardItem::Card {
        cards: cards.clone(),
    }];
    reward.pending_card_choice = Some(cards);
    reward.pending_card_reward_index = Some(0);
    session.engine_state = EngineState::RewardScreen(reward);
}

fn forced_probe_choices(
    session: &RunControlSession,
    probe_card: CardId,
) -> Vec<OwnerChoice> {
    let surface = build_decision_surface(session);
    let mut choices = card_reward_owner_choices(session, &surface);
    for choice in &mut choices {
        let Some(DecisionCandidateKind::CardRewardPick { card, upgrades }) =
            card_reward_kind(&choice.key)
        else {
            continue;
        };
        if card != probe_card {
            continue;
        }
        let admission = assess_reward_admission_from_master_deck(
            &session.run_state.master_deck,
            card,
            upgrades,
        );
        choice.annotation = candidate_annotation(
            DecisionPipelineContext::reward(DeckPlanSnapshot::from_run_state(&session.run_state)),
            DecisionCandidateKind::CardRewardPick { card, upgrades },
            Some(admission),
        );
        let ChoiceAnnotation::Candidate(decision) = &mut choice.annotation else {
            unreachable!();
        };
        decision.evaluation.lane = CandidateLane::Probe;
        decision.evaluation.adjudication =
            CandidateLaneAdjudication::uncapped(CandidateLane::Probe);
        decision.evaluation.expansion =
            ExpansionPlan::InspectOnly("challenger smoke probe");
        choice.expansion = OwnerChoiceExpansion::InspectOnly("challenger smoke probe");
    }
    choices.sort_by_key(|choice| match card_reward_kind(&choice.key) {
        Some(DecisionCandidateKind::CardRewardSkip) => 0,
        Some(DecisionCandidateKind::CardRewardPick { card, .. }) if card == probe_card => 1,
        _ => 2,
    });
    choices
}
```

The test performs these exact steps:

1. Create one `RunControlSession` and record its starting deck.
2. Install a reward containing `Corruption`, build forced-Probe choices, and call `plan_policy_expansions` with only `boss_scaling_plan = Missing` so the challenger opens an Exhaust commitment.
3. Execute only the planned challenger expansion through `expand_registered_owner`; assert the child deck contains `Corruption` while the untouched baseline deck does not.
4. On that child session, install a reward containing `DarkEmbrace`, build forced-Probe choices, and call the planner again. The existing commitment must choose `DarkEmbrace` instead of Skip.
5. Execute the second expansion, save that one-child frontier, load it, and inspect the restored policy and path.

Assert:

```rust
assert_eq!(restored_policy.divergence_count, 2);
assert_eq!(restored_branch.path.len(), 2);
assert_eq!(restored_branch.policy_lane.label(), "challenger-1");
assert_eq!(restored_branch.session.run_state.seed, seed);
assert_ne!(restored_branch.session.run_state.master_deck, baseline_deck);
```

The named cards are fixture mechanics for opening and supporting a commitment, not production allowlists or outcome assertions. Do not assert victory or a fixed HP delta.

- [ ] **Step 2: Run and verify the smoke contract**

```powershell
cargo test --lib challenger_execution_smoke -- --nocapture
```

Expected: one smoke test passes and reports two path steps after checkpoint restoration.

- [ ] **Step 3: Commit**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/challenger_execution_smoke.rs
git commit -m "test: cover resumed challenger execution"
```

---

### Task 8: Completion Verification

**Files:**
- Verify all files changed in Tasks 1-7.

- [ ] **Step 1: Format and inspect**

```powershell
cargo fmt --all
cargo fmt --all -- --check
git diff --check
git status --short
```

- [ ] **Step 2: Run focused policy and runtime suites together**

```powershell
cargo test --lib challenger_choice_policy::tests
cargo test --lib branch_policy_lane::tests
cargo test --lib policy_expansion_plan::tests
cargo test --lib branch_frontier::tests
cargo test --lib frontier_checkpoint::tests
cargo test --lib challenger_execution_smoke
```

- [ ] **Step 3: Run the full library suite once**

```powershell
cargo test --lib
```

Expected: all library tests pass.

- [ ] **Step 4: Compile the CLI and run architecture boundaries**

```powershell
cargo test --bin branch_tiny
cargo test --test architecture_runtime_boundaries
```

Expected: `branch_tiny` compiles and all seven architecture tests pass.

- [ ] **Step 5: Verify strategy/runtime boundaries**

```powershell
$runControlMatches = rg -n "ChallengerPolicyState|BranchPolicyLane|select_challenger_choice" src/eval/run_control
if ($LASTEXITCODE -eq 0) { $runControlMatches; throw "challenger strategy leaked into run-control" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking run-control boundary" }

$decisionMatches = rg -n "BranchPolicyLane|plan_policy_expansions" src/ai/strategy/decision_pipeline.rs
if ($LASTEXITCODE -eq 0) { $decisionMatches; throw "runtime lane identity leaked into production decision pipeline" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking production decision boundary" }
```

- [ ] **Step 6: Record final state**

```powershell
git status --short --branch
git log -10 --oneline
```

Expected: clean local `master`, no push, and focused commits for policy, lane identity, planning, execution, retention, persistence, and smoke verification.
