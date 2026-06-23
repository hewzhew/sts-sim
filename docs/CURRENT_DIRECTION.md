# Current Direction

Main line:

```text
simulator -> state representation -> search/rollout -> value -> policy improvement
```

The project is currently about building a usable Rust simulator and AI search
stack for Slay the Spire. The most important loop is:

1. run a real simulator session from Neow onward
2. make or automate low-risk non-combat decisions under explicit boundaries
3. capture stable combat starts
4. run Combat Search V2 over complete combat trajectories
5. compare whole-combat outcomes, not step-by-step imitation

## Active Work

- simulator correctness and Java-mechanics parity when real runs expose bugs
- observation boundaries: public, hidden, random, and privileged simulator state
- Combat Search V2 value, rollout, frontier, and special-phase handling
- Phase 1 non-combat policy quality: route, deck, card reward, shop, campfire,
  event, and boss relic decisions under explicit boundaries

## Route/Map Handling

Route choices currently use an auto-run planner as the default campaign path,
not a normal `BranchBoundary` expansion at every map screen. The planner emits
a full typed `MapDecisionPacketV1`; `CampaignJournal` records both the selected
route decision and the route candidate pool. Coverage-gap continuation can then
target unobserved route candidates deliberately.

Planner stops are still route/map decisions. They should record a typed route
candidate pool with `selected_index = None`, not fall back to a generic
non-combat record that loses map alternatives.

This split is intentional for now: default campaign runs stay bounded, while
route/map alternatives remain inspectable and replayable from journal data.
Route labels, `go N` commands, and top-candidate summaries are display or
compatibility surfaces; new analysis should consume typed route candidates
(`target`, `action`, `features`, `projection`, `needs`, `evaluation`).
Coverage-gap targets and continuation branches should preserve typed route
origin fields as well, so replay and learning tools do not need to parse route
display strings.

Coverage-gap continuation defaults to filling missing historical candidate
coverage before extending branches that only executed the target action. Use an
explicit progress filter such as `--coverage-gap-progress target_only` when the
goal is frontier expansion for already-started coverage targets.

## Closed Foundation

Phase 0 run-control automation and provenance boundaries are closed. Keep
`n`/`nr`/`ar`, trace/replay/bookmarks, non-combat boundary records, and combat
capture artifacts working; otherwise avoid expanding this layer. New
route/card/shop/event/campfire/boss-relic policy behavior belongs to Phase 1.

## Not The Main Line

- old Python watch UI and recording UI
- Workbench or DecisionFrame expansion
- LLM prompt engineering and LLM reviewer flows
- live CommunicationMod control as the default development path
- treating route/card/search decisions as teacher labels

LLM and live-game adapters may return later, but only as consumers of stable
public observation and action contracts. They do not define simulator truth or
search quality.

## Evidence Rules

- A trace records what happened. It is not a policy-quality claim.
- A guarded autopilot decision is `behavior_policy_not_teacher`.
- A combat search result is budgeted evidence unless explicitly validated by
  exact replay and a benchmark context.
- Human baseline comparison is whole-combat outcome comparison, not stepwise
  action agreement.
- New report, journal, and learning-sample fields should pass
  [Report Field Admission](REPORT_FIELD_ADMISSION.md): classify the field as a
  fact, diagnostic, verdict, or label; do not add winner-like summary fields
  when the evidence only supports candidate facts or diagnostics.
