# Challenger Strategy Repair Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Subagents are disabled for this repository session at the user's request. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let bounded challenger lanes turn current deck and known-boss repair evidence into one safe, auditable counterfactual construction action without changing the production baseline.

**Architecture:** A new strategy-owned decision context derives open pressure and one conservative Exhaust payoff commitment from `RunState`. Challenger policy reconciliation and selection consume that context, permit only evidence-gated evaluated rejects, and pass typed selection evidence through owner-audit into durable branch paths and combat cases. Production scoring, hard candidate filters, branch limits, and run-control ownership remain unchanged.

**Tech Stack:** Rust 2021, serde/serde_json, existing `ai::strategy`, `ai::boss_mechanics_v1`, owner-audit `branch_tiny`, Cargo unit and architecture tests.

## Global Constraints

- Work only in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Start every code task from a clean Git status and commit each independently reviewed task.
- Do not run `cargo clean`.
- Do not change production candidate scoring, candidate lane assignment, or owner legality filters.
- Do not increase the two-challenger identity cap, branch budget, search budget, or wall-time contract.
- Do not hardcode card IDs, seed positions, or shop inventories into production strategy rules.
- Do not infer a damage or defense cause from HP loss or missing search wins.
- Preserve old frontier, branch-path, and combat-case deserialization with additive serde defaults.
- Keep durable run output under `artifacts/runs`, never under `target`.
- Use focused tests during red/green work; run the full library and `architecture_runtime_boundaries` suites at completion.

---

## File Structure

- Create `src/ai/strategy/challenger_decision_context.rs`: derive current static pressure, mapped known-boss pressure, gold, and automatic package commitments from `RunState`.
- Modify `src/ai/strategy/mod.rs`: export the new focused strategy module.
- Modify `src/ai/strategy/challenger_policy_state.rs`: reconcile persistent lane memory with current context, merge matched pressure, and complete supported commitment requirements.
- Modify `src/ai/strategy/challenger_choice_policy.rs`: centralize repair matching, reject safety, deterministic ranking, and seed-policy construction.
- Modify `src/runtime/branch/owner_audit/policy_expansion_plan.rs`: adapt owner choices into policy views, preserve baseline behavior, and attach typed selection evidence.
- Modify `src/runtime/branch/owner_audit/branch_generation.rs`: build one decision context per branch boundary and pass it to policy planning.
- Modify `src/runtime/branch/owner_audit/owner_choice_expander.rs`: copy planned selection evidence onto the executed branch-path step.
- Modify `src/runtime/branch/owner_audit/branch_path.rs`: serialize additive policy-selection evidence with serde defaults.
- Modify `src/runtime/branch/owner_audit/combat_gap_case.rs`: retain policy-selection evidence in projected combat-case decision evidence.
- Modify focused fixtures in `src/runtime/branch/owner_audit/challenger_execution_smoke.rs` only if the changed planner signature requires it.

---

### Task 1: Derive Typed Challenger Decision Context

**Files:**
- Create: `src/ai/strategy/challenger_decision_context.rs`
- Modify: `src/ai/strategy/mod.rs`
- Test: inline tests in `src/ai/strategy/challenger_decision_context.rs`

**Interfaces:**
- Consumes: `DeckPlanSnapshot::from_run_state(&RunState)`, `boss_mechanic_pressure_profile_v1(&RunState, EncounterId)`, `DeckRoleInventory`, and `open_inventory_pressure(DeckStrategicDeficitSummary)` moved from `challenger_choice_policy`.
- Produces: `pub struct ChallengerDecisionContext { pub deck_plan: DeckPlanSnapshot, pub gold: i32, pub current_pressure: Vec<PressureHypothesis>, pub automatic_commitments: Vec<StrategyCommitmentKind> }` and `pub fn challenger_decision_context(run_state: &RunState) -> ChallengerDecisionContext`.

- [ ] **Step 1: Export the new module and write failing context tests**

Add `pub mod challenger_decision_context;` beside the other challenger modules in `src/ai/strategy/mod.rs`.

