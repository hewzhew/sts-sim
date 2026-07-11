# Route Reliability Projection Design

## Context

The current route planner already has useful control boundaries: typed legal
targets, deterministic candidate ordering, separate safety and value terms, a
machine-readable map decision packet, and a run-control adapter that applies
only the selected typed action. The failure is in the policy model rather than
the actuator.

For seed `20260711004`, the accepted mainline entered Act 2 floor 19 at 59 HP,
left Centurion and Healer at 44 HP, left Snake Plant at 20 HP, and then reached
a forced Book of Stabbing. The route planner eventually labelled the Book route
unsafe, but only after no alternative remained. Earlier decisions charged only
the immediate room's HP loss while treating hallway fights before the elite as
positive preparation.

The repository also has two map projections. `route_window_facts` is the newer
value-free fact boundary and distinguishes must/can/cannot/unknown claims and
partial coverage. `route_planner_v1::features::path_summary` independently
enumerates the map and collapses paths into minima and maxima. Route scoring
then combines maxima that may belong to different suffixes. A candidate can
therefore receive elite, campfire, shop, and event value that no single
continuation contains.

Finally, the old path summary treats both shops and campfires as recovery
boundaries. A campfire guarantees a heal opportunity; a shop only offers a
conditional resource-conversion opportunity. They must not satisfy the same HP
safety proof.

## Considered Approaches

### Add cumulative hallway loss to the old summary

Subtract the existing 14-HP hallway p90 estimate for every forced hallway before
the first elite. This would catch the current seed cheaply, but it would deepen
the duplicate map model, retain cross-path maximum scoring, and preserve the
shop-as-recovery error. It is rejected.

### Add candidate-scoped reliability projection over shared path facts

Expose candidate-scoped visible path families from `route_window_facts`. For
each legal next node, evaluate every observed suffix sequentially, stop HP risk
at the first campfire or after the first elite, and select one real continuation
by safety first and value second. This preserves the current typed policy and
run-control boundaries while replacing the unreliable projection core. This is
the selected approach.

### Build a full-act stochastic route solver

Model encounter distributions, reward outcomes, potion use, shops, campfire
choices, and future deck states in a dynamic program or beam search. The
current outcome models are not calibrated enough to make this reliable, and it
would turn the first route repair into a large research project. It is deferred.

## Decision

### One shared path-family source

`route_window_facts` will expose a typed, value-free `RouteWindowPathFamily`
and a candidate-scoped builder. A path contains ordered visible nodes; the
family carries the same coverage status and limitations already used by route
window facts. The existing aggregate fact builder and route planner will both
consume this enumerator, so path-budget and unmodelled-mobility semantics have
one owner.

The legacy `RoutePathSummaryV1` remains as an aggregate evidence projection for
existing consumers during this first phase, but the planner will derive it from
the shared family rather than running another DFS. It will no longer drive
future reward scoring. Retiring the aggregate type from downstream strategy and
learning schemas is a separate migration, not a prerequisite for fixing route
selection.

### Sequential HP risk projection

Each observed suffix will receive a typed `RoutePathViabilityV1`:

- cumulative p90 HP loss from the candidate node through the first campfire,
  first elite, or visible horizon;
- projected HP after that risk segment;
- whether an elite was included before recovery;
- whether a campfire was reached before that elite;
- whether a shop was seen, recorded only as liquidity;
- whether the suffix survives the projected segment.

The first implementation centralizes the planner's existing uncalibrated room
estimates: 14 HP for a hallway, 40 HP for an elite, 60 HP for a boss, and the
existing unknown-room belief estimate. These numbers remain explicitly
uncalibrated behavior estimates. Their first upgrade is correct accumulation
and provenance, not pretending they are encounter-specific forecasts.

A campfire ends the current danger segment because it provides a guaranteed
heal decision. The projection does not assume the owner will heal or invent the
amount healed. The next map decision is recomputed after the campfire. A shop
does not end the danger segment.

### Candidate selection over real suffixes

For every legal next node, the planner will evaluate each observed suffix with
its own path summary, viability, value factors, score terms, and safety flag.
The representative suffix is chosen by:

1. safety class (`Ok`, then `RiskyButAllowed`, then reject);
2. total value score;
3. stable path index.

The route candidate's score and human reasons come from that representative
suffix. Family-level path count may still contribute flexibility value, because
real alternative continuations are themselves useful, but elite, campfire,
shop, event, and reward access must come from the same suffix.

If complete coverage proves that every suffix fails its projected HP segment,
the candidate is `RejectUnlessNoAlternative`. If coverage is partial and no
surviving suffix was observed, the candidate is risky rather than conclusively
rejected. A surviving observed suffix remains a valid existential continuation
even when other unobserved suffixes may exist.

Existing deck-readiness and immediate-room risk gates remain in force. Shop
access is removed from the elite HP bailout condition; it may influence value,
but it cannot prove survival.

### Evidence and control boundary

`RouteCandidateTraceV1` and `MapDecisionPacketV1` will expose the family
coverage, observed/surviving path counts, representative path index, and the
representative viability projection. Schema versions advance while old fields
remain readable through serde defaults.

The planner continues to select only the next map node. It does not commit the
runtime to the representative suffix. Replanning after every room is required
because actual HP, rewards, potions, and unknown-room outcomes change the state.
Run-control remains a typed actuator and gains no strategy logic.

## Boundaries

- Do not add `route_planner_v2` or a second scene-local map traversal.
- Do not add encounter-name, seed, Snake Plant, or Book of Stabbing special cases.
- Do not introduce full-act dynamic programming, combat search calls, or learned loss models.
- Do not treat a shop as guaranteed healing or stop HP accumulation at a shop.
- Do not assume that reaching a campfire means the campfire owner must rest.
- Do not lock exact heuristic totals in tests.
- Do not rerun a full seed as a unit regression; use semantic fixtures and the preserved capsule for final inspection.
- Preserve typed candidate ordering and the run-control application boundary.

## Verification

Use test-driven development to prove:

1. Candidate-scoped path families preserve ordered nodes and coverage semantics.
2. Adding a forced damage room cannot increase projected remaining HP or improve viability.
3. Raising current HP cannot make the same suffix less viable.
4. A shop does not stop cumulative HP loss; a campfire does.
5. A low-HP hallway-to-elite chain is rejected when a campfire continuation is available.
6. Value factors for the selected continuation never combine elite access from one suffix with campfire access from another.
7. Route trace and map packet evidence serialize the selected viability and coverage.

Run formatting, focused route-window and route-planner tests, the full library
suite, and architecture tests. After code verification, inspect the preserved
seed evidence without rerunning preceding floors and confirm that the new model
would gate the 44-HP hallway-to-Book continuation before it becomes forced.

## Success Criteria

- Route safety accounts for cumulative visible combat pressure, not only the next room.
- Shop liquidity and campfire recovery have distinct safety semantics.
- Candidate value and risk describe one real continuation.
- The route planner and route-window consumers share path enumeration and coverage provenance.
- The current seed's failure chain is represented by a general invariant rather than a seed-specific rule.
