# Autopilot Boundary

`run_control` automation exists to reduce manual repetition. It may execute
routine progress, low-risk rewards, non-combat policy decisions, and combat
search handoffs. It is not a teacher label, not an optimality proof, and not a
replacement for simulator correctness or search validation.

Every automated non-trivial decision must be treated as:

```text
label_role = behavior_policy_not_teacher
```

## Evidence Contract

`NonCombatDecisionRecordV1` is the shared record boundary for non-combat
automation and non-combat stops.

The record must stay hidden-free:

- public observations are allowed
- declared distributions and beliefs are allowed
- privileged simulator state and hidden future outcomes are forbidden

All generated records must pass `validate_noncombat_decision_record_v1` before
they are attached to run-control trace annotations. The session trace recorder
validates again before writing annotations, and trace loading validates existing
annotations. Invalid old traces should be fixed, regenerated, or retired instead
of replaying silently.

Do not invent a new ad hoc report shape for a non-combat policy. Adapt the
policy to this record boundary or add typed fields to the record deliberately.

## Automation Sites

Current automation sites include:

- route planner map movement
- card reward policy decisions
- routine reward claiming
- shop compiler execution heads
- campfire policy choices
- bounded event policies
- combat search handoff when an executable complete trajectory is found

These sites may make decisions, but their decisions remain behavior-policy
evidence. If an automated decision later looks bad, fix that decision site's
policy/compiler or its inputs. Do not reinterpret trace replay, benchmark
baselines, or campaign reports as teacher labels.

## Stop Conditions

Autopilot should stop when the current site lacks a bounded policy answer.

Common stop reasons:

- no legal or meaningful candidate
- low confidence or narrow margin
- high-agency choice without a current policy/compiler
- observation-boundary uncertainty that matters for the choice
- combat search cannot find an executable complete trajectory under the current
  budget

Do not encode a stale fixed list such as "shops always stop" or "events always
stop." Shops, campfires, events, boss relics, and other high-impact sites need
their own policy/compiler boundaries; whether automation runs depends on that
boundary, not on the screen name alone.

## Route Planner

Route planning is a behavior policy over currently visible map choices. It may
select a route, stop, or emit candidate evidence for later campaign
continuation. A selected route is not an optimal route label.

Route records should keep candidate evidence split by role:

- need vector: current run pressure
- value factors: candidate-side opportunities and risks
- score terms: current weighted behavior-policy projection

Tools should inspect those layers separately. Do not hide route quality
assumptions inside one-off command code or display strings.

## Card Reward Policy

Card reward automation may auto-pick only when the policy boundary allows it.
Declined evaluations should still emit a `NonCombatDecisionRecordV1` with a
stopped selection, preserving candidate evidence without pretending a choice was
made.

If a card reward auto-pick later looks bad, the fix belongs in card reward
policy, candidate facts, or downstream evaluation. It does not make the trace a
negative teacher label.

## Routine Reward Claiming

Routine reward automation may claim rewards that do not represent a meaningful
strategic branch:

- gold and stolen gold
- potions only when an empty slot exists and Sozu is absent
- ordinary visible relic rewards only when no `SapphireKey` reward is present on
  the same screen

Safe reward auto-claims still emit `behavior_policy_not_teacher` records so
trace/replay can explain state changes.

Use `auto-reward relic off` to disable safe relic auto-claiming for the current
session. `auto-reward all on|off` includes this relic gate together with
gold/stolen-gold and empty-slot potion claiming. Session traces record this
configuration so replay starts with the same reward automation boundary.

## Combat Search Handoff

Combat search may be used during autopilot to skip fights only when it applies
an executable complete trajectory. The result is search evidence, not a human
baseline.

Manual plus search-assisted fights must not be saved as pure human baselines.
Use combat captures, benchmark suites, and search evidence for that workflow
instead.
