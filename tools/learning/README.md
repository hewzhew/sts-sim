# Learning Tools

This directory is intentionally narrow now. The active route is not a learned
selector and not a local tactical oracle. The current mainline is:

```text
Rust engine
-> verified teacher diagnostics
-> audit harmful overrides
-> improve leaf / continuation / evidence quality
-> only later distill a proposer or policy
```

## Current Mainline

Use these files when working on the current route:

- `return_q_common.py`
  - shared JSONL driver process wrapper and file helpers
- `eval_verified_adv_override_rust_runner.py`
  - runs the Rust-side verified override episode/batch policy
- `run_verified_teacher_diagnostics.py`
  - compares rule/plan baselines against verified teacher variants and writes a
    compact report
- `audit_verified_teacher_pending_coverage.py`
  - checks how much decision-state / pending-choice coverage the teacher sees
- `run_verified_adv_override_prefilter_grid.py`
  - optional verifier prefilter experiment; not a default policy

The current teacher policy is conservative:

```text
default action = rule_baseline_v0
override only when cloned engine return verifies a candidate advantage
```

The latest useful improvement is selective extended verification:

```text
H8 evaluates all scoped root candidates.
If the H8-best candidate has low evidence, confirm only {rule, best} at a
longer horizon such as H16.
Accept only if the confirmed advantage clears the confirmation margin.
```

This is an evidence-quality mechanism, not a card/action value rule.

## Current Decision

Do not treat these as active mainlines:

- shallow `card/action + query` labels
- candidate-pack labels as a primary training target
- absolute return-Q direct selectors
- learned advantage override selectors
- learned verified proposer pruning
- top-K coverage as proof that a proposer works
- full MCTS / recursive search driven by a weak learned Q

Those experiments are either diagnostic fixtures or negative baselines.

## Archived Routes

Failed or non-mainline experiments moved out of the root directory live under:

- `_archive/2026_05_failed_routes/return_q_negative_baselines/`
- `_archive/2026_05_failed_routes/learned_adv_override_negative_baseline/`
- `_archive/2026_05_failed_routes/verified_proposer_negative_baselines/`
- `_archive/2026_05_failed_routes/candidate_pack_diagnostics/`

See `_archive/2026_05_failed_routes/README.md` for the rationale.

## Draw / Cashout / Query Tools

The draw marginal and cashout tools remain useful only as diagnostics,
fixtures, and leakage/regression tests. They are not training proof:

- `build_draw_query_axis_dataset.py`
- `run_trace_draw_marginal_mining.py`
- `analyze_draw_marginal_labels.py`
- cashout and micro-probe scripts

Use them when checking plumbing, split hygiene, or negative controls. Do not use
their labels as the main supervised signal.

## Verified Teacher Smoke

Build:

```powershell
cargo build --release --bin full_run_env_driver
```

Run a small diagnostic:

```powershell
python tools\learning\run_verified_teacher_diagnostics.py `
  --out target\verified_teacher_smoke.json `
  --report-out target\verified_teacher_smoke.md `
  --episodes 20 `
  --seed-start 98100 `
  --max-steps 160 `
  --cases rule,h8_fixed `
  --oracle-margin 1.0 `
  --evidence-gate horizon_cap_any_v1 `
  --low-evidence-margin 2.0 `
  --confirm-low-evidence-horizon-decisions 16 `
  --confirm-low-evidence-margin 2.0 `
  --keep-episodes
```

Key metrics:

- reward delta vs rule
- defeats and rescued/harmed seed counts
- override rate
- low-evidence rejects
- confirmation accepts/rejects
- candidate eval count and policy step eval count
- horizon stop reason distribution
- override payoff/context distribution

## Refactor Rule

Before adding a new learning script, answer:

```text
Does this improve the verified teacher, explain harmful overrides, or reduce
verified compute without changing the verifier's truth source?
```

If not, put it under `_archive/` or a clearly named diagnostic directory from
the start.
