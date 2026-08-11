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
their sum equals terminal attempts. They also stream one bounded terminal
progress aggregate: floor sum/range and sorted act counts, with no trajectory
retention. These are evidence, not a shaped score. Neither run API stores
trajectories, writes JSON, defines a game
policy, or turns terminal HP and gold into reward.

`sts_learning.evaluation.evaluate_held_out_behavior` creates a fresh population
from one explicit `HELD_OUT` schedule and a typed slot/terminal/step bound. It
hard-codes zero recovery and attaches no experience buffer or trainer. The
policy must expose one typed behavior manifest id, and every returned choice is
checked against it before execution; the result is bound to that id. It also
contains only the start/end schedule and terminal-target aggregates, including
a bounded exact terminal-floor histogram, so the
same schedule plus the same policy RNG state repeats the same prefix. A
step-limited result stays visibly incomplete, and its small victory/defeat
sample is not automatically interpreted as generalized policy quality.
`evaluate_paired_held_out_behaviors` accepts one typed pair contract containing
that shared schedule and evaluation bound. It rejects reused policy objects or
manifest identities before creating an environment, then calls the same
single-policy evaluator for each distinct frozen behavior. Its typed result
retains both manifest-owned evaluations and a fixed `right - left` integer
delta for terminal attempts, victories, defeats, terminal floor sum, and batch
steps. The delta is
arithmetic only: it creates no ranking, improvement claim, or teacher label.
The result is comparable only when both sides complete the same terminal
target; budget exhaustion on either side remains explicit incomplete evidence.
Equivalent policy RNG initial states remain a caller-owned input because the
generic evaluator does not inspect opaque policy state.

`sts_learning.paired_run_compare` is the maintained publication-level adapter
for that contract. Its default full-behavior scope runs two ordinary one-slot
evaluators from the same held-out seed schedule, initial policy RNG seed,
ascension, potion action surface, terminal target, and step bound. Before
comparing, it requires equal model definition/configuration, behavior-rule
implementation/configuration, and semantic schema identities, then aligns every
completed terminal seed.

Passing `-StrategicBehavior` selects the combat-anchor-only scope. Both sides
recover the same strategic publication and strategic RNG seed; the baseline
and candidate directories supply distinct verified combat anchors. Typed combat
rows use their anchor greedily, while all strategic rows remain sampled from the
shared source. Greedy combat selection consumes no policy RNG. This isolates
which scorer owns combat rows, but it does not force later game states or
strategic decision counts to remain equal after combat outcomes diverge.

Both scopes retain complete V10 `evaluation.json` files and write only raw
per-seed win, act/floor, HP, gold, combat-count, and potion differences. The V2
comparison contract records its scope and the relevant strategic and combat
manifest identities. The shared RNG claim is deliberately limited to the same
initial stream: after paths diverge, later strategic draws may occur at
different decisions.

The maintained command for applying a published combat-trained scorer to that
complete run evaluator is:

```powershell
.\learning\dev.ps1 evaluate-run `
  -Behavior <completed-combat-training-directory> `
  -Output <fresh-run-evaluation-directory> `
  -Ascension 20 -Attempts 8 -MaxBatchSteps 4096 `
  -BehaviorSeed 10000 -HeldOutSeedStart 0
```

Compare two publications on one exact complete-run prefix with:

```powershell
.\learning\dev.ps1 compare-run-paired `
  -BaselineBehavior <baseline-directory> `
  -CandidateBehavior <candidate-directory> `
  -Output <fresh-paired-run-directory> `
  -Ascension 20 -Attempts 8 -MaxBatchSteps 4096 `
  -BehaviorSeed 10000 -HeldOutSeedStart 0 -RunPotionLane never
```

To change only the scorer used for combat rows, keep one strategic publication
fixed and compare two combat anchors:

```powershell
.\learning\dev.ps1 compare-run-paired `
  -BaselineBehavior <baseline-combat-anchor-directory> `
  -CandidateBehavior <candidate-combat-anchor-directory> `
  -StrategicBehavior <shared-strategic-directory> `
  -Output <fresh-scoped-paired-run-directory> `
  -Ascension 20 -Attempts 8 -MaxBatchSteps 4096 `
  -BehaviorSeed 10000 -HeldOutSeedStart 0 -RunPotionLane never
```

It uses zero recovery, attaches no trainer, and writes only compact terminal
progress aggregates. Combat and run decisions share the bridge semantic schema,
but combat-only training does not imply that route, reward, shop, or event
choices are competent; their behavior remains part of this end-to-end result.

Warm-start bounded whole-run on-policy training with:

```powershell
.\learning\dev.ps1 train-run `
  -Behavior <completed-combat-training-directory> `
  -Output <fresh-run-training-directory> `
  -Slots 4 -Generations 1 -AttemptsPerUpdate 8 `
  -MaxBatchSteps 4096 -EvaluationAttempts 16 `
  -HeldOutSeedStart 1000000
```

Value-PPO defaults to its provenance-bound global advantage normalization.
`-RunAdvantageNormalization off` is the explicit single-variable ablation;
`auto` preserves the selected update profile. The journal records both the
configured choice and the observed pre/post-normalization signal distributions.

Calibrate the scalar critic without changing actor logits, then consume that
calibration in a fresh PPO cohort with:

