# AI Combat Kernel Contract

This is the foundation contract for real Slay the Spire combat AI work in this
repository.

It intentionally does not define a bot, a planner, a neural network, a card
reward model, or a full-run learner. It defines the one loop every later system
must stand on:

```text
combat origin
  -> real engine state
  -> next decision requiring player input
  -> legal action descriptors
  -> chosen action
  -> next decision or combat terminal
```

Anything that cannot fit this loop is not a combat AI foundation.

## Design Rules

Hard rules:

- There is no opaque `PendingChoice`.
- There is no policy access to raw `CombatState`.
- There is no baseline bot continuation inside the kernel.
- There is no reward shaping inside the kernel.
- There is no seed-specific strategy branch.
- There is no fixture parser as the runtime contract.
- There is no training wrapper until the decision loop is real.

The kernel is allowed to know executable truth. The actor is not.

## Layer 1: Combat Origin

A combat cannot be started from vague loose fields and then treated as real.
There are only two valid origins:

```text
CombatOrigin::RunSnapshot {
  run_state,
  rng_state,
}

CombatOrigin::AuthoredCombat {
  spec,
  rng_state,
  purpose,
}
```

`RunSnapshot` is the parity path. It comes from an actual run state and carries
the RNG state needed to continue deterministically.

`AuthoredCombat` is a probe path. It can test a mechanic or build a small
training task, but it is not evidence about real developing-run deck
distribution.

`CombatStartSpec`, JSON fixtures, and hand-written deck slices are allowed only
as temporary adapters into `AuthoredCombat`. They are not the AI contract.

## Layer 2: Combat Kernel

`CombatKernel` owns mechanism only:

```text
start(origin) -> CombatHandle
advance_to_boundary(handle) -> KernelBoundary
legal_actions(handle) -> Vec<ActionDescriptor>
apply_action(handle, action_id) -> KernelStepResult
terminal(handle) -> Option<CombatTerminal>
```

`advance_to_boundary` advances internal engine ticks until one of these
boundaries is reached:

```text
KernelBoundary::Decision(DecisionState)
KernelBoundary::Terminal(CombatTerminal)
KernelBoundary::Error(KernelError)
```

The kernel must not expose `RewardScreen` as a combat boundary. If the
underlying engine reaches a reward screen after victory, the kernel maps that to
`CombatTerminal::Won`.

Required terminals:

```text
CombatTerminal::Won
CombatTerminal::Lost
CombatTerminal::Escaped
CombatTerminal::Error
```

If the engine cannot reach a decision or terminal within a fixed tick budget,
that is `KernelError::TickBudgetExceeded`. It is not a bad action and not a
training label.

## Layer 3: Decision State

`DecisionState` is the core object. Every state that needs player input must be
represented explicitly.

```text
DecisionState {
  id: DecisionId,
  kind: DecisionKind,
  actor_view: ActorView,
  critic_view: Option<CriticView>,
  action_descriptors: Vec<ActionDescriptor>,
}
```

There is no generic pending bucket. If the engine enters a new kind of input
substate, the kernel must either expose it as a typed `DecisionKind` or return
`KernelError::UnsupportedDecisionKind`.

Initial decision kinds:

```text
DecisionKind::TurnAction
DecisionKind::SelectFromHand
DecisionKind::SelectFromDrawPile
DecisionKind::SelectFromDiscardPile
DecisionKind::SelectFromExhaustPile
DecisionKind::SelectFromGeneratedCards
DecisionKind::SelectCardReward
DecisionKind::ChooseOne
DecisionKind::Confirm
DecisionKind::OrderCards
```

This list is allowed to grow. It is not allowed to collapse back into
`PendingChoice`.

### Action Descriptors

The normal solution for substate explosion is state-dependent legal actions.
The kernel returns concrete legal candidates for the current decision:

```text
ActionDescriptor {
  id: ActionId,
  verb: ActionVerb,
  arguments: ActionArguments,
  source: Option<ActionSource>,
  target: Option<ActionTarget>,
  visible_cost: Option<EnergyCost>,
}
```

Initial verbs:

```text
ActionVerb::PlayCard
ActionVerb::UsePotion
ActionVerb::EndTurn
ActionVerb::SelectCandidate
ActionVerb::Confirm
ActionVerb::Cancel
ActionVerb::Skip
```

Examples:

```text
PlayCard {
  hand_slot: 2,
  target: MonsterSlot(0),
}

SelectCandidate {
  candidate_index: 3,
}

EndTurn
```

`ActionId` is stable only inside the current `DecisionState`. A training task may
map descriptors into a fixed categorical action space, a candidate-scoring head,
or an autoregressive action head. The kernel does not pretend one global flat
action vocabulary solves every UI substate.

Forced choices may auto-resolve only when they are mechanically forced and
strategy-free. Any meaningful player choice must become a `DecisionState`.

## Layer 4: Views

The kernel produces views. It does not decide which view a learning algorithm is
allowed to train on.

