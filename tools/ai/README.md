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

Create a local Python environment:

```powershell
& 'C:\Users\17239\.cache\codex-runtimes\codex-primary-runtime\dependencies\python\python.exe' -m venv .venv-ai
.\.venv-ai\Scripts\python.exe -m pip install -r tools\ai\requirements.txt
```

Run a short masked PPO training pass:

```powershell
.\.venv-ai\Scripts\python.exe tools\ai\train_micro_jaw_worm_ppo.py --total-timesteps 20000
```

Evaluate a saved policy against the same seed range as a random legal baseline:

```powershell
.\.venv-ai\Scripts\python.exe tools\ai\eval_micro_jaw_worm_policy.py --episodes 100
```

Run the two-target slime task:

```powershell
cargo build --bin micro_two_slimes_env
.\.venv-ai\Scripts\python.exe tools\ai\smoke_micro_two_slimes_env.py --episodes 5
.\.venv-ai\Scripts\python.exe tools\ai\train_micro_two_slimes_ppo.py --total-timesteps 20000
.\.venv-ai\Scripts\python.exe tools\ai\eval_micro_two_slimes_policy.py --episodes 1000
```

Extract sourced deck slices from replay/live JSON or JSONL files:

```powershell
.\.venv-ai\Scripts\python.exe tools\ai\extract_deck_slices.py path\to\frames.jsonl --source-kind replay
```

Deck slices are provenance-tagged combat inputs, not proof that a card pick was
good. See `deck_slice_schema.md`.

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