```powershell
.\learning\dev.ps1 train-run `
  -Behavior <completed-combat-training-directory> `
  -Output <fresh-critic-calibration-directory> `
  -Slots 4 -Generations 1 -AttemptsPerUpdate 8 `
  -RunPolicyUpdate critic-calibration -CriticFitSteps 256 `
  -DecisionScope strategic -CombatDecisionRule greedy `
  -RunPotionLane never -Ascension 20

.\learning\dev.ps1 train-run `
  -Behavior <same-completed-combat-training-directory> `
  -CriticInitializationBehavior <fresh-critic-calibration-directory> `
  -Output <fresh-actor-critic-training-directory> `
  -RunPolicyUpdate ppo-clip-value `
  -DecisionScope strategic -CombatDecisionRule greedy `
  -RunPotionLane never -Ascension 20
```

The calibration optimizer owns the complete scorer but freezes every shared
encoder and actor tensor, reuses one fixed complete-attempt cohort for 256
supervised value-head steps by default, applies unit scalar value loss without
value or finite gradient clipping, and publishes its distinct trainer identity.
PPO accepts it only after verifying the source was critic-only, used the same
ascension, potion lane, decision scope and combat
anchor, and still matches every actor tensor from `-Behavior`. The PPO command
then starts a new seed/RNG cohort; the calibration attempts never become its
actor experience.

Before consuming a calibration in actor PPO, challenge its frozen critic on a
fresh complete-attempt cohort without publishing another behavior:

```powershell
.\learning\dev.ps1 probe-run-critic `
  -Behavior <completed-run-publication-with-scalar-critic> `
  -Output <fresh-probe-directory> -Ascension 20 `
  -ProbeTrainAttempts 24 -ProbeHeldOutAttempts 8 `
  -ProbeHeadFitSteps 256 -MaxBatchSteps 32768 `
  -BehaviorSeed 10000 -HeldOutSeedStart 1000000 `
  -RunPotionLane never
```

The probe holds actor tensors fixed, splits only by complete attempts, and
compares constant, direct public-run-feature ridge, published-critic, and
ephemeral head-only predictions. Its output is learnability evidence, not a
checkpoint or a capability claim.

The session copies, rather than aliases, the combat scorer; generation zero
then belongs to the whole-run terminal-floor objective and a new behavior
manifest. Training and evaluation use the stable disjoint seed partitions,
both with zero recovery. Every generation records its bounded terminal-floor
histogram, and only a complete requested generation set is published before the
final held-out evaluation. The publication is the frozen behavior only; the
command deliberately does not claim resumable optimizer/environment state when
other asynchronous slots still contain open attempts.

`sts_learning.strategic_demonstrations` is the bounded in-memory path for
bootstrapping a strategic scorer from the maintained production owner. It asks
the bridge for semantic candidates and production ordinals in the same frame,
retains only exactly labeled strategic-root rows, and records seed, act, floor,
and strategic context without writing a corpus artifact. Combat remains fixed
to one immutable scorer; symbolic selection and unlabeled strategic rows are
explicit fallback traffic and never become demonstrations. Every completed run
also retains
its seed-aligned terminal reward, act, floor, HP, and max HP as compact scalar
columns, so coverage and future continuation targets do not have to be inferred
from aggregate win counts or display logs. The default combat anchor requires
an exact current publication. Historical weights are admitted
only through the separately selected `COMPATIBLE_WEIGHT_IMPORT` mode, which
still verifies the complete journal, unique durable identities, checkpoint
hash, semantic schema, behavior/model configuration, and strict tensor
compatibility. It records any allowed historical model-definition, optimizer,
or trainer provenance digest difference instead of treating the source as a
current publication.

An optional typed `CombatRetryCoverageConfig` may retain one opaque checkpoint
per live combat root to widen demonstration coverage. The first attempt remains
greedy; only a failed retry uses the separately seeded categorical sampler, and
each combat has an explicit retry ceiling. Restoring a combat never replays or
duplicates a strategic teacher row. Normal victories/defeats, total retry work,
rescued combats, and per-terminal-run retry counts remain separate, so this
coverage curriculum cannot masquerade as a zero-recovery run result. The mode
is disabled by default and remains bounded by the collector's existing batch
step, row, byte, and wall-time limits.

`sts_learning.strategic_behavior_cloning` consumes that in-memory corpus
without inventing a second feature schema. It classifies each original run seed
through the stable `SeedPartitionSpec` before concatenation, rejects an empty or
overlapping train/held-out side, and trains only a deep copy of the frozen
combat anchor. The result reports fixed-epoch train and held-out cross-entropy,
overall agreement, and per-strategic-context agreement; it does not promote or
publish a policy. A later mixed policy must continue routing combat through the
unchanged anchor rather than assuming supervised strategic updates preserved
combat competence.

