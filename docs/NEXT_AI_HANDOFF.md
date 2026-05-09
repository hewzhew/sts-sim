# Next AI Handoff

This handoff is for the next assistant taking over the Slay the Spire simulator / AI work.

The previous assistant repeatedly drifted into weak audits, heuristic patches, and diagnostic tools that were too easy to mistake for policy progress. Treat the current state as an unfinished, dirty worktree with useful debugging artifacts, not as an approved architecture.

## Current User Intent

The user wants a simulator-backed AI training/planning route that actually improves play, not another layer of:

- single-action score patches;
- neutral/exact/frontier selectors;
- labels that later get downgraded;
- broad seed averages used to hide concrete failure;
- audit reports that do not become actionable planning improvements.

The current active direction is not “make the old bot slightly less bad.” The user wants a process closer to:

```text
run one seed to death
-> inspect death fight, HP, deck, route, shop/campfire/event choices
-> branch at meaningful earlier decisions
-> collect alternate outcomes and failure causes
-> repair the decision process
-> rerun the same seed
-> only then generalize to canary/heldout seeds
```

Do not restart by proposing PPO, MaskablePPO, generic RL Gym wrappers, exact-turn takeover, neutral selectors, or more action-score bonuses. The user has explicitly rejected those as previous failed routes.

## Important User Corrections

The user has made several hard corrections. Follow them.

1. Do not confuse diagnostic oracle search with deployable policy.
   - Oracle search may use simulator/RNG/future state to diagnose whether a combat was theoretically winnable.
   - Policy-valid planning must only use information legally available to the agent and must replan after each observed state change.

2. Do not treat budget exhaustion as a conclusion.
   - If search budget is exhausted, the result is `UNRESOLVED`.
   - It is not proof of loss.
   - It is not permission to select an action.

3. Do not search one card click at a time and pretend that is planning.
   - Combat planning should compile current-turn action sequences into turn-level macro edges, then search across turns.
   - Single-action greedy scoring caused repeated bad fixes around Speed Potion, Seeing Red, Second Wind, Fire Breathing, Frail block, Apparition timing, etc.

4. Do not write boss/card-specific scripts.
   - Bronze Automaton / seed 5201 is a diagnostic case, not a place to hard-code Bronze behavior.
   - If a Bronze-specific failure exposes a general mechanism, name the general mechanism: scheduled large attack, stolen-card minion, scarce intangible resource, multi-turn survival resource, tempo loss, etc.

5. Do not turn run-level failures into magic-number rules.
   - The user strongly rejects “if HP < X avoid monster” style patches unless they are clearly temporary and replaced by branch outcome evidence.

6. Do not generate action labels, winners, or preferences from weak evidence.
   - BranchComparison/outcome diffs are outcome evidence, not teacher labels.

## Dirty Worktree Snapshot

As of this handoff, `git status --short` showed these modified/untracked source files:

```text
M  src/app/branch_evaluator.rs
M  src/bin/full_run_env_driver/main.rs
M  src/bin/sts_dev_tool/main.rs
M  src/bot/event/mod.rs
M  src/bot/map/mod.rs
M  src/bot/reward/mod.rs
M  src/cli/full_run_smoke/batch.rs
M  src/cli/full_run_smoke/bot.rs
M  src/cli/full_run_smoke/features.rs
M  src/cli/full_run_smoke/mod.rs
M  src/cli/full_run_smoke/observation.rs
M  src/cli/full_run_smoke/trace.rs
M  src/cli/full_run_smoke/types.rs
M  src/content/cards/mod.rs
M  src/content/cards/runtime_impl.rs
M  src/engine/action_handlers/cards.rs
M  src/engine/run_loop.rs
M  src/projection/combat/monster_preview.rs
?? tools/learning/death_backtrace_repair.py
```

Do not blindly commit all of this. Some changes are useful, some are questionable, and some are probably failed direction residue. Review diffs before preserving anything.

The last sanity check run by the previous assistant:

```powershell
cargo check -q --bin sts_dev_tool
```

It passed.

## What Was Added Recently

### `tools/learning/death_backtrace_repair.py`

Purpose:

- Read a single full-run trace.
- Report the death floor/fight, HP, deck, relics, potions, prior choices, and likely repair targets.
- It is diagnostic only.

Do not treat its output as policy, labels, or proof.

Known useful output for seed `5201` after an Apparition-scoring patch:

```text
Death: Act 2 floor 32, Bronze Automaton
Entry HP: 35/40
Deck size: 29
Potions: FearPotion, EssenceOfSteel
Relics: BurningBlood, Kunai, OddMushroom, OrnamentalFan, RunicCube,
        BloodyIdol, LizardTail(used), TinyChest, SmilingMask
Main remaining repair target:
  combat_floor32_lethal_end_turn_backtrace
```

