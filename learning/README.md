# Online learning caller

`learning/` owns Python-side policy inference, optimizer state, seed
scheduling, curriculum, and evaluation accounting. It consumes the installed
`sts-learning-bridge` wheel; simulator mechanics, typed legality, checkpoint
contents, and NumPy semantic-schema production remain in Rust.

The first maintained component is `sts_learning.recovery`. It records the
current episode seed and generation for each environment slot. Missing initial
seed identity is not accepted. It keeps no trajectory or checkpoint history
and never decides automatically that a defeat should be retried. A training
caller explicitly performs:

```text
record terminal defeat
  -> prepare a budget-checked recovery ticket
  -> restore one opaque checkpoint batch through the bridge
  -> commit the ticket
```

If bridge restoration fails, the ledger remains at the pending defeat. The
held-out constructor fixes the recovery budget at zero, so evaluation cannot
silently become a training-style resurrection run.
Starting new episodes uses the same two-phase rule around the bridge's atomic
`reset_slots`: a failed reset leaves every completed ledger generation intact.
It also leaves the old episode seed intact; the validated new seed is committed
only after the environment reset succeeds.

`sts_learning.seeds` assigns a seed to training or held-out evaluation with a
stable seed-only hash before any recovery attempt or derived trajectory exists.
Its schedule is immutable: planning a vector reset returns a separate advanced
schedule, and `reset_scheduled_with_accounting` returns that schedule only after
the environment reset and ledger commit succeed. Failed resets therefore
consume neither an episode generation nor a seed. A held-out ledger accepts
only held-out seeds and retains its structural zero-recovery budget.

`sts_learning.outcomes.TerminalStepBatch.from_bridge_step` copies only the
seven compact terminal integer columns from one vector step. It validates row
alignment, slot bounds, uniqueness, and public outcome ranges without retaining
the bridge dictionary or any observation tensor. `RecoveryLedger` accepts only
this typed batch. It holds at most one pending defeat outcome per slot, clears
that row only after successful recovery, and attaches the final exact terminal
facts to `EpisodeOutcome` when the episode completes. It is not a trajectory
buffer or a second mutation journal.
Recovery events and completed outcomes carry the unchanged episode seed, so
retries cannot be mistaken for new independent runs.
`record_terminal` also returns every terminal attempt with its seed, generation,
attempt index, and recovery count; callers do not reconstruct intermediate
attempt lineage by joining mutable slot state after the fact.

`sts_learning.driver` is the bounded online population loop. One immutable
`SeedSchedule.plan(range(slot_count))` creates the bridge environments, recovery
ledger, next schedule cursor, and opaque episode-root checkpoint bank together,
so initial seed identity cannot drift between owners. A `BatchPolicy` receives
one ragged semantic decision batch and returns every active row's candidate
ordinal in one call, including symbolic-selection rounds. The driver then
performs one atomic environment step and copies only its compact terminal rows.

An explicit `BatchCurriculum` returns a `RecoveryPlan` for the whole terminal
batch. Selected defeats restore through one opaque checkpoint subset; all other
defeats complete, and completed slots reset together from the next immutable
seed plan. The atomic reset returns each replacement episode-root checkpoint
before the ledger and schedule commit, then updates the same opaque bank without
exposing or serializing simulator sessions. `run(batch_steps=N)` keeps only
aggregate counts and timing; `advance()` returns at most one bounded step's
attempts, completions, and recovery events. Neither API stores trajectories,
writes JSON, defines a game policy, or turns terminal HP and gold into reward.

The bridge verification command installs a fresh wheel and runs both bridge
smoke tests and these caller contracts:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```
