# Boss Victory HP Boundary Design

## Problem

Owner-audit currently applies the same 25%-max-HP reserve to every combat.
That is correct as a conservative persistent-health rule for hallways, elites,
event combats, and the first A20 Act 3 boss, but it is wrong for an Act boss
whose victory is followed by an Act transition or the end of the run.

Combat-search terminal HP already includes deterministic victory hooks such as
Burning Blood and Black Blood. It does not include the later dungeon transition:
A0-A4 restore to full, while A5+ restore 75% of missing HP. Consequently, the
A0 Collector line ending at 9/79 HP was rejected even though it would enter Act
3 at 79/79 HP.

## Approaches Considered

1. Remove the reserve for every boss-marked combat. This is too broad because
   event bosses and the first A20 double boss can have same-Act consequences.
2. Lower the fixed 25% threshold for bosses. This still ignores the actual
   recovery boundary and invents another unexplained threshold.
3. Classify whether a room-boss victory has another same-Act combat before a
   recovery/end boundary. This uses existing game state and is the selected
   approach.

## Decision

Keep the existing 25% reserve unless the active combat is a real
`MonsterRoomBoss` and winning it cannot lead directly to another same-Act boss.

- Act 1 and Act 2 room bosses: unlimited HP loss for a complete surviving win;
  the next Act transition provides mandatory recovery.
- Act 3 first boss at A20 while a second boss remains: retain the 25% reserve.
- Act 3 final room boss and Act 4 final boss: unlimited HP loss; the run ends or
  transitions before another risk.
- Event combats carrying boss metadata: retain the reserve because they are not
  Act-transition boundaries.
- Hallways and elites: unchanged.

`Unlimited` only removes the owner-audit HP-loss rejection. Search must still
produce an executable complete win, so a dead player is never accepted.

## Scope

Change only owner-audit survival policy and its focused lane-option tests. Do
not change generic run-control, combat search ranking, campfire Rest-vs-Smith,
or invent a future-Act pressure model.

## Verification

Use focused tests for an Act 2 room boss, an ordinary combat, an event boss,
and the first/second A20 double-boss boundary. Then run the full library and
architecture-sensitive suites and rerun the bounded seed once.
