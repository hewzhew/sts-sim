# Oracle victory witnesses

These files preserve exact, simulator-validated action witnesses. They are
evidence that a specific state is winnable, not a claim that bounded search
must rediscover the line from every earlier prefix.

## Current six-seed regression suite

`a0_seed006_012_current.manifest.json` is the authoritative fast regression
set for the current seed006--012 milestone. It pins the expected terminal
fingerprint and compact replay facts for six complete, seed-initialized A0
runs. One process verifies all six journals:

```text
cargo test-run-witnesses
```

The optional read-only owner audit replays the same witnesses and compares
their strategic decisions with the current owner. Policy disagreement remains
diagnostic: it cannot invalidate an exact terminal witness.

```text
cargo audit-run-witnesses
```

The exact replay layer currently completes in a few seconds on the development
machine; replay plus owner audit remains a lightweight second layer.
The suite manifest SHA-256 is
`D2D585992F25E848F557F8E748A210471E750AD0468BB5B09DB04F34F58B62DC`.

## seed 20260713006, A0, Awakened One

- `seed20260713006_a0_autonomous_full_run.continuation.json` began in a new F0
  workspace and preserves the current revalidated F0-to-F48 victory.
- Exact replay verifies seed 20260713006, A0, 225 journal entries,
  197 decisions, 28 combat resolutions, 741 combat actions, terminal victory
  at 14/80 HP, and final fingerprint
  `c7af4e517a02bd04ffb2f282afa3cb5d5f6ebfebeb79a82c6181df50b6b82ae1`.
- A current-policy audit reports 197 rank-zero agreements, no divergence, and
  combat provenance of 8 mature-policy proposals plus 20 search witnesses.
- SHA-256: autonomous full-run continuation
  `403DD394377521A29AB4555309F9B8A2D57C0005725750635F5B232166839D39`;
  compact replay report
  `D13D91F5D4A86460EE712C68038B1D468FB4EF8389712C6930E2C98C87ED58DD`.

## seed 20260713007, A0, Awakened One

- `seed20260713007_a0_autonomous_full_run.continuation.json` preserves the
  current revalidated F0-to-F48 victory.
- Independent replay verifies 208 journal entries, 181 decisions, 27 combat
  resolutions, 484 combat actions, terminal victory at 9/80 HP, and final
  fingerprint
  `77e03b3c2107b332fc75e444c2a6f138bcfca816fb35f5ffc4a963c9af1b5ef4`.
- A current-policy audit reports 179 rank-zero agreements and two rank-one
  historical choices, with no action absent from the owner surface. Combat
  provenance is 4 mature-policy proposals plus 23 search witnesses.
- SHA-256: autonomous full-run continuation
  `FDF5D649896E5BADE02E7EE9CC818F978DC36A82D5C696895841ABE5B2A004A1`;
  compact replay report
  `8D8F83458954CFB3F6D64D6421570B97FEA5E4740DA62CC864C0034C2C109AB7`.

A later capability-migration run preserves a distinct, explicitly
non-autonomous result:

- `seed20260713007_a0_relevant_capability_guided_full_run.continuation.json`
  contains the exact F0-to-F48 journal. The ordinary production owner and
  combat portfolio reached the Act 3 boss at 45/80 HP. At that exact combat
  root, V2 supplied one replay-verified 37-action, 14 HP donor witness for
  offline distillation only.
- The typed action residual and boundary-value prototype read neither exact
  state hashes nor witness actions at runtime. With the resulting immutable
  guidance bundle, the production local-turn graph independently generated a
  different 39-action witness in 2.325 seconds and won at 17/80 HP.
- This is evidence that the new search can inherit useful tactical semantics
  from an exact donor and improve on its demonstrated line. It is not evidence
  that the single-demonstration artifact generalizes, and it is deliberately
  kept separate from the autonomous baseline above.
- The exact boss root, donor, and newly generated winner are preserved as
  `seed20260713007_a0_relevant_capability_awakened_one.combat-case.json`,
  `seed20260713007_a0_relevant_capability_awakened_one.v2-donor.actions.json`,
  and `seed20260713007_a0_relevant_capability_awakened_one.actions.json`.
  The approximately 1 MiB generated guidance bundle is not checked in; it can
  be rebuilt deterministically from the case and donor using
  `build-action-imitation`, `build-value-prototype`, and
  `build-combat-guidance-bundle`.
