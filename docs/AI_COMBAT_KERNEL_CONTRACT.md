# AI Combat Kernel Contract

This document defines the foundation for real Slay the Spire combat AI work in
this repository.

The kernel is not a bot, not a planner, not a Gym environment, and not a
CleanRL-shaped wrapper. It is the deterministic combat state machine boundary
that rollout, replay, search, and training systems must use.

The canonical loop is:

```text
CombatOrigin
  -> KernelSession { handle, current_decision }
  -> choose one ActionDescriptor by its local ActionId
  -> KernelTransition
  -> KernelOutcome
```

The canonical training/replay record is:

```text
DecisionFrame
+ ActionDescriptor snapshot
+ RecordedActionTrace
+ KernelTransition
```

Anything outside this loop is downstream.

## Hard Rules

- There is no opaque `PendingChoice`.
- There is no raw `CombatState` access from task, collector, trainer, or Python
  adapter code.
- There is no baseline bot continuation inside the kernel.
- There is no reward shaping inside the kernel.
- There is no seed-specific strategy branch.
- There is no fixture parser as the runtime contract.
- There is no `RewardScreen` boundary in `CombatKernel`.
- There is no `CombatTerminal::Error`.
- There is no CleanRL or Gym constraint on kernel shape.
- There is no default emission of privileged or debug data into trainable traces.

Kernel mechanical type names must not contain algorithm words such as Actor,
Critic, Policy, Value, PPO, CleanRL, Trainer, or RewardShaping.

The kernel may know executable truth. Public data used for action selection may
not.

## Layer 1: Combat Origin

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

`AuthoredCombat` is a probe path. It can test a mechanic or build a small
training task, but it is not evidence about real developing-run deck
distribution.

`CombatStartSpec`, JSON fixtures, and hand-written deck slices are allowed only
as temporary adapters into `AuthoredCombat`. They are not the AI contract.

### Replay Identity

`rng_state` alone is too vague. Every origin must carry a replay identity:

```text
ReplayIdentity {
  game_version,
  engine_commit,
  contract_schema_version,
  content_manifest_hash,
  mod_manifest_hash,
  origin_hash,
  rng_snapshot,
  state_hash,
  decision_counter,
  action_trace_hash,
}
```

The replay identity exists so that:

```text
same origin + same recorded action trace -> same state hash sequence
```

The recorded action trace is not just semantic intent. It includes the local
action id, descriptor snapshot, semantic key, public refs, schema version, and
hashes. If replay diverges, the result is a replay fault, not a training loss.

## Layer 2: Combat Kernel

`CombatKernel` owns mechanism only.

```text
trait CombatKernel {
  start(origin) -> Result<KernelSession, KernelFault>

  step(
    handle,
    decision_id,
    action_id,
  ) -> Result<KernelTransition, KernelFault>

  snapshot(handle) -> Result<OpaqueKernelSnapshot, KernelFault>
  restore(snapshot) -> Result<KernelSession, KernelFault>
  fork(handle) -> Result<KernelSession, KernelFault>
}
```

`KernelSession` means "attached to a current decision", not "combat just
started". `start`, `restore`, and `fork` all return the same shape:

```text
KernelSession {
  handle,
  current_decision: DecisionFrame,
  replay_identity: ReplayIdentity,
}
```

The field must not be called `first_decision`, because restored and forked
sessions resume at the current boundary.

`step` is atomic. It validates that `decision_id` is still current, resolves the
local `ActionId` to the current descriptor, applies the action, advances
internal engine ticks until the next decision or outcome, and returns one
`KernelTransition`.

There must not be separate authoritative calls for:

```text
legal_actions(handle)
terminal(handle)
```

The current legal actions live inside the current `DecisionFrame`. The terminal,
truncation cause, or fault lives inside the `KernelTransition`.

### Invalid Step Semantics

Invalid step attempts must not mutate engine state.

These cases return `KernelOutcome::Rejected` with unchanged state hash:

```text
StaleDecisionId
InvalidActionId
ActionNotInCurrentDecision
```

Required behavior:

```text
state_hash_before == state_hash_after
rng_hash_before == rng_hash_after
action_descriptor_snapshot == None
```

Collectors may count these as illegal action attempts. They are not combat
terminal states and not environment rewards.

`HandleNotFound` returns `Err(KernelFault::HandleNotFound)` before a transition
is formed.

### Kernel Transition

