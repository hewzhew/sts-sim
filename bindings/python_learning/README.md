# Python learning bridge

This standalone Maturin crate is intentionally excluded from the root Cargo
workspace. It batches `LearningEnvPoolV1` control calls into NumPy arrays
without per-step JSON or one Python call per environment slot.

The control surface exposes slot identity, decision phase, ragged candidate row
splits, candidate counts, and an optional dense legal-action mask. Calling
`decision_batch(semantic=True)` additionally returns semantic schema version 4
as five sparse, columnar NumPy table families:

- `token`: token kinds plus per-decision row splits;
- `categorical`: token index, typed field id, and categorical value;
- `scalar`: token index, typed field id, and `float32` value;
- `relation`: source token, typed relation, and target token;
- `candidate_token_indices`: direct alignment with the flattened candidate
  rows already described by `candidate_row_splits`.

`semantic_schema()` returns the numeric enum dictionaries and categorical
vocabulary sizes from the same Rust definitions that produce the arrays. A
trainer therefore does not need a duplicate Python feature dictionary or a
source-code lookup to interpret field ids.

Strategic rows encode run facts, context/history, cards, relics, potion slots,
the public map graph, every `PlannerAction` variant, and typed candidate target
edges. Combat rows encode the complete public learning observation: encounter
and turn state, counters and ordered histories, player, monsters and intents,
cards and damage projections, powers and payload cards, relics, potions, orbs,
stances, and public encounter counters. Atomic candidates link directly to
their hand card, potion slot, and monster target. Indexed choices also carry
their reason and destination, not just the offered card or stance.

Symbolic-selection rows retain the unchanged matching combat or strategic
observation, the complete selection family/domain, the ordered chosen prefix,
and the current append-or-submit candidates. Run-level multi-card domains link
back to their eligible master-deck card tokens without enumerating complete
combinations. Public entity collections use their declared ordered/unordered
evidence. Candidate order remains fixed by its observation-local action
ordinal. Internal entity ids and card UUIDs are used only to resolve
relationships and never cross the bridge as feature values. An unexpected
action outside the maintained surface fails encoding instead of producing a
partial row.

`checkpoint_slot(slot_index)` returns an opaque, in-process exact checkpoint;
`restore_slot(slot_index, checkpoint)` restores both the simulator and any
unfinished symbolic-selection prefix without rerolling RNG. Checkpoints are
created only on an explicit caller request and do not form an automatic
history. A recovery curriculum therefore owns checkpoint retention, retry
counts, and replacement policy without becoming part of environment
mechanics.

Cross-process callers may explicitly snapshot the complete fixed batch with
`checkpoint_bytes(max_bytes=...)` and construct a fresh owner with
`LearningBatchEnv.from_checkpoint_bytes(payload, expected_slots=...,
max_bytes=...)`. The versioned opaque payload contains each exact run-control
session plus only the candidate-ordinal prefix needed to rebuild bridge-local
symbolic selection or a prepared action. Restore requires the exact slot count,
checks the caller byte limit before decoding, reconstructs every environment,
and replays every ordinal through the current typed decoder before exposing the
first slot. A corrupt, stale, or no-longer-legal prefix therefore fails closed.
The bridge does not inspect the payload in Python, use pickle, write files, or
publish an automatic checkpoint stream.

For vectorized curricula, `checkpoint_slots(slot_indices)` and
`restore_slots(slot_indices, checkpoints)` perform one cross-language call for
the whole selected subset. Restore validates every target, reconstructs every
environment, and observes every replacement before changing the first slot;
an invalid batch therefore cannot leave a partially restored pool.
An opaque checkpoint batch can select an ordered slot subset and return an
updated copy with replacement checkpoints for existing slots. This lets a
continuous caller retain one bounded episode-root bank across asynchronous
slot resets without inspecting sessions or falling back to per-slot bridge
calls. Selection rejects duplicate or missing slots, and update never adds a
new slot implicitly.
`reset_slots(slot_indices, seeds)` applies the same all-or-nothing rule when a
caller starts new episodes in terminal slots. Continuous callers use
`reset_slots_checkpointed(slot_indices, seeds)` to receive the exact new root
checkpoints from that same atomic bridge operation; there is no interval where
new episode seeds are active but only old roots are available.
Every recovery checkpoint is bound to the slot that created it. The ordinary
restore API rejects cross-slot cloning so recovery counts cannot be attached to
the wrong environment lineage.

The bridge still contains no policy, optimizer, automatic reset, or PyTorch
dependency. Its semantic arrays are an input contract, not evidence that a
particular model or learning objective is correct.
Terminal step batches retain aligned `terminal_slot_indices`,
`terminal_reward`, public `terminal_act`, `terminal_floor`, `terminal_hp`,
`terminal_max_hp`, and `terminal_gold` columns. These raw outcome facts support
progress and lower-tail auxiliary targets without turning HP or gold into a
handcrafted reward.

Run the maintained end-to-end verification with:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```

The script builds a wheel, runs the Rust semantic contract tests, installs the
wheel without dependency mutation into a fresh isolated environment that can
see the target Python's existing NumPy, and runs `tests/smoke.py` plus the
separate online caller contracts under `learning/tests`. It keeps the wheel,
environment, and complete logs below one fresh ignored
`.oracle-lab/python-learning-bridge/` directory and prints only a compact
summary plus that artifact location.
