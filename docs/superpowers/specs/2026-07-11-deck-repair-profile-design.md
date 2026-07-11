# Deck Repair Profile and Pandora Offer Evidence Design

## Goal

Represent Pandora's Box as a high-variance transform opportunity without treating it as either a
universal upgrade or an energy failure. After any disruptive deck change, expose the concrete
repair needs of the resulting deck so shop removal and campfire upgrades can respond to the deck
that exists now.

The first implementation is score-free. It adds typed evidence, admits a narrow class of safe
functional-card removals, and lets campfire upgrades recognize urgent reliability repair. It does
not change combat search or decide Pandora-versus-Choker from one aggregate score.

## Problem

The simulator executes Pandora's Box correctly, but strategy recognizes it only at the boss-relic
offer. Card rewards, shop removal, and campfire upgrades receive an ordinary `RunState` afterward
and have no shared representation of a large structural transition.

The current seed `20260711002` makes the gap visible:

- Pandora transformed eight remaining starter basics into eight ordinary Ironclad cards.
- The resulting deck had several individually useful cards but competing setup, exhaust,
  strength, block, and frontload directions.
- Card rewards partially responded to a thin AOE signal, but no owner formed a shared repair plan.
- Shop removal exposed every non-starter card as an unsupported candidate. The deck-mutation
  compiler can distinguish redundant functional cards from singleton core cards, but the shop
  adapter maps every functional target to an unsupported policy class.
- Three later campfire upgrades selected Clothesline, Fiend Fire, and Spot Weakness while five
  unupgraded Apparitions remained time-sensitive. The upgrade planner knows the Apparition
  mechanic, but no deck-level repair evidence raises the reliability value of retaining part of
  that defensive package.

The old Choker path beating Collector proves that the seed has a lower-variance route. It does not
prove that Pandora was the wrong offer, that the realized Pandora deck was unwinnable, or that an
`Act 2 energy gap` is a sufficient relic model.

## Decisions

### Separate offer-time uncertainty from post-pick repair

Pandora reasoning has two different boundaries:

1. **Offer evidence** uses only information available before the pick. It describes how much of
   the current deck would be transformed and how exposed current attack and block density are to
   the random replacement. It must not inspect the seeded cards that Pandora would generate.
2. **Repair evidence** derives from the concrete deck after the pick. It does not need to remember
   which exact cards came from Pandora. It describes current deficits, redundant functions, and
   reliability upgrades, and naturally becomes empty when the deck no longer needs repair.

Do not add a persistent `Pandora mode`. The relic may remain for the whole run, while the repair
need is transient and belongs to the current deck shape.

### Add score-free `PandoraOfferProfileV1`

Derive a small offer profile from the public current `RunState`:

- remaining starter Strike count;
- remaining starter Defend count;
- total transform target count;
- deck size and transform-target share;
- whether non-starter cards already provide frontload, block, access, and scaling support;
- offer horizon: after Act 1 or after Act 2;
- variance class, which is high for Pandora's full random replacement.

Counts and categorical support are evidence, not an expected card-value calculation. Eight
targets may justify more transform upside than two targets, while simultaneous removal of most
basic attacks or blocks remains visible as short-term volatility.

Boss-relic admission may render this profile beside `TransformAgency`. The first implementation
must not convert it into a new rank, peek at generated identities, or use the realized
`decision_delta` to justify the pre-pick choice.

### Add general `DeckRepairProfileV1`

Derive one repair profile from the current `RunState` by composing existing boundaries instead of
creating another card list:

- `deck_strategic_deficit` supplies missing or thin deck functions;
- the deck-mutation compiler supplies target-loss and redundancy evidence for removal;
- the upgrade planner and central card analysis supply mechanical upgrade deltas and upgrade
  debts.

The profile contains no total repair score. It exposes:

- missing/thin function facts already supported by the strategic deficit;
- exact low-loss functional removal candidates, including UUID/deck index and target-loss
  evidence;
- exact reliability-repair upgrade candidates and their mechanical reason;
- optional diagnostic source tags such as owning Pandora, without changing behavior from the tag
  alone.

The existing energy/playability deficit remains visible but is not expanded into another setup
contention model in this pass.

Unknown cards or unsupported semantics do not become repair targets.

### Admit only evidence-backed functional removal

Shop removal may gain one new policy class for a low-loss functional repair target. Admission
requires all of the following:

- the deck-mutation compiler marks the target as `RedundantFunctional` rather than `Functional`
  or `CoreFunctional`;
