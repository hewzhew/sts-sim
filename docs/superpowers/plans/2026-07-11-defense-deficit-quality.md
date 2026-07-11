# Defense Deficit Quality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent starter-quality Defends and Iron Waves from independently declaring Act 2 defense adequate, while preserving the existing strong-defense path to adequacy.

**Architecture:** Keep the correction inside `deck_strategic_deficit`: reduce only the capped contribution from low-quality block, leaving role inventory, deficit levels, reward scoring, HP pressure, and combat-owner policy unchanged. Verify the shared semantic correction both at the deficit layer and at the reward scoring consumer.

**Tech Stack:** Rust, built-in `#[test]` unit tests, Cargo, Git worktrees.

## Global Constraints

- Do not change the owner combat HP reserve or accept the rejected Snake Plant line directly.
- Do not change the general low-HP `survival_pressure` thresholds.
- Do not thread branch combat history into `RunStrategicFacts` or core deck analysis in this pass.
- Do not special-case Second Wind, Headbutt, Snake Plant, Black Blood, or seed `20260711004`.
- Do not reweight strong block, Weak, or enemy-strength-down roles.
- Do not add a full seed/capsule regression test; verify the seed only after focused and full suites pass.

---

### Task 1: Correct low-quality defense semantics

**Files:**
- Modify: `src/ai/strategy/deck_strategic_deficit.rs:382-398`
- Modify: `src/ai/strategy/decision_pipeline.rs:1580-end` (tests only)

**Interfaces:**
- Consumes: `assess_deck_strategic_deficit(deck: &[CombatCard], facts: RunStrategicFacts) -> DeckStrategicDeficit` and the existing `reward_card_with_act_and_hp` test helper.
- Produces: unchanged public types and signatures; only `block_or_mitigation_units` returns at most two units from Defend/Iron Wave copies alone.

- [ ] **Step 1: Add the failing starter-defense semantic test**

Append to the existing `deck_strategic_deficit.rs` test module:

```rust
#[test]
fn act2_starter_defends_alone_remain_thin() {
    let deck = [
        CardId::Strike,
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, id)| card(id, index as u32 + 1))
    .collect::<Vec<_>>();

    let deficit = assess_deck_strategic_deficit(&deck, act2_facts());

    assert_eq!(deficit.block_or_mitigation, StrategicDeficitLevel::Thin);
}
```

- [ ] **Step 2: Run the semantic test and verify RED**

Run:

```powershell
cargo test --lib act2_starter_defends_alone_remain_thin
```

Expected: FAIL because the current three-unit low-quality cap returns `Adequate`.

- [ ] **Step 3: Add the strong-defense preservation test**

Append a separate test to the same module:

```rust
#[test]
fn act2_real_block_access_can_close_starter_defense_gap() {
    let deck = [
        CardId::Strike,
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
        CardId::ShrugItOff,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, id)| card(id, index as u32 + 1))
    .collect::<Vec<_>>();

    let deficit = assess_deck_strategic_deficit(&deck, act2_facts());

    assert_eq!(
        deficit.block_or_mitigation,
        StrategicDeficitLevel::Adequate
    );
}
```

- [ ] **Step 4: Add the failing reward-consumer regression**

Append to the `decision_pipeline.rs` test module:

```rust
#[test]
fn act2_second_wind_sees_gap_behind_starter_defends() {
    let deck = vec![
        CardId::Strike,
        CardId::Strike,
        CardId::Strike,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Defend,
        CardId::Bash,
        CardId::Headbutt,
        CardId::Cleave,
        CardId::Offering,
        CardId::Offering,
    ];

    let second_wind = reward_card_with_act_and_hp(&deck, CardId::SecondWind, 0, 2, 52, 74);

    assert!(
        second_wind
            .scores
            .iter()
            .any(|score| score.by == "strategic-survival-gap"),
        "Second Wind should expose the real Act 2 defense gap: {:?}",
        second_wind.scores
    );
    assert!(
        !second_wind
            .scores
            .iter()
            .any(|score| score.by == "strategic-burden-no-gap"),
        "starter Defends must not suppress a survival repair: {:?}",
        second_wind.scores
    );
}
```

