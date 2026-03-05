---
description: Pre-flight checklist before and after modifying Rust code. Run this to avoid common mistakes like forgetting to install the new .pyd or missing a lifecycle hook.
---

# Dev Checklist — Rust Code Changes

## Before Starting
// turbo
1. Check current branch: `git status`
2. Quick scan of `known_issues.md` and `SKILL.md` for relevant blind spots

## After Rust Code Changes
// turbo
3. Build: `cargo build --release 2>&1 | Select-Object -Last 10`
4. If build fails, fix errors and rebuild

// turbo
5. Install to Python venv: `maturin develop --release`
   - ⚠️ `cargo build` alone does NOT update the .pyd in `.venv/`!

6. Kill existing Python processes: `Stop-Process -Name python -Force -ErrorAction SilentlyContinue`

// turbo
7. Restart play_ui: `$env:PYTHONIOENCODING='utf-8'; python scripts/play_ui.py`

## Quick Smoke Test (in browser)
8. Open http://localhost:5050
9. Click Map 0 → verify 5 cards in hand, enemies visible
10. Play a Strike → verify damage dealt
11. End Turn → verify new cards drawn

## Before Committing
12. Run `cargo test` (if tests exist for changed area)
13. Check that `interop.rs` lifecycle hooks are wired:
    - `spawn_combat_enemies()` calls `initialize_combat_deck()` + `on_battle_start()` + `start_turn()` + `on_turn_start()`
    - `execute_end_turn()` calls `on_turn_end()` + `end_turn()` + enemy turn + `start_turn()` + `on_turn_start()`
