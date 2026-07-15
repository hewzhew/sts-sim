# Combat Search Incumbent Portfolio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace first-accepted post-primary combat lane commits with a deterministic candidate portfolio that preserves a monotone incumbent and commits the real run-control session once.

**Architecture:** Keep the primary lane as a fast path. Split search-engine identity from attempt policy, run every post-primary producer against one immutable root session, rank applicable trial sessions through a small explicit incumbent, and commit only the selected trial. Reuse the old duplicate hallway fallback budget for a complementary Lazy producer and keep all non-selected attempts as diagnostics only.

**Tech Stack:** Rust 2021, Cargo, serde/serde_json, existing `combat_search_v2`, run-control, and owner-audit modules.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Do not run `cargo clean`.
- Do not change combat mechanics, route policy, shop policy, card reward policy, potion semantics, or inner search expansion behavior.
- Keep the primary accepted-line and operation-budget-chunk fast paths.
- Post-primary attempts must start from one immutable root and commit the real session at most once.
- Immediate and Lazy hallway attempts must stay within the existing configured worst-case portfolio budget.
- Add default tests only for architecture invariants; keep the seed006 Transient replay opt-in.
- Use focused tests for red/green work, then run the full library and `architecture_runtime_boundaries` suites.

---

### Task 1: Separate Engine Identity From Attempt Policy

**Files:**
- Modify: `src/ai/combat_search_v2/plugins.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Modify: `src/eval/run_control/combat_search_setup.rs`
- Modify: `src/eval/run_control/combat_search.rs`
- Modify: `src/eval/run_control/auto_step.rs`
- Modify: `src/bin/combat_case_review/search_runner.rs`
- Modify: `src/bin/combat_case_review/search_intervention.rs`
- Modify: `src/bin/combat_case_review/quality_lanes/specs.rs`
- Modify: `src/bin/combat_case_review/frozen_panel_lanes/types.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_recipe.rs`

**Interfaces:**
- Produces: `CombatSearchEngineProfile`, `CombatSearchAttemptPolicy`, `CombatSearchProfile::engine_fingerprint()`, and the nested `CombatSearchProfile { label, engine, policy }` representation.
- Preserves: `CombatSearchProfile::to_config()` and all existing builder methods so later tasks do not change engine semantics.

- [ ] **Step 1: Write failing profile-identity tests**

Add tests to `plugins.rs` proving policy and label are excluded from engine identity while child-rollout policy is included:

```rust
fn test_profile(label: &'static str) -> CombatSearchProfile {
    CombatSearchProfile {
        label,
        engine: CombatSearchEngineProfile {
            budget: CombatSearchBudgetSpec {
                max_nodes: 50,
                wall_ms: 100,
            },
            plugins: CombatSearchPluginStack::default(),
        },
        policy: CombatSearchAttemptPolicy {
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::None,
        },
    }
}

#[test]
fn engine_identity_ignores_label_and_attempt_policy() {
    let base = test_profile("immediate");
    let renamed = CombatSearchProfile {
        label: "renamed",
        policy: CombatSearchAttemptPolicy {
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            artifacts: CombatSearchArtifactPluginId::FullTrace,
        },
        ..base
    };

    assert_eq!(base.engine, renamed.engine);
    assert_eq!(base.engine_fingerprint(), renamed.engine_fingerprint());
}

