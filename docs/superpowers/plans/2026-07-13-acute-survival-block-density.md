# Acute Survival Block Density

## Observed gap

At seed `20260713003`, Act 3 floor 42 offers `Impervious+`, `Second Wind+`, and
`Rampage+` at 22/82 HP.  The decision pipeline gives both defensive cards the
same 100 score because it records only the shared `Provides(Block)` semantic.
It therefore skips a deterministic 40-block card while under acute survival
pressure.

## Design

Reuse `card_analysis_profile_v1` rather than adding an Awakened One exception.
While `DeckPlanSnapshot::survival_pressure()` is true, reward and shop card
candidates receive an additional density component when their static block
chunk is large enough to cover a full enemy hit by itself:

- burst block: +25;
- solid, low, dynamic, or absent block: no density credit.

The existing semantic survival score remains responsible for block, draw, and
mitigation utility.  The new component only distinguishes how much immediate,
deterministic block one draw supplies.  Dynamic hand-scaled block such as
`Second Wind` is explicitly excluded even if its static card definition changes
later.  The component does not apply outside acute survival pressure and does
not claim that dynamic exhaust engines are weak.

## Regression boundary

- The real F42-shaped deck must put `Impervious+` on the mainline.
- `Second Wind+` must not receive static burst-block credit merely because it
  can be powerful with a favorable hand.
- Ordinary solid block such as `Ghostly Armor` must not cross the mainline
  threshold through this rule.
- The same `Impervious+` must not receive this emergency promotion at healthy
  HP.
