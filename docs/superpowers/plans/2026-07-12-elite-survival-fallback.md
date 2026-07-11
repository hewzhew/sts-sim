# Elite Survival Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recast the existing final elite quality lane as a bounded survival fallback so a clean complete win can commit without adding a third search.

**Architecture:** Rename the elite-only `NonBossPotionRescue` lane to `EliteSurvivalFallback`, preserving its five-second quality profile, clean-win acceptance, and one-potion cap. Keep the strict elite primary reserve, but set the renamed final lane's HP-loss gate to unlimited and retain accepted-high-loss evidence.

**Tech Stack:** Rust, Cargo tests, saved `CombatCase`, `branch_tiny` run capsules.

## Global Constraints

- Work in the stable checkout on `fix/elite-survival-fallback`; do not create a worktree.
- Do not run `cargo clean`.
- Do not add a third elite search lane.
- Do not increase elite search budgets or potion allowance.
- Do not change hallway or boss portfolios.
- Do not change the quarter-max-HP reserve formula.
- Do not change route selection or route artifact schemas.

---

### Task 1: Rename and narrow the elite-only final lane

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lanes.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_spec.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs`

**Interfaces:**
- Consumes: the existing elite post-primary `NonBossPotionRescue` lane.
- Produces: `CombatSearchLaneKind::EliteSurvivalFallback` with label `elite_survival_fallback`, clean-win acceptance, and the unchanged one-lane elite post-primary plan.

- [ ] **Step 1: Change the elite plan test to the wished-for label**

Rename the test and assert the label rather than the old enum identity:

```rust
#[test]
fn elite_plan_ends_with_one_survival_fallback() {
    let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
        stakes: CombatSearchStakes::Elite,
        time_eater_boss: false,
        nonboss_potion_rescue_signal: true,
    });

    assert_eq!(plan.lane_labels(), vec!["elite_survival_fallback"]);
    assert!(!plan.should_report());
}
```

- [ ] **Step 2: Run the plan test and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests::elite_plan_ends_with_one_survival_fallback
```

Expected: assertion failure; actual label is `nonboss_potion_rescue`.

- [ ] **Step 3: Apply the semantic rename without changing HP policy**

Make these exact replacements:

```rust
// combat_search_lanes.rs
EliteSurvivalFallback,
```

```rust
// combat_search_lane_spec.rs
CombatSearchLaneKind::EliteSurvivalFallback => {
    dirty_rejecting_spec("elite_survival_fallback")
}
```

```rust
// combat_search_portfolio_plan.rs elite arm
CombatSearchStakes::Elite => vec![CombatSearchLane::new(
    CombatSearchLaneKind::EliteSurvivalFallback,
)],
```

Rename the `lane_profile` match arm and the existing bounded-profile test from
`NonBossPotionRescue` / `nonboss_potion_rescue_uses_bounded_quality_profile`
to `EliteSurvivalFallback` /
`elite_survival_fallback_uses_bounded_quality_profile`. Do not change
`options.search.max_hp_loss` in this task.

- [ ] **Step 4: Run focused owner tests and verify GREEN**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_
```

Expected: all focused owner combat-search tests pass with the renamed label.

- [ ] **Step 5: Commit the semantic rename**

```powershell
git add src/runtime/branch/owner_audit/combat_search_lanes.rs src/runtime/branch/owner_audit/combat_search_lane_spec.rs src/runtime/branch/owner_audit/combat_search_lane_options.rs src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs
git commit -m "refactor: name elite survival fallback"
```

### Task 2: Give only the final elite lane survival HP semantics

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`

**Interfaces:**
- Consumes: `CombatSearchLaneKind::EliteSurvivalFallback`, the existing strict owner reserve, and the existing quality profile.
- Produces: strict elite primary `Limit(27)` at 45/74 HP and fallback `Unlimited`, with unchanged budget, potion, rollout, frontier, and acceptance settings.

- [ ] **Step 1: Extend the fallback profile test with the HP boundary and acceptance contract**

