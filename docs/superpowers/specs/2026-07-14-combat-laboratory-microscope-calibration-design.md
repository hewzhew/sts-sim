# Combat Laboratory Microscope Calibration Design

## Purpose

Combat Laboratory V1 is now durable and resumable, but the accepted seed006-derived
`8 x 2` pilot classified all sixteen cells as `coverage_limited`. The raw evidence
shows that this does not mean the search saw no complete outcomes: the `immediate`
profile exact-replayed six winning incumbents and the `lazy_on_pop` profile
exact-replayed one, while every cell exhausted the 3,000 ms wall budget and none
reached the node cap.

This delivery calibrates what the laboratory records before spending more compute.
It preserves exact-replayed incumbent evidence without weakening coverage semantics,
freezes build mode in artifact provenance, and runs one small release-build
feasibility experiment with an explicit any-surviving-win acceptance threshold.

## Non-Goals

- Do not treat a time- or node-limited search as exhaustive.
- Do not relabel a coverage-limited cell as a resolved win merely because it found
  a winning incumbent.
- Do not feed laboratory output into combat, route, run-control, campfire, shop, or
  acquisition policy.
- Do not change either search profile, action ordering, rollout behavior, or combat
  simulator mechanics.
- Do not overwrite, migrate, or append to the accepted
  `artifacts/runs/combat-lab-seed006-pilot` artifact.
- Do not add permanent assertions that either profile wins, retains exact HP, or is
  superior on seed006.
- Do not automatically escalate to a 10-second or larger experiment in this
  delivery; report the 4 x 2 result first.

## Evidence Behind the Direction

The accepted pilot has these observations:

- all sixteen cells: `time_budget_limited`;
- node-budget hits: zero;
- `immediate`: six replay-validated winning incumbents, two replay-validated losing
  incumbents, 5,171 expanded nodes total;
- `lazy_on_pop`: one replay-validated winning incumbent, five replay-validated losing
  incumbents, two cells without a complete incumbent, 16,263 expanded nodes total.

The current acceptance function returns `AcceptedCompleteCandidate` for a win only
when it has zero HP loss without a remaining external-payoff opportunity, or when
`stop_on_win_hp_loss_at_most` is explicitly set and satisfied. The maintained pilot
sets that field to `null`, and the deck contains Feed. Increasing wall time alone
therefore risks producing more deadline-limited cells while continuing to discard
the HP/action evidence of already replayed incumbents.

## Considered Approaches

### Increase wall time only

This is the smallest configuration change, but it does not repair observation loss,
does not prevent debug/release evidence from sharing an artifact, and may still end
at the deadline because nonzero-loss wins have no configured acceptance threshold.
Rejected as the first step.

### Add only an any-win acceptance threshold

Setting the threshold to 87 for an 88 HP start makes any surviving win acceptable.
This would expose feasibility quickly, but unaccepted complete incumbents would still
lose their HP/action evidence and build-mode provenance would remain ambiguous.
Useful as part of the calibration, insufficient alone.

### Preserve incumbents, freeze build identity, then run a small feasibility matrix

This is the selected approach. It keeps coverage and feasibility as separate facts,
makes wall-clock evidence provenance explicit, and limits the first new experiment
to eight cells.

## Raw Cell Evidence

Add an optional nested value to `CombatLabCellRecordV1`:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabReplayedCandidateV1 {
    pub terminal: SearchTerminalLabel,
    pub outcome_order_key: CombatSearchV2OutcomeOrderKeyReport,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub actions: usize,
    pub cards_played: u32,
    pub potions_used: u32,
    pub draw_history: Vec<DomainCardSnapshot>,
    pub action_history: Vec<ClientInput>,
}
```

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub replayed_candidate: Option<CombatLabReplayedCandidateV1>,
```

The field is populated whenever the selected complete trajectory exact-replays and
its terminal is win or loss, regardless of whether search coverage is exhaustive,
accepted, time-limited, or node-limited. It is absent when there is no complete
trajectory, sample construction fails, or exact replay fails.

Candidate presence never changes `outcome_class` or `coverage_status`. The existing
top-level `final_hp`, `hp_loss`, `turns`, action/draw histories, and outcome key retain
their current resolved-only meaning. Resolved cells therefore carry both the nested
candidate and their existing resolved fields; coverage-limited cells carry only the
nested candidate. This intentional duplication keeps the V1 resolved contract stable
and makes the weaker incumbent claim unmistakable.

Exact replay mismatch remains an `ExecutionError`, sets `halt_experiment = true`, and
stores no candidate.

The new optional field is a backward-compatible additive V1 change. Historical cell
JSON without it deserializes as `None`; the accepted pilot remains readable and its
bytes remain untouched.

## Candidate Summary

