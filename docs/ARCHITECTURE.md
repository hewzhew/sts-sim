# Architecture

This file is the maintained architecture contract for current AI, runner, and
artifact work. It replaces the old set of narrow boundary notes.

## Guiding Rule

```text
unified typed representation
  -> explicit phases
  -> pluggable decision owners
  -> execution applies typed decisions without reinterpreting policy
```

Free-form strings are display and provenance only. If a decision needs to be
continued, replayed, compared, or learned from, it needs typed identity first.

## Cargo Package Boundary

The maintained oracle command path has one compile-time dependency direction:

```text
oracle executable hosts -> sts_oracle_runtime -> sts_oracle_run_control
                                              -> sts_oracle_eval -> sts_simulator
learning callers -> sts_oracle_learning -> sts_oracle_learning_env
                                      -> sts_oracle_run_control
combat-search frontend -> capability worker -> sts_oracle_eval
```

`sts_simulator` owns game content, state, engine transitions, simulation, and
stable lower policy layers. `sts_oracle_eval` owns combat evaluation and
exact-search orchestration, including the production-independent typed
`CombatCaseCoreV1` consumed by fixed-combat frontends.
`sts_oracle_run_control` consumes that lower surface and owns exact run
sessions, non-combat decisions, combat application, and the
diagnostic/production envelopes surrounding that core.
`sts_oracle_learning_env` consumes run-control and owns exact single-episode
learning environments plus opaque combat-root artifacts.
`sts_oracle_learning` is the downstream model-facing adapter and owns ragged
model views and batched pools; neither learning crate owns a policy objective.
`sts_oracle_runtime` consumes run-control and owns the interactive analysis
session/workbench, production-owner parity, branch execution, scheduling,
persistence, and resident services. Analysis source may remain physically below
the historical `src/eval/` tree, but it is compiled only by this low-optimization
orchestration owner.
Command hosts contain only supported adapters and cross-layer integration
contracts; they own no policy semantics. A capability with a high edit rate
may use a dependency-light frontend and a dedicated worker so help and syntax
checks do not compile the engine. The worker still delegates semantic work to
evaluation or a lower owner. Lower layers must never import learning,
run-control, branch runtime, or a command host.

Some evaluation, run-control, learning-environment, learning-adapter, runtime,
and command sources still live
physically below the historical root `src/` tree and are attached from their
single Cargo owner with explicit paths. `src/eval/mod.rs` is the combat-eval
owner boundary; run-control and learning modules below the same physical tree
are attached only by their named crates. Physical proximity is not permission
for a reverse dependency, duplicate owner, or broad facade dependency.

Use `cargo test-core` and `cargo test-control` for their respective unit-test
harnesses, `cargo architecture` for dependency-free source-boundary checks,
and `cargo check-workspace` for every target. `test-control` explicitly names
combat eval, run-control, learning, runtime, and command owners; a dependency's
private unit tests are never assumed to run transitively. Do not merge the
harnesses again through test features or replace them with many integration-test
executables.

## AI Layers

New AI code must choose an owner layer before it is written:

- `domain`: stable game facts and vocabulary. No value judgments.
- `analysis`: profiles derived from public state. No scene choice.
- `strategy`: typed deck facts, package state, deficits, admission rules, and
  small shared evaluators used by policies.
- `policy`: thin scene adapters for reward, shop, campfire, event, route, Neow,
  boss relic, and run-choice decisions.
- `runtime`: branch execution, scheduling, journals, replay, capsules, budgets,
  and artifact writing.
- `legacy`: still-required old code that is not the design target.

The intended flow is:

```text
domain -> analysis -> strategy -> policy -> runtime
```

The learned-policy path is a separate clean-room flow:

```text
domain -> hidden-free observation + complete typed action surface
       -> learned policy/value -> runtime
```

It must not pass through current strategy scores, owner ranks, explanation
strings, or search-private state. Existing owners may supply behavior
trajectories, but their selected actions are provenance rather than teacher
labels.

Shared card mechanics used by that path belong to `content::cards::mechanics`,
not to an analysis or reward-policy table. The profile owns upgrade-sensitive
effective target, exhaust, and ethereal facts and exposes positive composable
roles with explicit `DefinitionOnly`, `Partial`, or `Complete` coverage. An
absent role is not a negative fact unless coverage is complete. Promoting a
card to complete coverage requires a production play/trigger action test in
the same change; learned adapters may project this domain contract but must not
silently infer missing roles from policy heuristics.

Do not add another scene-local strategic model when reward, shop, route, and
branch retention need the same concept. Shared concepts belong in `analysis` or
`strategy`; scene-specific button mapping belongs in `policy`; applying a
typed action belongs in `runtime`.

## Non-Combat Automation

Run-control automation reduces manual repetition. It may execute bounded
route, reward, shop, campfire, event, run-choice, and combat-handoff decisions.
It is not a teacher label and not proof that a policy is good.

Every automated non-trivial decision has this role:

```text
label_role = behavior_policy_not_teacher
```

Non-combat decision records must stay hidden-free:

- public observations are allowed,
- declared distributions and beliefs are allowed,
- privileged simulator futures are forbidden.

Persisted analysis nodes may retain the owner rank observed when their choice
surface was materialized, but an active `owner` or rank-based steering command
must recompute the current typed owner order and join it to that exact surface
by candidate id. Display labels and stale numeric ranks are not execution
authority.

Automation should stop when the current site lacks a bounded policy answer. Do
not encode stale global rules such as "shops always stop" or "events always
stop"; each high-agency site needs its own owner/compiler boundary.

`LearningEnvV1` is the in-process online-learning boundary over the same
run-control mutation authority. It exposes strategic decisions through
`PlannerObservation` and `LegalCandidateSet`, combat decisions through the
typed combat action surface, exact checkpoint/restore, and only sparse terminal
reward. A curriculum or recovery controller stays outside the environment and
must not change RNG when restoring a checkpoint.

Every fresh learning run is constructed with an explicit ascension level in
`0..=20`; the bridge has no default ascension. Run training, run evaluation,
and run-derived combat-root collection preserve that configuration in their
durable provenance. Exact combat-root consumers inherit ascension from the
root and never accept a filename-derived or caller-overridden replacement.

The combat learning branch does not serialize raw combat state. Its explicit
projection uses domain card, potion, enemy, power, relic, orb, stance, and
intent identities rather than display labels. It includes turn/phase counters,
dynamic public card state, public card-zone contents, powers, stance/orbs,
relics, and the typed action surface. Indexed generated-card and stance choices
carry one ordered typed candidate list on that action surface, so submitted
indices do not require a duplicate observation payload. The engine records
monster moves in a separate append-only public history only when that monster
actually begins its turn; the mechanics move history remains private because
its final entry is the current unexecuted roll. Encounter-local move ids are
namespaced by typed enemy identity. Public encounter counters project only
facts that are visible and not already represented by history, powers, cards,
or ordinary entity state, currently Hexaghost's active orbs and each
Looter/Mugger's stolen gold. Private protocol flags, unrevealed random damage,
and raw monster runtime bundles remain absent. With these fields the combat
observation is complete for the maintained in-process learning environment.
The combat-local observation is not a complete run-continuation state by
itself. Every live `LearningCombatBoundaryV1` therefore also carries one typed
public run context: declared run goal, act, floor, keys, revealed map, and the
current encounter identity when known. A detached exact combat position marks
that context explicitly unavailable instead of reconstructing it from a case
name or exact checkpoint. The Rust model view retains this public context; its
numeric bridge projection remains a separate schema migration and may not be
silently inferred from combat-root collection metadata.

`LearningModelDecisionV1` is the in-process model adapter over those
boundaries. Its observation views omit schema labels, artifact identity, opaque
candidate ids, and mechanics manifests while retaining typed public semantics.
Candidate sets remain ragged and use row splits as their default batch mask;
rectangular padding masks are materialized only when a backend asks for one.
Atomic combat inputs carry aligned typed indexed-choice semantics, including
the choice reason and destination as well as the selected card or stance.
Combat and run-level multi-card selection families are decoded as an ordered
append-or-submit language without enumerating complete payloads, and only
explicit submit produces an environment action. Active decoders can themselves
form a ragged batch; each row retains the unchanged matching combat or
strategic parent observation, so autoregressive selection does not fall back to
one backend call per environment slot. The adapter does not serialize per-step
JSON and does not define a feature dictionary, network architecture, or policy
objective.

The first clean-room information boundary is capture-only.
`PublicInformationSnapshotV1` content-addresses one typed, sanitized public
observation, the history facts already folded into that snapshot, and the
complete ordered candidate surface actually exposed to the deployed policy.
Dedicated strategic and combat projectors remove card/potion UUIDs, monster
entity ids, exact candidate ids, and other resolution handles before hashing;
observation-local ordinals preserve distinct executable choices. The snapshot
has no exact root id, combat-state hash, live RNG cursor, normalized trajectory
prefix, or trajectory instance id, and it grants no search or training-label
authority. In particular, there is intentionally no executable
`ChanceEnsembleV1` or `SearchPolicyTargetV1` yet. The caller may assemble the
capture-only snapshots into a neutral public behavior trajectory, but search
targets additionally require a conditional sampler that covers every hidden
chance source and a typed information-set-search receipt rather than
caller-supplied strings and aggregate visit counts.

The Java-faithful engine action surface and the learning-policy candidate
surface are intentionally distinct. A free `DiscardPotion` UI action remains
engine-legal, but the policy surface withholds it unless another action at the
same unchanged decision can immediately refill the opened slot: a different
usable `EntropicBrew` or a currently playable `Alchemize`, with potion gain not
blocked by `Sozu`. This is a mechanical action prior, not a potion tier,
retained-value score, or permission to spend.

`LearningEnvPoolV1` owns a fixed set of independent environments and exposes
all non-terminal slots as one aligned ragged model batch. It prepares every
selected action against the unchanged slots before applying any of them, so an
invalid model output cannot partly advance a batch. An unexpected engine error
poisons the pool instead of allowing mixed advancement to continue. Terminal
slots leave the active batch and return only through an explicit caller-owned
replacement or reset; curriculum, recovery, seed scheduling, numeric encoding,
and policy inference remain outside the pool. This is the maintained boundary
for amortizing a future Rust or Python backend call across environments without
per-step JSON or one foreign-language call per slot.

`CombatLearningEnvV1` is a separate combat-episode boundary; it does not
reinterpret leaving combat as a run victory, defeat, or strategic decision. An
immutable `CombatLearningRootV1` binds the exact normalized run-session
fingerprint and exact combat-state hash to one combat-root checkpoint. It also
captures one compact typed `CombatLearningRootContextV1` from public root facts:
act, floor, ascension, turn, encounter flags, entity and inventory counts, deck,
relic and hand counts, and root HP. The context is collection metadata rather
than a second observation, feature dictionary, reward, or teacher label; it is
not repeated in decision rows. A separate exact resource snapshot retains root
HP/max HP, gold, and every potion slot by typed identity. Terminal episodes
retain those same resource axes after combat. These snapshots are outcome
facts, not a static potion ranking or an exchange rate among survival, HP,
gold, and potion identities. Every spawned episode carries the root identity
plus an explicit replicate index,
accepts only `LearningActionV1::CombatInput`, reuses the complete combat
observation and legal-action surface above, and terminates with the existing
typed `CombatBaselineOutcomeV1`. Its in-memory checkpoint retains current
session state and the unchanged root/replicate lineage; it is not a second
durable checkpoint format.

`CombatLearningEnvPoolV1` creates a fixed non-empty set of numbered replicates
from one immutable root and exposes their active decisions as one ragged model
batch. It validates the entire action round before the first mutation, poisons
itself after an unexpected partial engine failure, and keeps terminal outcomes
aligned to replicate identity. This same-root grouping is execution lineage,
not an estimator or teacher label. Grouped baselines, combat policy gradients,
or search-improved targets must be separate caller-owned objectives and may not
substitute unrelated run seeds for same-root replicates.

An active replicate may explicitly rebase its current exact session as a new
immutable combat root. The Python bridge exposes that operation only at an
undecoded simulator decision and returns an opaque in-process recovery root
that binds the new root identity to its source root identity and replicate
index. Partially decoded symbolic actions and terminal replicates cannot be
captured. No pool records automatic history, and no raw session payload crosses
the bridge. Spawning a group from the recovery root therefore preserves both
exact current-state identity and the otherwise-cross-slot parent lineage
without weakening ordinary slot-bound checkpoint restore.

The Python bridge may derive one `CombatLearningBatchEnv` only from an
undecoded combat-root slot in an existing typed batch. Python receives the
shared root identities, one frozen native view of the compact root context,
replicate indices, ordinary sparse semantic action rows, and typed combat
terminal columns; it does not receive or reconstruct the root session. Deriving
or running the group does not advance the source run slot. This bridge surface
owns neither reward shaping nor durable combat-group checkpoint publication.
Every current combat decision row has a companion frozen progress record in the
same replicate order: turn, player HP/max HP, current living-enemy HP, encounter
max-HP total, and exact potion UUID/domain identity columns. The caller binds it to the chosen
ordinal before mutation and retains model-call sequence order, allowing one
replicate's chronological decisions to be reconstructed without parsing JSON.
Selection-prefix rows intentionally repeat the unchanged simulator progress.
These records are trajectory facts, not model inputs, rewards, or teacher labels.
The caller may join consecutive records into action-aligned transitions. It
keeps terminal win, future player-HP change, and future enemy-HP change as
independent undiscounted return-to-go axes; potion identities remain typed
before/after facts with no scalar exchange rate. The terminal bridge currently
does not expose potion UUIDs, so a terminal after-state must mark that UUID
column unavailable instead of guessing continuity from equal potion ids.
These axis-specific returns must not be sent through one critic or normalized
across unlike axes and unlike decision times. The combat value-PPO consumer
owns three fixed-semantic value columns in enum order: win, player-HP change,
and enemy-HP change. It selects only the column matching the group's typed
win-first axis, uses that decision's return-to-go as the critic target, and
forms the actor advantage from the same row and column. It never changes one
scalar head's meaning after observing the terminal group and never centers
residuals across unrelated decision times. Policy-only REINFORCE/PPO retain
the terminal same-root comparison objective and do not pretend to consume a
critic rollout.
The source batch can enumerate all current undecoded combat roots and their
frozen contexts in one call without creating replicate groups or cloning their
sessions; only caller-selected roots pay group construction cost.
It may also export an explicit non-empty selection of those current roots as
the existing opaque `CombatLearningRootBatchArtifactV1` bytes. Rust rejects
duplicate, non-combat, decoded, terminal, and oversized selections before
serialization; Python may persist or forward the payload but never inspects a
session checkpoint.

