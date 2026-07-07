# Runbook

This file keeps current local commands in one place. It is command-oriented;
architecture rules belong in [ARCHITECTURE.md](ARCHITECTURE.md).

## Branch Tiny And Branch Panels

`branch_tiny` is the lightweight owner-audit runner. It writes run capsules
with `summary.json`, `path.json`, optional `frontier.json`, optional
`terminal.json`, and combat cases when combat search blocks.

Run one seed:

```powershell
cd D:\rust\sts_simulator
cargo run --bin branch_tiny -- --seed 1552225673 --ascension 0 --class ironclad --max-branches 1 --wall-ms 60000
```

Run a small panel:

```powershell
cargo run --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Use the panel to classify blockers. Do not treat one seed as a strategy verdict.

For bounded continuation, use `drain`:

```powershell
cargo run --bin branch_panel -- panel drain --seeds 1552225671 1552225672 --capsule-root tools/artifacts/panels/current --max-slices 3 --slice-ms 60000
```

Compare named search profiles without mutating the base seed capsules:

```powershell
cargo run --bin branch_panel -- panel compare --profiles baseline,double-search --seeds 1552225671 1552225672 --capsule-root tools/artifacts/panels/current --max-slices 1 --slice-ms 60000
```

Compare capsules are materialized under `_compare/<profile>/<seed>`.

`tools/gap_panel.py` is a deprecated compatibility wrapper over
`branch_panel`; do not add new panel semantics there.

## Continue A Capsule

When a capsule soft-stops with a frontier, continue from the capsule instead of
rerunning from Neow:

```powershell
cargo run --bin branch_tiny -- --continue-capsule <capsule-dir>
```

Continuation may inherit relevant run-contract values such as `wall_ms` from
the capsule manifest. Override only when the investigation needs a different
contract.

## Combat Case Review

For saved combat gaps, start from the case:

```powershell
cargo run --bin combat_case_review -- --case <case.json> --ladder
```

Review output is diagnostic. It does not mutate runner policy and does not
prove a deck is good or bad by itself.

## Campaign Launcher

The Rust campaign namespace owns campaign behavior:

```powershell
cargo run --profile fast-run --bin branch_campaign_driver -- campaign run --preset quick --seed 1 --rounds 0
cargo run --profile fast-run --bin branch_campaign_driver -- campaign artifacts resolve latest --json
```

`tools/campaign.ps1` is a local launcher for source selection, output
allocation, and small continuation runs:

```powershell
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
.\tools\campaign.ps1 -From latest -Inspect
```

The wrapper must not own manifests, milestone loops, coverage-gap policy,
report shaping, or artifact schema semantics.

## Manual Run Play Driver

Use `run_play_driver` for manual or semi-automatic inspection of one simulator
run:

```powershell
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
cargo run --profile fast-run --bin run_play_driver -- --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
```

Common commands:

| Command | Meaning |
| --- | --- |
| `ar` | auto-run with guarded route/card/search helpers until a boundary stops |
| `n` | guarded advance without route planning |
| `nr` | guarded advance with route planning |
| `rs` / `rg` | route suggestion / execute one route choice |
| `bd` | show current non-combat decision record summary |
| `sc` | run combat search from the current combat boundary |
| `sd` | inspect or update search defaults |
| `mark <name>` | save a replay bookmark while recording |
| `q` | quit cleanly |

Useful panels:

```text
deck | map | mf | bd | relics | potions | draw | discard | exhaust | inspect <id> | details | raw
```

Resume a recorded bookmark:

```powershell
cargo run --profile fast-run --bin run_play_driver -- --goto <name> --search-wall-ms 100
```

Reward-screen note: opening a card reward and skipping that card reward are
different from leaving an outer reward screen while other rewards remain.

## Combat Search Driver

Use `combat_search_v2_driver` for fixed combat starts, captures, and benchmark
suites:

```powershell
cargo run --release --bin combat_search_v2_driver -- --start-spec <path>
```

Common investigation switches include:

```text
--combat-snapshot <path>
--benchmark-spec <path>
--validate-only
--potion-policy all --max-potions-used <n>
--max-hp-loss <n|off>
```

If combat search reports unresolved, it only failed to find an executable
complete win under the current contract. It did not prove the fight unwinnable.

## Verification

For core code changes:

```powershell
cargo fmt --check
cargo check --all-targets
cargo check --release --all-targets
cargo build --profile fast-run --bin branch_campaign_driver
cargo build --release --bin run_play_driver
cargo build --release --bin combat_search_v2_driver
git diff --check
```

For documentation-only changes:

```powershell
git diff --check
```

Run targeted tests only when the changed surface has a stable structural
contract worth protecting. Do not add or preserve tests for retired probes,
temporary reports, or prose-only behavior.
