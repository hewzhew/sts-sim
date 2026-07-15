# Deletion-Driven Campfire Prospect Migration Design

## Goal

Replace the current threshold-driven Campfire policy with one production decision owner that compares the consequences of every legal Campfire action under the same public-information model.

This is the first deliberately narrow migration toward a consequence-first run planner. It must prove that the project can replace an old semantic owner and delete it in the same delivery. It must not add another permanent snapshot, score layer, policy version, or fallback path.

The migration is successful when Campfire decisions are made from explicit survival and growth prospects, every legal option remains representable, stochastic rewards do not leak hidden RNG, and `campfire_policy_v1` no longer exists in production source.

## Why Campfire Is The First Boundary

The present Campfire path spreads one decision across several layers:

- the engine determines legal actions and applies their mechanics;
- `campfire_policy_v1` turns coarse run facts into Rest-versus-Smith verdicts and fixed HP gates;
- the upgrade planner carries a Rest-versus-Smith field even though its durable responsibility is upgrade value;
- owner-audit interprets the policy result for execution;
- the branch experiment adds an independent Smith score threshold and tag whitelist;
- deck mutation configuration reaches into Campfire configuration for an upgrade-priority threshold.

This makes it possible for two callers to disagree about the same state, makes unsupported cases look like policy rejections, and prevents removal of temporary thresholds because other modules have learned their names.

Campfire is bounded enough to migrate completely: its action families and exact mechanics are finite, while Smith and Toke exercise the candidate-scaling problem and Dig and Dream Catcher exercise the hidden-randomness problem. A successful migration therefore establishes the architectural rules needed later by route, reward, event, and shop decisions without designing those systems now.

## Scope

This design covers:

- legal Campfire candidate enumeration;
- deterministic candidate transition projection;
- public-information treatment of stochastic consequences;
- a shared consequence schema for survival, timed threat coverage, growth, and uncertainty;
- selection of one production Campfire decision;
- consumption of that decision by owner-audit and the branch experiment;
- removal of the old Campfire semantic owner and its leaked configuration.

This design does not change combat mechanics, map generation, encounter selection, card reward generation, relic generation, combat search expansion, or route policy. It does not attempt to solve Smith, Toke, Dig, route, shop, and reward planning as a joint cross-product. It does not require seed006 to win and does not encode a named seed's action as a regression assertion.

## Migration Invariants

The delivery obeys these non-negotiable invariants:

1. There is exactly one production Campfire semantic owner after cutover.
2. There is no `campfire_policy_v2`, long-lived shadow policy, or old-policy fallback.
3. A new production reader is not added until its old semantic reader is removed in the same delivery.
4. Unsupported evidence widens uncertainty or keeps alternatives alive; it never silently becomes zero value, rejection, or an invitation to run the old policy.
5. Production policy file count and line count decrease at cutover. Moving old thresholds behind new names is not migration.
6. Shared abstractions are extracted only when a second migrated production consumer needs them. Campfire may define a small local prospect vocabulary, but it must not introduce a repository-wide `StrategicSnapshot` or duplicate `route_window_facts`.
7. The information available to a decision must match information available to a real player at that point in the run.

## Ownership And Data Flow

The production flow is:

```text
Engine legality + public run state + route_window_facts
                    |
                    v
          legal Campfire candidates
                    |
                    v
       exact transitions / chance prospects
                    |
                    v
        one Campfire prospect comparator
                    |
                    v
             CampfireDecision
               /          \
              v            v
       owner-audit     branch display
       applies one     retains explicit
       decision        alternatives only
```

The engine remains the sole authority for legality and mechanics. A canonical `legal_campfire_candidates` boundary enumerates semantic candidates such as `Smith { deck_index }` and `Toke { deck_index }`. It uses the same engine rules that validate execution. UI-level placeholder options may remain an interface concern, but policy, owner-audit, and branch tooling must not independently reimplement target legality.

`route_window_facts` remains the source for visible route and encounter-window facts. This migration does not create another route window type.

The upgrade planner reports upgrade consequences, including the best upgrade-debt reduction for each legal Smith target. It does not issue a Rest-versus-Smith verdict. The durable `best_smith_debt_paid` fact moves onto the upgrade plan itself so shop and random-upgrade consumers do not depend on a Campfire decision structure.

The new unversioned Campfire owner produces a `CampfireDecision` containing the selected candidate, its prospect, nondominated alternatives, evidence coverage, and a machine-readable decision reason. Owner-audit executes that result. The branch experiment may display or retain alternatives already identified by the same comparator, but it may not define independent score thresholds, tag allowlists, or action semantics.

