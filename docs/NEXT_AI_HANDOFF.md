# Next AI Handoff

Date: 2026-05-18
Branch: `codex/evidence-path-cleanup-20260509`
Workspace: `D:\rust\sts_simulator`
Java source reference: `D:\rust\cardcrawl`
CommunicationMod reference: `D:\rust\CommunicationMod`

## Purpose

This file is the durable working memory for context compaction. At the start of
any resumed turn, read only:

1. `git status --short`
2. `git log --oneline -5`
3. this file

Do not re-read broad source trees just to rediscover recent state. Use this file
to choose the next narrow Java/Rust evidence packet.

When selecting that packet, prefer the durable indexes:

- `docs/JAVA_SOURCE_MAP.md`
- `docs/MECHANICS_AUDIT_LEDGER.md`

Update those files whenever a Java source owner, Rust owner, audit status, or
next-lane recommendation changes.

## Current Rule

Continue Java-source-backed mechanics cleanup for a Rust simulator intended for
AI use.

Allowed:

- Preserve Java gameplay semantics from `D:\rust\cardcrawl`.
- Change Rust architecture when the current one hides or distorts Java state.
- Omit UI/VFX only when it is truly presentation-only.
- Keep UI-tied Java behavior only when it mutates gameplay state, consumes
  gameplay RNG, gates choices, changes visibility, or affects replay.
- Encode resolved source comparisons as tests, audit notes, and commits.

Forbidden:

- Strategy heuristics, seed patches, bot compatibility layers, CleanRL/Gym-first
  constraints, or policy logic.
- Simulating UI effects for their own sake.
- Treating Java private mechanical fields as inferable from `move_history`
  unless Java itself only uses history.
- Re-reading large trees after compaction without first checking this file.

## Latest Mechanics Checkpoint

Latest code commit:

- `72e808e Match Tiny House upgrade candidates`

Recent commits:

- `72e808e Match Tiny House upgrade candidates`
- `a1e216c Update handoff after Pandora's Box audit`
- `0a795a8 Match Pandora's Box confirmation obtain order`
- `83793f4 Update handoff after Astrolabe audit`
- `586fff0 Match Astrolabe transform upgrade semantics`
- `4895ac6 Lock Cursed Key chest obtain hooks`
- `b2cc6ce Lock shop card fast obtain ordering`
- `dcec769 Lock reward card obtain hooks`
- `4d3d455 Lock Note For Yourself manual obtain hooks`
- `a69430d Lock basic curse obtain hooks`
- `b84fd78 Lock grid event obtain hooks`
- `1022eb3 Lock delayed obtain hooks in Mausoleum and Mind Bloom`
- `a3b9be9 Lock relic-before-curse obtain hooks`
- `298b8b1 Lock remaining match event obtain hooks`
- `3426913 Lock event card obtain after deck costs`
- `525fe0b Lock event card cost before obtain hooks`
- `56bec7f Lock remaining delayed event obtain hooks`
- `81789e4 Lock delayed event card obtain ordering`
- `aadd74e Defer multi-transform obtains like Java effects`
- `84ad08f Use Java upgrade helper for master deck upgrades`
- `7529c30 Match Bonfire reward before card removal`
- `6352ba4 Store We Meet Again card option by uuid`
- `8e4b627 Match Golden Wing purge screen flow`
- `a75011b Match obtain hook order with Java`
- `5e0aff5 Match Vampires strike removal order`
- `c5a3cd5 Update handoff after Drug Dealer audit`
- `f775832 Lock Drug Dealer transform order`
- `f6abf75 Match Neow transform-two removal order`
- `7dd8cf4 Match campfire toke selection with Java`
- `08bcd42 Match fountain curse removal order with Java`
- `e841d93 Update handoff after shop purge audit`
- `d4155a0 Match shop purge selection with Java`
- `d8c5796 Separate direct master deck removal paths`
- `efbf00f Match master deck removal hooks with Java`
- `d3c080e Align master deck copy state with Java`
- `af79d1b Queue Anger discard copies as stat snapshots`
- `8d48e33 Match generated reward discard upgrade counts`
- `ab78536 Lock stat equivalent copy state for queued copies`
- `23d034d Lock Nightmare stat equivalent payload state`
- `773cc7a Lock stasis and discard deck generated card semantics`
- `3f90fe6 Match Java discard temp card amount gate`
- `59ce922 Lock draw pile large temp card semantics`
- `5531d87 Update handoff after potion hand card audit`
- `3ec5e96 Construct potion hand cards with state`
- `9ba64f7 Update handoff after static hand card audit`
- `d618731 Construct static hand card producers with state`
- `c4bdd90 Update handoff after hand card construction audit`
- `7d9e17a Prepare concrete hand cards at construction`
- `be1bb3c Update handoff after constructed hand card audit`

`72e808e` summary:

- Audited `TinyHouse` against Java's upgrade, max HP, room reward, potion, and
  combat reward screen flow.
- Java checked:
  - `D:\rust\cardcrawl\relics\TinyHouse.java`
  - `D:\rust\cardcrawl\screens\CombatRewardScreen.java`
  - `D:\rust\cardcrawl\rooms\AbstractRoom.java`
  - `D:\rust\cardcrawl\rewards\RewardItem.java`
  - `D:\rust\cardcrawl\cards\AbstractCard.java`
- Java result:
  - `TinyHouse.onEquip` builds upgrade candidates strictly with
    `AbstractCard.canUpgrade()`, so status and curse cards are excluded and
    `SearingBlow` remains always eligible.
  - It shuffles candidates with `new Random(miscRng.randomLong())`, upgrades
    at most one card, and runs bottled-card description checks.
  - It immediately increases max HP/current HP by 5.
  - It adds 50 gold and a `miscRng` potion to current-room rewards, then
    `CombatRewardScreen.open(label)` copies those rewards and appends the
    ordinary card reward through `setupItemReward`.
- Rust result:
  - `tiny_house::on_equip` now uses the shared Java upgrade helper
    `can_upgrade_card_once` for candidate collection.
  - Added a regression proving Tiny House does not upgrade status/curse cards.

Verification for `72e808e`:

- `cargo test tiny_house --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1405 passed`

Next source-backed lane:

- Continue relic obtain/equip audit with `Cauldron` and `Orrery`.

`0a795a8` summary:

- Audited `PandorasBox` against Java's direct deck removal, random card
  generation, preview hooks, confirmation grid, and fast obtain path.
- Java checked:
  - `D:\rust\cardcrawl\relics\PandorasBox.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
  - `D:\rust\cardcrawl\screens\select\GridCardSelectScreen.java`
  - `D:\rust\cardcrawl\vfx\FastCardObtainEffect.java`
  - `D:\rust\cardcrawl\cards\Soul.java`
  - Egg relic sources for `onPreviewObtainCard`
- Java result:
  - `PandorasBox.onEquip` removes all cards tagged `STARTER_STRIKE` or
    `STARTER_DEFEND` directly from `masterDeck.group`, bypassing normal removal
    hooks.
  - It generates the same number of cards with
    `AbstractDungeon.returnTrulyRandomCard()` using `cardRandomRng`.
  - Only Egg relics override `onPreviewObtainCard`, so generated cards preview
    upgrade before being placed into the confirmation grid.
  - The confirmation grid stores cards with `CardGroup.addToBottom`, which
    inserts at Java index 0; confirming the grid queues `FastCardObtainEffect`
    in that reversed group order.
  - `FastCardObtainEffect` then runs ordinary obtain hooks such as Ceramic Fish
    before `Soul.obtain`.
- Rust result:
  - Generated Pandora cards are now collected first, then obtained in the
    reverse order matching Java's confirmation-grid order.
  - Added regressions proving:
    - generated-vs-obtained card order is reversed like Java;
    - Egg preview upgrades happen;
    - Ceramic Fish fires once per generated card.

Verification for `0a795a8`:

- `cargo test pandoras_box --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1404 passed`

Next source-backed lane:

- Continue relic obtain/equip audit with `Cauldron` and `Orrery`.

`586fff0` summary:

- Audited `Astrolabe` against Java's selection, transform, and obtain path.
- Java checked:
  - `D:\rust\cardcrawl\relics\Astrolabe.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
  - `D:\rust\cardcrawl\dungeons\AbstractDungeon.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
- Java result:
  - `Astrolabe.onEquip` uses `masterDeck.getPurgeableCards()`, excluding
    `AscendersBane`, `CurseOfTheBell`, and `Necronomicurse`.
  - If there are three or fewer candidates, Java calls `giveCards` immediately;
    otherwise it opens a non-cancelable grid selection for exactly three cards.
  - `giveCards` removes selected cards from the master deck, transforms each
    with `AbstractDungeon.transformCard(card, true, miscRng)`, then queues
    `ShowCardAndObtainEffect` for transformed cards.
  - Java's `autoUpgrade=true` still calls `transformedCard.canUpgrade()`, so a
    transformed curse/status must not become upgraded.
- Rust result:
  - `Astrolabe` now uses deferred transform obtain for the auto-transform path.
  - `RunPendingChoiceReason::TransformUpgraded` now uses the same deferred
    transform obtain path.
  - Transformed cards are only pre-upgraded when
    `can_upgrade_card_once(transformed_card)` is true.
  - Added a regression proving a purgeable curse transformed by Astrolabe
    remains an unupgraded curse.

Verification for `586fff0`:

- `cargo test astrolabe --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1403 passed`

Next source-backed lane:

- Continue relic obtain/equip audit with `PandorasBox`, `TinyHouse`,
  `Cauldron`, and `Orrery`.

`71c92b1` summary:

- Audited `Necronomicon` against Java's `ShowCardAndObtainEffect` obtain path.
- Java checked:
  - `D:\rust\cardcrawl\relics\Necronomicon.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
- Java result:
  - `Necronomicon.onEquip` queues
    `ShowCardAndObtainEffect(new Necronomicurse())`.
  - That means the Necronomicurse uses ordinary delayed obtain semantics:
    Omamori can intercept it, and normal obtain hooks such as Ceramic Fish run
    before `souls.obtain`.
  - `Necronomicon.onUnequip` is different: it directly removes the first
    Necronomicurse from `masterDeck.group`, so it must not trigger the curse's
    removal self-regeneration hook.
- Rust result:
  - No business logic change was needed.
  - Added regressions proving:
    - `CeramicFish` gold is emitted before `Necronomicurse` `CardObtained`.
    - Omamori can block the Necronomicurse from `Necronomicon.onEquip`.

Verification for `71c92b1`:

- `cargo test necronomicon --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1402 passed`

Next source-backed lane:

- Continue relic obtain/equip audit with `Astrolabe`, `PandorasBox`,
  `TinyHouse`, `Cauldron`, and `Orrery`.

`72da496` summary:

- Audited `CallingBell` against Java's confirmation-grid and fast obtain path.
- Java checked:
  - `D:\rust\cardcrawl\relics\CallingBell.java`
  - `D:\rust\cardcrawl\screens\select\GridCardSelectScreen.java`
  - `D:\rust\cardcrawl\vfx\FastCardObtainEffect.java`
- Java result:
  - `CallingBell.onEquip` opens a confirmation grid containing
    `CurseOfTheBell`; it does not directly add the card at that line.
  - Confirming that grid queues `FastCardObtainEffect` for the curse.
  - `FastCardObtainEffect` means Omamori can intercept the curse, and normal
    obtain hooks such as Ceramic Fish run before `souls.obtain`.
  - `CallingBell.update` then opens three screenless relic rewards once the
    confirmation screen is down.
- Rust result:
  - No business logic change was needed.
  - Added regressions proving:
    - `CeramicFish` gold is emitted before `CurseOfTheBell` `CardObtained`.
    - Omamori can block `CurseOfTheBell`, while the three relic rewards still
      open.

Verification for `72da496`:

- `cargo test calling_bell --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1400 passed`

Next source-backed lane:

- Continue relic obtain/equip audit with `Necronomicon`, `Astrolabe`,
  `PandorasBox`, `TinyHouse`, `Cauldron`, and `Orrery`.

`4895ac6` summary:

- Audited the non-event chest curse obtain path for `CursedKey`.
- Java checked:
  - `D:\rust\cardcrawl\relics\CursedKey.java`
  - `D:\rust\cardcrawl\rewards\chests\AbstractChest.java`
  - `D:\rust\cardcrawl\helpers\CardLibrary.java`
- Java result:
  - `CursedKey.onChestOpen(false)` adds a
    `ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse())` to
    `topLevelEffects`.
  - The ordinary chest then adds gold/relic/key rewards and opens the reward
    screen.
  - The queued curse obtain still uses the standard
    `ShowCardAndObtainEffect` ordering: relic `onObtainCard` before
    `souls.obtain`.
- Rust result:
  - No business logic change was needed.
  - Added a treasure-room regression proving `CursedKey` curse obtain emits
    `CeramicFish` gold before the curse `CardObtained` event.

Verification for `4895ac6`:

- `cargo test cursed_key_chest --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1398 passed`

`b2cc6ce` summary:

- Audited shop card purchase against Java's `ShopScreen.purchaseCard` and
  `FastCardObtainEffect`.
- Java checked:
  - `D:\rust\cardcrawl\shop\ShopScreen.java`
  - `D:\rust\cardcrawl\vfx\FastCardObtainEffect.java`
- Java result:
  - `purchaseCard` first creates a `FastCardObtainEffect` for the hovered card.
  - It then spends gold and handles Courier replacement/removal.
  - The fast obtain effect later runs relic `onObtainCard` before
    `souls.obtain`.
  - If Omamori blocks a curse in the effect constructor, the purchase still
    spends gold because the spend happens after the effect is constructed.
- Rust result:
  - No business logic change was needed.
  - Added shop-handler regressions proving:
    - Shop spend is emitted before `CeramicFish` obtain-hook gold, which is
      emitted before `CardObtained`.
    - A shop curse blocked by Omamori is not obtained, but gold is still spent.

Verification for `b2cc6ce`:

- `cargo test shop_handler --all-targets` -> `16 passed`
- `cargo test --all-targets` -> `1397 passed`

`dcec769` summary:

- Audited ordinary reward-screen card selection against Java's
  `RewardItem.claimReward` -> `CardRewardScreen` -> `FastCardObtainEffect`
  path.
- Java checked:
  - `D:\rust\cardcrawl\rewards\RewardItem.java`
  - `D:\rust\cardcrawl\screens\CardRewardScreen.java`
  - `D:\rust\cardcrawl\vfx\FastCardObtainEffect.java`
  - `D:\rust\cardcrawl\cards\Soul.java`
- Java result:
  - Claiming a `CARD` reward opens `CardRewardScreen` and does not immediately
    remove the reward item via `RewardItem.claimReward`.
  - Selecting a card queues `FastCardObtainEffect`.
  - `FastCardObtainEffect` has the same key obtain mechanics as
    `ShowCardAndObtainEffect`: Omamori can intercept curses in the constructor,
    and later update runs relic `onObtainCard` before `souls.obtain`.
- Rust result:
  - No business logic change was needed.
  - Added reward-handler regressions proving:
    - Reward card selection emits `CeramicFish` gold before `CardObtained`.
    - Omamori intercepts a curse selected from a reward-card row and no
      `CardObtained` event is emitted for that curse.

Verification for `dcec769`:

- `cargo test rewards::handler --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1395 passed`

`4d3d455` summary:

- Audited the non-`ShowCardAndObtainEffect` manual obtain path in
  `NoteForYourself`.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\NoteForYourself.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
- Java result:
  - Taking the stored note card manually calls every relic's `onObtainCard`.
  - Then it calls `masterDeck.addToTop(obtainCard)`.
  - Then it calls every relic's `onMasterDeckChange`.
  - Then it opens
    `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`.
  - This path does not use `ShowCardAndObtainEffect`, so Omamori does not
    intercept curses, but normal obtain hooks such as `CeramicFish` and Egg
    upgrades still apply.
- Rust result:
  - No business logic change was needed.
  - Added regressions proving:
    - `CeramicFish` gold is emitted before the note card `CardObtained`
      record.
    - `MoltenEgg` upgrades an unupgraded stored Attack note card before it is
      added to the master deck.

Verification for `4d3d455`:

- `cargo test note_for_yourself --all-targets` -> `9 passed`
- `cargo test --all-targets` -> `1393 passed`

`a69430d` summary:

- Continued the Java `ShowCardAndObtainEffect` sweep into the remaining simple
  curse obtain event paths.
- Java checked:
  - `D:\rust\cardcrawl\events\city\ForgottenAltar.java`
  - `D:\rust\cardcrawl\events\exordium\GoldenIdolEvent.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `ForgottenAltar` Desecrate queues `ShowCardAndObtainEffect(new Decay())`.
  - `GoldenIdol` Run trap queues `ShowCardAndObtainEffect(new Injury())`.
  - The queued effect later calls relic `onObtainCard` before `souls.obtain`.
- Rust result:
  - No business logic change was needed.
  - Added `CeramicFish` ordering regressions proving the obtain hook gold is
    emitted before the Decay/Injury `CardObtained` records.

Verification for `a69430d`:

- `cargo test forgotten_altar --all-targets` -> `7 passed`
- `cargo test golden_idol --all-targets` -> `10 passed`
- `cargo test --all-targets` -> `1391 passed`

`b84fd78` summary:

- Continued the Java `ShowCardAndObtainEffect` sweep into grid-selection event
  obtains.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\Duplicator.java`
  - `D:\rust\cardcrawl\events\city\TheLibrary.java`
  - `D:\rust\cardcrawl\cards\AbstractCard.java`
  - `D:\rust\cardcrawl\relics\MoltenEgg2.java`
  - `D:\rust\cardcrawl\relics\ToxicEgg2.java`
  - `D:\rust\cardcrawl\relics\FrozenEgg2.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
- Java result:
  - `AbstractCard.makeStatEquivalentCopy()` copies combat cost flags,
    `freeToPlayOnce`, base stats, bottle flags, seen/locked, and `misc`.
  - `Duplicator` clears bottle flags on the copied card, then queues
    `ShowCardAndObtainEffect`.
  - Egg relics implement `onPreviewObtainCard` by calling `onObtainCard`, and
    `ShowCardAndObtainEffect.update()` runs `onObtainCard` before
    `souls.obtain`, so duplicated/selected event cards can still be upgraded by
    Egg at obtain time.
- Rust result:
  - No business logic change was needed.
  - Added regressions proving:
    - Duplicator's copied unupgraded Strike is upgraded by `MoltenEgg` during
      obtain, while the original selected master-deck Strike remains unupgraded.
    - `CeramicFish` gold is emitted before the Duplicator/TheLibrary
      `CardObtained` records.

Verification for `b84fd78`:

- `cargo test duplicator --all-targets` -> `3 passed`
- `cargo test the_library --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1389 passed`

`1022eb3` summary:

- Continued the Java event `ShowCardAndObtainEffect` sweep into delayed obtain
  paths with meaningful pre-obtain ordering:
  - `Mausoleum` constructs the possible Writhe effect before immediately
    obtaining the random relic.
  - `MindBloom` low-floor Desire immediately gains 999 gold before queueing two
    Normality `ShowCardAndObtainEffect`s.
- Java checked:
  - `D:\rust\cardcrawl\events\city\TheMausoleum.java`
  - `D:\rust\cardcrawl\events\beyond\MindBloom.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Rust result:
  - No business logic change was needed.
  - Added `CeramicFish` ordering regressions:
    - `Mausoleum`: forced newly obtained `CeramicFish` is obtained before the
      delayed Writhe obtain hook, and that hook emits gold before Writhe's
      `CardObtained` record.
    - `MindBloom`: the 999-gold event precedes both Normality obtain hooks, and
      each Normality hook emits `CeramicFish` gold before that Normality's
      `CardObtained` record.

Verification for `1022eb3`:

- `cargo test mausoleum --all-targets` -> `5 passed`
- `cargo test mind_bloom --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1387 passed`

`a3b9be9` summary:

- Continued the Java `ShowCardAndObtainEffect` event sweep into choices that
  construct a curse obtain effect before immediately obtaining a relic.
- Java checked:
  - `D:\rust\cardcrawl\events\exordium\BigFish.java`
  - `D:\rust\cardcrawl\events\city\Addict.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `BigFish` Box constructs the Regret `ShowCardAndObtainEffect`, then calls
    `spawnRelicAndObtain`.
  - `Addict` Rob constructs the Shame `ShowCardAndObtainEffect`, then calls
    `spawnRelicAndObtain`.
  - Omamori interception is based on the pre-relic snapshot because it happens
    in the effect constructor, but other obtain hooks such as newly obtained
    `CeramicFish` see the curse later when the delayed effect resolves.
- Rust result:
  - No business logic change was needed.
  - Added `CeramicFish` ordering regressions proving `RelicObtained` happens
    before the delayed curse obtain hook, and that hook emits its gold before
    the curse `CardObtained` record.

Verification for `a3b9be9`:

- `cargo test big_fish --all-targets` -> `7 passed`
- `cargo test addict --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1385 passed`

`298b8b1` summary:

- Continued the Java event `ShowCardAndObtainEffect` sweep into the remaining
  match/transform event paths that already had business logic but lacked focused
  obtain-hook ordering locks.
- Java checked:
  - `D:\rust\cardcrawl\events\exordium\LivingWall.java`
  - `D:\rust\cardcrawl\events\shrines\GremlinMatchGame.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `LivingWall` Change removes the selected card, calls
    `AbstractDungeon.transformCard`, then queues `ShowCardAndObtainEffect` for
    the transformed replacement; the queued effect later runs relic
    `onObtainCard` before the replacement is fully obtained.
  - `GremlinMatchGame` creates board pairs after preview-obtain hooks, matches by
    `cardID`, and queues `ShowCardAndObtainEffect(chosenCard.makeCopy())`; that
    delayed effect later runs `onObtainCard` before `souls.obtain`.
