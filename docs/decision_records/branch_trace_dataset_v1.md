# Branch Trace Dataset V1

## Decision

The AI mainline records branch outcomes instead of action labels.

`BranchTraceV1` is the first contract for this path:

```text
DecisionEnv timestep
-> force candidate action
-> continue with an explicit continuation policy and horizon
-> record public transition summaries, rewards, terminal/truncated state, and outcome
```

`BranchComparisonV1` compares two traces from the same decision and horizon. It is branch outcome data, not a policy preference oracle.

Supported horizon modes in the initial driver entry point:

- `fixed_decisions`: force the candidate and continue for at most `horizon_decisions` behavior decisions.
- `combat_end_v1`: force the candidate and continue until combat win/death/truncation, or until the same `horizon_decisions` cap is reached.

Unsupported adaptive modes must fail clearly rather than silently falling back to a different boundary.

`combat_end_v1` has explicit censoring semantics:

- If combat/death/natural terminal is reached, `boundary_reached=true` and `outcome_censored=false`.
- If the horizon cap fires before combat end, `boundary_reached=false`, `truncated=true`, `outcome_censored=true`, and `truncation_reason=horizon_cap_before_combat_end`.

Those capped traces may still be useful fixed-horizon branch data, but they are not complete combat-end labels.

## Scope

This contract is allowed to store:

- public observation and public candidate set from the decision point
- forced action prefix
- continuation policy id
- horizon spec
- public transition summaries
- reward events
- final branch outcome
- debug info for audit

This contract is not allowed to produce:

- selected action
- better action
- action imitation label
- neutral/exact/frontier preference
- live takeover command

Every trace currently has:

```text
trainable_as_action_label = false
```

## Rationale

Previous selector-style routes were rejected as policy sources:

- one-step delta selector
- generic dominance
- neutral hypothesis action
- exact-turn best_line
- frontier_eval value

Their useful output is diagnostic evidence, not action truth. The next trainable object is branch outcome/value/risk/search allocation, not selected action imitation.

## Current Entry Points

- Rust driver command: `branch_trace`
- Python collector: `tools/learning/collect_branch_traces.py`

The driver command clones the current `FullRunEnv` and must not advance the live environment. The collector follows a behavior policy only to visit states and writes JSONL branch trace batches for offline analysis/training.

The collector currently defaults to combat decision types only (`--decision-type-prefixes combat`) and `candidate_scope=controlled_v1`, so event/map/reward screens are visited by the behavior policy but not traced as combat branch outcome data. Passing an empty prefix list intentionally traces all decision types, but that is not the V1 combat data default.

The driver also applies candidate scope filtering inside `branch_trace`; script-side filtering is not the only guard.

## Quality Gate V1

Every `branch_trace` batch now emits a `validation_report`.

The validator checks only dataset invariants:

- branch traces are versioned and have a non-empty `state_hash_before`
- forced prefixes and forced action keys match
- branch traces are not action labels
- redaction says model input is public observation and hidden state is not in the observation
- comparisons only pair branches from the same decision, same `state_hash_before`, same horizon, same continuation policy, same sim/content version, and same seed
- comparison roles must not become action-label-like
- `combat_end_v1` cap-before-combat-end traces must be censored/truncated and cannot masquerade as full combat outcome data

This is deliberately not a policy-quality validator. It cannot say a branch is better; it only says the branch outcome record is structurally safe enough to keep.

The driver also emits a `candidate_sampling_spec_v1` object. It records the scope, requested/included/excluded counts, and explicitly states that neutral signals, legacy best moves, exact-turn best lines, and frontier eval scores are not sampling sources for this command.

## Paired Scenario V0

`BranchComparisonV1` now carries paired-scenario metadata. The current mode is intentionally narrow:

```text
pairing_schema_version = paired_scenario_v0
pairing_mode = same_initial_env_seed_single_scenario_v0
common_random_policy = shared_initial_rng_no_realignment_v0
```

This means both branches start from the same exact environment and RNG fingerprint. The evaluator does not try to realign RNG streams after different branch actions consume randomness differently. Instead, each trace records:

- `scenario_seed_id`
- `rng_state_before_hash`
- `rng_state_after_hash`
- `rng_consumed`

Each comparison records:

- `scenario_seed_id`
- `rng_before_hash`
- `left_rng_after_hash`
- `right_rng_after_hash`
- `rng_diverged`
- `rng_divergence_reason`
- `paired_validity_status`

So paired comparisons remain outcome diffs, not action preferences. A comparison can be valid as a shared-initial-scenario comparison while still reporting `rng_diverged=true`; downstream training must decide whether that comparison is suitable for a specific value/risk/search-allocation target.

The validator rejects missing pairing metadata, mismatched initial RNG fingerprints, mismatched scenario ids, inconsistent `rng_diverged` flags, and invalid pairings without reasons.

## Branch Value / Risk Export V0

`tools/learning/export_branch_value_risk_dataset.py` converts branch trace JSONL into two training-data candidates:

- `branch_value_risk_example_v0`
- `branch_pair_outcome_diff_example_v0`

The default export is conservative:

- branch rows require complete, uncensored `combat_end` traces
- pair rows require complete combat-end traces on both sides
- pair rows require valid shared-initial-scenario pairing
- pair rows exclude RNG-diverged pairs unless explicitly requested
- every row keeps `trainable_as_action_label=false`

The pair dataset is an ordered outcome-diff dataset. It is not a winner/preference/action-label dataset. It may be used for branch value, risk, or search-allocation modeling, but not for direct action imitation.

## Value / Risk Feature Audit V1

`tools/learning/train_branch_value_risk_baseline.py` is a dependency-free learnability and audit script. It trains small hashed-linear baselines for:

- branch `hp_delta`
- branch `total_reward`
- branch HP-loss risk thresholds
- ordered pair outcome diffs

It can also emit:

- `branch_value_risk_prediction_v0`
- `branch_pair_outcome_diff_prediction_v0`

These prediction files are audit artifacts. They keep `trainable_as_action_label=false`, do not contain selected actions, and do not contain winner/preference labels.

The first 100-seed H12 audit found useful branch-level signal:

```text
branch hp_delta R2 ~= 0.54
branch total_reward R2 ~= 0.59
hp_loss>=5 AUC ~= 0.91
hp_loss>=10 AUC ~= 0.93
```

It also found that pairwise HP differences are much harder than branch value:

```text
pair hp_diff R2 ~= 0.08
pair hp_diff nonzero sign accuracy ~= 0.85
```

The largest pairwise HP error bucket is `end_turn -> play_card`, especially when the true HP difference is large. This should guide the next data/model work toward richer branch context and pair/search-allocation targets, not toward direct action imitation.

## Branch Outcome Model V1.1

`BranchOutcomeModel V1.1` adds decision-context and opportunity-cost features to the exported dataset:

