# Construction Reliability Boundary Implementation Plan

> **For agentic workers:** Execute this plan task-by-task with test-driven
> development and verification before completion.

**Goal:** Correct three upstream construction decisions that make the mainline
fragile before combat search begins.

**Architecture:** Reuse `deck_construction_pressure` for effective access,
add one narrow deployability-debt fact to acquisition, and make Sozu's Act 2
energy lane conditional on concrete potion-synergy relics.

**Tech Stack:** Rust 2021, built-in test harness, Cargo.

## Global Constraints

- Do not change Collector combat search or add boss-specific tactics.
- Do not force an archetype, exact reward, exact relic, or seed trajectory.
- Add semantic boundary tests only; do not snapshot temporary behavior.
- Do not use subagents for this implementation.

### Task 1: Effective access

**Files:**
- Modify: `src/ai/strategy/deck_strategic_deficit.rs`
- Modify: `src/ai/strategy/deck_construction_pressure.rs`
- Modify: `src/ai/strategy/acquisition.rs`

- [ ] Add a failing strategic-deficit test separating one real draw source
  plus cantrips from two real draw sources.
- [ ] Map construction `card_flow` pressure to strategic access and make the
  focused test pass.
- [ ] Add failing acquisition tests proving a cantrip only fixes access when
  it changes pressure, while real draw still receives gap credit.
- [ ] Expose the smallest reusable real-draw/card-flow helper and make the
  focused acquisition tests pass.

### Task 2: Deployability debt

**Files:**
- Modify: `src/ai/strategy/deck_construction_pressure.rs`
- Modify: `src/ai/strategy/acquisition.rs`

- [ ] Add a failing reward-policy test for an ordinary expensive card in an
  energy-thin deck that already carries expensive cards.
- [ ] Record the minimal construction evidence needed to identify that debt.
- [ ] Downgrade only non-hard-gap, non-energy solutions to `Speculative` and
  make focused tests pass.

### Task 3: Constrained Act 2 energy relic

**Files:**
- Modify: `src/ai/strategy/boss_relic_admission.rs`

- [ ] Add failing tests for Sozu versus Black Blood with and without a
  potion-synergy relic.
- [ ] Admit Sozu to the Act 2 energy-gap mainline only when its potion lock is
  not strategically contradicted and make focused tests pass.

### Task 4: Verification and one bounded run

- [ ] Run formatting and all library/integration tests.
- [ ] Run exactly one bounded single-branch mainline for seed `20260710002`.
- [ ] Inspect construction decisions and report the first genuine blocker;
  do not turn the resulting exact path into a regression test.
- [ ] Self-review the diff, merge locally, re-run merged verification, and
  remove the temporary worktree and branch.
