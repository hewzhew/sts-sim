---
description: Diagnose and fix simulator divergences using sim_diag.py toolkit. Use after shadow verify shows divergences, or when investigating specific step mismatches.
---

# Diagnose Divergences

Use this workflow when the shadow verifier (`run_shadow_verify.py`) reports divergent steps, or when you need to investigate specific simulation mismatches.

**Prerequisite**: Fresh `.pyd` must be installed. See `/dev-checklist`.

// turbo-all

## Phase 1: Get the Numbers

1. Run summary to see current accuracy:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py summary 2>$null
```

2. If accuracy looks wrong or you need fresh data, re-run the full verifier first:
```
cd c:\Dev\rust\bottled_ai_fresh && python run_shadow_verify.py 2>$null
```

## Phase 2: Categorize All Divergences

3. Generate the full divergence report:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py report --out logs/report_full.txt --relics 2>$null
```

4. Read `logs/report_full.txt` — divergences are grouped by category (player_hp, enemy_hp, etc.). Look for:
   - **Cascading patterns**: same Δ across consecutive steps = one root cause
   - **Common powers**: check what powers are active (Pain, Thorns, Hex, etc.)
   - **Common cards**: same card causing divergence across files = card logic bug

## Phase 3: Deep Inspect Specific Steps

5. For any suspicious step, use inspect to see the full state:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py inspect <STEP> --file <PARTIAL_FILENAME> 2>$null > logs/inspect_<STEP>.txt
```

6. Read the inspect output. Check:
   - **Player powers** that might trigger on card play (Pain, Hex, BeatOfDeath)
   - **Enemy powers** that trigger on attacked (Thorns, FlameBarrier, Caltrops, CurlUp)
   - **Card type** interactions (Corruption + Skills, Burst + Skills)
   - **Hand contents** for curse/status cards with passive effects

## Phase 4: Filter by Specific Field

7. Isolate one category at a time:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py report --field player_hp --out logs/report_player_hp.txt 2>$null
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py report --field enemy_hp --out logs/report_enemy_hp.txt 2>$null
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py report --combat "Guardian" --out logs/report_guardian.txt 2>$null
```

## Phase 5: Fix → Build → Verify

8. After making a fix in Rust source, rebuild and install:
```
cd c:\Dev\rust\sts_sim && cargo build --release 2>&1 | Select-String "error"
```

9. Install the fresh .pyd to BOTH locations:
```
cd c:\Dev\rust\sts_sim && Copy-Item "target\release\sts_sim.dll" "target\release\sts_sim.pyd" -Force && Copy-Item "target\release\sts_sim.pyd" "C:\Dev\rust\bottled_ai_fresh\sts_sim.pyd" -Force
```

10. Re-run summary to measure improvement:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py summary 2>$null
```

## Common Pitfalls

- **Stale .pyd**: Python loads from `bottled_ai_fresh/sts_sim.pyd` NOT site-packages. Always copy there.
- **Terminal garbled output**: Always redirect to file with `--out` or `> file.txt 2>$null`
- **Don't write Python one-liners**: Use sim_diag.py subcommands instead. They handle encoding, paths, and caching.
- **CWD matters**: The Rust sim needs to be initialized from `c:\Dev\rust\sts_sim` to find card data. `sim_diag.py` handles this internally.