`sts_learning.experience` provides the optional bounded training handoff. Each
decision batch is copied before policy inference into a recursively frozen,
read-only view of the bridge-owned semantic schema; it does not define another
feature dictionary. Every row carries its exact slot, seed, episode generation,
attempt index, and recovery count alongside the selected candidate ordinal and
typed selection probability. Each batch also retains the exact behavior
manifest identity returned by that model call, not policy scores or a
reconstructed later version. Row selection, segment rotation, and attempt
assembly preserve known and unknown probabilities without reinterpretation.
The policy-neutral capture and choice-validation kernel lives separately in
`sts_learning.decision_rows`, so same-root combat learning reuses the identical
immutable payload and byte accounting rather than growing a second trajectory
format. `sts_learning.combat_experience.CombatGroupDriver` binds those rows to
the exact combat root and numbered replicate, rejects mixed behavior manifests,
and retains one complete group under independent decision, payload-byte,
model-round, and transition limits. A memory overflow is detected before the
corresponding bridge choice; a partial group is an error, not a fabricated
terminal sample. Completed groups expose win, player-terminal-HP-ratio,
enemy-HP-progress, and potion-retention leave-one-out axes separately and
define no exchange rate among them. Enemy-HP progress remains diagnostic
evidence by default; a separately provenanced opt-in objective may select it
only for groups whose terminals are all exact losses. Unresolved escapes never
enter that fallback.
`sts_learning.combat_rollout` reconstructs every replicate in retained
model-call order and attributes each observed public-state change to the
preceding decision. It keeps sparse terminal win, future player-HP change, and
future enemy-HP change as three independent undiscounted return-to-go columns.
Potion UUID/domain identities remain before/after facts and receive no scalar
price; because the terminal bridge has only potion ids, terminal UUIDs stay
explicitly unavailable. Combat value PPO consumes these columns through three
fixed-semantic value heads. The typed win-first selector chooses one matching
column for each exact-root group, and every row uses its own return-to-go minus
that row's matching pre-update value; residuals are never centered across
unrelated decision times. Policy-only combat REINFORCE/PPO deliberately retain
the terminal same-root comparison path. Scalar whole-run critics and the
multi-value combat critic have distinct model provenance and cannot be
silently recovered into one another.
Before running a group, callers may select or stratify roots through the
bridge-owned frozen root context; the caller must not decode semantic feature
numbers or copy the full combat observation into a parallel metadata schema.
The bridge discovers all currently eligible roots in one call, so corpus
selection does not create throwaway replicate groups or require per-slot probes.
The policy-facing action rows exclude a mechanically idle potion discard unless
the same decision also exposes an immediate refill through another usable
`EntropicBrew` or a playable `Alchemize`, and `Sozu` does not block the gain;
the simulator's complete legal action surface remains unchanged.
The maintained production handoff is one byte-bounded
`CombatLearningRootBatchArtifactV1`, not a Python continuation parser or a
guessed branch-capsule path. `cargo oracle-lab learning-root collect` advances
one to 64 explicit fresh seeds through current production non-combat owners and
emits their first combats as one batch; `learning-root export` converts public
continuations already at useful later combat boundaries. The caller reads those
opaque bytes and constructs a fresh bridge batch with the exact expected root
count; Rust revalidates every combat boundary and root identity first.
For bounded search-improvement feasibility work,
`LearningBatchEnv.from_combat_public_chance_particles(...)` derives an
in-memory population whose public observation and legal candidates equal one
selected source root while hidden draw order and combat RNG streams vary. The
current sampler keeps RNG consumption counters but samples streams
independently and rejects hidden current intents; it is not yet a
run-seed-consistent posterior or a certified teacher source.
`LearningBatchEnv.from_combat_entry_floor_chance_particles(...)` is the more
physical natural-entry feasibility surface. It keeps the source run's exact
upstream state and seven persistent RNG streams, varies the five floor-local
streams through the production combat-start constructor, and rejects every
candidate whose full public decision differs. Its accepted values are
floor-seed bases, not claims that a complete run with that seed would produce
the fixed deck, route, relics, and combat. It therefore remains conditioned
floor-chance evidence rather than a public-history run-seed posterior or a
teacher source. A bounded scan that cannot fill its requested population is
`unknown`; exact multi-monster public boundaries may be too sparse for this
rejection sampler. Persisting accepted floor seeds can avoid repeated work but
does not upgrade them into a complete-run posterior.
`learning-root select` derives one explicit ordered root subset from a
canonical batch after revalidating its declared source width; selected slots
stay opaque and duplicate or out-of-range slots publish nothing.
`learning-root merge` combines two or more canonical artifacts under the same
validation and fresh-output boundary. Single-root inputs remain the default;
an explicit expected root count for every input admits already canonical
multi-root batches for joint curricula. Rust revalidates every root, rejects a
declared-width mismatch or repeated exact identity, and caps the combined batch
at 64 roots. This lets rare selectors publish honest one-root shards and lets a
bounded rehearsal corpus coexist with a new competence frontier without an
ad-hoc binary concatenation or Python checkpoint decoding.
`sts_learning.collect_run_combat_roots` supplies the bounded corpus path when
useful later continuations do not yet exist. It advances one published frozen
behavior over one explicit seed partition and `SeedPartitionSpec`, aligns
compact public run facts with undecoded combat-root contexts, and captures at
most one typed floor-, prior-combat-count-, and usable-potion-qualified root per
seed. A maximum floor can close a narrow causal tier instead of letting a
minimum-floor selector silently admit much later routes. Rust merges the canonical single-root
payloads without exposing their sessions. The command publishes one fresh
opaque batch only after reaching its declared root target; a step/deadline,
identity, alignment, or byte failure leaves no output. Its receipt records the
collected seed/site/resource facts plus canonical card/upgrade counts and relic
identities. It also retains the exact typed strategic candidate set and chosen
ordinal for every preceding run decision on that seed, so route, event, and
card-reward behavior can be audited without decoding the opaque combat root.
These are behavior facts, not teacher labels, and assign no potion value. The
requested ascension is checked against every exact combat root before any
artifact is published; a reset or import that changed difficulty is a hard
collection failure.

