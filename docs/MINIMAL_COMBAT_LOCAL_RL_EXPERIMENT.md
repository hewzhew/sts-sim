# Minimal Combat-Local RL Experiment

This is the first RL-facing experiment harness built on top of the natural
combat start path.

It is intentionally small:

- fixed deck
- fixed boss
- combat-only
- explicit train/eval seed split
- terminal-first reward
- no PPO-wide benchmark sweep
- no belief work
- no scorer expansion

## Current Anchor

Config:

- `data/rl_experiments/ironclad_hexaghost_disarm_v1.json`

Natural start:

- `data/boss_validation/hexaghost_v1/start_spec.json`

Purpose:

- test whether a policy can improve on a single boss/deck distribution
- measure robustness over held-out seeds
- keep the reward close to actual combat success, not heuristic soup

## Reward Shape

The current harness uses `reward_mode = minimal_rl` inside `GymCombatEnv`.

Reward terms:

- `victory_reward = +1.0`
- `defeat_reward = -1.0`
- small HP-loss shaping:
  - `player_hp_delta * hp_loss_scale`
- catastrophe penalty when post-step visible unblocked damage is large

Current defaults:

- `hp_loss_scale = 0.02`
- `catastrophe_unblocked_threshold = 18`
- `catastrophe_penalty = 0.25`

This is deliberately smaller and cleaner than the legacy combat env reward.

## Train / Eval Split

The key requirement is that natural-start RNG is real and split by seed.

This harness now supports `start_spec + seed_hint`, so the same deck/boss can
be reinitialized with different combat RNG.

Current config carries:

- `train_seeds`
- `eval_seeds`

The intended use is:

- train only on `train_seeds`
- judge progress only on disjoint `eval_seeds`

## CLI

```powershell
python tools/learning/run_minimal_combat_local_rl.py `
  --config data/rl_experiments/ironclad_hexaghost_disarm_v1.json
```

Outputs land under `tools/artifacts/learning_dataset/` by default:

- `<name>_ppo_model.zip`
- `<name>_rl_metrics.json`
- `<name>_rl_eval_episodes.jsonl`

## What This Harness Is For

- answering whether fixed boss/deck combat-local RL is worth pursuing
- testing seed-robust learning instead of single lucky clears
- creating a clean bridge from natural combat starts into policy learning

## Explicit Non-Goals

- not a full-run RL stack
- not a multi-boss benchmark
- not a belief learner
- not an auxiliary-head training stack yet
- not a replacement for boss validation packs

Those remain separate.

## Next Layer, But Not In This First Harness

If this fixed-deck/fixed-boss harness shows signal, the next additions should
be:

- auxiliary prediction heads
  - next-window HP loss
  - threat relief before the first enemy window
- step/episode trace export for offline analysis
- stronger held-out seed evaluation

That work is intentionally deferred until this minimal loop produces something
better than luck.
