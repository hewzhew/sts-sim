# Combat Search Incumbent Portfolio Design

## Goal

Make post-primary combat search behave as a candidate portfolio: independent search attempts propose replayable outcomes from one immutable combat root, a monotone incumbent selects the best trustworthy candidate, and the real `RunControlSession` is committed exactly once.

This fixes the seed006 Transient failure mode where an Immediate search with more generated nodes committed a 38 HP win before a Lazy search that could produce a 48 HP win was considered.

## Problem

The current owner-audit portfolio combines four responsibilities inside a lane:

- engine configuration and budget;
- run-control acceptance policy;
- fallback permissions;
- immediate mutation of the real session.

`run_lane_attempt` searches a cloned session, but writes the clone back as soon as that lane is accepted. `run_combat_portfolio_step` then stops on the first non-gap result. Consequently, lane order is also the final candidate ranking, and adding or reordering a lane can make the committed result worse.

The hallway quality and survival lanes also currently use the same core Immediate engine configuration. Their different HP and fallback policies cause the same expensive search to be linked to two policy roles rather than treated as one candidate generator plus separate adjudication.

Node count, elapsed time, and first acceptance are not quality proofs. A time-bounded search attempt is partial evidence and must never replace a better replayed incumbent merely because it ran later or generated more nodes.

## Scope

This change covers the owner-audit combat portfolio and the profile boundary it consumes. It does not change combat mechanics, card ordering, route policy, shop policy, card reward policy, potion semantics, or the inner `combat_search_v2` expansion algorithm.

The ordinary primary lane remains a fast path. If primary produces an accepted line or a valid operation-budget chunk, it commits as it does today. The incumbent portfolio opens only after primary reports a genuine combat gap.

## Profile Boundary

Split `CombatSearchProfile` into explicit engine and attempt-policy values:

```rust
pub struct CombatSearchEngineProfile {
    pub budget: CombatSearchBudgetSpec,
    pub plugins: CombatSearchPluginStack,
}

pub struct CombatSearchAttemptPolicy {
    pub acceptance: CombatSearchAcceptancePluginId,
    pub artifacts: CombatSearchArtifactPluginId,
}

pub struct CombatSearchProfile {
    pub label: &'static str,
    pub engine: CombatSearchEngineProfile,
    pub policy: CombatSearchAttemptPolicy,
}
```

The engine profile is the search identity. Labels, acceptance plugins, artifact verbosity, HP reserve gates, and fallback permissions are not part of that identity. Two profiles with equal engine values represent the same core search configuration even when their labels or policies differ.

Owner-audit lane policy remains outside `combat_search_v2`. It specifies the HP reserve gate, whether internal no-win rescue is allowed, whether Smoke Bomb survival is allowed, and whether an operation-budget chunk may commit.

## Candidate Lifecycle

`run_lane_attempt` must stop mutating the caller's session. It receives an immutable root, runs on a clone, and returns a `CombatSearchLaneAttempt` containing:

- the trial session when the attempt produced an applicable result;
- the command outcome, status, and auto-stop facts;
- the effective engine profile and lane policy;
- the best complete-win summary, when present;
- actual post-combat run HP and search-reported potion use;
- whether the result is a reserve-compliant win, relaxed win, or survival fallback.

The orchestrator owns mutation. It may immediately commit a primary fast-path result, but post-primary attempts are collected without touching the real session. After the planned attempts finish, the orchestrator commits only the selected attempt's trial session and auto-applied steps.

All post-primary attempts start from the same immutable root. A rejected or accepted attempt cannot affect the state searched by a later attempt.

## Incumbent Ordering

Introduce `CombatSearchIncumbent` with one `offer` operation. It may replace the current candidate only when the offered candidate is strictly preferable under this explicit order:

1. a terminal run victory outranks every non-terminal result;
2. a reserve-compliant complete combat win outranks a relaxed complete win;
3. a relaxed complete win outranks a non-win survival fallback;
4. within one tier, a candidate strictly dominates when it has no lower post-combat run HP, uses no more potions, discards no more potions, and improves at least one of those values;
5. if those resource facts are equal, fewer combat turns and then fewer actions break the tie;
6. incomparable candidates preserve the earlier incumbent and record that reason instead of hiding a resource trade inside an arbitrary scalar score.