- Rust result:
  - No business logic change was needed.
  - Added `CeramicFish` ordering regressions:
    - `LivingWall` Change emits the obtain-hook gold before the Rust
      `CardTransformed` record for the replacement.
    - `Match and Keep` emits the obtain-hook gold before the `CardObtained`
      record for the matched copy.

Verification for `298b8b1`:

- `cargo test living_wall --all-targets` -> `6 passed`
- `cargo test match_and_keep --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1383 passed`

`3426913` summary:

- Continued the Java event `ShowCardAndObtainEffect` sweep into branches that
  pay damage or mutate the master deck before delayed permanent card obtains.
- Java checked:
  - `D:\rust\cardcrawl\events\city\Nest.java`
  - `D:\rust\cardcrawl\events\city\Vampires.java`
- Java result:
  - `Nest` Join Cult applies `DamageInfo(null, 6)` immediately, then queues a
    delayed Ritual Dagger `ShowCardAndObtainEffect`.
  - `Vampires` Accept decreases max HP, removes all starter Strikes by scanning
    the master deck from the end toward the front, then queues five delayed
    Bite `ShowCardAndObtainEffect`s.
- Rust result:
  - No business logic change was needed in these event paths.
  - Added `CeramicFish` ordering regressions proving the immediate damage/max
    HP/deck removal effects happen before delayed card obtain hooks and
    `CardObtained` events.

Verification for `3426913`:

- `cargo test nest --all-targets` -> `4 passed`
- `cargo test vampires --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1381 passed`

`525fe0b` summary:

- Continued the remaining Java event `ShowCardAndObtainEffect` sweep into
  event branches that pay HP/max HP costs before delayed card obtains.
- Java checked:
  - `D:\rust\cardcrawl\events\city\Ghosts.java`
  - `D:\rust\cardcrawl\events\city\KnowingSkull.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `Ghosts` decreases max HP immediately, then queues 5 Apparition
    `ShowCardAndObtainEffect`s at A0 or 3 at A15+. Each delayed effect later
    runs `onObtainCard` before `souls.obtain`.
  - `KnowingSkull` card reward applies HP_LOSS damage and increments the card
    cost immediately, then queues a colorless card `ShowCardAndObtainEffect`
    whose obtain hooks resolve later.
- Rust result:
  - No business logic change was needed in these event paths.
  - Added `CeramicFish` ordering regressions proving the HP/max HP cost events
    happen before delayed card obtain hooks and `CardObtained` events.

Verification for `525fe0b`:

- `cargo test ghosts --all-targets` -> `3 passed`
- `cargo test knowing_skull --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1379 passed`

`56bec7f` summary:

- Finished the current named `ShowCardAndObtainEffect` event-order candidates
  from the handoff queue by locking the remaining `Mushrooms` and
  `GremlinWheelGame` delayed obtain hook behavior.
- Java checked:
  - `D:\rust\cardcrawl\events\exordium\Mushrooms.java`
  - `D:\rust\cardcrawl\events\shrines\GremlinWheelGame.java`
  - Previously in the same lane:
    `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
    and `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `Mushrooms` Eat heals immediately, then the delayed Parasite
    `ShowCardAndObtainEffect` later runs obtain hooks and adds the card.
  - `GremlinWheelGame` curse result obtains Decay through
    `ShowCardAndObtainEffect`; that effect runs `onObtainCard` before
    `souls.obtain` when it resolves.
- Rust result:
  - No business logic change was needed in either event path.
  - Added `CeramicFish` ordering regressions proving the delayed card effects
    run obtain hooks before `CardObtained`, and that immediate heal remains
    before the delayed Parasite obtain in `Mushrooms`.

Verification for `56bec7f`:

- `cargo test mushrooms --all-targets` -> `7 passed`
- `cargo test gremlin_wheel --all-targets` -> `12 passed`
- `cargo test --all-targets` -> `1377 passed`

`81789e4` summary:

- Continued the Java-source-backed delayed `ShowCardAndObtainEffect` audit for
  event branches where the Rust behavior was already aligned but not locked by
  focused ordering tests.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\GoldShrine.java`
  - `D:\rust\cardcrawl\events\exordium\Sssserpent.java`
  - `D:\rust\cardcrawl\events\beyond\WindingHalls.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
  - `D:\rust\cardcrawl\vfx\RainingGoldEffect.java`
  - `D:\rust\cardcrawl\relics\CeramicFish.java`
  - `D:\rust\cardcrawl\relics\BloodyIdol.java`
  - `D:\rust\cardcrawl\characters\AbstractPlayer.java`
- Java result:
  - `ShowCardAndObtainEffect` performs curse/Omamori interception in the
    constructor but performs relic `onObtainCard`, `souls.obtain`, and
    `onMasterDeckChange` only when the effect later updates.
  - `GoldShrine` Desecrate gains 275 gold immediately, then the delayed Regret
    effect later runs obtain hooks and adds the card.
  - `Sssserpent` confirm constructs the Doubt obtain effect before adding
    `RainingGoldEffect`, but `player.gainGold` is immediate and the actual
    Doubt obtain happens later.
  - `WindingHalls` Embrace deals damage immediately, then the two delayed
    Madness effects later resolve one by one; each effect runs `onObtainCard`
    before `souls.obtain`.
- Rust result:
  - No business logic change was needed in these three event paths.
  - Added regressions using `CeramicFish` to prove event gold/damage happens
    before delayed card obtain hooks and `CardObtained` events.
  - Added `GoldenShrine` tests for A15 pray gold and Omamori blocking Regret
    without blocking the immediate 275 gold.

Verification for `81789e4`:

- `cargo test golden_shrine --all-targets` -> `3 passed`
- `cargo test sssserpent --all-targets` -> `4 passed`
- `cargo test winding_halls --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1375 passed`

`aadd74e` summary:

- Continued the Java-source-backed event obtain/transform order audit through
  `ForgottenAltar`, `GoldenIdolEvent`, `Transmogrifier`, `Designer`, and
  `DrugDealer`.
- Java checked:
  - `D:\rust\cardcrawl\events\city\ForgottenAltar.java`
  - `D:\rust\cardcrawl\events\exordium\GoldenIdolEvent.java`
  - `D:\rust\cardcrawl\events\shrines\Transmogrifier.java`
  - `D:\rust\cardcrawl\events\shrines\Designer.java`
  - `D:\rust\cardcrawl\events\city\DrugDealer.java`
  - `D:\rust\cardcrawl\dungeons\AbstractDungeon.java`
- Java result:
  - `ForgottenAltar` needed no Rust change: `increaseMaxHp(5, false)` still
    heals, the hp loss amount is computed before max HP gain, and Decay has no
    same-branch immediate relic mutation. Ownerless event damage still skips
    Torii and can be reduced by Tungsten Rod, matching current helper behavior.
  - `GoldenIdolEvent` needed no Rust change: trap damage / max HP loss are
    based on constructor-time event values, and Injury has no same-branch
    immediate relic mutation.
  - `AbstractDungeon.update()` moves `effectsQueue` into `effectList` only
    after the current update pass, so `effectsQueue.add(new
    ShowCardAndObtainEffect(...))` does not immediately obtain the card.
  - For multi-card transforms in `Designer` and `DrugDealer`, Java removes and
    transforms each selected card first, queuing `ShowCardAndObtainEffect`;
    the actual replacement obtains resolve later after all selected cards have
    already been removed.
- Rust result:
  - Added `RunState::transform_card_uuids_deferred_obtain_with_source`, which
    preserves Java's per-card remove/transform RNG order while deferring the
    actual replacement obtain hooks until after all selected cards are removed.
  - The generic run pending `Transform` / `TransformNonBottled` path now uses
    the deferred-obtain helper for multi-card non-Neow transforms.
  - The existing Neow transform-two special case remains separate, because
    Java Neow transform-two removes both cards before creating the effects.
  - Corrected the `DrugDealer` regression expectation from
    remove/obtain/remove/obtain to remove/remove/obtain/obtain.

Verification for `aadd74e`:

- `cargo test drug_dealer --all-targets` -> `7 passed`
- `cargo test designer --all-targets` -> `18 passed`
- `cargo test transmogrifier --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1370 passed`

`84ad08f` summary:

- Continued the Java-source-backed permanent deck mutation audit through
  `MindBloom`, `ShiningLight`, and `AccursedBlacksmith`.
- Java checked:
  - `D:\rust\cardcrawl\events\beyond\MindBloom.java`
  - `D:\rust\cardcrawl\events\exordium\ShiningLight.java`
  - `D:\rust\cardcrawl\events\shrines\AccursedBlacksmith.java`
  - `D:\rust\cardcrawl\cards\AbstractCard.java`
  - `D:\rust\cardcrawl\rooms\AbstractRoom.java`
- Java result:
  - Permanent/event card upgrades call each card's `upgrade()` method, which
    mutates the concrete card instance. For ordinary damage/block upgrades,
    Java `upgradeDamage` / `upgradeBlock` add to the current concrete
    `baseDamage` / `baseBlock`, not just to a separate upgrade counter.
  - `AccursedBlacksmith` Rummage constructs
    `ShowCardAndObtainEffect(Pain)` before calling
    `spawnRelicAndObtain(WarpedTongs)`. Omamori interception is based on the
    pre-relic snapshot at effect construction time, while the actual card
    obtain resolves after the relic has already been obtained.
- Rust result:
  - `RunState::upgrade_card_with_source` now calls
    `content::cards::upgrade_card_once_java` instead of directly incrementing
    `upgrades`, so master-deck upgrades preserve Java concrete-card mutation
    semantics.
  - `AccursedBlacksmith` Rummage now obtains Warped Tongs before resolving
    Pain obtain, while still using the pre-relic Omamori snapshot for the Pain
    constructor-time block check.
  - Added regressions for concrete `base_damage_override` upgrade behavior and
    AccursedBlacksmith relic-before-card event ordering.

Verification for `84ad08f`:

- `cargo test master_deck_upgrade_uses_java_card_upgrade_helper --all-targets`
  -> `1 passed`
- `cargo test accursed_blacksmith --all-targets` -> `6 passed`
- `cargo test shining_light --all-targets` -> `4 passed`
- `cargo test mind_bloom --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1370 passed`

`7529c30` summary:

- Continued the shrine event audit into Bonfire / Bonfire Elementals.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\Bonfire.java`
- Java result:
  - When a selected card returns from the grid, Java stores it in
    `offeredCard`.
  - Java then calls `setReward(offeredCard.rarity)` before calling
    `player.masterDeck.removeCard(offeredCard)`.
  - Therefore rarity rewards such as Spirit Poop, healing, or max HP gain
    happen before ordinary master-deck removal hooks such as Parasite's max HP
    loss.
- Rust result:
  - The shared pending deck-purge path now special-cases
    `BonfireElementals` and `BonfireSpirits` to apply the rarity reward before
    calling ordinary hook-preserving `remove_card_from_deck_with_source`.
  - Both Bonfire event modules expose an `apply_offer_reward` helper used by
    direct screen-2 tests and by pending-choice resolution.
  - Added a Parasite regression proving Spirit Poop obtain precedes the
    offered curse's `CardRemoved` and `MaxHpChanged(-3)` events.

Verification for `7529c30`:

- `cargo test bonfire --all-targets` -> `14 passed`
- `cargo test --all-targets` -> `1369 passed`

`6352ba4` summary:

- Continued the shrine event audit into We Meet Again.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\WeMeetAgain.java`
  - `D:\rust\cardcrawl\characters\AbstractPlayer.java`
- Java result:
  - Constructor order is random potion, gold amount, random non-basic card.
  - `getRandomPotion()` shuffles actual potion objects with
    `miscRng.randomLong()` and keeps an object reference.
  - `getRandomNonBasicCard()` shuffles eligible master-deck cards with
    `miscRng.randomLong()` and keeps an `AbstractCard` object reference.
  - Giving a card removes that stored card through ordinary
    `player.masterDeck.removeCard(cardOption)`, then obtains a random
    screenless relic.
- Rust result:
  - `WeMeetAgain` no longer stores `cardOption` as an 8-bit deck index in
    `internal_state`.
  - The event now stores the selected card uuid in `EventState.extra_data`,
    leaving `internal_state` for gold amount and potion slot.
  - Live event semantics round-trip now preserves that uuid, and a regression
    covers a card past deck index 255 so this cannot silently truncate again.

Verification for `6352ba4`:

- `cargo test we_meet_again --all-targets` -> `11 passed`
- `cargo test --all-targets` -> `1368 passed`

`8e4b627` summary:

- Continued the event/master-deck mutation audit into Golden Wing.
- Java checked:
  - `D:\rust\cardcrawl\events\exordium\GoldenWing.java`
  - `D:\rust\cardcrawl\helpers\CardHelper.java`
- Java result:
  - The remove-card path first damages the player and changes the event to
    the `PURGE` screen.
  - Only the next button press opens
    `CardGroup.getGroupWithoutBottledCards(player.masterDeck.getPurgeableCards())`.
  - The selected card is then removed through ordinary
    `player.masterDeck.removeCard(c)`.
  - The attack option's availability uses `CardHelper.hasCardWithXDamage(10)`,
    which scans all master-deck attack cards and checks their `baseDamage`.
- Rust result:
  - Golden Wing no longer collapses damage and deck selection into one action.
  - `current_screen = 1` now represents Java's `PURGE` screen and exposes a
    proceed action; that second action opens the non-bottled purge selection.
  - Existing removal still uses ordinary hook-preserving
    `remove_card_from_deck_with_source`.

Verification for `8e4b627`:

- `cargo test golden_wing --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1367 passed`

`a75011b` summary:

- Continued the permanent master-deck mutation audit into ordinary obtain
  ordering.
- Java checked:
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
  - `D:\rust\cardcrawl\relics\DarkstonePeriapt.java`
  - `D:\rust\cardcrawl\relics\CeramicFish.java`
- Java result:
  - `ShowCardAndObtainEffect.update()` iterates player relics and calls
    `r.onObtainCard(card)`.
  - Only after those obtain hooks does Java call
    `AbstractDungeon.getCurrRoom().souls.obtain(card, true)`.
  - Java then iterates relics again for `r.onMasterDeckChange()`.
- Rust result:
  - `RunState` now resolves `DeckManager` obtain actions before emitting
    `CardObtained` / `CardTransformed` and pushing obtained cards into
    `master_deck`.
  - This applies to ordinary obtain, manual no-interception obtain,
    stat-equivalent copy obtain, and transformed-card obtain.
  - Added a `RunState` regression proving Darkstone's `MaxHpChanged` event
    precedes the `CardObtained` event for an obtained curse.

Verification for `a75011b`:

- `cargo test ordinary_obtain_runs_relic_obtain_hooks_before_master_deck_add_like_java --all-targets`
  -> `1 passed`
- `cargo test darkstone_periapt --all-targets` -> `1 passed`
- `cargo test ceramic_fish --all-targets` -> `1 passed`
- `cargo test note_for_yourself --all-targets` -> `7 passed`
- `cargo test transform_two --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1367 passed`

`5e0aff5` summary:

- Continued the permanent master-deck mutation audit into the Vampires event.
- Java checked:
  - `D:\rust\cardcrawl\events\city\Vampires.java`
- Java result:
  - `replaceAttacks()` iterates `player.masterDeck.group` from
    `size() - 1` down to `0`.
  - Every card tagged `STARTER_STRIKE` is removed through ordinary
    `masterDeck.removeCard(card)`.
  - Only after all starter strikes are removed does Java queue five
    `ShowCardAndObtainEffect(new Bite())` effects.
- Rust result:
  - `replace_attacks()` now collects starter-strike UUIDs in reverse
    master-deck order before calling the ordinary removal path.
  - Added a regression with mixed starter strikes and non-strikes; expected
    `CardRemoved` UUID order is `104, 103, 101`.

Verification for `5e0aff5`:

- `cargo test vampires --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1366 passed`

`f775832` summary:

- Added a source-specific regression for Drug Dealer transform order.
- Java checked:
  - `D:\rust\cardcrawl\events\city\DrugDealer.java`
- Java result:
  - Drug Dealer's Test Subject path iterates `gridSelectScreen.selectedCards`
    and for each card performs `masterDeck.removeCard(card)`,
    `AbstractDungeon.transformCard(card, false, miscRng)`, then queues the
    replacement `ShowCardAndObtainEffect` before moving to the next selected
    card.
  - This is deliberately different from Neow `TRANSFORM_TWO_CARDS`, which
    removes both selected old cards before creating replacements.
- Rust result:
  - No business logic change; added a two-Parasite event-order regression so
    future refactors cannot accidentally route Drug Dealer through Neow's
    batch-removal transform path.

Verification for `f775832`:

- `cargo test drug_dealer --all-targets` -> `7 passed`

`f6abf75` summary:

- Continued the permanent master-deck transform audit into Neow transform
  rewards.
- Java checked:
  - `D:\rust\cardcrawl\neow\NeowReward.java`
  - `D:\rust\cardcrawl\dungeons\AbstractDungeon.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
  - Nearby transform references in `DrugDealer`, `Designer`,
    `Transmogrifier`, `LivingWall`, and `Astrolabe`.
- Java result:
  - `TRANSFORM_CARD` computes the transformed card with `NeowEvent.rng`,
    then removes the selected card through `masterDeck.removeCard`, then
    queues `ShowCardAndObtainEffect`.
  - `TRANSFORM_TWO_CARDS` is special: it removes both selected cards through
    `masterDeck.removeCard` first, then transforms/obtains replacement 1 and
    replacement 2 with `NeowEvent.rng`.
  - Other checked multi-transform paths are not the same batch-removal shape:
    Drug Dealer, Designer, and Astrolabe remove/transform/obtain each selected
    card sequentially.
- Rust result:
  - `RunState` now separates transform removal, transform-result generation,
    and transformed-card obtain.
  - Ordinary transform selection still processes selected cards sequentially.
  - Neow multi-transform selection now uses a dedicated
    `transform_card_uuids_after_removing_all_with_source` path so all selected
    cards run their Java `onRemoveFromMasterDeck` hooks and deck-change refresh
    before any replacement is obtained.
  - Added a Neow regression using two Parasites: both `MaxHpChanged(-3)`
    events must occur before the first `CardTransformed` event.

Verification for `f6abf75`:

- `cargo test transform_two --all-targets` -> `2 passed`
- `cargo test neow --all-targets` -> `15 passed`
- `cargo test drug_dealer --all-targets` -> `6 passed`
- `cargo test designer --all-targets` -> `18 passed`
- `cargo test astrolabe --all-targets` -> `1 passed`
- `cargo test transmogrifier --all-targets` -> `2 passed`
- `cargo test living_wall --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1364 passed`

`7dd8cf4` summary:

- Continued the permanent master-deck removal audit into Java campfire toke /
  Peace Pipe selection.
- Java checked:
  - `D:\rust\cardcrawl\vfx\campfire\CampfireTokeEffect.java`
  - `D:\rust\cardcrawl\ui\campfire\TokeOption.java`
  - `D:\rust\cardcrawl\relics\PeacePipe.java`
- Java result:
  - Peace Pipe option is usable only when
    `CardGroup.getGroupWithoutBottledCards(player.masterDeck.getPurgeableCards())`
    is non-empty.
  - Campfire toke opens the same non-bottled purgeable card list.
  - The chosen card is removed through ordinary `masterDeck.removeCard`, so
    card removal hooks and master-deck-change hooks should run.
- Rust result:
  - Campfire toke availability now uses the Java non-bottled purgeable gate.
  - Direct campfire toke execution rejects unpurgeable and bottled indices.
  - Full-run legal campfire actions expose only non-bottled purgeable cards.

Verification for `7dd8cf4`:

- `cargo test toke --all-targets` -> `3 passed`
- `cargo test campfire --all-targets` -> `8 passed`

`08bcd42` summary:

- Continued the permanent master-deck removal audit into Fountain of Curse
  Removal.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\FountainOfCurseRemoval.java`
  - `D:\rust\cardcrawl\cards\curses\Parasite.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
- Java result:
  - Fountain iterates the master deck from `size() - 1` down to `0`, removing
    removable curses through ordinary `masterDeck.removeCard`.
  - Therefore removable curses are removed in reverse master-deck order and
    `onRemoveFromMasterDeck` hooks run.
- Rust result:
  - Fountain now removes removable curses in reverse master-deck order.
  - Regression covers removable curses, bottled curses, and unpurgeable special
    curses. It verifies `Parasite` max HP loss and `CardRemoved` order:
    `Doubt`, `Parasite`, `Injury`.

