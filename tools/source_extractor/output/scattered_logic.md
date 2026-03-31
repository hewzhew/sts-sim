# StS Scattered Logic Index

**This file is critical.** Many relic/power behaviors are not in their own classes,
but scattered across engine code via `hasRelic()`/`hasPower()` checks.
When porting to Rust, translating the relic/power class alone is insufficient —
the corresponding engine-side checks must also be implemented.

Format: For each entity checked by the engine, all check sites (file, line, context) are listed.

## RELIC Scattered Logic (84 relics checked by engine)

### Akabeko (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `unlock\relics\watcher\AkabekoUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Akabeko");`

### Art of War **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `unlock\relics\silent\ArtOfWarUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Art of War");`

### Black Star (own hooks: onEnterRoom, onVictory, makeCopy)

Referenced in 1 locations:

- `rooms\MonsterRoomElite.java` L82: `if (AbstractDungeon.player.hasRelic("Black Star")) {`

### Blood Vial (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `events\city\Vampires.java` L43: `this.hasVial = AbstractDungeon.player.hasRelic("Blood Vial");`

### Bloody Idol (own hooks: makeCopy)

Referenced in 2 locations:

- `events\city\ForgottenAltar.java` L102: `if (AbstractDungeon.player.hasRelic("Bloody Idol")) {`
- `events\city\ForgottenAltar.java` L107: `AbstractRelic bloodyIdol = RelicLibrary.getRelic("Bloody Idol").makeCopy();`

### Blue Candle (own hooks: makeCopy, onUseCard)

Referenced in 2 locations:

- `cards\AbstractCard.java` L905: `if (this.type == CardType.CURSE && this.costForTurn < -1 && !AbstractDungeon.player.hasRelic("Blue Candle")) {`
- `unlock\relics\ironclad\BlueCandleUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Blue Candle");`

### Bottled Flame (own hooks: onEquip, onUnequip, atBattleStart, makeCopy)

Referenced in 11 locations:

- `cards\CardGroup.java` L586: `tmp = RelicLibrary.getRelic("Bottled Flame");`
- `cards\CardGroup.java` L632: `tmp = RelicLibrary.getRelic("Bottled Flame");`
- `cards\CardGroup.java` L686: `tmp = RelicLibrary.getRelic("Bottled Flame");`
- `cards\CardGroup.java` L733: `tmp = RelicLibrary.getRelic("Bottled Flame");`
- `characters\AbstractPlayer.java` L2089: `if (c.inBottleFlame && this.hasRelic("Bottled Flame")) {`
- `characters\AbstractPlayer.java` L2090: `((BottledFlame)this.getRelic("Bottled Flame")).setDescriptionAfterLoading();`
- `core\CardCrawlGame.java` L948: `((BottledFlame)AbstractDungeon.player.getRelic("Bottled Flame")).setDescriptionAfterLoading();`
- `saveAndContinue\SaveAndContinue.java` L207: `if (AbstractDungeon.player.hasRelic("Bottled Flame")) {`
- `saveAndContinue\SaveFile.java` L315: `this.bottled_flame = AbstractDungeon.player.hasRelic("Bottled Flame") ? (((BottledFlame)AbstractDungeon.player.getRelic((String)"Bottled Flame")).card != null ? ((BottledFlame)AbstractDungeon.player.g...`
- `screens\MasterDeckViewScreen.java` L334: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Flame");`
- `screens\select\GridCardSelectScreen.java` L646: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Flame");`

### Bottled Lightning (own hooks: onEquip, onUnequip, atBattleStart, makeCopy)

Referenced in 11 locations:

- `cards\CardGroup.java` L599: `tmp = RelicLibrary.getRelic("Bottled Lightning");`
- `cards\CardGroup.java` L645: `tmp = RelicLibrary.getRelic("Bottled Lightning");`
- `cards\CardGroup.java` L699: `tmp = RelicLibrary.getRelic("Bottled Lightning");`
- `cards\CardGroup.java` L746: `tmp = RelicLibrary.getRelic("Bottled Lightning");`
- `characters\AbstractPlayer.java` L2092: `if (c.inBottleLightning && this.hasRelic("Bottled Lightning")) {`
- `characters\AbstractPlayer.java` L2093: `((BottledLightning)this.getRelic("Bottled Lightning")).setDescriptionAfterLoading();`
- `core\CardCrawlGame.java` L963: `((BottledLightning)AbstractDungeon.player.getRelic("Bottled Lightning")).setDescriptionAfterLoading();`
- `saveAndContinue\SaveAndContinue.java` L212: `if (AbstractDungeon.player.hasRelic("Bottled Lightning")) {`
- `saveAndContinue\SaveFile.java` L316: `this.bottled_lightning = AbstractDungeon.player.hasRelic("Bottled Lightning") ? (((BottledLightning)AbstractDungeon.player.getRelic((String)"Bottled Lightning")).card != null ? ((BottledLightning)Abst...`
- `screens\MasterDeckViewScreen.java` L345: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Lightning");`
- `screens\select\GridCardSelectScreen.java` L657: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Lightning");`

### Bottled Tornado (own hooks: onEquip, onUnequip, atBattleStart, makeCopy)

Referenced in 11 locations:

- `cards\CardGroup.java` L612: `tmp = RelicLibrary.getRelic("Bottled Tornado");`
- `cards\CardGroup.java` L658: `tmp = RelicLibrary.getRelic("Bottled Tornado");`
- `cards\CardGroup.java` L712: `tmp = RelicLibrary.getRelic("Bottled Tornado");`
- `cards\CardGroup.java` L759: `tmp = RelicLibrary.getRelic("Bottled Tornado");`
- `characters\AbstractPlayer.java` L2095: `if (c.inBottleTornado && this.hasRelic("Bottled Tornado")) {`
- `characters\AbstractPlayer.java` L2096: `((BottledTornado)this.getRelic("Bottled Tornado")).setDescriptionAfterLoading();`
- `core\CardCrawlGame.java` L978: `((BottledTornado)AbstractDungeon.player.getRelic("Bottled Tornado")).setDescriptionAfterLoading();`
- `saveAndContinue\SaveAndContinue.java` L217: `if (AbstractDungeon.player.hasRelic("Bottled Tornado")) {`
- `saveAndContinue\SaveFile.java` L317: `this.bottled_tornado = AbstractDungeon.player.hasRelic("Bottled Tornado") ? (((BottledTornado)AbstractDungeon.player.getRelic((String)"Bottled Tornado")).card != null ? ((BottledTornado)AbstractDungeo...`
- `screens\MasterDeckViewScreen.java` L356: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Tornado");`
- `screens\select\GridCardSelectScreen.java` L668: `AbstractRelic tmp = RelicLibrary.getRelic("Bottled Tornado");`

### Burning Blood (own hooks: onVictory, makeCopy)

Referenced in 1 locations:

- `relics\BlackBlood.java` L36: `return AbstractDungeon.player.hasRelic("Burning Blood");`

### Busted Crown (own hooks: updateDescription, onEquip, onUnequip, makeCopy)

Referenced in 2 locations:

- `rewards\RewardItem.java` L298: `if (AbstractDungeon.player.hasRelic("Busted Crown")) {`
- `rewards\RewardItem.java` L299: `AbstractDungeon.player.getRelic("Busted Crown").flash();`

### Cables **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 5 locations:

