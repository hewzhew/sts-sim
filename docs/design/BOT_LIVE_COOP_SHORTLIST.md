# Bot Live-Coop Stupidity Shortlist

## Purpose

This is not a "make the bot strong" roadmap.

It is a bounded shortlist of bot/live-comm issues that most directly increase
human workload during `Assisted_Progression` / `BossHandoff` runs.

The bar for inclusion is:

- clearly increases live interaction cost, or
- causes avoidable stalls / skipped value, and
- can be fixed locally without reopening a full bot rewrite.

## Priority 1: Shop potion replacement is missing

### Symptom

When potion slots are full, the bot cannot decide to discard an existing potion
 in order to buy a stronger shop potion.

### Evidence

- Shop policy only considers potions when there is already an empty slot:
  - [agent_run_policy.rs](/d:/rust/sts_simulator/src/bot/agent_run_policy.rs:44)
  - [agent_run_policy.rs](/d:/rust/sts_simulator/src/bot/agent_run_policy.rs:103)
  - [agent_run_policy.rs](/d:/rust/sts_simulator/src/bot/agent_run_policy.rs:174)
- Live shop command generation only exposes `Potion(idx)` when `can_buy` is true:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:243)
- Protocol already exposes blocked shop potions via `blocked_reason`:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:329)

### Why it matters

This is a direct live-coop tax. The human has to notice and manually handle a
class of clearly valuable shop decisions.

### Recommended fix shape

- Add a narrow "discard-then-buy" shop path for `blocked_reason =
  potion_slots_full`.
- Reuse the existing reward replacement scoring style instead of inventing a
  separate subsystem.

## Priority 2: Reward potion replacement undervalues Elixir and similar potions

### Symptom

The bot sees a full-slot reward potion, has permission to discard, but often
decides not to replace because the offered potion is scored too low.

### Evidence

- Reward replacement path exists:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:565)
- Reward potion scoring bottoms out at a generic fallback:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:642)
- Shop potion scoring also falls back to a generic low score:
  - [agent_run_policy.rs](/d:/rust/sts_simulator/src/bot/agent_run_policy.rs:533)
- This matched the observed `Elixir` case in the April 16 live run.

### Why it matters

This does not usually "hard stall", but it creates obvious missed value in the
part of the run that should be safest to automate.

### Recommended fix shape

- Add explicit scores for `Elixir` and any other repeatedly under-valued potion.
- Keep the change local to potion scoring before changing replacement logic.

## Priority 3: Card rewards are too easy to skip when not handed off

### Symptom

If card rewards are not explicitly handed to the human, the bot can end up
skipping them too aggressively.

### Evidence

- Reward-screen card choice falls back to `Proceed` when no pick is recommended:
  - [agent.rs](/d:/rust/sts_simulator/src/bot/agent.rs:121)
- Live `CARD_REWARD` maps `Proceed` / `Cancel` directly to `SKIP`:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:70)
- Reward skip behavior is intentionally available in heuristics:
  - [reward_heuristics.rs](/d:/rust/sts_simulator/src/bot/reward_heuristics.rs:92)

### Why it matters

In `BossHandoff` this is masked by human card-reward handoff, but outside that
mode it makes the bot look much dumber than necessary and increases supervision.

### Recommended fix shape

- Keep skip logic, but make it more conservative in live-coop profiles or
  require a stronger skip margin for normal card rewards.

## Priority 4: Reward claim logic is split between bot policy and live bridge

### Symptom

Potion replacement at rewards is handled in `live_comm_noncombat`, not in the
bot's own `RewardScreen` policy. The result is that reward behavior depends on
where the decision is executed, not only on the abstract policy.

### Evidence

- Bot reward logic when items exist is very simple:
  - [agent.rs](/d:/rust/sts_simulator/src/bot/agent.rs:139)
- Reward potion replacement lives only in live bridge code:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:565)

### Why it matters

This is a maintenance smell: the same "reward decision" is partially in bot
policy and partially in live glue. That makes behavior harder to predict and
audit.

### Recommended fix shape

- Do not solve this with a rewrite.
- First extract a shared helper for "should replace potion X with potion Y".
- Keep the command emission in live bridge, but move scoring/decision policy
  closer to bot code.

## Priority 5: Shop choice generation is command-limited, not intent-aware

### Symptom

The bot can only choose among already-buyable shop actions. It cannot express
"I want that blocked potion if I can make room first."

### Evidence

- `build_live_shop_choices()` only emits already buyable options:
  - [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:225)

### Why it matters

This is the bridge-side counterpart of Priority 1. Even if higher-level policy
improves, the live adapter still cannot realize that intent.

### Recommended fix shape

- Add a narrow live-only expansion path for blocked shop potions.
- Do not generalize this into a large planner yet.

## Priority 6: Event fallback is still "first available choice"

### Symptom

There is still a trivial `choose_best_index()` fallback that always returns
`0`.

### Evidence

- [live_comm_noncombat.rs](/d:/rust/sts_simulator/src/cli/live_comm_noncombat.rs:28)

### Why it matters

This is less urgent than potion/reward issues, but it is exactly the kind of
silent stupidity that forces human correction in long observation runs.

### Recommended fix shape

- Either remove dead fallback paths if unused, or route them through a minimal
  safer heuristic.

## Suggested fix order

1. Shop potion replacement (`potion_slots_full`)
2. Potion scoring for `Elixir` and other repeated misses
3. Card reward skip conservatism
4. Reward replacement logic split cleanup

## Explicit non-goals

- no bot architecture rewrite
- no MCTS/search redesign
- no attempt to make the bot "good" in general
- no changes to combat engine semantics for bot convenience
