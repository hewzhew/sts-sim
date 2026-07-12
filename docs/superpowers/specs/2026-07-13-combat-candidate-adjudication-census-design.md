# Combat Candidate Adjudication Census Design

## Goal

Explain whether a clean combat victory was present among the winning trajectories that a bounded
`combat_case_review` search already retained. The census is diagnostic evidence only. It must not
change search, candidate selection, acceptance policy, or the run.

## Chosen Approach

Extend the opt-in `--adjudicate` review with a bounded census of the winning candidates already
stored in each ladder report. Exact action traces are deduplicated, replayed through the existing
run-control outcome observer, and classified by the existing ordinary and clean-only acceptance
policies.

Two alternatives are rejected for now:

- Adding curse avoidance to combat-search scoring would couple combat search to run-level policy
  before we know whether search discovery or later selection is responsible.
- Running another search solely for the census would make the diagnostic expensive and would no
  longer describe the evidence produced by the reviewed search.

## Output

When `--adjudicate` is enabled, each ladder row that retained winning candidates may include a
candidate census with:

- retained and unique candidate counts;
- replayed and replay-failed counts;
- clean-accepted and new-curse-rejected counts;
- gained-curse counts grouped by card id;
- the best clean candidate, if one exists, using the existing accepted-outcome preference;
- a typed conclusion distinguishing `clean_candidate_present`, `all_replayed_candidates_dirty`,
  `no_retained_candidates`, and `incomplete_due_to_replay_failures`.

Existing review JSON remains unchanged when `--adjudicate` is absent. The existing single focused
line adjudication remains available and is not reinterpreted as a census.

## Ownership and Data Flow

1. The existing ladder performs its bounded searches once.
2. The CLI passes the retained winning trajectories and their exact search configuration to a
   public run-control diagnostic API.
3. Run-control deduplicates exact action traces, projects the saved case once, and evaluates each
   unique candidate through `evaluate_combat_candidate_line_outcome`.
4. The existing `CombatLineAcceptancePolicy` classifies every observed outcome. The census only
   aggregates typed results; it does not infer curses from combat metadata or duplicate policy
   rules in the CLI.
5. The CLI attaches the per-ladder summaries to the review artifact.

The census is capped by the search report's existing retained-candidate limit. It does not imply
that unretained terminal wins were inspected.

## Error Handling

- A ladder row with no retained winning trajectory reports `no_retained_candidates`.
- Individual replay failures are counted and preserved without discarding successful evaluations.
- If any replay fails and no clean candidate is found, the conclusion is
  `incomplete_due_to_replay_failures`, not `all_replayed_candidates_dirty`.
- Projection failure produces a typed census-level failure and does not suppress the rest of the
  combat review.

## Verification

- A library test supplies duplicate clean and dirty trajectories and proves exact-trace
  deduplication plus ordinary/clean-only classification use the existing owner.
- A library test proves replay failures cannot produce an `all_replayed_candidates_dirty`
  conclusion.
- A CLI test proves census output is absent without `--adjudicate` and additive when enabled.
- Run the saved Writhing Mass case once with the existing bounded ladder. The result must state how
  many retained unique wins are clean versus `Parasite`-gaining without claiming anything about
  the thousands of unretained terminal wins.
- At completion, run the full library, `combat_case_review`, and architecture-boundary suites.

## Non-Goals

- No new search, larger search budget, or full-seed rerun.
- No Writhing Mass action ordering, scoring, or encounter-specific rule.
- No change to production clean-line selection.
- No assumption that all terminal wins were retained or adjudicated.
- No new standalone diagnostic binary or historical artifact rewrite.
