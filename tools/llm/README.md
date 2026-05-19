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