- `actions\defect\DarkImpulseAction.java` L27: `if (AbstractDungeon.player.hasRelic("Cables") && !(AbstractDungeon.player.orbs.get(0) instanceof EmptyOrbSlot) && AbstractDungeon.player.orbs.get(0) instanceof Dark) {`
- `actions\defect\ImpulseAction.java` L25: `if (AbstractDungeon.player.hasRelic("Cables") && !(AbstractDungeon.player.orbs.get(0) instanceof EmptyOrbSlot)) {`
- `actions\defect\TriggerEndOfTurnOrbsAction.java` L19: `if (AbstractDungeon.player.hasRelic("Cables") && !(AbstractDungeon.player.orbs.get(0) instanceof EmptyOrbSlot)) {`
- `characters\AbstractPlayer.java` L2257: `if (this.hasRelic("Cables") && !(this.orbs.get(0) instanceof EmptyOrbSlot)) {`
- `unlock\relics\defect\CablesUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Cables");`

### Calipers (own hooks: makeCopy)

Referenced in 1 locations:

- `actions\GameActionManager.java` L343: `if (!AbstractDungeon.player.hasRelic("Calipers")) {`

### CeramicFish (own hooks: use, onObtainCard, makeCopy)

Referenced in 1 locations:

- `unlock\relics\watcher\CeramicFishUnlock.java` L13: `this.relic = RelicLibrary.getRelic("CeramicFish");`

### Champion Belt **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 3 locations:

- `actions\common\ApplyPowerAction.java` L105: `if (AbstractDungeon.player.hasRelic("Champion Belt") && this.source != null && this.source.isPlayer && this.target != this.source && this.powerToApply.ID.equals("Vulnerable") && !this.target.hasPower(...`
- `actions\common\ApplyPowerAction.java` L106: `AbstractDungeon.player.getRelic("Champion Belt").onTrigger(this.target);`
- `monsters\city\Champ.java` L151: `if (AbstractDungeon.player.hasRelic("Champion Belt")) {`

### Chemical X (own hooks: makeCopy)

Referenced in 24 locations:

- `actions\defect\ReinforcedBodyAction.java` L35: `if (this.p.hasRelic("Chemical X")) {`
- `actions\defect\ReinforcedBodyAction.java` L37: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\DoppelgangerAction.java` L36: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\DoppelgangerAction.java` L38: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\MalaiseAction.java` L39: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\MalaiseAction.java` L41: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\MulticastAction.java` L38: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\MulticastAction.java` L40: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\SkewerAction.java` L41: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\SkewerAction.java` L43: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\TempestAction.java` L35: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\TempestAction.java` L37: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\TransmutationAction.java` L36: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\TransmutationAction.java` L38: `this.p.getRelic("Chemical X").flash();`
- `actions\unique\WhirlwindAction.java` L41: `if (this.p.hasRelic("Chemical X")) {`
- `actions\unique\WhirlwindAction.java` L43: `this.p.getRelic("Chemical X").flash();`
- `actions\watcher\BrillianceAction.java` L34: `if (this.p.hasRelic("Chemical X")) {`
- `actions\watcher\BrillianceAction.java` L36: `this.p.getRelic("Chemical X").flash();`
- `actions\watcher\CollectAction.java` L35: `if (this.p.hasRelic("Chemical X")) {`
- `actions\watcher\CollectAction.java` L37: `this.p.getRelic("Chemical X").flash();`
- `actions\watcher\ConjureBladeAction.java` L34: `if (this.p.hasRelic("Chemical X")) {`
- `actions\watcher\ConjureBladeAction.java` L36: `this.p.getRelic("Chemical X").flash();`
- `actions\watcher\DivinePunishmentAction.java` L35: `if (p.hasRelic("Chemical X")) {`
- `actions\watcher\DivinePunishmentAction.java` L37: `p.getRelic("Chemical X").flash();`

### Circlet (own hooks: onEquip, onUnequip, makeCopy)

Referenced in 11 locations:

- `events\city\CursedTome.java` L147: `possibleBooks.add(RelicLibrary.getRelic("Circlet").makeCopy());`
- `events\city\ForgottenAltar.java` L103: `AbstractDungeon.getCurrRoom().spawnRelicAndObtain(Settings.WIDTH / 2, Settings.HEIGHT / 2, RelicLibrary.getRelic("Circlet").makeCopy());`
- `events\exordium\GoldenIdolEvent.java` L73: `this.relicMetric = AbstractDungeon.player.hasRelic(ID) ? RelicLibrary.getRelic("Circlet").makeCopy() : RelicLibrary.getRelic(ID).makeCopy();`
- `relics\AbstractRelic.java` L208: `if (this.relicId.equals("Circlet") && p != null && p.hasRelic("Circlet")) {`
- `relics\AbstractRelic.java` L209: `AbstractRelic circ = p.getRelic("Circlet");`
- `relics\AbstractRelic.java` L241: `if (this.relicId == "Circlet" && AbstractDungeon.player.hasRelic("Circlet")) {`
- `relics\AbstractRelic.java` L242: `AbstractRelic circ = AbstractDungeon.player.getRelic("Circlet");`
- `relics\AbstractRelic.java` L266: `if (this.relicId == "Circlet" && AbstractDungeon.player.hasRelic("Circlet")) {`
- `relics\AbstractRelic.java` L267: `AbstractRelic circ = AbstractDungeon.player.getRelic("Circlet");`
- `rooms\AbstractRoom.java` L504: `if (relic.relicId == "Circlet" && AbstractDungeon.player.hasRelic("Circlet")) {`
- `rooms\AbstractRoom.java` L505: `AbstractRelic circ = AbstractDungeon.player.getRelic("Circlet");`

### CloakClasp (own hooks: onPlayerEndTurn, makeCopy)

Referenced in 1 locations:

- `unlock\relics\watcher\CloakClaspUnlock.java` L13: `this.relic = RelicLibrary.getRelic("CloakClasp");`

### Cracked Core (own hooks: atPreBattle, makeCopy)

Referenced in 1 locations:

- `relics\FrozenCore.java` L33: `return AbstractDungeon.player.hasRelic("Cracked Core");`

### CultistMask (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `events\shrines\FaceTrader.java` L93: `if (!AbstractDungeon.player.hasRelic("CultistMask")) {`

### Dark Core **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `characters\AbstractPlayer.java` L2181: `if (this.hasRelic("Dark Core") && !(orbToSet instanceof Dark)) {`

### DataDisk (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `unlock\relics\defect\DataDiskUnlock.java` L13: `this.relic = RelicLibrary.getRelic("DataDisk");`

### Dead Branch (own hooks: onExhaust, makeCopy)

Referenced in 1 locations:

- `unlock\relics\ironclad\DeadBranchUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Dead Branch");`

### Dream Catcher (own hooks: makeCopy)

Referenced in 2 locations:

- `vfx\campfire\CampfireSleepEffect.java` L68: `if (AbstractDungeon.player.hasRelic("Dream Catcher")) {`
- `vfx\campfire\CampfireSleepEffect.java` L69: `AbstractDungeon.player.getRelic("Dream Catcher").flash();`

### Du-Vu Doll **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `unlock\relics\silent\DuvuDollUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Du-Vu Doll");`

### Ectoplasm (own hooks: updateDescription, onEquip, onUnequip, makeCopy)

Referenced in 2 locations:

- `characters\AbstractPlayer.java` L703: `if (this.hasRelic("Ectoplasm")) {`
- `characters\AbstractPlayer.java` L704: `this.getRelic("Ectoplasm").flash();`

### Emotion Chip (own hooks: wasHPLost, onVictory, makeCopy)

Referenced in 1 locations:

- `unlock\relics\defect\EmotionChipUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Emotion Chip");`

### Enchiridion (own hooks: atPreBattle, makeCopy)

Referenced in 2 locations:

- `events\city\CursedTome.java` L140: `if (!AbstractDungeon.player.hasRelic("Enchiridion")) {`
- `events\city\CursedTome.java` L141: `possibleBooks.add(RelicLibrary.getRelic("Enchiridion").makeCopy());`

### FaceOfCleric (own hooks: onVictory, makeCopy)

Referenced in 1 locations:

- `events\shrines\FaceTrader.java` L96: `if (!AbstractDungeon.player.hasRelic("FaceOfCleric")) {`

### Frozen Eye (own hooks: makeCopy)

Referenced in 5 locations:

- `screens\DrawPileViewScreen.java` L226: `if (!AbstractDungeon.player.hasRelic("Frozen Eye")) {`
- `screens\DrawPileViewScreen.java` L265: `if (!AbstractDungeon.player.hasRelic("Frozen Eye")) {`
- `screens\ExhaustPileViewScreen.java` L218: `if (!AbstractDungeon.player.hasRelic("Frozen Eye")) {`
- `ui\panels\DrawPilePanel.java` L176: `if (!AbstractDungeon.player.hasRelic("Frozen Eye")) {`
- `ui\panels\DrawPilePanel.java` L181: `} else if (!AbstractDungeon.player.hasRelic("Frozen Eye")) {`

### Ginger (own hooks: makeCopy)

Referenced in 2 locations:

- `actions\common\ApplyPowerAction.java` L113: `if (AbstractDungeon.player.hasRelic("Ginger") && this.target.isPlayer && this.powerToApply.ID.equals("Weakened")) {`
- `actions\common\ApplyPowerAction.java` L114: `AbstractDungeon.player.getRelic("Ginger").flash();`

### Girya (own hooks: atBattleStart, makeCopy)

Referenced in 2 locations:

- `vfx\campfire\CampfireLiftEffect.java` L38: `if (AbstractDungeon.player.hasRelic("Girya")) {`
- `vfx\campfire\CampfireLiftEffect.java` L39: `AbstractDungeon.player.getRelic("Girya").flash();`

### Golden Idol (own hooks: makeCopy)

Referenced in 8 locations:

- `dungeons\AbstractDungeon.java` L1941: `if (!player.hasRelic("Golden Idol") && (float)AbstractDungeon.player.currentHealth / (float)AbstractDungeon.player.maxHealth > 0.5f) continue block16;`
- `events\beyond\MoaiHead.java` L33: `if (AbstractDungeon.player.hasRelic("Golden Idol")) {`
- `events\beyond\MoaiHead.java` L34: `this.imageEventText.setDialogOption(OPTIONS[2], !AbstractDungeon.player.hasRelic("Golden Idol"));`
- `events\beyond\MoaiHead.java` L36: `this.imageEventText.setDialogOption(OPTIONS[3], !AbstractDungeon.player.hasRelic("Golden Idol"));`
- `events\city\ForgottenAltar.java` L41: `if (AbstractDungeon.player.hasRelic("Golden Idol")) {`
- `events\city\ForgottenAltar.java` L42: `this.imageEventText.setDialogOption(OPTIONS[0], !AbstractDungeon.player.hasRelic("Golden Idol"), (AbstractRelic)new BloodyIdol());`
- `events\city\ForgottenAltar.java` L44: `this.imageEventText.setDialogOption(OPTIONS[1], !AbstractDungeon.player.hasRelic("Golden Idol"), (AbstractRelic)new BloodyIdol());`
- `rewards\RewardItem.java` L107: `if (AbstractDungeon.player.hasRelic("Golden Idol")) {`

### GoldenEye (own hooks: makeCopy)

Referenced in 2 locations:

- `actions\utility\ScryAction.java` L23: `if (AbstractDungeon.player.hasRelic("GoldenEye")) {`
- `actions\utility\ScryAction.java` L24: `AbstractDungeon.player.getRelic("GoldenEye").flash();`

### GremlinMask (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `events\shrines\FaceTrader.java` L99: `if (!AbstractDungeon.player.hasRelic("GremlinMask")) {`

### Ice Cream (own hooks: makeCopy)

Referenced in 3 locations:

- `core\EnergyManager.java` L26: `if (AbstractDungeon.player.hasRelic("Ice Cream")) {`
- `core\EnergyManager.java` L28: `AbstractDungeon.player.getRelic("Ice Cream").flash();`
- `core\EnergyManager.java` L29: `AbstractDungeon.actionManager.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, AbstractDungeon.player.getRelic("Ice Cream")));`

### Juzu Bracelet (own hooks: makeCopy)

Referenced in 4 locations:

- `helpers\EventHelper.java` L150: `if (AbstractDungeon.player.hasRelic("Juzu Bracelet")) {`
- `helpers\EventHelper.java` L151: `AbstractDungeon.player.getRelic("Juzu Bracelet").flash();`
- `helpers\EventHelper.java` L161: `if (AbstractDungeon.player.hasRelic("Juzu Bracelet")) {`
- `helpers\EventHelper.java` L162: `AbstractDungeon.player.getRelic("Juzu Bracelet").flash();`

### Lizard Tail (own hooks: onTrigger, makeCopy)

Referenced in 2 locations:

- `characters\AbstractPlayer.java` L1476: `} else if (this.hasRelic("Lizard Tail") && ((LizardTail)this.getRelic((String)"Lizard Tail")).counter == -1) {`
- `characters\AbstractPlayer.java` L1478: `this.getRelic("Lizard Tail").onTrigger();`

### Mark of the Bloom **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `characters\AbstractPlayer.java` L1466: `if (!this.hasRelic("Mark of the Bloom")) {`
- `events\beyond\MindBloom.java` L95: `AbstractDungeon.getCurrRoom().spawnRelicAndObtain((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f, RelicLibrary.getRelic("Mark of the Bloom").makeCopy());`

### Meat on the Bone **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `rooms\AbstractRoom.java` L420: `if (AbstractDungeon.player.hasRelic("Meat on the Bone")) {`
- `rooms\AbstractRoom.java` L421: `AbstractDungeon.player.getRelic("Meat on the Bone").onTrigger();`

### Medical Kit (own hooks: makeCopy, onUseCard)

Referenced in 1 locations:

- `cards\AbstractCard.java` L902: `if (this.type == CardType.STATUS && this.costForTurn < -1 && !AbstractDungeon.player.hasRelic("Medical Kit")) {`

### Membership Card (own hooks: onEnterRoom, makeCopy)

Referenced in 6 locations:

