# Combat Line Adjudication Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the combat search profile the only source of line-acceptance policy, adjudicate each complete line once in run-control, and preserve the final typed decision through owner-audit and run artifacts.

**Architecture:** Combat search continues to discover and score combat-state trajectories. A focused run-control adjudication module maps the profile acceptance plugin to a policy, observes run-level effects from exact replay, and returns a typed accepted, rejected, or replay-failed result. Owner-audit consumes the result without recounting curses, while additive trace and capsule fields distinguish raw search feasibility from execution acceptance.

**Tech Stack:** Rust 2021, serde/serde_json, existing combat-search V2 and run-control APIs, Cargo library tests, `architecture_runtime_boundaries`.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree because it would duplicate the large Cargo test build.
- Execute inline with `superpowers:executing-plans`; do not use subagents while the app cannot control subagent effort.
- Start from a clean Git status and make one bounded local commit per task.
- Never run `cargo clean`.
- Use focused tests for red/green work. Run the complete library and `architecture_runtime_boundaries` suites only at the final checkpoint.
- Do not change combat ordering, scoring, repair, potion policy, partial-line behavior, card rewards, deck construction, routing, shops, or campfires.
- Do not add Writhing Mass strategy rules or move master-deck knowledge into `ai::combat_search_v2`.
- Do not add seed-order, exact action-sequence, exact HP, transient score, node-count, or wall-time assertions.
- Preserve old capsule and trace deserialization through additive serde defaults; never fabricate execution acceptance from an old search-level `accepted_win`.

---

## File Responsibility Map

- Create `src/eval/run_control/combat_line_adjudication.rs`: serializable adjudication vocabulary and the only mapping from `CombatSearchAcceptancePluginId` to run-level acceptance behavior.
- Modify `src/eval/run_control/combat_line_outcome.rs`: observe exact replay effects and return the public observed-outcome type; retain clean-alternative ranking only.
- Modify `src/eval/run_control/combat_line_selector.rs`: require an explicit policy, select/repair the line, and invoke clean-alternative work only for clean-only policy.
- Modify `src/eval/run_control/combat_search_setup.rs`: construct a named effective search policy for explicit profiles and legacy manual search.
- Modify `src/eval/run_control/combat_search.rs`: pass the policy to the selector and attach the resulting adjudication to accepted and rejected outcomes.
- Modify `src/eval/run_control/combat_search_rejection.rs`: carry structured adjudication alongside the compatibility rejection enum and rendered message.
- Modify `src/eval/run_control/session.rs`: expose the latest adjudication on `RunControlCommandOutcome` without making it resumable state.
- Modify `src/eval/run_control/auto_step.rs`: retain structured rejection detail while aggregating automatic search attempts.
- Modify `src/eval/run_control/trace_annotation.rs` and `src/eval/run_control/combat_line_trace.rs`: persist adjudication with the corresponding search performance snapshot and summary.
- Modify `src/runtime/branch/owner_audit/combat_search_lane_runner.rs`, `combat_search_lanes.rs`, and `owner_audit.rs`: remove the second dirty-win decision and keep only commit policy.
- Delete `src/runtime/branch/owner_audit/combat_search_dirty_win.rs`: its production behavior is replaced by run-control adjudication.
- Modify `src/runtime/branch/owner_audit/primary_search_outcome.rs`, `src/runtime/branch/owner_audit/run_capsule_format.rs`, `src/runtime/branch/owner_audit/run_slice_result.rs`, `src/runtime/branch/slice_result.rs`, and `src/runtime/branch/panel.rs`: project the final adjudication and label legacy raw wins honestly.
- Modify `tests/architecture_runtime_boundaries.rs`: protect the single-owner boundary without locking implementation details of combat strategy.

---

### Task 1: Add the typed adjudication contract

**Files:**
- Create: `src/eval/run_control/combat_line_adjudication.rs`
- Modify: `src/eval/run_control/mod.rs`
- Modify: `src/eval/run_control/combat_line_outcome.rs`

**Interfaces:**
- Consumes: `CombatSearchAcceptancePluginId`, `CombatTerminal`, and the existing `CardSnapshot`.
- Produces: `CombatLineAcceptancePolicy::from_plugin`, `CombatLineObservedOutcomeV1`, `CombatLineCleanlinessV1`, `CombatLineRejectionReasonV1`, and `CombatLineAdjudicationV1`.

- [ ] **Step 1: Write the failing policy contract test**

