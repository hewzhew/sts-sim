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
ordinal in one typed call, including symbolic-selection rounds, together with
the caller-owned SHA-256 identity of the exact behavior-policy manifest used
for that inference and one typed selection probability per row. The probability
is either known at selection time or explicitly unknown; deterministic policies
record `1.0`. It is never reconstructed later from logits, display strings, or
a different checkpoint. A naked ordinal list or malformed probability column
is rejected before mutation. The driver then performs one atomic environment
step and copies only its compact terminal rows.

An explicit `BatchCurriculum` returns a `RecoveryPlan` for the whole terminal
batch. Selected defeats restore through one opaque checkpoint subset; all other
defeats complete, and completed slots reset together from the next immutable
seed plan. The atomic reset returns each replacement episode-root checkpoint
before the ledger and schedule commit, then updates the same opaque bank without
exposing simulator sessions to Python. A separate process-resume owner may
persist the bridge's versioned, bounded opaque bank bytes and must supply the
exact ordered slot identities when restoring them. `run(batch_steps=N)` keeps only
aggregate counts and timing; `advance()` returns at most one bounded step's
attempts, completions, and recovery events.
`run_until_terminal_attempts(terminal_attempts=T, max_batch_steps=N)` removes
caller-written advance loops while preserving the same boundary: it stops only
after a complete vector transition, reports whether the target or step limit
ended the prefix, and never flushes experience or triggers training and policy
promotion. Both run summaries aggregate typed terminal victories and defeats;
their sum equals terminal attempts, but they are not a shaped score. Neither
run API stores trajectories, writes JSON, defines a game
policy, or turns terminal HP and gold into reward.

`sts_learning.evaluation.evaluate_held_out_behavior` creates a fresh population
from one explicit `HELD_OUT` schedule and a typed slot/terminal/step bound. It
hard-codes zero recovery and attaches no experience buffer or trainer. The
policy must expose one typed behavior manifest id, and every returned choice is
checked against it before execution; the result is bound to that id. It also
contains only the start/end schedule and terminal-target aggregates, so the
same schedule plus the same policy RNG state repeats the same prefix. A
step-limited result stays visibly incomplete, and its small victory/defeat
sample is not automatically interpreted as generalized policy quality.
`evaluate_paired_held_out_behaviors` accepts one typed pair contract containing
that shared schedule and evaluation bound. It rejects reused policy objects or
manifest identities before creating an environment, then calls the same
single-policy evaluator for each distinct frozen behavior. Its typed result
retains both manifest-owned evaluations and a fixed `right - left` integer
delta for terminal attempts, victories, defeats, and batch steps. The delta is
arithmetic only: it creates no ranking, improvement claim, or teacher label.
The result is comparable only when both sides complete the same terminal
target; budget exhaustion on either side remains explicit incomplete evidence.
Equivalent policy RNG initial states remain a caller-owned input because the
generic evaluator does not inspect opaque policy state.

`sts_learning.experience` provides the optional bounded training handoff. Each
decision batch is copied before policy inference into a recursively frozen,
read-only view of the bridge-owned semantic schema; it does not define another
feature dictionary. Every row carries its exact slot, seed, episode generation,
attempt index, and recovery count alongside the selected candidate ordinal and
typed selection probability. Each batch also retains the exact behavior
manifest identity returned by that model call, not policy scores or a
reconstructed later version. Row selection, segment rotation, and attempt
assembly preserve known and unknown probabilities without reinterpretation.
`sts_learning.manifests` gives that identity an exact bounded owner. A behavior
manifest references externally stored model checkpoints, model definitions,
model configurations, behavior-rule implementations and configurations,
semantic schemas, optimizer configurations, and trainer implementations only
through typed SHA-256 content ids, together with its schema version and training
step. Equal weights under greedy and stochastic selection cannot share one
manifest identity. The fixed-capacity registry stores only those
small bindings: it does not copy a model, optimizer, checkpoint payload, file
path, or display label into experience. Unknown ids, conflicting claimed ids,
registry overflow, and exact-binding mismatches are rejected rather than
guessed or silently evicted.

