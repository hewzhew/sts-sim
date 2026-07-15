# Campfire Threat Panel Design

## Goal

Build an offline Campfire threat panel that tests every shuffle-alignable exact
Campfire candidate against the relevant public encounter families under one
fixed search contract. The panel separates the candidate's actual post-Campfire
resources from its combat-capability change, so a healing delta cannot hide an
upgrade delta and one named encounter cannot stand in for the whole threat
domain.

This is a feasibility instrument for the deletion-driven Campfire prospect
migration. It does not select a Campfire action, populate the production
`SurvivalDistribution`, or become a reader in run-control, owner-audit, route
planning, or the current Campfire policy.

## Why The Single-Encounter Pilot Is Insufficient

The reconstructed seed006 Transient pilot established that paired combat starts
and exact replay work. It did not establish a Campfire preference:

- it compared only Rest with one Heavy Blade Smith target;
- it used one encounter whose five-turn damage race has unusual mechanics;
- Rest changed starting HP while Smith changed the deck, so the reported final
  HP combined immediate resources and combat capability;
- all cells exhausted their time budgets, making their replayed winning lines
  valid candidates rather than exhaustive outcome proofs;
- it did not measure later fights, persistent resource use, or upgrade growth
  beyond that encounter.

More samples of the same cell would narrow only one conditional estimate. The
next instrument must broaden the represented threat domain and expose the two
causes of a candidate difference before sequential route continuation is built.

## Considered Approaches

### Deepen the single Transient comparison

Run more shuffle samples or give the two existing cells larger search budgets.
This improves search coverage for one conditional fight but preserves encounter
selection bias, the single-Smith assumption, and the HP-versus-capability
confound. It is rejected as the next architectural step.

### Build a paired public-encounter threat panel

Compile all alignable exact candidates across the public eligible encounter
families implied by the root and visible route room types. Run both an actual
consequence lens and a root-HP capability lens with matched enemy and shuffle
randomness. Preserve raw cells and report stratified paired differences without
collapsing them into one score. This is the selected approach.

### Simulate full route corridors immediately

Carry combat outcomes through rewards, events, shops, routes, and later fights
until the next Campfire or act terminal. This is eventually required for a true
survival-window distribution, but it currently requires a reliable
combat-to-run-state completion boundary and frozen continuation owners. Building
it now would couple the laboratory to run-control and make failure attribution
difficult. It is deferred until the threat panel validates the lower-level
scenario and replay contracts.

## Scope

The first threat panel covers:

- one `CampfireEvaluationBatch` and its immutable evaluation context;
- every legal exact candidate that can share a defined shuffle alignment;
- a diagnostic unchanged-root reference that is never a legal candidate;
- public encounter entries relevant to visible hallway, elite, and boss room
  types;
- two explicitly different observation lenses;
- a fixed Combat Search V2 profile and common budget;
- exact replay of every retained complete trajectory;
- append-only raw cells, resumable execution, and stratified summaries;
- typed scenario, execution, replay, and coverage gaps.

The first threat panel does not:

- simulate consecutive rooms;
- automate card rewards, events, shops, or future Campfires;
- consume hidden encounter queues or live RNG cursors;
- estimate chance Campfire suffixes such as Dig or Dream Catcher;
- invent shuffle alignment for deck-identity-changing Toke candidates;
- rank or select a production Campfire action;
- claim calibrated death probabilities or fill production prospect fields;
- add encounter-name rules to combat search or Campfire policy.

## Ownership And Data Flow

```text
public RunState + CampfireEvaluationSpec
                  |
                  v
        CampfireEvaluationBatch
        (legality + transitions)
                  |
        +---------+----------+
        |                    |
        v                    v
public encounter panel   typed candidate gaps
        |              (chance/deck identity)
        v
candidate x lens x encounter x shuffle cells
        |
        v
fixed Combat Search V2 profile
        |
        v
shared exact-replay adapter
        |
        v
append-only evidence journal
        |
        v
stratified paired summaries
```

The engine remains the owner of Campfire legality, transitions, encounter
construction, card and relic mechanics, and combat stepping. The threat panel
consumes those owners. It must not reimplement their rules.

