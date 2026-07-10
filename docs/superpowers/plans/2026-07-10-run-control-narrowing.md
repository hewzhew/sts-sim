# Run-Control First-Round Narrowing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire standalone combat SearchEvidence and legacy run-play strategic auto-planning while preserving the shared run-control execution kernel and historical trace compatibility.

**Architecture:** `CombatSearchV2Report` owns its schema identity beside the report type. Run-play and owner-audit automatic advancement share routine-only non-combat behavior; the older branch-experiment mode retains its two explicit event compatibility routines. SearchEvidence creation and loading disappear, but the historical trace artifact enum value and serialized field remain readable.

**Tech Stack:** Rust 2021, serde/serde_json, cargo test, cargo fmt, git.

## Global Constraints

- Do not move `RunControlSession` from `eval` to `runtime` in this plan.
- Do not change combat-search heuristics, policy quality, route planning, reward valuation, or owner decisions.
- Do not retire `branch_experiment` or `branch_campaign` in this plan.
- Keep manual typed commands, recorded card-reward picks, and Singing Bowl actions working.
- Keep `SessionTraceArtifactKind::CombatSearchEvidence` and `SessionTraceArtifactRefV1::search_evidence_path` deserializable for historical traces.
- Removed `save=case|path` search options must return an unknown-option error.
- Use `apply_patch` for source and document edits.

---

### Task 1: Make the Combat Search Report Own Its Schema Identity

**Files:**
- Modify: `src/ai/combat_search_v2/types/report/core.rs`
- Modify: `src/ai/combat_search_v2/search/finalize.rs`
- Modify: `src/ai/combat_search_v2/search/tests.rs`

**Interfaces:**
- Produces: `COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME: &str`
- Produces: `COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION: u32`
- Consumes: existing `CombatSearchV2Report` construction and search report tests

- [ ] **Step 1: Change the report contract test to require producer-owned constants**

Import the new constants in `search/tests.rs` and replace repeated literals:

```rust
use crate::ai::combat_search_v2::{
    COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME, COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION,
};

assert_eq!(report.schema_name, COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME);
assert_eq!(report.schema_version, COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION);
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
cargo test --lib 'ai::combat_search_v2::search::tests::search_report_declares_privileged_policy_evidence_boundary' -- --exact
```

Expected: compilation fails because the two constants do not exist.

- [ ] **Step 3: Add the constants and use them in the producer**

Add beside `CombatSearchV2Report` in `types/report/core.rs`:

```rust
pub const COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME: &str = "CombatSearchV2Report";
pub const COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION: u32 = 11;
```

Use them in `search/finalize.rs`:

```rust
schema_name: COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME,
schema_version: COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION,
```

- [ ] **Step 4: Run focused tests and verify GREEN**

Run:

```powershell
cargo test --lib 'ai::combat_search_v2::search::tests'
```

