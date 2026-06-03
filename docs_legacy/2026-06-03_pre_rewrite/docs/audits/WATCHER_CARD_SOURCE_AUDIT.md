# Watcher Card Source Audit

Date: 2026-05-16

This audit records the current Watcher card/relic/potion/power parity closure before moving to the
monster audit. It is not a permanent proof that Watcher is finished. It is a source-backed stop
point: the active registered Watcher card set, Watcher-generated cards, stance/power hooks, and
Watcher-specific relic/potion hooks have been checked closely enough that the next highest-value
work is monster runtime parity.

## Source Roots

Java source roots used:

- `D:/rust/cardcrawl/cards/purple`
- `D:/rust/cardcrawl/cards/tempCards`
- `D:/rust/cardcrawl/actions/watcher`
- `D:/rust/cardcrawl/powers/watcher`
- relevant Watcher relic and potion Java source files under `D:/rust/cardcrawl/relics` and
  `D:/rust/cardcrawl/potions`

Rust roots used:

- `src/content/cards/watcher`
- `src/content/cards/colorless` for Watcher-generated temporary cards
- `src/content/powers/watcher/mod.rs`
- `src/content/relics`
- `src/content/potions`
- `src/engine/action_handlers`

## Coverage Summary

| Area | Current status | Notes |
| --- | --- | --- |
| Registered purple card pool | Short-term closed | Java source contains `Discipline.java` and `Unraveling.java`, but they are source-present and unregistered by the active card library. Rust should not add them to the playable pool unless source registration changes are intentionally modeled. |
| Card definitions and upgrades | Short-term closed | Watcher card files have been migrated to the card-definition/evaluator pattern used by the other completed classes. Upgrade-sensitive values should come from evaluated card state, not ad hoc test mutation. |
| Normal play effects | Short-term closed | Active Watcher use paths were checked against Java card sources and Watcher action sources. Runtime behavior is covered by card/action tests for stance entry/exit, scry, mark, retain growth, generated cards, X-cost, delayed turn skip, and similar mechanics. |
| `canUse` / playability | Short-term closed | Java purple/temp card `canUse` overrides are limited to `DeusExMachina` and `SignatureMove`. Rust models Deus as unplayable from hand and Signature Move as requiring no other Attack in hand. Forced/autoplay paths are not assumed to bypass this unless the engine path explicitly does so. |
| Draw/retain/stance hooks | Short-term closed | `DeusExMachina.triggerWhenDrawn`, `FlurryOfBlows.triggerExhaustedCardsOnStanceChange`, and retain-growth cards (`Perseverance`, `SandsOfTime`, `WindmillStrike`) have explicit Rust coverage. |
| Watcher powers | Short-term closed | Mantra, Devotion, Like Water, Mental Fortress, Rushdown, Nirvana, Master Reality, Study, Battle Hymn, Foresight, Wave of the Hand, Deva, Collect, EnergyDown, Vigor, Omega, and related sentinel-power behavior have been checked. |
| Watcher relics | Short-term closed | Pure Water, Holy Water, Damaru, Teardrop Locket, Violet Lotus, Golden Eye, Melange, Cloak Clasp, and Duality have source-backed tests or metadata coverage. Starter-boss relic replacement gates are locked. |
| Watcher potions | Short-term closed | Bottled Miracle, Stance Potion, and Ambrosia have metadata/action tests. |
| Generated/temp cards | Short-term closed | Smite, Safety, Insight, Miracle, Through Violence, Expunger, Beta, and Omega have Rust implementations and tests. Shiv is shared Silent/generated content, not Watcher-specific. |
| Watcher action classes | Short-term closed for active gameplay paths | Active card-linked action classes are represented by Rust action handlers or direct card/power logic. UI/VFX/pacing action classes are intentionally omitted when they do not mutate gameplay state. |

## Active Action Classes Checked

These Java action families were used as source truth for Watcher mechanics and have Rust equivalents
or direct Rust logic:

- `ChangeStanceAction`
- `CollectAction`
- `ConjureBladeAction`
- `CrushJointsAction`
- `FearNoEvilAction`
- `FollowUpAction`
- `HeadStompAction` / Sash Whip behavior
- `IndignationAction`
- `InnerPeaceAction`
- `JudgementAction`
- `LessonLearnedAction`
- `MeditateAction`
- `OmniscienceAction`
- `PressEndTurnButtonAction` and `SkipEnemiesTurnAction` as the Vault turn-skip path
- `SanctityAction`
- `SpiritShieldAction`
- `TriggerMarksAction`
- `WallopAction`

## Intentional Omissions

These are not currently modeled as Rust gameplay mechanics:

- VFX/SFX/action-pacing-only classes, such as `ExpungeVFXAction`, when they only animate or wait.
- UI glow checks when the underlying runtime action has a separate mechanical implementation.
- Java debug-only branches, such as debug damage in starter cards.
- Source-present but unregistered/deprecated Watcher content unless it becomes reachable through the
  real Java registration path.

If a future Java-source pass finds that one of these classes mutates real combat state, this audit
must be reopened and the omission must be reclassified.

## Risks Carried Forward

These are shared-mechanic risks, not Watcher-specific blockers:

- Draw-pile API consistency: top/bottom/random insertion must be expressed through one Rust API.
- Generated cards entering draw pile, hand, discard, exhaust, or limbo must preserve Java zone
  semantics and card-instance identity.
- Random target selection must be source-checked, especially for multi-enemy fights and dead/escaped
  monsters.
- Choice actions must preserve source, purpose, constraints, selected-so-far state, and replayable
  candidate identity.
- Post-combat cleanup and end-of-combat hooks need a source-backed pass before run-level AI data is
  trusted.
- Card instance copying, temporary cost, cost-for-combat, misc value, and generated-card upgrades
  remain high-risk cross-class mechanics.

## Decision

Do not start events yet. Move next to a systematic monster audit:

1. Act 1 hallway monsters.
2. Act 1 elites.
3. Act 1 bosses.
4. Act 2 hallway/elites/bosses.
5. Act 3 hallway/elites/bosses.
6. Act 4.

For each monster, compare Java and Rust on move selection RNG, intent structure, hit counts, block,
buff/debuff applications, status-card generation, summons/splits/revives, death behavior, turn
counters, and ascension thresholds.