The bot improved from earlier deaths but still died to Bronze Automaton. This does not prove combat-only repair is sufficient.

### `sts_dev_tool combat plan-search-from-trace`

Recently added CLI command:

```powershell
target\release\sts_dev_tool.exe combat plan-search-from-trace `
  --trace-file target\seed_5201_apparition_score_trace\episode_0000_seed_5201.json `
  --step-index 469 `
  --out target\seed_5201_floor32_turn_plan_search_v1.json `
  --max-nodes 1000000 `
  --beam-width 4096 `
  --max-depth-decisions 120 `
  --turn-sequence-beam-width 512 `
  --max-turn-sequence-actions 24 `
  --include-frontier
```

Current status:

- It was shifted from single-action expansion toward current-turn sequence expansion.
- It is still not a policy.
- It still has search scheduling problems.
- It still must be interpreted as diagnostic/oracle-like unless policy-valid visibility rules are explicitly enforced.

Important result:

```text
It did not find a clear line in the tested budget.
But search was still limited by node budget / beam width / turn sequence beam width.
Therefore the result is UNRESOLVED, not proof that the fight was unwinnable.
```

Do not let this tool silently become the next “selector.”

### Apparition Scoring Patch

`src/cli/full_run_smoke/bot.rs` got an `intangible_survival_card_score(...)` helper that makes Apparition more attractive under real incoming damage.

This changed seed `5201` trajectory:

```text
Before: death at Act2 floor23 or floor32 depending earlier patches.
After: still death at Act2 floor32 Bronze Automaton,
       but one more combat win and changed route/shop behavior.
```

This is not a final fix. It is a symptom patch. Do not continue expanding this style of scoring patch unless the user explicitly asks for a temporary baseline.

## Known Artifacts

Useful recent artifacts:

```text
target\seed_5201_apparition_score_summary.json
target\seed_5201_apparition_score_trace\episode_0000_seed_5201.json
target\seed_5201_apparition_score_death_backtrace_v3.json
target\seed_5201_apparition_score_death_backtrace_v3.md
target\seed_5201_floor32_plan_search_cap10_v2.json
target\seed_5201_floor32_plan_search_wide_v1.json
target\seed_5201_floor32_plan_search_fullish_v1.json
target\seed_5201_floor32_turn_plan_search_v1.json
```

These are local artifacts, not source truth. They can guide diagnosis but should not be treated as durable benchmark evidence.

## Main Lessons From Failed Routes

### Neutral / exact-turn / frontier paths

These repeatedly produced evidence-like outputs that were too easy to misuse as decisions.

Do not revive:

```text
neutral one-step damage selector
generic effect dominance
exact_turn_solver.best_line as authority
frontier_eval/value as truth
post_turn_frontier as current decision observation
```

If any old code remains, treat it as baseline/diagnostic only.

### Branch outcome data factory

This direction was healthier:

```text
DecisionEnv / DecisionRecord
BranchTrace / BranchComparison
validation / censoring / pairing / redaction
branch value / risk / search allocation models
```

But it stalled when open-loop branch evidence was repeatedly summarized without a closed-loop improvement path.

If continuing this direction, force every data collection step to answer:

```text
Which future decision will this change?
What is the closed-loop experiment?
What are pass/fail criteria?
If it fails, what evidence gap explains it?
```

### Run-level audits

High-gold death and campfire/rest reports were useful only after being corrected from crude interpretations.

Example:

```text
"death with high gold" did not mean "shop entered but bought nothing".
Many high-gold deaths had no actual shop opportunity after gold was gained.
```

So future run-level work must record opportunities, not just visited rooms:

```text
available map paths
reachable shops/campfires/elites
gold when shop became reachable
shop entered/exited gold
offers available
what could have been bought
what was actually bought
```

## Recommended Next Process

Do not start with a new broad architecture. Start with one concrete failure and keep the loop honest.

### Step 1: Freeze One Reproducible Failure

Use seed `5201` after the latest trace as the diagnosis seed:

```text
target\seed_5201_apparition_score_trace\episode_0000_seed_5201.json
```

Current failure:

```text
Act2 floor32 Bronze Automaton death
entry HP 35/40
deck has Apparitions and some synergy
bot still dies to large boss pressure
```

Before changing strategy, produce a stable combat-entry snapshot or reliable trace replay point. The next AI should verify that replaying to step `469` lands on the same combat state.

### Step 2: Separate Oracle Diagnosis From Policy

Build or clean up two modes:

```text
Oracle diagnostic mode:
  May use full simulator state/RNG.
  Only answers WIN_PROVEN / LOSS_PROVEN / UNRESOLVED.
  Never outputs a deployable policy action.