```text
KernelTransition {
  previous_decision_id,
  attempted_action_id,
  action_descriptor_snapshot: Option<ActionDescriptor>,

  public_events,
  privileged_event_bundle: Option<PrivilegedEventBundle>,

  outcome,

  state_hash_before,
  state_hash_after,
  rng_hash_before,
  rng_hash_after,
  engine_version,
  contract_schema_version,
  content_manifest_hash,
  action_trace_hash,
}
```

`action_descriptor_snapshot` is `Some(ActionDescriptor)` only for an accepted
action. It is required for every accepted action so a trace remains interpretable
even if later code changes descriptor generation.

`privileged_event_bundle` is optional and absent by default. If present, it must
carry a privilege manifest and the trace is not action-selection-trainable
unless a collector explicitly strips the privileged fields before writing the
trainable record.

### Kernel Outcome

```text
KernelOutcome::Decision(DecisionFrame)
KernelOutcome::Terminal(CombatTerminal)
KernelOutcome::Truncated(TruncationCause)
KernelOutcome::Rejected(StepRejection)
KernelOutcome::Fault(KernelFault)
```

`CombatTerminal` is only:

```text
Won
Lost
Escaped
```

Faults and truncations are not combat terminals and must not be converted into
negative rewards by the kernel.

Allowed truncation causes:

```text
TickBudgetExceeded
UnsupportedDecisionKind
ExternalProcessLost
MaxDecisionLimit
ReplayMismatch
```

The underlying engine may internally reach a reward screen after victory. The
combat kernel must map that to `CombatTerminal::Won` and stop. Post-combat
rewards belong to a run-level kernel.

### Snapshot, Restore, Fork

Search, replay, and debugging require cloning state without smuggling raw engine
objects into task code.

```text
OpaqueKernelSnapshot {
  snapshot_id,
  decision_id,
  replay_identity,
  schema_version,
  engine_version,
  content_manifest_hash,
  state_hash,
  private_payload,
}
```

`private_payload` contains engine and RNG state, but it is kernel-private. It may
be passed only to `restore`. It must not be serialized into trainable episode
records, inspected by task code, or exposed through Python `info`.

`restore(snapshot)` must produce a `KernelSession` whose `current_decision` hash
matches the decision hash at the time the snapshot was taken.

`fork(handle)` must be equivalent to `snapshot(handle)` followed by
`restore(snapshot)`.

## Layer 3: Decision Frame

`DecisionFrame` is the only player-input boundary.

```text
DecisionFrame {
  id,
  kind,
  public_observation,
  privileged_observation: Option<PrivilegedObservation>,
  actions,
  choice,
  schema_version,
  state_hash,
}
```

Every state that needs player input must be represented explicitly. If the
engine enters an unsupported substate, the kernel returns
`KernelOutcome::Truncated(UnsupportedDecisionKind)`.

There is no generic pending bucket.

`privileged_observation` is absent by default. If present, it must carry a
privilege manifest and must not be written into trainable action-selection data.

### Decision Kinds

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

`SelectCardReward` is forbidden in `CombatKernel`. Card rewards are run-level
decisions. Combat-generated choices must use `SelectFromGeneratedCards`.

### Choice Spec

`DecisionKind` alone is not enough. Every `DecisionFrame` must include choice
constraints:

```text
ChoiceSpec {
  source_zone,
  min_select,
  max_select,
  selected_so_far,
  remaining_min,
  remaining_max,
  ordered,
  any_number,
  can_skip,
  can_cancel,
  requires_confirm,
  auto_confirm_when_complete,
}
```

This is required for Headbutt, Hologram-like selection, Exhume-like selection,
Gambling Chip-like selection, generated-card selection, Scry, and ordered card
placement.

Forced choices may auto-resolve only when they are mechanically forced and
strategy-free. Any meaningful player choice must become a `DecisionFrame`.

## Layer 4: Action Descriptor

The normal solution for substate explosion is state-dependent legal candidates.
The kernel returns concrete actions for the current `DecisionFrame`:

```text
ActionDescriptor {
  id,
  semantic_key,
  verb,
  arguments,
  public_refs,
  engine_ref,
  visible_cost,
  constraints,
}
```

`ActionId` is an execution token scoped to one `DecisionFrame`. It must never be
used as a learning label.

`engine_ref` is an opaque execution reference. It is not serializable to public
or trainable data.

`ActionSemanticKey` is the stable, serializable action identity:

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
  source_zone,
  candidate_card_id,
  candidate_index_class,
}

