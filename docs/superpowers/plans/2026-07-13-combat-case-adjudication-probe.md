# Combat Case Adjudication Probe Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in `combat_case_review --adjudicate` probe that replays one bounded complete search line once and reports ordinary versus clean-only run-control adjudication without pretending the saved case is a resumable run.

**Architecture:** A new run-control module projects the persistent combat context stored in `CombatCase`, invokes the existing exact candidate-line observer, and applies both acceptance plugins to the same `CombatLineObservedOutcomeV1`. `combat_case_review` retains the config and full trajectory produced by its existing ladder, selects the trajectory matching review focus, and serializes the library's typed probe result additively.

**Tech Stack:** Rust 2021, serde/serde_json, clap, existing combat-search V2 and run-control APIs, Cargo library and binary tests.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree because it duplicates the large Cargo build.
- Start implementation from a clean status on `agent/combat-case-adjudication-probe`; do not implement directly on `master`.
- Execute inline with `superpowers:executing-plans`; do not dispatch subagents while effort cannot be controlled.
- Never run `cargo clean`.
- Keep the probe opt-in. Existing review JSON is unchanged when `--adjudicate` is absent.
- Reuse `evaluate_combat_candidate_line_outcome` and `CombatLineAcceptancePolicy`; do not inspect `CombatState.meta_changes` in the CLI and do not add another curse counter.
- Project only the combat case's trusted persistent context. Never emit a `RunControlSessionCheckpointV1` or claim the case can resume the run.
- Run one search ladder and one exact replay per selected line. Both policies must consume the same cloned observed outcome.
- Do not change combat search ordering, scoring, budgets, Writhing Mass strategy, owner-audit lane policy, or historical artifacts.
- Write durable experiment evidence under `artifacts/runs`, not under `target`.
- Use focused red/green tests during implementation. Run the complete library, binary, and `architecture_runtime_boundaries` suites only at the final checkpoint.

---

## File Responsibility Map

- Create `src/eval/run_control/combat_case_adjudication.rs`: project a non-resumable session from `CombatCase`, replay one trajectory once, and return the typed dual-policy result.
- Modify `src/eval/run_control/mod.rs`: register the module and export only the probe function/result type.
- Create `src/bin/combat_case_review/adjudication_probe.rs`: retain ladder candidates and select the candidate corresponding to review focus.
- Modify `src/bin/combat_case_review.rs`: register the new binary module.
- Modify `src/bin/combat_case_review/args.rs`: add `--adjudicate`.
- Modify `src/bin/combat_case_review/options.rs`: expose the flag and make it imply the existing bounded ladder.
- Modify `src/bin/combat_case_review/review_pipeline/ladder.rs`: preserve each winning ladder trajectory with the exact config that produced it.
- Modify `src/bin/combat_case_review/review_pipeline.rs`: run the probe after focus selection.
- Modify `src/bin/combat_case_review/case_payload/types.rs` and `case_payload.rs`: add an optional top-level `adjudication_probe` field.

---

### Task 1: Add the run-control combat-case probe API

**Files:**
- Create: `src/eval/run_control/combat_case_adjudication.rs`
- Modify: `src/eval/run_control/mod.rs`

**Interfaces:**
- Consumes: `CombatCase`, `CombatSearchV2Config`, and `CombatSearchV2TrajectoryReport`.
- Produces: `adjudicate_combat_case_line_v1(source_review, case, config, trajectory) -> CombatCaseAdjudicationProbeV1`.
- `CombatCaseAdjudicationProbeV1` is serde-tagged with `no_complete_line`, `projection_failed`, `replay_failed`, and `adjudicated` statuses.

- [ ] **Step 1: Create the implementation branch and verify the starting state**

Run:

```powershell
git switch -c agent/combat-case-adjudication-probe
git status -sb
```

Expected: the branch is `agent/combat-case-adjudication-probe` and the worktree is clean.

- [ ] **Step 2: Write failing projection and shared-outcome tests**

