# LLM Integration Handoff

Date: 2026-05-19

This file is the working memory for the LLM-controller route. It should describe
the demo/product direction and current harness, not detailed Java mechanics
debug history.

## Goal

Build a demonstrable LLM-assisted Slay the Spire decision harness:

```text
Rust simulator truth
  -> public observation
  -> legal action candidates
  -> LLM/human/controller chooses one action
  -> simulator validates and steps
  -> trace/eval/probe summarizes the result
```

The value proposition is not "the LLM is already strong at the game." The value
is an auditable tool loop where cheap models can read game state, choose legal
actions, receive feedback, and produce traces that humans or later learners can
inspect.

## Current Implementation

Primary files:

- `src/bin/full_run_env_driver/main.rs`
  - JSON line driver.
  - Supports `reset`, `observation`, `decision_env_observation`,
    `decision_env_step`, `decision_record_step`, and `close`.
- `tools/llm/llm_full_run_controller.py`
  - Starts `full_run_env_driver`.
  - Gets `decision_env_observation`.
  - Builds a compact public-state prompt.
  - Asks a provider to return strict JSON with `action_id`.
  - Validates `action_id` against current legal candidates.
  - Steps the driver and records choice/reward/info.
- `tools/llm/README.md`
  - Usage examples for dry-run, mock, and OpenAI-compatible APIs.

Committed anchor:

- `ae783a2b Add LLM controller harness demo`

## Modes

```text
dry_run:
  No model call. Writes prompt/report for inspection.

mock:
  Local placeholder chooser. Tests the adapter loop and action validation.

openai_compatible:
  Calls BASE_URL/chat/completions.
  Environment:
    LLM_API_KEY
    LLM_BASE_URL
    LLM_MODEL
```

DeepSeek-style example:

```powershell
$env:LLM_API_KEY = "<key>"
$env:LLM_BASE_URL = "https://api.deepseek.com"
$env:LLM_MODEL = "deepseek-chat"

python tools\llm\llm_full_run_controller.py `
  --provider openai_compatible `
  --steps 5 `
  --out tools\artifacts\llm_demo\llm_run.json
```

## Current Prompt Contract

The model sees:

- public decision type and engine state;
- HP, floor, act, boss, gold, deck summary;
- combat summary when in combat;
- hand card ids, costs, playability, and coarse semantics;
- screen counts / rewards / next nodes where public;
- legal candidate ids and action keys.

The model must return:

```json
{"action_id": 0, "confidence": "medium", "reason": "short reason"}
```

The harness does not trust the model. It validates that `action_id` is legal and
falls back to the first legal action if needed.

## Non-Goals And Safety

- LLM decisions are not teacher labels.
- LLM decisions are not mechanics truth.
- The prompt must not include private/oracle state.
- One run is not an evaluation claim.
- This lane must not reintroduce PPO/Gym-first training or weak baseline
  continuation labels.

## Immediate Next Tasks

1. Create a portfolio demo wrapper.
   - Run `dry_run`.
   - Run `mock`.
   - Optionally run `openai_compatible`.
   - Write a short markdown report under `tools/artifacts/llm_demo/`.

2. Add run summarization.
   - Convert JSON records into readable text:
     - decision type;
     - candidate count;
     - selected action;
     - model reason;
     - reward/done/floor/hp.

3. Add optional combat probe context.
   - At combat decisions, call existing combat plan-probe only when requested.
   - Summarize top few plan comparisons for the LLM.
   - Mark probe output as diagnostic, not oracle.

4. Add a tiny seed-suite comparison.
   - random/masked baseline from `sts_dev_tool run-batch`;
   - `mock`;
   - one LLM provider if configured.
   - Report illegal action rate, deterministic crashes, average floor, and
     qualitative examples.

5. Improve prompt budget.
   - Reduce full deck/map verbosity by default.
   - Include only top-level deck summary plus current hand/screen/action
     candidates.
   - Add `--prompt-detail` later if useful.

## Demo Story

For portfolio/resume use, describe this as:

> A Java-source-backed Rust Slay the Spire simulator exposed through a public
> observation and legal-action driver, with an experimental LLM controller
> harness that validates every model-selected action and records auditable traces
> for replay, debugging, and future data collection.

Avoid claiming:

- strong gameplay;
- A20H readiness;
- learned policy quality;
- search/value labels.
