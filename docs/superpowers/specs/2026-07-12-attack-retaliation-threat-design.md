# Attack Retaliation Threat Design

## Goal

Make combat-search action ordering understand the immediate HP price of damaging an enemy that
reacts to attacks. The motivating A3F42 line correctly kills two Exploders, then spends roughly
50 gross HP across four attacks into a Spiker whose Thorns has grown from 3 to 15. An eight-second
search without the early HP-loss stop improves the final HP by only three, so this is not merely a
budget shortage.

This slice adds a generic, engine-derived attack-retaliation fact and routes it through the existing
reactive-risk ordering. It does not add a Spiker-specific score.

## Evidence Boundary

The simulator already executes `resolve_power_on_attacked` exactly. Combat search also resolves a
card into its ordered damage actions, including one action per hit for cards such as Twin Strike
and Pummel. The missing link is diagnostic and ordering knowledge: `card_play_effect_facts`
observes `on_card_played` reactions such as Sharp Hide, but does not observe ordinary `on_attacked`
reactions such as Thorns.

The new fact is a current-state hint derived from visible powers and resolved card actions. It does
not claim a future plan is optimal, and it may conservatively count queued hits that exact execution
later suppresses because an earlier hit killed the owner. Exact combat simulation remains the source
of truth.

## Chosen Design

### Engine-derived retaliation fact

Add a small combat-search fact module that asks each visible target power what one ordinary player
damage event would trigger through `resolve_power_on_attacked`. It records only returned player HP
damage and keeps source attribution separate from strategic value.

The target-level fact exposes:

- retaliation damage per ordinary player damage event;
- the visible power sources contributing to that retaliation;
- visible pending Thorns growth when the owner's current intent applies more Thorns to itself.

The helper inspects powers and visible planned mechanics rather than matching `EnemyId::Spiker`.
This naturally covers ordinary Thorns wherever it appears. Sharp Hide remains on its existing
`on_card_played` path and must not be double counted.

### Card-action projection

While resolving a card's direct actions, observe every damage event aimed at a living enemy. For
each event, resolve that target's attack retaliation and accumulate:

- `attack_retaliation_trigger_count_hint`;
- `attack_retaliation_player_hp_loss_hint`.

The accumulated HP loss is also added to the existing `ReactiveCardPlayEffectFacts::player_hp_loss`.
That makes the current `reactive_risk_score` the single ordering consumer instead of creating a
parallel Spiker score.

Expected ordering behavior:

- an attack against a neutral target remains unchanged;
- an attack into Thorns sorts behind otherwise comparable damage without retaliation;
- among comparable attacks into the same Thorns target, fewer hits are preferred;
- among single-hit attacks with the same retaliation cost, existing damage progress still prefers
  the larger hit;
- lethal roles and immediate visible-lethal prevention keep their existing role ranks.

Conservative rollout already reuses shared action ordering, so it receives the fact without a
rollout-specific weight.

### Diagnostics

Project the target retaliation fact into action facts and add aggregate fields to the enemy mechanics
report for current retaliation and visible pending growth. Keep the fields descriptive; frontier
value does not consume them in this slice.

Action diagnostics must distinguish ordinary reactive HP loss from the attack-retaliation portion,
so a future investigation can explain why a multi-hit action was delayed.

## Alternatives Rejected

### Spiker-specific target priority

Matching `EnemyId::Spiker` and assigning a target penalty is smaller but duplicates engine rules,
does not explain multi-hit cost, and would require more enemy exceptions later.

### Full minimum-HP scoring

Tracking exact intra-action minimum HP is valuable but crosses the engine-step boundary. A terminal
attack can apply Thorns and then Black Blood before the next stable state, so a `SearchNode` field
updated only after stable actions would produce false precision. Exact intra-action attrition gets a
separate design after this slice; no hard minimum-HP rejection is added here.

## Tests

Focused contracts cover:

- a single-hit attack into Thorns reports one retaliation trigger and the correct HP loss;
- Twin Strike reports two triggers and twice the HP loss;
- non-attack and neutral-target actions remain retaliation-neutral;
- existing Sharp Hide facts are not double counted;
- ordering prefers a comparable single-hit attack over a multi-hit attack into Thorns;
- ordering remains unchanged without retaliation;
- visible pending Thorns growth appears in the enemy mechanics report;
- conservative rollout selects through shared retaliation-aware ordering.

Then run all action-fact, action-priority, action-ordering, rollout-selector, library, and architecture
boundary tests.

## Frozen-Case Verification

Rerun the A3F42 capture with the same eight-second lazy all-potion diagnostic and record:

- complete-win status and final HP;
- attacks and damage-event count directed at the Spiker;
- retaliation HP paid by the selected trajectory;
- whether Demon Form or non-attack damage becomes part of the selected line.

The frozen case is diagnostic evidence, not a permanent card-sequence test. If action ordering now
exposes and respects retaliation but the long search still selects four growing-Thorns attacks, stop
and design a separate cross-turn setup/frontier consumer. Do not add an untyped Spiker weight.