Create the new file with the public type/function declarations and these tests before implementing the derivation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::runtime::combat::CombatCard;

    fn replace_deck(run: &mut RunState, cards: &[CardId]) {
        run.master_deck = cards
            .iter()
            .enumerate()
            .map(|(index, &card)| CombatCard::new(card, 70_000 + index as u32))
            .collect();
    }

    #[test]
    fn two_exhaust_sources_without_payoff_open_one_payoff_commitment() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        replace_deck(&mut run, &[CardId::TrueGrit, CardId::BurningPact]);

        let context = challenger_decision_context(&run);

        assert_eq!(
            context.automatic_commitments,
            vec![StrategyCommitmentKind::ExhaustEngine]
        );
    }

    #[test]
    fn one_incidental_exhaust_source_does_not_open_commitment() {
        let mut run = RunState::new(2, 0, false, "Ironclad");
        replace_deck(&mut run, &[CardId::TrueGrit, CardId::Strike]);

        let context = challenger_decision_context(&run);

        assert!(context.automatic_commitments.is_empty());
    }

    #[test]
    fn automaton_block_or_kill_answer_opens_both_safe_response_axes() {
        let mut run = RunState::new(3, 0, false, "Ironclad");
        run.act_num = 2;
        run.boss_key = Some(EncounterId::Automaton);
        replace_deck(
            &mut run,
            &[CardId::Strike, CardId::Defend, CardId::Bash],
        );

        let context = challenger_decision_context(&run);

        assert!(context.current_pressure.iter().any(|hypothesis| {
            hypothesis.axis == PressureAxis::DelayCapacity
                && hypothesis.supporting_evidence.iter().any(|evidence| {
                    evidence.source == PressureEvidenceSource::EncounterThreat
                        && evidence.label == "block50_or_kill_before_beam"
                })
        }));
        assert!(context.current_pressure.iter().any(|hypothesis| {
            hypothesis.axis == PressureAxis::ResolutionTempo
                && hypothesis.supporting_evidence.iter().any(|evidence| {
                    evidence.source == PressureEvidenceSource::EncounterThreat
                        && evidence.label == "block50_or_kill_before_beam"
                })
        }));
    }
}
```

- [ ] **Step 2: Run the focused tests and verify the red state**

Run:

```powershell
cargo test --lib challenger_decision_context -- --nocapture
```

Expected: compilation fails because `ChallengerDecisionContext` and `challenger_decision_context` have declarations but no implementation, or the three new assertions fail with empty derived evidence.

- [ ] **Step 3: Implement context derivation and move static pressure ownership**

Implement the module with these exact public interfaces and boss mapping:

```rust
use crate::ai::boss_mechanics_v1::{
    boss_mechanic_pressure_profile_v1, BossMechanicMissingAnswerV1,
};
use crate::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficitSummary, StrategicDeficitLevel,
};
use crate::ai::strategy::pressure_assessment::{
    EvidenceConfidence, PressureAxis, PressureCoverage, PressureEvidence,
    PressureEvidenceSource, PressureHypothesis,
};
use crate::state::run::RunState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChallengerDecisionContext {
    pub deck_plan: DeckPlanSnapshot,
    pub gold: i32,
    pub current_pressure: Vec<PressureHypothesis>,
    pub automatic_commitments: Vec<StrategyCommitmentKind>,
}

pub fn challenger_decision_context(run_state: &RunState) -> ChallengerDecisionContext {
    let deck_plan = DeckPlanSnapshot::from_run_state(run_state);
    let mut current_pressure = open_inventory_pressure(deck_plan.strategic_deficit);
    if let Some(boss) = run_state.boss_key {
        let profile = boss_mechanic_pressure_profile_v1(run_state, boss);
        for answer in profile.missing_answers {
            for axis in axes_for_missing_answer(answer) {
                merge_open_hypothesis(
                    &mut current_pressure,
                    open_hypothesis(
                        axis,
                        EvidenceConfidence::Medium,
                        PressureEvidenceSource::EncounterThreat,
                        answer.label(),
                    ),
                );
            }
        }
    }
    current_pressure.sort_by_key(|hypothesis| hypothesis.axis);

    let mut automatic_commitments = Vec::new();
    if deck_plan.roles.exhaust_stream_units >= 2 && deck_plan.roles.exhaust_payoff_units == 0 {
        automatic_commitments.push(StrategyCommitmentKind::ExhaustEngine);
    }

    ChallengerDecisionContext {
        deck_plan,
        gold: run_state.gold,
        current_pressure,
        automatic_commitments,
    }
}

pub fn open_inventory_pressure(facts: DeckStrategicDeficitSummary) -> Vec<PressureHypothesis> {
    let mut hypotheses = Vec::new();
    push_static_if_open(&mut hypotheses, facts.frontload_damage, PressureAxis::ResolutionTempo, "frontload inventory is missing or thin");
    push_static_if_open(&mut hypotheses, facts.aoe_or_minion_control, PressureAxis::MultiTargetControl, "multi-target inventory is missing or thin");
    push_static_if_open(&mut hypotheses, facts.block_or_mitigation, PressureAxis::DelayCapacity, "delay inventory is missing or thin");
    push_static_if_open(&mut hypotheses, facts.boss_scaling_plan, PressureAxis::GrowthHorizon, "scaling inventory is missing or thin");
    if is_open(facts.deck_access) || is_open(facts.energy_or_playability) {
        merge_open_hypothesis(
            &mut hypotheses,
            open_hypothesis(
                PressureAxis::Deployability,
                EvidenceConfidence::Low,
                PressureEvidenceSource::DeckCapability,
                "access or playability inventory is missing or thin",
            ),
        );
    }
    hypotheses.sort_by_key(|hypothesis| hypothesis.axis);
    hypotheses
}