A caller may reconstruct a bounded reverse curriculum from an already completed
winning same-root episode by replaying that replicate's recorded ordinals from
the unchanged exact root. Replay must reproduce every typed terminal fact
before any derived root is admitted, and retains only a caller-bounded
terminal-nearest window. The ordinals prove replay identity; they are not
supervised action labels and do not enter the on-policy loss. This mechanism
cannot manufacture a teacher for an all-loss root: such a root remains in the
rescue backlog until a separately verified winning trajectory exists.
The maintained in-process recovery trainer discovers that win under one frozen
behavior, selects the highest-final-HP verified replicate with replicate index
as the deterministic tie-break, and requires the exact requested suffix-root
count before constructing an update. The caller declares the source artifact's
exact root count and one zero-based source slot; import validates the count,
while discovery and replay remain bound to that slot without extracting a new
artifact. Discovery, replay, and every derived root use the same explicit
potion lane. The derived groups then sample fresh
on-policy actions under that unchanged behavior; the selected replay ordinals
never enter their loss. Later optimizer updates may reuse the same immutable
suffix roots, but each update still samples under its own current frozen
behavior. The ordinary combat-training journal records the curriculum and
source artifact width, selected slot, and exact source facts so its final
publication remains recoverable by the same behavior loader.
For a production-context `CombatCase`, the offline learning-root producer may
replay a typed winning action file through the restored run-control session and
export only a bounded terminal-nearest window as an ordinary opaque root batch.
The producer must observe a new typed win before writing anything. Search
actions remain replay evidence outside the artifact; they neither cross the
Python bridge nor become policy labels.
Two or more canonical root artifacts may be composed without exposing their
checkpoints. Every input binds an explicit expected root count (single-root is
the compatibility default); Rust revalidates canonical encoding, every exact
root identity and context, the total byte/root bounds, and cross-input identity
uniqueness before publishing one fresh batch. This is the maintained boundary
for a small joint frontier/rehearsal curriculum. It does not rank roots, infer
teacher labels, or authorize unbounded corpus concatenation.
Before composition, one canonical batch may be reduced to an explicit ordered
set of source slots. Selection revalidates the source width and every retained
identity, rejects repeated or out-of-range slots, and never exposes checkpoint
fields. This provides typed curriculum balancing; slot order and membership
remain caller-declared experiment configuration rather than a learned rank.

Production runs hand combat boundaries to this bridge only through the
versioned `CombatLearningRootBatchArtifactV1` envelope owned by run control.
The first-combat collector advances a fresh seed through one narrow public
production non-combat step that hides private owner routing and selects the
same first auto-expandable candidate as the baseline branch policy. It stops
before combat search and contributes the exact in-memory checkpoint directly.
For later boundaries, the production-side exporter decodes a public
continuation and contributes its exact session checkpoint. In both paths the
shared eval layer independently recomputes and binds the combat-root identity
and compact context. Bridge import is
caller-byte-bounded, validates every root and exact root count before creating
the pool, and exposes no session fields to Python. The bridge does not depend
on runtime artifact types, guess continuation JSON fields, or read private
branch cutpoint schemas.
The bounded later-combat sampler advances one frozen published behavior, or one
explicit strategic-source/combat-anchor composition, over one seed partition
and stable seed-only partition spec. It then inspects only the bridge's aligned
public run context and undecoded combat-root context. In the composed scope,
typed combat rows use the verified anchor greedily while strategic rows retain
the source categorical rule and RNG; the two scorers must share model and
semantic contracts. This scope requires an explicit whole-run potion lane so
`trained` cannot ambiguously name two publication histories.
It captures at most one qualifying root per seed, filters by typed minimum and
optional maximum floor, exact prior-combat count, and usable-potion count, and may require
one bridge-validated canonical potion identity in one exact slot. Encounter
selection may first choose the broad typed ordinary, elite, boss, or any fight
class, then require one canonical identity, one root per distinct identity, or
an explicit fixed quota for each of several canonical identities. The identity
modes are mutually exclusive; every quota must be complete before publication.
It then asks Rust to
merge canonical single-root payloads without exposing checkpoint fields. The
sampler writes one fresh batch only after the requested root count is complete;
a deadline, step bound, duplicate root, incomplete encounter quota, context
mismatch, or byte overflow
publishes nothing. Its compact receipt binds the partition, partition spec,
capture bounds, and observed prior-combat count as corpus provenance, not a policy
label or a potion-value judgment. The receipt reuses the held-out resource
trace owner to attach earlier same-seed combat HP, gold, and concrete potion
transitions plus ordered canonical enemy identities; it does not retain
decisions, sessions, or a second trajectory format. History is sliced strictly
before the captured `(act, floor)` boundary, even when another slot keeps the
batch alive and the captured episode later advances.
The minimum usable-potion selector may be zero for an unconditioned run-derived
combat corpus; concrete potion rescue corpora still bind an exact identity and
slot.
An optional distinct-encounter contract deduplicates canonical `EncounterId`
inside one batch. It is explicit rather than a silent default,
because repeated exact roots can still be useful for distributional training.
An exact encounter selector binds one canonical `EncounterId`; the bridge
normalizes it through the simulator's typed parser so an unknown name fails
before any run advances. Variable members of encounters such as `GremlinGang`
do not change that identity.
Persisting the identity changed the serialized run checkpoint layout. Combat
root artifacts and cross-process learning checkpoints therefore use format
version 2 and reject version 1 before payload decoding; old evidence is
regenerated rather than guessed or silently migrated.

Terminal learning steps retain the typed run result plus public terminal act,
floor, HP, max HP, and gold. The Python bridge returns those facts as compact
columns aligned only to terminal slots. They are outcome evidence for progress
and lower-tail targets, not a shaped reward or a context-free resource score.
Combat learning distinguishes an exact victory from leaving through Smoke
Bomb: the latter is a completed episode with terminal kind `Unresolved` and
`terminal_won=false`, so an escape cannot become a win target merely because
the run reached a post-combat screen.
An explicit `public_run_contexts()` snapshot separately exposes every current
slot's seed, typed boundary kind, act/floor, HP/max HP, gold, and concrete
potion-slot identities. Active combat contexts additionally expose ordered
canonical enemy identities and their canonical encounter identity; non-combat
contexts expose neither. It
clones no session and retains no history.
At a combat boundary, resource fields come from the active combat after
pre-battle triggers rather than the persistent run snapshot, which is not
synchronized again until combat resolution.
Evaluation callers may use successive snapshots to identify cross-combat
resource transitions, but the bridge does not assign potion value, aggregate
the facts, or feed them into the terminal reward.
Whole-run evaluation may bind either the ordinary `All` combat potion surface
or a `Never` counterfactual that removes combat potion use and discard from
model candidates. It does not support root-slot lanes because a complete run
has no single immutable combat-root inventory. The lane is evaluation
provenance, not a resource price or a training reward. Routine evaluation
inherits the published behavior's training lane, preventing an untrained
potion action surface from appearing silently at deployment; explicit `All`
and `Never` remain available for bounded counterfactuals.
The first whole-run training handoff follows the same rule for its training and
held-out environments. A `Never` run session rejects cross-process resume until
that checkpoint schema binds the potion surface; it may still publish the
frozen learned behavior, which carries no environment checkpoint.

The standalone `bindings/python_learning` Maturin crate is excluded from the
root Cargo workspace. Python supplies observation-local candidate ordinals,
while Rust owns typed root/selection decoding and batched environment mutation.
Slot ids, decision phase, candidate counts, row splits, terminal results, and
optional dense masks cross the boundary as NumPy arrays. An opt-in, versioned
sparse semantic graph encodes complete strategic, combat-root, and
symbolic-selection rows as token, categorical-feature, scalar-feature,
relation-edge, and candidate-token NumPy tables. Combat rows retain the full
public learning observation, ordered/unordered evidence, action targets,
indexed-choice reason/destination, and symbolic selection family plus chosen
prefix. Run-selection rows retain the full strategic observation and link each
selection-domain token to its eligible master-deck card. Opaque
observation/candidate ids and entity UUIDs are not features;
internal identities may only resolve graph relationships. An unexpected
out-of-surface combat input fails closed rather than emitting a partial row.
Monster intent follows the game's information boundary. Single-step monster
plans materialize their unambiguous `visible_spec`; multi-step plans must
declare one explicitly. A public observation uses an explicit protocol mirror
when available and otherwise falls back only to that visible spec, never to
private move RNG or arbitrary execution steps. Current monster damage preview
is projected through the simulator damage pipeline. Card projections likewise
ignore mutable rendering caches: every public card gets fresh current damage,
block, and magic values, while hand and limbo attacks additionally carry
damage aligned to monster order. Other card zones do not repeat the per-target
projection. Runic Dome hides both monster intent and monster damage preview
before either source is consulted.
`semantic_schema()` exposes enum dictionaries and categorical vocabulary sizes
from the same Rust definitions, avoiding a second Python feature dictionary.
Changing an encoded field's meaning or turning a previously constant field into
live information requires a semantic-schema version bump even when the table
shape is unchanged. Checkpoints trained under the prior meaning must fail
closed rather than silently consuming the new distribution.
Semantic schema v8 also declares identity-residual categorical fields. Card
and potion identities are encoded alongside shared typed mechanics: card
definition type, rarity, upgrade-sensitive effective target and lifecycle
flags, base/upgrade numerics, multi-damage, explicit mechanic-role coverage,
and reviewed positive effect roles; potion definition facts and composable
effect roles. Missing card roles remain unknown for definition-only and partial
profiles rather than becoming negative features. The
PyTorch scorer zero-initializes only the declared CardId/PotionId residual
vocabularies, so an identity not exercised by training contributes its shared
mechanics but no random identity vector. Enemy, power, and relic identities
remain ordinary embeddings until comparable mechanical projections exist;
they must not be relabeled as residuals merely to hide missing semantics.
The simulator's legal combat action surface remains complete. A separate
exact-state equivalence projection maps only proven duplicate starter-basic
plays (same complete runtime card signature and target) and supported
single-card pending selections to a canonical original input. Search and the
learning environment consume that shared projection; the model-facing combat
surface retains one representative per class and executes its unchanged typed
`ClientInput`. Duplicate non-starter cards, different targets, and different
runtime card state remain separate. Schema v8 also omits physical hand position
and play-card hand index from semantic features. Candidate-to-card graph edges
retain the chosen mechanics without teaching categorical multiplicity or an
unstable UI ordering; ordered draw/limbo evidence and potion/indexed-choice
addresses keep their distinct semantics.
The bridge still owns no policy, optimizer, automatic reset, or PyTorch
dependency. Keeping the crate standalone prevents Python build dependencies
from entering ordinary simulator checks.

An undecoded combat slot may create a fixed-size group of exact same-root
combat episodes. Rust owns the normalized run-session root id, exact combat
state hash, numbered replicate lineage, and terminal outcome facts; each
bridge transition repeats both root identities and emits aligned terminal win,
player HP/max HP, total actionable living-enemy HP at the root and terminal
boundary, gold, concrete potion-slot identities, turn, potion-action counts,
and card-play columns only for newly terminal replicates. Escaped, dying, and
half-dead entities retained by the engine do not become remaining-HP evidence.
The Python caller rejects a terminal batch from another root before mutating
its bounded accumulator and completes a group only after receiving exactly one
outcome for every replicate. Same-root leave-one-out evidence remains four
independent axes: win, player terminal-HP ratio, enemy-HP progress, and potion
retention. Enemy-HP progress is diagnostic evidence by default. A separately
provenanced opt-in objective may select it only when every replicate is an
exact loss; escaped or otherwise unresolved terminals remain ineligible. There
is no default scalar exchange rate among survival, either HP axis, and potions,
and the execution primitive is not itself a trainer or teacher.

Held-out combat evaluation preserves these resource facts without reducing
them to one score. Its root and replicate records make HP and gold deltas,
concrete starting/final/lost/gained potion identities, and use/discard counts
independently inspectable. The evaluator also joins each selected opaque root
to the bridge-owned public run context and requires its seed, canonical
encounter identity, and ordered monster identities to agree with the spawned
exact root. Encounter aggregates therefore stratify the same terminal facts;
they do not become a reward, teacher label, or substitute for exact roots. A
lost identity is a multiset inventory fact, not a
claim about whether spending it was strategically correct. Cross-combat value
requires an exact run continuation and remains outside the combat evaluator.
Evaluation may opt into a bounded pre-action diagnostic trace for the first
caller-declared replicates of each root. Rust emits that record only after a
symbolic selection becomes one decoded action; it contains the typed public
combat observation, decoded action, selection prefix, and exact replicate
identity. Python adds the frozen model round, selected ordinal, and selection
probability, then writes the records to a separate JSONL sidecar. The ordinary
evaluation summary retains only the sidecar schema, filename, record count,
and replicate bound. Tracing defaults to zero and is evidence only: it neither
changes the model batch nor becomes reward, experience, or a teacher label.
Published behavior recovery also retains the training root artifact digest and
training potion lane from the bounded journal. Evaluation rejects an artifact
with that same digest before constructing any combat groups, so a training-set
measurement cannot be labeled held-out.
Each evaluation also declares one model-facing potion lane. `All` exposes the
ordinary learning candidate surface; `Never` removes every potion use and
discard candidate; `RootSlots` admits only the exact starting potion identities
occupying caller-declared root slots. Rust binds those identities by UUID, so
using one does not authorize a generated replacement in the same slot. None of
the lanes changes the engine's legal action surface. Running them on the same
exact roots and behavior RNG streams is a bounded counterfactual coverage check,
not a potion-value judgment, reward, or training policy. The caller-owned
potion sweep composes `Never`, every filled root slot as its own `RootSlots`
lane, and `All` under that unchanged identity boundary. It retains each
ordinary evaluation artifact and writes one compact comparison index; it does
not add a scoring rule or infer which resource tradeoff is best.
Among winning replicates, the evaluator may additionally report an
`observed-resource` Pareto frontier over final HP, max HP, gold, and exact
potion-identity multisets. One result orders another only when every observed
axis is no worse. Different potion identities and any HP/potion tradeoff stay
incomparable. The frontier intentionally omits deck mutations, persistent relic
counters, future encounters, and route options, so it is not continuation
dominance or a training reward by itself.

