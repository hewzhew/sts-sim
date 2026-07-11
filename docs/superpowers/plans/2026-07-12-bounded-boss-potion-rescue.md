# Bounded Boss Potion Rescue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one bounded lazy potion rescue after an authoritative boss primary search returns `CombatGap`.

**Architecture:** Preserve the current boss primary profile and existing portfolio orchestration. Change only the boss post-primary plan and the existing `BossPotionRescue` profile so normal primary successes remain cost-free and rescue accepts only complete executable lines.

**Tech Stack:** Rust 2021, Cargo unit and integration tests, existing combat-search lane/profile abstractions.

## Global Constraints

- Execute inline in the current workspace; do not dispatch subagents.
- Do not change boss primary, elite, hallway, potion, or combat-search-internal behavior.
- Enable only `BossPotionRescue`; leave `BossNoPotion`, `BossTimeEaterClock`, and `QualityRealHp` disabled.
- Do not add the frozen Collector outcome as a permanent unit test.

---

### Task 1: Schedule One Boss Rescue Lane

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs:9-30`
- Test: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs:58-68`

**Interfaces:**
- Consumes: `CombatSearchPortfolioContext { stakes: CombatSearchStakes::Boss, .. }`
- Produces: `CombatSearchPortfolioPlan::after_primary(...)` containing exactly one `CombatSearchLaneKind::BossPotionRescue`

- [ ] **Step 1: Replace the stale empty-plan test with the required lane contract**

```rust
#[test]
fn boss_plan_schedules_only_potion_rescue_after_primary_gap() {
    let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
        stakes: CombatSearchStakes::Boss,
        time_eater_boss: false,
        nonboss_potion_rescue_signal: false,
    });

    assert_eq!(
        plan.lane_kinds(),
        vec![CombatSearchLaneKind::BossPotionRescue]
    );
    assert!(!plan.should_report());
}
```

- [ ] **Step 2: Run the exact test and verify RED**

Run:

```powershell
cargo test --lib boss_plan_schedules_only_potion_rescue_after_primary_gap
```

Expected: FAIL because the current boss plan returns an empty vector.

- [ ] **Step 3: Add exactly one boss rescue lane**

Replace the boss match arm with:

```rust
CombatSearchStakes::Boss => vec![CombatSearchLane::new(
    CombatSearchLaneKind::BossPotionRescue,
)],
```

- [ ] **Step 4: Run the exact test and verify GREEN**

Run:

```powershell
cargo test --lib boss_plan_schedules_only_potion_rescue_after_primary_gap
```

Expected: PASS.

- [ ] **Step 5: Commit the independently tested portfolio change**

```powershell
git add src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs
git commit -m "fix: restore bounded boss rescue lane"
```

### Task 2: Make Boss Potion Rescue Lazy in Every Act

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs:110-118`
- Remove: `src/runtime/branch/owner_audit/combat_search_lane_options.rs:200-208`
- Test: `src/runtime/branch/owner_audit/combat_search_lane_options.rs:483-504`

**Interfaces:**
- Consumes: `CombatSearchLaneKind::BossPotionRescue`, `CombatSearchRequest`, and `RunControlSession`
- Produces: a boss-budget `CombatSearchProfile` with `LazyOnPop`, adaptive no-potion rollout evaluation, all legal potions, existing boss potion budget, and `AcceptedLineOnly`

- [ ] **Step 1: Add an Act 2 rescue-profile contract**

Insert this test without changing the existing primary-boss test:

```rust
#[test]
fn act2_boss_potion_rescue_uses_lazy_complete_win_profile() {
    let mut session = session_with_combat_stakes(true, false);
    session.run_state.act_num = 2;
    let request = CombatSearchRequest::from_session(&session, test_args());
    let lane = CombatSearchLane::new(CombatSearchLaneKind::BossPotionRescue);
    let options = lane_options(lane, &request, &session);
    let config = options.search.profile.expect("profile").to_config();

    assert_eq!(config.max_nodes, test_args().boss_search_nodes);
    assert_eq!(
        config.wall_time.map(|duration| duration.as_millis() as u64),
        Some(test_args().boss_search_ms)
    );
    assert_eq!(
        config.child_rollout_policy,
        sts_simulator::ai::combat_search_v2::CombatSearchV2ChildRolloutPolicy::LazyOnPop
    );
    assert_eq!(
        config.rollout_policy,
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    );
    assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::All);
    assert_eq!(config.max_potions_used, Some(3));
    assert_eq!(
        lane.acceptance_plugin(),
        sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId::AcceptedLineOnly
    );
}
```

- [ ] **Step 2: Run the exact test and verify RED**

Run:

```powershell
cargo test --lib act2_boss_potion_rescue_uses_lazy_complete_win_profile
```

Expected: FAIL because Act 2 currently selects `Immediate` child rollout.

- [ ] **Step 3: Pin the rescue lane to `LazyOnPop` and remove the obsolete act switch**

Use the fixed plugin directly in the `BossPotionRescue` profile:

```rust
CombatSearchLaneKind::BossPotionRescue => profile_with_budget(
    lane.label(),
    request.args,
    LaneSearchBudget::Boss,
    CombatSearchChildRolloutPluginId::LazyOnPop,
)
.with_rollout_plugin(CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion)
.with_potion_policy(CombatSearchV2PotionPolicy::All)
.with_max_potions_used(boss_potion_budget(session)),
```

Delete `boss_potion_rescue_child_rollout_plugin`; no act-specific child-rollout decision remains.

- [ ] **Step 4: Run both focused contracts and verify GREEN**

Run:

```powershell
cargo test --lib boss_plan_schedules_only_potion_rescue_after_primary_gap
cargo test --lib act2_boss_potion_rescue_uses_lazy_complete_win_profile
```

Expected: both tests PASS.

- [ ] **Step 5: Commit the independently tested profile change**

```powershell
git add src/runtime/branch/owner_audit/combat_search_lane_options.rs
git commit -m "fix: defer boss potion rescue rollouts"
```

### Task 3: Verify the Repair and Frozen Evidence

**Files:**
- Verify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs`
- Verify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Verify: `artifacts/runs/bounded-mainline-seed-20260712001-combat-upgrade-coverage/combat_cases/seed20260712001_g23_b0023_a2f32_thecollector.json`

**Interfaces:**
- Consumes: the two committed lane/profile changes and the existing frozen Collector case
- Produces: fresh formatting, unit, architecture, and executable-win evidence

- [ ] **Step 1: Check formatting and patch hygiene**

Run:

```powershell
cargo fmt -- --check
git diff --check
```

Expected: both commands exit 0.

- [ ] **Step 2: Run the complete library and architecture suites**

Run:

```powershell
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: both suites pass with zero failures.

- [ ] **Step 3: Re-run the motivating frozen battle under the proven lazy diagnostic profile**

Run:

```powershell
cargo run --quiet --bin combat_case_review -- --case "artifacts\runs\bounded-mainline-seed-20260712001-combat-upgrade-coverage\combat_cases\seed20260712001_g23_b0023_a2f32_thecollector.json" --ladder --compact
```

Expected: `slow_potion_diagnostic` reports a complete win and classification `PotionRescueWon`.

- [ ] **Step 4: Confirm repository state and summarize evidence**

Run:

```powershell
git status --short
git log -3 --oneline
```

Expected: no uncommitted tracked changes; the design and two implementation commits are present.
