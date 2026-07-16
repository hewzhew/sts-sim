# Answer deployment evidence contract

## Purpose

The durable trajectory must distinguish an answer that the run merely owns from
one that the committed combat execution could actually deploy.  This is
observation evidence, not a new combat policy and not a score inferred from the
combat result.

## Authority and stages

`RunProgressJournalV1::CombatResolution` remains the single durable authority.
Every accepted combat action records a compact pre-action opportunity snapshot
from the real engine state and its exact legal-move mask.  Projection derives,
per card or potion instance and pressure axis:

1. `claimed`: the instance's typed semantics claim that axis;
2. `reached`: the instance was present in an observed hand or potion belt;
3. `playable`: at least one exact legal play/use existed in an observed state;
4. `applied`: the committed trajectory actually played/used the instance.

The stages are monotone.  `applied` implies `playable`, `playable` implies
`reached`, and `reached` implies `claimed` for projected claims.

## Observation boundary

- Only committed actions are evidence. Search candidates, rollouts, rendered
  reasons, final HP, and hypothetical replays are not evidence.
- Card playability comes from exact `ClientInput::PlayCard` legal moves and is
  joined to the pre-action hand by card UUID. Potion usability is treated the
  same way by slot and potion UUID.
- A complete-victory resolution closes the encounter observation window. A turn
  segment leaves unresolved instances censored at the committed segment head.
- Smoke Bomb escape is an observed application, but it is not converted into a
  victory or combat-answer claim.
- Generated combat cards may be reached, playable, and applied. Their typed
  claim is evaluated against the run deck visible to the committed resolution;
  it is never backfilled as a reward decision.

## V1 scope

V1 observes cards and manually usable potions. Relics and passive potion
triggers are excluded because they do not share the same legal-action lifecycle.
The first horizon is the committed combat resolution; exact threat-turn or
phase deadlines require a later typed encounter-window owner.

## Persistence and consumers

Pre-action opportunity snapshots are embedded in the existing combat trajectory
record, so they inherit content-addressed segment persistence and replay
verification. A deterministic `deployment.json` projection is indexed beside
`behavior.json` and `outcomes.json`.

No run policy may consume this V1 projection. It exists for diagnosis,
comparison, and later calibration. Any future policy consumer requires an
explicit contract revision and a separate migration.