Verification for `08bcd42`:

- `cargo test fountain --all-targets` -> `2 passed`

Next narrow packet:

- Continue the Java-source-backed audit from permanent master-deck mutation
  into the remaining transform/remove edges:
  - `Cleric`, `EmptyCage`, and any remaining event/relic removal paths not yet
    checked in this packet.
  - Transform paths where Java order differs by source:
    `Designer`, `DrugDealer`, `Transmogrifier`, `LivingWall`, `Astrolabe`, and
    Neow are now source-compared at least once; future work should add
    source-specific event-order tests if a new relic/card hook makes the
    ordering observable.
- Keep the current distinction:
  - ordinary `CardGroup.removeCard(...)` paths run card removal hooks and
    master-deck-change hooks;
  - direct `masterDeck.group.remove(...)` / iterator removal paths bypass those
    hooks;
  - Neow `TRANSFORM_TWO_CARDS` removes all selected old cards before obtaining
    replacements, unlike the other checked multi-transform sources.

`d4155a0` summary:

- Continued the permanent master-deck removal audit into Java shop purge
  selection.
- Java checked:
  - `D:\rust\cardcrawl\screens\shop\ShopScreen.java`
- Java result:
  - `ShopScreen.purchasePurge()` opens
    `CardGroup.getGroupWithoutBottledCards(AbstractDungeon.player.masterDeck.getPurgeableCards())`.
  - Therefore the shop purge picker excludes unpurgeable cards such as
    Ascender's Bane and excludes cards attached to bottle relics.
  - The selected card is removed from `player.masterDeck` through the ordinary
    master-deck removal path, so removal hooks and master-deck-change hooks
    should run.
- Rust result:
  - `legal_shop_actions()` now exposes `ClientInput::PurgeCard(idx)` only for
    cards that are both purgeable and not bottled.
  - `shop_handler` now rejects unpurgeable or bottled purge indices even if a
    caller sends them directly.
  - Shop purge now calls
    `remove_card_from_deck_with_source(uuid, DomainEventSource::Shop)` instead
    of the generic deck-mutation source.
  - Added action-mask and handler regressions using Strike, Ascender's Bane,
    and a Bottled Flame-attached Defend.

Verification for `d4155a0`:

- `cargo test legal_shop_purge_actions_use_java_non_bottled_purgeable_cards --all-targets`
  -> `1 passed`
- `cargo test shop_purge_uses_java_non_bottled_purgeable_cards_and_shop_source --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1360 passed`

Next narrow packet:

- Continue permanent master-deck mutation audit by checking event/relic remove
  and transform paths against Java's two explicit families:
  - `CardGroup.removeCard(...)`: should run card removal hooks, then
    master-deck-change relic hooks.
  - Direct `masterDeck.group.remove(...)` / iterator remove: should bypass
    those hooks.
- Good next Java/Rust comparison targets:
  - `FountainOfCurseRemoval`, `Cleric`, `CampfireTokeEffect`, `EmptyCage`,
    `Falling`, `WeMeetAgain`, `Bonfire`, `GoldenWing`.
  - Transform paths in `LivingWall`, `DrugDealer`, `Designer`,
    `Transmogrifier`, `Astrolabe`, and Neow transform rewards.
  - For each, verify both the execution handler and any full-run action mask,
    because shop purge proved mask and execution can drift together.

`d8c5796` summary:

- Continued the master-deck removal audit into Java paths that bypass
  `CardGroup.removeCard()`.
- Java checked:
  - `D:\rust\cardcrawl\relics\PandorasBox.java`
  - `D:\rust\cardcrawl\screens\select\GridCardSelectScreen.java`
  - `D:\rust\cardcrawl\vfx\FastCardObtainEffect.java`
  - `D:\rust\cardcrawl\relics\Necronomicon.java`
- Java result:
  - Pandora's Box removes starter Strike/Defend cards by directly iterating
    `player.masterDeck.group` and calling iterator `remove()`, so removal
    does not call `AbstractCard.onRemoveFromMasterDeck()` or relic
    `onMasterDeckChange()`.
  - Pandora's replacement cards later enter through
    `FastCardObtainEffect`, which does run Omamori interception and ordinary
    obtain hooks; Rust should keep using the ordinary obtain path for the
    generated replacements.
  - Necronomicon `onUnequip()` directly calls
    `player.masterDeck.group.remove(cardToRemove)`, so it removes one
    Necronomicurse without triggering Necronomicurse regeneration and without
    firing master-deck-change relic hooks.
- Rust result:
  - Added `RunState::remove_card_from_deck_without_removal_hooks_with_source`
    for Java direct `masterDeck.group` removals.
  - Normal `remove_card_from_deck_with_source` still emits CardRemoved, then
    runs `onRemoveFromMasterDeck` hooks and deck-change refresh.
  - Pandora's Box now uses the direct-removal helper for removed starters.
  - Necronomicon `on_unequip` now uses the direct-removal helper and no longer
    refreshes Du-Vu Doll after removing its curse.
  - Strengthened Necronomicon unequip regression with a stale Du-Vu Doll
    counter assertion to lock the direct `group.remove` behavior.

Verification for `d8c5796`:

- `cargo test necronomicon_on_unequip_removes_one_necronomicurse_without_regenerating_it --all-targets`
  -> `1 passed`
- `cargo test pandoras_box_replaces_only_starter_strike_defend_with_relic_source --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1358 passed`

Next narrow packet:

- Continue permanent master-deck mutation audit by checking transform and
  remove callers against Java's two explicit families:
  - `CardGroup.removeCard(...)`: should run card removal hooks, then
    master-deck-change relic hooks.
  - Direct `masterDeck.group.remove(...)` / iterator remove: should bypass
    those hooks.
- Good next Java/Rust comparison targets:
  - `FountainOfCurseRemoval`, `Cleric`, `ShopScreen`, `CampfireTokeEffect`,
    `EmptyCage`, `Falling`, `WeMeetAgain`, `Bonfire`, `GoldenWing`.
  - Transform paths in `LivingWall`, `DrugDealer`, `Designer`,
    `Transmogrifier`, `Astrolabe`, and Neow transform rewards.
  - For each, verify Rust uses the hook-preserving or direct-removal helper
    that matches the Java source, not a generic "remove card" assumption.

`efbf00f` summary:

- Continued permanent master-deck removal audit into Java
  `onRemoveFromMasterDeck()` hooks.
- Java checked:
  - `D:\rust\cardcrawl\cards\curses\Parasite.java`
  - `D:\rust\cardcrawl\cards\curses\Necronomicurse.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
  - `D:\rust\cardcrawl\vfx\NecronomicurseEffect.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndObtainEffect.java`
  - `D:\rust\cardcrawl\cards\Soul.java`
- Java result:
  - Only Parasite and Necronomicurse override
    `AbstractCard.onRemoveFromMasterDeck()` under normal card sources.
  - `CardGroup.removeCard(c)` removes the card, calls
    `c.onRemoveFromMasterDeck()`, then calls relic
    `onMasterDeckChange()`.
  - Parasite directly decreases max HP by 3.
  - Necronomicurse starts `NecronomicurseEffect`, which directly inserts a
    fresh Necronomicurse into `player.masterDeck`; it does not use ordinary
    `ShowCardAndObtainEffect` / Soul obtain interception.
  - Therefore Omamori, Darkstone Periapt, Ceramic Fish, and Egg obtain hooks
    must not affect the Necronomicurse self re-add.
- Rust result:
  - `DeckAction::TriggerObtainCard` was renamed to
    `ReaddCardToMasterDeck` to make the non-obtain semantics explicit.
  - `RunState::remove_card_from_deck_with_source()` now resolves card removal
    hooks before refreshing master-deck-change relic state, matching Java's
    remove-card order.
  - Necronomicurse removal now directly re-adds one fresh master-deck card
    without ordinary obtain hooks or Omamori interception.
  - `RunState::next_card_uuid()` now uses the current maximum master-deck UUID
    instead of deck length, preventing remove-then-obtain paths from colliding
    with existing obtained card instances.
  - Added focused regressions for Parasite max HP loss and Necronomicurse
    direct self re-add in the presence of Omamori, Darkstone Periapt, and
    Ceramic Fish.

Verification for `efbf00f`:

- `cargo test removing_parasite_runs_master_deck_removal_hook_before_deck_change_refresh --all-targets`
  -> `1 passed`
- `cargo test removing_necronomicurse_readds_directly_without_ordinary_obtain_hooks --all-targets`
  -> `1 passed`
- `cargo test necronomicurse --all-targets` -> `3 passed`
- `cargo test parasite --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1358 passed`

Next narrow packet:

- Continue Java-source audit around permanent master-deck mutation/removal
  paths:
  - Transform and mass-removal flows that may intentionally bypass
    `onRemoveFromMasterDeck()` (`PandorasBox`, `Astrolabe`, event transforms,
    direct relic unequip removal) versus purge/remove flows that should call
    the hook.
  - Events that display stat-equivalent previews but mutate selected master
    cards directly (`Designer`, `UpgradeShrine`, `ShiningLight`,
    `BackToBasics`, `MindBloom`) should be checked for whether Rust mutates
    the real card and not a preview artifact.
  - Keep an eye on UUID/ref stability for every remove-then-add path.

`d3c080e` summary:

- Continued card instance copy audit into permanent master-deck duplication
  paths.
- Java checked:
  - `D:\rust\cardcrawl\events\shrines\Duplicator.java`
  - `D:\rust\cardcrawl\relics\DollysMirror.java`
  - `D:\rust\cardcrawl\cards\AbstractCard.java`
- Java result:
  - Both Duplicator and Dolly's Mirror use selected-card
    `makeStatEquivalentCopy()`, then clear bottle flags before
    `ShowCardAndObtainEffect`.
  - `makeStatEquivalentCopy()` copies permanent card state such as upgrades,
    base stat mutations, `misc`, cost / cost-for-turn flags, and
    `freeToPlayOnce`.
  - It does not copy transient rendered damage/block/magic or multi-damage
    arrays.
- Rust result:
  - `RunState::add_card_instance_copy_to_deck_from()` no longer copies
    transient rendered `base_*_mut` fields into master-deck duplicates.
  - It now preserves `cost_for_turn` and `free_to_play_once`, matching the
    Java stat-equivalent copy payload represented by current Rust card state.
  - Strengthened both Dolly's Mirror and Duplicator tests with permanent vs
    transient field assertions. Bottle attachment remains intentionally tracked
    by relic UUID, so the copied card naturally lacks the original bottle link.

Verification for `d3c080e`:

- `cargo test duplicate_selection_preserves_stat_equivalent_card_state_without_copying_bottle_attachment --all-targets`
  -> `1 passed`
- `cargo test duplicate_selection_obtains_stat_equivalent_copy_with_event_source --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1356 passed`

Next narrow packet:

- Continue Java-source audit around permanent card mutation/removal paths:
  - Events that display stat-equivalent previews but mutate selected master
    cards directly (`Designer`, `UpgradeShrine`, `ShiningLight`,
    `BackToBasics`, `MindBloom`) should be checked for whether Rust copies
    only preview state or accidentally mutates preview artifacts.
  - Removal paths should keep watching `onRemoveFromMasterDeck()` hooks,
    especially `Parasite` max HP loss and `Necronomicurse` re-obtain behavior.
  - If staying with relic audit instead, resume Java stateful relic counters:
    `Dodecahedron`, `HappyFlower`, `IncenseBurner`, `HornCleat`,
    `CaptainsWheel`, `StoneCalendar`, `MercuryHourglass`.

`af79d1b` summary:

- Continued the Java-source generated/source-copy audit into Watcher preview
  card producers and Anger.
- Java checked:
  - `D:\rust\cardcrawl\cards\purple\Alpha.java`
  - `D:\rust\cardcrawl\cards\tempCards\Beta.java`
  - `D:\rust\cardcrawl\cards\purple\Pray.java`
  - `D:\rust\cardcrawl\cards\purple\CarveReality.java`
  - `D:\rust\cardcrawl\cards\purple\DeceiveReality.java`
  - `D:\rust\cardcrawl\cards\purple\ReachHeaven.java`
  - `D:\rust\cardcrawl\cards\purple\Evaluate.java`
  - `D:\rust\cardcrawl\powers\watcher\StudyPower.java`
  - `D:\rust\cardcrawl\cards\red\Anger.java`
- Java result:
  - Alpha / Beta / Pray / Reach Heaven / Evaluate / Study generate fresh
    preview cards into the draw pile; the current Rust by-id draw action is
    mechanically equivalent for these audited cards because the source objects
    carry no extra `misc`, transient cost, UUID, or rendered-stat state.
  - Carve Reality and Deceive Reality already use the constructed hand-card
    boundary for Smite / Safety.
  - Anger is different: Java queues `this.makeStatEquivalentCopy()` into
    `MakeTempCardInDiscardAction`, so the queued source snapshot must preserve
    permanent card state but must not carry rendered `damage/block/magic` or
    multi-damage values.
- Rust result:
  - `Anger` now queues a stat-equivalent source snapshot for its discard copy,
    instead of queueing the evaluated/rendered card clone.
  - Strengthened the Anger runtime test to lock permanent state preservation
    and transient rendered-state reset at queue time.

Verification for `af79d1b`:

- `cargo test ironclad_common_utility_runtime_actions_match_java_use_methods --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1356 passed`

Next narrow packet:

- Continue Java `makeStatEquivalentCopy()` audit with remaining gameplay
  sources:
  - `DivinePunishmentAction` is referenced only by deprecated
    `DEPRECATEDCleanseEvil`; keep out of normal-scope implementation unless
    deprecated cards become in scope.
  - Event/relic/card obtain or upgrade previews that only call
    `ShowCardBrieflyEffect` are UI-only unless they mutate master deck,
    consume gameplay RNG, or affect replay-visible state.
  - Good next non-preview gameplay packet: master-deck duplication/removal
    paths (`Duplicator`, `DollysMirror`, event card options) or relic hook
    stateful counters, depending on whether the next lane stays with card-copy
    state or returns to relic audit.

`8d48e33` summary:

- Continued the Java `makeStatEquivalentCopy()` choice-resolution audit into
  Discovery / Foreign Influence / ChooseOneColorless / Codex.
- Java checked:
  - `D:\rust\cardcrawl\actions\unique\DiscoveryAction.java`
  - `D:\rust\cardcrawl\actions\watcher\ForeignInfluenceAction.java`
  - `D:\rust\cardcrawl\actions\utility\ChooseOneColorless.java`
  - `D:\rust\cardcrawl\actions\unique\CodexAction.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToHandEffect.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToDiscardEffect.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToDrawPileEffect.java`
- Java result:
  - Discovery explicitly applies Master Reality to both generated source
    copies, then `ShowCardAndAddToHandEffect` applies another Master Reality
    upgrade to actual hand cards.
  - Discovery cards that overflow to discard go through
    `ShowCardAndAddToDiscardEffect(src, x, y)`, where the second Master Reality
    upgrade applies only to a visual stat-equivalent copy. The actual inserted
    source card keeps only the explicit upgrade.
  - ChooseOneColorless has the same hand-vs-discard Master Reality split:
    hand gets explicit + hand-effect upgrades; full-hand discard keeps only
    the explicit upgrade.
  - Foreign Influence has no explicit Master Reality upgrade; hand gets one
    effect upgrade, full-hand discard gets none on the inserted source card.
  - Codex uses the draw-pile effect path, so the inserted stat-equivalent copy
    gets one Master Reality upgrade.
- Rust result:
  - Discovery pending-choice resolution now applies one or two Master Reality
    call sites depending on whether each selected copy enters discard or hand.
  - CardReward `Hand` destination now applies two call sites only for actual
    hand insertion and one call site for full-hand discard overflow.
  - Added Searing Blow regressions because ordinary cards hide duplicate
    upgrade calls after the first upgrade.

Verification for `8d48e33`:

- `cargo test discovery --all-targets` -> `7 passed`
- `cargo test foreign_influence --all-targets` -> `4 passed`
- `cargo test card_reward --all-targets` -> `12 passed`
- `cargo test --all-targets` -> `1356 passed`

Next narrow packet:

- Continue the Java-source generated/source-copy audit through remaining
  gameplay-relevant `makeStatEquivalentCopy()` paths that are not pure preview
  UI:
  - Watcher generated draw/hand chain: `Alpha`, `Beta`, `Pray`, `CarveReality`,
    `DeceiveReality`, and `DivinePunishmentAction`.
  - Anger discard copy path.
  - `MakeTempCardAtBottomOfDeckAction` remains daily-mod-only in Java; keep out
    of scope unless daily mods become in scope.
- For each path, check whether Java uses source-card insertion, stat-equivalent
  effect copy insertion, Master Reality explicit/effect upgrades, UUID
  preservation, `misc`, cost, and rendered-stat reset.

`ab78536` summary:

- Continued the source-copy / UUID / misc propagation audit into queued copy
  paths.
- Java checked:
  - `D:\rust\cardcrawl\actions\watcher\OmniscienceAction.java`
  - `D:\rust\cardcrawl\actions\unique\DualWieldAction.java`
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInHandAction.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToHandEffect.java`
- Java result:
  - Omniscience queues the selected original once, then queues
    `playAmt - 1` stat-equivalent copies with `purgeOnUse`.
  - The selected original keeps its current rendered fields; the extra copies
    come from `makeStatEquivalentCopy()`.
  - Dual Wield's selected branch removes the selected original first, then
    creates `amount` hand/discard copies through `MakeTempCardInHandAction`.
  - Copies that enter hand have powers applied after stat-equivalent copying;
    overflow discard copies keep the reset source-card rendered fields.
- Rust result:
  - Existing implementation already matched these paths.
  - Strengthened Omniscience and Dual Wield tests to distinguish permanent
    card state (`upgrades`, `misc`, base-damage override, free-to-play) from
    transient rendered damage.

Verification for `ab78536`:

- `cargo test copy --all-targets` -> `15 passed`
- `cargo test omniscience_selection_removes_draw_card_and_queues_autoplay_copies --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1354 passed`

`23d034d` summary:

- Continued source-copy / UUID / misc propagation audit into Nightmare.
- Java checked:
  - `D:\rust\cardcrawl\actions\unique\NightmareAction.java`
  - `D:\rust\cardcrawl\powers\NightmarePower.java`
  - `D:\rust\cardcrawl\cards\AbstractCard.java`
- Java result:
  - `NightmareAction` applies a `NightmarePower` carrying the selected card.
  - `NightmarePower` stores `copyMe.makeStatEquivalentCopy()` and immediately
    calls `resetAttributes()`.
  - `AbstractCard.triggerWhenCopied()` is empty in the Java base class, and no
    current card override was found in the decompiled card sources.
- Rust result:
  - Existing implementation already matched the Nightmare payload flow.
  - Strengthened the Nightmare test to prove permanent stat-equivalent fields
    are preserved while `costForTurn` and rendered damage are reset.

Verification for `23d034d`:

- `cargo test nightmare_matches_java_card_and_power_payload_flow --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1354 passed`

Next narrow packet:

- Continue the generated/source-copy audit through remaining Java
  `makeStatEquivalentCopy()` call sites:
  - Discovery / Foreign Influence / ChooseOneColorless / Codex choice
    resolution.
  - Omniscience and Dual Wield are now locked more tightly.
  - `MakeTempCardAtBottomOfDeckAction` is only used by the Controlled Chaos
    daily mod in Java; do not implement unless daily mods become in scope.
- Keep an eye on bottle flags: Java `makeStatEquivalentCopy()` copies
  `inBottle*` flags, while Rust represents bottle ownership at run/relic UUID
  level. This is already intentionally different for master-deck duplication,
  but should be revisited if combat-generated bottled flags ever affect
  mechanics.

`773cc7a` summary:

- Finished the immediate Stasis / discard+deck follow-up from the discard
  generated-card audit without changing business code.
