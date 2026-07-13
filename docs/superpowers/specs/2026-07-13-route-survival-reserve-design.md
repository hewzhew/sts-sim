# Route survival reserve design

## Evidence

After route resource realization, seed `20260713002` selected an Act 2 floor 17
Monster continuation whose representative suffix projected 82 p90 HP loss from
85 HP. The packet still labeled the suffix `Ok` and its `hp_loss` term was
exactly zero. The run then lost 49 HP in the next hallway and reached Collector
with materially less development than the prior trajectory.

The zero came from `avoid_damage = 1 - hp_ratio`: a full-health deck whose block
score barely clears the thin threshold assigns no price at all to projected HP
loss. Separately, route safety treats every positive projected HP value as a
surviving, safe margin, so a three-HP p90 projection remains `Ok`.

## Design

1. Give route HP conservation a small non-zero baseline even at full health.
   Health remains spendable, but no visible 82-HP projection is free.
2. Downgrade a path to `RiskyButAllowed` when its p90 projection preserves no
   more than one quarter of max HP. This is a reserve warning, not a universal
   rejection and not a claim that the p90 estimate is calibrated truth.
3. Keep the existing hard rejection for projections at or below zero and the
   existing low-HP recovery rules.

These rules use only projected damage and player HP capacity. They do not name
a seed, encounter, act, card, or route node type.

## Non-goals

- Do not forbid spending HP for rewards.
- Do not treat shops as healing.
- Do not redesign family-level route commitment in this patch.
- Do not change combat search or deck construction scoring here.
