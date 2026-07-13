# Persistent Run Burden Cutpoint Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in diagnostic that locates the last clean player boundary before a retained combat win first adds a persistent curse, then reports every legal one-action affordance from that boundary without changing search or run behavior.

**Architecture:** Run-control remains the sole semantic owner: it projects the saved combat case, exactly replays already-retained winning trajectories, captures the pre-burden `RunControlSession`, groups only fully equivalent combat-plus-run contexts, and applies each legal input once to a clone. `combat_case_review` only invokes the typed API under `--adjudicate` and attaches the result to the matching ladder row; it contains no curse, monster, move-id, or search logic.

**Tech Stack:** Rust 2021, existing `RunControlSession`, Combat Search V2 report/config types, `EngineCombatStepper`, `serde`, `blake2`, Cargo unit/binary/integration tests, PowerShell verification commands.

## Global Constraints

- The probe is diagnostic evidence only; it must not change search, acceptance policy, combat execution, or the run.
- The public entry point is `probe_combat_case_persistent_burden_cutpoints_v1(source_review, case, config, report) -> CombatCasePersistentBurdenCutpointProbeV1`.
- `SearchReview.persistent_burden_cutpoint_probe` is absent unless `--adjudicate` is enabled.
- The fixed examination cap is exactly `16` unique cutpoints in retained report order, and the serialized result discloses the cap, whether it was hit, and how many cutpoints were omitted.
- Cutpoint equivalence includes the exact combat-state hash plus current/max HP, gold, complete master-deck card state, relic state, potion state, and every run RNG counter; combat hash alone is insufficient.
- One-action outcomes are exactly: clean combat victory, new curse, living-enemy planned-move change without a curse, neutral, or input application failure.
- Conclusion precedence is exactly: clean immediate win, then clean plan change, then probe failures, then no one-action escape.
- A cutpoint-cap hit alone does not force an incomplete conclusion.
- Do not add suffix replay, multi-action counterfactuals, full-seed replay, a new combat search, search scoring, frontier ordering, timed-threat changes, candidate-retention changes, policy changes, or a standalone diagnostic binary.
- Do not add Writhing Mass, Implant, `Parasite`, move-id, or `meta_changes` policy rules to the CLI or the production run-control probe.
- Do not commit generated review artifacts under `artifacts/runs/`.

---

## File Structure

- Create `src/eval/run_control/combat_case_retained_candidates.rs`: one shared owner for report-order retained-win deduplication and retained indices.
- Modify `src/eval/run_control/combat_case_candidate_census.rs`: consume the shared retained-candidate owner; keep census summarization local.
- Modify `src/eval/run_control/combat_line_outcome.rs`: share retained-candidate iteration and newly-gained-curse detection with the new probe.
- Create `src/eval/run_control/persistent_burden_cutpoint_probe.rs`: public serialized types, top-level orchestration, fixed cap, conclusion aggregation, and source-label accessor.
- Create `src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs`: exact prefix replay, first-curse cutpoint capture, persistent identity, and grouping.
- Create `src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs`: legal-input enumeration, one stable application, mechanical classification, and aggregate counts.
- Create `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`: focused run-control fixtures and regression tests, including generic first-burden capture and Writhing Mass Reactive behavior.
- Modify `src/eval/run_control/combat_candidate_line.rs`: expose the existing potion-budget filter to sibling run-control modules without changing its behavior.
- Modify `src/eval/run_control/mod.rs`: register the focused modules and export only the public diagnostic API/types.
- Modify `src/bin/combat_case_review/adjudication_probe.rs`: invoke the typed probe once per retained ladder run when adjudication is enabled.
- Modify `src/bin/combat_case_review/review_pipeline.rs`: attach probe results to the matching ladder rows.
- Modify `src/bin/combat_case_review/search_types.rs`: serialize the optional field and attach only matching source labels.
- Modify `src/bin/combat_case_review/search_review.rs`: initialize the new optional field to `None`.
- Modify `tests/architecture_runtime_boundaries.rs`: lock the semantic ownership boundary at the CLI adapter.

### Task 1: Share Retained Candidate and Curse-Delta Facts

**Files:**
- Create: `src/eval/run_control/combat_case_retained_candidates.rs`
- Modify: `src/eval/run_control/mod.rs`
- Modify: `src/eval/run_control/combat_case_candidate_census.rs`
- Modify: `src/eval/run_control/combat_line_outcome.rs`

**Interfaces:**
- Consumes: `CombatSearchV2Report`, `CombatSearchV2TrajectoryReport`, and `CombatCard`.
- Produces: `unique_retained_win_trajectories(report: &CombatSearchV2Report) -> RetainedWinTrajectories<'_>` and `newly_gained_curses(before: &[CombatCard], after: &[CombatCard]) -> Vec<CardSnapshot>` for Tasks 2 and 3.

- [ ] **Step 1: Add failing tests for report-order deduplication and curse UUID deltas**

Register `mod combat_case_retained_candidates;` in `src/eval/run_control/mod.rs`. Create `src/eval/run_control/combat_case_retained_candidates.rs` with the data contract and tests first:

```rust
use crate::ai::combat_search_v2::{
    CombatSearchV2Report, CombatSearchV2TrajectoryReport,
};

pub(super) struct RetainedWinTrajectory<'a> {
    pub(super) retained_index: usize,
    pub(super) trajectory: &'a CombatSearchV2TrajectoryReport,
}

pub(super) struct RetainedWinTrajectories<'a> {
    pub(super) retained_candidate_count: usize,
    pub(super) trajectories: Vec<RetainedWinTrajectory<'a>>,
}

pub(super) fn unique_retained_win_trajectories(
    _report: &CombatSearchV2Report,
) -> RetainedWinTrajectories<'_> {
    unimplemented!("deduplicate retained win trajectories")
}

#[cfg(test)]
mod tests {
    use crate::ai::combat_search_v2::{
        CombatSearchV2ActionTrace, CombatSearchV2StateSummary,
        CombatSearchV2TrajectoryReport, SearchTerminalLabel,
    };
    use crate::state::core::ClientInput;

    use super::unique_action_trace_indices;

    fn trajectory(keys: &[&str]) -> CombatSearchV2TrajectoryReport {
        CombatSearchV2TrajectoryReport {
            terminal: SearchTerminalLabel::Win,
            estimated: false,
            actions: keys
                .iter()
                .enumerate()
                .map(|(step_index, action_key)| CombatSearchV2ActionTrace {
                    step_index,
                    action_id: step_index,
                    action_key: (*action_key).to_string(),
                    action_debug: (*action_key).to_string(),
                    input: ClientInput::EndTurn,
                })
                .collect(),
            final_hp: 30,
            final_max_hp: 40,
            persistent_run_value: 0,
            final_block: 0,
            hp_loss: 10,
            turns: 2,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 1,
            enemy_final_state: Vec::new(),
            final_state: CombatSearchV2StateSummary {
                engine_state: "RewardScreen".to_string(),
                terminal: SearchTerminalLabel::Win,
                player_hp: 30,
                player_block: 0,
                energy: 0,
                turn_count: 2,
                living_enemy_count: 0,
                total_enemy_hp: 0,
                visible_incoming_damage: 0,
                enemy_slots: Vec::new(),
                hand_count: 0,
                draw_count: 0,
                discard_count: 0,
                exhaust_count: 0,
                limbo_count: 0,
                queued_cards_count: 0,
            },
        }
    }

    #[test]
    fn retained_action_trace_dedup_preserves_first_report_index() {
        let first = trajectory(&["a", "b"]);
        let duplicate = trajectory(&["a", "b"]);
        let distinct = trajectory(&["a", "c"]);
        let retained = [&first, &duplicate, &distinct];

        assert_eq!(unique_action_trace_indices(&retained), vec![0, 2]);
    }
}
```

