# AI Combat Kernel Contract

This document is the engineering contract for real Slay the Spire combat AI work
in this repository.

The kernel is not a bot, not a planner, not a Gym environment, and not a
CleanRL-shaped wrapper. It is the deterministic combat state-machine boundary
used by rollout, replay, search, and training systems.

The canonical public loop is:

```text
CombatOrigin
  -> KernelSession { handle, current_decision: PublicDecisionFrame }
  -> choose one PublicActionDescriptor by local ActionId
  -> KernelTransition
  -> KernelOutcome
```

The canonical replay/audit record is:

```text
PublicDecisionFrame
+ PublicActionDescriptor snapshot
+ RecordedActionTrace
+ KernelTransition
```

The canonical trainable action-selection record is a sanitized public subset:

```text
PublicDecisionFrame
+ PublicActionDescriptor snapshot
+ RecordedActionTrace public fields
+ public_events
+ public terminal/rejection/truncation summary
```

Private engine references, privileged observations, debug oracle data, raw state,
hidden RNG, full private state hashes, and snapshot payloads are not part of
trainable action-selection records. Replay/audit artifacts may contain private
hashes only when marked non-trainable.

## Hard Rules

- There is no opaque `PendingChoice`.
- There is no raw `CombatState` access from task, collector, trainer, Python
  adapter, or dataset code.
- There is no baseline bot continuation inside the kernel.
- There is no reward shaping inside the kernel.
- There is no seed-specific strategy branch.
- There is no fixture parser as the runtime contract.
- There is no `RewardScreen` boundary in `CombatKernel`.
- There is no `CombatTerminal::Error`.
- There is no CleanRL or Gym constraint on kernel shape.
- There is no default emission of privileged or debug data into trainable traces.
- There is no transparent snapshot payload outside kernel-owned storage.

Kernel mechanical type names must not contain algorithm words such as Actor,
Critic, Policy, Value, PPO, CleanRL, Trainer, or RewardShaping.

The kernel may know executable truth. Public data used for action selection may
not.

## Combat Kernel API

```text
trait CombatKernel {
  start(origin: CombatOrigin) -> Result<KernelSession, KernelCallError>

  step(
    handle: CombatHandle,
    decision_id: DecisionId,
    action_id: ActionId,
  ) -> Result<KernelTransition, KernelCallError>

  snapshot(handle: CombatHandle) -> Result<OpaqueKernelSnapshotHandle, KernelCallError>
  restore(snapshot: OpaqueKernelSnapshotHandle) -> Result<KernelSession, KernelCallError>
  fork(handle: CombatHandle) -> Result<KernelSession, KernelCallError>
}
```

`KernelSession` means "attached to the current decision". `start`, `restore`,
and `fork` all return the same shape:

```text
KernelSession {
  handle: CombatHandle,
  current_decision: PublicDecisionFrame,
  replay_identity: ReplayIdentity,
}
```

The current decision field must not be called `first_decision`; restored and
forked sessions resume at the current boundary.

There must not be separate authoritative calls for:

```text
legal_actions(handle)
terminal(handle)
```

The current legal actions live inside `PublicDecisionFrame`. The next decision,
terminal, rejection, abort, replay fault, or truncation lives inside
`KernelTransition`.

## Combat Origin

A combat cannot be started from loose fields and then treated as real. There are
two valid origins:

```text
CombatOrigin::RunSnapshot {
  run_state,
  replay_identity,
}

CombatOrigin::AuthoredCombat {
  spec,
  replay_identity,
  purpose,
}
```

`RunSnapshot` is the parity path. It comes from an actual run state and carries
the replay identity needed to continue deterministically.

`AuthoredCombat` is a probe path. It can test mechanics or build small tasks, but
it is not evidence about real developing-run deck distribution.

`CombatStartSpec`, JSON fixtures, and hand-written deck slices are allowed only
as temporary adapters into `AuthoredCombat`. They are not the AI contract.

## Public Decision Frame

`PublicDecisionFrame` is the only default player-input boundary exposed to
collectors, Python adapters, replay artifacts, and trainable action-selection
records.

