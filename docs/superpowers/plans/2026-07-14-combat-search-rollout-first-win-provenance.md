# Rollout First-Win Provenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Report the main-search generated-node count at which an eventually exact-replayed rollout win witness was observed.

**Architecture:** Pair the rollout cache's selected terminal-win witness with an immutable discovery-node snapshot. Every rollout evaluation supplies its current main-search node count, while post-loop promotion publishes that snapshot only after the existing exact replay succeeds.

**Tech Stack:** Rust, Cargo unit tests, existing `combat_search_v2` search and rollout-cache modules.

## Global Constraints

- A rollout estimate remains estimate-only until exact replay verifies the selected witness.
- Root rollout discovery uses node count `0`; child and deferred rollout discovery use the current generated-node count; a turn-plan seed uses the count including that generated seed.
- Do not change rollout scheduling, cache keys, witness ranking, frontier order, early stopping, action selection, or combat outcomes.
- Work in `D:\rust\sts_simulator`; do not create a worktree and do not run `cargo clean`.

---

### Task 1: Carry rollout discovery provenance through exact promotion

**Files:**
- Modify: `src/ai/combat_search_v2/search/tests.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/mod.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/estimate.rs`
- Modify: `src/ai/combat_search_v2/search/rollout_timing.rs`
- Modify: `src/ai/combat_search_v2/search/bootstrap.rs`
- Modify: `src/ai/combat_search_v2/search/child_rollout.rs`
- Modify: `src/ai/combat_search_v2/search/node_deferred_rollout.rs`
- Modify: `src/ai/combat_search_v2/search/turn_plan_seeding.rs`
- Modify: `src/ai/combat_search_v2/search/loop_state/trajectories.rs`
- Modify: `src/ai/combat_search_v2/search/rollout_terminal_promotion.rs`

**Interfaces:**
- Consumes: `CombatSearchV2Stats::nodes_generated`, `RolloutNodeEstimate`, and the existing exact replay bridge.
- Produces: `ReplayableTerminalWinWitness { estimate, nodes_generated_at_discovery }` and `SearchLoopState::remember_win_observed_at(...)`.

- [ ] **Step 1: Write the failing search regression**

Add this test beside the existing rollout-promotion tests in `search/tests.rs`:

```rust
#[test]
fn exact_replayed_rollout_reports_when_its_witness_was_discovered() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 1,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            ..CombatSearchV2Config::default()
        },
        &TwoTurnWinStepper,
    );

    assert_eq!(report.stats.nodes_expanded, 1);
    assert_eq!(report.stats.nodes_generated, 1);
    assert!(report.stats.node_budget_hit);
    assert!(report.outcome.complete_win_found);
    assert_eq!(
        report.stats.nodes_to_first_win,
        Some(0),
        "the exact-replayed root rollout witness was discovered before the main search generated a node"
    );
}
```

- [ ] **Step 2: Run the regression and verify RED**

Run:

```powershell
cargo test --lib ai::combat_search_v2::search::tests::exact_replayed_rollout_reports_when_its_witness_was_discovered -- --nocapture
```

Expected: FAIL because the old post-loop call reports `Some(1)` instead of `Some(0)`.

- [ ] **Step 3: Add typed witness provenance to the rollout cache**

In `rollout_cache/mod.rs`, add the internal carrier and update the cache field:

```rust
#[derive(Clone, Debug)]
pub(super) struct ReplayableTerminalWinWitness {
    pub(super) estimate: RolloutNodeEstimate,
    pub(super) nodes_generated_at_discovery: u64,
}

pub(super) best_replayable_terminal_win: Option<ReplayableTerminalWinWitness>,
```

In `rollout_cache/estimate.rs`, add `nodes_generated_at_discovery: u64` to `estimate`, pass it into `observe_estimate`, and replace the selected witness together with its snapshot whenever the existing `better_rollout_estimate` rule selects the new estimate:

```rust
fn observe_estimate(
    &mut self,
    estimate: &RolloutNodeEstimate,
    nodes_generated_at_discovery: u64,
) {
    if estimate.is_replayable_terminal_win() {
        let replace = self
            .best_replayable_terminal_win
            .as_ref()
            .map(|current| {
                better_rollout_estimate(estimate.clone(), current.estimate.clone()) == *estimate
            })
            .unwrap_or(true);
        if replace {
            self.best_replayable_terminal_win = Some(ReplayableTerminalWinWitness {
                estimate: estimate.clone(),
                nodes_generated_at_discovery,
            });
        }
    }
}
```

