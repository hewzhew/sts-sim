# New AI Architecture

This is the current target shape for `src/ai`. It is not a migration log and it
is not a promise to preserve old `_v1` behavior. Its purpose is to stop new work
from expanding the old mixed naming model.

## Layer Contract

New AI code must choose one layer before it is written:

- `domain`: stable game facts and vocabulary. No value judgments.
- `analysis`: profiles derived from facts and run state. No final scene choice.
- `strategy`: jobs, gates, formation, package state, deck debt, and assessment.
- `policy`: thin scene adapters for reward, shop, campfire, event, route, Neow,
  and boss relic decisions.
- `runtime`: campaign scheduling, branch execution, journal, replay, and budget.
- `legacy`: still-required old code that is no longer a design target.

The intended flow is:

```text
domain -> analysis -> strategy -> policy -> runtime
```

Runtime can record and schedule. Policy can generate candidates. Strategy owns
the high-level interpretation. Analysis owns reusable profiles. Domain owns game
facts.

## Rules

Do not add a new `*_policy_v1` or `*_profile_v1` unless it is explicitly a
compatibility/report boundary.

Do not create another scene-local strategic model. If reward, shop, route, and
branch retention need the same concept, it belongs in `analysis` or `strategy`.

Do not keep a duplicate module just because migration is expensive. If a module
is not wired into the canonical flow and has no clear owner, prefer deleting it
and preserving only the useful idea in this document or in a new typed layer.

Do not use free-form strings as the owner of strategy semantics. Strings are
acceptable for trace labels after a typed fact or verdict already exists.

## Current Migration Target

The first real migration target is assessment:

```text
analysis profiles -> strategy assessment -> thin policy candidate verdicts
```

The old world still contains useful code, but new work should not extend it
without first deciding whether the concept belongs in `domain`, `analysis`,
`strategy`, `policy`, or `runtime`.

## What This Replaces

The removed `acquisition_saturation_v1` / `AcquisitionThesis` path was a mixed
layer: it used scene-local heuristics, strategy-like language, and branch
retention effects at the same time. That shape should not be rebuilt under a new
name.
