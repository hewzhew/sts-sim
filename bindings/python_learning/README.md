# Python learning bridge

This standalone Maturin crate is intentionally excluded from the root Cargo
workspace. It batches `LearningEnvPoolV1` control calls into NumPy arrays
without per-step JSON or one Python call per environment slot.

The current surface is a control-plane smoke only. It exposes slot identity,
decision phase, ragged candidate row splits, candidate counts, and an optional
dense legal-action mask. It does not yet expose trainable semantic observation
or candidate features and must not be treated as a policy or learning result.

Run the maintained end-to-end verification with:

```powershell
.\bindings\python_learning\verify.ps1 -Python <python-3.12-executable>
```

The script builds a wheel, installs it without dependency mutation into a fresh
isolated environment that can see the target Python's existing NumPy, and runs
`tests/smoke.py`. It keeps the wheel, environment, and complete logs below one
fresh ignored `.oracle-lab/python-learning-bridge/` directory and prints only a
compact summary plus that artifact location.