Policy-valid mode:
  Uses only visible/legal information.
  Replans each decision.
  Cannot use future draw/order/random outcomes as if known.
```

If this separation is not explicit in types/output, stop and fix it first.

### Step 3: Make Combat Search Turn-Level

Do not expand one card click as the main outer search node.

Use:

```text
turn-start state
-> enumerate current-turn sequences until EndTurn / next-turn boundary
-> compressed turn-plan edge
-> next turn-start state
```

This should replace the current mix of single-action and partial turn expansion. The search heuristic can order frontier nodes but must not become final action proof.

### Step 4: Produce Cause Diff, Not Just Winning Trace

If oracle search finds a win:

```text
Do not directly imitate it.
Compare baseline death trace vs oracle win trace.
Explain:
  which scarce resource was spent too early;
  which future threat window was missed;
  which turn needed defense/intangible/potion allocation;
  whether damage tempo or survival resource was the bottleneck.
```

Only the cause diff should guide policy design.

If oracle search cannot prove a win:

```text
Do not say "impossible" unless search is actually complete.
If unresolved, report limiting factor:
  node budget
  beam pruning
  turn-sequence beam pruning
  state dedup assumption
  visibility/oracle ambiguity
```

### Step 5: If Combat Is Winnable, Implement Policy-Valid Replanning

Do not patch Apparition/Bronze/FireBreathing scores.

Instead:

```text
when combat is dangerous:
  run bounded policy-valid turn-level planner
  select first action of best proven/safe plan
  after every action/state change, replan
otherwise:
  cheap baseline is acceptable temporarily
```

Danger examples should be generic:

```text
boss/elite fight
low HP
incoming lethal or near lethal
scheduled large attack
stolen-card minion
scarce survival resource
```

### Step 6: If Combat Is Not Winnable From Entry, Backtrack Run Decisions

If the Bronze entry state is genuinely losing or unresolved after serious oracle search, do not keep patching combat.

Backtrack:

```text
event step240
deck selection / removal step288
shop choices
campfire rest/smith choices
route choices
card reward choices
```

For each branch:

```text
force alternate decision
continue with the best available combat planner, not the known-bad baseline
measure whether the key boss/floor death changes
```

## What Not To Do Next

Do not:

- add another `+300` score bonus;
- define more vague readiness/audit windows without closed-loop use;
- run 200 seeds and average away the failure;
- claim branch data is useful without showing what decision it changes;
- call an unresolved search “loss proven”;
- call an oracle trace “policy”;
- write Bronze-specific strategy rules;
- revive neutral/exact/frontier as decision authority;
- treat dirty source changes as approved.

## Minimal Commands For Next AI

Check current compile:

```powershell
cargo check -q --bin sts_dev_tool
```

Re-run seed `5201` with current candidate policy:

```powershell
target\release\sts_dev_tool.exe run-batch `
  --episodes 1 `
  --seed 5201 `
  --ascension 0 `
  --class ironclad `
  --max-steps 1800 `
  --policy rule_baseline_v1_candidate `
  --reward-shaping-profile baseline `
  --determinism-check `
  --summary-out target\seed_5201_current_summary.json `
  --trace-dir target\seed_5201_current_trace
```

Generate death backtrace:

```powershell
python tools\learning\death_backtrace_repair.py `
  --trace-file target\seed_5201_current_trace\episode_0000_seed_5201.json `
  --out-json target\seed_5201_current_death_backtrace.json `
  --out-md target\seed_5201_current_death_backtrace.md
```

Run current diagnostic search from the known Bronze entry:

```powershell
target\release\sts_dev_tool.exe combat plan-search-from-trace `
  --trace-file target\seed_5201_apparition_score_trace\episode_0000_seed_5201.json `
  --step-index 469 `
  --out target\seed_5201_floor32_turn_plan_search_next.json `
  --max-nodes 1000000 `
  --beam-width 4096 `
  --max-depth-decisions 120 `
  --turn-sequence-beam-width 512 `
  --max-turn-sequence-actions 24 `
  --include-frontier
```

Interpret result conservatively:

```text
best_complete_clear present -> oracle found a clear line.
search_limited_by non-empty -> result is not complete.
budget_exhausted true -> UNRESOLVED.
no clear + pruned/budgeted -> not proof of impossible.
```

## Final State Assessment

The project is not hopeless, but the previous assistant repeatedly confused diagnostics, labels, and policies. The next AI should not continue that pattern.

The most valuable near-term work is:

```text
one concrete death
-> reproducible snapshot
-> oracle diagnostic search with honest completeness
-> cause diff
-> policy-valid replanning or run-level backtrack
```

Anything else is likely to repeat the same failure pattern.