The combat-group caller captures each semantic decision batch before policy
inference through the same policy-neutral frozen-row owner used by run
experience. It records the chosen ordinal, selection probability, and one
unchanged behavior-manifest identity only after the bridge accepts that choice.
Decision count, retained payload bytes, model rounds, and environment
transitions all have mandatory hard limits; a batch that would exceed memory
fails before environment mutation. Partial groups are never delivered as
completed training experience.
The first maintained differentiable combat objective consumes only complete
same-root groups and selects one leave-one-out axis per root. A group with any
win/loss variation uses only its win axis. Once every replicate wins, a typed
all-win configuration selects either no fallback axis for strict win-only
ablation or terminal-HP ratio so early combats keep learning resource
preservation after survival is solved. This selection is bound into trainer
provenance. An independent all-loss configuration defaults to no fallback; its
explicit enemy-HP-progress mode applies only when every terminal is an exact
loss and that axis varies. Unresolved terminals never enter this fallback, and
potion retention remains typed evidence outside the loss. The lexicographic
selection therefore creates no scalar exchange rate among survival, either HP
axis, and potions. Groups have equal total weight,
replicates have equal total weight inside a group regardless of combat length,
and each replicate divides its weight across its own retained decisions. The
objective rechecks exact behavior manifests and recorded selection propensities
against the scorer through contiguous semantic microbatches. Concat row and byte
limits bound each materialization, while group-equal, replicate-equal, and
decision-local weights remain global across the complete delivery. Backward
gradients accumulate across those microbatches and one epoch performs at most
one optimizer step; KL, clipping, entropy, and value-loss diagnostics use the
same global weights. The baseline update rule is one exact REINFORCE step. The
optional PPO-clip rule first performs that same
on-policy check, then may reuse the immutable batch for a bounded number of
optimizer epochs against its recorded behavior probabilities. Ratio clipping,
gradient-norm clipping, entropy regularization, and a target-KL early stop are
part of the typed trainer provenance; approximate KL, clip fraction, entropy,
and actual optimizer-step count remain training diagnostics rather than held-out
evidence.
The distinct PPO-clip-value rule adds three fixed-semantic value columns over
the shared decision-row state. The win-first group selector chooses win return,
future player-HP change for an eligible all-win group, or future enemy-HP
change for an explicitly eligible exact all-loss group. Each actor advantage
is its own decision-local return-to-go minus the matching pre-update value;
there is no whole-group residual centering across turns. The heads start at
zero, and actor advantages plus behavior probabilities are frozen when the
delivery is collected and remain unchanged across PPO epochs, while only the
selected critic column receives weighted mean-squared return loss. An
actor-only warm start initializes every shared policy key and leaves the three
value columns at their defined zero-output initialization. Actor-only, scalar
actor-critic, and multi-value combat scorers retain distinct exact model and
trainer identities.
The training root source also binds one explicit model-facing potion lane.
`Never` is the primary resource-preserving lane for roots that can already win
without potion actions; it makes terminal-HP refinement honest by removing
potion use/discard candidates rather than pricing their outcomes. An all-loss
no-potion group remains no-signal by default. A bounded investigation may
explicitly enable enemy-HP progress to test whether damage support can be
learned without reopening potions; a concrete-potion rescue question still
uses `RootSlots` for one starting identity at a time. Training never silently
reopens unrestricted potion actions.
The synchronous combat-win trainer has its own objective configuration and
trainer provenance; it cannot reuse a terminal floor-return behavior manifest.
Each delivery contains exactly the declared number of complete groups. No
selected-axis signal skips backward and optimizer mutation, while a nonzero
signal whose policy gradient is exactly zero also cannot claim a training step.
A REINFORCE delivery applies exactly one optimizer step. A PPO-clip or
PPO-clip-value delivery
applies one or more bounded optimizer steps until its epoch cap, zero gradient,
or target-KL stop, then discards the batch. The trainer retains scalar counters
and bounded identity evidence rather than completed combat payloads.

The first combat generation runner deliberately narrows that contract to one
fixed exact root, one group per call, and `groups_per_update == 1`. Construction
requires one exact scorer/optimizer/registry/controller/provenance chain. The
live behavior stays frozen while the group runs. A no-signal or zero-gradient
result leaves it unchanged; one finite bounded update must be followed by one
immediate atomic live promotion at the resulting optimizer step. If promotion temporarily fails, the runner
retains only a compact pending result and retries promotion before requesting
another group, so it cannot apply a second delivery to the same experience. A changed source
root is rejected before policy or environment mutation. This runner owns no
cross-root scheduler, durable checkpoint cadence, or HP/potion scalar target.

The compact combat session factory is the maintained artifact-to-runner
composition. Its bridge adapter exposes only the installed semantic schema and
the byte-bounded opaque combat-root artifact loader. Configuration binds the
expected root count, selected root slot, replicate count, relation-aware scorer,
categorical behavior, Adam optimizer, one-group win-first objective, experience and
concat limits, CPU device, and immutable-store capacity. Creating generation
zero imports and validates the artifact before constructing one exact owner
chain; callers do not manually assemble a registry, publisher, controller, or
trainer. Live advancement writes no files. Explicit behavior publication stores
only the active frozen scorer and manifest, not optimizer state or a claim of
durable combat-training resume.

Every combat generation result reduces its completed group to one compact
per-axis signal summary before dropping the experience payload. Those summaries
may then form a caller-bounded census over distinct exact roots. The summary
and census retain only root identity plus group, replicate, and decision
counts; they neither keep semantic payloads nor turn signal coverage into a
policy target.
The diagnostic combat census runner visits every declared artifact root from
identical initial model weights and caller-supplied independent behavior RNG
seeds. It imports the opaque root batch once, then selects each root from that
shared typed source instead of decoding the complete batch once per slot. Each
root owns an isolated temporary trainer; any local update is discarded after
its compact generation result is captured, and no behavior is published. This
measures signal coverage and is not a shared cross-root training scheduler.
The census also derives one bounded exact-slot competence plan. Mixed
win/loss roots are the survival frontier. All-win roots are either a
terminal-HP resource frontier when that configured axis has signal or solved.
All-loss roots remain an explicit rescue backlog. An identity-checking selected
root source exposes only the trainable frontiers to an ordinary batch trainer;
it cannot silently route rescue or solved slots, and the rescue backlog remains
part of evaluation accounting. This partition is curriculum selection
evidence, not itself a rescue algorithm or teacher target.

The bounded combat batch generation runner is the first shared cross-root
training owner. Every update forks the same active frozen scorer and manifest
onto one distinct caller-seeded RNG stream per exact root. It collects all
complete groups before calling the synchronous trainer, whose objective gives
every distinct root equal total weight. Collection failure restores every
behavior RNG and performs no trainer or controller mutation. A completed
delivery attempts at most one optimizer step and one live promotion; a
temporary promotion failure retains only the compact batch result and retries
promotion without collecting or training again. The compact batch session
imports the opaque artifact once and owns this complete graph. It is not a
held-out evaluator, curriculum, durable training-resume protocol, or evidence
that an update improved play.
Generation zero may optionally copy parameters from one fully verified
published combat behavior into a fresh trainable shadow before the new
controller is bound. This is warm-start initialization under the destination
root set, potion lane, objective, and fresh optimizer—not optimizer resume or
continuation of the source training step. The new journal retains the source
manifest/checkpoint identity while the new behavior manifest starts its own
training provenance. Warm-start admission verifies the source publication and
still requires exact model configuration, semantic schema, behavior rule, and
checkpoint tensor compatibility. Historical model-definition, optimizer, or
trainer-provenance digests may differ when the tensor contract remains
compatible; every such difference is recorded. Any admitted provenance
difference forces an actor-only copy, leaving the destination combat critic at
its defined initialization instead of importing an obsolete value function.

For a nonzero update count, the maintained `train-combat` command first runs
that census from the exact destination initialization, potion lane, replicate
count, and per-root behavior seeds. The already validated root source is reused;
the artifact is not decoded again for census or training. The command then
constructs its shared trainer only over the plan's survival and resource
frontiers. Default all-loss roots stay in the journaled rescue backlog instead
of receiving an invented damage target, while solved roots remain outside the
optimizer. A zero-update baseline skips census and publishes initialization
only.

The command repeats a caller-bounded number of shared-model updates over that
fixed selected frontier. It may start randomly or from that verified parameter
copy. It journals only compact
configuration, generation, per-root outcome/signal, and completion facts, then
explicitly publishes the final active behavior. Its experiment directory must
be fresh; the journal and published scorer do not contain optimizer state and
cannot resume training or substitute for held-out evaluation.

The separate combat held-out evaluator recovers exactly one published frozen
behavior from its durable manifest and tensor-only checkpoint, verifies the
complete maintained scorer, categorical rule, optimizer, trainer, schema, and
training-step provenance against the originating training journal, then runs a
distinct opaque root artifact with independent caller-seeded policy RNG streams.
It constructs no optimizer, trainer, controller, promotion owner, or experience
collector. Semantic decision batches are discarded after each choice; only
compact per-replicate terminal facts survive. Its fresh single-file result is
competence evidence for that exact manifest and evaluation sample, not an
improvement claim without a comparable frozen baseline.
Its result also carries a bridge-read audit of every opaque evaluation root:
seed, act/floor, ascension, encounter and ordered monsters, entry HP, canonical
card/upgrade counts, relics, and potion slots. Artifact filenames and prior
training journals are never used to reconstruct that identity.

### Learning Control Plane

The simulator, bridge, collector, learner, evaluator, and behavior-release
decision are separate roles. A command may compose those roles synchronously
in one process, but its filesystem layout or call order must not become their
implicit protocol. The same role boundaries must remain valid if collection
and learning later move to separate processes.

There are two deliberately different behavior transitions:

1. A learner may rotate its immutable active-generation behavior after a real
   optimizer step so that the next collection remains on-policy. This is a
   training-local transition and makes no competence claim.
2. A durably published behavior is a candidate until a separate behavior gate
   compares it with one accepted baseline. Only the gate's complete evidence
   may make it eligible for an explicit accepted-registry update.

The active-generation controller and the accepted-behavior registry therefore
must not share mutation authority. Until the accepted registry and gate result
types exist, every current training publication remains a candidate regardless
of its generation number or held-out summary; callers must not reconstruct an
"accepted latest" behavior by scanning directory names.

The first control-plane migration uses four immutable, versioned contracts:

- `CombatExperimentManifestV1` binds the source behavior, exact root cohorts,
  explicit source slots and curriculum roles, model-facing potion lane,
  objective/value axis, model and optimizer provenance, replicate count,
  update bounds, and one explicit policy RNG seed vector indexed by exact root
  and replicate. A numeric seed base is CLI shorthand only and is expanded
  before identity is computed.
- `BehaviorCandidateManifestV1` binds one published behavior manifest to the
  experiment manifest and its source accepted behavior. It contains no
  `accepted`, `better`, rank, or mutable current-model field.
- `BehaviorGateContractV1` binds one candidate/baseline pair to ordered paired
  exact-root cohorts, an independent held-out cohort, frozen mechanism
  sentinels, fixed-decision audit cases, completion bounds, and predeclared
  per-root and aggregate regression floors.
- `BehaviorGateResultV1` records `pass`, `fail`, or `incomparable`, the exact
  completed evidence identities, every triggered floor, and the candidate and
  baseline identities. Missing, censored, mismatched, or partially completed
  evidence is incomparable rather than a pass or failure.

An accepted-behavior registry is append-only and scoped by an explicit
competence domain such as character, ascension, semantic schema, and decision
scope. Its update consumes one passing gate result and one exact candidate
identity. The first implementation is read-only with respect to this registry:
it may produce a typed recommendation, but a human must explicitly perform the
accepted update. Training, evaluation, directory discovery, and checkpoint
publication cannot update it as a side effect.

Exact roots are referenced through small typed cohorts rather than copied into
a replay service. A cohort binds an opaque artifact digest, expected root
count, ordered slots, partition, and one declared role: training frontier,
rehearsal, independent held-out, or mechanism sentinel. Selection and canonical
merge remain the only artifact composition authorities. Cohorts never attach a
teacher action, scalar difficulty, or inferred continuation value. A training
consumer declares an exact per-root consumption budget; repeating a convenient
root or silently dropping an all-loss root cannot change the experiment
identity.

The behavior gate runs cheap, local evidence before broader evaluation:

1. Structural admission verifies every manifest, schema, scorer, checkpoint,
   root, seed-vector, and provenance identity. A schema or model migration also
   requires an explicit behavior-equivalence or declared-reset audit; matching
   tensor shapes alone is insufficient.
2. A fixed-decision policy audit scores the complete typed legal-candidate set
   without mutation. For both baseline and candidate it records raw logits,
   normalized probabilities, stable candidate ids, rank, top-two margin, and
   exact model/root/schema identity, then reports candidate-minus-baseline
   probability and rank deltas. Display strings are optional diagnostics and
   never candidate identity.
3. Mechanism sentinels run paired evaluations on small frozen exact-root suites
   for previously demonstrated capabilities such as Nob Enrage discipline,
   Sentries focus fire, or Lagavulin setup. Their gates are outcome/resource
   regression floors, not hand-authored action labels. The policy audit may
   explain a regression but does not silently become a teacher.
4. Paired comparison runs baseline and candidate from the same exact root and
   the same explicitly enumerated policy RNG seed for each replicate. It emits
   per-root differences before aggregates. A repeated root is a repeated
   measurement, not an independent encounter sample.
5. Only after the local floors survive does the gate run the declared
   independent cohort. Training roots and recovery suffixes cannot substitute
   for this cohort even when their aggregate is favorable.

Early stopping is contractual: stop immediately when a required regression
floor is irrecoverably exceeded, an identity mismatch appears, or the remaining
budget cannot make the comparison complete. Small root suites do not justify a
generic p-value or an SPRT claim. Statistical promotion rules may be added only
after the sampling unit, root clustering, minimum effect, error budgets, and
stopping rule are all explicit in the gate schema.

