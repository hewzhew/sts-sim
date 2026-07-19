# Policy-guided lazy oracle combat search

Status: implementation contract. This replaces the production combat witness
selection path; it is not an additional rescue lane.

## Goal

Find one exact, replayable combat win for the full-information A0 oracle run,
within an explicit work budget. The immediate campaign target is seed006 through
the Act 3 boss. This design is only the combat solver; route, reward, shop, Neow,
and campfire decisions remain separate run decisions.

The solver may use expert knowledge to decide where to spend work. Expert
knowledge is never terminal evidence and never removes a legal action.

## Current facts

The present production path in `src/ai/combat_search_v2` has four properties
that matter to this migration:

1. `prepare_node_expansion` materializes and steps every ordered atomic action
   at a concrete state before another frontier item can compete for work.
2. `NodePriority` is a lexicographic stack led by rollout value and followed by
   the most recent action prior, state value, and several local hints. It is not
   a path policy and does not express how much probability mass an entire line
   has accumulated.
3. the exact-state and resource-coverage tables are useful, but root-round
   scheduling, turn-plan seeding, rollout promotion, and full atomic expansion
   compete as separate control mechanisms.
4. `sts_combat_planner::TurnOptionGeneratorSession` already owns exact,
   resumable generation of complete player-turn options and structured
   selection transactions. It currently uses FIFO/BFS work and is not the
   production combat owner.

The replacement therefore reuses the exact simulator, replay, state key, and
structured-selection machinery. It does not preserve the current frontier
priority or rollout portfolio as fallback behavior.

## One owner and one graph

`OracleCombatWitnessSession` in `sts_combat_planner` becomes the only owner of
an oracle combat attempt. The run-control layer creates it, grants work quanta,
and accepts only an exactly replayed terminal win.

The shared graph contains only concrete stable boundaries:

- a stable player-turn start;
- a stable terminal win, loss, or escape.

Atomic card, potion, targeting, and selection prefixes are private work inside
a turn-option generator. They are not graph states, transposition keys, replay
steps, or value targets.

```text
stable turn state
  -> resumable policy-guided turn generator
  -> one exact complete-turn option
  -> stable next-turn or terminal state
```

Each stable state owns one resumable generator. Generating a turn option does
not exhaust the generator: its remaining work stays available and competes
with already discovered successor states.

## Policy is guidance, not a verdict

At every concrete atomic decision, a policy returns a positive distribution
over the exact legal action surface.

```text
pi(a | s) = (1 - epsilon) * normalized_expert_weight(a, s)
            + epsilon / legal_action_count
```

Requirements:

- every legal action has non-zero probability;
- probabilities are normalized at the state where the choice is made;
- equivalent actions are compressed before normalization;
- the policy returns typed weights/probabilities, never a prose reason;
- missing expert knowledge produces a uniform distribution, not an invented
  score;
- oracle truth comes only from simulator transitions and replay.

The first production policy is an adapter over the existing typed combat
action facts and ordering roles. The adapter converts their ordering into
weights. It does not copy the existing lexicographic `NodePriority`, and it
does not use rollout or a combat outcome estimate. Later human or witness data
may replace the weights without changing the search contract.

## Path priority

For a discovered stable state `n`, retain:

```text
atomic_depth(n)       number of real player inputs on the path
negative_log_pi(n)    -sum(log(pi(a_i | s_i)))
```

The initial queue key is the log form of Levin tree search:

```text
levin_log_priority(n) = log(max(1, atomic_depth(n))) + negative_log_pi(n)
```

Lower is better. A terminal win is checked immediately and replayed; it does
not wait behind heuristic work.

No learned or hand-written state value participates in the first production
key. PHS can later add a non-negative remaining-cost estimate only after that
estimate has a tested contract. Until then this is Levin tree search with an
expert policy, not a value-guided search pretending to know the future.

## Partial expansion

The central tractability rule is that earning one frontier turn does not mean
materializing every child.

Each stable state has two kinds of work that share one record:

1. `next_option`: ask its turn generator for the next best complete-turn
   option under accumulated local policy probability;
2. `expand_successor`: schedule the concrete successor returned by an option.

After one option is produced, both the successor and the still-live generator
return to the global frontier. The generator's queue key is a lower bound for
its best not-yet-produced option: parent path cost plus the best retained
atomic-prefix policy cost. Thus generated lines and ungenerated siblings
compete under the same unit.

This is partial expansion, not progressive truncation. Exhausting a budget
leaves exact residual work; it does not declare omitted actions bad or absent.

## Complete-turn generator ordering

`TurnOptionGeneratorSession` changes from FIFO work to a min-priority heap.
Every private prefix stores:

```text
prefix_atomic_depth
prefix_negative_log_pi
sequence_id
```

Expanding a prefix:

1. obtains the exact legal action language from the stepper;
2. compresses proven equivalent actions;
3. requests a normalized positive policy distribution;
4. creates private successor prefixes with cumulative log probability;
5. stops and emits an option at the next stable player-turn or terminal
   boundary.

The heap is deterministic: equal priorities use insertion sequence, then the
canonical action address. Split work quanta must produce the same option order
as one combined quantum.

## Combinatorial actions

Hand/Grid/Scry and potion-created selection windows remain factorized. A subset
is not generated as one member of an eager power set.

The selection cursor becomes a policy-guided prefix tree:

```text
candidate 0: include / exclude
candidate 1: include / exclude
...
submit one complete legal ClientInput
```

Cardinality constraints force branches when possible. Proven symmetric
candidates share a canonical count decision. Order-sensitive selections add a
second ordering phase only for the selected order-sensitive members.

Each include/exclude decision receives non-zero conditional probability. A
completed selection contributes one real simulator input and one unit of
atomic path depth; virtual prefix decisions contribute policy cost for search
ordering but never appear in replay or concrete-state depth.

Potion discard remains legal in full oracle mode. The policy may give it very
small mass, but legality and the epsilon floor prevent silent removal.

## Exact duplicate handling

The outer graph uses the existing exact combat state key at stable boundaries.
For the first cutover:

- exact duplicates merge;
- a duplicate keeps the path with lower Levin priority, then lower
  `negative_log_pi`, then lower atomic depth;
- no approximate combat dominance rule may delete a state;
- no state-value comparison may delete a state;
- private generator prefixes never enter the concrete transposition table.

Resource dominance can return only after each relation has a mechanics proof.
This may cost memory initially, but it prevents setup lines from being removed
by an unproven local comparison.

## Work budget

One attempt reports and limits three real resources:

```text
agenda_pops       global or private work items selected
engine_steps      simulator steps actually executed
wall_time         external safety deadline
```

Every atomic transition reserves its whole engine-step allowance before it
starts, preserving deterministic resumability. Rollouts have no separate
budget because the first solver has no rollout subsystem.

The session is anytime and resumable. A quantum may end with:

- `WitnessFound`;
- `AgendaBudget`;
- `EngineStepBudget`;
- `Deadline`;
- `FrontierExhausted`;
- `MechanicsGap`;
- `ReplayMismatch`.

`FrontierExhausted` is the only no-witness result that means all represented
legal work was consumed. A budget stop is never reported as evidence of loss.

## Exact witness

On discovering a terminal win, reconstruct the complete atomic input path and
replay it from the combat root with exact successor hashes. Accept only if:

- every input is legal at replay time;
- every expected successor hash matches;
- the final stable state is a terminal win;
- no transition exceeded its engine-step contract.

The first verified witness ends the feasibility search. Improving ending HP or
resource use is a different objective and must not delay the seed006 oracle
milestone.

## Production cutover

There is no runtime fallback from the new owner to `combat_search_v2`.

The cutover is complete only when all of the following land together on the
production path:

1. policy-guided resumable turn generation;
2. global Levin-priority witness session;
3. exact boundary transposition;
4. exact terminal witness replay;
5. run-control calls the new session for oracle combat;
6. the old root-round, rollout-promotion, turn-plan portfolio, and state-value
   selection path is unreachable from oracle execution.

Unreachable legacy code is removed after the production entry and focused
contracts pass, within the same cutover series. It is not kept as a hidden
rescue and is not deleted before the replacement is executable.

## Focused evidence

The implementation does not use the 2,800-test root library as its edit loop.
Planner contracts live in `sts_combat_planner` and cover:

1. all legal atomic actions receive positive normalized probability;
2. a high-fanout state produces one option while retaining residual work;
3. split and combined quanta produce identical option order and counters;
4. subset selection is lazy and replay-exact;
5. exact duplicate states keep the better path without approximate pruning;
6. a known small combat yields a replay-verified witness;
7. a budget stop retains work and never becomes a loss claim.

After focused contracts pass, run one frozen ordinary combat and then continue
the seed006 oracle trajectory from its first combat stop. The acceptance test
for the campaign remains a full replay from Neow through the Act 3 boss.

## Deliberately deferred

These are compatible extensions, not prerequisites or fallback lanes:

- PHS remaining-cost heuristic;
- learned policy weights from verified witnesses;
- novelty features for policy blind spots;
- proven resource dominance;
- hidden-information belief search;
- quality optimization after the first win.

They may change search order, never simulator truth or witness verification.
