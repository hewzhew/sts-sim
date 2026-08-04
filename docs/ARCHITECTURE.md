# Architecture

This file is the maintained architecture contract for current AI, runner, and
artifact work. It replaces the old set of narrow boundary notes.

## Guiding Rule

```text
unified typed representation
  -> explicit phases
  -> pluggable decision owners
  -> execution applies typed decisions without reinterpreting policy
```

Free-form strings are display and provenance only. If a decision needs to be
continued, replayed, compared, or learned from, it needs typed identity first.

## Cargo Package Boundary

The maintained oracle command path has one compile-time dependency direction:

```text
command hosts -> sts_oracle_runtime -> sts_oracle_eval -> sts_simulator
```

`sts_simulator` owns game content, state, engine transitions, simulation, and
stable lower policy layers. `sts_oracle_eval` owns combat evaluation,
exact-search orchestration, run-control, and their artifact contracts.
`sts_oracle_runtime` consumes that public surface and owns branch execution,
scheduling, persistence, and resident services. Command hosts contain only
supported adapters and cross-layer integration contracts; they own no policy
semantics. Lower layers must never import branch runtime or a command host.

Some evaluation, runtime, and command sources still live physically below the
historical root `src/` tree and are attached from their single Cargo owner with
explicit paths. `src/eval` is compiled only by `sts_oracle_eval`;
`sts_oracle_runtime` re-exports its public module without compiling a second
copy. That layout detail is not permission for a reverse dependency or
duplicate owner.

Use `cargo test-core` and `cargo test-control` for their respective unit-test
harnesses, `cargo architecture` for dependency-free source-boundary checks,
and `cargo check-workspace` for every target. Do not merge the harnesses again
through test features or replace them with many integration-test executables.

## AI Layers

New AI code must choose an owner layer before it is written:

- `domain`: stable game facts and vocabulary. No value judgments.
- `analysis`: profiles derived from public state. No scene choice.
- `strategy`: typed deck facts, package state, deficits, admission rules, and
  small shared evaluators used by policies.
- `policy`: thin scene adapters for reward, shop, campfire, event, route, Neow,
  boss relic, and run-choice decisions.
- `runtime`: branch execution, scheduling, journals, replay, capsules, budgets,
  and artifact writing.
- `legacy`: still-required old code that is not the design target.

The intended flow is:

```text
domain -> analysis -> strategy -> policy -> runtime
```

Do not add another scene-local strategic model when reward, shop, route, and
branch retention need the same concept. Shared concepts belong in `analysis` or
`strategy`; scene-specific button mapping belongs in `policy`; applying a
typed action belongs in `runtime`.

## Non-Combat Automation

Run-control automation reduces manual repetition. It may execute bounded
route, reward, shop, campfire, event, run-choice, and combat-handoff decisions.
It is not a teacher label and not proof that a policy is good.

Every automated non-trivial decision has this role:

```text
label_role = behavior_policy_not_teacher
```

Non-combat decision records must stay hidden-free:

- public observations are allowed,
- declared distributions and beliefs are allowed,
- privileged simulator futures are forbidden.

Persisted analysis nodes may retain the owner rank observed when their choice
surface was materialized, but an active `owner` or rank-based steering command
must recompute the current typed owner order and join it to that exact surface
by candidate id. Display labels and stale numeric ranks are not execution
authority.

Automation should stop when the current site lacks a bounded policy answer. Do
not encode stale global rules such as "shops always stop" or "events always
stop"; each high-agency site needs its own owner/compiler boundary.

On an ordinary reward screen, the reward owner claims typed low-agency public
resources before opening a nested card-reward choice. This lets the card owner
observe already-claimed gold, non-conflicting relics, and empty-slot potions
instead of evaluating the card surface against a stale pre-reward run state.

