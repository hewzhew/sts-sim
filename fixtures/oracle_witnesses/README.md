# Oracle victory witnesses

These files preserve exact, simulator-validated action witnesses. They are
evidence that a specific state is winnable, not a claim that bounded search
must rediscover the line from every earlier prefix.

## seed 20260713007, A0, Awakened One

- Run target: no keys; stop after the Act 3 boss.
- Combat start: Act 3 floor 48, 78/85 HP, one Block Potion.
- Action witness: `seed20260713007_a0_awakened_one.actions.json` (68 actions).
- Diagnostic milestone sidecar:
  `seed20260713007_a0_awakened_one.milestones.json`. Its `observed` fields are
  exact replay facts; its `interpretation` fields are human diagnostic notes,
  not search rules or claims that every winning line must match the same cards.
- Verified result: victory on turn 14 with 13/85 HP.
- A second exact witness,
  `seed20260713007_a0_awakened_one.layered-proof-cache.actions.json`, was
  reconstructed by the new layered generator and exact solved-suffix cache:
  T3 was solved by turn-rerooted policy-discrepancy search, then the proof was
  folded through T2 and T1 until the root generator matched it naturally.
  The resulting 90-action root replay wins on turn 18 with 16/85 HP. This is
  proof-cache composition, not a claim that an unguided T0 search found the
  whole line in one pass; no V2 donor participates in the reconstruction.
- The independently discovered 77-action T3 suffix used as the seed proof is
  preserved in
  `seed20260713007_a0_awakened_one.t3-policy-discrepancy.actions.json`.
  `oracle_lab combat-case-fold-solved-suffix` can now compile that exact proof
  back through a supplied sequence of player-turn boundary states in one
  process. Corridor actions reconstruct subproblem roots only and have no
  search-order authority; every fold must generate the cached successor and
  replay the composed terminal witness exactly.
- Layered proof-cache action SHA-256:
  `3D4800407A41E700C53BA122559E3E619B7066B6B9C449F25B71D25B81F0BF29`.
- Combat start case SHA-256:
  `155F29BFB291C7C87652DD27421F947A3AB43225776F4A0A17274D3ADE4C7BA1`.
- Final exact combat-state hash:
  `5fefdaf61e268b357784defec2daea09b5ad1143f9638efed13534b325ed6c19`.
- Materialized analysis workspace SHA-256:
  `4163A0B9CE6C4E4988712A70A140F43C4FD7F52B90DF423081F5769B1DB968C6`.
- Materialized journal result: `complete_victory`, status `Victory`, 68 stored
  exact actions. This workspace predates the distinct provenance enum and
  therefore labels the source `search_combat`; the checked-in action fixture
  is the externally supplied exact witness. New acceptances are recorded as
  `oracle_exact_actions` after exact replay validation.
- The same F0 journal now also retains node 144, whose Act 3 boss edge uses the
  90-action layered proof-cache witness. Full replay from seed initialization
  verifies 192 journal entries, 22 combat resolutions, 558 combat actions,
  terminal victory at F48, and 16/85 HP. The previous 13 HP child remains in
  the workspace as separate historical evidence.

The full workspace is intentionally kept outside Git because it is roughly
50 MiB. The hashes above prevent a different local artifact from being
silently mistaken for the verified one.

### Exact-corridor shadow control

`oracle_lab combat-case` accepts the paired diagnostic arguments
`--shadow-corridor-case` and `--shadow-corridor-actions`. The lab replays and
validates the complete action fixture, then adds the exact player-turn states
as one extra, guide-only search ordering. It does not change legal actions,
duplicate ownership, pruning, or terminal validation.

The optional `--shadow-corridor-guide typed-feature` control replaces exact
hash membership with normalized distance over the existing typed progress,
survival, horizon, and realized-setup components. It still learns its
per-turn prototypes from the verified demonstration, but it cannot recognize
a candidate by identity. This separates "the state representation can point
toward a winning region" from "the exact future state was memorized".

Training and inference can also be separated. `oracle_lab
build-value-prototype` validates the source witness once and writes a small,
versioned artifact containing only typed feature prototypes. A later
`combat-case --shadow-value-prototype <artifact>` run loads neither the source
actions nor their combat case and never compares candidate exact hashes. The
artifact is still a one-demonstration lab model; it is not a general combat
value model or production fallback.

`combat-case --export-witness-actions <path>` writes an action list only when
the planner has produced a replay-verified terminal win. This permits a
model-guided search result to become the next generation's training witness
without copying diagnostic traces or accepting an unverified rollout.

Optional one-turn loss evidence is stricter than an observed losing action.
The planner records a state only after its complete-turn generator finishes
without a mechanics gap and every complete option is terminal loss. Collection
is disabled by default and bounded when enabled. These negative prototypes are
stored as training evidence only.

The matching positive evidence is also exact: a state is recorded only after
the complete-turn generator has produced a legal successor at the next player
turn (or a terminal win), together with that exact turn witness. Positive and
negative collection are independently bounded and perform no search-time scan
when their limits remain at the default zero.

