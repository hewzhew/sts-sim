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