The current route owner has one deliberately narrow elite-growth prior: on
ascension 0 during Act 1, a direct elite arrival may enter the typed
`EliteGrowth` band only while current HP is at least three quarters of maximum
HP, recent durable combat attrition is not high when that fact is available,
and the visible Slime Boss plan is not exposed or potion-backed. An unresolved
or potion-backed Slime Boss plan also makes same-band route comparisons protect
campfire scope before optional elite count. Recovery and funded-liquidity bands
still take precedence. This is a public-state behavior prior, not a claim that
an elite is safe, a hidden-future lookup, or a substitute for measuring the
resulting continuation.

Slime Boss preparation is a shared typed strategy fact. It records capability
coverage, static attack inventory, strong-AoE and burst-finisher sources, exact
damage-potion identities, and whether two distinct potions can cover the opening
and post-split stages. One light AoE source is presence, not an established
post-split plan; the plan requires multiple AoE sources, one typed strong-AoE
source, or one typed burst finisher. Static attack inventory is not presented as
an exact three-turn simulation. `PotionBacked` means that continuation resources
are carrying an unresolved deck plan; it is not equivalent to `Established` and
is not permission to spend those resources on an avoidable elite.

Card-taking owners share the strategy-layer acquisition contract. A shop
adapter must pass the currently available purge price as an explicit
opportunity cost; it must not assume that a purge is still available after it
has already been used. A shared `NoPolicySupport` result is absence of shared
support, so a scene may retain a purchase only when it has stronger typed
durable-asset evidence such as efficient access, persistent required-capability
improvement, package support, or new upgrade scope. A purchase that consumes a
live purge reserve for only scene-local tactical gap coverage remains
speculative. After a visit has already spent gold, another card or potion with
neither shared acquisition support nor durable-asset evidence also remains
speculative; affordability by itself is not a bundle plan. Random card
discovery potions are not existing-deck access and cannot claim draw/energy
consistency coverage from that discovery trait alone.

Relic-sensitive card value also stays typed and public-state-only. Mummified
Hand power tempo records the candidate Power's paid base cost and the number of
owned positive-cost cards that can receive its trigger. Reward and upgrade
owners may consume that fact; they must not infer the relic interaction from a
display label or promote every Power without its package and boss context.

Card-reward ordering preserves policy bands and exact threat-gap closure before
using narrower tie-breaks. Within the same band and after those gap comparisons,
a typed Boss damage-plan improvement takes precedence over the raw count of
generic capability improvements. This tie-break may distinguish an established
long-fight engine from several coarse capability promotions, but it must not
promote a candidate across a stronger band or a real threat-gap closure.

Upgrade redundancy is measured across cards that provide the same capability,
not only duplicate card ids. Weak and Vulnerable duration providers therefore
share coverage groups. Upgrade roles must describe what the upgrade changes:
a card that already draws but whose upgrade changes only Block does not pay an
access-recovery debt.

Card-reward gap closure remains subject to typed marginal-quality gates. A
cost-two-or-more tactical candidate whose only semantic roles are frontload,
Weak, and Vulnerable stays speculative when one of its one-turn debuffs
duplicates an owned source of the same debuff. The gate is duration- and
upgrade-sensitive, and its typed audit fact must remain separate from the
underlying capability delta. Exact duplicates whose only additional value is
a light AoE tactical bundle are likewise low-marginal when their apparent
improvements merely promote already-supported source counts to strong. The
first light AoE source remains eligible to close a real gap, and intrinsically
strong AoE is outside this duplicate gate.

A candidate that injects a persistent Status directly into the draw pile also
carries a typed handling assessment. Evolve draw recovery and Medical Kit's
unrestricted Status exhaust count as covered; hand-exhaust cards and
Fire Breathing count only as conditional because the required pieces still
have to meet in combat. Exhaust payoffs are supporting evidence, not handling
by themselves. Conditional or unsupported persistent draw-pile pollution may
not use a coarse capability delta to rise above the speculative band. Ethereal
Statuses and injections into other zones remain outside this gate.

## Runner And Combat

The runner owns run progression:

- selecting or applying non-combat owner decisions,
- deciding when combat search is allowed,
- setting search budgets and potion policy,
- applying an exact returned combat line,
- saving run capsules, frontier checkpoints, and `CombatCase` artifacts.

Combat search owns only the in-combat problem:

- legal combat action enumeration,
- action ordering, rollout, and search policy,
- exact execution of candidate combat lines,
- combat outcome facts and diagnostics.

Combat search must not decide rewards, shops, events, campfires, routes, branch
retention, or deck-building causes. A combat result can expose a symptom; it is
not by itself a deck-construction verdict.

`CombatCase` is the preferred handoff from runner to combat investigation. If a
branch-tiny combat gap cannot be investigated from a saved case, fix the case
payload or the review entrypoint instead of creating another report format.

A case without a validated typed `production_context` supports isolated combat
replay only. Exact production-state restoration requires that context to bind
the case's exact combat-root hash, normalized run-session fingerprint, and
run-control checkpoint; all three identities must validate before use. Exact
production-owner parity additionally requires a typed owner-policy snapshot;
defaults or caller-supplied guesses do not qualify. A counterfactual or
descendant case must clear this context when its position changes. Display
paths, branch ids, filenames, and synthetic run projection do not upgrade a
case to production parity.

New production case producers leave the legacy display `path` empty. Exact run
identity comes from the validated context, while committed decision history
remains owned by the journal/workspace. Legacy paths stay readable for old
diagnostics but candidate display text is not copied into new case payloads.

Combat evidence manifests embed the case's typed replay identity rather than
repeating an unbound root string. New manifests resolve case, action, and trace
artifacts only relative to the manifest file. Legacy manifests may use their
compatibility resolver, but undeclared files are never promoted by same-stem or
single-case-directory inference.

Potions are run resources. Combat may consider potion actions only when the
runner explicitly opens a potion policy and budget. A diagnostic fact such as
"potion rescue found a win" does not automatically mean the main runner should
spend that potion.

Run-level potion realization and reward acquisition remain separate atomic
decisions. A reward owner may expose a mechanically valid out-of-combat potion
effect before a visible potion reward, but that conversion does not authorize
discarding another potion or invent a context-free ranking among retained and
offered identities.

Strategic search starts from an exact potion-free stage. Resident run search
carries that witness in its combat-work checkpoint; owner-audit staging carries
an opaque replay-adjudicated attempt in memory until the portfolio commits one
line. If the incumbent misses the configured run-quality satisfaction, the
runner may open bounded one-potion lanes across active potion identities. A
spending witness may replace the protected incumbent only by satisfying that
quality contract; a higher-HP result that still misses the target is not
permission to exchange continuation value for a marginal local improvement.
Within each standard strategic stage, an insufficient first win remains the
safe incumbent but does not end that stage; search stops when a new win reaches
the configured satisfaction or the already granted stage allowance ends.
Literal `FirstCompleteWin` survival search keeps first-win termination.
While that incumbent is insufficient, portfolio service stays with the exact
member that produced it until quality is reached or that member completes.
With no productive witness, non-Boss combat retains the ordinary local-graph /
discrepancy round-robin schedule. Each Boss stage instead serves the local
graph as its primary until that member completes or the stage allowance ends;
policy discrepancy remains a bounded fallback when local completion leaves
allowance without an acceptable witness. A later identity inheriting an already
satisfying witness receives one complete local challenge rather than a full
quality-polishing allowance.

Inside the local graph, one shared boundary agenda rotates between the anchor
and the available semantic guide lanes. A guide entry selects one exact
boundary once; it receives 128 coherent generator-work units, while repeated
and exhaustive service remains owned by the anchor. This value controls
preemption granularity, not combat value or admission, and production and
laboratory hosts use the same planner constant. Full local-graph reports retain
the effective value in their search specification so budget-cliff comparisons
remain reconstructible.

