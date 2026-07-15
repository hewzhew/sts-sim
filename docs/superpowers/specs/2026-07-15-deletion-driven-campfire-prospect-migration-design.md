# Deletion-Driven Campfire Prospect Migration Design

## Goal

Replace the current threshold-driven Campfire policy with one production decision owner that compares the consequences of every legal Campfire action under one explicit public-information, horizon, and continuation-policy contract.

This is the first deliberately narrow migration toward a consequence-first run planner. It must prove that the project can replace an old semantic owner and delete it in the same delivery. It must not add another permanent snapshot, score layer, policy version, or fallback path.

The migration is successful when Campfire decisions are made from explicit run-feasibility, survival, and growth prospects, every legal option remains representable, stochastic rewards do not leak hidden RNG, and `campfire_policy_v1` no longer exists in production source.

A prospect is not an intrinsic property of an action. It is the consequence of a root state, a Campfire action, a public scenario distribution, fixed continuation policies, and a named evaluation horizon. Those evaluation conditions are part of the provenance and are identical across candidates from the same root.

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

The first cutover does not require a learned model or a complete threat ontology. Canonical outcome fields must be representable, but an unsupported timing decomposition remains `Partial` or `Unsupported`; the producer must not fabricate strategic facts to fill a schema.

## Migration Invariants

The delivery obeys these non-negotiable invariants:

1. There is exactly one production Campfire semantic owner after cutover.
2. There is no `campfire_policy_v2`, long-lived shadow policy, or old-policy fallback.
3. A new production reader is not added until its old semantic reader is removed in the same delivery.
4. Unsupported evidence widens uncertainty or keeps alternatives alive; it never silently becomes zero value, rejection, or an invitation to run the old policy.
5. All candidates from one root use the same horizon, continuation contract, public scenario distribution, and information cutoff.
6. Actual outcome variation and uncertainty about the estimated outcome distribution are represented separately.
7. Future decisions may condition only on information revealed before those decisions.
8. Shared abstractions are extracted only when a second migrated production consumer needs them. Campfire may define a small local prospect vocabulary, but it must not introduce a repository-wide `StrategicSnapshot` or duplicate `route_window_facts`.
9. The information available to a decision must match information available to a real player at that point in the run.
10. The legacy Campfire owner and all of its semantic readers disappear at cutover. Raw line count is reported as an audit signal, not used as a substitute for proving deletion.

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

The engine remains the sole authority for legality and mechanics. A canonical `legal_campfire_candidates` boundary enumerates semantic candidates such as `Smith { card_uuid }` and `Toke { card_uuid }`. A deck index may remain a display and input concern, but a decision binds to the existing stable `CombatCard.uuid` and a root-state fingerprint so deck reordering cannot silently retarget it. The boundary uses the same engine rules that validate execution. UI-level placeholder options may remain an interface concern, but policy, owner-audit, and branch tooling must not independently reimplement target legality.

`route_window_facts` remains the source for visible route and encounter-window facts. This migration does not create another route window type.

The upgrade planner reports upgrade consequences, including the best upgrade-debt reduction for each legal Smith target. It does not issue a Rest-versus-Smith verdict. The durable `best_smith_debt_paid` fact moves onto the upgrade plan itself so shop and random-upgrade consumers do not depend on a Campfire decision structure.

The new unversioned Campfire owner produces a `CampfireDecision` containing the evaluation-context fingerprint, root-state fingerprint, selected candidate, its prospect, nondominated alternatives, candidates removed by proven dominance, field-level evidence coverage, and a machine-readable decision reason. Owner-audit verifies the fingerprints before execution. A mismatch is a stale-decision gap, not permission to compute a second verdict. The branch experiment may display or retain alternatives already identified by the same comparator, but it may not define independent score thresholds, tag allowlists, or action semantics.

## Evaluation Context

Every candidate from one root uses one `CampfireEvaluationContext` identifying:

- the public-information cutoff and root-state fingerprint;
- the run goal, distinguishing ordinary Act 3 victory from Heart eligibility or Heart victory when applicable;
- a named finite horizon, initially until the next Campfire or act terminal and capped by the visible route window;
- the `route_window_facts` schema, configuration, and content fingerprint;
- the continuation evaluation profile, including the source commit and the combat, route, reward, event, and shop configurations used by the experiment;
- the public scenario-distribution identifier and mechanics version.

