# Combat Case Review Orphan Lenses Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove six unconsumed saved-combat experiment families, the unused optional ladder row,
and the Collector-only search policy while preserving the active combat-review and panel contracts.

**Architecture:** First remove the CLI, pipeline, and JSON ownership chain so the supported binary
has a smaller explicit surface. Then remove the Collector policy that becomes unreachable, simplify
frontier/action ordering back to their generic paths, and update current-state documentation. Each
layer is independently compiled, tested, and committed.

**Tech Stack:** Rust 2021, Cargo, Clap, Serde, Python `unittest`, PowerShell, Git.

## Global Constraints

- Work in the stable `D:\rust\sts_simulator` checkout; do not create a worktree.
- Execute inline in the current session; do not dispatch subagents.
- Preserve `quality_lanes`, `frozen_panel_lanes`, `line_lab`, `counterfactual_hp`, and all evidence
  derived from them.
- Preserve the generic decision microscope, `KeyCardOnline`, `TurnBoundaryFrontierSeed`, tactical
  turn-boundary policies, and `ReviewSearchIntervention`.
- Do not change combat-search defaults, runner policy, run-control, rewards, routing, shops, events,
  campfires, or game mechanics.
- Do not add deprecated flag aliases, null compatibility fields, replacement probes, or tests that
  freeze the removed behavior.
- Do not edit historical files under `docs/superpowers/specs` or `docs/superpowers/plans` except for
  this plan and its approved design specification.
- Do not touch ignored run artifacts, `target`, `.venv-ai`, remote refs, or public `master`.
- Begin each task with a clean worktree and end it with a focused green commit.

---

## File Responsibility Map

### Saved-case adapter layer

- `src/bin/combat_case_review.rs`: declares the binary's modules.
- `src/bin/combat_case_review/args.rs`: owns the public Clap flags.
- `src/bin/combat_case_review/options.rs`: maps public flags into internal review configuration.
- `src/bin/combat_case_review/review_pipeline.rs`: orchestrates retained review operations.
- `src/bin/combat_case_review/review_pipeline/ladder.rs`: owns the two-row generic ladder.
- `src/bin/combat_case_review/case_payload.rs`: assembles the serialized root review.
- `src/bin/combat_case_review/case_payload/types.rs`: defines root and intermediate payload fields.
- `src/bin/combat_case_review/search_runner.rs`: owns shared retained search profiles.
- `src/bin/combat_case_review/key_card_lifecycle.rs`: keeps lifecycle reporting but stops
  re-exporting probe-only helpers.

### Collector policy layer

- `src/ai/combat_search_v2/collector_tactic.rs`: Collector-only value and target ranking; delete.
- `src/ai/combat_search_v2/plugins.rs` and
  `src/ai/combat_search_v2/types/config/policies.rs`: action-prior IDs and config conversion.
- `src/ai/combat_search_v2/action_priority/priority.rs` and
  `src/ai/combat_search_v2/action_priority/play_card/mod.rs`: root-action ordering.
- `src/ai/combat_search_v2/frontier/priority.rs`: generic frontier comparison after tactic removal.
- `src/ai/combat_search_v2/frontier/queue.rs` and
  `src/ai/combat_search_v2/search/loop_state/mod.rs`: queue construction without a tactic-only
  action-prior parameter.
- `src/ai/combat_search_v2/action_priority/tests.rs`,
  `src/ai/combat_search_v2/frontier/tests.rs`, and plugin tests: delete only Collector assertions.

### Current-state documentation

- `docs/architecture/supported-surfaces.md`: authoritative counts, supported nested contracts, and
  retirement history.
- `docs/architecture/combat_experiment_panel.review-draft.md`: historical status notice only.

---

### Task 1: Retire the saved-case experiment adapters

**Files:**