```text
PublicDecisionFrame {
  id: DecisionId,
  kind: DecisionKind,
  public_observation: PublicObservation,
  actions: Vec<PublicActionDescriptor>,
  choice: ChoiceSpec,
  schema_version: SchemaVersion,
  public_observation_hash: PublicObservationHash,
  choice_spec_hash: ChoiceSpecHash,
  action_set_hash: ActionSetHash,
  decision_hash: DecisionHash,
}
```

If the engine enters an input state the kernel cannot expose, the result is
`KernelOutcome::Aborted(KernelAbort::UnsupportedBoundary)`, not a trainable
decision and not an episode truncation.

Initial combat decision kinds:

```text
TurnAction
SelectFromHand
SelectFromDrawPile
SelectFromDiscardPile
SelectFromExhaustPile
SelectFromGeneratedCards
ChooseOne
Confirm
OrderCards
Scry
```

`SelectCardReward` is forbidden in `CombatKernel`. Post-combat card rewards are
run-level decisions. Combat-generated choices must use
`SelectFromGeneratedCards`.

## Public and Kernel Action Descriptors

Kernel execution descriptors and public descriptors are different types.

```text
KernelActionDescriptor {
  id: ActionId,
  public: PublicActionDescriptor,
  engine_ref: OpaqueEngineActionRef,
}
```

`KernelActionDescriptor` never leaves kernel-owned execution state.
`engine_ref` must never be serialized into Python adapters, traces, datasets,
logs, or trainable records.

```text
PublicActionDescriptor {
  id: ActionId,
  descriptor_hash: DescriptorHash,
  semantic_key: ActionSemanticKey,
  verb: ActionVerb,
  arguments: PublicActionArguments,
  public_refs: Vec<PublicEntityRef>,
  visible_cost: Option<EnergyCost>,
  constraints: ActionConstraints,
  choice_context: ChoiceContext,
}
```

`ActionId` is a local execution token scoped to one `PublicDecisionFrame`. It is
allowed in public descriptors because it is needed to call `step`, but it must
never be used as a learning label.

Only `PublicActionDescriptor` may be stored as a descriptor snapshot.

### Action Constraints

`ActionConstraints` is not a junk drawer.

```text
ActionConstraints {
  requires_target: bool,
  allowed_target_refs: Vec<PublicTargetRef>,
  requires_energy: Option<i32>,
  choice_constraints_hash: ChoiceSpecHash,
}
```

If `actions` contains only legal actions, disabled reasons belong in
`PublicObservation.cards.hand[].unplayable_reason_public`, not in
`PublicActionDescriptor`.

### Action Semantic Key

`ActionSemanticKey` is stable, serializable action identity for learning and
analysis. It is not sufficient for exact replay by itself.

```text
ActionSemanticKey::PlayHandCard {
  hand_slot_class,
  card_id,
  upgraded,
  target_kind,
  target_slot_class,
}

ActionSemanticKey::UsePotion {
  potion_slot_class,
  potion_id,
  target_kind,
  target_slot_class,
}

ActionSemanticKey::EndTurn

ActionSemanticKey::SelectCandidate {
  decision_kind,
  choice_context,
  source,
  candidate_public_ref,
  candidate_card_id,
  candidate_slot_class,
}

ActionSemanticKey::Confirm { decision_kind, choice_context }
ActionSemanticKey::Cancel { decision_kind, choice_context }
ActionSemanticKey::Skip { decision_kind, choice_context }
```

The same visible card selected under different mechanisms must not share the
same semantic identity. A Headbutt-like "put on top of draw pile" choice and a
Hologram-like "return to hand" choice need different `choice_context` values.

### Recorded Action Trace

Every accepted or rejected action attempt must be recorded as:

```text
RecordedActionTrace {
  decision_id: DecisionId,
  original_action_id: ActionId,
  public_descriptor_snapshot: Option<PublicActionDescriptor>,
  semantic_key: Option<ActionSemanticKey>,
  public_argument_refs: Vec<PublicEntityRef>,
  action_schema_version: SchemaVersion,
  recorded_decision_hash: DecisionHash,
  recorded_action_set_hash: ActionSetHash,
  recorded_descriptor_hash: Option<DescriptorHash>,
}
```

For accepted actions, descriptor snapshot, semantic key, and descriptor hash are
required. For invalid action ids they are absent and the transition must be a
state-preserving rejection.

