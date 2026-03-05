---
description: Build sts_sim Rust code and deploy .pyd to ALL targets for verification
---

# Build & Deploy sts_sim

Use this workflow after modifying Rust source code to rebuild and deploy the Python extension.

## ⚠️ Dual-Target Architecture

Python loads `sts_sim` from **two different locations** depending on context:

| Context | Python | Loads from |
|---------|--------|-----------|
| `bottled_ai_fresh/sim_diag.py` | Global `C:\Python314\python.exe` | `AppData/Roaming/Python/Python314/site-packages/sts_sim/` |
| `bottled_ai_fresh/` (direct .pyd) | Global `C:\Python314\python.exe` | `bottled_ai_fresh/sts_sim.pyd` (if exists) |
| `.venv` scripts | `.venv/Scripts/python.exe` | `.venv/Lib/site-packages/sts_sim/` |

**Both targets must be updated after Rust changes.** Missing either causes "changes not taking effect" debugging nightmares (30+ min wasted, 2026-03-05).

## Steps

// turbo-all

1. Build release wheel:
```
cd c:\Dev\rust\sts_sim && maturin build --release 2>&1 | Select-Object -Last 3
```

2. Install to global Python (the one sim_diag.py and verify scripts use):
```
C:\Python314\python.exe -m pip install --force-reinstall (Get-ChildItem "c:\Dev\rust\sts_sim\target\wheels\*.whl" | Sort-Object LastWriteTime -Descending | Select-Object -First 1).FullName 2>&1 | Select-Object -Last 3
```

3. Copy .pyd to bottled_ai_fresh as well (for any direct imports):
```
Copy-Item "c:\Dev\rust\sts_sim\target\release\sts_sim.dll" "c:\Dev\rust\bottled_ai_fresh\sts_sim.pyd" -Force; Write-Host "Deployed sts_sim.pyd to bottled_ai_fresh"
```

4. Run Watcher verification:
```
cd c:\Dev\rust\sts_sim && python C:\tmp\verify_watcher.py 2>NUL
```

## Verification

After deploy, confirm the module is fresh:
```python
import sts_sim, os, datetime
f = sts_sim.__file__
d = os.path.dirname(f)
for fn in os.listdir(d):
    if fn.endswith('.pyd'):
        mt = datetime.datetime.fromtimestamp(os.path.getmtime(os.path.join(d, fn)))
        print(f"{fn}: {mt}")
```
The `.pyd` timestamp should match the build time.

## Notes

- `cargo build --release` only compiles — does NOT install to Python
- `maturin develop --release` installs to `.venv/` ONLY (not global Python)
- `maturin build --release` builds a wheel in `target/wheels/`
- **The .dll and .pyd are binary-identical** — just different extensions
- Python does NOT hot-reload .pyd — restart the Python process after deploy
