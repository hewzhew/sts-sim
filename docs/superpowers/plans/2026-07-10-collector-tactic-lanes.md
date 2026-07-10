# Collector Tactic Lanes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in, same-total-budget Collector experiment that compares single-head control with a direct boss race without changing default search or runner behavior.

**Architecture:** Two typed action-prior plugins provide dynamic Collector target hints and a Collector-specific frontier value. Exact terminal outcomes and rollout safety remain ahead of the tactical value; the existing default comparator path remains unchanged. `combat_case_review` owns the two-lane orchestration and splits one configured budget evenly.

**Tech Stack:** Rust 2021, Clap, Serde, built-in test harness, Cargo.

## Global Constraints

- Do not enable either tactic in the main runner or default combat search.
- Do not change card rewards, campfire decisions, map routing, or owner policy.
- Do not add a general production Collector policy, scripted prefixes, or a root-only prior.
- Do not assert a b0094 win, fixed turn, or fixed action sequence in tests.
- Do not use subagents for this implementation.

---

### Task 1: Add typed Collector tactical priors

**Files:**
- Create: `src/ai/combat_search_v2/collector_tactic.rs`
- Modify: `src/ai/combat_search_v2/mod.rs`
- Modify: `src/ai/combat_search_v2/plugins.rs`
- Modify: `src/ai/combat_search_v2/types/config/policies.rs`

**Interfaces:**
- Produces: `CombatSearchActionPriorPluginId::{CollectorSingleHeadControl, CollectorBossRace}` and matching `CombatSearchV2SetupBiasPolicy` variants.
- Produces: `collector_tactic_value(&CombatState, CombatSearchActionPriorPluginId) -> CollectorTacticValueV0`.
- Produces: `collector_tactic_target_rank(&CombatState, Option<usize>, CombatSearchActionPriorPluginId) -> i32`.

- [ ] **Step 1: Write failing plugin round-trip tests**

Extend `plugins.rs` tests to assert both new plugin IDs round-trip through `CombatSearchV2SetupBiasPolicy` and expose labels `collector_single_head_control` and `collector_boss_race`.

- [ ] **Step 2: Run the focused test and observe the missing variants**

Run: `cargo test --lib combat_search_v2::plugins::tests::collector_tactic_plugins_round_trip_through_config_policy`

Expected: compilation fails because the Collector variants do not exist.

- [ ] **Step 3: Add the typed variants and tactical fact module**

Add the two variants to both enums and both conversion matchers. In `collector_tactic.rs`, identify living monsters with `is_alive_for_action()` and `EnemyId::from_id`. Implement these stable semantics:

```rust
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(super) struct CollectorTacticValueV0 {
    applicable: i32,
    formation: i32,
    primary_progress: i32,
    secondary_progress: i32,
}
```

- Boss race: lower living Collector HP is higher priority.
- Single-head control: one living head outranks two, which outranks zero; with two heads, lower minimum head HP is progress; with one head, lower Collector HP is progress.
- Target rank: boss race favors The Collector; control favors the lowest-HP head while two live, then favors The Collector and penalizes the remaining head while one lives.
- Default and key-card plugins return zero-valued facts.

- [ ] **Step 4: Run the focused tests**

Run: `cargo test --lib combat_search_v2::plugins::tests::collector_tactic_plugins_round_trip_through_config_policy`

Expected: PASS.

- [ ] **Step 5: Commit the typed policy surface**

```text
git add src/ai/combat_search_v2/collector_tactic.rs src/ai/combat_search_v2/mod.rs src/ai/combat_search_v2/plugins.rs src/ai/combat_search_v2/types/config/policies.rs
git commit -m "feat: add collector tactical priors"
```

### Task 2: Apply tactics to action and frontier ordering

**Files:**
- Modify: `src/ai/combat_search_v2/action_priority/priority.rs`
- Modify: `src/ai/combat_search_v2/action_priority/play_card/mod.rs`
- Modify: `src/ai/combat_search_v2/action_priority/tests.rs`
- Modify: `src/ai/combat_search_v2/frontier/priority.rs`
- Modify: `src/ai/combat_search_v2/frontier/queue.rs`
- Modify: `src/ai/combat_search_v2/frontier/tests.rs`
- Modify: `src/ai/combat_search_v2/search/loop_state/mod.rs`