Create `combat_case_adjudication.rs` with a `#[cfg(test)]` module. Use this exact fixture shape:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::relics::{RelicId, RelicState};
    use crate::eval::combat_case::{
        CombatCaseGap, CombatCaseRngSummary, CombatCaseRunSummary, CombatCaseSource,
    };
    use crate::runtime::combat::CombatCard;
    use crate::state::core::EngineState;

    fn projected_case() -> CombatCase {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.player_class = "Ironclad".to_string();
        combat.meta.master_deck_snapshot = vec![CombatCard::new(CardId::Strike, 41)];
        combat.entities.player.current_hp = 37;
        combat.entities.player.max_hp = 61;
        combat.entities.player.gold = 123;
        combat.entities.player.relics = vec![RelicState::new(RelicId::Mango)];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7)), None];
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        CombatCase::new(
            CombatCaseSource {
                seed: 99,
                ascension: 3,
                generation: 4,
                branch_id: 5,
                parent_id: Some(3),
            },
            CombatCaseGap {
                boundary: "Combat".to_string(),
                reason: "test".to_string(),
                search_nodes: 10,
                search_ms: 20,
                rescue_search_nodes: 30,
                rescue_search_ms: 40,
            },
            CombatCaseRunSummary {
                act: 3,
                floor: 42,
                hp: 37,
                max_hp: 61,
                gold: 123,
                deck_size: 1,
                relic_count: 1,
                potion_slots: 2,
            },
            Vec::new(),
            None,
            Vec::new(),
            CombatCaseRngSummary::from_pool(&position.combat.rng.pool),
            position,
        )
    }

    #[test]
    fn projected_session_uses_combat_case_context_without_becoming_checkpoint() {
        let case = projected_case();
        let session = project_combat_case_session(&case).expect("project session");

        assert_eq!(session.run_state.seed, 99);
        assert_eq!(session.run_state.ascension_level, 3);
        assert_eq!(session.run_state.act_num, 3);
        assert_eq!(session.run_state.floor_num, 42);
        assert_eq!(session.run_state.current_hp, 37);
        assert_eq!(session.run_state.max_hp, 61);
        assert_eq!(session.run_state.gold, 123);
        assert_eq!(session.run_state.master_deck, case.position.combat.meta.master_deck_snapshot);
        assert_eq!(session.run_state.relics, case.position.combat.entities.player.relics);
        assert_eq!(session.run_state.potions, case.position.combat.entities.potions);
        assert_eq!(
            session.active_combat.as_ref().map(|active| &active.combat_state),
            Some(&case.position.combat)
        );
    }

    #[test]
    fn dual_policy_results_share_one_observed_dirty_outcome() {
        let outcome = CombatLineObservedOutcomeV1 {
            terminal: CombatTerminal::Win,
            final_hp: 44,
            hp_loss: 0,
            potions_used: 0,
            action_count: 32,
            gold_delta: 0,
            ritual_dagger_growth: 0,
            gained_curses: vec![CardSnapshot {
                id: CardId::Parasite,
                uuid: 9001,
                upgrades: 0,
            }],
        };

        let results = adjudicate_observed_outcome(outcome.clone());

        assert_eq!(results.len(), 2);
        assert!(matches!(
            &results[0],
            CombatLineAdjudicationV1::Accepted {
                policy: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                cleanliness: CombatLineCleanlinessV1::Dirty,
                observed_outcome,
            } if observed_outcome == &outcome
        ));
        assert!(matches!(
            &results[1],
            CombatLineAdjudicationV1::Rejected {
                policy: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
                observed_outcome,
                ..
            } if observed_outcome == &outcome
        ));
    }
}
```

- [ ] **Step 3: Run the focused tests and verify the API is absent**

Run:

```powershell
cargo test --lib projected_session_uses_combat_case_context_without_becoming_checkpoint
cargo test --lib dual_policy_results_share_one_observed_dirty_outcome
```

Expected: compilation fails because `project_combat_case_session` and `adjudicate_observed_outcome` do not exist.

- [ ] **Step 4: Implement projection and dual-policy replay in the new module**

Implement this public result shape and entrypoint:

```rust
pub const COMBAT_CASE_PROJECTION_TRUST_V1: &str = "combat_case_projected_run_context_v1";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CombatCaseAdjudicationProbeV1 {
    NoCompleteLine,
    ProjectionFailed {
        source_review: String,
        error: String,
    },
    ReplayFailed {
        source_review: String,
        projection_trust: String,
        action_count: usize,
        adjudications: Vec<CombatLineAdjudicationV1>,
    },
    Adjudicated {
        source_review: String,
        projection_trust: String,
        action_count: usize,
        observed_outcome: CombatLineObservedOutcomeV1,
        adjudications: Vec<CombatLineAdjudicationV1>,
    },
}

