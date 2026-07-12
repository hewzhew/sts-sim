# Combat Line Adjudication Boundary Design

**Date:** 2026-07-12

## Purpose

Combat search can currently find a replayable complete win and still leave the run at a combat
gap because three layers partially own the question of whether that line may execute. Search
reports an accepted complete candidate, run-control applies an older global clean-win rule, and
owner-audit may count curses again after a trial run. Artifacts then mix the search-level result
with the later rejection and omit the concrete deck change that caused it.

The A3F42 Writhing Mass case from seed `20260712002` exposes the boundary failure. All four search
lanes found complete wins. The final lane retained a zero-potion line that ended at 44 HP with no
HP loss, but run-control rejected it because the fight added `Parasite`. The lane profile had an
explicit acceptance plugin, but that plugin was not consumed by run-control.

This delivery creates one explicit combat-line adjudication boundary. A profile supplies the
acceptance policy, run-control replays and adjudicates a candidate once, owner-audit commits or
does not commit that typed result, and artifacts preserve both raw search facts and the final
execution decision.

## Scope

The delivery must:

- make the effective combat search profile the only source of line-acceptance policy;
- distinguish raw search feasibility from permission to execute a candidate;
- adjudicate run-level effects with a complete `RunControlSession`, outside combat search;
- allow ordinary accepted-line profiles to execute a replayable win that gains a curse;
- require clean-only profiles to reject a win that gains a curse;
- remove owner-audit's independent production dirty-win decision;
- persist the selected policy, observed line outcome, cleanliness, and rejection reason;
- keep existing capsules and combat cases readable through additive schema changes;
- preserve search ordering, scoring, repair, potion, and partial-line behavior except where policy
  routing necessarily avoids an irrelevant clean-alternative search.

The delivery must not:

- change how Writhing Mass is played or how Reactive rerolls are valued;
- teach search to avoid `Parasite`;
- change card rewards, deck construction, routing, shops, or campfires;
- move master-deck or run-state knowledge into the combat search core;
- rewrite the whole run-control or owner-audit subsystem;
- add seed-order, exact action-sequence, exact HP, or transient score assertions;
- perform unrelated test or source cleanup.

## Current Failure

The current data flow is:

```text
CombatSearchProfile.acceptance
  -> CombatSearchProfile::to_config() drops acceptance
  -> search reports an accepted complete candidate
  -> run-control constructs CombatLineAcceptancePolicy::default()
  -> the default rejects every newly gained curse
  -> owner-audit may count curses and decide again
  -> summary combines search-level accepted_win with a dirty-win combat gap
```

This is architectural drift rather than a search failure. The acceptance plugin profile was added
after the global run-control rule, but the newer policy identity was never wired into the older
adjudicator.

The responsibilities are also duplicated:

- `ai::combat_search_v2` decides whether a complete combat terminal satisfies search-local stop
  criteria such as HP-loss limits;
- `eval::run_control` replays a selected line and observes run-level side effects;
- `runtime::branch::owner_audit` independently compares curse counts around a trial run;
- capsule projection labels the raw search candidate as accepted even when execution was rejected.

## Considered Approaches

### Only pass the missing field

Run-control could receive the profile's acceptance plugin while retaining the owner-audit curse
guard and existing artifact projection. This is a small patch, but it leaves two production
adjudicators and two inconsistent result vocabularies. A later policy change could drift again.
This approach is rejected.

### Move acceptance into combat search

Combat search could directly classify dirty lines. Curse acquisition is not fully represented by
the combat position: it must be observed against the master deck by replaying through the run
session. Moving that knowledge into search would couple combat-state exploration to run-level
state and make the hot search path more expensive. This approach is rejected.

### One run-control adjudication boundary

The selected approach keeps search responsible for candidate discovery and search-local limits.
Run-control receives the profile policy explicitly, performs the existing exact line replay, and
returns one typed adjudication. Owner-audit consumes that result without reinterpreting it.
Artifacts serialize both layers. This fixes the current case without widening search knowledge or
rewriting unrelated run-control behavior.

## Ownership Boundaries

### Search profile

`CombatSearchProfile` remains the deployable identity for budgets, potion policy, search plugins,
acceptance, and artifact policy. Its `acceptance` field is the only source of run-level
line-acceptance behavior.

An entry point that has no explicit profile must materialize a named effective profile before
search begins. Legacy manual search uses `manual_default` with
`CleanAcceptedLineNoNewCurse`, preserving its current conservative behavior. Downstream code must
not synthesize an unnamed acceptance default.

`CombatSearchProfile::to_config()` may continue to omit acceptance because
`CombatSearchV2Config` configures combat-state search and cannot evaluate master-deck effects.
The caller retains the effective profile and passes its acceptance plugin separately to the
run-control adjudicator.

