# Key Resource Event Bucket Template v0.1

This version is intentionally narrow.

It is **not** a general combat ontology.
It is a **working template for mitigation-type key resources**.

The purpose is to stop analyzing random seeds as if they were the object of
study, and instead analyze a small set of **decision-relevant event labels**.

## Scope

This template currently applies only to:

- mitigation-type key resources

Examples:

- `Disarm`
- `Piercing Wail`
- `Malaise`
- some mitigation potions

This version does **not** claim to cover:

- setup/payoff cards
- combo assembly
- general lethal-timing planning
- full belief-state planning

## Core Question

For a mitigation resource, first answer only these 3 questions:

1. Is it reachable before the first key attack window?
2. If reachable, is it playable before that window?
3. Does its value show up by window 1, or only by window 2 / later?

## Window Definitions

This version fixes exactly two windows.

### `window_1`

Definition:

- the **first major attack window**

Operational meaning:

- the first enemy attack window that is materially dangerous enough to change
  the decision about using the mitigation resource

Guardrail:

- `window_1` must be explicitly specified in the motif document **before**
  looking at the outcome analysis
- it must not be redefined after seeing the results
- the motif document should also state why this window was chosen

### `window_2`

Definition:

- the **next major attack window after `window_1`**

No further windows are part of v0.1.

If value is not visible by `window_2`, the current document may note that, but
the bucket system does not expand further.

## Two-Layer Labels

The old “4 buckets” were too mixed.
This version uses **two separate label layers**.

### Layer A: Opportunity State

Exactly one of:

- `unavailable_before_window_1`
- `available_but_unplayable_before_window_1`
- `available_and_playable_before_window_1`

#### `unavailable_before_window_1`

Definition:

- the resource is not reachable before `window_1`

#### `available_but_unplayable_before_window_1`

Definition:

- the resource is reachable before `window_1`
- but cannot be legally or practically played before `window_1`

For v0.1, “unplayable” means only:

- insufficient energy, or
- not actually in hand at a decision point before `window_1`

It does **not** yet include:

- broader opportunity cost
- “worth it” judgments
- combo opportunity loss

Interpretation note:

- in v0.1, `playable` means only **physically playable**
- it does not mean “strategically correct to play”

#### `available_and_playable_before_window_1`

Definition:

- the resource is reachable before `window_1`
- and there exists at least one legal decision point before `window_1` where it
  can be played

### Layer B: Value Timing

Exactly one of:

- `immediate_by_window_1`
- `delayed_to_window_2`
- `mostly_tail_risk_by_window_2`

#### `immediate_by_window_1`

Definition:

- the main mitigation value is already visible by `window_1`

#### `delayed_to_window_2`

Definition:

- the main mitigation value is not visible by `window_1`
- but becomes visible by `window_2`

#### `mostly_tail_risk_by_window_2`

Definition:

- mean hp-loss does not separate much by `window_2`
- but the worst-case / bad-tail outcomes improve by `window_2`

Interpretation note:

- for mitigation-type resources, `mostly_tail_risk_by_window_2` is not a
  fringe case
- it may be one of the main value forms

## Metrics

v0.1 uses only 5 primary metrics.

### Primary metrics

- `P(reachable_before_window_1)`
- `P(playable_when_reached_before_window_1)`
- `mean_hp_loss_to_window_1`
- `mean_hp_loss_to_window_2`
- `worst_20p_hp_loss_to_window_2`

Optional supporting metric:

- `catastrophe_rate_to_window_2`

### Not part of v0.1

These are explicitly deferred:

- `block_budget_saved`
- `lethal_timing_delta`
- general kill-speed metrics
- global reward summaries

## Probability Space

All probabilities in this template must name their probability space.

Default for v0.1:

- a fixed natural-start deck
- a fixed encounter
- a fixed seed set

So:

- `P(reachable_before_window_1)` means:
  the fraction of seeds in the chosen seed set where the resource is reachable
  before `window_1`

This is a local study definition, not a universal probability.

Sensitivity note:

- all probabilities in v0.1 are only valid for the explicitly named seed set
- if a different seed set materially changes the conclusion, the motif document
  should say so

## Minimal Output

Each motif document using this template should contain:

- `resource_role`
- `window_1`
- `window_2`
- Layer A label definitions
- Layer B label definitions
- the 5 primary metrics
- one short interpretation of what transfers beyond the specific card

## Appendix Boundary

If a motif document also contains:

- RL probe results
- policy metrics
- training traces

those belong to an appendix / probe section, not to the core template
definition.

## Anti-Drift Rule

If the document starts introducing:

- more than 2 windows
- setup/payoff language
- “worth it” logic hidden inside `playable`
- undefined budget metrics

then it has left v0.1 and should be split into a later version instead of being
expanded in place.