- Fresh replay verifies seed 20260713007, A0, 208 journal entries, 181
  decisions, 27 combat resolutions, 521 combat actions, terminal victory at
  F48, and final fingerprint
  `3bdc01bf89bc0e3b4bbff45e6de66e0681ec8e5b20b1c18341fe3f38835fd03d`.
- SHA-256: full-run continuation
  `C61BAB981323E98A4D20B5BB943994BEBA409FC568E922A7A61897F41F9211AA`;
  combat case
  `B0999E519A23F049FCBF80736F0E46C959347AA1B48103F8E58EBE7A56F9ED88`;
  new-search witness
  `C0AEC68E17B1C4BDF0B2791048626DF6BC5E5E142D51CA0B0898487148D8D3AC`;
  donor witness
  `54784E55606ECDBF54C1ADE3E2CC18AC2DE42FF1EEDFD69B4F3C7B975814F70E`.

The strategy-to-search handoff has a separate deterministic regression:

- `seed20260713007_a0_awakened_one_managed_t4.combat-case.json` is the exact
  37/85 HP player-turn-4 state reached by the managed phase-control line.
- The typed plan-compatible mainline advances without branching while its
  strategic stage is unchanged. Immediately before its first exact option
  would cross a typed plan milestone, one bounded local-turn-graph suffix
  search is rerooted at the pre-milestone state.
- With a 5,000-work suffix allowance this performs one probe, consumes 3,317
  generation work, and independently produces the checked-in 57-action,
  17-HP witness. The combined prefix and suffix are replayed exactly from the
  unchanged T4 root; neither a V2 donor nor corridor actions participate.
- The lightweight `combat_contract` runner exposes `--expect-witness`,
  `--expect-min-final-hp`, and `--expect-max-plan-suffix-work` so this
  capability and its deterministic work ceiling can be checked directly.
- The compact regression command is:

  ```text
  ol-contract.cmd --case fixtures/oracle_witnesses/seed20260713007_a0_awakened_one_managed_t4.combat-case.json --typed-plan-guide --plan-compatible-policy-line --plan-compatible-suffix-work 5000 --expect-witness --expect-min-final-hp 17 --expect-max-plan-suffix-work 3317
  ```

- SHA-256: managed T4 case
  `FC38B4BC64D74AACE177155754B3E1427AF3D6ECED46191FBB0CC9B96342FCE7`;
  independently generated witness
  `B726D3F9125FB889052978BC60711A487112EFEBFCAB69DA2F1DB79E01791A35`.

A multi-trajectory accumulation control keeps that boundary explicit:

- `combat_guidance_boss_corpus_v1.manifest.json` names six verified Boss
  demonstrations (361 exact actions) across seeds 006 through 009. The
  generated artifacts remain build outputs and are not checked in.
- With the seed007 relevant-capability donor included, the combined bundle
  trained 331 ranked decisions and the production local-turn graph generated
  a new 40-action witness in 1.722 seconds / 3,056 generation work, again
  winning at 17/80 HP. The exact result is preserved as
  `seed20260713007_a0_relevant_capability_awakened_one.six-boss-corpus.actions.json`
  with SHA-256
  `C87E7EDE11A0E12B25E0163C69036AAF3F0D1E7F72E41DBC1FAF7EA5DE9FD377`.
- A held-out control built from the other five demonstrations did not find a
  witness in 10 seconds and gave the Demon Form root family only 68 generation
  work. This proves that the corpus can accumulate an explicitly demonstrated
  capability without catastrophic interference; it does not yet prove
  zero-shot transfer to a strategically different deck.

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

`oracle_lab combat-case-legacy-global` accepts the paired diagnostic arguments
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
`combat-case-legacy-global --shadow-value-prototype <artifact>` run loads
neither the source actions nor their combat case and never compares candidate
exact hashes. The artifact is still a one-demonstration lab model; it is not a
general combat value model or production fallback.