Add this test to `combat_line_outcome.rs` before extracting the helper:

```rust
#[test]
fn newly_gained_curses_uses_uuid_and_ignores_preexisting_curses() {
    let before = vec![
        CombatCard::new(CardId::Parasite, 7),
        CombatCard::new(CardId::Strike, 8),
    ];
    let after = vec![
        CombatCard::new(CardId::Parasite, 7),
        CombatCard::new(CardId::Strike, 8),
        CombatCard::new(CardId::Parasite, 9),
        CombatCard::new(CardId::Defend, 10),
    ];

    assert_eq!(newly_gained_curses(&before, &after).len(), 1);
    assert_eq!(newly_gained_curses(&before, &after)[0].uuid, 9);
}
```

- [ ] **Step 2: Run the focused tests and confirm the red state**

Run:

```powershell
cargo test --lib retained_action_trace_dedup_preserves_first_report_index
cargo test --lib newly_gained_curses_uses_uuid_and_ignores_preexisting_curses
```

Expected: FAIL because `unique_action_trace_indices` and `newly_gained_curses` are not implemented yet.

- [ ] **Step 3: Implement the shared retained-candidate owner**

Implement `combat_case_retained_candidates.rs` with action-key sequence equality and first-occurrence indices:

```rust
use std::collections::HashSet;

use crate::ai::combat_search_v2::{
    CombatSearchV2Report, CombatSearchV2TrajectoryReport,
};

pub(super) struct RetainedWinTrajectory<'a> {
    pub(super) retained_index: usize,
    pub(super) trajectory: &'a CombatSearchV2TrajectoryReport,
}

pub(super) struct RetainedWinTrajectories<'a> {
    pub(super) retained_candidate_count: usize,
    pub(super) trajectories: Vec<RetainedWinTrajectory<'a>>,
}

pub(super) fn unique_retained_win_trajectories(
    report: &CombatSearchV2Report,
) -> RetainedWinTrajectories<'_> {
    let retained = report
        .best_win_trajectory
        .iter()
        .chain(&report.win_candidate_trajectories)
        .collect::<Vec<_>>();
    let unique_indices = unique_action_trace_indices(&retained);
    let trajectories = unique_indices
        .into_iter()
        .map(|retained_index| RetainedWinTrajectory {
            retained_index,
            trajectory: retained[retained_index],
        })
        .collect();
    RetainedWinTrajectories {
        retained_candidate_count: retained.len(),
        trajectories,
    }
}

fn unique_action_trace_indices(
    retained: &[&CombatSearchV2TrajectoryReport],
) -> Vec<usize> {
    let mut seen = HashSet::<Vec<&str>>::new();
    retained
        .iter()
        .enumerate()
        .filter_map(|(index, trajectory)| {
            let fingerprint = trajectory
                .actions
                .iter()
                .map(|action| action.action_key.as_str())
                .collect::<Vec<_>>();
            seen.insert(fingerprint).then_some(index)
        })
        .collect()
}
```

In `combat_case_candidate_census.rs`, replace the local `HashSet`/`action_trace_fingerprint` loop with:

```rust
let retained = unique_retained_win_trajectories(report);
let retained_candidate_count = retained.retained_candidate_count;
if retained_candidate_count == 0 {
    return empty_census(source_review);
}
let unique_candidate_count = retained.trajectories.len();
// project the session exactly as before
for retained in retained.trajectories {
    let action_count = retained.trajectory.actions.len();
    let line = CombatCandidateLine::from_search_trajectory(retained.trajectory);
    let evaluation = evaluate_combat_candidate_line_outcome(
        &session,
        &case.position,
        config,
        line,
    )
    .map(|evaluation| (retained.retained_index, evaluation.outcome))
    .map_err(|error| CombatCaseCandidateReplayFailureV1 {
        retained_index: retained.retained_index,
        action_count,
        error,
    });
    evaluations.push(evaluation);
}
```

Pass `unique_candidate_count` into `summarize_evaluations` and delete the old local fingerprint helper/test.

- [ ] **Step 4: Extract one curse-delta helper and make existing outcome evaluation consume it**

Add to `combat_line_outcome.rs`:

```rust
pub(super) fn newly_gained_curses(
    before: &[CombatCard],
    after: &[CombatCard],
) -> Vec<CardSnapshot> {
    let before_uuids = before.iter().map(|card| card.uuid).collect::<HashSet<_>>();
    after
        .iter()
        .filter(|card| {
            !before_uuids.contains(&card.uuid)
                && get_card_definition(card.id).card_type == CardType::Curse
        })
        .map(|card| CardSnapshot {
            id: card.id,
            uuid: card.uuid,
            upgrades: card.upgrades,
        })
        .collect()
}
```

In `evaluate_combat_candidate_line_outcome`, clone `session.run_state.master_deck` before replay and replace the inline curse query with:

```rust
let before_master_deck = session.run_state.master_deck.clone();
// apply the exact candidate line to trial
let gained_curses = newly_gained_curses(
    &before_master_deck,
    &trial.run_state.master_deck,
);
```

Replace the private `win_candidate_trajectories` helper with `unique_retained_win_trajectories(report).trajectories` in `find_accepted_alternative_in_report`, preserving report order and existing best-clean selection.

- [ ] **Step 5: Run focused and neighboring tests**

Run:

```powershell
cargo test --lib retained_action_trace_dedup_preserves_first_report_index
cargo test --lib newly_gained_curses_uses_uuid_and_ignores_preexisting_curses
cargo test --lib combat_case_candidate_census
cargo test --lib combat_line_outcome
```

Expected: all commands PASS; existing census counts and best-clean choice remain unchanged.

- [ ] **Step 6: Commit the shared facts refactor**

```powershell
git add src/eval/run_control/mod.rs src/eval/run_control/combat_case_retained_candidates.rs src/eval/run_control/combat_case_candidate_census.rs src/eval/run_control/combat_line_outcome.rs
git commit -m "refactor: share retained combat candidate facts"
```

### Task 2: Locate and Group the First Persistent-Burden Cutpoint

**Files:**
- Create: `src/eval/run_control/persistent_burden_cutpoint_probe.rs`
- Create: `src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs`
- Create: `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`
- Modify: `src/eval/run_control/combat_candidate_line.rs`
- Modify: `src/eval/run_control/mod.rs`

