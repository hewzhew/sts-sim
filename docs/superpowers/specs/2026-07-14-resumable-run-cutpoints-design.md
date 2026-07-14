# Resumable Run Cutpoints Design

## Goal

Make expensive run diagnostics resume from the exact state that matters instead of
replaying a seed prefix. A combat gap must retain the session from immediately
before combat search, and an Act boss relic decision must retain the session before
any relic is selected. Counterfactual branches restored from one cutpoint must
therefore share the same run state, RNG streams, visible candidates, and history.

The first consumer is the seed006 Act 2 comparison of Black Blood, Coffee Dripper,
and Philosopher's Stone. The implementation is general run-control reliability
infrastructure rather than seed-specific strategy code.

## Evidence and Failure Mode

At commit `0b8cefd5`, the historical seed006 run reached Act 3 floor 40 with the
same visible CLI contract that a fresh run later used. The fresh run stopped at Act
2 floor 20 and had already diverged in current/max HP and card selection. The combat
searches were wall-clock limited, so different node coverage selected different
legal combat lines; their persistent effects then changed later owner decisions.

The fresh command also requested `--frontier-checkpoint`, but the real combat-gap
stop exhausted the frontier and wrote only a combat case. It did not preserve a
session that could re-enter the failed search with a larger budget. The simulator's
single-combat capture remains replayable; the missing reliability boundary is the
multi-room run orchestration around it.

## Selected Approach

Add first-class resumable cutpoints to the owner-audit runtime.

1. Immediately before a combat portfolio mutates a session, retain a clone of the
   branch as a `pre_combat_search` cutpoint. When artifact output is enabled, write
   it provisionally before search; retain it only when the portfolio ends in a
   combat gap, budget gap, or resumable pause. The persisted branch must be in the
   pre-search running state, so resuming with larger budgets executes the search
   again instead of restoring an inert post-gap status.
2. When a branch reaches the Act boss relic owner boundary, persist an
   `owner_decision` cutpoint before expansion. Initially, Boss Relic is the only
   durable owner-decision kind; other owner kinds can be added later from evidence.
3. Store cutpoints using the existing full `RunControlSessionCheckpointV1` and
   frontier schema. Do not introduce a second partial run-state schema.
4. Give every cutpoint a small manifest containing its kind, act, floor, boundary,
   branch id, full session fingerprint, branch-control/history fingerprint,
   candidate fingerprints, source generation, and artifact trust. The payload
   remains an ordinary resumable frontier file.
5. A restored experiment may override branch count and search budgets using the
   existing `--resume-frontier` contract. It must validate its expected fingerprint
   and visible candidate set before expanding choices.

The normal run policy, combat action ordering, owner scores, and game mechanics do
not change.

## Artifact Layout and Retention

For a run capsule, write cutpoints below `cutpoints/`:

```text
cutpoints/
  inflight_pre_combat_b0036.frontier.json
  inflight_pre_combat_b0036.manifest.json
  latest_pre_combat_search.frontier.json
  latest_pre_combat_search.manifest.json
  a2f32_boss_relic.frontier.json
  a2f32_boss_relic.manifest.json
```

Each branch being searched uses one branch-named inflight pair so a successful
branch cannot erase another branch's gap. A gap promotes its inflight pair to the
single `latest_pre_combat_search` pair; success removes its inflight pair and removes
the latest pair only when the fingerprints match. Inflight files are bounded by the
branch cap and only the newest promoted recovery point is kept. This avoids
presenting a stale successful combat as the current recovery point while still
making the current combat gap recoverable.

Boss relic cutpoints are durable and named by act/floor/boundary. There are at most
three in a normal run, so retaining them does not create an unbounded checkpoint
journal. Writes use a temporary sibling followed by atomic replacement so an
interrupted write cannot destroy the previous valid recovery point.

Without a run capsule or an explicit frontier artifact root, the runtime keeps the
in-memory pre-search clone for result construction but performs no implicit
filesystem writes.

## Reproducible Experiment Contract

Add an opt-in reproducible-search contract for counterfactual runs. Search still has
both node and wall limits, but a comparison arm is considered reproducible only
when each relevant search either exhausts its deterministic frontier, reaches its
node limit, or finds an accepted exact-replayed result before the wall safety limit.
If the wall limit is the deciding stop, the arm is reported as
`wall_safety_limited` and is not ranked against sibling arms.

This mode does not claim exhaustive play and does not force a weak accepted line.
It turns machine-speed variance into an explicit invalid-comparison result instead
of silently treating it as policy evidence.

The seed006 boss relic experiment will restore one Boss Relic cutpoint, verify the
13/101 HP, 167 gold, 15-card deck, relic set, RNG fingerprint, and three expected
candidates, then fork the three relic choices. If an exact historical cutpoint does
not exist, evidence projection remains diagnostic-only and must not be labeled an
exact counterfactual.

## Data Flow

```text
running branch
  -> capture pre-search branch
  -> combat portfolio
       -> success: continue normally
       -> gap/pause: persist captured branch as resumable cutpoint

running branch at Boss Relic
  -> fingerprint and persist owner cutpoint
  -> ordinary owner expansion

resume frontier
  -> validate manifest and fingerprint
  -> override branch/search budgets
  -> fork visible candidates
  -> report comparable or wall-safety-limited arms
```

## Error Handling

- Refuse to resume when the frontier and manifest fingerprints disagree.
- Refuse an expected-boundary experiment when act, floor, boundary, or visible
  candidate keys differ.
- A failed atomic artifact write is an artifact error, not a game-policy decision;
  the run must report it rather than pretending a recovery point exists.
- A wall-safety stop remains resumable and carries explicit termination provenance.
- Existing frontier checkpoints without manifests remain loadable for ordinary
  resume. Strict experiment validation requires a cutpoint manifest.

## Alternatives Rejected

- Replaying the entire seed prefix is expensive and cannot reproduce a wall-clock
  limited search history reliably.
- Reconstructing a run from a combat capture and a path summary loses schedule,
  pool, map, and owner-history state; it is useful as projected evidence but is not
  an exact checkpoint.
- Editing serialized frontier JSON by hand is schema-fragile and bypasses session
  invariants.
- Saving every owner boundary forever would make the artifact surface noisy and
  repeat the unbounded-journal problem. This delivery retains only the latest
  pre-combat recovery point and the few Act boss relic cutpoints.

## Verification

Test-first coverage will establish:

1. A combat gap persists the branch from before search, and resuming it with a new
   budget invokes search again from the same session fingerprint.
2. A successful combat does not leave a misleading post-gap recovery artifact.
3. Reaching Boss Relic writes one exact owner cutpoint before choice expansion.
4. Loading a cutpoint preserves run state, RNG streams, visible candidate order,
   and session fingerprint.
5. A manifest/payload mismatch and an unexpected candidate set fail closed.
6. A wall-decided reproducible arm is labeled `wall_safety_limited` and excluded
   from sibling ranking.
7. Retention overwrites the latest pre-combat slot and preserves distinct Boss
   Relic cutpoints.

Finish with focused tests, the full library suite, architecture-boundary tests,
formatting, and a clean diff. After the infrastructure passes, regenerate an exact
seed006 Boss Relic cutpoint and run the three-arm comparison; do not use the earlier
divergent prefix as evidence.

## Non-Goals

- No strategy-score changes for cards, routes, campfires, shops, potions, or relics.
- No attempt to make wall-clock execution speed identical across machines.
- No claim that one seed proves a globally optimal Boss relic.
- No migration of old combat captures into exact run checkpoints.
- No general journal of every transient owner boundary in this delivery.
