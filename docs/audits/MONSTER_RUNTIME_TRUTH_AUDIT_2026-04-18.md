# Monster Runtime Truth Audit 2026-04-18

This audit is the phase-2 follow-up to [MONSTER_RUNTIME_STATE_FRAMEWORK.md](../design/MONSTER_RUNTIME_STATE_FRAMEWORK.md).

Scope:

- already-migrated stateful semantic monsters
- protocol truth completeness for hidden runtime state
- explicit execution-time runtime patches
- retirement of `move_history` fallback for hidden runtime truth

## Rules

- Hidden runtime truth must come from protocol truth or factory/spawn initialization.
- Execution-time runtime changes must flow through `Action::UpdateMonsterRuntime`.
- `move_history` is allowed only for Java rules that explicitly depend on `lastMove` / `lastTwoMoves`.
- `move_history` must not recover hidden runtime truth such as `first_turn`, `used_hex`, or `is_flying`.

## Audit Table

| Monster | Hidden runtime truth | Protocol truth | Explicit runtime patch | Hidden-truth history fallback retired | Allowed history use | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Hexaghost | `activated`, `orb_active_count`, `burn_upgraded`, divider cache | Yes | Yes | Yes | Sequence history only | Good |
| Lagavulin | `idle_count`, `debuff_turn_count`, `is_out`, `is_out_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| The Guardian | `damage_threshold`, `damage_taken`, `is_open`, `close_up_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| Byrd | `first_move`, `is_flying` | Yes | Yes | Yes | `PECK`/`SWOOP`/`CAW` sequencing | Good |
| Chosen | `first_turn`, `used_hex` | Yes | Yes | Yes | `DRAIN`/`DEBILITATE` sequencing | Good |
| Looter | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Mugger | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Shelled Parasite | `first_move` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Healer | None | N/A | N/A | N/A | Heal/attack/buff sequencing only | Good |
| Snake Plant | None | N/A | N/A | N/A | `lastMove`/`lastMoveBefore`/`lastTwoMoves` sequencing only | Good |
| Louse | `bite_damage` | Yes | N/A | Yes | None | Good |
| Snecko | `first_turn` | Yes | Yes | Yes | `lastTwoMoves(BITE)` sequencing | Good |
| Red Slaver | `first_turn`, `used_entangle` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Gremlin Nob | `used_bellow` | Yes | Yes | Yes | `lastMove`/`lastMoveBefore`/`lastTwoMoves` sequencing only | Good |
| Gremlin Leader | `gremlin_slots` | Yes | Yes | Yes | Rally/Encourage/Stab sequencing only | Good |
| Gremlin Wizard | `current_charge` | Yes | Yes | Yes | None for charge cadence | Good |
| Cultist | `first_move` | Yes | Yes | Yes | None | Good |
| Jaw Worm | `first_move`, `hard_mode` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Slime Boss | `first_turn` | Yes | Yes | Yes | None for post-opening cycle | Good |
| Large Slimes | `split_triggered` | Yes | Yes | Yes | Attack/debuff sequencing only | Good |
| Sentry | `first_move` | Yes | Yes | Yes | Later Bolt/Beam alternation only | Good |
| Spheric Guardian | `first_move`, `second_move` | Yes | Yes | Yes | Post-opening `lastMove(BIG_ATTACK)` branch only | Good |
| The Collector | `initial_spawn`, `ult_used`, `turns_taken`, `enemy_slots` | Yes | Yes | Yes | `lastMove(REVIVE)` / `lastTwoMoves(FIREBALL)` sequencing only | Good |
| Champ | `first_turn`, `num_turns`, `forge_times`, `threshold_reached` | Yes | Yes | Yes | `lastMove`/`lastMoveBefore` sequencing only | Good |
| Darkling | `first_move`, `nip_dmg` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Reptomancer | `first_move`, `dagger_slots` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Nemesis | `first_move`, `scythe_cooldown` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Giant Head | `count` | Yes | Yes | Yes | `lastTwoMoves(GLARE/COUNT)` sequencing only | Good |
| Time Eater | `used_haste` | Yes | Yes | Yes | `lastMove` / `lastTwoMoves` sequencing only | Good |
| Donu | `is_attacking` | Yes | Yes | Yes | None | Good |
| Deca | `is_attacking` | Yes | Yes | Yes | None | Good |

## Notes

### Byrd

- `runtime_state.first_move` and `runtime_state.is_flying` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is intentional Java sequence logic, not hidden-state recovery.

### Chosen

- `runtime_state.first_turn` and `runtime_state.used_hex` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is limited to the Java branch that avoids repeating `DRAIN` and `DEBILITATE`.

### Louse

- `runtime_state.bite_damage` is now treated as strict protocol truth.
- Rust no longer falls back to `move_base_damage` during split truth import.
- This removes the last hidden-state recovery path for Louse parity.

### Looter and Mugger

- `runtime_state.slash_count` and `runtime_state.stolen_gold` are now exported by `CommunicationMod`.
- Rust semantic execution updates thief runtime explicitly when gold is stolen.
- Death rewards now rely on seeded thief runtime truth instead of reconstructing it from sequence history.