fn axes_for_missing_answer(answer: BossMechanicMissingAnswerV1) -> Vec<PressureAxis> {
    use BossMechanicMissingAnswerV1::*;
    match answer {
        DarkEchoBlockPlan | ExecuteBlockPlan => vec![PressureAxis::DelayCapacity],
        HasteBurstOrSetupPlan | ChampTransitionBurst => vec![
            PressureAxis::ResolutionTempo,
            PressureAxis::GrowthHorizon,
        ],
        FocusedKillOrderPlan => vec![PressureAxis::ResolutionTempo],
        CollectorMinionPlan => vec![PressureAxis::MultiTargetControl],
        Block50OrKillBeforeBeam => vec![
            PressureAxis::DelayCapacity,
            PressureAxis::ResolutionTempo,
        ],
        StasisRecoveryPlan => vec![PressureAxis::Deployability],
        PhasePowerPlan | TimeWarpCounterPlan | ArtifactStripPlan | TurnFourDebuffPlan => Vec::new(),
    }
}
```

Keep `open_hypothesis`, `merge_open_hypothesis`, `push_static_if_open`, and `is_open` private. `merge_open_hypothesis` must deduplicate evidence by `(source, label)`, keep coverage `Open`, and retain the higher confidence. Remove the old duplicate `open_inventory_pressure` and helper functions from `challenger_choice_policy.rs`, importing this module instead.

- [ ] **Step 4: Run focused context and existing choice-policy tests**

Run:

```powershell
cargo test --lib challenger_decision_context -- --nocapture
cargo test --lib challenger_choice_policy -- --nocapture
```

Expected: all context tests pass; all existing choice-policy pressure tests still pass after the ownership move.

- [ ] **Step 5: Commit the context boundary**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/challenger_decision_context.rs src/ai/strategy/challenger_choice_policy.rs
git commit -m "feat: derive challenger repair context"
```

---

### Task 2: Reconcile Persistent Policy Memory

**Files:**
- Modify: `src/ai/strategy/challenger_policy_state.rs`
- Test: inline tests in `src/ai/strategy/challenger_policy_state.rs`

**Interfaces:**
- Consumes: `&ChallengerDecisionContext` from Task 1 and `&CandidatePressureResponse`.
- Produces: `ChallengerPolicyState::reconcile_context`, `merge_matched_pressure`, and `satisfy_supported_requirements`.

- [ ] **Step 1: Write failing reconciliation and package-completion tests**

Add tests using this helper and assertions:

```rust
fn context_with(
    current_pressure: Vec<PressureHypothesis>,
    automatic_commitments: Vec<StrategyCommitmentKind>,
) -> ChallengerDecisionContext {
    let run = crate::state::run::RunState::new(10, 0, false, "Ironclad");
    ChallengerDecisionContext {
        deck_plan: crate::ai::strategy::deck_plan::DeckPlanSnapshot::from_run_state(&run),
        gold: run.gold,
        current_pressure,
        automatic_commitments,
    }
}

#[test]
fn context_opens_automatic_commitment_once() {
    let mut state = ChallengerPolicyState::new(1);
    let context = context_with(Vec::new(), vec![StrategyCommitmentKind::ExhaustEngine]);

    state.reconcile_context(&context);
    state.reconcile_context(&context);

    assert_eq!(state.commitments.len(), 1);
    assert_eq!(state.commitments[0].requirements, vec![CommitmentRequirement::Payoff]);
    assert_eq!(state.commitments[0].horizon, CommitmentHorizon::CurrentActBoss);
}

#[test]
fn missing_current_axis_becomes_partial_not_covered() {
    let mut state = ChallengerPolicyState::new(1);
    state.active_pressure.push(PressureHypothesis {
        axis: PressureAxis::DelayCapacity,
        coverage: PressureCoverage::Open,
        confidence: EvidenceConfidence::Low,
        supporting_evidence: Vec::new(),
        contradicting_evidence: Vec::new(),
    });

    state.reconcile_context(&context_with(Vec::new(), Vec::new()));

    assert_eq!(state.active_pressure[0].coverage, PressureCoverage::PartiallyCovered);
}

#[test]
fn exhaust_support_completes_payoff_without_covering_pressure() {
    let mut state = ChallengerPolicyState::new(1);
    state.reconcile_context(&context_with(
        Vec::new(),
        vec![StrategyCommitmentKind::ExhaustEngine],
    ));
    state.active_pressure.push(PressureHypothesis {
        axis: PressureAxis::GrowthHorizon,
        coverage: PressureCoverage::Open,
        confidence: EvidenceConfidence::Medium,
        supporting_evidence: Vec::new(),
        contradicting_evidence: Vec::new(),
    });
    let response = CandidatePressureResponse {
        supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
        ..CandidatePressureResponse::default()
    };

    state.satisfy_supported_requirements(&response);

    assert_eq!(state.commitments[0].status, CommitmentStatus::Completed);
    assert_eq!(state.active_pressure[0].coverage, PressureCoverage::Open);
}
```

- [ ] **Step 2: Run policy-state tests and verify they fail**

Run:

```powershell
cargo test --lib challenger_policy_state -- --nocapture
```

Expected: compilation fails because the three new methods do not exist.

- [ ] **Step 3: Implement idempotent reconciliation and explicit requirement completion**

Add these public methods to `impl ChallengerPolicyState`:

```rust
pub fn reconcile_context(&mut self, context: &ChallengerDecisionContext) {
    for &kind in &context.automatic_commitments {
        let already_known = self.commitments.iter().any(|commitment| {
            commitment.kind == kind
                && matches!(commitment.status, CommitmentStatus::Active | CommitmentStatus::Completed)
        });
        if !already_known {
            self.open_commitment(StrategyCommitment {
                kind,
                status: CommitmentStatus::Active,
                requirements: default_requirements(kind),
                horizon: CommitmentHorizon::CurrentActBoss,
                burden_units: 0,
            });
        }
    }

    for remembered in &mut self.active_pressure {
        if let Some(current) = context
            .current_pressure
            .iter()
            .find(|current| current.axis == remembered.axis)
        {
            *remembered = merge_pressure_hypotheses(remembered.clone(), current.clone());
        } else if remembered.coverage == PressureCoverage::Open {
            remembered.coverage = PressureCoverage::PartiallyCovered;
        }
    }
}

pub fn merge_matched_pressure(&mut self, matched: &[PressureHypothesis]) {
    for hypothesis in matched {
        if let Some(existing) = self
            .active_pressure
            .iter_mut()
            .find(|existing| existing.axis == hypothesis.axis)
        {
            *existing = merge_pressure_hypotheses(existing.clone(), hypothesis.clone());
        } else {
            self.active_pressure.push(hypothesis.clone());
        }
    }
    self.active_pressure.sort_by_key(|hypothesis| hypothesis.axis);
}

pub fn satisfy_supported_requirements(&mut self, response: &CandidatePressureResponse) {
    for &kind in &response.supports_commitments {
        let requirement = match kind {
            StrategyCommitmentKind::ExhaustEngine => CommitmentRequirement::Payoff,
            StrategyCommitmentKind::StrengthScaling | StrategyCommitmentKind::BlockEngine => {
                CommitmentRequirement::Source
            }
            StrategyCommitmentKind::UpgradeAccess => CommitmentRequirement::Deployability,
            StrategyCommitmentKind::SelfDamageEngine if response.repeatable_self_damage_supply => {
                CommitmentRequirement::RepeatableSupply
            }
            StrategyCommitmentKind::SelfDamageEngine => continue,
        };
        self.observe_requirement(kind, requirement);
    }
}
```

The private `merge_pressure_hypotheses` must use this exact coverage order: `Open` dominates every
other value, then `PartiallyCovered`, then `Unknown`, and `Covered` only survives when both inputs
are `Covered`. It retains the higher confidence and deduplicates supporting and contradicting
evidence by `(source, label)`. Add the required imports from `challenger_decision_context` and
`pressure_assessment`.

- [ ] **Step 4: Run policy-state and JSON compatibility tests**

Run:

```powershell
cargo test --lib challenger_policy_state -- --nocapture
```

Expected: all old and new tests pass, including the existing JSON round trip.

- [ ] **Step 5: Commit policy reconciliation**

```powershell
git add src/ai/strategy/challenger_policy_state.rs
git commit -m "feat: reconcile challenger policy tasks"
```

---

### Task 3: Centralize Safe Repair Selection

**Files:**
- Modify: `src/ai/strategy/challenger_choice_policy.rs`
- Test: inline tests in `src/ai/strategy/challenger_choice_policy.rs`

**Interfaces:**
- Consumes: reconciled `ChallengerPolicyState`, `&ChallengerDecisionContext`, and enriched `PolicyCandidateView` values.
- Produces: `PolicySelectionClass`, `PolicyChoiceSelection`, `select_challenger_candidate`, `select_challenger_choice`, and the context-aware `seed_challenger_policy`.

- [ ] **Step 1: Define the selection contract and write failing safety tests**

Change `PolicyCandidateView` to:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyCandidateView {
    pub choice_index: usize,
    pub lane: CandidateLane,
    pub raw_lane: CandidateLane,
    pub auto_allowed: bool,
    pub hard_filtered: bool,
    pub has_reject_cap: bool,
    pub inspect_only_reason: Option<String>,
    pub response: CandidatePressureResponse,
}
```

Add the intended result types:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicySelectionClass {
    OrdinaryChallenger,
    PressureRepair,
    CommitmentRepair,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyChoiceSelection {
    pub choice_index: usize,
    pub class: PolicySelectionClass,
    pub matched_pressure: Vec<PressureHypothesis>,
    pub matched_commitments: Vec<StrategyCommitmentKind>,
    pub overrode_reject: bool,
}
```

Add tests with one helper that fills every field:

```rust
fn candidate(
    lane: CandidateLane,
    hard_filtered: bool,
    has_reject_cap: bool,
    response: CandidatePressureResponse,
) -> PolicyCandidateView {
    PolicyCandidateView {
        choice_index: 1,
        lane,
        raw_lane: lane,
        auto_allowed: lane != CandidateLane::Reject,
        hard_filtered,
        has_reject_cap,
        inspect_only_reason: (lane == CandidateLane::Reject)
            .then(|| "candidate score rejected".to_string()),
        response,
    }
}

#[test]
fn hard_filtered_reject_never_becomes_policy_action() {
    let policy = ChallengerPolicyState::new(1);
    let context = context_with_open_axis(PressureAxis::GrowthHorizon);
    let rejected = candidate(
        CandidateLane::Reject,
        true,
        false,
        CandidatePressureResponse {
            axes: vec![PressureAxis::GrowthHorizon],
            ..CandidatePressureResponse::default()
        },
    );

    assert!(select_challenger_candidate(&policy, &context, &rejected).is_none());
}

#[test]
fn scored_reject_can_answer_current_open_pressure() {
    let policy = ChallengerPolicyState::new(1);
    let context = context_with_open_axis(PressureAxis::GrowthHorizon);
    let rejected = candidate(
        CandidateLane::Reject,
        false,
        false,
        CandidatePressureResponse {
            axes: vec![PressureAxis::GrowthHorizon],
            ..CandidatePressureResponse::default()
        },
    );

    let selection = select_challenger_candidate(&policy, &context, &rejected)
        .expect("scored pressure repair should be eligible");

    assert_eq!(selection.class, PolicySelectionClass::PressureRepair);
    assert!(selection.overrode_reject);
}

#[test]
fn cap_rejected_candidate_requires_direct_commitment_support() {
    let mut policy = ChallengerPolicyState::new(1);
    policy.reconcile_context(&context_with_exhaust_commitment());
    let broad_only = candidate(
        CandidateLane::Reject,
        false,
        true,
        CandidatePressureResponse {
            axes: vec![PressureAxis::GrowthHorizon],
            ..CandidatePressureResponse::default()
        },
    );
    let direct = candidate(
        CandidateLane::Reject,
        false,
        true,
        CandidatePressureResponse {
            supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
            ..CandidatePressureResponse::default()
        },
    );

    assert!(select_challenger_candidate(
        &policy,
        &context_with_open_axis(PressureAxis::GrowthHorizon),
        &broad_only,
    )
    .is_none());
    assert_eq!(
        select_challenger_candidate(&policy, &context_with_exhaust_commitment(), &direct)
            .expect("direct commitment repair should pass")
            .class,
        PolicySelectionClass::CommitmentRepair,
    );
}
```

The local test helpers `context_with_open_axis` and `context_with_exhaust_commitment` must construct full `ChallengerDecisionContext` values with a default Ironclad `RunState`, one open hypothesis or one automatic commitment respectively.

- [ ] **Step 2: Run the focused choice-policy tests and verify the red state**

Run:

```powershell
cargo test --lib challenger_choice_policy -- --nocapture
```

Expected: compilation fails until the new selection functions and fields exist.

- [ ] **Step 3: Implement one repair matcher used by seeding and continuation**

Implement these signatures:

```rust
pub fn select_challenger_candidate(
    policy: &ChallengerPolicyState,
    context: &ChallengerDecisionContext,
    candidate: &PolicyCandidateView,
) -> Option<PolicyChoiceSelection>;

pub fn select_challenger_choice(
    policy: &ChallengerPolicyState,
    context: &ChallengerDecisionContext,
    candidates: &[PolicyCandidateView],
) -> Option<PolicyChoiceSelection>;

pub fn seed_challenger_policy(
    lane_id: u8,
    checkpoint_ref: impl Into<String>,
    context: &ChallengerDecisionContext,
    candidate: &PolicyCandidateView,
) -> Option<(ChallengerPolicyState, PolicyChoiceSelection)>;
```

`select_challenger_candidate` must:

```rust
let matched_commitments = active_commitment_matches(policy, &candidate.response);
let matched_active = open_pressure_matches(&policy.active_pressure, &candidate.response);
let matched_current = open_pressure_matches(&context.current_pressure, &candidate.response);
let matched_pressure = merge_matched_pressure(matched_active, matched_current);

let ordinarily_eligible = candidate.lane != CandidateLane::Reject
    && (candidate.auto_allowed
        || (candidate.lane == CandidateLane::Probe
            && response_has_policy_meaning(&candidate.response)));
let repair_eligible = candidate.lane == CandidateLane::Reject
    && !candidate.hard_filtered
    && if candidate.has_reject_cap {
        !matched_commitments.is_empty()
    } else {
        !matched_commitments.is_empty() || !matched_pressure.is_empty()
    };
if !ordinarily_eligible && !repair_eligible {
    return None;
}

let class = if !matched_commitments.is_empty() {
    PolicySelectionClass::CommitmentRepair
} else if !matched_pressure.is_empty() {
    PolicySelectionClass::PressureRepair
} else {
    PolicySelectionClass::OrdinaryChallenger
};
```

`select_challenger_choice` must rank by `(commitment miss, active-pressure miss, current-pressure miss, choice_index)` and return the full selection. `seed_challenger_policy` must reconcile automatic commitments first, reject a candidate with neither pressure nor commitment matches, merge matched pressure, record divergence, satisfy supported requirements, and return both state and evidence.

- [ ] **Step 4: Run all strategy-policy tests**

Run:

```powershell
cargo test --lib challenger_choice_policy -- --nocapture
cargo test --lib challenger_policy_state -- --nocapture
cargo test --lib challenger_signature -- --nocapture
```