This control plane intentionally borrows role separation from Acme, typed
collection and consumption contracts from Reverb, candidate-to-accepted gating
from KataGo, paired and early-stopped comparisons from Stockfish Fishtest,
behavior-preserving migration discipline from OpenAI Five, and frozen coverage
sentinels from AlphaStar. It does not import Reverb's service, a distributed
worker fleet, a full self-play or opponent league, or large-scale training
infrastructure. The maintained single-machine path stays compact and
synchronous until measured throughput—not orchestration discomfort—requires a
different deployment.

The first implemented diagnostic slice is the fixed-decision policy audit. It
may replay one explicit typed-ordinal prefix, requires that prefix to finish at
an undecoded combat boundary, captures the resulting exact root identity, and
scores the one unchanged candidate surface under two frozen publications. The
audit emits content-identified typed candidates, raw logits, normalized
probabilities, ranks, and deltas; it chooses no audited action and consumes no
policy RNG. Paired exact-root comparison is also implemented for both greedy
and sampled policies: it runs both publications through the ordinary held-out
evaluator, verifies every root and replicate identity, preserves each resource
axis, emits per-root differences before aggregates, and makes no `better` or
acceptance claim. Sampled evaluation expands the declared seed base into one
explicit root-major root-by-replicate seed matrix. The scorer still receives
one ragged batch per model round, while each ready row samples only from the RNG
stream owned by its replicate slot; a terminated or divergent replicate cannot
advance another replicate's stream.

The remaining first control-plane slice is the read-only behavior gate and its
experiment/candidate manifests. It must not begin as a hand-authored JSON
ceremony around historical directories. Current combat training still owns one
sampled RNG stream per root, shared by that root's replicates, so it cannot
honestly emit the root-by-replicate seed matrix required by
`CombatExperimentManifestV1`. Training RNG ownership must migrate first; then
publication should emit the experiment and candidate manifests automatically,
and the gate may consume those identities together with fixed audits and paired
comparisons. Older candidates remain diagnostic evidence and must not be
retroactively guessed into a conforming manifest from filenames or partial
journals.

The same verified scorer may be evaluated over complete held-out runs because
combat training and run execution share the one bridge-owned semantic schema.
This zero-recovery diagnostic retains only terminal victory and floor-progress
aggregates. It does not claim that combat-only training taught route, reward,
shop, or other strategic decisions; those candidates remain part of the tested
policy surface and their weakness is part of the whole-run result.

The run-derived combat-root collector may derive one explicit scoped behavior
for curriculum construction. With one publication, typed public run context
selects greedy argmax for combat rows while strategic rows retain that source's
categorical rule and RNG. With a separately declared strategic publication and
combat anchor, strategic rows remain owned by the former while combat argmax is
scored only by the latter. The combined rule has its own manifest identity and
the V7 receipt binds both source manifest/checkpoint identities. It neither
relabels either publication nor treats untrained strategic argmax as competence.
Its purpose is to keep later-root collection on the same scoped policy surface
used by the combat-anchor-only whole-run diagnostic.

Whole-run training may use the strategic decision scope with an anchored
version of that mixed behavior. It imports the fully verified warm-start combat
scorer as an immutable combat anchor and separately copies its actor parameters
into the trainable strategic scorer. The mixed rule binds both the strategic
sampling rule and the exact combat-anchor manifest identity: combat rows use
anchor argmax with deterministic probability `1.0`, while strategic rows use
the trainable scorer and retain their exact categorical propensity. Combat
decisions remain in complete-attempt transition and return-to-go evidence, but
they do not enter the strategic actor/value loss.

Every promotion freezes only the updated strategic scorer. Its mixed manifest
changes while the combat-anchor manifest and checkpoint remain unchanged. A
durable run publication owns both scorers in its local immutable stores, and
its journal binds the anchor manifest, checkpoint, and scorer configuration.
Recovery must reconstruct and validate both scorers before the first choice,
then bind typed run progress from the restored environment. An unbound mixed
publication cannot silently guess whether a row is combat. The earlier
same-scorer mixed rule remains valid for bounded collector diagnostics, but it
is not the production whole-run strategic-training rule.

A fresh whole-run on-policy session may also copy that compatible frozen scorer
as generation zero. The copy shares no mutable parameters; the new run trainer
immediately binds its own objective and behavior manifest. The command records
the source combat manifest/checkpoint as provenance, trains only on the stable
training seed partition, publishes the final frozen behavior without claiming
an optimizer-resume boundary, and evaluates it on the disjoint held-out
partition with zero recovery. Full session resume remains stricter and rejects
open attempts at asynchronous slot boundaries.

Python recovery curricula may hold explicit opaque single-slot checkpoints.
Saving one clones that exact in-memory run-control state only when requested;
restoring it also restores any unfinished symbolic decoder or already selected
action and therefore does not reroll RNG. Explicit process-resume callers may
instead request one versioned, caller-byte-bounded snapshot of the complete
fixed batch. It stores each exact run-control session plus the candidate
ordinal prefix needed for bridge-local decoder state; fresh restore requires
the exact slot count and replays that prefix through the current typed decoder
before exposing any slot. It therefore serializes neither inference features
nor private draft/action objects, and a stale or malformed prefix fails closed.
The bridge uses no pickle, keeps no automatic history, and owns no retry,
retention, filesystem publication, or resurrection policy.
Vectorized callers use opaque checkpoint batches. Every target and replacement
boundary is validated before the first mutation, so a failed recovery batch
cannot partly restore the environment pool.
Each recovery checkpoint is bound to its source slot; ordinary recovery cannot
clone it into another slot and thereby detach accounting from environment
lineage. Any future cross-slot curriculum primitive must model that lineage
explicitly instead of weakening restore.

The separate `learning/` Python package is the online-training caller owner. It
may own seed scheduling, policy inference, optimizer state, curricula, and
evaluation accounting, but it may not reproduce simulator mechanics, inspect
opaque checkpoints, or define a second semantic feature dictionary. Recovery
and episode-reset accounting use caller-prepared tickets and commit only after
the bridge's atomic batch operation succeeds. The ledger retains only current
slot seeds, generations, and at most one compact pending terminal outcome per
slot;
trajectory history belongs in an explicit training artifact, not an unbounded
in-memory side channel. Held-out ledgers have a structurally zero recovery
budget.

The caller copies terminal bridge output into one typed step batch containing
only aligned slot, reward, act, floor, HP, max-HP, and gold integers. It rejects
missing, misaligned, duplicate, or out-of-pool rows before ledger mutation and
does not retain the bridge dictionary or observation tensors. A completed
episode attaches its final exact terminal row and recovery count; intermediate
attempt rows remain immediate caller evidence, not an implicit trajectory log.

Every ledger slot has an exact unsigned 64-bit episode seed from construction.
Recovery tickets bind both seed and generation and never change either. Reset
tickets validate and freeze replacement seeds before calling the environment;
only a successful atomic reset commits the new seed and next generation.
Snapshots, recovery events, and completed outcomes expose that lineage.
Recording one terminal step returns typed lineage for every attempt, including
defeats that will be recovered, so downstream training does not reconstruct
identity through a later mutable slot join.

Training and held-out seed partitions use one stable seed-only hash before any
recovery attempt or derived trajectory is created. The caller's seed schedule
is immutable; an advanced cursor becomes visible only after atomic environment
reset and ledger commit succeed. Reset failure therefore consumes neither a
ledger generation nor a seed, and ledger mode rejects the opposite partition.

The online batch driver creates its initial environment population, ledger,
next schedule cursor, and episode-root checkpoints from one seed plan. Policy
inference is called once per ragged decision round, never once per slot. Its
typed result contains aligned candidate ordinals plus the caller-owned SHA-256
identity of the exact behavior-policy manifest used for that call and one typed
selection probability per row. A policy records either the probability known
at selection time or explicit unknown; deterministic selection records `1.0`.
Neither probability nor manifest identity may be reconstructed later from
logits, display text, or a newer checkpoint. Naked ordinal lists and malformed
probability rows are rejected before environment mutation. After one atomic
environment step, a caller-owned curriculum chooses recovery slots
for the complete terminal batch; the driver restores that opaque checkpoint
subset together, completes the remaining defeats, and resets completed slots
from one next seed plan. Reset and creation of its replacement root checkpoints
are one bridge operation; ledger and schedule commit only after both succeed.
The checkpoint bank supports slot-keyed subset selection and replacement plus
an explicit versioned, caller-byte-bounded opaque serialization boundary for
process resume. Fresh restore requires the exact ordered source-slot identities
and reconstructs every private bridge state before exposing the bank. Its
format is distinct from the current-environment snapshot, and Python never
inspects a session. The driver retains one bounded step result or compact
aggregate statistics and is not a trajectory, replay, model, optimizer, or
shaped-reward owner.
Its terminal-target run executes only whole vector transitions and stops after
the first transition that reaches the requested attempt count or at its
explicit batch-step limit. One transition may therefore honestly exceed the
target when several slots terminate together. The typed result distinguishes
target completion from limit exhaustion; it does not implicitly flush
experience, train, or promote behavior.
Both fixed-step and terminal-target summaries count terminal victories and
defeats directly from typed attempt records, with their sum equal to terminal
attempts. Without retaining attempts, they also stream the terminal floor sum,
minimum, maximum, and sorted act counts. These are prefix outcomes and progress
evidence, not a shaped score or an automatic claim about policy quality.

The caller-owned held-out evaluator builds a fresh population only from an
explicit `HELD_OUT` seed schedule, uses zero recovery, installs no experience
buffer or trainer, and returns the schedule endpoints plus one terminal-target
result. It requires one typed behavior manifest identity, checks every policy
choice against that identity before execution, and binds the result to it; a
mutable controller cannot silently mix generations inside one evaluation.
Reusing the same schedule and the same policy RNG state therefore
repeats the same seed prefix without retaining trajectories. A budget-exhausted
prefix remains incomplete evidence; small victory/defeat counts are not
silently converted into a generalization or teacher-label claim.
Whole-run evaluation accepts either a completed combat-training publication or
a completed whole-run-training publication. The latter is classified by its
exact journal schema, then recovered through its durable manifest and
checkpoint stores with the recorded objective provenance. It is never
retrained merely to change held-out seeds. Corrupt or boundary-incomplete
journals fail closed rather than falling through to another artifact parser.
The caller-owned paired held-out evaluator receives one immutable schedule and
one evaluation bound for two distinct frozen behavior manifest identities. It
rejects a reused policy object or identity before environment creation and
delegates each side to the same manifest-locked single-policy evaluator with a
fresh population. The result retains both schedule endpoints, completion and
limit state, terminal counts, victories, defeats, terminal progress, and batch
steps. Its only derived values are fixed-direction `right - left` integer
differences, including terminal floor sum; it does
not emit `better`, `worse`, a win-rate improvement, or a teacher label. A pair
is comparable only when both sides complete the same terminal target. Either
side exhausting its batch-step bound remains explicitly incomparable. Policy
RNG equivalence is prepared by the caller and may be asserted before the call;
the evaluator does not introspect or serialize opaque policy state.
The publication-level paired-run adapter makes that preparation explicit. Its
full-behavior scope runs the ordinary one-slot evaluator twice with one
held-out schedule, initial policy RNG seed, ascension, potion action surface,
terminal target, and step bound. Before aligning terminal seeds, it requires
equal executed model definition/configuration, behavior-rule implementation and
configuration, and semantic schema identities plus distinct executed behavior
manifests. Its optional combat-anchor-only scope instead recovers one identical
strategic publication for both sides and attaches two distinct verified combat
anchors. Typed combat rows use the selected anchor greedily; route, reward,
shop, event, and other strategic rows remain sampled from the shared strategic
source. The two anchors must share the source model definition/configuration,
semantic schema, and categorical behavior rule. The resulting composite
behavior configurations are expected to differ only by anchor identity.

Both complete evaluations remain independently inspectable. The comparison
contains only typed per-seed outcome/resource axes and fixed-direction
arithmetic. Full-behavior RNG scope is the same initial stream per behavior;
once actions diverge, path-dependent random-draw consumption is not described
as stepwise common randomness. Combat-anchor-only scope starts both sides from
the same strategic RNG stream, while greedy combat selection consumes no policy
RNG. This isolates the scorer used on combat rows, not game-state consequences:
different combat outcomes can still expose different later strategic decisions
and therefore different later stream positions.

