# Defense Deficit Quality Design

## Context

Fresh bounded seed `20260711004` reached A2F21 Snake Plant at 33/74 HP. Combat search found
complete wins, but the best saved line ended at 17 HP while the owner required an 18 HP reserve.
Exact replay showed the player falling to 4 HP before Black Blood healed to 16, so relaxing the
combat survival gate would accept a genuinely fragile line rather than fix the upstream problem.

The route had already suffered repeated high-loss Act 2 combats. Nevertheless, the static deck
assessment reported `block_or_mitigation=adequate`. Its low-quality contribution currently caps
Defend and Iron Wave copies at three units, while the generic level boundary classifies every value
above two as adequate. Three starter Defends can therefore declare Act 2 defense adequate without
any stronger block or mitigation tool. That false state suppresses survival repairs such as Second
Wind through the `strategic-burden-no-gap` penalty.

## Decision

Correct the static defense-quality floor: low-quality block from Defend and Iron Wave may contribute
at most two units. Those cards can prevent defense from being missing, but cannot independently move
the deck beyond `Thin`. A non-low-quality block or mitigation contribution must be present before the
existing unit threshold can classify the deck as `Adequate`.

Keep the existing typed data flow unchanged:

1. `DeckRoleInventory` and `StrategicCounts` describe the deck.
2. `block_or_mitigation_units` separates strong contributions from low-quality copies.
3. `block_or_mitigation_level` maps the resulting units to the existing deficit levels.
4. Reward scoring consumes that deficit and admits survival repairs through existing
   `strategic-survival-gap` and construction-pressure rules.

This is a semantic correction inside the static deck model, not a seed-specific card bonus.

## Boundaries

- Do not change the owner combat HP reserve or accept the rejected Snake Plant line directly.
- Do not change the general low-HP `survival_pressure` thresholds.
- Do not thread branch combat history into `RunStrategicFacts` or core deck analysis in this pass.
- Do not special-case Second Wind, Headbutt, Snake Plant, Black Blood, or seed `20260711004`.
- Do not reweight strong block, Weak, or enemy-strength-down roles.
- Do not add a full seed/capsule regression test; verify the seed only after focused and full suites
  pass.

Recent realized combat attrition remains a separate follow-up. It crosses the branch-runtime/core-AI
boundary and needs its own design instead of being smuggled into this static correction.

## Verification

Use test-driven development to add semantic tests proving:

1. An Act 2 deck whose only block is three or four starter Defends remains `Thin`.
2. Adding a reliable non-low-quality block or mitigation card can move that same deck to
   `Adequate` through the existing thresholds.
3. In the relevant Act 2 reward shape, Second Wind is no longer tagged with
   `strategic-burden-no-gap` solely because starter Defends falsely closed the defense gap; it gains
   the existing `strategic-survival-gap` signal instead.
4. Low-quality copies do not become `Missing`, and unrelated frontload, access, energy, scaling, and
   burden fields remain unchanged.

Run the focused deck-deficit and decision-pipeline tests, then formatting, the full library suite,
and architecture-boundary tests. After local merge, rerun bounded seed `20260711004` from a fresh
capsule with the established single-branch budget and inspect its next real outcome.

## Success Criteria

- Starter-quality block alone cannot declare Act 2 defense adequate.
- Existing strong block and mitigation semantics still close a thin defense gap.
- Survival reward scoring changes through the shared deficit model, without card- or seed-specific
  overrides.
- Combat survival policy and unrelated strategic fields remain unchanged.