ActionSemanticKey::Confirm { decision_kind }
ActionSemanticKey::Cancel { decision_kind }
ActionSemanticKey::Skip { decision_kind }
```

Every recorded action must store:

```text
RecordedActionTrace {
decision_id
action_id
action_descriptor_snapshot
action_semantic_key
public_argument_refs
action_schema_version
}
```

Downstream tasks may map `ActionSemanticKey` into:

```text
fixed global action vocabulary
candidate-scoring model input
autoregressive verb/argument targets
```

If a descriptor lacks enough stable public metadata for the selected encoding,
the task must reject the decision as unsupported instead of guessing.

Replay uses the full recorded action trace, not the semantic key alone. The
semantic key is for learning and analysis; the descriptor snapshot plus local
execution token is what makes exact replay auditable.

## Layer 5: Observations

Kernel observation names are mechanism names:

```text
PublicObservation:
  legal player-visible information only

PrivilegedObservation:
  optional mechanically true but non-public training observation

DebugOracle:
  replay/debug truth, never emitted to training by default
```

`PublicObservation` is the only observation allowed for action selection.

`PrivilegedObservation` may be used by a declared asymmetric value estimator or
evaluator. It must never be used by an action sampler or behavior-cloning target.

`DebugOracle` is not a training observation. It may contain exact draw order,
hidden RNG, executable monster steps, and raw internal references.

### Public Observation Minimum

```text
player:
  hp
  max_hp
  block
  energy
  powers
  stance
  orbs
  class_specific_public_state

relics:
  relic_ids
  visible_counters
  visible_charges
  visible_disabled_flags

potions:
  slots
  potion_ids
  usable_flags
  target_requirements

cards:
  hand:
    public_card_ref
    card_id
    upgrade
    cost_for_turn
    rendered_damage
    rendered_block
    rendered_magic
    exhaust_flag
    ethereal_flag
    retain_flag
    innate_flag
    playable_flag
    unplayable_reason_public
  draw_pile:
    observation_mode
    visible_cards_or_counts
  discard_pile
  exhaust_pile
  limbo_public_cards
  card_in_play_public_ref

monsters:
  public_monster_ref
  slot
  hp
  block
  powers
  alive
  escaped
  half_dead
  intent_visibility
  visible_intent: Option<VisibleIntent>
  previous_public_moves

combat:
  turn_index
  combat_step_index
  decision_kind
  phase
  public_history
```

Observation mode must state whether draw pile order is visible, hidden, or
represented only as counts. Do not silently mix these modes.

### Public History

```text
public_history:
  cards_played_this_turn:
    total
    by_type
    public_card_ids
  cards_played_this_combat:
    total
    by_type
  attacks_played_this_turn
  skills_played_this_turn
  powers_played_this_turn
  cards_discarded_this_turn
  cards_drawn_this_turn
  energy_spent_this_turn
  hp_lost_this_turn
  unblocked_damage_taken_this_turn
  times_damaged_this_turn
  card_in_play_public_ref
  limbo_public_cards
  previous_public_monster_moves
```

This history is public, not oracle. It is required for cards and relics whose
behavior depends on turn/combat events.

### Intent Contract

Monster intent is a structured public object, not a single damage number.

```text
VisibleIntent {
  kind,
  damage_per_hit,
  hit_count,
  block,
  debuffs,
  status_effects,
  target_scope,
  is_attack,
  is_buff,
  is_debuff,
  is_escape,
  is_sleep,
  is_unknown_to_player,
}
```

Every monster intent must be classified:

```text
IntentVisibility::Visible
IntentVisibility::MissingVisible
IntentVisibility::OracleOnly
```

If executable truth says `Attack 11` but `PublicObservation` says no visible
intent, the task must either:

- fix the observation bridge,
- mark the state as `OracleOnlyIntent`, or
- reject it for public action-selection training.

Training around `MissingVisibleIntent` is forbidden.

### Leakage Rules

`DebugOracle` must not be emitted by the default Python training wrapper.

`DebugOracle` may only be requested with:

```text
debug=true
replay=true
non_training_build=true
```

Any episode trace containing `DebugOracle` must be marked:

```text
action_selection_trainable=false
```

`PrivilegedObservation` and `privileged_event_bundle` must carry a manifest:

```text
PrivilegeManifest {
  fields,
  allowed_consumers,
  forbidden_consumers,
}
```

Example fields:

```text
exact_draw_order
hidden_rng_digest
future_monster_rolls
```

Allowed consumers may include value estimators or evaluators. Forbidden
consumers must include action-selection models, action samplers, and
behavior-cloning targets.

## Layer 6: Combat Task Adapter

`CombatTask` is downstream of the kernel.

Allowed:

```text
encode_public_observation(decision) -> Tensor
encode_privileged_observation(decision) -> Optional<Tensor>
encode_action_space(decision.actions) -> MaskOrCandidates
reward(previous_decision, action, transition) -> f32
metrics(episode) -> CombatMetrics
```

Forbidden:

- reading raw `CombatState`,
- inventing mechanics,
- treating kernel faults as negative reward,
- using `DebugOracle` fields as action-selection input,
- using `PrivilegedObservation` as action-selection input,
- using local `ActionId` as a global neural action id,
- calling the old bot,
- producing card-pick conclusions from combat-only tasks.

Action encoding is task-local:

```text
narrow smoke task:
  fixed categorical ids + invalid action mask

