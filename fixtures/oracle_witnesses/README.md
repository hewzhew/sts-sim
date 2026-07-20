# Oracle victory witnesses

These files preserve exact, simulator-validated action witnesses. They are
evidence that a specific state is winnable, not a claim that bounded search
must rediscover the line from every earlier prefix.

## seed 20260713007, A0, Awakened One

- Run target: no keys; stop after the Act 3 boss.
- Combat start: Act 3 floor 48, 78/85 HP, one Block Potion.
- Action witness: `seed20260713007_a0_awakened_one.actions.json` (68 actions).
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
