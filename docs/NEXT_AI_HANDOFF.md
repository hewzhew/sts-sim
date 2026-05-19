# Next AI Handoff

Date: 2026-05-19
Branch: `codex/evidence-path-cleanup-20260509`
Workspace: `D:\rust\sts_simulator`
Java source reference: `D:\rust\cardcrawl`
CommunicationMod reference: `D:\rust\CommunicationMod`

This is the short default resume file. It should stay small.

At the start of a resumed session, read only:

1. `git status --short`
2. `git log --oneline -5`
3. this file

Then pick the lane below. Do not read the long Java debug handoff unless the
next task is explicitly mechanics parity/debug.

## Current Project Shape

This repo is a Rust, headless Slay the Spire simulator plus AI-facing harness
work:

- Java-source-backed simulator mechanics parity.
- Public observation and legal action candidate surfaces.
- Deterministic replay and trace/evaluation tooling.
- Experimental LLM controller adapters over the public driver contract.

No current code claims A20H strength. The near-term demo value is:

```text
trusted simulator surface
  -> public observation + legal actions
  -> LLM/human/controller chooses action
  -> simulator validates and steps
  -> trace/replay/probe records what happened
```

## Active Lanes

### Lane A: LLM Integration And Demo

Status: active and currently more urgent.

Read next:

- `docs/LLM_INTEGRATION_HANDOFF.md`
- `tools/llm/README.md`
- `tools/llm/llm_full_run_controller.py`
- `src/bin/full_run_env_driver/main.rs`

Current committed anchor:

- `ae783a2b Add LLM controller harness demo`

What exists:

- `tools/llm/llm_full_run_controller.py`
  - `dry_run`: emits a compact prompt without API calls.
  - `mock`: smoke-tests the controller loop locally.
  - `openai_compatible`: calls `/chat/completions` with `LLM_API_KEY`,
    `LLM_BASE_URL`, and `LLM_MODEL`.
- It reads only public `decision_env_observation` payloads and legal candidates.
- It validates returned `action_id` before stepping the simulator.
- It records model choice, selected action key, reward, done, and env info.

Good next tasks:

- Add a concise demo command/script that runs `dry_run`, `mock`, and optionally
  `openai_compatible`.
- Add a prompt/report summarizer that turns one run into a readable markdown
  artifact for portfolio use.
- Add optional combat probe context to the prompt only at combat decision
  points, clearly marked diagnostic and non-oracle.
- Add a small evaluation wrapper comparing random, mock, and LLM controllers on
  tiny seed suites without claiming policy truth.

Forbidden in this lane:

- Treating LLM choices as teacher labels.
- Letting private/oracle state into prompts.
- Reintroducing Gym/PPO or baseline-as-teacher paths.
- Claiming strategy quality from one or two seeds.

### Lane B: Java Mechanics Parity Cleanup

Status: still valuable, but no longer the default resume context.

Read next only when doing mechanics debug:

- `docs/MECHANICS_ACCEPTANCE_STANDARD.md`
- `docs/JAVA_MECHANICS_DEBUG_HANDOFF.md`
- `docs/JAVA_SOURCE_MAP.md`
- `docs/MECHANICS_AUDIT_LEDGER.md`

Current committed anchors:

- `3ce85a11 Align encounter queue room lifecycle`
- `5ee9f994 Cover medium slime move chains`
- `6b0090fa Cover small slime move chains`

Use this lane for source-backed bug fixes:

```text
exact Java owner
  -> exact Rust owner
  -> focused source-backed test
  -> smallest Rust fix
  -> ledger / audit update
```

Do not broadly re-audit locked mechanisms without a reopen reason from
`MECHANICS_ACCEPTANCE_STANDARD.md`.

## Recent Verification Anchors

- `cargo test --all-targets` -> `1442 passed` after encounter lifecycle fix.
- `python -m py_compile tools\llm\llm_full_run_controller.py`
- `python tools\llm\llm_full_run_controller.py --provider dry_run --steps 1 --out tools\artifacts\llm_demo\dry_run.json`
- `python tools\llm\llm_full_run_controller.py --provider mock --steps 5 --out tools\artifacts\llm_demo\mock_run.json`

## Update Rule

Keep this file short. Put detailed lane history elsewhere:

- LLM route details go in `docs/LLM_INTEGRATION_HANDOFF.md`.
- Java/debug details go in `docs/JAVA_MECHANICS_DEBUG_HANDOFF.md`.
- Mechanism status goes in `docs/MECHANICS_AUDIT_LEDGER.md`.
- Design docs belong under `docs/design/`.
