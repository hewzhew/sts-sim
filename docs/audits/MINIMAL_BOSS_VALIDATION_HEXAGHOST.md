# Minimal Boss Validation: Ironclad vs Hexaghost

This document defines the next-step validation pack after the current
`decision_audit` scorer work.

It is intentionally small.

The purpose is **not** to prove that the agent can play a whole boss fight.
The purpose is to check whether the current method shift has practical value:

> in a fixed boss/deck setting, can we reduce obviously bad choices and express
> gray-zone decisions as preferences instead of fake "single best move" claims?

## Goal

Validate one concrete claim:

> the project should move from "force a single best first move" toward
> "prefer A / prefer B / close enough + short risk rationale" for boss-relevant
> tactical states.

This validation pack should answer whether that shift looks useful in practice.

## Why Hexaghost

Hexaghost is a good first boss target because it stresses exactly the things we
care about:

- multi-hit pressure windows
- timing-sensitive mitigation
- irreversible resource spending
- setup-vs-survival tradeoffs
- gray-zone tactical decisions where "least bad" matters more than unique optimality

It is also already represented in repo assets:

- `tests/decision_audit/hexaghost_frame_178.json`
- `tests/decision_audit/hexaghost_frame_203.json`
- preference seed samples around frame `202` in
  `data/combat_lab/policy_seed_set_20260412_214122.jsonl`

## Why This Pack Avoids Belief First

The first pass should **not** mix in uncertain monster-belief questions.

Use:

- fixed replay frames with explicit observed state
- author specs with explicit intent fields

Avoid:

- "unknown intent" states as the main evidence for the pack

Reason:

If the pack fails under fixed truth-like states, the problem is not belief yet.
If the pack works there and later breaks under uncertainty, then belief becomes
the next isolated problem.

## Deck Profile

Use one simple Ironclad midrange deck profile:

- a few upgraded attacks
- one or two timing-sensitive mitigation tools
- limited potion support
- no elaborate combo engine

The current replay-derived deck around the existing Hexaghost frames is already
close enough for the first validation pass:

- `Pummel+`
- `Sword Boomerang+`
- `Clothesline+`
- `Strike+`
- `Pommel Strike`
- `Armaments+`

This is good enough because it contains:

- multi-hit payoff timing
- immediate pressure handling
- nontrivial sequencing under limited energy

## Validation Output Format

For each state, do **not** ask for a full ranking over all legal moves.

Instead, compare only 2 to 3 candidate lines and produce:

- `prefer A`
- `prefer B`
- `close enough`

Plus one short rationale from this set:

- current-window relief
- next-window risk
- irreversible resource spend
- setup deferral / payoff timing

This pack is successful if the reasoning becomes stable and not obviously fake.

## The Three-State Pack

### State 1: Known-Intent Defensive Pressure Anchor

Status: ready from existing data

Primary source:

- preference seed sample frame `202` in
  `data/combat_lab/policy_seed_set_20260412_214122.jsonl`

Observed state summary:

- encounter: `Hexaghost`
- player hp: `16`
- energy: `3`
- incoming: `14`
- hand: `Pommel Strike`, `Pummel+`, `Clothesline+`, `Strike+`, `Strike+`
- monster intent is explicit attack with `5 x 2`

Why this state matters:

- it is a real boss pressure window
- it already exposes "damage now vs mitigation now"
- it does not require belief to interpret

Candidate lines for first pass:

- `A`: prioritize immediate mitigation / weaken line
  Example anchor: `Clothesline+` line
- `B`: prioritize immediate damage line
  Example anchor: `Pommel Strike` or `Pummel+` line
- optional `C`: `EndTurn` if it is strategically relevant as a control

Expected use:

- not to prove exact best play
- to check whether the system can reliably reject the most reckless damage-first line

### State 2: Known-Intent Burn-in-Hand Pressure Anchor

Status: ready from existing fixture

Primary source:

- `tests/decision_audit/hexaghost_frame_203.json`

Observed state summary:

- encounter: `Hexaghost`
- player hp: `16`
- energy: `2`
- hand: `Pummel+`, `Clothesline+`, `Strike+`, `Strike+`, `Burn`
- monster intent is explicit attack with `7 x 2`
- `Hexaghost` already has `Strength=2`

Why this state matters:

- it is another hard pressure window
- it combines status burden with immediate multi-hit damage
- it is a boss state where "just race" can be suicidal

Candidate lines for first pass:

- `A`: immediate mitigation / safer line
- `B`: damage-race line
- optional `C`: close substitute line if two mitigation choices are both plausible

Expected use:

- check whether preference-style output can keep "damage race" from dominating when
  survival pressure is too high

### State 3: Authored Disarm-Timing Boss State

Status: to author next

Primary target motif:

- "against Hexaghost multi-hit opener / strong multi-hit window, should `Disarm`
  be spent early rather than held?"

Why this state matters:

- this is the practical knowledge you explicitly care about
- it should be learned as a state relation, not as a monster-id hardcode

Requirements for the authored state:

- explicit `Hexaghost` attack intent
- hand contains `Disarm`
- at least one tempting alternative line that spends energy on damage or setup
- enough pressure that holding `Disarm` is risky
- simple enough that the comparison is interpretable

Candidate lines:

- `A`: `Disarm` now
- `B`: hold `Disarm`, spend energy on damage / setup
- optional `C`: mixed line if needed

Success condition:

- the system should at least prefer the safer early mitigation line or mark the
  alternatives as clearly worse under current-window risk

## Success Criteria

This pack is good enough if it demonstrates all three:

1. It can reject obviously bad boss lines in high-pressure windows.
2. It can express gray-zone states as `prefer A / prefer B / close enough`
   without pretending every state has a single clean optimum.
3. It produces short rationales that reuse a small stable set of risk concepts
   instead of inventing a new explanation every time.

This pack is **not** required to show:

- whole-fight victory improvement
- belief correctness
- generalization across all bosses
- final learner-ready supervision

## Immediate Next Actions

1. Keep State 1 and State 2 as the first two fixed anchors.
2. Author State 3 as a small explicit Hexaghost + `Disarm` boss spec.
3. For each state, compare only the named candidate lines.
4. Record output in the format:

   - decision: `prefer A / prefer B / close enough`
   - rationale: one sentence using the allowed risk vocabulary

## Explicit Deferrals

Not part of this validation pack:

- `Evolve`
- `Fire Breathing`
- full belief repair
- broad boss benchmarking
- learned preference pipeline
- full `decision_audit` rewrite

This pack exists to answer one smaller question first:

> does the method shift have visible boss-fight value under fixed, interpretable
> tactical states?
