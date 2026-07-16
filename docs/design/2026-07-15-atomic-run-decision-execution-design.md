# Atomic Run Decision Execution and Auto-Run Removal

## Status

Accepted design direction. Two deletion slices are complete: the REPL target,
bookmarks, multi-operation `AutoRun`, command parser/help/dispatcher, raw-command
trace replay, and live trace recorder are physically removed. Run-control
progress is atomic. Owner-audit and decision surfaces carry `RunDecisionAction`
values from candidate enumeration through execution.

Historical schema-v6-through-v15 traces remain available through a data-only
reader for validation and dataset export. Their string command fields are
compatibility data, not executable instructions: the reader imports no parser
or run executor. The typed run-job journal and boundary-visit denominator remain
incomplete. The first `BoundedRunDriver` slice now owns noncombat repetition,
progress-step and wall budgets, terminal stops, and the handoff at a combat
boundary. Owner-audit combat lanes call the combat resolution port directly.

This design was triggered by the first live calibration of the typed planner
capture boundary. It supersedes any plan that preserves `ar`, `AutoRun`, or a
multi-operation `AutoStep` as a production or diagnostic execution surface.

## Calibration Finding

The seed `20260715001` pilot produced four state-changing `SessionTraceV1`
steps:

1. leave the Neow intro;
2. choose a Neow bonus;
3. choose the first map node;
4. resolve the first combat.

Only the explicit Neow choice became a planner behavior event. The route and
combat operations were selected inside an aggregate automation command, so the
command-level recorder could not link them to the typed candidates observed at
their actual boundaries.

The resulting coverage report said that its one captured Neow site was fully
represented. It did not count the missed map boundary in its denominator. That
report was internally consistent but operationally misleading.

This is not a missing trace hook. The execution transaction is at the wrong
level.

## Decision

Every run-level mutation must be represented by one atomic progress step:

```text
observe public boundary
  -> enumerate the complete legal candidate set
  -> ask one policy or human for one candidate id
  -> validate membership and availability
  -> execute exactly one selected candidate
  -> persist one typed transaction
  -> return control to the caller
```

No semantic command may repeat this sequence internally.

The following live concepts will be removed rather than wrapped:

- `RunControlCommand::AutoRun`;
- the `ar`, `auto-run`, `autorun`, and `run-auto` command aliases;
- `apply_auto_run` and `apply_owner_audit_auto_run`;
- `RunControlAutoStepOptions::max_operations`;
- the loop and operation budget inside `apply_guarded_auto_step`;
- aggregate action results that hide several decision boundaries behind one
  selected command.

`n` may remain only as a request for one atomic progress step. It must not mean
"advance until a human boundary" and it must not accept an operation count.

## Typed Execution Boundary

The live boundary should expose types equivalent to the following. Exact Rust
names may change during implementation, but their ownership must not.

```rust
struct RunDecisionBoundary {
    observation: PlannerObservation,
    legal_candidates: LegalCandidateSet,
}

struct RunDecisionSelection {
    candidate_id: String,
    policy: BehaviorPolicyManifest,
    probability: SelectionProbability,
}

struct RunDecisionTransaction {
    boundary: RunDecisionBoundary,
    selection: RunDecisionSelection,
    before: PublicBoundaryFingerprint,
    after: PublicBoundaryFingerprint,
    outcome_delta: PlannerOutcomeDelta,
    provenance: BehaviorProvenance,
}

enum RunProgressStep {
    Decision(RunDecisionTransaction),
    ForcedTransition(ForcedTransitionRecord),
    CombatResolution(CombatResolutionRecord),
    Stop(RunProgressStop),
}
```

The types must preserve these distinctions:

- A real run decision has a complete or explicitly incomplete candidate set
  and one selected member.
- A forced transition is recorded for trajectory continuity but is not emitted
  as a behavior-learning example.
- A combat resolver may commit an executable combat line as one sub-owner
  result. It must not pretend that every combat card action was a run-planner
  candidate.
- A stop is data, not a formatted reason string. Human-readable text is a view
  over a typed stop kind and typed details.

## Ownership

### Legality and observation

The engine-facing boundary owns public observation and complete legal
enumeration. The policy may not construct, filter, truncate, or mutate the
candidate set.

### Selection

A human adapter, the current behavior policy, and a future learned planner all
implement the same role: select one candidate id from the supplied set or
decline with a typed stop. Selection does not mutate the session.

### Execution

The executor validates that the selected id belongs to the exact enumerated
set, is still available, and matches the current boundary fingerprint. It then
performs one mutation and returns the completed typed transaction.

### Repetition and budgets

Repetition belongs to an outer bounded runner, not to `RunControlCommand` or
`RunControlSession::apply_command`:

```text
BoundedRunDriver
  owns max_decisions + wall deadline + stop policy
  repeatedly calls execute_one_progress_step
  persists every returned step before asking for the next one
```

Search-node and search-time budgets remain local to the combat resolver.
Run-level decision budgets do not enter the semantic decision payload.

## Application Shape

The next application boundary is a typed job, not an interactive command
language:

```text
RunJobSpec
  -> BoundedRunDriver
  -> public boundary + complete candidates
  -> policy selects candidate id
  -> atomic executor
  -> append-only RunJournal
  -> reports / datasets / experiments
```

`RunJobSpec` is a Rust contract that may be serialized for reproducibility. It
is generated by automation and is not a command vocabulary a human must learn.
A future process adapter may accept only `--spec <path>` and `--output <path>`;
that adapter owns no policy, loop, rendering, or semantic defaults.

Interactive panels, bookmarks, raw command replay, and terminal-specific help
are not part of this architecture. Counterfactual work starts from typed saved
states and experiment specs rather than a replayed command prefix.

## Process Boundary and Owner-Audit Consequences

`run_play_driver` is retired rather than narrowed. The following capabilities
move independently:

- live run execution moves to `BoundedRunDriver`;
- typed capture moves to `RunJournal`;
- exact replay consumes typed transactions;
- combat snapshots are captured automatically by runtime conditions;
- calibration and counterfactual work use experiment specs;
- bookmarks, terminal panels, and command help are deleted without replacement.

`branch_tiny` and owner-audit must stop calling a run-control auto-run helper.
Their scheduler owns repeated bounded work. In particular, a combat lane should
call the combat resolution port directly against its cloned session and return
one typed combat result. It should not enter a generic loop that might also
claim rewards, select routes, or cross later owner boundaries.

This boundary is now enforced in code: owner choices and routines contain
`RunDecisionAction`, candidate actions expose `executable_action`, and the
owner-audit source tree is guarded against `RunControlCommand`,
`apply_command`, and `executable_command` returning.

## Trace and Replay

The next trace schema records boundary visits before selection. Therefore a
human-required, policy-declined, incomplete, or unresolved boundary still
appears in the coverage denominator.

Successful decision records reference the content-addressed observation and
candidate set already defined by the planner core. Failed selections attach a
typed capture gap to the boundary visit rather than disappearing.

The future journal may replay typed transactions and recorded combat
trajectories. Historical `SessionTraceV1` files are read, validated, and
exported only; no current component parses or re-executes their raw command
strings.

## Coverage Contract

Coverage must use observed decision boundaries as its denominator, not only
successfully finalized behavior events.

For every decision site it reports at least:

- boundary visits;
- complete and incomplete candidate enumerations;
- linked and unlinked selections;
- executed and unexecuted selections;
- outcome attachments by horizon;
- typed representation or execution gaps.

A missing behavior event is therefore visible evidence, not an invisible row.

## Deletion-Driven Migration

The migration is complete only when the old execution path is physically
absent. It proceeds in the following order:

1. **Complete.** Retire the REPL and bookmarks. Add the atomic progress-step result and
   boundary-visit trace record. Extract one loop iteration from
   `apply_guarded_auto_step` without preserving an internal loop in the new
   API.
2. **In progress, ten vertical slices complete.** Ordinary executable input
   candidates now produce `RunDecisionTransactionV1`: the complete before
   boundary and candidate set, canonical selected candidate, selection source,
   typed action, observed result, after boundary, and trace annotations. The
   compatibility `RunProgressOutcome` is projected from that transaction rather
   than owning this path. The routine policy now selects its one automatic
   candidate through the same transaction executor, and aggregate progress
   projection preserves that transaction for a future journal consumer. Route
   policy selections now resolve back to one public candidate id and execute
   through that transaction boundary; the public map surface also enumerates
   legal Wing Boots jumps instead of relying on a hidden input-gate exception.
   The custom `SkipCardReward` and `SingingBowlCardReward` candidates now use the
   same executor. Singing Bowl's two engine inputs are internal forced
   transitions committed as one decision step, and custom composite execution
   is trialed on a cloned session before commit. Owner-visible decisions now
   also preserve their public candidate id through policy selection, branch
   path evidence, and atomic execution. This includes a constrained binding
   from the parameterized `SelectionSubmit` candidate to the owner's typed
   multi-card selection; arbitrary parameterized actions remain rejected.
   Reward policy now selects and executes exactly one public reward candidate
   per call, records a `RewardPolicy` transaction, and returns control before
   considering the next reward. Candidate execution no longer triggers hidden
   follow-up reward claims, and unavailable potion rewards are preserved for an
   explicit public exit instead of being deleted. Empty-campfire exit is now a
   separately typed forced transition: it requires an empty candidate set,
   preserves the decision-step counter, and is not emitted as a behavior
   decision. Committed combat search execution now produces a separate typed
   combat-resolution record for a complete victory, partial turn segment, or
   Smoke Bomb escape. Its internal card and potion inputs preserve the
   run-level decision-step counter, and the complete trajectory is applied to a
   cloned session before one atomic commit. Search rejection and diagnostic-only
   trials produce no combat-resolution record. Decision transactions, forced
   transitions, combat resolutions, and automatic stops now share one ordered
   `RunProgressStepV1` sum type. `RunProgressOutcome` carries only that sequence;
   the four parallel semantic fields have been deleted, and a stop is required
   to be the final step. A successful atomic executor result no longer emits
   the fake `ProgressApplied` stop: it carries exactly one mutation variant,
   while actual boundaries and budgets carry one typed stop. Other non-candidate forced
   transitions still need migration or explicit classification outside the
   candidate-decision contract.
