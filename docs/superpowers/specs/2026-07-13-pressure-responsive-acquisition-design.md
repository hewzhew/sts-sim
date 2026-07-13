# Pressure-Responsive Acquisition Design

## Problem

The reward scorer and acquisition contract can reach contradictory conclusions.
On seed `20260713003`, Clothesline at A1F1 scored 290, Flame Barrier at A1F5
scored 180, and Iron Wave at A2F20 scored 280.  All had a raw mainline lane,
but acquisition capped them to inspect-only probes, so zero-score Skip won.

The cap is still useful for saturated low-margin attacks.  Removing it globally
would reintroduce deck bloat.

## Decision

Use two narrow, independently explainable exceptions:

1. A reward that is an immediate survival stabilizer under actual HP pressure
   is an accepted construction role even when the card is normally low-margin.
2. Known-Guardian evidence owns two concrete early survival repairs: the first
   Weak source (Clothesline) and the first substantial repeatable block card
   (Flame Barrier).  These become hard boss-survival contacts, not generic card
   quality exceptions.

Healthy, role-saturated low-margin filler remains speculative.  A card that
only touches an unrelated soft gap also remains a probe.

## Verification

Add regressions for Guardian Clothesline, Guardian Flame Barrier, and low-HP
Iron Wave.  Keep the existing saturation and soft-gap tests unchanged, run the
strategy suite, and rerun the bounded seed.