Expected: all filtered search tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/combat_search_v2/types/report/core.rs src/ai/combat_search_v2/search/finalize.rs src/ai/combat_search_v2/search/tests.rs
git commit -m "Centralize combat search report schema"
```

---

### Task 2: Retire the SearchEvidence Command Contract

**Files:**
- Modify: `src/eval/run_control/commands.rs`
- Modify: `src/eval/run_control/commands/options.rs`
- Modify: `src/eval/run_control/commands/help.rs`
- Modify: `src/eval/run_control/commands/tests.rs`
- Modify: `src/eval/run_control/mod.rs`
- Modify: `src/eval/run_control/auto_step.rs`
- Modify: `src/eval/run_control/combat_auto_policy.rs`
- Modify: `src/ai/combat_auto_policy_v1/types.rs`
- Modify: `src/ai/combat_auto_policy_v1/policy.rs`
- Modify: `src/ai/combat_auto_policy_v1/tests.rs`

**Interfaces:**
- Removes: `RunControlSearchEvidenceTarget`
- Removes: `RunControlSearchCombatOptions::evidence`
- Removes: `CombatAutoSearchContextV1::evidence_requested`
- Preserves: all other search-combat option keys and profile behavior

- [ ] **Step 1: Replace the accepting parser test with a rejection contract**

Replace the `sc save=case` equality assertion in `commands/tests.rs` with:

```rust
#[test]
fn search_combat_rejects_retired_search_evidence_option() {
    let error = parse_run_control_command("sc save=case")
        .expect_err("standalone search evidence is retired");
    assert!(error.contains("unknown search-combat option 'save'"));
}
```

- [ ] **Step 2: Run the parser test and verify RED**

Run:

```powershell
cargo test --lib 'eval::run_control::commands::tests::search_combat_rejects_retired_search_evidence_option' -- --exact
```

Expected: test fails because the parser still accepts `save=case`.

- [ ] **Step 3: Remove the command option and help surface**

Delete `RunControlSearchEvidenceTarget`, the `evidence` field, the parser aliases
`save|evidence|output|out`, `parse_search_evidence_target`, and the help text
`[save=case|path]`. Remove the re-export from `run_control/mod.rs`.

- [ ] **Step 4: Remove the evidence-only auto-search exception**

Delete `CombatAutoSearchContextV1::evidence_requested` and simplify:

```rust
let no_potion_first = context.high_stakes_potion_budget.is_some()
    && !context.has_potion_policy_override()
    && context.hp_loss_gate.is_limited();
```

Remove evidence setup/assertions from the AI policy tests and `auto_step.rs`.
`combat_auto_policy.rs` must stop reading a removed command field.

- [ ] **Step 5: Run parser and combat-auto tests and verify GREEN**

Run:

```powershell
cargo test --lib 'eval::run_control::commands::tests'
cargo test --lib 'ai::combat_auto_policy_v1::tests'
cargo test --lib 'eval::run_control::auto_step::tests'
```

Expected: all three filtered suites pass.

- [ ] **Step 6: Commit**

```powershell
git add src/eval/run_control/commands.rs src/eval/run_control/commands/options.rs src/eval/run_control/commands/help.rs src/eval/run_control/commands/tests.rs src/eval/run_control/mod.rs src/eval/run_control/auto_step.rs src/eval/run_control/combat_auto_policy.rs src/ai/combat_auto_policy_v1/types.rs src/ai/combat_auto_policy_v1/policy.rs src/ai/combat_auto_policy_v1/tests.rs
git commit -m "Retire search evidence command options"
```

---

### Task 3: Remove SearchEvidence Creation and Loader Plumbing

**Files:**
- Delete: `src/eval/run_control/search_evidence.rs`
- Modify: `src/eval/run_control/mod.rs`
- Modify: `src/eval/run_control/combat_search.rs`
- Modify: `src/eval/run_control/combat_search_setup.rs`
- Modify: `src/eval/run_control/combat_search_render.rs`
- Modify: `src/eval/run_control/combat_search_rejection.rs`
- Modify: `src/eval/run_control/combat_line_executor.rs`
- Modify: `src/eval/run_control/combat_no_win_fallback.rs`
- Modify: `src/eval/run_control/session.rs`
- Modify: `src/eval/run_control/session_trace.rs`
- Modify: `src/eval/run_control/session/tests.rs`

**Interfaces:**
- Removes: `save_combat_search_evidence_v1`, loader, validator, envelope types
- Removes: `RunControlCommandOutcome::search_evidence_path`
- Removes: saved-evidence path parameters from combat line/rejection/fallback helpers
- Preserves: `SessionTraceArtifactKind::CombatSearchEvidence`
- Preserves: `SessionTraceArtifactRefV1::search_evidence_path`

- [ ] **Step 1: Add a historical trace compatibility test**

Replace the recorder test that creates new evidence with a deserialization test:

```rust
#[test]
fn historical_search_evidence_artifact_kind_remains_readable() {
    let kind: SessionTraceArtifactKind =
        serde_json::from_str("\"combat_search_evidence\"").expect("historical kind");
    assert_eq!(kind, SessionTraceArtifactKind::CombatSearchEvidence);
}
```

Run it once before deletion as a characterization guard; it should pass.

- [ ] **Step 2: Delete the stale producer/consumer integration test**

Delete only
`run_control_search_combat_can_save_search_evidence_for_capture_case` from
`session/tests.rs`. Retain adjacent capture and ordinary search tests.

- [ ] **Step 3: Remove evidence creation and outcome plumbing**

Apply these mechanical API changes:

```rust
// combat_search.rs
let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());

