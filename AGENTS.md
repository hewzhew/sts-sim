# Repository Workflow

- Work in the stable checkout at `D:\rust\sts_simulator` on a local feature branch. Do not create Git worktrees for this repository; switching source roots duplicates the large Cargo test build.
- Start changes only from a clean Git status and make frequent local commits.
- Never run `cargo clean` in this repository while legacy run capsules remain under `target`.
- Write new durable run capsules and experiment evidence under `artifacts/runs`, never under a Cargo profile directory.
- Delete build caches only when the resolved target is a known direct child of `target` and Cargo marker files prove it is a cache.
- Use focused tests during red/green work. Run the full library and `architecture_runtime_boundaries` suites at completion checkpoints.
