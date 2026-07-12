# Combat Case Review Orphan Lenses Retirement Design

**Date:** 2026-07-12

## Purpose

`combat_case_review` remains a supported diagnostic binary, but its saved-case review adapter has
accumulated several opt-in, one-off experiments that no current tool or operator workflow invokes.
They enlarge the binary, its JSON schema, and the shared combat-search implementation even though
the maintained frozen-case and success-feedback panels use only two named lane families.

This delivery retires six unconsumed experiment families and the unconsumed optional
`turn-plan-ladder` branch while preserving the supported review ladder, both active panels, and the
evidence probes that still feed boss diagnosis. The cleanup is a diagnostic-surface decision, not a
claim that the experiments never produced useful observations.

The working tree is clean at `008326e63cb8b9e471e409dfa9d9ba8d6f941b81`. The immutable remote
backup `backup/pre-cleanup-20260712` at
`1ee108d0f53806f6b53c5169b74949b28e8648ce` also contains the complete pre-cleanup implementation.

## Current Evidence

The `src/bin/combat_case_review/` tree currently contains 89 Rust files, about 6,933 physical
lines, and 25 `#[test]` markers. Repository-wide active-caller searches found:

- `tools/frozen_case_panel.py` invokes `--frozen-panel-lanes` and consumes
  `frozen_panel_lanes`;
- `tools/success_feedback_panel.py` invokes `--quality-lanes` and consumes `quality_lanes`;
- current README and runbook examples invoke the generic `--ladder` path;
- no current source, tool, test fixture, README, or runbook invokes the six experiment flags or
  `--turn-plan-ladder`;
- historical specifications and plans mention some experiments, but the repository cleanup
  authority explicitly excludes historical plans when establishing active support.

The six standalone experiment families occupy 19 Rust files and about 1,684 physical lines before
shared wiring is removed:

| Experiment family | Rust files | Physical lines | `#[test]` markers |
| --- | ---: | ---: | ---: |
| Boss setup lane | 1 | 63 | 0 |
| Forced potion opening lanes | 1 | 373 | 0 |
| Key-card counterfactual | 4 | 264 | 2 |
| Key-card decision microscope | 4 | 261 | 0 |
| Root-action role duel | 8 | 542 | 1 |
| Collector tactic lanes | 1 | 181 | 2 |
| **Total** | **19** | **1,684** | **5** |

The Collector lane is also the only active owner of
`CombatSearchActionPriorPluginId::{CollectorSingleHeadControl, CollectorBossRace}`. Its library
closure includes `collector_tactic.rs`, Collector-specific frontier and action-ordering fields,
enum variants, conversions, labels, and focused tests. Removing only the lane adapter would leave
this strategy code unreachable.

Two superficially similar probes are not retirement candidates in this delivery:

- `line_lab` feeds combat-deficit evidence and the boss-pressure lens; its library implementation
  is also used by run-control's no-win fallback;
- `counterfactual_hp` feeds the Awakened One failure-evidence frame and reuses active quality-lane
  comparison types.

## Considered Approaches

### Hide or deprecate the unused flags

This preserves compatibility but leaves every module, schema field, test, and link-time dependency
in place. There is no maintained caller requiring a deprecation window, so this does not achieve
the repository cleanup goal. This approach is rejected.

### Retire all optional probes, including line-lab and HP evidence

This removes more code, but crosses into current boss-evidence ownership and run-control behavior.
Those dependency chains need their own evidence and design rather than being pulled into a CLI
cleanup. This approach is rejected for this delivery.

### Retire the complete orphan ownership chains

The selected approach removes the six CLI/payload experiment families and the unused ladder branch,
then removes only the Collector-specific library code made unreachable by that boundary. It keeps
generic capabilities when another maintained surface still owns them. This gives real source and
link-boundary reduction without turning the pass into a combat-search redesign.

## Product Boundary After Retirement

### Retained `combat_case_review` behavior

- saved `CombatCase` loading and the generic fast/slow `--ladder`;
- focus selection, exact witness replay, and focus-prior rerun;
- `--quality-lanes` and `tools/success_feedback_panel.py`;
- `--frozen-panel-lanes` and `tools/frozen_case_panel.py`;
- `line_lab`, combat-deficit evidence, and boss-pressure aggregation;
- counterfactual-HP and Awakened One evidence;
- static boss matchup, acquisition pressure, Awakened One path, Champ phase, strategic feedback,
  and key-card lifecycle reports;
- generic decision-microscope and review-search-intervention helpers still used by retained code.

### Retired `combat_case_review` behavior

- `--boss-setup-lane` and `boss_setup_lane` output;
- `--forced-potion-opening-lanes` and `forced_potion_opening_lanes` output;
- `--key-card-counterfactual` and `key_card_counterfactual` output;
- `--key-card-decision-microscope` and `key_card_decision_microscope` output;
- `--root-action-role-duel` and `root_action_role_duel` output;
- `--collector-tactic-lanes` and `collector_tactic_lanes` output;
- `--turn-plan-ladder` and its third diagnostic ladder row.

No deprecated flag alias, always-null JSON compatibility field, or replacement probe is kept.

## Explicit Keep Boundary

Name overlap is not authority to delete a shared implementation. The following remain:

- `src/bin/combat_case_review/quality_lanes.rs` and its submodules;
- `src/bin/combat_case_review/frozen_panel_lanes.rs` and its submodules;
- `src/bin/combat_case_review/line_lab.rs` and
  `src/ai/combat_search_v2/line_lab/`;