The collector may instead receive a candidate combat anchor through `-Behavior`
and a fixed strategic source through `-StrategicBehavior`. This reproduces the
combat-anchor-only run surface during curriculum collection: combat rows use
the anchor greedily, while strategic rows keep the source categorical RNG.
Scoped collection requires an explicit whole-run potion lane (`all` or `never`)
and writes a V7 receipt binding the strategic, anchor, and combined manifests.
It does not select roots by their later outcome.
The independent `-CombatFightClass any|ordinary|elite|boss` axis uses typed
mechanical flags, not an encounter-name list. This permits a lifecycle tier to
learn across ordinary fights before elite and boss mechanisms are introduced as
separate corpora.
The minimum usable-potion filter accepts zero, allowing an ordinary
run-derived combat corpus that is not biased toward potion-bearing paths.
The model-facing strategic surface canonicalizes deterministic free reward
steps before inference: gold is claimed, one unique potion reward is claimed
into an empty slot when Sozu is absent, and one unique card reward is opened
before the policy sees the actual card-versus-skip choice. Multiple potion or
card-reward items remain policy choices because their order can matter. The
underlying planner surface retains the full legal actions; this is action-space
reduction, not a card or potion value label.
An explicit distinct-encounter mode admits at most one root for each canonical
encounter identity within a batch; repeated encounters remain the normal
default when distributional frequency is the intended evidence.
An optional exact encounter selector is normalized by the installed bridge
before advancing a run. Variable members such as a `GremlinGang` roll retain
one stable encounter identity while their exact monster identities remain in
the root receipt.
An explicit encounter-quota mode accepts several canonical encounter targets
with a fixed root count for each. It derives the total batch width from those
quotas, stops admitting an encounter once its target is full, and publishes
nothing unless every target is complete. Quota, exact-encounter, and
distinct-encounter modes are mutually exclusive. The receipt reports both the
requested and captured count per encounter, while the ordinary one-root-per-seed
identity rule remains unchanged.
It keeps the requested run potion lane separate from the resolved combat
potion lane, including when `trained` inherits `all` or `never`.
Its receipt composes the existing typed run-resource trace and includes prior
same-seed combat HP/gold/potion transitions for each captured root. It retains
ordered canonical enemy identities but no action rows or session history, and
excludes the captured combat itself.
An optional typed selector binds one bridge-validated canonical potion identity
to one exact inventory slot before capture. This is the corpus boundary for a
concrete `root-slots` rescue lane; it does not group different potion identities
under rarity or role heuristics.
Their row projection attaches all three columns to each retained replicate
decision without choosing a training axis or scalar weighting.
`sts_learning.torch_outcomes.on_policy_combat_win_loss` is the first narrow
training consumer. It batches complete distinct-root groups into a bounded
sequence of scorer calls, verifies their exact behavior manifests and sampled
propensities, and
selects one same-root advantage axis lexicographically. Any mixed win/loss
group uses only win advantage. The typed all-win axis is either `NONE` for a
strict win-only ablation or `TERMINAL_HP` so solved early combats continue
learning resource preservation. The independent all-loss axis defaults to
`NONE`; explicit `ENEMY_HP_PROGRESS` applies only when every terminal is an
exact loss and the axis varies. `UNRESOLVED` terminals remain no-signal, and
potion retention stays excluded rather than being silently exchanged for HP.
Every replicate contributes equal total weight regardless of how many
decisions it needed. Both selected fallback axes are part of exact trainer
provenance.
`sts_learning.torch_combat_training.SynchronousCombatWinTrainer` gives this
objective a separate provenance identity and consumes exactly the configured
number of complete groups. No selected-axis signal and exactly zero policy
gradient are typed no-update results; only a finite nonzero gradient performs
one optimizer step. Semantic concat row and byte limits apply to each
materialized microbatch rather than silently capping the total decisions in an
update. Microbatch losses retain the original group/replicate/decision weights,
backward gradients accumulate in trajectory order, and each PPO epoch still
performs at most one optimizer step. KL, clipping, entropy, and value-loss
diagnostics are reduced under those same global row weights. The trainer retains
separate win/HP signal counters and bounded identity evidence, not group payloads.
`sts_learning.torch_combat_generation.BoundedCombatWinGenerationRunner` is the
first bounded live composition. It fixes one exact source root, requires one
group per update, validates the scorer/optimizer/registry/controller chain, and
keeps behavior frozen for the complete group. Only a real optimizer step is
promoted. A temporary promotion failure retains one compact pending result and
retries it before requesting another group, while root drift fails before any
new policy or environment mutation. The runner does not yet own cross-root
scheduling, durable combat-training resume, or a scalar HP/potion objective.
Each generation result carries the completed group's compact four-axis signal
summary after its bounded experience has been consumed. Cross-root diagnostics
can therefore build the existing bounded census without retaining or reopening
semantic decision payloads.
`sts_learning.torch_combat_census.CombatWinSignalCensusRunner` owns that routine
diagnostic composition. It reads one bounded opaque batch or reuses one already
validated source, starts every root from identical random or verified warm-start
weights under the declared potion lane, requires one explicit behavior RNG seed per root,
and returns compact generation results plus the aggregate census. The artifact
is imported once and every slot selects from that shared typed source; slot-local
trainers and stores live only in temporary directories, their updates are
discarded, and nothing is published. The runner measures whether diverse roots
provide win, player-HP, enemy-HP-progress, or potion signal—it does not train
one behavior across roots.

Run bounded shared-model training over one opaque multi-root artifact through
the configured Python runtime:

```powershell
.\learning\dev.ps1 train-combat `
  -Artifact <combat-roots.bin> `
  -Behavior <optional-published-combat-behavior> `
  -Output <fresh-experiment-directory> `
  -Roots <artifact-root-count> `
  -Replicates 8 `
  -Updates <bounded-update-count> `
  -ModelSeed 0 `
  -BehaviorSeedBase 1000 `
  -CombatLearningRate 0.001 `
  -PotionLane never `
  -CombatAllLossAxis none
```

For a nonzero update count, the command first censuses every declared root under
the exact destination initialization, potion lane, replicate count, and behavior
seed. It reuses the already decoded source, then trains only mixed-win survival
frontier roots and all-win roots with real terminal-HP variation. Default
all-loss roots are journaled as rescue and solved roots are excluded from the
optimizer; neither disappears from root audits. Every update collects only that
selected frontier under one frozen behavior, applies at most one shared
optimizer step, and immediately promotes only a real update.
The command appends compact generation and per-root signal facts to
`training.jsonl`, then explicitly publishes the final behavior checkpoint.
The Adam learning rate is an explicit, provenance-bound training parameter;
`-CombatLearningRate` defaults to `0.001`. Lowering it changes the destination
optimizer identity and is the supported way to test whether one PPO epoch is
moving farther than its post-step KL diagnostic can safely distinguish.
Its configuration record also persists each exact root's seed, act/floor,
actual ascension, entry HP, potion identities, encounter/monster and elite/boss
facts, canonical card/upgrade counts, and relic identities. This audit is read
from the opaque root through the bridge; filenames are never trusted as
curriculum identity.
The output directory must be absent or empty; optimizer resume is not implied.
An initialization-only `-Updates 0` publication may use one exact root for a
narrow action-surface audit. A nonzero update remains multi-root and requires
at least two distinct source roots.
Publication makes the frozen scorer reproducible, but it does not make that
scorer the accepted or best behavior. The current live promotion inside one
training session only rotates the immutable on-policy collector after a real
optimizer step. A separate candidate-to-accepted behavior gate is the next
control-plane boundary; until its typed contracts exist, compare publications
explicitly and never infer an accepted latest model from directory names or
training-step order. See `Learning Control Plane` in
[`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md).
When `-Behavior` is supplied, the command first verifies that publication and
copies its frozen scorer parameters into the fresh destination shadow. The
destination still starts a new optimizer, step-zero manifest, root-set
objective, potion lane, and journal; source manifest/checkpoint ids are retained
as initialization provenance. Exact model configuration, semantic schema,
behavior rule, and checkpoint shapes remain mandatory. Compatible historical
model-definition, optimizer, or trainer-provenance digest differences are
written to `warm_start_provenance_mismatches` and force actor-only import, so an
old critic is never presented as a resumed current critic. Omitting `-Behavior`
keeps random initialization.
`-PotionLane never` removes potion use/discard candidates for every generated
group while preserving the simulator legality surface. Use it for roots with
observed no-potion wins so terminal-HP refinement cannot spend inventory. A
no-potion all-loss root remains no-signal by default. The explicit
`-CombatAllLossAxis enemy-hp-progress` mode can investigate bounded damage
support when every terminal is an exact loss; it excludes unresolved escapes
and does not claim victory or price potions. A concrete-potion rescue lane uses
`-PotionLane root-slots -PotionSlots 0` to admit only the exact starting potion
UUID in root slot 0. A replacement later generated in that slot is not
admitted. This command does not assign potion values.

Evaluate the published frozen behavior on a distinct held-out root batch with:

```powershell
.\learning\dev.ps1 evaluate-combat `
  -Artifact <held-out-combat-roots.bin> `
  -Behavior <completed-training-directory> `
  -Output <fresh-evaluation-directory> `
  -Roots <artifact-root-count> `
  -Replicates 8 `
  -BehaviorSeedBase 10000 `
  -PotionLane all
```

This command verifies the exact durable manifest, checkpoint, maintained model
profile, schema, trainer provenance, training step, training root artifact
digest, and training potion lane before evaluation. It rejects the training
artifact itself before constructing combat groups, uses independent explicit
RNG streams per root and replicate, and writes one compact
`evaluation.json`; it creates no optimizer, trainer, experience collector, or
promotion owner. The root and terminal records preserve HP/max HP, gold,
actionable living-enemy HP, concrete potion-slot identities, lost/gained
identity deltas, potion use/discard counts, turn, and card facts as separate
axes. Per-root enemy-HP ranges and signal-replicate counts make all-loss
variation inspectable without selecting it as an objective. Evaluation also
stores `root_audits` read directly from the opaque roots: seed, act/floor,
ascension, encounter and ordered monsters, entry HP, canonical card/upgrade
counts, relics, and potion slots. Artifact filenames are never trusted as
identity. Evaluation also
reports the published combat all-loss axis, so an opt-in behavior cannot be
mistaken for a default win/HP-only behavior. Potion identity deltas are
multiset inventory facts; the evaluator neither assigns potion tiers nor
invents an HP/gold/potion exchange
rate. Cross-combat resource value requires an exact continuation and remains
outside this command. These facts measure that exact manifest on the bounded
held-out sample and are not an improvement claim without a same-input frozen
baseline.

Audit one unchanged exact combat decision under two distinct frozen combat
publications with:

```powershell
.\learning\dev.ps1 audit-combat-policy `
  -Artifact <combat-roots.bin> `
  -BaselineBehavior <baseline-training-directory> `
  -CandidateBehavior <candidate-training-directory> `
  -Output <fresh-audit-directory> `
  -Roots <artifact-root-count> `
  -RootSlot <zero-based-root-slot> `
  -DecisionOrdinals <optional-explicit-prefix> `
  -PotionLane never
