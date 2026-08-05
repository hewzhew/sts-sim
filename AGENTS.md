# Repository Agent Guidance

This file applies to the entire repository.

## Purpose

Keep this file limited to durable collaboration rules, safety boundaries, and
pointers to maintained sources of truth. Do not turn it into a changelog or
copy fast-changing milestone status into it. Reconstruct current state from
the repository and update the relevant maintained document in the same change
when code and documentation diverge.

Communicate with the project owner in Chinese. Match their gentle tone, explain
uncertainty plainly, and keep handoffs compact and evidence-based.

## Re-establish Context Safely

1. Read `README.zh-CN.md` (or `README.md`), then `docs/README.md`.
2. Read the relevant parts of `docs/ARCHITECTURE.md`,
   `docs/RUNBOOK.md`, and `docs/TESTING.md`.
3. Inspect `git status --short --branch`, the current branch, and a short
   recent `git log` before changing files.
4. Use targeted symbol and file searches. Do not begin with recursive dumps or
   large generated artifacts.
5. For the current local potion investigation, consult
   `.oracle-lab/reports/potion-strategy-investigation-2026-07-30.md` when it
   exists. It is ignored local research evidence, not a substitute for code or
   maintained documentation.

Never read, open, recover, summarize, or analyze old Codex task/session JSONL
files or extract history from `~/.codex`. Rebuild trustworthy context from the
project itself.

## Architecture And Evidence Boundaries

- Preserve the separation among typed simulator state, policy/strategy,
  execution, and diagnostics.
- Owners produce typed decisions. Runtime executes those decisions without
  reparsing display text.
- Combat search solves combat-local questions. Run-level continuation value
  must not be invented as a combat score.
- Panels, audits, traces, and reports are evidence. They are not automatically
  teacher labels or authoritative policy.
- Prefer exact root identity, replayable witnesses, typed facts, and explicit
  unknowns over heuristic reconstruction.
- If active code and a maintained document disagree, fix one or update both in
  the same change.

## Potion Strategy Research

Use these principles unless stronger measured evidence replaces them:

1. Search for a no-potion win first.
2. Land a verified win once it reaches the required strategic HP quality;
   ordinary fights must not repeatedly spend budget polishing small local HP
   differences without a run-level reason.
3. If quality is insufficient, evaluate bounded rescue lanes by concrete
   potion identity and exact context, normally one additional potion at a
   time.
4. A full inventory is continuation pressure, not permission to waste a
   potion.
5. Do not rank spend/keep decisions by rarity, a static potion tier, or a
   context-free retained-value score.
6. Preserve UUID, mechanical role, dependencies, route coverage, supply facts,
   shop uncertainty, recovery opportunities, and unresolved future state.
7. Only validated, reconstructible continuation context and pressure may enter
   retained-value evidence. Legacy, missing, conflicting, or mismatched facts
   stay explicitly unavailable or rejected.
8. Keep shadow audits non-authoritative until bounded case collections justify
   a policy change.

## Local Experiments And Tool Safety

- Prefer repository files and CLI tools. Browser Use, Chrome control, and
  Computer Use are authorized again after the earlier GPU instability. Start
  UI automation with one lightweight surface at a time, avoid running those
  surfaces concurrently until stability is established, and stop if GPU or
  window instability returns.
- Do not generate images for engineering work unless explicitly requested.
- Keep terminal output bounded: use summaries, counts, targeted searches, and
  at most a few dozen relevant log lines.
- Write complete large JSON, corpus output, build logs, and experiment results
  below ignored `.oracle-lab/` locations, then report only aggregate findings.
- Use a fresh path for every experiment. Do not overwrite prior evidence.
- Diagnose on one replayable exact root first, then reproduce the result through
  the production owner from a copied saved workspace. Escalate next to two to
  five contract-selected sentinels. Consecutive 20-seed panels are milestone
  soak evidence, not the default edit-test loop.
- Before starting a multi-seed panel, state the distributional question that
  only the panel can answer and the early-stop signal. Stop a running panel
  when repeated budget-censored outcomes no longer distinguish the competing
  hypotheses; preserve its already durable partial summary.
- Treat a budget-limited missing witness as unknown, not as proof of loss,
  potion value, or policy quality.
- Use the V2 `.\ol.cmd contract`, `artifact`, and `case` surfaces for routine
  exact-combat evidence as documented in `docs/RUNBOOK.md`. Rebuild through
  `cargo oracle-lab contract --help`.
- Repeated shell discovery, guessed JSON paths, manual artifact naming, large
  default output, stale help, or a second ad-hoc query needed to classify one
  experiment are control-plane rot signals. Stop the active investigation and
  repair the owning typed tool, compact schema, or contract test before
  continuing. Do not normalize the workaround into a runbook.
- Prefer a deliberate breaking migration over maintaining duplicate routine
  experiment surfaces. Legacy reports and catalogs are not automatically V2
  evidence; import an exact root and regenerate a fresh V2 artifact instead of
  teaching new tools to guess old schemas.

## Verification

- Start with the narrowest relevant tests.
- For documentation-only changes, run `git diff --check`.
- For code changes, use the maintained verification commands in
  `docs/RUNBOOK.md`. Before handoff, cover the affected compilation owners and
  run broader checks in proportion to risk.
- Redirect large test/build output to `.oracle-lab/reports/` and surface only
  final counts and a short failure tail.
- Run `cargo fmt --all -- --check` and `git diff --check` before committing.

## Git And Handoff Hygiene

- Preserve unrelated user changes and inspect the worktree before editing.
- Keep commits small, honest, and independently reviewable.
- Do not use destructive Git commands to discard work.
- Do not push, open a PR, or mutate remote state unless the project owner asks.
- At handoff, report confirmed behavior, remaining uncertainty, verification
  results, the commit hash when committed, and whether the worktree is clean.
