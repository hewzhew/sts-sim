# Repair-Aware Opening Capacity Design

## Goal

Model Runic Pyramid as a powerful retention tool whose value depends on current coverage,
available repair paths, and exact combat capacity. Do not turn unupgraded Apparitions or
generated opening cards into permanent penalties merely because the current strategy does not
yet use them well.

## Problem

The current run-level model treats `RunicPyramid + unupgraded Apparition` as a newly introduced
startup liability. That interpretation is too strong:

- Runic Pyramid does not retain an unupgraded Ethereal Apparition; the card exhausts normally.
  Pyramid therefore fails to extend that card's lifetime, but does not create the expiration.
- Armaments can repair an Apparition in combat, and Armaments+ can repair every eligible card
  currently in hand. Apotheosis is another live combat repair path.
- After an Act 1 or Act 2 boss relic decision, permanent upgrade opportunities can still exist.
  Their exact count is unknown before the next map is available, so they are potential repair,
  not guaranteed completion.

The combat evaluator has a separate capacity blind spot. It counts every energy-affordable
hand card as playable even when Velvet Choker has only one or zero card plays remaining. It also
estimates the next draw without accounting for cards that Runic Pyramid would keep against the
ten-card hand limit. Enchiridion and Toolbox cards are generated before the opening draw and
enter the real hand, but this capacity cost is not reflected in those estimates.

## Decision

### Repair-aware Pyramid coverage

Extend `DeckStartupProfileV1` with a categorical Apparition retention-coverage result:

- `NotApplicable`: Pyramid or Apparition is absent.
- `Ready`: every Apparition is already upgraded.
- `CombatRepairAvailable`: at least one unupgraded Apparition remains and the deck has live
  Armaments or Apotheosis access. The profile separately records whether the access can upgrade
  one selected card or the whole current hand.
- `FutureUpgradeWindow`: no live combat repair exists, but `run_state.act_num <= 2`, so future
  permanent upgrades may still be acquired.
- `Limited`: unupgraded Apparitions remain and neither a live repair nor a future permanent
  upgrade window is visible from the current run phase.

The existing `has_pyramid_unupgraded_apparition` field remains as a raw compatibility fact so
serialized consumers do not lose the field. It no longer increases `combat_shape_risk` and is
not, by itself, an introduced startup liability.

New serialized profile fields use Serde defaults so older stored profiles remain readable. The
coverage enum's default is `NotApplicable`; missing generated-option counters and tradeoff flags
default to zero and false.

`FutureUpgradeWindow` is intentionally weaker than `CombatRepairAvailable`: it may explain why
coverage is not permanent, but it cannot certify a boss-relic mainline choice by itself.

### Generated opening options and Choker budgeting

The startup profile records known generated opening options without assigning card-name scores:

- Enchiridion contributes one generated Power option that costs zero during its opening turn.
- Toolbox contributes one chosen colorless option at its normal evaluated cost.

These are options, not mandatory plays. The profile exposes categorical evidence when Velvet
Choker must budget opening actions around them, and when Runic Pyramid can retain an unplayed
non-Ethereal option at the cost of future hand capacity. Neither flag is a standalone burden or
a reason to reject Pyramid, Enchiridion, or Toolbox.

Other relic-generated opening cards are outside this pass. They can join the same fact model
later when a concrete decision requires them.

### Boss-relic admission consumes coverage, not fear

Runic Pyramid admission reports its Apparition coverage and opening-action budgeting evidence.
It must not emit `IntroducesStartupLiability` solely because projected Pyramid makes
`has_pyramid_unupgraded_apparition` true.

`Ready` and `CombatRepairAvailable` are positive support evidence. `FutureUpgradeWindow` and
`Limited` remain inspectable uncertainty. StrategicPower's default lane remains `Probe`; this
pass removes a false burden and adds truthful evidence rather than declaring Pyramid universally
best or promoting it through a new aggregate score.

