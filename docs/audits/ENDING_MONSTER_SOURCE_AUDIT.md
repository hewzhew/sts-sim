# Ending Monster Source Audit

Date: 2026-05-17

This audit is Java-source-driven. The Java source root is `D:/rust/cardcrawl/monsters/ending`,
and the Rust implementation root is `src/content/monsters/ending`.

The goal is source parity for combat mechanics, not UI parity. Animation, VFX, SFX, dialogue,
screen shake, and render-only randomness are omitted unless they mutate combat state or consume
gameplay RNG. `AbstractDungeon.aiRng`, queued action order, monster private fields, ascension
thresholds, death/escape flags, and combat RNG consumption are treated as mechanics.

## Current Pass Scope

Covered in this pass:

- Spire Shield
- Spire Spear
- Corrupt Heart

## Source Findings

### Spire Shield

Java private state:

- `moveCount`

Rust reconstructs `moveCount` from move history. This matches the Java roll boundary because
`moveCount` increments inside `getMove()` after the next move is selected.

Source-backed details preserved:

- Pre-battle applies `Surrounded` to the player, then Artifact to Shield.
- Artifact amount is 2 at A18+, otherwise 1.
- Turn pattern follows `moveCount % 3`:
  - case 0 randomly Fortify or Bash using `aiRng.randomBoolean()`;
  - case 1 avoids repeating Bash;
  - case 2 always Smash.
- Bash attacks first, then applies either Focus -1 if the player has orbs and the extra
  `aiRng.randomBoolean()` succeeds, otherwise Strength -1.
- Fortify blocks every monster, matching Java's monster-list iteration.
- Smash at A18+ blocks for 99; below A18 it blocks for the current damage output, not merely the
  base damage.
- Death cleanup removes `Surrounded` from the player and `BackAttack` from surviving partner
  monsters using Java's `!isDead && !isDying` style flag check.

### Spire Spear

Java private state:

- `moveCount`
- `skewerCount`

Rust derives `moveCount` from move history and derives `skewerCount` from ascension level. This
matches Java's roll timing and constructor thresholds.

Source-backed details preserved:

- Pre-battle Artifact amount is 2 at A18+, otherwise 1.
- Turn pattern follows `moveCount % 3`:
  - case 0 chooses Burn Strike unless the previous move was Burn Strike, otherwise Piercer;
  - case 1 always Skewer;
  - case 2 randomly Piercer or Burn Strike using `aiRng.randomBoolean()`.
- Burn Strike is two attacks.
- Burn Strike adds two Burns to discard below A18.
- At A18+, Burn Strike uses `MakeTempCardInDrawPileAction(new Burn(), 2, false, true)`, which Rust
  models as top-of-draw-pile insertion through `CardDestination::DrawPileTop`.
- Piercer applies Strength +2 to every monster.
- Skewer hits 4 times at A3+, otherwise 3 times.
- Death cleanup shares the Shield/Spear surrounded cleanup path.

### Corrupt Heart

Java private state:

- `isFirstMove`
- `moveCount`
- `buffCount`

Rust carries these explicitly in `CorruptHeartRuntimeState`. This is necessary because Heart's
rotation and buff ladder cannot be safely inferred from public intent alone.

Source-backed details preserved:

- Pre-battle applies Invincible and Beat of Death.
- Invincible is 300 below A19 and 200 at A19+.
- Beat of Death is 1 below A19 and 2 at A19+.
- First move is always Debilitate and does not increment `moveCount`.
- After the first move, `moveCount % 3` controls the rotation:
  - case 0 randomly Blood Shots or Echo Attack using `aiRng.randomBoolean()`;
  - case 1 avoids repeating Echo Attack;
  - case 2 buffs.
- Debilitate applies Vulnerable, Weak, then Frail, then adds Dazed, Slimed, Wound, Burn, and Void
  through separate random draw-pile insertion actions.
- Blood Shots hit count is 15 at A4+, otherwise 12.
- Echo Attack damage is 45 at A4+, otherwise 40.
- Buff reads negative Strength at take-turn time and gains `2 + abs(negative strength)`.
- Buff follow-up ladder matches Java `buffCount`:
  - 0: Artifact +2
  - 1: Beat of Death +1
  - 2: Painful Stabs
  - 3: Strength +10
  - otherwise: Strength +50
- `buffCount` increments only after the buff move executes.

## Follow-Up Watch Points

- Shield and Spear `moveCount` are safe as history-derived state in normal roll/execute flow. If a
  future live-import path can represent a planned-but-unrecorded ending move, this should become
  explicit runtime state.
- Heart is already explicit runtime state and should stay that way; do not collapse it back into
  move-history inference.
