# Marginal AoE Quality Implementation Plan

1. Add failing role-inventory and strategic-deficit tests for strong versus
   repeated weak AoE.
2. Add failing acquisition and decision-pipeline tests reconstructed from the
   seed `20260713003` failure.
3. Centralize strong-AoE classification in `deck_role_inventory` and expose the
   count through `DeckPlanSnapshot`.
4. Make acquisition delta and strategic-gap scoring candidate-quality-aware.
5. Run focused strategy tests, commit the behavior, and rerun the bounded seed.

