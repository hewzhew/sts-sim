# Combat Case Adjudication Probe Design

## Goal

Let `combat_case_review` show why a complete combat-search line is accepted or rejected by
run-control. The probe compares ordinary acceptance and clean-only acceptance against the same
exactly replayed line. It is diagnostic evidence only: it must not claim that a saved capsule can
resume or that the full seed can win.

## Chosen Approach

Add an opt-in adjudication probe to `combat_case_review`, backed by a small public run-control
library API. The CLI continues to own case loading, bounded search selection, and JSON assembly.
Run-control owns projection of the saved combat case into a minimal combat session, exact line
replay, run-level outcome observation, and policy adjudication.

Two alternatives are intentionally rejected:

- Inferring curses from `CombatState.meta_changes` in the CLI would create a second adjudicator and
  repeat the owner split that was just removed.
- Adding another diagnostic binary would expand the tool surface without providing a distinct
  responsibility.

## User Interface

Add `--adjudicate` to `combat_case_review`. It requires or implies the existing bounded ladder so
there is a current complete trajectory with full actions. Existing output is unchanged when the
flag is absent.

When a complete line exists, add a top-level `adjudication_probe` object containing:

- the source ladder/profile label and exact action count;
- a trust label stating that persistent run context was projected from the combat case;
- the single `CombatLineObservedOutcomeV1` produced by exact replay;
- the result for `AcceptedLineOnly`;
- the result for `CleanAcceptedLineNoNewCurse`.

If no complete line exists or exact replay fails, the object records a typed unavailable/error
status. It does not fabricate an accepted or rejected result.

## Data Flow and Ownership

1. The existing bounded ladder searches the saved `CombatPosition`.
2. Review orchestration selects one complete winning trajectory and retains the configuration that
   produced it.
3. A public run-control probe API projects the minimum persistent context from `CombatCase`:
   seed, ascension, act, floor, HP, max HP, gold, combat master-deck snapshot, player relics,
   potions, active combat state, and combat context.
4. The line is replayed once through the existing exact candidate-line path. Run-control observes
   deck, gold, potion, HP, and external-payoff effects using the existing adjudication boundary.
5. The same observed outcome is classified by both requested acceptance plugins. No second search
   or second run-effect observer is allowed.
6. The CLI serializes the evidence into the review artifact.

The projected session is deliberately not returned as a resumable checkpoint. Map state, reward
history, and future run RNG are outside this combat-only artifact's trust boundary.

## Error Handling

- Missing complete trajectory: `status=no_complete_line`.
- Case/session projection failure: `status=projection_failed` with a concise error.
- Exact replay failure: preserve `CombatLineAdjudicationV1::ReplayFailed` for both policy views.
- Outcome/policy disagreement in internal invariants: fail the probe explicitly; do not silently
  fall back to search-level `complete_win`.

Probe errors do not suppress the existing combat review. The review command still exits normally
when raw review evidence is valid, while the probe object explains its own failure.

## Verification

- A focused library test proves one observed outcome is reused for ordinary and clean-only policy
  classification and that a gained `Parasite` yields accepted-dirty versus rejected-new-curse.
- A projection test proves the saved combat master deck, relics, potions, HP, floor, and active
  position are copied into the minimal session without creating a resumable checkpoint.
- A CLI serialization test proves `--adjudicate` is additive and absent by default.
- Run the saved Writhing Mass case with bounded budgets and confirm the review contains raw search
  feasibility plus the two typed policy results. This verifies the diagnostic symptom only, not a
  full-seed victory.

## Non-Goals

- No full-seed replay or capsule resurrection.
- No Writhing Mass strategy/scoring changes.
- No new clean-line search or action-order heuristic.
- No changes to owner-audit lane commit policy.
- No historical artifact rewrite.
