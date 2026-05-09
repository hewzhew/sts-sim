# AI Combat Kernel Implementation Plan

This plan turns `AI_COMBAT_KERNEL_CONTRACT.md` into maintainable implementation
work. It is intentionally narrow: the goal is a trustworthy combat kernel and
trace boundary, not a trainer, planner, or full-run agent.

## Stop Conditions

Stop and repair the kernel if any of these happen:

- a task, Python adapter, collector, or dataset writer needs raw `CombatState`;
- `engine_ref` appears outside kernel-owned execution state;
- a trace stores `KernelActionDescriptor` instead of `PublicActionDescriptor`;
- a trainable action-selection dataset stores full private state hashes, hidden
  RNG, privileged bundles, debug oracle data, or snapshot payloads;
- replay uses original `ActionId` without checking decision/action-set hashes;
- `KernelAbort` or `ReplayFault` is treated as a trainable truncation;
- terminal metrics require raw state access;
- Python `info`, logs, callbacks, or datasets contain privileged/debug/private
  fields;
- privileged data is accessed without an explicit request capability and
  consumer manifest;
- a test passes only because it uses a fixture parser as runtime behavior.

## Phase -1: Ground One Real Combat State

Purpose: stop designing around abstract nouns. Before the kernel API is
implemented, ground the state model on one complete real combat.

Deliverables:

```text
docs/AI_COMBAT_STATE_V0_SCHEMA.md
one hard-coded complete Ironclad vs Jaw Worm authored combat state
one fixed public action script
one smoke binary that runs the same combat five times
state/hash trace output for each run
```

Rules:

- the authored state must be complete for that combat, not a partial
  `CombatState`;
- no `DecisionFrame` abstraction is required yet;
- no Python wrapper is required yet;
- no training code is allowed;
- every field needed to replay the combat must be named in the schema.

Acceptance:

- same authored state + same RNG + same action script produces identical final
  result five times;
- each run emits the same ordered state/hash sequence;
- missing state required for determinism is added to
  `AI_COMBAT_STATE_V0_SCHEMA.md`;
- the schema explicitly distinguishes combat-level fields from future run-level
  fields.
- `RunCombatSnapshot` is explicitly unsupported until
  `docs/RUN_STATE_SNAPSHOT_SPEC.md` exists.

Forbidden:

- pretending `run_state` exists;
- relying on a fixture parser as runtime;
- manually patching state after combat starts to keep the demo alive;
- writing any model/trainer/collector code.

## Phase 0: Type Skeleton and Compile Boundary

Purpose: create names and ownership boundaries before touching real combat.

Deliverables:

```text
src/ai/combat_kernel/mod.rs
src/ai/combat_kernel/types.rs
src/ai/combat_kernel/hash.rs
src/ai/combat_kernel/public_refs.rs
src/ai/combat_kernel/trace.rs
```

Required types:

```text
CombatOrigin
ReplayProvenance
ReplayCursor
CombatHandle
KernelSession
PublicDecisionFrame
KernelActionDescriptor
PublicActionDescriptor
RecordedActionTrace
ChoiceSpec
ChoiceContext
ChoiceSource
SelectionSemantics
PublicObservation
PublicCombatEvent
PrivilegedDataRequest
PrivilegedDecisionData
PrivilegeManifest
KernelTransition
KernelOutcome
CombatTerminalReport
EpisodeTruncation
KernelAbort
ReplayFault
KernelCallError
OpaqueKernelSnapshotHandle
```

Acceptance:

- code compiles with empty/non-engine implementations;
- types are traceable back to `AI_COMBAT_STATE_V0_SCHEMA.md`;
- `KernelActionDescriptor` is not serializable;
- `PublicActionDescriptor` is serializable;
- `OpaqueKernelSnapshotHandle` contains no byte payload accessible outside the
  kernel module;
- no training, PPO, Gym adapter, or search code is added.

Forbidden:

- wiring to old bot;
- adding reward shaping;
- adding a Python wrapper;
- inventing a fake combat state to make types easy.

## Phase 1: Canonical Serialization and Hashing

Purpose: make replay determinism testable before real gameplay.

Deliverables:

```text
canonical serialization helpers
FullStateHash placeholder interface
PublicObservationHash
ChoiceSpecHash
DescriptorHash
ActionSetHash
DecisionHash
OutcomeHash
ActionTraceHash
```

Rules:

- sorted map keys only;
- no Rust `HashMap` iteration order in hash input;
- no pointer addresses;
- no allocation order;
- no debug-format strings;
- no Python object identity.

Acceptance:

- same public decision serialized twice produces identical hash;
- permuting a map internally does not change hash;
- changing descriptor order changes `ActionSetHash`;
- changing one public descriptor changes `DecisionHash`;
- hashes are not included in tensor/observation encoder APIs.
- full private hashes are written only to non-trainable replay audit artifacts;
- trainable datasets contain at most public observation/decision hashes as
  integrity metadata.

Forbidden:

- using `Debug` formatting as canonical serialization;
- using platform-dependent hashers;
- using hash values as model features.

## Phase 2: Public Ref Lifecycle

Purpose: prevent duplicate cards/monsters from corrupting action identity.

Deliverables:

```text
PublicCardRef
PublicMonsterRef
PublicPotionRef
PublicEntityRef
ref lifecycle documentation in code comments
```

Rules:

- monster refs are stable for the entire combat and never reused;
- potion refs are stable while a potion remains in a slot;
- visible card refs are stable while a concrete visible card is publicly
  trackable;
- card refs entering hidden zones are tombstoned unless the selected observation
  mode legally preserves identity;
- refs must not reveal hidden draw order.

Acceptance:

- duplicate cards in hand produce distinct `PublicCardRef`s;
- duplicate monsters produce distinct `PublicMonsterRef`s;
- killed monster refs are not reused;
- using a potion invalidates or updates the correct potion ref only.

Forbidden:

- identifying visible entities only by card id or monster id;
- reusing refs to avoid allocation;
- deriving hidden order from refs.

## Phase 3: Public and Kernel Action Descriptors

Purpose: split execution handles from traceable public semantics.

Deliverables:

```text
KernelActionDescriptor
PublicActionDescriptor
ActionSemanticKey
ActionConstraints
ChoiceContext
deterministic descriptor ordering
```

Rules:

- `KernelActionDescriptor` owns `OpaqueEngineActionRef`;
- only `PublicActionDescriptor` enters decisions/traces/datasets/Python;
- local `ActionId` is never a learning label;
- descriptor order is canonical;
- descriptor hash excludes local `ActionId`;
- action set hash is ordered descriptor hashes.

Acceptance:

- same state produces identical ordered descriptor hashes across two builds/runs;
- no `engine_ref` appears in public serialized output;
- duplicate card actions are distinguishable by public refs;
- duplicate monster targets are distinguishable by refs and slots.

Forbidden:

- storing `engine_ref` in `RecordedActionTrace`;
- relying on `HashMap` iteration for descriptor order;
- mapping local `ActionId` directly to a global model class.

## Phase 4: Outcome Taxonomy and Transition Shape

Purpose: prevent training/replay systems from confusing terminal, truncation,
abort, replay failure, and invalid action attempts.

Deliverables:

```text
KernelTransition
KernelOutcome
StepRejection
EpisodeTruncation
KernelAbort
ReplayFault
KernelCallError
CombatTerminalReport
FinalPublicCombatSummary
```

Rules:

- `KernelCallError` means no transition was formed;
- `Rejected` means invalid action attempt and no state mutation;
- `EpisodeTruncation` means collector/evaluation horizon only;
- `KernelAbort` means engine/kernel/bridge failure or unsupported boundary;
- `ReplayFault` means replay audit failure;
- `Terminal` always carries `CombatTerminalReport`.

Acceptance:

- stale decision id leaves state/rng hash unchanged;
- invalid action id leaves state/rng hash unchanged;
- `HandleNotFound` returns call error, not transition;
- terminal report exposes final hp, max hp, turn count, hp lost, and terminal
  reason;
- `ExternalProcessLost`, `TickBudgetExceeded`, and replay faults are not
  truncations.

Forbidden:

- `CombatTerminal::Error`;
- treating abort/replay fault as trainable truncation;
- computing metrics by reading raw state after terminal.

## Phase 5: Public Observation Schema

Purpose: make observations typed enough that implementation cannot claim
compliance with vague labels.

Deliverables:

```text
PublicObservation
PublicPlayer
PublicPower
PublicRelic
PublicPotionBelt
PublicCardState
PublicCardInHand
PublicCardZoneObservation
PublicMonster
IntentObservation
VisibleIntent
PublicCombatState
PublicHistory
```

Acceptance:

- Ironclad starter public observation can be encoded without raw state access;
- relic counters and potion slots have typed public representations;
- card rendered damage/block/magic are present for hand cards when visible;
- Defect orbs and Watcher stance have public state slots even if v0 starts with
  Ironclad;
- missing visible intent marks frame not action-selection-trainable and prevents
  default sampler calls.
- privileged decision data is obtainable only through explicit request API;
- default `KernelSession.current_decision` contains public data only.

Forbidden:

- untyped `powers` strings;
- naked `discard_pile`/`exhaust_pile` lists without visibility mode;
- `visible_intent_damage` as the only intent field;
- silently training through missing visible intent.
- embedding privileged data in the default decision frame.

## Phase 6: Public Combat Events

Purpose: let rewards and metrics use transition data without raw state access.

Deliverables:

```text
PublicCombatEvent
CardPlayed
DamageDealt
DamageTaken
CardMoved
EnergySpent
MonsterDied
TurnStarted
TurnEnded
```

Acceptance:

- events are chronological;
- events contain only public information;
- hp lost can be computed from events or terminal report;
- card movement events respect zone visibility;
- privileged events are in a separate optional bundle with manifest.

Forbidden:

- mixing privileged events into `public_events`;
- reconstructing reward metrics by diffing raw engine state;
- logging hidden card identity through public card movement events.

## Phase 7: Opaque Snapshot, Restore, Fork

Purpose: support search and replay without leaking state or aliasing branches.

Deliverables:

```text
OpaqueKernelSnapshotHandle
kernel-owned snapshot storage
restore
fork
fork generation/session identity
```

Rules:

- Python/task code can hold a snapshot handle but cannot inspect payload;
- serializable debug snapshots are kernel-owned, non-trainable artifacts;
- stepping child cannot mutate parent;
- stepping parent cannot mutate child;
- stale decisions from parent are invalid in child and vice versa.

Acceptance:

- snapshot handle has no accessible bytes payload;
- restore returns the same current decision hash;
- fork parent/child diverge independently after different actions;
- original snapshot can be restored after parent/child mutations.

Forbidden:

- passing raw snapshot bytes to Python;
- exposing hidden RNG through snapshot metadata;
- using fork before fork isolation passes.

## Phase 8: AuthoredCombatV0 Jaw Worm Through Real Engine

Purpose: first real engine path with no strategy logic.

Deliverables:

```text
AuthoredCombatV0 starter Ironclad vs Jaw Worm
TurnAction decision
PlayCard descriptors
EndTurn descriptor
PublicObservation for the first turn
KernelTransition for at least one player action
```

Acceptance:

- start returns `KernelSession.current_decision`;
- origin is built from `AI_COMBAT_STATE_V0_SCHEMA.md`;
- action descriptors contain public descriptor hashes;
- no `engine_ref` in printed public output;
- one legal card play transitions through real engine;
- end turn advances through monster turn to next decision or terminal;
- visible intent is not missing.

Forbidden:

- manual half-constructed `CombatState`;
- untyped `run_state`;
- old bot decision calls;
- reward shaping;
- training wrapper.

