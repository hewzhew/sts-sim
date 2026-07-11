# Rollout Terminal Promotion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Return a complete executable combat trajectory when a bounded rollout found a win that exact replay verifies.

**Architecture:** Retain one deterministic replayable terminal rollout in `RolloutCache`. At search finish, replay it once from the original root and pass the resulting exact `SearchNode` through the existing trajectory book. Keep `finalize.rs` data-only and isolate the bridge in a focused search module.

**Tech Stack:** Rust, generic `CombatStepper`, existing rollout estimates, search nodes, and focused search tests.

## Global Constraints

- Never promote an estimate without exact replay.
- Attempt at most one promotion and at most 96 actions.
- Do not rerun search, change owner policy, or add seed-specific logic.
- Preserve action IDs by resolving each preview against the current legal-action list.

---

### Task 1: Record the complete replayable witness contract

**Files:**
- Modify: `src/ai/combat_search_v2/rollout_estimate/types.rs`
- Modify: `src/ai/combat_search_v2/rollout_estimate/build.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/mod.rs`
- Modify: `src/ai/combat_search_v2/rollout_cache/estimate.rs`

- [ ] Add total action count and a predicate that accepts only complete,
      untruncated terminal wins.
- [ ] Retain the best replayable witness deterministically when estimates are
      observed.
- [ ] Add focused unit coverage for rejecting a preview truncated at the
      96-action boundary.

### Task 2: Exact replay promotion

**Files:**
- Create: `src/ai/combat_search_v2/search/rollout_terminal_promotion.rs`
- Modify: `src/ai/combat_search_v2/search.rs`
- Modify: `src/ai/combat_search_v2/search/tests.rs`

- [ ] Add a failing search test: a one-node budget prevents exact child
      expansion even though the root rollout reaches a one-action win.
- [ ] Verify RED with the focused test filter.
- [ ] Replay one retained witness from the original root, reconstruct action
      traces/counters, validate exact terminal state, and call `remember_win`.
- [ ] Run focused promotion and rejection tests until GREEN.

### Task 3: Verification and bounded-seed acceptance

**Files:**
- No additional source files expected.
- Reuse: `target/bounded-mainline-20260711001/`

- [ ] Run `cargo test --lib`.
- [ ] Run `cargo test --test architecture_runtime_boundaries`.
- [ ] Run `cargo fmt --all -- --check` and `git diff --check`.
- [ ] Rerun the Time Eater frozen case with the same main-search budget and
      confirm the report contains the exact complete trajectory without a
      witness-prior search rerun.
- [ ] Commit, merge locally to `master`, and clean the isolated worktree.
