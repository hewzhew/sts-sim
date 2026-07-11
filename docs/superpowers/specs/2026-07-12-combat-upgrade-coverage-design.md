# Combat-Upgrade Coverage for Permanent Upgrade Planning

## Goal

Let the permanent upgrade planner recognize truthful combat-upgrade access from Armaments,
Armaments+, and Apotheosis. Ordinary damage and block upgrades may become less urgent when the
deck can repair them during combat, while timing-sensitive and strategically critical upgrades
retain their permanent value.

The resulting upgrade plan remains the single input to Rest-vs-Smith and campfire policy. The
campfire owner must not acquire card-specific Armaments rules.

## Problem

The current planner evaluates every permanent upgrade as though combat upgrades do not exist.
This can overstate the value of Smith when a deck already has useful Armaments coverage,
especially for basic cards whose upgrades are only ordinary damage or block increments.

The existing startup profile contains two raw access counters, but they are not consumed by the
upgrade planner. It also records Apotheosis as whole-hand access even though the simulator's
`UpgradeAllCardsInCombat` action upgrades eligible cards across all combat zones. Reusing those
counters directly would preserve a false semantic equivalence between Armaments+ and
Apotheosis.

A direct rule in campfire policy or the run-control owner would create a second upgrade policy,
couple execution to individual card knowledge, and make future explanations disagree with the
planner. A combat rollout at every campfire would be expensive and noisy. It may be useful as a
diagnostic later, but it is not the behavior-policy boundary for this pass.

## Decision

### One typed source of combat-upgrade facts

Add a small `combat_upgrade_coverage_v1` analysis boundary. It consumes only the current
`RunState` and exact card semantics and produces sources with one of three scopes:

- `SelectedCardInHand`: unupgraded Armaments can upgrade one eligible card currently in hand.
- `WholeHand`: Armaments+ can upgrade every eligible card currently in hand.
- `AllCombatZones`: Apotheosis upgrades eligible cards throughout the current combat.

Each source records its deck index, card id, upgrade level, and scope. The profile may expose
scope counts for compact consumers, but it does not assign a deck score, predict draw order, or
claim that a source will be available before a particular target is played.

Multiple sources improve the evidence that combat repair exists but do not widen their scopes.
For example, two unupgraded Armaments remain selected-card access rather than becoming
whole-hand access.

`DeckStartupProfileV1` derives its compatibility counters from this typed profile instead of
matching the same card ids independently:

- `combat_upgrade_selected_access_count` counts `SelectedCardInHand` sources.
- `combat_upgrade_hand_access_count` counts `WholeHand` sources only.
- a new Serde-defaulted `combat_upgrade_all_access_count` counts `AllCombatZones` sources.

Pyramid/Apparition combat-repair coverage recognizes all three scopes. Older serialized startup
profiles remain readable because the new counter defaults to zero. Existing fields remain
present; only the incorrect classification of Apotheosis as whole-hand access is corrected.

### Candidate-specific coverage class

The upgrade planner classifies each permanent upgrade candidate before granting combat-upgrade
credit:

- `OrdinaryStat`: the upgrade is primarily an ordinary damage, block, or numeric increment and
  carries no timing-sensitive mechanical delta or strategic role.
- `TimingSensitive`: delayed access can materially change whether the upgrade works. This
  includes cost reduction, exhaust control or removal, Ethereal removal, and the existing core
  mechanic, engine, consistency, defensive-survival, scaling, phase-burst, debuff-coverage, or
  transitional-power roles.
- `NoCombatCredit`: combat upgrading cannot reproduce the permanent value, or discounting the
  provider would be circular. Innate deltas, Armaments itself, and Apotheosis itself are in this
  class.

`FrontloadDamage`, `LowMarginalRepeat`, and `Speculative` do not by themselves make a candidate
timing-sensitive. This allows basic attacks and blocks to receive truthful coverage while a
Whirlwind carrying phase-burst responsibility remains protected.

Classification is fail-closed. If a future mechanical delta or role is not understood, the
candidate receives no combat-upgrade credit and keeps its permanent priority.

### Bounded priority adjustment

Combat-upgrade coverage never removes a candidate and never creates an aggregate Armaments
score. It can lower an eligible candidate by at most one ordinal priority level:

- selected-card access may lower only ordinary starter or already low-marginal repeated targets;
- whole-hand or all-combat-zone access may lower any `OrdinaryStat` target;
- additional sources cannot stack further reductions;
- `ImportantBeforeBoss` and `CriticalBeforeBoss` urgency is never reduced;
- candidates paying timing-sensitive or boss-specific debt are never reduced;
- `TimingSensitive` candidates receive explanatory evidence only, not a priority reduction;
- `NoCombatCredit` candidates are unchanged.

The adjustment runs after the existing debt ledger has annotated candidates. When permitted, it
lowers candidate urgency by one step and recomputes the candidate verdict through the existing
ordering rules. The debt ledger remains truthful about which upgrades could pay a debt; the
adjusted candidate verdict determines whether that debt is currently strong enough to justify
Smith.

This intentionally models available repair capacity rather than exact access probability. A
later access-reliability pass may use draw, retention, or bottling facts, but those facts must not
be guessed here.

### Planner and campfire data flow

The final data flow is one-way:

1. Exact card/runtime semantics produce `CombatUpgradeCoverageProfileV1`.
2. The upgrade planner builds mechanical candidates and the existing debt ledger.
3. Each candidate receives a typed coverage class and bounded coverage credit.
4. Candidate urgency and verdict determine `best_smith` and the existing Rest-vs-Smith result.
5. Campfire policy consumes the adjusted upgrade plan through its current score hints and
   strategy tags.
6. The campfire owner continues to delegate Rest and Smith to campfire policy unchanged.

Neither `campfire_policy_v1` nor `campfire_owner` matches Armaments, Apotheosis, or individual
upgrade deltas. Existing HP thresholds, recovery-pressure gates, and Smith score thresholds stay
unchanged in this pass.

## Evidence and diagnostics

`UpgradePlanV1` exposes the combat-upgrade coverage profile. Each upgrade candidate exposes its
typed coverage class and granted credit. Human-readable evidence includes:

- the source card and exact scope;
- why the candidate is ordinary, timing-sensitive, or ineligible;
- whether priority changed and the previous and resulting ordinal levels;
- when a possible combat repair was observed but deliberately did not lower a hard debt.

Rest-vs-Smith reasons mention combat-upgrade coverage only when it changes which candidate can
justify Smith. No evidence claims a particular draw order or guaranteed in-combat upgrade.

Unknown or unsupported semantics produce an explicit conservative reason and no priority
change.

## Stable tests

Add only relationship tests that protect architectural truth:

1. The fact profile distinguishes unupgraded Armaments, Armaments+, and Apotheosis as selected
   hand, whole hand, and all combat zones respectively. Existing startup counters are derived
   from those scopes, including the corrected Apotheosis counter.
2. Whole-hand coverage lowers ordinary Strike/Defend-style permanent upgrade priority by one
   level, while selected-card coverage is limited to ordinary starter or low-marginal targets.
3. Armaments itself, an Innate delta, and a Whirlwind carrying phase-burst debt receive no
   priority reduction.
4. One campfire-policy boundary fixture shows that an ordinary Smith candidate no longer clears
   the existing gate solely because combat-repairable value was overstated. The test asserts the
   planner's typed verdict relationship and the policy action class, not exact aggregate scores.

Existing owner-policy delegation tests are sufficient to protect the run-control boundary. Do
not duplicate the same behavior at owner, policy, and planner layers.

Tests must not assert a complete seed path, exact random draw, boss outcome, long-run win rate,
or exact total score. No full campaign replay is required for this pass.

## Non-goals

- Do not model exact Armaments/target co-draw probability.
- Do not run combat search or campaign counterfactuals from campfire policy.
- Do not conclude that a deck with Armaments should always Rest.
- Do not lower high-severity, boss-specific, or timing-sensitive upgrade debt.
- Do not redesign recovery pressure, route risk, campfire HP thresholds, or owner fallback order.
- Do not add potion timing, Kunai/Nunchaku throughput, card reward, or route policy changes.
- Do not use Apotheosis's stronger scope to introduce a general Apotheosis valuation model.
- Do not add full-seed or boss-victory regression tests.
