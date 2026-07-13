# Smoke Bomb survival fallback implementation plan

1. Add failing tests proving that an escape-only run-control option applies Smoke Bomb after search finds no win and that only non-boss survival lanes enable it.
2. Add the explicit option and dispatch the cheap Smoke Bomb fallback without invoking the full rescue chain.
3. Wire elite and hallway survival lane options and evidence bookkeeping.
4. Run focused tests, commit, and rerun seed `20260713002` to verify progress beyond Giant Head.
5. Continue the bounded mainline run and address the next general reliability obstruction if evidence supports a bounded fix.
