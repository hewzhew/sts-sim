# Campaign Journal

`CampaignJournal` is the campaign-level decision event log. It exists because
branch campaign decisions need a stable source of truth that is separate from
compact reports, checkpoint snapshots, and human-readable inspection output.

## Purpose

The journal answers questions such as:

- what decision boundary was reached
- which candidates were available
- which candidates were kept, frozen, pruned, or applied
- which branch and checkpoint state the decision belonged to
- which later outcomes can be linked back to that decision

Reports and inspect commands should be views over the journal when possible.
They should not become independent places that reconstruct decision history.

## Current Scope

The current implementation records these decision event shapes produced by
branch campaign parent expansion:

```text
CampaignJournalV1
  events[]
    reward_candidate_set
      decision_id
      boundary_title
      frontier_key
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    shop_branch_candidate_set
      decision_id
      boundary_title
      frontier_key
      candidates[]
        command
        label
        semantic_class
        disposition: kept
    shop_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_frontier_count
      rollout_head_plan_id
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    campfire_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_option_count
      selected_plan_id
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    event_candidate_pool
      decision_id
      boundary_title
      frontier_key
      game_event_id
      candidate_count
      branch_option_count
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    boss_relic_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      branch_option_count
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    route_candidate_pool
      decision_id
      boundary_title
      frontier_key
      candidate_count
      selected_index
      map_decision_packet?
      route_candidates[]        # typed route/map snapshot
        candidate_id
        command
        target_node
        action
        safety_flag
        score_terms
        value_factors
        path_summary
        projection metadata
      candidates[]
        command
        label
        semantic_class
        disposition: kept | pruned
    route_decision
      decision_id
      route_branch_id
      selected_index
      selected_candidate_id
      selected_route_candidate? # typed selected route/map snapshot
      target
      move_kind
      safety
      command
      elite_prep_bp
      first_elite
```

`BranchCampaignReportV1` now carries `journal` as a top-level field. The older
`rounds[].decision_observations` field remains as a compatibility summary, but
new inspection should prefer `journal`. The `--inspect-journal` report view
prints journal events directly; `--inspect-decision-observations` remains a
reward-only compatibility view.

Schema version 2 adds `route_candidates[]` to `route_candidate_pool`. The generic
`candidates[]` list remains the cross-decision coverage/scheduling surface; the
route-specific list preserves typed map target, action, path projection, and
route value-factor data so route inspection and coverage-gap continuation do not
depend on flattened `semantic_class` strings.

Schema version 3 adds `selected_route_candidate` to `route_decision`. This makes
the selected move self-contained for inspection and learning export; consumers no
longer need to rejoin against a route candidate pool or parse `target`/`command`
strings just to recover selected route path, projection, and evaluation fields.

Decision-outcome dataset export also prefers `journal` when available. It uses
the journal `decision_id` as the sibling group identity and links an observed
branch outcome to a candidate when the branch command sequence starts with the
journal parent commands plus the candidate command. Older command-prefix
reconstruction remains as a fallback for reports without journal events.

The same export prints `DecisionCandidateCoverageV1`, a report-level diagnostic
for how many journal candidates have any observed descendant branch. This is a
coverage check for campaign scheduling and learning data readiness; it is not a
candidate value estimate.

Coverage gap continuation builds on that diagnostic. It can plan unobserved
journal candidates with `--plan-coverage-gap-continuation --inspect-report ...`,
then execute a bounded set with `--execute-coverage-gap-continuation --resume ...
--resume-checkpoint ...`. Execution creates temporary active branches from the
journal parent commands plus the missing candidate command and then uses the
normal branch campaign runner. This mechanism is for targeted data coverage, not
for saying that the missing candidate is strategically better. The planner
balances decision buckets, and route/map gaps are further interleaved by typed
route lane (action, room, projection coverage, first-elite shape) so a small
budget does not only replay same-looking map alternatives.

For route/map candidates, continuation targets carry structured
`target_origin` provenance from the journal `MapDecisionPacketV1` when it is
available, and fall back to `route_candidates[]` when a compact or legacy event
does not have the full packet. That origin records the typed target room, route
action, candidate pool completeness, route projection coverage, visible path
summary, and first elite segment. Coverage-gap tooling should use this
provenance to explain and schedule missing route candidates; it should not parse
`go N` commands or display labels to recover map identity.

Lineage inspection also uses the typed route snapshot. For example,
`--inspect-lineage-decisions --inspect-query CompleteWithinBudget` searches
route projection coverage and renders route candidates with target, action,
path coverage, visible path count, and elite/fire/shop ranges. This is an audit
view over recorded candidates, not a route scoring rule.

## Boundaries

`CampaignJournal` is not a strategy engine. It must not decide what to pick or
rank branches.

`CampaignJournal` is not a combat trace. Combat details should live in combat
capture or combat trace artifacts and be linked by reference if needed.

`CampaignJournal` is not a checkpoint. Checkpoints restore state; journal events
explain decision provenance.

## Intended Data Flow

```text
decision boundary
  -> candidate enumeration / evaluation
  -> CampaignJournal event
  -> branch/report summary view
  -> inspect view
  -> learning/outcome dataset
```

The important direction is one-way: reports and datasets are derived from
journal events, not the other way around.

## Migration Plan

1. Reward candidate sets are the first event source.
2. Shop branch frontier candidates were the second event source.
3. Full shop compiler candidate pools are now captured in
   `BranchExperimentReportV1.shop_plan_candidate_pools` and surfaced as
   `shop_candidate_pool` journal events.
4. Campfire compiler candidate pools are now captured in
   `BranchExperimentReportV1.campfire_plan_candidate_pools` and surfaced as
   `campfire_candidate_pool` journal events.
