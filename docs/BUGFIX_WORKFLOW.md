# Bugfix Workflow

Use this when a `live_comm` run reports a concrete parity bug and you want to avoid editing from raw logs.

## Rule

Do not start from the full run.

Always go through:
1. field-scoped extract
2. minimization
3. Java source inspection
4. targeted regression
5. implementation
6. full validation + next live run

Also follow:

- [PROTOCOL_TRUTH_RULES.md](d:\rust\sts_simulator\docs\protocol\PROTOCOL_TRUTH_RULES.md)

## Wrapper

The default entrypoint is:

```powershell
python D:\rust\sts_simulator\tools\analysis\bugfix_workflow.py from-live `
  --field player.power[Draw Reduction] `
  --from-response-id 777 `
  --to-response-id 782
```

This automatically runs:
- `live_regression.py extract`
- `live_regression.py minimize`

And writes:
- an extracted fixture in `tests/live_regressions/`
- a minimized fixture in `tests/live_regressions/`
- a `.notes.md` template beside them

## Required Discipline

For every parity bug:

- Pick exactly one primary field first.
- Use the minimized fixture, not the raw run, as the active reproduction target.
- Read the Java source that actually controls future state.
- Add a targeted regression before broad refactors.
- Only after that, touch Rust implementation code.

## Typical Commands

### 1. Generate artifacts

```powershell
python D:\rust\sts_simulator\tools\analysis\bugfix_workflow.py from-live `
  --field player.energy `
  --from-response-id 777 `
  --to-response-id 782
```

### 2. Re-run the minimized fixture directly

```powershell
$env:LIVE_REGRESSION_FIXTURE="D:\rust\sts_simulator\tests\live_regressions\player_energy_r777_782.min.json"
cargo test --test live_regression_driver replay_single_fixture_from_env -- --nocapture
```

### 3. Validate after the fix

```powershell
cargo test -q
```

Then run a fresh `live_comm` round.

## Field Selection Guidance

Pick the most causal field first, not the noisiest one.

Examples:
- `player.power[Draw Reduction]`
- `player.energy`
- `discard_pile_size`
- `monster[0].hp`

Avoid trying to fix an entire combat summary in one pass.

## When To Export Internal Java State

Default to Java-first parity for any state that:
- affects future move selection
- affects queued or delayed resolution
- is hidden/private in Java

Do not rely on surrogates like `move_history` when Java uses a dedicated internal state machine.

If protocol truth is missing, add it. Do not patch around it with previous-state carry
in the live path.

## Output Convention

Each workflow run creates:
- `<slug>.json`
- `<slug>.min.json`
- `<slug>.notes.md`

The notes file is the human checklist. Fill it in before broad code edits.
