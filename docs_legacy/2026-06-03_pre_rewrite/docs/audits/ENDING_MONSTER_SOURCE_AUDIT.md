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

Rust carries `moveCount` explicitly in `SpireShieldRuntimeState`. Java increments this private field
inside `getMove()` after selecting the next move, so Rust updates it through the roll-move runtime
hook rather than inferring it from truncated move history. `CommunicationMod` exports the private
field as `monster.runtime_state.move_count`, and Rust state sync treats it as strict protocol truth.

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

Rust carries `moveCount` explicitly in `SpireSpearRuntimeState`. Java increments this private field
inside `getMove()` after selecting the next move, so Rust updates it through the roll-move runtime
hook rather than inferring it from truncated move history. `CommunicationMod` exports the private
field as `monster.runtime_state.move_count`, and Rust state sync treats it as strict protocol truth.
Rust still derives `skewerCount` from ascension level because Java sets it only in the constructor
from the A3 threshold and never mutates it afterward.

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

- Shield, Spear, and Heart runtime counters are explicit Java-private-field mirrors. Do not collapse
  them back into move-history inference; live snapshots only expose a truncated move history and
  cannot reconstruct the full private counters.