Add this test module to the new `combat_line_adjudication.rs` file before defining the production types:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;

    fn parasite_outcome() -> CombatLineObservedOutcomeV1 {
        CombatLineObservedOutcomeV1 {
            terminal: CombatTerminal::Win,
            final_hp: 44,
            hp_loss: 0,
            potions_used: 0,
            action_count: 32,
            gold_delta: 0,
            ritual_dagger_growth: 0,
            gained_curses: vec![CardSnapshot {
                id: CardId::Parasite,
                uuid: 9001,
                upgrades: 0,
            }],
        }
    }

    #[test]
    fn acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly() {
        let outcome = parasite_outcome();

        for plugin in [
            CombatSearchAcceptancePluginId::AcceptedLineOnly,
            CombatSearchAcceptancePluginId::AcceptedLineOrPrimaryChunk,
        ] {
            assert_eq!(
                CombatLineAcceptancePolicy::from_plugin(plugin).adjudicate(outcome.clone()),
                CombatLineAdjudicationV1::Accepted {
                    policy: plugin,
                    cleanliness: CombatLineCleanlinessV1::Dirty,
                    observed_outcome: outcome.clone(),
                }
            );
        }

        assert_eq!(
            CombatLineAcceptancePolicy::from_plugin(
                CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
            )
            .adjudicate(outcome.clone()),
            CombatLineAdjudicationV1::Rejected {
                policy: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
                reason: CombatLineRejectionReasonV1::NewCurse {
                    cards: outcome.gained_curses.clone(),
                },
                observed_outcome: outcome,
            }
        );
    }
}
```

- [ ] **Step 2: Run the focused test and verify the contract is absent**

Run:

```powershell
cargo test --lib eval::run_control::combat_line_adjudication::tests::acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly -- --exact
```

Expected: compilation fails because the adjudication types and module exports do not exist yet.

- [ ] **Step 3: Implement the serializable vocabulary and explicit policy mapping**

Define the types in `combat_line_adjudication.rs` with the following public shape:

```rust
use serde::{Deserialize, Serialize};

use crate::ai::combat_search_v2::CombatSearchAcceptancePluginId;
use crate::sim::combat::CombatTerminal;