## Candidate Set And Scaling

The candidate families are:

- `Rest`;
- one `Smith` candidate for every legal upgrade target;
- one `Toke` candidate for every legal removal target;
- `Dig` when Shovel makes it legal;
- `Lift` when Girya makes it legal;
- `Recall` when the red key can be taken.

All legal candidates receive a cheap prospect evaluation before any pruning. Smith and Toke are independent families:

```text
Value(Smith) = best prospect among all legal Smith targets
Value(Toke)  = best prospect among all legal Toke targets
```

The evaluator never constructs a Smith-by-Toke-by-route cross-product. Route-window facts and the baseline capability envelope are computed once. Candidate deltas are evaluated in batches, making the ordinary cost linear in deck size.

No top-K heuristic may discard unevaluated legal targets. A candidate may be pruned only when conservative lower and upper bounds prove it dominated by another candidate. Candidates whose intervals overlap remain nondominated. A small nondominated or close finalist set may receive more expensive simulation or search evidence, but the cheap pass must preserve complete legal coverage.

## Consequence Prospect

A candidate does not receive one universal scalar score. It produces a `CampfireProspect` with four parts:

### Survival distribution

- probability interval for death within the visible planning window;
- HP quantiles at the end of that window, at least p10, p50, and p90;
- resource-loss distribution when potion or other consumable use is modeled;
- the earliest credible failure turn and associated failure modes.

### Threat-resolution distribution

- time by which immediate multi-target threats can be cleared;
- time by which scaling or phase threats can be killed or controlled;
- delay supplied by block, Weak, Strength reduction, Intangible, potions, and other mitigation;
- probability that the required cards and resources are accessible and deployable before each deadline.

### Growth distribution

- upgrade or removal debt paid;
- change in draw quality, action supply, energy use, and damage/block access;
- expected and lower-tail value of stochastic rewards such as Dig and Dream Catcher;
- irreversible costs and preserved future option value.

### Evidence quality

- `Exact` for exact deterministic transition facts;
- `Calibrated` for an estimator validated on in-distribution held-out data;
- `Partial` when only some relevant failure modes or outcomes are represented;
- `Unsupported` when the estimator cannot defend a useful interval;
- coverage, sample count, provenance, and uncertainty bounds supporting that label.

These fields are predictions of consequences, not instructions to Rest, Smith, or choose any other action.

## Overall Pressure As A Timing Relationship

“Overall pressure” is not another weighted score and is not synonymous with recent HP loss. It is the relationship between a threat envelope and a capability envelope.

The threat envelope records public or distributional deadlines: incoming damage, multi-target clearance requirements, enemy scaling, debuffs, status-card pollution, and phase changes. The capability envelope records when the deck can resolve, delay, or survive those threats, including draw probability, energy, action limits, retained hands, potion access, and relevant relic interactions.

For a particular threat:

- `T_fail` is the earliest credible turn on which the threat causes an unacceptable state;
- `T_resolve` is the distribution of turns on which the deck can eliminate or control it;
- `T_delay` is the time gained by mitigation.

The relevant feasibility question is the distribution of:

```text
T_resolve <= T_fail + T_delay
```

Damage improves the left side by resolving enemies sooner. Defense improves the right side by delaying failure. Draw, energy, Pyramid, and Choker affect whether the required actions can be deployed. Area damage changes the resolution schedule for multi-target encounters. This avoids the false inference that high damage taken always means “take more defense”: in a scaling or deadline encounter, insufficient damage access may be the cause of the same HP loss.

For unknown future encounters, the prospect integrates over the public eligible encounter pool. It must not use the hidden next enemy identity. The output retains per-failure-mode probabilities so an apparent average does not hide a catastrophic Reptomancer-like multi-target gap.

## Exact And Stochastic Transitions

Deterministic mechanics are applied to cloned `EngineState` and `RunState` through the same authoritative transition path used by execution:

- Rest's immediate healing;
- Smith for a particular card;
- Toke for a particular card;
- Lift;
- Recall.

Rest with Dream Catcher is a mixed prospect: the healing transition is exact, while the generated reward is stochastic.

Dig and Dream Catcher must not be evaluated by cloning the live seeded state and observing the next actual relic or card reward. That would reveal hidden future information even if the clone is discarded. Instead, their prospects are distributions over publicly eligible outcomes under the real generation rules. The evaluator may enumerate or sample that public distribution using an analysis RNG independent of the run RNG. It may use known pool eligibility and state-visible exclusions, but never the live RNG counter, hidden pool order, or realized future outcome.