`combat-case-legacy-global --export-witness-actions <path>` writes an action
list only when the legacy planner has produced a replay-verified terminal win.
This permits a model-guided search result to become the next generation's
training witness without copying diagnostic traces or accepting an unverified
rollout.

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
  current seed-initialized journal. Exact replay verifies 204 journal entries,
  180 decisions, 24 combat resolutions, 759 combat actions, terminal victory
  at 9/96 HP, and final fingerprint
  `2b46444d0869565cf323d07ae9eb7e8501b65c72413f90450aff99fe126a11cd`.
- A current-policy audit reports 180 rank-zero agreements, no divergence, and
  combat provenance of 6 mature-policy proposals plus 18 search witnesses.
- SHA-256: combat case
  `47F6BA08C540150BABD186F9959AD7EC27D8CC87FBD69A52BC037ECAFDF3C58C`;
  T3 suffix
  `D5C7EFE43EC79D315F8F743FB3886C229671755C31C9501002974E6774F869CF`;
  layered root witness
  `F6DD35F49AE31DCC723F07CF575F73F9F0C9CCFBE5E4565944641A4B0E63427D`;
  full-run continuation
  `82277CB531895202A6AC20F45586FA0C1994A73952E6148B29460FE6E5850D59`;
  compact replay report
  `B491492FF6E8A0DDCDC18E8FA7F1DB628098242CB913F1E36DC411A82122907F`.

A second full-run witness records the typed reward-semantics repair:

- `seed20260713008_a0_body_slam_fiend_fire_full_run.continuation.json` follows
  the production owner after it recognizes a supported block payoff at A2F20
  (`Body Slam+`) and a supported hand-exhaust conversion at A2F30
  (`Fiend Fire`). A current-policy audit reports 185 rank-zero agreements,
  zero nonzero-rank choices, and no choice absent from the production owner
  surface. All 25 combats came from production search or its mature policy
  proposal; none used manual exact actions or a V2 donor.
- At Donu and Deca, ordinary production `advance` used the portfolio's
  policy-discrepancy member to find and commit the checked-in 79-action
  witness in 5.9 seconds. It entered at 84/93 HP and won at 33/93 HP without
  a trained artifact. The journal records the combat as `search_combat`.
- Fresh import and replay verify seed 20260713008, A0, 215 journal entries,
  190 decisions, 25 combat resolutions, 687 combat actions, terminal victory
  at F48, and final fingerprint
  `53266ac0b66c99c9721ccba90a4da3fc7910d0e3862156bdc3b23bfc83d2d4a3`.
- SHA-256: full-run continuation
  `FDEF8A84BC0230A16800DEA5AAB385BA809A9FA0B9FC405AEF47742372BCE520`;
  Donu and Deca combat case
  `FFFB7088B4D0FE74C7AC24613CFF5DE722FBBFC2F31D69802DF726F3D2C4FCBE`;
  policy-discrepancy witness
  `FB4B8098DD4079502788D2F1C9991CE62D6003C3397FE8E0AB631F63A652EE2B`.

## seed 20260713009, A0, Time Eater

This run preserves both the clean production-search cutover comparison and a
complete F0-to-F48 exact victory:

- `seed20260713009_a0_autonomous_full_run.continuation.json` is a later,
  stronger production witness. It began in a new F0 workspace and used the
  current owner and production local-turn graph for every decision and all 21
  combats. It did not import a prior continuation, combat action witness,
  proof cache, corridor, imitation/value artifact, or V2 donor.
- Every stored combat trajectory in that journal has source `search_combat`.
  After workspace creation, one `oracle_lab_client live run` invocation owned
  all remaining owner/search/accept boundaries. The final Time Eater search
  independently found its 56-action witness in 13.398 seconds and ended at
  9/72 HP.
- Importing the checked-in autonomous continuation into a fresh workspace and
  replaying node 0 verifies 195 journal entries, 174 decisions, 21 combat
  resolutions, 426 combat actions, terminal victory at F48, and final
  fingerprint
  `034e327088f9e701ecd9e3b37396f25a3a255a62c310246973b517a9f4fd4a9c`.
