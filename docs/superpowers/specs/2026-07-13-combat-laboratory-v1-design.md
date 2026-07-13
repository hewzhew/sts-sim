# Combat Laboratory V1 Design

**Date:** 2026-07-13

## Goal

Add a small, resumable offline laboratory that measures how one concrete combat scenario behaves
across controlled shuffle samples and explicitly versioned search profiles. The first version must
separate descriptive evidence about draw-order sensitivity, profile sensitivity, and their
interaction without feeding any result back into route planning, card acquisition, combat policy,
or automatic parameter updates.

The first acceptance experiment is a bounded `8 samples x 2 profiles` study of the seed006
Reptomancer encounter. It should answer whether the observed danger is broadly shared across
shuffle samples, primarily associated with one profile, or concentrated in particular
sample-profile combinations.

## Motivation and Existing Foundation

The project can search and replay an exact combat position, compare several combat-search policy
dimensions, and retain combat cases. It cannot yet answer distributional questions such as:

- how often a concrete deck survives one encounter under controlled shuffle variation;
- the mean, spread, median, and tail of observed HP loss;
- whether two profiles differ consistently on the same samples;
- whether one profile is fragile only on particular draw orders;
- whether a result is a resolved combat outcome or merely a search-coverage limit.

`CombatCase` and `CombatCaptureV1` preserve exact positions after a combat has reached a stable
boundary. `CombatStartSpec` and `build_natural_combat_start` can construct a natural encounter from
deck, relic, potion, HP, encounter, and seed inputs. Existing benchmark and policy-comparison code
already supplies exact trajectory metrics and replay validation. V1 reuses those capabilities; it
does not introduce another combat simulator.

An exact mid-combat capture has already incorporated earlier randomness. In particular, a capture
made after the opening hand was drawn cannot truthfully support opening-shuffle counterfactuals.
V1 therefore uses a combat-start specification for shuffle sampling. Exact captures remain valid
for fixed-position profile comparison, but fixed-position comparison is not presented as a
draw-order distribution.

## Considered Approaches

### Single-profile Monte Carlo

Run the current profile repeatedly under different shuffle seeds. This produces a distribution but
cannot distinguish profile weakness from draw-order sensitivity. It also provides no paired
evidence when a profile changes. This is useful as a minimal smoke mode but insufficient as the
laboratory's primary design.

### Shuffle-sample by profile crossed experiment (selected)

Construct one exact starting position per controlled shuffle sample, clone that same position for
each profile, and run the complete sample-profile matrix. This permits paired comparisons and a
descriptive partition into shuffle, profile, and non-additive interaction components. It directly
matches the questions that motivated the laboratory while remaining small enough for one desktop.

### Automatic fitting and live policy feedback

Fit encounter-risk or policy parameters from the experiment and inject them into route or combat
decisions. This is a future goal, but it would make the measuring system influence the system being
measured before the sampling and profile contracts are trustworthy. V1 deliberately excludes it.

## Product and Ownership Boundary

The laboratory is evaluation infrastructure. Its library implementation belongs under
`src/eval/combat_lab_v1/`. The existing `combat_search_v2_driver` receives the maintained CLI
entrypoint because it already owns whole-combat starts, snapshots, benchmarks, and guidance labs.
No new binary is added, and substantial orchestration does not remain in the driver's `main.rs`.

The laboratory may:

- compile one combat-start scenario under controlled RNG variants;
- invoke existing combat-search entrypoints with explicit profile configurations and common
  budgets;
- exact-replay selected complete trajectories;
- record raw sample-profile evidence;
- aggregate descriptive statistics and paired comparisons;
- resume or extend a deterministic sample schedule within the same immutable experiment contract.

The laboratory may not:

- alter combat-search defaults or choose the mainline run-control profile;
- update route threat, reward, shop, campfire, or acquisition policy;
- train a model or emit promoted parameters;
- treat coverage-limited search as a combat loss;
- mutate arbitrary deck, relic, potion, HP, or enemy factors in V1;
- add enemy-specific behavior to combat search.

## Experiment Contract