```text
ActorView:
  legal player-visible information only

CriticView:
  optional privileged training view for asymmetric actor-critic

DebugOracle:
  replay/debug truth, never policy data
```

Policy/inference may consume only `ActorView`.

An asymmetric actor-critic task may use `CriticView` for value training if the
task declares that privilege explicitly and reports it in metrics. `CriticView`
must never be used to choose actions at inference time.

`DebugOracle` can contain exact draw order, executable monster steps, hidden RNG,
and full internal state references. It is for replay and diagnosis only. It must
not be serialized into actor training data by accident.

### ActorView Minimum

Initial actor-visible fields:

```text
player:
  hp
  max_hp
  block
  energy
  powers

cards:
  hand card ids/upgrades/current costs
  visible draw pile information under the selected observation mode
  discard pile cards
  exhaust pile cards

monsters:
  slot
  hp
  block
  powers
  alive/escaped/half-dead flags
  visible intent
  visible intent damage when player-visible

combat:
  turn count
  decision kind
```

Observation mode must state whether draw pile order is visible, hidden, or
represented only as counts. Do not silently mix these modes.

### Intent Contract

Monster intent must be classified:

```text
VisibleIntent
MissingVisibleIntent
OracleOnlyIntent
```

If executable truth says `Attack 11` but `ActorView` says no visible intent, the
task must either:

- fix the observation bridge,
- mark the state as `OracleOnlyIntent`, or
- reject it for policy-mode training.

Training around `MissingVisibleIntent` is forbidden.

## Layer 5: Combat Task Adapter

`CombatTask` is where learning and evaluation begin. It is downstream of the
kernel.

Allowed:

```text
encode_actor_view(decision) -> Tensor
encode_critic_view(decision) -> Optional<Tensor>
encode_action_space(decision.action_descriptors) -> MaskOrCandidates
reward(previous_decision, action, step_result, next_boundary) -> f32
metrics(episode) -> CombatMetrics
```

Forbidden:

- reading raw `CombatState`,
- inventing mechanics,
- treating kernel errors as negative reward,
- using debug oracle fields as actor input,
- calling the old bot,
- producing card-pick conclusions from combat-only tasks.

Action encoding is task-local:

```text
narrow micro task:
  fixed categorical ids + invalid action mask

variable UI task:
  candidate descriptors + candidate scoring

compound command task:
  autoregressive verb/argument heads
```

The kernel only guarantees legal descriptors. It does not guarantee a single
neural output shape.

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
kernel_error_rate
unsupported_decision_rate
missing_visible_intent_rate
illegal_action_rate
critic_privilege_mode
```

`win_rate` alone is not a useful signal.

## Provenance Is Metadata

Deck/run provenance is important, but it is not a core kernel layer.

Record it as metadata attached to origins, episodes, and datasets:

```text
live_run
run_replay
authored_probe
weak_policy_rollout
randomized_probe
```

Rules:

- `live_run` and `run_replay` can support distribution claims.
- `authored_probe` can support mechanic claims.
- `weak_policy_rollout` can support coverage claims, not quality claims.
- `randomized_probe` can find crashes, not prove strategic value.

Do not use random deck insertion as evidence about real deckbuilding.

## Minimal Acceptance Gate

Before real-combat PPO, search, or value learning, the kernel must pass this
manual gate:

```text
1. Start starter Ironclad vs JawWorm from AuthoredCombat with explicit RNG state.
2. Reach DecisionKind::TurnAction.
3. Produce legal PlayCard and EndTurn descriptors.
4. Apply a legal action by ActionId.
5. Step through a full player turn and monster turn.
6. Return to DecisionKind::TurnAction or CombatTerminal.
7. Produce ActorView at every decision.
8. Report zero MissingVisibleIntent in policy mode.
9. Finish combat as CombatTerminal::Won or CombatTerminal::Lost.
10. Use no old bot, no fixture parser as runtime, and no reward screen boundary.
```

If this fails, the next task is kernel/view repair. Not PPO.

## Implementation Order

The first useful implementation is small and strict:

```text
1. Add typed structures for CombatOrigin, CombatKernel, DecisionState,
   ActionDescriptor, ActorView, CriticView, and CombatTerminal.
2. Implement AuthoredCombat starter Ironclad vs JawWorm through the real combat
   engine, with explicit RNG state.
3. Implement TurnAction descriptors for playable hand cards and EndTurn.
4. Implement ActorView intent classification.
5. Add one smoke binary that prints decisions/actions/views and exits on
   terminal.
6. Only then add a CombatTask adapter.
```

Do not implement training in the same change as the kernel.

## Current Status

- Micro Jaw Worm PPO proves the Rust/Python RL loop can run.
- Micro two-slimes proves target masks can train.
- Both are toy environments, not real-combat foundations.
- The real foundation is the decision loop in this document.
- Any old audit shell, seed patch, or baseline continuation is outside this
  contract.