**Interfaces:**
- Consumes: `collector_tactic_value` and `collector_tactic_target_rank` from Task 1.
- Produces: `FrontierQueue::new_with_action_prior(frontier, action_prior)` while preserving `FrontierQueue::new(frontier)` as the default test/legacy constructor.

- [ ] **Step 1: Write failing action-order tests**

Create equal-card comparisons using Strike against a living Collector and Torch Heads:

- boss race ranks the Collector target above a Torch Head;
- two-head control ranks the lower-HP head above the other head;
- one-head control ranks the Collector above the surviving head.

Assert the structured `collector_tactic` priority field rather than a full action sequence.

- [ ] **Step 2: Write failing frontier-order tests**

Create nodes with identical rollout estimates and player state, then assert:

- control ranks one living head above zero living heads;
- control ranks concentrated head damage above equally large spread damage;
- boss race ranks lower Collector HP above equal damage dealt only to a head;
- `priority_for_node` with the default plugin keeps the existing fewer-enemy preference.

- [ ] **Step 3: Run the focused tests and observe failures**

Run:

```text
cargo test --lib combat_search_v2::action_priority::tests::collector_
cargo test --lib combat_search_v2::frontier::tests::collector_
```

Expected: compilation fails because the tactical priority field and plugin-aware frontier constructor are missing.

- [ ] **Step 4: Add local target ordering**

Add `collector_tactic: i32` to `ActionOrderingPriority`, initialize it to zero, compare it after reactive safety and before generic setup/progress tie-breaks, and populate it in `priority_for_play_card` from `collector_tactic_target_rank`.

- [ ] **Step 5: Add plugin-aware frontier ordering**

Keep `priority_for_node(node)` as a default wrapper. Add a plugin-aware implementation that, only for a Collector tactic plugin, compares:

```text
exact terminal rank
rollout evaluated/outcome/survival/risk gate
Collector tactic value
existing rollout value
existing action prior/state/potion/turn/length values
```

Store the action-prior plugin in `FrontierQueue`; have the search loop construct the queue from `plugins.frontier` and `plugins.action_prior`. For `Default` and `KeyCardOnline`, bypass the new gate/value so the existing ordering is unchanged.

- [ ] **Step 6: Run focused ordering tests**

Run:

```text
cargo test --lib combat_search_v2::action_priority::tests::collector_
cargo test --lib combat_search_v2::frontier::tests::collector_
cargo test --lib combat_search_v2::frontier::tests::frontier_
```

Expected: all matching tests pass.

- [ ] **Step 7: Commit the search behavior**

```text
git add src/ai/combat_search_v2/action_priority src/ai/combat_search_v2/frontier src/ai/combat_search_v2/search/loop_state/mod.rs
git commit -m "feat: apply collector priors to search ordering"
```

### Task 3: Add the opt-in two-lane review experiment

**Files:**
- Create: `src/bin/combat_case_review/collector_tactic_lanes.rs`
- Modify: `src/bin/combat_case_review.rs`
- Modify: `src/bin/combat_case_review/args.rs`
- Modify: `src/bin/combat_case_review/options.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`
- Modify: `src/bin/combat_case_review/case_payload.rs`
- Modify: `src/bin/combat_case_review/case_payload/types.rs`

**Interfaces:**
- Produces: CLI flag `--collector-tactic-lanes`.
- Produces: optional `collector_tactic_lanes` review payload with total/per-lane budgets, skipped reason, and two lane results.
- Consumes: `quality_lane_total_nodes` / `quality_lane_total_ms`, falling back to slow budget fields.

- [ ] **Step 1: Write failing lane-spec and budget tests**

In the new module, define a private two-element spec array and a pure budget splitter. Assert:

```rust
assert_eq!(specs.len(), 2);
assert_eq!(specs[0].prior, CombatSearchActionPriorPluginId::CollectorSingleHeadControl);
assert_eq!(specs[1].prior, CombatSearchActionPriorPluginId::CollectorBossRace);
assert_eq!(split_budget(1_600_000, 20_000, 2), (800_000, 10_000));
```

- [ ] **Step 2: Run the focused binary test and observe failure**

Run: `cargo test --bin combat_case_review collector_tactic_lanes::tests::`

Expected: compilation fails until the module and review wiring exist.

- [ ] **Step 3: Implement the review-only lane bundle**

Add `ReviewOptions::collector_tactic_lanes`. Return `None` unless the flag is set. If the saved combat has no living `TheCollector`, return a typed payload with `skipped_reason: "not_collector_fight"` and no lanes. Otherwise:

- resolve one total budget and divide it evenly across exactly two lanes;
- build both profiles from `review_all_potions_profile`;
- change only `with_action_prior_plugin(spec.prior)`;
- run both with `run_profile_search`;
- record `SearchReview` plus `review_focus` for each lane;
- serialize contract `review_only_same_total_budget_split_across_two_collector_tactics_no_runner_policy_change`.

- [ ] **Step 4: Run the focused binary tests**

Run: `cargo test --bin combat_case_review collector_tactic_lanes::tests::`

Expected: both spec and budget tests pass.

- [ ] **Step 5: Run proportional verification**

Run:

```text
cargo fmt --check
cargo test --lib combat_search_v2::plugins::tests::
cargo test --lib combat_search_v2::action_priority::tests::
cargo test --lib combat_search_v2::frontier::tests::
cargo test --bin combat_case_review collector_tactic_lanes::tests::
cargo test --lib
cargo test --bin combat_case_review
git diff --check
```

Expected: formatting succeeds, all matching and complete library/binary tests pass, and no whitespace errors are reported.

- [ ] **Step 6: Commit the review experiment**

```text
git add src/bin/combat_case_review.rs src/bin/combat_case_review/collector_tactic_lanes.rs src/bin/combat_case_review/args.rs src/bin/combat_case_review/options.rs src/bin/combat_case_review/review_pipeline.rs src/bin/combat_case_review/case_payload.rs src/bin/combat_case_review/case_payload/types.rs docs/superpowers/plans/2026-07-10-collector-tactic-lanes.md
git commit -m "feat: add collector tactic review lanes"
```

### Task 4: Run the saved b0094 experiment and integrate locally

**Files:**
- Generate (ignored): `target/bounded-multibranch-20260710002-campfire-owner/collector-tactic-b0094.json`

**Interfaces:**
- Consumes: saved case `target/bounded-multibranch-20260710002-campfire-owner/combat_cases/seed20260710002_g24_b0094_a2f32_thecollector.json`.
- Produces: a two-lane diagnostic artifact, not a regression fixture.

- [ ] **Step 1: Run equal per-lane budgets matching the previous 800k-node/10s review scale**

Run:

```text
cargo run --release --bin combat_case_review -- --case target/bounded-multibranch-20260710002-campfire-owner/combat_cases/seed20260710002_g24_b0094_a2f32_thecollector.json --collector-tactic-lanes --quality-lane-total-nodes 1600000 --quality-lane-total-ms 20000 --write-review target/bounded-multibranch-20260710002-campfire-owner/collector-tactic-b0094.json
```

Expected: one JSON review with two lanes, each reporting 800,000 max nodes and 10,000 wall ms.

- [ ] **Step 2: Summarize without promoting the experiment to policy**

Compare complete win status, best death/win turn, remaining enemy identities/HP, player HP, potion use, and opening action previews. Classify the result as:

- control-only success: evidence for missing Collector formation modeling;
- race-only success: evidence for excessive minion-count preference;
- both success: generic prior mismatch, with no production tactic selected yet;
- neither success: search tactic alone is insufficient and deck/energy/access remains the stronger blocker.

- [ ] **Step 3: Verify the final branch and merge locally**

Run the complete test suite from the feature branch, merge it into `master`, rerun the complete suite on merged `master`, remove the owned worktree, and delete the merged feature branch.
