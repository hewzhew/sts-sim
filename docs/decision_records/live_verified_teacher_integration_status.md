# Live Verified Teacher Integration Status

## Context

The strongest current policy experiment is the verified advantage override
teacher implemented in:

- `src/bin/full_run_env_driver/main.rs`
- `src/bin/full_run_env_driver/candidate_evaluation_impl.rs`
- `src/bin/full_run_env_driver/verified_override_impl.rs`

That teacher runs inside the offline `FullRunEnv`. It can clone the full run
environment, force candidate actions, roll out continuations, and confirm
horizon artifacts at combat-end boundaries.

The live CommunicationMod path is different:

- `src/bin/play/main.rs`
- `src/cli/live_comm/*`
- combat decisions in `src/cli/live_comm/combat.rs`

Live combat currently chooses through the normal Rust combat chooser:

```text
diagnose_root_search_with_runtime_and_root_inputs(...)
```

It does not maintain a synchronized `FullRunEnv` mirror.

There is now also a live snapshot shadow path in:

- `src/bot/combat/snapshot_teacher.rs`

This path imports the current Java truth snapshot into Rust `CombatState`, uses
the protocol-exported root action space, clones only that current snapshot, and
rolls forward inside Rust combat for a bounded number of decisions. It does not
maintain cross-frame carry state or a full-run mirror.

## Decision

Do not silently treat the offline verified teacher as a live bot.

`play.exe` now accepts explicit live teacher flags:

```text
--live-comm-verified-teacher-mode off|shadow|takeover
--live-comm-verified-teacher-shadow
--live-comm-verified-teacher-takeover
```

Current behavior:

- `off`: existing live behavior.
- `shadow`: runs the snapshot teacher shadow and writes diagnostic records; live
  actions still come from the normal chooser.
- `takeover`: fails fast with a clear error instead of pretending the teacher
  controls live play.

## Reason

The offline teacher depends on a cloneable `FullRunEnv` with hidden run state and
deterministic future rollout. A live CommunicationMod frame provides Java
snapshot truth and protocol action space. To avoid recreating a carry/mirror
semantic layer, the live integration should not guess missing history.

The current shadow path therefore uses only the current imported snapshot. Its
first contract is deliberately narrow:

- no live takeover
- no full-run mirror
- no hidden history reconstruction
- dominance-only diagnostic evidence against the normal live chooser
- all candidate roots come from CommunicationMod's protocol action space

Takeover remains disabled until shadow logs show the evidence is useful and the
chosen `ClientInput` maps back to the expected protocol command.

## Next Work

Recommended next implementation order:

1. Run `--live-comm-verified-teacher-shadow` on real combat frames and inspect
   `snapshot_teacher_shadow` records.
2. Check whether dominance suggestions are frequent enough to matter and whether
   they look sane in boss/elite fights.
3. Add command-mapping assertions for any suggested override candidate.
4. Only after shadow recommendations are auditable, consider takeover mode.
