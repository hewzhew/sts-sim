# Complete Event Owner Coverage Design

## Status

Implemented and verified on 2026-07-10. Event-owner completeness is enforced
by exhaustive `EventId` dispatch and seed-free executable cross-boundary owner
contracts, including real pending-choice, reward, combat, portal, and repeated
event transitions.

## Goal

Every event boundary that the maintained owner-audit mainline can route to an
event owner must produce one deterministic, executable decision. Missing event
policy must become a compile-time omission instead of a failure discovered
after an expensive seed run.

This change completes reliable owner coverage. It does not claim that every
event choice is strategically optimal.

## Problem

`EventId` currently has 53 variants. `Neow` is deliberately routed to the
separate `NeowStart` owner. Of the remaining 52 regular events:

- 37 have explicit arms in `event_room_policy_action`;
- `Ssssserpent` and `SensoryStone` depend on option-local
  `ConservativeAuto` markers;
- 13 have neither an explicit event-owner policy nor a reliable marked
  fallback.

The current fallback scans visible event options for exactly one
`ConservativeAuto` marker. A missing or duplicated marker becomes
`MissingMarkedPolicy` or `AmbiguousMarkedPolicy` only when a run reaches that
event. Discovering the omission through a multi-seed panel costs minutes and
does not protect the next newly added event.

Panel reuse, source-identity matching, frontier continuation, and checkpoint
replay do not solve that ownership gap. They are not part of this design or
its verification strategy.

## Considered Approaches

### 1. Keep the generic marker fallback and mark the missing events

This is a small diff, but the compiler cannot prove that every event has one
marker on every reachable screen. It preserves the same delayed-failure mode.

### 2. Add a generic lowest-risk enabled-option fallback

This would mechanically keep runs moving, but a new or malformed event could
silently receive an unintended choice. It trades a visible coverage failure
for hidden policy drift and is unsuitable for reliability-first work.

### 3. Exhaustive event dispatch with explicit policies (selected)

Use an exhaustive `match EventId` with no wildcard fallback. Keep the 37
working policies, add explicit policies for the remaining 15 regular events,
and explicitly identify `Neow` as owned by `NeowStart`. Remove the obsolete
marker mechanism after no event depends on it.

This makes a newly added `EventId` fail compilation until its owner is
classified.

## Scope

### In scope

- Add explicit event-owner dispatch for:
  - `Ssssserpent`;
  - `GoldenShrine`;
  - `Addict`;
  - `Colosseum`;
  - `KnowingSkull`;
  - `TheJoust`;
  - `SensoryStone`;
  - `AccursedBlacksmith`;
  - `Duplicator`;
  - `FountainOfCurseCleansing`;
  - `GremlinWheelGame`;
  - `Lab`;
  - `NoteForYourself`;
  - `SecretPortal`;
  - `UpgradeShrine`.
- Make regular-event dispatch exhaustive over `EventId`.
- Keep `Neow` routed to and tested against the existing `NeowStart` owner.
- Remove `EventOwnerPolicyKind`, `EventOptionSemantics::owner_policy`, the
  `OwnerPolicy` selector, marker-selection fallback, and the
  missing/ambiguous-marker gap variants.
- Narrow the content event-owner API to event-room option selection. Remove
  its stale `RunPendingChoice` submission path; the maintained mainline already
  routes those states to the dedicated `RunChoice` owner.
- Reuse the existing event-resource budget, route facts, deck-plan facts, and
  deck-mutation compiler where they already express the required decision.
- Add fast owner contracts for every newly explicit policy and all of its
  reachable event screens.

### Out of scope

- Panel execution, reuse, scheduling, or source-identity behavior.
- Frontier and capsule checkpoint behavior.
- Full-seed or multi-seed acceptance runs for event-owner completeness.
- Rewriting the 37 existing policies solely to make their style uniform.
- Perfect expected-value play or full strategic tuning for every event.
- Changes to event mechanics, rewards, RNG parity, or option semantics except
  removal of the unused owner marker field.