```

The optional prefix is the flattened sequence of model-facing ordinals, not a
display action string. Every ordinal is replayed against the current typed
candidate surface; symbolic selection rounds remain explicit, and the prefix
must finish at a new undecoded combat boundary. Rust then captures that exact
decision identity without exposing its checkpoint. The command scores the
unchanged semantic batch once with each behavior and writes
`policy-audit.json` containing the source and decision root identities, replayed
typed candidates, every current typed legal candidate, raw logits, normalized
probabilities, ranks, top-two margins, and candidate-minus-baseline probability
and rank deltas. Candidate ids are content identities over the exact decision,
ordinal, and typed semantics. The command chooses no audited action, consumes
no policy RNG, creates no trainer, and cannot accept or publish a behavior.
The first implementation accepts combat-training publications; run-publication
and automatic gate composition remain outside this diagnostic slice.

Run one paired comparison over a complete exact-root batch with:

```powershell
.\learning\dev.ps1 compare-combat-paired `
  -Artifact <held-out-combat-roots.bin> `
  -BaselineBehavior <accepted-or-baseline-directory> `
  -CandidateBehavior <candidate-directory> `
  -Output <fresh-comparison-directory> `
  -Roots <artifact-root-count> `
  -Replicates 2 `
  -BehaviorSeedBase 10000 `
  -CombatDecisionRule greedy `
  -PotionLane never
```

The command runs the ordinary held-out evaluator twice under the same exact
roots, root order, decision rule, potion lane, and explicit root-major
root-by-replicate seed matrix. Greedy remains the command default and consumes
no policy RNG; pass `-CombatDecisionRule sampled` for paired stochastic
evaluation. Sampled rows share one batched scorer call but draw only from the
generator owned by that replicate slot, so divergence or early termination in
one replicate cannot move another replicate's stream.
It retains both complete `evaluation.json` artifacts, then writes
`paired-comparison.json` with exact contract/comparison identities, every
root/replicate alignment, win transitions, and separate HP, enemy-HP, gold,
potion, turn, and card axes. Root differences precede aggregate differences;
the artifact emits no `better`, `accepted`, or scalar resource score. Sampled
pairing establishes aligned stochastic measurements, not statistical
independence among repeats of the same exact root and not an acceptance claim.

`-PotionLane never` runs the same frozen behavior and roots with every potion
use/discard candidate removed from the model-facing action surface. The engine
legality surface remains complete. Matching `all` and `never` runs with the
same behavior seeds are a bounded no-potion coverage comparison, not a reward,
a static potion ranking, or a change to training.
`-PotionLane root-slots -PotionSlots <zero-based-slot>` provides the next
bounded rescue comparison for one exact starting potion identity per root.
Run separate fresh outputs for separate slots; use a multi-slot array only when
the experiment explicitly asks about a combined fallback.
The routine comparison is one command:

```powershell
.\learning\dev.ps1 evaluate-combat-potions `
  -Artifact <held-out-combat-roots.bin> `
  -Behavior <completed-training-directory> `
  -Output <fresh-sweep-directory> `
  -Roots <artifact-root-count> `
  -Replicates 8 `
  -BehaviorSeedBase 10000
```

It evaluates `never`, each filled root potion slot independently, and `all`
under the same frozen behavior and RNG streams. It retains every lane's
ordinary `evaluation.json` and writes one compact `potion-sweep.json` index.
The sweep removes manual lane coordination; it still does not rank potions or
reduce HP and inventory to one reward.
For local classification only, each root includes an observed-resource Pareto
frontier over winning replicates using final HP, max HP, gold, and exact potion
multisets. A result with more HP but a different or smaller potion inventory is
left incomparable; the frontier does not model deck mutations, relic counters,
future encounters, or route value and is not automatically a training target.

The fixed-behavior census includes a typed competence plan over exact source slots.
All-loss roots enter a rescue backlog, mixed win/loss roots form the survival
frontier, and all-win roots either form the configured terminal-HP resource
frontier or remain solved. CombatFrontierRootSource exposes only the two
trainable frontiers and rechecks root identity whenever it creates a group;
rescue and solved slots cannot enter through its selected index surface. The
plan is bounded selection evidence, not a rescue algorithm or permission to
drop hard roots from evaluation accounting.
`sts_learning.torch_combat_batch_session.CombatWinBatchSessionFactory` owns the
first bounded shared update. Its config requires the selected frontier width to
equal the win-first objective's exact group delivery width without exceeding a
separate explicit source-root bound; a single trainable frontier root is valid.
One artifact import constructs one mutable shadow
model, one active frozen controller, one trainer, and one independent
caller-seeded behavior stream per root. Every root finishes under the same
behavior manifest before the trainer sees any group; distinct roots receive
equal objective weight. Collection failure restores all behavior RNG streams
without training, an all-no-signal delivery preserves the active behavior, and
a real optimizer step can cause only one live promotion. A temporary promotion
failure retries only that promotion. This session owns no held-out evaluation,
curriculum, durable optimizer resume, or claim that the new behavior is better.
`sts_learning.torch_combat_session.CombatWinSessionFactory` removes the manual
owner wiring around that runner. It accepts only a byte-bounded opaque
production root artifact (or its file), an exact root count and selected slot,
typed replicate/model/optimizer/resource configuration, and two explicit RNG
seeds. `new_from_artifact_*()` creates generation zero; `session.advance()` runs
at most one group; `session.publish_active_behavior()` is the only durable write.
Publication stores the active frozen behavior, not optimizer resume state. The
first maintained profile is relation-aware, one group per update, and CPU-only.
`sts_learning.combat_signals` reduces a completed group to nonzero replicate
and decision support per axis. Its cross-root census requires an explicit group
bound, rejects duplicate exact roots, and retains no semantic payload. Signal
coverage is diagnostic evidence, not an optimizer target.
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