#[test]
fn engine_identity_distinguishes_child_rollout_policy() {
    let immediate = test_profile("immediate")
        .with_child_rollout_plugin(CombatSearchChildRolloutPluginId::Immediate);
    let lazy = test_profile("lazy")
        .with_child_rollout_plugin(CombatSearchChildRolloutPluginId::LazyOnPop);

    assert_ne!(immediate.engine, lazy.engine);
    assert_ne!(immediate.engine_fingerprint(), lazy.engine_fingerprint());
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```powershell
cargo test --lib ai::combat_search_v2::plugins::tests::engine_identity -- --nocapture
```

Expected: compilation fails because `CombatSearchEngineProfile`, `CombatSearchAttemptPolicy`, and nested fields do not exist.

- [ ] **Step 3: Introduce the split profile types**

Replace the flat profile fields in `plugins.rs` with:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchEngineProfile {
    pub budget: CombatSearchBudgetSpec,
    pub plugins: CombatSearchPluginStack,
}

impl CombatSearchEngineProfile {
    pub fn fingerprint(self) -> String {
        serde_json::to_string(&self).expect("combat search engine profile should serialize")
    }

    pub fn to_config(self) -> CombatSearchV2Config {
        let defaults = CombatSearchV2Config::default();
        CombatSearchV2Config {
            max_nodes: self.budget.max_nodes,
            wall_time: Some(Duration::from_millis(self.budget.wall_ms)),
            potion_policy: self.plugins.potion.policy,
            max_potions_used: self.plugins.potion.max_potions_used,
            rollout_policy: self.plugins.rollout.into(),
            child_rollout_policy: self.plugins.child_rollout.into(),
            turn_plan_policy: self.plugins.turn_plan.into(),
            frontier_policy: self.plugins.frontier.into(),
            phase_guard_policy: self.plugins.phase_guard.into(),
            setup_bias_policy: self.plugins.action_prior.into(),
            ..defaults
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchAttemptPolicy {
    pub acceptance: CombatSearchAcceptancePluginId,
    pub artifacts: CombatSearchArtifactPluginId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CombatSearchProfile {
    pub label: &'static str,
    pub engine: CombatSearchEngineProfile,
    pub policy: CombatSearchAttemptPolicy,
}
```

Update builders to mutate `self.engine.plugins`, `with_acceptance` to mutate `self.policy.acceptance`, `to_config()` to delegate to `self.engine.to_config()`, and add:

```rust
pub fn engine_fingerprint(self) -> String {
    self.engine.fingerprint()
}
```

Update all listed constructors and field reads mechanically to the nested shape. `effective_search_profile` must read `profile.policy.acceptance`; no acceptance behavior changes.

- [ ] **Step 4: Run focused profile and setup tests**

Run:

```powershell
cargo test --lib ai::combat_search_v2::plugins::tests -- --nocapture
cargo test --lib eval::run_control::combat_search::tests::search_config_uses_profile_as_default_config_source -- --nocapture
```

Expected: both commands pass; existing profile-to-config assertions remain unchanged.

- [ ] **Step 5: Check every profile consumer**

Run:

```powershell
cargo check --all-targets
```

Expected: all library and binary profile constructors compile with no flat-field references remaining.

- [ ] **Step 6: Commit the profile boundary**

```powershell
git add src/ai/combat_search_v2 src/eval/run_control src/bin/combat_case_review src/runtime/branch/owner_audit
git commit -m "refactor: separate combat search engine profile"
```

---

### Task 2: Add A Monotone Combat Search Incumbent

**Files:**
- Create: `src/runtime/branch/owner_audit/combat_search_incumbent.rs`
- Modify: `src/runtime/branch/owner_audit.rs`

**Interfaces:**
- Produces: `CombatSearchCandidateTier`, `CombatSearchCandidateFacts`, `CombatSearchIncumbentDecision`, and `CombatSearchIncumbent::{new, offer, selected_index}`.
- Consumes later: post-primary attempt indices and exact candidate facts; it never owns or mutates a `RunControlSession`.

- [ ] **Step 1: Register the module and write failing ordering tests**

Add `#[path = "owner_audit/combat_search_incumbent.rs"] mod combat_search_incumbent;` and tests covering:

```rust
#[test]
fn same_cost_higher_hp_replaces_incumbent() {
    let mut incumbent = CombatSearchIncumbent::new();
    incumbent.offer(0, reserve_win(38, 2));
    let decision = incumbent.offer(1, reserve_win(48, 2));

    assert!(decision.replaced);
    assert_eq!(decision.reason, "strict_resource_dominance");
    assert_eq!(incumbent.selected_index(), Some(1));
}

#[test]
fn incomparable_resource_trade_preserves_incumbent() {
    let mut incumbent = CombatSearchIncumbent::new();
    incumbent.offer(0, reserve_win(38, 1));
    let decision = incumbent.offer(1, reserve_win(48, 2));

    assert!(!decision.replaced);
    assert_eq!(decision.reason, "incomparable_resource_trade");
    assert_eq!(incumbent.selected_index(), Some(0));
}

#[test]
fn relaxed_win_cannot_replace_reserve_compliant_win() {
    let mut incumbent = CombatSearchIncumbent::new();
    incumbent.offer(0, reserve_win(25, 2));
    let decision = incumbent.offer(1, relaxed_win(50, 0));

    assert!(!decision.replaced);
    assert_eq!(decision.reason, "lower_candidate_tier");
}
```

- [ ] **Step 2: Run the module test and verify it fails**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_incumbent::tests -- --nocapture
```

Expected: compilation fails because the new module types are not implemented.

- [ ] **Step 3: Implement the incumbent**

Implement these exact public-within-owner-audit shapes:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CombatSearchCandidateTier {
    SurvivalFallback,
    RelaxedCompleteWin,
    ReserveCompliantCompleteWin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatSearchCandidateFacts {
    pub(super) terminal_run_victory: bool,
    pub(super) tier: CombatSearchCandidateTier,
    pub(super) combat_final_hp: i32,
    pub(super) run_hp: i32,
    pub(super) potions_used: u32,
    pub(super) potions_discarded: u32,
    pub(super) turns: u32,
    pub(super) action_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatSearchIncumbentDecision {
    pub(super) replaced: bool,
    pub(super) reason: &'static str,
}

pub(super) struct CombatSearchIncumbent {
    selected: Option<(usize, CombatSearchCandidateFacts)>,
}
```

`offer` must apply the design order: terminal victory, tier, Pareto dominance over run HP/potions used/potions discarded, then turns/actions only when the resource tuple is equal. Preserve the incumbent on incomparable facts.

- [ ] **Step 4: Run the incumbent tests**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_incumbent::tests -- --nocapture
```

Expected: all incumbent ordering tests pass in under one second after the binary is linked.

- [ ] **Step 5: Commit the incumbent**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/combat_search_incumbent.rs
git commit -m "feat: add combat search incumbent ordering"
```

---

### Task 3: Make Lane Attempts Non-Mutating Candidates

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_runner.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_output.rs`

**Interfaces:**
- Produces: `CombatSearchLaneAttempt::{candidate_facts, commit_into, mark_incumbent_decision}`.
- Changes: `run_lane_attempt(root: &RunControlSession, ...)` no longer writes to `root`.
- Consumes: `CombatSearchCandidateFacts` and the owner reserve limit returned by `owner_audit_hp_loss_limit`.

- [ ] **Step 1: Write failing non-mutation and candidate-classification tests**

Add a focused runner test using the existing blank combat helpers. Preserve a clone of the root, run a one-node rejected attempt, and assert its engine state, run HP, and active-combat HP are unchanged. Add a pure helper test for tier classification:

```rust
assert_eq!(
    candidate_tier(Some(42), Some(60)),
    CombatSearchCandidateTier::ReserveCompliantCompleteWin
);
assert_eq!(
    candidate_tier(Some(67), Some(60)),
    CombatSearchCandidateTier::RelaxedCompleteWin
);
assert_eq!(
    candidate_tier(None, Some(60)),
    CombatSearchCandidateTier::SurvivalFallback
);
```

- [ ] **Step 2: Run focused tests and verify failure**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_lane_runner::tests -- --nocapture
```

Expected: compilation fails because attempts do not retain a trial session or candidate facts and the runner still takes `&mut RunControlSession`.

- [ ] **Step 3: Separate candidate generation from commit**

Change the attempt fields from immediate `committed` mutation to:

```rust
trial_session: Option<RunControlSession>,
pub(super) applicable: bool,
pub(super) selected: bool,
pub(super) incumbent_reason: &'static str,
pub(super) candidate_facts: Option<CombatSearchCandidateFacts>,
pub(super) engine_fingerprint: String,
```

Change the runner signature to immutable input:

```rust
pub(super) fn run_lane_attempt(
    root: &RunControlSession,
    request: &CombatSearchRequest,
    lane: CombatSearchLane,
) -> Result<CombatSearchLaneAttempt, String>
```

Always run on `let mut trial = root.clone();`. Compute `applicable` with the existing `lane_commits` predicate but never assign `trial` to `root`.

For post-primary lanes, set the run-control HP limit to `RunControlHpLossLimit::Unlimited` so an exact clean win can become a relaxed candidate. Preserve the owner reserve separately and classify a `best_win` summary by its `hp_loss`. A successful applicable result without `best_win` is `SurvivalFallback`.

Implement commit as the only mutation method:

```rust
pub(super) fn commit_into(&mut self, session: &mut RunControlSession) -> Result<(), String> {
    if !self.applicable {
        return Err(format!("lane {} has no applicable trial", self.label));
    }
    let trial = self
        .trial_session
        .take()
        .ok_or_else(|| format!("lane {} trial session already consumed", self.label))?;
    *session = trial;
    self.selected = true;
    Ok(())
}
```

- [ ] **Step 4: Split diagnostic collection from selected output**

Make `CombatSearchPortfolioOutput::collect_attempt` collect search summaries and high-loss diagnostics from every attempt, but not auto steps or applied operations. Add:

```rust
pub(super) fn collect_selected_attempt(&mut self, attempt: &CombatSearchLaneAttempt) {
    let Some(outcome) = attempt.outcome.as_ref() else { return; };
    self.auto_steps.extend(outcome.auto_applied_steps.clone());
    self.applied_operations = attempt.applied_operations;
}
```

- [ ] **Step 5: Run runner and output tests**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_lane_runner::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_output::tests -- --nocapture
```

Expected: the root non-mutation, tier classification, and selected-output ownership tests pass.

- [ ] **Step 6: Commit non-mutating attempts**

```powershell
git add src/runtime/branch/owner_audit/combat_search_lane_options.rs src/runtime/branch/owner_audit/combat_search_lane_runner.rs src/runtime/branch/owner_audit/combat_search_portfolio_output.rs
git commit -m "refactor: return combat search trial candidates"
```

---

### Task 4: Arbitrate Post-Primary Attempts And Commit Once

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_orchestrator.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_result.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_commit.rs`

**Interfaces:**
- Consumes: non-mutating `CombatSearchLaneAttempt` values and `CombatSearchIncumbent`.
- Produces: one selected trial session, selected status/stop kind, and diagnostics from all attempted lanes.

- [ ] **Step 1: Extract an injectable post-primary arbitration helper and write failing tests**

Create a helper that accepts a root session, planned lanes, and a closure matching:

```rust
FnMut(&RunControlSession, CombatSearchLane) -> Result<CombatSearchLaneAttempt, String>
```

Use synthetic attempts to prove:

- every closure call receives the same root HP;
- a 38 HP/two-potion attempt followed by a 48 HP/two-potion attempt selects the latter;
- only the selected attempt's trial session is committed;
- lower-tier and incomparable later attempts do not replace the incumbent;
- all attempts remain in diagnostic output.

- [ ] **Step 2: Run orchestrator tests and verify failure**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_orchestrator::tests -- --nocapture
```

Expected: compilation fails because the orchestrator still mutates per lane and stops at first acceptance.

- [ ] **Step 3: Preserve the primary fast path explicitly**

Run primary as a mutable local attempt. If it has an accepted line or valid operation-budget chunk, call `primary.commit_into(session)`, collect it as selected output, and return exactly as before. If it reports a true combat gap, leave the real session untouched and enter post-primary arbitration.

- [ ] **Step 4: Implement post-primary incumbent arbitration**

Run all planned post-primary lanes against `&root`. Offer each applicable candidate to `CombatSearchIncumbent`, record its decision reason, and do not break on the first accepted result. After all producers finish:

```rust
if let Some(selected_index) = incumbent.selected_index() {
    attempts[selected_index].commit_into(session)?;
    output.collect_selected_attempt(&attempts[selected_index]);
    status = attempts[selected_index].status.clone();
} else {
    status = attempts
        .last()
        .map(|attempt| attempt.status.clone())
        .unwrap_or(status);
}
```

Collect diagnostics from every attempt only after its final incumbent reason and selected flag are known.

- [ ] **Step 5: Run orchestrator and existing commit tests**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_orchestrator::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::combat_search_lane_commit::tests -- --nocapture
```

Expected: primary chunk behavior remains green; post-primary tests prove same-root execution and exactly one commit.

- [ ] **Step 6: Commit the orchestration boundary**

```powershell
git add src/runtime/branch/owner_audit/combat_search_orchestrator.rs src/runtime/branch/owner_audit/combat_search_portfolio_result.rs src/runtime/branch/owner_audit/combat_search_lane_commit.rs
git commit -m "fix: arbitrate combat search candidates before commit"
```

---

### Task 5: Make Hallway Producers Complementary And Trace The Winner

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lanes.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_runner.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_report.rs`
- Modify: `src/runtime/branch/owner_audit/combat_portfolio_json.rs`
- Modify: `src/runtime/branch/owner_audit/render.rs`
- Modify: `src/eval/run_control/trace_annotation.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`

**Interfaces:**
- Produces: complementary Immediate/Lazy hallway profiles, duplicate-producer suppression, and winner metadata in both trace summaries and portfolio reports.
- Preserves: the number and configured budget of the two existing semantic-potion hallway roles.

- [ ] **Step 1: Write failing schedule and trace tests**

Add tests proving:

```rust
assert_eq!(quality.engine.plugins.child_rollout, CombatSearchChildRolloutPluginId::Immediate);
assert_eq!(survival.engine.plugins.child_rollout, CombatSearchChildRolloutPluginId::LazyOnPop);
assert_eq!(quality.engine.budget, survival.engine.budget);
assert_ne!(quality.engine_fingerprint(), survival.engine_fingerprint());
```

Add a duplicate-plan test that supplies the same lane twice and expects the second producer to be suppressed. Add JSON assertions that exactly one attempt has `"selected": true`, every attempt has `engine_fingerprint` and `incumbent_reason`, and portfolio `action_keys` comes from the selected attempt.

- [ ] **Step 2: Run focused schedule/report tests and verify failure**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::combat_portfolio_json::tests -- --nocapture
```

Expected: the survival profile is still Immediate and trace/report winner fields do not exist.

- [ ] **Step 3: Switch only the hallway survival producer to Lazy**

In `lane_profile`, construct `HallwaySurvivalFallback` with `CombatSearchChildRolloutPluginId::LazyOnPop`. Keep its budget, phase guard, semantic potion limit, internal no-win rescue, and Smoke Bomb permissions unchanged.

- [ ] **Step 4: Suppress duplicate producers by effective identity**

Define a producer key containing `CombatSearchEngineProfile`, `internal_no_win_rescue_enabled`, and `smoke_bomb_survival_fallback_enabled`. Deduplicate the post-primary lane vector in stable order before execution. Record `duplicate_engine_suppressed` for diagnostics when suppression occurs; labels and artifact policy must not make otherwise identical producers distinct.

- [ ] **Step 5: Extend trace and report schemas compatibly**

Add optional/defaulted fields to `CombatSearchTraceSummary` and concrete fields to `CombatSearchLaneReport`:

```rust
pub engine_fingerprint: Option<String>,
pub portfolio_candidate_tier: Option<String>,
pub portfolio_selected: Option<bool>,
pub portfolio_decision: Option<String>,
```

Populate them in `combat_search_summaries`. Add equivalent `engine_fingerprint`, `candidate_tier`, `selected`, `incumbent_reason`, `combat_final_hp`, `run_hp`, `potions_used`, and `turns` values to owner-audit capsule and trace JSON. Change `combat_portfolio_report` to derive its action list from `attempts.iter().find(|attempt| attempt.selected)`.

- [ ] **Step 6: Update existing report fixtures and run focused tests**

Run:

```powershell
cargo test --lib eval::run_control::combat_line_trace::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::combat_portfolio_json::tests -- --nocapture
cargo test --lib runtime::branch::owner_audit::run_capsule_format::tests -- --nocapture
```

Expected: backward-compatible summary deserialization passes and new owner-audit JSON identifies the selected incumbent.

- [ ] **Step 7: Commit schedule and observability**

```powershell
git add src/runtime/branch/owner_audit src/eval/run_control/trace_annotation.rs
git commit -m "feat: trace complementary combat search portfolio"
```

---

### Task 6: Verify The Architecture And Seed006 Probe

**Files:**
- Modify only if verification exposes a defect in files already listed above.
- Generate ignored evidence under: `artifacts/runs/seed006-combat-incumbent-portfolio-20260715/`

**Interfaces:**
- Verifies: default architectural invariants and the opt-in Transient evidence without adding a linked seed regression.

- [ ] **Step 1: Format and check the diff**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
```

Expected: both commands exit zero.

- [ ] **Step 2: Run focused owner-audit coverage together**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_ -- --nocapture
```

Expected: all incumbent, lane, plan, report, and orchestrator tests pass.

- [ ] **Step 3: Run full required test suites**

Run:

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both suites pass with zero failures.

- [ ] **Step 4: Rebuild the bounded owner-audit executable without deleting caches**

Run:

```powershell
cargo build --bin branch_tiny
```

Expected: the binary links successfully using the existing target cache.

- [ ] **Step 5: Run the opt-in seed006 mainline probe**

Run:

```powershell
& 'target\debug\branch_tiny.exe' `
  --seed 20260713006 `
  --ascension 0 `
  --objective first-victory `
  --generations 60 `
  --max-branches 1 `
  --auto-ops 64 `
  --search-nodes 50000 `
  --search-ms 1000 `
  --rescue-search-nodes 2000000 `
  --rescue-search-ms 20000 `
  --boss-search-nodes 2000000 `
  --boss-search-ms 20000 `
  --wall-ms 3600000 `
  --run-capsule 'artifacts\runs\seed006-combat-incumbent-portfolio-20260715'
```

Expected: the Transient portfolio trace contains both Immediate and Lazy semantic-potion attempts, exactly one selected candidate, no duplicate engine fingerprint, and it does not select the known 38 HP/two-potion line when the replayable 48 HP/two-potion line is present.

- [ ] **Step 6: Inspect status and commit any verification-only fixes**

Run:

```powershell
git status --short
git diff --check
```

Expected: generated run evidence is ignored; only intentional source changes appear. If Step 1-5 required a source correction, commit it as:

```powershell
git add src
git commit -m "fix: preserve combat portfolio verification invariants"
```

- [ ] **Step 7: Record the final commit range**

Run:

```powershell
git log --oneline 1d4e2ac..HEAD
```

Expected: focused commits exist for the profile split, incumbent, non-mutating candidates, single-commit orchestration, and winner trace.