## Replay Action Rule

The original `ActionId` is never trusted by itself during replay.

Exact replay may use `original_action_id` only if all are true:

```text
current_decision_hash == recorded_decision_hash
current_action_set_hash == recorded_action_set_hash
descriptor_hash(original_action_id) == recorded_descriptor_hash
```

Otherwise replay must fall back to semantic matching:

```text
semantic_key + public_argument_refs + choice_context
```

Zero matches or multiple matches are replay faults. The replay system must not
guess.

## Canonical Action Ordering

Action descriptor order must be deterministic and canonical.

Descriptor order must not depend on:

```text
HashMap iteration order
pointer address
allocation order
Python adapter order
nondeterministic engine object order
UI render order unless that order is explicitly part of public observation
```

Each `PublicDecisionFrame` must include `action_set_hash`, computed from ordered
public descriptor hashes.

## Choice Spec

`DecisionKind` alone is not enough. Every public decision must include constraints
and context:

```text
ChoiceSpec {
  source: ChoiceSource,
  context: ChoiceContext,
  min_select: u8,
  max_select: u8,
  selected_so_far: Vec<CandidateRef>,
  remaining_min: u8,
  remaining_max: u8,
  ordered: bool,
  any_number: bool,
  can_skip: bool,
  can_cancel: bool,
  requires_confirm: bool,
  auto_confirm_when_complete: bool,
  selection_semantics: SelectionSemantics,
}
```

```text
ChoiceSource::Zone {
  zone: CardZone,
  visibility: ZoneVisibility,
}

ChoiceSource::GeneratedCards {
  generation_context: ChoiceCause,
}

ChoiceSource::VirtualCandidates {
  candidate_list_id: CandidateListId,
}

ChoiceSource::Multiple(Vec<ChoiceSource>)
```

```text
ChoiceContext {
  caused_by: ChoiceCause,
  operation: ChoiceOperation,
  destination: Option<CardZoneOrVirtualDestination>,
  selection_role: SelectionRole,
}
```

```text
ChoiceCause::CardEffect { card_id, upgraded, public_card_ref }
ChoiceCause::RelicEffect { relic_id }
ChoiceCause::PotionEffect { potion_id }
ChoiceCause::PowerEffect { power_id }
ChoiceCause::SystemRule
```

```text
ChoiceOperation::MoveCard
ChoiceOperation::CopyCard
ChoiceOperation::TransformCard
ChoiceOperation::ExhaustCard
ChoiceOperation::DiscardCard
ChoiceOperation::PutOnTopOfDrawPile
ChoiceOperation::PutOnBottomOfDrawPile
ChoiceOperation::AddToHand
ChoiceOperation::PlayGeneratedCard
ChoiceOperation::ChooseGeneratedOption
ChoiceOperation::ToggleSelection
ChoiceOperation::ConfirmSelection
```

```text
SelectionSemantics::SelectExactlyOneAndApply
SelectionSemantics::ToggleCandidatesThenConfirm
SelectionSemantics::OrderSelectedCards
SelectionSemantics::ChooseOneGeneratedOption
SelectionSemantics::ScryKeepDiscard
```

Without `ChoiceContext` and `SelectionSemantics`, `SelectCandidate` is ambiguous
and must not be emitted.

## Public Entity Ref Rule

Public refs are part of the public trace contract.

```text
PublicMonsterRef:
  stable for the entire combat
  never reused after monster death or escape
  independent from slot index

PublicPotionRef:
  stable while a potion remains in its slot
  slot index must also be recorded

PublicCardRef:
  stable while a concrete visible card instance remains publicly trackable
  never reused within a combat trace
  entering a hidden zone may tombstone the ref
  entering a hidden zone may preserve the ref only if the chosen observation
  mode keeps that identity legally public
```

Public refs must not reveal hidden draw order.

Duplicate visible cards and duplicate monsters must remain distinguishable by
public refs and slot/class metadata.

## Observations

`PublicObservation` is the only observation allowed for default action selection.

Privileged and debug data are separate side channels:

```text
PrivilegedDecisionData:
  optional mechanically true but non-public data for explicitly declared
  evaluators or value estimators

DebugOracle:
  replay/debug truth, never emitted into trainable records by default
```