- no curse or starter-basic cleanup target is available;
- removing the target does not worsen a currently missing/thin function that the card supplies;
- the shop can afford the purge;
- the existing unified shop evaluation admits the resulting single-step plan.

Do not make every transformed or ordinary card removable merely because Pandora is owned. A
singleton functional card remains protected. The current seed may therefore expose no safe purge
even after this feature; the important change is that the owner can express a justified ordinary
card cleanup when one exists.

### Let campfires repair fragile function before generic growth

Campfire upgrade ordering may consume reliability-repair evidence when a concrete upgrade makes a
needed function less draw-order-sensitive or less likely to disappear before use. Representative
mechanical signals include:

- removing Ethereal or Exhaust from a needed card;
- lowering the cost of a needed setup or defensive card;
- making a controlled effect reliable enough to pay an existing upgrade debt.

Repeated unupgraded Apparitions are the motivating case: each upgraded copy creates one retained
defensive option, so their upgrade value must not be discarded merely as duplicate defensive
density. The profile does not require every Apparition to be upgraded and does not hard-code an
exact count. It exposes the repair fact; the existing rest-versus-smith safety boundary remains
authoritative.

Generic damage, scaling, and debuff upgrades remain valid when no stronger reliability repair is
present.

### Keep reward and combat behavior out of the first pass

Card rewards already consume strategic deficit and showed partial adaptation after Pandora. The
first implementation may attach the repair profile to diagnostics but must not add a
Pandora-specific reward score or rewrite card acquisition.

Combat search remains responsible for piloting the concrete deck. Apparition timing, Fiend Fire
exhaust choices, and Collector target selection are not changed by this design. A later combat
task may consume repair evidence only if a frozen case proves a distinct combat-search defect.

## Data Flow

1. At a boss-relic offer, `PandoraOfferProfileV1` reads the current deck and central strategic
   facts without applying the relic or advancing RNG.
2. Boss-relic evidence renders transform opportunity and volatility; selection rank is unchanged
   in the first pass.
3. After Pandora or any other mutation resolves, the simulator remains authoritative for the
   resulting deck.
4. `DeckRepairProfileV1` derives current repair facts from strategic deficit, deck-mutation target
   loss, upgrade planning, and mechanics semantics.
5. Shop policy may admit an exact redundant-functional purge through its existing compiler and
   unified evaluator.
6. Campfire policy may prefer a reliability-repair smith target while preserving the existing
   rest safety gate.
7. Run evidence renders why a repair target was admitted or protected. No consumer infers repair
   solely from the Pandora relic ID.

## Unknown and Fallback Behavior

- Hidden Pandora results remain hidden until the simulator resolves the relic.
- Missing mechanics semantics produce no repair bonus and no automatic removal.
- A functional target without explicit low-loss evidence remains protected.
- Ties fall back to existing owner ordering.
- All counts use saturating arithmetic.
- The profiles are derived from `RunState`; no new persisted lifecycle state or checkpoint schema
  is required.
- Simulator legality and deck-mutation compiler selection rules remain authoritative.

## Stable Tests

- A deck with eight starter basics reports more Pandora transform targets than a deck with two,
  without exposing generated card identities.
- Pandora offer evidence separately reports starter attacks and starter blocks at risk.
- Owning Pandora with an already coherent deck does not by itself create repair targets.
- A duplicate low-marginal functional card may be exposed as a low-loss removal candidate when it
  supplies no thin/missing function.
- A singleton core functional card is protected from automatic shop purge.
- A curse or starter-basic cleanup target remains ahead of functional repair.
- Multiple unupgraded Apparitions can expose an Ethereal-removal reliability upgrade without
  requiring every copy to be upgraded.
- A generic damage upgrade does not outrank a stronger reliability repair solely because it has a
  larger raw damage delta.
- Rest-favored campfire safety still blocks smith automation.
- Unknown semantics do not create repair admission.

Tests assert durable relationships and protection gates. They do not lock exact aggregate scores,
the Pandora roll for a seed, complete seed paths, Collector outcomes, or a fixed number of cards
that must be repaired.

## Non-Goals

- Do not add a persistent Pandora lifecycle flag or provenance to every transformed card.
- Do not predict or peek at Pandora's generated card identities.
- Do not reduce Pandora and Choker to an `Act 2 energy gap` comparison.
- Do not assign one total deck-repair score.
- Do not automatically purge arbitrary functional cards.
- Do not redesign card reward acquisition in the first pass.
- Do not change combat action ordering, state value, or Collector tactics.
- Do not run a full seed as a regression test for the implementation.
- Do not change boss-relic ranking until the new evidence has been observed in a bounded run and a
  separate behavior design is approved.
