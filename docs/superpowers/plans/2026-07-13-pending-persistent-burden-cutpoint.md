# Pending Persistent Burden Cutpoint Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the cutpoint probe observe newly pending combat curse additions at the action that creates them while treating combat-finish materialization as the same existing burden.

**Architecture:** A private run-control burden ledger combines materialized run-deck curses with active-combat `MetaChange::AddCardToMasterDeck` curse additions and computes deterministic positive count deltas by `CardId`. The locator and one-action classifier consume the ledger; finalized candidate adjudication remains UUID-based and Combat Search V2 is unchanged.

**Tech Stack:** Rust 2021, existing `RunControlSession`, `CombatState::meta_changes`, `serde`, Cargo unit/binary/integration tests, PowerShell verification.

## Global Constraints

- No Writhing Mass, Implant, Parasite, or move-id specialization in production code.
- Do not change engine meta-change creation or combat-finish absorption.
- Do not add search, suffix replay, candidate-cap, ordering, or acceptance-policy behavior.
- Keep full-line candidate adjudication on finalized UUID-aware run-deck deltas.
- The CLI remains a typed adapter and must not inspect cards or combat meta changes.
- Work in `D:\rust\sts_simulator`; do not create a worktree or run `cargo clean`.

---

## File Structure

- Create `src/eval/run_control/persistent_burden_cutpoint_probe/burden.rs`: private composite ledger and positive-delta owner.
- Modify `src/eval/run_control/persistent_burden_cutpoint_probe.rs`: register the helper and expose corrected typed count evidence.
- Modify `src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs`: locate the first composite burden delta instead of finalized deck mutation.
- Modify `src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs`: classify one action with the same composite delta.
- Modify `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`: red/green timing, transfer, unresolved-action, and existing-fixture assertions.
- Modify `src/eval/run_control/mod.rs`: re-export the new public count type.

### Task 1: Add the Composite Persistent-Curse Ledger

**Files:**
- Create: `src/eval/run_control/persistent_burden_cutpoint_probe/burden.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`

**Interfaces:**
- Consumes: `RunControlSession`, `MetaChange`, `get_card_definition`, and `CardType::Curse`.
- Produces: `PersistentCurseBurdenSnapshot::capture(session)` and `newly_gained_persistent_curses(before, after) -> Vec<PersistentBurdenGainedCurseCountV1>`.

- [ ] **Step 1: Add a failing pending-to-materialized transfer test**

In `tests.rs`, build a base session, add one pending curse meta change, then move it into the run deck in a clone:

```rust
#[test]
fn persistent_curse_burden_does_not_double_count_materialization() {
    let (mut pending, _) = fixture_cutpoint_session();
    pending
        .active_combat
        .as_mut()
        .expect("active combat")
        .combat_state
        .meta
        .meta_changes
        .push(MetaChange::AddCardToMasterDeck(CardId::Parasite));
    let pending_snapshot = PersistentCurseBurdenSnapshot::capture(&pending);

    let mut materialized = pending.clone();
    materialized.active_combat = None;
    materialized
        .run_state
        .master_deck
        .push(CombatCard::new(CardId::Parasite, 99));
    let materialized_snapshot = PersistentCurseBurdenSnapshot::capture(&materialized);

    assert!(newly_gained_persistent_curses(
        &pending_snapshot,
        &materialized_snapshot,
    )
    .is_empty());
}
```

- [ ] **Step 2: Run the test and confirm the red state**

Run:

```powershell
cargo test --lib persistent_curse_burden_does_not_double_count_materialization
```

Expected: FAIL because `burden`, `PersistentCurseBurdenSnapshot`, and `newly_gained_persistent_curses` do not exist.

- [ ] **Step 3: Define the corrected public count evidence**

In `persistent_burden_cutpoint_probe.rs`, add:

```rust
use crate::content::cards::CardId;

mod burden;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistentBurdenGainedCurseCountV1 {
    pub card: CardId,
    pub count: usize,
}
```

- [ ] **Step 4: Implement the private composite ledger**

Create `burden.rs`:

```rust
use std::collections::HashMap;

use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::runtime::combat::MetaChange;

use super::super::session::RunControlSession;
use super::PersistentBurdenGainedCurseCountV1;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct PersistentCurseBurdenSnapshot {
    counts: HashMap<CardId, usize>,
}

impl PersistentCurseBurdenSnapshot {
    pub(super) fn capture(session: &RunControlSession) -> Self {
        let mut counts = HashMap::new();
        for card in &session.run_state.master_deck {
            if get_card_definition(card.id).card_type == CardType::Curse {
                *counts.entry(card.id).or_default() += 1;
            }
        }
        if let Some(active) = session.active_combat.as_ref() {
            for change in &active.combat_state.meta.meta_changes {
                if let MetaChange::AddCardToMasterDeck(card_id) = change {
                    if get_card_definition(*card_id).card_type == CardType::Curse {
                        *counts.entry(*card_id).or_default() += 1;
                    }
                }
            }
        }
        Self { counts }
    }
}

pub(super) fn newly_gained_persistent_curses(
    before: &PersistentCurseBurdenSnapshot,
    after: &PersistentCurseBurdenSnapshot,
) -> Vec<PersistentBurdenGainedCurseCountV1> {
    let mut gained = after
        .counts
        .iter()
        .filter_map(|(card, after_count)| {
            let count = after_count.saturating_sub(before.counts.get(card).copied().unwrap_or(0));
            (count > 0).then_some(PersistentBurdenGainedCurseCountV1 {
                card: *card,
                count,
            })
        })
        .collect::<Vec<_>>();
    gained.sort_by_key(|entry| entry.card as i32);
    gained
}
```

- [ ] **Step 5: Run the ledger test and focused module tests**

Run:

```powershell
cargo test --lib persistent_curse_burden_does_not_double_count_materialization
cargo test --lib persistent_burden_cutpoint_probe::tests
```

Expected: the new test passes and existing tests still compile after importing the new helper under `#[cfg(test)]`.

- [ ] **Step 6: Commit the ledger**

```powershell
git add src/eval/run_control/persistent_burden_cutpoint_probe.rs src/eval/run_control/persistent_burden_cutpoint_probe/burden.rs src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs
git commit -m "fix: model pending persistent curse burden"
```

### Task 2: Correct Cutpoint and One-Action Semantics