Set the elite test session to 45/74 HP, construct both primary and fallback
options, and add these assertions before the existing profile assertions:

```rust
let mut session = session_with_combat_stakes(false, true);
let player = &mut session
    .active_combat
    .as_mut()
    .expect("active combat")
    .combat_state
    .entities
    .player;
player.current_hp = 45;
player.max_hp = 74;
let request = CombatSearchRequest::from_session(&session, test_args());
let primary = lane_options(CombatSearchLane::primary(), &request, &session);
let lane = CombatSearchLane::new(CombatSearchLaneKind::EliteSurvivalFallback);
let fallback = lane_options(lane, &request, &session);

assert_eq!(
    primary.search.max_hp_loss,
    Some(RunControlHpLossLimit::Limit(27))
);
assert_eq!(
    fallback.search.max_hp_loss,
    Some(RunControlHpLossLimit::Unlimited)
);
assert_eq!(
    lane.acceptance_plugin(),
    CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
);
assert!(lane.rejects_new_curses());
```

Continue to derive `config` from `fallback.search.profile` and retain the
existing assertions for 300,000-node cap selection, 5,000 ms, immediate child
rollout, adaptive rollout, round-robin frontier, semantic potion policy, and a
one-potion maximum.

- [ ] **Step 2: Run the fallback test and verify RED**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests::elite_survival_fallback_uses_bounded_quality_profile
```

Expected: fallback reports `Some(Limit(27))` instead of `Some(Unlimited)`;
primary and profile assertions pass.

- [ ] **Step 3: Add the minimal typed HP override**

```rust
options.search.max_hp_loss = Some(match lane.kind() {
    CombatSearchLaneKind::HallwaySurvivalFallback
    | CombatSearchLaneKind::EliteSurvivalFallback => RunControlHpLossLimit::Unlimited,
    _ => owner_audit_hp_loss_limit(session),
});
```

- [ ] **Step 4: Run the fallback and focused owner tests and verify GREEN**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests::elite_survival_fallback_uses_bounded_quality_profile
cargo test --lib runtime::branch::owner_audit::combat_search_
```

Expected: the fallback test and all focused owner combat-search tests pass.

- [ ] **Step 5: Commit the HP policy change**

```powershell
git add src/runtime/branch/owner_audit/combat_search_lane_options.rs
git commit -m "fix: allow bounded elite survival wins"
```

### Task 3: Verify the exact Book profile without retaining probe code

**Files:**
- Temporarily create then delete: `src/bin/tmp_elite_survival_probe.rs`
- Read: `artifacts/runs/route-reliability-seed-20260711004-survival-fallback/combat_cases/seed20260711004_g18_b0018_a2f23_bookofstabbing.json`

**Interfaces:**
- Consumes: the saved Book position and the exact final elite profile.
- Produces: local evidence that the unchanged bounded profile still finds the 13 HP / one-potion complete win.

- [ ] **Step 1: Create a temporary exact-profile probe**

Use `apply_patch` to create this file:

```rust
use std::path::Path;

use sts_simulator::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId,
    CombatSearchBudgetSpec, CombatSearchChildRolloutPluginId, CombatSearchFrontierPluginId,
    CombatSearchPhaseGuardPluginId, CombatSearchPluginStack, CombatSearchProfile,
    CombatSearchRolloutPluginId, CombatSearchV2PotionPolicy,
};
use sts_simulator::eval::combat_case::load_combat_case;

fn main() {
    let case = load_combat_case(Path::new(
        "artifacts/runs/route-reliability-seed-20260711004-survival-fallback/combat_cases/seed20260711004_g18_b0018_a2f23_bookofstabbing.json",
    ))
    .expect("load Book case");
    let profile = CombatSearchProfile {
        label: "tmp_elite_survival_probe",
        budget: CombatSearchBudgetSpec {
            max_nodes: 300_000,
            wall_ms: 5_000,
        },
        plugins: CombatSearchPluginStack {
            child_rollout: CombatSearchChildRolloutPluginId::Immediate,
            rollout: CombatSearchRolloutPluginId::EnemyMechanicsAdaptiveNoPotion,
            frontier: CombatSearchFrontierPluginId::RoundRobinEvalBuckets,
            phase_guard: CombatSearchPhaseGuardPluginId::ChampSplitGuard,
            ..CombatSearchPluginStack::default()
        },
        acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
        artifacts: CombatSearchArtifactPluginId::None,
    }
    .with_potion_policy(CombatSearchV2PotionPolicy::SemanticBudgeted)
    .with_max_potions_used(1);
    let report = run_combat_search_v2(
        &case.position.engine,
        &case.position.combat,
        profile.to_config(),
    );
    let best = report.best_win_trajectory.as_ref();
    println!(
        "complete_win={} final_hp={:?} hp_loss={:?} potions_used={:?} nodes={} terminal_wins={} elapsed_ms={}",
        report.outcome.complete_win_found,
        best.map(|line| line.final_hp),
        best.map(|line| line.hp_loss),
        best.map(|line| line.potions_used),
        report.stats.nodes_expanded,
        report.stats.terminal_wins,
        report.stats.elapsed_ms,
    );
}
```

- [ ] **Step 2: Run the probe**

```powershell
cargo run --quiet --bin tmp_elite_survival_probe
```

Expected: `complete_win=true`, `final_hp=Some(13)`, `hp_loss=Some(32)`, and
`potions_used=Some(1)`.

- [ ] **Step 3: Delete the temporary probe immediately**

Use `apply_patch` to delete `src/bin/tmp_elite_survival_probe.rs`, then verify:

```powershell
git status --short
```

Expected: no temporary probe remains.

### Task 4: Run completion suites and one fresh bounded seed

**Files:**
- Write ignored evidence: `artifacts/runs/route-reliability-seed-20260711004-elite-survival-fallback/`
- No tracked source changes expected.

**Interfaces:**
- Consumes: the merged feature behavior and the established seed contract.
- Produces: full verification plus the first real post-Book blocker or terminal result.

- [ ] **Step 1: Run completion tests**

```powershell
cargo fmt --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
git diff --check
```

Expected: formatting passes; 2,686 library tests and seven architecture tests
pass; diff check is clean.

- [ ] **Step 2: Run the fresh bounded seed**

```powershell
cargo run --quiet --bin branch_tiny -- --seed 20260711004 --ascension 0 --objective first-victory --generations 64 --max-branches 1 --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 200000 --rescue-search-ms 3000 --boss-search-nodes 800000 --boss-search-ms 10000 --wall-ms 60000 --run-capsule artifacts/runs/route-reliability-seed-20260711004-elite-survival-fallback
```

Expected: the run no longer stops at A2F23 Book of Stabbing for an HP-loss
policy rejection. If it stops later, report the first new blocker without
modifying behavior in this slice.

- [ ] **Step 3: Verify high-loss evidence and final repository state**

```powershell
$root = 'artifacts/runs/route-reliability-seed-20260711004-elite-survival-fallback'
$result = Get-Content "$root/result.json" -Raw | ConvertFrom-Json
$bookEvidence = Get-ChildItem "$root/accepted_high_loss_combat" -Filter '*bookofstabbing.evidence.json'
Write-Output "status=$($result.status.kind):$($result.status.reason)"
Write-Output "book_evidence_count=$($bookEvidence.Count)"
if ($bookEvidence.Count -gt 0) {
    $evidence = Get-Content $bookEvidence[0].FullName -Raw | ConvertFrom-Json
    Write-Output "book_lane=$($evidence.lane)"
}
git status --short --branch
git log -6 --oneline
```

Expected: `book_evidence_count=1`, `book_lane=elite_survival_fallback`, and the
tracked worktree is clean. The result status may expose a later blocker and is
not required to be a victory.