`sts_learning.manifest_catalog` durably stores the manifest's canonical,
versioned binary payload under that same SHA-256 identity. Count,
per-manifest-byte, and total-byte limits are mandatory, and the catalog never
loads JSON or evicts implicitly. It uses the same atomic content-store kernel as
model checkpoints, rejects partial/foreign/corrupt files on reopen, and can
hydrate a fresh in-memory registry as one all-or-nothing batch.

`sts_learning.torch_checkpoints` is the optional persistence owner for model
weights. It writes a versioned tensor-only format with sorted state keys,
explicit dtype/shape, canonical bytes, and a SHA-256-derived filename; it never
loads pickle. Its caller-supplied limits cap checkpoint count, each payload, and
total retained bytes, with no implicit eviction. Publication uses a flushed
same-directory temporary plus an atomic hard link. Reopening verifies every
owned filename, size, digest, and tensor stream and rejects leftover partial or
foreign files. Restore first builds a fresh model and validates all keys,
dtypes, and shapes, so an incumbent scorer is not partially overwritten. A
`BehaviorManifestTemplate` then binds the checkpoint identity to the fixed
model/config/behavior-rule/schema/optimizer/trainer identities and exact
training step.

`sts_learning.torch_resume` covers the two mutable PyTorch owners that a model
checkpoint does not: the optimizer and the injected categorical generator. It
uses a separate canonical, caller-byte-bounded tensor/scalar tree with no
pickle or executable values. Optimizer restore validates parameter-group and
parameter-id topology before hydrating a disposable fresh optimizer, then
requires the hydrated state to reproduce the exact bytes. Generator restore
validates its device and uint8 state tensor, returns a fresh generator, and
likewise requires canonical byte equality.

`sts_learning.torch_behavior` makes publication and promotion separate typed
operations. Publication prepares the checkpoint and manifest, previews all
three owners, then commits checkpoint, durable catalog row, and in-memory
registry in that order before returning an executable candidate. Same-process
promotion refuses a bare checkpoint, a missing catalog row, or an unregistered
manifest. Restart recovery needs only the manifest id and newly opened owners;
it verifies/materializes a fresh scorer before atomically hydrating the fresh
registry. Both paths validate schema version, enter evaluation mode, and freeze
gradients. A concrete policy adapter also verifies that the manifest carries
its exact behavior-rule binding before it can execute. They never hand the
optimizer's live shadow scorer directly to behavior, so subsequent shadow
updates cannot drift a published manifest. The
live policy emits the recovered registered manifest id on every batched choice.
The publisher's exact preview permits a fully existing identity at capacity,
while its novel preview requires count and byte room for one additional
same-shape checkpoint, manifest, and registry row even when the current digest
already exists. This lets bounded training fail before mutation when its next
updated model cannot be published without blocking idempotent promotion retry.
Both return non-authoritative typed summaries; only `publish()` can return the
authority accepted by promotion.
`CategoricalTorchBehaviorController` is the stable policy object retained by a
long-running driver. It accepts only increasing training steps and swaps its
internal frozen categorical policy only after publication and promotion both
succeed; a failed promotion leaves the prior live policy and injected selection
RNG unchanged. A fresh inactive controller can recover the active generation
from its durable manifest identity after restart.

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

`sts_learning.semantic_batch` provides the NumPy-only row algebra needed to
retain or train on exact attempt subsets without pinning unrelated environment
slots. A selection validates the complete structural bridge schema, copies rows
in the requested order, compacts token storage, and reindexes categorical,
scalar, relation, and candidate token columns. It interprets no numeric feature
id. Unknown bridge fields and cross-row graph edges fail closed instead of
being silently discarded. Scoring a selected row is contract-tested to equal
scoring that same row in its original batch.

`sts_learning.semantic_concat` combines independently retained rows into one
training batch without defining feature meanings. It reuses strict row
validation, requires matching schema versions and NumPy dtypes, and reindexes
all token, candidate, fact, and relation tables. Temporal repeats of one slot
remain distinct rows, while cross-row edges, dense-mask disagreement, overflow,
and malformed schemas fail closed. Both maximum rows and maximum input-array
bytes are mandatory; the limits also expose a conservative bound for validation
copies, output, and transient arrays.