An encounter-owned typed service bias may give one existing guide lane a
bounded number of additional turns in that rotation. It does not duplicate
guide entries: each selected boundary remains one-shot, receives the same
quantum, and then leaves completeness to the anchor. The default planner has no
bias. Production currently concentrates the survival view for an all-Darkling
group because reincarnation loops make raw turn horizon unbounded; laboratory
controls may omit or reweight a typed lane for fixed-root attribution. The
effective production bias is retained in each stage trace.

During `ImproveVerifiedWin`, a quality-reaching spending witness remains an
exact candidate but cannot by itself end the refinement quantum. Early
satisfaction requires a clean potion-free witness at the same quality target.
When several clean witnesses reach that target, run control minimizes potion
use before applying the ordinary final-HP and persistent-payoff ordering.
`FindAnyWin` survival rescue keeps its separate first-reserve-compliant-win
contract: it may land below the search-quality target, but never below the
owner's captured strategic survival floor. Guaranteed full-heal boundaries
retain their explicit unlimited limit.
After every configured quality stage is exhausted, analysis advance may
materialize an insufficient-quality fallback only when it still preserves that
survival floor. A lower verified win remains an exact incumbent for diagnosis,
but autonomous run reports budget-unknown and does not silently accept it.
Autonomous refinement gives each active potion identity its own exact search
stage. The current stage's slot mask constrains newly generated or proposed
witnesses; it does not discard an exact-verified incumbent inherited from an
earlier identity stage. That incumbent remains checkpointed and must replay
exactly from the unchanged combat root before a stage promotion or process
restore can retain it. Non-Boss search divides the configured generation-work
allowance by the number of active identities, then gives one equal share to the
potion-free primary and to each concrete identity. Boss search additionally
keeps one final canonical multi-potion fallback and divides generation work by
`active identities + 1`; it reaches that fallback only when the clean and
single-identity stages found no acceptable witness. Thus a high-branching
potion cannot starve a simpler slot. Stage wall allowances include the clean
primary in their divisor (`active identities + 1` for non-Boss and
`active identities + 2` for Boss), so every configured stage receives time
without exceeding the caller's combat wall. Total bounded generation work is
at most `1 + 1 / active identities` allowances for non-Boss combat and
`1 + 1 / (active identities + 1)` allowances for Boss combat, while the
caller's combat wall deadline remains authoritative. Slot order follows
deterministic slot identity, not a potion value ranking.

Opening an exact identity stage changes the legal potion-slot contract, not the
shared action prior. Production does not globally raise every admitted root
potion action to the highest policy weight: that intervention can replace a
useful sparse corridor with a worse potion-first corridor. A laboratory audit
may opt into the old root challenge as an explicit A/B control, but its result
is non-authoritative until selected exact sentinels justify a narrower policy.

Semantic victory stages admit authorized potion use but omit explicit potion
discard. Discard remains simulator-legal and an all-legal diagnostic may opt in
for a concrete slot-generation or revive-priority case; it is not a generic
way to diversify a sparse combat search. Checkpointed incumbents containing a
discard remain ineligible when restored under a semantic stage.

Production combat progress carries the exact root hash and one compact typed
trace row per served stage. A row freezes that stage's slot contract, charged
local/discrepancy work, effective guide-service bias, proposal counts, the local
graph's best exact candidate, its satisfaction and typed portfolio disposition,
the selected incumbent, remaining allowance, and exit reason. Keeping both
candidates distinguishes "search did not find it" from "the run-level quality
gate rejected it."
Completed rows survive workspace checkpoint restore. This trace is diagnostic
evidence for locating an owner scheduling divergence; it does not rank potions
or authorize a policy change.

Pre-A5 Act 1/2 room Boss victories deterministically restore full HP while
retaining the potion inventory. Owner-audit therefore searches those boundaries
without active potion expenditure first and lands any exact clean win; only a
missing no-potion witness opens the remaining high-stakes Boss rescue budget.
Other owner-audit Boss boundaries retain their canonical high-stakes search
contract. The autonomous resident runner uses the clean, single-identity, then
multi-potion fallback schedule above for every strategic Boss search.