Expected: all tests pass; the old test named `reject_candidate_cannot_become_a_challenger_action` is replaced by the two narrower hard-filter and scored-repair contracts.

- [ ] **Step 5: Commit the safe repair selector**

```powershell
git add src/ai/strategy/challenger_choice_policy.rs
git commit -m "feat: gate challenger repair choices"
```

---

### Task 4: Integrate Context and Evidence with Owner Planning

**Files:**
- Modify: `src/runtime/branch/owner_audit/policy_expansion_plan.rs`
- Modify: `src/runtime/branch/owner_audit/branch_generation.rs`
- Modify: `src/runtime/branch/owner_audit/challenger_execution_smoke.rs`
- Test: inline tests in `policy_expansion_plan.rs` and `branch_generation.rs`

**Interfaces:**
- Consumes: `challenger_decision_context(&RunState)`, `PolicyChoiceSelection`, enriched `PolicyCandidateView`, and existing `OwnerChoice` evaluation metadata.
- Produces: `PolicyExpansion { choice_index, child_lane, selection_evidence }` and `PolicyExpansionEvidence` for Task 5.

- [ ] **Step 1: Write the failing baseline-preservation and rejected-payoff integration tests**

Add a runtime evidence type next to `PolicyExpansion`:

```rust
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PolicyExpansionClass {
    Production,
    OrdinaryChallenger,
    PressureRepair,
    CommitmentRepair,
}
```

Extend the policy-expansion tests with:

```rust
#[test]
fn baseline_keeps_production_while_challenger_takes_rejected_exhaust_payoff() {
    let mut run = sts_simulator::state::run::RunState::new(22, 0, false, "Ironclad");
    run.act_num = 2;
    run.gold = 200;
    run.master_deck = vec![
        sts_simulator::runtime::combat::CombatCard::new(CardId::TrueGrit, 80_001),
        sts_simulator::runtime::combat::CombatCard::new(CardId::BurningPact, 80_002),
    ];
    let context = challenger_decision_context(&run);
    let choices = vec![
        shop_leave_choice(),
        rejected_card_choice(
            DecisionCandidateKind::ShopBuyCard {
                card: CardId::FeelNoPain,
                upgrades: 0,
                price: 75,
            },
            vec![RewardAdmissionReason::Supports(PackageKind::Exhaust)],
            true,
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
    assert_eq!(plan[0].selection_evidence.class, PolicyExpansionClass::Production);
    assert_eq!(plan[1].choice_index, 1);
    assert_eq!(plan[1].child_lane.label(), "challenger-1");
    assert_eq!(
        plan[1].selection_evidence.class,
        PolicyExpansionClass::CommitmentRepair,
    );
    assert!(plan[1].selection_evidence.overrode_reject);
}
```

`shop_leave_choice` must create an auto-allowed `DecisionCandidateKind::ShopLeave` choice.
`rejected_card_choice` must create a scored `CandidateEvaluation` with final lane `Reject`, inspect
reason `candidate score rejected`, and an acquisition `LaneCap::Reject` when its boolean parameter
is true. Keep the existing production-baseline assertion unchanged so the test proves both outputs
from the same planning call.

- [ ] **Step 2: Run owner planning tests and verify the changed signatures fail**

Run:

```powershell
cargo test --bin branch_tiny policy_expansion_plan -- --nocapture
cargo test --bin branch_tiny branch_generation -- --nocapture
```

Expected: compilation fails until the planner accepts `&ChallengerDecisionContext` and every `PolicyExpansion` supplies evidence.

- [ ] **Step 3: Adapt candidate metadata without weakening hard filters**

Change the planner signature to:

```rust
pub(super) fn plan_policy_expansions(
    lane: &BranchPolicyLane,
    context: &ChallengerDecisionContext,
    choices: &[OwnerChoice],
    lane_budget: usize,
    checkpoint_ref: &str,
) -> Vec<PolicyExpansion>
```

Build each `PolicyCandidateView` from the existing evaluation:

```rust
let evaluation = &decision.evaluation;
let inspect_only_reason = evaluation.inspect_only_reason().map(str::to_string);
let hard_filtered = evaluation.lane == CandidateLane::Reject
    && inspect_only_reason.as_deref() != Some("candidate score rejected");
let has_reject_cap = evaluation
    .adjudication
    .caps
    .iter()
    .any(|cap| cap.cap == LaneCap::Reject);
PolicyCandidateView {
    choice_index,
    lane: evaluation.lane,
    raw_lane: evaluation.adjudication.raw_lane,
    auto_allowed: choice.auto_expand_allowed(),
    hard_filtered,
    has_reject_cap,
    inspect_only_reason,
    response,
}
```

Do not infer hard filtering from score or an empty score list; use only the stable inspection-reason boundary above.

- [ ] **Step 4: Preserve baseline and evolve challenger state**

In baseline planning, always emit the existing production expansion first with `PolicyExpansionClass::Production`. For each other candidate, call the Task 3 `seed_challenger_policy`; emit a challenger only when it returns a policy and selection, then derive `PolicyExpansionEvidence` from the selection and original view.