- Removing unrelated panel or checkpoint code. The maintained owner path will
  no longer depend on those systems for event coverage, but their deletion is
  a separate decision.

## Architecture

### Exhaustive owner classification

The content event-owner entry point is narrowed to event-room option selection.
Its event dispatch becomes exhaustive:

```text
EventRoom + EventState
  -> exhaustive EventId classification
     -> explicit event policy selector
     -> exactly one enabled visible option
     -> typed owner command

Neow
  -> NeowStart owner (existing separate route)
```

There is no wildcard event-policy fallback. A future `EventId` addition must
update the exhaustive classification before the crate compiles.

An explicit arm may call a shared policy helper, but it may not delegate to a
generic "choose anything enabled" rule. Forced and single-option screens may
use a narrow helper that asserts there is exactly one enabled option.

### Selector contract

Each event-room policy returns a selector that must resolve to exactly one
enabled option from `get_event_options`. Zero matches and multiple matches
remain honest owner gaps because they indicate disagreement between event
mechanics and the explicit policy. They are not replaced with index-zero
fallback behavior.

After an event option opens a pending deck selection, the boundary router must
hand control to the existing `RunChoice` owner. The event source must remain
attached to the pending choice, and the `RunChoice` owner must return exactly
the number of targets requested by the engine. The duplicate deck-mutation
submission path inside the content event owner is removed.

### Marker retirement

After `Ssssserpent` and `SensoryStone` receive explicit selectors, no production
owner needs `EventOwnerPolicyKind`. Removing the marker field also removes a
second, manually synchronized policy representation from event option
semantics.

`WomanInBlue` already has an explicit owner function; its marker writes are
therefore removed without changing its decisions.

## Policy Rules for the Newly Explicit Events

The first complete pass favors survival, deterministic progress, and existing
typed budget facts.

### Forced or nearly forced flows

- `GremlinWheelGame`: spin on the initial screen, then leave. If the wheel
  opens a purge selection, use the existing deck-mutation compiler.
- `Lab`: take the potion reward, allow reward automation to resolve it, then
  leave.
- `Colosseum`: proceed through the mandatory introduction and first combat.
  After the first combat, flee instead of entering the optional second fight.
- `TheJoust`: proceed and choose the lower-variance "bet against owner" line;
  continue through result screens and then leave.

### Free or deck-positive actions

- `GoldenShrine`: desecrate only when an available Omamori charge prevents the
  curse; otherwise take the curse-free gold prayer. Leave on completion
  screens.
- `FountainOfCurseCleansing`: drink when at least one removable curse exists;
  otherwise leave.
- `UpgradeShrine`: upgrade when an enabled upgrade target exists; otherwise
  leave. Target selection uses the deck-mutation compiler.
- `AccursedBlacksmith`: forge when an upgrade target exists. If no upgrade is
  possible, rummage only when Omamori prevents `Pain`; otherwise leave.
- `Duplicator`: duplicate only when the deck-mutation compiler identifies an
  acceptable duplicate target; otherwise leave.
- `NoteForYourself`: advance the introduction. Take the stored card only when
  it is not the known ignorable default and a safe purge target exists;
  otherwise ignore it. The removal uses the existing compiler.

### Resource and risk trades

- `Ssssserpent`: accept the gold only when Omamori prevents `Doubt`; otherwise
  decline. Continue and leave correctly if restoring a state already past the
  first screen.
- `Addict`: rob only when Omamori prevents `Shame`. Otherwise pay 85 gold only
  when the relic can be afforded without consuming a reserved or
  route-breaking gold budget; leave when neither condition holds.
- `KnowingSkull`: advance the introduction. Buy the gold option only while its
  current HP cost remains within the event HP budget and gold gain is not
  blocked; re-evaluate after every repeat. Leave when the next purchase would
  consume reserved HP.