- Delete: `src/bin/combat_case_review/boss_setup_lane.rs`
- Delete: `src/bin/combat_case_review/collector_tactic_lanes.rs`
- Delete: `src/bin/combat_case_review/forced_potion_opening.rs`
- Delete: `src/bin/combat_case_review/key_card_counterfactual.rs`
- Delete: `src/bin/combat_case_review/key_card_counterfactual/execution.rs`
- Delete: `src/bin/combat_case_review/key_card_counterfactual/movement.rs`
- Delete: `src/bin/combat_case_review/key_card_counterfactual/types.rs`
- Delete: `src/bin/combat_case_review/key_card_decision_microscope.rs`
- Delete: `src/bin/combat_case_review/key_card_decision_microscope/digest.rs`
- Delete: `src/bin/combat_case_review/key_card_decision_microscope/execution.rs`
- Delete: `src/bin/combat_case_review/key_card_decision_microscope/types.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/basis.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/config.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/execution.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/selection.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/selection_tests.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/transition.rs`
- Delete: `src/bin/combat_case_review/root_action_role_duel/types.rs`
- Modify: `src/bin/combat_case_review.rs`
- Modify: `src/bin/combat_case_review/args.rs`
- Modify: `src/bin/combat_case_review/options.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`
- Modify: `src/bin/combat_case_review/review_pipeline/ladder.rs`
- Modify: `src/bin/combat_case_review/case_payload.rs`
- Modify: `src/bin/combat_case_review/case_payload/types.rs`
- Modify: `src/bin/combat_case_review/search_runner.rs`
- Modify: `src/bin/combat_case_review/key_card_lifecycle.rs`
- Modify: `src/bin/combat_case_review/key_card_lifecycle/tests.rs`
- Test: `src/bin/combat_case_review/` retained Rust tests
- Test: `tests/test_frozen_case_panel.py`
- Smoke check: `tools/success_feedback_panel.py`

**Interfaces:**

- Consumes: approved design
  `docs/superpowers/specs/2026-07-12-combat-case-review-orphan-lenses-retirement-design.md`.
- Produces: `combat_case_review` with the existing root schema name and retained ladder, quality,
  frozen, line-lab, HP, boss, lifecycle, and strategic-feedback fields.
- Removes: seven public flags and six optional nested JSON fields named in the design.

- [ ] **Step 1: Prove the retained consumers are green before deletion**

Run:

```powershell
git status --short
cargo test --bin combat_case_review
python tests/test_frozen_case_panel.py
python -m py_compile tools/success_feedback_panel.py
python tools/success_feedback_panel.py --help
```

Expected: the status command prints nothing; Rust reports 25 passing tests; the frozen-panel tests
pass; the success-feedback tool compiles and prints its help. Stop on the first failure instead of
beginning deletion from an untrusted baseline.

- [ ] **Step 2: Delete the 19 experiment-family files**

Use `apply_patch` deletion blocks for exactly the files listed in this task. The deleted roots and
subdirectories are:

```text
boss_setup_lane.rs
collector_tactic_lanes.rs
forced_potion_opening.rs
key_card_counterfactual.rs
key_card_counterfactual/
key_card_decision_microscope.rs
key_card_decision_microscope/
root_action_role_duel.rs
root_action_role_duel/
```

Expected: `git status --short` shows 19 deleted Rust files and no other deletion.

- [ ] **Step 3: Remove the modules, flags, and option fields**

In `src/bin/combat_case_review.rs`, delete only the six `#[path = ...]` declarations and matching
`mod` items. Keep the resulting diagnostic declarations, including:

```rust
#[path = "combat_case_review/counterfactual_hp.rs"]
mod counterfactual_hp;
#[path = "combat_case_review/frozen_panel_lanes.rs"]
mod frozen_panel_lanes;
#[path = "combat_case_review/key_card_lifecycle.rs"]
mod key_card_lifecycle;
#[path = "combat_case_review/line_lab.rs"]
mod line_lab;
#[path = "combat_case_review/quality_lanes.rs"]
mod quality_lanes;
#[path = "combat_case_review/search_intervention.rs"]
mod search_intervention;
```

Delete these fields from both `Args` and `ReviewOptions`, and delete their `from_args` assignments:

```text
turn_plan_ladder
forced_potion_opening_lanes
boss_setup_lane
key_card_counterfactual
key_card_decision_microscope
root_action_role_duel
collector_tactic_lanes
```

Also delete `boss_setup_lane` from the same lists even though the six-family count already includes
it. The retained option tail must still contain:

```rust
pub(super) line_lab: bool,
pub(super) line_lab_ms: u64,
pub(super) line_lab_cuts: usize,
pub(super) quality_lanes: bool,
pub(super) frozen_panel_lanes: bool,
pub(super) quality_lane_total_nodes: Option<usize>,
pub(super) quality_lane_total_ms: Option<u64>,
pub(super) counterfactual_hp_probe: bool,
pub(super) counterfactual_hp_levels: String,
```

Expected: the retired field names have no match in `args.rs` or `options.rs`; all retained fields
still map one-to-one from `Args` to `ReviewOptions`.

- [ ] **Step 4: Narrow ladder and pipeline orchestration**

In `review_pipeline/ladder.rs`, remove `CombatSearchTurnPlanPluginId` from the import and delete the
entire `if options.turn_plan_ladder { ... }` block. Keep the two-row result:

```rust
let reviews = vec![fast_review, slow_review];

ReviewLadderRun {
    reviews,
    line_lab_parent: slow_report.best_complete_trajectory,
}
```

In `review_pipeline.rs`, remove imports and calls for all six deleted runners. The retained middle
of `build_review` must have this ownership order:

```rust
let line_lab = run_line_lab(&options, &case, ladder_run.line_lab_parent.as_ref());
let combat_deficit_evidence = line_lab.as_ref().map(derive_combat_deficit_evidence);
let boss_pressure_lens = boss_pressure_lens(&case, &ladder, line_lab.as_ref());
let frozen_panel_lanes = run_frozen_panel_lanes(&options, &case);
let quality_lanes = if options.quality_lanes {
    Some(run_quality_lanes(&options, &case))
} else {
    None
};
let counterfactual_hp_probe = if options.counterfactual_hp_probe {
    Some(run_counterfactual_hp_probe(&options, &case))
} else {
    None
};
```

Remove the corresponding six fields from the `CombatCaseReviewArtifacts` construction. Do not
alter any later boss, Awakened One, Champ, lifecycle, or strategic-feedback calculation.

- [ ] **Step 5: Remove the retired JSON fields and adapter-only helpers**

In `case_payload/types.rs`, remove the six deleted type imports and these fields from both
`CombatCaseReview` and `CombatCaseReviewArtifacts`:

```text
boss_setup_lane
forced_potion_opening_lanes
key_card_counterfactual
key_card_decision_microscope
root_action_role_duel
collector_tactic_lanes
```

In `case_payload.rs`, remove the same fields from artifact destructuring and root construction.
Keep `frozen_panel_lanes` adjacent to the retained evidence fields.

In `search_runner.rs`, remove `CombatSearchActionPriorPluginId` from the import and delete the
now-unreferenced function:

```rust
pub(crate) fn review_key_setup_profile(
    label: &'static str,
    nodes: usize,
    wall_ms: u64,
    options: &ReviewOptions,
) -> CombatSearchProfile {
    review_all_potions_profile(label, nodes, wall_ms, options)
        .with_action_prior_plugin(CombatSearchActionPriorPluginId::KeyCardOnline)
}
```

In `key_card_lifecycle.rs`, keep the internal `targets` module for lifecycle tracking but narrow the
root re-export to:

```rust
pub(super) use types::KeyCardLifecycleReport;
```

`targets::key_card_targets`, `KeyCardReason`, and `KeyCardTarget` remain internally owned by the
retained lifecycle implementation. Update `key_card_lifecycle/tests.rs` to import the retained enum
directly with `use super::types::KeyCardReason;`.

- [ ] **Step 6: Format and prove the public surface changed exactly once**

Run:

```powershell
cargo fmt --all
$help = (cargo run --quiet --bin combat_case_review -- --help | Out-String)
$retired = @(
  '--turn-plan-ladder', '--boss-setup-lane', '--forced-potion-opening-lanes',
  '--key-card-counterfactual', '--key-card-decision-microscope',
  '--root-action-role-duel', '--collector-tactic-lanes'
)
$retired | ForEach-Object { if ($help -match [regex]::Escape($_)) { throw "retired flag remains: $_" } }
$kept = @('--ladder', '--quality-lanes', '--frozen-panel-lanes', '--line-lab', '--counterfactual-hp-probe')
$kept | ForEach-Object { if ($help -notmatch [regex]::Escape($_)) { throw "retained flag missing: $_" } }
rg -n "boss_setup_lane|forced_potion_opening|key_card_counterfactual|key_card_decision_microscope|root_action_role_duel|collector_tactic_lanes|turn_plan_ladder" src/bin tools tests
if ($LASTEXITCODE -eq 0) { throw 'retired active-source symbol remains' }
if ($LASTEXITCODE -gt 1) { throw "rg failed with $LASTEXITCODE" }
```

