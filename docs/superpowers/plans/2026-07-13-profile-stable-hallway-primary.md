# Profile-Stable Hallway Primary Search

## Observed gap

The same seed `20260713003` reached Act 2 floor 20 Spheric Guardian with the
same visible pre-combat HP, deck, gold, and potions in two runs.  The older
artifact did not retain an exact combat capture, so invisible state identity
cannot be proven from those two runs alone.  An exact replay of the new capture
nevertheless proves that the hallway primary's lazy child-rollout policy
produces radically different accepted results across build profiles:

- the older run found a 4-loss win;
- the exact capture under the current debug profile found a 16-loss win;
- five optimized-profile replays all stopped on the same 50-loss win.

The hard owner reserve permits up to 55 loss here, so the optimized search
commits its first survivable result after about 0.29 seconds.  A later immediate
escalation lane is never reached because the primary did not report a gap.

On the exact same capture, immediate child rollout found the same 10-loss win
under both optimized and debug builds.  The optimized run completed in about
0.20 seconds; the debug run found it within the existing one-second budget.

## Design

Use `Immediate` child rollout for the hallway primary profile.  The exact
capture then finds the same 10-loss line in both build profiles.  Preserve:

- the existing primary node and wall budgets;
- semantic potion policy with at most one potion;
- immediate high-stakes elite and boss primaries;
- lazy boss rescue lanes whose frozen fixtures specifically require lazy
  exploration;
- the post-primary immediate escalation as a larger-budget fallback when the
  primary genuinely finds no acceptable line.

This removes a build-speed-dependent policy split at the first hallway lane
without increasing configured budgets or weakening the HP-loss gate.

## Verification

- The lane-profile unit test must require immediate child rollout while keeping
  the primary budget and one semantic potion.
- The frozen Spheric Guardian capture must find the 10-loss line with both
  optimized and debug binaries.
- Owner-audit and combat-search regression suites must remain green.
