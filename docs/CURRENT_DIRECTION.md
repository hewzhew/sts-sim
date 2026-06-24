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

## What Is Not Current Mainline

- adding more PowerShell wrapper switches
- preserving legacy wrapper manifests
- treating coverage-gap milestone output as normal campaign lifecycle
- keeping stale docs or helper scripts searchable after their owner is retired

If a stale tool or document conflicts with the three maintained lifecycle
concepts, remove it or move the behavior into an explicit Rust command.