Optional online experience retention is one explicitly bounded segment, not a
driver history. Before policy inference, the caller recursively copies and
freezes the bridge's existing semantic decision batch without re-declaring its
feature schema. Rows are aligned to exact slot, seed, episode generation,
attempt index, recovery count, the subsequently selected candidate ordinal, and
its typed selection probability. Whole-run training additionally captures the
bridge's compact public seed, act, floor, and combat-boundary flag at that same pre-inference
boundary through an explicit optional provider. It does not recover those facts
from semantic feature arrays or inspect an opaque session. Every retained
decision batch also carries that exact behavior manifest identity. Lineage and
public progress are provenance for credit diagnostics, not semantic features,
teacher labels, or stored policy-score vectors. Explicit unknown remains
unknown through row selection, segment rotation, and complete-attempt assembly.
One complete attempt may then be projected into
`PublicAttemptTrajectoryV1`. Every chronological row requires the aligned Rust
public snapshot and typed public run progress, and retains its frozen semantic
payload, exact lineage, behavior-manifest identity, selected ordinal, and
selection probability. The progress and snapshot must agree on decision domain,
and the snapshot phase and ordered candidate count must agree with the frozen
semantic row.
Non-terminal rows carry raw environment reward `0`; only the final row carries
the bridge terminal reward and `terminated = true`. This owner computes no
floor shaping, return, advantage, value target, search target, or teacher label.
`SynchronousPolicyTrainer` is the sole routine adapter from a delivered
`CompletedAttemptExperience` to this immutable public trajectory, and performs
that conversion exactly once per delivery. Credit diagnostics, rollout target
projection, the first loss evaluation, and every later PPO epoch reuse those
same trajectories. Those consumers do not accept collection batches or inspect
opaque environment/session state.
Behavior manifests are caller-owned, content-addressed records over typed
SHA-256 identities for the external model checkpoint, model definition, model
configuration, behavior-rule implementation, behavior-rule configuration,
semantic schema, optimizer configuration, and trainer implementation, plus the
exact schema version and training step. A model checkpoint therefore cannot
claim the same behavior identity when logits are converted to actions by a
different rule. A fixed-size
registry resolves those identities without retaining model objects, checkpoint
payloads, file paths, or display strings. Unknown identities, conflicting
claimed identities, capacity overflow, and any expected checkpoint/config/schema
mismatch fail closed; the registry never evicts an older binding implicitly.
The online controller may explicitly replace its exact active row only after
the synchronous update chain has consumed all experience from that behavior.
This atomic live rotation retains no behavior history and therefore does not
make optimizer-step count consume durable-owner capacity.
The maintained categorical baseline derives those non-checkpoint identities
from one canonical machine encoding of the complete bridge-provided semantic
schema, typed scorer and Adam configuration, explicit device type, maintained
implementation versions, and the PyTorch runtime version. Mapping order cannot
change an identity, unsupported schema values fail closed, and Python does not
redeclare or interpret the bridge feature names while hashing them.
The manifest itself has a canonical versioned binary encoding. A separate
durable catalog stores that exact payload under its manifest SHA-256 with
mandatory count, per-manifest-byte, and total-byte limits. It shares the atomic
content-store kernel with checkpoints, rejects foreign or partial files, and
can hydrate a fresh in-memory registry as one all-or-nothing batch. The catalog
contains only small typed identities and counters, never model or optimizer
objects.
Optional PyTorch model checkpoints use a separate versioned tensor-only binary
format rather than pickle. Canonical state keys, explicit dtype and shape, and
little-endian tensor bytes determine the `MODEL_CHECKPOINT` SHA-256 identity.
The caller-owned file store has mandatory checkpoint-count, per-checkpoint-byte,
and total-byte limits and never evicts implicitly. It publishes a fully flushed
temporary file through an atomic same-directory link, verifies all files and
digests when reopened, and rejects partial, foreign, corrupt, or unsupported
entries. Restore decodes and validates every key, dtype, and shape into a newly
created model before that model can replace a live scorer; it does not load
pickle or partially overwrite the incumbent. A manifest template binds this
checkpoint to fixed model/config/schema/optimizer/trainer provenance and an
explicit training step.
Optimizer and explicitly injected categorical-generator state use a distinct
versioned, caller-byte-bounded tensor/scalar tree. It admits only finite scalar
values, strings, bytes, bounded containers, and supported dense tensor dtypes;
it admits no pickle or executable object. Optimizer hydration validates exact
parameter-group and parameter-id topology on a disposable fresh owner before
requiring canonical byte reproduction. Generator hydration validates its
device and uint8 state tensor and returns a fresh owner with the same next
sample. These payloads are resume components, never behavior manifests.
A live PyTorch behavior binding clones the optimizer-owned shadow scorer into a
fresh in-process model, freezes it, computes its canonical checkpoint and
manifest identities, and atomically rotates the active registry row without
writing files. The shadow model is never reused as live behavior, so later
training cannot silently change the policy. Durable publication is separate and
explicit: it re-encodes the active frozen scorer, requires the exact same
binding, previews checkpoint-store, catalog, and registry conflicts, then
commits checkpoint followed by manifest. A checkpoint file by itself is not
executable authority. After restart, recovery begins from only a manifest id
and fresh store/catalog/registry owners; it verifies and materializes the
checkpoint before hydrating the registry. A missing checkpoint therefore
cannot leave a partially executable registry row. Any live-promotion failure
leaves the incumbent policy and registry row unchanged.
Publication exposes two non-mutating previews with deliberately different
capacity semantics. Exact preview accepts an already stored identical
checkpoint/manifest/registry binding and is therefore suitable for retry.
Novel preview ignores identity deduplication and requires count and byte room
for one additional same-shape checkpoint, manifest, and registry row; it is a
conservative reservation for a model whose parameter values will change after
training. Neither preview writes a file or consumes a registry row, and its
typed preview summary is not accepted as publication or promotion authority.

A long-lived categorical behavior controller is the stable policy object held
by the online driver across generations. It accepts only strictly increasing
training steps, completes an exact in-memory frozen-scorer promotion, and only
then replaces its internal live policy. It does not publish merely because an
optimizer step completed. A failed promotion leaves the incumbent live policy,
active registry row, successful-promotion counter, and injected
selection-generator state unchanged. The controller retains only the active
binding, its training step, and a compact promotion count; explicit resume
publication first makes that binding durable, while restart recovery begins
from an inactive controller and the saved durable manifest identity.

Every buffer has mandatory decision-row and retained-payload-byte limits. The
byte accounting includes owned NumPy storage and Python payload metadata; the
row limit bounds lineage and choice metadata. A complete incoming batch either
fits or seals the preceding segment before admission, and a single oversized
batch fails before environment mutation.

Sealed segments identify every represented attempt as either carrying its
exact terminal attempt record or being censored at that segment boundary. A
continued attempt may therefore appear in a later segment under the same exact
lineage without the earlier fragment being relabeled. The driver delivers each
sealed prior segment synchronously to a caller-owned sink, applies the current
choice through the bridge, and only then commits that choice to the open
segment. It keeps no delivery queue; a sink failure stops before the choice,
while a rejected choice produces no experience row. Checkpoints, simulator
sessions, display text, JSON, policy scores, and inferred outcomes are not
experience payloads.

The Python caller may structurally select rows from a semantic decision batch
to avoid retaining unrelated environment slots. Selection copies and compacts
the requested token ranges and reindexes every token-bearing sparse table and
candidate boundary without interpreting numeric feature ids. It rejects
unknown bridge fields, cross-row relations, and malformed boundaries. Model
scores for a selected row must equal its scores in the original batch; this is
the boundary that permits later per-attempt memory accounting without a second
semantic dictionary or whole-batch retention.
The same structural algebra can concatenate independently retained decision
batches for training. It first applies the strict row validator, requires an
exact schema version and column dtype agreement, and then reindexes token
splits, candidate splits, categorical/scalar token references, relations, and
candidate tokens without interpreting feature ids. Repeated slot ids across
time are legal; cross-row relations, optional-mask disagreement, dtype overflow,
and malformed tables fail closed. Callers must supply row and input-array-byte
limits. Validation copies, output, and transient split arrays have an explicit
conservative additional-memory bound rather than an unbounded convenience API.

Complete attempt assembly remains a caller-owned synchronous segment sink. It
retains independently selected read-only decision rows under explicit maximum
open-attempt, decision-per-attempt, and payload-byte-per-attempt limits. A
terminal record closes only its exact seed/generation/attempt lineage. If an
attempt crosses either content limit, its arrays are released immediately and
the compact open marker is delivered as dropped at terminal; censored or
dropped fragments never become complete training samples. A segment may
contain both a terminal generation and its reset replacement, so the open
attempt limit applies after terminal closure. Downstream sink failure commits
neither sequence progress nor tentative assembler state.

The maintained on-policy path places one separate bounded update batcher after
attempt assembly. It retains an exact configured number of complete attempts
from one behavior manifest before delivering a single optimizer batch. The
attempt count, total decision count, and retained payload bytes are all hard
limits. A mixed-manifest attempt or batch, duplicate lineage, overfull terminal
delivery, or resource overflow fails before optimizer mutation. Dropped-only
rows pass through synchronously without entering the pending tensor batch. A
sink exception poisons the batcher and releases pending arrays because the
downstream optimizer may have mutated partially. Pending attempt payload is
live-only and is never admitted to durable resume. The batcher owns no durable
snapshot or lifetime counters; resume admission asks it only to prove that it is
empty and healthy.

An optional PyTorch candidate scorer lives only in the Python `learning/`
owner and is not imported by the ordinary package root. Construction consumes
one bridge `semantic_schema()` result and derives numeric embedding dimensions
and categorical offsets from it; Python does not retain a named game-feature
dictionary. Forward inference consumes the existing sparse semantic NumPy
tables and emits one flat logit tensor with the bridge's unchanged ragged
candidate row splits. The caller chooses CPU or CUDA by placing the model on
that device. The bridge, default caller dependency set, simulator workspace,
and experience format remain PyTorch-free.

The first stochastic adapter is a temperature-scaled ragged categorical rule
bound into the behavior manifest independently of model weights. It validates
all rows and probability tensors before consuming randomness, then samples by
inverse CDF from one explicitly injected `torch.Generator` on the scorer's
device. It never uses the global generator. The resulting policy choice carries
the selected probability computed in that same call. The manifest identifies
the distribution rule and canonical temperature, not mutable RNG state; a
caller that needs exact continuation across process restart must separately own
and restore its random-stream state before resuming decisions.

The first terminal objective is an on-policy categorical policy loss, not
imitation or raw-logit value regression. It accepts only immutable public
attempt trajectories produced from bounded complete-attempt deliveries,
resolves every decision's exact behavior manifest,
and applies `-advantage * log P(selected | state)` to each sampled decision.
The typed advantage mode either uses the raw terminal return, subtracts the
mean terminal return of every other attempt, or uses one of the explicit
matched ablations described below. Every centered mode requires at least two
independent attempts. Global leave-one-out preserves the on-policy expectation while
removing the batch's common return level; it does not add a learned value model
or shaped reward. The maintained floor-progress return is an explicit,
replaceable training-target projection over neutral trajectory facts. It
reserves `+1` for
victory and maps defeat floor
`f` to `-1 + 2 * min(f, target_floor - 1) / target_floor`. Deeper failed runs
therefore carry ordered progress evidence but never tie a victory. Negative
returns decrease the sampled action's relative probability and positive returns
increase it. A forced single-candidate row has exactly zero policy gradient;
under value PPO its critic row still trains. Terms are averaged within each
attempt before attempts are averaged, so longer attempts do not gain
more weight merely by containing more decisions. Censored and dropped attempts
cannot enter through its input type. The objective requires the manifest's
categorical rule to match its typed configuration and recomputes every recorded
selection propensity from the current shadow scorer before mutation. Unknown
or mismatched propensity is explicitly off-policy and rejected. Any future
off-policy correction requires a separate objective with declared assumptions.
The whole-run PPO-clip-value rule adds a scalar critic over the same decision
state and requires the explicit decision-local GAE advantage mode. Public
trajectory decisions are chronological environment steps; decision floors and
acts cannot move backward, and the terminal record must match the exact lineage.
Floor advancement is an additive
transition reward of `2 * delta_floor / target_floor`, capped at
`target_floor - 1`. Defeat adds `-1` on the final transition. Victory adds the
exact terminal adjustment that makes the complete reward sum remain the
historical reserved `+1`. Progress before the first retained decision is kept
as a separate non-trainable prefix reward, so it is conserved without being
credited to a later action. Reverse return/GAE calculation follows the
Stable-Baselines3 recurrence under the deliberately fixed `gamma = 1` and
`lambda = 1` profile: decision count is not treated as elapsed game time, no
complete attempt is bootstrapped, and each value target is the exact
Monte-Carlo continuation return. Value rows retain equal total attempt weight;
actor weights are renormalized only across multi-candidate rows inside each
attempt, so a forced action contributes neither actor loss, entropy, KL, nor
advantage normalization weight. The critic fits the unnormalized decision-local
return-to-go under equal total attempt weight. Actor advantages, behavior
probabilities, and pre-update value predictions remain frozen across bounded
PPO epochs. Actor probability ratios and value changes are clipped separately;
target KL may stop later epochs, and diagnostics include actor/value clip
fractions plus attempt-weighted explained variance. The diagnostics preserve
the eligible actor residual both before and after configured advantage
normalization and count every normalization-induced sign change, including
positive-from-nonpositive and negative-from-nonnegative directions. This keeps
environment return, critic residual, and optimizer signal distinct without
declaring any one sign change correct or incorrect. The V3 whole-run
publication and trainer identity bind this return, GAE, actor-mask, and
value-clipping contract. Earlier V2 publications do not bind that contract;
in particular, their value PPO optimized a terminal-broadcast target. They are
deliberately not recovered as the new algorithm.
For whole-run public trajectories, the trainer also emits
a bounded non-authoritative comparison against a remaining-horizon target. A
defeat observed from decision floor `d` maps its later terminal floor `f` to
`-1 + 2 * (min(f, target_floor - 1) - min(d, target_floor - 1)) /
(target_floor - min(d, target_floor - 1))`; victory remains `+1`. The comparison
reports only counts, signs, ranges, means, and decision-floor groups. It does
not change the decision-local PPO loss, add HP or potion prices, or survive as
experience payload. The same diagnostic also reports a matched-floor
leave-one-out advantage: each attempt's remaining-progress target is centered
only against other independent attempts that reached that decision floor. A
floor reached by only one attempt has zero comparison signal. Selecting the
provenance-bound matched-floor advantage modes remain explicit REINFORCE
ablations; value PPO accepts only decision-local GAE. This avoids
calling an unmatched state better or worse merely because it occurred later in
the run, but remains an explicit ablation rather than an assumed improvement.
Episode-root retry diagnostics additionally retain one bounded row per exact
episode seed and generation: retry count, terminal-floor span, and matched
episode/floor/context signal distributions for all and strategic decisions.
These rows expose root learning potential for sampler experiments; they do not
select, replay, or weight roots by themselves.
Combat and strategic decision rows are additionally counted as separate typed
scopes so a whole-run update cannot hide which surface dominates its credit
mass. The provenance-bound decision scope either trains every retained row or
selects only strategic rows and renormalizes each attempt across those rows.
Strategic-only rejects attempts without a strategic decision and never silently
turns combat rows into strategic rows. `All` remains the maintained default;
strategic-only is an interference ablation, not an assumed improvement.
Decision-time progress also carries the bridge's typed strategic context kind
for Map, CardReward, Event, Shop, Reward, Campfire, BossRelic, RunChoice, or
Treasure boundaries. Diagnostics report strategic-scope attempt-equal weight and a
second leave-one-out comparison matched by both floor and context. The explicit
provenance-bound floor-plus-context mode can apply that tighter comparison to
the loss as a separate ablation. It removes floor-only contrasts between unlike
combat/strategic sites and leaves unsupported groups explicitly at zero; it is
not an assumed improvement.
For the explicit paired-root sampling ablation, a third matcher additionally
binds the episode seed and generation. Only retries of the same exact episode
root may then baseline one another at a matching floor and typed context;
another seed is unsupported even when every visible progress field agrees.
One typed objective configuration owns terminal-return semantics, advantage
mode, and the number of attempts per update. The trainer implementation
artifact binds the return kind, target floor, advantage mode, decision scope, and attempts per
update; runner construction and
process restore reject any conflicting runtime config.
After validation, all retained decision payloads in one delivery are combined
into one semantic ragged batch and scored by exactly one model call. Flat row
weights are `1 / (attempt_count * decisions_in_that_attempt)`, which is
mathematically the same attempt-equal objective without one tiny forward per
historical decision. The trainer must provide explicit semantic concat row and
array-byte limits; vectorization does not weaken the existing memory bound.
A synchronous optional trainer serves behind the update batcher. A non-empty
training delivery must contain exactly the configured attempts per update and
performs exactly one optimizer step; dropped-only deliveries only update
accounting. The trainer keeps only aggregate counters plus the most recent
bounded manifest-id and selection-probability sequences, and never queues
attempts or tensor payloads. Unknown
manifest identity, incompatible behavior rule, unknown propensity, or a
propensity that does not match the shadow scorer fails before optimizer
mutation; an exception during backward or optimizer mutation poisons the
trainer so partially mutable state cannot be retried as if it were clean.
Dropped-only deliveries update accounting but never the model. The trainer owns
a shadow policy model; an exact frozen live binding is created before that
model can become behavior, while durable publication remains explicit.