The same rule applies to downstream random consequences reached by otherwise deterministic actions. Exact prefixes remain exact; hidden suffixes become distributions.

## Comparison And Decision Semantics

Reliability is lexicographically prior to median growth, but only when the evidence supports the comparison. The comparator uses this order:

1. robustly lower death-risk upper bound;
2. robustly better lower-tail HP outcome;
3. better coverage of the earliest timed failure modes;
4. better median growth and option preservation;
5. lower uncertainty when consequences are otherwise indistinguishable.

“Robustly” means the relevant confidence or credible intervals separate. Overlapping intervals do not authorize a winner; both candidates remain nondominated.

When the branch budget supports alternatives, the decision exposes the nondominated set. When mainline execution must choose one candidate, it selects the best worst credible survival outcome, then avoids irreversible loss, then preserves future option value. If those rules still cannot distinguish candidates, the decision reports an explicit `decision_gap` with the missing evidence. It does not call the removed policy.

Dig is therefore not inherently unsafe and Rest is not inherently correct. A healthy state may accept Dig's variance for strong expected growth; a dangerous state may prefer Rest's stable lower tail. Likewise, a Smith upgrade helps survival only to the extent that the upgraded card can be drawn and deployed before a relevant deadline.

## Predictor Boundary And Feasibility Gate

`CampfireProspect` is a stable consequence contract, not a commitment to a particular machine-learning technique. Its producer may combine exact mechanics, analytic estimates, bounded combat simulation, and a calibrated learned outcome predictor. Production contains one producer for each fact, not an analytic policy plus a model policy competing for authority.

Before changing the production Campfire owner, an offline feasibility experiment uses existing Combat Lab captures and generated counterfactual states. Experimental code and artifacts are not wired into run control. If the experiment fails, it may be frozen outside production or removed; the old owner is not partially migrated.

The grouped dataset rules are:

- provenance records simulator commit, mechanics/schema version, information boundary, search coverage, and label type;
- all counterfactual siblings from one root state remain in the same train, validation, or test partition;
- complete seeds are held out from training, and at least one encounter family is held out for out-of-distribution evaluation;
- a bounded search result labeled “no win found” is coverage evidence, not an “unwinnable” ground-truth label;
- paired ranking examples contain two legal actions from the same root whose outcome order is resolved by exact replay or sufficiently covered search; unresolved pairs count against coverage instead of becoming guessed labels;
- exact replayed outcomes outrank search estimates as labels.

The prospect producer is eligible for production cutover only when the offline gate shows:

- 100% enumeration and cheap evaluation coverage for legal Smith and Toke targets;
- zero uses of live hidden RNG state in stochastic prospect generation;
- the lower bound of the 95% grouped-bootstrap interval for pairwise survival-ranking accuracy improvement over the current Campfire-policy-derived baseline is greater than zero;
- death-probability calibration no worse than a current-HP plus encounter-window baseline on held-out seeds;
- empirical p10, p50, and p90 HP coverage within five percentage points of nominal coverage on in-distribution held-out data;
- withheld-family states widen to `Partial` or `Unsupported` rather than presenting unjustified calibrated confidence;
- median batch latency no more than 100 ms and p95 no more than 250 ms for a synthetic 64-card legal-target state on the local reference machine.

These criteria assess consequence quality and information safety. They do not require a specific action on seed006 or a global win-rate claim from a small sample.

## Failure And Coverage Behavior

Transition application failure for an engine-declared legal deterministic candidate is a mechanics or integration bug. It is surfaced and blocks the decision; it is not converted into a low prospect.

Prediction failure produces a wider interval and `Partial` or `Unsupported` coverage. Unknown consequences remain eligible. `Unsupported` must never map to zero, `Reject`, `InspectOnly`, or a sentinel numeric score.

Out-of-distribution detection and low search coverage must abstain honestly. A mainline decision may still be necessary, but it is then made by the conservative comparison rule over stated bounds and accompanied by a decision gap. There is no semantic fallback to `campfire_policy_v1`.

## Production Cutover And Required Deletion

Once the feasibility gate passes, one cutover changes both owner-audit and the branch experiment to consume `CampfireDecision`. The same change deletes the old semantic surface.

Required deletion includes:

- the entire `src/ai/campfire_policy_v1/` directory;
- the `campfire_policy_v1` export from `src/ai/mod.rs`;
- `RestVsSmithPlanV1` and `RestVsSmithVerdictV1`;
- the 45/60/70 HP gates and `CampfirePolicyConfigV1`;
- the deck mutation compiler's dependency on Campfire configuration;
- the branch experiment's `MIN_INSPECT_ONLY_SMITH_BRANCH_SCORE` and Campfire tag whitelist;
- tests that lock old gates, verdict tags, or temporary scalar scores.

The upgrade planner retains pure upgrade facts needed by shop and random-upgrade consumers, but no Rest-versus-Smith verdict. Deck mutation receives any durable upgrade-priority fact from its proper upgrade/deck source rather than a Campfire config default.

The cutover is incomplete if old symbols remain reachable, if a caller can compute a second Campfire verdict, or if production policy code only grows. The old policy is not kept for comparison after merge; comparison evidence belongs in the offline artifact produced before cutover.

## Diagnostics

Every decision report includes:

- all legal candidate families and target counts;
- candidates removed by proven dominance and the bounds proving it;
- nondominated finalists;
- survival and HP distributions;
- earliest unresolved timed threats;
- growth and irreversible-cost summaries;
- evidence coverage and provenance;
- selected action or explicit decision gap;
- confirmation that stochastic evaluation used an independent analysis distribution.

Diagnostics must be descriptive rather than authoritative. A tag may explain why the owner chose a candidate, but no downstream consumer may turn that tag into a second policy.

## Verification

Default regression coverage locks architectural and information-boundary invariants rather than temporary strategy opinions:

- engine legality and candidate enumeration agree for every Campfire family;
- every legal Smith and Toke target reaches the cheap evaluation pass before pruning;
- deterministic candidates produce identical prospects from identical inputs;
- stochastic prospects do not read or advance the live run RNG and are invariant to its hidden cursor;
- Dream Catcher keeps exact healing separate from its stochastic reward distribution;
- `Unsupported` candidates remain eligible and cannot become numeric zero or automatic rejection;
- dominance pruning occurs only when conservative intervals separate;
- owner-audit and branch display consume the same `CampfireDecision`;
- only owner-audit applies the selected transition to the real run;
- no whole-seed assertion locks Rest, Smith, or another action for seed006;
- grouped calibration tests enforce seed/root isolation and nominal quantile coverage;
- architecture tests prove there is one Campfire owner and no legacy fallback.

Mechanics tests for Rest, Smith, Toke, Dig, Lift, Recall, Dream Catcher, and relevant relic/card generation remain engine tests. The new tests must not duplicate those mechanics in policy fixtures.

Post-cutover source guards include:

```powershell
rg "campfire_policy_v1|RestVsSmithVerdictV1|RestVsSmithPlanV1|MIN_INSPECT_ONLY_SMITH_BRANCH_SCORE" src
```

The command must return no production-source matches. Focused Campfire and owner-audit tests run during development; completion also runs the repository's required library and architecture boundary suites.

## Alternatives Rejected

- Adding `StrategicSnapshot`, `ScenarioWindow`, and `ActionProjection` beside existing layers would duplicate `RunStrategySnapshotV2`, `route_window_facts`, and current policy projections without proving any migration.
- Keeping the old policy as a fallback would make unsupported evidence silently preserve its thresholds forever.
- Cloning the seeded run and previewing Dig or Dream Catcher would improve apparent decisions by cheating across the player's information boundary.
- Evaluating only a heuristic top-K Smith or Toke targets would make performance depend on the temporary heuristic and could permanently hide the best target.
- Treating recent HP loss as generic defense pressure would confuse slow damage access, multi-target deadlines, and actual mitigation shortages.
- Training a direct action policy first would entangle labels with the current flawed policy. Consequence prediction is inspectable, calibratable, and reusable without granting the model semantic ownership.
- Migrating route, reward, shop, and Campfire together would make deletion and failure attribution impossible to audit.

## Completion Criteria

The project is complete only when:

1. the offline consequence producer passes the stated feasibility and information-safety gate;
2. all legal Campfire candidates are represented under one consequence contract;
3. owner-audit executes one `CampfireDecision` and branch tooling only exposes its alternatives;
4. stochastic choices use public distributions rather than realized future RNG;
5. old Campfire policy code, configuration leaks, thresholds, tags, and fallback paths are deleted;
6. source guards, focused tests, full library tests, and architecture boundary tests pass;
7. the production policy surface has fewer files and lines than before the migration.

Passing these criteria authorizes later migrations to reuse proven concepts, but it does not automatically create a global planning framework. The next boundary must earn its own deletion-driven cutover.