pub fn adjudicate_combat_case_line_v1(
    source_review: impl Into<String>,
    case: &CombatCase,
    config: &CombatSearchV2Config,
    trajectory: &CombatSearchV2TrajectoryReport,
) -> CombatCaseAdjudicationProbeV1 {
    let source_review = source_review.into();
    let session = match project_combat_case_session(case) {
        Ok(session) => session,
        Err(error) => {
            return CombatCaseAdjudicationProbeV1::ProjectionFailed {
                source_review,
                error,
            };
        }
    };
    let line = CombatCandidateLine::from_search_trajectory(trajectory);
    match evaluate_combat_candidate_line_outcome(&session, &case.position, config, line) {
        Ok(evaluation) => {
            let observed_outcome = evaluation.outcome;
            CombatCaseAdjudicationProbeV1::Adjudicated {
                source_review,
                projection_trust: COMBAT_CASE_PROJECTION_TRUST_V1.to_string(),
                action_count: trajectory.actions.len(),
                adjudications: adjudicate_observed_outcome(observed_outcome.clone()),
                observed_outcome,
            }
        }
        Err(error) => CombatCaseAdjudicationProbeV1::ReplayFailed {
            source_review,
            projection_trust: COMBAT_CASE_PROJECTION_TRUST_V1.to_string(),
            action_count: trajectory.actions.len(),
            adjudications: replay_failures(error),
        },
    }
}
```

Use exactly these policies, in this stable order:

```rust
const PROBE_POLICIES: [CombatSearchAcceptancePluginId; 2] = [
    CombatSearchAcceptancePluginId::AcceptedLineOnly,
    CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
];

fn adjudicate_observed_outcome(
    outcome: CombatLineObservedOutcomeV1,
) -> Vec<CombatLineAdjudicationV1> {
    PROBE_POLICIES
        .into_iter()
        .map(|plugin| {
            CombatLineAcceptancePolicy::from_plugin(plugin).adjudicate(outcome.clone())
        })
        .collect()
}

fn replay_failures(error: String) -> Vec<CombatLineAdjudicationV1> {
    PROBE_POLICIES
        .into_iter()
        .map(|policy| CombatLineAdjudicationV1::ReplayFailed {
            policy,
            error: error.clone(),
        })
        .collect()
}
```

`project_combat_case_session` must create `RunControlSession::new` with the case seed, ascension,
and canonical player class, then overwrite only these trusted fields: `act_num`, `floor_num`,
`current_hp`, `max_hp`, `gold`, `master_deck`, `relics`, `potions`, and `rng_pool`. Set both session
and active-combat engine states from `case.position.engine`. Infer `RoomType` only from combat meta:
boss, elite, otherwise hallway. Do not project map state or expose the session publicly.

- [ ] **Step 5: Register and export the narrow API**

In `run_control/mod.rs`, add:

```rust
mod combat_case_adjudication;

pub use combat_case_adjudication::{
    adjudicate_combat_case_line_v1, CombatCaseAdjudicationProbeV1,
    COMBAT_CASE_PROJECTION_TRUST_V1,
};
```

- [ ] **Step 6: Run focused tests and the existing adjudication contract**

Run:

```powershell
cargo fmt --all
cargo test --lib projected_session_uses_combat_case_context_without_becoming_checkpoint
cargo test --lib dual_policy_results_share_one_observed_dirty_outcome
cargo test --lib acceptance_plugins_adjudicate_the_same_dirty_outcome_explicitly
```

Expected: all three commands exit 0 with one matching test each and zero failures.

- [ ] **Step 7: Commit the library boundary**

Run:

```powershell
git add src/eval/run_control/combat_case_adjudication.rs src/eval/run_control/mod.rs
git commit -m "feat: add combat case adjudication probe"
```

---

### Task 2: Integrate the opt-in probe into combat-case review

**Files:**
- Create: `src/bin/combat_case_review/adjudication_probe.rs`
- Modify: `src/bin/combat_case_review.rs`
- Modify: `src/bin/combat_case_review/args.rs`
- Modify: `src/bin/combat_case_review/options.rs`
- Modify: `src/bin/combat_case_review/review_pipeline/ladder.rs`
- Modify: `src/bin/combat_case_review/review_pipeline.rs`
- Modify: `src/bin/combat_case_review/case_payload/types.rs`
- Modify: `src/bin/combat_case_review/case_payload.rs`

**Interfaces:**
- Consumes: Task 1's `adjudicate_combat_case_line_v1` and `CombatCaseAdjudicationProbeV1`.
- Produces: `ReviewAdjudicationCandidate`, `run_adjudication_probe`, and additive review JSON field `adjudication_probe`.

- [ ] **Step 1: Write failing CLI option and serialization tests**

In `args.rs`, add a test that expects the new option:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjudicate_flag_parses() {
        let args = Args::try_parse_from(["combat_case_review", "--case", "case.json", "--adjudicate"])
            .expect("parse adjudicate flag");
        assert!(args.adjudicate);
    }
}
```

