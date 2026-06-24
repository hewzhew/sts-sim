# Campaign Journal

`CampaignJournal` is the campaign-level decision event log. It is the source of
truth for non-combat decision boundaries, candidate pools, and continuation
provenance. Reports, inspect commands, and learning/outcome exports should be
views over the journal whenever possible.

## Role

The journal records:

- which decision boundary was reached
- which branch, replay root, and checkpoint context the decision belonged to
- which candidates were available
- each candidate's stable identity, command, typed summary, and admission trace
- which candidate was selected or applied when a policy made a choice
- which later branch outcomes can be linked back to the decision

The journal lets later tools ask:

```text
What choices existed here?
Which choices were observed later?
Which choices were never continued?
What exact campaign state can continue this candidate?
```

## What It Is Not

`CampaignJournal` is not a strategy engine. It must not decide what to pick,
rank branches, or reinterpret candidate quality.

`CampaignJournal` is not a checkpoint. Checkpoints restore simulator state;
journal events explain decision provenance and candidate identity.

`CampaignJournal` is not a combat trace. Combat details belong in combat
captures, search reports, or diagnostic sidecars and should be linked by
reference when needed.

`CampaignJournal` is not a training label store. Learning exports may read it,
but journal presence alone does not mean a candidate is good or bad.

## Event Families

Current campaign journal events are grouped by decision site:

- `reward_candidate_set`
- `shop_candidate_pool`
- `campfire_candidate_pool`
- `event_candidate_pool`
- `boss_relic_candidate_pool`
- `route_candidate_pool`
- `route_decision`

Compatibility views may still expose older summary fields. New inspection,
continuation, and learning code should consume journal events and typed
candidate fields directly.

## Candidate Admission

Candidate admission is the structured scheduling trace for a candidate. It is
the current machine-readable replacement for parsing display labels or old
kept/pruned terminology.

Use:

- `admission.status`: scheduled, deferred, rejected, or unknown
- `reason_category`
- `reason_code`
- `source`
- `lane`

Do not parse free-form reason text for control flow. Free-form text is display
and debugging material only.

## Route And Map Candidates

Route/map decisions need typed identity because display strings such as `go N`,
route labels, and top-candidate summaries are not stable analysis inputs.

Route candidate pools should preserve:

- typed target room and map node identity
- route action
- route projection coverage
- visible path summary
- first-elite or key segment metadata when available
- separated need, value-factor, and score-term layers

Use `MapDecisionPacketV1` / `RouteMoveCandidateV1` data when available. Route
coverage-gap tools should use typed `target_origin` provenance rather than
recovering map identity from commands or rendered labels.

Route planner selections are recorded as `route_decision` events for selected
action provenance. Full route comparison should read the surrounding
`route_candidate_pool`.

## Coverage-Gap Continuation

Coverage-gap continuation starts from journal candidates that do not yet have an
observed descendant branch. It can deliberately continue missing reward, shop,
event, campfire, boss relic, or route candidates instead of guessing from the
current scheduled/parked workset.

This mechanism is for data coverage and auditing. It is not a value estimate
and does not claim that a missing candidate is strategically better.

Coverage-gap planning should balance targets by decision type and typed route
lane when possible, so a small budget does not only replay same-looking
alternatives. Execution results should preserve target provenance so later
reports can explain which historical candidate produced each branch.

## Data Flow

```text
decision boundary
  -> candidate enumeration / policy evaluation
  -> CampaignJournal event
  -> branch/report summary view
  -> inspect view
  -> coverage-gap continuation
  -> learning/outcome export
```

Reports and datasets are derived from journal events. They should not become
independent places that reconstruct decision history.

## Design Rules

- Give every decision a stable `decision_id`.
- Give every candidate a stable `candidate_id`.
- Keep display labels separate from machine identity.
- Store public boundary identity and candidate structure at decision time.
- Prefer structured fields over parsing rendered reports.
- Interpret branch commands relative to the report/checkpoint `run_prelude`,
  not relative to a fresh process start.
- Keep large diagnostics in sidecars unless they are needed for normal
  continuation.
- Treat compatibility fields as views, not new sources of truth.
- Follow [Report Field Admission](REPORT_FIELD_ADMISSION.md) before adding new
  journal, report, or learning-sample fields.
