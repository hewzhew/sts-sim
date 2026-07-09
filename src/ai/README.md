# AI Module Map

`src/ai` contains policy, strategy, search, and diagnostic code. It is not one
flat policy layer. New code should choose an owner layer before it is added.

## Current Layers

| Area | Modules | Boundary |
| --- | --- | --- |
| shared facts and analysis | `analysis`, `strategic`, `strategy`, `route_window_facts`, `event_resource_budget` | Public-state facts and reusable strategy inputs. These modules should not click buttons or apply actions. |
| deck mutation | `deck_mutation_compiler_v1` | Shared compiler for remove/transform/sacrifice-style deck changes. Event owners should not copy card-target ordering. |
| non-combat policies | `*_policy_v1`, `noncombat_*`, `route_planner_v1` | Scene-specific policy adapters. They choose typed candidates and should delegate shared valuation to strategy/compiler modules. |
| combat automation | `combat_auto_policy_v1`, `combat_search_v2`, `combat_state_key`, `combat_state_snapshot` | In-combat search, keys, snapshots, and automation support. Combat search does not decide reward, shop, route, or event policy. |
| boss matchup work | `boss_matchup`, `boss_mechanics_v1` | Boss-specific facts and review-facing matchup analysis. Keep this evidence-oriented unless a separate policy boundary is explicitly introduced. |

## Naming Notes

Some active modules still end in `_v1`. That suffix is historical, not a license
to add parallel replacements. Prefer improving or retiring an existing boundary
over creating another scene-local copy of the same concept.

Stable public names should not include `_v1` once a boundary is intentionally
promoted. `route_window_facts` and `event_resource_budget` are examples of this
direction; their serialized output can still carry `schema_version = 1`.

## Design Rules

- Typed decisions and candidate ids are control surfaces. Display strings are
  only for humans.
- Shared deck, route, resource, and boss facts belong in analysis/strategy
  modules, not inside one event or shop owner.
- Event owners may decide whether to enter an event option. Card target choice
  belongs in the deck mutation compiler or a typed pending-choice bridge.
- Combat diagnostics may expose symptoms. They must not silently mutate
  non-combat policy.
- Offline learning exports record behavior-policy samples. They are not proof
  that a decision was good.

## Cleanup Direction

When cleaning this tree, prefer this order:

1. document the current boundary;
2. remove duplicate local logic from scene owners;
3. move shared facts into analysis/strategy/compiler modules;
4. delete the old entrypoint once nothing calls it.

Do not keep compatibility layers for convenience after the replacement boundary
is accepted.
