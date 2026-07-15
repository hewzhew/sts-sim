# Campfire Evaluation Context And Evidence Batch Plan

## Goal

Build the first offline Campfire evaluation batch in which every legal candidate shares one explicit public root, route window, run goal, horizon, continuation profile, scenario distribution, and mechanics identity. Record exact immediate HP evidence without inventing survival, threat, feasibility, or growth values.

This slice does not rank candidates, call the legacy Campfire policy, or enter run control.

## Task 1: Define The Shared Public Evaluation Context

**Files:**

- Modify: `src/eval/fingerprint.rs`
- Create: `src/eval/campfire_evaluation.rs`
- Modify: `src/eval/mod.rs`

1. Add failing tests proving that hidden RNG state and hidden pool ordering do not change the Campfire context fingerprint, while a public HP or deck change does.
2. Expose the existing canonical JSON fingerprint helper within the crate.
3. Define a Campfire-local public root observation containing only player-visible run facts needed to identify this decision boundary.
4. Define and validate the evaluation specification: declared run goal, finite route horizon and budget, continuation profile plus source identity, public scenario-distribution identifier, and mechanics version.
5. Build route-window facts once and include their schema, configuration, content, and fingerprint in one context fingerprint.

## Task 2: Build A Complete Candidate Evidence Batch

**Files:**

- Modify: `src/eval/campfire_evaluation.rs`

1. Add failing tests proving that every canonical legal candidate appears exactly once and that Smith/Toke targets remain expanded by stable card UUID.
2. Project every candidate through `campfire_projection` under the same context.
3. Record exact immediate HP before/after evidence from the authoritative transition or exact stochastic prefix.
4. Represent run feasibility, survival distribution, threat resolution, and growth as field-level `Unsupported` evidence with machine-readable limitations. Unsupported fields cannot carry a numeric value.
5. Add chance and post-reveal recourse limitations for Dig and Dream Catcher without drawing from live RNG.

## Task 3: Verify And Commit

1. Run formatting plus focused Campfire evaluation, projection, and engine tests.
2. Run the full library suite and `architecture_runtime_boundaries` suite.
3. Commit the plan and implementation in small local commits, leaving the stable checkout clean.