`CombatLabSpecV1` names one base scenario, one deterministic shuffle schedule, at least one profile,
and one common resource budget.

### Base scenario

The base scenario references a `CombatStartSpec`. At startup the laboratory resolves and snapshots
the referenced content, validates that the encounter can be built, and fingerprints the resolved
scenario. Relative paths resolve from the laboratory-spec directory.

`CombatStartSpec` is a portable synthetic natural start. Its seed initializes fresh RNG streams; it
does not reproduce counters already consumed during an entire campaign. A scenario transcribed
from seed006 therefore preserves the selected deck, resources, encounter, and laboratory base
seed, but is labeled `seed006_derived` rather than an exact pre-combat replay. Exact campaign RNG
reproduction would require a future capture taken immediately before natural combat construction
and is not inferred from a post-draw `CombatCase`.

The base scenario fixes:

- player class, ascension, current and maximum HP;
- complete master deck, upgrades, and supported per-card state;
- relic identities and counters;
- potion identities and slots;
- encounter and room type;
- the base RNG state used for monster construction, HP, intentions, and all non-shuffle streams.

Unsupported start state is a preflight error. V1 must not silently approximate an unsupported card
field, relic state, potion state, or encounter.

### Shuffle schedule

The experiment contract stores a schedule-generator name, schema version, and schedule seed. A
sample index deterministically produces one shuffle seed. The generator is versioned so that an
implementation change cannot silently reinterpret old sample indices.

For a sample, the laboratory constructs the scenario with the base RNG configuration and replaces
only the shuffle RNG stream before natural combat initialization. Monster composition, monster HP,
initial monster plans, and every non-shuffle RNG stream must remain equal to the base scenario.

The resulting stable start position is compiled once per sample and cloned for every profile. This
is stronger than recompiling independently per profile: both profiles receive byte-equivalent
initial combat state, including the same opening hand and hidden draw pile.

The shared shuffle stream is an initial environmental condition, not a promise that both profiles
will observe the same draw history forever. Draw, discard, exhaust, card generation, and reshuffle
actions can consume or transform state differently. Those later divergences are the profile by
shuffle interaction that the experiment is intended to retain.

### Profile contract and information scope

Each profile has:

- a stable experiment-local ID and human label;
- an information-scope classification;
- a complete serialized combat-search configuration excluding the common budget;
- a canonical configuration hash.

Existing combat search receives the exact `CombatPosition` and may inspect hidden state, including
draw-pile order. Profiles built from it are therefore labeled `exact_state_oracle`. They provide a
search upper bound and controlled search-policy comparison, not a claim about a deployable player
that knows only visible information.

A future `observable_policy` category must receive an explicit observation projection and cannot
be aggregated into the same profile comparison without a report-level warning and separate
grouping. V1 does not implement that category.

Profiles are not permanent source-code obligations. The current tree should retain only profiles
with a current mainline, historical-baseline, or distinct diagnostic role. A redundant or obsolete
profile implementation may be deleted. Historical artifacts retain its ID, serialized
configuration, input fingerprint, Git commit, and results; exact reproduction after deletion uses
the recorded commit rather than compatibility code in current mainline.

The current implementation must never substitute a newer profile when an artifact names an
unavailable historical profile. It may render the historical evidence as readable but not
currently rerunnable.

### Common budget

Node, wall-time, action, rollout, and potion resource limits that define comparison cost live in a
common budget block. Every profile in one experiment receives the same budget. Policy knobs may
differ by profile; comparison resource limits may not.

The frozen manifest records both configured limits and observed budget/deadline exhaustion for
every cell.

## Execution Model

The runner is sequential in V1. It starts one process, compiles each sample once, runs all profiles
for that sample, records the completed cells, then advances to the next sample. It does not invoke
Cargo, relink a test binary, or start a process per matrix cell.

Sample-major order preserves paired evidence under interruption. A target of eight samples means:

```text
sample 0: profile A, profile B, checkpoint
sample 1: profile A, profile B, checkpoint
...
sample 7: profile A, profile B, checkpoint
```