The optional bounded categorical generation runner composes one exact
driver-to-assembler-to-update-batcher-to-trainer-to-controller chain without
becoming another experience store. It verifies object identity and counter
agreement for that chain, the shared
manifest registry, the shadow scorer, and every optimizer parameter before any
environment mutation. Each generation is exactly one optimizer step beyond the
active behavior's training step, not a terminal count or wall-clock guess;
larger step counts are rejected because a second update against experience from
the unchanged frozen behavior would be off-policy. A call advances at most its
declared batch-step limit and explicitly flushes experience only after a
terminal batch. Completed slots park at their terminal boundary until every
environment slot in that cohort has completed. Attempts per update must contain
an exact number of whole cohorts. Cohort generations require zero recovery and
fail closed if an attempt exceeds its retention bounds. Intermediate cohorts refill together under
the unchanged behavior; the final cohort remains parked until the optimizer
step and live promotion both complete, then refills together under the new
behavior. A fast slot therefore cannot start an old-behavior episode and finish
it after promotion under a new manifest. Reaching the one-step target freezes
and promotes the current shadow scorer exactly once without durable I/O. The result retains only
aggregate progress and the optional live binding. The runner itself remains an
in-process composition; exact restart is owned by the separate explicit
six-component resume boundary and can never be inferred from a model checkpoint
alone.
The separate paired-root generation mode is deliberately single-slot. After a
defeat it restores the unchanged episode-root checkpoint, preserving simulator
RNG while the stochastic behavior stream produces another trajectory. A typed
per-episode attempt cap bounds each root so one update can cover multiple
independent roots; victory completes a root early. At the exact attempt-update
boundary the curriculum completes the current defeat instead of restoring it,
so the environment is terminal before optimizer mutation and no episode
crosses promotion. The recovery budget is exactly one less than the per-root
attempt cap, episode-matched credit is mandatory, and held-out evaluation
remains structurally zero-recovery. Generation evidence reports both distinct
sampled episodes and recovery count.
The process-resume admission boundary is deliberately strict. It accepts only
an environment between decisions with no terminal accounting in flight, a full
episode-root bank, an empty experience buffer, no open attempt-assembly state,
an empty healthy update batcher, matching segment sequence indices, a healthy
trainer, and an active behavior generation no newer than the shadow optimizer.
Any violation is a typed rejection; no owner silently drops state to
manufacture a resumable checkpoint.
At that strict boundary, one separate canonical metadata component preserves
the seed schedule, active ledger lineage, experience/assembler sequence and
aggregate counters, trainer counters and bounded last-evidence fields,
controller manifest/promotion state, and optimizer-step generation target.
The live-only update batcher has no encoded state: admission requires it to be
quiescent and restore creates a fresh empty owner from the objective config and
resource limits. Other fresh owners accept only the corresponding typed
snapshots. This metadata has no simulator session, model tensor, optimizer
tensor, generator tensor, or experience payload; those remain distinct
components bound by the final resume manifest.
The resume store owns exactly six immutable component kinds: current
environment, episode-root bank, shadow model, optimizer, categorical generator,
and generation metadata. One publication batch-previews aggregate distinct
component capacity, writes every component through the shared atomic
content-store kernel, and publishes one small canonical manifest last. A
manifest binds every kind exactly once by digest and stored byte count. Reopen
and resolve revalidate every envelope, kind, digest, and size; component files
without a manifest are inert, while a manifest with any unavailable component
is not resumable. The live categorical publisher first makes the exact active
behavior binding durable, then captures all six resume components from one
admitted runner boundary rather than accepting caller-assembled identities.
The categorical restorer resolves that manifest before constructing owners,
materializes a fresh environment and episode-root bank through the bridge,
hydrates a fresh shadow scorer, exact optimizer topology, and categorical
generator, then recovers the frozen active behavior through the durable
behavior catalog. It rebuilds the ledger, schedule, empty experience buffer,
attempt assembler, empty update batcher, trainer, controller, driver, and
generation runner from the
typed metadata. The complete fresh runner is exposed only after its own strict
resume boundary exactly reproduces the saved boundary; a missing component,
incompatible runtime factory, foreign slot identity, or partial owner graph
therefore fails closed.
The compact categorical session factory is the sole maintained assembly path
for this baseline. One typed bridge binding, training-partition configuration,
algorithm profile, resource limits, curriculum, and experiment root create
either generation zero or a restored runner. `advance_generation` performs no
durable I/O. Its bounded `advance_generations` composition stops at the first
incomplete generation and retains only aggregate counts, not per-generation
results or attempts. Callers choose checkpoint cadence explicitly through
`publish`. Pending update batches remain live-only and fail resume admission.
Restore additionally requires the saved
slot count, seed-partition rule, and recovery budget to match the session
configuration. The first maintained profile is CPU-only, collects eight
same-behavior complete attempts per optimizer update, requires at least one
relation layer so candidate-target edges can affect logits, and does not
silently select another device.

On an ordinary reward screen, the reward owner claims typed low-agency public
resources before opening a nested card-reward choice. This lets the card owner
observe already-claimed gold, non-conflicting relics, and empty-slot potions
instead of evaluating the card surface against a stale pre-reward run state.

The current route owner has one deliberately narrow elite-growth prior: on
ascension 0 during Act 1, a direct elite arrival may enter the typed
`EliteGrowth` band only while current HP is at least three quarters of maximum
HP, recent durable combat attrition is not high when that fact is available,
and the visible Slime Boss plan is not exposed or potion-backed. An unresolved
or potion-backed Slime Boss plan also makes same-band route comparisons protect
campfire scope before optional elite count. Recovery and funded-liquidity bands
still take precedence. This is a public-state behavior prior, not a claim that
an elite is safe, a hidden-future lookup, or a substitute for measuring the
resulting continuation.

Within one route band, guaranteed campfire scope is strategic evidence and
precedes raw observed-path cardinality after elite and optional campfire scope
have tied. A route having more enumerated continuations is a representation
fact; it must not by itself outrank another route that guarantees an additional
campfire.

Slime Boss preparation is a shared typed strategy fact. It records capability
coverage, static attack inventory, strong-AoE and burst-finisher sources, exact
damage-potion identities, and whether two distinct potions can cover the opening
and post-split stages. One light AoE source is presence, not an established
post-split plan; the plan requires multiple AoE sources, one typed strong-AoE
source, or one typed burst finisher. Static attack inventory is not presented as
an exact three-turn simulation. `PotionBacked` means that continuation resources
are carrying an unresolved deck plan; it is not equivalent to `Established` and
is not permission to spend those resources on an avoidable elite.

Card-taking owners share the strategy-layer acquisition contract. A shop
adapter must pass the currently available purge price as an explicit
opportunity cost; it must not assume that a purge is still available after it
has already been used. A shared `NoPolicySupport` result is absence of shared
support, so a scene may retain a purchase only when it has stronger typed
durable-asset evidence such as efficient access, persistent required-capability
improvement, package support, or new upgrade scope. A purchase that consumes a
live purge reserve for only scene-local tactical gap coverage remains
speculative. After a visit has already spent gold, another card or potion with
neither shared acquisition support nor durable-asset evidence also remains
speculative; affordability by itself is not a bundle plan. Random card
discovery potions are not existing-deck access and cannot claim draw/energy
consistency coverage from that discovery trait alone.

Conditional relic value must expose its activation requirement separately from
its effect trait. Orange Pellets therefore requires a payable same-turn Attack,
Skill, and Power sequence before DebuffControl can establish a current strategic
asset. A one-card owned upgrade that makes the sequence payable remains typed as
an upgrade path, not as current activation and not as permission to spend gold.
The feasibility check consumes card types, actual persistent card costs,
unconditional card energy gain, and permanent energy-relic supply; it does not
infer activation from the relic label or from merely owning all three types.

Relic-sensitive card value also stays typed and public-state-only. Mummified
Hand power tempo records the candidate Power's paid base cost and the number of
owned positive-cost cards that can receive its trigger. Reward and upgrade
owners may consume that fact; they must not infer the relic interaction from a
display label or promote every Power without its package and boss context.

Card-reward ordering preserves policy bands and exact threat-gap closure before
using narrower tie-breaks. Within the same band and after those gap comparisons,
a typed Boss damage-plan improvement takes precedence over the raw count of
generic capability improvements. This tie-break may distinguish an established
long-fight engine from several coarse capability promotions, but it must not
promote a candidate across a stronger band or a real threat-gap closure.

Upgrade redundancy is measured across cards that provide the same capability,
not only duplicate card ids. Weak and Vulnerable duration providers therefore
share coverage groups. Upgrade roles must describe what the upgrade changes:
a card that already draws but whose upgrade changes only Block does not pay an
access-recovery debt.

An exhaust-payoff Power's cost reduction pays setup-tempo debt only when the
owned deck already has a typed exhaust executor. Mummified Hand keeps its
separate Power-tempo contract and suppresses this additional debt so the same
tempo support is not counted twice. This is upgrade evidence for an existing
engine, not acquisition support for an otherwise speculative Power.

Card-reward gap closure remains subject to typed marginal-quality gates. A
cost-two-or-more tactical candidate whose only semantic roles are frontload,
Weak, and Vulnerable stays speculative when one of its one-turn debuffs
duplicates an owned source of the same debuff. The gate is duration- and
upgrade-sensitive, and its typed audit fact must remain separate from the
underlying capability delta. Multiple light AoE sources establish availability
but do not become strong multi-target control merely by crossing a source-count
threshold. Exact duplicates whose only additional value is that light AoE
tactical bundle are likewise low-marginal. The first light AoE source remains
eligible to close a real gap, and intrinsically strong AoE remains a distinct
typed capability.

A candidate that injects a persistent Status directly into the draw pile also
carries a typed handling assessment. Evolve draw recovery and Medical Kit's
unrestricted Status exhaust count as covered; hand-exhaust cards and
Fire Breathing count only as conditional because the required pieces still
have to meet in combat. Exhaust payoffs are supporting evidence, not handling
by themselves. Conditional or unsupported persistent draw-pile pollution may
not use a coarse capability delta to rise above the speculative band. Ethereal
Statuses and injections into other zones remain outside this gate.

## Runner And Combat

The runner owns run progression:

- selecting or applying non-combat owner decisions,
- deciding when combat search is allowed,
- setting search budgets and potion policy,
- applying an exact returned combat line,
- saving run capsules, frontier checkpoints, and `CombatCase` artifacts.

Combat search owns only the in-combat problem:

- legal combat action enumeration,
- action ordering, typed state guidance, and exact search policy,
- exact execution of candidate combat lines,
- combat outcome facts and diagnostics.

Combat search must not decide rewards, shops, events, campfires, routes, branch
retention, or deck-building causes. A combat result can expose a symptom; it is
not by itself a deck-construction verdict.

`CombatCase` is the preferred handoff from runner to combat investigation. If a
branch-tiny combat gap cannot be investigated from a saved case, fix the case
payload or the review entrypoint instead of creating another report format.

The serialized case remains one flat artifact, but its Rust ownership is
layered. `CombatCaseCoreV1` contains the exact position, typed source and gap,
and the derived run/combat/RNG facts that can be consumed without run-control.
Run-control adds diagnostic history and an optional production context around
that core. Production-context capture and restoration receive the core and
context explicitly; the context owner must not depend back on the full case or
on analysis-session/explorer queues. The immutable owner combat-budget payload
is a separate data contract from the run explorer methods that schedule it.

A case without a validated typed `production_context` supports isolated combat
replay only. Exact production-state restoration requires that context to bind
the case's exact combat-root hash, normalized run-session fingerprint, and
run-control checkpoint; all three identities must validate before use. Exact
production-owner parity additionally requires a typed owner-policy snapshot;
defaults or caller-supplied guesses do not qualify. A counterfactual or
descendant case must clear this context when its position changes. Display
paths, branch ids, filenames, and synthetic run projection do not upgrade a
case to production parity.

New production case producers leave the legacy display `path` empty. Exact run
identity comes from the validated context, while committed decision history
remains owned by the journal/workspace. Legacy paths stay readable for old
diagnostics but candidate display text is not copied into new case payloads.

Combat evidence manifests embed the case's typed replay identity rather than
repeating an unbound root string. New manifests resolve case, action, and trace
artifacts only relative to the manifest file. Legacy manifests may use their
compatibility resolver, but undeclared files are never promoted by same-stem or
single-case-directory inference.

Potions are run resources. Combat may consider potion actions only when the
runner explicitly opens a potion policy and budget. A diagnostic fact such as
"potion rescue found a win" does not automatically mean the main runner should
spend that potion.

Run-level potion realization and reward acquisition remain separate atomic
decisions. A reward owner may expose a mechanically valid out-of-combat potion
effect before a visible potion reward, but that conversion does not authorize
discarding another potion or invent a context-free ranking among retained and
offered identities.

