# Play Guide

`play` is a local debug harness for the simulator. It is useful for:

- quick manual or autoplay smoke tests
- inspecting local run flow without Java
- hosting the Rust side of `live_comm`

It is not a stable public interface, and it is not the authoritative description of
the Java protocol. For Java-connected work, prefer `tools/live_comm/` and the
`docs/live_comm/` runbooks.

## Basic Usage

`play` accepts positional `seed` and `ascension`, then free-form flags:

```powershell
cargo run --release --bin play
cargo run --release --bin play -- 42 15
cargo run --release --bin play -- 42 0 --class silent --auto --summary
```

Important current flags:

- `--class <ironclad|silent|defect|watcher>`
- `--auto`
- `--summary`
- `--silent`
- `--fast-act1`
- `--panic-watch`
- `--boss-only`
- `--live-comm`

`play` does not currently expose a proper `--help` surface. If you change flags,
verify them against [`src/bin/play/main.rs`](../src/bin/play/main.rs).

## Local Manual Commands

Current manual parser lives in [`src/cli/input.rs`](../src/cli/input.rs).

Combat:

- `play <idx> [target]`
- `end`
- `potion <slot> [target]`

Map / event / reward / campfire / shop:

- `go <x>` or `<number>` on map/event screens
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
- `relics`
- `potions`
- `draw`
- `discard`
- `exhaust`
- `a` / `step`
- `auto`
- `auto run`
- `manual`
- `skip`
- `fast`
- `help`
- `quit`

## Boundaries

Keep these distinctions in mind:

- the manual `play` parser is narrower than the bot's internal noncombat logic
- campfire input here is still only `rest` and `smith`
- `live_comm` noncombat handling supports more screens and audit behavior than the local parser
- checked-in `live_comm` profiles are the preferred way to launch Java-connected runs; do not hand-maintain large flag bundles in shell history

## When To Use Something Else

- use [live_comm/README.md](live_comm/README.md) for parity debugging or Java-connected runs
- use [../tools/learning/README.md](../tools/learning/README.md) for combat environment and learning experiments
- use [BUGFIX_WORKFLOW.md](BUGFIX_WORKFLOW.md) when you are fixing a parity bug from archived run logs rather than doing ad hoc local play