Two tempting direct guides were tested and rejected. Nearest-loss repulsion
had no independent fixed-work benefit. A one-turn survivability classifier
separated held-out positive and negative evidence well, but made the actual
long-horizon search worse: at 15,000 work the model-only search found the
47-action, 13 HP victory without that head and found no witness with it.
Consequently neither experiment remains in search ordering; the exact samples
remain available for a future long-horizon value or backup model.

This is a perfect-information upper-bound experiment, not a production
policy. It answers whether better state evaluation can expose a known victory
corridor through the current planner; a normal run must never read this
fixture or its future state hashes.

## seed 20260713008, A0, Donu and Deca

- Run target: no keys; stop after the Act 3 boss.
- Combat start: Act 3 floor 48, 56/93 HP, Skill Potion, Liquid Memories, and
  Block Potion.
- `seed20260713008_a0_donu_deca.t3-local-graph.actions.json` is the 43-action,
  12 HP terminal suffix independently found from the exact T3 state by the
  base local-turn graph search.
- `seed20260713008_a0_donu_deca.layered-proof-cache.actions.json` is the
  68-action root witness compiled backwards from that suffix. T2, T1, and T0
  required 368, 3,504, and 7,880 generation work respectively; the one-process
  fold completed in 1.642 seconds. No V2 donor or trained action/value artifact
  participated. The exact corridor reconstructed predecessor roots only.
- `seed20260713008_a0_donu_deca.combat-case.json` preserves the exact combat
  root. The layered root witness ends with 12/93 HP.
- `seed20260713008_a0_full_run.continuation.json` contains the complete
  seed-initialized journal. Importing it into a fresh workspace and replaying
  node 0 verifies 212 journal entries, 25 combat resolutions, 660 combat
  actions, terminal victory at F48, and final fingerprint
  `ae461b09a80cd602f2e039afa38c373c8a54e6dde002bb99d43a033d74758a43`.
- SHA-256: combat case
  `47F6BA08C540150BABD186F9959AD7EC27D8CC87FBD69A52BC037ECAFDF3C58C`;
  T3 suffix
  `D5C7EFE43EC79D315F8F743FB3886C229671755C31C9501002974E6774F869CF`;
  layered root witness
  `F6DD35F49AE31DCC723F07CF575F73F9F0C9CCFBE5E4565944641A4B0E63427D`;
  full-run continuation
  `72BDFD54DB3C83260E388703C70E6ED7452BE3A0C95E89976F44077D92BDD1E3`.

## seed 20260713009, A0, Time Eater

This run preserves both the clean production-search cutover comparison and a
complete F0-to-F48 exact victory:

- At Act 1 floor 16 the player enters Slime Boss at 60/72 HP with Whirlwind,
  Cleave, Shrug It Off+, Rupture, Bloodletting, and one Gamblers Brew.
- The production global agenda, with V2 disabled, spent 30 seconds, 190,977
  generation work, 113,916 atomic transitions, and retained roughly 30,000
  exact states without a witness.
- The independent base local-turn graph, with no donor, imitation artifact,
  value artifact, or corridor, found a replay-verified 38-action victory in
  0.787 seconds and ended at 23/72 HP.
- `seed20260713009_a0_slime_boss.combat-case.json` and
  `seed20260713009_a0_slime_boss.local-turn-graph.actions.json` preserve the
  exact comparison. They are the production-cutover regression case; they do
  not justify further tuning of the superseded global agenda.
- SHA-256: combat case
  `AD99F8321C4F055568713A85DF5F2F14C90D7E26DFB1D4B83921C733AAE69E29`;
  local-turn graph witness
  `22B3C7E931CCB35CDD14F1F959F847A73B4D852C703CAAEE26DC3F6460E573BB`.
- After the cutover, the production run used only the local-turn graph for
  Act 2 and Act 3 combat search. It reached Time Eater at 45/72 HP, where a
  30-second exact search found no witness. A full-health diagnostic found a
  32-action, 13 HP witness; the original history showed that the final
  campfire had chosen `Smith Armaments` instead of `Rest`.
- Changing only that legal F47 decision produced a 66/72 HP boss entry. The
  same 32 exact actions replayed legally from this state and defeated Time
  Eater with 7/72 HP. The combat root and accepted witness are preserved as
  `seed20260713009_a0_time_eater.combat-case.json` and
  `seed20260713009_a0_time_eater.local-turn-graph.actions.json`.
- `seed20260713009_a0_full_run.continuation.json` is the complete promoted
  journal. Fresh replay verifies seed 20260713009, A0, 200 journal entries,
  178 decisions, 22 combat resolutions, 496 combat actions, F48 terminal
  victory, and final fingerprint
  `666c4a870cddd938e06dc9711a87b16db8207a825f94c8ed7b2cef254273447c`.
- SHA-256: Time Eater combat case
  `9AAE5E6888034ABF53C34D25F61A3EFAEC5378794AABE215C29DE49E5578DC46`;
  Time Eater witness
  `0F5F5A20834D5BC8F8C86F2F1BC88F95A483E1DB008D263B9364BC6774356840`;
  full-run continuation
  `92539B043C505AB3926CA8085776D901CB946DB19D732B8BD956B95925564326`;
  replay report
  `C0701893B755B58D64454E1E58B34E7A3FEE0874E871143CDBB8E1F2748E2EC0`.
