# Play Guide

`run_play_driver` is a thin shell over `eval::run_control`. It is useful for:

- manually driving a real simulator run from Neow onward
- saving exact combat starts for Combat Search V2
- saving whole-combat baseline outcomes after you finish a fight

It is not a stable public interface, and it is not the authoritative description of
the Java protocol. Java-connected work is retired until a new adapter is built.

## Basic Usage

`run_play_driver` accepts explicit flags and then a small command shell:

```powershell
cargo run --release --bin run_play_driver -- --seed 42 --ascension 0 --class silent
cargo run --release --bin run_play_driver -- --script commands.txt
```

Important current flags:

- `--class <ironclad|silent|defect|watcher>`
- `--seed <n>`
- `--ascension <n>`
- `--final-act`
- `--skip-neow`
- `--script <path>`

The binary owns no simulator semantics; it delegates to `engine::run_loop` and
`eval::run_control`. For strategy benchmarks, do not use `--skip-neow`; the
point is to let real Neow, route, reward, shop, event, and campfire decisions
produce the combat start state.

The default view is a compact decision panel. It shows the current decision,
candidate commands, and a small context block. Use `d` when you need the older
low-level engine details, `r` for raw Rust debug output, and `h` for the full
categorized command list.

## Local Manual Commands

Current manual parser lives under
[`src/eval/run_control`](../src/eval/run_control/mod.rs). `eval::run_play`
is only a compatibility re-export.

Combat:

- `play <idx> [target]`
- `end`
- `potion <slot> [target]`
- `actions`
- `action <idx>`
- `capture <path> [label]`
- `capture-case <benchmark_dir> <case_id> [label]`

Map / event / reward / campfire / shop:

- `go <x>`
- `event <idx>`
- `claim <idx>`
- `pick <idx>`
- `proceed`
- `relic <idx>`
- `rest`
- `smith <deck_idx>`
- `select <idx1> <idx2> ...`
- `choose <idx1> <idx2> ...`
- `buy card <idx>`
- `buy relic <idx>`
- `buy potion <idx>`
- `purge <deck_idx>`
- `cancel`

Inspection and mode control:

- `state`
- `d` / `details`
- `r` / `raw`
- `h` / `help`
- `actions`
- `quit`

Benchmark artifacts:

- `save-baseline <path> [case_id]`
- `save-baseline-case <benchmark_dir> <case_id>`
- `bench-add <benchmark_dir> <case_id>`

The case commands use this layout:

```text
<benchmark_dir>/
  benchmark.json
  captures/<case_id>.capture.json
  baselines/<case_id>.baseline.json
```

## Boundaries

Keep these distinctions in mind:

- the manual `play` parser is deliberately a human run-control surface, not a
  bot policy
- `capture` only writes active stable combat decision boundaries; post-combat
  states and transient combat-start requests are rejected
- combat input advances to the next stable boundary before returning control
- `save-baseline-case` uses the last completed whole-combat outcome; it does
  not record stepwise action agreement
- campfire input here is still only `rest` and `smith`
- old `live_comm` noncombat handling and profile launch paths are not active
  architecture

## When To Use Something Else

- use [live_comm/README.md](live_comm/README.md) only for legacy boundary notes
- use `cargo run --bin combat_search_v2_driver -- --start-spec <spec.json>` for
  synthetic combat starts
- use `cargo run --bin combat_search_v2_driver -- --combat-snapshot <capture.json>`
  for exact stable combat positions captured from simulator state
- use `cargo run --bin combat_search_v2_driver -- --benchmark-spec <benchmark.json>`
  for capture/baseline benchmark reports
