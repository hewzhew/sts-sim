# Combat Survival Escalation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make owner-audit reject combat wins below a 25% max-HP reserve and try one bounded quality rescue after an elite primary gap.

**Architecture:** Keep generic run-control semantics unchanged. A focused owner-audit survival helper supplies a finite `RunControlHpLossLimit`, while the existing outer portfolio reuses the dormant `NonBossPotionRescue` lane as the single elite quality rescue.

**Tech Stack:** Rust, built-in unit test harness, Cargo.

## Global Constraints

- Do not increase default primary, rescue, or boss budgets.
- Do not re-enable run-control's internal no-win rescue.
- Do not add seed panels, checkpoint continuation, or source-replay tests.
- Boss portfolio behavior remains unchanged.
- Execute inline without subagents.

---

### Task 1: Owner-Audit Survival Gate

**Files:**
- Create: `src/runtime/branch/owner_audit/combat_search_survival.rs`
- Modify: `src/runtime/branch/owner_audit.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_recipe.rs`

**Interfaces:**
- Consumes: `RunControlSession::visible_player_hp()` and `RunControlHpLossLimit::Limit(u32)`.
- Produces: `owner_audit_hp_loss_limit(&RunControlSession) -> RunControlHpLossLimit`.

- [x] **Step 1: Write failing recipe and lane-option tests**

Add one recipe test that expects `max_hp_loss == None`, proving the recipe no longer overrides policy, and one lane-options table test for `(80,80)->60`, `(54,80)->34`, and `(17,85)->0`.

- [x] **Step 2: Verify the tests fail for the current unlimited gate**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_recipe::tests::recipe_leaves_hp_loss_policy_to_owner_audit
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests::lane_options_attach_survival_hp_loss_gate
```

Expected: both fail because the recipe currently returns `Some(Unlimited)`.

- [x] **Step 3: Implement the minimal survival helper and wire it into lane options**

Use this policy:

```rust
pub(super) fn owner_audit_hp_loss_limit(session: &RunControlSession) -> RunControlHpLossLimit {
    let (current_hp, max_hp) = session.visible_player_hp();
    let max_hp = max_hp.max(1);
    let reserve_hp = max_hp / 4 + i32::from(max_hp % 4 != 0);
    let max_hp_loss = current_hp.saturating_sub(reserve_hp).max(0) as u32;
    RunControlHpLossLimit::Limit(max_hp_loss)
}
```

Remove `Unlimited` from `CombatSearchRecipe`; after constructing lane options, attach `Some(owner_audit_hp_loss_limit(session))`.

- [x] **Step 4: Verify focused tests pass**

Run both commands from Step 2. Expected: one test passed in each command.

- [x] **Step 5: Commit the survival gate**

```powershell
git add src/runtime/branch/owner_audit.rs src/runtime/branch/owner_audit/combat_search_survival.rs src/runtime/branch/owner_audit/combat_search_lane_options.rs src/runtime/branch/owner_audit/combat_search_recipe.rs
git commit -m "fix: gate owner audit combat hp loss"
```

### Task 2: Single Elite Quality Rescue

**Files:**
- Modify: `src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs`
- Modify: `src/runtime/branch/owner_audit/combat_search_lane_options.rs`

**Interfaces:**
- Consumes: `CombatSearchLaneKind::NonBossPotionRescue` and the existing `HallwayQuality` bounded budget.
- Produces: exactly one post-primary `NonBossPotionRescue` for elite stakes.

- [x] **Step 1: Write failing portfolio and lane-profile tests**

Change the stale elite-plan assertion to expect exactly `NonBossPotionRescue`. Add a lane-options test requiring a 5,000-ms-capped quality budget, immediate child rollout, adaptive rollout, round-robin frontier, semantic potion policy, and one-potion maximum.

- [x] **Step 2: Verify the tests fail against the current empty elite plan and boss-budget rescue profile**

Run:

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests::elite_plan_adds_one_bounded_quality_rescue
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests::nonboss_potion_rescue_uses_bounded_quality_profile
```

Expected: the first reports `left: []`; the second reports the current 9,999-ms boss budget instead of 5,000 ms.

- [x] **Step 3: Implement the single elite lane**

Return `vec![CombatSearchLane::new(CombatSearchLaneKind::NonBossPotionRescue)]` for elite stakes. Build that lane with `quality_profile(..., LaneSearchBudget::HallwayQuality, Immediate, ChampSplitGuard).with_max_potions_used(1)`.

- [x] **Step 4: Verify focused tests and all owner-audit combat-search tests pass**

```powershell
cargo test --lib runtime::branch::owner_audit::combat_search_portfolio_plan::tests
cargo test --lib runtime::branch::owner_audit::combat_search_lane_options::tests
cargo test --lib runtime::branch::owner_audit::combat_search_
```

Expected: all selected tests pass with zero failures.

- [x] **Step 5: Commit the elite rescue**

```powershell
git add src/runtime/branch/owner_audit/combat_search_portfolio_plan.rs src/runtime/branch/owner_audit/combat_search_lane_options.rs
git commit -m "fix: add bounded elite combat rescue"
```

### Task 3: Final Verification

**Files:**
- Modify: none unless verification reveals a directly related defect.

**Interfaces:**
- Consumes: the completed survival gate and elite rescue.
- Produces: fresh verification evidence for the complete library.

- [x] **Step 1: Format and inspect the patch**

```powershell
cargo fmt --check
git diff --check
git status --short
```

- [x] **Step 2: Run the complete library suite**

```powershell
cargo test --lib
```

Expected: all library tests pass with zero failures.

- [x] **Step 3: Commit the implementation plan with any final formatting-only change**

```powershell
git add docs/superpowers/plans/2026-07-10-combat-survival-escalation.md
git commit -m "docs: plan combat survival escalation"
```

## Execution Note

Static review after the two planned red-green cycles found that a finite HP
gate would otherwise reactivate run-control's generic no-potion and potion
rescue staging inside every owner-audit lane. A third red-green regression now
pins each lane profile's potion policy into the command options, keeping the
owner-audit outer portfolio authoritative without changing the selected policy.
