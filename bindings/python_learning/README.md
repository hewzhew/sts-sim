# Python learning bridge

This standalone Maturin crate is intentionally excluded from the root Cargo
workspace. It batches `LearningEnvPoolV1` control calls into NumPy arrays
without per-step JSON or one Python call per environment slot.

The control surface exposes slot identity, decision phase, ragged candidate row
splits, candidate counts, and an optional dense legal-action mask. Calling
`decision_batch(semantic=True)` additionally returns semantic schema version 1
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
edges. Public entity collections are emitted in canonical semantic order;
candidate order alone remains fixed by its observation-local action ordinal.
Internal UUIDs are used only to resolve edges and never cross the bridge as
feature values. Combat and symbolic-selection rows currently expose
`SEMANTIC_NOT_ENCODED`, zero semantic tokens, and the
`SEMANTIC_NO_CANDIDATE_TOKEN` sentinel for every candidate. Callers must not
replace that explicit incompleteness with zero-filled combat features.

The bridge still contains no policy, optimizer, automatic reset, or PyTorch
dependency. Its semantic arrays are an input contract, not evidence that a
particular model or learning objective is correct.

Run the maintained end-to-end verification with:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```

The script builds a wheel, installs it without dependency mutation into a fresh
isolated environment that can see the target Python's existing NumPy, and runs
`tests/smoke.py`. It keeps the wheel, environment, and complete logs below one
fresh ignored `.oracle-lab/python-learning-bridge/` directory and prints only a
compact summary plus that artifact location.