### Shelled Parasite

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_move` to be protocol-seeded or factory-seeded.
- `move_history` is still used for Java's explicit `lastMove` / `lastTwoMoves` branching only.

### Healer

- `Healer` does not require hidden runtime truth from protocol.
- The semantic migration exposed a small framework gap, so group-heal intent is now represented explicitly as `MonsterMoveSpec::Heal(HealSpec { target: AllMonsters, ... })`.
- Execution still expands group heal and group strength buff into per-monster `Action::Heal` / `Action::ApplyPower` calls in `take_turn_plan`, which keeps the target set explicit and avoids pretending helpers already support generic group targeting.
- Action-family review aligned two generic handlers with Java:
  - `handle_heal` now ignores `is_dying` monster targets, matching `AbstractCreature.heal(...)`.
  - `handle_apply_power` now ignores escaped monster targets, matching `ApplyPowerAction.update()`.
- `RollMonsterMove` was reviewed but left unchanged in this pass; Java itself does not add an extra dead/escaped guard there, and changing it now risks interfering with half-dead/revival style monsters.

### Snake Plant

- `Snake Plant` does not require hidden runtime truth from protocol.
- The semantic migration is purely a Java-sequence port:
  - `lastTwoMoves(CHOMPY_CHOMPS)`
  - `lastMove(SPORES)`
  - and at A17 specifically `lastMoveBefore(SPORES)`
- `use_pre_battle_action` is now wired through semantic dispatch so `Malleable` no longer depends on legacy paths.
- `on_death` is also routed through semantic dispatch to the default no-op implementation, which removes one more unsupported hallway-monster death edge.

### Large Slimes

- `runtime_state.split_triggered` is exported for `AcidSlime_L` and `SpikeSlime_L`.
- Rust uses the private Java latch in addition to `nextMove != SPLIT`; this covers states where a later roll temporarily changes the planned move while Java still remembers that the split interrupt already fired.
- The latch is updated immediately when the split interrupt fires. It is not queued as a Java action; only the Java `SetMoveAction` equivalent remains queued behind existing actions.

### Snecko

- `runtime_state.first_turn` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_turn` to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit `lastTwoMoves(BITE)` rule.

### Red Slaver

- `runtime_state.first_turn` and `runtime_state.used_entangle` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit repeat rules around `STAB` and `SCRAPE`.

### Gremlin Nob

- `runtime_state.used_bellow` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this latch to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit `SKULL_BASH` / `BULL_RUSH` repeat rules.

### Gremlin Leader

- `runtime_state.gremlin_slots` is now exported by `CommunicationMod` from Java
  `GremlinLeader.gremlins`.
- Rust Rally execution uses the exported slot members to find the first null/dying slot, matching
  Java `SummonGremlinAction.identifySlot(...)`.
- Draw positions are still used to place the summoned monster in the same coordinate frame, but
  they are no longer the source of slot occupancy truth.

### Gremlin Wizard

- `runtime_state.current_charge` is now exported by `CommunicationMod`.
- Rust semantic turn execution requires this counter to be protocol-seeded or factory-seeded.
- Charge cadence comes from Java's private `currentCharge`, not from consecutive Charge history.
- Runtime patches are emitted during `take_turn_plan`, because Java mutates `currentCharge` inside
  `takeTurn()` rather than `getMove()`.

### Cultist

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires it to be protocol-seeded or factory-seeded.
- The opening Incantation gate comes from this Java private field, not from empty move history.

### Jaw Worm

- `runtime_state.first_move` and `runtime_state.hard_mode` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening Chomp gate comes from Java's private `firstMove`, not from empty move history.
- Jaw Worm Horde uses Java hard mode: `hard_mode=true` and `first_move=false`; the pre-battle
  Strength/Block bonus still comes from `hard_mode`.
- Remaining history usage is limited to Java's explicit Chomp/Bellow/Thrash repeat rules.

### Slime Boss

- `runtime_state.first_turn` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires this field to be protocol-seeded or factory-seeded.
- The opening Sticky gate comes from Java's private `firstTurn`, not from empty move history.
- After `first_turn=false`, Java `getMove()` is a no-op; Rust therefore preserves the current
  planned move if a roll is requested, while the ordinary cycle remains driven by `takeTurn()`
  `SetMonsterMove` actions.

### Sentry

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires it to be protocol-seeded or factory-seeded.
- Opening Bolt/Beam parity uses the monster slot only while `first_move` is true. After that, move
  history is used only for Java's explicit Bolt/Beam alternation.

### Spheric Guardian

- `runtime_state.first_move` and `runtime_state.second_move` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires both fields to be protocol-seeded or factory-seeded.
- The opening Harden and Bash+Frail gates come from Java's private latches, not from move-history
  length. After both latches are false, move history is used only for Java's explicit
  `lastMove(BIG_ATTACK)` branch.

### The Collector