3. **Complete for owner-audit progression.** The outer
   `BoundedRunDriver` now owns bounded noncombat repetition, the progress-step
   budget, wall deadline, terminal stop, and an explicit stop-before-combat
   handoff. Owner-audit uses it for noncombat progression, while combat lanes
   call the direct combat resolution port. Its callback protocol now also owns
   repetition across combat portfolio chunks and owner routines. `runner.rs`
   performs one dispatch at a time and contains no loop, operation counter, or
   separate owner-routine budget; every committed routine or combat chunk
   consumes the same driver-owned progress budget. The driver also owns one
   ordered `RunProgressJournalV1` segment for each bounded drive. Callback
   progress is reported as actual `RunProgressStepV1` records rather than a
   parallel integer count, so journal length is the budget truth. Owner-audit
   branches retain only the most recent segment to avoid duplicating ancestral
   history across every branch; trace and capsule schema v3 serialize that
   typed segment directly instead of flattening it into legacy `auto_steps`.
   Explicit owner selections are prepended to the same segment before the
   following automatic progression. Stops remain outside the committed journal
   because they describe why the drive yielded rather than a state mutation.
   Other branch-level
   generation and process-slice loops remain separate orchestration horizons,
   not run-progress executors.
4. **Complete for live boundary capture.** Planner capture no longer depends on
   command preparation/finalization. `BoundedRunDriver` captures the public
   boundary before each callback and retains selected, yielded, forced, and
   failed outcomes. Explicit owner selections and each freshly enumerated
   single shop transaction use the same typed capture ticket. The former
   branch-only boss-preview bundle executor is retired; no stored purchase
   sequence may cross a decision boundary. Trace/capsule schema v3
   serializes the recent segment, and live coverage counts deduplicated visits
   rather than finalized behavior events. Projection into trajectory-scoped
   behavior events and outcome attachments remains a later planner-data slice,
   not a reason to restore command capture.
5. **Complete.** Delete the command enum, parser, help, dispatcher, raw replay,
   recorder, aliases, command-only panels, and tests that asserted that shell.
6. **Complete for the deleted kernel.** Architecture guards reject the retired
   files, symbols, recorder, and replay executor. A future bounded driver still
   needs its own atomic-loop invariant.

There is no compatibility fallback from the new bounded runner to `AutoRun`.
If a consumer has not migrated, the build should fail until it does.

## Acceptance Criteria

The migration is accepted only when all of the following hold:

1. Production source contains no `RunControlCommand::AutoRun`,
   `apply_auto_run`, `apply_owner_audit_auto_run`, or run-control
   `max_operations`.
2. Cargo metadata contains no `run_play_driver`; bookmark, REPL, parser, help,
   dispatcher, recorder, and raw replay source files are absent.
3. One selected run candidate produces exactly one decision transaction, and
   its id belongs to the recorded candidate set.
4. A bounded driver that performs N mutations persists N separately linked
   records in one ordered committed-progress journal; decisions, forced
   transitions, and combat resolutions are typed variants, while the typed
   yield/stop remains separate from committed mutation history.
5. A visited map or reward boundary whose selection cannot be linked increases
   the coverage denominator and reports a typed gap.
6. Owner-audit combat lanes cannot advance into rewards, routes, or later
   owners while evaluating one combat candidate.
7. New replay does not execute raw command strings.
8. Core stop/control flow uses typed variants rather than policy `reason`
   strings.

## Non-Goals

This migration does not choose a learned model, improve card acquisition,
promote a new production policy, or attempt to make seed006 win. Its purpose is
to make decision evidence faithful enough that those later tasks can be
measured without inheriting the auto-run architecture.
