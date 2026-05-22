# Play Guide

`run_play_driver` is a local debug harness for the simulator. It is useful for:

- quick manual or autoplay smoke tests
- inspecting local run flow without Java

It is not a stable public interface, and it is not the authoritative description of
the Java protocol. Java-connected work is retired until a new adapter is built.

## Basic Usage

`run_play_driver` accepts explicit flags and then a small command shell:

```powershell
cargo run --release --bin run_play_driver -- --seed 42 --ascension 0 --class silent
cargo run --release --bin run_play_driver -- --seed 42 --skip-neow
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
`eval::run_play`.

## Local Manual Commands

Current manual parser lives in [`src/eval/run_play.rs`](../src/eval/run_play.rs).

Combat:

- `play <idx> [target]`
- `end`
- `potion <slot> [target]`
- `actions`
- `action <idx>`
- `capture <path> [label]`

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
- `actions`
- `help`
- `quit`

## Boundaries

Keep these distinctions in mind:

- the manual `play` parser is narrower than the bot's internal noncombat logic
- `capture` only writes active stable combat decision boundaries; post-combat
  states and current `EventCombat` wrapper states are rejected
- campfire input here is still only `rest` and `smith`
- old `live_comm` noncombat handling and profile launch paths are not active
  architecture

## When To Use Something Else

- use [live_comm/README.md](live_comm/README.md) only for legacy boundary notes
- use `cargo run --bin combat_search_v2_driver -- --start-spec <spec.json>` for
  synthetic combat starts
- use `cargo run --bin combat_search_v2_driver -- --combat-snapshot <capture.json>`
  for exact stable combat positions captured from simulator state