### Combat search

Combat search reports raw feasibility and quality facts:

- whether a complete candidate was found;
- terminal state, final combat HP, HP loss, potion use, turns, actions, and cards played;
- search coverage, budget, timing, and candidate evidence.

Search-local `accepted_complete_candidate` continues to mean that a complete candidate satisfied
the configured search stop criteria. It does not mean that run-control has authorized execution.
New artifact fields and render labels must make that distinction explicit.

### Run-control adjudicator

Run-control owns exact candidate replay against a cloned `RunControlSession` and observes effects
that combat search does not own:

- newly gained curse cards, including stable card identity;
- gold changes;
- Ritual Dagger growth;
- potion consumption;
- final HP and action count;
- terminal and replay validity.

The selector receives an explicit policy derived from the effective profile. It never calls
`CombatLineAcceptancePolicy::default()`.

### Owner-audit

Owner-audit owns portfolio order and commit policy. It may run a lane in a cloned session, inspect
the returned adjudication, and decide whether that lane's already-adjudicated outcome is
committable. It must not recount curses or change an accepted result into a dirty rejection.

A debug-only consistency assertion may compare the recorded deck delta with the trial session,
but it cannot participate in production branching or status construction.

## Result Model

Search and execution use separate typed results.

```text
CombatSearchResult
  NoCompleteCandidate
  CompleteCandidate {
    line,
    raw_metrics,
    report
  }

CombatLineAdjudication
  Accepted {
    line,
    cleanliness,
    observed_outcome
  }
  Rejected {
    reason,
    observed_outcome
  }
  ReplayFailed {
    error
  }
```

`cleanliness` is either `Clean` or `Dirty`. A dirty accepted result retains structured dirty
effects rather than reducing them to a flag.

The initial rejection vocabulary is intentionally narrow:

```text
CombatLineRejectionReason
  NewCurse { cards }
```

Future resource rules may add typed variants only when a profile consumes them. They must not be
introduced merely as diagnostic labels.

## Acceptance Semantics

| Acceptance plugin | Complete replayable line | Line gains a curse | Clean alternative work |
| --- | --- | --- | --- |
| `AcceptedLineOnly` | Accept | `AcceptedDirty` | None |
| `AcceptedLineOrPrimaryChunk` | Accept; existing chunk behavior remains the fallback | `AcceptedDirty` | None |
| `CleanAcceptedLineNoNewCurse` | Accept when clean | Reject dirty candidate | Search same report, then the existing bounded clean/no-potion alternative path |

The policy does not choose between arbitrary complete lines. Existing complete-line scoring and
repair continue to select the preferred candidate. The adjudicator only determines whether the
selected candidate is executable under the named profile and, for clean-only policy, whether an
existing clean replacement is available.

Ordinary profiles must not spend the clean-alternative rerun budget. This both restores declared
semantics and removes wasted work in fights where every winning line receives an unavoidable
curse.

## Data Flow

```text
RunControlSearchCombatOptions
  -> materialize effective CombatSearchProfile
  -> derive CombatSearchV2Config for search-local behavior
  -> retain CombatSearchAcceptancePluginId for adjudication
  -> run combat search
  -> if no complete candidate: report search failure
  -> repair/select preferred complete candidate
  -> replay candidate through cloned RunControlSession
  -> produce CombatLineAdjudication using explicit policy
  -> execute only Accepted adjudication
  -> return outcome and adjudication to owner-audit
  -> owner-audit applies commit policy without reinterpretation
  -> persist raw search facts and final adjudication
```

State mutation remains transactional. Rejected and replay-failed lines never partially modify the
live session.

## Error Handling

- `NoCompleteCandidate` is a normal search outcome, not a dirty-win rejection.
- A policy rejection leaves the session unchanged and records the exact typed reason.
- Candidate replay failure is an internal consistency error. It must not become an ordinary
  combat gap or fall through to partial execution.
- Missing effective policy is a construction error. Public entry points must materialize a
  profile before invoking search.
- Artifact persistence failure remains explicit; adjudication evidence must not disappear
  silently.
- A future unknown serialized acceptance label must fail descriptively rather than fall back to a
  permissive policy.

## Artifact And Compatibility Contract

Capsule result and summary schemas receive an additive `execution_adjudication` section:

```text
execution_adjudication:
  policy
  status
  cleanliness
  selected_line
  observed_outcome
  rejection_reason
```

`observed_outcome` includes the concrete gained curse cards. The Writhing Mass case therefore
records `Parasite` instead of only `dirty winning line`.