- Java checked again:
  - `D:\rust\cardcrawl\powers\StasisPower.java`
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInHandAction.java`
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInDiscardAction.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToHandEffect.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToDiscardEffect.java`
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInDiscardAndDeckAction.java`
- Java result:
  - `StasisPower.onDeath()` chooses hand vs discard based on hand size at
    death time.
  - If Stasis queues the hand action, `MakeTempCardInHandAction` can still
    overflow to discard at execution time if the hand is full by then.
  - In that overflow case, the same-UUID source card keeps the constructor-time
    Master Reality upgrade, while the discard visual copy upgrade does not
    affect the inserted source card.
  - Direct full-hand Stasis discard uses `MakeTempCardInDiscardAction(card,
    true)`, whose same-UUID constructor deliberately skips Master Reality.
  - `MakeTempCardInDiscardAndDeckAction` creates separate stat-equivalent draw
    and discard copies. Each destination effect applies one Master Reality
    upgrade to its inserted non-status/non-curse card.
- Rust result:
  - Existing implementation already matched these paths.
  - Added regressions for Stasis queued-hand overflow and discard+deck one
    Master Reality upgrade per destination.

Verification for `773cc7a`:

- `cargo test master_reality --all-targets` -> `8 passed`
- `cargo fmt` was run; the two known unrelated rustfmt noise files were
  restored afterwards.
- `cargo test --all-targets` -> `1354 passed`

Next narrow packet:

- Continue generated-card cleanup into source-copy / UUID / misc propagation:
  - Recheck `makeStatEquivalentCopy`, `makeSameInstanceOf`, and `triggerWhenCopied`
    assumptions against Java for remaining hand/draw/discard/exhaust generation.
  - Watch delayed powers that store card snapshots, especially Nightmare,
    Stasis, and generation powers that queue later copies.
  - Keep the Exordium monster helper on the watch list as a separate monster
    status-card path, not a player Master Reality path.

`3f90fe6` summary:

- Continued generated-card cleanup into discard-pile insertion semantics.
- Java checked:
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInDiscardAction.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToDiscardEffect.java`
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInDiscardAndDeckAction.java`
- Java result:
  - `MakeTempCardInDiscardAction(AbstractCard card, int amount)` does not apply
    Master Reality in its constructor.
  - `MakeTempCardInDiscardAction.update()` only creates discard effects inside
    `if (this.numCards < 6)`. In the decompiled Java source there is no large
    amount fallback branch, so `amount >= 6` is a no-op.
  - `ShowCardAndAddToDiscardEffect(AbstractCard card)` applies one Master
    Reality upgrade to the actual card that is inserted into discard.
  - `ShowCardAndAddToDiscardEffect(AbstractCard srcCard, float x, float y)`
    applies Master Reality only to the visual copy and inserts `srcCard`, which
    matches the previously audited hand-overflow source-card path.
  - `MakeTempCardInDiscardAndDeckAction` queues separate draw and discard
    effects from separate stat-equivalent copies.
- Rust result:
  - `handle_make_temp_card_in_discard` and `handle_make_copy_in_discard` now
    return without mutation when `amount >= 6`, matching Java's missing large
    amount fallback.
  - Added a focused regression covering both generated-by-id and copied-card
    discard paths for `amount == 6`.
  - Existing discard+deck handling already uses separate draw/discard copies
    and remains on the watch list for source-copy UUID and Master Reality tests.

Verification for `3f90fe6`:

- `cargo test make_temp_card_in_discard_large_amount_matches_java_no_effect --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1353 passed`

Next narrow packet:

- Finish the discard/discard+deck audit:
  - Confirm Stasis same-UUID discard path behavior against Java.
  - Confirm `MakeTempCardInDiscardAndDeckAction` UUID separation and one Master
    Reality call per destination are adequately locked by tests.
  - If existing coverage is enough, record that and move to remaining generated
    card source-copy / UUID / misc propagation paths.
- Keep the Exordium monster helper on the watch list, but do not fold it into
  player generated-card/Master Reality handling without Java evidence.

`59ce922` summary:

- Continued the generated-card audit into draw-pile insertion semantics.
- Java checked:
  - `D:\rust\cardcrawl\actions\common\MakeTempCardInDrawPileAction.java`
  - `D:\rust\cardcrawl\vfx\cardManip\ShowCardAndAddToDrawPileEffect.java`
  - `D:\rust\cardcrawl\cards\CardGroup.java`
  - Representative producers: Pride, Wild Strike, Mark of Pain,
    Metamorphosis, Chrysalis, Conjure Blade.
- Java result:
  - `MakeTempCardInDrawPileAction` itself does not apply Master Reality in its
    constructor.
  - On update, each action card copy receives one Master Reality upgrade if
    applicable.
  - For `amount < 6`, the `ShowCardAndAddToDrawPileEffect(x, y, ...)`
    constructor copies again and applies a second Master Reality upgrade to the
    actual inserted card.
  - For `amount >= 6`, the short effect constructor inserts the source card
    directly, so the actual inserted card only receives the action-update
    Master Reality upgrade.
  - Java draw-pile top is `CardGroup.group.last`; Rust draw-pile top is index
    0, so all top/bottom/random insertion must stay behind the existing
    helper API.
- Rust result:
  - Existing draw-pile top/bottom/random helper semantics already matched the
    Java `CardGroup` ordering.
  - Existing `handle_make_temp_card_in_draw_pile` / `handle_make_copy_in_draw_pile`
    already modeled `amount < 6` vs `amount >= 6` Master Reality call counts.
  - Added a focused `amount == 6` Searing Blow regression to lock the Java
    source-card effect path.
  - Removed stale Pride comments that still described `MakeTempCardInDrawPile`
    as a temporary stub.

Verification for `59ce922`:

- `cargo test make_temp_card_in_draw_pile_large_amount_uses_java_src_card_path --all-targets`
  -> `1 passed`
- `cargo fmt` was run; the two known unrelated rustfmt noise files were
  restored afterwards.
- `cargo test --all-targets` -> `1352 passed`

Next narrow packet:

- Continue generated-card cleanup by auditing remaining discard-pile and
  discard+deck paths against Java:
  - `MakeTempCardInDiscardAction`
  - `ShowCardAndAddToDiscardEffect`
  - `MakeTempCardInDiscardAndDeckAction`
- Confirm whether existing Rust one-call Master Reality handling for discard
  paths is complete for concrete copies, status/curse exclusions, and
  source-copy UUID behavior.
- Keep the Exordium monster helper on the watch list, but do not fold it into
  player generated-card/Master Reality handling without Java evidence.

`3ec5e96` summary:

- Finished the potion side of the Java `MakeTempCardInHandAction` audit.
- Java checked:
  - `D:\rust\cardcrawl\potions\BottledMiracle.java`
  - `D:\rust\cardcrawl\potions\CunningPotion.java`
  - `D:\rust\cardcrawl\potions\AttackPotion.java`
  - `D:\rust\cardcrawl\potions\SkillPotion.java`
  - `D:\rust\cardcrawl\potions\PowerPotion.java`
  - `D:\rust\cardcrawl\potions\ColorlessPotion.java`
- Java result:
  - Only Bottled Miracle and Cunning Potion directly queue
    `MakeTempCardInHandAction`.
  - Attack / Skill / Power / Colorless potions queue `DiscoveryAction`, so they
    are not part of the static generated-hand-card constructor issue.
- Rust changes:
  - `get_potion_actions` now receives `&CombatState`, not only scalar context.
  - Bottled Miracle and Cunning Potion now use
    `make_constructed_temp_card_in_hand_action(...)`.
  - Cunning Potion preserves Java's explicit upgraded Shiv before the
    `MakeTempCardInHandAction` constructor boundary.
- Added regression coverage for Master Reality:
  - Bottled Miracle constructs upgraded Miracle payloads when Master Reality is
    present.
  - Cunning Potion constructs upgraded Shiv payloads and does not invent extra
    upgrade state.

Verification for `3ec5e96`:

- `cargo test potion --all-targets` -> `68 passed`
- `cargo fmt` was run; the two known unrelated rustfmt noise files were
  restored afterwards.
- `cargo test --all-targets` -> `1351 passed`

Remaining `MakeTempCardInHand` audit surface after `3ec5e96`:

- Central execution arm:
  - `src/engine/action_handlers/mod.rs`
- Monster helper:
  - `src/content/monsters/exordium/mod.rs`
- Tests/comments still mention Java `MakeTempCardInHandAction` where the Java
  source name is intentionally being referenced.
- `rg -n "MakeTempCardInHand" src\content\potions src\engine\action_handlers\cards.rs -g "*.rs"`
  now reports only comments/tests around Java source names and Stasis behavior,
  not potion `Action::MakeTempCardInHand` production.

Next narrow packet at `3ec5e96` was draw-pile generated-card behavior; resolved
in `59ce922`.

`d618731` summary:

- Continued the Java `MakeTempCardInHandAction` constructor/effect audit for
  static hand-card producers.
- Added `make_constructed_temp_card_in_hand_action(card_id, amount, upgraded,
  state)` so static generated cards can still pass through the explicit Java
  construction boundary and receive constructor-time Master Reality.
- Converted state-aware Shiv/Miracle/Smite/Safety/Wound producers from
  `Action::MakeTempCardInHand` to `Action::MakeConstructedCopyInHand`:
  - Blade Dance and Cloak and Dagger.
  - Power Through and Necronomicurse exhaust return.
  - Carve Reality, Deceive Reality, Deus Ex Machina, Battle Hymn, and Collect.
  - Infinite Blades and Blade Fury.
  - Ninja Scroll, Pure Water, and Holy Water battle-start relic hooks.
- Potion generated-card paths were intentionally left for a later packet at
  this commit; they were fixed in `3ec5e96`.
- Kept the Exordium monster status-card helper static for now; it is a
  status-card monster path, not the player generated-card/Master Reality path.

Verification for `d618731`:

- `cargo fmt` was run; the two known unrelated rustfmt noise files were
  restored afterwards:
  - `src/cli/full_run_smoke/observation.rs`
  - `src/content/events/secret_portal.rs`
- `cargo test --all-targets` -> `1350 passed`

Remaining `MakeTempCardInHand` audit surface after `d618731`:

- Central execution arm:
  - `src/engine/action_handlers/mod.rs`
- Monster helper:
  - `src/content/monsters/exordium/mod.rs`
- Tests/comments for the converted paths still mention Java
  `MakeTempCardInHandAction` where the Java source name is intentionally being
  referenced.

Next narrow packet at `d618731` was potion generated-card actions; resolved in
`3ec5e96`.

`7d9e17a` summary:

- Continued the Java `MakeTempCardInHandAction` constructor/effect audit.
- Added `prepare_make_temp_card_in_hand_constructor()` as the explicit Rust
  boundary for concrete generated cards that have reached Java
  `MakeTempCardInHandAction` construction.
- Converted remaining content-level concrete hand-card producers from
  `MakeCopyInHand` to `MakeConstructedCopyInHand`:
  - Infernal Blade / Distraction / Jack of All Trades / Transmutation
    materialized random hand-card actions.
  - Magnetism / Creative AI / Hello World turn-start powers.
  - Nightmare delayed copies.
  - Endless Agony draw trigger.
  - Dual Wield sole-candidate and hand-select copy paths.
- Added regression coverage showing a Magnetism-generated card receives
  constructor-time Master Reality before the queued action executes, and keeps
  that upgrade after Master Reality is removed before execution.
- After this commit, `rg -n "MakeCopyInHand" src\content src\engine -g "*.rs"`
  only reports the central execution arm in `engine/action_handlers/mod.rs`;
  content producers no longer use it directly for Java
  `MakeTempCardInHandAction` semantics.

Verification for `7d9e17a`:

- `cargo test magnetism_make_temp_constructor_master_reality_persists_until_execution --all-targets`
  -> `1 passed`
- Targeted generated-card/card-copy tests passed:
  - `transmutation_x_cost_action_matches_java_energy_and_chemical_x_timing`
  - `magnetism_power_locks_random_colorless_cards_at_turn_start`
  - `jack_of_all_trades_locks_random_colorless_cards_when_used`
  - `creative_ai_and_hello_world_powers_sample_defect_random_pools`
  - `nightmare_selection_returns_original_and_start_turn_copies_payload`
  - `ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`
  - `distraction_matches_java_random_skill_free_for_turn`
  - `hand_select_copy` tests
- `cargo test --all-targets` -> `1350 passed`

Next narrow packet:

- Continue the generated-card audit with static `MakeTempCardInHand { card_id,
  amount, upgraded }` producers such as Shiv/Miracle/Smite/Safety paths. Those
  still encode the Java constructor boundary less explicitly than concrete
  `MakeConstructedCopyInHand` producers.
- Then inspect `MakeTempCardInDrawPileAction` /
  `ShowCardAndAddToDrawPileEffect` constructor/effect Master Reality and
  random/bottom/top insertion behavior.
- After hand/draw/discard generated-card actions are clean, resume relic audit
  around custom actions, execution-time state, and source-sensitive effects.

`24d3c00` summary:

- Java checked:
  - `actions/common/MakeTempCardInHandAction.java`
  - `vfx/cardManip/ShowCardAndAddToHandEffect.java`
  - `vfx/cardManip/ShowCardAndAddToDiscardEffect.java`
  - `DeadBranch`
  - `Enchiridion`
- Fixed concrete generated-card hand actions:
  - Java `MakeTempCardInHandAction` applies Master Reality in its constructor
    before the action is queued.
  - Java `ShowCardAndAddToHandEffect` applies Master Reality again when the
    generated card enters hand, then runs copied-card/hand refresh mechanics.
  - Java hand-full overflow through
    `ShowCardAndAddToDiscardEffect(srcCard, x, y)` upgrades only the visual
    copy at effect time; the actual `srcCard` added to discard has only the
    constructor-time upgrade.
  - Rust added `Action::MakeConstructedCopyInHand` for cards that have already
    passed the Java constructor boundary.
  - `DeadBranch` and `Enchiridion` now apply constructor-time Master Reality
    before queuing the constructed action.
- Regression coverage:
  - hand path receives constructor + hand-effect Master Reality upgrades.
  - delayed execution keeps constructor-time upgrade after Master Reality is
    removed before action execution.
  - full-hand overflow discard receives only the constructor-upgraded actual
    card.
  - Enchiridion queued payload is already constructor-upgraded.

Verification for current dirty work:

- `cargo test enchiridion_applies_make_temp_card_constructor_reality_before_queue_execution --all-targets`
  -> `1 passed`
- `cargo test constructed_make_copy_in_hand_separates_constructor_and_effect_reality_calls --all-targets`
  -> `1 passed`
- `cargo test dead_branch --all-targets` -> `1 passed`
- `cargo test enchiridion --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1349 passed`

Next narrow packet:

- Continue generated-card/source audit for other concrete producers of Java
  `MakeTempCardInHandAction`, especially card/potion/relic paths that hand a
  preselected concrete card into the action rather than sampling inside the
  shared Rust handler.
- Then resume relic audit around custom actions, private execution-time state,
  and source-sensitive effects.

`0e0b90c` summary:

- Java checked:
  - `Toolbox`
  - `ChooseOneColorless`
  - `AbstractDungeon.returnTrulyRandomColorlessCardInCombat()`
  - Existing generated-card relic paths for `NilrysCodex`, `DeadBranch`, and
    `Enchiridion`
- Fixed Toolbox / colorless reward pool:
  - Java `ChooseOneColorless.generateCardChoices()` samples
    `returnTrulyRandomColorlessCardInCombat()`.
  - That method iterates `srcColorlessCardPool` order and filters HEALING
    cards; it does not concatenate uncommon then rare pools.
  - Rust `SuspendForCardReward { pool: Colorless }` now uses the existing
    `random_colorless_in_combat_pool()` helper instead of rebuilding a
    rarity-grouped pool locally.
  - Added an engine-level regression that executes the
    `SuspendForCardReward` action and verifies the three generated choices and
    `cardRandomRng` counter against Java-order colorless combat pool sampling.
- Confirmed existing tests for:
  - `NilrysCodex` execution-time basically-dead guard.
  - `DeadBranch` immediate random card sampling before queuing hand copy.
  - `Enchiridion` immediate random Power sampling and zero-for-turn handling.

Verification for `0e0b90c`:

- `cargo test colorless_card_reward_uses_java_random_colorless_combat_pool_order --all-targets`
  -> `1 passed`
- `cargo test nilrys_codex --all-targets` -> `1 passed`
- `cargo test dead_branch --all-targets` -> `1 passed`
- `cargo test enchiridion --all-targets` -> `1 passed`
- `cargo test toolbox --all-targets` -> no named tests matched, but the new
  engine-level colorless reward test covers Toolbox's shared action path.
- `cargo test --all-targets` -> `1347 passed`

Next narrow packet:

- Continue generated-card/source audit:
  - `DeadBranch` and `Enchiridion` may still deserve a focused check against
    `MakeTempCardInHandAction` hand-full behavior and Master Reality, depending
    on whether the shared `MakeCopyInHand` handler already covers it.
  - Check broad relic/card uses of `returnTrulyRandomColorlessCardInCombat`,
    `returnTrulyRandomCardInCombat`, and `DamageAllEnemiesAction(null, ...)`
    for local pool/source rewrites.

`29e0699` summary:

- Java checked:
  - `Nunchaku`
  - `PenNib`
  - `InkBottle`
  - `Sundial`
  - `UnceasingTop`
- Fixed attack-counter relic edge semantics:
  - Java `Nunchaku.onUseCard()` and `PenNib.onUseCard()` increment the counter
    directly with `++counter`.
  - Rust previously normalized negative counters to 0 before incrementing.
  - Rust now preserves Java's direct increment semantics.
  - Regression tests cover `counter = -1`:
    - Nunchaku goes to 0 and fires because Java uses `counter % 10 == 0`.
    - Pen Nib goes to 0 and does not add Pen Nib power because Java checks
      `counter == 9`.
- Confirmed without code changes:
  - `InkBottle` already uses Java `++counter` / `counter == 10` semantics.
  - `Sundial` already uses Java `++counter` / `counter == 3` semantics.
  - `UnceasingTop` keeps only mechanical state (`canDraw` as `amount`,
    `disabledUntilEndOfTurn` as `used_up`) and omits UI-only screen/pulse
    behavior.

Verification for `29e0699`:

- `cargo test nunchaku --all-targets` -> `1 passed`
- `cargo test pen_nib --all-targets` -> `1 passed`
- `cargo test ink_bottle --all-targets` -> `1 passed`
- `cargo test sundial --all-targets` -> `1 passed`
- `cargo test unceasing_top --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1346 passed`

Next narrow packet:

- Continue relic audit from Java source.
- Good candidates:
  - `NilrysCodex`, `DeadBranch`, `Toolbox`, `Enchiridion`, `Discovery`-like
    generated-card paths if still suspicious.
  - Source-sensitive `NO_SOURCE` interactions if broad search finds more relic
    `DamageAllEnemiesAction(null, ...)` mismatches.
  - Relics with custom anonymous actions or state read at action execution time.

`2007560` summary:

- Java checked:
  - `deprecated/DEPRECATEDDodecahedron`
  - `MercuryHourglass`
  - `LetterOpener`
  - counter/timing packet around `HappyFlower`, `IncenseBurner`,
    `HornCleat`, `CaptainsWheel`, and `StoneCalendar`
- Fixed Dodecahedron timing:
  - Java deprecated Dodecahedron queues an anonymous bottom action from
    `atTurnStart()`.
  - That anonymous action checks whether the player is full HP when it
    executes, then queues `GainEnergyAction(1)`.
  - Rust previously checked full HP immediately in the relic hook.
  - Rust now queues `Action::DodecahedronTurnStartCheck`; the check action
    reads current HP at execution time and queues energy behind current
    pending actions.
- Fixed null source all-enemy relic damage:
  - Java `MercuryHourglass` and `LetterOpener` use
    `DamageAllEnemiesAction(null, ...)`.
  - Rust now emits `source: NO_SOURCE` for both, matching existing
    `StoneCalendar` and `Charon's Ashes` handling.
  - Tests now assert the null-source semantics so source-sensitive effects
    such as owner checks do not silently regress.
- Confirmed without code changes:
  - `HappyFlower`, `IncenseBurner`, `HornCleat`, `CaptainsWheel`, and
    `StoneCalendar` already mutate their counters synchronously like Java.

Verification for `2007560`:

- `cargo test dodecahedron --all-targets` -> `1 passed`
- `cargo test mercury_hourglass --all-targets` -> `1 passed`
- `cargo test letter_opener --all-targets` -> `1 passed`
- `cargo test happy_flower --all-targets` -> `1 passed`
- `cargo test incense_burner --all-targets` -> `1 passed`
- `cargo test horn_cleat --all-targets` -> `1 passed`
- `cargo test captains_wheel --all-targets` -> `1 passed`
- `cargo test stone_calendar --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1346 passed`

Next narrow packet:

- Continue relic source audit from Java, prioritizing relics whose Java code
  queues custom/anonymous actions, uses `DamageAllEnemiesAction(null, ...)`, or
  stores private counters/booleans that Rust may have represented indirectly.
- Good candidates:
  - `LetterOpener` follow-up with related on-use counter relics if needed.
  - `Nunchaku`, `PenNib`, `InkBottle`, `Sundial`, `UnceasingTop`.
  - Source-sensitive relic/power interactions around `NO_SOURCE`.

`d245435` summary:

- Java checked:
  - `AbstractPlayer.playCard()`
  - `AbstractMonster.applyPowers()`, `calculateDamage()`,
    `applyBackAttack()`, and `removeSurroundedPower()`
  - `CardGroup.refreshHandLayout()`
  - `SpireShield` / `SpireSpear` constructors, `usePreBattleAction()`, and
    `die()`
- Fixed Surrounded / BackAttack state modeling:
  - Added `PlayerEntity.facing_left`, the mechanical Rust equivalent of Java
    `AbstractPlayer.flipHorizontal` for Shield/Spear.
  - Added `content::powers::core::surrounded` to synchronize BackAttack markers
    from Surrounded + facing + monster side.
  - From-hand targeted card play now flips the player toward the selected
    target while Surrounded is active, matching Java `playCard()`.
  - Applying/removing Surrounded now synchronizes BackAttack markers.
  - Shield/Spear factory and spawn paths set mechanical left/right positions;
    protocol imports with absolute drawX are handled by type for this pair.
  - Stable combat state keys include `facing_left` so searches do not merge
    opposite Shield/Spear facing states.