// RunControlCommandOutcome
// remove search_evidence_path entirely

// rejection input
pub(super) struct CombatSearchRejectionOutcome {
    pub(super) result: &'static str,
    pub(super) detail: Option<String>,
    pub(super) rejection: RunControlCombatSearchRejection,
    pub(super) trace_source: &'static str,
}
```

Remove `save_search_evidence_if_requested`, `next_available_evidence_path`,
`render_saved_evidence_note`, every `saved_evidence` argument, and every
`saved_search=` message fragment. Do not alter combat selection, fallback,
replay, trace annotations, or rejection kinds.

- [ ] **Step 4: Remove new trace recording but keep old trace shape**

Delete `record_search_evidence_artifact` and the outcome-path branch in
`record_command_outcome`. Keep the enum variant and optional serialized field
unchanged.

- [ ] **Step 5: Delete the module and public exports**

Delete `search_evidence.rs`, `mod search_evidence`, and its `pub use` block.

- [ ] **Step 6: Run focused compile/tests and verify GREEN**

Run:

```powershell
cargo test --lib 'eval::run_control::session_trace::tests::historical_search_evidence_artifact_kind_remains_readable' -- --exact
cargo test --lib 'eval::run_control::session::tests::run_control_search_combat_applies_complete_winning_trajectory' -- --exact
cargo test --lib 'eval::run_control::combat_search::tests'
```

Expected: all pass and the crate compiles without SearchEvidence symbols.

- [ ] **Step 7: Confirm no production SearchEvidence symbols remain**

Run:

```powershell
rg -n "SearchEvidence|search_evidence|saved_evidence|search_evidence_path" src/eval/run_control src/ai/combat_auto_policy_v1
```

Expected: only the deliberately preserved historical trace enum variant and
serialized field/test remain.

- [ ] **Step 8: Commit**

```powershell
git add src/eval/run_control
git commit -m "Remove standalone combat search evidence"
```

---

### Task 4: Make Run-Play Automatic Advancement Routine-Only

**Files:**
- Modify: `src/eval/run_control/auto_step.rs`
- Modify: `src/eval/run_control/auto_run.rs`
- Modify: `src/eval/run_control/noncombat_auto.rs`
- Modify: `src/eval/run_control/session/tests.rs`

**Interfaces:**
- Replaces: `NonCombatAutoMode::FullPlanner`
- Produces: routine-only run-play and owner-audit modes that never choose strategic non-combat actions
- Preserves: `NonCombatAutoMode::BranchExperimentBoundary` compatibility routines

- [ ] **Step 1: Rewrite the campfire auto-run test for the desired boundary**

Replace the stale low-HP campfire policy test with:

```rust
#[test]
fn run_control_auto_run_stops_at_low_hp_campfire_without_choosing() {
    let mut session = test_session_at_campfire_with_hp(20, 80);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(RunControlAutoStepOptions {
            max_operations: Some(1),
            ..Default::default()
        }))
        .expect("auto-run should stop at the campfire owner boundary");

    assert!(matches!(session.engine_state, EngineState::Campfire));
    assert_eq!(session.run_state.current_hp, 20);
    assert_eq!(outcome.auto_stop.as_ref().map(|stop| stop.kind), Some(RunControlAutoStopKind::HumanBoundary));
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(record.site, DecisionSiteKindV1::Campfire);
}
```

- [ ] **Step 2: Rewrite representative event, shop, run-choice, and card-reward tests**

For each site, reuse existing fixtures and assert:

```rust
assert_eq!(outcome.auto_stop.as_ref().map(|stop| stop.kind), Some(RunControlAutoStopKind::HumanBoundary));
assert!(outcome.action_result.is_none());
assert_eq!(session.engine_state, state_before);
```

Keep separate tests that prove routine/forced single candidates still advance.
Delete assertions about policy confidence, compiler provenance, or selected
strategic actions from the run-play auto path.

- [ ] **Step 3: Run the rewritten tests and verify RED**

Run the exact rewritten campfire, event, shop, run-choice, and card-reward
tests. Expected: they fail because `FullPlanner` still mutates those choices.

- [ ] **Step 4: Replace FullPlanner with routine-only behavior**

Rename the mode and route both run-play and owner-audit through no strategic
non-combat policy application:

```rust
pub(in crate::eval::run_control) enum NonCombatAutoMode {
    RoutineOnly,
    BranchExperimentBoundary,
}