The existing categorical boss-relic order remains authoritative. In a same-lane offer, removing
the false startup burden allows Pyramid's strategic class to compete normally. Explicit Act 2
energy-gap priority and real run-debt treatment remain unchanged.

### Combat evaluation uses exact remaining capacity

The simulator remains authoritative for legal actions and end-of-turn behavior. The search
evaluator aligns its estimates with that state:

1. If Velvet Choker is present, remaining card-play capacity is
   `6 - cards_played_this_turn`, saturating at zero.
2. `hand.playable_cards` cannot exceed that remaining capacity. Energy affordability is still
   required; this change only removes impossible extra plays from the estimate.
3. Next-turn draw count uses the existing simulator helper, including modifiers such as Snecko
   Eye's draw bonus.
4. If the current turn ended at the evaluated state, projected retained cards are:
   - with Runic Pyramid: every non-Ethereal hand card, plus Ethereal cards with explicit or
     intrinsic retain;
   - without Runic Pyramid: only cards with explicit or intrinsic retain.
5. The next-draw estimate is capped by `10 - projected_retained_cards` before reading the top of
   the draw pile.

This makes Enchiridion and Toolbox participate automatically after their concrete generated
cards enter the hand. No special combat-search score is attached to either relic or to a random
generated card identity.

The capacity calculation describes "end the turn from this state." A child state that plays a
card releases a hand slot and receives its own fresh evaluation, so the model does not guess
which current card will be played later.

## Data Flow

1. `DeckStartupProfileV1` derives raw Apparition counts, combat upgrade access, future-upgrade
   availability, generated opening options, and Choker/Pyramid tradeoff facts from `RunState`.
2. Projected boss-relic admission clones the run, adds Pyramid only to the clone, and compares
   repair-aware coverage without mutating the live state.
3. Boss-relic annotation records coverage/support evidence and no longer converts uncovered
   Apparitions into a hard startup burden.
4. Natural combat start generates Enchiridion and Toolbox cards through existing simulator
   actions before the opening draw.
5. Combat search reads the resulting real hand, Choker counter, Pyramid ownership, Ethereal and
   retain semantics, and hand capacity when constructing state-value facts.

## Stable Tests

- Pyramid plus already upgraded Apparitions reports `Ready`.
- Pyramid plus unupgraded Apparitions and Armaments+ reports `CombatRepairAvailable`, including
  whole-hand repair access.
- Pyramid plus unupgraded Apparitions at `act_num <= 2` reports `FutureUpgradeWindow` when no
  live combat repair exists.
- A profile at `act_num >= 3` with no live repair reports `Limited` but does not call Pyramid the
  source of the Apparitions' Ethereal expiration.
- Projecting Pyramid does not mutate the live `RunState` and does not emit
  `IntroducesStartupLiability` for unupgraded Apparitions alone.
- Enchiridion and Toolbox are counted as generated opening options; Enchiridion is additionally
  identified as a zero-cost-this-turn option.
- With five Choker plays already used, three affordable hand cards yield one estimated playable
  card; with six plays used, they yield zero.
- A Pyramid hand that would retain eight cards leaves two next-turn draw slots.
- Unupgraded Ethereal Apparitions do not consume Pyramid's next-turn retained-hand slots unless
  explicit retain protects them.
- Generated cards already present in hand affect the same retained-hand capacity without
  card-name branches in combat search.

Tests assert categorical evidence and exact capacity relationships. They do not lock aggregate
scores, complete seed paths, random Enchiridion/Toolbox outputs, or boss outcomes.

## Non-Goals

- Do not change Toolbox's fixed shop-relic purchase score in this pass.
- Do not predict the exact number or targets of future Act 3 campfire upgrades.
- Do not treat a possible future permanent upgrade as guaranteed.
- Do not add a full-route, full-seed, or boss-outcome regression test.
- Do not redesign combat search, turn-plan enumeration, or card action ordering.
- Do not add Time Eater-specific action capacity to this pass; its exact simulator mechanics
  remain active.
- Do not broaden generated-opening modeling to every relic until a concrete consumer needs it.
