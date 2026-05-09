# AI Direction

Current answer: this repo is not ready to build an A20H Slay the Spire agent by
adding smarter bot rules.

The active AI work is frozen to simulator and evaluation infrastructure.

## What Can Count As Evidence

A decision can be used as AI evidence only if it comes from one of these sources:

- engine/protocol truth plus deterministic replay
- full-run outcome statistics on declared seed suites
- hand-authored or mechanically verified scenario tests with a named oracle source

The baseline bot is not an oracle. It can exercise the simulator and provide a
comparison point, but it cannot label better long-run decisions.

## Removed From Active Direction

- BranchTrace / branch comparison datasets
- candidate rollout scoring as policy evidence
- verified advantage override teacher
- live snapshot teacher shadow/takeover mode
- DecisionRecord `teacher_label`
- return-Q / pairwise teacher training
- Gym/PPO training scripts
- single-seed counterfactual policy patches
- hand-written macro-policy modules for reward, shop, event, map, campfire, and
  boss relic decisions
- live noncombat sidecar shadows that present heuristic macro decisions as
  evidence

## Current Viable Work

1. simulator parity
2. deterministic replay and DecisionRecord capture
3. legal observation versus internal state separation
4. outcome evaluation from externally supplied policies or random-masked smoke
   runs
5. scenario fixtures only when their oracle source is clear
6. after the truth/eval layer is stable, decide whether a small explicit search
   module or a full-run learner is worth building

No current module has authority to say a macro decision is good unless the claim
is backed by replay, scenario oracle, or held-out full-run outcomes.

The remaining `src/bot/` code is combat-oriented diagnostic/search machinery
plus representation helpers. Noncombat routing in live communication should be
treated as protocol fallback or human handoff, not strategy.
