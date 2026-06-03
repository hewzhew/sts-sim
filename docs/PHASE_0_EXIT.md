# Phase 0 Exit

Phase 0 is the run-control and provenance foundation for non-combat automation.
It is not the phase where non-combat policy becomes strong.

## Status

Phase 0 is closed once this checklist remains true:

- `run_play_driver` starts from the real Neow boundary, without skip-Neow
  shortcuts.
- `n`, `nr`, and `ar` reduce repetitive play while stopping at human-required
  strategic boundaries.
- Route-planner, card-reward, and safe reward automation decisions are recorded
  as `behavior_policy_not_teacher`.
- Human-required non-combat stops can emit `NonCombatDecisionRecordV1` with
  `data_role = HumanBoundaryNotTeacher`.
- Stopped behavior policies can be persisted as boundary records without
  pretending an action was selected.
- `SessionTraceV1`, `--record`, `--continue-trace`, `--goto`, and bookmarks can
  replay or resume run prefixes without becoming benchmarks or labels.
- `CombatCaptureV1`, `CombatBaselineOutcomeV1`, and `BenchmarkSuiteV1` remain
  combat-search artifacts, not full-run proof artifacts.
- Existing trace annotations pass central validation before save/load.
- Active docs describe the current commands and boundaries; legacy docs are
  historical only.

If a future bug breaks one of those lines, fix it as a Phase 0 maintenance bug.
If a future request adds smarter route/card/shop/event/campfire decisions, it is
Phase 1 work.

## What Phase 0 Allows

Phase 0 allows only low-risk automation that reduces manual repetition:

- routine single-action progress
- gold and stolen-gold reward claiming
- potion reward claiming when an empty slot exists and Sozu is absent
- ordinary relic reward claiming when no same-screen `SapphireKey` exists
- route planner map movement as behavior-policy evidence
- high-confidence card reward picks as behavior-policy evidence
- combat search handoff when a complete executable win is found within budget

Every non-trivial automated decision must either emit a validated
`NonCombatDecisionRecordV1` or stop at a human boundary that can be recorded.

## What Phase 0 Does Not Include

Do not keep adding these under the Phase 0 name:

- stronger route strategy
- deck archetype or card-synergy understanding
- shop buying or purge policy
- campfire rest/smith/toke/dig/lift policy
- event policy beyond accurate visible option semantics
- boss relic selection
- live CommunicationMod control
- LLM harnesses or prompt formats
- claiming route/card/search choices are teacher labels
- proving combat-search optimality

Those are Phase 1 or later.

## Change Rule

After Phase 0 closure:

- Bugfixes are allowed when current commands violate this document.
- Documentation fixes are allowed when active docs drift from current code.
- Small trace/schema compatibility fixes are allowed when replay or bookmarks
  break.
- New policy behavior must declare a Phase 1 owner and must not be merged as
  "just Phase 0 polish".

## Phase 1 Entry

Phase 1 should focus on non-combat policy quality:

- route planner quality and risk calibration
- deck strength and archetype summaries
- card reward scoring beyond conservative high-confidence gates
- shop, campfire, event, and boss relic policy boundaries
- public/hidden/random observation contracts for future live-game or LLM
  consumers

The main question for Phase 1 is not "can automation proceed?" It is "was this
non-combat decision good, explainable, and compatible with search feedback?"