**Interfaces:**
- Consumes: `unique_retained_win_trajectories`, `newly_gained_curses`, `project_combat_case_session`, `filter_combat_search_legal_actions`, and the existing replay potion budget.
- Produces: internal `LocatedBurdenCutpoint`, `GroupedBurdenCutpoint`, and `locate_and_group_cutpoints(base_session, config, report) -> CutpointLocationReport` for Task 3.

- [ ] **Step 1: Add failing tests for pre-trigger capture, full identity, and grouping**

Create `persistent_burden_cutpoint_probe/tests.rs` with three direct assertions:

```rust
#[test]
fn first_gained_curse_captures_pre_trigger_session() {
    let (session, config, trajectory) = fixture_line_with_neutral_then_curse_input();
    let located = locate_candidate_cutpoint(&session, &config, 0, &trajectory)
        .expect("replay")
        .expect("burden cutpoint");

    assert_eq!(located.trigger_step_index, 1);
    assert!(newly_gained_curses(
        &session.run_state.master_deck,
        &located.session.run_state.master_deck,
    )
    .is_empty());
    let mut triggered = located.session.clone();
    triggered.apply_input(located.trigger_input.clone()).expect("trigger");
    assert_eq!(
        newly_gained_curses(
            &located.session.run_state.master_deck,
            &triggered.run_state.master_deck,
        )
        .len(),
        1
    );
}

#[test]
fn cutpoint_identity_includes_persistent_context_not_only_combat_hash() {
    let (session, position) = fixture_cutpoint_session();
    let base = cutpoint_identity(&session, &position);

    let mut changed_gold = session.clone();
    changed_gold.run_state.gold += 1;
    assert_ne!(base.canonical, cutpoint_identity(&changed_gold, &position).canonical);

    let mut changed_growth = session.clone();
    changed_growth.run_state.master_deck[0].misc_value += 1;
    assert_ne!(base.canonical, cutpoint_identity(&changed_growth, &position).canonical);

    let mut changed_rng = session.clone();
    changed_rng.run_state.rng_pool.card_rng.counter += 1;
    assert_ne!(base.canonical, cutpoint_identity(&changed_rng, &position).canonical);

    let mut changed_plan = position.clone();
    let next = changed_plan.combat.entities.monsters[0]
        .planned_move_id()
        .wrapping_add(1);
    changed_plan.combat.entities.monsters[0].set_planned_move_id(next);
    assert_ne!(base.canonical, cutpoint_identity(&session, &changed_plan).canonical);
}

#[test]
fn equivalent_cutpoints_group_and_keep_first_report_order() {
    let first = fixture_located_cutpoint(3, "same");
    let equivalent = fixture_located_cutpoint(8, "same");
    let distinct = fixture_located_cutpoint(9, "different");
    let grouped = group_cutpoints(vec![first, equivalent, distinct]);

    assert_eq!(grouped.len(), 2);
    assert_eq!(grouped[0].candidate_frequency, 2);
    assert_eq!(grouped[0].retained_indices, vec![3, 8]);
    assert_eq!(grouped[1].retained_indices, vec![9]);
}
```

The fixture `fixture_line_with_neutral_then_curse_input` must create a minimal projected combat session whose first legal input does not mutate the master deck and whose second input executes a generic `Action::AddCardToMasterDeck`; it must not teach production code a card or monster name. Keep all fixture-only monster setup in `tests.rs`.

- [ ] **Step 2: Run the cutpoint tests and confirm the red state**

Run:

```powershell
cargo test --lib persistent_burden_cutpoint_probe::tests::first_gained_curse_captures_pre_trigger_session
cargo test --lib persistent_burden_cutpoint_probe::tests::cutpoint_identity_includes_persistent_context_not_only_combat_hash
cargo test --lib persistent_burden_cutpoint_probe::tests::equivalent_cutpoints_group_and_keep_first_report_order
```

Expected: FAIL because the cutpoint module and helpers do not exist.

- [ ] **Step 3: Expose the existing potion budget without duplicating policy**

Change only the visibility in `combat_candidate_line.rs`:

```rust
pub(super) fn enforce_replay_potion_budget(
    choices: Vec<crate::sim::combat_action::CombatActionChoice>,
    config: &CombatSearchV2Config,
    potions_used: u32,
) -> Vec<crate::sim::combat_action::CombatActionChoice> {
    // keep the existing body byte-for-byte
}
```

- [ ] **Step 4: Implement exact prefix replay and first-curse capture**

In `persistent_burden_cutpoint_probe/cutpoint.rs`, define:

```rust
pub(super) struct LocatedBurdenCutpoint {
    pub(super) retained_index: usize,
    pub(super) trigger_step_index: usize,
    pub(super) trigger_action_key: String,
    pub(super) trigger_input: ClientInput,
    pub(super) potions_used_before: u32,
    pub(super) identity: BurdenCutpointIdentity,
    pub(super) session: RunControlSession,
    pub(super) position: CombatPosition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BurdenCutpointIdentity {
    pub(super) state_hash: String,
    pub(super) canonical: String,
}

pub(super) fn locate_candidate_cutpoint(
    base_session: &RunControlSession,
    config: &CombatSearchV2Config,
    retained_index: usize,
    trajectory: &CombatSearchV2TrajectoryReport,
) -> Result<Option<LocatedBurdenCutpoint>, String> {
    let mut trial = base_session.clone();
    trial.mark_current_combat_search_resolved();
    let mut potions_used = 0u32;

    for action in &trajectory.actions {
        let position = trial.current_active_combat_position()?;
        let choices = enforce_replay_potion_budget(
            filter_combat_search_legal_actions(
                EngineCombatStepper.legal_action_choices(&position),
                config.potion_policy,
                &position.combat,
            ),
            config,
            potions_used,
        );
        let Some(choice) = choices.iter().find(|choice| {
            choice.input == action.input && choice.action_key == action.action_key
        }) else {
            return Err(format!(
                "persistent burden cutpoint replay drift at step {}: expected {}",
                action.step_index, action.action_key
            ));
        };

        let before = trial.run_state.master_deck.clone();
        let clean_session = trial.clone();
        trial.apply_input(choice.input.clone())?;
        let gained = newly_gained_curses(&before, &trial.run_state.master_deck);
        if !gained.is_empty() {
            let identity = cutpoint_identity(&clean_session, &position);
            return Ok(Some(LocatedBurdenCutpoint {
                retained_index,
                trigger_step_index: action.step_index,
                trigger_action_key: action.action_key.clone(),
                trigger_input: action.input.clone(),
                potions_used_before: potions_used,
                identity,
                session: clean_session,
                position,
            }));
        }
        if matches!(choice.input, ClientInput::UsePotion { .. }) {
            potions_used = potions_used.saturating_add(1);
        }
    }
    Ok(None)
}
```

Do not call `replay_candidate_line`: that helper returns only the final position and would discard the needed pre-trigger session.

- [ ] **Step 5: Implement collision-safe persistent identity and grouping**