Expected: help contains all five retained flags and none of the seven retired flags; `rg` returns
one and is converted into success by the explicit checks.

- [ ] **Step 7: Run focused verification**

Run:

```powershell
cargo fmt --all -- --check
cargo test --bin combat_case_review
python tests/test_frozen_case_panel.py
python -m py_compile tools/success_feedback_panel.py
python tools/success_feedback_panel.py --help
cargo check --bin combat_case_review
git diff --check
```

Expected: all commands pass. The Rust binary test count is lower only by tests deleted with the
retired modules; both active Python consumer suites remain green.

- [ ] **Step 8: Commit the adapter retirement**

Run:

```powershell
git add src/bin/combat_case_review.rs src/bin/combat_case_review
git commit -m "chore: retire orphan combat review lenses"
git status --short
```

Expected: the commit succeeds and the final status command prints nothing.

---

### Task 2: Remove the orphaned Collector policy and record the retirement

**Files:**

- Delete: `src/ai/combat_search_v2/collector_tactic.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Modify: `src/ai/combat_search_v2/plugins.rs`
- Modify: `src/ai/combat_search_v2/types/config/policies.rs`
- Modify: `src/ai/combat_search_v2/action_priority/play_card/mod.rs`
- Modify: `src/ai/combat_search_v2/action_priority/priority.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`
- Modify: `src/ai/combat_search_v2/frontier/priority.rs`
- Modify: `src/ai/combat_search_v2/frontier/queue.rs`
- Modify: `src/ai/combat_search_v2/frontier/tests.rs`
- Modify: `src/ai/combat_search_v2/search/loop_state/mod.rs`
- Modify: `docs/architecture/supported-surfaces.md`
- Modify: `docs/architecture/combat_experiment_panel.review-draft.md`
- Test: retained `ai::combat_search_v2` library tests
- Test: all supported binaries and architecture boundaries

**Interfaces:**

- Consumes: Task 1's absence of `collector_tactic_lanes`.
- Produces: action-prior/config enums containing only `Default` and `KeyCardOnline`; generic
  frontier queue construction independent of action-prior identity.
- Preserves: node `action_prior_score`, generic action ordering, key-card setup bias, rollout value,
  frontier lanes, and all non-Collector tactic behavior.

- [ ] **Step 1: Run retained-policy baseline tests**

Run:

```powershell
git status --short
cargo test --lib combat_search_v2::plugins::tests::plugin_ids_implement_their_role_traits
cargo test --lib combat_search_v2::action_priority::tests::key_card_setup_bias_promotes_strength_scaling_power
cargo test --lib combat_search_v2::frontier::tests::frontier_priority_continues_retaliation_protection_before_raw_enemy_progress
```

Expected: clean status and three passing retained-policy tests.

- [ ] **Step 2: Delete the Collector module and enum variants**

Delete `collector_tactic.rs` and remove `mod collector_tactic;` from `mod.rs`.

In `types/config/policies.rs`, leave:

```rust
pub enum CombatSearchV2SetupBiasPolicy {
    Default,
    KeyCardOnline,
}

impl CombatSearchV2SetupBiasPolicy {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::KeyCardOnline => "key_card_online",
        }
    }
}
```

In `plugins.rs`, leave the same two action-prior IDs and remove `is_collector_tactic`:

```rust
pub enum CombatSearchActionPriorPluginId {
    Default,
    KeyCardOnline,
}

