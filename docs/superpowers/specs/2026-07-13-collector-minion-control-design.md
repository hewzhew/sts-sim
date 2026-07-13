# Collector minion-control design

## Evidence

After first real draw was correctly admitted, seed `20260713002` selected `Battle Trance+` over
`Cleave` on Act 2 floor 21 and regressed from Act 3 floor 43 to a Collector combat gap. The choice
was made with Collector already known. Cleave received only generic AOE-gap credit; Collector was
absent from the boss threat table and from the decision pipeline's boss-specific support evidence.

## Decision

- Describe Collector as an AOE/minion-control, high-incoming-damage, long-fight boss in the shared
  threat profile.
- In the production decision pipeline, grant boss-relevant minion-control evidence only when the
  known boss is Collector, the deck's AOE/minion-control deficit is missing or thin, and the
  candidate has semantic area damage.
- Do not name specific cards and do not reward AOE after the deficit is adequate.

The score must be large enough for the known-boss repair to beat a generic access improvement in
the captured comparison, while leaving non-Collector and already-adequate decks unchanged.