In new `adjudication_probe.rs`, add:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabled_probe_without_complete_line_is_typed() {
        assert_eq!(
            run_adjudication_probe(true, &[], None, None),
            Some(CombatCaseAdjudicationProbeV1::NoCompleteLine)
        );
    }

    #[test]
    fn disabled_probe_is_absent_from_review_artifacts() {
        assert_eq!(run_adjudication_probe(false, &[], None, None), None);
    }
}
```

Give `run_adjudication_probe` test-only optional case access by accepting
`case: Option<&CombatCase>`; the no-candidate path must return before unwrapping it.

- [ ] **Step 2: Run tests and verify the CLI surface is absent**

Run:

```powershell
cargo test --bin combat_case_review adjudicate_flag_parses
cargo test --bin combat_case_review enabled_probe_without_complete_line_is_typed
cargo test --bin combat_case_review disabled_probe_is_absent_from_review_artifacts
```

Expected: compilation fails because `Args.adjudicate` and `run_adjudication_probe` do not exist.

- [ ] **Step 3: Add the option and make it imply the bounded ladder**

Add to `Args`:

```rust
#[arg(long, help = "Replay one bounded complete line through ordinary and clean-only run-control adjudication")]
pub(super) adjudicate: bool,
```

Add `pub(super) adjudicate: bool` to `ReviewOptions`, and initialize:

```rust
ladder: args.ladder || args.adjudicate,
adjudicate: args.adjudicate,
```

The existing fast/slow node and wall limits remain the only search budgets.

- [ ] **Step 4: Preserve the exact config and trajectory for every winning ladder review**

Create this type in `adjudication_probe.rs`:

```rust
pub(super) struct ReviewAdjudicationCandidate {
    pub(super) source_review: &'static str,
    pub(super) config: CombatSearchV2Config,
    pub(super) trajectory: CombatSearchV2TrajectoryReport,
}
```

Add `adjudication_candidates: Vec<ReviewAdjudicationCandidate>` to `ReviewLadderRun`. Refactor
`run_ladder_profile` to clone the final config before search and return an optional candidate built
only from `report.best_win_trajectory.clone()`. The no-ladder return initializes an empty vector.

- [ ] **Step 5: Select the focused line and call the library probe**

Implement in `adjudication_probe.rs`:

```rust
pub(super) fn run_adjudication_probe(
    enabled: bool,
    candidates: &[ReviewAdjudicationCandidate],
    focus_label: Option<&str>,
    case: Option<&CombatCase>,
) -> Option<CombatCaseAdjudicationProbeV1> {
    if !enabled {
        return None;
    }
    let candidate = focus_label
        .and_then(|label| {
            candidates
                .iter()
                .find(|candidate| candidate.source_review == label)
        })
        .or_else(|| candidates.first());
    let Some(candidate) = candidate else {
        return Some(CombatCaseAdjudicationProbeV1::NoCompleteLine);
    };
    let Some(case) = case else {
        return Some(CombatCaseAdjudicationProbeV1::ProjectionFailed {
            source_review: candidate.source_review.to_string(),
            error: "combat case unavailable".to_string(),
        });
    };
    Some(adjudicate_combat_case_line_v1(
        candidate.source_review,
        case,
        &candidate.config,
        &candidate.trajectory,
    ))
}
```

Register `mod adjudication_probe;` in `combat_case_review.rs`. In `review_pipeline.rs`, destructure
`ReviewLadderRun`, compute `review_focus`, then call `run_adjudication_probe` before moving `case`
into `assemble_combat_case_review`.

- [ ] **Step 6: Add the optional JSON field**

Add to `CombatCaseReview`:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub(super) adjudication_probe: Option<CombatCaseAdjudicationProbeV1>,
```

Add the same field without serde attributes to `CombatCaseReviewArtifacts`, carry it through the
destructure in `assemble_combat_case_review`, and assign it to the final review. When
`--adjudicate` is absent the field must remain `None` and be omitted from JSON.

