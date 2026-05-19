# Codex CLI Resume Pack

Use this file when a stateless or relay-style Codex CLI session needs to pick
up this repository without spending a large context budget rediscovering the
project.

## Paste This First

```text
You are working in D:\rust\sts_simulator.

Do not broadly rescan the source tree. Resume from the durable handoff protocol.

Read exactly these first:
1. git status --short
2. git log --oneline -5
3. docs/NEXT_AI_HANDOFF.md

Then summarize:
- current uncommitted changes
- latest five commits
- active lane
- current immediate packet
- files to inspect next
- expected verification commands

Rules:
- Java source truth is in D:\rust\cardcrawl.
- CommunicationMod truth is in D:\rust\CommunicationMod.
- Do not add policy heuristics, teacher labels, PPO/Gym work, or bot strategy
  compatibility work unless explicitly asked.
- If working on LLM integration, read docs/LLM_INTEGRATION_HANDOFF.md.
- If working on Java-source-backed mechanics parity cleanup, read
  docs/MECHANICS_ACCEPTANCE_STANDARD.md and
  docs/JAVA_MECHANICS_DEBUG_HANDOFF.md.
- If a mechanism is already locked, do not reopen it without a reopen reason
  from docs/MECHANICS_ACCEPTANCE_STANDARD.md.
- Preserve user/uncommitted changes. Never reset or revert them unless asked.
```

## Session Contract

Treat chat context as disposable. Treat repository files and commits as the
memory system.

At the start of each session:

```powershell
git status --short
git log --oneline -5
Get-Content -Raw docs\NEXT_AI_HANDOFF.md
```

Do not read broad directories until the short handoff identifies a lane.

At the end of each meaningful chunk, update `docs/NEXT_AI_HANDOFF.md` with:

- current active lane and latest commit hash or `uncommitted`
- exact next source packet or LLM harness task
- verification commands
- pointer to the lane-specific handoff for details

Commit code and handoff updates when the chunk is coherent. If a session closes,
the next CLI session should need only this file plus the short
`docs/NEXT_AI_HANDOFF.md`.

## Current Project Direction

The project is a Rust, headless Slay the Spire simulator and parity/evaluation
tooling stack. Current work has two lanes:

- LLM controller/demo integration over public observation and legal action
  candidates.
- Java-source-backed mechanics parity cleanup when a narrow bug/evidence packet
  is selected.

The repo still does not claim a strong learned policy.

Current authoritative entrypoints:

- `README.md`
- `docs/AI_DIRECTION.md`
- `docs/REPOSITORY_MAP.md`
- `docs/NEXT_AI_HANDOFF.md`
- `docs/LLM_INTEGRATION_HANDOFF.md`
- `docs/MECHANICS_ACCEPTANCE_STANDARD.md`
- `docs/MECHANICS_AUDIT_LEDGER.md`
- `docs/JAVA_MECHANICS_DEBUG_HANDOFF.md`

Only use additional docs or source files after the handoff narrows the packet.