The deterministic schedule is not bounded by the immutable experiment identity. A first run may
request a total target of 8 samples; a later resume may request 16 or 32. Existing sample indices
and cell keys remain identical and are not rerun. Reducing the requested total does not delete
evidence.

V1 exposes no parallel execution option. This limits peak memory, avoids search contention, and
keeps runtime interpretation simple on the user's machine. Parallel scheduling can be designed
later if sequential measurements prove too slow.

## Cell Outcome Contract

Every cell key includes the experiment identity, sample index, derived shuffle seed, profile ID,
profile hash, and common-budget hash. A cell records:

- exact initial-state and relevant RNG fingerprints;
- search terminal label and coverage status;
- exact-replay validation status for any selected complete trajectory;
- start HP, final HP, HP loss, turns, actions, cards played, and potions used when resolved;
- compact actual draw history and key action history from exact replay;
- expanded/generated nodes, nodes to first win, deadline and node-budget flags;
- structured error information when execution violates an invariant.

Aggregation uses three top-level result classes:

- `resolved_win`: an exact-replayed winning trajectory;
- `resolved_loss`: an exact-replayed losing trajectory produced by a non-coverage-limited search,
  or by a future deterministic executor that ran to terminal;
- `coverage_limited`: no resolved outcome may be claimed under the recorded search coverage.

Search coverage and terminal outcome remain separate raw fields. A deadline, node cap, unresolved
trajectory, or missing complete replay cannot be converted into `resolved_loss`. Execution errors
are also separate from all three outcome classes and are excluded from outcome statistics.

## Artifact and Resume Contract

One laboratory run writes a durable directory under `artifacts/runs` containing:

- `manifest.json`: immutable resolved experiment contract, source snapshots and hashes, Git commit,
  schema versions, and environment metadata;
- `cells.jsonl`: append-only raw cell records in completion order;
- `checkpoint.json`: atomically replaced progress index and journal digest;
- `summary.json`: reproducible aggregate derived only from the manifest and cell journal.

The journal is the evidence authority. The checkpoint accelerates resume but cannot override or
invent journal cells. Summary generation is idempotent and may be repeated after any interruption.

Resume validates the immutable scenario, schedule generator and seed, profile set and hashes,
common budget, schema versions, and code identity. A mismatch is rejected with a field-level
explanation. The requested total sample count is an execution bound rather than part of the
immutable identity, so increasing it is the only ordinary extension allowed in the same artifact.

If a process stops after writing some profiles for one sample, resume finishes the missing cells
for that sample before advancing. Aggregation identifies incomplete pairs explicitly and excludes
them from paired comparisons.

## Statistical Summary

All statistics are descriptive estimates over the completed deterministic schedule. V1 makes no
statistical-significance, causal, calibration, or population-accuracy claim.

For each profile, `summary.json` reports:

- requested, completed, resolved, win, loss, coverage-limited, and error counts;
- wins divided by all completed non-error cells and wins divided by resolved cells, with both
  denominators shown;
- win-conditioned HP-loss mean, standard deviation, median, and nearest-rank p90;
- resolved terminal-HP mean, standard deviation, median, and p10, including zero HP for resolved
  deaths;
- turn, potion-use, and search-cost summaries;
- deadline and node-budget exhaustion rates.

For every profile pair, the report uses only shared sample indices and provides:

- outcome-pair counts such as both win, left-only win, right-only win, both loss, and unresolved;
- paired final-HP and HP-loss deltas where both sides have comparable resolved outcomes;
- how often each profile is strictly better under the existing complete-trajectory outcome order;
- the first action divergence and compact draw-history divergence when exact replay supplies them.

When at least two profiles and enough fully resolved paired samples exist, the report performs a
descriptive two-way decomposition of terminal HP into:

- shuffle-sample main effect;
- profile main effect;
- non-additive sample-profile interaction.

The report includes the eligible sample count and the unexplained/interaction share. It does not
label the shares statistically significant. If coverage limits make a balanced resolved matrix
unavailable, decomposition is omitted with a reason rather than filling missing cells or treating
them as deaths.

## Failure Handling

- Invalid or unsupported scenario: fail preflight before writing any result cell.
- Scenario isolation failure: halt after recording the invariant error; do not run further profiles
  against a contaminated sample.
