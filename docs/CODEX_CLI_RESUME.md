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
3. docs/MECHANICS_ACCEPTANCE_STANDARD.md
4. docs/NEXT_AI_HANDOFF.md

Then summarize:
- current uncommitted changes
- latest five commits
- current immediate packet
- exact Java files and Rust files to inspect next
- focused tests expected after changes

Rules:
- Java source truth is in D:\rust\cardcrawl.
- CommunicationMod truth is in D:\rust\CommunicationMod.
- Do not add policy heuristics, teacher labels, PPO/Gym work, or bot strategy
  compatibility work unless explicitly asked.
- Continue Java-source-backed mechanics parity cleanup.
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
Get-Content -Raw docs\MECHANICS_ACCEPTANCE_STANDARD.md
Get-Content -Raw docs\NEXT_AI_HANDOFF.md
```

Do not read broad directories until the handoff identifies a narrow packet.

At the end of each meaningful chunk, update `docs/NEXT_AI_HANDOFF.md` with:

- latest commit hash or `uncommitted`
- files changed
- tests run and result
- exact next source packet
- unresolved suspicion, if any

Commit code and handoff updates when the chunk is coherent. If a session closes,
the next CLI session should need only this file plus `docs/NEXT_AI_HANDOFF.md`.

## Current Project Direction

The project is a Rust, headless Slay the Spire simulator and parity/evaluation
tooling stack. The active work is simulator mechanics parity, not training a
strong AI policy.

Current authoritative entrypoints:

- `README.md`
- `docs/AI_DIRECTION.md`
- `docs/REPOSITORY_MAP.md`
- `docs/MECHANICS_ACCEPTANCE_STANDARD.md`
- `docs/MECHANICS_AUDIT_LEDGER.md`
- `docs/NEXT_AI_HANDOFF.md`

Only use additional docs or source files after the handoff narrows the packet.