Static potion identity tiers may describe audit evidence but do not admit
production spending. Passive death insurance and explicit escape remain
outside active victory search. When the runner opens concrete potion slots,
that exact mask is part of the combat-search engine profile and must be
enforced by atomic expansion, turn-plan enumeration, rollout-witness replay,
fallback repair, and final executable-line replay; a trace-only slot mask is
not an admission contract.

Accepted combat lines must be exact executable lines from the current combat
state. Frontiers, near misses, rollout samples, and dirty diagnostic lines are
evidence, not runnable campaign actions.

The complete-turn planner retains exact terminal witnesses on a typed
non-dominated frontier instead of collapsing every victory into its local
HP-first view. The retained facts include final and maximum HP, recoverable
gold (including pending `StolenGold` rewards), persistent card growth,
external burdens, potion expenditure, and action count. The planner's selected
`witness` remains a compatibility view for tactical progress; it is not the
run-level continuation decision. Run control owns adjudication across the
frontier with its existing satisfaction, potion-quality, and continuation
contracts. Diagnostic reports may expose compact terminal-outcome summaries,
but must not duplicate every retained action line.

An autonomous run wall stops additional search and owner scheduling; it does
not discard a verified incumbent returned by the final admitted search call.
Runtime first materializes that exact combat line as one atomic transaction,
records any wall overshoot, and then stops at the resulting boundary. A
terminal outcome already reached by that transaction takes precedence over a
run-wall stop classification.

### Combat Search Orchestration

Combat search code should keep these phases separate:

```text
portfolio context -> portfolio plan -> search profile -> search execution
                  -> acceptance -> trace/render/rejection
```

`branch_tiny` owns campaign-level portfolio orchestration. It should choose
which search profiles to run, execute them, and commit or reject results. It
must not reinterpret combat strategy hidden inside a lane name.

Combat search profiles are the boundary between orchestration and search
policy. A profile is an explicit bundle of:

- a budget,
- action-prior / phase-guard plugins,
- rollout and frontier plugins,
- potion policy,
- acceptance policy,
- artifact policy.

Changing action ordering should usually add or modify an action-prior plugin.
Changing frontier scheduling should touch a frontier plugin. Changing what
counts as an acceptable result should touch acceptance. Runner code should only
run profiles and apply typed outcomes.

A typed encounter plan may expose categorical action timing to one bounded,
plan-compatible exact prefix proposal. Strategy owns the timing semantics; the
planner materializes the prefix as ordinary graph edges; run-control only gates
the proposal, charges its actual work, and reports diagnostic counts. The
proposal never prunes alternatives, claims a win, or becomes another global
frontier scheduler. Acceptance still requires the ordinary exact witness and
replay contract. Production admission is explicit and evidence-backed per
encounter plan; merely having categorical action timing does not opt a plan in.

## Gap Semantics

Gaps are typed stops, not verdicts:

- `automation_gap`: a non-combat owner boundary has no bounded answer.
- `combat_gap`: current runner/search settings did not produce an acceptable
  executable combat line.
- `budget_gap`: configured wall-clock or slice budget ended.
- `potion_rescue`: diagnostic or retry path found a potion-assisted line.
- `still_no_win_after_review`: review settings still found no accepted line.

None of these proves why the run is bad. The next investigation step must be
explicit: search policy, potion gate, reward/shop choices, deck facts, or owner
coverage.

## Campaign Artifacts

Campaign artifacts are storage and replay surfaces, not strategy authority.
Keep these responsibilities separate:

```text
checkpoint  exact simulator state needed to resume execution
state       scheduler/workset state needed to continue a campaign
journal     append-only decision facts and candidate pools
report      bounded projection for inspection and tools
diagnostic  opt-in sidecar data for large explanations and traces
```

Checkpoint owns exact resume state. State owns scheduling data. Journal owns
decision facts and candidate identity. Report is a cheap projection.
Diagnostics are opt-in sidecars for large or narrow-use explanations.