- [ ] **Step 5: Run the reward regression and verify RED**

Run:

```powershell
cargo test --lib act2_second_wind_sees_gap_behind_starter_defends
```

Expected: FAIL because the current static deficit is `Adequate`, so the evaluation emits `strategic-burden-no-gap` instead of `strategic-survival-gap`.

- [ ] **Step 6: Implement the minimal semantic correction**

In `block_or_mitigation_units`, change only the low-quality cap:

```rust
let low_quality_cap = counts
    .defend_count
    .saturating_add(counts.iron_wave_count)
    .min(2);
```

Do not change `block_or_mitigation_level`, reward score weights, or HP thresholds.

- [ ] **Step 7: Run focused tests and verify GREEN**

Run:

```powershell
cargo test --lib act2_starter_defends_alone_remain_thin
cargo test --lib act2_real_block_access_can_close_starter_defense_gap
cargo test --lib act2_second_wind_sees_gap_behind_starter_defends
cargo test --lib deck_strategic_deficit::tests
cargo test --lib decision_pipeline::tests
```

Expected: all focused tests PASS with no warnings or failures.

- [ ] **Step 8: Format, inspect scope, and commit**

Run:

```powershell
cargo fmt --check
git diff --check
git diff --stat
git diff -- src/ai/strategy/deck_strategic_deficit.rs src/ai/strategy/decision_pipeline.rs
git add src/ai/strategy/deck_strategic_deficit.rs src/ai/strategy/decision_pipeline.rs
git commit -m "fix: preserve defense gaps behind starter block"
```

Expected: exactly the two test-bearing source files change, and the commit succeeds.

### Task 2: Verify, merge locally, and rerun the bounded seed

**Files:**
- Verify: all Rust library code and `tests/architecture_runtime_boundaries.rs`
- Generate (ignored): `target/bounded-mainline-20260711004-defense-quality/`

**Interfaces:**
- Consumes: Task 1 commit on branch `fix/defense-deficit-quality`.
- Produces: a fast-forwarded local `master` and a fresh run capsule identifying the merged commit with `git_dirty=false`.

- [ ] **Step 1: Run full verification in the isolated worktree**

Run:

```powershell
cargo fmt --check
git diff --check HEAD^
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: formatting and diff checks succeed; all library and architecture tests pass.

- [ ] **Step 2: Fast-forward local master and clean the worktree**

From `D:\rust\sts_simulator` run:

```powershell
git merge --ff-only fix/defense-deficit-quality
git worktree remove .worktrees/defense-deficit-quality
git branch -d fix/defense-deficit-quality
git status --short
```

Expected: master fast-forwards, the isolated worktree and feature branch are removed, and status is clean.

- [ ] **Step 3: Verify the merged master**

Run from `D:\rust\sts_simulator`:

```powershell
cargo fmt --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
```

Expected: the merged checkout passes the same complete suites.

- [ ] **Step 4: Run seed `20260711004` from a fresh capsule**

First verify `target\bounded-mainline-20260711004-defense-quality` does not exist. Then run:

```powershell
cargo run --profile fast-run --quiet --bin branch_tiny -- --seed 20260711004 --ascension 0 --generations 64 --max-branches 1 --objective first-victory --auto-ops 64 --search-nodes 50000 --search-ms 1000 --rescue-search-nodes 200000 --rescue-search-ms 3000 --boss-search-nodes 800000 --boss-search-ms 10000 --wall-ms 60000 --run-capsule target\bounded-mainline-20260711004-defense-quality
```

Expected: the capsule completes or records the next real gap without reusing the old Snake Plant capsule.

- [ ] **Step 5: Compare the new route with the old evidence**

Read:

```powershell
Get-Content target\bounded-mainline-20260711004-defense-quality\manifest.json -Raw
Get-Content target\bounded-mainline-20260711004-defense-quality\summary.json -Raw
```

Compare the selected A2 rewards in the new `path.json` with
`target\bounded-mainline-20260711004\path.json`. Confirm the manifest identifies the merged commit,
`git_dirty=false`, and report whether Second Wind is admitted, whether Snake Plant is reached, and
the first new terminal or gap. Do not implement another behavior change in this task.
