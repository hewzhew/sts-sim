# Challenger Strategy Repair Loop Design

**Date:** 2026-07-12

## Purpose

The durable trajectory diagnostics can now show that a run reached a known boss with unresolved
construction problems and that relevant cards appeared earlier. The challenger policy still
cannot turn that evidence into a later construction action:

- pressure is copied into a challenger only when the lane is created;
- later deck and known-boss context does not refresh the policy;
- a package that already has sources but lacks a payoff does not open a commitment by itself;
- every `Reject` candidate is ineligible even when it directly repairs an active challenger task.

This delivery closes that action loop without changing the production baseline. Its success
criterion is that a challenger can select one legal candidate that the baseline rejected, record
the exact repair task and override reason, and continue as the same bounded policy lane. A win is
useful downstream evidence but is not an implementation requirement.

## Considered Approaches

### Add card- or boss-specific score bonuses

Giving named cards such as Feel No Pain, Spot Weakness, or Whirlwind more score would be quick, but
it would not connect diagnosis to policy memory. Every new seed would need another exception, and
the system could not explain which unresolved task authorized the action. This approach is
rejected.

### Let challengers execute any rejected candidate

This would remove the immediate wall but would also admit unaffordable, unsupported, stale, or
operationally unsafe choices. It would erase the distinction between an uncertain strategic
candidate and one that cannot be executed safely. This approach is rejected.

### Add an evidence-gated strategy repair path

The selected approach derives current repair context from the run, preserves the production
choice, and gives only challenger lanes a narrow path through ordinary policy rejection. A repair
must match an open pressure hypothesis or a specific active commitment. Hard candidate filters
remain authoritative.

## Decision Context

At every registered owner boundary, derive a typed challenger decision context from the current
`RunState`. The context contains:

- the current `DeckPlanSnapshot`;
- current gold for shop affordability evidence;
- open static inventory pressure;
- pressure derived from the known boss's typed missing-answer profile;
- package commitments justified by the current deck inventory.

The context is an input to policy planning, not persistent run-control state. The strategy layer
owns its derivation and interpretation. Owner-audit passes the current run state and legal choices
to the strategy planner; it does not define card or boss rules.

Missing boss identity produces no encounter pressure. Missing admission or an unsupported
candidate produces no semantic repair response. Neither case is guessed from labels or card
names.

## Pressure Derivation

Static `Missing` or `Thin` inventory continues to open the existing pressure axes:

- frontload opens `ResolutionTempo`;
- area or minion control opens `MultiTargetControl`;
- block or mitigation opens `DelayCapacity`;
- boss scaling opens `GrowthHorizon`;
- access or playability opens `Deployability`.

Known-boss missing answers add encounter evidence without asserting a single cause. A missing
answer may expose more than one valid response axis. The first delivery uses this complete,
deliberately small mapping:

| Missing answer | Response axes |
| --- | --- |
| `DarkEchoBlockPlan` | `DelayCapacity` |
| `HasteBurstOrSetupPlan` | `ResolutionTempo`, `GrowthHorizon` |
| `FocusedKillOrderPlan` | `ResolutionTempo` |
| `ChampTransitionBurst` | `ResolutionTempo`, `GrowthHorizon` |
| `ExecuteBlockPlan` | `DelayCapacity` |
| `CollectorMinionPlan` | `MultiTargetControl` |
| `Block50OrKillBeforeBeam` | `DelayCapacity`, `ResolutionTempo` |
| `StasisRecoveryPlan` | `Deployability` |

`PhasePowerPlan`, `TimeWarpCounterPlan`, `ArtifactStripPlan`, and `TurnFourDebuffPlan` remain typed
diagnostic evidence in this delivery but do not authorize a rejected card. Their answers do not
fit the existing five pressure axes precisely enough for a safe repair gate.

Every boss-derived hypothesis retains the original typed missing-answer label as
`EncounterThreat` evidence. The axis is a response direction, not a claim that the observed HP loss
was caused by missing damage or missing defense.

Static adequacy alone never changes a pressure hypothesis to `Covered`. When a later deck snapshot
no longer reports `Missing` or `Thin` for the corresponding static gap and no mapped current-boss
answer keeps the axis open, reconciliation changes the remembered hypothesis to
`PartiallyCovered`. It is retained for diagnostics but cannot authorize a rejected candidate. HP
loss or search failure remains unattributed unless separate evidence identifies an axis.

## Package Commitment Derivation

The first delivery adds only one automatic package transition: an Exhaust package with at least
two source units and no payoff opens an `ExhaustEngine` commitment requiring `Payoff`, with a
`CurrentActBoss` horizon.

This rule uses `DeckRoleInventory` semantics, not card IDs. In the investigated deck, True Grit and
Burning Pact provide the source inventory; Feel No Pain or Dark Embrace can respond through the
existing Exhaust support semantics. A single incidental Exhaust source is not enough to create
the automatic commitment.

Opening is idempotent. An already active or completed equivalent commitment is not duplicated.
Selecting a candidate that semantically provides the missing payoff completes the package
requirement, but it does not mark boss pressure as covered. The package closure and the survival
claim remain separate evidence.

Other automatic package transitions are out of scope. Existing commitments opened by challenger
choices continue to work as before.

## Challenger Selection

The baseline always keeps its existing production choice. Challenger selection is ordered by:

1. direct support for an active commitment;
2. response to that lane's already active open pressure;
3. response to newly derived current pressure;
4. the ordinary production fallback.