`sts_learning.attempts.BoundedAttemptAssembler` consumes those ordered sealed
segments as a synchronous sink. It owns independently sliced, read-only rows
for each exact attempt lineage across segment boundaries and delivers one
complete attempt only after its matching terminal record arrives. Open-attempt,
per-attempt decision, and per-attempt payload-byte limits form a hard retained
memory bound. An over-limit attempt releases all retained arrays immediately,
remains a compact dropped marker until terminal, and is reported as dropped
rather than relabeled as complete. One delivery contains every terminal from a
segment; sink failure commits neither assembler state nor segment sequence.
The batches inside one completed attempt may intentionally carry different
behavior manifest ids when online updates occurred while that attempt remained
active.

`sts_learning.torch_policy` is an optional, device-agnostic PyTorch baseline
over that same bridge-owned semantic graph. It is intentionally absent from
the package root, so ordinary caller imports still require only NumPy. The
scorer derives every embedding-table dimension and categorical offset from one
injected `semantic_schema()` result; it does not copy enum names, card ids, or
feature dictionaries into Python. Token kinds, categorical and scalar facts,
relations, and row-pooled context produce one flat candidate-logit tensor whose
boundaries are the unchanged bridge `candidate_row_splits`.

The optional module supplies a ragged cross-entropy loss and a greedy
`BatchPolicy` adapter. It also supplies a temperature-scaled ragged categorical
sampler that validates every row before consuming randomness and samples by
inverse CDF from an explicitly injected `torch.Generator`; it refuses the
global generator. The selected probability is returned from that same sampling
call. Temperature has a canonical behavior-rule configuration identity, so two
temperatures cannot share a manifest. Both CPU and CUDA execution are selected
by where the caller places the model and independent generator; the scorer
contains no hard-coded device. The first contract is deliberately
architecture-neutral: finite batched logits, exact row boundaries, finite
loss, backward gradients, and an optimizer update. It is not yet a claim that
this small graph network is the final policy model or that a particular
training objective is sufficient.

`CheckpointedCategoricalTorchPolicy` promotes or recovers only a publication
whose exact behavior-rule binding matches its typed categorical configuration.
A fixed injected generator reproduces the same local choices after model
recovery without touching global RNG. The manifest identifies the behavior
distribution; mutable generator state remains caller-owned and must be restored
separately if a future durable training runner resumes mid-stream.

`sts_learning.torch_outcomes` supplies the first honest terminal objective.
It consumes only `CompletedAttemptExperience`, resolves every behavior manifest
before scoring, and regresses only the actually selected candidate value to the
sparse terminal `-1/1` outcome. It never turns the behavior choice into a
teacher label or assigns that outcome to unselected candidates. Decision errors
are averaged inside each attempt and then across attempts, so a long attempt has
the same total weight as a short one. The result retains the batch-aligned
manifest-id sequence and decision-aligned selection probabilities for every
attempt; censored and dropped attempt types are rejected structurally. The
current objective deliberately does not importance-weight by those
probabilities. A future off-policy objective must declare that new contract and
handle unknown propensity explicitly.
All validated decision payloads in one delivery are concatenated and scored in
one model call. Per-row weights preserve the exact attempt-equal loss, while
eliminating one small PyTorch forward per historical decision. The caller must
inject semantic concat row and input-array-byte limits; batching is never an
excuse for unbounded replay memory.

`sts_learning.torch_training.SynchronousValueTrainer` plugs directly into the
complete-attempt assembler as a synchronous shadow-model sink. One delivery
causes at most one optimizer step; the trainer retains no attempt queue and no
semantic arrays, only scalar totals and the latest bounded manifest-id and
selection-probability sequences. Its required concat limits bound the
one-forward replay batch.
Explicitly unknown selection probability remains valid evidence; an unknown
behavior manifest fails before mutation. A backward or optimizer exception
poisons the trainer instead of inviting a retry over possibly partial state.
Dropped-only deliveries never train. If the trained
scorer is later promoted into behavior, the caller must publish its new exact
checkpoint manifest first; the trainer does not silently rewrite behavior
identity.

