# Archived Failed Routes, May 2026

These tools are preserved for audit and reproduction, but they are not current
mainline training surfaces.

## Why This Exists

Several experiments produced useful negative evidence:

- shallow draw/query-axis labels did not survive grouped hash splits
- absolute short-return Q with linear or small nonlinear features did not improve
  closed-loop play
- learned advantage override models did not reproduce the precision of verified
  oracle override
- candidate-only / learned proposer filters were not reliable enough to prune
  verified H8 decisions
- candidate-pack oracle labels were too dependent on truncated or hand-designed
  local utility protocols

Keeping these files in the root learning directory made it too easy to confuse a
negative baseline with the active route. They are archived here for reference.

## Subdirectories

- `return_q_negative_baselines/`: absolute and pairwise return-Q collectors,
  trainers, and closed-loop evaluators that failed the policy gate.
- `learned_adv_override_negative_baseline/`: learned safe-override attempts that
  did not reach verified teacher precision.
- `verified_proposer_negative_baselines/`: candidate-only, sklearn, and listwise
  proposer experiments that were not strong enough to prune verifier candidates.
- `candidate_pack_diagnostics/`: candidate-pack trainability and dominance
  audits. These remain useful as diagnostics but are not a training mainline.

Archived scripts may require `PYTHONPATH=tools/learning` or minor path updates if
they are run from their archive location.