This ordering deliberately makes candidate replacement monotone. Adding a new lane cannot replace an incumbent with a candidate that is worse in any recorded resource dimension. In the observed Transient case, the 48 HP Lazy result strictly dominates the 38 HP Immediate result because their potion cost is equal.

The HP reserve tier is evaluated by owner-audit after candidate generation. Post-primary search therefore must not discard a replayable complete win solely because it exceeds the normal reserve: the candidate may still become the relaxed incumbent if no reserve-compliant win exists.

## Portfolio Schedule And Budget

Keep the existing primary fast path. For a hallway combat gap with a usable potion, run these post-primary roles from the same root:

1. the existing no-potion Immediate escalation;
2. an Immediate semantic-potion quality attempt;
3. a Lazy semantic-potion survival attempt with the existing no-win and Smoke Bomb fallback permissions.

The third role replaces the duplicate Immediate survival search. It uses the budget that the old survival fallback already consumed, so the configured worst-case portfolio budget does not increase. Immediate and Lazy are deliberately complementary engine profiles, not labels for the same search.

Elite and boss lane sets retain their current budgets and producer count in this phase. They use the same non-mutating attempt and incumbent machinery even when only one post-primary candidate exists.

The plan builder must reject duplicate engine profiles inside one candidate-producing portfolio. A fallback producer is distinct only when its effective engine profile or candidate-generation behavior is distinct; changing acceptance text or artifact verbosity is insufficient.

## Reliability And Commit Boundary

- Only exact replayed and run-control-adjudicated complete lines may enter a complete-win tier.
- Coverage labels, node counts, rollout estimates, and wall-clock duration never outrank replayed outcome facts.
- A failed, truncated, or inapplicable attempt contributes diagnostics but cannot become incumbent.
- The real session is unchanged until the primary fast path or final incumbent is committed.
- Exactly one post-primary attempt contributes committed `auto_steps`.
- All attempts continue to contribute search performance summaries and diagnostics.
- Primary operation-budget chunks remain outside cross-lane arbitration because they intentionally advance only a bounded prefix and require continuation from that new root.

## Trace Contract

Each portfolio attempt report must expose:

- lane label and stable engine fingerprint;
- effective child-rollout, frontier, potion, node, and wall-time configuration;
- candidate tier, final combat HP, post-combat run HP, potion use, and turns when available;
- whether the attempt was selected;
- the incumbent decision reason, including strict improvement, lower tier, incomparable resource trade, invalid result, or duplicate engine suppression.

The portfolio-level action list must come from the selected attempt rather than the last attempted lane.

## Alternatives Rejected

- Making Lazy the global default uses one seed-specific observation as a universal policy and can regress encounters where Immediate finds the useful branch first.
- Keeping first-accepted commit semantics makes lane order an implicit evaluator and permits monotonicity failures.
- Comparing only final HP silently treats every potion as free.
- Comparing only potion count preserves resources even when doing so crosses a dangerous HP threshold.
- Sharing a mutable frontier, transposition table, or parallel incumbent between engines would add nondeterminism and synchronization complexity before the sequential commit boundary is trustworthy.
- Adding a Transient-specific rule would hide the general orchestration defect.

## Verification

Default regression coverage should test architectural invariants rather than lock one seed's temporary policy:

- engine identity ignores label and attempt policy but distinguishes Immediate from Lazy;
- duplicate engine profiles are suppressed by the portfolio plan;
- lane attempts leave the real session unchanged;
- only the selected incumbent commits its trial session and auto steps;
- a same-cost higher-HP candidate replaces the incumbent;
- a lower-tier or resource-incomparable candidate cannot replace it;
- portfolio reports identify the selected lane and use its action list;
- the hallway schedule contains complementary Immediate and Lazy profiles within the existing worst-case budget.

The seed006 Transient capture remains an opt-in diagnostic probe, not a permanently linked default test. When run, the candidate set with at most two potions must not select the known 38 HP line while a replayable 48 HP line is present.

Completion verification runs focused owner-audit tests during red/green work, then the full library and `architecture_runtime_boundaries` suites required by `AGENTS.md`.
