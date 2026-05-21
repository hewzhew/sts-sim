# AI Combat Rust Migration Ledger

This ledger decides what happens to the existing Rust simulator while rebuilding
the combat kernel. Existing code is evidence, not authority. A module is kept
only when it preserves Java-derived combat semantics and supports deterministic
replay.

## Status Values

```text
keep:
  matches the Java-derived schema and can be used directly

rewrite:
  useful concept, but state shape or semantics are incomplete

delete:
  seed-driven, policy-polluted, fixture-driven, or incompatible with replay

adapter_only:
  allowed only to import/export CombatStateSnapshot or bridge legacy data

unknown:
  not reviewed yet; cannot be a dependency of the new kernel
```

`unknown` is a blocker for kernel implementation. It is safer than pretending an
old module is valid.

## Required Columns

Each row must contain:

```text
rust_path
current_role
status
java_source_mapping
combat_schema_mapping
kept_features
missing_or_wrong_features
required_action
acceptance_check
notes
```

## Initial Rust Inventory

This table starts the migration. It is deliberately strict: most old runtime
code is not allowed into the new kernel without a source-mapping review.

| Rust path | Current role | Status | Java source mapping | Combat schema mapping | Required action |
| --- | --- | --- | --- | --- | --- |
| `src/ai/micro_jaw_worm.rs` | removed toy RL micro-environment | delete | none as real engine; only conceptual combat demo | none | removed from active Rust compile surface |
| `src/ai/micro_two_slimes.rs` | removed toy RL micro-environment | delete | none as real engine; only mask demo | none | removed from active Rust compile surface |
| `src/runtime/rng.rs` | LibGDX/Java RNG replica | keep/review | `random/Random.java`, `RandomXS128` | `CombatRngState` | verify state words and counter serialization |
| `src/runtime/combat.rs` | current combat state | rewrite | `AbstractDungeon`, `AbstractPlayer`, `AbstractRoom`, `MonsterGroup` | `CombatStateSnapshot` | compare field-by-field against source coverage ledger |
| `src/runtime/action.rs` | current action model | rewrite | `AbstractGameAction`, `GameActionManager`, concrete actions | `ActionManagerState`, `ActionState` | replace opaque or simplified actions with typed resumable payloads |
| `src/engine/core.rs` | current engine tick/input loop | rewrite | `GameActionManager`, room update, pending choices | future `CombatKernel` transition | remove reward-screen and run-level coupling from combat kernel path |
| `src/engine/pending_choices.rs` | current pending choice handling | rewrite | `GridCardSelectScreen`, `HandCardSelectScreen`, generated choices | `ChoiceScreenState`, `ChoiceSpec` | replace generic pending choice with source-backed choice state |
| `src/state/core.rs` | mixed run/combat engine state and client inputs | adapter_only/rewrite | multiple Java screens and run states | not canonical combat state | do not expose to new kernel as authority |
| `src/state/selection.rs` | current selection abstraction | rewrite | grid/hand/select screens | `ChoiceSpec`, `ChoiceContext` | keep concepts only after source mapping |
| `src/content/cards` | card ids/effects | rewrite | `cards/*` Java classes | `CardInstance`, action/effect handlers | audit every implemented card against Java semantics |
| `src/content/relics` | relic ids/hooks | rewrite | `relics/*` Java classes | `RelicInstance`, hook handlers | audit counters, used-up state, hook order |
| `src/content/potions` | potion effects | rewrite | `potions/*` Java classes | `PotionInstance`, potion actions | audit target, potency, thrown/discarded behavior |
| `src/content/powers` | power ids/hooks | rewrite | `powers/*` Java classes | `PowerInstance`, hook handlers | audit power ordering, priority, concrete payloads |
| `src/content/monsters` | monster factory/AI | rewrite | `monsters/*` Java classes | `MonsterState`, `MonsterMoveState`, `IntentState` | audit move history, intent construction, RNG use |
| `src/content/stances` | stance state/hooks | rewrite | `stances/*` Java classes | `StanceState` | audit Watcher hook timing |
| `src/protocol/java.rs` | Java/communication protocol structures | adapter_only | bridge/protocol only, not game semantics | import/export only | cannot define canonical state |
| `src/projection` | observation/projection helpers | rewrite | public observation rules | `PublicObservation` later | must not read private state for policy data |
| `src/testing` | old fixture/scenario support | adapter_only/delete | none unless source-mapped | import/export only | cannot be runtime behavior |
| `src/verification` | old verification helpers | unknown | unknown | unknown | review before reuse |
| `src/diff` | replay/diff tools | adapter_only/rewrite | bridge comparison | replay audit only | may be reused only after leakage review |
| `src/cli/live_comm*` | live communication tooling | adapter_only | external game bridge | extraction/probe support | cannot define simulator mechanics |

## Keep Criteria

A module can move from `rewrite` or `unknown` to `keep` only when:

```text
1. Java source rows are named.
2. Schema paths are named.
3. Hidden state and RNG use are represented.
4. It serializes into or derives from CombatStateSnapshot.
5. It has deterministic replay checks.
6. It does not contain policy, seed-specific, or baseline continuation logic.
```

## Delete Criteria

Delete or quarantine a module when:

```text
1. It exists only for a toy RL micro-task.
2. It encodes policy decisions as simulator behavior.
3. It requires raw state access outside kernel boundaries.
4. It cannot distinguish public, privileged, and debug data.
5. It cannot be made deterministic without rewriting most of it.
```

## First Implementation Dependency Rule

The first `CombatStateSnapshot` implementation may depend only on rows whose
status is `keep`, `rewrite`, or `adapter_only` with an explicit required action.
It must not depend on `unknown`.