- [ ] **Step 7: Run focused binary tests**

Run:

```powershell
cargo fmt --all
cargo test --bin combat_case_review adjudicate_flag_parses
cargo test --bin combat_case_review enabled_probe_without_complete_line_is_typed
cargo test --bin combat_case_review disabled_probe_is_absent_from_review_artifacts
cargo test --bin combat_case_review focus_witness_line_prefers_hidden_full_actions_over_json_preview
```

Expected: all commands exit 0; the new tests each pass once and the existing focus test remains green.

- [ ] **Step 8: Commit the CLI integration**

Run:

```powershell
git add src/bin/combat_case_review.rs src/bin/combat_case_review/args.rs src/bin/combat_case_review/options.rs src/bin/combat_case_review/adjudication_probe.rs src/bin/combat_case_review/review_pipeline.rs src/bin/combat_case_review/review_pipeline/ladder.rs src/bin/combat_case_review/case_payload.rs src/bin/combat_case_review/case_payload/types.rs
git commit -m "feat: expose combat case adjudication review"
```

---

### Task 3: Verify the saved symptom and finish the branch

**Files:**
- Write diagnostic artifact: `artifacts/runs/writhingmass-adjudication-20260713.json`
- Verify only: `target/bounded-mainline-20260712002/combat_cases/seed20260712002_g34_b0034_a3f42_writhingmass.json`

**Interfaces:**
- Consumes: the complete Task 1 and Task 2 implementation.
- Produces: fresh bounded evidence that raw search feasibility and execution policy decisions are reported separately.

- [ ] **Step 1: Run the saved case with bounded adjudication**

Run:

```powershell
New-Item -ItemType Directory -Force artifacts\runs | Out-Null
cargo run --quiet --bin combat_case_review -- --case "target\bounded-mainline-20260712002\combat_cases\seed20260712002_g34_b0034_a3f42_writhingmass.json" --adjudicate --fast-nodes 200000 --fast-ms 2000 --slow-nodes 300000 --slow-ms 5000 --compact --write-review "artifacts\runs\writhingmass-adjudication-20260713.json"
```

Expected: exit 0 and print the artifact path. This is a bounded combat rerun, not a full seed rerun.

- [ ] **Step 2: Inspect the typed result without printing the large artifact**

Run:

```powershell
$review = Get-Content "artifacts\runs\writhingmass-adjudication-20260713.json" -Raw | ConvertFrom-Json
[pscustomobject]@{
  RawCompleteWins = @($review.ladder | Where-Object complete_win).Count
  ProbeStatus = $review.adjudication_probe.status
  SourceReview = $review.adjudication_probe.source_review
  ActionCount = $review.adjudication_probe.action_count
  PolicyResults = @($review.adjudication_probe.adjudications | ForEach-Object {
    "{0}:{1}" -f $_.policy,$_.status
  }) -join ','
  GainedCurses = @($review.adjudication_probe.observed_outcome.gained_curses | ForEach-Object id) -join ','
} | ConvertTo-Json
```

Expected: at least one raw complete win, a non-null typed probe status, one source review, and two
policy results. Record the actual curse/result outcome; do not hard-code that this timed search must
rediscover the historical `Parasite` line.

- [ ] **Step 3: Run completion suites once**

Run:

```powershell
cargo test --lib
cargo test --bin combat_case_review
cargo test --test architecture_runtime_boundaries
```

Expected: all commands exit 0 with zero failures.

- [ ] **Step 4: Audit ownership, formatting, and worktree scope**

Run:

```powershell
rg -n "meta_changes|master_deck_curse_count|reject_dirty_win_status" src/bin/combat_case_review/adjudication_probe.rs src/eval/run_control/combat_case_adjudication.rs
git diff --check
git status --short
```

Expected: `rg` finds none of the prohibited duplicate decision paths, `git diff --check` exits 0,
and only the intended durable artifact is uncommitted before the final commit decision.

- [ ] **Step 5: Commit durable evidence and report without merging**

If the artifact is reasonably sized and contains no transient absolute paths, run:

```powershell
git add artifacts/runs/writhingmass-adjudication-20260713.json
git commit -m "test: record writhing mass adjudication probe"
```

Otherwise leave the artifact untracked and report why. Report the implementation commits, test
counts, raw complete-win count, both policy results, and whether the selected line gained a curse.
Do not merge to `master` or push until the user requests local merge/publish.
