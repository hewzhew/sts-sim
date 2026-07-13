# Card-flow capability progress implementation plan

1. Add failing unit coverage in `deck_construction_pressure.rs` for first real draw versus another
   small cantrip in a thin-flow deck.
2. Add a failing decision-pipeline regression matching the Act 3 seed state and requiring Battle
   Trance to remain mainline and outrank Cleave.
3. Extend `candidate_improves_card_flow` to recognize first real draw and first energy access while
   preserving the existing pressure-level comparison.
4. Run focused strategy tests and inspect the exact candidate lane/caps.
5. Build the fast runner and rerun seed `20260713002` with the same bounded contract.
6. Run full verification and commit the behavior change independently of generated artifacts.