- `runtime_state.initial_spawn`, `ult_used`, `turns_taken`, and `enemy_slots` are exported by
  `CommunicationMod`.
- `enemy_slots` is mapped from Java monster instance ids to Rust entity ids during state sync.
- Rust move selection and revive execution use the current slot members, not a scan of every
  TorchHead in the monster group. This matches Java's private `enemySlots` map and avoids reviving
  stale dying TorchHead objects left behind after prior revives.

### Champ

- `runtime_state.first_turn`, `num_turns`, `forge_times`, and `threshold_reached` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires this runtime truth to be protocol-seeded or factory-seeded before branch selection runs.
- `Champ` also uses the new semantic `on_roll_move` hook to model Java's `getMove()` side effects explicitly, instead of recovering them later from `move_history`.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastMoveBefore` sequencing rules around `EXECUTE` and `GLOAT`.

### Darkling

- `runtime_state.first_move` and `runtime_state.nip_dmg` are exported by `CommunicationMod`.
- Rust state sync marks the Darkling runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java clears `firstMove` inside `getMove()` only when the opening branch is consumed. Rust now
  mirrors that through `Action::UpdateMonsterRuntime`; generic `SetMonsterMove` and
  `RollMonsterMove` no longer clear the flag as a side effect.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastTwoMoves` branching.

### Reptomancer

- `runtime_state.first_move` and `runtime_state.dagger_slots` are exported by `CommunicationMod`.
- Rust state sync maps Java dagger slot monster instance ids to Rust entity ids in a second pass,
  matching the Java private `daggers[4]` array instead of deriving occupancy from draw position.
- Spawn Dagger execution now uses `Action::SpawnReptomancerDagger`, which spawns the dagger and
  updates the corresponding runtime slot together.
- Remaining history usage is limited to Java's explicit repeat rules around Snake Strike, Spawn
  Dagger, and Big Bite.

### Nemesis

- `runtime_state.first_move` and `runtime_state.scythe_cooldown` are exported by
  `CommunicationMod`.
- Rust state sync marks the Nemesis runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java decrements `scytheCooldown` at the start of every `getMove()` and resets it to 2 only when
  Scythe is selected. Rust mirrors this through `Action::UpdateMonsterRuntime` emitted from
  `on_roll_move`.
- The opening branch is gated by Java private `firstMove`, not by empty move history.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastTwoMoves` repeat rules.

### Giant Head

- `runtime_state.count` is exported by `CommunicationMod`.
- Rust state sync marks the Giant Head runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java initializes `count=5`, decrements it once in `usePreBattleAction()` at A18+, and then
  decrements it inside every `getMove()` until it reaches -6. Rust mirrors the A18 pre-battle
  mutation immediately in the pre-battle hook and roll-time mutation through
  `Action::UpdateMonsterRuntime`.
- `It Is Time` damage is computed from the Java private count after the roll-time decrement, not
  from move-history length.
- Remaining history usage is limited to Java's explicit Glare/Count repeat rules.

### Time Eater

- `runtime_state.used_haste` is exported by `CommunicationMod`.
- Rust state sync marks the Time Eater runtime slice as protocol-seeded and semantic roll logic
  requires factory/protocol seeding before branch selection.
- Java sets `usedHaste=true` inside `getMove()` when the half-HP Haste branch is selected. Rust
  mirrors that roll-time mutation through `Action::UpdateMonsterRuntime`.
- Java private `firstTurn` controls only dialogue in `takeTurn()` and is intentionally omitted.
- Haste heal amount remains execution-time state, matching Java's queued
  `HealAction(this.maxHealth / 2 - this.currentHealth)`.
- Remaining history usage is limited to Java's explicit repeat rules around Reverberate, Head Slam,
  and Ripple.

### Donu / Deca

- `runtime_state.is_attacking` is exported by `CommunicationMod` for both Donu and Deca.
- Rust state sync marks both runtime slices as protocol-seeded and semantic roll logic requires
  factory/protocol seeding before branch selection.
- Java mutates `isAttacking` inside `takeTurn()` after queueing the branch's visible actions and
  before queueing `RollMoveAction`. Rust mirrors that with `Action::UpdateMonsterRuntime`
  immediately before `Action::RollMonsterMove`.
- Rust no longer derives their alternation from `move_history`.

### The Guardian

- `runtime_state.guardian_threshold`, `damage_taken`, `is_open`, and `close_up_triggered` are now all treated as strict protocol truth.
- Rust split import seeds `entity.guardian` directly from those fields instead of silently falling back to the runtime default.
- This closes the live parity bug where imported Guardian state kept `damage_threshold = 0`, causing any positive hit to mis-trigger defensive mode.

## Outcome

Phase 2 is complete for the migrated hallway/stateful monsters that currently matter for live act1/act2 frontier work:

- hidden runtime truth no longer falls back to `move_history` for Byrd and Chosen
- thief runtime truth is explicit and protocol-seeded for Looter and Mugger
- Louse truth import is now strict
- remaining stateful debt is concentrated in monsters that have not completed semantic migration