- legal candidate count
- playable card/candidate counts
- playable attack/block/draw/debuff/exhaust/setup counts
- public damage/block opportunity totals
- `end_turn_with_playable_cards`
- `end_turn_with_unspent_energy`
- incoming damage minus current block

These are public decision-context features. They describe what actions were available at the decision point; they are not policy rules and do not mark any action as preferred.

The V1.1 audit also adds non-policy training targets:

- decision-centered branch advantage, such as branch HP delta minus same-decision mean
- pair residual correction over branch-model differences
- pair tail classifiers such as `abs(hp_diff)>=5/10/15`

The first V1.1 100-seed H12 result:

```text
branch hp_delta R2 ~= 0.53
branch total_reward R2 ~= 0.56
hp_loss>=5 AUC ~= 0.91
hp_loss>=10 AUC ~= 0.92

pair hp_diff from branch models:
  R2 improved from ~=0.08 to ~=0.23

large-pair sign remains good:
  abs(diff)>=10 sign accuracy ~=0.90
  abs(diff)>=15 sign accuracy ~=1.00
```

Two important negative results were also recorded:

```text
decision-centered advantage regression is weak with this shallow linear model
pair residual correction improves severe-underestimation detection but worsens HP-diff MAE
```

So V1.1 is useful as feature/target audit evidence, not as a policy. The next useful direction is richer pair/tail/search-allocation modeling and harder state coverage, not action imitation.

## Hard-State Mining V0

`tools/learning/mine_branch_hard_states.py` consumes pair prediction audit rows and produces hard-state sampling targets. It is a data/search allocation tool, not a policy.

The miner prioritizes pairs when it sees signals such as:

- true large HP outcome gaps
- branch-model severe underestimation
- residual model recovering tail magnitude
- high predicted pair tail probability
- EndTurn-vs-play-card opportunity-cost pairs
- high incoming damage
- multi-enemy states
- large public damage opportunity

The first 100-seed H12 mining run produced:

```text
input pair predictions: 965
candidate hard pairs before cap: 788
emitted hard pairs: 500

top reasons:
  end_turn->play_card opportunity-cost pairs
  high incoming damage
  large public damage opportunity
  multi-enemy states
  true abs(hp_diff)>=10
  branch-model severe underestimation
```

Hard-state rows keep `trainable_as_action_label=false`. They are intended to drive targeted recollection, deeper branch evaluation, and search-allocation training.

## Targeted Recollection V0

`tools/learning/collect_targeted_branch_traces.py` replays behavior-policy
episodes to mined hard-state decision points and collects deeper branch traces.
It preserves the normal `branch_trace_collection_record_v1` shape so existing
exporters can consume the output.

The first smoke used the top 50 hard-state rows:

```text
target hard rows:        50
unique target decisions: 31
reached target decisions: 31
missed target decisions:  0

horizon: combat_end_v1 / H16
branch traces: 208
pair comparisons: 625
matched target pair action keys: 81
missing target pair action keys: 0

live_env_changed_count: 0
validation_issue_count: 0
redaction_violation_count: 0
trainable_action_label_count: 0
winner_or_preference_field_count: 0
```

The exported targeted dataset contained:

```text
branch rows: 208
pair rows:   625
rng-diverged pairs: 0
```

This confirms hard-state mining can drive a focused recollection pass without
using neutral/exact/frontier selectors as authorities.

The hard slice is intentionally biased toward difficult cases. A small 14-seed
smoke split found:

```text
branch hp_delta R2 ~= 0.58
hp_loss>=5 AUC ~= 0.89
hp_loss>=10 AUC ~= 0.64
pair hp_diff R2 from branch models ~= 0.51
pair abs(hp_diff)>=10 AUC ~= 0.92
```

The useful result is not that this small slice is a stable benchmark. The useful
result is that targeted recollection enriches exactly the distribution we need:
high HP-loss, large pair gaps, and EndTurn-vs-play-card opportunity-cost cases.
The low `hp_loss>=10` branch AUC and remaining severe-underestimate misses show
that hard-state expansion should continue before treating the model as a strong
allocator.

## Search Allocation Signal Audit V0

`tools/learning/evaluate_search_allocation_signals.py` evaluates whether current
branch/pair model outputs are useful for assigning deeper search budget. It does
not choose an action and does not create action labels.

The audit groups rows by decision and asks questions like:

```text
If budget K permits deeper evaluation of K branches, does a score include:
  the true best branch by branch outcome?
  the true worst/high-risk branch?

If budget K permits deeper evaluation of K branch pairs, does a score include:
  pairs with abs(hp_diff)>=5/10/15?
  pairs where the branch model severely underestimates a large gap?
```

The first 100-seed H12 audit used:

```text
branch predictions: 884
pair predictions:   965
```

Useful signal was present:

```text
branch pred_total_reward, budget 1:
  true-best-branch decision recall ~= 0.84

branch hp_loss_ge_10 risk, budget 1:
  hp_loss>=10 decision recall = 1.00

pair tail_abs_ge_10 probability:
  budget 1 abs(hp_diff)>=10 decision recall ~= 0.80
  budget 2 abs(hp_diff)>=10 decision recall ~= 0.95
  budget 3 abs(hp_diff)>=10 decision recall = 1.00
```

This supports the current mainline interpretation:

```text
branch/pair models are not policy controllers;
they are becoming useful search-allocation and hard-state mining signals.
```

The audit output remains a search-allocation report. It contains no `winner`,
`preferred_action`, `selected_action`, `teacher_choice`, or trainable action
label fields.

## Search Allocation Gate V1

`tools/learning/evaluate_search_allocation_gate.py` turns the previous signal
audit into a formal pre-search gate. It still is not a policy:

```text
model/audit signal
-> allocate deeper branch or pair evaluation budget
-> report coverage and misses
```

It does not output action labels, winners, preferences, selected actions, or
takeover decisions. Its output uses allocation language only:

- `allocated_items`
- `missed_target_items`
- `trainable_role = search_allocation_recollection_target`
- `trainable_as_action_label = false`

The first gate combines the 100-seed H12 baseline slice with the targeted
hard-state H16 slice. Required checks focus on cases where current signals are
already strong enough to be useful as search allocation:

```text
baseline branch hp_loss>=10 risk, budget 1
baseline pair abs(hp_diff)>=10 tail signal, budget 2
baseline pair abs(hp_diff)>=15 tail signal, budget 2
hard branch hp_loss>=10 risk, budget 1
hard pair abs(hp_diff)>=10 tail signal, budget 3
hard pair abs(hp_diff)>=15 tail signal, budget 5
```

The severe-underestimate objective is tracked as a watchlist target rather than
a required gate. A miss there means "collect or model this harder case next",
not "choose a different action now".

The gate also reports random-expected recall and lift. This prevents a high
recall number from looking meaningful when a budget would have covered the same
cases by chance.

## Gate-Driven Targeted Recollection V1