- Confirmed:
  - Existing Shield/Spear death cleanup was already attached to `on_death`,
    not `take_turn_plan`.
  - Central death dispatch marks the dying monster before monster `on_death`,
    matching Java `super.die()` before Shield/Spear cleanup loops.

Verification for `d245435`:

- `cargo test surrounded --all-targets` -> `7 passed`
- `cargo test spire_shield --all-targets` -> `9 passed`
- `cargo test spire_spear --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1346 passed`

No-code relic audit after `d245435`:

- Java checked:
  - `Damaru`
  - `AncientTeaSet`
  - `ArtOfWar`
  - `RunicCapacitor`
  - `EmotionChip`
  - `HoveringKite`
- Rust checked:
  - `src/content/relics/damaru.rs`
  - `src/content/relics/ancient_tea_set.rs`
  - `src/content/relics/art_of_war.rs`
  - `src/content/relics/runic_capacitor.rs`
  - `src/content/relics/emotion_chip.rs`
  - `src/content/relics/hovering_kite.rs`
  - `src/content/relics/hooks.rs`
- Result:
  - No code change needed.
  - Java private booleans are already represented by Rust `counter`, `amount`,
    or `used_up` and are mutated synchronously in relic hooks.
  - Java visual-only `RelicAboveCreatureAction`, flash, pulse, and sound paths
    remain intentionally omitted where they do not alter mechanics.
  - Existing Rust tests cover the mechanical hooks for Ancient Tea Set, Art of
    War, Damaru via the watcher relic hook test, Emotion Chip, Hovering Kite,
    and Runic Capacitor.

Verification for no-code relic audit:

- `cargo test ancient_tea_set --all-targets` -> `1 passed`
- `cargo test art_of_war --all-targets` -> `1 passed`
- `cargo test emotion_chip --all-targets` -> `1 passed`
- `cargo test hovering_kite --all-targets` -> `1 passed`
- `cargo test runic_capacitor --all-targets` -> `1 passed`

`63487e7` summary:

- Java checked:
  - `MutagenicStrength.atBattleStart()`.
- Clarified and locked Mutagenic Strength queue order:
  - Java calls `addToTop(Strength)` and then `addToTop(LoseStrength)`.
  - Rust already emitted the correct Java call order, so actual queue execution
    after `queue_actions()` is `LoseStrength` then `Strength`.
  - The Rust comment incorrectly said `addToBot`; it now states the Java
    `addToTop` ordering.
  - Added a regression that checks both the returned call order and the actual
    queued execution order.

Verification for `63487e7`:

- `cargo test mutagenic_strength --all-targets` -> `1 passed`
- `cargo test brimstone --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1342 passed`

`22db4c7` summary:

- Java checked:
  - `Brimstone.atTurnStart()`.
- Fixed Brimstone action ordering:
  - Java calls `addToTop` for player Strength first, then `addToTop` for each
    monster in monster-list order.
  - Because later `addToTop` actions execute before earlier ones, the actual
    queued execution order is last monster, earlier monsters, then player.
  - Rust previously emitted the apparent execution order into `ActionInfo`
    records, but the shared `queue_actions()` helper reversed top insertions
    again, causing player Strength to execute first.
  - Rust now emits the Java call order and tests the actual queued order after
    `queue_actions()`.

Verification for `22db4c7`:

- `cargo test brimstone --all-targets` -> `2 passed`
- `cargo test red_skull --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1341 passed`

`d86fe7e` summary:

- Java checked:
  - `RedSkull`, `BloodVial`, player `damage()` bloodied transition, and
    `AbstractCreature.heal()`.
- Fixed Red Skull timing/state:
  - Java `RedSkull.atBattleStart()` queues a custom bottom action and checks
    `player.isBloodied` only when that action resolves.
  - Rust previously generated Strength during the relic hook from the initial
    HP value, which could incorrectly apply Red Skull after an earlier
    top-inserted battle-start heal such as Blood Vial.
  - Rust now queues `Action::RedSkullBattleStartCheck`, executes the bloodied
    check at action resolution time, and stores Java's private `isActive` flag
    in `RelicState.used_up`.
  - HP threshold hooks now consult that active flag, so healing from bloodied
    to non-bloodied no longer queues `-3 Strength` unless Red Skull was
    actually active.
  - Red Skull now subscribes to victory cleanup to reset the active flag.

Verification for `d86fe7e`:

- `cargo test red_skull --all-targets` -> `3 passed`
- `cargo test blood_vial --all-targets` -> `2 passed`
- `cargo test hp_loss --all-targets` -> `27 passed`
- `cargo test --all-targets` -> `1341 passed`

`4c05934` summary:

- Java checked:
  - `PlatedArmorPower`, `RupturePower`, `SplitPower`, and large slime / Slime
    Boss damage overrides.
- Fixed HP-loss power action insertion:
  - Java `RupturePower.wasHPLost()` uses `addToTop`.
  - Java `PlatedArmorPower.wasHPLost()` uses `addToBot`.
  - Rust previously queued all hp-lost power actions to the front.
  - Rust now routes Plated Armor's `ReducePower` to the back while preserving
    existing top behavior for Rupture and existing immediate/bottom split
    interrupt handling for large slimes.
- Added regression:
  - Existing queued actions stay ahead of Plated Armor's `ReducePower`.

Verification for `4c05934`:

- `cargo test plated_armor_hp_loss_reduction_is_added_to_bottom_like_java --all-targets`
  -> `1 passed`
- `cargo test rupture_and_reaper_execution_hooks_match_java_sources --all-targets`
  -> `1 passed`
- `cargo test large_slime_split --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1339 passed`

`369d112` summary:

- Java checked:
  - `NeowsLament`.
- Fixed Neow's Lament battle-start timing:
  - Java decrements `counter`, calls `setCounter(-2)` / `usedUp()` on the
    third use, and mutates each monster's `currentHealth = 1` synchronously
    inside `atBattleStart()`.
  - Rust previously returned queued `SetCurrentHp`, `UpdateRelicCounter`, and
    `UpdateRelicUsedUp` actions.
  - Rust now mutates `RelicState.counter`, `RelicState.used_up`, and monster
    `current_hp` before the hook returns, and produces no gameplay actions for
    this relic.
- Follow-up search:
  - `rg "Action::UpdateRelicCounter|Action::UpdateRelicUsedUp|IncrementRelicCounter|UpdateRelicCounter|UpdateRelicUsedUp" src\content\relics src\engine`
    now finds only engine handlers/comments, not content relic producers.

Verification for `369d112`:

- `cargo test neows_lament --all-targets` -> `2 passed`
- `cargo test shared_event_special_relic_followup_metadata_matches_java_sources --all-targets`
  -> `1 passed`
- `cargo test initial_battle_start --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1338 passed`

`c52b087` summary:

- Java checked:
  - `Necronomicon`, `Lantern`, and `UnceasingTop`.
- Fixed Necronomicon activation timing:
  - Java `Necronomicon.onUseCard()` sets private `activated = false`
    synchronously before enqueuing the replayed attack.
  - Rust modeled availability as `RelicState.used_up`, but was changing it via
    queued `Action::UpdateRelicUsedUp`.
  - Rust now mutates `used_up` immediately in `on_use_card`, and
    `at_turn_start` immediately restores it to `false`.
- Confirmed by source/test review:
  - Lantern first-turn state is already synchronous via `used_up`.
  - Unceasing Top already stores `canDraw` in `amount` and
    `disabledUntilEndOfTurn` in `used_up`, with mechanical UI/screen gates
    intentionally represented only where they affect headless legality.

Verification for `c52b087`:

- `cargo test necronomicon --all-targets` -> `3 passed`
- `cargo test unceasing_top --all-targets` -> `1 passed`
- `cargo test lantern --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1337 passed`

`52fb5c8` summary:

- Java checked:
  - `Kunai`, `Shuriken`, `LetterOpener`, `OrnamentalFan`, `OrangePellets`,
    and `Inserter`.
- Fixed Rust relic counter mutation timing:
  - Java updates these relic counters synchronously inside `atTurnStart()` or
    `onUseCard()`, then queues gameplay actions such as Strength, Dexterity,
    all-enemy damage, block, RemoveDebuffs, or IncreaseMaxOrb.
  - Rust still had several older helpers that returned
    `Action::UpdateRelicCounter`, delaying the visible/internal counter update
    until the action queue drained.
  - Rust now mutates `RelicState` immediately in the relic hook helpers and
    only returns the real queued gameplay actions.
  - `OrangePellets` now treats negative/default counter values as empty flags,
    matching Java's false/false/false static booleans.

Verification for `52fb5c8`:

- `cargo test letter_opener --all-targets` -> `1 passed`
- `cargo test attack_counter_relics --all-targets` -> `1 passed`
- `cargo test orange_pellets --all-targets` -> `3 passed`
- `cargo test inserter --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1337 passed`

`227a871` summary:

- Java `OrangePellets.atTurnStart()` was checked against Rust's existing
  special-case reset in `engine/core.rs`.
- Fixed Rust Orange Pellets turn reset:
  - Java resets the Attack/Skill/Power flags through the relic `atTurnStart`
    hook.
  - Rust previously reset the counter only in the regular new-player-turn path,
    so a stale combo state absorbed from the previous combat could survive into
    the first turn of a new combat.
  - Rust now subscribes Orange Pellets to the `at_turn_start` relic hook and
    removes the core special case.

Verification for `227a871`:

- `cargo test orange_pellets --all-targets` -> `3 passed`
- `cargo test shared_shop_relic_gap_batch_two_metadata_matches_java_sources --all-targets`
  -> `1 passed`
- `cargo test initial_battle_start --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1337 passed`

`c0fbef0` summary:

- Java `GainEnergyAndEnableControlsAction`, Java `AbstractPlayer.gainEnergy`,
  Java `triggerOnGainEnergy`, Java relic/power `onEnergyRecharge`, Java
  `AbstractPlayer.preBattlePrep()`, Rust first-turn energy initialization, and
  Rust energy-related powers were checked.
- No Rust opening-energy action was added:
  - Java opening `GainEnergyAndEnableControlsAction` calls card
    `triggerOnGainEnergy`, relic `onEnergyRecharge`, and power
    `onEnergyRecharge`.
  - Java active cards and relics do not override those gain-energy hooks.
  - Java powers that override `onEnergyRecharge` are `CollectPower`,
    `EnergizedPower`, `EnergizedBluePower`, and `DevaPower`.
  - Java `preBattlePrep()` clears player powers before combat, and the modeled
    normal opening combat path does not create those powers before the opening
    energy refill.
  - Therefore Rust's existing first-turn energy initialization is mechanically
    equivalent for currently modeled normal combat.
- While auditing Java relic start hooks, fixed `VelvetChoker` public counter
  parity:
  - Java `onEquip` / `onUnequip` still own the energy-master delta.
  - Java `atBattleStart` and `atTurnStart` reset `counter = 0`.
  - Java `onPlayCard` increments public `counter` up to `6`.
  - Java `canPlay` gates playability from `counter >= 6`, while Rust continues
    using the already-correct engine turn counter for the hard gameplay gate.
  - Java `onVictory` sets `counter = -1`.
  - Rust now subscribes Velvet Choker to battle-start, turn-start, use-card,
    and victory hooks so public observation/replay sees the Java counter.

Verification for `c0fbef0`:

- `cargo test velvet_choker_public_counter_matches_java_turn_and_victory_hooks --all-targets`
  -> `1 passed`
- `cargo test shared_boss_relic_third_batch_metadata_matches_java_sources --all-targets`
  -> `1 passed`
- `cargo test velvet_choker --all-targets` -> `2 passed`
- `cargo test initial_battle_start --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1335 passed`

`3fda120` summary:

- Java `AbstractRoom.update()`, Java `AbstractPlayer.applyStartOfCombat*`,
  Java `AbstractPlayer.applyStartOfTurn*`, Java `AbstractCreature`
  start-of-turn hooks, Rust `PreBattleTrigger`, Rust
  `BattleStartPreDrawTrigger`, and Rust `BattleStartTrigger` were checked.
- Fixed Rust initial combat hook queue construction:
  - Java constructs the whole opening action queue before `actionManager`
    drains it.
  - `atBattleStartPreDraw` hook methods run before the initial
    `DrawCardAction` is appended, so their bottom actions stay before the
    opening draw.
  - `atBattleStart`, `atTurnStart` relics, `atTurnStartPostDraw` relics,
    card `atTurnStart`, power `atStartOfTurn`, and orb start hooks run after
    the opening draw action is already queued but before it executes.
  - Rust no longer queues a later synthetic `BattleStartTrigger` behind the
    draw; `BattleStartPreDrawTrigger` now synchronously builds the Java-order
    queue.
  - Initial combat calls post-draw relic hooks but not post-draw power hooks,
    matching `AbstractRoom.update()`.
- Added regression tests proving:
  - Lantern fires through first-turn `applyStartOfTurnRelics` and its
    `addToTop` energy action lands before the opening draw;
  - Gambling Chip opens its hand-select after the opening draw has populated
    the hand;
  - an artificial `DrawCardNextTurn` power does not fire its post-draw hook at
    initial battle start.

Verification for `3fda120`:

- `cargo test initial_battle_start_runs_turn_start_relics_before_opening_draw_like_java --all-targets`
  -> `1 passed`
- `cargo test initial_battle_start_gambling_chip_suspends_after_opening_draw_like_java --all-targets`
  -> `1 passed`
- `cargo test initial_battle_start_does_not_run_power_post_draw_hooks_like_java --all-targets`
  -> `1 passed`
- `cargo test initial_battle_start --all-targets` -> `3 passed`
- `cargo test post_draw --all-targets` -> `4 passed`
- `cargo test gambling_chip --all-targets` -> `2 passed`
- `cargo test lantern --all-targets` -> `1 passed`
- `cargo test ring_of_the_serpent_increases_opening_and_turn_start_draw_count --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1334 passed`

`012e056` summary:

- Java `GameActionManager`, Java `AbstractCreature.atStartOfTurnPostDraw`,
  Java post-draw powers, Java `VoidCard`, Rust `PostDrawTrigger`, and Rust
  draw-card handling were checked.
- Fixed regular new-turn post-draw hook queue ordering:
  - Java calls `atStartOfTurnPostDraw` hook methods before `DrawCardAction`
    executes.
  - Those hook methods use `addToBot`, so their actions land behind the
    already-queued turn-start `DrawCardAction`, but ahead of actions generated
    while that draw action executes.
  - Rust's synthetic `PostDrawTrigger` now runs before the queued
    `DrawCards`, so it appends hook actions behind `DrawCards` and ahead of
    draw-generated actions.
- Fixed `Void` draw trigger ordering:
  - Java `VoidCard.triggerWhenDrawn()` uses `addToBot(new LoseEnergyAction(1))`.
  - Rust had modeled it as top insertion.
  - Rust now queues the energy loss to the bottom.
- Added a regression test proving `DrawCardNextTurn` post-draw actions remain
  ahead of Void's draw-generated energy loss.

Verification for `012e056`:

- `cargo test turn_start_post_draw_hooks_queue_before_draw_generated_actions_like_java --all-targets`
  -> `1 passed`
- `cargo test post_draw --all-targets` -> `3 passed`
- `cargo test draw_card_next --all-targets` -> `0 matched`
- `cargo test gambling_chip --all-targets` -> `1 passed`
- `cargo test void --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1331 passed`

`ea1570c` summary:

- Java `GameActionManager.getNextAction()`, Java
  `MonsterGroup.applyEndOfTurnPowers()`, Java `DrawReductionPower`, and Rust
  `tick_engine()` combat turn-transition logic were checked.
- Fixed Rust's round-end queue timing:
  - Java calls monster `atEndOfTurn`, player `atEndOfRound`, and monster
    `atEndOfRound` hooks as synchronous hook methods that enqueue actions.
  - Java does not drain those queued actions before it runs the following
    player start-of-turn hook methods and constructs the next-turn
    `DrawCardAction`.
  - Rust was draining monster end-of-turn actions before `atEndOfRound`, and
    draining round cleanup before the player start-of-turn setup.
  - Rust now queues the collective end-of-turn and end-of-round actions and
    leaves them in order ahead of the queued player start-of-turn actions.
- Added a regression test for `DrawReductionPower`:
  - Java queues `ReducePowerAction`, then constructs the next-turn
    `DrawCardAction` from the still-reduced `player.gameHandSize`.
  - Rust now draws 4 cards on the expiration turn, then removes
    `DrawReduction` before player control returns.

Verification for `ea1570c`:

- `cargo test draw_reduction_decay_is_queued_before_next_turn_draw_count_like_java_game_hand_size --all-targets`
  -> `1 passed`
- `cargo test blur_retains_player_block_through_next_turn_while_power_ticks_down --all-targets`
  -> `1 passed`
- `cargo test draw_reduction --all-targets` -> `2 passed`
- `cargo test end_of_round --all-targets` -> `1 passed`
- `cargo test monster_pre_turn_invincible_resets_before_poison_like_java_at_start_of_turn --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1330 passed`

`10997a8` summary:

- Java `MonsterGroup.applyPreTurnLogic()`, Java
  `AbstractCreature.applyStartOfTurnPowers()`, Java `InvinciblePower`, Java
  `PoisonPower`, Java `PoisonLoseHpAction`, Rust power start hooks, and Rust
  poison damage handling were checked.
- Fixed monster start-of-turn `Invincible` timing:
  - Java `InvinciblePower.atStartOfTurn()` immediately resets `amount` to
    `maxAmt` during the monster group's pre-turn power pass.
  - Rust was resetting `Invincible` later, just before the monster's
    `takeTurn()`.
  - Rust now resets `Invincible` in the normal `resolve_power_instance_at_turn_start`
    hook and no longer performs a second pre-`takeTurn` reset.
- Fixed monster `PoisonLoseHp` to use the normal HP_LOSS damage pipeline:
  - Java `PoisonLoseHpAction` calls `target.damage(new DamageInfo(...,
    HP_LOSS))`, so `InvinciblePower.onAttackedToChangeDamage` caps it.
  - Rust was manually subtracting monster HP and bypassing `Invincible`.
  - Rust now routes monster poison HP loss through `apply_damage_to_monster_via_pipeline`
    before decrementing/removing Poison and running post-combat cleanup.
- Added a regression test proving start-of-turn `Invincible` resets before
  Poison HP loss and is not reset again before the monster acts.

Verification for `10997a8`:

- `cargo test monster_pre_turn_invincible_resets_before_poison_like_java_at_start_of_turn --all-targets`
  -> `1 passed`
- `cargo test invincible --all-targets` -> `5 passed`
- `cargo test poison --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1329 passed`

`4fd646b` summary:

- Java `MonsterGroup.areMonstersBasicallyDead()` and Rust
  `settle_victory_if_ready` were checked after the `MonsterGroup` lifecycle
  packet exposed the filter mismatch.
- Fixed Rust victory settlement to use the same Java predicate:
  - Java `areMonstersBasicallyDead()` only treats monsters as absent when they
    are `isDying` or `isEscaping`.
  - Java does not treat `currentHealth <= 0` as basically dead by itself.
  - Rust previously inferred victory from `current_hp <= 0` unless rebirth
    powers were present.
  - Rust now delegates victory readiness to
    `CombatState::are_monsters_basically_dead_java()`.
- Added a regression test proving a zero-HP monster that is not dying/escaping
  does not settle combat victory.

Verification for `4fd646b`:

- `cargo test victory_settlement_uses_java_basically_dead_flags_not_zero_hp --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1328 passed`

`556788e` summary:

- Java `MonsterGroup.applyPreTurnLogic()`, `MonsterGroup.queueMonsters()`,
  `MonsterGroup.applyEndOfTurnPowers()`, Java `GameActionManager` monster
  queue handling, Java `AbstractCreature.applyTurnPowers()`, Java
  `FadingPower`, Java `ExplosivePower`, and Rust `engine/core.rs` were checked.
- Fixed Rust monster `duringTurn()` lifecycle timing:
  - Java calls `m.takeTurn(); m.applyTurnPowers();` for one monster, then
    drains the queued actions before dequeuing the next monster.
  - Only Java `FadingPower` and `ExplosivePower` override `duringTurn()`.
  - Rust was resolving `Fading` / `Explosive` inside the group-level
    end-of-turn power pass, after all monsters had acted.
  - Rust now has a separate `resolve_power_during_turn` hook and queues those
    actions immediately after each monster's `takeTurn` actions, before the
    current monster's queue is drained and before the next monster acts.
- Added tests proving:
  - `Fading` and `Explosive` no longer fire from
    `resolve_power_at_end_of_turn`;
  - their Java action order is preserved through the new `duringTurn` hook;
  - `Explosive` damage can kill the player before the next monster is dequeued,
    matching Java `GameActionManager`.

Verification for `556788e`:

- `cargo test monster_during_turn_powers_fire_before_next_monster_turn_like_java_apply_turn_powers --all-targets`
  -> `1 passed`
- `cargo test explosive_and_fading_countdowns_match_java_during_turn_action_order --all-targets`
  -> `1 passed`
- `cargo test explosive --all-targets` -> `2 passed`
- `cargo test fading --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1327 passed`

