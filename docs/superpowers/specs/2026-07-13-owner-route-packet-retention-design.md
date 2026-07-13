# Owner route packet retention design

## Problem

Run-control already emits a typed `MapDecisionPacketV1` for every route selection, including the complete legal candidate pool, projected paths, needs, safety, and score terms. Owner-audit reduces each auto-applied route step to visible state changes, so branch traces and capsules cannot explain whether a later missed shop was unreachable or merely outscored.

## Design

Retain the existing typed map decision packet on the corresponding `RunControlAutoAppliedStepV1`. Only route selection/candidate-pool annotations are extracted; combat trajectories and unrelated annotations are not copied. The optional JSONL trace serializer writes the packet beside that route step.

This is evidence retention, not a new route model and not a behavior change. Checkpoints need no new persistent field because auto-applied steps describe only the current advance and are already discarded when a frontier checkpoint is restored.

## Boundaries

- Reuse `MapDecisionPacketV1`; do not invent a second route schema.
- Keep non-route auto steps compact.
- Preserve existing human rendering.
- Add a regression check at the auto-step/trace boundary where the packet was lost.