`sts_learning.torch_behavior` separates live promotion from durable
publication. Live promotion copies the optimizer-owned shadow scorer into a
fresh model, freezes it, computes the same canonical checkpoint/manifest
identity used by persistence, and atomically replaces the exact active registry
row without writing files. The registry therefore retains one live binding
rather than one row per optimizer step. Durable publication later re-encodes
that frozen scorer and refuses to write unless its exact binding is unchanged;
it commits checkpoint and catalog row only at an explicit checkpoint boundary.
Restart recovery needs only the durable manifest id and newly opened owners; it
verifies/materializes a fresh scorer before hydrating the fresh registry. Every
path validates schema and behavior-rule identity, enters evaluation mode, and
freezes gradients. The live policy emits its exact registered manifest id on
every batched choice.
`CategoricalTorchBehaviorController` is the stable policy object retained by a
long-running driver. It accepts only increasing training steps and swaps its
internal frozen categorical policy only after live binding and registry rotation
succeed; a failed promotion leaves the prior live policy, registry row, and
injected selection RNG unchanged. `publish_active()` makes the current binding
durable without switching policy. A fresh inactive controller can recover the
active generation from its durable manifest identity after restart.

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
The generic assembler can represent batches with different behavior manifest
ids, but the maintained on-policy training path rejects such an attempt rather
than treating it as one policy sample.

`sts_learning.attempt_batching.BoundedAttemptUpdateBatcher` is the next
synchronous owner in that maintained path. It collects exactly the configured
number of complete attempts from one frozen behavior before one trainer call.
Attempts, decisions, and retained payload bytes are independently bounded;
mixed manifests, repeated lineage, overfull deliveries, and resource overflow
fail before optimizer mutation. Dropped-only deliveries pass through without
joining the tensor batch. A downstream exception poisons the owner and releases
pending arrays. A partially filled update batch is deliberately live-only and
cannot be encoded as a durable resume point. The owner exposes only its current
pending bounds and `require_quiescent()`; it has no durable snapshot or lifetime
accounting surface.

`sts_learning.torch_policy` is an optional, device-agnostic PyTorch baseline
over that same bridge-owned semantic graph. It is intentionally absent from
the package root, so ordinary caller imports still require only NumPy. The
scorer derives every embedding-table dimension and categorical offset from one
injected `semantic_schema()` result; it does not copy enum names, card ids, or
feature dictionaries into Python. Token kinds, categorical and scalar facts,
relations, and row-pooled context produce one flat candidate-logit tensor whose
boundaries are the unchanged bridge `candidate_row_splits`.
For schema-declared CardId and PotionId residual fields, the scorer initializes
the complete categorical slices to zero. Shared Rust-owned card and potion
mechanics therefore define the initial representation, while training may add
an identity-specific residual only for identities it actually observes.

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
before scoring, and applies the on-policy terminal loss
`-return * log P(selected | state)`. `FloorProgressReturnConfig` reserves `+1`
for victory; a defeat at floor `f` receives
`-1 + 2 * min(f, target_floor - 1) / target_floor`, so deeper failed runs carry
more information but never equal a win. Negative returns lower the sampled
action's relative probability and positive returns raise it; single-candidate
forced decisions have exactly zero gradient. Terms are averaged inside each
attempt and then across attempts, so a long attempt has the same total weight
as a short one.
The objective requires every manifest to carry the configured categorical
rule and verifies each recorded selection probability against the current
shadow scorer before mutation. Unknown or mismatched propensity is rejected as
off-policy instead of silently approximated. Censored and dropped attempt types
remain structurally excluded. A future off-policy objective must declare a new
contract and an explicit correction.
All validated decision payloads in one delivery are concatenated and scored in
one model call. Per-row weights preserve the exact attempt-equal loss, while
eliminating one small PyTorch forward per historical decision. The caller must
inject semantic concat row and input-array-byte limits; batching is never an
excuse for unbounded replay memory. The opt-in whole-run `ppo-clip-value`
profile keeps that exact attempt-equal weighting, trains one state-value row
per eligible decision against its unnormalized decision-local return-to-go,
and freezes the resulting
rollout advantage across its bounded PPO epochs. This is long-horizon credit,
not a static HP reward: early combat HP, Burning Blood recovery, potions, and
later route consequences remain part of the observed state and final outcome
rather than receiving a hand-written exchange rate.