Combat Lab remains the owner of fixed search-profile compilation, bounded
search outcome classification, and exact trajectory replay semantics. A small
persistence-independent replay adapter is extracted from Combat Lab so both
laboratories validate complete trajectories through the same code. The threat
panel does not fabricate a `ResolvedCombatLabSpecV1` merely to reuse its artifact
store.

## Public Encounter Panel

### Information boundary

The simulator's `RunState.monster_list` and `elite_monster_list` are generated
from hidden RNG and consumed in a fixed order. The production Campfire owner
must not inspect those lists or their RNG cursor. The threat panel therefore
uses public content definitions and visible route room types, not the realized
future queue.

The content layer exposes a pure, immutable eligible-pool description by act
and encounter tier. An entry records:

- `encounter_id`;
- room type or encounter tier;
- public content weight before any hidden roll;
- pool/mechanics version;
- limitations required to interpret the weight.

Encounter tier is not inferred from floor number alone. The experiment contract
either supplies the tier from explicit public encounter-history evidence or
requests a conservative union of every still-plausible tier and records that
limitation. It never reads the length or front of the hidden scheduled queue to
decide whether the next hallway is weak or strong.

The first Act 3 strong hallway pool contains the eight stable content entries
already used by encounter generation. Elite entries are included only when an
elite room is present on at least one covered visible path before the horizon.
A visible boss identity is exact public information and may be included as its
own stratum. An unknown room does not silently become a hallway; its unresolved
room-type contribution remains a typed limitation.

No-repeat constraints that require public encounter history may be added later
through an explicit history observation. In their absence, the panel reports an
unconditioned eligible pool and does not call its weights exact next-encounter
probabilities.

### Analysis RNG boundary

The exact Campfire projection carries the live run's RNG pool because the engine
transition kernel operates on `RunState`. The threat-panel compiler must not use
that hidden pool to realize future combat scenarios. After applying the exact
candidate transition, it replaces the projected RNG pool with a deterministic
analysis pool derived from the experiment schedule, then applies the separately
derived shuffle seed. Every candidate and lens in a matched cell receives the
same analysis streams.

The manifest records the analysis-RNG derivation version and base seed. The live
seed, RNG counters, scheduled encounter queues, and future realized pool order
are not inputs to the public encounter panel. Exact transition facts remain
exact; the generated combat position is an offline sampled scenario rather than
the run's hidden realized future.

### No premature aggregation

Independent encounter cells are conditional threat probes, not a simulated
route corridor. The first summary reports every encounter stratum separately.
It may show content weights as provenance, but it must not multiply independent
single-fight outcomes into a probability of reaching the next Campfire.

## Candidate Coverage

The input candidate set comes only from `CampfireEvaluationBatch`:

- `Exact` projections whose stable deck UUID sequence matches the public root
  are shuffle-alignable;
- Smith remains alignable because upgrading preserves card identity and order;
- Rest, Lift, and Recall remain alignable when their projections are exact;
- `Chance` projections record `ChanceOutcomeNotMaterialized`;
- `ChanceThenDecision` projections record
  `PostRevealRecourseNotMaterialized`;
- deck-identity changes record `DeckIdentityChanged` until an explicit causal
  alignment contract exists.

Every alignable exact candidate receives the same cheap-pass scenario and
shuffle schedule. There is no heuristic top-K Smith target. Later deepening may
allocate more search budget to overlapping or coverage-limited cells, but it
cannot erase the cheap-pass evidence or label an unevaluated candidate as
dominated.

The panel also compiles an `UnchangedRoot` diagnostic reference. It is clearly
typed as a reference rather than `CampfireCandidate` and can never reach a
production selector or executor.

## Two Observation Lenses

### Actual consequence lens

`ActualConsequence` preserves the exact projected state. Rest starts combat
with its real healed HP; Smith starts with its real upgraded card; Lift carries
its real strength state; Recall carries its real key state. This lens answers:

> After taking this action, what executable combat outcomes did the fixed
> search contract find for this threat and matched random scenario?

Its final HP and potion use describe the combined immediate and capability
consequence for that one cell.

### Root-HP capability lens

