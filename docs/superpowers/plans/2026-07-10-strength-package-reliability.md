# Strength Package Reliability Implementation Plan

1. Add a failing inventory test showing that a strength multiplier is recorded without
   being counted as a stable strength source.
2. Add a failing acquisition test showing that a second stable source repairs a live
   multiplier package even when the purchase crosses purge reserve.
3. Add negative assertions for a third source, a conditional source, and a payoff.
4. Implement multiplier inventory and an explicit package-reliability strategic delta,
   construction role, and policy reason.
5. Add one decision-pipeline integration check, run focused tests, then run the full
   library and architecture-sensitive suites.