`tools/learning/build_gate_recollection_targets.py` converts
`search_allocation_gate_v1` miss rows into targeted recollection rows consumable
by `collect_targeted_branch_traces.py`.

The converter keeps the distinction between gate failure and policy decision:

```text
gate miss
-> recollection target
-> deeper BranchTrace batch
```

It does not produce action labels or preferences. The target rows use:

```text
trainable_role = search_allocation_recollection_target
trainable_as_action_label = false
label_policy.action_label = false
```

The first baseline+hard gate miss queue contained:

```text
gate miss rows:       6
recollection targets: 20
target decisions:     5
required-gate targets: 8
watch-gate targets:   12
```

The watch targets are dominated by
`branch_model_severe_underestimate_ge_10`, which remains the main blind spot for
future model/search-allocation work.

The first H20 recollection pass reached all gate targets:

```text
target decisions reached: 5 / 5
target rows reached:      20 / 20
branch traces:            31
pair comparisons:         86
missing target keys:      0

live_env_changed_count:             0
validation_issue_count:             0
redaction_violation_count:          0
trainable_action_label_count:       0
winner_or_preference_field_count:   0
```

Exporting that recollection produced:

```text
branch rows: 31
pair rows:   86
rng-diverged pairs: 0
```

This is a narrow hard supplement, not a standalone benchmark. Its purpose is to
feed the next targeted training/evidence pass on the specific cases the gate
missed.

## Branch Outcome Model V1.2 Gate Supplement

`tools/learning/merge_branch_value_risk_datasets.py` merges baseline branch
datasets with hard recollection supplements while namespacing branch ids. This
prevents identical `seed:step:branch:n` ids from different horizons from
colliding inside pair lookup.

The first V1.2 merge used:

```text
baseline H12 branches:   4175
baseline H12 pairs:      4439
top50 H16 branches:       208
top50 H16 pairs:          625
gate H20 branches:         31
gate H20 pairs:            86

merged branches:         4414
merged pairs:            5150
```

The model was retrained with the same seed-based split and then predictions
were sliced back into:

```text
baseline: baseline source rows
hard:     hard_top50 + gate_h20 source rows
```

This keeps the gate comparison honest: the gate rows remain hard heldout rows
under the existing split because their seeds are in the test partition.

V1.2 model-level smoke:

```text
train branches: 3291
test branches:  1123
pairs:          5150

hp_delta R2 ~=       0.47
total_reward R2 ~=   0.50
hp_loss>=10 AUC ~=   0.92
```

The important result is the search-allocation gate, not raw regression score.
Required checks still pass, and the previous severe-underestimate watch failure
is now covered:

```text
previous hard branch_model_severe_underestimate_ge_10 K5 recall: 0.0
V1.2 hard branch_model_severe_underestimate_ge_10 K5 recall:     0.9375
```

This does not make the model a policy. It means gate-driven recollection changed
the hard search-allocation signal in the intended direction. The next failure
mode to watch is that `hard abs(hp_diff)>=10` K3 recall moved down to the
required threshold neighborhood, so future gate versions should track both
tail-gap coverage and severe-underestimate coverage together.

## Evidence-Conditioned Model V2

`tools/learning/export_branch_value_risk_dataset.py` now exports a narrow
`evidence_features` object for each branch row and pair side. These features are
limited to the first public transition summary after the forced branch action:

```text
evidence_scope = one_step_public_transition
evidence_horizon_lt_label_horizon = true
```

This evidence is intentionally shorter than the combat-end label horizon. Full
rollout length fields such as public summary count and reward event count are
not used as model features, because they would be label-adjacent combat-duration
signals rather than one-step evidence.

The V2 merge used the same source sizes as V1.2:

```text
baseline H12 branches:   4175
baseline H12 pairs:      4439
top50 H16 branches:       208
top50 H16 pairs:          625
gate H20 branches:         31
gate H20 pairs:            86

merged branches:         4414
merged pairs:            5150
```

V2 improved branch outcome and pair-diff modeling without creating action
labels:

```text
                       V1.2       V2
hp_delta R2            0.470      0.509
total_reward R2        0.502      0.507
hp_loss>=5 AUC         0.857      0.876
hp_loss>=10 AUC        0.918      0.925
advantage hp R2        0.413      0.442

pair hp diff R2
from branch model      0.126      0.370

pair abs(diff)>=10 AUC 0.777      0.819
```

The search-allocation gate remains the governing check. V2 passed all required
checks:

```text
baseline hp_loss>=10 K1 recall:        1.000
baseline abs(hp_diff)>=10 K2 recall:   0.950
baseline abs(hp_diff)>=15 K2 recall:   1.000

hard hp_loss>=10 K1 recall:            1.000
hard abs(hp_diff)>=10 K3 recall:       0.969
hard abs(hp_diff)>=15 K5 recall:       1.000
```

The severe-underestimate watch bucket uses `residual_branch_gap_high`, which
allocates search budget to pairs where the residual-corrected pair model
disagrees sharply with the branch-difference model. This is a search-allocation
signal for model disagreement, not an action preference:

```text
hard branch_model_severe_underestimate_ge_10 K5 recall:
  V1.2 = 0.9375  (old tail-probability gate)
  V2   = 0.8889  (residual_branch_gap_high)
```

This keeps the watch bucket above threshold without reintroducing rollout-length
features. It is still a search-allocation model, not a policy, teacher, winner,
or action label source.

## Evidence-Conditioned V2.1 Hard-Tail Repair

The V2 gate miss queue was converted into another targeted recollection batch:

```text
gate miss rows:       7
target rows:         40
target decisions:     4
required targets:    21
watch targets:       19
```

The H24 recollection pass was narrow and clean:

```text
target decisions reached:       4 / 4
target rows reached:           40 / 40
branch traces:                 23
pair comparisons:              55

live_env_changed_count:         0
validation_issue_count:         0
redaction_violation_count:      0
trainable_action_label_count:   0
winner_or_preference_count:     0
truncated_trace_count:          0
```

Exporting the batch produced:

```text
branch rows: 23
pair rows:   55
rng-diverged pairs: 0
```

The merged V2.1 dataset used:

```text
baseline H12 branches:   4175
baseline H12 pairs:      4439
top50 H16 branches:       208
top50 H16 pairs:          625
gate H20 branches:         31
gate H20 pairs:            86
repair H24 branches:       23
repair H24 pairs:          55

merged branches:         4437
merged pairs:            5205
```

V2.1 is a mixed result:

```text
                       V2        V2.1
hp_delta R2            0.509     0.518
total_reward R2        0.507     0.514
hp_loss>=10 AUC        0.925     0.923
advantage hp R2        0.442     0.439

hard abs(hp_diff)>=10 K3 recall:
                       0.969     0.938

hard severe-underestimate K5 recall
using residual_branch_gap_high:
                       0.889     0.889
```