## Phase 9: Terminal Boundary and No Reward RNG

Purpose: combat terminal metrics must be usable without entering run rewards.

Deliverables:

```text
CombatTerminalReport
FinalPublicCombatSummary
terminal RNG boundary check
```

Acceptance:

- win maps to `CombatTerminalReport.kind = Won`;
- reward screen is not exposed by combat kernel;
- reward-generation RNG is not consumed;
- terminal report states whether combat-end hooks were applied;
- metrics can compute final hp, hp lost, turn count, and monsters alive without
  raw state.

Forbidden:

- treating reward screen as a decision;
- generating card rewards in combat kernel;
- querying raw state for terminal metrics.

## Phase 10: Choice Context Cases

Purpose: prove substate decisions are meaningful, not just named.

Deliverables:

```text
Headbutt-like discard selection
Hologram-like discard selection
Exhume-like exhaust selection
generated-card selection
ordered card choice
```

Acceptance:

- `ChoiceSpec.context` differs for Headbutt-like and Hologram-like effects;
- same candidate card under different operations has different semantic key;
- min/max/any_number/requires_confirm are correct;
- ordered choices record selected_so_far and remaining counts;
- duplicate candidates are disambiguated by public refs or replay faults.

Forbidden:

- `SelectCandidate` without `choice_context`;
- guessing ambiguous replay actions;
- using candidate index alone as identity.

## Phase 11: Python Adapter Safety

Purpose: allow smoke adapters without leaking private data or shaping the kernel.

Deliverables:

```text
public-only JSON/debug print format
optional narrow Gym-compatible smoke adapter
leakage assertions
```

Acceptance:

- observation contains only `PublicObservation`;
- `info` contains no privileged observation, debug oracle, hidden RNG, snapshot
  payload, raw state, or full private state hash;
- logs/callback payloads/dataset records pass the same leakage check;
- fixed `Discrete(N)` adapters are marked temporary and cannot alter kernel
  types.

Forbidden:

- adding CleanRL as a design dependency;
- exposing privileged data for convenience;
- making Python adapter order define action order.

## Phase 12: CombatTask Adapter

Purpose: only after kernel trace integrity exists, add task-level encoding and
reward adapters.

Deliverables:

```text
CombatTask adapter over PublicDecisionFrame
candidate/mask encoding from PublicActionDescriptor
reward from public events + terminal report
metrics from public trace
```

Acceptance:

- no raw `CombatState` imports;
- no `KernelActionDescriptor` or `engine_ref` access;
- no local `ActionId` used as semantic label;
- abort and replay fault samples are not trainable;
- missing visible intent frames are rejected by default collector;
- metrics report win rate, hp lost, truncation, abort, replay fault, leakage,
  stale decision, invalid action.

Forbidden:

- training in the same change that introduces the adapter;
- card-pick conclusions from combat-only tasks;
- baseline continuation as evidence.

## Release Criteria for "Maintainable V0"

V0 is maintainable only when all are true:

```text
1. Contract types compile and are separated by module visibility.
2. Public descriptor snapshots cannot contain engine refs.
3. Replay action matching checks decision/action-set/descriptor hashes.
4. Public refs distinguish duplicate cards and monsters.
5. Outcome taxonomy separates terminal/truncation/rejection/abort/replay fault.
6. Terminal report provides public metrics without raw state.
7. Hashes use canonical serialization.
8. Snapshot/fork isolation passes.
9. Python leakage tests pass.
10. Jaw Worm real-engine path passes without old bot or fixture runtime.
11. Headbutt/Hologram-like choice context differs in trace.
12. Multi-step choice fork/cancel/confirm state is replayable.
13. Pre-combat/run-level choices are rejected before fake combat decisions.
14. Dynamic cost/generated choices update descriptor hashes and context.
15. No trainer is required to validate the kernel.
```

If any release criterion fails, the project is still in kernel construction, not
AI training.
