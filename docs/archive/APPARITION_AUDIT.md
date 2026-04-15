# Apparition Audit

## Scope

This note audits `Apparition` as a full strategy/resource problem rather than a single
"play it earlier" heuristic.

The question is not whether the bot can legally click `Apparition`. The question is whether it:

- chooses the `Ghosts` event correctly
- commits to an Apparition route when it does choose it
- budgets a finite set of Apparitions across a fight and across the rest of the run
- understands when stacking is worthwhile
- understands the downstream cost of halving max HP

## Summary

Current behavior is not "Apparition mostly works with bad timing."

Current behavior is closer to:

1. `Ghosts` acceptance is over-simplified at run level.
2. Combat evaluation treats `Apparition` mostly as a current-turn panic button.
3. Existing scoring explicitly discourages stacking once any `Intangible` is already active.
4. Tests mostly cover "incoming damage is high this turn," but not "play now to secure next turn"
   or "budget a limited Apparition pool across the fight."
5. Engine semantics appear to support stacking at the power layer; the stronger evidence points to
   a planning/evaluation problem, not a current parity bug.

## Evidence From Recent Logs

### Window A: only Apparition remains, bot still ends turn

`live_comm_debug.txt`, `F312-F314`:

- `F312`: hand `[Apparition, Apparition, Defend, Intimidate, Apparition]`
- `F313`: after `Intimidate`, hand becomes `[Apparition, Apparition, Defend, Apparition, Apparition]`
- `F313`: bot plays `Defend`
- `F314`: hand is `[Apparition, Apparition, Apparition, Apparition]`
- `F314`: `END score=1109`, legal Apparition play is `1108`, bot chooses `EndTurn`

This is not an engine problem. This is a clear valuation/planning problem.

### Window B: upgraded Apparition is held when harmless, which is fine

`live_comm_debug.txt`, `F296-F304`:

- Byrds fight
- bot spends energy on `Dark Embrace`, `True Grit`, `Armaments`, `Defend`, `Defend`
- ends with `Apparition+` unplayed and `E=0`

This is not obviously wrong. It is an example of "holding Apparition can be correct."

The contrast with Window A is the key point:

- the system does sometimes hold correctly
- but the criteria are too narrow and too current-turn-focused

### Window C: Apparition is sometimes used correctly

`live_comm_debug.txt`, `F345-F346`:

- one enemy left
- player at `25/40`
- bot plays `Apparition+`
- gains `Intangible=1`

This only proves the bot can play the card. It does not prove good Apparition resource planning.

## Current Run-Level Logic Is Too Crude

`src/bot/noncombat_families.rs`, `choose_ghosts_option(...)`:

- accepts if `act_num <= 2` and `hp_ratio >= 0.55`
- otherwise refuses

This ignores:

- draw/filter density
- exhaust outlets
- hand smoothing
- whether the deck can convert temporary safety into actual wins
- whether the remaining path and boss make Apparition especially attractive

So the bot can accept `Ghosts` for a deck that cannot really operate Apparitions well.

## Current Combat Logic Is Too Narrow

### Current timing score

`src/bot/strategy_families.rs`, `apparition_timing_score(...)`:

- if `current_intangible > 0`, returns `-10_000`
- otherwise values Apparition using:
  - `current_hp`
  - `imminent_unblocked_damage`
  - `total_incoming_damage`
  - `apparitions_in_hand`
  - upgrade status
  - `Runic Pyramid`

This has two major implications:

1. Any existing `Intangible` makes another Apparition look strongly wrong.
2. The score is dominated by current-turn danger.

That means the system is not expressing:

- "spend 2 energy now to guarantee a safe next turn"
- "stack now because next-turn enemy scaling is severe"
- "stack now because this hand will be worse next turn"
- "stack now because this is one of the few fights where burning 2 Apparitions is correct"

### Current hand-shaping score

`apparition_hand_shaping_score(...)` reuses the same timing score.

So the same narrow assumptions leak into:

- topdeck decisions
- hand selection shaping
- delay/keep decisions

## Existing Tests Are Too Local

Relevant tests currently focus on:

- frontloading Apparition under high current incoming
- holding upgraded Apparition when safe
- special cases like `Runic Pyramid`

Examples:

- `unupgraded_apparition_is_frontloaded_under_incoming_damage`
- `upgraded_apparition_can_be_held_when_safe`
- `upgraded_apparition_is_frontloaded_in_runic_pyramid_lethal_window`
- `apparition_priority_spikes_in_multi_hit_frail_window`

What is missing:

- stacking while `Intangible` is already active
- spending an Apparition to protect the next turn instead of the current turn
- budgeting Apparitions across an entire boss or elite fight
- preserving one Apparition for a later, more dangerous combat

## Engine Semantics Check

Current engine evidence points to Apparition stacking being supported at the power layer:

- `CardId::Apparition` applies `PowerId::IntangiblePlayer`
- `handle_apply_power(...)` stacks existing power amounts by `existing.amount += amount`
- `IntangiblePlayer` decays via `ApplyPower(-1)` at end of round

So if the player has 1 stack and receives another +1, the power layer should become 2.

What the bot currently does not understand is the strategic value of that stack.

The strongest evidence is the explicit scoring rule:

- `if current_intangible > 0 { return -10_000; }`

That is a planning/evaluation problem, not a sign that the engine cannot stack.

## Proper Problem Decomposition

Apparition should be treated as a four-layer problem.

### 1. Run-level route selection

Questions:

- Should this deck take `Ghosts` at all?
- If it does, should the deck start prioritizing draw/filter/exhaust tools differently?
- Does the current boss/path make the route unusually strong or unusually risky?

### 2. Fight-level budgeting

Questions:

- How many Apparitions should this fight consume?
- Is this a short burst fight, a scaling fight, or a stallable fight?
- Is spending two Apparitions in one combat correct?
- Should one be saved for the next elite/boss?

### 3. Turn-level usage

Questions:

- Should Apparition be played this turn?
- Is the reason current incoming, next-turn danger, or hand quality decay?
- Is stacking correct?
- Is holding correct even though the card is playable?

### 4. Engine/state semantics

Questions:

- Does `IntangiblePlayer` stack and decay correctly?
- Do search and evaluation read the correct current stack?
- Are turn boundaries represented correctly for Apparition planning?

## Recommended Next Steps

Do not start with another small heuristic patch.

Recommended order:

1. Add dedicated Apparition audit tests that cover:
   - stacking while `IntangiblePlayer > 0`
   - playing to secure the next turn instead of this turn
   - saving Apparition in a harmless window
   - spending multiple Apparitions in one scaling fight when justified
2. Split Apparition evaluation into explicit layers:
   - route acceptance
   - fight budget
   - turn timing
3. Remove the blanket rule that any active `Intangible` makes another Apparition strongly bad.
4. Add a fight-level "remaining Apparition budget" feature instead of only checking hand count.
5. Revisit `Ghosts` acceptance so it stops being a pure `act/hp_ratio` rule.

## Related Follow-Up: Slimed

The recent `Slimed` behavior appears to be a separate tactical issue:

- spare energy is often left unused
- `Slimed` remains in hand/deck longer than it should

That problem looks much more like a tactical/end-turn rule problem than an Apparition planning
problem, and should be handled separately.