`RootHpCapability` preserves the candidate's exact non-HP transition but resets
only current HP to the public root current HP before combat construction. It
does not normalize the deck, relic counters, keys, max HP, potions, gold, or any
other resource. This deliberately synthetic lens answers:

> Holding current HP fixed, how did the candidate's mechanical change alter
> this threat response?

For ordinary Rest without Dream Catcher, this lens matches the unchanged-root
reference. For Smith it isolates the upgraded card under the same current HP.
The lens is diagnostic and must never be described as a legal campaign
trajectory.

If a future Campfire mechanic changes both current HP and another persistent
resource, only current HP is reset and the retained differences remain visible.
The panel must not broaden this into an undocumented “resource normalization.”

## Scenario Alignment

A cell identity includes:

- evaluation-context fingerprint;
- diagnostic-reference or Campfire candidate identity;
- lens;
- encounter entry and room type;
- enemy/non-shuffle analysis seed;
- shuffle sample index and derived shuffle seed;
- search profile hash and common-budget hash;
- source and mechanics identity.

Within one encounter and sample, candidates and lenses share the same analysis
enemy RNG and shuffle seed. The compiler verifies aligned enemy identities,
initial monster states, and UUID draw order wherever deck identity is preserved.
Differences outside the declared candidate transition and lens transform are an
isolation error, not outcome evidence.

The panel remains `ExactStateOracle`: combat search may inspect the full sampled
combat position. This is offline feasibility evidence and not a claim about the
information available to a human or production Campfire decision.

## Search, Replay, And Cell Semantics

Each cell runs one frozen Combat Search V2 profile with the common experiment
budget. Execution happens sequentially inside one process so Cargo is not
invoked or linked per cell.

Every non-estimated complete trajectory is converted to one witness and exact
replayed. The shared replay adapter verifies legality, terminal state, and final
HP. A mismatch is a halting execution error for the experiment contract.

Each cell records at least:

- candidate/reference, lens, encounter, sample, and fingerprints;
- starting HP and public content weight provenance;
- search coverage status and selected terminal;
- replay validation status;
- the replayed complete candidate, even when search coverage is limited;
- final HP, HP loss, turns, actions, cards played, and potions used for replayed
  candidates;
- expanded/generated nodes and budget/deadline exhaustion;
- typed construction, search, or exact-replay error.

Outcome classification retains Combat Lab semantics:

- exhaustive or accepted complete wins and losses may be resolved;
- node-limited, time-limited, frontier-open, or unresolved cells are
  `CoverageLimited`;
- a replayed win inside a coverage-limited cell remains a valid found candidate
  but not a resolved win-rate observation;
- execution failures never become losses.

## Artifact And Resume Contract

The threat panel writes under `artifacts/runs`:

- `manifest.json` for the immutable resolved experiment contract;
- `cells.jsonl` as the append-only raw evidence authority;
- `checkpoint.json` as a rebuildable resume accelerator;
- `summary.json` derived only from the manifest and journal.

Rerunning with the same target resumes missing cells. Increasing the sample
target extends the deterministic schedule. Changing candidates, lenses,
encounter panel, profile, budget, source identity, or mechanics version rejects
resume rather than mixing evidence.

## Summary Contract

The primary summary is stratified by:

```text
lens -> encounter -> candidate/reference
```

For each stratum it reports:

- requested, completed, resolved, coverage-limited, and error counts;
- replayed complete/win/loss candidate counts;
- terminal-HP and HP-loss distribution summaries for replayed candidates;
- turns and potion-use summaries;
- search-cost and exhaustion summaries.

Matched cells produce paired candidate differences for final HP, HP loss,
turns, and potions. Pair summaries keep incomplete/coverage-limited pairs
visible. The report explicitly lists threat strata where the direction of a
paired difference reverses.

The first version does not produce a universal scalar score, a global action
ranking, an averaged “Act 3 pressure” value, or a production decision. Content
weights are displayed beside strata but are not used to fabricate corridor
survival probabilities.

## Bounded Execution Strategy

The initial cheap pass uses a small explicit shuffle target and a short common
wall budget across all alignable candidates and public encounter strata. Its
purpose is coverage and failure-mode discovery, not final precision.

After the cheap pass:

- construction or replay errors block further interpretation;
- zero complete candidates identify search or mechanics gaps;
- ordering reversals identify encounter-conditioned capability differences;
- overlapping results and budget-limited cells may receive a uniformly defined
  deeper tier;
- no deep tier may retroactively relabel cheap-pass gaps as losses.

The journal makes both tiers resumable. One optimized diagnostic binary is
built once; cells do not trigger Rust test relinking.

## Error And Gap Behavior

- A legal exact projection that fails to compile is an integration error.
- A public encounter entry that cannot build a natural start is a mechanics or
  scenario-construction error.
- A cross-candidate alignment mismatch is an isolation error.
- An estimated or mismatching complete line is an exact-replay error.
- Chance suffixes and deck-identity changes are typed gaps, not zero-valued
  outcomes.
- Partial route coverage limits which room-type claims are universal; it does
  not erase observed encounter strata.
- Search coverage limits remain field-local and never imply campaign death.

## Architecture Boundaries

- The module remains under `src/eval` and has no live production reader.
- Live decision layers are guarded from importing the threat panel or its
  artifacts.
- Public encounter-pool extraction belongs with content mechanics and contains
  no Campfire preference.
- Route-window code contributes visible room-type exposure only; it does not
  run combat search or score Campfire actions.
- Combat Lab and the threat panel share replay semantics, not artifact schemas
  or fake resolved contracts.
- The threat panel may not modify Combat Search ordering to improve a named
  candidate or encounter.
- No seed006 assertion or preferred action is added to the regression suite.

## Verification

Use test-driven development to prove:

1. Static public pool extraction never reads `monster_list`,
   `elite_monster_list`, or live RNG streams.
2. Scenario compilation replaces the projected live RNG pool with matched,
   versioned analysis streams before combat construction.
3. Every alignable exact candidate receives every requested encounter, lens,
   and shuffle cell.
4. Chance and deck-identity gaps remain explicit.
5. `ActualConsequence` preserves projected HP while `RootHpCapability` resets
   only current HP.
6. Matched cells share encounter state and shuffle order where alignment is
   declared.
7. Complete trajectories use the same exact-replay adapter as Combat Lab.
8. Coverage-limited replayed wins are retained without becoming resolved wins.
9. Journals resume without repeating completed cells and reject contract drift.
10. Summaries retain encounter strata, paired incompleteness, and direction
   reversals without emitting a global action score.
11. Architecture tests reject imports from live decision layers.

Run formatting, focused threat-panel and Combat Lab tests, the full library
suite, and architecture-boundary tests. Then execute one bounded reconstructed
seed006 pilot and report its provenance and limitations separately from the
regression suite.

## Transition To Sequential Corridors

The threat panel is not the final survival estimator. It earns the next phase
only when:

- scenario construction and exact replay have zero unexplained errors;
- all alignable candidates achieve complete cheap-pass cell coverage;
- the two lenses produce interpretable, reproducible differences;
- search coverage and runtime are measured rather than guessed;
- encounter-stratified output identifies where single-fight conclusions are
  stable or reverse.

The next phase then adds a dedicated combat-completion projection that carries
HP, max HP, relic counters, potions, gold, and other persistent combat changes
back into an offline `RunState`. Only after sequential public corridor samples
exist may the Campfire prospect producer estimate survival to the next
Campfire or act terminal.

When the corridor layer arrives, it consumes the same raw scenario, search, and
replay cell contracts. The threat panel summary remains a diagnostic projection
or is deleted; it does not become a second production decision owner.

## Completion Criteria

This slice is complete when:

1. all public encounter and candidate inputs have explicit provenance;
2. every alignable exact candidate receives identical cheap-pass coverage;
3. actual consequence and root-HP capability are separate typed lenses;
4. all retained complete lines are exact replayed through one shared adapter;
5. coverage-limited, error, and unsupported cases remain distinguishable;
6. summaries are encounter-stratified and expose paired reversals without a
   global score;
7. execution is resumable in one process and does not relink per cell;
8. no live Campfire behavior or production prospect field changes;
9. focused, full-library, and architecture verification passes;
10. a bounded pilot demonstrates whether the broader microscope reveals signal
    beyond the original Transient/Heavy Blade comparison.
