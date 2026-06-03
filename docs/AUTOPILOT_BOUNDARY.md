# Autopilot Boundary

Autopilot exists to reduce manual repetition. It is not a teacher, not a proof
of strategy quality, and not a replacement for simulator/search validation.

## Allowed Autopilot

- routine single-action progress
- automatic low-risk reward claiming, such as gold and potions when slots exist
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

## Card Reward Policy

The card reward policy may auto-pick only when confidence and margin gates pass.
Low-confidence rewards must stop for human choice.

If a card reward auto-pick later looks bad, the fix belongs in the card reward
policy boundary, not in trace replay or benchmark labels.

## Combat Search Handoff

Combat search can be used during autopilot to skip fights. This is acceptable
when the search applies an executable complete trajectory. The outcome is still
search evidence, not a human baseline.

Manual plus search-assisted fights should not be saved as pure human baselines.