5. Event branch candidate pools are now captured in
   `BranchExperimentReportV1.event_candidate_pools` and surfaced as
   `event_candidate_pool` journal events.
6. Boss relic candidate pools are now captured in
   `BranchExperimentReportV1.boss_relic_candidate_pools` and surfaced as
   `boss_relic_candidate_pool` journal events.
7. Route planner candidate pools are now captured in
   `BranchExperimentReportV1.route_candidate_pools` and surfaced as
   `route_candidate_pool` journal events. New route pools carry a typed
   `MapDecisionPacketV1` with `RouteMoveCandidateV1` entries; legacy candidate
   labels and summaries are compatibility/display views only. Route packets
   include candidate-pool provenance (`legal_candidate_count`,
   `complete_legal_pool`, ordering) and per-candidate projection metadata
   (`path_budget`, `observed_path_count`, coverage). `possibly_truncated`
   coverage means the visible-map DFS explicitly exhausted its configured path
   budget, not merely that the observed path count happened to equal the budget.
   It is a coverage warning, not evidence that a route is good or bad.
   New route trace annotations keep `top_candidates` as a short display
   summary, but the full candidate pool should live only in
   `MapDecisionPacketV1`. The legacy `candidate_pool` summary field is a
   fallback for older traces that do not have a typed packet.
   Route move evaluation records three separate layers:
   `needs` for current-run pressure, `value_factors` for candidate-side route
   opportunities/risks, and `score_terms` for the weighted contributions used
   by the current behavior policy. New analysis should inspect those layers
   separately instead of treating the final score as the only explanation.
8. Route planner selections remain surfaced as `route_decision` journal events
   for compatibility and selected-action evidence. New route decisions also
   carry `selected_index`, `selected_candidate_id`, selected candidate rank,
   typed target node, typed safety flag, route candidate-pool provenance, and a
   typed `selected_route_candidate` snapshot so the selected move can be
   inspected without parsing the display label, legacy safety string, or `go N`
   command.
9. Campaign route evidence summaries aggregate both selected route decisions
   and the surrounding route candidate pools. The summary is a report diagnostic
   about visible route coverage and safety distribution, not a route policy
   score.
10. Link milestone outcomes to prior `decision_id` values.
11. Gradually remove report-only decision attachments once views read from the
   journal directly.

## Design Rules

- Give every decision a stable `decision_id`.
- Give every candidate a stable `candidate_id`.
- Keep display labels separate from machine identity.
- Store public boundary identity and candidate structure at decision time.
- Prefer structured fields over parsing strings from rendered reports.
- For map choices, consume `MapDecisionPacketV1` / `RouteMoveCandidateV1`
  (`target`, `action`, `features`, `projection`, `needs`, `evaluation`) rather
  than `RoutePlannerCandidateSummaryV1` strings. Use route projection coverage
  to distinguish complete visible projections from budget-limited projections.
- Keep old report fields only as compatibility views, not as new sources of
  truth.
- Treat candidate `admission` as the structured scheduling trace. Legacy
  `disposition` (`kept`/`pruned`) is still serialized for compatibility, but
  new continuation and replay tooling should prefer `admission.status`,
  `reason_category`, `reason_code`, `source`, and `lane`. The legacy
  `admission.reason` string remains compatibility/debug text, not a field to
  parse for control flow.
- Interpret branch `commands` relative to the report/checkpoint
  `run_prelude`, not relative to a fresh process start. New reports record the
  replay root and prefix commands explicitly; continuation tools should consume
  that prelude instead of reconstructing Neow or CLI prefix state.

## Current Caveats

- `shop_branch_candidate_set` exists for reports generated during the first
  shop-journal migration. New reports should prefer `shop_candidate_pool`.
- Shop candidate pools are the compiler candidate pool, not a raw shop inventory
  dump. They include single-action plans, stop/leave plans, and portfolio plans
  that the compiler generated for the active compile mode.
- Campfire candidate pools are the campfire compiler candidate pool, including
  rest/smith/stop plans and deck-mutation-derived target metadata where
  available.
- Event candidate pools are generated event branch candidates after public event
  semantics and policy/deck-mutation annotations are attached, with final branch
  admission marked separately.
- Boss relic candidate pools are complete boss relic choice sets with projected
  run debt and policy class metadata; all options remain branch candidates.
- Route candidate pools are full route planner option sets emitted while
  expanding a parent branch. They are eligible for journal coverage diagnostics
  and targeted coverage-gap continuation.
- Route decisions are the selected route planner actions emitted while
  expanding a parent branch. They record typed selected-action identity
  (`selected_candidate_id`, candidate rank, typed target, typed safety) plus
  candidate-pool provenance. The legacy target/safety strings are display and
  old-report compatibility only. New continuation and learning paths should
  use route decisions for selected-action provenance and `route_candidate_pool`
  for full candidate analysis.
- Decision-outcome samples only include candidates that have an observed
  descendant branch in the report. A candidate that was recorded in the journal
  but never continued by campaign scheduling is still visible in
  `--inspect-journal`, but it does not yet get a synthetic zero-observation
  outcome row.
- Candidate semantics still include legacy `semantic_class` strings from branch
  retention; these are provenance, not proof of strategic correctness.
- Decision-outcome dataset export now links observed branch outcomes back to
  journal decision ids. Milestone outcome events are not yet stored directly in
  the journal.
- Reports written before `run_prelude` still need compatibility fallback when
  running continuation tools. New reports/checkpoints should not infer replay
  prefix from CLI arguments.
- Existing `decision observations` output is still reward-compatible legacy
  terminology. Prefer `--inspect-journal` for new debugging.