So the H24 repair data is useful as a small hard-tail audit supplement, but it
does not clearly dominate V2. The real V2.1 correction is the gate scoring
change: severe-underestimate allocation should be driven by model-disagreement
signals such as `residual_branch_gap_high`, not by ordinary tail probability
alone.

## Tree Model Ablation V0

To check whether the V2 plateau was caused by the dependency-free hashed linear
model, two tree-family ablations were trained on the same V2 evidence dataset:

```text
tools/learning/train_tree_branch_value_risk_ablation.py
```

This remains a branch-outcome/search-allocation experiment. It does not train an
action policy and it does not emit winners, preferences, selected actions, or
teacher choices.

The ablation uses the same `BranchTrace` / `BranchComparison` inputs and writes
the same branch/pair prediction artifact shape consumed by the gate. The split
helper only separates baseline and hard rows by branch-id source:

```text
tools/learning/split_prediction_artifacts_by_source.py
```

Model results:

```text
                         linear V2   HGBDT     ExtraTrees
hp_delta R2              0.509       0.676     0.662
total_reward R2          0.507       0.676     0.670
hp_loss>=10 AUC          0.925       0.933     0.925

pair hp diff R2
from branch model        0.370       0.529     0.517

pair residual-corrected
hp diff R2               0.382       0.506     0.503

pair abs(diff)>=10 AUC   0.819       0.862     0.830
```

The result is clear: model expressivity was a real bottleneck for branch value
and pair magnitude. Tree models substantially improve branch outcome and pair
diff prediction without changing label semantics.

Gate results are more nuanced:

```text
                         linear V2   HGBDT     ExtraTrees
hard abs(diff)>=10 K3    0.969       0.969     0.969
hard abs(diff)>=15 K5    1.000       1.000     0.929

hard severe-underestimate
K5 via residual gap      0.889       0.500     0.667
```

The required gate checks still pass for both tree models, but the
severe-underestimate watch bucket does not automatically improve. That watch
bucket is model-specific because the target is "where this model underestimates
a large pair gap"; improving the base predictor changes which examples count as
severe misses.

Conclusion:

```text
Tree models validate that the current feature/data path has learnable signal.
They improve value and pair-diff prediction materially.

They do not by themselves solve search allocation for residual blind spots.
The next improvement should be a dedicated allocation model/objective, not just
a stronger value regressor.

## Search Allocation Model V0

The next step made search allocation an explicit supervised objective instead
of deriving budget priority indirectly from value/tail scores:

```text
tools/learning/train_search_allocation_model.py
```

The model consumes branch/pair prediction artifacts and appends audit-only
allocation scores under `search_allocation_signals`. It does not choose actions,
produce winners, or create preference labels. The targets are search-budget
buckets such as:

```text
branch:
  hp_loss_ge_5
  hp_loss_ge_10

pair:
  abs_hp_diff_ge_10
  abs_hp_diff_ge_15
  branch_model_severe_underestimate_ge_10
  residual_model_severe_underestimate_ge_10
  end_turn_play_card_abs_hp_diff_ge_10
```

The gate now supports an additional profile:

```text
tools/learning/evaluate_search_allocation_gate.py --check-profile allocation_model
```

The default profile remains unchanged for earlier artifacts.

V0 used the HGBDT value/pair artifacts as inputs and trained an ExtraTrees
allocation classifier on the baseline prediction rows, then evaluated allocation
on the held-out hard slice:

```text
search_allocation_model_v0_extra_trees_hgbdt_v2.summary.json
search_allocation_gate_v1_allocation_model_extra_trees_hgbdt_v2_baseline_hard.summary.json
```

Key hard-slice gate results:

```text
hard hp_loss>=10 K1 recall:            1.000
hard abs(hp_diff)>=10 K3 recall:       1.000
hard abs(hp_diff)>=15 K5 recall:       1.000

hard branch-model severe-underestimate
K5 recall:                             0.750
```

Compared to the HGBDT value/tail gate:

```text
                         HGBDT value/tail   Allocation V0
hard abs(diff)>=10 K3    0.969              1.000
hard abs(diff)>=15 K5    1.000              1.000
hard severe K5           0.500              0.750
```

This confirms that severe-underestimate and hard-tail budget assignment should
be modeled as search allocation directly. A stronger branch value model helps,
but it is not sufficient by itself.

The allocation model summary includes in-sample fit diagnostics only to confirm
the target is learnable on the baseline source. Generalization is judged by the
hard-slice gate, not by those fit diagnostics.
```

## Fresh Generalization Audit V0

The next audit separated training and scoring sources explicitly instead of
using an internal split from one dataset:

```text
tools/learning/score_tree_branch_value_risk_holdout.py
```

The script trains tree branch outcome models on a declared train source and
scores a declared holdout source. It records:

```text
score_rows_used_for_fit = false
explicit_train_score_split = true
branch_seed_overlap_count = 0
pair_seed_overlap_count = 0
```

The fresh smoke used seeds `2001..2010`, while the training source stayed on
the existing 100-seed H12 branch outcome dataset. A first attempt with
`fixed_decisions` correctly exported zero value/risk rows because the current
exporter only admits complete combat-end outcomes. The actual fresh audit used
`combat_end_v1:12`.

Fresh collection quality:

```text
decisions:                       1134
traces:                          4552
comparisons:                     9446
determinism_mismatch_count:      0
live_env_changed_count:          0
validation_issue_count:          0
redaction_violation_count:       0
trainable_action_label_count:    0
outcome_censored_count:          1605
```

Exported complete combat-end rows:

```text
branch rows:                     2947
pair rows:                       5645
skipped combat_end_not_reached:  1605 traces
skipped rng_diverged:            133 pairs
```

HGBDT branch outcome holdout metrics:

```text
hp_delta R2:                     0.389
total_reward R2:                -0.119
hp_loss>=10 AUC:                 0.741
pair hp diff R2:                 0.256
pair abs(hp_diff)>=10 AUC:       0.811
pair abs(hp_diff)>=15 AUC:       0.789
```

The fresh gate did not pass. Both the allocation-model profile and the older
value/tail profile failed the required pair `abs(hp_diff)>=10` K3 recall check:

```text
allocation_model profile:
  branch hp_loss>=10 K1 recall:      0.916  pass
  pair abs(hp_diff)>=10 K3 recall:   0.829  fail
  pair abs(hp_diff)>=15 K5 recall:   0.929  pass
  severe-underestimate K5 recall:    0.776  watch pass

default value/tail profile:
  branch hp_loss>=10 K1 recall:      0.976  pass
  pair abs(hp_diff)>=10 K3 recall:   0.867  fail
  pair abs(hp_diff)>=15 K5 recall:   0.952  pass
  severe-underestimate K5 recall:    0.569  watch pass
```

Interpretation:

```text
The branch outcome data path is working and the model still has signal on
fresh seeds, but the previous hard-slice success did not generalize cleanly.

The dedicated allocation model is useful as a target family, but V0 overfits
the mined hard slice enough that the default value/tail score beats it on this
fresh smoke for abs>=10 K3 recall.
```

