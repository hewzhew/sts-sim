# Live Comm Parity Workflow

This workflow is retired.

The old strict/survey parity loop depended on live protocol import, state-sync
rebuild, and replay/case artifacts. Those active code paths have been removed.

Current rule:

- use local Rust simulator/search first
- use Java source for mechanics truth
- treat old live logs as historical evidence only
- rebuild Java-connected parity later as a new adapter, not by restoring the old
  state-sync stack
