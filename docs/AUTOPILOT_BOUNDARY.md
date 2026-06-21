# Autopilot Boundary

Autopilot exists to reduce manual repetition. It is not a teacher, not a proof
of strategy quality, and not a replacement for simulator/search validation.

## Allowed Autopilot

- routine single-action progress
- automatic low-risk reward claiming, such as gold, safe relic rewards, and
  potions when slots exist
- route-planner map decisions
- high-confidence card reward policy decisions
- combat search handoff when a complete executable win is found within budget

Every non-trivial autopilot decision must be treated as:

```text
label_role = behavior_policy_not_teacher
```

## Decision Records

Route-planner and card-reward autopilot decisions also export
`NonCombatDecisionRecordV1`. This is the shared Phase 0 record boundary for
non-combat behavior-policy evidence.

Human-required non-combat stops, such as Neow choices, events, shops,
campfires, boss relics, and remaining rewards, export the same record shape with
`data_role = HumanBoundaryNotTeacher`.

The record must stay hidden-free:

- public observations are allowed
- known distributions and beliefs are allowed when declared
- hidden simulator state is forbidden

Adding a new non-combat autopilot policy should adapt to this record boundary
instead of inventing another report shape.

All generated `NonCombatDecisionRecordV1` values must pass the central
`validate_noncombat_decision_record_v1` gate before being attached to run-control
trace annotations. The gate checks schema identity, hidden-state exclusion,
candidate references, evidence/value references, and whether human-boundary
records accidentally select an action.
The session trace recorder runs the same validation again before writing
annotations, so future producers cannot silently bypass the boundary.
Trace loading also validates existing annotations; old traces with invalid
non-combat records must be fixed, regenerated, or treated as retired debug
artifacts instead of replaying silently.
Stopped/no-candidate policy records can also be persisted as boundary records
without requiring a separate human-boundary annotation.

## Human Boundaries

Autopilot should stop at:

- ambiguous card rewards
- important events
- shops, campfires, boss relics, and other high-impact non-combat choices
- combat search unresolved states
- observation-boundary uncertainty that matters for the decision

## Route Planner

Route planning is a behavior policy over currently visible map choices. It
should produce explainable candidate evidence, but its chosen route is not an
optimal route label.

The planner should be tuned by changing:

- need estimation
- node features
- risk model
- future path search
- safety gates

Do not hide route quality assumptions inside one-off command code.
If safety gates reject automatic map movement, the declined planner evaluation
still emits `NonCombatDecisionRecordV1` with `selection.status = Stopped`, so
candidate evidence remains available without pretending a route was chosen.
Route records keep candidate evidence split into `NeedVector`, `ValueFactors`,
and `ScoreTerms`: needs describe current run pressure, value factors describe
candidate-side opportunities and risks, and score terms are the current weighted
behavior-policy projection. New tooling should inspect those layers separately.

## Card Reward Policy

The card reward policy may auto-pick only when confidence and margin gates pass.
Low-confidence rewards must stop for human choice.
Those declined policy evaluations still emit `NonCombatDecisionRecordV1` as
behavior-policy evidence, with `selection.status = Stopped`; this records why
automation declined without converting the decision into a teacher label.

If a card reward auto-pick later looks bad, the fix belongs in the card reward
policy boundary, not in trace replay or benchmark labels.

## Reward Automation

Reward automation may claim rewards that do not represent a meaningful
strategic branch:

- gold and stolen gold
- potions only when an empty slot exists and Sozu is absent
- ordinary visible relic rewards only when no `SapphireKey` reward is present on
  the same screen

Safe relic auto-claim emits `NonCombatDecisionRecordV1` with
`selection.status = Selected`. The record is still
`behavior_policy_not_teacher`, and exists so trace/replay can explain why the
automation changed state.

Use `auto-reward relic off` to disable safe relic auto-claiming for the current
session. `auto-reward all on|off` includes this relic gate together with
gold/stolen-gold and empty-slot potion claiming. Session traces record this
configuration so replay starts with the same reward automation boundary.

Do not extend this boundary to boss relics, blue-key tradeoffs, card rewards,
or event/shop/campfire choices. Those remain separate decision sites.

## Combat Search Handoff

Combat search can be used during autopilot to skip fights. This is acceptable
when the search applies an executable complete trajectory. The outcome is still
search evidence, not a human baseline.

Manual plus search-assisted fights should not be saved as pure human baselines.