Current status:

```text
Action policy training:          still no
Live takeover:                   no
Comparison winner/preference:    still forbidden
Next step:                       inspect fresh misses and mine new train-only
                                 recollection targets, then re-audit on a
                                 separate fresh seed band.
```

## Fresh Miss Mining V0

The failed fresh gate was audited before any additional recollection. The
required hard-slice miss was:

```text
slice:                            hard
allocation kind:                  pair
objective:                        abs(hp_diff)>=10
score:                            allocation_abs_ge_10_probability_high
budget:                           3
miss rows:                        18
miss decisions:                   18
missed pair targets:              55
```

The missed targets were concentrated in opportunity-cost style pairs:

```text
end_turn -> play_card:            34
play_card -> play_card:           21

top missed pair/card kinds:
  end_turn -> Strike:             16
  end_turn -> Defend:              9
  Strike -> Strike:                6
  end_turn -> SeverSoul:           3
  Warcry -> Defend:                3

magnitude buckets:
  10..14 HP:                      32
  15..19 HP:                      16
  >=20 HP:                         7
```

Those misses were converted into targeted recollection rows and collected as
consumed training data, not as future holdout:

```text
target decision count:            18
target rows:                      55
target decisions reached:         18
matched target action keys:       73
missing target action keys:        0
trace count:                     145
comparison count:                585
live_env_changed_count:            0
invalid_branch_batch_count:        0
validation_issue_count:            0
redaction_violation_count:         0
trainable_action_label_count:      0
outcome_censored_count:            1
```

The complete combat-end export contributed:

```text
branch rows:                     144
pair rows:                       561
skipped combat_end_not_reached:    1
skipped rng_diverged pairs:         7
```

The recollected rows were merged into the 100-seed training source:

```text
merged branch rows:             4319
merged pair rows:               5000
baseline source rows:           4175 branches / 4439 pairs
fresh miss recollection rows:    144 branches /  561 pairs
```

## Fresh Generalization V1 After Recollection

A new fresh holdout band used seeds `3001..3010`. These seeds were not used in
the recollection or merged training source.

Collection quality:

```text
decisions:                       1099
traces:                          4409
comparisons:                     8844
determinism_mismatch_count:         0
live_env_changed_count:             0
invalid_force_count:                0
validation_issue_count:             0
redaction_violation_count:          0
trainable_action_label_count:       0
outcome_censored_count:          1542
complete pair ratio:            0.620
rng diverged pair ratio:        0.223
```

Exported complete combat-end rows:

```text
branch rows:                     2867
pair rows:                       5274
skipped combat_end_not_reached:  1542 traces
skipped rng_diverged:             208 pairs
```

HGBDT holdout metrics after the recollection merge:

```text
hp_delta R2:                     0.169
total_reward R2:                 0.034
hp_loss>=5 AUC:                  0.863
hp_loss>=10 AUC:                 0.807
pair hp diff R2:                 0.287
pair abs(hp_diff)>=10 AUC:       0.817
pair abs(hp_diff)>=15 AUC:       0.778
abs>=10 sign accuracy:           0.841
severe underestimate rate:       0.383
```

The allocation gate still did not pass on the separate fresh band:

```text
allocation_model profile:
  branch hp_loss>=10 K1 recall:      0.933  pass
  pair abs(hp_diff)>=10 K3 recall:   0.855  fail
  pair abs(hp_diff)>=15 K5 recall:   0.982  pass
  severe-underestimate K5 recall:    0.890  watch pass
```

Interpretation:

```text
Targeted miss mining and H16 recollection helped slightly, but did not solve
the recurring hard-slice failure on a separate seed band.

The failing family remains mid-large pair opportunity cost, especially
end_turn-vs-play_card and card-vs-card cases around abs(hp_diff)>=10 under a
K3 search allocation budget.

The next step should not be blind recollection. It should compare Fresh V1
misses against Fresh V0 misses and then change model features/objectives or
allocation structure if the same family repeats.
```

## Fresh V0 vs Fresh V1 Miss Comparison

The same required hard-slice gate was audited on both fresh bands:

```text
slice:                            hard
allocation kind:                  pair
objective:                        abs(hp_diff)>=10
score:                            allocation_abs_ge_10_probability_high
budget:                           3
```

Miss volume:

```text
Fresh V0 miss rows:               18
Fresh V0 missed pair targets:     55
Fresh V1 miss rows:               21
Fresh V1 missed pair targets:     61
```

The coarse action family persisted:

```text
Fresh V0:
  end_turn -> play_card:          34  (61.8%)
  play_card -> play_card:         21  (38.2%)

Fresh V1:
  end_turn -> play_card:          29  (47.5%)
  play_card -> play_card:         32  (52.5%)
```

The card-level surface shifted across seed bands, but the family did not:

```text
persistent:
  end_turn -> Strike:             16 ->  6
  end_turn -> Defend:              9 ->  5
  end_turn -> Bash:                1 ->  6
  end_turn -> ShrugItOff:          1 ->  1
  Strike -> Strike:                6 ->  1

new or larger in Fresh V1:
  Trip -> Strike:                  0 ->  5
  BurningPact -> Defend:           0 ->  4
  end_turn -> Inflame:             0 ->  4
  PowerThrough -> Inflame:         0 ->  4
  Defend -> Inflame:               0 ->  4
```

Magnitude shifted away from the largest tail but remained a K3 allocation
problem:

```text
Fresh V0:
  10..14 HP:                      32
  15..19 HP:                      16
  >=20 HP:                         7

Fresh V1:
  10..14 HP:                      48
  15..19 HP:                      13
  >=20 HP:                         0
```

Interpretation:

```text
The targeted H16 recollection likely reduced the most extreme >=20 HP misses,
but it did not solve the broader abs(hp_diff)>=10 search-allocation problem.

The miss family is stable at the action-family level:
  end_turn opportunity cost
  play_card-vs-play_card opportunity cost

The card names change with the run distribution, so continuing to chase card
instances or small miss batches is unlikely to fix the model. The next useful
work should change pair-context features, opportunity-cost representation, or
the allocation objective/structure.
```

## Family Coverage Allocation Audit V0

The next audit tested whether the failure is partly caused by pair-level top-K
budget duplication. It compared two selection modes over the same pair
prediction artifacts:

```text
pair_topk:
  choose the top K scored pairs directly.

family_topk:
  map pairs into contrast families, choose at most one representative per
  family, then allocate K family representatives.
```

This remains an audit of search allocation. It does not create action labels,
winners, preferences, or policy decisions.

On Fresh V1, for threshold `abs(hp_diff)>=10`, budget `K=3`, and score
`allocation_abs_hp_diff_ge_10_probability`:

```text
primary_tag family mode:
  pair_topk decision family recall:       0.889
  family_topk decision family recall:     0.961

  pair_topk target family recall:         0.614
  family_topk target family recall:       0.793

  pair_topk regret-mass recall:           0.675
  family_topk regret-mass recall:         0.826

  pair_topk duplicate budget / decision:  0.850
  family_topk duplicate budget / decision:0.000
```

The same pattern appears with the other useful scores:

```text
primary_tag / K=3 / abs>=10:
  allocation_pair_priority:
    family recall:     0.617 -> 0.790
    regret mass:       0.685 -> 0.820

  tail_abs_hp_diff_ge_10_probability:
    family recall:     0.597 -> 0.801
    regret mass:       0.646 -> 0.829

  residual_corrected_abs_hp_diff:
    family recall:     0.585 -> 0.787
    regret mass:       0.629 -> 0.818

  branch_model_abs_hp_diff:
    family recall:     0.605 -> 0.816
    regret mass:       0.664 -> 0.841
```

`action_kind` family mode reached perfect family coverage at K=3, but it is too
coarse to trust as the final representation because it collapses all
play-card-vs-play-card contrasts into one family. The more useful signal is
that `primary_tag` and `end_turn_split` both improve coverage without changing
the underlying model, simply by avoiding duplicate pair-budget allocation.

Interpretation:

```text
The recurring K3 miss is not only a model-scoring problem. It is also an
allocation-objective problem.

Pair top-K repeatedly spends multiple budget slots on the same contrast family.
Family-deduplicated allocation recovers much more high-regret family mass from
the same scores and the same budget.

The next implementation target should be a formal contrast/family allocator:
  decision-level context
  candidate-relative opportunity-cost features
  multi-label contrast families
  representative pair selection
  Fresh V2 ablation
```

## Family Coverage Gate V0

The family coverage audit was converted into an explicit gate over the same
Fresh V1 artifact. The gate checks search-allocation coverage, not action
preference or policy quality.

Required Fresh V1 checks:

```text
abs>=10 / K=3 / primary_tag / family_topk /
score=allocation_abs_hp_diff_ge_10_probability

decision family recall >= 0.93:
  actual:                         0.961  pass

regret-mass recall >= 0.80:
  actual:                         0.826  pass

target family recall >= 0.75:
  actual:                         0.793  pass

duplicate budget <= 0.0:
  actual:                         0.000  pass
```

Watch Fresh V1 check:

```text
pair_topk duplicate budget <= 0.50:
  actual:                         0.850  fail as expected
```

Internal train-plus-recollection checks also passed the required family gate:

```text
decision family recall:           1.000
regret-mass recall:               0.972
target family recall:             0.967
duplicate budget:                 0.000
```

Interpretation:

```text
The same score that failed the pair-level K3 gate can pass a family-coverage
gate when allocation is deduplicated by contrast family.

This confirms that the current bottleneck is not just scorer weakness. Pair
top-K allocation is spending budget redundantly inside the same family.

This still is not a policy. It is a stronger search-allocation contract:
  allocate evidence requests to contrast families,
  then choose representative pairs inside selected families.
```

## Family Evidence Request Builder V0

The family allocator was made into a concrete request builder:

```text
input:
  pair prediction artifact

allocation:
  group pairs by decision
  map pairs to contrast families
  keep the top-scoring representative pair per family
  emit up to K family representative evidence requests

default output:
  no realized outcome targets
  no action labels
  no winners/preferences
  compatible with targeted branch trace collection
```

Fresh V1 request build:

```text
decision count:                   569
request count:                   1498
budget:                             3
family mode:                 primary_tag
score: allocation_abs_hp_diff_ge_10_probability

requests per decision:
  3 requests:                     427
  2 requests:                      75
  1 request:                       67

request pair kind:
  end_turn -> play_card:          930
  play_card -> play_card:         568

audit abs>=10 family recall:    0.793
audit abs>=10 regret mass:      0.826
```

Top requested families on Fresh V1:

```text
end_turn_vs_block:                345
end_turn_vs_damage:               339
damage_vs_damage:                 149
damage_vs_block:                   97
block_vs_block:                    83
end_turn_vs_setup:                 81
block_vs_damage:                   77
end_turn_vs_draw:                  55
end_turn_vs_vulnerable:            40
end_turn_vs_exhaust:               39
```

A one-decision targeted-collection smoke confirmed that request rows can drive
branch tracing:

```text
target decisions reached:           1 / 1
target rows reached:                3 / 3
matched target action keys:         5
missing target action keys:         0
trace count:                        8
comparison count:                  28
live_env_changed_count:             0
validation_issue_count:             0
redaction_violation_count:          0
trainable_action_label_count:       0
winner_or_preference_field_count:   0
```

Interpretation:

```text
The allocator is now a concrete offline data-collection primitive:
  decision -> contrast families -> representative pairs -> branch trace request

It still is not a policy and still does not decide what action to take.
```

## Family Search Allocation Model V0

The first family-level model uses the same branch outcome dataset line, but its
training unit is a decision-local contrast family:

```text
input:
  pair prediction rows with outcome diffs and search-allocation signals

family row:
  decision_key
  contrast family
  representative pair
  aggregated pair scores
  family-level high-regret targets

model output:
  family_abs_ge_10_probability
  family_abs_ge_15_probability
  family_regret_mass_abs10
  family_priority
```

This is still search allocation only. It does not produce an action label, a
winner, a preference, or a live takeover decision.

Training data used:

```text
branch rows:                      5808
pair rows:                        8951

source 1:
  train_plus_abs10_recollection
    branch rows:                  4319
    pair rows:                    5000

source 2:
  family_requests_h16_consumed
    branch rows:                  1489
    pair rows:                    3951
```

Fresh V2 holdout used:

```text
seeds:                         4001-4010
branch rows:                      2733
pair rows:                        5051
seed overlap with train:             0
```

Fresh V2 family model row metrics:

```text
family rows:                      2446
abs>=10 family AUC:              0.746
abs>=15 family AUC:              0.688
regret-mass regression:         weak
```

The row-level AUC is not the main gate. The allocation gate asks whether K
family evidence requests cover high-regret family mass inside each decision.

Fresh V2 allocation gate:

```text
score: family_abs_ge_10_probability
budget:                              3
eligible decisions:                127

decision family recall >= 0.93:
  actual:                         0.945  pass

regret-mass recall >= 0.80:
  actual:                         0.812  pass

target family recall >= 0.75:
  actual:                         0.762  pass

duplicate budget <= 0.0:
  actual:                         0.000  pass

watch abs>=15, K=5 decision recall >= 0.90:
  actual:                         0.975  pass
```

Interpretation:

```text
The family model is not a strong value oracle, but it is good enough as a
search-allocation layer on Fresh V2. It can spend K=3 requests across distinct
contrast families and cover most high-regret family mass without duplicating
budget inside the same family.
```

## Model-Guided Evidence Requests V0