**Files:**
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`
- Modify: `src/eval/run_control/mod.rs`

**Interfaces:**
- Consumes: Task 1's composite snapshot and positive delta.
- Produces: corrected `trigger_gained_curse_counts` and per-input `gained_curse_counts` JSON evidence.

- [ ] **Step 1: Add the red cutpoint-timing regression**

Change the generic fixture queue order so the curse becomes pending before a stable selection and combat completes only after the selection:

```rust
combat.queue_action_back(Action::AddCardToMasterDeck {
    card_id: CardId::Parasite,
});
combat.queue_action_back(Action::SuspendForHandSelect {
    min: 1,
    max: 1,
    can_cancel: false,
    filter: HandSelectFilter::Any,
    reason: HandSelectReason::Exhaust,
});
combat.queue_action_back(Action::InstantKill { target: monster_id });
```

Replace the old finalization expectation with:

```rust
#[test]
fn pending_curse_is_located_before_later_combat_completion() {
    let (session, config, trajectory) = fixture_line_with_neutral_then_curse_input();
    let located = locate_candidate_cutpoint(&session, &config, 0, &trajectory)
        .expect("replay")
        .expect("burden cutpoint");

    assert_eq!(located.trigger_step_index, 0);
    assert_eq!(located.trigger_gained_curse_counts.len(), 1);
    assert_eq!(located.trigger_gained_curse_counts[0].card, CardId::Parasite);
    assert_eq!(located.trigger_gained_curse_counts[0].count, 1);
}
```

- [ ] **Step 2: Add the red unresolved one-action classification regression**

```rust
#[test]
fn unresolved_pending_curse_action_is_classified_as_new_curse() {
    let (session, config, trajectory) = fixture_line_with_neutral_then_curse_input();
    let located = locate_candidate_cutpoint(&session, &config, 0, &trajectory)
        .expect("replay")
        .expect("burden cutpoint");
    let outcome = probe_cutpoint_actions(&located, &config)
        .into_iter()
        .find(|outcome| outcome.input == trajectory.actions[0].input)
        .expect("trigger outcome");

    assert_eq!(outcome.terminal, CombatTerminal::Unresolved);
    assert_eq!(outcome.kind, PersistentBurdenCutpointInputOutcomeKindV1::NewCurse);
    assert_eq!(outcome.gained_curse_counts[0].card, CardId::Parasite);
}
```

- [ ] **Step 3: Run both tests and confirm they fail for the old timing**

Run:

```powershell
cargo test --lib pending_curse_is_located_before_later_combat_completion
cargo test --lib unresolved_pending_curse_action_is_classified_as_new_curse
```

Expected: FAIL because the locator still waits for run-deck materialization and the outcome type still carries `gained_curses`.

- [ ] **Step 4: Make the locator consume the composite ledger**

In `cutpoint.rs`, add `trigger_gained_curse_counts` to `LocatedBurdenCutpoint`, capture the ledger before and after each input, and store the first positive delta:

```rust
let before = PersistentCurseBurdenSnapshot::capture(&trial);
let clean_session = trial.clone();
trial.apply_input(choice.input.clone())?;
let after = PersistentCurseBurdenSnapshot::capture(&trial);
let gained = newly_gained_persistent_curses(&before, &after);
if !gained.is_empty() {
    // existing identity and cutpoint construction
    trigger_gained_curse_counts: gained,
}
```

Delete the locator's finalized `newly_gained_curses` import. Keep full candidate adjudication unchanged.

- [ ] **Step 5: Make one-action outcomes consume the same ledger**

Replace `PersistentBurdenCutpointInputOutcomeV1.gained_curses` with:

```rust
pub gained_curse_counts: Vec<PersistentBurdenGainedCurseCountV1>,
```

Add to `PersistentBurdenCutpointSummaryV1`:

```rust
pub trigger_gained_curse_counts: Vec<PersistentBurdenGainedCurseCountV1>,
```

In `outcomes.rs`, capture the composite ledger around `trial.apply_input`, classify `NewCurse` from the positive delta, populate failed outcomes with an empty vector, and copy the trigger counts into the public cutpoint summary.

- [ ] **Step 6: Export the count type and update focused assertions**

Add `PersistentBurdenGainedCurseCountV1` to the `pub use persistent_burden_cutpoint_probe::{...}` list in `src/eval/run_control/mod.rs`. Replace existing probe-test checks of `outcome.gained_curses` with `outcome.gained_curse_counts`; do not change UUID-aware full-line tests.

- [ ] **Step 7: Run focused and neighboring tests**

Run:

```powershell
cargo test --lib persistent_burden_cutpoint_probe::tests
cargo test --lib combat_case_candidate_census
cargo test --lib combat_line_outcome
cargo test --bin combat_case_review persistent_burden_probe
cargo test --test architecture_runtime_boundaries
```

Expected: all pass. The CLI boundary test continues to reject `meta_changes` in adapter code.

- [ ] **Step 8: Re-run the saved case once and inspect corrected timing**

Run:

```powershell
cargo run --profile fast-run --bin combat_case_review -- --case target/bounded-mainline-20260712002/combat_cases/seed20260712002_g34_b0034_a3f42_writhingmass.json --adjudicate --fast-nodes 200000 --fast-ms 2000 --slow-nodes 300000 --slow-ms 5000 --write-review artifacts/runs/writhingmass-pending-burden-cutpoints-20260713.json
```

Print each lane's trigger-step distribution, trigger actions, typed curse counts, aggregate outcomes,
and conclusion without rerunning search. Expected: cutpoints occur at the actual pending-curse
transition, not uniformly at final killing Whirlwinds. Any plan-change conclusion must come from an
action whose composite burden count remains unchanged.

- [ ] **Step 9: Run final verification and commit**

Run:

```powershell
cargo test --lib
cargo test --bin combat_case_review
cargo test --test architecture_runtime_boundaries
git diff --check
git status --short
```

Expected: all tests pass, diff check is empty, and status contains only the intended tracked changes.

Commit:

```powershell
git add src/eval/run_control/mod.rs src/eval/run_control/persistent_burden_cutpoint_probe.rs src/eval/run_control/persistent_burden_cutpoint_probe/burden.rs src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs
git commit -m "fix: locate pending persistent burden transitions"
```
