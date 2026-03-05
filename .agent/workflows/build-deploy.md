---
description: Build sts_sim Rust code and deploy .pyd to bottled_ai_fresh for verification
---

# Build & Deploy sts_sim

Use this workflow after modifying Rust source code to rebuild and deploy the Python extension.

## Steps

// turbo-all

1. Build the release binary:
```
cd c:\Dev\rust\sts_sim && cargo build --release 2>&1 | Select-Object -Last 5
```

2. Copy the .dll as .pyd to bottled_ai_fresh (cargo build produces .dll, Python needs .pyd):
```
Copy-Item "c:\Dev\rust\sts_sim\target\release\sts_sim.dll" "c:\Dev\rust\bottled_ai_fresh\sts_sim.pyd" -Force; Write-Host "Deployed sts_sim.pyd"
```

3. Run verification summary:
```
cd c:\Dev\rust\bottled_ai_fresh && python sim_diag.py summary 2>$null
```

## Notes

- `cargo build --release` produces `target/release/sts_sim.dll`
- `maturin develop --release` produces `target/release/sts_sim.pyd` (and installs to site-packages)
- `bottled_ai_fresh` imports the LOCAL `sts_sim.pyd` from its own directory, so we must copy there
- The .dll and .pyd are identical binaries, just different extensions
- If the timestamp on .pyd didn't change, the build was a no-op (no code changes detected)
