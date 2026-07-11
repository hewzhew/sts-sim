# Apparition Upgrade Redundancy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep repeated Apparition upgrades independently valuable so five-copy decks can execute reliability Smith plans without weakening rest safety.

**Architecture:** Correct the central card-analysis stack behavior for Apparition and let the existing upgrade planner, repair profile, campfire evaluator, and owner consume that semantic fact unchanged. Prove the real five-copy shape at the planner and campfire boundaries, then verify with one fresh bounded owner-audit run.

**Tech Stack:** Rust, Cargo unit/integration tests, `branch_tiny` owner-audit runner.

## Global Constraints

- Do not bypass the campfire upgrade-score gate for repair tags.
- Do not change numeric Smith thresholds or RecoveryPressure/`RestFavored` checks.
- Do not change boss-relic ordering, Pandora admission, or runner scripts.
- Do not add exact-seed, capsule, frontier, or checkpoint regression tests.
- Do not use subagents; execute inline in the isolated worktree.

---

### Task 1: Correct repeated Apparition upgrade semantics

**Files:**
- Modify: `src/ai/card_analysis_v1.rs:721`
- Modify: `src/ai/upgrade_planner_v1.rs:1034`
- Modify: `src/ai/campfire_policy_v1/tests.rs:211`

**Interfaces:**
- Consumes: `card_analysis_profile_v1(CardId, u8) -> CardAnalysisProfileV1`, `plan_upgrades_v1(&RunState) -> UpgradePlanV1`, and `plan_campfire_decision_v1(&CampfireDecisionContextV1, &CampfirePolicyConfigV1) -> CampfireDecisionV1`.
- Produces: `CardAnalysisUpgradeStackBehaviorV1::DensityPositive` for Apparition and unchanged typed repair/campfire outputs.

- [ ] **Step 1: Write the failing five-copy planner test**

Add this test to `upgrade_planner_v1.rs`:

```rust
#[test]
fn repeated_apparition_upgrades_keep_independent_reliability_value() {
    let mut run = RunState::new(1, 0, false, "Ironclad");
    run.master_deck = (0..5)
        .map(|index| CombatCard::new(CardId::Apparition, 100 + index))
        .collect();

    let plan = plan_upgrades_v1(&run);
    let apparitions = plan
        .candidates
        .iter()
        .filter(|candidate| candidate.card == CardId::Apparition)
        .collect::<Vec<_>>();

    assert_eq!(apparitions.len(), 5);
    assert!(apparitions.iter().all(|candidate| {
        candidate.redundancy.stack_behavior == StackBehaviorV1::DensityPositive
            && !candidate.roles.contains(&UpgradeRoleV1::LowMarginalRepeat)
    }));
}
```

- [ ] **Step 2: Expand the existing campfire tests to the real five-copy shape**

In both `reliability_repair_smith_precedes_generic_growth_when_safe` and
`rest_favored_still_blocks_reliability_repair_smith`, replace the two Apparitions with five:

```rust
run_state.master_deck = std::iter::once(
    crate::runtime::combat::CombatCard::new(CardId::Cleave, 1),
)
.chain((0..5).map(|index| {
    crate::runtime::combat::CombatCard::new(CardId::Apparition, 100 + index)
}))
.collect();
```

For the healthy test, identify Apparition Smith targets from the deck rather than hard-coded indices:

```rust
let is_apparition_smith = |choice: CampfireChoice| match choice {
    CampfireChoice::Smith(index) => {
        run_state.master_deck.get(index).is_some_and(|card| card.id == CardId::Apparition)
    }
    _ => false,
};
```

Use this helper for the candidate and selected-action assertions while retaining the existing
`repair_priority == Some(DeckRepairUpgradePriorityV1::Reliability)` and repair-tag assertions.

- [ ] **Step 3: Run the new tests and verify the semantic red state**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib upgrade_planner_v1::tests::repeated_apparition_upgrades_keep_independent_reliability_value
cargo test --lib campfire_policy_v1::tests::reliability_repair_smith_precedes_generic_growth_when_safe
```

Expected: both FAIL because five Apparitions still receive generic saturation and no Apparition
Smith plan clears the existing score gate.

- [ ] **Step 4: Implement the minimal central semantic correction**

At the start of `upgrade_stack_behavior_v1` in `card_analysis_v1.rs`, add:

```rust
if card == CardId::Apparition {
    return CardAnalysisUpgradeStackBehaviorV1::DensityPositive;
}
```

Do not change `redundancy_saturated`, campfire configuration, evaluator thresholds, or repair
priority ordering.

- [ ] **Step 5: Run focused tests and verify green behavior and rest safety**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib upgrade_planner_v1::tests::repeated_apparition_upgrades_keep_independent_reliability_value
cargo test --lib campfire_policy_v1::tests::reliability_repair_smith_precedes_generic_growth_when_safe
cargo test --lib campfire_policy_v1::tests::rest_favored_still_blocks_reliability_repair_smith
cargo test --lib card_analysis_v1::tests::
cargo test --lib deck_repair_profile_v1::tests::
cargo test --lib campfire_policy_v1::tests::
```

Expected: all PASS; the healthy five-copy campfire selects an Apparition and the recovery case
still selects Rest.

- [ ] **Step 6: Commit the semantic fix**

```powershell
git add src/ai/card_analysis_v1.rs src/ai/upgrade_planner_v1.rs src/ai/campfire_policy_v1/tests.rs
git commit -m "fix: preserve repeated Apparition upgrade value"
```

---

### Task 2: Verify, merge, and run one fresh bounded mainline

**Files:**
- Verify: `src/ai/card_analysis_v1.rs`
- Verify: `src/ai/upgrade_planner_v1.rs`
- Verify: `src/ai/campfire_policy_v1/tests.rs`
- Generate ignored artifact: `target/bounded-mainline-20260711002-apparition-redundancy/`

**Interfaces:**
- Consumes: the committed semantic correction from Task 1 and the established owner-audit run contract.
- Produces: a clean merged `master` and a fresh capsule whose manifest source identity matches the merged commit.

- [ ] **Step 1: Run final verification in the feature worktree**

Run:

```powershell
cargo fmt --all -- --check
git diff --check
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: formatting and diff checks succeed; 0 test failures.

- [ ] **Step 2: Finish the branch with a local fast-forward merge**

Use `superpowers:finishing-a-development-branch`, selecting the already-approved local merge
workflow. Verify the merged commit on `master` before removing the owned `.worktrees` worktree and
deleting the feature branch.

- [ ] **Step 3: Run one fresh bounded single-branch capsule from merged master**

Ensure `target/bounded-mainline-20260711002-apparition-redundancy` does not already exist, then run:

```powershell
cargo run --profile fast-run --quiet --bin branch_tiny -- --seed 20260711002 --ascension 0 --generations 64 --max-branches 1 --objective first-victory --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 200000 --rescue-search-ms 3000 --boss-search-nodes 800000 --boss-search-ms 10000 --wall-ms 60000 --run-capsule target\bounded-mainline-20260711002-apparition-redundancy
```

Expected: a new capsule is created from Neow without resume/frontier arguments.

- [ ] **Step 4: Verify source identity and compare the first real stop**

Read `manifest.json`, `summary.json`, and `path.json`. Confirm:

```text
manifest.source_identity.git_commit == git rev-parse --short HEAD
manifest.source_identity.git_dirty == false
```

Compare the selected path, Apparition upgrade count, first real stop, HP, deck, and combat-search
telemetry against `target/bounded-mainline-20260711002-deck-repair-50194d77`. Report evidence without
claiming general win-rate improvement from one seed.
