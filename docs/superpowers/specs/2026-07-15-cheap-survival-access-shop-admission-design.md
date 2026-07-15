# Cheap Survival-Access Shop Admission Design

Date: 2026-07-15

## Context

Seed `20260713006` exposes a deterministic Act 1 shop failure. After purchasing a 75-gold Strike removal at A1F2, the owner reevaluates the shop with 43 gold and still sees an unupgraded `Shrug It Off` priced at 25 gold. The candidate is not unavailable and the shop is not stale; the decision pipeline marks it inspect-only with `shop card has no acquisition policy support`, then leaves.

The current acquisition contract treats only a small explicit set as premium, automatically admits upgraded shop cards, and otherwise requires a package repair or modeled strategic gap. Unupgraded `Shrug It Off` is ordinary. Its one-card draw is intentionally not counted as real deck-access improvement, while the starter deck's raw block count makes the static mitigation deficit look adequate. Its low price is recorded as `AcquisitionOpportunityCost::Cheap` but does not itself create an acquisition reason.

The repository already has multi-step shop portfolio enumeration for branch diagnostics. Normal owner execution still takes one action and reevaluates the shop. The A1F2 failure therefore does not require a general shop execution rewrite: the immediate blocker is the post-purge acquisition gate.

## Goal

Allow a genuinely cheap, burden-free shop card that compresses immediate survival and still-needed access into one card to remain behavior-eligible after shop reevaluation.

For the seed006 witness, `Shrug It Off` at 25 gold must no longer be inspect-only after the Strike purge. Existing sequential execution may then buy it before leaving, and existing branch diagnostics may retain `Remove Strike + Shrug It Off` as a complete affordable alternative.

## Non-goals

- Do not special-case `Shrug It Off` by card ID.
- Do not classify ordinary-price support cards as premium.
- Do not promote every multi-step portfolio plan into normal owner execution.
- Do not change Pyramid, Fiend Fire, boss relic, potion, route, or combat-search policy in this change.
- Do not assert that the owner must always purchase the card or that this exact bundle is globally optimal.

## Considered approaches

### Card-specific premium exception

Mark `Shrug It Off` as premium or directly allow it in Act 1 shops. This repairs the witness quickly but turns acquisition into a growing card tier list and does not explain why the 25-gold offer is different from an ordinary-price offer. Rejected.

### Cheap survival-access compression admission

Add an explicit, auditable acquisition fact for a shop candidate that is cheap, burden-free, provides a survival mechanic, and provides an access mechanic that the role inventory still lacks. Admit that fact as a contextual purchase. This is the selected approach.

### General multi-step shop execution

Let `ExecuteOne` rank and execute arbitrary two- or three-step shop portfolios. This could compare the complete entry-to-exit state, but it affects every shop and does not remove the acquisition gate that currently blocks the Shrug step. Deferred until the narrower admission repair has been measured.

## Design

Add a boolean acquisition fact named `cheap_survival_access_compression` to `CardAcquisitionReport`.

It is true only when all of the following hold:

1. The source is a shop.
2. The existing opportunity-cost classifier returns `Cheap` (currently price at most 35 gold).
3. Reward admission class is `ImmediateWork`, excluding burdened immediate work.
4. The candidate provides at least one survival mechanic: Block, Weak, permanent enemy Strength loss, or temporary enemy Strength loss.
5. The candidate provides Card Draw while the current role inventory has fewer than two draw units, or provides Energy while the current role inventory has no energy unit.

The fact is deliberately narrower than "cheap good card". It represents role compression: one low-opportunity-cost card contributes to survival without consuming a separate deck slot for access. `Shrug It Off` satisfies it in the A1F2 witness through Block plus Card Draw. A cheap single-role attack, a burdened access card, or an ordinary-price Shrug does not satisfy it.

Add `AcquisitionPolicyReason::CheapSurvivalAccessCompression`. In the shop policy decision, check this fact after existing premium, upgrade, package, boss-scaling, and hard-gap cases but before the final `NoPolicySupport` rejection. Return `ContextTake`, not `AutoAcquire`.

The decision pipeline continues to use the existing acquisition filter and scoring pipeline. No owner-loop changes are needed: after the removal, the newly admitted candidate can remain on the mainline and compete with leaving using the existing reevaluation.

## Evidence and observability

The report field and policy reason must appear in debug output, so a future run can distinguish this admission from premium-card logic or hard-gap repair. The fix must not relabel static deck access: a small cantrip may remain insufficient to close the formal deck-access deficit while still being admitted as cheap role compression.

Existing branch portfolio enumeration should be inspected after the change. If it already exposes the pre-purchase `Remove Strike + Shrug It Off` combination, no portfolio production change is required. If it does not, that is separate evidence for a later shop portfolio task rather than permission to broaden this patch.

## Test design

Use test-first development.

1. Construct the exact post-purge strategic state: Act 1, four Strikes, four Defends, Bash, Berserk, 43 gold, and an unupgraded 25-gold `Shrug It Off`.
2. Assert that the current acquisition/decision pipeline does not inspect the candidate and reports `CheapSurvivalAccessCompression`. This test must fail before production changes because the current result is `NoPolicySupport`.
3. Assert that an otherwise identical ordinary-price candidate does not receive the new reason unless an existing independent acquisition rule admits it.
4. Assert that a cheap single-role immediate card does not receive the new reason.
5. Run focused acquisition and decision-pipeline tests, then the full library and `architecture_runtime_boundaries` suites at the completion checkpoint.

The regression boundary is semantic eligibility and evidence. It must not hard-code a complete run trajectory, an exact candidate score, or a permanent requirement to choose Shrug.

## Success criteria

- The seed006 post-purge Shrug candidate is behavior-eligible with an explicit contextual reason.
- Cheapness alone does not admit unrelated ordinary cards.
- Ordinary-price Shrug behavior is unchanged unless another existing rule applies.
- No general shop execution, route, reward, potion, boss relic, or combat-search behavior changes.
- Focused tests, the full library suite, and architecture runtime boundaries pass.