fn apply_noncombat_policy(
    session: &mut RunControlSession,
    mode: NonCombatAutoMode,
) -> Result<Option<NonCombatAutoApplication>, String> {
    match mode {
        NonCombatAutoMode::RoutineOnly => Ok(None),
        NonCombatAutoMode::BranchExperimentBoundary => {
            super::noncombat_auto::apply_branch_experiment_noncombat_policy(session)
        }
    }
}
```

Remove FullPlanner-only policy-stop annotation handling. Preserve the existing
human-boundary annotation path.

- [ ] **Step 5: Run routine-only and compatibility tests and verify GREEN**

Run:

```powershell
cargo test --lib 'eval::run_control::session::tests::run_control_auto_run_stops_at_low_hp_campfire_without_choosing' -- --exact
cargo test --lib 'run_control_auto_run'
cargo test --lib 'eval::run_control::auto_run::tests'
cargo test --lib 'eval::run_control::auto_step::tests'
```

Expected: run-play strategic boundaries stop, branch-experiment event routines
still pass, and owner-audit emits no legacy policy record.

- [ ] **Step 6: Commit**

```powershell
git add src/eval/run_control/auto_step.rs src/eval/run_control/auto_run.rs src/eval/run_control/noncombat_auto.rs src/eval/run_control/session/tests.rs
git commit -m "Stop run-play automation at owner boundaries"
```

---

### Task 5: Prune FullPlanner-Only Policy Entrypoints and Types

**Files:**
- Delete: `src/eval/run_control/boss_relic_policy.rs`
- Delete: `src/eval/run_control/campfire_policy.rs`
- Delete: `src/eval/run_control/run_choice_policy.rs`
- Modify: `src/eval/run_control/mod.rs`
- Modify: `src/eval/run_control/event_policy.rs`
- Modify: `src/eval/run_control/shop_policy.rs`
- Modify: `src/eval/run_control/card_reward_auto.rs`
- Modify: `src/eval/run_control/noncombat_auto.rs`
- Modify: `src/eval/run_control/session.rs`
- Modify: `src/eval/run_control/render.rs`
- Modify: `src/runtime/branch/owner_audit/trace_format.rs`
- Modify: affected focused tests identified by compiler/`rg`

**Interfaces:**
- Removes: FullPlanner policy application entrypoints
- Removes: `RunControlAutoAppliedKindV1::NoncombatPolicy`
- Removes: `RunControlAutoStopKind::NoncombatPolicyStop`
- Preserves: branch-experiment Match and Keep / Note For Yourself helpers
- Preserves: public `shop_plan_step_input_and_label_v1`
- Preserves: recorded card-reward and Singing Bowl commands

- [ ] **Step 1: Remove module declarations and FullPlanner-only entrypoints**

Delete the three single-purpose modules. Remove only the general auto-policy
functions from the mixed modules:

```text
event_policy.rs:
  remove apply_event_policy_choice
  keep apply_match_and_keep_policy_choice
  keep apply_note_for_yourself_policy_choice