impl CombatSearchActionPriorPluginId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::KeyCardOnline => "key_card_online",
        }
    }

    pub(in crate::ai::combat_search_v2) fn prioritizes_key_card_online(self) -> bool {
        matches!(self, Self::KeyCardOnline)
    }
}
```

Reduce both `From` implementations to the `Default` and `KeyCardOnline` arms, and delete
`collector_tactic_plugins_round_trip_through_config_policy`. Keep
`plugin_ids_implement_their_role_traits` unchanged.

- [ ] **Step 3: Remove Collector root-action ordering**

In `action_priority/play_card/mod.rs`, remove the Collector import, the `collector_tactic` and
`preserves_last_collector_head` locals, and the last-head exception from lethal classification.
The first role branch becomes:

```rust
let (role, role_rank) = if target_lethal {
    (ActionOrderingRole::LethalCard, ROLE_LETHAL_CARD)
} else if prevents_visible_lethal {
```

Remove `collector_tactic` from the constructed `ActionOrderingPriority`.

In `action_priority/priority.rs`, remove the field, neutral initialization, and comparator link:

```rust
pub(in crate::ai::combat_search_v2) reactive_risk: i32,
pub(in crate::ai::combat_search_v2) targets_timed_threat: i32,
```

```rust
.then_with(|| self.reactive_risk.cmp(&other.reactive_risk))
.then_with(|| self.targets_timed_threat.cmp(&other.targets_timed_threat))
```

Delete exactly these three tests from `action_priority/tests.rs`:

```text
collector_boss_race_prior_targets_collector_before_torch_head
collector_control_prior_focuses_weaker_head_while_two_live
collector_control_prior_preserves_last_head_and_targets_collector
```

- [ ] **Step 4: Restore a generic frontier queue**

In `frontier/priority.rs`, reduce the value import to retained generic types:

```rust
use super::super::value::{combat_search_state_value, CombatSearchStateValueV1};
```

Remove the Collector imports, `collector_tactic_gate` field/comparator/initializer,
`CollectorTacticFrontierGate`, and `collector_tactic_frontier_gate`. Replace the two priority
functions with one production function:

```rust
pub(in crate::ai::combat_search_v2::frontier) fn priority_for_node(
    node: &SearchNode,
) -> NodePriority {
    let terminal_rank = match terminal_label(&node.engine, &node.combat) {
        SearchTerminalLabel::Win => 3,
        SearchTerminalLabel::Unresolved => 2,
        SearchTerminalLabel::Loss => 1,
    };
    NodePriority {
        terminal_rank,
        rollout_value: rollout_priority_value(&node.rollout_estimate),
        action_prior_rank: action_prior_rank(node.action_prior_score),
        action_ordering_frontier_hint: node.action_ordering_frontier_hint,
        state_value: combat_search_state_value(node),
        potion_tactical_priority: node.potion_tactical_priority,
        potion_conservation: -((node.potions_used + node.potions_discarded) as i32),
        turn_branch_priority: node.last_turn_branch_priority,
        shorter_line: -(node.actions.len() as i32),
    }
}
```

In `frontier/queue.rs`, import `priority_for_node`, remove the `action_prior` field and
`new_with_action_prior`, make `new` available outside tests, and construct entries with:

```rust
pub(in crate::ai::combat_search_v2) fn new(
    policy: impl Into<CombatSearchFrontierPluginId>,
) -> Self {
    Self {
        policy: policy.into(),
        single: BinaryHeap::new(),
        lanes: FrontierLanes::new(),
        next_sequence_id: 0,
    }
}
```

```rust
priority: priority_for_node(&node),
```

In `search/loop_state/mod.rs`, replace the tactic-era constructor with:

```rust
frontier: FrontierQueue::new(plugins.frontier),
```

Delete these seven tests and the `collector_node` helper from `frontier/tests.rs`, then remove
`priority_for_node_with_action_prior` from its import:

```text
collector_control_frontier_prefers_one_living_head_to_zero
collector_control_frontier_prefers_concentrated_head_damage
collector_control_frontier_does_not_skip_the_initial_spawn_window
collector_control_frontier_does_not_stall_in_the_initial_spawn_window
collector_boss_race_frontier_prefers_damage_on_collector
collector_tactic_frontier_queue_uses_configured_prior
collector_tactic_prior_is_neutral_outside_collector_fights
```

- [ ] **Step 5: Prove the Collector policy closure is gone**

Run:

```powershell
cargo fmt --all
rg -n "CollectorSingleHeadControl|CollectorBossRace|collector_tactic|collector_single_head_control|collector_boss_race" src tools tests
if ($LASTEXITCODE -eq 0) { throw 'Collector tactic symbol remains in active code' }
if ($LASTEXITCODE -gt 1) { throw "rg failed with $LASTEXITCODE" }
cargo check --lib
```

Expected: active-code search returns no match and the library compiles. Ordinary `TheCollector`
enemy/content references are deliberately outside this symbol list and remain present.

- [ ] **Step 6: Update current-state documentation and counts**

At the top of `docs/architecture/combat_experiment_panel.review-draft.md`, change the status to:

```markdown
Status: historical review draft. Frozen Panel V0a remains supported; the manual key-card and
root-action probes described as interventions were retired on 2026-07-12.
```

In `docs/architecture/supported-surfaces.md`:

- remove Collector tactic from the `combat_case_review` nested-schema examples;
- replace the statement that recent Collector lane history proves support with evidence from the
  active frozen and success-feedback panel consumers;
- add one retirement-history entry naming the six lens families, optional turn-plan ladder, and
  Collector-only policy closure;
- record exact post-retirement repository counts produced by:

```powershell
$rust = @(rg --files -g '*.rs')
$physical = 0
foreach ($file in $rust) { $physical += (Get-Content -LiteralPath $file).Count }
$tests = (rg -n '#\[test\]' -g '*.rs' | Measure-Object).Count
$cfgTests = (rg -n '#\[cfg\(test\)\]' -g '*.rs' | Measure-Object).Count
$tracked = @(git ls-files | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
$bytes = 0
foreach ($file in $tracked) { $bytes += (Get-Item -LiteralPath $file).Length }
[pscustomobject]@{
  RustFiles = $rust.Count
  PhysicalRustLines = $physical
  TestMarkers = $tests
  CfgTestMarkers = $cfgTests
  TrackedFiles = $tracked.Count
  TrackedBytes = $bytes
}
```

Expected: every Rust/source/test measure is below the recorded post-campaign baseline of 1,796
Rust files, 328,197 physical Rust lines, 2,720 test markers, and 424 cfg-test markers. Copy the
measured integers exactly; do not estimate them in the authority document.

- [ ] **Step 7: Run focused retained-policy verification**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib combat_search_v2::plugins::tests::plugin_ids_implement_their_role_traits
cargo test --lib combat_search_v2::action_priority::tests::key_card_setup_bias_promotes_strength_scaling_power
cargo test --lib combat_search_v2::frontier::tests::
cargo test --bin combat_case_review
python tests/test_frozen_case_panel.py
python -m py_compile tools/success_feedback_panel.py
python tools/success_feedback_panel.py --help
```

Expected: all retained plugin, ordering, frontier, binary, and panel-consumer tests pass.

- [ ] **Step 8: Run full completion verification**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
python -m unittest discover -s tests -p 'test_*.py'
cargo check --bins
cargo metadata --no-deps --format-version 1 | Out-File -Encoding utf8 target/cleanup-cargo-metadata.json
$metadata = Get-Content -Raw target/cleanup-cargo-metadata.json | ConvertFrom-Json
$bins = @($metadata.packages.targets | Where-Object kind -Contains 'bin' | Select-Object -ExpandProperty name | Sort-Object)
$expected = @('branch_panel','branch_tiny','combat_case_review','combat_search_v2_driver','rl_dataset_export','run_play_driver')
if (Compare-Object $expected $bins) { throw "unexpected binary set: $($bins -join ', ')" }
git diff --check
```

Expected: 100% of library and architecture tests pass, all Python tests pass, all six binaries
compile, Cargo metadata lists exactly the six expected binaries, and the diff check is clean.
Writing metadata below ignored `target/` is permitted verification output and must not be staged.

- [ ] **Step 9: Commit the Collector closure and documentation**

Run:

```powershell
git add src/ai/combat_search_v2 docs/architecture/supported-surfaces.md docs/architecture/combat_experiment_panel.review-draft.md
git commit -m "chore: remove orphan collector search policy"
git status --short
```

Expected: the commit succeeds and the final status command prints nothing. Do not push; public
remote publication remains a separate user decision.

---

## Completion Handoff

Report both implementation commit IDs, exact measured count reductions, focused and full test
totals, the six surviving binary names, clean worktree status, and the unchanged backup ref. State
explicitly that active Frozen/Quality panels, line-lab, HP evidence, key-card lifecycle, generic
decision microscope, key-setup bias, and turn-boundary policies remain supported.
