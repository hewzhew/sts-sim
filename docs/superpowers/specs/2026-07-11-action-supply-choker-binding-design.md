# Action-Supply and Choker-Binding Design

## Goal

Model Velvet Choker around the supply of playable actions and evidence that its six-card limit
actually binds. Do not treat all generated cards as equivalent, and do not turn a single opening
option such as Enchiridion into a permanent Choker liability.

## Problem

The current startup profile counts Enchiridion and Toolbox as generated opening options and then
derives Choker/Pyramid combination flags from their presence. Those flags are only evidence today,
but the abstraction is still too broad:

- Enchiridion adds one Power before the opening draw, makes it cost zero for that turn, and then
  does not generate another card during the combat.
- Toolbox is also a once-per-combat opening option, but the selected card keeps its normal
  evaluated cost.
- Nilry's Codex offers delayed supply once per turn by shuffling a selected card into the draw
  pile.
- Dead Branch can add a card to the hand after every Exhaust trigger and can participate in
  same-turn chains.
- Blade Dance-like fan-out, Corruption/Dead Branch chains, and additional-play effects such as
  Double Tap can consume several Choker slots in one turn even though they do not share one
  generic "generated card" shape.

The combat evaluator already caps its affordable-card count by Choker's remaining slots. That is
correct for capacity, but it collapses two different states into the same result: a hand with no
additional affordable actions and a hand with several affordable actions stranded by the cap.

The run-debt model separately records Choker's hard rule as `CardPlayCapDebt`. A hard rule is a
truthful constraint, but the presence of the constraint does not prove that the current deck is
exposed to it or that a particular combat line lost value because of it.

## Decision

### Separate constraint, exposure, and binding

Choker reasoning has three layers:

1. **Constraint:** Velvet Choker is present and allows at most six card plays per turn. This is a
   simulator fact and may remain visible as a run-debt contract.
2. **Exposure:** the current deck or relic set contains mechanics that can repeatedly or
   immediately supply enough actions to approach the limit. Exposure is categorical evidence,
   not a score and not proof of harm.
3. **Binding:** in a concrete combat state, affordable actions exceed the remaining Choker slots.
   Binding is exact state evidence. It still does not claim that every stranded action was
   strategically valuable.

Decision consumers may not add ad hoc card/relic-ID branches or infer binding from an ID. The
central mechanics registry may map IDs to structural traits, and combat state proves binding. In
particular, an opening-once source is not repeatable exposure merely because its generated card
is cheap.

### Represent action-supply shape in mechanics semantics

Extend the existing card/relic mechanics boundary with small categorical action-supply traits.
The traits describe how actions become available rather than assigning source-specific scores:

- `OpeningOnce`: contributes an option once, before or during the opening hand.
- `DelayedPerTurn`: contributes an option for a future draw or later turn on a recurring cadence.
- `SameTurnBurst`: one action immediately creates multiple potential follow-up plays.
- `TriggeredRepeatable`: a trigger may supply another action multiple times in one combat or turn.
- `AdditionalPlay`: an effect plays a card or copy in addition to the initiating action; these
  plays consume Choker slots even when they do not originate from the hand.
- `CostOrResourceCompression`: draw, energy, or cost changes can make an existing action supply
  executable. This is supporting exposure, not card generation.
- `Recursive`: a supplied action can reproduce the same supply path.

Traits may carry only stable structural details needed by consumers: cadence, immediate-versus-
delayed delivery, minimum known fan-out, temporary zero-cost status, and optionality. Unknown or
random card identity is not converted into an average card value.

The first implementation should use the existing `card_semantics_v1` mechanics profiles (and the
corresponding relic profile) as the registry boundary. It must not add another independent list
inside boss-relic admission or combat search.

### Aggregate facts without an action-pressure score

An `ActionSupplyProfileV1` derived from `RunState` aggregates the mechanics traits and preserves
source labels for explanation. It exposes facts such as:

- count of opening-once options;
- presence of same-turn burst supply;
- presence of triggered repeatable or recursive supply;
- presence of additional-play effects;
- presence of cost/resource compression that can activate a large supply.

The profile does not calculate a total action-pressure score. A source may belong to more than
one category, and a category count is not itself a penalty.

Enchiridion is classified as opening-once, immediate-hand, one option, zero-cost-this-turn,
non-recursive. It may remain visible in diagnostics but does not create repeatable Choker
exposure. Toolbox is also opening-once and non-recursive. Nilry's Codex is delayed-per-turn and
optional. Dead Branch is triggered-repeatable, immediate-hand, and potentially recursive.

