# Post-Gap Hallway Potion Rescue Design

## Problem

On the pressure-responsive seed003 route, the primary hallway search found no
win and its immediate no-potion escalation found only a 75-to-13 win.  The HP
reserve correctly rejected that line, but the portfolio stopped because potion
rescue was gated by *starting* HP being at most 50%.

The saved Spheric Guardian fixture disproves that stop: the existing quality
lane finds a one-potion 75-to-54 win, and the line lab finds an even safer
two-potion line.  The missing behavior is orchestration, not a looser HP limit.

## Decision

After primary search has already produced a combat gap, any non-boss combat
with a usable potion may schedule the bounded hallway quality/potion lane.
Starting HP is not a useful gate at that point: a high-HP combat can still have
only catastrophically expensive discovered wins.

The lane remains bounded, uses semantic potion actions, preserves the existing
HP acceptance limit, and runs only after the ordinary primary search fails.
The no-potion immediate escalation remains first.

## Verification

Add a context regression for a 75/80 hallway with a usable potion, retain the
no-potion/no-rescue boundary, run the owner-audit portfolio tests, and rerun the
saved seed.