Capsule campaign history is an immutable `RunTrajectorySegmentV1` DAG. Each
segment contains one ordered `RunProgressJournalV1` plus planner-boundary visit
occurrences; large observations and legal-candidate sets live once in
content-addressed payload tables. Branch checkpoints persist only a verified
trajectory head id and depth. Every pending segment must be committed before a
frontier, cutpoint, terminal result, or soft-pause checkpoint can be written.

Behavior events and raw-horizon outcomes are read-only projections rebuilt by
walking a durable head to its root. A resumable or prematurely stopped head
produces typed censored outcomes, never a fabricated defeat. These events are
behavior-policy evidence, not teacher labels.

`SessionTraceV1` remains available for interactive trace consumers, but it is
not capsule campaign-history authority. The optional `trace.jsonl` output may
render durable head references and can truncate without losing capsule
evidence. Result, summary, coverage, behavior, and outcome files are
rebuildable projections and must not become a second decision-history owner.

Default reports should reference state, journal, checkpoint, and diagnostics
instead of inlining large payloads. Compression is not a license to store
unbounded data.

An oracle analysis workspace is an editable variation workbench, not the
archive authority for every state it has ever materialized. Explorer
checkpoints externalize map graphs, map state, decks, relics, potions, and run
schedules into typed content-addressed payload tables. Replay steps and emitted
events use shared prefix DAGs. Payload hashes declare their algorithm and are
validated during hydration; legacy inline checkpoints remain readable.

One workspace may carry one combat scratch DAG bound to an immutable run combat
node and its exact root hash. The root position remains owned by that run node;
each scratch child persists only its parent id, one typed `ClientInput`, and the
exact successor hash. Restore topologically replays every delta from the bound
root and rejects missing parents, cycles, illegal inputs, transition truncation,
or hash drift. Normal agent play uses node-local hand, potion-slot, and monster
indices paired with an immutable scratch node id. The runtime resolves that
pair to exact card UUID, potion UUID, and monster entity identity before checking
legality; a local index never becomes durable identity or resolves a display
label. End-turn, atomic, and structured-selection selectors follow the same
node-bound rule, and selectors can fork directly from retained nodes. The
diagnostic view retains internal identities and exact-hash action refs, but the
compact observation omits them and exposes effective card costs, legal local
targets, a top-first draw order, combat relic state, locked monster move steps,
relevant monster runtime state, and node-local structured-selection domains.
Scratch navigation never creates run variations. A bounded
descendant search may append a replay-verified potion-free suffix to scratch,
but only an explicit terminal-victory commit may materialize the complete root
prefix as one atomic combat witness and clear the scratch DAG. Run journals do
not record individual scratch card actions.

A fresh active workspace may also be rebuilt from one exact committed node
after journal and final-state fingerprints are verified; the historical
workspace remains immutable evidence. This bounds the active edit loop without
pretending that a one-node continuation preserves the discarded variation DAG
or resident combat-search frontier. Repacking only changes storage and retains
the complete variation DAG; compaction deliberately starts a new one-node
workbench. Neither operation overwrites its source.

## Journal And Candidate Identity

Decision history belongs in the journal. It records:

- the decision boundary,
- branch and checkpoint context,
- available candidates,
- stable candidate ids and typed summaries,
- candidate admission and disposition,
- selected or applied candidates when a policy chose one.

Every decision needs a stable `decision_id`. Every candidate needs a stable
`candidate_id`. Display labels, command strings, and rendered summaries must
not be parsed for control flow.

Candidate admission is the structured scheduling trace:

- `admission.status`,
- `reason_category`,
- `reason_code`,
- `source`,
- `lane`.

Route, map, reward, shop, event, campfire, boss-relic, and run-choice
candidates should carry typed identity that can be continued without
recovering meaning from text.

## Report Field Admission

Reports, journals, summaries, and learning samples are interfaces. A quick
field can become an accidental policy surface.

Every new output field should be one of:

- `fact`: raw state or candidate data.
- `diagnostic`: intermediate view for debugging a model or scheduler.
- `verdict`: explicit conclusion with a named evaluator and evidence limits.
- `label`: training or evaluation target with a documented source.

If a field does not fit one of these classes, do not add it. Do not present
diagnostic extremes such as `furthest`, `best_hp`, or `cleanest` as winners
unless the evaluator really supports a winner claim.

Tests should protect stable structure, not prose. Avoid tests whose main
assertion is a human-facing adjective.

Potion spend-urgency output is a diagnostic question, not a verdict. It may
place an exact-root reserve delta beside validated inventory, supply, route,
recovery, and shop facts, but it must not collapse them into a score or spend
threshold. Route order must come from a typed modal ordering fact; aggregate
room counts cannot establish order. Missing, unknown, conflicting, or
root-mismatched evidence remains explicitly unavailable or rejected.

Reward-screen potion replacement belongs to the run-control reward owner. A
replacement must select a typed slot, potion identity, and UUID; the runtime
must not recover that identity from a label or generic discard command. The
owner may replace only when a bounded public-state contract discharges the
specific opportunity cost. The current production contract is deliberately
narrow: an incoming Fruit Juice may replace the newest duplicate inventory
identity, and an incoming Strength Potion may replace Fear Potion only when
card semantics confirm both a concrete Strength payoff and deck-based
Vulnerable coverage. All other full-inventory comparisons remain unresolved.

Before replacement is necessary, reward transaction order must preserve
deterministic run resources. When a claimable Fruit Juice shares one reward
surface with ordinary potion rewards, the reward owner claims Fruit Juice
first so the existing potion-space step can realize it before any remaining
reward would require replacement. This ordering is driven by typed potion
identity, not reward labels.

Event-origin deck selections must include the event's typed post-selection
effect instead of treating every `PurgeNonBottled` boundary as an ordinary
cleanup. Bonfire sacrifice ordering may prefer a selectable Uncommon or Rare
card without a core-function or unsupported-loss classification when its
exact public-state recovery is at least the owner's strategic reserve
magnitude, even if current HP has not yet crossed below that reserve. Mark of
the Bloom remains an explicit healing blocker in that projection.

Deterministic post-victory HP carryover is also an owner-captured fact. In
particular, an offline audit must not infer a room-Boss act-transition heal from
an encounter name, floor number, or combat `is_boss` flag. The run-control
owner evaluates the exact combat context and persists the typed result;
diagnostics may validate and display it, while legacy absence remains
unavailable.

The strategic survival and search-quality HP-loss limits used by an owner are
owner-captured facts too. A combat audit may compare exact no-potion and potion
witnesses against those captured limits, but it must not reconstruct the limits
from final HP, a hand-configured reserve, or a copied policy formula. Crossing a
quality limit is diagnostic evidence that a local improvement matters to the
current owner contract; it is not by itself authority to spend a potion.

The most recent completed combat's start HP, pre-relic end-combat HP, raw loss,
turn count, and potion count are a compact owner fact rather than a diagnostic
trace. This fact survives external checkpoints even when detailed combat
outcomes and search traces are cleared. Legacy checkpoints without it remain
explicitly unavailable; route policy must not reconstruct it from net map HP.

## Prohibited Crossings

- Do not use strings as decisions when a typed action, candidate key, or case
  field exists.
- Do not let combat review mutate runner policy.
- Do not let combat search choose non-combat owner actions.
- Do not let runner code inspect hidden futures except through explicit
  diagnostic experiments.
- Do not add another summary/report layer when a capsule `summary.json`,
  `CombatCase`, journal event, or existing review output can carry the fact.
- Do not preserve a duplicate module just because migration is uncomfortable.

## Change Rule

Any change that moves behavior across these boundaries must update this file in
the same commit. Small search heuristics, runner retry gates, potion policies,
owner bridges, artifact shapes, and report fields all count when they change
who owns a decision.