- `shop\ShopScreen.java` L218: `if (AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L270: `} else if (AbstractDungeon.player.hasRelic("The Courier") && AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L274: `} else if (AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L379: `if (AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L393: `if (AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L653: `if (AbstractDungeon.player.hasRelic("Membership Card")) {`

### MutagenicStrength (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `events\city\DrugDealer.java` L68: `if (!AbstractDungeon.player.hasRelic("MutagenicStrength")) {`

### Necronomicon (own hooks: onEquip, onUnequip, onUseCard, makeCopy)

Referenced in 12 locations:

- `cards\AbstractCard.java` L924: `if (AbstractDungeon.player.hasRelic("Necronomicon")) {`
- `cards\AbstractCard.java` L925: `if (this.cost >= 2 && this.type == CardType.ATTACK && AbstractDungeon.player.getRelic("Necronomicon").checkTrigger()) {`
- `cards\AbstractCard.java` L926: `AbstractDungeon.player.getRelic("Necronomicon").beginLongPulse();`
- `cards\AbstractCard.java` L928: `AbstractDungeon.player.getRelic("Necronomicon").stopPulse();`
- `cards\curses\Necronomicurse.java` L31: `if (AbstractDungeon.player.hasRelic("Necronomicon")) {`
- `cards\curses\Necronomicurse.java` L32: `AbstractDungeon.player.getRelic("Necronomicon").flash();`
- `cards\curses\Necronomicurse.java` L39: `if (AbstractDungeon.player.hasRelic("Necronomicon")) {`
- `cards\curses\Necronomicurse.java` L40: `AbstractDungeon.player.getRelic("Necronomicon").flash();`
- `characters\AbstractPlayer.java` L827: `if (!this.isDraggingCard && this.hasRelic("Necronomicon")) {`
- `characters\AbstractPlayer.java` L828: `this.getRelic("Necronomicon").stopPulse();`
- `events\city\CursedTome.java` L137: `if (!AbstractDungeon.player.hasRelic("Necronomicon")) {`
- `events\city\CursedTome.java` L138: `possibleBooks.add(RelicLibrary.getRelic("Necronomicon").makeCopy());`

### Nilry's Codex **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `events\city\CursedTome.java` L143: `if (!AbstractDungeon.player.hasRelic("Nilry's Codex")) {`
- `events\city\CursedTome.java` L144: `possibleBooks.add(RelicLibrary.getRelic("Nilry's Codex").makeCopy());`

### Nloth's Gift **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `events\shrines\Nloth.java` L61: `if (AbstractDungeon.player.hasRelic("Nloth's Gift")) {`
- `events\shrines\Nloth.java` L77: `if (AbstractDungeon.player.hasRelic("Nloth's Gift")) {`

### NlothsMask (own hooks: makeCopy)

Referenced in 1 locations:

- `events\shrines\FaceTrader.java` L102: `if (!AbstractDungeon.player.hasRelic("NlothsMask")) {`

### Odd Mushroom (own hooks: makeCopy)

Referenced in 4 locations:

- `events\exordium\Mushrooms.java` L79: `if (AbstractDungeon.player.hasRelic("Odd Mushroom")) {`
- `powers\VulnerablePower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\VulnerablePower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\VulnerablePower.java` L60: `if (this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom")) {`

### Omamori (own hooks: use, makeCopy)

Referenced in 5 locations:

- `unlock\relics\ironclad\OmamoriUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Omamori");`
- `vfx\FastCardObtainEffect.java` L21: `if (card.color == AbstractCard.CardColor.CURSE && AbstractDungeon.player.hasRelic("Omamori") && AbstractDungeon.player.getRelic((String)"Omamori").counter != 0) {`
- `vfx\FastCardObtainEffect.java` L22: `((Omamori)AbstractDungeon.player.getRelic("Omamori")).use();`
- `vfx\cardManip\ShowCardAndObtainEffect.java` L28: `if (card.color == AbstractCard.CardColor.CURSE && AbstractDungeon.player.hasRelic("Omamori") && AbstractDungeon.player.getRelic((String)"Omamori").counter != 0) {`
- `vfx\cardManip\ShowCardAndObtainEffect.java` L29: `((Omamori)AbstractDungeon.player.getRelic("Omamori")).use();`

### Pandora's Box **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `unlock\relics\silent\PandorasBoxUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Pandora's Box");`

### Paper Crane (own hooks: makeCopy)

Referenced in 3 locations:

- `powers\WeakPower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane") ? DESCRIPTIONS[0] + 40 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\WeakPower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane") ? DESCRIPTIONS[0] + 40 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\WeakPower.java` L60: `if (!this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane")) {`

### Paper Frog (own hooks: makeCopy)

Referenced in 3 locations:

- `powers\VulnerablePower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\VulnerablePower.java` L54: `this.description = this.amount == 1 ? (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTION...`
- `powers\VulnerablePower.java` L63: `if (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Frog")) {`

### Philosopher's Stone **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `actions\common\ReviveMonsterAction.java` L22: `if (AbstractDungeon.player.hasRelic("Philosopher's Stone")) {`
- `actions\unique\SpawnDaggerAction.java` L45: `if (AbstractDungeon.player.hasRelic("Philosopher's Stone")) {`

### Prayer Wheel (own hooks: makeCopy)

Referenced in 2 locations:

- `screens\CombatRewardScreen.java` L83: `if (AbstractDungeon.getCurrRoom() instanceof MonsterRoom && AbstractDungeon.player.hasRelic("Prayer Wheel") && !(AbstractDungeon.getCurrRoom() instanceof MonsterRoomElite) && !(AbstractDungeon.getCurr...`
- `unlock\relics\ironclad\PrayerWheelUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Prayer Wheel");`

### PreservedInsect (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `core\AbstractCreature.java` L360: `if (AbstractDungeon.player.hasRelic("PreservedInsect") && !this.isPlayer && AbstractDungeon.getCurrRoom().eliteTrigger) {`

### PrismaticShard (own hooks: makeCopy, onEquip)

Referenced in 1 locations:

- `dungeons\AbstractDungeon.java` L1435: `card = player.hasRelic("PrismaticShard") ? CardLibrary.getAnyColorCard(rarity) : AbstractDungeon.getCard(rarity);`

### PureWater (own hooks: atBattleStartPreDraw, makeCopy)

Referenced in 1 locations:

- `relics\HolyWater.java` L34: `return AbstractDungeon.player.hasRelic("PureWater");`

### Question Card (own hooks: makeCopy)

Referenced in 2 locations:

- `rewards\RewardItem.java` L295: `if (AbstractDungeon.player.hasRelic("Question Card")) {`
- `rewards\RewardItem.java` L296: `AbstractDungeon.player.getRelic("Question Card").flash();`

### Red Mask (own hooks: atBattleStart, makeCopy)

Referenced in 3 locations:

- `events\beyond\TombRedMask.java` L29: `if (AbstractDungeon.player.hasRelic("Red Mask")) {`
- `events\beyond\TombRedMask.java` L47: `} else if (buttonPressed == 1 && !AbstractDungeon.player.hasRelic("Red Mask")) {`
- `events\city\MaskedBandits.java` L70: `if (AbstractDungeon.player.hasRelic("Red Mask")) {`

### Regal Pillow (own hooks: makeCopy)

Referenced in 5 locations:

- `ui\campfire\RestOption.java` L31: `if (AbstractDungeon.player.hasRelic("Regal Pillow")) {`
- `ui\campfire\RestOption.java` L36: `if (AbstractDungeon.player.hasRelic("Regal Pillow")) {`
- `vfx\campfire\CampfireSleepEffect.java` L47: `if (AbstractDungeon.player.hasRelic("Regal Pillow")) {`
- `vfx\campfire\CampfireSleepEffect.java` L59: `if (AbstractDungeon.player.hasRelic("Regal Pillow")) {`
- `vfx\campfire\CampfireSleepEffect.java` L60: `AbstractDungeon.player.getRelic("Regal Pillow").flash();`

### Ring of the Snake **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `relics\RingOfTheSerpent.java` L40: `return AbstractDungeon.player.hasRelic("Ring of the Snake");`

### Runic Capacitor (own hooks: atPreBattle, makeCopy)

Referenced in 1 locations:

- `unlock\relics\defect\RunicCapacitorUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Runic Capacitor");`

### Runic Dome (own hooks: updateDescription, onEquip, onUnequip, makeCopy)

Referenced in 2 locations:

- `monsters\AbstractMonster.java` L244: `if (this.intentAlphaTarget == 1.0f && !AbstractDungeon.player.hasRelic("Runic Dome") && this.intent != Intent.NONE) {`
- `monsters\AbstractMonster.java` L735: `if (!(this.isDying || this.isEscaping || AbstractDungeon.getCurrRoom().phase != AbstractRoom.RoomPhase.COMBAT || AbstractDungeon.player.isDead || AbstractDungeon.player.hasRelic("Runic Dome") || this....`

### Runic Pyramid (own hooks: makeCopy)

Referenced in 2 locations:

- `actions\common\DiscardAtEndOfTurnAction.java` L35: `if (!AbstractDungeon.player.hasRelic("Runic Pyramid") && !AbstractDungeon.player.hasPower("Equilibrium")) {`
- `powers\RetainCardPower.java` L36: `if (isPlayer && !AbstractDungeon.player.hand.isEmpty() && !AbstractDungeon.player.hasRelic("Runic Pyramid") && !AbstractDungeon.player.hasPower("Equilibrium")) {`

### SacredBark (own hooks: onEquip, makeCopy)

Referenced in 6 locations:

- `potions\AbstractPotion.java` L634: `if (AbstractDungeon.player != null && AbstractDungeon.player.hasRelic("SacredBark")) {`
- `potions\AttackPotion.java` L28: `this.description = AbstractDungeon.player == null || !AbstractDungeon.player.hasRelic("SacredBark") ? AttackPotion.potionStrings.DESCRIPTIONS[0] : AttackPotion.potionStrings.DESCRIPTIONS[1];`
- `potions\ColorlessPotion.java` L27: `this.description = AbstractDungeon.player == null || !AbstractDungeon.player.hasRelic("SacredBark") ? ColorlessPotion.potionStrings.DESCRIPTIONS[0] : ColorlessPotion.potionStrings.DESCRIPTIONS[1];`
- `potions\DuplicationPotion.java` L30: `this.description = AbstractDungeon.player == null || !AbstractDungeon.player.hasRelic("SacredBark") ? DuplicationPotion.potionStrings.DESCRIPTIONS[0] : DuplicationPotion.potionStrings.DESCRIPTIONS[1];`
- `potions\PowerPotion.java` L28: `this.description = AbstractDungeon.player == null || !AbstractDungeon.player.hasRelic("SacredBark") ? PowerPotion.potionStrings.DESCRIPTIONS[0] : PowerPotion.potionStrings.DESCRIPTIONS[1];`
- `potions\SkillPotion.java` L28: `this.description = AbstractDungeon.player == null || !AbstractDungeon.player.hasRelic("SacredBark") ? SkillPotion.potionStrings.DESCRIPTIONS[0] : SkillPotion.potionStrings.DESCRIPTIONS[1];`

### Shovel (own hooks: makeCopy)

Referenced in 1 locations:

- `unlock\relics\ironclad\ShovelUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Shovel");`

### Singing Bowl (own hooks: makeCopy)

Referenced in 4 locations:

- `screens\CardRewardScreen.java` L407: `} else if (AbstractDungeon.player.hasRelic("Singing Bowl")) {`
- `screens\CardRewardScreen.java` L439: `if (AbstractDungeon.player.hasRelic("Singing Bowl")) {`
- `ui\buttons\SingingBowlButton.java` L83: `AbstractDungeon.player.getRelic("Singing Bowl").flash();`
- `unlock\relics\ironclad\SingingBowlUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Singing Bowl");`

### SlaversCollar (own hooks: updateDescription, onVictory, makeCopy)

Referenced in 2 locations:

- `characters\AbstractPlayer.java` L1571: `if (AbstractDungeon.player.hasRelic("SlaversCollar")) {`
- `characters\AbstractPlayer.java` L1572: `((SlaversCollar)AbstractDungeon.player.getRelic("SlaversCollar")).beforeEnergyPrep();`

### Smiling Mask (own hooks: onEnterRoom, makeCopy)

Referenced in 5 locations:

- `shop\ShopScreen.java` L221: `if (AbstractDungeon.player.hasRelic("Smiling Mask")) {`
- `shop\ShopScreen.java` L267: `if (AbstractDungeon.player.hasRelic("Smiling Mask")) {`
- `shop\ShopScreen.java` L269: `AbstractDungeon.player.getRelic("Smiling Mask").stopPulse();`
- `shop\ShopScreen.java` L338: `if (AbstractDungeon.player.hasRelic("Smiling Mask")) {`
- `unlock\relics\silent\SmilingMaskUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Smiling Mask");`

### Snake Skull **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 2 locations:

- `actions\common\ApplyPowerAction.java` L38: `if (AbstractDungeon.player.hasRelic("Snake Skull") && source != null && source.isPlayer && target != source && powerToApply.ID.equals("Poison")) {`
- `actions\common\ApplyPowerAction.java` L39: `AbstractDungeon.player.getRelic("Snake Skull").flash();`

### Sozu (own hooks: updateDescription, onEquip, onUnequip, makeCopy)

Referenced in 11 locations:

- `actions\common\ObtainPotionAction.java` L28: `if (AbstractDungeon.player.hasRelic("Sozu")) {`
- `actions\common\ObtainPotionAction.java` L29: `AbstractDungeon.player.getRelic("Sozu").flash();`
- `events\city\KnowingSkull.java` L122: `if (AbstractDungeon.player.hasRelic("Sozu")) {`
- `events\city\KnowingSkull.java` L123: `AbstractDungeon.player.getRelic("Sozu").flash();`
- `potions\EntropicBrew.java` L37: `} else if (AbstractDungeon.player.hasRelic("Sozu")) {`
- `potions\EntropicBrew.java` L38: `AbstractDungeon.player.getRelic("Sozu").flash();`
- `rewards\RewardItem.java` L266: `if (AbstractDungeon.player.hasRelic("Sozu")) {`
- `rewards\RewardItem.java` L267: `AbstractDungeon.player.getRelic("Sozu").flash();`
- `shop\StorePotion.java` L75: `if (AbstractDungeon.player.hasRelic("Sozu")) {`
- `shop\StorePotion.java` L76: `AbstractDungeon.player.getRelic("Sozu").flash();`
- `vfx\ObtainPotionEffect.java` L21: `if (!AbstractDungeon.player.hasRelic("Sozu")) {`

### Spirit Poop (own hooks: makeCopy)

Referenced in 3 locations:

- `events\shrines\Bonfire.java` L116: `if (!AbstractDungeon.player.hasRelic("Spirit Poop")) {`
- `events\shrines\Bonfire.java` L117: `AbstractDungeon.getCurrRoom().spawnRelicAndObtain((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f, RelicLibrary.getRelic("Spirit Poop").makeCopy());`
- `screens\GameOverScreen.java` L330: `if (AbstractDungeon.player.hasRelic("Spirit Poop")) {`

### SsserpentHead (own hooks: onEnterRoom, makeCopy)

Referenced in 1 locations:

- `events\shrines\FaceTrader.java` L105: `if (!AbstractDungeon.player.hasRelic("SsserpentHead")) {`

### Strange Spoon (own hooks: makeCopy)

Referenced in 2 locations:

- `actions\utility\UseCardAction.java` L107: `if (this.exhaustCard && AbstractDungeon.player.hasRelic("Strange Spoon") && this.targetCard.type != AbstractCard.CardType.POWER) {`
- `actions\utility\UseCardAction.java` L112: `AbstractDungeon.player.getRelic("Strange Spoon").flash();`

### StrikeDummy (own hooks: makeCopy)

Referenced in 1 locations:

- `unlock\relics\watcher\StrikeDummyUnlock.java` L13: `this.relic = RelicLibrary.getRelic("StrikeDummy");`

### Symbiotic Virus (own hooks: atPreBattle, makeCopy)

Referenced in 1 locations:

- `unlock\relics\defect\VirusUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Symbiotic Virus");`

### TeardropLocket (own hooks: atBattleStart, makeCopy)

Referenced in 1 locations:

- `unlock\relics\watcher\TeardropUnlock.java` L13: `this.relic = RelicLibrary.getRelic("TeardropLocket");`

### The Courier **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 10 locations:

- `shop\ShopScreen.java` L215: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\ShopScreen.java` L270: `} else if (AbstractDungeon.player.hasRelic("The Courier") && AbstractDungeon.player.hasRelic("Membership Card")) {`
- `shop\ShopScreen.java` L272: `} else if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\ShopScreen.java` L376: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\ShopScreen.java` L390: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\ShopScreen.java` L583: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\ShopScreen.java` L650: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\StorePotion.java` L86: `if (AbstractDungeon.player.hasRelic("The Courier")) {`
- `shop\StoreRelic.java` L105: `if (this.relic.relicId.equals("The Courier") || AbstractDungeon.player.hasRelic("The Courier")) {`
- `unlock\relics\silent\CourierUnlock.java` L13: `this.relic = RelicLibrary.getRelic("The Courier");`

### Tiny Chest (own hooks: onEquip, makeCopy)

Referenced in 3 locations:

- `helpers\EventHelper.java` L99: `if (AbstractDungeon.player.hasRelic("Tiny Chest")) {`
- `helpers\EventHelper.java` L100: `AbstractRelic r = AbstractDungeon.player.getRelic("Tiny Chest");`
- `unlock\relics\silent\TinyChestUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Tiny Chest");`

### Turnip (own hooks: makeCopy)

Referenced in 3 locations:

- `actions\common\ApplyPowerAction.java` L119: `if (AbstractDungeon.player.hasRelic("Turnip") && this.target.isPlayer && this.powerToApply.ID.equals("Frail")) {`
- `actions\common\ApplyPowerAction.java` L120: `AbstractDungeon.player.getRelic("Turnip").flash();`
- `unlock\relics\defect\TurnipUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Turnip");`

### Unceasing Top (own hooks: atPreBattle, makeCopy)

Referenced in 1 locations:

- `actions\GameActionManager.java` L193: `if (this.cardQueue.size() == 1 && this.cardQueue.get((int)0).isEndTurnAutoPlay && (top = AbstractDungeon.player.getRelic("Unceasing Top")) != null) {`

### White Beast Statue **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `rooms\AbstractRoom.java` L596: `if (AbstractDungeon.player.hasRelic("White Beast Statue")) {`

### WingedGreaves **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 4 locations:

- `map\MapRoomNode.java` L103: `if (node.y != edge.dstY || !AbstractDungeon.player.hasRelic("WingedGreaves") || AbstractDungeon.player.getRelic((String)"WingedGreaves").counter <= 0) continue;`
- `map\MapRoomNode.java` L218: `if (!normalConnection && wingedConnection && AbstractDungeon.player.hasRelic("WingedGreaves")) {`
- `map\MapRoomNode.java` L221: `AbstractDungeon.player.getRelic("WingedGreaves").setCounter(-2);`
- `screens\DungeonMapScreen.java` L153: `boolean bl = flightMatters = AbstractDungeon.player.hasRelic("WingedGreaves") || ModHelper.isModEnabled("Flight");`

### Yang **(NO hooks in own class — logic is ENTIRELY engine-side)**

Referenced in 1 locations:

- `unlock\relics\watcher\YangUnlock.java` L13: `this.relic = RelicLibrary.getRelic("Yang");`

## POWER Scattered Logic (30 powers checked by engine)

### Accuracy (own hooks: use, upgrade, makeCopy)

Referenced in 1 locations:

- `cards\tempCards\Shiv.java` L26: `this.baseDamage = AbstractDungeon.player != null && AbstractDungeon.player.hasPower("Accuracy") ? 4 + AbstractDungeon.player.getPower((String)"Accuracy").amount : 4;`

### Artifact **(NO hooks in own class)**

Referenced in 12 locations:

- `actions\common\ApplyPoisonOnRandomMonsterAction.java` L64: `if (this.target.hasPower("Artifact")) {`
- `actions\common\ApplyPoisonOnRandomMonsterAction.java` L68: `this.target.getPower("Artifact").flashWithoutSound();`
- `actions\common\ApplyPoisonOnRandomMonsterAction.java` L69: `this.target.getPower("Artifact").onSpecificTrigger();`
- `actions\common\ApplyPowerAction.java` L105: `if (AbstractDungeon.player.hasRelic("Champion Belt") && this.source != null && this.source.isPlayer && this.target != this.source && this.powerToApply.ID.equals("Vulnerable") && !this.target.hasPower(...`
- `actions\common\ApplyPowerAction.java` L125: `if (this.target.hasPower("Artifact") && this.powerToApply.type == AbstractPower.PowerType.DEBUFF) {`
- `actions\common\ApplyPowerAction.java` L129: `this.target.getPower("Artifact").flashWithoutSound();`
- `actions\common\ApplyPowerAction.java` L130: `this.target.getPower("Artifact").onSpecificTrigger();`
- `cards\colorless\DarkShackles.java` L29: `if (m != null && !m.hasPower("Artifact")) {`
- `cards\deprecated\DEPRECATEDPeace.java` L35: `if (mo.hasPower("Artifact")) continue;`
- `cards\green\PiercingWail.java` L44: `if (mo.hasPower("Artifact")) continue;`
- `powers\SadisticPower.java` L37: `if (power.type == AbstractPower.PowerType.DEBUFF && !power.ID.equals("Shackled") && source == this.owner && target != this.owner && !target.hasPower("Artifact")) {`
- `powers\ShiftingPower.java` L35: `if (!this.owner.hasPower("Artifact")) {`

### BackAttack **(NO hooks in own class)**

Referenced in 6 locations:

- `monsters\AbstractMonster.java` L986: `if (applyBackAttack && !this.hasPower("BackAttack")) {`
- `monsters\AbstractMonster.java` L1006: `if (this.hasPower("BackAttack")) {`
- `monsters\ending\SpireShield.java` L166: `if (!m.hasPower("BackAttack")) continue;`
- `monsters\ending\SpireSpear.java` L173: `if (!m.hasPower("BackAttack")) continue;`
- `potions\SmokeBomb.java` L47: `if (m.hasPower("BackAttack")) {`
- `ui\panels\PotionPopUp.java` L245: `if (!m.hasPower("BackAttack")) continue;`

### Barricade (own hooks: use, upgrade, makeCopy)

Referenced in 2 locations:

- `actions\GameActionManager.java` L342: `if (!AbstractDungeon.player.hasPower("Barricade") && !AbstractDungeon.player.hasPower("Blur")) {`
- `monsters\MonsterGroup.java` L96: `if (!m.hasPower("Barricade")) {`

### Blur (own hooks: use, upgrade, makeCopy)

Referenced in 1 locations:

- `actions\GameActionManager.java` L342: `if (!AbstractDungeon.player.hasPower("Barricade") && !AbstractDungeon.player.hasPower("Blur")) {`

### Buffer (own hooks: use, upgrade, makeCopy)

Referenced in 1 locations:

- `powers\deprecated\DEPRECATEDHotHotPower.java` L38: `if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner && damageAmount > 0 && !this.owner.hasPower("Buffer")) {`

### CannotChangeStancePower (own hooks: atEndOfTurn, updateDescription)

Referenced in 1 locations:

- `actions\watcher\ChangeStanceAction.java` L32: `if (AbstractDungeon.player.hasPower("CannotChangeStancePower")) {`

### Conserve **(NO hooks in own class)**

Referenced in 1 locations:

- `core\EnergyManager.java` L32: `} else if (AbstractDungeon.player.hasPower("Conserve")) {`

### Constricted **(NO hooks in own class)**

Referenced in 2 locations:

- `monsters\beyond\SpireGrowth.java` L96: `if (AbstractDungeon.ascensionLevel >= 17 && !AbstractDungeon.player.hasPower("Constricted") && !this.lastMove((byte)2)) {`
- `monsters\beyond\SpireGrowth.java` L104: `if (!AbstractDungeon.player.hasPower("Constricted") && !this.lastMove((byte)2)) {`

### Corruption (own hooks: use, upgrade, makeCopy)

Referenced in 6 locations:

- `actions\unique\ExhumeAction.java` L53: `if (AbstractDungeon.player.hasPower("Corruption") && c.type == AbstractCard.CardType.SKILL) {`
- `actions\unique\ExhumeAction.java` L90: `if (AbstractDungeon.player.hasPower("Corruption") && c.type == AbstractCard.CardType.SKILL) {`
- `characters\AbstractPlayer.java` L1330: `if (this.hasPower("Corruption")) {`
- `characters\AbstractPlayer.java` L1360: `if (!(c.costForTurn <= 0 || c.freeToPlay() || c.isInAutoplay || this.hasPower("Corruption") && c.type == AbstractCard.CardType.SKILL)) {`
- `vfx\cardManip\ShowCardAndAddToHandEffect.java` L45: `if (AbstractDungeon.player.hasPower("Corruption") && card.type == AbstractCard.CardType.SKILL) {`
- `vfx\cardManip\ShowCardAndAddToHandEffect.java` L67: `if (AbstractDungeon.player.hasPower("Corruption") && card.type == AbstractCard.CardType.SKILL) {`

### Electro **(NO hooks in own class)**

Referenced in 2 locations:

- `orbs\Lightning.java` L59: `if (AbstractDungeon.player.hasPower("Electro")) {`
- `orbs\Lightning.java` L68: `if (AbstractDungeon.player.hasPower("Electro")) {`

### Entangled **(NO hooks in own class)**

Referenced in 1 locations:

- `cards\AbstractCard.java` L857: `if (AbstractDungeon.player.hasPower("Entangled") && this.type == CardType.ATTACK) {`

### Equilibrium (own hooks: use, upgrade, makeCopy)

Referenced in 2 locations:

- `actions\common\DiscardAtEndOfTurnAction.java` L35: `if (!AbstractDungeon.player.hasRelic("Runic Pyramid") && !AbstractDungeon.player.hasPower("Equilibrium")) {`
- `powers\RetainCardPower.java` L36: `if (isPlayer && !AbstractDungeon.player.hand.isEmpty() && !AbstractDungeon.player.hasRelic("Runic Pyramid") && !AbstractDungeon.player.hasPower("Equilibrium")) {`

### Focus **(NO hooks in own class)**

Referenced in 3 locations:

- `dungeons\AbstractDungeon.java` L2636: `if (player.hasPower("Focus")) {`
- `orbs\AbstractOrb.java` L80: `AbstractPower power = AbstractDungeon.player.getPower("Focus");`
- `orbs\Dark.java` L74: `AbstractPower power = AbstractDungeon.player.getPower("Focus");`

### FreeAttackPower (own hooks: updateDescription, onUseCard)

Referenced in 1 locations:

- `cards\AbstractCard.java` L2046: `return AbstractDungeon.player != null && AbstractDungeon.currMapNode != null && AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && AbstractDungeon.player.hasPower("FreeAttackPower...`

### Intangible **(NO hooks in own class)**

Referenced in 3 locations:

- `core\AbstractCreature.java` L918: `if (poisonAmt > 0 && this.hasPower("Intangible")) {`
- `monsters\beyond\Nemesis.java` L106: `if (!this.hasPower("Intangible")) {`
- `monsters\beyond\Nemesis.java` L114: `if (info.output > 0 && this.hasPower("Intangible")) {`

### IntangiblePlayer **(NO hooks in own class)**

Referenced in 2 locations:

- `characters\AbstractPlayer.java` L1378: `if (damageAmount > 1 && this.hasPower("IntangiblePlayer")) {`
- `monsters\AbstractMonster.java` L610: `if (info.output > 0 && this.hasPower("IntangiblePlayer")) {`

### Lockon **(NO hooks in own class)**

Referenced in 2 locations:

- `cards\DamageInfo.java` L138: `if (isOrbDamage && AbstractDungeon.getMonsters().monsters.get(i).hasPower("Lockon")) {`
- `orbs\AbstractOrb.java` L92: `if (target.hasPower("Lockon")) {`

### Mantra **(NO hooks in own class)**

Referenced in 1 locations:

- `powers\watcher\DevotionPower.java` L37: `if (!AbstractDungeon.player.hasPower("Mantra") && this.amount >= 10) {`

### MasterRealityPower (own hooks: updateDescription)

Referenced in 13 locations:

- `actions\common\MakeTempCardAtBottomOfDeckAction.java` L27: `if (c.type != AbstractCard.CardType.CURSE && c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\common\MakeTempCardInDiscardAction.java` L32: `if (!sameUUID && this.c.type != AbstractCard.CardType.CURSE && this.c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\common\MakeTempCardInDrawPileAction.java` L50: `if (c.type != AbstractCard.CardType.CURSE && c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\common\MakeTempCardInDrawPileAction.java` L58: `if (c.type != AbstractCard.CardType.CURSE && c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\common\MakeTempCardInHandAction.java` L28: `if (this.c.type != AbstractCard.CardType.CURSE && this.c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\common\MakeTempCardInHandAction.java` L43: `if (this.c.type != AbstractCard.CardType.CURSE && this.c.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\unique\DiscoveryAction.java` L53: `if (AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `actions\utility\ChooseOneColorless.java` L35: `if (AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `vfx\cardManip\ShowCardAndAddToDiscardEffect.java` L31: `if (this.card.type != AbstractCard.CardType.CURSE && this.card.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `vfx\cardManip\ShowCardAndAddToDiscardEffect.java` L44: `if (card.type != AbstractCard.CardType.CURSE && card.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `vfx\cardManip\ShowCardAndAddToDrawPileEffect.java` L38: `if (this.card.type != AbstractCard.CardType.CURSE && this.card.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `vfx\cardManip\ShowCardAndAddToHandEffect.java` L37: `if (card.type != AbstractCard.CardType.CURSE && card.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`
- `vfx\cardManip\ShowCardAndAddToHandEffect.java` L59: `if (card.type != AbstractCard.CardType.CURSE && card.type != AbstractCard.CardType.STATUS && AbstractDungeon.player.hasPower("MasterRealityPower")) {`

### Minion **(NO hooks in own class)**

Referenced in 4 locations:

- `actions\unique\FeedAction.java` L34: `if (!(!((AbstractMonster)this.target).isDying && this.target.currentHealth > 0 || this.target.halfDead || this.target.hasPower("Minion"))) {`
- `actions\unique\GreedAction.java` L33: `if (!(!((AbstractMonster)this.target).isDying && this.target.currentHealth > 0 || this.target.halfDead || this.target.hasPower("Minion"))) {`
- `actions\unique\RitualDaggerAction.java` L35: `if (!(!this.target.isDying && this.target.currentHealth > 0 || this.target.halfDead || this.target.hasPower("Minion"))) {`
- `actions\watcher\LessonLearnedAction.java` L36: `if (!(!((AbstractMonster)this.target).isDying && this.target.currentHealth > 0 || this.target.halfDead || this.target.hasPower("Minion"))) {`

### Mode Shift **(NO hooks in own class)**

Referenced in 2 locations:

- `monsters\exordium\TheGuardian.java` L274: `if (this.getPower("Mode Shift") != null) {`
- `monsters\exordium\TheGuardian.java` L276: `this.getPower("Mode Shift").updateDescription();`

### No Draw **(NO hooks in own class)**

Referenced in 10 locations:

- `actions\common\DrawCardAction.java` L64: `if (AbstractDungeon.player.hasPower("No Draw")) {`
- `actions\common\DrawCardAction.java` L65: `AbstractDungeon.player.getPower("No Draw").flash();`
- `actions\common\FastDrawCardAction.java` L22: `} else if (AbstractDungeon.player.hasPower("No Draw")) {`
- `actions\common\FastDrawCardAction.java` L23: `AbstractDungeon.player.getPower("No Draw").flash();`
- `actions\defect\ScrapeAction.java` L30: `} else if (AbstractDungeon.player.hasPower("No Draw")) {`
- `actions\defect\ScrapeAction.java` L31: `AbstractDungeon.player.getPower("No Draw").flash();`
- `actions\watcher\PathVictoryAction.java` L16: `if (AbstractDungeon.player.hasPower("No Draw")) {`
- `actions\watcher\PathVictoryAction.java` L17: `AbstractDungeon.player.getPower("No Draw").flash();`
- `powers\EvolvePower.java` L34: `if (card.type == AbstractCard.CardType.STATUS && !this.owner.hasPower("No Draw")) {`
- `relics\UnceasingTop.java` L44: `if (!(!AbstractDungeon.actionManager.actions.isEmpty() || !AbstractDungeon.player.hand.isEmpty() || AbstractDungeon.actionManager.turnHasEnded || !this.canDraw || AbstractDungeon.player.hasPower("No D...`

### Poison **(NO hooks in own class)**

Referenced in 8 locations:

- `actions\unique\BaneAction.java` L36: `if (this.m.hasPower("Poison")) {`
- `actions\unique\CorpseExplosionAction.java` L23: `if (this.target.hasPower("Poison")) {`
- `actions\unique\DoublePoisonAction.java` L22: `if (this.target != null && this.target.hasPower("Poison")) {`
- `actions\unique\PoisonLoseHpAction.java` L49: `if ((p = this.target.getPower("Poison")) != null) {`
- `actions\unique\TriplePoisonAction.java` L22: `if (this.target.hasPower("Poison")) {`
- `core\AbstractCreature.java` L845: `if (this.hasPower("Poison")) {`
- `core\AbstractCreature.java` L910: `if (!this.hasPower("Poison")) {`
- `relics\TheSpecimen.java` L32: `if (m.hasPower("Poison")) {`

### Regeneration **(NO hooks in own class)**

Referenced in 1 locations:

- `actions\unique\RegenAction.java` L36: `if (this.target.isPlayer && (p = this.target.getPower("Regeneration")) != null) {`

### Strength **(NO hooks in own class)**

Referenced in 4 locations:

- `actions\unique\LimitBreakAction.java` L25: `if (this.duration == Settings.ACTION_DUR_XFAST && this.p.hasPower("Strength")) {`
- `cards\red\HeavyBlade.java` L41: `AbstractPower strength = AbstractDungeon.player.getPower("Strength");`
- `cards\red\HeavyBlade.java` L53: `AbstractPower strength = AbstractDungeon.player.getPower("Strength");`
- `monsters\ending\CorruptHeart.java` L116: `if (this.hasPower("Strength") && this.getPower((String)"Strength").amount < 0) {`

### Surrounded **(NO hooks in own class)**

Referenced in 6 locations:

- `cards\CardGroup.java` L187: `if (AbstractDungeon.player.hasPower("Surrounded") && AbstractDungeon.getCurrRoom().monsters != null) {`
- `characters\AbstractPlayer.java` L1273: `if (this.hasPower("Surrounded")) {`
- `monsters\AbstractMonster.java` L1002: `return AbstractDungeon.player.hasPower("Surrounded") && (AbstractDungeon.player.flipHorizontal && AbstractDungeon.player.drawX < this.drawX || !AbstractDungeon.player.flipHorizontal && AbstractDungeon...`
- `monsters\ending\SpireShield.java` L162: `if (AbstractDungeon.player.hasPower("Surrounded")) {`
- `monsters\ending\SpireSpear.java` L169: `if (AbstractDungeon.player.hasPower("Surrounded")) {`
- `ui\panels\PotionPopUp.java` L196: `if (AbstractDungeon.player.hasPower("Surrounded")) {`

### Vigor **(NO hooks in own class)**

Referenced in 1 locations:

- `cards\deprecated\DEPRECATEDCrescentKick.java` L37: `this.glowColor = AbstractDungeon.player.hasPower("Vigor") ? AbstractCard.GOLD_BORDER_GLOW_COLOR.cpy() : AbstractCard.BLUE_BORDER_GLOW_COLOR.cpy();`

### Vulnerable **(NO hooks in own class)**

Referenced in 2 locations:

- `actions\unique\DropkickAction.java` L26: `if (this.target != null && this.target.hasPower("Vulnerable")) {`
- `cards\red\Dropkick.java` L34: `if (m.isDeadOrEscaped() || !m.hasPower("Vulnerable")) continue;`

### Weakened **(NO hooks in own class)**

Referenced in 2 locations:

- `actions\unique\HeelHookAction.java` L26: `if (this.target != null && this.target.hasPower("Weakened")) {`
- `cards\green\HeelHook.java` L34: `if (m.isDeadOrEscaped() || !m.hasPower("Weakened")) continue;`

## OTHER Scattered Logic

### FairyPotion

- `characters\AbstractPlayer.java` L1467: `if (this.hasPotion("FairyPotion")) {`