variable UI task:
  candidate descriptors + candidate scoring

compound command task:
  autoregressive verb/argument heads
```

The kernel only guarantees legal descriptors and replayable transitions. It does
not guarantee a single neural output shape.

### Required Metrics

Every combat task must report:

```text
episodes
win_rate
avg_final_hp
avg_hp_lost
min_final_hp
avg_turns
truncated_rate
fault_rate
stale_decision_rate
invalid_action_rate
unsupported_decision_rate
missing_visible_intent_rate
privileged_observation_mode
oracle_leakage_rate
replay_mismatch_rate
```

`win_rate` alone is not useful.

## Python and Training Boundary

The kernel is not a Gym environment and is not shaped around CleanRL.

CleanRL-style scripts may be used only as disposable smoke tests or as references
for small algorithm fragments. They are not the project trainer, not the data
pipeline, and not a kernel constraint.

A Python adapter may expose a Gymnasium-compatible `reset/step` API for narrow
fixed-vocabulary tests. The canonical training data is still:

```text
DecisionFrame + ActionDescriptor + RecordedActionTrace + KernelTransition
```

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
  source_zone=ExhaustPile
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
  Won maps to CombatTerminal::Won
  reward screen is not exposed by CombatKernel

K. ReplayDeterminism
  same origin + same recorded action trace -> identical state_hash sequence

L. LeakageTest
  PublicObservation contains no DebugOracle fields
  default training trace contains no DebugOracle
  default training trace contains no privileged_event_bundle
  trace with DebugOracle is action_selection_trainable=false

M. InvalidStepSafety
  stale decision_id does not mutate state
  invalid action_id does not mutate state
  unchanged state/rng hashes are reported

N. SnapshotOpacity
  task/Python code can hold and pass OpaqueKernelSnapshot
  task/Python code cannot inspect private payload
```

If these fail, the next task is kernel/view/replay repair. Not PPO, not search,
and not reward tuning.

## Implementation Order

The first implementation must be narrow and mechanical:

```text
1. Add typed structures for ReplayIdentity, CombatOrigin, KernelSession,
   DecisionFrame, ChoiceSpec, ActionDescriptor, PublicObservation,
   PrivilegedObservation, KernelTransition, OpaqueKernelSnapshot,
   CombatTerminal, TruncationCause, and KernelFault.

2. Implement AuthoredCombat starter Ironclad vs Jaw Worm through the real combat
   engine with explicit ReplayIdentity.

3. Implement TurnAction descriptors for playable hand cards, usable potions, and
   EndTurn.

4. Implement invalid step safety for stale decision ids and invalid action ids.

5. Implement PublicObservation intent, relic, potion, card-render, class-state,
   and public-history fields.

6. Implement KernelTransition with state/rng/action trace hashes.

7. Implement opaque snapshot/restore/fork.

8. Add smoke binaries for the acceptance gate cases.

9. Only then add a CombatTask adapter.
```

Do not implement training in the same change as the kernel.

## Current Status

- Micro Jaw Worm PPO proves the Rust/Python RL loop can run.
- Micro two-slimes proves target masks can train.
- Both are toy environments, not real-combat foundations.
- CleanRL is now only a disposable reference/smoke tool.
- The real foundation is `DecisionFrame + RecordedActionTrace +
  KernelTransition + replay identity`.
- Any old audit shell, seed patch, baseline continuation, transparent snapshot,
  privileged trace leak, weak-controller-as-teacher path, or Gym-first shape is
  outside this contract.