Default kernel start/step returns `PublicDecisionFrame`, not a privileged
envelope. Privileged data requires an explicit request capability and manifest.

### Public Observation Schema

```text
PublicObservation {
  player: PublicPlayer,
  relics: Vec<PublicRelic>,
  potions: PublicPotionBelt,
  cards: PublicCardState,
  monsters: Vec<PublicMonster>,
  combat: PublicCombatState,
}
```

```text
PublicPlayer {
  hp,
  max_hp,
  block,
  energy,
  powers: Vec<PublicPower>,
  stance,
  orbs,
  class_specific_public_state,
}

PublicPower {
  power_id,
  amount,
  amount2,
  misc,
  visible_flags,
}

PublicRelic {
  relic_id,
  visible_counters,
  visible_charges,
  visible_disabled_flags,
}

PublicPotionBelt {
  slots: Vec<PublicPotionSlot>,
}

PublicPotionSlot {
  slot_index,
  public_potion_ref,
  potion_id,
  usable,
  target_requirements,
}

PublicCardState {
  hand: Vec<PublicCardInHand>,
  draw_pile: PublicCardZoneObservation,
  discard_pile: PublicCardZoneObservation,
  exhaust_pile: PublicCardZoneObservation,
  limbo_public_cards: Vec<PublicCardRef>,
  card_in_play_public_ref: Option<PublicCardRef>,
}

PublicCardInHand {
  public_card_ref,
  hand_slot,
  card_id,
  upgrade,
  cost_for_turn,
  rendered_damage,
  rendered_block,
  rendered_magic,
  flags,
  playable,
  unplayable_reason_public,
}

PublicCardZoneObservation {
  visibility,
  cards,
  counts_by_card_id,
  total_count,
  order_visible,
}

PublicMonster {
  public_monster_ref,
  slot,
  monster_id,
  hp,
  max_hp,
  block,
  powers: Vec<PublicPower>,
  lifecycle,
  intent: IntentObservation,
  previous_public_moves,
}

PublicCombatState {
  turn_index,
  combat_step_index,
  decision_kind,
  phase,
  public_history,
}
```

Observation mode must state whether draw pile order is visible, hidden, or
represented only as counts. Do not silently mix these modes.

### Intent Observation

Monster intent is a structured public object, not a damage number.

```text
IntentObservation::Visible(VisibleIntent)
IntentObservation::UnknownToPlayer
IntentObservation::MissingVisibleBridgeBug
IntentObservation::OracleOnly
```

```text
VisibleIntent {
  kind,
  damage_per_hit,
  hit_count,
  block,
  debuffs,
  status_effects,
  target_scope,
}
```

If any monster has `MissingVisibleBridgeBug`:

```text
action_selection_trainable=false
default collector rejects the frame
missing_visible_intent_rate increments
default action sampler must not be called
```

Training around missing visible intent is forbidden.

### Public History

```text
PublicHistory {
  cards_played_this_turn,
  cards_played_this_combat,
  attacks_played_this_turn,
  skills_played_this_turn,
  powers_played_this_turn,
  cards_discarded_this_turn,
  cards_drawn_this_turn,
  energy_spent_this_turn,
  hp_lost_this_turn,
  unblocked_damage_taken_this_turn,
  times_damaged_this_turn,
  card_in_play_public_ref,
  limbo_public_cards,
  previous_public_monster_moves,
}
```

This history is public, not oracle. It is required for cards and relics whose
behavior depends on turn/combat events.

## Kernel Transition

```text
KernelTransition {
  replay_identity_before: ReplayIdentity,
  replay_identity_after: ReplayIdentity,

  previous_decision_id: DecisionId,
  attempted_action_id: ActionId,
  recorded_action: RecordedActionTrace,

  public_events: Vec<PublicCombatEvent>,
  privileged_event_bundle: Option<PrivilegedEventBundle>,

  outcome: KernelOutcome,

  full_state_hash_before: FullStateHash,
  full_state_hash_after: FullStateHash,
  public_observation_hash_after: Option<PublicObservationHash>,
  decision_hash_after: Option<DecisionHash>,
  rng_hash_before: RngHash,
  rng_hash_after: RngHash,
  outcome_hash: OutcomeHash,
  action_trace_hash: ActionTraceHash,
}
```