Existing `primary_search.status` and raw search reports remain readable for compatibility. Their
rendering and documentation identify them as search-level facts. Top-level final status derives
from execution adjudication when the new field is present.

Old capsules and combat cases without the additive field continue to deserialize. Consumers
render their execution decision as `legacy_unknown`; they must not infer execution acceptance from
an old search-level `accepted_win` value. No historical artifact is rewritten.

Trace annotations for a rejected candidate preserve:

- profile and acceptance plugin;
- raw candidate summary;
- adjudication status;
- concrete rejection reason and cards;
- observed final HP, HP loss, potion use, and action count.

This evidence survives auto-step aggregation instead of being collapsed to a generic reason.

## Production Deletions

After the new path is wired and verified, the delivery removes:

- the implicit `Default` implementation used to choose production combat-line acceptance;
- owner-audit's `reject_dirty_win_status` production helper;
- pre/post curse counting used only for that second decision;
- duplicate tests that protect the removed owner-audit adjudicator;
- render branches that produce a generic dirty-win reason when structured adjudication is
  available.

It retains:

- exact line replay and run-level outcome observation;
- clean-alternative selection for clean-only profiles;
- lane commit policy;
- game-mechanic and Java parity tests for Writhing Mass and `Parasite`;
- legacy artifact deserialization.

## Stable Test Contract

The delivery adds or reshapes only three behavioral contracts:

1. A pure policy test covers all three acceptance plugin mappings and proves there is no implicit
   fallback policy.
2. One integration-level adjudication fixture replays the same complete line that gains
   `Parasite`: `AcceptedLineOnly` returns `AcceptedDirty`, while
   `CleanAcceptedLineNoNewCurse` returns `Rejected(NewCurse)`.
3. One artifact round-trip proves that policy, final adjudication, and concrete `Parasite`
   evidence survive serialization while a legacy artifact still loads as `legacy_unknown`.

Tests must not lock a particular seed, action order, final HP number, search score, node count, or
wall time. Existing tests for the removed duplicate owner decision are deleted only after the new
adjudication contract covers the same architectural behavior.

## Implementation Phases

### Phase 1: Explicit policy and adjudication types

Introduce policy conversion, typed cleanliness, typed rejection, and typed adjudication. Change
the selector interface to require policy and remove its hidden default. No owner or artifact
behavior changes in this phase.

### Phase 2: Run-control wiring

Materialize an effective profile at the run-control boundary, retain its acceptance plugin beside
the search config, and apply the policy during line selection. Ordinary profiles accept replayable
dirty wins without invoking clean-alternative search. Clean-only behavior remains conservative.

### Phase 3: Owner boundary consolidation

Expose adjudication on `RunControlCommandOutcome`, update owner-audit lane handling to consume it,
and remove its independent curse-count decision. Commit policy remains unchanged.

### Phase 4: Durable diagnostics

Add the execution-adjudication artifact fields, preserve detailed trace evidence through auto-step
aggregation, and distinguish search-level acceptance from execution acceptance in summaries and
review output.

### Phase 5: Verification and bounded cleanup

Run the focused red-green contracts, exercise the saved Writhing Mass combat evidence through the
new adjudication path, then run the repository-required complete library and
`architecture_runtime_boundaries` suites. Remove only the now-orphaned duplicate helpers, tests,
and rendering branches.

No full seed rerun is required to prove this architecture contract. A later bounded mainline run
may determine how far the corrected seed proceeds, but that is behavioral follow-up evidence, not
part of the refactor's correctness proof.

## Verification

The delivery is complete when:

1. no production selector constructs an implicit combat-line acceptance default;
2. every run-control combat search has an explicit effective profile and acceptance plugin;
3. the same curse-gaining fixture is accepted by ordinary policy and rejected by clean-only
   policy;
4. ordinary policy does not run clean-alternative search;
5. owner-audit contains no independent production dirty-win adjudication;
6. rejected or replay-failed lines leave the live session unchanged;
7. new artifacts preserve policy, final adjudication, and `Parasite` details;
8. legacy artifacts load without fabricating an execution decision;
9. focused tests pass;
10. the complete library and `architecture_runtime_boundaries` suites pass;
11. the worktree is clean after bounded commits.

## Future Writhing Mass Strategy Research

After this delivery, a separate investigation may use explicit dirty-line evidence to study:

- when attacking into Reactive can reroll the Mega Debuff;
- the opportunity cost of extending the fight to avoid `Parasite`;
- whether current search ordering discovers clean wins when they exist;
- how Omamori, curse synergies, removal access, current HP, and upcoming path affect the value of
  accepting the curse.

That work must use the adjudication output as evidence. It must not be folded into the acceptance
boundary or encoded as a special-case repair for this seed.
