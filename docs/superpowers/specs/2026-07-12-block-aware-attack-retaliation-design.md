# Block-Aware Attack Retaliation Design

## Goal

Make attack-retaliation facts describe three different quantities truthfully:

- raw damage emitted by retaliation hooks;
- player block consumed while resolving that damage;
- player HP actually lost after the engine damage pipeline.

The result must remain generic and engine-derived. It must not add a Spiker-specific rule, a fixed retaliation coefficient, or path/frontier behavior.

## Root Cause

`attack_retaliation_player_hp_loss_for_event` currently resolves target `on_attacked` hooks and sums `DamageInfo.output.max(base)`. That value is the retaliation action's raw damage, not player HP loss.

The real player damage path subsequently applies final-receive powers such as Intangible, deducts block, applies relic and power `onAttackedToChangeDamage` hooks such as Torii and Buffer, and finally applies `onLoseHpLast` such as Tungsten Rod. Calling the pre-defense value “HP loss” therefore overstates danger whenever any of those defenses are present. It also hides the fact that fully blocked retaliation still consumes a finite resource.

## Considered Approaches

### 1. Share the engine's numeric player-damage resolver (recommended)

Extract the numeric stages of `handle_damage` into one engine-owned resolver. The live damage handler and AI projection both call it. The AI projects a sequence on a lazily cloned combat state so block and stateful defenses such as Buffer are consumed in the same order without mutating the real state.

This gives one source of truth and keeps sequential multi-hit projections correct. The clone is created only after an actual retaliation action is found, so non-retaliation cards keep the current cheap path.

### 2. Infer retaliation from the exact child-state delta

This avoids another projection, but the delta mixes retaliation with block gained by the card, other reactive damage, healing, and unrelated side effects. It cannot attribute raw damage or block consumption reliably. Rejected.

### 3. Reimplement the damage formula in combat-search code

This would be initially small, but it would duplicate hook ordering and immediately risk drifting when the engine gains another defensive power or relic. Rejected.

## Engine Boundary

Add an engine-owned `PlayerDamageResolution` returned by a shared numeric resolver. It reports:

- `raw_damage`: the calculated damage before player defenses;
- `block_consumed`: the decrease in player block;
- `damage_before_hp_loss_hooks`: the value passed to existing `on_attacked` hooks before Tungsten Rod;
- `hp_loss`: damage remaining after `onLoseHpLast`.

The resolver owns the existing ordering of final receive, block, relic modifiers, power modifiers, and final HP-loss modifiers. It may mutate defensive state needed by those stages, notably block and Buffer. It does not subtract HP, enqueue hooks, update cards, or perform death/revival. `handle_damage` performs those side effects after calling the resolver, preserving current engine behavior.

Combat search never calls the resolver on the live state. Its retaliation observer lazily clones the combat state once per candidate card and applies every emitted retaliation damage action to that scratch state in action order. This preserves sequential block and Buffer consumption across multi-hit attacks.

`LoseHp` retaliation actions, if a generic power emits one, remain direct HP loss and consume no block. This slice does not reinterpret all unrelated `on_card_played` reactive actions; it fixes only damage attributed to attack retaliation.

## Combat-Search Facts

For each candidate card, retain and expose:

- `attack_retaliation_trigger_count_hint`: number of positive retaliation damage/HP-loss events, even when defense reduces HP loss to zero;
- `attack_retaliation_raw_player_damage_hint`: raw emitted damage;
- `attack_retaliation_player_block_loss_hint`: sequentially consumed current block;
- `attack_retaliation_player_hp_loss_hint`: actual projected HP loss.

`reactive.player_hp_loss` receives only projected HP loss. Reactive ordering risk includes both retaliation block loss and HP loss, because consuming three block and losing zero HP is safer than losing three HP but is not free. Raw damage remains diagnostic and does not receive an additional score, preventing double counting.

Target facts describe the current retaliation source with truthful names:

- `raw_player_damage_per_damage_event`;
- `projected_player_block_loss_for_next_damage_event`;
- `projected_player_hp_loss_for_next_damage_event`;
- existing power-source count and visible growth.

The misleading `player_hp_loss_per_damage_event` field is removed rather than preserved with false semantics.

## Diagnostics

Ordering action samples expose raw damage, block loss, and HP loss separately. Search-wide counters count an action whenever raw retaliation is positive, including fully blocked actions, and aggregate:

- retaliation action observations;
- trigger observations;
- raw damage hints;
- block-loss hints;
- HP-loss hints;
- maximum HP-loss hint for one candidate.

These remain candidate observations, not unique paths and not realized trajectory loss. The schema change is intentionally corrective: downstream evidence must not treat an old raw-damage field as true HP loss.

## Testing

The implementation uses focused red/green tests for:

1. engine parity: the shared resolver and live `handle_damage` agree on block consumption and HP loss;
2. a fully blocked single Thorns event reports raw damage and block loss but zero HP loss;
3. two Thorns events consume shared block sequentially, then lose HP;
4. Intangible, Buffer, and Tungsten Rod remain governed by the engine resolver rather than AI special cases;
5. trigger counting and diagnostic action counting remain nonzero when HP loss is fully blocked;
6. reactive ordering treats consumed block as defense-resource loss without adding raw damage again.

After focused tests, run formatting, the full library suite, and `architecture_runtime_boundaries`. Rerun the frozen A3F42 diagnostic only as ignored evidence, compare its new raw/block/HP counters, and do not turn the trajectory into a permanent assertion.

## Non-Goals

- no Spiker or Shapes-specific target priority;
- no fixed scalar conversion from retaliation to progress;
- no path ledger, Pareto frontier, pruning, or state-identity change;
- no general rewrite of every reactive-card hook;
- no duplicate AI-owned player damage pipeline;
- no permanent frozen-seed trajectory test.
