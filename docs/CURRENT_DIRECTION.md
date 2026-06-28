# Current Direction

The current campaign-tooling priority is deliberately small:

```text
source selection
  -> output allocation
  -> minimal continuation
```

Everything else is secondary until this path is stable and easy to reason
about.

The next lifecycle design is documented in
[CAMPAIGN_WORKSPACE_V2.md](CAMPAIGN_WORKSPACE_V2.md). It replaces the idea that
each driver invocation is the durable experiment object with a workspace model:
workspace -> attempts -> snapshots/observations -> derived views.

## Maintained Campaign Launcher

`tools/campaign.ps1` is a local launcher. It may:

- build or locate `branch_campaign_driver`
- resolve a source artifact such as `latest` or `run:<id>`
- allocate a new run output artifact
- run a new campaign, continue a source for a small round budget, or inspect a
  source checkpoint summary

It must not own:

- wrapper manifests
- coverage-gap orchestration
- milestone loops
- scratch-latest shortcut semantics
- report shaping
- learning/export policy

Those features either belong directly in Rust commands or are retired from the
main workflow.

## Practical Commands

```powershell
cd D:\rust\sts_simulator
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
.\tools\campaign.ps1 -From latest -Inspect
```

Use the Rust campaign namespace directly when debugging driver behavior:

```powershell
cargo run --profile fast-run --bin branch_campaign_driver -- campaign run --preset quick --seed 1 --rounds 0
cargo run --profile fast-run --bin branch_campaign_driver -- campaign artifacts resolve latest --json
```

## Decision Debugging Baseline

When investigating bad non-combat choices on a fixed seed, first run with:

```powershell
.\tools\campaign.ps1 -Mode deep -RetentionProfile advisory_only
```

`advisory_only` keeps candidate generation and journals, but mutes branch
retention and campaign scheduler strategy influence. Use it as the baseline
before changing card reward, shop, campfire, event, or route strategy.

## Combat Search Experiment Notes

Current combat gap review starts from a saved `combat_case`:

```powershell
cargo run --bin combat_case_review -- --case <case.json> --ladder
```

This is the preferred entry for branch-tiny combat gaps. Do not revive old
report/probe readers for this workflow; write or load a `combat_case` and keep
review output structured JSON.

Current small combat benchmarks:

```text
combat_gap_case: seed1552225672 A2F19 Spheric Guardian
baseline line probe: win, hp_loss=9
line repair probe: win, hp_loss=7

combat_gap_case: fixtures/combat_cases/seed1700000123_a2f23_slavers_b0034.json
1000ms default tactical turn-plan seed: no complete win
1000ms diagnostic_only: no-potion win, final_hp ~= 32
2000ms: no-potion win, final_hp ~= 32
```

The Spheric Guardian case is useful for complete-line repair: find one
executable win, then cut a prefix and repair the suffix. The Slavers case is
different: it is a first-win discovery threshold case. It should be used to
guard against expensive turn-plan seeding hiding a cheap complete win inside
the smaller budget.

Two ideas are worth keeping for later cases, but are not current wins on the
Spheric Guardian hallway benchmark:

- NMCS-style one-step lookahead may help long fights where the search cannot
  find the first complete win. It is a coarse guide toward plausible half-lines,
  not a good precision improver once a win already exists.
- Setup pull-forward may help cases where a delayed power is visibly the
  bottleneck. On the Spheric Guardian case it produced legal attempts but no
  improvement, so do not keep a generic implementation without a better case.

## What Is Not Current Mainline

- adding more PowerShell wrapper switches
- preserving legacy wrapper manifests
- treating coverage-gap milestone output as normal campaign lifecycle
- keeping stale docs or helper scripts searchable after their owner is retired

If a stale tool or document conflicts with the three maintained lifecycle
concepts, remove it or move the behavior into an explicit Rust command.
