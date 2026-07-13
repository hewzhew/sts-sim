# Smoke Bomb survival fallback design

## Problem

The owner-audit portfolio correctly excludes Smoke Bomb from combat-win search, but its elite and hallway survival lanes also disable the run-control no-win fallback that performs an explicit, non-victory Smoke Bomb escape. Enabling the existing fallback wholesale would repeat the complete-line and turn-pool solvers before escaping, adding several seconds to every failed survival lane.

## Design

Keep three meanings separate:

- normal search finds and accepts combat victories;
- full no-win rescue may run additional winning-line solvers and then escape;
- Smoke Bomb survival fallback only performs a legal escape and never reports a combat victory.

Add an explicit `allow_smoke_bomb_survival_fallback` run-control option. When full no-win rescue is disabled, `apply_search_combat` may still apply only the Smoke Bomb fallback when this option is enabled. Owner-audit enables it for `EliteSurvivalFallback` and `HallwaySurvivalFallback`; primary, quality, and boss lanes remain unchanged. `DiagnosticRescue` retains the full rescue chain.

## Safety boundaries

- Do not admit Smoke Bomb into semantic winning-search actions.
- Do not enable escape for boss combat.
- Do not repeat complete-line or turn-pool rescue in survival lanes.
- Preserve existing manual/default run-control behavior.
- Record the survival lane as having an internal no-win rescue capability in portfolio evidence.