shop_policy.rs:
  remove apply_shop_policy_action
  keep shop_plan_step_input_and_label_v1

card_reward_auto.rs:
  remove apply_card_reward_policy_pick
  remove apply_card_reward_item_open
  remove card_reward_policy_stop_annotation
  keep recorded/manual/Singing Bowl paths and their required helpers
```

- [ ] **Step 2: Remove unreachable auto-result variants**

Delete `NoncombatPolicy` and `NoncombatPolicyStop` plus their renderer/formatter
match arms. These types are not serialized compatibility surfaces.

- [ ] **Step 3: Use compiler and symbol search to prune private dead helpers**

Run:

```powershell
cargo check --lib
rg -n "apply_planner_noncombat_policy|FullPlanner|apply_campfire_policy_action|apply_shop_policy_action|apply_run_choice_policy_deck_selection|apply_boss_relic_policy_pick|apply_event_policy_choice|apply_card_reward_policy_pick|card_reward_policy_stop_annotation" src
```

Fix compile errors without adding compatibility shims. Remove private helpers
that became unreachable; retain helpers used by explicit commands.

- [ ] **Step 4: Run focused retained behavior tests**

Run:

```powershell
cargo test --lib 'eval::run_control::commands::tests'
cargo test --lib 'eval::run_control::auto_run::tests'
cargo test --lib 'recorded_card_reward'
cargo test --lib 'singing_bowl'
cargo test --lib 'eval::run_control::route_policy::tests'
```

Expected: retained manual, branch-experiment, and route behavior passes.

- [ ] **Step 5: Commit**

```powershell
git add src/eval/run_control src/runtime/branch/owner_audit/trace_format.rs
git commit -m "Prune legacy run-control policy entrypoints"
```

---

### Task 6: Full Verification and Documentation Check

**Files:**
- Modify only if verification reveals an in-scope defect

**Interfaces:**
- Consumes: all previous tasks
- Produces: verified first-round narrowing with an explicit failure count

- [ ] **Step 1: Format and inspect the diff**

Run:

```powershell
cargo fmt --all -- --check
git diff --check HEAD~4
git status --short
```

Expected: formatting and whitespace checks pass; only intentional changes are
present.

- [ ] **Step 2: Run architecture-sensitive focused tests**

Run:

```powershell
cargo test --lib 'ai::combat_search_v2::search::tests'
cargo test --lib 'eval::run_control::commands::tests'
cargo test --lib 'eval::run_control::auto_step::tests'
cargo test --lib 'eval::run_control::auto_run::tests'
cargo test --lib 'eval::run_control::session_trace::tests'
cargo test --lib 'eval::run_control::session::tests'
cargo test --bin branch_tiny
```

Expected: all focused suites pass.

- [ ] **Step 3: Run the full library suite**

Run:

```powershell
cargo test --lib
```

Expected: zero failures. If unrelated failures appear, report exact names and
do not claim the suite passes.

- [ ] **Step 4: Confirm the narrowed boundaries**

Run:

```powershell
rg -n "FullPlanner|CombatSearchEvidenceV1|save_combat_search_evidence_v1|RunControlSearchEvidenceTarget" src
rg -n "CombatSearchEvidence|search_evidence_path" src/eval/run_control/session_trace.rs
git status -sb
```

Expected: the first search returns no production symbols; the second returns
only historical trace compatibility fields/variant/test; the worktree is
clean after commits.

- [ ] **Step 5: Final review**

Review each design success criterion against the diff and test evidence. Do
not add combat portfolio or session relocation work to this implementation.