- Exact replay mismatch: record an execution error and halt the experiment, because downstream HP
  statistics would not be trustworthy.
- Search deadline or node cap: record `coverage_limited` and continue; this is expected evidence,
  not an infrastructure failure.
- Interrupted process: preserve the append-only journal and resume from its last valid digest.
- Truncated final journal line: ignore only that incomplete line after validating all earlier
  records, then rerun that exact cell.
- Resume contract mismatch: refuse to merge; require a new artifact directory.
- Missing historical profile implementation: keep the artifact readable and report it as not
  rerunnable from the current commit.

## Testing Strategy

Tests protect experimental integrity rather than temporary combat-policy outcomes.

Focused unit tests prove:

1. sample indices deterministically derive the same shuffle seeds;
2. changing a sample changes the shuffle stream while preserving monster entities, initial plans,
   and non-shuffle RNG fingerprints;
3. every profile receives an identical clone of one sample's initial position;
4. profile execution cannot mutate the position used by another profile;
5. journal and checkpoint resume neither duplicate nor omit cells;
6. increasing the target sample count preserves all prior cell keys;
7. manifest, profile, or budget mismatches reject resume;
8. aggregation distinguishes resolved wins, resolved losses, coverage limits, and errors;
9. paired statistics use only shared eligible sample indices;
10. interaction decomposition is emitted only for a balanced resolved matrix;
11. summary regeneration from the same manifest and journal is byte-stable where timestamps are
    excluded.

A small integration test uses a cheap start specification and a `2 x 2` matrix with tiny budgets to
exercise CLI dispatch, artifact creation, interruption/resume, and summary regeneration in one
process.

Tests must not assert that a named profile wins a particular encounter, retains an exact HP value,
or permanently outperforms another profile. The seed006 Reptomancer `8 x 2` run is durable
experiment evidence, not a permanent behavioral regression test.

Repository completion follows `AGENTS.md`: focused tests during implementation, then formatting,
the full library suite, and `architecture_runtime_boundaries` at the completion checkpoint.

## Resource Limits and Acceptance

The first operator run is sequential and requests eight samples with two profiles. It must not
recompile or relink per cell, and it must write progress after every complete sample row. A stopped
run must resume without repeating completed cells. The same artifact must be extensible to 16 or
32 total samples by changing only the requested execution bound.

Acceptance requires:

- a valid seed006 Reptomancer combat-start specification representing the selected HP, deck,
  relics, potions, and encounter, explicitly labeled as seed006-derived rather than an exact
  campaign-RNG replay;
- two explicitly named `exact_state_oracle` profiles under one common budget;
- sixteen recorded raw cells for the `8 x 2` pilot unless an infrastructure invariant error halts
  the run; coverage-limited cells still count as recorded cells but not as resolved outcomes;
- a summary that exposes sample counts, outcome coverage, HP distributions, paired deltas, and an
  interaction decomposition or a precise reason it is unavailable;
- no source or runtime dependency from route planning, run-control selection, or acquisition policy
  to laboratory output.

## Future Stages

V1 intentionally leaves a path toward more advanced work without implementing it now:

1. add observation-only controller profiles that cannot inspect hidden draw order;
2. add explicitly crossed scenario factors such as HP, potion availability, or one card upgrade;
3. collect transition datasets with observation, action, reward, and provenance contracts suitable
   for offline policy evaluation or reinforcement-learning research;
4. validate encounter-risk estimators on held-out scenarios;
5. only after those checks, design a separate promotion boundary for calibrated route threat.

No future stage is authorized merely because V1 emits an artifact. Each needs its own evidence,
design, validation, and user approval.

## Non-Goals

- no live route-threat update;
- no automatic learning, parameter fitting, or reinforcement-learning training;
- no large sample default or multi-process scheduler;
- no arbitrary deck-construction or run-history branching;
- no resampling of randomness that occurred before a mid-combat capture;
- no claim that exact-state search represents a human-information policy;
- no requirement to retain every historical profile implementation in current source;
- no permanent win/HP assertion for seed006 or any other temporary strategy behavior.
