# Marginal AoE Quality Design

## Goal

Stop repeated low-output area attacks from claiming that they repair an Act 2+
multi-enemy gap. Preserve credit for the first source of area damage and for a
genuinely strong area-damage card that upgrades a shallow package.

## Evidence

The bounded run for seed `20260713003` took three copies of `Cleave`. Before the
third copy the strategic model reported a thin AoE package; after adding it the
model reported adequate solely because the special shallow-AoE rule matched
exactly two weak AoE cards. The candidate therefore received strategic-gap
credit even though another identical weak attack did not change the package's
quality. A later Act 2 hallway case had a narrow 14 HP win but no reserve-safe
win, so forcing combat fallback would hide this upstream construction error.

## Behavior

- Track strong AoE units next to total AoE units in the deck role inventory.
- In Act 2 and later, any package containing only weak AoE remains `Thin`, no
  matter how many duplicate weak cards it contains.
- A candidate repairs a missing AoE gap when it supplies any AoE.
- A candidate repairs a thin all-weak AoE package only when the candidate is a
  strong AoE source.
- If the deck already has one strong AoE but is thin only because it has a
  single AoE unit, a second AoE unit may still close the density gap.
- Known boss-specific evidence, such as Collector minion control, remains a
  separate source of authority and is not removed by this generic rule.

## Boundaries

This change does not enable turn-plan frontier seeding, relax the owner HP
reserve, or make turn-pool rescue authoritative. It changes only construction
quality and the score/lane credit derived from that quality.

## Verification

- Three weak Cleaves remain a thin AoE package.
- A weak AoE candidate does not claim to improve an existing all-weak package.
- A strong AoE candidate does repair that package.
- The decision pipeline keeps the repeated weak Cleave below mainline while a
  useful alternative can remain eligible.

