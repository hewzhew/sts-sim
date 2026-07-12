# Timed Enemy Threat Design

## Goal

Teach combat search to recognize enemy-owned mechanics that will cause a large,
forced player loss after a finite number of owner turns and can be canceled by
defeating that enemy. The first source-backed mechanic is Explosive Power on an
Exploder.

The motivating A3F35 capture proves this is an ordering problem rather than a
deck inevitability: a forced second-turn Exploder kill finishes at 55 HP, while
the otherwise comparable Repulsor-focus prefix finishes at 13 HP.

## Source Contract

`PowerId::Explosive` is the authoritative countdown state. At a player decision
boundary, a positive amount `N` means the power will trigger during its owner's
`N`th upcoming monster turn if the owner remains alive. When the amount reaches
one, the power queues owner suicide followed by 30 raw Thorns-type damage to the
player.

The AI must read the power, not infer the countdown from move history or
`ExploderRuntimeState.turn_count`. Future block, Intangible, and other mitigation
are not known by this fact, so it reports raw damage rather than claiming an
inevitable HP loss.

## Considered Approaches

1. **Typed per-target facts with staged consumers (recommended).** Extract an
   exact timed-threat fact from combat state, expose it in diagnostics, and use
   a lexicographic target-ordering key. Let rollout benefit through its existing
   reuse of action ordering. Add frontier value only if the first stage is
   insufficient.
2. Hard-code `EnemyId::Exploder` as a preferred target. This is small but mixes
   content identity with policy, cannot explain urgency, and will not generalize
   to another enemy carrying the same power.
3. Add one combined timed-threat score to action ordering, rollout, and frontier
   simultaneously. This may improve the capture quickly, but arbitrary weights
   would obscure which consumer fixed the problem and make regressions difficult
   to diagnose.

Use approach 1. It preserves the current no-prune/no-terminal-claim boundary and
keeps the first behavior change attributable to one ordering key.

## Fact Model

Add a focused `timed_enemy_threat` module under `ai/combat_search_v2`. It owns a
small immutable fact:

```rust
struct TimedEnemyThreatV1 {
    source_entity_id: EntityId,
    kind: TimedEnemyThreatKind,
    owner_turns_until_trigger: u32,
    raw_player_damage: i32,
    canceled_by_owner_death: bool,
}

enum TimedEnemyThreatKind {
    ForcedPlayerDamage,
}
```

The extractor scans living, actionable monsters. A monster with positive
`PowerId::Explosive` yields one fact with:

- `owner_turns_until_trigger = power amount`;
- `raw_player_damage = EXPLOSION_DAMAGE`;
- `canceled_by_owner_death = true`.

Export `EXPLOSION_DAMAGE` from the content power implementation so engine
resolution and AI facts cannot silently drift to different damage values.
Missing, zero, or negative power amounts and dead, dying, escaped, or half-dead
owners yield no active threat.

The model intentionally does not include target value, a recommendation, a
killability claim, predicted future block, or a combined score.

## First-Stage Consumers

### Diagnostics

Extend action target facts with an optional timed-threat object containing kind,
owner turns until trigger, raw player damage, and cancellation-by-death. Extend
the enemy-mechanics report with aggregate count, minimum owner turns until
trigger, and total raw timed damage. These fields describe state only.

Rename the report policy label away from the stale
`typed_act1_enemy_mechanics_fact_profile_no_direct_score` wording because the
profile already contains later-act and boss mechanics.

### Action ordering

Keep attacks against timed threats in the existing `DamageProgress` role. Add
three lexicographic fields to `ActionOrderingPriority`:

1. `targets_timed_threat` (`1` for an active cancelable threat, otherwise `0`);
2. `timed_threat_urgency` (`-(owner_turns_until_trigger as i32)`, so fewer owner
   turns sort first);
3. `timed_threat_raw_damage` (larger raw consequences sort first after urgency).

Compare these fields after existing reactive-risk and collector-specific terms,
but before generic phase setup and target-progress tie-breakers. This makes the
change conservative: immediate reactive costs and explicit encounter tactics
remain authoritative, while otherwise comparable damage actions prefer the
finite-horizon threat.

Populate the three fields only when the action has positive progress against one
specific enemy target. Non-damaging targeted actions and all-enemy actions retain
their existing ordering; existing lethal and total-progress logic continues to
own those cases.

Do not create a new role rank and do not change action legality, equivalence,
pruning, dominance, acceptance, or terminal scoring.

### Rollout

Make no separate rollout change. Conservative and enemy-mechanics-adaptive
rollouts already call the shared semantic action ordering before their one-step
probe. The new key therefore reaches rollout without a second policy or a
duplicated threat score.

## Deferred Frontier Stage

Do not change frontier value in the first implementation. After the ordering
change, rerun the frozen A3F35 capture and inspect whether unforced search keeps
damaging Exploder until it dies before the trigger.

Only if search starts correctly but later frontier selection abandons the target,
add a second design slice that consumes the same facts as separate lexicographic
state fields such as active timed-threat count and raw timed damage. Do not add a
weighted scalar as part of this design.

## Tests

Add only mechanism and boundary tests:

1. Explosive amount three yields one fact with three owner turns, 30 raw damage,
   and cancellation by owner death.
2. Explosive amount one reports one owner turn; dead or powerless owners produce
   no fact.
3. Action target diagnostics expose the exact timed-threat fact.
4. With otherwise equivalent damage actions, an Explosive target orders before
   a neutral target; a shorter countdown orders before a longer countdown.
5. Without Explosive Power, existing target ordering is unchanged.
6. A defensive action that prevents visible immediate HP loss still orders
   before ordinary timed-threat damage progress, proving this is not an
   unconditional attack rule.
7. The conservative rollout fallback observes the shared ordering rather than a
   separate Exploder-specific rule.

Do not add the full A3F35 outcome as a unit test. Use its saved capture and the
same bounded diagnostic commands as verification evidence after focused tests.

## Verification Evidence

After implementation:

1. Confirm the initial A3F35 report exposes Explosive amount three as a 30-damage
   timed threat.
2. Rerun corrected root-action and turn-plan guidance labs and verify that their
   one-step facts contain no energy exploit.
3. Run an unforced bounded search and verify its second-turn damage ordering
   maintains Exploder focus through a kill-before-trigger line.
4. Compare complete final HP and nodes-to-first-win with the current corrected
   baseline; record the result without promoting it to a teacher label.
5. Run the full library and architecture-boundary suites.

The diagnostic succeeds if an unforced search reproduces a kill-before-trigger
line. A fixed final-HP threshold is deliberately not part of the code contract.

## Scope Boundaries

- No card, relic, route, reward, deck-building, or run-control policy changes.
- No Exploder identity special case in action ordering.
- No Spiker priority change; its immediate Thorns loss remains represented by
  exact one-step reactive risk.
- No prediction of future draws, future block, or public-information policy.
- No frontier, pruning, dominance, transposition, acceptance, or terminal-value
  change in the first stage.
- No permanent seed outcome test and no reuse of pre-fix Writhe guidance as
  evidence.

## Success Criteria

- Engine and AI share one source constant for Explosive damage.
- Timed threats are exact, per-target, auditable facts with no embedded policy
  score.
- Equivalent damage actions prefer the more urgent cancelable timed threat while
  immediate survival ordering remains intact.
- Rollout receives the behavior only through shared action ordering.
- The frozen A3F35 diagnostic demonstrates unforced Exploder kill-before-trigger
  behavior, or the result explicitly justifies a separate frontier-stage design.
