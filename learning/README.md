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

`sts_learning.experience` provides the optional bounded training handoff. Each
decision batch is copied before policy inference into a recursively frozen,
read-only view of the bridge-owned semantic schema; it does not define another
feature dictionary. Every row carries its exact slot, seed, episode generation,
attempt index, and recovery count alongside the selected candidate ordinal.
An `ExperienceSegmentBuffer` requires both a maximum decision count and a
maximum retained-payload byte count. The byte count conservatively includes
owned NumPy buffers and headers plus mappings, keys, and scalar values; the row
limit separately bounds lineage and ordinal metadata.

When the next complete decision batch would cross either bound, the current
segment seals before that batch is retained. Each attempt fragment in a sealed
segment contains either its exact `TerminalAttemptRecord` or an explicit
`censored` state; a limit boundary never fabricates defeat. `OnlineBatchDriver`
accepts the buffer only together with a synchronous `ExperienceSegmentSink` and
hands off sealed segments immediately instead of queueing them. Sink failure is
fail-stop before the current model choice mutates the environment. The choice
is committed to the new segment only after the bridge accepts it, so a rejected
environment action cannot become training experience. One open segment remains
bounded across `run()` calls and can be deliberately sealed by
`flush_experience()`.

`sts_learning.torch_policy` is an optional, device-agnostic PyTorch baseline
over that same bridge-owned semantic graph. It is intentionally absent from
the package root, so ordinary caller imports still require only NumPy. The
scorer derives every embedding-table dimension and categorical offset from one
injected `semantic_schema()` result; it does not copy enum names, card ids, or
feature dictionaries into Python. Token kinds, categorical and scalar facts,
relations, and row-pooled context produce one flat candidate-logit tensor whose
boundaries are the unchanged bridge `candidate_row_splits`.

The optional module supplies a ragged cross-entropy loss and a greedy
`BatchPolicy` adapter. Both CPU and CUDA execution are selected by where the
caller places the model; the scorer contains no hard-coded device. The first
contract is deliberately architecture-neutral: finite batched logits, exact
row boundaries, finite loss, backward gradients, and an optimizer update. It
is not yet a claim that this small graph network is the final policy model or
that a particular training objective is sufficient.

The bridge verification command installs a fresh wheel and runs both bridge
smoke tests and these caller contracts:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```

When that isolated verification environment exposes PyTorch, the same caller
test discovery also enables the scorer's synthetic optimizer contract and its
real-bridge semantic-batch integration test.
