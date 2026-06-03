# Known Limits

This file lists current boundaries that should stay visible during development.

## Simulator Correctness

The Rust simulator is still being audited against Java behavior. Real runs may
expose mechanics bugs in events, rewards, relics, powers, potions, or special
combat phases. Fix these as simulator issues, not as search-policy quirks.

## Observation Boundary

Public observation, hidden state, and privileged simulator state are not fully
settled everywhere. This matters for:

- Runic Dome and enemy intent visibility
- random outcomes that should be known only as distributions
- relics such as Dead Branch that create large hidden/random branches
- future live-game adapter work

## Combat Search

Combat Search V2 is useful but not complete.

Known weak areas:

- high-fanout pending choices
- split or phase-changing monsters
- long fights with large setup effects
- potion strategy
- random or hidden-information branches
- value estimates for unresolved frontiers

Do not treat unresolved search as a loss proof. Do not treat a budgeted win as
an optimality proof.

## Autopilot

`ar`, `n`, and `nr` are convenience tools. They can make bad decisions if their
policy boundaries are incomplete. Their outputs are behavior-policy evidence
only.

## Legacy Docs

Historical documents were moved to:

```text
docs_legacy/2026-06-03_pre_rewrite/docs/
```

Use them for archaeology, not default guidance.