Strategic search starts from an exact potion-free stage. Resident run search
carries that witness in its combat-work checkpoint; owner-audit staging carries
an opaque replay-adjudicated attempt in memory until the portfolio commits one
line. If the incumbent misses the configured run-quality satisfaction, the
runner may open bounded one-potion lanes across active potion identities. A
spending witness may replace the protected incumbent only by satisfying that
quality contract; a higher-HP result that still misses the target is not
permission to exchange continuation value for a marginal local improvement.
Within each standard strategic stage, an insufficient first win remains the
safe incumbent but does not end that stage; search stops when a new win reaches
the configured satisfaction or the already granted stage allowance ends.
Literal `FirstCompleteWin` survival search keeps first-win termination.
While that incumbent is insufficient, portfolio service stays with the exact
member that produced it until quality is reached or that member completes.
With no productive witness, non-Boss combat retains the ordinary local-graph /
discrepancy round-robin schedule. Each Boss stage instead serves the local
graph as its primary until that member completes or the stage allowance ends;
policy discrepancy remains a bounded fallback when local completion leaves
allowance without an acceptable witness. A later identity inheriting an already
satisfying witness receives one complete local challenge rather than a full
quality-polishing allowance.

Inside the local graph, one shared boundary agenda rotates among the anchor,
independent typed-proposal root and immediate-continuation queues, and the
available semantic guide lanes. An exact boundary whose typed proposal source
is applicable and a guide entry each select one exact boundary once. The first
proposal or guide service to reach a fresh boundary may pay
for one 128-work coherent grounding batch. Other views may agree on that same
shared state, but after grounding they resume it at the ordinary 4-work
preemption quantum rather than multiplying the coherent batch once per view.
Both proposal queues are FIFO. A newly applicable root cannot be starved by a
stream of continuations, and neither proposal queue replaces the anchor.
Proposal privilege is deduplicated for the exact state's lifetime, not merely
while it is pending. When a proposal materializes an exact non-terminal
successor, that successor receives at most one continuation opportunity; if it
was already pending as a proposal root, the same claim moves queues instead of
being duplicated. The next boundary is not inherited automatically. Repeated
and exhaustive service remains owned by the anchor. These values control
preemption granularity, not combat value or admission, and production and
laboratory hosts use the same planner constants. Full local-graph reports
retain the effective values in their search specification so budget-cliff
comparisons remain reconstructible.

The root's first complete-turn expansion is also allowance-aware. It receives
one eighth of the caller's generation work, with a 64-work floor for usable
allowances and a 2,048-work ceiling; an allowance below the floor is never
overdrawn. This keeps routine 4,096-work contracts from spending half their
budget repeatedly enumerating the root while deeper exact boundaries starve.
Production and V2 contract hosts call the same planner helper.

An encounter-owned typed service bias may give one existing guide lane a
bounded number of additional turns in that rotation. It does not duplicate
guide entries: each selected boundary remains one-shot, receives the same
quantum, and then leaves completeness to the anchor. The default planner has no
bias. The shared encounter owner concentrates the survival view for an
all-Darkling group because reincarnation loops make raw turn horizon unbounded.
Production and the compact V2 contract project the same typed bias; laboratory
controls may omit or reweight a lane for fixed-root attribution. The effective
production bias is retained in each stage trace.

During `ImproveVerifiedWin`, a quality-reaching spending witness remains an
exact candidate but cannot by itself end the refinement quantum. Early
satisfaction requires a clean potion-free witness at the same quality target.
When several clean witnesses reach that target, run control minimizes potion
use before applying the ordinary final-HP and persistent-payoff ordering.
`FindAnyWin` survival rescue keeps its separate first-reserve-compliant-win
contract: it may land below the search-quality target, but never below the
owner's captured strategic survival floor. Guaranteed full-heal boundaries
retain their explicit unlimited limit.
After every configured quality stage is exhausted, analysis advance may
materialize an insufficient-quality fallback only when it still preserves that
survival floor. A lower verified win remains an exact incumbent for diagnosis,
but autonomous run reports budget-unknown and does not silently accept it.
Autonomous refinement gives each active potion identity its own exact search
stage. The current stage's slot mask constrains newly generated or proposed
witnesses; it does not discard an exact-verified incumbent inherited from an
earlier identity stage. That incumbent remains checkpointed and must replay
exactly from the unchanged combat root before a stage promotion or process
restore can retain it. Non-Boss search divides the configured generation-work
allowance by the number of active identities, then gives one equal share to the
potion-free primary and to each concrete identity. Boss search additionally
keeps one final canonical multi-potion fallback and divides generation work by
`active identities + 1`; it reaches that fallback only when the clean and
single-identity stages found no acceptable witness. Thus a high-branching
potion cannot starve a simpler slot. Stage wall allowances include the clean
primary in their divisor (`active identities + 1` for non-Boss and
`active identities + 2` for Boss), so every configured stage receives time
without exceeding the caller's combat wall. Total bounded generation work is
at most `1 + 1 / active identities` allowances for non-Boss combat and
`1 + 1 / (active identities + 1)` allowances for Boss combat, while the
caller's combat wall deadline remains authoritative. Slot order follows
deterministic slot identity, not a potion value ranking.

Strategic advance and current-stage diagnosis are separate typed operations.
Strategic advance may stop at configured quality, promote an exact potion
identity, and materialize an accepted witness. A combat probe instead receives
one total generation-work cap, a portfolio preemption quantum, and one wall
deadline. It temporarily disables both portfolio-level and local-graph
satisfaction termination, serves only the already selected exact stage, and
retains its frontier and incumbent without promotion or materialization. Its
typed stop distinguishes work-budget completion, wall expiry, stage
exhaustion, and zero progress. Only a later explicit combat acceptance may
turn that retained incumbent into a run child.

Opening an exact identity stage changes the legal potion-slot contract, not the
shared action prior. Production does not globally raise every admitted root
potion action to the highest policy weight: that intervention can replace a
useful sparse corridor with a worse potion-first corridor. A laboratory audit
may opt into the old root challenge as an explicit A/B control, but its result
is non-authoritative until selected exact sentinels justify a narrower policy.

Semantic victory stages admit authorized potion use but omit explicit potion
discard. Discard remains simulator-legal and an all-legal diagnostic may opt in
for a concrete slot-generation or revive-priority case; it is not a generic
way to diversify a sparse combat search. Checkpointed incumbents containing a
discard remain ineligible when restored under a semantic stage.

Production combat progress carries the exact root hash and one compact typed
trace row per served stage. A row freezes that stage's slot contract, charged
local/discrepancy work, effective guide-service bias, proposal counts, the local
graph's best exact candidate, its satisfaction and typed portfolio disposition,
the selected incumbent, remaining allowance, and exit reason. Keeping both
candidates distinguishes "search did not find it" from "the run-level quality
gate rejected it."

A mechanically terminal victory in which a living Looter or Mugger still holds
stolen gold remains an executable witness, but its typed unrecovered-theft fact
cannot end strategic quality refinement. Remaining concrete potion identities
must receive their bounded stages before run control may settle for that loss.
The production local planner receives the same typed terminal constraint for
HP-quality searches. It retains partial-recovery victories on the terminal
frontier but only reports satisfaction when at least one frontier witness both
meets the HP target and has no unrecovered stolen gold; the local HP-selected
compatibility witness cannot hide a lower-HP qualifying outcome.
Within a potion stage, a fully recovered witness that satisfies the configured
HP quality target outranks a partial-recovery witness; an over-target full
recovery does not acquire that preference merely from encounter identity.
Completed rows survive workspace checkpoint restore. This trace is diagnostic
evidence for locating an owner scheduling divergence; it does not rank potions
or authorize a policy change.

Pre-A5 Act 1/2 room Boss victories deterministically restore full HP while
retaining the potion inventory. Owner-audit therefore searches those boundaries
without active potion expenditure first and lands any exact clean win; only a
missing no-potion witness opens the remaining high-stakes Boss rescue budget.
Other owner-audit Boss boundaries retain their canonical high-stakes search
contract. The autonomous resident runner uses the clean, single-identity, then
multi-potion fallback schedule above for every strategic Boss search.

Static potion identity tiers may describe audit evidence but do not admit
production spending. Passive death insurance and explicit escape remain
outside active victory search. When the runner opens concrete potion slots,
that exact mask is part of the combat-search engine profile and must be
enforced by atomic expansion, turn-plan enumeration, terminal-frontier
admission, and final executable-line replay; a trace-only slot mask is not an
admission contract.

Accepted combat lines must be exact executable lines from the current combat
state. Frontiers, near misses, heuristic samples, and dirty diagnostic lines
are evidence, not runnable campaign actions.

The complete-turn planner retains exact terminal witnesses on a typed
non-dominated frontier instead of collapsing every victory into its local
HP-first view. The retained facts include final and maximum HP, recoverable
gold (including pending `StolenGold` rewards), persistent card growth,
external burdens, potion expenditure, and action count. The planner's selected
`witness` remains a compatibility view for tactical progress; it is not the
run-level continuation decision. Run control owns adjudication across the
frontier with its existing satisfaction, potion-quality, and continuation
contracts. Diagnostic reports may expose compact terminal-outcome summaries,
but must not duplicate every retained action line.

An autonomous run wall stops additional search and owner scheduling; it does
not discard a verified incumbent returned by the final admitted search call.
Runtime first materializes that exact combat line as one atomic transaction,
records any wall overshoot, and then stops at the resulting boundary. A
terminal outcome already reached by that transaction takes precedence over a
run-wall stop classification.

### Combat Search Orchestration

Combat search code should keep these phases separate:

```text
portfolio context -> portfolio plan -> search profile -> search execution
                  -> acceptance -> trace/render/rejection
```

`branch_tiny` owns campaign-level portfolio orchestration. It should choose
which search profiles to run, execute them, and commit or reject results. It
must not reinterpret combat strategy hidden inside a lane name.

Combat search profiles are the boundary between orchestration and search
policy. A profile is an explicit bundle of:

- a budget,
- action-prior / phase-guard plugins,
- typed guide and frontier plugins,
- potion policy,
- acceptance policy,
- artifact policy.

Changing action ordering should usually add or modify an action-prior plugin.
Changing frontier scheduling should touch a frontier plugin. Changing what
counts as an acceptable result should touch acceptance. Runner code should only
run profiles and apply typed outcomes.

A typed encounter plan may expose categorical action timing to one bounded,
plan-compatible exact prefix proposal. Strategy owns the timing semantics; the
planner marks exact boundaries where the source is applicable before their
generator runs, materializes the prefix as ordinary graph edges, and retains
typed proposal provenance on those edges. Each root-eligible boundary receives
one deduplicated proposal-root service opportunity while remaining in the
anchor queue for ordinary completeness. Anchor, guide, and ordinary generator
service never execute proposals; proposal-root service admits only
root-eligible stages, while inherited proposal-continuation service may admit
the continuation-only stage at that exact boundary. A continuation-only
proposal cannot enter the global proposal-root queue. A materialized proposal's
immediate exact successor receives one opportunity in the independent
continuation queue; later continuation-only stages must materialize another
typed proposal to carry that lineage onward.
Run-control only gates the proposal, charges its actual work, and reports root
and continuation enqueue, generation, and service counts.
Proposal-root and proposal-continuation service stop after the applicable
prefix is completed or rejected; unused allowance is not spent on ordinary
enumeration in that privileged view. The same node remains available to the
anchor for later completeness work.
The proposal never prunes alternatives, claims a win, recursively services its
descendants, or becomes another global frontier scheduler. Acceptance still
requires the ordinary exact witness and replay contract. Production admission
is explicit and evidence-backed per encounter plan; merely having categorical
action timing does not opt a plan in.

The double-thief encounter owns two narrow current-turn proposals. One splits
two Strikes around a playable Power Through bridge. The continuation may
instead play Shrug It Off, take an exact one-Strike lethal on a thief carrying
stolen gold, and convert already-held non-attack fuel with Second Wind only
when the resulting conservative block covers the other thief's visible
attack. The check does not assume that Shrug draws useful fuel. These proposals
encode encounter timing and exact resource composition, not a global card
preference or a hard-coded turn-plan index. Once only one thief carrying stolen
gold remains, its visible Smoke Bomb or Escape turn admits a bounded pressure
prefix from two one-cost Strikes, applying a playable one-cost Thunderclap
first when present. If the combat ends before the prefix's trailing End Turn,
the exact terminal boundary completes the proposal without fabricating that
now-illegal input. Only the first bridge is root-eligible. The exhaust-block,
Smoke Bomb, and Escape stages are continuation-only, so they extend an existing
typed lineage without broadcasting proposal-root claims across every
lookalike state.

Maintained production and V2 contract execution do not install an external
complete-line or bounded-lookahead witness producer. Terminal evidence must be
reached through the exact graph, retained on its typed non-dominated frontier,
and replayed from the unchanged root. Restored checkpoints may carry an
already verified incumbent, but restoring one does not mint work or bypass the
current potion and terminal contracts.

`OracleRunCombatWorkCheckpointV1` is the small durable boundary shared by
analysis jobs and the run explorer. It retains bounded allowance, continuity
accounting, the exact potion contract, and at most one replay-exact incumbent.
It never serializes a local-graph/discrepancy session, portfolio queue, planner
snapshot, or analysis/explorer scheduling state. The live combat-work owner
captures and restores this contract; restore rebuilds the tactical frontier
from the enclosing branch's exact root and replays every incumbent action,
including successor and terminal-position verification, before admitting it.

Live orchestration holds `OracleResidentCombatJobV1`, an opaque capability over
the tactical work owner. Analysis, scratch search, and the run explorer may
start or restore a job, grant bounded work, request typed evidence, checkpoint
it, or atomically finish a verified result. They cannot name the underlying
`OracleRunCombatWorkV1` or access local-graph/discrepancy sessions and queues;
that implementation type is confined to its owner and this facade.
`OracleResidentCombatJobEvidenceV1` is a separate owned snapshot containing
only accounting, queue counts, typed progress snapshots, and incumbent/candidate
facts. Status labels are owned data, and the evidence carries no live session,
frontier entry, borrowed implementation state, or mutation authority.
Explicit analysis edits enter the run explorer through
`OracleRunExplorerExplicitTransactionsV1`. The capability atomically commits a
typed decision identity, a verified combat job, or a currently legal Smoke
Bomb escape and returns only child identity, display label, and HP facts.
Prepared branches, decision-supply registration, selection-family release,
identity indexes, and commit ordering remain private to the explorer owner.
The resident combat-job facade, its owned evidence, budget projection, explorer
checkpoint, and this explicit transaction capability are the complete lower
surface used by the runtime-owned analysis session. Moving that session out of
run-control does not make live planner sessions or explorer registries public.