`privileged_event_bundle` is absent by default. If present, it must carry a
privilege manifest. A record containing it is not action-selection-trainable
unless a collector explicitly strips privileged fields before writing the
trainable record.

## Outcome Taxonomy

The kernel separates call errors, rejections, terminal states, collector
truncations, kernel aborts, and replay faults.

```text
KernelCallError:
  HandleNotFound
  SnapshotNotOwnedByKernel
  SnapshotExpired
  SchemaIncompatible
```

Call errors happen before a transition is formed.

```text
KernelOutcome::Decision(PublicDecisionFrame)
KernelOutcome::Terminal(CombatTerminalReport)
KernelOutcome::Rejected(StepRejection)
KernelOutcome::Truncated(EpisodeTruncation)
KernelOutcome::Aborted(KernelAbort)
KernelOutcome::ReplayFault(ReplayFault)
```

```text
StepRejection:
  StaleDecisionId
  InvalidActionId
  ActionNotInCurrentDecision
```

Rejected actions must not mutate state:

```text
full_state_hash_before == full_state_hash_after
rng_hash_before == rng_hash_after
recorded_action.public_descriptor_snapshot == None
```

```text
EpisodeTruncation:
  MaxDecisionLimit
  CollectorHorizonReached
  EvaluationBudgetExceeded
  UserRequestedStop
```

Only collector-controlled non-MDP horizons are truncations.

```text
KernelAbort:
  EnginePanic
  TickBudgetExceeded
  UnsupportedBoundary { observed_engine_substate, public_partial_snapshot }
  ExternalProcessLost
  InternalInvariantViolation
```

Kernel abort samples are not trainable.

```text
ReplayFault:
  OriginMismatch
  StateHashMismatch
  DecisionHashMismatch
  ActionSetHashMismatch
  DescriptorHashMismatch
  AmbiguousSemanticReplay
  NoMatchingReplayAction
  ReplayVersionMismatch
```

Replay fault samples are not trainable and do not contribute to truncation rate.

## Combat Terminal Report

```text
CombatTerminalReport {
  kind: CombatTerminalKind,
  final_public_summary: FinalPublicCombatSummary,
  terminal_reason: TerminalReason,
  combat_end_hooks_applied: bool,
  reward_screen_reached: bool,
  reward_generation_started: bool,
  replay_identity_at_terminal: ReplayIdentity,
}
```

```text
CombatTerminalKind:
  Won
  Lost
  Escaped
```

```text
FinalPublicCombatSummary {
  player_hp,
  player_max_hp,
  monsters_alive,
  turn_index,
  cards_played_this_combat,
  hp_lost_this_combat,
}
```

The terminal report must expose final public combat metrics without requiring raw
state access.

The combat kernel must define whether combat-end hooks have been applied. It
must not consume post-combat reward-generation RNG.

## Public Combat Events

`public_events` are chronological and contain only player-visible content.
Privileged events must not be mixed into `public_events`.

Minimum event schema:

```text
PublicCombatEvent::CardPlayed { card_ref, card_id, upgraded, targets }
PublicCombatEvent::DamageDealt { source, target, amount, blocked, hp_loss }
PublicCombatEvent::DamageTaken { target, amount, hp_loss }
PublicCombatEvent::CardMoved { card_public_ref, card_id, from, to, visibility }
PublicCombatEvent::EnergySpent { amount }
PublicCombatEvent::MonsterDied { monster_ref }
PublicCombatEvent::TurnEnded { turn_index }
PublicCombatEvent::TurnStarted { turn_index }
```

Combat task rewards and metrics may use public observations, public events, and
terminal reports. They must not recover metrics by reading raw state.

## Snapshot, Restore, Fork

Snapshots crossing into task or Python code must be capability handles, not
inspectable bytes:

```text
OpaqueKernelSnapshotHandle {
  snapshot_id,
  owner_kernel_id,
  decision_id,
  replay_identity,
  schema_version,
  engine_version,
  content_manifest_hash,
  state_hash,
}
```

Engine state and RNG state are stored in kernel-owned snapshot storage. The
private payload must not be serialized into trainable records, exposed through
Python `info`, logged, or exported in datasets.