- `src/bin/combat_case_review/counterfactual_hp.rs` and its submodules;
- `src/bin/combat_case_review/key_card_lifecycle.rs`;
- `src/bin/combat_case_review/search_intervention.rs`, because retained focus and quality flows use
  it;
- generic `explain_combat_search_v2_initial_decision`, because
  `combat_search_v2_driver` and guidance labs use it;
- `KeyCardOnline` setup-bias behavior, because the retained frozen panel uses it;
- `TurnBoundaryFrontierSeed` and tactical turn-boundary policies, because
  `combat_search_v2_driver`, run-control options, and combat-search tests use them;
- all game mechanics, run-control ownership, and unrelated combat-search policies.

If implementation discovers another active caller for a retired symbol, stop and reassess rather
than moving that caller or silently widening the retirement boundary.

## Layered Retirement

### Layer 1: Saved-case experiment adapters

Delete the 19 experiment-family files and remove their module declarations, Clap arguments,
`ReviewOptions` fields, pipeline calls, artifact fields, and serialized root fields. Remove only the
`turn_plan_ladder` option and its conditional third ladder profile; retain the underlying
turn-boundary plugin.

The layer must leave `combat_case_review --help` with `--quality-lanes`,
`--frozen-panel-lanes`, `--line-lab`, and `--counterfactual-hp-probe`, while none of the seven
retired flags remain. The active Python panel tests and the binary's remaining Rust tests must pass
before library cleanup begins.

### Layer 2: Orphaned Collector search policy

Delete `src/ai/combat_search_v2/collector_tactic.rs` and remove its module declaration. Remove only
the two Collector action-prior/setup-bias variants, conversions, labels, action-ordering field,
frontier-priority field, calculations, and Collector-specific tests.

Do not change the default comparator semantics for any retained action-prior policy. After removal,
active source must contain no Collector tactic symbol or label, while the ordinary Collector enemy
implementation and all non-tactic combat behavior remain intact.

## JSON and Consumer Contract

The root schema name remains `combat_case_review`. Removing the six optional nested keys is an
intentional breaking change to an unsupported portion of that schema. Retained keys keep their
names and shapes.

`tools/frozen_case_panel.py` and `tools/success_feedback_panel.py` must not be changed to compensate
for the retirement. Their existing tests prove that the two supported nested contracts survive.
Default ladder output must continue to contain the fast no-potion and slow all-potion rows in that
order.

## Documentation Contract

`docs/architecture/supported-surfaces.md` remains the current-state authority. Its
`combat_case_review` entry must stop naming Collector tactic output as a supported nested schema,
and its retirement history must record the removed lens families and Collector policy closure.

`docs/architecture/combat_experiment_panel.review-draft.md` remains as historical reasoning but
must receive a prominent historical/superseded status note so its manual-probe language cannot be
mistaken for current support. Historical files under `docs/superpowers/specs` and
`docs/superpowers/plans` remain unchanged.

## Verification Strategy

Layer 1 focused verification:

```powershell
cargo fmt --all -- --check
cargo test --bin combat_case_review
python -m unittest tests.test_frozen_case_panel tests.test_success_feedback_panel
cargo check --bin combat_case_review
```

Layer 2 focused verification must cover retained plugin conversions, action ordering, frontier
ordering, and search behavior with focused library filters before the full suite.

Final verification:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo test --test architecture_runtime_boundaries
python -m unittest discover -s tests -p 'test_*.py'
cargo check --bins
git diff --check
```

Additional structural checks must prove:

- Cargo metadata still lists exactly the six supported binaries;
- active source and tools contain none of the retired flags, payload fields, Collector tactic
  variants, or Collector tactic labels;
- active panel tools and their nested JSON keys remain present;
- all explicit keep-boundary paths and symbols remain present;
- the worktree contains no unexpected generated or ignored-file changes.

Post-removal Rust-file, physical-line, test-marker, actual-test, and tracked-byte counts are recorded
in the handoff and supported-surface inventory. Counts are evidence of the declared closure, not a
quota that authorizes unrelated deletion.

## Commit and Rollback Discipline

Use two bounded implementation commits after the design and plan commits:

1. retire the six saved-case experiment adapters and optional ladder branch;
2. remove the orphaned Collector search policy and update current-state documentation.

Each implementation commit must compile and pass its focused checks. If a layer proves harmful,
use `git revert` in reverse dependency order. Do not reset, force-push, rewrite the immutable backup,
or push public `master` without separate user authorization.

## Failure Handling

- Active consumer found: stop and classify it before deleting the producer.
- Retained panel schema changes: revert the accidental change; do not modify the consumer test to
  accept it.
- Collector removal changes default-policy ordering: restore the neutral default path before
  continuing.
- Focused or full test failure: diagnose the first failure before starting the next layer.
- Unexpected dirty or generated files: preserve them and identify their owner.
- Need for an old experiment later: recover it from Git history or the immutable backup in a
  separate checkout rather than adding compatibility code to current mainline.

## Non-Goals

- Do not remove or redesign `combat_case_review` itself.
- Do not narrow run-control in this delivery.
- Do not change combat search defaults, runner policy, card rewards, routing, shops, events, or
  campfire ownership.
- Do not remove active frozen-case or success-feedback tooling.
- Do not retire line-lab, HP counterfactual, Boss, Awakened One, Champ, strategic-feedback, or
  key-card lifecycle evidence.
- Do not reorganize surviving modules merely to increase the deletion count.
- Do not delete historical specifications or implementation plans.
- Do not clean Cargo caches, Python environments, ignored artifacts, or generated run capsules.
