# Apparition Upgrade Redundancy Design

## Context

Fresh owner-audit seed `20260711002` was built and run from clean source identity
`50194d77`; the capsule manifest, executable timestamp, and newly emitted Pandora offer evidence
confirm that the run did not use a stale script or binary.

The run still reaches A2F32 Collector with five unupgraded Apparitions. The repair profile exposes
all five as reliability upgrades, but the upgrade planner assigns Apparition to the generic
redundancy group. Five copies therefore trigger the generic `same_card_count >= 3` saturation rule,
add `LowMarginalRepeat`, and reduce each candidate to a score below the campfire Smith gate. Repair
priority never reaches executable-plan ordering.

## Decision

Treat an Apparition upgrade as independently valuable reliability work. Each upgraded copy removes
Ethereal from that concrete card; additional unupgraded copies do not make that mechanical delta
redundant. Model Apparition with the existing density-positive upgrade stack behavior instead of the
generic saturation behavior.

This is a card-analysis semantic correction, not a campfire exception. The repair profile,
upgrade planner, campfire evaluator, and owner continue to communicate through their existing typed
fields and tags.

## Boundaries

- Do not bypass the campfire upgrade-score gate for repair tags.
- Do not change the numeric Smith thresholds.
- Do not weaken RecoveryPressure or `RestFavored` checks.
- Do not change boss-relic ordering or Pandora admission in this pass.
- Do not add an exact seed replay, capsule, or checkpoint regression test.
- Do not change runner scripts; the source-identity investigation found them current.

## Verification

Add semantic tests proving:

1. Five unupgraded Apparitions are not classified as low-marginal repeated upgrades.
2. In a healthy campfire context with five Apparitions and ordinary growth targets, an Apparition
   reliability repair is executable and selected.
3. The same five-Apparition shape still chooses Rest when the existing rest-vs-smith policy favors
   recovery.

Run focused card-analysis, upgrade-planner, deck-repair, and campfire tests, then the full library
and architecture-boundary suites. After merge, run one fresh bounded single-branch seed
`20260711002` capsule with the established budget and compare the first real stop. Do not reuse an
old frontier or checkpoint.

## Success Criteria

- Apparition repetition no longer creates `LowMarginalRepeat` solely from copy count.
- Exact repair evidence clears the existing Smith gate without a score override.
- Rest safety remains unchanged.
- No unrelated card's redundancy behavior changes.