Insert this replacement block before the existing `if estimate.truncated` counter block; leave that counter block and the rest of `observe_estimate` byte-for-byte unchanged.

- [ ] **Step 4: Supply the discovery snapshot at every rollout source**

Add `nodes_generated_at_discovery: u64` to `timed_rollout_estimate` and forward it to `RolloutCache::estimate`:

```rust
let estimate = rollout_cache.estimate(
    node,
    stepper,
    config,
    deadline,
    nodes_generated_at_discovery,
);
```

Pass exact source counts at the four call sites:

```rust
// bootstrap.rs
RolloutEstimateSource::Root,
0,

// child_rollout.rs
RolloutEstimateSource::Child,
loop_state.stats.nodes_generated,

// node_deferred_rollout.rs
RolloutEstimateSource::DeferredChild,
loop_state.stats.nodes_generated,

// turn_plan_seeding.rs, before the existing record_node_generated call
let nodes_generated_at_discovery = loop_state.stats.nodes_generated.saturating_add(1);
```

Keep the current rollout execution order and all policy inputs unchanged.

- [ ] **Step 5: Publish provenance only after exact replay**

In `search/loop_state/trajectories.rs`, preserve the ordinary API while adding an explicit observation-count variant:

```rust
pub(in crate::ai::combat_search_v2::search) fn remember_win(
    &mut self,
    node: SearchNode,
    config: &CombatSearchV2Config,
) -> bool {
    self.remember_win_observed_at(node, config, self.stats.nodes_generated)
}

pub(in crate::ai::combat_search_v2::search) fn remember_win_observed_at(
    &mut self,
    node: SearchNode,
    config: &CombatSearchV2Config,
    nodes_generated_at_discovery: u64,
) -> bool {
    self.stats.terminal_wins = self.stats.terminal_wins.saturating_add(1);
    if self.stats.nodes_to_first_win.is_none() {
        self.stats.nodes_to_first_win = Some(nodes_generated_at_discovery);
    }
    self.trajectories.remember_win(node, config)
}
```

In `search/rollout_terminal_promotion.rs`, replay `witness.estimate` and, only after success, call:

```rust
let accepted = loop_state.remember_win_observed_at(
    node,
    config,
    witness.nodes_generated_at_discovery,
);
if accepted {
    loop_state.mark_accepted_complete_candidate();
}
```

- [ ] **Step 6: Run GREEN and focused regressions**

Run:

```powershell
cargo test --lib ai::combat_search_v2::search::tests::exact_replayed_rollout_reports_when_its_witness_was_discovered -- --nocapture
cargo test --lib ai::combat_search_v2::search::tests::terminal_rollout_is_promoted_only_after_exact_replay -- --nocapture
cargo test --lib ai::combat_search_v2::search::tests::exact_replayed_terminal_rollout_honors_hp_loss_acceptance_threshold -- --nocapture
cargo test --lib ai::combat_search_v2::search::tests -- --nocapture
```

Expected: all commands PASS; the focused module currently contains 24 tests before this addition and should contain 25 afterward.

- [ ] **Step 7: Run completion gates**

Run:

```powershell
cargo test --lib -q
cargo test --test architecture_runtime_boundaries -q
cargo test --bin combat_search_v2_driver -q
cargo fmt --all -- --check
git diff --check
```

Expected: 0 failures, formatting clean, and no whitespace errors.

- [ ] **Step 8: Review and commit the implementation**

Confirm the diff contains only the listed search/rollout-cache files and the regression, then run:

```powershell
git add -- src/ai/combat_search_v2/search/tests.rs src/ai/combat_search_v2/rollout_cache/mod.rs src/ai/combat_search_v2/rollout_cache/estimate.rs src/ai/combat_search_v2/search/rollout_timing.rs src/ai/combat_search_v2/search/bootstrap.rs src/ai/combat_search_v2/search/child_rollout.rs src/ai/combat_search_v2/search/node_deferred_rollout.rs src/ai/combat_search_v2/search/turn_plan_seeding.rs src/ai/combat_search_v2/search/loop_state/trajectories.rs src/ai/combat_search_v2/search/rollout_terminal_promotion.rs
git commit -m "fix: preserve rollout win discovery count"
```

Expected: one implementation commit and a clean worktree.
