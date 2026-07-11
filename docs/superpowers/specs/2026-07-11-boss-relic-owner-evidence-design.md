# Boss Relic Owner Evidence Design

## Goal

Prevent the narrow mainline owner from selecting a boss relic solely because its broad class
sorts first when the repository already contains evidence that the relic adds a concrete run
debt or introduces a known startup liability.

## Decision

### Keep one owner, enrich its admission result

`boss_relic_owner` remains the runtime decision owner and continues to consume
`BossRelicAdmission`. The fix does not create another boss-relic selector and does not route the
mainline through the human-boundary policy. Instead, `assess_boss_relic_admission` incorporates
the existing run-debt projection and a before/after startup-profile projection for the candidate
relic.

The projected startup profile is produced from a temporary cloned run state containing the
candidate relic. This is an assessment-only projection; it must not mutate the live run.

### Use categorical burden, not a new aggregate score

Admission records two existing kinds of evidence:

- the candidate adds at least one run-debt contract; and
- the candidate changes a known startup-liability flag from false to true.

The startup comparison is intentionally limited to established boolean liabilities in
`DeckStartupProfileV1`. For this pass, the required boundary is
`has_pyramid_unupgraded_apparition`: Runic Pyramid offered to a deck with unupgraded
Apparitions introduces startup liability and can no longer be unconditional mainline.

Candidate ordering is lexicographic:

1. admission lane;
2. an explicit Act 2 energy-gap priority already established by admission;
3. categorical burden: no new burden, added run debt, then introduced startup liability; and
4. the existing boss-relic class order.

An Act 2 energy-gap mainline remains ahead of candidates that do not solve that gap even when
it adds a known debt. Outside that explicit priority, a same-lane route-value candidate with no
new burden may precede a constrained energy relic, and both precede a strategic relic that
introduces startup liability.

### Remove unconditional strategic mainline status

`StrategicPower` is not intrinsically safe. Its default lane becomes `Probe`; a future relic may
earn mainline status only through explicit deck-relative support. This prevents Runic Pyramid
and Snecko Eye from becoming the single narrow path merely because they are high-impact relics.

All executable boss-relic candidates remain visible and auto-expandable under the existing
owner contract. This pass changes their evidence and order, not branch fanout or stop semantics.

## Data Flow

1. The owner requests admission for every offered relic.
2. Admission computes the existing deck-relative lane and class.
3. Run-debt projection reports newly added contracts.
4. A temporary projected run produces the candidate startup profile.
5. Admission records categorical burden reasons and demotes an introduced startup liability to
   `Probe`.
6. The owner sorts by lane, explicit energy-gap priority, burden, and the existing class order
   before the narrow frontier chooses its first expandable candidate.

## Stable Tests

- Runic Pyramid is a probe by default rather than unconditional mainline.
- Runic Pyramid offered to a deck with an unupgraded Apparition records an introduced startup
  liability.
- A same-lane candidate with no new burden orders before an added-debt candidate, which orders
  before a candidate that introduces startup liability.
- A mainline energy-gap candidate remains ahead of burden-free probe candidates.
- Projection does not add the candidate relic to the live `RunState`.

Tests assert admission categories and ordering relationships. They do not lock an exact seed
path, aggregate score, full boss-relic tier list, or combat outcome.

## Non-Goals

- Do not model Velvet Choker together with Runic Pyramid in the startup profile in this pass.
- Do not model Enchiridion, Toolbox, or other generated opening cards in this pass.
- Do not choose a universally best relic among Sozu, Sacred Bark, and every future offer.
- Do not change combat search, card acquisition, route planning, branch fanout, or stop behavior.
- Do not add a frozen-seed or full-run regression test.
