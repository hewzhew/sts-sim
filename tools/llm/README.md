# LLM Harness Tools

This directory contains experimental controller adapters that connect external
LLMs to the simulator through the public `full_run_env_driver` contract.

The intent is to test an LLM-assisted decision loop:

```text
public observation + legal action candidates
  -> compact prompt
  -> LLM chooses one legal action id
  -> driver validates and steps
  -> trace/report records the choice
```

This is not a teacher-label pipeline. LLM decisions are controller behavior and
may be logged for analysis, replay, and debugging, but they are not trusted
mechanics truth and are not promoted to policy labels by default.

## Dry Run

Inspect the prompt without calling an API:

```powershell
python tools\llm\llm_full_run_controller.py `
  --provider dry_run `
  --steps 1 `
  --out tools\artifacts\llm_demo\dry_run.json
```

## Mock Smoke Test

Run the adapter loop with a local placeholder chooser:

```powershell
python tools\llm\llm_full_run_controller.py `
  --provider mock `
  --steps 5 `
  --out tools\artifacts\llm_demo\mock_run.json
```

By default the controller now writes compact JSONL decision journal events. Use
`--trace-level full` only when you need the older full debug JSON.

## Planner + Simulator Tools

Run the LLM as a bounded planner that requests simulator evidence before the
final action. Combat can be committed by search/verifier instead of asking the
model to do tactical arithmetic:

```powershell
python tools\llm\llm_full_run_controller.py `
  --provider mock `
  --agent-mode planner `
  --combat-decision-owner search `
  --trace-level compact `
  --steps 120 `
  --out tools\artifacts\runs\planner_mock_seed42.jsonl `
  --log-decisions
```

Supported planner tools:

- `combat_turn_probe`
- `candidate_afterstate_summary`
- `campfire_rest_smith_eval`

Planner/search journal events are controller behavior, not teacher labels, not
policy-quality evidence, and not training data.

## Portfolio Demo Wrapper

Run the safe demo sequence and write a markdown report:

```powershell
python tools\llm\run_llm_demo.py
```

This writes:

- `tools\artifacts\runs\{run_id}\manifest.json`
- `tools\artifacts\runs\{run_id}\status.json`
- `tools\artifacts\runs\{run_id}\inputs\run_config.json`
- `tools\artifacts\runs\{run_id}\raw\dry_run.json`
- `tools\artifacts\runs\{run_id}\raw\mock_run.json`
- `tools\artifacts\runs\{run_id}\reports\demo_report.md`

If `LLM_API_KEY`, `OPENAI_API_KEY`, or `DEEPSEEK_API_KEY` is configured, the
wrapper also runs `openai_compatible` and writes
`tools\artifacts\runs\{run_id}\raw\llm_run.json`. To force or disable that
behavior:

```powershell
python tools\llm\run_llm_demo.py --openai-compatible always
python tools\llm\run_llm_demo.py --openai-compatible never
```

Each run directory is self-contained. `manifest.json` declares that the
artifacts are descriptive controller behavior, not teacher labels, not policy
evaluation, and not training data. `reports\demo_report.md` is a generated
human-readable view over the run artifacts.

## OpenAI-Compatible API

Set environment variables, then run:

```powershell
$env:LLM_API_KEY = "<key>"
$env:LLM_BASE_URL = "https://api.deepseek.com"
$env:LLM_MODEL = "deepseek-chat"

python tools\llm\llm_full_run_controller.py `
  --provider openai_compatible `
  --steps 5 `
  --out tools\artifacts\llm_demo\llm_run.json
```

The model must return JSON:

```json
{"action_id": 0, "confidence": "medium", "reason": "short reason"}
```

The harness validates the selected `action_id` against the current legal
candidate ids before stepping the simulator.