Serializable snapshots for replay/debug may be written only by kernel-owned
replay storage. They must be marked non-trainable and excluded from dataset
export.

### Fork Isolation

After `fork(parent)`:

```text
stepping child must not mutate parent
stepping parent must not mutate child
RNG streams diverge only through actions applied to each branch
handles must carry generation/session identity
stale decisions from parent are invalid in child and vice versa
```

## Replay Identity and Hashing

```text
ReplayIdentity {
  game_version,
  engine_commit,
  contract_schema_version,
  content_manifest_hash,
  mod_manifest_hash,
  origin_hash,
  rng_snapshot_digest,
  full_state_hash,
  public_observation_hash,
  decision_hash,
  decision_counter,
  action_trace_hash,
}
```

All replay hashes must use canonical serialization with sorted map keys.

Forbidden hash inputs:

```text
HashMap iteration order
pointer addresses
allocation order
debug-format strings
Python object identity
```

Required hashes:

```text
full_state_hash:
  engine-private deterministic state, including hidden state and RNG

public_observation_hash:
  PublicObservation only

choice_spec_hash:
  ChoiceSpec only

public_descriptor_hash:
  PublicActionDescriptor without local ActionId

action_set_hash:
  ordered public descriptor hashes

decision_hash:
  decision_id + decision_kind + public_observation_hash + choice_spec_hash +
  action_set_hash

outcome_hash:
  public outcome data + terminal/truncation/abort/replay-fault classification

action_trace_hash:
  rolling hash(previous_action_trace_hash, decision_hash,
  selected_descriptor_hash, outcome_hash)
```

Hashes are replay/audit metadata. They must never be encoded into
action-selection or value-estimation tensors. Full private hashes must not be
written into trainable datasets; public observation and decision hashes may be
stored only as metadata for dataset integrity checks.

## Privileged and Debug Leakage Rules

Default Python adapters must not expose any of these through observations,
`info`, callbacks, logger payloads, dataset records, or artifacts:

```text
PrivilegedDecisionData
PrivilegedEventBundle
DebugOracle
snapshot private payload
hidden RNG
raw engine state
full private state hash
```

Kernel-owned replay audit artifacts are the only exception for full private
hashes. Those artifacts must be marked non-trainable and excluded from
action-selection datasets.

`PrivilegedDecisionData` and `PrivilegedEventBundle` must carry:

```text
PrivilegeManifest {
  fields,
  allowed_consumers,
  forbidden_consumers,
}
```

Allowed consumers may include explicitly declared evaluators or value estimators.
Forbidden consumers must include action-selection models, action samplers, and
behavior-cloning targets.

Any record containing `DebugOracle` must be marked:

```text
action_selection_trainable=false
```

## Python and Training Boundary

CleanRL-style scripts may be used only as disposable smoke tests or references
for small algorithm fragments. They are not the project trainer, not the data
pipeline, and not a kernel constraint.

Any adapter that requires flattening `PublicDecisionFrame` into a single fixed
`Discrete(N)` space is temporary. It must not change kernel types, replay trace
shape, or public action descriptor semantics.

The real collector/trainer/replay/evaluator must be owned by this project and
must support:

```text
public/privileged observation split
recurrent state
candidate scoring
autoregressive action heads
deterministic replay
mixed search/RL data
```

Do not ask whether the kernel can be fed to CleanRL. Ask whether the transition
trace can be replayed, audited, searched, and trained on without semantic leaks.

## Provenance Is Metadata

Deck/run provenance is metadata attached to origins, episodes, and datasets. It
is not a kernel layer.

```text
live_run
run_replay
authored_probe
weak_controller_rollout
randomized_probe
```

Rules:

- `live_run` and `run_replay` can support distribution claims.
- `authored_probe` can support mechanic claims.
- `weak_controller_rollout` can support coverage claims, not quality claims.
- `randomized_probe` can find crashes, not prove strategic value.

Do not use random deck insertion as evidence about real deckbuilding.

## Acceptance Gate

The kernel cannot move to real rollout until these checks exist and pass:

```text
A. BasicCombatTurn
  starter Ironclad vs Jaw Worm
  TurnAction -> PlayCard/EndTurn -> monster turn -> TurnAction/Terminal

B. TargetedVsUntargeted
  targeted card
  targetless card
  potion with target
  potion without target

C. DiscardPileSelection
  Headbutt/Hologram-like flow
  TurnAction -> SelectFromDiscardPile -> TurnAction

D. ExhaustPileSelection
  Exhume-like flow
  source=ExhaustPile
  candidate refs stable

E. HandSelection
  discard/exhaust/select-from-hand
  min/max/any_number/requires_confirm covered

F. GeneratedCards
  SelectFromGeneratedCards
  selected generated card has stable semantic metadata

G. OrderedChoice
  OrderCards
  ordered=true
  selected_so_far and remaining count correct

H. RelicCounterVisibility
  visible relic counters in PublicObservation
  no hidden relic internals in PublicObservation

I. PublicHistoryMechanics
  cards_played_this_turn
  attacks_played_this_turn
  damage_taken_this_turn
  card_in_play
  limbo

J. TerminalBoundary
  Won maps to CombatTerminalReport
  reward screen is not exposed by CombatKernel

K. ReplayDeterminism
  same origin + same recorded action trace -> identical state_hash sequence

L. LeakageTest
  PublicObservation contains no DebugOracle fields
  default training trace contains no DebugOracle
  default training trace contains no PrivilegedEventBundle
  trace with DebugOracle is action_selection_trainable=false

M. InvalidStepSafety
  stale decision_id does not mutate state
  invalid action_id does not mutate state
  unchanged state/rng hashes are reported

N. SnapshotOpacity
  task/Python code can hold and pass OpaqueKernelSnapshotHandle
  task/Python code cannot inspect private payload

O. DuplicateCardIdentity
  hand contains two identical Strike cards
  discard contains duplicate upgraded/unupgraded copies
  descriptors and candidate refs remain distinguishable
  no semantic collision

P. DuplicateMonsterIdentity
  two monsters with same monster_id
  target descriptors use stable public_monster_ref and slot
  killing one does not retarget the other

Q. ChoicePurposeDisambiguation
  same card selected from discard under Headbutt-like and Hologram-like effects
  semantic keys differ by choice_context and operation

R. CanonicalActionOrdering
  same origin + same state generates identical ordered descriptor hashes
  no HashMap iteration dependency

S. ExactReplayActionSetHash
  recorded action_set_hash matches current action_set_hash before using
  original_action_id

T. SemanticReplayAmbiguity
  duplicate candidates cause semantic replay ambiguity unless public refs
  disambiguate
  ambiguity returns ReplayFault, not guessed action

U. ForkIsolation
  fork at a decision
  step parent and child differently
  verify no aliasing and independent state hashes

V. TerminalNoRewardRng
  winning combat reaches CombatTerminalReport
  reward screen is not exposed
  reward-generation RNG is not consumed by CombatKernel

W. TerminalPayload
  final hp/turns/hp lost are available from CombatTerminalReport
  task does not read raw CombatState for metrics

X. PythonInfoLeakage
  Gym-compatible adapter observation/info/log payload contains no privileged
  observation, DebugOracle, snapshot private payload, hidden RNG, or full raw
  state hash

Y. PublicRefLifecycle
  visible card/monster/potion refs are stable under movement/death/use
  refs are never reused within a combat trace
```

If these fail, the next task is kernel/view/replay repair. Not PPO, not search,
and not reward tuning.

## Implementation Rule

Do not implement training in the same change as the kernel.

The implementation sequence belongs in
`docs/AI_COMBAT_KERNEL_IMPLEMENTATION_PLAN.md`. That plan is part of this
contract: a kernel change is not maintainable unless it can point to the phase
and acceptance gate it satisfies.

## Current Status

- Micro Jaw Worm PPO proves the Rust/Python RL loop can run.
- Micro two-slimes proves target masks can train.
- Both are toy environments, not real-combat foundations.
- CleanRL is now only a disposable reference/smoke tool.
- The real foundation is `PublicDecisionFrame + PublicActionDescriptor +
  RecordedActionTrace + KernelTransition + ReplayIdentity`.
- Any old audit shell, seed patch, baseline continuation, transparent snapshot,
  privileged trace leak, weak-controller-as-teacher path, or Gym-first shape is
  outside this contract.
