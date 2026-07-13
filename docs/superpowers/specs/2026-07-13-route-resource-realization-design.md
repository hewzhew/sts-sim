# Route resource realization design

## Evidence

Seed `20260713002`, Act 3 floor 41, exposed the complete typed route pool:

- player 86/101 HP, 405 gold, Time Eater ahead;
- selected Rest path scored 1.22 and had no remaining shop path;
- Event path scored 0.55, while its representative controllable continuation contained a Shop before the next Elite.

The score gap had two structural causes. A controllable shop on the representative path received only the small whole-family optionality value, while one campfire access received both full heal and full upgrade utility even though one campfire action cannot realize both.

## Design

1. Value shops on the selected representative path as controllable access. Keep a smaller additional bonus when every continuation in the family contains a shop, so guaranteed families remain preferable to merely reachable shops.
2. Allocate overlapping campfire access to the larger of heal or upgrade need first. Only non-overlapping residual access may score the other action. Keep separate `heal` and `upgrade` score terms, but make them realized rather than additive fantasies.
3. Treat seven floors before an act boss as the final practical shop-search window when at least twice the normal shop gold is unconverted. The previous five-floor window activates after important forks on the observed Act 3 map.

These rules use route resources and player needs only. They do not name Time Eater, a seed, a card, or a fixed gold amount.

## Non-goals

- Do not make shops universally outrank safety.
- Do not treat shops as HP recovery.
- Do not change combat search or shop purchase scoring here.
- Do not claim the counterfactual shop guarantees victory; first restore the route option, then rerun.