- `SensoryStone`: preserve the existing HP-based focus rule as an explicit
  selector: three cards when 10 HP is safe, two when 5 HP is safe, otherwise
  one. Continue and leave on the surrounding screens.
- `SecretPortal`: decline the early boss portal in this reliability-first
  coverage pass. The optional high-risk shortcut can receive a strategic gate
  later without affecting completeness.

## Error Handling

- Missing `event_state`, a non-event engine state, or a selector that matches
  zero/multiple visible choices remains an explicit owner gap.
- `MissingMarkedPolicy` and `AmbiguousMarkedPolicy` cease to exist because
  marked-policy discovery ceases to exist.
- `Neow` reaching the regular event-owner entry point is reported as a routing
  error that names `NeowStart`; normal boundary routing must prevent it.
- No generic enabled-option fallback is introduced.

## Test Strategy

### Compile-time coverage

- The production `match EventId` has no wildcard arm. Adding a new event
  without owner classification fails compilation.
- A boundary-router test confirms `Neow` goes to `NeowStart` and representative
  regular events go to `Owner::Event`.

### Fast event-owner contracts

For each of the 15 newly explicit regular events:

- construct representative run state for every reachable screen;
- build the real structured event options and decision surface;
- call the production event owner;
- assert that exactly one enabled candidate is selected;
- apply the selected typed command where the transition is local and cheap;
- assert the expected next engine state or screen.

State-dependent branches receive paired tests, including low/high HP,
with/without Omamori, sufficient/insufficient gold reserve, available/missing
deck targets, and removable/no removable curse.

The coverage matrix and its shared fixtures live in a dedicated event-owner
test module rather than enlarging the already broad `owner_policy.rs` test
section. Individual event modules keep only mechanic-specific tests; the new
matrix protects the owner interface and typed boundary handoffs.

### Cross-boundary contracts

- Upgrade, duplicate, note, blacksmith, and wheel purge flows enter
  `RunPendingChoice`, route to `Owner::RunChoice`, and produce the required
  number of legal deck targets.
- Lab and Sensory Stone correctly hand off to reward automation.
- Colosseum's mandatory first fight is executable and its optional second
  fight is declined by the reliability policy.
- Secret Portal is declined without entering boss combat.
- Repeated Knowing Skull decisions eventually leave when HP becomes reserved.

### Verification commands

During red-green work, run only the focused owner-policy and event tests. At
the end run:

```powershell
cargo fmt --check
cargo test --lib content::events::owner_policy
cargo test --lib runtime::branch::owner_audit
cargo test --lib
git diff --check
```

No fixed-seed panel, frontier resume, checkpoint replay, or source-identity
reuse is required for acceptance.

## Implementation Sequence

1. Add failing focused tests for each missing event policy and the exhaustive
   dispatch contract.
2. Add explicit policies for forced and free/deck-positive events.
3. Add explicit policies for resource/risk events using existing typed budget
   facts.
4. Make event dispatch exhaustive and classify `Neow` as separately owned.
5. Convert `Ssssserpent` and `SensoryStone` from markers to explicit selectors.
6. Remove the now-unused marker types, fields, selector arm, gap variants, and
   marker-only tests.
7. Remove content event-owner pending-selection submission and verify that all
   affected transitions route to `RunChoice`.
8. Run focused verification, then the full library suite once.

## Success Criteria

- All 52 regular events have explicit, compiler-enforced owner classification.
- `Neow` remains covered by the separate `NeowStart` owner.
- No regular event can fail with `MissingMarkedPolicy` or
  `AmbiguousMarkedPolicy`; those concepts and their marker representation are
  removed.
- Every added event policy resolves to one enabled executable option on all
  tested reachable screens.
- Deck-selection, reward, and combat handoffs preserve their typed ownership
  boundaries.
- Event-room option selection and pending deck selection have one owner each;
  the content event owner no longer duplicates `RunChoice` execution.
- Focused owner verification is seed-free and suitable for the inner edit
  loop.
- The full library suite passes without running an expensive multi-seed panel.
