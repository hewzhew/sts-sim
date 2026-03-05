---
description: Run 1000-game simulations to verify combat engine correctness after changes
---

# Verify Simulator Correctness

Run this workflow after any change to `engine.rs`, `enemy.rs`, `state.rs`, or monster data.

// turbo-all

## Steps

1. Build all test binaries:
```
cargo build --release --bin sentries_test --bin jaw_worm_test
```

2. Run Jaw Worm test (expect 50-70% win rate for greedy Ironclad):
```
cd c:\Dev\rust\sts_sim && target\release\jaw_worm_test.exe
```

3. Run Sentries test (expect 5-15% win rate — elite fight, heavy damage):
```
cd c:\Dev\rust\sts_sim && target\release\sentries_test.exe
```

4. Check the output for these sanity indicators:
   - **Win rate in expected range** (see above)
   - **Avg Dazes > 0** for Sentries (Bolt mechanic is working)
   - **No "⚠️" warnings** in the sanity check section
   - **Avg turns reasonable**: Jaw Worm 3-8, Sentries 5-15

5. If win rates are outside expected range:
   - Run the verbose game (seed=42) and trace turn-by-turn
   - Check if damage/effects are being applied correctly
   - Check for duplicate `plan_enemy_moves` calls (common bug)
   - Compare against Java source using SKILL.md Lookup Recipes