The first migration does not add a fingerprint API to every existing policy owner. A single serialized evaluation profile plus source identity records the frozen continuation contract. Changing any recorded input invalidates cached prospects.

Recall is evaluated as a run-feasibility obligation before survival and growth. The current run contract cannot distinguish every victory target, so production Recall cutover requires an explicit goal source rather than inferring Heart intent from a generic `FirstVictory` label.

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
Frontier(Smith) = nondominated prospects among all legal Smith targets
Frontier(Toke)  = nondominated prospects among all legal Toke targets
```

The evaluator never constructs a Smith-by-Toke-by-route cross-product. Route-window facts and the baseline capability envelope are computed once. Candidate deltas are evaluated in batches, making the ordinary cost linear in deck size.

No top-K heuristic may discard unevaluated legal targets. A candidate may be pruned only when deterministic or statistically valid paired bounds prove it dominated by another candidate. Candidates whose relevant bounds overlap remain nondominated. A small unresolved frontier may receive more expensive simulation or search evidence, but the cheap pass must preserve complete legal coverage.

## Consequence Prospect

A candidate does not receive one universal scalar score. It produces a `CampfireProspect` whose fields carry their own evidence status. A mixed prospect may contain an exact prefix, a calibrated stochastic field, and an unsupported diagnostic without collapsing the entire candidate to one label.

The schema separates actual future variation, such as the distribution of deaths or HP, from uncertainty about an estimate caused by finite samples, search coverage, model error, or distribution shift. A predicted HP p10 and the confidence interval around that estimated p10 are different values.

### Run feasibility

- whether the declared run goal remains achievable;
- mandatory or deferrable key acquisition and its visible deadline;
- irreversible actions that remove a required future option;
- exact or bounded evidence supporting that claim.

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
- `Calibrated` for an estimator empirically validated on its stated domain;
- `Partial` when only some relevant failure modes or outcomes are represented;
- `Unsupported` when the estimator cannot defend a useful bound;
- coverage, sample count, provenance, applicability domain, and epistemic uncertainty supporting that field.

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

Deterministic mechanics are projected from the root through the same authoritative transition kernel used by execution:

- Rest's immediate healing;
- Smith for a particular card;
- Toke for a particular card;
- Lift;
- Recall.

Projection may suppress logging or persistence side effects, but it may not duplicate mechanics or mutate the real run. The evaluator distinguishes exact transitions, chance transitions, and chance followed by a later decision.

Rest with Dream Catcher is the third form: exact healing, then a public distribution over generated reward screens, then a card-reward decision made after the cards are revealed. The existing card-reward owner remains the frozen recourse policy; Campfire must not invent another reward policy or reduce the screen to one random scalar.

Dig and Dream Catcher must not be evaluated by cloning the live seeded state and observing the next actual relic or card reward. That would reveal hidden future information even if the clone is discarded. Instead, their prospects are distributions over publicly eligible outcomes under the real generation rules. The evaluator may enumerate or sample that public distribution using an analysis RNG independent of the run RNG. It may use known pool eligibility and state-visible exclusions, but never the live RNG counter, hidden pool order, or realized future outcome.

Candidate comparisons reuse matched public scenarios only where a semantic alignment is explicitly defined. The original simulator's mutable RNG streams are not replaced with an event-keyed RNG for this migration. When divergent paths cannot be causally aligned, the evaluator uses independent analysis samples and wider uncertainty rather than pretending draw indices represent the same event.

The same rule applies to downstream random consequences reached by otherwise deterministic actions. Exact prefixes remain exact; hidden suffixes become distributions.

## Comparison And Decision Semantics

The comparator implements a robust lexicographic partial order. It uses this order:

1. declared run-feasibility obligations;
2. robustly lower paired death risk;
3. robustly better paired lower-tail HP outcome;
4. better coverage of the earliest represented timed failure modes;
5. better median growth and option preservation;
6. lower epistemic uncertainty when consequences are otherwise indistinguishable.

“Robustly” means a deterministic bound or confidence bound on the paired candidate difference separates. Comparing many Smith and Toke targets requires simultaneous bounds or comparison-with-the-best evidence; dozens of unrelated nominal 95% intervals are not dominance proof. Failure to reject equality is not proof of equivalence.

When the branch budget supports alternatives, the decision exposes the nondominated set. When mainline execution must choose one candidate, it selects the best lower envelope of credible survival, then avoids irreversible loss, then preserves future option value. Unsupported imagined upside cannot displace a well-supported safer alternative. If evidence still cannot distinguish candidates, a deterministic final tie rule selects an executable action while reporting an explicit `decision_gap`. It does not call the removed policy.

Dig is therefore not inherently unsafe and Rest is not inherently correct. A healthy state may accept Dig's variance for strong expected growth; a dangerous state may prefer Rest's stable lower tail. Likewise, a Smith upgrade helps survival only to the extent that the upgraded card can be drawn and deployed before a relevant deadline.

## Predictor Boundary And Feasibility Gate

`CampfireProspect` is a stable consequence contract, not a commitment to a particular machine-learning technique. Its producer may combine exact mechanics, analytic estimates, bounded combat simulation, and a learned outcome predictor. Production contains one authoritative producer for each field, not an analytic policy plus a model policy competing for authority.

Before changing the production Campfire owner, an offline feasibility experiment uses existing Combat Lab captures and generated counterfactual states. Experimental code and artifacts are not wired into run control. If the experiment fails, it may be frozen outside production or removed; the old owner is not partially migrated.

The grouped dataset rules are:

- provenance records simulator commit, mechanics/schema version, information boundary, horizon, continuation profile, scenario generator, search coverage, and label type;
- partitions are assigned by seed and root before counterfactual siblings are generated;
- all counterfactual siblings from one root state remain in the same train, validation, or test partition;
- held-out seeds are not used for feature selection, calibration, threshold selection, or early stopping;
- at least one encounter family is held out as a distribution-shift probe;
- a bounded search result labeled “no win found” is coverage evidence, not an “unwinnable” ground-truth label;
- one exact replay is an exact result for one scenario and continuation, not an exact expected-distribution label;
- paired examples require exact enumeration or sufficiently covered scenarios under the same continuation contract; unresolved pairs count against coverage instead of becoming guessed labels;
- exact replayed outcomes outrank search estimates as labels.

The mandatory production cutover gate is deliberately narrower than the research report. It requires:

- 100% enumeration and cheap evaluation coverage for legal Smith and Toke targets;
- zero uses of live hidden RNG state in stochastic prospect generation;
- identical public inputs reproduce deterministic prospects;
- every compared candidate records the same evaluation context;
- on resolved held-out roots, grouped paired regret is non-inferior to the current policy-derived baseline and no well-covered catastrophic reversal is introduced;
- every mainline case yields an executable decision, while robust preference rate and decision-gap rate are reported separately;
- median batch latency no more than 100 ms and p95 no more than 250 ms for a synthetic 64-card legal-target state on the local reference machine.

A field may claim `Calibrated` only when its own held-out evidence supports that status. Death forecasts report a proper score such as Brier or log loss together with calibration and sharpness. HP distribution fields report pinball loss or CRPS, non-crossing quantiles, and empirical coverage. Decision research reports paired regret, catastrophic reversals, action-family strata, and a risk-versus-coverage curve. These are promotion gates for evidence claims, not a requirement to fabricate every field before deleting the old semantic owner.

Distribution-shift probes lose `Calibrated` status unless empirical evidence supports that domain. They are not required to widen by a particular numerical amount.

These criteria assess consequence quality and information safety. They do not require a specific action on seed006 or a global win-rate claim from a small sample.

## Failure And Coverage Behavior

Transition application failure for an engine-declared legal deterministic candidate is a mechanics or integration bug. It is surfaced and blocks the decision; it is not converted into a low prospect.

Prediction failure is field-local. It widens epistemic uncertainty and produces `Partial` or `Unsupported` for that field without erasing exact prefixes or supported sibling fields. Unknown consequences remain eligible. `Unsupported` must never map to zero, `Reject`, `InspectOnly`, or a sentinel numeric score.

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

The cutover is incomplete if old symbols remain reachable or if a caller can compute a second Campfire verdict. The old policy is not kept for comparison after merge; comparison evidence belongs in the offline artifact produced before cutover. Legacy owner and reader deletion is the correctness gate. Production file and line counts are reported, and unexplained semantic growth blocks review, but raw line count alone cannot prove or disprove simplification.

## Diagnostics

Every decision report includes:

- root-state, horizon, continuation-profile, public-scenario, and mechanics fingerprints;
- all legal candidate families and target counts;
- candidates removed by proven dominance and the bounds proving it;
- nondominated finalists;
- hard run obligations and unresolved feasibility gaps;
- survival and HP distributions with outcome variation separate from estimation uncertainty;
- earliest represented and unresolved timed threats;
- growth and irreversible-cost summaries;
- field-level evidence coverage, applicability domain, and provenance;
- selected action or explicit decision gap;
- confirmation that stochastic evaluation used an independent analysis distribution and did not inspect hidden run state.

Diagnostics must be descriptive rather than authoritative. A tag may explain why the owner chose a candidate, but no downstream consumer may turn that tag into a second policy.

## Verification

Default regression coverage locks architectural and information-boundary invariants rather than temporary strategy opinions:

- engine legality and candidate enumeration agree for every Campfire family;
- candidate execution binds to a stable card UUID and matching root-state fingerprint;
- every legal Smith and Toke target reaches the cheap evaluation pass before pruning;
- deterministic candidates produce identical prospects from identical inputs;
- stochastic prospects do not read or advance the live run RNG and are invariant to its hidden cursor;
- Dream Catcher keeps exact healing, chance reward generation, and the post-reveal reward decision separate;
- all candidates from one root share one horizon, continuation profile, and public scenario contract;
- outcome distributions and epistemic confidence cannot serialize into the same field;
- `Unsupported` fields preserve exact sibling evidence, and unsupported candidates remain eligible;
- dominance pruning occurs only from deterministic or statistically valid paired bounds;
- multi-candidate pruning uses simultaneous comparison evidence;
- owner-audit and branch display consume the same `CampfireDecision`;
- only owner-audit applies the selected transition to the real run;
- stale decisions fail closed rather than targeting a reordered deck;
- no whole-seed assertion locks Rest, Smith, or another action for seed006;
- grouped calibration and comparison tests enforce seed/root isolation and any claimed nominal quantile coverage;
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
- Treating a prospect as action-only would make its meaning drift whenever a continuation policy or horizon changes.
- Cloning the seeded run and previewing Dig or Dream Catcher would improve apparent decisions by cheating across the player's information boundary.
- Modeling Dream Catcher as a random scalar would erase its post-reveal card choice and create a hidden second reward policy.
- Replacing the simulator's original RNG streams with event-keyed randomness would expand the first migration and risk mechanics drift; matched analysis scenarios are used only where semantic alignment is explicit.
- Evaluating only a heuristic top-K Smith or Toke targets would make performance depend on the temporary heuristic and could permanently hide the best target.
- Collapsing Smith or Toke to one family value before global comparison would hide a partial-order frontier behind an undocumented tie rule.
- Treating recent HP loss as generic defense pressure would confuse slow damage access, multi-target deadlines, and actual mitigation shortages.
- Requiring every threat-resolution field or calibration study before cutover would turn a bounded owner replacement into an indefinite research program.
- Training a direct action policy first would entangle labels with the current flawed policy. Consequence prediction is inspectable, calibratable, and reusable without granting the model semantic ownership.
- Using raw line count as the primary migration proof would reward dense code and could hide a second owner behind fewer lines.
- Migrating route, reward, shop, and Campfire together would make deletion and failure attribution impossible to audit.

## Completion Criteria

The project is complete only when:

1. the offline consequence producer passes the stated cutover and information-safety gate under a recorded evaluation context;
2. all legal Campfire candidates are represented under one consequence contract with stable target identities;
3. run obligations, outcome variation, epistemic uncertainty, and field-level evidence are distinguishable;
4. owner-audit executes one fingerprinted `CampfireDecision` and branch tooling only exposes its alternatives;
5. stochastic choices use public chance distributions and non-anticipating recourse rather than realized future RNG;
6. old Campfire policy code, configuration leaks, thresholds, tags, and fallback paths are deleted;
7. source guards, focused tests, full library tests, and architecture boundary tests pass;
8. the legacy semantic owner and its readers have disappeared, with no replacement duplicate policy layer;
9. production policy growth is explicitly audited, while evidence schemas and tests are not penalized merely for making uncertainty visible.

Passing these criteria authorizes later migrations to reuse proven concepts, but it does not automatically create a global planning framework. The next boundary must earn its own deletion-driven cutover.