`894274a` summary:

- Java `MonsterGroup.usePreBattleAction()`, Java `AbstractMonster` universal
  pre-battle hook, Java Louse pre-battle code, and Rust
  `handle_pre_battle_trigger` were checked.
- Fixed Rust normal combat pre-battle RNG stream:
  - Java `MonsterGroup.usePreBattleAction()` calls each monster's
    `usePreBattleAction()` without changing RNG streams.
  - The only monster pre-battle code found in Java that consumes dungeon RNG is
    Louse Curl Up, and it explicitly uses `AbstractDungeon.monsterHpRng`.
  - Rust was passing `PreBattleLegacyRng::Misc` from the group-level
    `PreBattleTrigger`, causing Louse Curl Up to consume `misc_rng`.
  - Rust now passes `PreBattleLegacyRng::MonsterHp` for group pre-battle,
    matching Java.
- Added a handler-level test proving Louse Curl Up consumes `monster_hp_rng`,
  leaves `misc_rng` untouched, and queues `BattleStartPreDrawTrigger` after the
  monster pre-battle action.
- Java `useUniversalPreBattleAction()` contains Daily/Endless/blight mechanics
  (`Lethality`, blights, `Time Dilation`) and was not implemented in this
  packet because those global modifiers are outside the currently modeled
  normal-run mechanics.

Verification for `894274a`:

- `cargo test monster_group_pre_battle_uses_monster_hp_rng_for_louse_curl_up_like_java --all-targets`
  -> `1 passed`
- `cargo test pre_battle --all-targets` -> `24 passed`
- `cargo test --all-targets` -> `1326 passed`

`3d4805e` summary:

- Java `MonsterHelper`, Java `MonsterGroup`, Java
  `com.megacrit.cardcrawl.random.Random`, and Rust monster factory were
  checked.
- Corrected the previous fixed-HP RNG conclusion from `06e5f9f`:
  - Java `AbstractMonster.setHp(int)` calls `setHp(hp, hp)`.
  - Java `Random.random(start, end)` increments its counter even when
    `start == end`.
  - Rust `spawn_monster` therefore again consumes exactly one monster HP RNG
    roll for every monster constructor, including fixed-HP monsters such as
    Spire Shield, Spire Spear, and Corrupt Heart.
- Fixed Java `MonsterHelper.bottomHumanoid()` / `bottomWildlife()` candidate
  construction parity:
  - Java `bottomGetWeakWildlife()` constructs `getLouse()`, `SpikeSlime_M`,
    and `AcidSlime_M` before selecting one with `miscRng`.
  - Java `bottomGetStrongHumanoid()` constructs `Cultist`, `getSlaver()`, and
    `Looter` before selecting one.
  - Java `bottomGetStrongWildlife()` constructs both `FungiBeast` and
    `JawWorm` before selecting one.
  - Rust now constructs the same temporary candidates at the eventual slot and
    discards the unselected objects, preserving constructor HP RNG and louse
    bite RNG consumption.
- Confirmed by source scan that the remaining MonsterHelper random pools
  (`spawnGremlins`, `spawnManySmallSlimes`, `spawnShapes`, `getAncientShape`,
  `spawnSmallSlimes`) choose keys before constructing objects and do not need
  this discarded-candidate treatment.

Verification for `3d4805e`:

- `cargo test factory --all-targets` -> `5 passed`
- `cargo test final_act_initializes_shield_spear_and_heart_context --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1325 passed`

`06e5f9f` summary:

- Java `TheEnding`, Java `MonsterRoomBoss`, Rust `RunState` final-act setup,
  and Rust monster factory final-act encounter creation were checked.
- This commit's fixed-HP RNG conclusion was superseded by `3d4805e`; do not use
  the `06e5f9f` commit message or its old tests as source truth for
  `setHp(int)`.
- Existing final-act run test still locks Java `TheEnding` map/context:
  rest -> shop -> elite Shield/Spear -> boss Heart -> true victory, encounter
  lists with three Shield/Spear and three Heart entries, boss key visibility,
  transition heal, potion drop reset, and card RNG band alignment.

`1879996` summary:

- `CorruptHeart`, Java `BeatOfDeathPower`, Java `InvinciblePower`, and Java
  `PainfulStabsPower` were checked.
- Fixed Heart buff-turn private runtime timing:
  - Java queues the Strength and follow-up buff actions, then synchronously
    increments private `buffCount` before queued actions can execute and before
    `RollMoveAction`.
  - Rust now emits the `CorruptHeart` runtime update before the queued
    `ApplyPower` actions, matching the same synchronous-state principle used
    for Maw/Exploder/Transient-style fixes.
- Added tests proving:
  - pre-battle `Invincible` and `BeatOfDeath` use Java's A19 gate
    (`Invincible 300 / Beat 1` below A19, `Invincible 200 / Beat 2` at A19+);
  - first `getMove()` selects Debilitate and only clears private `firstMove`,
    without incrementing `moveCount`;
  - Painful Stabs follow-up uses Java sentinel amount `-1`;
  - buff turn cleanses negative Strength by adding `-Strength + 2`, picks the
    Java `buffCount == 1` Beat of Death follow-up, and updates private
    `buffCount` before queued powers execute.
- Confirmed existing tests already cover:
  - `InvinciblePower` storing its Java `maxAmt` reset amount in `extra_data`;
  - `InvinciblePower.onAttackedToChangeDamage` capping both ordinary damage and
    HP_LOSS.

Verification for `1879996`:

- `cargo test corrupt_heart --all-targets` -> `5 passed`
- `cargo test beat_of_death --all-targets` -> `1 passed`
- `cargo test invincible --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1321 passed`

`5fe09ea` summary:

- `SpireShield`, `SpireSpear`, Java `SurroundedPower`, Java
  `BackAttackPower`, and Java `AbstractMonster.calculateDamage()` BackAttack
  placement were checked.
- Fixed Shield pre-battle `Surrounded` action trace parity:
  - Java `new SurroundedPower(player)` has sentinel `amount == -1`.
  - Rust now emits `ApplyPower { power_id: Surrounded, amount: -1 }` before
    the Shield Artifact action.
- Fixed existing `BackAttack` damage behavior in the shared monster damage
  pipeline:
  - Java multiplies monster damage by `1.5` when `applyBackAttack()` /
    `BackAttackPower` is active, after player receive modifiers and before
    final receive powers such as Intangible.
  - Rust now applies the same multiplier when the source monster already has
    `PowerId::BackAttack`.
- Added Shield tests proving:
  - Surrounded sentinel then Artifact A18 pre-battle order;
  - Bash does not consume the Focus/Strength random roll when the player has
    no orbs;
  - Bash consumes `ai_rng.randomBoolean()` during `takeTurn()` when the player
    has an orb and can apply Focus;
  - Fortify loops every monster in the group, including zero-HP non-dying
    monsters.
- Added Spear tests proving:
  - Artifact uses Java's A18 gate;
  - A18 Burn Strike queues two attacks, then two Burns to draw pile top, then
    `RollMonsterMove`;
  - Piercer buffs every monster in the group, including zero-HP non-dying
    monsters;
  - Skewer uses imported/runtime `skewer_count`, not just ascension defaults.
- Important unresolved boundary:
  - Java automatic BackAttack application/removal depends on UI-tied facing
    state (`player.flipHorizontal`, `drawX`, `AbstractMonster.applyBackAttack()`).
  - Rust currently has no player facing/drawX model. This packet fixed the
    damage multiplier when `BackAttack` is already present, but did not fake
    automatic facing-based BackAttack creation. Treat that as a separate
    architecture packet if live protocol or parity work requires it.

Verification for `5fe09ea`:

- `cargo test back_attack --all-targets` -> `3 passed`
- `cargo test spire_shield --all-targets` -> `9 passed`
- `cargo test spire_spear --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1318 passed`

`24e4618` summary:

- Java `SpawnMonsterAction.update()`, Java `PhilosopherStone.onSpawnMonster`,
  and Rust spawn/relic hooks were checked.
- Fixed spawned-monster relic hook timing:
  - Java calls `r.onSpawnMonster(m)` before `m.init()`, `m.applyPowers()`, and
    `addMonster(...)`.
  - `PhilosopherStone.onSpawnMonster` directly calls `monster.addPower(...)`;
    it does not queue an `ApplyPowerAction`.
  - Rust now applies on-spawn relic effects as immediate `AbstractCreature`
    `addPower`-style state mutation before inserting the spawned monster and
    before rolling its first move.
- Fixed the same direct hook semantics for `Darkling` reincarnate:
  - Java queues Heal / ChangeState / ApplyPower(Regrow), then synchronously
    calls relic `onSpawnMonster(this)` during `takeTurn()` construction.
  - Rust now mutates the Darkling immediately instead of appending a queued
    Strength action.
- Added/updated tests proving:
  - Philosopher Stone spawn Strength is present immediately and only Minion
    remains queued to the front for spawned minions;
  - Philosopher Stone battle-start and spawn hooks both apply Strength;
  - existing Darkling reincarnate ordering tests still pass with direct hook
    mutation.

Verification for `24e4618`:

- `cargo test spawn_monster --all-targets` -> `1 passed`
- `cargo test darkling --all-targets` -> `8 passed`
- `cargo test philosopher --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1309 passed`

`bf619c7` summary:

- Dedicated `Reptomancer` / `SnakeDagger` packet was checked against Java
  source.
- Fixed the remaining Minion pre-battle source parity for both `Reptomancer`
  and `GremlinLeader`:
  - Java uses `new ApplyPowerAction(m, m, new MinionPower(this))`.
  - Rust now emits `ApplyPower { source: minion, target: minion,
    power_id: Minion, amount: -1 }` instead of using the summoner as source.
- Added Reptomancer tests proving:
  - initial dagger slots are mapped by Java monster-group index:
    daggers after Reptomancer go to `daggers[0]`, daggers before it go to
    `daggers[1]`;
  - A18 spawn turns fill the first available Java `daggers[]` slots and queue
    both spawns before `RollMonsterMove`;
  - Java `canSpawn()` counts zero-HP or escaped non-dying monsters because it
    skips only `this` and `isDying`;
  - Snake Strike queues two 16-damage hits at A3+, then Weak, then roll.
- Existing SnakeDagger tests still lock Java firstMove runtime truth and
  explode using `LoseHPAction`, not `SuicideAction`.
- Java VFX/animation/WaitAction effects remain presentation-only.

Verification for `bf619c7`:

- `cargo test reptomancer --all-targets` -> `10 passed`
- `cargo test snake_dagger --all-targets` -> `4 passed`
- `cargo test gremlin_leader --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1308 passed`

`5aa6309` summary:

- `Exploder` and Java `ExplosivePower` were checked.
- Fixed `Exploder.takeTurn()` timing:
  - Java synchronously increments private `turnCount` before the queued
    attack body and before `RollMoveAction`.
  - Rust now emits the `Exploder` runtime update first, then the attack body
    when present, then `RollMonsterMove`.
  - The Java UNKNOWN/BLOCK branch still increments `turnCount` before rolling,
    even though the switch body has no queued gameplay action.
- Confirmed pre-battle `ExplosivePower` amount is 3.
- Confirmed existing Rust `ExplosivePower` countdown order already matches
  Java: countdown reduces the power until amount 1, then queues suicide before
  the 30 THORNS player damage.
- Java `AnimateSlowAttackAction`, animation startup randomness, and explosion
  VFX were treated as presentation-only.

Verification for `5aa6309`:

- `cargo test exploder --all-targets` -> `5 passed`
- `cargo test explosive --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1304 passed`

`a8e2118` summary:

- `Repulsor` and Java `MakeTempCardInDrawPileAction` were checked.
- No business logic change was needed.
- Added tests proving:
  - low roll selects Attack only when Java `lastMove(ATTACK)` is false;
  - `num >= 20` selects Daze;
  - A2+ Attack queues one 13-damage attack before `RollMoveAction`;
  - Daze queues `MakeTempCardInDrawPileAction(new Dazed(), 2, true, true)`
    as `MakeTempCardInDrawPile { random_spot: true, to_bottom: false }`,
    then `RollMoveAction`.
- Java `AnimateSlowAttackAction`, animation startup randomness, and card-display
  effects were treated as presentation-only after confirming the gameplay
  mutation is the underlying draw-pile insertion.

Verification for `a8e2118`:

- `cargo test repulsor --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1302 passed`

`945681d` summary:

- `OrbWalker`, Java `GenericStrengthUpPower`, and Java
  `MakeTempCardInDiscardAndDeckAction` were checked.
- No business logic change was needed.
- Added tests proving:
  - pre-battle `GenericStrengthUpPower` uses Java's A17 gate: amount 3 below
    A17, amount 5 at A17+;
  - `getMove(int)` uses Java `lastTwoMoves(CLAW)` / `lastTwoMoves(LASER)`
    gates without recursive rerolling;
  - A2+ Laser queues damage 11, `MakeTempCardInDiscardAndDeckAction(Burn)`,
    then `RollMoveAction`;
  - existing Laser test keeps the shared action as one
    `MakeTempCardInDiscardAndDeck`, not two hand-expanded add-card actions.
- Java `AnimateSlowAttackAction`, `ChangeStateAction`, `WaitAction`, hit
  animation, animation startup randomness, and card-display effects were
  treated as presentation-only after confirming the gameplay mutation is the
  underlying draw/discard pile insertion.

Verification for `945681d`:

- `cargo test orb_walker --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1299 passed`

`87044fb` summary:

- `WrithingMass`, Java `ReactivePower`, Java `MalleablePower`, and Java
  `AddCardToDeckAction` were checked.
- Fixed Writhing Mass pre-battle `Reactive` amount:
  - Java `ReactivePower` does not set amount and therefore inherits
    `AbstractPower.amount == -1`.
  - Rust now applies `Reactive` with amount `-1` and treats `Reactive` as a
    sentinel amount power.
- Fixed `ReactivePower.onAttacked` queue direction:
  - Java `ReactivePower` calls `addToBot(new RollMoveAction(owner))`.
  - Rust now queues Reactive rerolls to the back, preserving existing queued
    actions ahead of the reroll.
- Confirmed existing Rust Writhing Mass runtime state already models Java
  `firstMove` and `usedMegaDebuff`, including first-move clearing and
  Mega-Debuff runtime update before adding Parasite.
- Java `FastShakeAction`, `AnimateFastAttackAction`, `AnimateSlowAttackAction`,
  `ChangeStateAction`, `WaitAction`, hit animation, and animation startup
  randomness were treated as presentation-only.

Verification for `87044fb`:

- `cargo test writhing_mass --all-targets` -> `6 passed`
- `cargo test reactive_power --all-targets` -> `2 passed`
- `cargo test sentinel_power_reapplication_matches_java_apply_power_special_cases --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1296 passed`

`9ce0e12` summary:

- `SpireGrowth` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - A17+ without player Constricted forces Constrict before the RNG Quick
    Tackle branch;
  - below A17, a low roll selects Quick Tackle before the non-constricted
    branch;
  - when player already has Constricted and the last two moves were Smash,
    Java fallback is Quick Tackle;
  - Constrict at A17 queues Constricted 12 before `RollMoveAction`;
  - Smash at A2+ queues one 25-damage attack before `RollMoveAction`.
- Java `AnimateFastAttackAction`, `AnimateSlowAttackAction`,
  `ChangeStateAction`, `WaitAction`, animation startup randomness, and Hurt
  animation were treated as presentation-only.

Verification for `9ce0e12`:

- `cargo test spire_growth --all-targets` -> `5 passed`
- `cargo test --all-targets` -> `1294 passed`

`17d05fd` summary:

- `Spiker` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - pre-battle Thorns follows Java ascension gates: 3 below A2, 4 at A2+, 7
    at A17+;
  - executed private `thornsCount > 5` forces Attack even when the RNG roll
    would otherwise allow Buff;
  - planned-but-unexecuted Buff history does not count as executed
    `thornsCount`;
  - low roll attacks only when the previous move was not Attack;
  - Attack queues one Java A2+ 9-damage attack before `RollMoveAction`;
  - Buff increments private `thornsCount` before queued Thorns
    `ApplyPowerAction`.
- Java animation and startup animation RNG remain presentation-only.

Verification for `17d05fd`:

- `cargo test spiker --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1289 passed`

`bcbd851` summary:

- `Maw` Java/Rust behavior was checked.
- Fixed `ROAR` timing:
  - Java queues Weak and Frail actions, then synchronously sets private
    `roared=true` before those queued debuffs execute, then queues
    `RollMoveAction`.
  - Rust now emits the private `roared` runtime update before queued Weak/Frail
    actions, preserving Java synchronous state mutation timing.
- Added tests proving:
  - imported private `roared=false` forces Roar even if history contains Roar;
  - imported private `turn_count` drives Nom hit count;
  - Java `lastMove(SLAM)` and `lastMove(NOMNOMNOM)` force Drool;
  - high roll after a non-attack move selects Slam and A2+ Slam damage 30;
  - RollMove increments Java private `turnCount`;
  - A17 Roar applies Weak/Frail 5 after the immediate `roared=true` update.
- Java `SFXAction`, `ShoutAction`, `AnimateSlowAttackAction`,
  `VFXAction(BiteEffect)`, animation, and death sound were treated as
  presentation-only. The Bite VFX `MathUtils` rolls are not gameplay RNG.

Verification for `bcbd851`:

- `cargo test maw --all-targets` -> `10 passed`
- `cargo test --all-targets` -> `1285 passed`

`d6a62f4` summary:

- `Transient`, Java `ShiftingPower`, and sentinel-power application were
  checked.
- Fixed `Transient.takeTurn()` timing:
  - Java queues the damage action, then synchronously increments private
    `count` and calls `setMove(...)` before queued damage can execute.
  - Rust now emits private `count` update and `SetMonsterMove` before the
    queued `MonsterAttack`, preserving Java synchronous move mutation timing.
- Fixed `ShiftingPower` amount truth:
  - Java `ShiftingPower` inherits `AbstractPower.amount == -1`.
  - Rust Transient pre-battle now applies Shifting with amount `-1`.
  - `PowerId::Shifting` is now a sentinel amount power so application/reapply
    keeps Java `-1` / stackPower(-1) behavior.
- Added tests proving:
  - Transient pre-battle applies Fading 5 below A17, Fading 6 at A17+, and
    Shifting `-1`;
  - Transient runtime count and next visible attack update happen before queued
    damage;
  - duplicate Shifting application follows Java default stackPower(-1)
    behavior.
- Java animation, `ChangeStateAction`, `WaitAction`, and achievement unlock were
  treated as presentation/meta-only.

Verification for `d6a62f4`:

- `cargo test transient --all-targets` -> `6 passed`
- `cargo test sentinel_power_reapplication_matches_java_apply_power_special_cases --all-targets`
  -> `1 passed`
- `cargo test --all-targets` -> `1283 passed`

`2aae03b` summary:

- `Donu` and `Deca` Java/Rust behavior were checked as a pair.
- No business logic change was needed.
- Added tests proving:
  - pre-battle Artifact uses Java's A19 gate: amount 2 below A19, amount 3
    at A19+;
  - Donu Beam at A4+ queues two 12-damage monster attacks, then updates
    private `isAttacking=false`, then rolls;
  - Donu Circle of Protection queues Strength for every monster in the current
    group, including a zero-HP non-dying ally object, before updating private
    `isAttacking=true` and rolling;
  - Deca Beam at A4+ queues two 12-damage monster attacks, then two Dazed into
    discard, then updates private `isAttacking=false`, then rolls;
  - Deca A19 Square of Protection queues block and Plated Armor interleaved per
    monster in Java loop order before updating private `isAttacking=true` and
    rolling.
- Java `ChangeStateAction`, `WaitAction`, SFX, BGM, unlock, death shake, and
  animation side effects were treated as presentation/meta-only because they do
  not mutate modeled gameplay state or gameplay RNG.

Verification for `2aae03b`:

- `cargo test donu --all-targets` -> `7 passed`
- `cargo test deca --all-targets` -> `15 passed`
- `cargo test --all-targets` -> `1282 passed`

`6c142a3` summary:

- `TimeEater` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - `lastTwoMoves(REVERBERATE)` blocks Reverberate and consumes Java
    `aiRng.random(50, 99)`;
  - `lastMove(HEAD_SLAM)` blocks Head Slam and consumes Java
    `aiRng.randomBoolean(0.66f)`;
  - `lastMove(RIPPLE)` blocks Ripple and consumes Java `aiRng.random(74)`;
  - A19 Ripple queues block, Vulnerable, Weak, Frail, then roll;
  - A19 Head Slam queues damage, Draw Reduction, two Slimed, then roll;
  - A19 Haste queues debuff removal, Shackled removal, execution-time heal,
    block, then roll.
- Existing TimeEater tests already covered execution-time Haste heal amount,
  Haste visible-spec placeholder, private `usedHaste`, and imported
  `usedHaste` not being reconstructed from history.
- Java first-turn `TalkAction`, `ChangeStateAction`, `WaitAction`, VFX/SFX,
  BGM, and unlock calls remain presentation/meta side effects outside the Rust
  combat simulator's modeled mechanics.

Verification for `6c142a3`:

