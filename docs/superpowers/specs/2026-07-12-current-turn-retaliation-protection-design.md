# Current-Turn Retaliation Protection Design

## Goal

Explore a payable defensive setup before an attack when changing only the order of those two cards reduces known same-turn retaliation HP loss. Keep the change generic, local to action ordering, and independent of enemy identity.

This slice does not solve cross-turn retaliation debt. It fixes the narrower, already reproduced ordering failure first and leaves broader search coverage work for later evidence.

## Evidence

The frozen A3F42 winning line reaches turn 6 with 59 HP, zero block, four energy, Heavy Blade and Flame Barrier in hand, and a Spiker carrying 13 Thorns.

The selected order is Heavy Blade followed by Flame Barrier:

- Heavy Blade reduces player HP from 59 to 46;
- Flame Barrier then leaves 16 block;
- the unchanged remainder of the winning line ends at 43 HP.

An exact replay with only those two card plays reversed is legal:

- Flame Barrier first leaves 59 HP and 16 block;
- Heavy Blade consumes 13 block and leaves 59 HP and 3 block;
- every later recorded input replays unchanged;
- the line ends at 56 HP.

The 13-HP difference is therefore caused by action order, not draw order, target choice, route state, or a missing combat rule.

## Root Cause

Action ordering recognizes block only against visible enemy attacks. When the retaliation owner is buffing or otherwise has no visible attack, Flame Barrier receives the ordinary `Block` role with rank 45. Heavy Blade receives `DamageProgress` with rank 60, so the frontier expands the damaging child first.

Attack-retaliation facts correctly penalize attacks relative to other attacks, but no fact currently says that a payable block card is setup for a later retaliation-bearing attack in the same hand. The search eventually contains both actions, but best-first progress ordering spends its early coverage on the HP-losing permutation.

## Considered Approaches

### 1. Bounded current-turn protection counterfactual (recommended)

For an immediate block Skill, inspect payable attacks already in hand. Compare the known HP loss of attack-then-block with block-then-attack using existing engine-derived retaliation facts and visible incoming damage. Promote the block card only when the reordered pair has strictly lower total known HP loss.

Benefits:

- directly matches the reproduced failure;
- no enemy-specific checks or retaliation coefficient;
- distinguishes strictly better ordering from cases where block merely moves the same damage between retaliation and the enemy turn;
- changes child generation order only and never removes an action.

Cost: block candidates in retaliation combats perform a small bounded scan of playable hand attacks and use scratch combat states for defense-aware projection.

### 2. Raise all block cards whenever retaliation exists

This is simpler, but it promotes block even when the two-card sequence is unaffordable, the attack does not lose HP, or visible enemy damage consumes the same block and makes both orders equivalent. Rejected as too broad.

### 3. Add cumulative retaliation debt to search nodes and frontier lanes

This may still be useful for cross-turn planning, but it is larger than the proven same-turn permutation failure. Implementing it first would mix path-coverage policy with a local ordering bug. Deferred.

## Architecture

Add a focused helper beside the existing current-turn attack setup logic:

```text
current_turn_retaliation_protection_score(
    combat,
    setup_card_index,
    setup_block,
    setup_cost,
) -> i32
```

The helper has one responsibility: return the maximum strictly positive known HP-loss reduction enabled by playing this block Skill before one payable attack already in hand.

It consumes:

- evaluated immediate block for the setup card;
- current energy and card costs;
- current player block;
- visible incoming damage;
- `card_play_effect_facts` for attack-retaliation raw, block-loss, and HP-loss projections.

It does not inspect `EnemyId`, change combat state, execute the cards, or predict later turns.

## Eligibility Boundary

A setup candidate is eligible only when all of these are true:

1. it is a Skill with positive immediate block;
2. it does not end the turn or consume hand cards as conversion fuel;
3. its cost is payable now;
4. after paying it, at least one Attack in hand remains payable and playable against a legal live target;
5. that attack has positive attack-retaliation HP loss in the current state;
6. projecting the setup block before that attack yields strictly lower combined known HP loss.

Negative-cost/X-cost attacks follow the existing payable-after-setup convention. Attacks and block cards that cannot both be paid receive no setup signal.

## Counterfactual Calculation

For each eligible attack/target pair, compare two orders without executing either line.

### Attack then block

Use current defense-aware attack facts:

- retaliation HP loss is the current projected HP loss;
- remaining current block is current block minus projected retaliation block loss;
- add the setup card's immediate block;
- subtract the resulting block from visible incoming damage.

### Block then attack

Clone the combat state, add only the setup card's immediate block to the scratch player block, and recompute the attack facts:

- retaliation HP loss is the scratch projected HP loss;
- remaining block is scratch block minus projected retaliation block loss;
- subtract that remaining block from the same visible incoming damage.

The score is:

```text
(attack-first retaliation HP + attack-first visible HP)
- (block-first retaliation HP + block-first visible HP)
```

Only a positive result qualifies. Raw retaliation damage is not added as a score, so block consumption and HP loss are not double-counted.

This combined comparison is important. If visible incoming damage is large enough to consume all protection, the two orders often have identical total HP loss; in that case the helper returns zero and preserves existing ordering.

## Ordering Integration

Add an explicit `CurrentTurnRetaliationProtection` action role with a semantic label. Its rank sits with other current-turn preventive/setup roles: above `DamageProgress`, below lethal prevention and durable/key setup.

The positive counterfactual score is used only as a tie-break among qualifying protection cards. The legal action set, original action identifiers, pruning, equivalence, rollout, frontier value, and exact state identity remain unchanged.

Diagnostics can identify the new behavior through the role label. This slice does not add another search-wide counter unless focused implementation evidence shows the role label is insufficient.

## Testing

Focused tests prove:

1. with zero visible incoming damage, enough energy, positive Thorns, Heavy Blade, and Flame Barrier, Flame Barrier receives the protection role and orders before Heavy Blade;
2. exact two-action execution confirms block-first loses less HP and preserves the legal action set;
3. without retaliation, the same block card keeps its existing role;
4. when the two cards cannot both be paid, no setup signal is emitted;
5. when visible incoming damage makes attack-first and block-first total HP loss equal, no artificial preference is introduced;
6. multi-hit retaliation uses the existing sequential block projection rather than hit-count arithmetic.

Repository completion requires formatting, the full library suite, and `architecture_runtime_boundaries`. The frozen A3F42 line is rerun as ignored evidence. An improved order or HP result is recorded, but the seed trajectory is not made a permanent regression assertion.

## Non-Goals

- no Spiker, Shapes, or Thorns-specific policy;
- no global block-card promotion;
- no fixed retaliation-to-progress coefficient;
- no cross-turn path ledger or new frontier lane;
- no change to action legality, pruning, merging, rollout, or state identity;
- no permanent assertion for the frozen A3F42 trajectory.