Add a nested per-profile summary:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatLabCandidateSummaryV1 {
    pub replayed_complete_candidates: usize,
    pub replayed_win_candidates: usize,
    pub replayed_loss_candidates: usize,
    pub replayed_win_rate_all_non_error: Option<f64>,
    pub replayed_win_rate_all_non_error_denominator: usize,
    pub win_hp_loss: CombatLabNumericSummaryV1,
    pub terminal_hp: CombatLabNumericSummaryV1,
    pub turns: CombatLabNumericSummaryV1,
    pub potions_used: CombatLabNumericSummaryV1,
    pub nodes_to_first_win: CombatLabNumericSummaryV1,
}
```

Add `pub candidates: CombatLabCandidateSummaryV1` to
`CombatLabProfileSummaryV1`. Its all-non-error denominator is completed profile cells
minus execution errors, matching the existing conservative denominator convention.
`win_hp_loss` includes only replayed winning candidates; `terminal_hp`, `turns`, and
`potions_used` include replayed winning and losing candidates. `nodes_to_first_win`
uses only cells with a replayed winning candidate and a reported first-win node.
Empty distributions expose count zero and `None` statistics. Existing resolved
summaries and pair/interaction decomposition remain unchanged and continue to
exclude coverage-limited candidates.

Candidate pair comparisons are deliberately deferred. The raw candidate block is
enough to add them later without another replay, while this delivery first establishes
correct per-profile feasibility and cost evidence.

Historical cells without `replayed_candidate` contribute zero candidate evidence;
the summary must never infer missing HP/action evidence from `search_terminal` alone.

## Build Identity

Wall-clock coverage depends on executable build mode. Extend
`CombatLabEnvironmentV1` with backward-compatible optional provenance:

```rust
#[serde(default)]
pub cargo_profile: Option<String>,
#[serde(default)]
pub debug_assertions: Option<bool>,
```

The build script exports Cargo's compile-time `PROFILE` through a stable
`cargo:rustc-env` value. New manifests always store `Some(profile)` and
`Some(cfg!(debug_assertions))`. Resume identity compares both fields exactly.

An old manifest without them remains deserializable and readable, but
`create_or_resume` refuses to append under a new executable because `None` does not
match the current `Some(...)`. The error names the missing build-provenance field.
This intentionally makes the accepted debug pilot historical read-only evidence
rather than silently mixing it with release cells.

No artifact or checkpoint schema number changes: both additions are optional,
backward-readable V1 fields. New artifacts are distinguished by their immutable
environment identity and experiment hash rather than by rewriting historical data.

## Feasibility Fixture and Run

Add a separate maintained lab fixture based on the same start:

- file: `fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json`;
- experiment ID: `seed006_reptomancer_feasibility_release_3s_4x2`;
- scenario ID: `seed006_derived_reptomancer`;
- schedule: unchanged `split_mix64_v1`, seed `20260713006`;
- profiles: unchanged `lazy_on_pop` and `immediate` exact-state oracles;
- common budget: unchanged except
  `stop_on_win_hp_loss_at_most = 87`;
- requested execution target: four samples at the CLI, not in immutable identity.

For the 88 HP start, threshold 87 means any win with at least 1 HP is acceptable.
It measures bounded win discovery, not optimal retained HP. A cell that fails to find
an accepted win remains coverage-limited; it is never converted to a loss.

Run only after implementation, review, all tests, and a clean commit:

```powershell
cargo run --release --bin combat_search_v2_driver -- \
  --lab-spec fixtures/combat_lab/seed006_reptomancer_feasibility_4x2.lab.json \
  --lab-output artifacts/runs/combat-lab-seed006-feasibility-release-3s \
  --lab-samples 4
```

The output directory is new and must not exist before the accepted run. The manifest
must record the final clean Git commit, `cargo_profile = "release"`, and
`debug_assertions = false`.

## Interpretation and Stop Conditions

The calibration succeeds operationally when:

- eight unique cells exist with zero infrastructure errors unless an exact replay
  invariant correctly halts the run;
- candidate summary counts equal the raw journal;
- every resolved win has a replayed winning candidate;
- coverage-limited cells may still expose replayed candidates without entering
  resolved HP or interaction statistics;
- rerunning target four appends zero cells and regenerates byte-identical summary;
- Git status remains clean and the old accepted pilot is byte-unchanged.

No behavioral outcome is required for code acceptance. After the run:

- accepted wins show that 3-second release feasibility is observable;
- replayed wins without acceptance indicate an acceptance/configuration defect;
- no replayed wins plus deadline exhaustion supports a separate 10-second experiment;
- node-budget exhaustion, if observed, supports raising `max_nodes` instead;
- candidate HP distributions may motivate a later, separately designed quality
  threshold experiment.

Do not automatically run any follow-up experiment.

## Error and Compatibility Handling

- Missing build provenance in an old manifest: readable JSON, mutation/resume refused
  with a precise identity mismatch.
- Candidate replay mismatch: execution error, no candidate, durable halt.
- Coverage limit with a valid candidate: preserve candidate, keep coverage-limited.
- No complete candidate: candidate absent, keep existing coverage classification.
- Historical journal without the optional field: deserialize successfully and report
  zero candidate evidence without inference.
- Candidate from another experiment: rejected by existing summary experiment-hash
  validation.

## Testing Strategy

Use strict RED/GREEN cycles for:

1. a time-limited exact-replayed win retaining candidate HP/action/draw evidence while
   resolved-only fields remain absent;
2. a time-limited exact-replayed loss retaining candidate evidence;
3. unresolved/no-trajectory and exact-replay-error cells storing no candidate;
4. resolved cells retaining existing fields and the matching candidate block;
5. candidate summary counts, rates, denominators, empty distributions, and numeric
   statistics while resolved/pair/interaction results remain unchanged;
6. historical cell JSON without the optional field deserializing and regenerating a
   zero-candidate summary;
7. new manifests storing build profile/debug assertions and resume rejecting either
   mismatch before journal mutation;
8. the feasibility fixture resolving with the exact threshold and otherwise identical
   profiles/budget;
9. final focused tests, full library tests, architecture boundaries, driver tests,
   formatting, diff checks, and clean status before the durable release run.

No test asserts seed006 wins, HP, or profile superiority.