Family predictions were converted into concrete evidence requests:

```text
input:
  family_search_allocation_model predictions

allocation:
  group by decision
  rank families by family_abs_ge_10_probability
  emit top K representative-pair requests

output:
  family_evidence_request_v0
  trainable_as_action_label=false
  no winner/preference/selected fields
```

Fresh V2 K=3 request build:

```text
decision count:                    528
request count:                    1393
budget:                              3
score: family_abs_ge_10_probability

request pair kind:
  end_turn -> play_card:           807
  play_card -> play_card:          586

audit selected abs>=10 target family rows: 214
audit selected abs>=10 regret mass:       5664.0
```

A targeted smoke using the new `targets_plus_behavior` mode verified that
model-guided requests can drive branch tracing without reverting to all-candidate
search:

```text
target decisions reached:           30 / 30
target rows reached:                76 / 76
matched target action keys:        114
missing target action keys:          0

trace count:                       122
comparison count:                  200

candidate_index_mode: targets_plus_behavior
requested action kinds:
  play_card:                       104
  end_turn:                         18

live_env_changed_count:              0
validation_issue_count:              0
redaction_violation_count:           0
trainable_action_label_count:        0
winner_or_preference_field_count:    0
outcome_censored_count:              0
truncated_trace_count:               0
```

This completes the offline plumbing:

```text
branch outcome data
-> pair prediction rows
-> family allocation model
-> model-guided family evidence requests
-> targeted deeper branch traces
```

The next step should remain offline: run the model-guided request collector at
larger scale, export those traces, and train the next branch value/risk model
with explicit source splits. It still should not be treated as a live policy.

## Model-Guided Recollection V0

The model-guided family requests were expanded from smoke to the full Fresh V2
request set using `targets_plus_behavior`, so branch tracing evaluates only the
requested family representatives plus the behavior anchor rather than all legal
candidates.

Full Fresh V2 targeted recollection:

```text
target seeds:                       10
target decisions:                  528 / 528
target rows:                      1393 / 1393
matched target action keys:       1926
missing target action keys:          0

candidate_index_mode: targets_plus_behavior
trace count:                      2073
comparison count:                 3235

requested action kinds:
  play_card:                      1612
  end_turn:                        461

live_env_changed_count:              0
validation_issue_count:              0
redaction_violation_count:           0
trainable_action_label_count:        0
winner_or_preference_field_count:    0
outcome_censored_count:              5
truncated_trace_count:               5
```

Exported complete, RNG-aligned data:

```text
branch rows:                      2068
pair rows:                        3144
skipped traces:
  combat_end_not_reached:            5
skipped comparisons:
  rng_diverged:                     81
  right_combat_end_not_reached:      7
  left_combat_end_not_reached:       3
```

Merged next training set:

```text
branch rows:                      7876
pair rows:                       12095

source train_plus_family_requests_v0:
  branch rows:                    5808
  pair rows:                      8951

source model_guided_family_fresh_v2_h16:
  branch rows:                    2068
  pair rows:                      3144
```

## Fresh V3 Independent Holdout

Fresh V3 uses seeds 5001-5010 and was not included in the training data.

Raw trace audit:

```text
combat decisions:                 1036
trace count:                      4158
comparison count:                 8366
determinism_mismatch_count:          0
live_env_changed_count:              0
invalid_force_count:                 0
validation_issue_count:              0
redaction_violation_count:           0
trainable_action_label_count:        0
winner_or_preference_field_count:    0

combat_end complete branches:     2633
death observed branches:           427
censored partial branches:        1098
```

Exported complete, RNG-aligned Fresh V3 holdout:

```text
branch rows:                      3060
pair rows:                        5891
skipped traces:
  combat_end_not_reached:         1098
skipped comparisons:
  left_combat_end_not_reached:    2064
  right_combat_end_not_reached:    297
  rng_diverged:                    114
```

## Branch Outcome Model After Model-Guided Recollection

The next HGBDT branch model was trained on the merged dataset above and scored
on Fresh V3.

Fresh V3 holdout:

```text
train branch rows:                7876
score branch rows:                3060
train pair rows:                 12095
score pair rows:                  5891
branch seed overlap:                 0
pair seed overlap:                   0

hp_delta R2:                     0.643
total_reward R2:                 0.437
hp_loss>=5 AUC:                  0.935
hp_loss>=10 AUC:                 0.912

pair hp diff R2:                 0.357
pair abs>=10 tail AUC:           0.833
pair abs>=15 tail AUC:           0.866
material sign abs>=10:           0.911
material sign abs>=15:           0.941
```

The persistent weakness is still magnitude compression:

```text
severe underestimate rate:
  abs>=10:                       0.346
  abs>=15:                       0.297
```

Interpretation:

```text
The model is useful for risk/search allocation and direction, but still not a
standalone value oracle. Large pairwise gaps are often directionally correct but
underestimated in magnitude.
```

## Family Search Allocation Model V1

Family model V1 was trained from the next branch model predictions and scored on
Fresh V3.

Family row metrics:

```text
train family rows:                1821
score family rows:                2850
abs>=10 family AUC:              0.759
abs>=15 family AUC:              0.778
```

Fresh V3 allocation gate:

```text
score: family_abs_ge_10_probability
budget:                              3
eligible abs>=10 decisions:        117

decision family recall >= 0.93:
  actual:                         0.991  pass

regret-mass recall >= 0.80:
  actual:                         0.853  pass

target family recall >= 0.75:
  actual:                         0.793  pass

duplicate budget <= 0.0:
  actual:                         0.000  pass

watch abs>=15, K=5 decision recall >= 0.90:
  actual:                         1.000  pass
```

Fresh V3 V1 request build:

```text
decision count:                    596
request count:                    1582
budget:                              3
score: family_abs_ge_10_probability

pair kind:
  end_turn -> play_card:          1070
  play_card -> play_card:          512

audit selected abs>=10 target family rows: 219
audit selected abs>=10 regret mass:       5850.0
```

This is the current healthy loop:

```text
branch outcome data
-> branch value/risk/tail model
-> family allocation model
-> model-guided family evidence requests
-> targeted deeper branch traces
-> larger branch outcome dataset
```

Still explicitly out of scope:

```text
action imitation
comparison winner labels
live takeover
legacy/exact/frontier selector revival
```

## Offline Model-Guided Search Runner V0

The family allocator was connected into a single offline runner:

```text
family model predictions
-> K contrast-family evidence requests
-> targeted branch traces
-> decision evidence bundles
-> coverage / abstain report
```

Important semantics:

```text
controller_decision: abstain only
selected_action_id: null
trainable_as_action_label: false
no winner/preference fields
no live takeover
```

The runner writes one `model_guided_search_evidence_bundle_v0` per reached
decision. Each bundle contains:

```text
behavior action id/key
requested contrast families
branch trace batch evidence
coverage counts
evidence_status:
  evidence_ready
  evidence_partial
  abstain
```

