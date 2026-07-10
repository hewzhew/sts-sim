# Boss Victory HP Boundary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop owner-audit from rejecting surviving Act-boss wins whose HP loss is erased or compressed before the next risk boundary.

**Architecture:** Keep the existing floor reserve as the default. Add one focused predicate beside `owner_audit_hp_loss_limit` that recognizes a real room-boss recovery/end boundary while excluding the first A20 double boss.

**Tech Stack:** Rust, existing `RunControlSession`, `CombatContext`, `RoomType`, and owner-audit unit tests.

## Global Constraints

- Do not change generic run-control combat-search semantics.
- Do not loosen hallway, elite, event-boss, or first-A20-boss HP gates.
- Do not change campfire policy or add a route-pressure heuristic.

---

### Task 1: Boss-aware owner survival limit

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_survival.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`

**Interfaces:**
- Consumes: `RunControlSession.active_combat`, `CombatContext::Room`, `RoomType::MonsterRoomBoss`, and `RunState::should_start_act3_double_boss()`.
- Produces: unchanged `owner_audit_hp_loss_limit(&RunControlSession) -> RunControlHpLossLimit` behavior for callers.

- [ ] **Step 1: Write failing focused tests**

Add tests asserting that an Act 2 room boss receives `Unlimited`, while a
hallway, event boss, and the first A20 double boss retain `Limit(current_hp -
max_hp/4)`. Also assert that the second A20 boss receives `Unlimited`.

- [ ] **Step 2: Verify RED**

Run:

```powershell
cargo test --lib boss_victory_hp_boundary -- --nocapture
```

Expected: the Act 2 room-boss assertion fails because current code returns
`Limit(21)` for 40/79 HP.

- [ ] **Step 3: Implement the minimal boundary predicate**

Return `RunControlHpLossLimit::Unlimited` only when the active combat context is
a room with `RoomType::MonsterRoomBoss` and
`run_state.should_start_act3_double_boss()` is false. Otherwise retain the
existing floor-reserve calculation.

- [ ] **Step 4: Verify GREEN and guard cases**

Run the focused filter again and confirm every boundary case passes.

### Task 2: Full verification and real-run acceptance

**Files:**
- No additional source files.
- Generate: `target/bounded-mainline-20260710002-boss-hp-boundary/`

- [ ] **Step 1: Run repository verification**

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
cargo fmt --all -- --check
git diff --check
```

- [ ] **Step 2: Commit the implementation**

```powershell
git add src/runtime/branch/owner_audit/combat_search_survival.rs src/runtime/branch/owner_audit/combat_search_lane_options.rs
git commit -m "fix: project boss victory hp boundary"
```

- [ ] **Step 3: Rerun the bounded seed**

Use the same seed and search budgets as the previous strength-package run.
Confirm the Collector win is committed instead of stopping with
`combat search win exceeded hp-loss limit`, then report the next real boundary.