- `cargo test time_eater --all-targets` -> `11 passed`
- `cargo test --all-targets` -> `1280 passed`

`9e6e73f` summary:

- `GiantHead` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - Java `lastTwoMoves(GLARE)` forces `COUNT` and decrements private `count`;
  - Java `lastTwoMoves(COUNT)` forces `GLARE` and decrements private `count`;
  - `IT_IS_TIME` stops decrementing private `count` at Java floor `-6` and
    caps the real damage table at starting damage + 30.
- Existing GiantHead tests already covered A18 pre-battle count decrement,
  SlowPower amount 0, count-driven `IT_IS_TIME`, and imported count not being
  reconstructed from move history.
- Java `ShoutAction`, SFX/death voice, animation, and MathUtils dialogue rolls
  were treated as presentation-only.

Verification for `9e6e73f`:

- `cargo test giant_head --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1274 passed`

`98ee287` summary:

- `Nemesis` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - Tri Attack queues three separate hits before self-Intangible and
    `RollMonsterMove`;
  - A18+ Tri Burn queues 5 Burns before self-Intangible and roll;
  - existing Intangible blocks the post-turn self-application, matching Java
    `hasPower("Intangible")`.
- Existing Nemesis tests already covered private `firstMove`,
  `scytheCooldown` pre-decrement, imported runtime truth, and Scythe cooldown
  reset.
- Java `ChangeStateAction`, `WaitAction`, `SFXAction`, `VFXAction`, fire
  particles, and `MathUtils` voice selection were treated as presentation-only
  because they do not mutate modeled gameplay state or gameplay RNG.

Verification for `98ee287`:

- `cargo test nemesis --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1271 passed`

`fcf0f0b` summary:

- `Reptomancer`, `SnakeDagger`, Java `SuicideAction`, Java
  `SpawnMonsterAction`, Java `LoseHPAction`, Java `FadingPower`, Java
  `ExplosivePower`, `TheCollector`, and `BronzeAutomaton` death cleanup were
  checked as one narrow packet.
- `Action::Suicide` now carries Java's `triggerRelics` flag.
- `handle_suicide(..., trigger_relics=true)` now sets HP to 0 and enters the
  central monster-death handler so power/relic death hooks run, matching
  Java `new SuicideAction(monster)`.
- Split slimes now emit `Suicide { trigger_relics: false }`, matching Java
  `new SuicideAction(this, false)`.
- Fading/Explosive and minion cleanup paths now emit
  `Suicide { trigger_relics: true }`.
- `Reptomancer`, `TheCollector`, and `BronzeAutomaton` death cleanup now emits
  minion suicides in Java `addToTop` mechanical order: while Java iterates the
  monster group forward, later minions' `SuicideAction` executes first.
- Added tests for:
  - default SuicideAction triggering The Specimen/Poison death hooks;
  - split-slime SuicideAction(false) skipping relic death hooks;
  - Reptomancer/Collector/Bronze cleanup reverse execution order;
  - updated split slime, Fading, Explosive, and SnakeDagger expectations.

Verification for `fcf0f0b`:

- `cargo test reptomancer --all-targets` -> `6 passed`
- `cargo test collector --all-targets` -> `11 passed`
- `cargo test bronze_automaton --all-targets` -> `7 passed`
- `cargo test snake_dagger --all-targets` -> `4 passed`
- `cargo test suicide --all-targets` -> `9 passed`
- `cargo test slime --all-targets` -> `10 passed`
- `cargo test fading --all-targets` -> `1 passed`
- `cargo test explosive --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1268 passed`

`c7a3546` summary:

- `Darkling`, Java `Darkling.damage()`, Java `RegrowPower`, Java
  `ApplyPowerAction`, Java `HealAction`, and the Rust death pipeline were
  checked.
- Fixed `REINCARNATE` action parity:
  - visible/spec power amount remains `Regrow 1`;
  - queued `Action::ApplyPower` now uses Java `ApplyPowerAction(..., 1)`;
  - the power handler still stores sentinel `Regrow.amount == -1`, matching the
    Java `RegrowPower` instance.
- Fixed `REINCARNATE` queue order to Java:
  `HealAction`, `ChangeStateAction("REVIVE")`, `ApplyPowerAction(Regrow, 1)`,
  relic `onSpawnMonster`, then `RollMoveAction`.
- Fixed first half-death timing:
  - Darkling is marked half-dead and not dying before power/relic death hooks;
  - powers remain visible to relic `onMonsterDeath` hooks, then are cleared;
  - `setMove(COUNT)` records an immediate move-history entry only when
    `nextMove != COUNT`;
  - queued `SetMoveAction(COUNT)` records the second Java move-history entry.
- Added tests for reincarnate queue order, duplicate COUNT move-history, the
  `nextMove != COUNT` guard, and The Specimen seeing Poison before Darkling
  powers are cleared.

Verification for `c7a3546`:

- `cargo test darkling --all-targets` -> `8 passed`
- `cargo test awakened_one --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1263 passed`

`30c73bb` summary:

- `AwakenedOne`, Java `AwakenedOne.damage()`, Java `UnawakenedPower`, and the
  Rust death pipeline were checked.
- Fixed pre-battle `Unawakened` amount to Java sentinel `-1`.
- Moved first-phase rebirth truth out of the Rust `Unawakened` power hook and
  into the central monster-death interrupt, matching Java ownership:
  `UnawakenedPower` has no `onDeath`; `AwakenedOne.damage()` mutates monster
  state immediately.
- First-phase lethal damage now immediately:
  - marks the monster half-dead and not dying;
  - removes debuffs, `Curiosity`, `Unawakened`, and `Shackled`;
  - sets runtime `form1=false`, `first_turn=true`;
  - sets planned move `REBIRTH` and writes one immediate move-history entry;
  - queues `ClearCardQueue` to the front and a later `SetMonsterMove(REBIRTH)`
    to the bottom, preserving Java's duplicate move-history behavior.
- Removed the now-dead `AwakenedRebirthClear` action variant/handler.
- Added tests for pre-battle power order/amounts, first-phase rebirth immediate
  state and queued `SetMoveAction`, and existing final-death Cultist escape.

Verification for `30c73bb`:

- `cargo test awakened_one --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1259 passed`

`a8e467e` summary:

- `Champ` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving:
  - crossing below half HP selects `ANGER` and mutates `threshold_reached`
    inside the Java `getMove()` equivalent;
  - threshold mode forces `EXECUTE` unless `lastMove(EXECUTE)` or
    `lastMoveBefore(EXECUTE)` blocks it;
  - the fourth pre-threshold roll forces `TAUNT` and resets `num_turns`;
  - A19 expands the Defensive Stance roll cap to `num <= 30` and increments
    `forge_times`;
  - `ANGER` queues first-turn runtime update, debuff cleanup, Shackled removal,
    Strength gain, then `RollMonsterMove`;
  - `FACE_SLAP` and `TAUNT` queue their debuffs in Java order.
- Java `TalkAction`, `ShoutAction`, VFX/SFX, and `MathUtils` dialogue/death
  quote rolls remain presentation-only for the Rust simulator.

Verification for `a8e467e`:

- `cargo test champ --all-targets` -> `8 passed`
- `cargo test --all-targets` -> `1257 passed`

`8385df0` summary:

- `BronzeAutomaton`, `BronzeOrb`, and Java `ApplyStasisAction` behavior were
  checked.
- Fixed `handle_apply_stasis` candidate selection: Java
  `CardGroup.getRandomCard(rng, rarity)` sorts matching cards by `cardID`
  before applying the RNG index. Rust now sorts rarity candidates by
  `cards::java_id(...)` before removal.
- Added tests for:
  - Stasis rarity-candidate ordering before `cardRandomRng` selection;
  - BronzeAutomaton first turn, Hyper Beam counter reset, post-Hyper no-counter
    increment, and normal Flail/Boost counter increments;
  - BronzeOrb usedStasis update, Support/Beam `lastTwoMoves` gates, and Stasis
    take-turn queue order.

Verification for `8385df0`:

- `cargo test bronze_automaton --all-targets` -> `6 passed`
- `cargo test bronze_orb --all-targets` -> `5 passed`
- `cargo test apply_stasis --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1251 passed`

`5232ea9` summary:

- `TheCollector` and `TorchHead` Java/Rust behavior were checked.
- No business logic change was needed.
- Added tests proving:
  - initial spawn queues two TorchHead spawns, then runtime update, then
    `RollMonsterMove`;
  - initial spawn is forced regardless of random roll;
  - turn-three `MEGA_DEBUFF` is forced until `ult_used` becomes true;
  - Fireball is blocked only by Java `lastTwoMoves(FIREBALL)`;
  - Mega Debuff queues Weak, Vulnerable, Frail, runtime update, then roll.
- Existing tests already covered Collector buff targeting, death cleanup, and
  enemy-slot-based revive behavior.

Verification for `5232ea9`:

- `cargo test collector --all-targets` -> `10 passed`
- `cargo test --all-targets` -> `1244 passed`

`6e9a4d6` summary:

- `GremlinLeader` Java/Rust behavior was checked.
- Fixed `GremlinLeader` and `Reptomancer` pre-battle Minion applications to use
  Java `AbstractPower.amount` sentinel `-1`.
- Fixed generic spawned-minion handling in `SpawnMonsterAction` /
  `SummonGremlinAction` equivalent code to queue Minion with `amount: -1`.
- Added GremlinLeader tests for Minion sentinel, Encourage queue order, STAB
  three-hit queue before `RollMonsterMove`, and existing slot-truth behavior.
- Added Reptomancer and generic spawned-minion sentinel tests.
- Confirmed GremlinLeader slot truth is already factory-seeded for authored
  encounters and state-sync-seeded for live truth import; Rally should continue
  to use `gremlin_slots`, not draw-position inference.

Verification for `6e9a4d6`:

- `cargo test gremlin_leader --all-targets` -> `8 passed`
- `cargo test reptomancer --all-targets` -> `5 passed`
- `cargo test spawned_minion_power_uses_java_sentinel_amount --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1240 passed`

`f511731` summary:

- `Taskmaster` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests proving Java's constant `SCOURING_WHIP` roll, wound-count
  ascension thresholds, below-A18 no-Strength path, and A18 queue order:
  damage, Wounds, Strength, then `RollMonsterMove`.
- Java `playSfx()` burns `MathUtils` only for voice selection and remains
  presentation-only for the Rust simulator.

Verification for `f511731`:

- `cargo test taskmaster --all-targets` -> `4 passed`
- `cargo test --all-targets` -> `1235 passed`

`0b984ca` summary:

- `Chosen` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests for the below-A17 second-roll Hex transition, Drain order
  (Weak then Strength), Debilitate order (attack then Vulnerable), and Poke
  two-hit execution before `RollMonsterMove`.

Verification for `0b984ca`:

- `cargo test chosen --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1231 passed`

`dc4622d` summary:

- `BookOfStabbing` Java/Rust behavior was checked.
- Fixed pre-battle `PainfulStabsPower` to use Java sentinel amount `-1`.
- Added tests for Painful Stabs pre-battle application, `stabCount` growth
  before visible hit count, A18 Big Stab incrementing future `stabCount`, and
  STAB take-turn multi-hit execution before `RollMonsterMove`.

Verification for `dc4622d`:

- `cargo test book_of_stabbing --all-targets` -> `5 passed`
- `cargo test painful_stabs --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1227 passed`

`aa55e3d` summary:

- Corrected sentinel-power action amounts to follow Java `AbstractPower.amount`
  truth: `ConfusionPower` and `BarricadePower` use `-1`, not synthetic `0` or
  `1`.
- `Snecko` Glare and `SneckoEye` now emit Confusion with `amount: -1`.
- `SphericGuardian` pre-battle Barricade now emits `amount: -1`, followed by
  Artifact `3` and block `40`.
- Added a focused SphericGuardian pre-battle queue-order test.

Verification for `aa55e3d`:

- `cargo test snecko --all-targets` -> `7 passed`
- `cargo test spheric_guardian --all-targets` -> `6 passed`
- `cargo test barricade --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1223 passed`

`632492c` summary:

- `Snecko` Java/Rust behavior was checked.
- Added tests for Glare, A17 Tail queuing Weak before Vulnerable, and Java
  `lastTwoMoves(BITE)` forcing Tail. The initial Confusion amount from this
  commit was corrected to Java sentinel `-1` in `aa55e3d`.

Verification for `632492c`:

- `cargo test snecko --all-targets` -> `7 passed`
- `cargo test --all-targets` -> `1222 passed`

`1ad40f2` summary:

- `SnakePlant` Java/Rust behavior was checked.
- No business logic change was needed.
- Added tests for the A17+ `lastMoveBefore(SPORES)` rule versus the lower
  ascension `lastMove(SPORES)` rule.
- Added a queue-order test for three Chompy Chomps damage actions before
  `RollMonsterMove`.

Verification for `1ad40f2`:

- `cargo test snake_plant --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1219 passed`

`8d16e69` summary:

- `Centurion` + `Healer` Java/Rust behavior was checked as a pair because both
  depend on ally state.
- No business logic change was needed.
- Existing Centurion tests already cover zero-HP non-dying ally counting for
  Protect rolls and `GainBlockRandomMonsterAction`.
- Added Healer tests proving Java-style loops count/target zero-HP non-dying
  allies for heal selection and heal execution.

Verification for `8d16e69`:

- `cargo test healer --all-targets` -> `2 passed`
- `cargo test centurion --all-targets` -> `2 passed`
- `cargo test --all-targets` -> `1217 passed`

`a4d74f4` summary:

- `ShelledParasite` Java/Rust timing was checked; no code change was needed.
  Existing tests already cover `firstMove`, STUN writing a FELL move before the
  roll, live truth import, and Plated Armor break triggering STUN.
- `Byrd` Java/Rust timing exposed a real issue: Java Headbutt queues damage but
  synchronously calls `setMove(GO_AIRBORNE)` before queued damage can execute.
- Rust Byrd Headbutt now records the next move before the queued attack, matching
  Java's synchronous `setMove(...)` timing.
- Added a focused Byrd Headbutt timing test.

Verification for `a4d74f4`:

- `cargo test shelled_parasite --all-targets` -> `4 passed`
- `cargo test byrd --all-targets` -> `3 passed`
- `cargo test --all-targets` -> `1215 passed`

`5ad39bc` summary:

- `TorchHead` Java source was checked against Rust.
- No business logic change was needed: Rust already emits one `MonsterAttack`
  followed by queued `SetMonsterMove`, matching Java's `DamageAction` followed
  by `SetMoveAction`.
- Java `update()` only emits `TorchHeadFireEffect` VFX and was not modeled.
- Added a focused parity test to lock that queue order.

Verification for `5ad39bc`:

- `cargo test torch_head --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1214 passed`

`0b0eec3` summary:

- `BanditPointy` Java source was checked against Rust.
- No business logic change was needed: Rust already emits two separate
  `MonsterAttack` actions followed by queued `SetMonsterMove`, matching Java's
  two `DamageAction`s followed by `SetMoveAction`.
- Added a focused parity test to lock that queue order.

Verification for `0b0eec3`:

- `cargo test bandit_pointy --all-targets` -> `1 passed`
- `cargo test --all-targets` -> `1213 passed`

`1ac61f2` summary:

- Gremlin escape turns now preserve Java's queued post-escape
  `SetMoveAction(ESCAPE)` for Fat Gremlin, Gremlin Warrior, Gremlin Thief,
  Gremlin Wizard, and Gremlin Tsundere.
- Gremlin Tsundere Protect now models Java timing: queued
  `GainBlockRandomMonsterAction` is preceded by the synchronous next-move
  update from `setMove(...)`, so the visible next intent changes before the
  queued block action can be interrupted.
- Gremlin Wizard Dope Magic now models Java timing: reset `currentCharge`, then
  record the synchronous next-move update, then execute queued damage.
- Added focused tests for the escape follow-up move and timing-sensitive Wizard
  / Tsundere branches.

Verification for `1ac61f2`:

- `cargo test gremlin --all-targets` -> `34 passed`
- `cargo test --all-targets` -> `1212 passed`

`874605d` summary:

- `Looter` and `Mugger` now distinguish Java synchronous `setMove(...)`
  mutations from queued `SetMoveAction(...)`.
- Looter/Mugger lunge-style attacks place the next Smoke Bomb move update
  before queued steal/damage actions so later queue cleanup cannot erase a Java
  immediate move mutation.
- Looter/Mugger escape turns now include the Java post-escape
  `SetMoveAction(ESCAPE)`.
- `Mugger.die()` burns one `aiRng.random(2)` for Java death voice selection,
  even when there is no stolen gold reward.

Verification for `874605d`:

- `cargo test looter --all-targets` -> `4 passed`
- `cargo test mugger --all-targets` -> `6 passed`
- `cargo test --all-targets` -> `1207 passed`

`d0adc3b` summary:

- `BanditBear.getMove(int)` in Java always sets `BEAR_HUG`; Rust
  `roll_move_plan` now always returns the Bear Hug plan. Maul/Lunge remain a
  `take_turn` `SetMonsterMove` chain.
- `BanditLeader.getMove(int)` in Java always sets `MOCK`; Rust
  `roll_move_plan` now always returns the Mock plan. Attack chain remains in
  `take_turn`.
- `Lagavulin` no longer uses an empty-history special branch as private state.
- `Red Slaver` tests now set explicit runtime fields (`first_turn`,
  `used_entangle`) rather than deriving them from history.
- Audit note updated in
  `docs/audits/MONSTER_RUNTIME_TRUTH_AUDIT_2026-04-18.md`.

Verification for `d0adc3b`:

- `cargo test bandit_bear --all-targets`
- `cargo test bandit_leader --all-targets`
- `cargo test lagavulin --all-targets`
- `cargo test slaver_red --all-targets`
- `cargo test --all-targets` -> `1202 passed`

## Current Audit Position

We are in monster/runtime parity work after broad card parity work.

The current monster architecture is still usable if these rules are followed:

- Java private gameplay fields become explicit Rust runtime fields, protocol
  imports, or factory-seeded state. They are not reconstructed from history.
- Java `lastMove`, `lastTwoMoves`, `lastMoveBefore` map to Rust
  `move_history`.
- Java `takeTurn()` chains that queue `SetMoveAction` become Rust queued
  `SetMonsterMove`, not `roll_move_plan`.
- Java `RollMoveAction` after a turn consumes monster AI RNG and records a move
  when Java does so, even if the next move is deterministic.
- UI/VFX classes are ignored only after checking that they do not mutate combat
  state, RNG, room state, map state, or visible choices.

Current text scans after `1ad40f2`:

- `src/content/monsters` has no remaining direct `move_history().is_empty`
  private-state pattern from the recent search.
- The obvious "private flags from history" smell was cleaned in the audited
  Red Slaver/Lagavulin/Bandit cases.

No uncommitted code changes were present after `012e056` before this handoff
update.

## Recent Source Findings Not Yet Needing Edits

Mixed `SetMoveAction` / `RollMoveAction` audit:

- `SlimeBoss`: Java split path does not queue `RollMoveAction`; Rust split path
  does not roll.
- `AcidSlime_L`: Java split path does not queue `RollMoveAction`; Rust guards
  roll with `if plan.move_id != SPLIT`.
- `SpikeSlime_L`: Java queues `RollMoveAction` after the switch, including the
  split path; Rust always pushes the post-turn roll after `execute_steps`.
- `Looter` / `Mugger`: fixed in `874605d`. Java contains both synchronous
  `setMove(...)` branches and queued `SetMoveAction(...)` branches; Rust now
  preserves the meaningful timing split for lunge/smoke/escape paths.
- Gremlin packet: fixed in `1ac61f2`. Java Gremlin escape paths queue
  `SetMoveAction(ESCAPE)` after `EscapeAction`; Rust now mirrors that for the
  audited Exordium Gremlins. Timing-sensitive synchronous `setMove(...)`
  branches in Gremlin Wizard and Gremlin Tsundere were preserved before queued
  actions.
- `BanditPointy`: checked in `0b0eec3`. No logic change needed; added a test
  for the two-hit damage queue before queued `SetMoveAction`.
- `TorchHead`: checked in `5ad39bc`. No logic change needed; added a test for
  damage before queued `SetMoveAction`; Java fire effect update is VFX-only.
- `ShelledParasite`: checked before `a4d74f4`; no code change needed. Existing
  tests cover first-move runtime state, STUN + roll timing, state import, and
  Plated Armor break.
- `Byrd`: fixed in `a4d74f4`. Headbutt now applies synchronous Java
  `setMove(GO_AIRBORNE)` timing before queued damage.
- `Centurion` + `Healer`: checked in `8d16e69`. No business logic change
  needed; added Healer tests for zero-HP non-dying ally inclusion.
- `SnakePlant`: checked in `1ad40f2`. No business logic change needed; added
  A17 `lastMoveBefore` and triple-hit queue tests.
- `Snecko`: fixed across `632492c` and `aa55e3d`. Glare now emits Confusion
  with Java sentinel amount `-1`, and tests lock Glare, A17 Tail debuff
  ordering, and the `lastTwoMoves(BITE)` Tail rule.