- SHA-256: autonomous full-run continuation
  `43F78E31236EA7DCC81C1412BF439FBA7110C027FB9B280B78DD408BF07B790E`;
  compact replay report
  `3932B8DCB908CC49347ABEE37FF74A04580D60F746CFCCB9547DE16D04B3589A`.
- `seed20260713009_a0_autonomous_full_run.replay.json` preserves the compact
  exact replay report. The 7.5 MiB resident analysis workspace remains outside
  Git; the continuation contains the complete committed journal and exact
  terminal session needed for independent verification.

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
  current journal. Fresh replay verifies seed 20260713009, A0, 210 journal
  entries, 186 decisions, 24 combat resolutions, 551 combat actions, F48
  terminal victory at 9/89 HP, and final fingerprint
  `a85758e4b89beaf16f949fe214a12880c9f7dc798ea398a8eff1c577344c870e`.
- A current-policy audit reports 181 rank-zero agreements and five historical
  nonzero-rank choices (maximum rank 3), with no action absent from the owner
  surface. Combat provenance is 2 mature-policy proposals plus 22 search
  witnesses.
- SHA-256: Time Eater combat case
  `9AAE5E6888034ABF53C34D25F61A3EFAEC5378794AABE215C29DE49E5578DC46`;
  Time Eater witness
  `0F5F5A20834D5BC8F8C86F2F1BC88F95A483E1DB008D263B9364BC6774356840`;
  full-run continuation
  `FFD8CCEE4F06B782F61AE24C384C65C94876D179038D1F425C2668C70F3E6574`;
  replay report
  `071745E2F26EFE15DDB7E94F1EE17ACE874E0393ABE47BE06FBC506CC7B08033`.

## seed 20260713010, A0, Donu and Deca

- `seed20260713010_a0_full_run.continuation.json` began at F0 with the
  production Neow choice `Max HP +8` and used the resident production owner
  and combat search through the Act 3 boss.
- The run entered Donu and Deca at 100/100 HP after choosing Rest at F47.
  Production search committed a 71-action exact victory and ended at
  17/100 HP.
- Fresh replay verifies 208 journal entries, 182 decisions, 26 combat
  resolutions, 755 combat actions, terminal victory at F48, and final
  fingerprint
  `9210ff82c55eae324a2aebe6c7ad144028ebf277a4866227781922b2f7a13583`.
- A current-policy audit reports 182 rank-zero agreements, no divergence, and
  combat provenance entirely within the production portfolio.
- SHA-256: full-run continuation
  `35C2E8F626128AFA2532F530D751E4CC156A13CF5386812D1ED38B2424DB4829`;
  compact replay report
  `F77B67D70D9C2A0755FDA5E52C3EB9FEE33DCD510ABDE0E95F0EE56BC9F8446F`.

## seed 20260713011, A0, Time Eater

- `seed20260713011_a0_full_run.continuation.json` began at F0 with the
  production Neow choice `Remove a card` and removed a Strike.
- The exact line has two rank-one strategic corrections. At F1 it chose
  Armaments over Searing Blow; at F36 it spent 237 saved gold by visiting the
  shop instead of another hallway. The shop owner then bought Sever Soul,
  Entrench, and Seeing Red.
- The corrected line entered Time Eater at 66/66 HP. Production search found a
  42-action exact victory in 1.43 seconds and ended at 11/66 HP.
- Fresh replay verifies 203 journal entries, 180 decisions, 23 combat
  resolutions, 661 combat actions, terminal victory at F48, and final
  fingerprint
  `0790316bc36f3b0333d5d4c4afba81e64808ba75ebe0ae60b1f7b0ff8736f067`.
- A current-policy audit reports 178 rank-zero agreements and two rank-one
  divergences, with no choice absent from the owner surface. Combat provenance
  is 4 mature-policy proposals plus 19 search witnesses.
- SHA-256: full-run continuation
  `E6673AE9292DB8245D22EE51E9F287819DABABE7E55F1BF5EC38F5925A3775BB`;
  compact replay report
  `6581C35ED15BCBB7AA0FA3222713C514B44665692FABAB90B75115BA0F16B950`.
