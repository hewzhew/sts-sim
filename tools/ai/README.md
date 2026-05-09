# Micro Jaw Worm RL Hook

This is the first restart surface: a fixed single-combat environment for wiring
Rust simulation to Python RL without using the old bot.

Build the JSONL driver:

```powershell
cargo build --bin micro_jaw_worm_env
```

Run a bridge smoke test:

```powershell
python tools\ai\smoke_micro_jaw_worm_env.py --episodes 5
```

Protocol:

```json
{"cmd":"reset","seed":1}
{"cmd":"step","action":0}
{"cmd":"close"}
```

Action space is fixed at 11 actions:

- `0..9`: play the card in that hand slot.
- `10`: end turn.

The response contains:

- `obs`: 96 `float32`-ready values.
- `action_mask`: 11 booleans.
- `reward`: dense combat reward.
- `done` / `truncated`.
- `info`: human-readable combat state and legal action list.

The Python `MinimalSpireEnv` wrapper in `micro_jaw_worm_env.py` exposes a
Gymnasium-compatible environment and returns `info["action_mask"]` for CleanRL
invalid-action masking scripts.