Build identity equality from the full canonical value, using the hash only as the serialized label:

```rust
pub(super) fn cutpoint_identity(
    session: &RunControlSession,
    position: &CombatPosition,
) -> BurdenCutpointIdentity {
    let run = &session.run_state;
    let combat_hash = combat_exact_state_hash_v1(&position.engine, &position.combat);
    let rng_counters = (
        run.rng_pool.monster_rng.counter,
        run.rng_pool.event_rng.counter,
        run.rng_pool.merchant_rng.counter,
        run.rng_pool.card_rng.counter,
        run.rng_pool.treasure_rng.counter,
        run.rng_pool.relic_rng.counter,
        run.rng_pool.potion_rng.counter,
        run.rng_pool.monster_hp_rng.counter,
        run.rng_pool.ai_rng.counter,
        run.rng_pool.shuffle_rng.counter,
        run.rng_pool.card_random_rng.counter,
        run.rng_pool.misc_rng.counter,
        run.rng_pool.math_rng.counter,
        run.neow_rng.counter,
    );
    let canonical = format!(
        "{:?}",
        (
            combat_hash,
            run.current_hp,
            run.max_hp,
            run.gold,
            &run.master_deck,
            &run.relics,
            &run.potions,
            rng_counters,
        )
    );
    let mut hasher = Blake2b512::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    let state_hash = digest[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    BurdenCutpointIdentity { state_hash, canonical }
}
```

Group by `identity.canonical` in insertion order, append retained indices, and keep the first session/trigger as the representative. Never group solely by `state_hash`:

```rust
pub(super) struct GroupedBurdenCutpoint {
    pub(super) representative: LocatedBurdenCutpoint,
    pub(super) candidate_frequency: usize,
    pub(super) retained_indices: Vec<usize>,
}

pub(super) fn group_cutpoints(
    located: Vec<LocatedBurdenCutpoint>,
) -> Vec<GroupedBurdenCutpoint> {
    let mut grouped = Vec::<GroupedBurdenCutpoint>::new();
    for cutpoint in located {
        if let Some(existing) = grouped.iter_mut().find(|existing| {
            existing.representative.identity.canonical == cutpoint.identity.canonical
        }) {
            existing.candidate_frequency += 1;
            existing.retained_indices.push(cutpoint.retained_index);
        } else {
            let retained_index = cutpoint.retained_index;
            grouped.push(GroupedBurdenCutpoint {
                representative: cutpoint,
                candidate_frequency: 1,
                retained_indices: vec![retained_index],
            });
        }
    }
    grouped
}
```

- [ ] **Step 6: Implement the report-wide locator and typed replay failures**

Add:

```rust
pub(super) struct CutpointLocationReport {
    pub(super) retained_candidate_count: usize,
    pub(super) unique_candidate_count: usize,
    pub(super) dirty_candidate_count: usize,
    pub(super) grouped: Vec<GroupedBurdenCutpoint>,
    pub(super) replay_failures: Vec<CombatCaseCandidateReplayFailureV1>,
}

pub(super) fn locate_and_group_cutpoints(
    base_session: &RunControlSession,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
) -> CutpointLocationReport {
    let retained = unique_retained_win_trajectories(report);
    let retained_candidate_count = retained.retained_candidate_count;
    let unique_candidate_count = retained.trajectories.len();
    let mut located = Vec::new();
    let mut replay_failures = Vec::new();

    for candidate in retained.trajectories {
        match locate_candidate_cutpoint(
            base_session,
            config,
            candidate.retained_index,
            candidate.trajectory,
        ) {
            Ok(Some(cutpoint)) => located.push(cutpoint),
            Ok(None) => {}
            Err(error) => replay_failures.push(CombatCaseCandidateReplayFailureV1 {
                retained_index: candidate.retained_index,
                action_count: candidate.trajectory.actions.len(),
                error,
            }),
        }
    }
    let dirty_candidate_count = located.len();
    CutpointLocationReport {
        retained_candidate_count,
        unique_candidate_count,
        dirty_candidate_count,
        grouped: group_cutpoints(located),
        replay_failures,
    }
}
```

- [ ] **Step 7: Run the cutpoint tests and neighboring replay tests**

Run:

```powershell
cargo test --lib persistent_burden_cutpoint_probe::tests
cargo test --lib combat_candidate_line
cargo test --lib combat_case_candidate_census
```

Expected: all commands PASS; the captured session has no newly gained curse, identity differences do not collapse, and grouping order is deterministic.

- [ ] **Step 8: Commit the cutpoint locator**

```powershell
git add src/eval/run_control/mod.rs src/eval/run_control/combat_candidate_line.rs src/eval/run_control/persistent_burden_cutpoint_probe.rs src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs
git commit -m "feat: locate persistent burden cutpoints"
```

### Task 3: Probe Every Legal One-Action Affordance

**Files:**
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe.rs`
- Create: `src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs`
- Modify: `src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs`
- Modify: `src/eval/run_control/mod.rs`

**Interfaces:**
- Consumes: `CutpointLocationReport`, grouped representative sessions/positions, `EngineCombatStepper`, and the reviewed search configuration.
- Produces: public `probe_combat_case_persistent_burden_cutpoints_v1` and all serialized `CombatCasePersistentBurdenCutpoint*V1` types for Task 4.

- [ ] **Step 1: Add failing tests for mechanical outcomes and conclusion precedence**

Add to `persistent_burden_cutpoint_probe/tests.rs`:

```rust
fn conclude_probe(
    clean_wins: usize,
    plan_changes: usize,
    input_failures: usize,
    replay_failures: usize,
) -> PersistentBurdenCutpointConclusionV1 {
    conclusion_from_aggregate(
        &PersistentBurdenCutpointAggregateV1 {
            clean_terminal_win_count: clean_wins,
            living_enemy_plan_change_count: plan_changes,
            input_failure_count: input_failures,
            ..PersistentBurdenCutpointAggregateV1::default()
        },
        replay_failures,
    )
}

#[test]
fn writhing_mass_reactive_attack_is_a_clean_plan_change() {
    let cutpoint = fixture_writhing_mass_reactive_cutpoint();
    let outcomes = probe_cutpoint_actions(&cutpoint, &CombatSearchV2Config::default());

    assert!(outcomes.iter().any(|outcome| {
        outcome.kind == PersistentBurdenCutpointInputOutcomeKindV1::LivingEnemyPlanChanged
            && matches!(outcome.input, ClientInput::PlayCard { .. })
            && outcome.gained_curses.is_empty()
    }));
}

#[test]
fn clean_win_then_plan_change_then_failures_define_conclusion_precedence() {
    assert_eq!(
        conclude_probe(1, 1, 1, 1),
        PersistentBurdenCutpointConclusionV1::CleanTerminalWinAvailable
    );
    assert_eq!(
        conclude_probe(0, 1, 1, 1),
        PersistentBurdenCutpointConclusionV1::BurdenTriggerPlanChangeAvailable
    );
    assert_eq!(
        conclude_probe(0, 0, 1, 1),
        PersistentBurdenCutpointConclusionV1::IncompleteDueToProbeFailures
    );
    assert_eq!(
        conclude_probe(0, 0, 0, 0),
        PersistentBurdenCutpointConclusionV1::NoOneActionEscapeObserved
    );
}

