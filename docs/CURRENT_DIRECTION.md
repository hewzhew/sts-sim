# Current Direction

The main line is:

```text
simulator correctness
  -> Rust-owned campaign application
  -> journaled decision candidate coverage
  -> combat/search evidence
  -> explicit exports for learning or analysis
```

The project is building a Rust simulator and AI-search workspace for Slay the
Spire. The campaign system is being redesigned around a single Rust application
boundary. The PowerShell wrapper is compatibility launch code, not the campaign
architecture.

## Architecture Priority

Read [Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md) first. It
is the target contract, not a description of current accidents. Supporting docs
and current code must move toward it.

The short version:

- Rust owns campaign semantics.
- PowerShell only builds and launches.
- Campaign artifacts have separate owners: checkpoint, state, journal, report,
  diagnostic, export, and manifest.
- The experiment model is journaled decision-candidate coverage, not
  active/frozen branch guessing.
- Reports are bounded projections, not checkpoint, journal, diagnostics, or
  training datasets.
- A migration step only counts when it removes a wrong owner, deletes a bad
  public surface, or replaces string/display identity with typed identity.

## Maintained Loop

The maintained development loop should become:

1. run or continue a campaign through the Rust campaign app
2. record typed decision candidate pools in `CampaignJournal`
3. plan coverage targets from journaled candidates
4. continue selected candidates to milestones or explicit blockers
5. inspect read-only views over artifacts
6. export learning or analysis samples only through explicit exporters

Current compatibility commands may not fully match this loop yet. When behavior
differs, prefer migrating code toward the architecture over documenting wrapper
accidents as normal use.

## Active Work

- simulator correctness and Java-mechanics parity when real runs expose bugs
- migrating source, output, latest, scratch, continuation, coverage, inspect,
  and artifact lifecycle ownership out of PowerShell and into Rust
- enforcing checkpoint/state/journal/report/diagnostic/export boundaries
- route/map candidate pools and candidate-coverage continuation
- Combat Search V2 quality, performance, and special-phase handling
- non-combat policy compilers for route, deck mutation, card reward, shop,
  campfire, event, and boss relic choices

## Stable Foundation

`run_play_driver`, traces, bookmarks, combat captures, and baseline artifacts
remain useful diagnostic tools. They are not the campaign scheduler and should
not define campaign artifact lifecycle.

`run_control` and existing policy compilers can remain as behavior policies and
evidence sources. They are not teacher labels and should not own experiment
scheduling.

## Not The Main Line

- old Python watch UI and recording UI
- Workbench or DecisionFrame expansion
- LLM prompt engineering as the default controller
- live CommunicationMod control as the default development path
- treating route/card/search decisions as teacher labels
- adding more wrapper switches for new probes

Adapters may return later, but only as consumers of stable public observation
and action contracts. They do not define simulator truth or search quality.

## Evidence Rules

- A trace records what happened. It is not a policy-quality claim.
- A guarded autopilot decision is behavior-policy evidence, not a teacher.
- A combat search result is budgeted evidence unless validated by exact replay
  and a benchmark context.
- Human baseline comparison is whole-combat outcome comparison, not stepwise
  action agreement.
- New artifact fields must pass [Report Field Admission](REPORT_FIELD_ADMISSION.md)
  and the ownership rules in
  [Campaign Artifact Architecture](CAMPAIGN_ARTIFACT_ARCHITECTURE.md).