In existing-challenger planning:

```rust
let mut contextual_policy = policy.clone();
contextual_policy.reconcile_context(context);
let selected = select_challenger_choice(&contextual_policy, context, candidates);
let selected_index = selected
    .as_ref()
    .map(|selection| selection.choice_index)
    .or(production_index);
```

Persist `contextual_policy` even when selection falls back to production. For a non-production choice, merge `selection.matched_pressure`, call `record_divergence`, then call `satisfy_supported_requirements`. Advance the decision-boundary horizon exactly once. Convert selection class deterministically; do not change candidate scores or annotations.

In `branch_generation.rs`, replace the strategic-deficit-only value with:

```rust
let context = challenger_decision_context(&branch.session.run_state);
plan_policy_expansions(
    &branch.policy_lane,
    &context,
    choices,
    branch_budget,
    &checkpoint_ref,
)
```

Update all focused fixtures and smoke tests to construct a real `RunState` context rather than a bare `DeckStrategicDeficitSummary`.

- [ ] **Step 5: Run owner planning and continuous-challenger smoke tests**

Run:

```powershell
cargo test --bin branch_tiny policy_expansion_plan -- --nocapture
cargo test --bin branch_tiny branch_generation -- --nocapture
cargo test --bin branch_tiny challenger_execution_smoke -- --nocapture
```

Expected: all tests pass. The rejected-payoff fixture produces baseline leave plus one challenger purchase without issuing more than two challenger identities.

- [ ] **Step 6: Commit owner planning integration**

```powershell
git add src/runtime/branch/owner_audit/policy_expansion_plan.rs src/runtime/branch/owner_audit/branch_generation.rs src/runtime/branch/owner_audit/challenger_execution_smoke.rs
git commit -m "feat: execute challenger strategy repairs"
```

---

### Task 5: Persist Policy Selection Evidence

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_path.rs`
- Modify: `src/runtime/branch/owner_audit/owner_choice_expander.rs`
- Modify: `src/runtime/branch/owner_audit/combat_gap_case.rs`
- Test: inline tests in `branch_path.rs`, `owner_choice_expander.rs`, and `combat_gap_case.rs`

**Interfaces:**
- Consumes: `PolicyExpansionEvidence` from Task 4.
- Produces: additive `BranchPathPolicySelectionSnapshot` in every planned path step and the same field inside combat-case `decision_evidence` JSON.

- [ ] **Step 1: Write failing branch-path and combat-case evidence tests**

Add the serializable snapshot type:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathPolicySelectionSnapshot {
    class: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    matched_pressure_axes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    matched_commitments: Vec<String>,
    original_lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_inspect_only: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    overrode_reject: bool,
    checkpoint_ref: String,
}
```

Add this field to `BranchPathStep`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub(super) policy_selection: Option<BranchPathPolicySelectionSnapshot>,
```

Add a branch-path JSON test that serializes a commitment-repair snapshot and asserts:

```rust
assert_eq!(value["policy_selection"]["class"], "commitment_repair");
assert_eq!(
    value["policy_selection"]["matched_commitments"][0],
    "exhaust_engine"
);
assert_eq!(value["policy_selection"]["original_lane"], "reject");
assert_eq!(value["policy_selection"]["overrode_reject"], true);
assert_eq!(
    value["policy_selection"]["checkpoint_ref"],
    "branch-0/step-0"
);
```

Extend `path_projection_keeps_complete_recorded_decision_evidence` in `combat_gap_case.rs` with:

```rust
assert_eq!(
    evidence["policy_selection"]["class"],
    "commitment_repair"
);
assert_eq!(evidence["policy_selection"]["overrode_reject"], true);
```

- [ ] **Step 2: Run path tests and verify the red state**

Run:

```powershell
cargo test --bin branch_tiny branch_path -- --nocapture
cargo test --bin branch_tiny combat_gap_case -- --nocapture
```

Expected: compilation fails until the additive field is supplied and projected.

- [ ] **Step 3: Convert planned evidence at the execution boundary**

Implement:

```rust
impl BranchPathPolicySelectionSnapshot {
    pub(super) fn from_evidence(evidence: &PolicyExpansionEvidence) -> Self {
        Self {
            class: policy_expansion_class_label(evidence.class).to_string(),
            matched_pressure_axes: evidence
                .matched_pressure_axes
                .iter()
                .map(serialized_enum_label)
                .collect(),
            matched_commitments: evidence
                .matched_commitments
                .iter()
                .map(serialized_enum_label)
                .collect(),
            original_lane: candidate_lane_label(evidence.original_lane).to_string(),
            original_inspect_only: evidence.original_inspect_only.clone(),
            overrode_reject: evidence.overrode_reject,
            checkpoint_ref: evidence.checkpoint_ref.clone(),
        }
    }
}
```

Use explicit labels in `policy_expansion_class_label`: `production`, `ordinary_challenger`,
`pressure_repair`, and `commitment_repair`. Implement the generic helper using the enums' existing
serde contracts rather than Debug text:

```rust
fn serialized_enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}
```

Test both `GrowthHorizon -> growth_horizon` and `ExhaustEngine -> exhaust_engine`.

In `owner_choice_expander.rs`, destructure or borrow the selection evidence before applying the command and set:

```rust
policy_selection: Some(BranchPathPolicySelectionSnapshot::from_evidence(
    &expansion.selection_evidence,
)),
```

The shop preview bundle path is not a policy-planned expansion, so set `policy_selection: None` there. Add `policy_selection: None` to legacy test fixtures.

- [ ] **Step 4: Preserve evidence in combat-case projection**

Add this property to the existing `decision_evidence` JSON in `combat_gap_case.rs`:

```rust
"policy_selection": &step.policy_selection,
```

Update `branch_path_step_with_all_evidence` to contain a real commitment-repair snapshot. Do not change `CombatCasePathStep` schema because `decision_evidence` is already additive `Value` and old cases already deserialize it as optional.

- [ ] **Step 5: Run path, expansion, and combat-case tests**

Run:

```powershell
cargo test --bin branch_tiny branch_path -- --nocapture
cargo test --bin branch_tiny owner_choice_expander -- --nocapture
cargo test --bin branch_tiny combat_gap_case -- --nocapture
```

Expected: all tests pass; existing branch path without the new field deserializes with `None`, and projected combat evidence includes the selected repair rationale.

- [ ] **Step 6: Commit durable repair evidence**

```powershell
git add src/runtime/branch/owner_audit/branch_path.rs src/runtime/branch/owner_audit/owner_choice_expander.rs src/runtime/branch/owner_audit/combat_gap_case.rs
git commit -m "feat: persist challenger repair evidence"
```

---

### Task 6: Verify Boundaries and Observe One Fresh Bounded Run

**Files:**
- Modify only if a verification failure identifies a defect in Tasks 1-5.
- Create runtime evidence under: `artifacts/runs/bounded-mainline-seed-20260712005-strategy-repair-loop`

**Interfaces:**
- Consumes: the completed strategy repair loop and existing durable trajectory diagnostics.
- Produces: verified permanent code and one bounded observation capsule; the run is evidence, not a regression assertion.

- [ ] **Step 1: Check formatting and whitespace**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
```

