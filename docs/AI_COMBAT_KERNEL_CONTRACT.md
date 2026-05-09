# AI Combat Kernel Contract

This document defines the foundation for any future Slay the Spire AI work in
this repository. It exists to prevent another cycle of ad hoc probes, seed
patches, fixture glue, and training wrappers being mistaken for a real system.

The AI stack must be split into four layers:

```text
CombatKernel
CombatPublicView
CombatTask
DeckSource
```

No PPO loop, search module, planner, rule bot, or card-pick evaluator is allowed
to bypass these boundaries.

## Non-Goals

This contract does not claim to solve:

- card reward quality,
- route planning,
- shop planning,
- full-run learning,
- A20H play,
- expert imitation,
- MCTS,
- neural value learning.

Those are downstream problems. They are not allowed to define the foundation.

## Layer 1: CombatKernel

`CombatKernel` is the mechanism layer.

It owns only typed combat state transition:

```text
start(slice) -> CombatHandle
legal_actions(handle) -> Vec<CombatAction>
step(handle, action) -> StepResult
terminal(handle) -> Option<CombatTerminal>
```

It must not contain:

- PPO-specific observation arrays,
- reward shaping,
- card-pick scoring,
- route logic,
- seed-specific branches,
- baseline bot calls,
- heuristic policy logic,
- JSON fixture parsing as core behavior.

### CombatSlice

The input to `start` must be typed, not a stringly test fixture:

```text
CombatSlice {
  seed: u64,
  player_class: PlayerClass,
  ascension: u8,
  encounter: EncounterId,
  room_type: RoomType,
  hp: i32,
  max_hp: i32,
  deck: Vec<CardInstanceSpec>,
  relics: Vec<RelicInstanceSpec>,
  potions: Vec<Option<PotionInstanceSpec>>,
}
```

`CombatStartSpec` may be used only as a temporary reference while implementing
this layer. It is not the AI contract. It is testing/fixture glue.

### CombatAction

The initial action set should be narrow:

```text
EndTurn
PlayCard { hand_index: usize, target: Option<MonsterSlot> }
```

Potion use, pending selections, and generated-choice cards are separate
capabilities. They should not be smuggled into v0.

### Stable Boundary

`step` must advance the engine to one of:

```text
CombatPlayerTurn
PendingChoice
RewardScreen
GameOver
Error
```

If the engine cannot reach one of these boundaries within a fixed iteration
limit, the result is `Error`, not a policy judgment.

### Kernel Truth

The kernel is allowed to know executable truth:

- exact draw pile order,
- monster executable turn steps,
- hidden random state,
- full `CombatState`.

But executable truth must not automatically enter policy observations.

## Layer 2: CombatPublicView

`CombatPublicView` is the player-visible observation layer.

It answers:

```text
What can a legal player see right now?
```

It must be derived from `CombatKernel` state through a single explicit function:

```text
public_view(handle) -> CombatPublicView
```

The public view must separate:

```text
visible:
  legal player information

oracle:
  simulator-only information used for debugging
```

The policy may consume only `visible`.

### Required Visible Fields

Initial v0 visible fields:

```text
player:
  hp
  max_hp
  block
  energy
  powers

cards:
  hand card ids/upgrades/current costs
  draw pile contents if legally known by the current simulator mode
  discard pile contents
  exhaust pile contents

monsters:
  slot
  hp
  block
  powers
  alive/escaped/half-dead flags
  visible intent
  visible intent damage when legally available

combat:
  turn count
  phase
```

### Intent Rule

Monster intent is a contract-critical field.

If the executable monster plan says:

```text
Attack 11
```

but the public visible intent is:

```text
None
```

then the observation layer is not ready for learning.

Do not train around this mismatch. Fix or explicitly classify it first.

Allowed classifications:

```text
VisibleIntent
  the player-visible intent is known and policy-legal

MissingVisibleIntent
  executable truth exists but visible observation is absent

OracleOnlyIntent
  executable truth exists but should not be exposed to policy
```

`CombatTask` must reject `MissingVisibleIntent` unless the task explicitly
declares it is an oracle diagnostic.

## Layer 3: CombatTask

`CombatTask` is the learning/evaluation layer.

It may define:

```text
obs_encoder(public_view) -> Tensor
action_ids(public_view, legal_actions) -> Mask
reward_adapter(previous_public_view, action, step_result) -> f32
metrics(episode) -> CombatMetrics
```

It must not:

- call into raw `CombatState`,
- read executable monster steps unless in oracle diagnostic mode,
- invent combat mechanics,
- score cards as deckbuilding conclusions,
- silently treat unresolved kernel errors as bad actions.

### Required Metrics

Every combat task must report at least:

```text
episodes
kill_rate
avg_final_hp
avg_hp_lost
min_final_hp
avg_turns
truncated_rate
kernel_error_rate
missing_visible_intent_rate
illegal_action_rate
```

`kill_rate` alone is not a sufficient metric.

### Baselines

Every task must compare against at least:

```text
random_legal
greedy_damage
greedy_block_or_survive
```

If a trained policy only beats `random_legal` on kill rate but not HP, the task
is too weak or the policy is not meaningful.

## Layer 4: DeckSource

`DeckSource` describes where a deck came from.

It is not a card-pick label.

Allowed source kinds:

```text
live
replay
authored_probe
weak_policy
```

Rules:

- `live` and `replay` can be used as combat-distribution evidence.
- `authored_probe` can test a specific combat mechanic.
- `weak_policy` can provide behavior coverage, not proof of good decisions.
- random card insertion is not a real developing-run distribution.

Deckbuilding evaluation is forbidden until there is a separate contract for
continuation policy and long-horizon attribution.

## Forbidden Patterns

Do not add:

```text
seed death -> if/bonus/penalty
probe binary -> permanent foundation
testing fixture parser -> AI runtime contract
random deck -> real deck distribution
baseline continuation death -> card choice is bad
oracle executable truth -> policy observation
kill_rate only -> success
```

These patterns caused the previous codebase collapse.

## Minimal Acceptance Gate

Before writing any PPO wrapper around real combat, the kernel must pass this
manual gate:

```text
1. Start starter Ironclad vs JawWorm from a typed CombatSlice.
2. Reach CombatPlayerTurn.
3. Produce legal PlayCard and EndTurn actions.
4. Step through at least one full player turn and monster turn.
5. Return to CombatPlayerTurn or terminal.
6. Produce CombatPublicView at every stable boundary.
7. Report zero MissingVisibleIntent for policy-mode tasks.
8. Finish combat into RewardScreen or GameOver without fixture glue.
```

If this fails, the next task is kernel/public-view repair, not PPO.

## Implementation Order

The next implementation should be:

```text
1. Remove any uncommitted probe code.
2. Add `src/ai/combat_kernel.rs` with typed data structures only.
3. Implement `start` for starter Ironclad vs JawWorm.
4. Implement `public_view`.
5. Add one binary smoke runner that uses only CombatKernel, not testing fixtures.
6. Only after the smoke runner passes, design `CombatTask`.
```

Do not implement training in the same change as the kernel.

## Current Status

As of this document:

- micro Jaw Worm PPO exists and proves the Rust/Python RL loop can run.
- micro two-slimes exists and proves target-mask training can run.
- both are toy environments, not real-combat foundations.
- `CombatStartSpec` has been useful as a spike reference but must not become the
  AI runtime contract.
- real combat stepping appears possible, but public intent semantics are not yet
  trustworthy enough for policy training.