#[test]
fn input_failure_cannot_report_no_one_action_escape() {
    let aggregate = PersistentBurdenCutpointAggregateV1 {
        clean_terminal_win_count: 0,
        burden_trigger_count: 2,
        living_enemy_plan_change_count: 0,
        neutral_count: 4,
        input_failure_count: 1,
    };
    assert_eq!(
        conclusion_from_aggregate(&aggregate, 0),
        PersistentBurdenCutpointConclusionV1::IncompleteDueToProbeFailures
    );
}
```

The Writhing Mass fixture must use the real Reactive power and engine action path. It may name Writhing Mass inside `tests.rs`, but `persistent_burden_cutpoint_probe.rs`, `cutpoint.rs`, and `outcomes.rs` must remain generic.

- [ ] **Step 2: Run the outcome tests and confirm the red state**

Run:

```powershell
cargo test --lib persistent_burden_cutpoint_probe::tests::writhing_mass_reactive_attack_is_a_clean_plan_change
cargo test --lib persistent_burden_cutpoint_probe::tests::clean_win_then_plan_change_then_failures_define_conclusion_precedence
cargo test --lib persistent_burden_cutpoint_probe::tests::input_failure_cannot_report_no_one_action_escape
```

Expected: FAIL because the public outcome and conclusion types are not defined.

- [ ] **Step 3: Define the public serialized contract and fixed cap**

In `persistent_burden_cutpoint_probe.rs`, add these exact public names:

```rust
pub const PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1: usize = 16;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistentBurdenCutpointConclusionV1 {
    CleanTerminalWinAvailable,
    BurdenTriggerPlanChangeAvailable,
    NoOneActionEscapeObserved,
    NoDirtyCandidateCutpoint,
    IncompleteDueToProbeFailures,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistentBurdenCutpointInputOutcomeKindV1 {
    CleanCombatVictory,
    NewCurse,
    LivingEnemyPlanChanged,
    Neutral,
    ApplyFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistentBurdenEnemyPlanChangeV1 {
    pub entity_id: usize,
    pub enemy: String,
    pub before_plan_id: u8,
    pub after_plan_id: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PersistentBurdenCutpointInputOutcomeV1 {
    pub action_key: String,
    pub input: ClientInput,
    pub kind: PersistentBurdenCutpointInputOutcomeKindV1,
    pub terminal: CombatTerminal,
    pub gained_curses: Vec<CardSnapshot>,
    pub living_enemy_plan_changes: Vec<PersistentBurdenEnemyPlanChangeV1>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersistentBurdenCutpointAggregateV1 {
    pub clean_terminal_win_count: usize,
    pub burden_trigger_count: usize,
    pub living_enemy_plan_change_count: usize,
    pub neutral_count: usize,
    pub input_failure_count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PersistentBurdenCutpointSummaryV1 {
    pub cutpoint_state_hash: String,
    pub candidate_frequency: usize,
    pub retained_indices: Vec<usize>,
    pub trigger_step_index: usize,
    pub trigger_action_key: String,
    pub trigger_input: ClientInput,
    pub player_hp: i32,
    pub player_block: i32,
    pub enemy_hp: Vec<i32>,
    pub outcomes: Vec<PersistentBurdenCutpointInputOutcomeV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CombatCasePersistentBurdenCutpointProbeV1 {
    NoDirtyCandidateCutpoint {
        source_review: String,
        retained_candidate_count: usize,
        unique_candidate_count: usize,
        replay_failures: Vec<CombatCaseCandidateReplayFailureV1>,
        conclusion: PersistentBurdenCutpointConclusionV1,
    },
    ProjectionFailed {
        source_review: String,
        error: String,
    },
    Probed {
        source_review: String,
        projection_trust: String,
        retained_candidate_count: usize,
        unique_candidate_count: usize,
        dirty_candidate_count: usize,
        candidates_with_cutpoint: usize,
        unique_cutpoint_count: usize,
        examined_cutpoint_count: usize,
        cutpoint_limit: usize,
        cutpoint_limit_hit: bool,
        omitted_cutpoint_count: usize,
        replay_failures: Vec<CombatCaseCandidateReplayFailureV1>,
        aggregate: PersistentBurdenCutpointAggregateV1,
        cutpoints: Vec<PersistentBurdenCutpointSummaryV1>,
        conclusion: PersistentBurdenCutpointConclusionV1,
    },
}
```

Use the actual `EntityId` scalar type for `entity_id` if it is not `usize`; keep the serialized field name unchanged.

- [ ] **Step 4: Implement one stable action classification**

In `outcomes.rs`, obtain legal choices with both search potion policy and prefix potion count:

```rust
pub(super) fn probe_cutpoint_actions(
    cutpoint: &LocatedBurdenCutpoint,
    config: &CombatSearchV2Config,
) -> Vec<PersistentBurdenCutpointInputOutcomeV1> {
    enforce_replay_potion_budget(
        filter_combat_search_legal_actions(
            EngineCombatStepper.legal_action_choices(&cutpoint.position),
            config.potion_policy,
            &cutpoint.position.combat,
        ),
        config,
        cutpoint.potions_used_before,
    )
    .into_iter()
    .map(|choice| probe_one_action(cutpoint, config, choice))
    .collect()
}

fn probe_one_action(
    cutpoint: &LocatedBurdenCutpoint,
    config: &CombatSearchV2Config,
    choice: CombatActionChoice,
) -> PersistentBurdenCutpointInputOutcomeV1 {
```

Inside `probe_one_action`, apply the choice through both the stable combat stepper and the cloned run session:

```rust
let step = EngineCombatStepper.apply_to_stable(
    &cutpoint.position,
    choice.input.clone(),
    CombatStepLimits {
        max_engine_steps: config.max_engine_steps_per_action,
        deadline: None,
    },
);
if step.truncated || step.timed_out {
    return failed_outcome(choice, format!(
        "one-action step truncated={} timed_out={} engine_steps={}",
        step.truncated, step.timed_out, step.engine_steps
    ));
}

let before_deck = cutpoint.session.run_state.master_deck.clone();
let mut trial = cutpoint.session.clone();
if let Err(error) = trial.apply_input(choice.input.clone()) {
    return failed_outcome(choice, error);
}
let gained_curses = newly_gained_curses(&before_deck, &trial.run_state.master_deck);
let plan_changes = living_enemy_plan_changes(
    &cutpoint.position.combat,
    &step.position.combat,
);
let kind = if step.terminal == CombatTerminal::Win && gained_curses.is_empty() {
    PersistentBurdenCutpointInputOutcomeKindV1::CleanCombatVictory
} else if !gained_curses.is_empty() {
    PersistentBurdenCutpointInputOutcomeKindV1::NewCurse
} else if !plan_changes.is_empty() {
    PersistentBurdenCutpointInputOutcomeKindV1::LivingEnemyPlanChanged
} else {
    PersistentBurdenCutpointInputOutcomeKindV1::Neutral
};
PersistentBurdenCutpointInputOutcomeV1 {
    action_key: choice.action_key,
    input: choice.input,
    kind,
    terminal: step.terminal,
    gained_curses,
    living_enemy_plan_changes: plan_changes,
    error: None,
}
}
```

`living_enemy_plan_changes` must match monsters by entity id, require both before and after entities to be alive for action, and compare `planned_move_id()` mechanically. A killed or escaped enemy is not a plan change. The original candidate trigger remains in the enumerated choices when legal.

Use these helpers so failures remain typed and plan changes remain purely mechanical:

```rust
fn failed_outcome(
    choice: CombatActionChoice,
    error: String,
) -> PersistentBurdenCutpointInputOutcomeV1 {
    PersistentBurdenCutpointInputOutcomeV1 {
        action_key: choice.action_key,
        input: choice.input,
        kind: PersistentBurdenCutpointInputOutcomeKindV1::ApplyFailed,
        terminal: CombatTerminal::Unresolved,
        gained_curses: Vec::new(),
        living_enemy_plan_changes: Vec::new(),
        error: Some(error),
    }
}

fn living_enemy_plan_changes(
    before: &CombatState,
    after: &CombatState,
) -> Vec<PersistentBurdenEnemyPlanChangeV1> {
    before
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .filter_map(|before_monster| {
            let after_monster = after
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == before_monster.id)?;
            if !after_monster.is_alive_for_action()
                || before_monster.planned_move_id() == after_monster.planned_move_id()
            {
                return None;
            }
            Some(PersistentBurdenEnemyPlanChangeV1 {
                entity_id: before_monster.id,
                enemy: before_monster.monster_type.to_string(),
                before_plan_id: before_monster.planned_move_id(),
                after_plan_id: after_monster.planned_move_id(),
            })
        })
        .collect()
}
```

Convert each group into its compact public summary with one representative session but the full frequency/index evidence:

```rust
pub(super) fn probe_grouped_cutpoint(
    cutpoint: GroupedBurdenCutpoint,
    config: &CombatSearchV2Config,
) -> PersistentBurdenCutpointSummaryV1 {
    let outcomes = probe_cutpoint_actions(&cutpoint.representative, config);
    PersistentBurdenCutpointSummaryV1 {
        cutpoint_state_hash: cutpoint.representative.identity.state_hash.clone(),
        candidate_frequency: cutpoint.candidate_frequency,
        retained_indices: cutpoint.retained_indices,
        trigger_step_index: cutpoint.representative.trigger_step_index,
        trigger_action_key: cutpoint.representative.trigger_action_key.clone(),
        trigger_input: cutpoint.representative.trigger_input.clone(),
        player_hp: cutpoint
            .representative
            .position
            .combat
            .entities
            .player
            .current_hp,
        player_block: cutpoint
            .representative
            .position
            .combat
            .entities
            .player
            .block,
        enemy_hp: cutpoint
            .representative
            .position
            .combat
            .entities
            .monsters
            .iter()
            .map(|monster| monster.current_hp)
            .collect(),
        outcomes,
    }
}
```

- [ ] **Step 5: Implement aggregation and deterministic conclusions**

Add:

```rust
fn conclusion_from_aggregate(
    aggregate: &PersistentBurdenCutpointAggregateV1,
    replay_failure_count: usize,
) -> PersistentBurdenCutpointConclusionV1 {
    if aggregate.clean_terminal_win_count > 0 {
        PersistentBurdenCutpointConclusionV1::CleanTerminalWinAvailable
    } else if aggregate.living_enemy_plan_change_count > 0 {
        PersistentBurdenCutpointConclusionV1::BurdenTriggerPlanChangeAvailable
    } else if replay_failure_count > 0 || aggregate.input_failure_count > 0 {
        PersistentBurdenCutpointConclusionV1::IncompleteDueToProbeFailures
    } else {
        PersistentBurdenCutpointConclusionV1::NoOneActionEscapeObserved
    }
}
```

Count each legal input exactly once. `cutpoint_limit_hit` is `unique_cutpoint_count > 16`; `omitted_cutpoint_count` is `unique_cutpoint_count.saturating_sub(16)`. Do not feed either value into `conclusion_from_aggregate`.

Aggregate the typed outcomes without reinterpreting their inputs:

```rust
fn aggregate_cutpoints(
    cutpoints: &[PersistentBurdenCutpointSummaryV1],
) -> PersistentBurdenCutpointAggregateV1 {
    let mut aggregate = PersistentBurdenCutpointAggregateV1::default();
    for outcome in cutpoints.iter().flat_map(|cutpoint| &cutpoint.outcomes) {
        match outcome.kind {
            PersistentBurdenCutpointInputOutcomeKindV1::CleanCombatVictory => {
                aggregate.clean_terminal_win_count += 1;
            }
            PersistentBurdenCutpointInputOutcomeKindV1::NewCurse => {
                aggregate.burden_trigger_count += 1;
            }
            PersistentBurdenCutpointInputOutcomeKindV1::LivingEnemyPlanChanged => {
                aggregate.living_enemy_plan_change_count += 1;
            }
            PersistentBurdenCutpointInputOutcomeKindV1::Neutral => {
                aggregate.neutral_count += 1;
            }
            PersistentBurdenCutpointInputOutcomeKindV1::ApplyFailed => {
                aggregate.input_failure_count += 1;
            }
        }
    }
    aggregate
}
```

- [ ] **Step 6: Implement the public run-control entry point**

Add to `persistent_burden_cutpoint_probe.rs`:

```rust
pub fn probe_combat_case_persistent_burden_cutpoints_v1(
    source_review: impl Into<String>,
    case: &CombatCase,
    config: &CombatSearchV2Config,
    report: &CombatSearchV2Report,
) -> CombatCasePersistentBurdenCutpointProbeV1 {
    let source_review = source_review.into();
    let base_session = match project_combat_case_session(case) {
        Ok(session) => session,
        Err(error) => {
            return CombatCasePersistentBurdenCutpointProbeV1::ProjectionFailed {
                source_review,
                error,
            };
        }
    };
    let located = locate_and_group_cutpoints(&base_session, config, report);
    if located.grouped.is_empty() {
        let conclusion = if located.replay_failures.is_empty() {
            PersistentBurdenCutpointConclusionV1::NoDirtyCandidateCutpoint
        } else {
            PersistentBurdenCutpointConclusionV1::IncompleteDueToProbeFailures
        };
        return CombatCasePersistentBurdenCutpointProbeV1::NoDirtyCandidateCutpoint {
            source_review,
            retained_candidate_count: located.retained_candidate_count,
            unique_candidate_count: located.unique_candidate_count,
            replay_failures: located.replay_failures,
            conclusion,
        };
    }

    let unique_cutpoint_count = located.grouped.len();
    let retained_candidate_count = located.retained_candidate_count;
    let unique_candidate_count = located.unique_candidate_count;
    let dirty_candidate_count = located.dirty_candidate_count;
    let replay_failures = located.replay_failures;
    let cutpoints = located
        .grouped
        .into_iter()
        .take(PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1)
        .map(|cutpoint| probe_grouped_cutpoint(cutpoint, config))
        .collect::<Vec<_>>();
    let aggregate = aggregate_cutpoints(&cutpoints);
    let conclusion = conclusion_from_aggregate(&aggregate, replay_failures.len());
    let examined_cutpoint_count = cutpoints.len();
    let cutpoint_limit_hit = unique_cutpoint_count > PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1;
    let omitted_cutpoint_count = unique_cutpoint_count
        .saturating_sub(PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1);

    CombatCasePersistentBurdenCutpointProbeV1::Probed {
        source_review,
        projection_trust: COMBAT_CASE_PROJECTION_TRUST_V1.to_string(),
        retained_candidate_count,
        unique_candidate_count,
        dirty_candidate_count,
        candidates_with_cutpoint: dirty_candidate_count,
        unique_cutpoint_count,
        examined_cutpoint_count,
        cutpoint_limit: PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1,
        cutpoint_limit_hit,
        omitted_cutpoint_count,
        replay_failures,
        aggregate,
        cutpoints,
        conclusion,
    }
}
```

`candidates_with_cutpoint` and `dirty_candidate_count` are deliberately identical in V1: both are the number of unique retained trajectories whose replay located a curse transition, not the number of groups.

Implement the label accessor exactly as follows so the CLI does not inspect variants:

```rust
impl CombatCasePersistentBurdenCutpointProbeV1 {
    pub fn source_review(&self) -> &str {
        match self {
            Self::NoDirtyCandidateCutpoint { source_review, .. }
            | Self::ProjectionFailed { source_review, .. }
            | Self::Probed { source_review, .. } => source_review,
        }
    }
}
```

- [ ] **Step 7: Export the typed API and run all probe tests**

Register `mod persistent_burden_cutpoint_probe;` and export:

```rust
pub use persistent_burden_cutpoint_probe::{
    probe_combat_case_persistent_burden_cutpoints_v1,
    CombatCasePersistentBurdenCutpointProbeV1,
    PersistentBurdenCutpointAggregateV1,
    PersistentBurdenCutpointConclusionV1,
    PersistentBurdenCutpointInputOutcomeKindV1,
    PersistentBurdenCutpointInputOutcomeV1,
    PersistentBurdenCutpointSummaryV1,
    PersistentBurdenEnemyPlanChangeV1,
    PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1,
};
```

Run:

```powershell
cargo test --lib persistent_burden_cutpoint_probe::tests
cargo test --lib combat_case_candidate_census
cargo test --lib combat_case_adjudication
```

Expected: all commands PASS, including the real Reactive plan-change fixture and failure-precedence assertion.

- [ ] **Step 8: Commit the run-control probe**

```powershell
git add src/eval/run_control/mod.rs src/eval/run_control/persistent_burden_cutpoint_probe.rs src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs src/eval/run_control/persistent_burden_cutpoint_probe/tests.rs
git commit -m "feat: probe persistent burden cutpoints"
```

### Task 4: Attach the Probe to Adjudicated Ladder Reviews

**Files:**
- Modify: `src/bin/combat_case_review/adjudication_probe.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`
- Modify: `src/bin/combat_case_review/search_types.rs`
- Modify: `src/bin/combat_case_review/search_review.rs`
- Modify: `tests/architecture_runtime_boundaries.rs`

**Interfaces:**
- Consumes: `probe_combat_case_persistent_burden_cutpoints_v1` and `CombatCasePersistentBurdenCutpointProbeV1`.
- Produces: optional per-row JSON field `persistent_burden_cutpoint_probe`, present only under `--adjudicate` and only on the matching source review.

- [ ] **Step 1: Add failing CLI serialization and attachment tests**

Extend the `review` helper in `search_types.rs` to accept both optional diagnostics, then add:

```rust
#[test]
fn persistent_burden_probe_is_omitted_when_absent() {
    let value = serde_json::to_value(review(None, None)).expect("serialize review");
    assert!(value.get("persistent_burden_cutpoint_probe").is_none());
}

#[test]
fn persistent_burden_probe_serializes_cap_and_typed_conclusion() {
    let probe = CombatCasePersistentBurdenCutpointProbeV1::Probed {
        source_review: "lane".to_string(),
        projection_trust: "combat_case_projection_v1".to_string(),
        retained_candidate_count: 17,
        unique_candidate_count: 17,
        dirty_candidate_count: 17,
        candidates_with_cutpoint: 17,
        unique_cutpoint_count: 17,
        examined_cutpoint_count: 16,
        cutpoint_limit: PERSISTENT_BURDEN_CUTPOINT_LIMIT_V1,
        cutpoint_limit_hit: true,
        omitted_cutpoint_count: 1,
        replay_failures: Vec::new(),
        aggregate: PersistentBurdenCutpointAggregateV1::default(),
        cutpoints: Vec::new(),
        conclusion: PersistentBurdenCutpointConclusionV1::NoOneActionEscapeObserved,
    };
    let value = serde_json::to_value(review(None, Some(probe))).expect("serialize review");
    assert_eq!(
        value["persistent_burden_cutpoint_probe"]["status"],
        "probed"
    );
    assert_eq!(
        value["persistent_burden_cutpoint_probe"]["conclusion"],
        "no_one_action_escape_observed"
    );
    assert_eq!(value["persistent_burden_cutpoint_probe"]["cutpoint_limit"], 16);
    assert_eq!(value["persistent_burden_cutpoint_probe"]["cutpoint_limit_hit"], true);
    assert_eq!(value["persistent_burden_cutpoint_probe"]["omitted_cutpoint_count"], 1);
}

#[test]
fn persistent_burden_probe_attaches_only_to_matching_review_label() {
    let mut matching = review(None, None);
    let mut other = review(None, None);
    other.label = "other";
    let probe = CombatCasePersistentBurdenCutpointProbeV1::ProjectionFailed {
        source_review: "lane".to_string(),
        error: "fixture".to_string(),
    };

    assert!(matching.attach_persistent_burden_cutpoint_probe(probe.clone()));
    assert!(!other.attach_persistent_burden_cutpoint_probe(probe));
    assert!(matching.persistent_burden_cutpoint_probe.is_some());
    assert!(other.persistent_burden_cutpoint_probe.is_none());
}
```

In `adjudication_probe.rs`, add a disabled-path test:

```rust
#[test]
fn disabled_persistent_burden_probe_is_absent() {
    assert_eq!(run_persistent_burden_cutpoint_probes(false, &[], None), None);
}
```

- [ ] **Step 2: Run the CLI tests and confirm the red state**

Run:

```powershell
cargo test --bin combat_case_review persistent_burden_probe
```

Expected: FAIL because the field, attach method, and runner do not exist.

- [ ] **Step 3: Add the optional field and label-safe attachment**

In `SearchReview` add beside the candidate census:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub(super) persistent_burden_cutpoint_probe:
    Option<CombatCasePersistentBurdenCutpointProbeV1>,
```

Add:

```rust
pub(super) fn attach_persistent_burden_cutpoint_probe(
    &mut self,
    probe: CombatCasePersistentBurdenCutpointProbeV1,
) -> bool {
    if self.label != probe.source_review() {
        return false;
    }
    self.persistent_burden_cutpoint_probe = Some(probe);
    true
}
```

Initialize the field to `None` in `search_review.rs` and every test constructor.

- [ ] **Step 4: Add the adjudication-gated runner**

In `adjudication_probe.rs` add:

```rust
pub(super) fn run_persistent_burden_cutpoint_probes(
    enabled: bool,
    runs: &[ReviewAdjudicationRun],
    case: Option<&CombatCase>,
) -> Option<Vec<CombatCasePersistentBurdenCutpointProbeV1>> {
    if !enabled {
        return None;
    }
    Some(
        runs.iter()
            .map(|run| match case {
                Some(case) => probe_combat_case_persistent_burden_cutpoints_v1(
                    run.source_review,
                    case,
                    &run.config,
                    &run.report,
                ),
                None => CombatCasePersistentBurdenCutpointProbeV1::ProjectionFailed {
                    source_review: run.source_review.to_string(),
                    error: "combat case unavailable".to_string(),
                },
            })
            .collect(),
    )
}
```

This function must not inspect trajectories, deck cards, monster types, move ids, or run deltas.

- [ ] **Step 5: Attach results in the review pipeline**

Immediately after candidate census attachment in `review_pipeline.rs`, add:

```rust
if let Some(probes) = run_persistent_burden_cutpoint_probes(
    options.adjudicate,
    &adjudication_runs,
    Some(&case),
) {
    for probe in probes {
        let attached = ladder.iter_mut().any(|review| {
            review.attach_persistent_burden_cutpoint_probe(probe.clone())
        });
        debug_assert!(attached, "persistent burden probe must match one ladder row");
    }
}
```

Import the runner beside `run_candidate_censuses`. Do not add a separate CLI flag: `--adjudicate` owns both diagnostics.

- [ ] **Step 6: Strengthen the architecture boundary**

Extend `combat_line_adjudication_has_one_production_owner` in `tests/architecture_runtime_boundaries.rs` so `src/bin/combat_case_review/adjudication_probe.rs`, `review_pipeline.rs`, and `search_types.rs` are jointly checked for these forbidden semantic strings:

```rust
for forbidden in [
    "meta_changes",
    "CardType::Curse",
    "master_deck_curse_count",
    "WrithingMass",
    "Parasite",
    "planned_move_id",
    "run_combat_search_v2",
] {
    assert!(
        !review_adapter.contains(forbidden),
        "combat_case_review adapters must not own `{forbidden}` semantics"
    );
}
```

Build `review_adapter` by concatenating only those three CLI adapter source files. The test intentionally permits typed field names such as `before_plan_id` in run-control output types while forbidding the CLI from calling combat mechanics.

- [ ] **Step 7: Run binary and architecture tests**

Run:

```powershell
cargo test --bin combat_case_review
cargo test --test architecture_runtime_boundaries
```

Expected: both commands PASS; absent fields remain absent without adjudication, and the CLI source audit finds no forbidden semantic ownership.

- [ ] **Step 8: Commit the CLI integration**

```powershell
git add src/bin/combat_case_review/adjudication_probe.rs src/bin/combat_case_review/review_pipeline.rs src/bin/combat_case_review/search_types.rs src/bin/combat_case_review/search_review.rs tests/architecture_runtime_boundaries.rs
git commit -m "feat: expose persistent burden cutpoint probe"
```

### Task 5: Verify the Saved Writhing Mass Case and the Full Boundary

**Files:**
- Runtime input only: `target/bounded-mainline-20260712002/combat_cases/seed20260712002_g34_b0034_a3f42_writhingmass.json`
- Generated, ignored output: `artifacts/runs/writhingmass-burden-cutpoints-20260713.json`
- No tracked file changes expected.

**Interfaces:**
- Consumes: the completed library and CLI diagnostic.
- Produces: one bounded real-case observation and final verification evidence; no committed artifact.

- [ ] **Step 1: Run the same bounded fast/slow ladder once**

Run:

```powershell
cargo run --profile fast-run --bin combat_case_review -- --case target/bounded-mainline-20260712002/combat_cases/seed20260712002_g34_b0034_a3f42_writhingmass.json --adjudicate --fast-nodes 200000 --fast-ms 2000 --slow-nodes 300000 --slow-ms 5000 --write-review artifacts/runs/writhingmass-burden-cutpoints-20260713.json
```

Expected: exit code 0 and the output path printed once. This is the only new ladder run in the plan.

- [ ] **Step 2: Print a compact cutpoint summary without rerunning search**

Run:

```powershell
$review = Get-Content -Raw artifacts/runs/writhingmass-burden-cutpoints-20260713.json | ConvertFrom-Json
$review.ladder | ForEach-Object {
    $probe = $_.persistent_burden_cutpoint_probe
    [pscustomobject]@{
        label = $_.label
        status = $probe.status
        conclusion = $probe.conclusion
        dirty_candidates = $probe.dirty_candidate_count
        unique_cutpoints = $probe.unique_cutpoint_count
        examined_cutpoints = $probe.examined_cutpoint_count
        clean_wins = $probe.aggregate.clean_terminal_win_count
        plan_changes = $probe.aggregate.living_enemy_plan_change_count
        burden_triggers = $probe.aggregate.burden_trigger_count
        failures = $probe.aggregate.input_failure_count
    }
} | Format-Table -AutoSize
```

Expected: each adjudicated ladder row has a typed probe. Report observed counts as evidence only; do not describe `no_one_action_escape_observed` as proof that no clean combat line exists.

- [ ] **Step 3: Audit production source for forbidden specialization and extra search**

Run:

```powershell
rg -n "WrithingMass|Parasite|Implant|run_combat_search_v2" src/eval/run_control/persistent_burden_cutpoint_probe.rs src/eval/run_control/persistent_burden_cutpoint_probe/cutpoint.rs src/eval/run_control/persistent_burden_cutpoint_probe/outcomes.rs src/bin/combat_case_review/adjudication_probe.rs src/bin/combat_case_review/review_pipeline.rs src/bin/combat_case_review/search_types.rs
```

Expected: no matches. Test-only Writhing Mass fixture references are outside this production-source audit.

- [ ] **Step 4: Run final verification from a clean build graph**

Run:

```powershell
cargo test --lib
cargo test --bin combat_case_review
cargo test --test architecture_runtime_boundaries
git diff --check
git status --short
```

Expected: all tests PASS, `git diff --check` prints nothing, and `git status --short` is empty because the generated artifact is ignored.

- [ ] **Step 5: Record the final handoff facts**

Report the three implementation commits, the exact test commands and results, the two ladder-row probe summaries, and the bounded interpretation. Do not amend or create another commit unless final verification required a tracked fix.