Expected: both commands exit 0 with no output.

- [ ] **Step 2: Run complete permanent-code verification**

Run:

```powershell
cargo test --lib
cargo test --bin branch_tiny
cargo test --test architecture_runtime_boundaries
```

Expected: every suite reports zero failures. Record the exact pass counts in the final handoff.

- [ ] **Step 3: Verify architecture ownership explicitly**

Run:

```powershell
rg -n "ChallengerDecisionContext|PolicyExpansionEvidence|policy_selection" src/eval/run_control src/ai/combat_search_v2 src/runtime/branch/owner_audit/branch_frontier.rs
```

Expected: no matches. The strategy context must not leak into run-control, combat search, or frontier retention.

- [ ] **Step 4: Run one fresh bounded three-lane evaluation**

Run:

```powershell
cargo run --profile fast-run --quiet --bin branch_tiny -- --seed 20260712005 --ascension 0 --objective first-victory --generations 64 --max-branches 3 --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 200000 --rescue-search-ms 3000 --boss-search-nodes 800000 --boss-search-ms 10000 --wall-ms 60000 --run-capsule artifacts/runs/bounded-mainline-seed-20260712005-strategy-repair-loop
```

Expected: the process reaches a safe bounded stop or terminal result and writes a capsule. If a repair opportunity appears, inspect the selected path step for `pressure_repair` or `commitment_repair`, its original reject evidence, and preserved baseline trajectory. If none appears, report that observation as inconclusive and rely on the deterministic owner-planning contract for correctness.

- [ ] **Step 5: Inspect the capsule without adding a seed assertion**

Run:

```powershell
rg -n '"policy_selection"|"pressure_repair"|"commitment_repair"|"trajectory_evaluation"' artifacts/runs/bounded-mainline-seed-20260712005-strategy-repair-loop
git status --short --branch
```

Expected: capsule summaries remain readable and Git shows only intentional source/doc changes or a clean tree. Runtime artifacts remain untracked/ignored and are not committed.

- [ ] **Step 6: Commit only verification fixes, if any**

If Tasks 1-5 required a source correction during full verification, stage only the corrected source and focused tests, then run:

```powershell
git commit -m "fix: preserve challenger repair boundaries"
```

If no correction was required, do not create an empty commit.

---

## Completion Evidence

The implementation is complete only when all of the following are true:

- production baseline selects the same action as before in focused fixtures;
- a source-only Exhaust package opens exactly one payoff commitment;
- hard-filtered and unaffordable candidates remain ineligible;
- a direct commitment repair can cross an evaluated acquisition/role cap Reject;
- a broad pressure match cannot cross a rejecting cap;
- selected repair evidence survives branch paths, capsule serialization, and combat-case projection;
- full library, branch-tiny, and architecture suites pass;
- one fresh bounded run is recorded without treating its result as a promotion decision.