## Gap Semantics

Gaps are typed stops, not verdicts:

- `automation_gap`: a non-combat owner boundary has no bounded answer.
- `combat_gap`: current runner/search settings did not produce an acceptable
  executable combat line.
- `budget_gap`: configured wall-clock or slice budget ended.
- `potion_rescue`: diagnostic or retry path found a potion-assisted line.
- `still_no_win_after_review`: review settings still found no accepted line.

None of these proves why the run is bad. The next investigation step must be
explicit: search policy, potion gate, reward/shop choices, deck facts, or owner
coverage.

## Campaign Artifacts

Campaign artifacts are storage and replay surfaces, not strategy authority.
Keep these responsibilities separate:

```text
checkpoint  exact simulator state needed to resume execution
state       scheduler/workset state needed to continue a campaign
journal     append-only decision facts and candidate pools
report      bounded projection for inspection and tools
diagnostic  opt-in sidecar data for large explanations and traces
```

Checkpoint owns exact resume state. State owns scheduling data. Journal owns
decision facts and candidate identity. Report is a cheap projection.
Diagnostics are opt-in sidecars for large or narrow-use explanations.

Capsule campaign history is an immutable `RunTrajectorySegmentV1` DAG. Each
segment contains one ordered `RunProgressJournalV1` plus planner-boundary visit
occurrences; large observations and legal-candidate sets live once in
content-addressed payload tables. Branch checkpoints persist only a verified
trajectory head id and depth. Every pending segment must be committed before a
frontier, cutpoint, terminal result, or soft-pause checkpoint can be written.

Behavior events and raw-horizon outcomes are read-only projections rebuilt by
walking a durable head to its root. A resumable or prematurely stopped head
produces typed censored outcomes, never a fabricated defeat. These events are
behavior-policy evidence, not teacher labels.

`SessionTraceV1` remains available for interactive trace consumers, but it is
not capsule campaign-history authority. The optional `trace.jsonl` output may
render durable head references and can truncate without losing capsule
evidence. Result, summary, coverage, behavior, and outcome files are
rebuildable projections and must not become a second decision-history owner.

Default reports should reference state, journal, checkpoint, and diagnostics
instead of inlining large payloads. Compression is not a license to store
unbounded data.

Routine fixed-root combat experiments use the breaking V2 contract artifact:
an atomically published directory containing one small stable `manifest.json`,
one opaque full `report.json`, and one replay-exact action sidecar per retained
non-dominated terminal candidate. The manifest owns the typed request, exact
root id, source-content identity, compact classification, compact per-depth
exact-search service accounting, terminal exact identities, candidate roles,
and paths to sidecars plus one compact exact-state service index. `artifact
search --state` distinguishes a state that was never retained from one whose
complete-turn generator was retained but never serviced, and reports its
current anchor, proposal-root, and proposal-continuation queue positions and
service counts, plus per-boundary proposal applicability, attempt outcome,
exact proposal-successor identities, and generator anchor/guide service
attribution separately from the shared boundary service source that actually
consumed work, without parsing the opaque report. `artifact summary`,
`artifact summaries`, `artifact search`, and `artifact rerun` read only
manifests; the plural summary owns typed multi-artifact collection instead of
delegating JSON aggregation to a shell. `artifact trace` replays the
contract-aligned candidate and defaults to compact action identity and rank
facts plus turn checkpoints. A checkpoint-only projection omits action policy
payloads entirely; complete per-action probability and choice payloads require
an explicit diagnostic opt-in.
`artifact compare` replays it alongside the local-HP candidate and locates their
first exact divergence. `artifact turn` resolves either candidate by semantic
role, inspects one exact complete-turn surface, and may follow displayed plan
indices or exact successor identities through replay-checked exact successors
without exposing sidecar paths, scratch ids, or intermediate case files. When
the caller already knows one successor identity, the command filters that
surface before display rather than requiring a large sibling dump and external
JSON parsing. Navigation may also return only the replay-checked reached state
without enumerating its next complete-turn surface. `artifact branch` uses that
same typed navigation owner to select
one bounded diagnostic prefix, then runs the inherited contract from its exact
successor without writing a descendant case or creating another report
protocol. The request stores the bounded public inputs, their expected search
root identity, and source candidate provenance. Suffix witnesses are
concatenated with the prefix and replayed from the unchanged original root
before contract classification or persistence. Prefix potion expenditures
reduce the suffix allowance, while generation accounting remains scoped to the
fresh suffix search. The manifest distinguishes original root identity from
search-root identity so `trace` remains full-root evidence and state service
queries remain honest about the graph actually searched. This is
counterfactual diagnostic evidence and never restores production-context
parity or admits the chosen prefix into policy. An explicit diagnostic may enumerate one additional exact
complete turn from every selected parent and aggregate terminal HP and
stolen-gold facts; it does not recurse, propose policy, or become a second
witness search. None of these commands may make callers recover paths or
restate constraints. They must not guess fields in the full report or earlier
artifact schemas. Combat cases enter the V2 catalog
explicitly, keyed by exact root identity, so routine discovery never depends on
filenames or recursive shell scans.

An oracle analysis workspace is an editable variation workbench, not the
archive authority for every state it has ever materialized. Explorer
checkpoints externalize map graphs, map state, decks, relics, potions, and run
schedules into typed content-addressed payload tables. Replay steps and emitted
events use shared prefix DAGs. Payload hashes declare their algorithm and are
validated during hydration; legacy inline checkpoints remain readable.

The live workbench, durable workspace envelope, filesystem store, and selected-
branch recovery projection are separate runtime owners. The envelope contains
only configuration plus the typed analysis-session checkpoint. The store owns
atomic JSON reads/writes and timing; recovery consumes that same typed loader
without restoring unrelated branches. Live workspace methods own construction,
navigation, and mutation and do not parse or write their own artifact.

One workspace may carry one combat line-lab DAG bound to an immutable run combat
node and its exact root hash. The root position remains owned by that run node;
each child persists only its parent id, one typed `ClientInput`, and the exact
successor hash. Restore topologically replays every delta from the bound root and
rejects missing parents, cycles, illegal inputs, transition truncation, or hash
drift. The persisted checkpoint retains the legacy `combat_scratch` field name
for workspace compatibility, but the line lab is the behavioral owner.

A line lab has one typed baseline. A root baseline contains no actions; a
resident-incumbent baseline imports the complete replay-verified incumbent into
the same DAG and records every immutable prefix. Normal callers navigate that
baseline or the active current line by observed turn and zero-based action
ordinal, and rewind the current line by one action, never by internal DAG node
id. Navigation defaults to the current line so a caller can work inside its new
branch without repeatedly naming the baseline. Typed card and potion ids resolve
automatically only when one current copy and, where required, one legal target
match. Duplicate copies and multiple legal targets return typed ambiguity
instead of guessing or mutating the line. The runtime resolves the selected
observation-local copy and target to exact internal identities before applying
the durable `ClientInput`.

One optional kept-line pointer protects reversible experimentation without
creating another line owner. Before semantic navigation leaves the first
non-baseline terminal win, the lab keeps that exact tip automatically; `keep`
may explicitly replace it with the current exact line, including an unresolved
one, and `restore` returns to it without exposing an internal node id. The kept
line is a caller-selected recovery point, not a claim that HP, potion use, or any
other combat-local ordering makes it strategically optimal. Its pointer is
validated and replayed with the rest of the checkpoint, while normal frames
expose only its typed line summary.

`open` establishes one complete decision frame. Ordinary semantic actions return
typed deltas without scratch node ids, while `observe` returns a full recovery
frame. A living monster retains its raw typed intent and separately exposes the
shared execution-semantic move preview: effective damage per hit, hit count,
total damage, and visible intent. The preview must use the simulator damage
pipeline rather than locally reconstructing Strength, Weak, Vulnerable, or
protocol overrides. `compare` owns the relationship between baseline and
current line: common prefix, first typed divergence, exact post-divergence
semantic actions, per-turn HP/block/enemy totals, potion expenditure, and
terminal truth. An unresolved current cursor reports an unknown suffix rather
than pretending the partial line lost. Bounded suffix search appends a
replay-verified complete win to the same DAG and returns its comparison; it does
not create another evidence owner.

The lower diagnostic scratch selectors remain temporarily callable during the
cutover for exact-hash and structured-selection work. They are compatibility
adapters over the same line-lab owner, receive no new workflow semantics, and
must be deleted once remaining semantic structured selections and callers have
moved. CLI JSON is compact by default; deltas and comparisons are response
projections, not persisted line history.
Resident execution and autosave timing belongs to the protocol response
envelope, not to the decision observation, delta, or navigation receipt. Typed
live commands print only the result projection; raw protocol callers retain the
timing metadata for performance diagnosis.
Line-lab navigation never creates run variations. A bounded
descendant search may append a replay-verified potion-free suffix to the lab,
but only an explicit terminal-victory commit may materialize the complete root
prefix as one atomic combat witness and clear the line lab. Run journals do not
record individual line-lab card actions.

A fresh active workspace may also be rebuilt from one exact committed node
after journal and final-state fingerprints are verified; the historical
workspace remains immutable evidence. This bounds the active edit loop without
pretending that a one-node continuation preserves the discarded variation DAG
or resident combat-search frontier. Repacking only changes storage and retains
the complete variation DAG; compaction deliberately starts a new one-node
workbench. Neither operation overwrites its source.

## Journal And Candidate Identity

Decision history belongs in the journal. It records:

- the decision boundary,
- branch and checkpoint context,
- available candidates,
- stable candidate ids and typed summaries,
- candidate admission and disposition,
- selected or applied candidates when a policy chose one.

Every decision needs a stable `decision_id`. Every candidate needs a stable
`candidate_id`. Display labels, command strings, and rendered summaries must
not be parsed for control flow.

Candidate admission is the structured scheduling trace:

- `admission.status`,
- `reason_category`,
- `reason_code`,
- `source`,
- `lane`.

Route, map, reward, shop, event, campfire, boss-relic, and run-choice
candidates should carry typed identity that can be continued without
recovering meaning from text.

## Report Field Admission

Reports, journals, summaries, and learning samples are interfaces. A quick
field can become an accidental policy surface.

Every new output field should be one of:

- `fact`: raw state or candidate data.
- `diagnostic`: intermediate view for debugging a model or scheduler.
- `verdict`: explicit conclusion with a named evaluator and evidence limits.
- `label`: training or evaluation target with a documented source.

If a field does not fit one of these classes, do not add it. Do not present
diagnostic extremes such as `furthest`, `best_hp`, or `cleanest` as winners
unless the evaluator really supports a winner claim.

Tests should protect stable structure, not prose. Avoid tests whose main
assertion is a human-facing adjective.

Potion spend-urgency output is a diagnostic question, not a verdict. It may
place an exact-root reserve delta beside validated inventory, supply, route,
recovery, and shop facts, but it must not collapse them into a score or spend
threshold. Route order must come from a typed modal ordering fact; aggregate
room counts cannot establish order. Missing, unknown, conflicting, or
root-mismatched evidence remains explicitly unavailable or rejected.

Reward-screen potion replacement belongs to the run-control reward owner. A
replacement must select a typed slot, potion identity, and UUID; the runtime
must not recover that identity from a label or generic discard command. The
owner may replace only when a bounded public-state contract discharges the
specific opportunity cost. The current production contract is deliberately
narrow: an incoming Fruit Juice may replace the newest duplicate inventory
identity; Gambler's Brew may replace the newest duplicate Fear when one Fear
remains and card semantics expose at least two native Vulnerable sources; and
an incoming Strength Potion may replace Fear Potion only when card semantics
confirm both a concrete Strength payoff and deck-based Vulnerable coverage.
The Gambler's Brew comparison is deliberately asymmetric: selective hand
redraw is the incoming resource, while Vulnerable coverage only discharges the
opportunity cost of the removed duplicate Fear. Exhaust roles do not mediate
that relationship. All other full-inventory comparisons remain unresolved.

Before replacement is necessary, reward transaction order must preserve
deterministic run resources. When a claimable Fruit Juice shares one reward
surface with ordinary potion rewards, the reward owner claims Fruit Juice
first so the existing potion-space step can realize it before any remaining
reward would require replacement. This ordering is driven by typed potion
identity, not reward labels.

Event-origin deck selections must include the event's typed post-selection
effect instead of treating every `PurgeNonBottled` boundary as an ordinary
cleanup. Bonfire sacrifice ordering may prefer a selectable Uncommon or Rare
card without a core-function or unsupported-loss classification when its
exact public-state recovery is at least the owner's strategic reserve
magnitude, even if current HP has not yet crossed below that reserve. Mark of
the Bloom remains an explicit healing blocker in that projection.

Deterministic post-victory HP carryover is also an owner-captured fact. In
particular, an offline audit must not infer a room-Boss act-transition heal from
an encounter name, floor number, or combat `is_boss` flag. The run-control
owner evaluates the exact combat context and persists the typed result;
diagnostics may validate and display it, while legacy absence remains
unavailable.

The strategic survival and search-quality HP-loss limits used by an owner are
owner-captured facts too. A combat audit may compare exact no-potion and potion
witnesses against those captured limits, but it must not reconstruct the limits
from final HP, a hand-configured reserve, or a copied policy formula. Crossing a
quality limit is diagnostic evidence that a local improvement matters to the
current owner contract; it is not by itself authority to spend a potion.

The most recent completed combat's start HP, pre-relic end-combat HP, raw loss,
turn count, and potion count are a compact owner fact rather than a diagnostic
trace. This fact survives external checkpoints even when detailed combat
outcomes and search traces are cleared. Legacy checkpoints without it remain
explicitly unavailable; route policy must not reconstruct it from net map HP.

## Prohibited Crossings

- Do not use strings as decisions when a typed action, candidate key, or case
  field exists.
- Do not let combat review mutate runner policy.
- Do not let combat search choose non-combat owner actions.
- Do not let runner code inspect hidden futures except through explicit
  diagnostic experiments.
- Do not add another summary/report layer when a capsule `summary.json`,
  `CombatCase`, journal event, or existing review output can carry the fact.
- Do not preserve a duplicate module just because migration is uncomfortable.

## Change Rule

Any change that moves behavior across these boundaries must update this file in
the same commit. Small search heuristics, runner retry gates, potion policies,
owner bridges, artifact shapes, and report fields all count when they change
who owns a decision.