- `SphericGuardian`: fixed in `aa55e3d`. Pre-battle Barricade now uses Java
  sentinel amount `-1`; tests lock Barricade, Artifact, and opening block order.
- `BookOfStabbing`: fixed in `dc4622d`. Pre-battle Painful Stabs now uses Java
  sentinel amount `-1`; tests lock `stabCount` roll-time growth and STAB
  multi-hit execution.
- `Chosen`: checked in `0b984ca`. No business logic change was needed; tests
  lock below-A17 Hex transition, Drain/Debilitate ordering, and Poke two-hit
  execution.
- `Taskmaster`: checked in `f511731`. No business logic change was needed;
  tests lock constant Scouring Whip roll, wound thresholds, A18 Strength
  ordering, and below-A18 no-Strength behavior.
- `GremlinLeader`: fixed in `6e9a4d6` and corrected in `bf619c7`.
  Pre-battle Minion and spawned Minion applications now use Java sentinel
  `-1`; the pre-battle source is now the minion itself, matching Java
  `ApplyPowerAction(m, m, new MinionPower(this))`. Tests lock Encourage queue
  order, STAB three-hit scheduling, and slot-truth behavior.
- `Reptomancer`: shared Minion sentinel parity was touched in `6e9a4d6`, shared
  death/suicide interactions were fixed in `fcf0f0b`, and dedicated move/slot
  behavior was checked in `bf619c7`. The pre-battle Minion source now matches
  Java, dagger slot initialization is locked, A18 double-spawn order is locked,
  Java `canSpawn()` non-dying counting is locked, and Snake Strike
  damage/damage/Weak/roll order is locked.
- `TheCollector` + `TorchHead`: checked in `5232ea9`. No business logic change
  was needed; tests lock initial spawn, Mega Debuff forcing, Fireball
  lastTwoMoves gate, debuff queue order, and existing enemy-slot revive truth.
- `BronzeAutomaton` + `BronzeOrb`: fixed in `8385df0`. `ApplyStasisAction`
  rarity candidate selection now sorts by Java `cardID` before RNG; tests lock
  Automaton runtime counters, Hyper Beam timing, BronzeOrb usedStasis, and
  Support/Beam history gates.
- `Champ`: checked in `a8e467e`. No business logic change was needed; tests
  lock half-HP Anger, Execute gating, fourth-turn Taunt reset, A19 Defensive
  Stance cap/forge counter, Anger cleanup queue order, and Face Slap/Taunt
  debuff order.
- `AwakenedOne`: fixed in `30c73bb`. First-phase death now follows Java
  `AwakenedOne.damage()` ownership instead of pretending `UnawakenedPower`
  owns the transition; tests lock sentinel amount, immediate half-dead/runtime
  mutation, power clearing, top-queued card queue clear, and duplicate
  `REBIRTH` move-history from immediate `setMove` plus queued `SetMoveAction`.
- `Darkling`: fixed in `c7a3546`. Half-death now follows Java
  `Darkling.damage()` ordering: half-dead before power/relic death hooks,
  powers clear after relic hooks, COUNT immediate `setMove` only when
  `nextMove != COUNT`, queued `SetMoveAction(COUNT)` duplicate history, and
  `REINCARNATE` queues heal, revive, Regrow stackAmount `1`, spawn relic hooks,
  then roll.
- `Reptomancer` + `SnakeDagger`: fixed in `fcf0f0b` as part of the shared Java
  `SuicideAction` packet. `SuicideAction(true)` now reaches monster
  death hooks; split slimes use `false`; Fading/Explosive and minion cleanup
  use `true`; Reptomancer/Collector/Bronze cleanup follows Java `addToTop`
  reverse mechanical order.
- `Nemesis`: checked in `98ee287`. No business logic change was needed; tests
  lock Tri Attack, Tri Burn, post-turn Intangible application/skip, and
  existing private `firstMove` / `scytheCooldown` behavior.
- `GiantHead`: checked in `9e6e73f`. No business logic change was needed; tests
  lock A18 pre-battle count decrement, SlowPower amount 0, lastTwoMove gates,
  private count floor, and `IT_IS_TIME` damage.
- `TimeEater`: checked in `6c142a3`. No business logic change was needed; tests
  lock Haste private state, recursive reroll RNG consumption, A19 move queues,
  and execution-time Haste healing.
- `Donu` + `Deca`: checked in `2aae03b`. No business logic change was needed;
  tests lock Artifact amount gates, private `isAttacking`, Beam damage/add-card
  ordering, and all-monster buff/protect loop ordering.
- `Transient`: fixed in `d6a62f4`. Runtime count / next-move mutation now
  happens before queued damage, and Shifting uses Java sentinel amount `-1`.
- `Maw`: fixed in `bcbd851`. Roar private `roared` update now happens before
  queued Weak/Frail actions, and tests lock turn-count and move-history gates.
- `Spiker`: checked in `17d05fd`. No business logic change was needed; tests
  lock pre-battle Thorns gates, private `thornsCount`, low-roll/lastMove gates,
  attack damage, and Buff ordering.
- `SpireGrowth`: checked in `9ce0e12`. No business logic change was needed;
  tests lock player Constricted context, A17 branch priority, low-roll Quick
  Tackle, Smash fallback, and Constricted/Smash execution queues.
- `WrithingMass`: fixed in `87044fb`. `Reactive` now uses Java sentinel amount
  `-1`, duplicate Reactive follows default `stackPower(-1)`, and
  `ReactivePower.onAttacked` queues `RollMonsterMove` to the back like Java
  `addToBot`. Existing runtime truth tests lock `firstMove`,
  `usedMegaDebuff`, recursive reroll gating, and Mega-Debuff Parasite ordering.
- `OrbWalker`: checked in `945681d`. No business logic change was needed; tests
  lock GenericStrengthUp A17 gate, lastTwoMoves gates, Laser damage/Burn/roll
  order, and use of the shared `MakeTempCardInDiscardAndDeck` action.
- `Repulsor`: checked in `a8e2118`. No business logic change was needed; tests
  lock low-roll/lastMove Attack gating, A2+ attack damage, and Dazed random
  draw-pile insertion action before roll.
- `Exploder`: fixed in `5aa6309`. Java `takeTurn()` synchronously increments
  private `turnCount` before queued damage or the empty UNKNOWN/BLOCK body and
  before `RollMoveAction`; Rust now emits the runtime update first in both
  attack and block turns. Tests also lock pre-battle Explosive amount 3 and the
  existing Explosive countdown suicide/damage ordering.
- `SpawnMonsterAction` / `PhilosopherStone`: fixed in `24e4618`. Java
  `SpawnMonsterAction.update()` calls relic `onSpawnMonster(m)` before
  `m.init()`, `m.applyPowers()`, and `addMonster(...)`, and
  `PhilosopherStone.onSpawnMonster` directly mutates `monster.addPower(...)`
  instead of queuing `ApplyPowerAction`. Rust now applies the spawn relic hook
  as immediate state mutation before insertion and before the spawned monster's
  first roll. `Darkling` reincarnate now uses the same direct hook semantics.
- `SpireShield` + `SpireSpear`: fixed/locked in `5fe09ea`. Shield
  pre-battle `Surrounded` now uses Java sentinel amount `-1`; existing
  `BackAttack` power now multiplies monster damage in the shared damage
  pipeline; tests lock Shield Bash Focus/Strength RNG timing, Fortify/Piercer
  all-monster loops without HP filtering, Spear Burn Strike queue order, Spear
  Artifact A18 gate, and runtime Skewer hit count.
- `CorruptHeart`: fixed/locked in `1879996`. Pre-battle Invincible /
  Beat of Death A19 gates are covered; first roll clears only `firstMove`;
  buff turns now emit the private `buffCount` runtime update before queued
  Strength/follow-up `ApplyPower` actions; tests lock negative-Strength cleanse
  and the `buffCount == 1` Beat of Death follow-up. Existing Invincible tests
  already cover Java `maxAmt` reset storage and ordinary/HP_LOSS damage caps.
- Monster factory constructor RNG: corrected in `3d4805e`. Java
  `setHp(int)` still calls `Random.random(hp, hp)`, so fixed-HP constructors
  consume one monster HP RNG roll. Java `bottomHumanoid()` / `bottomWildlife()`
  also construct unselected candidate monsters before selecting one; Rust now
  preserves those discarded candidate HP RNG and louse bite RNG consumptions.
- MonsterGroup pre-battle RNG stream: fixed in `894274a`. Rust group-level
  pre-battle now passes `MonsterHp` so Louse Curl Up consumes the same
  `AbstractDungeon.monsterHpRng` stream as Java. Java universal pre-battle
  Daily/Endless/blight hooks remain intentionally unmodeled.
- Monster `duringTurn()` lifecycle: fixed in `556788e`. Java
  `GameActionManager` calls `m.applyTurnPowers()` immediately after each
  monster `takeTurn()`, before the next monster is dequeued. Only Java
  `FadingPower` and `ExplosivePower` override `duringTurn()`, so Rust now
  handles those in a dedicated per-monster hook instead of the group-level
  `applyEndOfTurnPowers()` pass.
- Victory settlement basically-dead predicate: fixed in `4fd646b`. Rust
  `settle_victory_if_ready` now uses Java `MonsterGroup.areMonstersBasicallyDead()`
  semantics (`isDying || isEscaping`) instead of inferring victory from
  `current_hp <= 0`.
- Monster pre-turn `Invincible` / Poison timing: fixed in `10997a8`. Rust now
  resets `Invincible` in the Java start-of-turn power pass and routes monster
  `PoisonLoseHp` through the HP_LOSS damage pipeline so `Invincible` caps it
  before Poison decrements.
- Monster group end-of-turn / end-of-round queue timing: fixed in `ea1570c`.
  Java queues collective end-of-turn and round-end actions, then runs the
  following player start-of-turn hook methods and constructs `DrawCardAction`
  before the queued cleanup actions execute. Rust now preserves that queue
  order; `DrawReduction` expiration is the locked regression case.
- Regular new-turn post-draw hook action order: fixed in `012e056`. Rust's
  synthetic `PostDrawTrigger` now runs before the queued `DrawCards` so hook
  actions append behind the turn-start draw but ahead of draw-generated actions.
  `VoidCard.triggerWhenDrawn()` now uses bottom insertion like Java
  `addToBot(new LoseEnergyAction(1))`.
- Initial combat hook queue construction: fixed in `3fda120`. Java
  `AbstractRoom.update()` builds the whole opening queue before draining
  actions; Rust now runs battle-start, first-turn relic, initial post-draw
  relic, card, power, and orb hook methods synchronously after queuing the
  opening draw. Initial post-draw power hooks are intentionally not called.

Source suspicion remaining after `5fe09ea`:

- Java automatic BackAttack application/removal is tied to `Surrounded`,
  `player.flipHorizontal`, `drawX`, and `AbstractMonster.applyBackAttack()`.
  Rust does not currently model player facing/drawX. Do not fake this inside
  Shield/Spear content; if needed, design a dedicated facing/BackAttack state
  packet and keep the multiplier behavior separate from automatic power
  creation.

Source suspicion resolved in `24e4618`:

- Java `SpawnMonsterAction.update()` calls relic `onSpawnMonster(m)` before the
  monster is inserted into `AbstractDungeon.getMonsters().monsters`. Rust now
  runs the modeled on-spawn relic hook before insertion. The only currently
  modeled on-spawn relic is Philosopher's Stone, and its hook is direct
  `addPower`-style mutation.

Split / victory timing:

- Java split uses `CannotLoseAction`, `SuicideAction`, `SpawnMonsterAction`,
  then `CanLoseAction`.
- Rust drains the action queue and settles victory only after pending actions
  drain, so the checked Slime split paths do not need UI/global CannotLose
  modeling just for premature reward prevention.

Random target audit:

- `src/engine/targeting.rs` has tests for manual target filtering and random
  target behavior.
- Random monster targeting includes zero-HP monsters when they are not dying,
  escaped, or half-dead, matching Java `MonsterGroup.getRandomMonster(true)`.
- `GainBlockRandomMonsterAction` is special: Java excludes source, `intent ==
  ESCAPE`, and `isDying`, but does not exclude `isEscaping`; Rust has dedicated
  tests for this behavior.
- Naming caveat: Rust `is_escaped` currently represents Java
  `isEscaping || escaped`. In normal Java escape flow this is usually safe
  because `escape()` sets `isEscaping = true` before `escaped = true`, but the
  lifecycle mapping should remain on the watch list.

## High-Risk Evergreen List

Keep these on the short list and revisit with narrow source packets:

1. Draw pile API and top/bottom conventions.
2. Generated cards entering draw/discard/hand, including random spot behavior.
3. Random target selection and monster lifecycle flags.
4. Pending choices, selection order, cancel/confirm behavior, and replay.
5. Post-combat cleanup and retained queued actions.
6. Card instance copying, UUID/misc propagation, and battle-instance mutation.
7. Potion discard/use affordances outside combat and during phase boundaries.
8. Map/boss/event/shop/chest/campfire visibility and room transition state.
9. Relic counters, relic hooks, and hidden vs public state.
10. Monster pools, event pools, and act/floor/ascension gates.
11. Java synchronous `setMove(...)` vs queued `SetMoveAction(...)`; do not
    collapse these when queued damage, death, or cleanup can intervene.
12. UI-tied but gameplay-relevant facing state, especially Act 4
    `Surrounded` / `BackAttack` creation and removal.

## Next Work Queue

Continue Java-source-backed mechanics audit before jumping back to machine
learning. The current narrow lane is permanent run-state mutation parity:
master-deck upgrade/remove/obtain order, delayed `ShowCardAndObtainEffect`
semantics, and relic/card hook ordering.

Recommended next packets:

0. Immediate next packet:
   - The named `ShowCardAndObtainEffect` event-order candidates in this lane
     are now checked. Next, either run a narrow source search for remaining
     Java event `ShowCardAndObtainEffect` / `effectsQueue` / `topLevelEffectsQueue`
     sites not already documented here, or move to the next highest-risk
     mechanics surface from the watch list below.
   - If continuing this lane, do not assume constructor order and actual obtain
     order are the same. Check whether a branch constructs a card obtain effect
     before or after immediate gameplay mutation such as relic obtain, gold
     gain, HP/max HP mutation, deck removal, or transform.
   - Already checked in the current lane:
     - `ForgottenAltar`: no Rust change needed.
     - `GoldenIdolEvent`: no Rust change needed.
     - `Designer`, `Transmogrifier`, `DrugDealer`: generic multi-card
       transform now removes/transforms all selected cards before deferred
       replacement obtains, except the existing Java-backed Neow special case.
     - `GoldShrine`, `Sssserpent`, and `WindingHalls`: no business change
       needed; delayed obtain ordering is now locked by CeramicFish event-order
       regressions in `81789e4`.
     - `Mushrooms` and `GremlinWheelGame`: no business change needed; delayed
       obtain hook ordering is locked by CeramicFish regressions in `56bec7f`.
     - `Ghosts` and `KnowingSkull`: no business change needed; HP/max HP cost
       before delayed obtain hooks is locked by CeramicFish regressions in
       `525fe0b`.
     - `Nest` and `Vampires`: no business change needed; damage/max HP/deck
       removal before delayed obtain hooks is locked by CeramicFish regressions
       in `3426913`.
   - Remaining obvious Java event `ShowCardAndObtainEffect` sites not yet
     summarized in this handoff lane include at least `Duplicator`, `TheLibrary`,
     `LivingWall`, and `GremlinMatchGame`. Some already have strong local
     tests; source-check before adding more.
   - Use the established pattern from `BigFish`, `Addict`, `Mausoleum`, and
     `AccursedBlacksmith`: if Java constructs the card obtain effect before a
     later immediate mutation, take an Omamori snapshot at construction time
     but resolve card obtain after the immediate mutation when gameplay hooks
     would see it.

1. Finish the mixed `SetMoveAction` / `RollMoveAction` monster sweep:
   - `AwakenedOne` was fixed in `30c73bb`.
   - `Darkling` was fixed in `c7a3546`.
   - `Looter` and `Mugger` were fixed in `874605d`.
   - Exordium Gremlins were fixed in `1ac61f2`.
   - `BanditPointy` was checked in `0b0eec3`.
   - `TorchHead` was checked in `5ad39bc`.
   - `ShelledParasite` was checked; no code change needed.
   - `Byrd` was fixed in `a4d74f4`.
   - `Centurion` + `Healer` were checked in `8d16e69`.
   - `SnakePlant` was checked in `1ad40f2`.
   - `Snecko` was fixed across `632492c` and `aa55e3d`.
   - `SphericGuardian` was fixed in `aa55e3d`.
   - `BookOfStabbing` was fixed in `dc4622d`.
   - `Chosen` was checked in `0b984ca`.
   - `Taskmaster` was checked in `f511731`.
   - `GremlinLeader` was fixed in `6e9a4d6`.
   - `TheCollector` was checked in `5232ea9`.
   - `BronzeAutomaton` + `BronzeOrb` were fixed in `8385df0`.
   - `Champ` was checked in `a8e467e`.
   - `AwakenedOne` was fixed in `30c73bb`.
   - `Darkling` was fixed in `c7a3546`.
   - `Reptomancer` + `SnakeDagger` shared death/suicide interactions were fixed
     in `fcf0f0b`.
   - `Nemesis` was checked in `98ee287`.
   - `GiantHead` was checked in `9e6e73f`.
   - `TimeEater` was checked in `6c142a3`.
   - `Donu` + `Deca` were checked in `2aae03b`.
   - `Transient` was fixed in `d6a62f4`.
   - `Maw` was fixed in `bcbd851`.
   - `Spiker` was checked in `17d05fd`.
   - `SpireGrowth` was checked in `9ce0e12`.
   - `WrithingMass` was fixed in `87044fb`.
   - `OrbWalker` was checked in `945681d`.
   - `Repulsor` was checked in `a8e2118`.
   - `Exploder` was fixed in `5aa6309`.
   - Dedicated `Reptomancer` move/slot behavior was fixed/locked in
     `bf619c7`.
   - Java `SpawnMonsterAction.update()` hook ordering was fixed in `24e4618`.
   - Act 4 `SpireShield` + `SpireSpear` coordinated runtime/move audit was
     fixed/locked in `5fe09ea`.
   - `CorruptHeart` runtime/power audit was fixed/locked in `1879996`.
   - Final-act encounter/factory initialization audit initially reached the
     wrong fixed-HP RNG conclusion in `06e5f9f`; this was corrected in
     `3d4805e`.
   - Java `MonsterHelper` encounter composition and factory constructor RNG
     audit was fixed/locked in `3d4805e`, including discarded candidate
     construction for Exordium Thugs / Wildlife.
   - Java `MonsterGroup.usePreBattleAction()` RNG stream was fixed/locked in
     `894274a`.
   - Java `GameActionManager` per-monster `applyTurnPowers()` timing was
     fixed/locked in `556788e`.
   - Java `MonsterGroup.areMonstersBasicallyDead()` victory readiness was
     fixed/locked in `4fd646b`.
   - Java monster start-of-turn `Invincible` and `PoisonLoseHpAction`
     interaction was fixed/locked in `10997a8`.
   - Java collective end-of-turn / atEndOfRound action queue timing was
     fixed/locked in `ea1570c`, with `DrawReductionPower` as the regression
     case.
   - Regular new-turn `atStartOfTurnPostDraw` hook action order and
     `VoidCard.triggerWhenDrawn()` insertion were fixed/locked in `012e056`.
   - Java initial combat hook queue construction around
     `AbstractRoom.update()` was fixed/locked in `3fda120`.
   - Java initial `GainEnergyAndEnableControlsAction` vs Rust first-turn energy
     initialization was checked in `c0fbef0`; no code change was needed.
   - Act 4 `Surrounded` / `BackAttack` facing semantics were fixed/locked in
     `d245435`.
   - Velvet Choker public counter hooks were fixed in `c0fbef0`.
   - Orange Pellets turn reset was fixed in `227a871`.
   - Java synchronous counter mutation for Kunai, Shuriken, Letter Opener,
     Ornamental Fan, Orange Pellets, and Inserter was fixed in `52fb5c8`.
   - Orichalcum `trigger` was source-checked: Java source only clears it and no
     normal source path sets it true, so no Rust state was added in this packet.
   - `Damaru`, `EmotionChip`, `HoveringKite`, `RunicCapacitor`,
     `AncientTeaSet`, and `ArtOfWar` were source-checked after `d245435`; no
     code change was needed.
   - This monster/relic hook list remains important, but it is not the current
     immediate packet while the event/master-deck mutation sweep is active.
2. For each monster packet, inspect only:
   - Java monster file.
   - Rust monster file.
   - Relevant action files if `takeTurn()` queues custom actions.
   - Existing test file or nearest module tests.
3. If source comparison is resolved, add or adjust a focused test, run the
   narrow tests, then commit.
4. If a source comparison exposes an architectural issue, write the issue here
   first before changing broad modules.

## Compression Control Protocol

Every meaningful chunk must end with:

- Latest commit hash or `uncommitted` status.
- Files changed.
- Tests run and result.
- Exact next source packet.
- Any unresolved suspicion moved into this file.

If context compacts, do not infer from memory. Resume from this file and the
latest five commits.