`sts_learning.torch_generation.BoundedCategoricalGenerationRunner` is the
first deliberately finite composition of these owners. Construction fails
before environment mutation unless the driver, attempt assembler, synchronous
trainer, categorical controller, shared registry, shadow scorer, and optimizer
parameters form one exact chain. A generation is an explicit positive number
of optimizer steps beyond the active behavior manifest's training step. Each
call has a caller-supplied batch-step limit, flushes the experience segment only
after a terminal batch, and promotes only after the shadow trainer reaches that
absolute target. It novel-previews capacity before training toward an unfinished
target and exact-previews an already reached target, so a durable failed
promotion remains retryable even when every owner is full. An exhausted call
leaves the old frozen behavior live while
preserving partial shadow progress for the next bounded call. Its result is
aggregate-only and never retains step results or attempts. The runner does not
own persistence: restarting exact training goes through the
separate six-component resume store and typed restorer. Its resume admission
boundary is fail-closed: the environment must be between decisions with no
terminal half-state, the episode-root bank must cover every slot, the experience
buffer must be flushed, the assembler must have no open attempt, segment
sequence ids must agree, the trainer must be healthy, and an active behavior
manifest must not be ahead of the shadow optimizer.
At an admitted boundary, `sts_learning.torch_resume_metadata` encodes the seed
schedule, active ledger lineage, segment/assembler counters, trainer counters
and bounded last-evidence fields, controller identity, promotion count, and
generation target as one canonical scalar component. Fresh ledger, empty
buffer, empty assembler, trainer, and controller owners have explicit restore
constructors; terminal half-states, open attempts, poisoned trainers, and
inconsistent sequence or parameter lineage remain unrepresentable. This
metadata is the scalar member of the final durable resume manifest, not an
independent resume authority.

`sts_learning.resume_store` is the bounded durable owner for the six immutable
resume components: current environment, episode-root bank, shadow model,
optimizer, categorical generator, and generation metadata. It batch-previews
all distinct component count/bytes before writing anything, commits components
through the shared flushed atomic content store, and publishes one small
canonical manifest last. Reopen verifies filenames, digests, envelopes, kinds,
sizes, and the complete six-way binding. The optional
`CategoricalGenerationResumePublisher` captures all six from one admitted live
runner boundary, so callers do not assemble manifests by hand.
`CategoricalGenerationResumeRestorer` resolves all six, creates fresh bridge
and PyTorch owners through typed factories, recovers the frozen active behavior,
and reconstructs the ledger-to-runner chain. It returns nothing until the
fresh runner reproduces the saved strict boundary exactly. Runtime policy,
optimizer, bridge decoder, curriculum, and memory-limit configuration remain
explicit caller inputs after restart rather than executable data hidden in a
checkpoint.

`sts_learning.torch_provenance` supplies the maintained categorical baseline's
real manifest identities instead of test-fixture digests. It canonically hashes
the complete bridge schema without interpreting feature names and binds typed
scorer, categorical-rule, Adam, device, implementation-version, and PyTorch
runtime facts. Unsupported or oversized schema trees fail closed.

`sts_learning.torch_session` and `torch_session_config` collapse the repeated
owner wiring into one bounded experiment-root factory. `new(...)` creates
generation zero only in an unused root; `publish()` emits an exact initial
resume point; `restore(id)` rebuilds fresh owners; and
`advance_generation(max_batch_steps=...)` publishes automatically only after a
real optimizer-step promotion. An unfinished bounded call returns no fake
durable resume point. Restore verifies the saved slot count, training seed
partition, and recovery budget against the supplied session configuration. The
first maintained profile is deliberately CPU-only. `recover_behavior(...)`
materializes a frozen manifest with an explicit fresh RNG seed for the existing
paired held-out evaluator; evaluation never reuses the mutable shadow model.

The bridge verification command installs a fresh wheel and runs both bridge
smoke tests and these caller contracts:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```

When that isolated verification environment exposes PyTorch, the same caller
test discovery also enables the scorer's synthetic optimizer contract and its
real-bridge semantic-batch integration test.