When a challenger selects a response to newly derived pressure, only the matched hypotheses are
added to that lane's policy memory. This preserves a lane's strategy focus instead of copying the
entire global backlog into every challenger. Repeated axes merge evidence deterministically.

If a challenger ID remains available, the baseline may seed a new challenger from a repair-eligible
candidate while still expanding only its own production choice as baseline. Existing global lane
limits, branch budgets, and semantic challenger deduplication remain unchanged.

## Reject Safety Gate

The planner distinguishes early hard filtering from a candidate that completed evaluation and was
then assigned to `Reject`.

An early filtered candidate is never repair eligible. This includes unsupported candidate kinds,
missing admission, unaffordable shop items, unsafe shop sequencing, unresolved operational risk,
and other filter-pass rejections.

An evaluated card candidate may be repair eligible under these rules:

- a scored rejection with no rejecting lane cap may respond directly to an open pressure or active
  commitment;
- a rejection caused by acquisition or role-saturation caps requires direct support for a specific
  active commitment; sharing only a broad pressure axis is insufficient;
- the candidate must still carry the executable legal owner command already produced for that
  boundary.

The repair gate never rewrites the candidate's production lane, score, cap, or inspection reason.
It authorizes one challenger expansion and records that override as counterfactual policy evidence.

## Policy Evolution

Before selection, a challenger reconciles current package commitments and the status of pressure
already in its memory. After selection:

- matched current pressure is merged into policy memory;
- newly opened commitments are recorded;
- specifically satisfied commitment requirements are completed;
- divergence count and checkpoint identity advance as today;
- unmatched active commitments retain their existing horizon and expiry behavior.

Taking one card does not certify a boss plan. A subsequent deck snapshot can reduce a static
hypothesis to partial coverage, a typed boss profile can show that a missing answer disappeared,
and later combat evidence can remain unknown or contradict the repair. This delivery does not
infer success from acquisition alone.

## Durable Selection Evidence

`PolicyExpansion` carries optional typed selection evidence into the resulting `BranchPathStep`.
The additive path field records:

- selection class: production, ordinary challenger, pressure repair, or commitment repair;
- matched pressure axes and commitment kinds;
- original candidate lane and inspection reason;
- whether a `Reject` was overridden;
- the divergence checkpoint reference.

The existing candidate pool remains the source of original scores, raw/final lanes, lane caps,
admission, and semantic pressure response. The new evidence does not duplicate those structures.
Together they must answer both "why did the challenger choose this?" and "what did the production
policy originally say?"

The path field uses serde defaults so existing frontier and capsule data remains readable. Policy
state already persists inside `BranchPolicyLane`; no separate artifact or checkpoint format is
introduced.

## Failure Behavior

- If decision context cannot derive a repair, planning falls back to existing behavior.
- If no candidate passes the repair gate, an existing challenger uses the production choice.
- If applying an authorized command fails, the branch uses the existing `ApplyFailed` status and
  retains the attempted repair evidence.
- If a commitment reaches its horizon without its requirement, it expires rather than silently
  disappearing.
- If pressure evidence is only partial or unknown, it cannot authorize a rejected candidate.
- If two challengers converge semantically, existing frontier deduplication may keep one; the lane
  cap is not raised to preserve card-level variety.

## Ownership Boundaries

- `ai::strategy` owns context derivation, pressure/commitment reconciliation, candidate response,
  repair eligibility, and challenger choice ordering.
- `ai::boss_mechanics_v1` remains the typed source of known-boss missing answers.
- owner-audit constructs legal choices, passes current state into planning, executes the selected
  command, and projects typed selection evidence into branch paths.
- run-control does not read policy tasks, choose cards, or interpret repair evidence.
- combat search does not consume construction commitments or candidate override reasons.
- the production decision pipeline remains unchanged in this delivery.

## Verification

Use a small number of structural contracts rather than seed-outcome assertions:

1. the baseline expansion remains byte-for-byte equivalent in selected choice and policy lane;
2. two Exhaust source units with no payoff open one payoff commitment, while one source does not;
3. an early hard-filtered Reject cannot pass the repair gate;
4. a scored Reject can answer open pressure, while a cap-rejected candidate requires direct
   commitment support;
5. selecting Exhaust payoff completes only the package requirement and leaves boss pressure
   unproven;
6. branch-path JSON round-trips selection class, matched task, original rejection, and override
   status;
7. a focused owner-planning fixture with a known boss, source-only Exhaust package, and rejected
   payoff proves that the challenger selects the repair while baseline leaves the shop.

Do not assert that a named seed must win, that a particular random reward must appear, or that one
card acquisition proves a pressure axis covered.

Implementation validation runs focused tests, `cargo fmt --all -- --check`, `cargo test --lib`,
`cargo test --test architecture_runtime_boundaries`, branch-tiny compilation, and
`git diff --check`. After permanent-code validation, run one fresh bounded mainline evaluation to
observe whether a real repair opportunity creates a different trajectory. Absence of such an
opportunity in that bounded sample is inconclusive, not a failed contract.

## Non-Goals

- Do not change or promote the production baseline.
- Do not hardcode card IDs, seed positions, or shop inventories into the repair policy.
- Do not infer that HP loss means missing defense or missing damage.
- Do not open all engine/package commitments automatically.
- Do not allow hard-filtered or unaffordable choices through the repair gate.
- Do not increase challenger count, branch budget, search budget, or run wall time.
- Do not declare a run solved because a package requirement was acquired.
- Do not add frontier, checkpoint, or source-replay machinery.