use super::transition_report::CardSnapshot;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatLineCleanlinessV1 {
    Clean,
    Dirty,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatLineRejectionReasonV1 {
    NewCurse { cards: Vec<CardSnapshot> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatLineObservedOutcomeV1 {
    pub terminal: CombatTerminal,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub potions_used: u32,
    pub action_count: usize,
    pub gold_delta: i32,
    pub ritual_dagger_growth: i32,
    pub gained_curses: Vec<CardSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CombatLineAdjudicationV1 {
    Accepted {
        policy: CombatSearchAcceptancePluginId,
        cleanliness: CombatLineCleanlinessV1,
        observed_outcome: CombatLineObservedOutcomeV1,
    },
    Rejected {
        policy: CombatSearchAcceptancePluginId,
        reason: CombatLineRejectionReasonV1,
        observed_outcome: CombatLineObservedOutcomeV1,
    },
    ReplayFailed {
        policy: CombatSearchAcceptancePluginId,
        error: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatLineAcceptancePolicy {
    plugin: CombatSearchAcceptancePluginId,
    reject_gained_curses: bool,
}

impl CombatLineAcceptancePolicy {
    pub(super) fn from_plugin(plugin: CombatSearchAcceptancePluginId) -> Self {
        Self {
            plugin,
            reject_gained_curses: matches!(
                plugin,
                CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
            ),
        }
    }

    pub(super) fn adjudicate(
        self,
        outcome: CombatLineObservedOutcomeV1,
    ) -> CombatLineAdjudicationV1 {
        if self.reject_gained_curses && !outcome.gained_curses.is_empty() {
            return CombatLineAdjudicationV1::Rejected {
                policy: self.plugin,
                reason: CombatLineRejectionReasonV1::NewCurse {
                    cards: outcome.gained_curses.clone(),
                },
                observed_outcome: outcome,
            };
        }
        CombatLineAdjudicationV1::Accepted {
            policy: self.plugin,
            cleanliness: if outcome.gained_curses.is_empty() {
                CombatLineCleanlinessV1::Clean
            } else {
                CombatLineCleanlinessV1::Dirty
            },
            observed_outcome: outcome,
        }
    }

    pub(super) fn requires_clean_line(self) -> bool {
        self.reject_gained_curses
    }

    pub(super) fn plugin(self) -> CombatSearchAcceptancePluginId {
        self.plugin
    }
}
```

Register the module in `run_control/mod.rs` and publicly re-export the four `V1` evidence types.
Do not implement `Default` for `CombatLineAcceptancePolicy`.

Change `evaluate_combat_candidate_line_outcome` in `combat_line_outcome.rs` to build
`CombatLineObservedOutcomeV1`. Keep `CombatLineEvaluation` and `CombatLineAlternative` internal;
replace their old `CombatLineOutcome` field with `CombatLineObservedOutcomeV1`. Update the renderer
and alternative ranking to use the public fields.

Keep the old `combat_line_outcome::CombatLineAcceptancePolicy` and `CombatLineAcceptance` only as
a one-commit compatibility bridge so Task 1 compiles with the unchanged selector. Update its
`classify` parameter to `&CombatLineObservedOutcomeV1`. Do not re-export that legacy policy. Task 2
deletes both legacy types immediately after the selector switches to the explicit policy.

- [ ] **Step 4: Run the focused test and the outcome module tests**

Run:

```powershell
cargo test --lib acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly
cargo test --lib combat_line_outcome
```

Expected: both commands exit 0; the policy test proves the same `Parasite` outcome is accepted by
ordinary profiles and rejected by the clean-only profile.

- [ ] **Step 5: Commit the contract**

```powershell
git add src/eval/run_control/combat_line_adjudication.rs src/eval/run_control/combat_line_outcome.rs src/eval/run_control/mod.rs
git commit -m "refactor: add typed combat line adjudication"
```

---

### Task 2: Wire explicit profile acceptance through run-control

**Files:**
- Modify: `src/eval/run_control/combat_search_setup.rs`
- Modify: `src/eval/run_control/combat_line_selector.rs`
- Modify: `src/eval/run_control/combat_search.rs`
- Modify: `src/eval/run_control/combat_search_rejection.rs`
- Modify: `src/eval/run_control/session.rs`
- Test: `src/eval/run_control/combat_search_setup.rs`

**Interfaces:**
- Consumes: Task 1's `CombatLineAcceptancePolicy` and adjudication types.
- Produces: `EffectiveCombatSearchProfile`, `PreparedCombatSearch.effective_profile`, an explicit-policy selector, and `RunControlCommandOutcome.execution_adjudication`.

- [ ] **Step 1: Write the failing effective-profile test**

Add this focused test to `combat_search_setup.rs`:

```rust
#[cfg(test)]
mod adjudication_tests {
    use super::*;
    use crate::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchBudgetSpec,
        CombatSearchPluginStack, CombatSearchProfile,
    };
    use crate::state::core::{
        ActiveCombat, CombatContext, EngineState, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;

    fn active_session() -> RunControlSession {
        let mut session = RunControlSession::new(Default::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn prepared_search_always_carries_named_acceptance() {
        let session = active_session();
        let manual = prepare_search_combat(&session, RunControlSearchCombatOptions::default())
            .expect("manual search should prepare");
        assert_eq!(manual.effective_profile.profile_id, "manual_default");
        assert_eq!(
            manual.effective_profile.acceptance,
            CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
        );

        let profile = CombatSearchProfile {
            label: "primary",
            budget: CombatSearchBudgetSpec {
                max_nodes: 10,
                wall_ms: 20,
            },
            plugins: CombatSearchPluginStack::default(),
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::None,
        };
        let prepared = prepare_search_combat(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        )
        .expect("profile search should prepare");
        assert_eq!(prepared.effective_profile.profile_id, "primary");
        assert_eq!(
            prepared.effective_profile.acceptance,
            CombatSearchAcceptancePluginId::AcceptedLineOnly
        );
    }
}
```

- [ ] **Step 2: Run the test and verify the effective profile is missing**

Run:

```powershell
cargo test --lib prepared_search_always_carries_named_acceptance
```

Expected: compilation fails because `PreparedCombatSearch` has no `effective_profile` field.

- [ ] **Step 3: Add the effective profile identity**

In `combat_search_setup.rs`, add:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EffectiveCombatSearchProfile {
    pub(super) profile_id: &'static str,
    pub(super) acceptance: CombatSearchAcceptancePluginId,
}

fn effective_combat_search_profile(
    options: &RunControlSearchCombatOptions,
) -> EffectiveCombatSearchProfile {
    options.profile.map_or(
        EffectiveCombatSearchProfile {
            profile_id: "manual_default",
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
        },
        |profile| EffectiveCombatSearchProfile {
            profile_id: profile.label,
            acceptance: profile.acceptance,
        },
    )
}
```

Add `effective_profile` to `PreparedCombatSearch` and compute it before moving `options` into the
prepared result. Do not add acceptance to `CombatSearchV2Config`.

- [ ] **Step 4: Require the policy in the selector**

Change the selector signature to:

```rust
pub(super) fn select_accepted_search_combat_line(
    session: &RunControlSession,
    start: &CombatPosition,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
    trajectory: &CombatSearchV2TrajectoryReport,
    policy: CombatLineAcceptancePolicy,
) -> CombatLineSelection
```

Keep replay and alternative failures inside `CombatLineSelection::ReplayFailed` by constructing
`CombatLineAdjudicationV1::ReplayFailed { policy: policy.plugin(), error }`. Give
`SelectedCombatLine` an `adjudication: CombatLineAdjudicationV1` field and replace
`DirtyRejected` with `Rejected { adjudication, detail }`.

The selection flow must be exactly:

```rust
let selected_eval = match evaluate_combat_candidate_line_outcome(
    session,
    start,
    config,
    selected_line.clone(),
) {
    Ok(evaluation) => evaluation,
    Err(error) => {
        return CombatLineSelection::ReplayFailed {
            adjudication: CombatLineAdjudicationV1::ReplayFailed {
                policy: policy.plugin(),
                error,
            },
        };
    }
};

match policy.adjudicate(selected_eval.outcome.clone()) {
    adjudication @ CombatLineAdjudicationV1::Accepted { .. } => {
        return CombatLineSelection::Selected(SelectedCombatLine {
            line: selected_eval.line,
            report: None,
            summary,
            adjudication,
        });
    }
    CombatLineAdjudicationV1::Rejected { .. } if policy.requires_clean_line() => {
        // Search the same report, then the existing bounded no-potion alternative.
    }
    rejected => {
        return CombatLineSelection::Rejected {
            detail: render_combat_line_outcome_detail(&selected_eval.outcome),
            adjudication: rejected,
        };
    }
}
```

When a clean alternative is found, adjudicate its observed outcome and store that accepted-clean
result on `SelectedCombatLine`. Ordinary policies return before either clean-alternative function
is called.

At the same time, delete the legacy `CombatLineAcceptance` enum, the legacy
`combat_line_outcome::CombatLineAcceptancePolicy`, and its `Default` implementation. Change
`find_accepted_alternative_in_report` and `find_clean_no_potion_alternative` to accept Task 1's
explicit policy. A same-report candidate is clean only when its adjudication matches:

```rust
CombatLineAdjudicationV1::Accepted {
    cleanliness: CombatLineCleanlinessV1::Clean,
    ..
}
```

- [ ] **Step 5: Attach the adjudication in `apply_search_combat`**

Extract `prepared.effective_profile`, create the policy with
`CombatLineAcceptancePolicy::from_plugin`, and pass it to the selector.

Add this field to `RunControlCommandOutcome` and initialize it to `None` in all three constructors:

```rust
pub execution_adjudication: Option<CombatLineAdjudicationV1>,
```

Add a crate-private builder:

```rust
pub(in crate::eval::run_control) fn with_execution_adjudication(
    mut self,
    adjudication: CombatLineAdjudicationV1,
) -> Self {
    self.execution_adjudication = Some(adjudication);
    self
}
```

For `Selected`, call `apply_selected_combat_candidate_line` and attach the accepted adjudication.
For `Rejected`, pass the structured adjudication into `CombatSearchRejectionOutcome`, preserve the
compatibility enum `DirtyWinningCandidateRejected`, and attach it in the rejection builder. For
`ReplayFailed`, return `Err` using the stored replay error so owner-audit classifies it as
`AdvanceFailed`, never as an ordinary combat gap.

Extend `CombatSearchRejectionOutcome` exactly as follows:

```rust
pub(super) struct CombatSearchRejectionOutcome {
    pub(super) result: &'static str,
    pub(super) detail: Option<String>,
    pub(super) rejection: RunControlCombatSearchRejection,
    pub(super) trace_source: &'static str,
    pub(super) execution_adjudication: Option<CombatLineAdjudicationV1>,
}
```

Set `execution_adjudication: None` for invalid-card, no-complete-candidate, and HP-limit
rejections. Set it to `Some(adjudication)` only for the policy rejection. In
`build_combat_search_rejection_outcome`, call `with_execution_adjudication` when that field is
present. This keeps the existing rejection enum as a stop category while the typed result carries
the evidence.

- [ ] **Step 6: Run focused run-control tests**

Run:

```powershell
cargo test --lib prepared_search_always_carries_named_acceptance
cargo test --lib eval::run_control::combat_search::tests
cargo test --lib eval::run_control::combat_line_selector
```

Expected: all commands exit 0. Existing HP-loss, potion, line repair, and partial-turn behavior
remains green.

- [ ] **Step 7: Commit the run-control wiring**

```powershell
git add src/eval/run_control/combat_search_setup.rs src/eval/run_control/combat_line_selector.rs src/eval/run_control/combat_search.rs src/eval/run_control/combat_search_rejection.rs src/eval/run_control/session.rs
git commit -m "fix: apply profile combat line acceptance"
```

---

### Task 3: Remove owner-audit's duplicate dirty-win decision

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_runner.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lanes.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Delete: `src/runtime/branch/owner_audit/combat_search_dirty_win.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**
- Consumes: Task 2's run-control adjudication and existing `lane_commits` behavior.
- Produces: a single production adjudicator; owner-audit only derives `BranchStatus` and commit state from the already-adjudicated outcome.

- [ ] **Step 1: Write the failing architecture boundary test**

Append this test to `tests/architecture_runtime_boundaries.rs`:

```rust
#[test]
fn combat_line_adjudication_has_one_production_owner() {
    let selector = std::fs::read_to_string(
        "src/eval/run_control/combat_line_selector.rs",
    )
    .expect("read combat line selector");
    let lane_runner = std::fs::read_to_string(
        "src/runtime/branch/owner_audit/combat_search_lane_runner.rs",
    )
    .expect("read combat search lane runner");
    let owner_audit = std::fs::read_to_string(
        "src/runtime/branch/owner_audit.rs",
    )
    .expect("read owner audit module");

    assert!(!selector.contains("CombatLineAcceptancePolicy::default()"));
    assert!(!lane_runner.contains("reject_dirty_win_status"));
    assert!(!lane_runner.contains("master_deck_curse_count"));
    assert!(!owner_audit.contains("combat_search_dirty_win.rs"));
}
```

- [ ] **Step 2: Run it and verify owner-audit still violates the boundary**

Run:

```powershell
cargo test --test architecture_runtime_boundaries combat_line_adjudication_has_one_production_owner -- --exact
```

Expected: FAIL because lane runner still imports `reject_dirty_win_status`, counts curses, and
owner-audit still registers the old module.

- [ ] **Step 3: Delete the duplicate production path**

In `combat_search_lane_runner.rs`:

- remove card-definition imports used only for curse counting;
- remove `before_curses`;
- replace the `reject_dirty_win_status(...)` call with
  `let status = lane_status(&trial, &outcome);`;
- delete `master_deck_curse_count`.

In `combat_search_lanes.rs`, delete `rejects_new_curses` and adjust the lane test to assert only the
acceptance plugin and commit policy. Delete `combat_search_dirty_win.rs` and remove its path module
declaration from `owner_audit.rs`.

Do not change `lane_commits`; it continues to own only accepted-line versus primary-chunk commit
semantics.

- [ ] **Step 4: Run the boundary and owner tests**

Run:

```powershell
cargo test --test architecture_runtime_boundaries combat_line_adjudication_has_one_production_owner -- --exact
cargo test --lib runtime::branch::owner_audit::combat_search_lanes::tests
cargo test --lib runtime::branch::owner_audit::combat_search_lane_commit::tests
```

Expected: all commands exit 0. No test remains for the deleted duplicate adjudicator.

- [ ] **Step 5: Commit the owner consolidation**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/combat_search_lane_runner.rs src/runtime/branch/owner_audit/combat_search_lanes.rs tests/architecture_runtime_boundaries.rs
git rm src/runtime/branch/owner_audit/combat_search_dirty_win.rs
git commit -m "refactor: remove duplicate dirty win owner guard"
```

---

### Task 4: Persist adjudication in run-control trace evidence

**Files:**
- Modify: `src/eval/run_control/trace_annotation.rs`
- Modify: `src/eval/run_control/combat_line_trace.rs`
- Modify: `src/eval/run_control/combat_search.rs`
- Modify: `src/eval/run_control/combat_search_rejection.rs`
- Modify: `src/eval/run_control/session.rs`
- Modify: `src/eval/run_control/auto_step.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`

**Interfaces:**
- Consumes: Task 2's `RunControlCommandOutcome.execution_adjudication`.
- Produces: additive `execution_adjudication` fields on `CombatSearchPerformanceSnapshotV1` and `CombatSearchTraceSummary`, plus automatic propagation through `with_trace_annotations`.

- [ ] **Step 1: Write the failing trace round-trip test**

Add this test to the existing test module in `combat_line_trace.rs`. It creates a real performance
annotation through the public search report path, so the test does not duplicate the large
snapshot literal:

```rust
#[test]
fn combat_search_trace_round_trips_dirty_adjudication() {
    use crate::ai::combat_search_v2::{
        run_combat_search_v2, CombatSearchAcceptancePluginId, CombatSearchV2Config,
    };
    use crate::content::cards::CardId;
    use crate::eval::run_control::{
        CombatLineAdjudicationV1, CombatLineCleanlinessV1,
        CombatLineObservedOutcomeV1, RunActionCardSnapshotV1,
    };
    use crate::state::core::EngineState;

    let mut combat = crate::test_support::blank_test_combat();
    combat.entities.monsters.clear();
    let start = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
    let report = run_combat_search_v2(
        &start.engine,
        &start.combat,
        CombatSearchV2Config {
            max_nodes: 1,
            ..CombatSearchV2Config::default()
        },
    );
    let session = RunControlSession::new(Default::default());
    let adjudication = CombatLineAdjudicationV1::Accepted {
        policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
        cleanliness: CombatLineCleanlinessV1::Dirty,
        observed_outcome: CombatLineObservedOutcomeV1 {
            terminal: CombatTerminal::Win,
            final_hp: 44,
            hp_loss: 0,
            potions_used: 0,
            action_count: 32,
            gold_delta: 0,
            ritual_dagger_growth: 0,
            gained_curses: vec![RunActionCardSnapshotV1 {
                id: CardId::Parasite,
                uuid: 9001,
                upgrades: 0,
            }],
        },
    };
    let mut annotations = vec![combat_search_performance_trace_annotation(
        "search_combat_rejected_dirty_win",
        &session,
        &start,
        &report,
    )];

    attach_execution_adjudication(&mut annotations, &adjudication);

    let json = serde_json::to_string(&annotations[0]).expect("serialize trace annotation");
    let restored: RunControlTraceAnnotationV1 =
        serde_json::from_str(&json).expect("deserialize trace annotation");
    assert_eq!(restored, annotations[0]);
    assert!(json.contains("Parasite"));
    assert!(json.contains("accepted_line_only"));
}
```

- [ ] **Step 2: Run the test and verify the trace schema lacks adjudication**

Run:

```powershell
cargo test --lib combat_search_trace_round_trips_dirty_adjudication
```

Expected: compilation fails because the performance snapshot has no `execution_adjudication`
field.

- [ ] **Step 3: Add additive trace fields**

Add to both `CombatSearchPerformanceSnapshotV1` and `CombatSearchTraceSummary`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub execution_adjudication: Option<CombatLineAdjudicationV1>,
```

Add `Default` to `CombatSearchTraceSummary`'s derives so focused projection fixtures can specify
only the fields relevant to their contract.

Add `execution_adjudication: None` to the three explicit `CombatSearchTraceSummary` test literals
in `src/runtime/branch/owner_audit/run_capsule_format.rs`. The JSON-built fixture in
`src/bin/combat_case_review/classification.rs` needs no source change because serde defaults the
new optional field.

Initialize the snapshot field to `None` in `combat_search_performance_snapshot`, and copy it in
`combat_search_trace_summaries`.

Add a focused helper in `combat_line_trace.rs`:

```rust
pub(super) fn attach_execution_adjudication(
    annotations: &mut [RunControlTraceAnnotationV1],
    adjudication: &CombatLineAdjudicationV1,
) {
    if let Some(snapshot) = annotations.iter_mut().rev().find_map(|annotation| match annotation {
        RunControlTraceAnnotationV1::CombatSearchPerformance { snapshot } => Some(snapshot),
        _ => None,
    }) {
        snapshot.execution_adjudication = Some(adjudication.clone());
    }
}
```

After accepted-line execution and after rejection-outcome construction, call this helper before
returning the outcome.

- [ ] **Step 4: Make aggregated auto outcomes retain the latest adjudication**

Update `RunControlCommandOutcome::with_trace_annotations` so it scans the incoming annotations for
the last performance snapshot with an adjudication and copies that value into
`self.execution_adjudication` before extending the vector. This makes `auto_step` aggregation
retain the typed result without adding another parallel accumulator.

Do not replace `combat_search_rejection` yet; it remains a compatibility stop-category enum.

- [ ] **Step 5: Preserve rejection detail through auto-step aggregation**

Alongside each existing `*_rejection_kind` local in `auto_step.rs`, retain
`outcome.execution_adjudication.clone()`. Change `combat_search_stop_reason` to return `String` and
accept both the compatibility rejection kinds and the collected adjudications. When the last
relevant adjudication is:

```rust
CombatLineAdjudicationV1::Rejected {
    reason: CombatLineRejectionReasonV1::NewCurse { cards },
    ..
}
```

render the reason with this code:

```rust
let gained_curses = cards
    .iter()
    .map(|card| format!("{:?}#{}", card.id, card.uuid))
    .collect::<Vec<_>>()
    .join(",");
format!(
    "combat search rejected line under clean-only policy: gained_curses=[{gained_curses}]"
)
```

Keep the existing HP-limit, invalid-identity, and no-complete-win fallbacks unchanged. Extend the
existing `combat_search_stop_reason` unit coverage with one `Parasite` assertion; do not add a
seed-specific test.

- [ ] **Step 6: Run focused trace and auto-step tests**

Run:

```powershell
cargo test --lib combat_search_trace_round_trips_dirty_adjudication
cargo test --lib eval::run_control::trace_annotation
cargo test --lib eval::run_control::auto_step::tests
```

Expected: all commands exit 0. The JSON includes the policy and `Parasite`, and auto aggregation
retains the same structured adjudication.

- [ ] **Step 7: Commit durable trace evidence**

```powershell
git add src/eval/run_control/trace_annotation.rs src/eval/run_control/combat_line_trace.rs src/eval/run_control/combat_search.rs src/eval/run_control/combat_search_rejection.rs src/eval/run_control/session.rs src/eval/run_control/auto_step.rs src/runtime/branch/owner_audit/run_capsule_format.rs
git commit -m "feat: persist combat line adjudication evidence"
```

---

### Task 5: Project final adjudication into capsule and slice schemas

**Files:**
- Modify: `src/runtime/branch/slice_result.rs`
- Modify: `src/runtime/branch/owner_audit/primary_search_outcome.rs`
- Modify: `src/runtime/branch/owner_audit/run_capsule_format.rs`
- Modify: `src/runtime/branch/owner_audit/run_slice_result.rs`
- Modify: `src/runtime/branch/panel.rs`

**Interfaces:**
- Consumes: Task 4's `CombatSearchTraceSummary.execution_adjudication`.
- Produces: additive top-level `execution_adjudication`, honest primary status derivation, and `legacy_unknown` for historical raw wins without a final decision.

- [ ] **Step 1: Write the failing projection and legacy tests**

Add the following helpers and test to a new test module in `primary_search_outcome.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, SearchTerminalLabel,
    };
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::{
        CombatLineAdjudicationV1, CombatLineCleanlinessV1,
        CombatLineObservedOutcomeV1, RunActionCardSnapshotV1,
    };
    use sts_simulator::sim::combat::CombatTerminal;

    fn search_attempt_fixture() -> CombatSearchTraceSummary {
        let line = CombatSearchTerminalLineSummary {
            terminal: SearchTerminalLabel::Win,
            final_hp: 44,
            hp_loss: 0,
            turns: 7,
            cards_played: 25,
            potions_used: 0,
            potions_discarded: 0,
            action_count: 32,
        };
        CombatSearchTraceSummary {
            source: "search_combat".to_string(),
            lane: Some("primary".to_string()),
            profile_id: Some("primary".to_string()),
            combat_kind: "hallway".to_string(),
            complete_trajectory_found: true,
            complete_win_found: true,
            best_complete: Some(line.clone()),
            best_win: Some(line),
            ..CombatSearchTraceSummary::default()
        }
    }

    fn dirty_accepted_adjudication() -> CombatLineAdjudicationV1 {
        CombatLineAdjudicationV1::Accepted {
            policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            cleanliness: CombatLineCleanlinessV1::Dirty,
            observed_outcome: CombatLineObservedOutcomeV1 {
                terminal: CombatTerminal::Win,
                final_hp: 44,
                hp_loss: 0,
                potions_used: 0,
                action_count: 32,
                gold_delta: 0,
                ritual_dagger_growth: 0,
                gained_curses: vec![RunActionCardSnapshotV1 {
                    id: CardId::Parasite,
                    uuid: 9001,
                    upgrades: 0,
                }],
            },
        }
    }

#[test]
fn primary_search_distinguishes_execution_acceptance_from_legacy_raw_win() {
    let mut accepted = search_attempt_fixture();
    accepted.execution_adjudication = Some(dirty_accepted_adjudication());
    let accepted_value = primary_search_outcome_value(&[accepted], None);
    assert_eq!(accepted_value["status"], "accepted_dirty_win");
    assert_eq!(
        accepted_value["execution_adjudication"]["observed_outcome"]
            ["gained_curses"][0]["id"],
        "Parasite"
    );

    let mut legacy = search_attempt_fixture();
    legacy.execution_adjudication = None;
    let legacy_value = primary_search_outcome_value(&[legacy], None);
    assert_eq!(legacy_value["status"], "legacy_unknown");
    assert!(legacy_value["accepted_line"].is_null());
}
}
```

Add this test in `run_capsule_format.rs`; import the Task 1 adjudication types and card identifiers
at the top of the existing test module:

```rust
#[test]
fn capsule_projects_execution_adjudication() {
    let adjudication = CombatLineAdjudicationV1::Accepted {
        policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
        cleanliness: CombatLineCleanlinessV1::Dirty,
        observed_outcome: CombatLineObservedOutcomeV1 {
            terminal: CombatTerminal::Win,
            final_hp: 44,
            hp_loss: 0,
            potions_used: 0,
            action_count: 32,
            gold_delta: 0,
            ritual_dagger_growth: 0,
            gained_curses: vec![RunActionCardSnapshotV1 {
                id: CardId::Parasite,
                uuid: 9001,
                upgrades: 0,
            }],
        },
    };
    let mut branch = sample_branch();
    branch.combat_search = vec![CombatSearchTraceSummary {
        source: "search_combat".to_string(),
        lane: Some("primary".to_string()),
        execution_adjudication: Some(adjudication.clone()),
        ..CombatSearchTraceSummary::default()
    }];
    let trajectory_evaluation = evaluation(vec![branch.clone()]);
    let summary = branch_summary_value(
        Path::new("target/test-capsule"),
        sample_args(),
        1,
        &branch,
        &Value::Null,
        &json!([]),
        &trajectory_evaluation,
        "gap",
        None,
        None,
    );
    let result = result_value(
        1,
        &branch,
        Value::Null,
        json!([]),
        &trajectory_evaluation,
    );
    let expected = serde_json::to_value(adjudication).expect("serialize adjudication");

    assert_eq!(summary["execution_adjudication"], expected);
    assert_eq!(result["execution_adjudication"], expected);
}
```

- [ ] **Step 2: Run the tests and verify projections still infer acceptance from raw wins**

Run:

```powershell
cargo test --lib primary_search_distinguishes_execution_acceptance_from_legacy_raw_win
cargo test --lib capsule_projects_execution_adjudication
```

Expected: compilation or assertion failure because the summary types and projections lack the new
field and current status logic treats any `best_win` as `accepted_win`.

- [ ] **Step 3: Extend public slice summaries additively**

Add this serde-defaulted field to `PrimarySearchOutcomeSummary`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub execution_adjudication: Option<CombatLineAdjudicationV1>,
```

Add the same optional field to `RunSliceResult`, initialize it to `None`, and add:

```rust
pub fn with_execution_adjudication(
    mut self,
    adjudication: Option<CombatLineAdjudicationV1>,
) -> Self {
    self.execution_adjudication = adjudication;
    self
}
```

Update the one direct `PrimarySearchOutcomeSummary` literal in `runtime/branch/panel.rs` with
`execution_adjudication: None`.

- [ ] **Step 4: Derive status only from final adjudication**

In `primary_search_outcome.rs`, get the primary attempt's adjudication and set:

```rust
let status = match primary_attempt.and_then(|attempt| attempt.execution_adjudication.as_ref()) {
    Some(CombatLineAdjudicationV1::Accepted {
        cleanliness: CombatLineCleanlinessV1::Clean,
        ..
    }) => "accepted_win",
    Some(CombatLineAdjudicationV1::Accepted {
        cleanliness: CombatLineCleanlinessV1::Dirty,
        ..
    }) => "accepted_dirty_win",
    Some(CombatLineAdjudicationV1::Rejected { .. }) => "no_accepted_line",
    Some(CombatLineAdjudicationV1::ReplayFailed { .. }) => "search_internal_error",
    None if primary_attempt.and_then(|attempt| attempt.best_win.as_ref()).is_some() => {
        "legacy_unknown"
    }
    None => "no_accepted_line",
};
```

Populate `accepted_line` only for an `Accepted` adjudication. Continue projecting
`best_complete_line` from raw search evidence. Store the cloned adjudication on
`PrimarySearchOutcomeSummary`.

- [ ] **Step 5: Add top-level capsule and slice projection**

Add a helper in `primary_search_outcome.rs`:

```rust
pub(super) fn latest_execution_adjudication(
    attempts: &[CombatSearchTraceSummary],
) -> Option<CombatLineAdjudicationV1> {
    attempts
        .iter()
        .rev()
        .find_map(|attempt| attempt.execution_adjudication.clone())
}
```

Use it in `branch_summary_value` and the branch result JSON in `run_capsule_format.rs` to add the
top-level key `execution_adjudication`. Use it in `run_slice_result.rs` to call
`with_execution_adjudication` whenever a selected branch exists.

Keep the existing `primary_search`, `combat_search_attempts`, `combat_search_history`, and
`failed_search` keys. Do not rewrite historical artifacts.

- [ ] **Step 6: Preserve legacy deserialization**

Add this serde test beside the public slice-result tests. It starts from the exact current default
schema, removes only the new key, and proves the additive field is optional:

```rust
#[test]
fn legacy_primary_search_outcome_without_adjudication_still_loads() {
    let mut value = serde_json::to_value(PrimarySearchOutcomeSummary::default())
        .expect("serialize default primary outcome");
    value
        .as_object_mut()
        .expect("primary outcome object")
        .remove("execution_adjudication");

    let restored: PrimarySearchOutcomeSummary =
        serde_json::from_value(value).expect("load legacy primary outcome");

    assert_eq!(restored.execution_adjudication, None);
}
```

Keep all new fields marked with `#[serde(default)]`.

Run:

```powershell
cargo test --lib primary_search_distinguishes_execution_acceptance_from_legacy_raw_win
cargo test --lib capsule_projects_execution_adjudication
cargo test --lib legacy_primary_search_outcome_without_adjudication_still_loads
cargo test --lib runtime::branch::owner_audit::run_capsule_format::tests
```

Expected: all commands exit 0. Raw search wins no longer masquerade as execution acceptance.

- [ ] **Step 7: Commit artifact projection**

```powershell
git add src/runtime/branch/slice_result.rs src/runtime/branch/owner_audit/primary_search_outcome.rs src/runtime/branch/owner_audit/run_capsule_format.rs src/runtime/branch/owner_audit/run_slice_result.rs src/runtime/branch/panel.rs
git commit -m "fix: distinguish search wins from execution acceptance"
```

---

### Task 6: Verify the saved symptom and complete the checkpoint

**Files:**
- Verify only: `target/bounded-mainline-20260712002/combat_cases/seed20260712002_g34_b0034_a3f42_writhingmass.json`
- Verify only: the exact tracked paths listed in the File Responsibility Map and changed by Tasks 1–5

**Interfaces:**
- Consumes: the completed implementation and the preserved Writhing Mass combat case.
- Produces: fresh evidence that search still finds a complete line, policy tests classify the `Parasite` outcome correctly, required repository suites pass, and the worktree is clean.

- [ ] **Step 1: Run all three stable contract categories together**

Run:

```powershell
cargo test --lib acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly
cargo test --lib combat_search_trace_round_trips_dirty_adjudication
cargo test --lib primary_search_distinguishes_execution_acceptance_from_legacy_raw_win
cargo test --test architecture_runtime_boundaries combat_line_adjudication_has_one_production_owner -- --exact
```

Expected: every command exits 0. These cover policy semantics, durable evidence, honest projection,
and single production ownership without pinning a combat sequence.

- [ ] **Step 2: Recheck the saved Writhing Mass search evidence**

Run:

```powershell
cargo run --quiet --bin combat_case_review -- --case "target\bounded-mainline-20260712002\combat_cases\seed20260712002_g34_b0034_a3f42_writhingmass.json" --ladder --compact
```

Expected: the review still reports at least one complete winning line. This command validates the
raw-search half of the original symptom; the Task 1 policy test validates that the resulting
`Parasite` outcome is accepted by `AcceptedLineOnly` and rejected by clean-only policy. Do not
infer full-seed victory from this case review.

- [ ] **Step 3: Run the repository completion suites once**

Run:

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both commands exit 0 with zero failed tests. Do not repeatedly rerun these large linked
test binaries during focused work.

- [ ] **Step 4: Audit the final source boundary and diff**

Run:

```powershell
rg -n "CombatLineAcceptancePolicy::default|reject_dirty_win_status|master_deck_curse_count" src/eval/run_control src/runtime/branch/owner_audit
git diff --check
git status --short
```

Expected: `rg` finds none of the removed production paths, `git diff --check` exits 0, and `git
status --short` is empty because every task ended in a bounded commit.

- [ ] **Step 5: Record the handoff without changing strategy scope**

Report:

- the five implementation commit hashes;
- focused and completion-suite pass counts;
- the saved case's raw complete-win result;
- ordinary-policy `AcceptedDirty` versus clean-policy `Rejected(NewCurse)` evidence;
- confirmation that no full seed rerun or Writhing Mass strategy change was made.

Do not add a sixth source commit solely for the verification report.