`sts_learning.torch_training.SynchronousPolicyTrainer` is the synchronous
shadow-policy sink behind the update batcher. A non-empty training delivery
must contain exactly the configured attempts per update. The compatibility
REINFORCE profile causes exactly one optimizer step; opt-in value PPO may apply
up to four clipped epochs, with KL early stop, entropy regularization, gradient
clipping, and explicit value-loss diagnostics. A dropped-only delivery only
updates accounting. The trainer
retains no attempt queue and no semantic arrays, only scalar totals and the
latest bounded manifest-id and selection-probability sequences. Its required
concat limits bound the one-forward replay batch.
Unknown behavior, unknown propensity, behavior-rule mismatch, and recomputed
probability mismatch all fail before mutation. A backward or optimizer
exception poisons the trainer instead of inviting a retry over possibly partial
state. Dropped-only deliveries never train. Promotion creates a new
exact in-memory behavior binding; durable checkpoints are explicit rather than
an optimizer side effect. The trainer never silently rewrites behavior
identity. The trainer implementation artifact binds the floor-return target and
attempts per update; restore and runner wiring reject either mismatch.

`sts_learning.torch_generation.BoundedCategoricalGenerationRunner` is the
first deliberately finite composition of these owners. Construction fails
before environment mutation unless the driver, attempt assembler, update
batcher, synchronous trainer, categorical controller, shared registry, shadow
scorer, and optimizer
parameters form one exact chain. A generation is exactly one optimizer step
beyond the active behavior manifest's training step; larger values are rejected
because the second update would already be off-policy against the still-live
frozen behavior. Each call has a caller-supplied batch-step limit and flushes
the experience segment only after a terminal batch. The behavior stays frozen
while the update batch fills, then promotes immediately after that one update.
Promotion freezes a fresh in-process scorer and rotates one active registry row;
it neither consumes durable capacity nor writes a resume point. An exhausted
call with no terminal update leaves the old frozen behavior live. Its result is
aggregate-only and never retains step results or attempts. The runner does not
own persistence: restarting exact training goes through the
separate six-component resume store and typed restorer. Its resume admission
boundary is fail-closed: the environment must be between decisions with no
terminal half-state, the episode-root bank must cover every slot, the experience
buffer must be flushed, the assembler must have no open attempt, the update
batcher must be empty and healthy, segment sequence ids must agree, the trainer
must be healthy, and an active behavior
manifest must not be ahead of the shadow optimizer.
At an admitted boundary, `sts_learning.torch_resume_metadata` encodes the seed
schedule, active ledger lineage, experience/assembler sequence state, trainer
counters and bounded last-evidence fields, controller identity, promotion
count, and generation target as one canonical scalar component. The live-only
update batcher contributes no metadata: admission first requires it to be empty
and healthy, and restore constructs a fresh empty owner from the typed objective
configuration. A generation still owns exactly one frozen-behavior delivery;
its published training step advances by the number of optimizer epochs that
actually committed. Fresh ledger, empty buffer, empty assembler, trainer, and
controller owners have explicit restore constructors; terminal half-states,
open attempts, poisoned trainers, and inconsistent sequence or parameter
lineage remain unrepresentable. This metadata is the scalar member of the final
durable resume manifest, not an independent resume authority.

`sts_learning.resume_store` is the bounded durable owner for the six immutable
resume components: current environment, episode-root bank, shadow model,
optimizer, categorical generator, and generation metadata. It batch-previews
all distinct component count/bytes before writing anything, commits components
through the shared flushed atomic content store, and publishes one small
canonical manifest last. Reopen verifies filenames, digests, envelopes, kinds,
sizes, and the complete six-way binding. The optional
`CategoricalGenerationResumePublisher` captures all six from one admitted live
runner boundary after first making the active behavior binding durable, so
callers do not assemble manifests by hand.
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
`advance_generation(max_batch_steps=...)` advances only live state. Callers set
checkpoint cadence explicitly by invoking `publish()`; neither completed nor
unfinished generation calls write files. `advance_generations(...)` removes
caller-written loops while remaining bounded: it stops at the first incomplete
generation and returns aggregate counters only. Restore verifies the saved slot
count, training seed partition, and recovery budget against the supplied
session configuration. The
first maintained profile is deliberately CPU-only and defaults to eight
same-behavior complete attempts per optimizer update. Maintained online
sessions require at least one relation layer: a relation-blind bag of tokens
cannot associate candidate actions with their card, potion, or monster targets
and is only a lower-level scorer test configuration. `recover_behavior(...)`
materializes a frozen manifest with an explicit fresh RNG seed for the existing
paired held-out evaluator; evaluation never reuses the mutable shadow model.

Use the repository's single learning-development entrypoint after configuring
one Python 3.12 runtime that already contains NumPy, PyTorch, and the bridge:

```powershell
.\learning\dev.ps1 configure -Python <python-3.12-with-torch-and-bridge>
.\learning\dev.ps1 doctor
.\learning\dev.ps1 refresh-bridge
.\learning\dev.ps1 test
.\learning\dev.ps1 verify -MaturinPython <python-with-maturin>
```

`test` fails if the configured training dependencies are unavailable and runs
the complete learning suite. `doctor` also rejects an installed bridge that is
missing a maintained `LearningBatchEnv` surface. `refresh-bridge` builds a fresh
wheel, passes its Rust/smoke/isolated-caller verification, and only then replaces
the bridge in the configured training Python without changing dependencies.
Its default `-BridgeProfile release` is the timing and milestone artifact;
`-BridgeProfile dev` is the short functional-experiment loop and must not be
used for throughput evidence.
On first setup, `refresh-bridge -Python <python.exe>` performs the same guarded
install and records that runtime only after `doctor` succeeds.
`verify` runs the configured suite first, then delegates to
the lower-level `bindings/python_learning/verify.ps1` for a fresh wheel, Rust
bridge contracts, and isolated minimal caller coverage. The lower-level command
allows optional PyTorch tests to skip and must not be used as evidence that the
training suite ran.