Smoke run:

```text
target decisions:                   30 / 30
request rows reached:               78 / 78
trace count:                       113
comparison count:                  167
missing target action keys:          0

bundle status:
  evidence_ready:                   30

live_env_changed_count:              0
validation_issue_count:              0
redaction_violation_count:           0
trainable_action_label_count:        0
winner_or_preference_field_count:    0
outcome_censored_count:              0
truncated_trace_count:               0
```

Full Fresh V3 offline run:

```text
target decisions:                  596 / 596
request rows:                     1582 / 1582
matched target action keys:       2157
missing target action keys:          0

candidate_index_mode: targets_plus_behavior
trace count:                      2311
comparison count:                 3547

requested action kinds:
  play_card:                      1751
  end_turn:                        560

bundle status:
  evidence_ready:                  593
  evidence_partial:                  3

abstain reasons:
  partial_or_censored_evidence:      3

live_env_changed_count:              0
invalid_branch_batch_count:          0
validation_issue_count:              0
redaction_violation_count:           0
trainable_action_label_count:        0
action_like_comparison_role_count:   0
winner_or_preference_field_count:    0
outcome_censored_count:              3
truncated_trace_count:               3
```

Interpretation:

```text
The model-guided search path is now connected offline end to end.

It can spend a family-level search budget, collect deeper branch evidence, and
emit a decision-local evidence bundle with explicit abstention/coverage status.

It still cannot select actions. The next step is an offline evidence interpreter
that consumes these bundles and measures whether the evidence would be enough
to reduce regret under a strict abstain-first rule.
```

## Abstain-First Evidence Interpreter V0

`tools/learning/interpret_model_guided_search_evidence.py` consumes
`model_guided_search_evidence_bundle_v0` rows and produces an offline
counterfactual audit. It is deliberately not a policy:

```text
input:
  model-guided family evidence bundles

strict comparison filter:
  behavior-vs-candidate comparison exists
  complete combat-end evidence on both sides
  uncensored and non-truncated traces
  paired_validity_status = valid
  rng_diverged = false

output:
  controller_decision.mode = abstain
  trainable_as_action_label = false
  no winner/preference/selected/teacher-choice fields
```

The interpreter reports whether the already-collected evidence contains a
material counterfactual candidate under narrow audit criteria:

```text
survival flip
combat progress flip
hp margin >= 5
reward margin >= 0.25
```

Finding such a candidate does not create a selected action. It only marks the
bundle as:

```text
evidence_material_alternative_found
material_alternative_requires_human_or_stronger_controller
```

Full Fresh V3 K=3 interpretation:

```text
bundle count:                         596

status:
  evidence_no_material_alternative:    518
  evidence_material_alternative_found:  74
  abstain_no_strict_behavior_comparison: 1
  abstain_partial_evidence:              3

material reasons:
  hp_margin:                            74

audit action kinds:
  play_card:                            71
  end_turn:                              3

best counterfactual hp-gain buckets:
  5..9:                                 68
  10..19:                                5
  >=20:                                  1

strict candidates compared to behavior: 1695
```

Safety invariants remained intact:

```text
trainable_as_action_label:              false
winner_or_preference_label_used:        false
interpreter_is_offline_counterfactual_audit_not_policy: true
controller_decision_is_abstain_only:    true
```

Interpretation:

```text
The family allocator and targeted branch evaluator are now producing usable
decision-local evidence bundles: 74 / 596 Fresh V3 decisions contained a strict
material behavior-vs-candidate counterfactual under complete, RNG-aligned
evidence.

This is not a takeover result. It is the first offline measurement that the
model-guided evidence path can surface concrete regret candidates while still
abstaining. The next work should remain offline: inspect those 74 material
alternatives, aggregate their contrast families, and design a stronger
evidence-conditioned controller or value target without converting them into
action labels.
```

## Material Alternative Audit V0

`tools/learning/audit_material_alternatives.py` aggregates the material
counterfactuals found by the abstain-first interpreter. It joins interpretation
rows back to the evidence bundles so it can inspect public context, requested
families, behavior action tags, counterfactual action tags, and repeated
decision clusters.

The audit is still offline analysis only:

```text
trainable_as_action_label=false
no winner/preference/selected/teacher-choice fields
no live decision
```

Fresh V3 K=3 material audit:

```text
material rows:                    74
coarse clusters:                  49
repeated clusters:                16

cluster size distribution:
  1 row:                          33
  2 rows:                          8
  3 rows:                          7
  4 rows:                          1
```

The dominant behavior/counterfactual pattern is clear:

```text
behavior primary tags:
  damage:                         66
  block:                           8

counterfactual primary tags:
  block:                          44
  damage:                          8
  resource_or_exhaust:             7
  setup:                           6
  end_turn:                        3
  draw:                            3
  debuff:                          2

behavior -> counterfactual:
  damage -> block:                44
  damage -> setup:                 5
  damage -> resource_or_exhaust:   5
  damage -> damage:                5
  damage -> end_turn:              3
```

Card-level surface:

```text
behavior cards:
  Strike:                         36
  Bash:                            8
  Defend:                          8
  SeverSoul:                       4

counterfactual cards:
  Defend:                         44
  Strike:                          5
  InfernalBlade:                   4
  Combust:                         3
  EndTurn:                         3
  FireBreathing:                   3
  Dropkick:                        3
  BurningPact:                     3
```

All material rows are same-combat-outcome HP improvements:

```text
combat_win_count_gain:             0 for all 74

hp gain:
  5..9:                           68
  10..19:                          5
  >=20:                            1
```

The `damage -> block` bucket is the main stable signal:

```text
damage -> block count:            44
mean hp gain:                     5.27
max hp gain:                      8

incoming damage:
  7..12:                          31
  1..6:                           12
  0:                               1
```

Interpretation:

```text
The material alternatives are not combat-win boundary artifacts. Both behavior
and counterfactual branches usually win the same combat; the counterfactual
simply preserves more HP.

The strongest recurring defect is not "the model should always block". It is a
search/value target: under the current forced-first-action plus
`rule_baseline_v0` continuation contract, many behavior damage branches finish
the same combat with less HP than block counterfactual branches in states with
visible incoming damage.

This should become a value/search-allocation learning target:
  when same-combat-win is likely,
  evaluate HP/resource preservation strongly enough that damage progress does
  not mask avoidable HP loss.
```

The large-gain cases are rare and should be inspected separately:

```text
hp gain >= 10:                     6
end_turn counterfactuals:           3
```

Those cases are useful for trace inspection and model-error analysis, not for
creating card-specific or action-specific rules.

Important limitation:

```text
This audit is Q^{rule_baseline_v0}-style branch evidence:
  force first action,
  then let the recorded continuation policy finish the branch.

It is not full same-turn plan dominance, not an optimal value oracle, and not a
claim that the first counterfactual action is always better in isolation.
```
