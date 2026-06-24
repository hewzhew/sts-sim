# Current Direction

Main line:

```text
simulator -> campaign state -> journaled candidates -> search/rollout -> value -> policy improvement
```

The project is building a usable Rust simulator and AI-search workspace for
Slay the Spire. The current maintained loop is:

1. run a deterministic campaign from Neow onward
2. keep a bounded scheduled/parked campaign workset
3. record non-combat decision candidate pools in `CampaignJournal`
4. use coverage-gap or sibling continuation to revisit unobserved candidates
5. use Combat Search V2 for complete combat trajectories inside branches
6. compare whole-run, milestone, combat, and sibling outcomes

## Campaign Architecture Direction

The campaign workflow is migrating to a Rust-owned campaign application:

```text
Campaign CLI -> CampaignApp -> ArtifactStore / ExperimentPlanner / CampaignEngine
```

`tools/campaign.ps1` should become a launcher. It should not own
source/latest/scratch semantics, milestone loops, coverage-gap orchestration, or
manifest writing. The target architecture is defined in
[Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md), the stable CLI
surface in [Campaign CLI Contract](CAMPAIGN_CLI_CONTRACT.md), and the migration
sequence in [Campaign Migration Plan](CAMPAIGN_MIGRATION_PLAN.md).

## Active Work

- simulator correctness and Java-mechanics parity when real runs expose bugs
- migrating campaign source/output, continuation, coverage, inspect, and
  artifact lifecycle ownership out of PowerShell and into Rust
- campaign lifecycle, checkpoint, journal, report, and sidecar boundaries
- route/map candidate pools and coverage-gap continuation
- Combat Search V2 quality, performance, and special-phase handling
- non-combat policy compilers for route, deck mutation, card reward, shop,
  campfire, event, and boss relic choices

## Stable Foundation

`run_play_driver`, `run_control`, trace/replay, bookmarks, non-combat decision
records, and combat captures should keep working. They are stable foundations
and diagnostic tools, not the main expansion layer.

New strategy and branch research should go through campaign, journal,
coverage-gap continuation, policy compilers, and combat search rather than
expanding the manual REPL or adding more ad hoc wrapper switches.

## Route And Journal

Route choices are handled by a planner during normal campaign runs so the
default workset stays bounded. The planner should still record typed route
candidate pools in `CampaignJournal`, allowing coverage-gap continuation to
revisit unobserved map alternatives deliberately.

Detailed route/map journal rules belong in
[Campaign Journal](CAMPAIGN_JOURNAL.md). The short rule is: consume typed route
candidates and candidate-pool provenance, not route display labels or `go N`
strings.

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
- A combat search result is budgeted evidence unless validated by exact replay
  and a benchmark context.
- Human baseline comparison is whole-combat outcome comparison, not stepwise
  action agreement.
- New report, journal, and learning-sample fields should pass
  [Report Field Admission](REPORT_FIELD_ADMISSION.md): classify the field as a
  fact, diagnostic, verdict, or label.
- Campaign artifacts should follow
  [Campaign Artifact Architecture](CAMPAIGN_ARTIFACT_ARCHITECTURE.md):
  checkpoint, journal, report, and diagnostic sidecar data have separate
  ownership.
