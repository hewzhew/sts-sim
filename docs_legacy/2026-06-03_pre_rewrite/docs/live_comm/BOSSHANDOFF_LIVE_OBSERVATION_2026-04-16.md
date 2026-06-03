# BossHandoff Live Observation (2026-04-16)

This note records a real `live_comm` run using the assisted progression profile
with human card-reward help and full boss-combat handoff enabled.

## Run

- profile:
  - `Ironclad_Assisted_Progression_BossHandoff.json`
- run id:
  - `20260416_150306`
- manifest:
  - [logs/runs/20260416_150306/manifest.json](../../logs/runs/20260416_150306/manifest.json)
- validation:
  - [logs/runs/20260416_150306/validation.json](../../logs/runs/20260416_150306/validation.json)
- focus trace:
  - [logs/runs/20260416_150306/focus.txt](../../logs/runs/20260416_150306/focus.txt)

## Outcome

- session exit reason: `GAME_OVER`
- classification label: `victory_tainted`
- validation status: `ok`
- reward loop detected: `false`
- trace incomplete: `false`

The run reached a real victory while using the intended collaboration mode:

- bot handled most normal progression and combat turns
- human handoff remained available for card rewards
- human boss-combat handoff remained available

## What This Proves

- `CommunicationMod` -> `live_comm` bootstrap is operational in the normal
  launcher path, not only in manual scenario mode.
- The `BossHandoff` profile is a viable day-to-day observation mode.
- The bot is weak, but it is not so unstable that it immediately deadlocks or
  throws the run away before handoff matters.
- Pending/noncombat flow appears stable enough for practical assisted runs.

This is a workflow viability checkpoint, not a claim that the bot is strong or
that engine parity is already acceptable.

## What Still Fails

The run is explicitly classified as `victory_tainted`, not clean.

Manifest counters:

- `engine_bugs`: `19`
- `content_gaps`: `2`
- `replay_failures`: `37`

One concrete engine issue visible in the focus trace is repeated player
`Strength` divergence during a Darkling combat, e.g.:

- `player.power[Strength].amount : Rust=6, Java=2`
- later `player.power[Strength].amount : Rust=9, Java=8`

So the correct conclusion is:

- the assisted `bot + handoff` collaboration mode is operational
- the engine/parity layer still contains known defects that can taint successful
  runs

## Recommended Use

Use this mode when the goal is:

- observing whether the integrated system can stay alive through a full run
- collecting realistic logs while retaining human control over high-leverage
  points
- sanity-checking that bot progression is "good enough to collaborate with"

Do not use this run as evidence that:

- full parity is solved
- the heuristic bot is strategically strong
- replay failures are low enough for unattended evaluation

## Next Actions

- keep `BossHandoff` as the preferred human-observation mode
- use `Engine_Strict` / `Engine_Survey` when the goal is bug hunting rather than
  practical collaboration
- mine this run's artifacts for one concrete parity fix at a time instead of
  treating "victory" as a reason to stop validation