Runic Pyramid is not action supply. It changes when existing hand options remain available and
therefore belongs to retention/hand-capacity reasoning. Retaining an unplayed Enchiridion Power
after its turn-only discount expires is a concrete combat-state tradeoff, not a static
Pyramid/Choker/Enchiridion combination liability.

### Preserve reminders but stop candidate-specific misattribution

The existing generated-opening counters remain useful raw diagnostics. In the first pass, the
serialized combination flags remain present and readable for compatibility, but are explicitly
deprecated and no new decision consumes them as proof of Choker exposure. Their later removal
requires a separate schema-migration decision.

Pyramid boss-relic admission must not report a Choker/generated-opening requirement solely
because projected Pyramid completes the three-relic combination. Remove the candidate-specific
`OpeningActionBudgetRequired` reason and its formatter after its emission is removed. A neutral
action-supply report may still say that one opening-once option exists. This preserves the
reminder without assigning an already-existing opening condition to the Pyramid candidate.

`CardPlayCapDebt` remains the statement that Choker imposes a long-term construction constraint.
The first pass must not add a new rank adjustment for action-supply traits. Any later ranking
change requires bounded evidence that the cap binds and strands useful actions; this design does
not replace one speculative fixed score with another.

### Report exact combat binding separately from card value

Combat search derives a small Choker-capacity report from the real state:

- cards already played this turn;
- remaining Choker slots, saturating at zero;
- energy-affordable cards currently in hand before applying the cap;
- affordable cards representable after applying the cap;
- affordable-card count stranded by the cap.

Generated cards already in hand participate naturally. Additional-play effects continue to obey
the simulator's legal-action and card-play counters; the report does not predict copies that have
not resolved yet.

`stranded_affordable_cards > 0` means the cap is binding on action quantity. It does not mean the
stranded cards are all worth playing. Combat search remains responsible for choosing actions and
evaluating child states. A future evidence pass may compare the best legal line with a bounded
counterfactual, but that is not required for the first implementation.

## Data Flow

1. Card and relic mechanics profiles classify stable action-supply traits once.
2. `ActionSupplyProfileV1` aggregates those traits from the current deck and relics without
   applying a score.
3. Strategic diagnostics distinguish Choker's structural constraint from repeatable/burst
   exposure. Opening-once supply stays visible but neutral.
4. Boss-relic admission may display the categorical exposure facts but does not attribute an
   existing Enchiridion opening option to a projected Pyramid candidate.
5. During combat, the simulator remains authoritative for generated cards, extra plays, Choker
   legality, and counters.
6. Combat evaluation reports whether the current cap is binding and how many affordable cards
   are stranded, without guessing their strategic value from card names.

## Unknown and Fallback Behavior

- A card or relic with no action-supply trait contributes no static exposure. Unknown must not be
  treated as harmful.
- Randomly generated card identity remains unknown until the simulator constructs the concrete
  card.
- Counts use saturating arithmetic.
- New serialized facts use Serde defaults so older evidence remains readable.
- Simulator legality wins over every strategic or diagnostic profile.

## Stable Tests

- Enchiridion is opening-once, immediate-hand, zero-cost-this-turn, and non-recursive.
- Enchiridion plus Choker does not create repeatable action-supply exposure or a new burden.
- Toolbox is opening-once without Enchiridion's temporary zero-cost fact.
- Nilry's Codex is optional delayed-per-turn supply rather than same-turn burst supply.
- Dead Branch is triggered-repeatable immediate-hand supply and can be marked recursive.
- Blade Dance is representative same-turn burst supply.
- Double Tap is representative additional-play supply classified independently of card
  generation.
- Pyramid admission does not gain a candidate-specific Choker/generated-opening reason from an
  already-owned Enchiridion.
- With five Choker plays used and three affordable hand cards, the capacity report shows one
  representable action and two stranded affordable cards.
- With fewer affordable cards than remaining slots, the report shows no binding.
- Unknown mechanics do not create exposure or penalties.

Tests assert categorical mechanics and exact capacity relationships. They do not lock aggregate
scores, random generated identities, complete seed paths, or boss outcomes.

## Non-Goals

- Do not assign average values to random Enchiridion, Toolbox, Codex, or Dead Branch results.
- Do not build an exhaustive card-by-card Choker blacklist.
- Do not infer strategic harm merely because six plays are reachable.
- Do not redesign combat action ordering or the full state-value function.
- Do not run a complete seed as a regression test for this pass.
- Do not merge Time Eater's twelve-card cycle with Choker's per-turn six-card constraint.
- Do not change Choker acquisition ranking until bounded binding evidence justifies a separate
  behavior change.
