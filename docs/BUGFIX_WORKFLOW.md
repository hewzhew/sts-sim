# Bugfix Workflow

Use this when a `live_comm` run reports a concrete parity bug and you want a
replayable artifact without editing from raw logs.

## Rule

Do not start from the full run.

Always go through:

1. select one target field
2. extract a small live window
3. minimize the bridge fixture
4. materialize a `CombatCase` witness and reduce it
5. inspect Java source
6. reproduce with the reduced case
7. implement and validate

Also follow:

- [protocol/PROTOCOL_TRUTH_RULES.md](protocol/PROTOCOL_TRUTH_RULES.md)

## Default Entry Point

```powershell
python tools\analysis\bugfix_workflow.py from-snapshot `
  --run-dir logs\runs\20260420_001126 `
  --snapshot-id f216_r216_s216_engine_bug
```

Equivalent explicit window workflow:

```powershell
python tools\analysis\bugfix_workflow.py from-live `
  --field monster[0].power[Metallicize] `
  --from-response-id 215 `
  --to-response-id 216
```

## Outputs

Each workflow run writes:

- `<slug>.json`
  - extracted legacy fixture
- `<slug>.min.json`
  - minimized legacy fixture
- `<slug>.case.witness.json`
  - extracted `CombatCase { basis = live_window }`
- `<slug>.case.min.witness.json`
  - minimized `CombatCase { basis = live_window }`
- `<slug>.case.json`
  - reduced case
- `<slug>.case.min.json`
  - reduced minimized case
- `<slug>.notes.md`
  - human checklist for the fix

Treat the `.case*.json` outputs as canonical. The fixture files are still emitted
only because the current extractor/minimizer bridge has not been removed yet.

## Required Discipline

For every parity bug:

- Pick exactly one primary field first.
- Use the minimized reduced case as the active reproduction target.
- Read the Java source that controls future state.
- Add or update a targeted regression before broad refactors.
- Only after that, touch Rust implementation code.

## Typical Commands

### 1. Generate witness and reduced case artifacts

```powershell
python tools\analysis\bugfix_workflow.py from-live `
  --field player.hp `
  --from-response-id 158 `
  --to-response-id 160
```

### 2. Re-run the minimized reduced case directly

```powershell
$env:COMBAT_CASE="D:\rust\sts_simulator\tests\live_regressions\player_hp_r158_160.case.min.json"
cargo test --test combat_case_driver replay_single_combat_case_from_env -- --nocapture
```

Or:

```powershell
cargo run --bin combat_case -- verify `
  --case D:\rust\sts_simulator\tests\live_regressions\player_hp_r158_160.case.min.json
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
- `player.hp`
- `monster[0].power[Metallicize]`

Avoid trying to fix an entire combat summary in one pass.

## When To Export More Java Truth

Default to Java-first parity for any state that:

- affects future move selection
- affects queued or delayed resolution
- is hidden/private in Java

Do not rely on surrogates like `move_history` when Java uses a dedicated internal
state machine.

If protocol truth is missing, add it. Do not patch around it with previous-state
carry in the live path.
