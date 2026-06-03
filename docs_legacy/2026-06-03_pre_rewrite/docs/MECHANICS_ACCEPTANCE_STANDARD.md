# Mechanics Acceptance Standard

This is the stopping rule for Java-source-backed simulator repair.

The goal is not to prove the whole game perfect. The goal is to prevent
unbounded re-auditing and to define when a mechanism is trustworthy enough for
AI infrastructure work.

## Core Rule

A mechanism can be treated as closed only when its gameplay semantics are
traceable to Java source, represented in Rust, protected by tests, and recorded
in the durable ledgers.

If a mechanism is `locked`, do not re-audit it by default.

Reopen a locked mechanism only when at least one of these is true:

- A related test fails.
- A new Java source owner or call path is found.
- A later code change touches the Rust owner or a shared helper it depends on.
- Live CommunicationMod truth contradicts the documented source-backed model.
- The ledger explicitly records a remaining risk that is now being addressed.

When reopening, write the reopen reason in the commit, test name, audit note, or
handoff. Do not silently recheck locked work.

## Status Meanings

```text
locked:
  Java source checked, Rust behavior protected by focused tests, evidence
  recorded, and no currently known AI-blocking risk remains for that mechanism.

partial:
  Some behavior is locked, but named branches, ordering, RNG, visibility, or
  execution-time states remain unreviewed.

source_checked:
  Java source was read and summarized, but Rust behavior is not yet protected by
  sufficient tests or implementation changes.

suspect:
  Likely mismatch or fragile abstraction. Do not build AI/training assumptions
  on this path.

unreviewed:
  No current source-backed claim.

unsupported_recorded:
  Java behavior is intentionally out of scope for the current simulator, with
  source path and non-trainable reason recorded.
```

## Per-Mechanism Lock Checklist

A mechanism cannot be marked `locked` unless all required items below are true:

```text
1. Java owner:
   Exact Java files were opened, not guessed from memory.

2. Rust owner:
   Exact Rust modules/functions are known.

3. Semantic summary:
   The behavior is described in gameplay terms, excluding UI/VFX-only detail.

4. Boundary decision:
   If Java UI/VFX hosts gameplay mutation, RNG, choice state, room state, or
   visibility, the mechanic is preserved in Rust without simulating UI itself.

5. Positive acceptance test:
   At least one test proves the expected source-backed behavior.

6. Negative or edge acceptance:
   At least one test guards the obvious wrong implementation, unless the
   mechanism is too trivial and the ledger says why.

7. Execution layer check:
   If an input/action-mask path exists, the low-level executor also rejects or
   preserves illegal/stale input. Masks alone are not enough.

8. Evidence recorded:
   Update `docs/JAVA_SOURCE_MAP.md`, `docs/MECHANICS_AUDIT_LEDGER.md`, and
   `docs/NEXT_AI_HANDOFF.md` when ownership, status, or next lane changes.

9. Verification recorded:
   Record the focused test command. For code changes, run and record
   `cargo test --all-targets` unless a blocker is explicitly documented.

10. Remaining risk named:
   If any branch remains unreviewed, the row is `partial`, not `locked`.
```

## AI Readiness Gate

Do not return to serious full-run AI training while any of these core paths are
`suspect`:

```text
combat action legality and execution
card definition / upgrade / play semantics
card zone ordering and generated-card insertion
monster planned moves, visible intent, and private runtime state
combat reward generation and reward claiming
boss transition and boss relic selection
relic obtain/equip/combat hooks
potion use/discard/obtain affordances
event gates, event effects, and event-started combats
shop generation, purchase, purge, and restock
campfire options and effects
map visibility, boss visibility, and key visibility
run-level RNG stream ownership
replay/state identity needed by evaluation
```

`partial` is acceptable only when the remaining risk is named and outside the
first AI training slice. For example, a custom-mod branch can remain out of scope
if it is marked `unsupported_recorded`.

## Anti-Loop Workflow

Every new audit packet should follow this order:

```text
1. Read:
   git status --short
   git log --oneline -5
   docs/MECHANICS_ACCEPTANCE_STANDARD.md
   docs/NEXT_AI_HANDOFF.md

2. Choose exactly one narrow packet from the ledger.

3. Check whether it is already locked.
   If locked, stop unless a reopen reason exists.

4. Open exact Java source files and exact Rust owners.

5. Add or adjust the smallest source-backed test that would fail under the
   obvious wrong implementation.

6. Change Rust only when the source comparison proves a mismatch or a missing
   guard.

7. Run focused tests, then full tests for code changes.

8. Commit code and docs separately when useful.

9. Push meaningful chunks so work is recoverable after context loss.
```

## When Bugs Become Harder To Find

As the simulator improves, the absence of obvious bugs is not a signal to
re-scan locked areas. It is a signal to move from broad cleanup to explicit
acceptance gates:

```text
mechanics parity tests
deterministic replay/state hash checks
heldout seed smoke runs
action-mask versus executor consistency tests
live CommunicationMod diff packets
AI-facing observation leakage checks
```

The project should return to machine-learning work only through a named slice,
not through a vague "the simulator feels cleaner" judgment.

## Reopen Record Format

Use this format in an audit note, handoff section, or commit message:

```text
Reopened: <mechanism>
Reason: <test failure | Java source owner found | Rust owner changed | live diff | ledger risk>
Java checked: <files>
Rust checked: <files/functions>
Result: <fixed | confirmed still locked | moved to partial/suspect>
Verification: <commands>
```

