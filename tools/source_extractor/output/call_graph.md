# StS Critical Call Graph

Method call sequences in core engine classes.

## AbstractCard
File: `cards\AbstractCard.java`

### isStarterStrike() (L273-275)

**Calls (in order):**
- L274: `this.hasTag(CardTags.STRIKE)`
- L274: `this.rarity.equals((Object)CardRarity.BASIC)`

### isStarterDefend() (L277-279)

**Calls (in order):**
- L278: `this.hasTag(CardTags.STARTER_DEFEND)`
- L278: `this.rarity.equals((Object)CardRarity.BASIC)`

### initialize() (L322-336)

**Calls (in order):**
- L323: `System.currentTimeMillis()`
- L324: `Gdx.files.internal("cards/cards.atlas")`
- L325: `Gdx.files.internal("oldCards/cards.atlas")`
- L326: `Gdx.files.internal("orbs/orb.atlas")`
- L327: `orbAtlas.findRegion("red")`
- L328: `orbAtlas.findRegion("green")`
- L329: `orbAtlas.findRegion("blue")`
- L330: `orbAtlas.findRegion("purple")`
- L331: `orbAtlas.findRegion("card")`
- L332: `orbAtlas.findRegion("potion")`
- L333: `orbAtlas.findRegion("relic")`
- L334: `orbAtlas.findRegion("special")`
- L335: `logger.info("Card Image load time: " + (System.currentTimeMillis() - startTime) + "ms")`
- L335: `System.currentTimeMillis()`

**Objects created:**
- L324: `TextureAtlas`
- L325: `TextureAtlas`
- L326: `TextureAtlas`

### initializeDynamicFrameWidths() (L338-355)

**Calls (in order):**
- L340: `gl.setText(FontHelper.cardTypeFont, AbstractCard.uiStrings.TEXT[0])`
- L343: `gl.setText(FontHelper.cardTypeFont, AbstractCard.uiStrings.TEXT[1])`
- L346: `gl.setText(FontHelper.cardTypeFont, AbstractCard.uiStrings.TEXT[2])`
- L349: `gl.setText(FontHelper.cardTypeFont, AbstractCard.uiStrings.TEXT[3])`
- L352: `gl.setText(FontHelper.cardTypeFont, AbstractCard.uiStrings.TEXT[7])`

### initializeTitle() (L357-368)

**Calls (in order):**
- L358: `FontHelper.cardTitleFont.getData().setScale(1.0f)`
- L358: `FontHelper.cardTitleFont.getData()`
- L359: `gl.setText(FontHelper.cardTitleFont, this.name, Color.WHITE, 0.0f, 1, false)`
- L367: `gl.reset()`

### initializeDescription() (L370-484)

**Calls (in order):**
- L371: `this.keywords.clear()`
- L373: `this.initializeDescriptionCN()`
- L376: `this.description.clear()`
- L378: `sbuilder.setLength(0)`
- L380: `this.rawDescription.split(" ")`
- L386: `sbuilder2.setLength(0)`
- L387: `sbuilder2.append(" ")`
- L388: `word.length()`
- L388: `word.charAt(word.length() - 1)`
- L388: `word.length()`
- L388: `Character.isLetterOrDigit(word.charAt(word.length() - 1))`
- L388: `word.charAt(word.length() - 1)`
- L388: `word.length()`
- L389: `sbuilder2.insert(0, word.charAt(word.length() - 1))`
- L389: `word.charAt(word.length() - 1)`
- L389: `word.length()`
- L390: `word.substring(0, word.length() - 1)`
- L390: `word.length()`
- L392: `word.toLowerCase()`
- L393: `GameDictionary.keywords.containsKey(keywordTmp = this.dedupeKeyword(keywordTmp))`
- L393: `this.dedupeKeyword(keywordTmp)`
- L394: `this.keywords.contains(keywordTmp)`
- L395: `this.keywords.add(keywordTmp)`
- L397: `gl.reset()`
- L398: `gl.setText(FontHelper.cardDescFont_N, sbuilder2)`
- L400: `gl.setText(FontHelper.cardDescFont_N, word)`
- L405: `word.isEmpty()`
- L405: `word.charAt(0)`
- L406: `gl.reset()`
- L407: `gl.setText(FontHelper.cardDescFont_N, sbuilder2)`
- L411: `this.keywords.contains("[R]")`
- L412: `this.keywords.add("[R]")`
- L418: `this.keywords.contains("[G]")`
- L419: `this.keywords.add("[G]")`
- L425: `this.keywords.contains("[B]")`
- L426: `this.keywords.add("[B]")`
- L432: `this.keywords.contains("[W]")`
- L433: `this.keywords.add("[W]")`
- L439: `word.equals("[W]")`
- L439: `this.keywords.contains("[W]")`
- L440: `this.keywords.add("[W]")`
- L446: `logger.info("ERROR: Tried to display an invalid energy type: " + this.color.name())`
- L446: `this.color.name()`
- L452: `word.equals("!D")`
- L452: `word.equals("!B")`
- L452: `word.equals("!M")`
- L453: `gl.setText(FontHelper.cardDescFont_N, word)`
- L454: `word.equals("NL")`
- L457: `this.description.add(new DescriptionLine(sbuilder.toString().trim(), currentWidth))`
- L457: `sbuilder.toString().trim()`
- ... (17 more)

**Objects created:**
- L457: `DescriptionLine`
- L466: `DescriptionLine`
- L479: `DescriptionLine`

### initializeDescriptionCN() (L486-651)

**Calls (in order):**
- L487: `this.description.clear()`
- L489: `sbuilder.setLength(0)`
- L491: `this.rawDescription.split(" ")`
- L492: `word.trim()`
- L493: `word.contains("NL")`
- L494: `word.equals("NL")`
- L494: `sbuilder.length()`
- L494: `word.isEmpty()`
- L498: `word.toLowerCase()`
- L499: `GameDictionary.keywords.containsKey(keywordTmp = this.dedupeKeyword(keywordTmp))`
- L499: `this.dedupeKeyword(keywordTmp)`
- L500: `this.keywords.contains(keywordTmp)`
- L501: `this.keywords.add(keywordTmp)`
- L503: `gl.setText(FontHelper.cardDescFont_N, word)`
- L506: `this.description.add(new DescriptionLine(sbuilder.toString(), currentWidth))`
- L506: `sbuilder.toString()`
- L507: `sbuilder.setLength(0)`
- L509: `sbuilder.append(" *").append(word).append(" ")`
- L509: `sbuilder.append(" *").append(word)`
- L509: `sbuilder.append(" *")`
- L512: `sbuilder.append(" *").append(word).append(" ")`
- L512: `sbuilder.append(" *").append(word)`
- L512: `sbuilder.append(" *")`
- L516: `word.isEmpty()`
- L516: `word.charAt(0)`
- L519: `this.keywords.contains("[R]")`
- L520: `this.keywords.add("[R]")`
- L524: `this.keywords.contains("[G]")`
- L525: `this.keywords.add("[G]")`
- L529: `this.keywords.contains("[B]")`
- L530: `this.keywords.add("[B]")`
- L534: `this.keywords.contains("[W]")`
- L535: `this.keywords.add("[W]")`
- L539: `this.keywords.contains("[W]")`
- L540: `this.keywords.add("[W]")`
- L544: `logger.info("ERROR: Tried to display an invalid energy type: " + this.color.name())`
- L544: `this.color.name()`
- L549: `this.description.add(new DescriptionLine(sbuilder.toString(), currentWidth))`
- L549: `sbuilder.toString()`
- L550: `sbuilder.setLength(0)`
- L552: `sbuilder.append(" ").append(word).append(" ")`
- L552: `sbuilder.append(" ").append(word)`
- L552: `sbuilder.append(" ")`
- L555: `sbuilder.append(" ").append(word).append(" ")`
- L555: `sbuilder.append(" ").append(word)`
- L555: `sbuilder.append(" ")`
- L559: `word.equals("!D!")`
- L562: `this.description.add(new DescriptionLine(sbuilder.toString(), currentWidth))`
- L562: `sbuilder.toString()`
- L563: `sbuilder.setLength(0)`
- ... (58 more)

**Objects created:**
- L506: `DescriptionLine`
- L549: `DescriptionLine`
- L562: `DescriptionLine`
- L575: `DescriptionLine`
- L588: `DescriptionLine`
- L601: `DescriptionLine`
- L611: `DescriptionLine`
- L628: `DescriptionLine`
- L637: `DescriptionLine`

### hasTag(CardTags tagToCheck) (L653-655)

**Calls (in order):**
- L654: `this.tags.contains((Object)tagToCheck)`

### upgradeName() (L703-708)

**Calls (in order):**
- L707: `this.initializeTitle()`

### dedupeKeyword(String keyword) (L722-728)

**Calls (in order):**
- L723: `GameDictionary.parentWord.get(keyword)`

### createCardImage() (L737-802)

**Calls (in order):**
- L740: `CURSE_BG_COLOR.cpy()`
- L741: `CURSE_TYPE_BACK_COLOR.cpy()`
- L742: `CURSE_FRAME_COLOR.cpy()`
- L743: `CURSE_DESC_BOX_COLOR.cpy()`
- L747: `COLORLESS_BG_COLOR.cpy()`
- L748: `COLORLESS_TYPE_BACK_COLOR.cpy()`
- L749: `COLORLESS_FRAME_COLOR.cpy()`
- L750: `Color.WHITE.cpy()`
- L751: `COLORLESS_DESC_BOX_COLOR.cpy()`
- L755: `RED_BG_COLOR.cpy()`
- L756: `RED_TYPE_BACK_COLOR.cpy()`
- L757: `RED_FRAME_COLOR.cpy()`
- L758: `RED_RARE_OUTLINE_COLOR.cpy()`
- L759: `RED_DESC_BOX_COLOR.cpy()`
- L763: `GREEN_BG_COLOR.cpy()`
- L764: `GREEN_TYPE_BACK_COLOR.cpy()`
- L765: `GREEN_FRAME_COLOR.cpy()`
- L766: `GREEN_RARE_OUTLINE_COLOR.cpy()`
- L767: `GREEN_DESC_BOX_COLOR.cpy()`
- L771: `BLUE_BG_COLOR.cpy()`
- L772: `BLUE_TYPE_BACK_COLOR.cpy()`
- L773: `BLUE_FRAME_COLOR.cpy()`
- L774: `BLUE_RARE_OUTLINE_COLOR.cpy()`
- L775: `BLUE_DESC_BOX_COLOR.cpy()`
- L778: `BLUE_BG_COLOR.cpy()`
- L779: `BLUE_TYPE_BACK_COLOR.cpy()`
- L780: `BLUE_FRAME_COLOR.cpy()`
- L781: `BLUE_RARE_OUTLINE_COLOR.cpy()`
- L782: `BLUE_DESC_BOX_COLOR.cpy()`
- L786: `logger.info("ERROR: Card color was NOT set for " + this.name)`
- L790: `BANNER_COLOR_COMMON.cpy()`
- L791: `IMG_FRAME_COLOR_COMMON.cpy()`
- L793: `BANNER_COLOR_UNCOMMON.cpy()`
- L794: `IMG_FRAME_COLOR_UNCOMMON.cpy()`
- L796: `BANNER_COLOR_RARE.cpy()`
- L797: `IMG_FRAME_COLOR_RARE.cpy()`
- L799: `CardHelper.getColor(43, 37, 65)`
- L801: `FRAME_SHADOW_COLOR.cpy()`

### makeSameInstanceOf() (L804-808)

**Calls (in order):**
- L805: `this.makeStatEquivalentCopy()`

### makeStatEquivalentCopy() (L810-834)

**Calls (in order):**
- L811: `this.makeCopy()`
- L813: `card.upgrade()`

### cardPlayable(AbstractMonster m) (L839-845)

**Calls (in order):**
- L840: `AbstractDungeon.getMonsters().areMonstersBasicallyDead()`
- L840: `AbstractDungeon.getMonsters()`

### hasEnoughEnergy() (L847-878)

**Calls (in order):**
- L853: `p.canPlayCard(this)`
- L857: `AbstractDungeon.player.hasPower("Entangled")`
- L862: `r.canPlay(this)`
- L866: `b.canPlay(this)`
- L870: `c.canPlay(this)`
- L873: `this.freeToPlay()`

### canUse(AbstractPlayer p, AbstractMonster m) (L901-909)

**Calls (in order):**
- L902: `AbstractDungeon.player.hasRelic("Medical Kit")`
- L905: `AbstractDungeon.player.hasRelic("Blue Candle")`
- L908: `this.cardPlayable(m)`
- L908: `this.hasEnoughEnergy()`

### update() (L913-951)

**Calls (in order):**
- L914: `this.updateFlashVfx()`
- L916: `Gdx.graphics.getDeltaTime()`
- L922: `MathHelper.cardLerpSnap(this.current_x, this.target_x)`
- L923: `MathHelper.cardLerpSnap(this.current_y, this.target_y)`
- L924: `AbstractDungeon.player.hasRelic("Necronomicon")`
- L925: `AbstractDungeon.player.getRelic("Necronomicon").checkTrigger()`
- L925: `AbstractDungeon.player.getRelic("Necronomicon")`
- L926: `AbstractDungeon.player.getRelic("Necronomicon").beginLongPulse()`
- L926: `AbstractDungeon.player.getRelic("Necronomicon")`
- L928: `AbstractDungeon.player.getRelic("Necronomicon").stopPulse()`
- L928: `AbstractDungeon.player.getRelic("Necronomicon")`
- L933: `MathHelper.cardLerpSnap(this.current_x, this.target_x)`
- L934: `MathHelper.cardLerpSnap(this.current_y, this.target_y)`
- L936: `MathHelper.cardLerpSnap(this.current_x, this.target_x)`
- L937: `MathHelper.cardLerpSnap(this.current_y, this.target_y)`
- L938: `this.hb.move(this.current_x, this.current_y)`
- L939: `this.hb.resize(HB_W * this.drawScale, HB_H * this.drawScale)`
- L941: `MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale * 0.9f)`
- L942: `MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale * 0.9f)`
- L944: `MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale)`
- L947: `MathHelper.angleLerpSnap(this.angle, this.targetAngle)`
- L949: `this.updateTransparency()`
- L950: `this.updateColor()`

### updateFlashVfx() (L953-960)

**Calls (in order):**
- L955: `this.flashVfx.update()`

### updateGlow() (L962-977)

**Calls (in order):**
- L964: `Gdx.graphics.getDeltaTime()`
- L966: `this.glowList.add(new CardGlowBorder(this, this.glowColor))`
- L970: `this.glowList.iterator()`
- L971: `i.hasNext()`
- L972: `i.next()`
- L973: `e.update()`
- L975: `i.remove()`

**Objects created:**
- L966: `CardGlowBorder`

### render(SpriteBatch sb) (L992-996)

**Calls (in order):**
- L994: `this.render(sb, false)`

### renderHoverShadow(SpriteBatch sb) (L998-1002)

**Calls (in order):**
- L1000: `this.renderHelper(sb, Settings.TWO_THIRDS_TRANSPARENT_BLACK_COLOR, ImageMaster.CARD_SUPER_SHADOW, this.current_x, this.c`

### renderInLibrary(SpriteBatch sb) (L1004-1031)

**Calls (in order):**
- L1005: `this.isOnScreen()`
- L1009: `this.makeCopy()`
- L1013: `copy.upgrade()`
- L1014: `copy.displayUpgrades()`
- L1015: `copy.render(sb)`
- L1017: `this.updateGlow()`
- L1018: `this.renderGlow(sb)`
- L1019: `this.renderImage(sb, this.hovered, false)`
- L1020: `this.renderType(sb)`
- L1021: `this.renderTitle(sb)`
- L1023: `this.renderDescriptionCN(sb)`
- L1025: `this.renderDescription(sb)`
- L1027: `this.renderTint(sb)`
- L1028: `this.renderEnergy(sb)`
- L1029: `this.hb.render(sb)`

### render(SpriteBatch sb, boolean selected) (L1033-1041)

**Calls (in order):**
- L1036: `this.flashVfx.render(sb)`
- L1038: `this.renderCard(sb, this.hovered, selected)`
- L1039: `this.hb.render(sb)`

### renderUpgradePreview(SpriteBatch sb) (L1043-1052)

**Calls (in order):**
- L1046: `this.initializeTitle()`
- L1047: `this.renderCard(sb, this.hovered, false)`
- L1049: `this.initializeTitle()`
- L1051: `this.resetAttributes()`

### renderWithSelections(SpriteBatch sb) (L1054-1056)

**Calls (in order):**
- L1055: `this.renderCard(sb, false, true)`

### renderCard(SpriteBatch sb, boolean hovered, boolean selected) (L1058-1081)

**Calls (in order):**
- L1060: `this.isOnScreen()`
- L1064: `this.updateGlow()`
- L1065: `this.renderGlow(sb)`
- L1066: `this.renderImage(sb, hovered, selected)`
- L1067: `this.renderTitle(sb)`
- L1068: `this.renderType(sb)`
- L1070: `this.renderDescriptionCN(sb)`
- L1072: `this.renderDescription(sb)`
- L1074: `this.renderTint(sb)`
- L1075: `this.renderEnergy(sb)`
- L1077: `this.renderBack(sb, hovered, selected)`
- L1078: `this.hb.render(sb)`

### renderTint(SpriteBatch sb) (L1083-1092)

**Calls (in order):**
- L1085: `this.getCardBgAtlas()`
- L1087: `this.renderHelper(sb, this.tintColor, cardBgImg, this.current_x, this.current_y)`
- L1089: `this.renderHelper(sb, this.tintColor, this.getCardBg(), this.current_x - 256.0f, this.current_y - 256.0f)`
- L1089: `this.getCardBg()`

### renderOuterGlow(SpriteBatch sb) (L1094-1099)

**Calls (in order):**
- L1098: `this.renderHelper(sb, AbstractDungeon.player.getCardRenderColor(), this.getCardBgAtlas(), this.current_x, this.current_y`
- L1098: `AbstractDungeon.player.getCardRenderColor()`
- L1098: `this.getCardBgAtlas()`

### getCardBg() (L1101-1107)

**Calls (in order):**
- L1104: `System.out.println("Add special logic here")`

### renderGlow(SpriteBatch sb) (L1126-1134)

**Calls (in order):**
- L1128: `this.renderMainBorder(sb)`
- L1130: `abstractGameEffect.render(sb)`
- L1132: `sb.setBlendFunction(770, 771)`

### renderMainBorder(SpriteBatch sb) (L1147-1171)

**Calls (in order):**
- L1150: `sb.setBlendFunction(770, 1)`
- L1164: `AbstractDungeon.getCurrRoom()`
- L1165: `sb.setColor(this.glowColor)`
- L1167: `sb.setColor(GREEN_BORDER_GLOW_COLOR)`
- L1169: `sb.draw(img, this.current_x + img.offsetX - (float)img.originalWidth / 2.0f, this.current_y + img.offsetY - (float)img.o`

### renderHelper(SpriteBatch sb, Color color, TextureAtlas.AtlasRegion img, float drawX, float drawY) (L1173-1176)

**Calls (in order):**
- L1174: `sb.setColor(color)`
- L1175: `sb.draw(img, drawX + img.offsetX - (float)img.originalWidth / 2.0f, drawY + img.offsetY - (float)img.originalHeight / 2.`

### renderHelper(SpriteBatch sb, Color color, TextureAtlas.AtlasRegion img, float drawX, float drawY, float scale) (L1178-1181)

**Calls (in order):**
- L1179: `sb.setColor(color)`
- L1180: `sb.draw(img, drawX + img.offsetX - (float)img.originalWidth / 2.0f, drawY + img.offsetY - (float)img.originalHeight / 2.`

### renderHelper(SpriteBatch sb, Color color, Texture img, float drawX, float drawY) (L1183-1186)

**Calls (in order):**
- L1184: `sb.setColor(color)`
- L1185: `sb.draw(img, drawX + 256.0f, drawY + 256.0f, 256.0f, 256.0f, 512.0f, 512.0f, this.drawScale * Settings.scale, this.drawS`

### renderHelper(SpriteBatch sb, Color color, Texture img, float drawX, float drawY, float scale) (L1188-1191)

**Calls (in order):**
- L1189: `sb.setColor(color)`
- L1190: `sb.draw(img, drawX, drawY, 256.0f, 256.0f, 512.0f, 512.0f, this.drawScale * Settings.scale * scale, this.drawScale * Set`

### renderSmallEnergy(SpriteBatch sb, TextureAtlas.AtlasRegion region, float x, float y) (L1193-1196)

**Calls (in order):**
- L1194: `sb.setColor(this.renderColor)`
- L1195: `sb.draw(region.getTexture(), this.current_x + x * Settings.scale * this.drawScale + region.offsetX * Settings.scale, thi`
- L1195: `region.getTexture()`
- L1195: `region.getRegionX()`
- L1195: `region.getRegionY()`
- L1195: `region.getRegionWidth()`
- L1195: `region.getRegionHeight()`

### renderImage(SpriteBatch sb, boolean hovered, boolean selected) (L1198-1218)

**Calls (in order):**
- L1201: `this.renderHelper(sb, Color.SKY, this.getCardBgAtlas(), this.current_x, this.current_y, 1.03f)`
- L1201: `this.getCardBgAtlas()`
- L1203: `this.renderHelper(sb, this.frameShadowColor, this.getCardBgAtlas(), this.current_x + SHADOW_OFFSET_X * this.drawScale, t`
- L1203: `this.getCardBgAtlas()`
- L1205: `this.renderHelper(sb, HOVER_IMG_COLOR, this.getCardBgAtlas(), this.current_x, this.current_y)`
- L1205: `this.getCardBgAtlas()`
- L1207: `this.renderHelper(sb, SELECTED_CARD_COLOR, this.getCardBgAtlas(), this.current_x, this.current_y)`
- L1207: `this.getCardBgAtlas()`
- L1210: `this.renderCardBg(sb, this.current_x, this.current_y)`
- L1211: `UnlockTracker.betaCardPref.getBoolean(this.cardID, false)`
- L1212: `this.renderJokePortrait(sb)`
- L1214: `this.renderPortrait(sb)`
- L1216: `this.renderPortraitFrame(sb, this.current_x, this.current_y)`
- L1217: `this.renderBannerImage(sb, this.current_x, this.current_y)`

### renderCardBg(SpriteBatch sb, float x, float y) (L1220-1243)

**Calls (in order):**
- L1223: `this.renderAttackBg(sb, x, y)`
- L1227: `this.renderSkillBg(sb, x, y)`
- L1231: `this.renderPowerBg(sb, x, y)`
- L1235: `this.renderSkillBg(sb, x, y)`
- L1239: `this.renderSkillBg(sb, x, y)`

### renderAttackBg(SpriteBatch sb, float x, float y) (L1245-1275)

**Calls (in order):**
- L1248: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_ATTACK_BG_RED, x, y)`
- L1252: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_ATTACK_BG_GREEN, x, y)`
- L1256: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_ATTACK_BG_BLUE, x, y)`
- L1260: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_ATTACK_BG_PURPLE, x, y)`
- L1264: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`
- L1268: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_ATTACK_BG_GRAY, x, y)`
- L1272: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`

### renderSkillBg(SpriteBatch sb, float x, float y) (L1277-1307)

**Calls (in order):**
- L1280: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_RED, x, y)`
- L1284: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_GREEN, x, y)`
- L1288: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLUE, x, y)`
- L1292: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_PURPLE, x, y)`
- L1296: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`
- L1300: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_GRAY, x, y)`
- L1304: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`

### renderPowerBg(SpriteBatch sb, float x, float y) (L1309-1339)

**Calls (in order):**
- L1312: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_POWER_BG_RED, x, y)`
- L1316: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_POWER_BG_GREEN, x, y)`
- L1320: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_POWER_BG_BLUE, x, y)`
- L1324: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_POWER_BG_PURPLE, x, y)`
- L1328: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`
- L1332: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_POWER_BG_GRAY, x, y)`
- L1336: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_SKILL_BG_BLACK, x, y)`

### renderPortraitFrame(SpriteBatch sb, float x, float y) (L1341-1376)

**Calls (in order):**
- L1346: `this.renderAttackPortrait(sb, x, y)`
- L1352: `this.renderSkillPortrait(sb, x, y)`
- L1358: `this.renderSkillPortrait(sb, x, y)`
- L1364: `this.renderSkillPortrait(sb, x, y)`
- L1370: `this.renderPowerPortrait(sb, x, y)`
- L1375: `this.renderDynamicFrame(sb, x, y, tOffset, tWidth)`

### renderAttackPortrait(SpriteBatch sb, float x, float y) (L1378-1396)

**Calls (in order):**
- L1384: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_ATTACK_COMMON, x, y)`
- L1388: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_ATTACK_UNCOMMON, x, y)`
- L1392: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_ATTACK_RARE, x, y)`

### renderDynamicFrame(SpriteBatch sb, float x, float y, float typeOffset, float typeWidth) (L1398-1424)

**Calls (in order):**
- L1407: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_COMMON_FRAME_MID, x, y, 0.0f, typeWidth)`
- L1408: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_COMMON_FRAME_LEFT, x, y, -typeOffset, 1.0f)`
- L1409: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_COMMON_FRAME_RIGHT, x, y, typeOffset, 1.0f)`
- L1413: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_UNCOMMON_FRAME_MID, x, y, 0.0f, typeWidth)`
- L1414: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_UNCOMMON_FRAME_LEFT, x, y, -typeOffset, 1.0f)`
- L1415: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_UNCOMMON_FRAME_RIGHT, x, y, typeOffset, 1.0f)`
- L1419: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_RARE_FRAME_MID, x, y, 0.0f, typeWidth)`
- L1420: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_RARE_FRAME_LEFT, x, y, -typeOffset, 1.0f)`
- L1421: `this.dynamicFrameRenderHelper(sb, ImageMaster.CARD_RARE_FRAME_RIGHT, x, y, typeOffset, 1.0f)`

### dynamicFrameRenderHelper(SpriteBatch sb, TextureAtlas.AtlasRegion img, float x, float y, float xOffset, float xScale) (L1426-1428)

**Calls (in order):**
- L1427: `sb.draw(img, x + img.offsetX - (float)img.originalWidth / 2.0f + xOffset * this.drawScale, y + img.offsetY - (float)img.`

### dynamicFrameRenderHelper(SpriteBatch sb, Texture img, float x, float y, float xOffset, float xScale) (L1430-1432)

**Calls (in order):**
- L1431: `sb.draw(img, x + xOffset * this.drawScale, y, 256.0f, 256.0f, 512.0f, 512.0f, this.drawScale * Settings.scale * xScale, `

### renderSkillPortrait(SpriteBatch sb, float x, float y) (L1434-1458)

**Calls (in order):**
- L1437: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_COMMON, x, y)`
- L1441: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_COMMON, x, y)`
- L1445: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_UNCOMMON, x, y)`
- L1449: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_RARE, x, y)`
- L1453: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_COMMON, x, y)`
- L1457: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_SKILL_COMMON, x, y)`

### renderPowerPortrait(SpriteBatch sb, float x, float y) (L1460-1477)

**Calls (in order):**
- L1466: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_POWER_COMMON, x, y)`
- L1470: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_POWER_UNCOMMON, x, y)`
- L1474: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_FRAME_POWER_RARE, x, y)`

### renderBannerImage(SpriteBatch sb, float drawX, float drawY) (L1479-1503)

**Calls (in order):**
- L1482: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_COMMON, drawX, drawY)`
- L1486: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_COMMON, drawX, drawY)`
- L1490: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_UNCOMMON, drawX, drawY)`
- L1494: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_RARE, drawX, drawY)`
- L1498: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_COMMON, drawX, drawY)`
- L1502: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BANNER_COMMON, drawX, drawY)`

### renderBack(SpriteBatch sb, boolean hovered, boolean selected) (L1505-1507)

**Calls (in order):**
- L1506: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BACK, this.current_x, this.current_y)`

### renderPortrait(SpriteBatch sb) (L1509-1529)

**Calls (in order):**
- L1520: `sb.setColor(this.renderColor)`
- L1521: `sb.draw(this.portrait, drawX, drawY + 72.0f, (float)this.portrait.packedWidth / 2.0f, (float)this.portrait.packedHeight `
- L1523: `sb.setColor(this.renderColor)`
- L1524: `sb.draw(img, drawX, drawY + 72.0f, 125.0f, 23.0f, 250.0f, 190.0f, this.drawScale * Settings.scale, this.drawScale * Sett`
- L1527: `sb.draw(this.portraitImg, drawX, drawY + 72.0f, 125.0f, 23.0f, 250.0f, 190.0f, this.drawScale * Settings.scale, this.dra`

### renderJokePortrait(SpriteBatch sb) (L1531-1551)

**Calls (in order):**
- L1542: `sb.setColor(this.renderColor)`
- L1543: `sb.draw(this.jokePortrait, drawX, drawY + 72.0f, (float)this.jokePortrait.packedWidth / 2.0f, (float)this.jokePortrait.p`
- L1545: `sb.setColor(this.renderColor)`
- L1546: `sb.draw(img, drawX, drawY + 72.0f, 125.0f, 23.0f, 250.0f, 190.0f, this.drawScale * Settings.scale, this.drawScale * Sett`
- L1549: `sb.draw(this.portraitImg, drawX, drawY + 72.0f, 125.0f, 23.0f, 250.0f, 190.0f, this.drawScale * Settings.scale, this.dra`

### renderDescription(SpriteBatch sb) (L1553-1653)

**Calls (in order):**
- L1555: `FontHelper.menuBannerFont.getData().setScale(this.drawScale * 1.25f)`
- L1555: `FontHelper.menuBannerFont.getData()`
- L1556: `FontHelper.renderRotatedText(sb, FontHelper.menuBannerFont, "? ? ?", this.current_x, this.current_y, 0.0f, -200.0f * Set`
- L1557: `FontHelper.menuBannerFont.getData().setScale(1.0f)`
- L1557: `FontHelper.menuBannerFont.getData()`
- L1560: `this.getDescFont()`
- L1562: `this.description.size()`
- L1562: `font.getCapHeight()`
- L1562: `font.getCapHeight()`
- L1563: `font.getCapHeight()`
- L1564: `this.description.size()`
- L1565: `this.description.get((int)i)`
- L1566: `this.description.get(i).getCachedTokenizedText()`
- L1566: `this.description.get(i)`
- L1567: `tmp.length()`
- L1567: `tmp.charAt(0)`
- L1568: `tmp.substring(1)`
- L1570: `tmp.length()`
- L1570: `tmp.charAt(tmp.length() - 2)`
- L1570: `tmp.length()`
- L1570: `Character.isLetter(tmp.charAt(tmp.length() - 2))`
- L1570: `tmp.charAt(tmp.length() - 2)`
- L1570: `tmp.length()`
- L1571: `tmp.charAt(tmp.length() - 2)`
- L1571: `tmp.length()`
- L1572: `tmp.substring(0, tmp.length() - 2)`
- L1572: `tmp.length()`
- L1575: `gl.setText(font, tmp)`
- L1576: `FontHelper.renderRotatedText(sb, font, tmp, this.current_x, this.current_y, start_x - this.current_x + AbstractCard.gl.w`
- L1576: `font.getCapHeight()`
- L1577: `Math.round(start_x + AbstractCard.gl.width)`
- L1578: `gl.setText(font, punctuation)`
- L1579: `FontHelper.renderRotatedText(sb, font, punctuation, this.current_x, this.current_y, start_x - this.current_x + AbstractC`
- L1579: `font.getCapHeight()`
- L1580: `gl.setText(font, punctuation)`
- L1584: `tmp.length()`
- L1584: `tmp.charAt(0)`
- L1585: `tmp.length()`
- L1586: `this.renderDynamicVariable(tmp.charAt(1), start_x, draw_y, i, font, sb, null)`
- L1586: `tmp.charAt(1)`
- L1589: `tmp.length()`
- L1590: `this.renderDynamicVariable(tmp.charAt(1), start_x, draw_y, i, font, sb, Character.valueOf(tmp.charAt(3)))`
- L1590: `tmp.charAt(1)`
- L1590: `Character.valueOf(tmp.charAt(3))`
- L1590: `tmp.charAt(3)`
- L1593: `tmp.equals("[R] ")`
- L1595: `this.renderSmallEnergy(sb, orb_red, (start_x - this.current_x) / Settings.scale / this.drawScale, -100.0f - (((float)thi`
- L1595: `this.description.size()`
- L1599: `tmp.equals("[R]. ")`
- L1601: `this.renderSmallEnergy(sb, orb_red, (start_x - this.current_x) / Settings.scale / this.drawScale, -100.0f - (((float)thi`
- ... (33 more)

### getDynamicValue(char key) (L1655-1687)

**Calls (in order):**
- L1660: `Integer.toString(this.block)`
- L1662: `Integer.toString(this.block)`
- L1664: `Integer.toString(this.baseBlock)`
- L1669: `Integer.toString(this.damage)`
- L1671: `Integer.toString(this.damage)`
- L1673: `Integer.toString(this.baseDamage)`
- L1678: `Integer.toString(this.magicNumber)`
- L1680: `Integer.toString(this.magicNumber)`
- L1682: `Integer.toString(this.baseMagicNumber)`
- L1685: `logger.info("KEY: " + key)`
- L1686: `Integer.toString(-99)`

### renderDescriptionCN(SpriteBatch sb) (L1689-1773)

**Calls (in order):**
- L1691: `FontHelper.menuBannerFont.getData().setScale(this.drawScale * 1.25f)`
- L1691: `FontHelper.menuBannerFont.getData()`
- L1692: `FontHelper.renderRotatedText(sb, FontHelper.menuBannerFont, "? ? ?", this.current_x, this.current_y, 0.0f, -200.0f * Set`
- L1693: `FontHelper.menuBannerFont.getData().setScale(1.0f)`
- L1693: `FontHelper.menuBannerFont.getData()`
- L1696: `this.getDescFont()`
- L1698: `this.description.size()`
- L1698: `font.getCapHeight()`
- L1698: `font.getCapHeight()`
- L1699: `font.getCapHeight()`
- L1700: `this.description.size()`
- L1702: `this.description.get((int)i)`
- L1703: `this.description.get(i).getCachedTokenizedTextCN()`
- L1703: `this.description.get(i)`
- L1706: `tmp.length()`
- L1707: `tmp.charAt(j)`
- L1707: `tmp.charAt(j)`
- L1707: `tmp.contains("[B]")`
- L1707: `tmp.charAt(j)`
- L1708: `tmp.substring(0, j)`
- L1709: `this.getDynamicValue(tmp.charAt(j))`
- L1709: `tmp.charAt(j)`
- L1710: `tmp.substring(j + 1)`
- L1716: `tmp.length()`
- L1717: `tmp.charAt(j)`
- L1717: `tmp.charAt(j)`
- L1717: `tmp.contains("[B]")`
- L1717: `tmp.charAt(j)`
- L1718: `tmp.substring(0, j)`
- L1719: `this.getDynamicValue(tmp.charAt(j))`
- L1719: `tmp.charAt(j)`
- L1720: `tmp.substring(j + 1)`
- L1726: `tmp.length()`
- L1726: `tmp.charAt(0)`
- L1727: `tmp.substring(1)`
- L1729: `tmp.length()`
- L1729: `tmp.charAt(tmp.length() - 2)`
- L1729: `tmp.length()`
- L1729: `Character.isLetter(tmp.charAt(tmp.length() - 2))`
- L1729: `tmp.charAt(tmp.length() - 2)`
- L1729: `tmp.length()`
- L1730: `tmp.charAt(tmp.length() - 2)`
- L1730: `tmp.length()`
- L1731: `tmp.substring(0, tmp.length() - 2)`
- L1731: `tmp.length()`
- L1734: `gl.setText(font, tmp)`
- L1735: `FontHelper.renderRotatedText(sb, font, tmp, this.current_x, this.current_y, start_x - this.current_x + AbstractCard.gl.w`
- L1735: `font.getCapHeight()`
- L1736: `Math.round(start_x + AbstractCard.gl.width)`
- L1737: `gl.setText(font, punctuation)`
- ... (20 more)

### renderDynamicVariable(char key, float start_x, float draw_y, int i, BitmapFont font, SpriteBatch sb, Character end) (L1775-1833)

**Calls (in order):**
- L1776: `sbuilder.setLength(0)`
- L1823: `sbuilder.append(Integer.toString(num))`
- L1823: `Integer.toString(num)`
- L1824: `gl.setText(font, sbuilder)`
- L1825: `FontHelper.renderRotatedText(sb, font, sbuilder.toString(), this.current_x, this.current_y, start_x - this.current_x + A`
- L1825: `sbuilder.toString()`
- L1825: `font.getCapHeight()`
- L1827: `FontHelper.renderRotatedText(sb, font, Character.toString(end.charValue()), this.current_x, this.current_y, start_x - th`
- L1827: `Character.toString(end.charValue())`
- L1827: `end.charValue()`
- L1827: `font.getCapHeight()`
- L1828: `sbuilder.append(end)`
- L1830: `sbuilder.append(' ')`
- L1831: `gl.setText(font, sbuilder)`

### getDescFont() (L1835-1840)

**Calls (in order):**
- L1838: `font.getData().setScale(this.drawScale)`
- L1838: `font.getData()`

### renderTitle(SpriteBatch sb) (L1842-1865)

**Calls (in order):**
- L1844: `FontHelper.cardTitleFont.getData().setScale(this.drawScale)`
- L1844: `FontHelper.cardTitleFont.getData()`
- L1845: `FontHelper.renderRotatedText(sb, FontHelper.cardTitleFont, LOCKED_STRING, this.current_x, this.current_y, 0.0f, 175.0f *`
- L1849: `FontHelper.cardTitleFont.getData().setScale(this.drawScale)`
- L1849: `FontHelper.cardTitleFont.getData()`
- L1850: `FontHelper.renderRotatedText(sb, FontHelper.cardTitleFont, UNKNOWN_STRING, this.current_x, this.current_y, 0.0f, 175.0f `
- L1854: `FontHelper.cardTitleFont.getData().setScale(this.drawScale)`
- L1854: `FontHelper.cardTitleFont.getData()`
- L1856: `FontHelper.cardTitleFont.getData().setScale(this.drawScale * 0.85f)`
- L1856: `FontHelper.cardTitleFont.getData()`
- L1859: `Settings.GREEN_TEXT_COLOR.cpy()`
- L1861: `FontHelper.renderRotatedText(sb, FontHelper.cardTitleFont, this.name, this.current_x, this.current_y, 0.0f, 175.0f * thi`
- L1863: `FontHelper.renderRotatedText(sb, FontHelper.cardTitleFont, this.name, this.current_x, this.current_y, 0.0f, 175.0f * thi`

### renderType(SpriteBatch sb) (L1867-1898)

**Calls (in order):**
- L1895: `font.getData().setScale(this.drawScale)`
- L1895: `font.getData()`
- L1897: `FontHelper.renderRotatedText(sb, font, text, this.current_x, this.current_y - 22.0f * this.drawScale * Settings.scale, 0`

### getPrice(CardRarity rarity) (L1900-1922)

**Calls (in order):**
- L1903: `logger.info("ERROR: WHY WE SELLIN' BASIC")`
- L1916: `logger.info("ERROR: WHY WE SELLIN' SPECIAL")`
- L1920: `logger.info("No rarity on this card?")`

### renderEnergy(SpriteBatch sb) (L1924-1964)

**Calls (in order):**
- L1930: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_RED_ORB, this.current_x, this.current_y)`
- L1934: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_GREEN_ORB, this.current_x, this.current_y)`
- L1938: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_BLUE_ORB, this.current_x, this.current_y)`
- L1942: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_PURPLE_ORB, this.current_x, this.current_y)`
- L1946: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_COLORLESS_ORB, this.current_x, this.current_y)`
- L1949: `this.renderHelper(sb, this.renderColor, ImageMaster.CARD_COLORLESS_ORB, this.current_x, this.current_y)`
- L1952: `Color.WHITE.cpy()`
- L1953: `AbstractDungeon.player.hand.contains(this)`
- L1953: `this.hasEnoughEnergy()`
- L1955: `this.freeToPlay()`
- L1959: `this.getCost()`
- L1960: `this.getEnergyFont()`
- L1961: `this.cardID.equals("Slimed")`
- L1961: `this.cardID.equals("Pride")`
- L1962: `FontHelper.renderRotatedText(sb, font, text, this.current_x, this.current_y, -132.0f * this.drawScale * Settings.scale, `

### updateCost(int amt) (L1966-1984)

**Calls (in order):**
- L1967: `this.cardID.equals("Pride")`
- L1967: `this.cardID.equals("Slimed")`
- L1982: `logger.info("Curses/Statuses cannot have their costs modified")`

### getCost() (L2032-2040)

**Calls (in order):**
- L2036: `this.freeToPlay()`
- L2039: `Integer.toString(this.costForTurn)`

### freeToPlay() (L2042-2047)

**Calls (in order):**
- L2046: `AbstractDungeon.getCurrRoom()`
- L2046: `AbstractDungeon.player.hasPower("FreeAttackPower")`

### getEnergyFont() (L2049-2052)

**Calls (in order):**
- L2050: `FontHelper.cardEnergyFont_L.getData().setScale(this.drawScale)`
- L2050: `FontHelper.cardEnergyFont_L.getData()`

### updateHoverLogic() (L2071-2082)

**Calls (in order):**
- L2072: `this.hb.update()`
- L2074: `this.hover()`
- L2075: `Gdx.graphics.getDeltaTime()`
- L2080: `this.unhover()`

### moveToDiscardPile() (L2089-2092)

**Calls (in order):**
- L2091: `AbstractDungeon.getCurrRoom()`

### teleportToDiscardPile() (L2094-2099)

**Calls (in order):**
- L2096: `AbstractDungeon.getCurrRoom()`
- L2098: `this.onMoveToDiscard()`

### renderCardTip(SpriteBatch sb) (L2104-2135)

**Calls (in order):**
- L2111: `locked.add(0, "locked")`
- L2112: `TipHelper.renderTipForCard(this, sb, locked)`
- L2117: `unseen.add(0, "unseen")`
- L2118: `TipHelper.renderTipForCard(this, sb, unseen)`
- L2122: `this.makeCopy()`
- L2126: `copy.upgrade()`
- L2127: `TipHelper.renderTipForCard(copy, sb, copy.keywords)`
- L2129: `TipHelper.renderTipForCard(this, sb, this.keywords)`
- L2132: `this.renderCardPreview(sb)`

**Objects created:**
- L2110: `None`
- L2116: `None`

### renderCardPreviewInSingleView(SpriteBatch sb) (L2137-2142)

**Calls (in order):**
- L2141: `this.cardsToPreview.render(sb)`

### renderCardPreview(SpriteBatch sb) (L2144-2153)

**Calls (in order):**
- L2152: `this.cardsToPreview.render(sb)`

### triggerOnEndOfPlayerTurn() (L2161-2165)

**Calls (in order):**
- L2163: `this.addToTop(new ExhaustSpecificCardAction(this, AbstractDungeon.player.hand))`

**Objects created:**
- L2163: `ExhaustSpecificCardAction`

### applyPowers() (L2209-2274)

**Calls (in order):**
- L2210: `this.applyPowersToBlock()`
- L2216: `r.atDamageModify(tmp, this)`
- L2221: `p.atDamageGive(tmp, this.damageTypeForTurn, this)`
- L2223: `player.stance.atDamageGive(tmp, this.damageTypeForTurn, this)`
- L2227: `p.atDamageFinalGive(tmp, this.damageTypeForTurn, this)`
- L2232: `MathUtils.floor(tmp)`
- L2235: `MathUtils.floor(tmp)`
- L2238: `AbstractDungeon.getCurrRoom()`
- L2239: `m.size()`
- L2245: `r.atDamageModify(tmp[i], this)`
- L2250: `p.atDamageGive(tmp[i], this.damageTypeForTurn, this)`
- L2252: `player.stance.atDamageGive(tmp[i], this.damageTypeForTurn, this)`
- L2258: `p.atDamageFinalGive(tmp[i], this.damageTypeForTurn, this)`
- L2270: `MathUtils.floor(tmp[i])`

### applyPowersToBlock() (L2276-2292)

**Calls (in order):**
- L2280: `p.modifyBlock(tmp, this)`
- L2283: `p.modifyBlockLast(tmp)`
- L2285: `MathUtils.floor(tmp)`
- L2291: `MathUtils.floor(tmp)`

### calculateDamageDisplay(AbstractMonster mo) (L2294-2296)

**Calls (in order):**
- L2295: `this.calculateCardDamage(mo)`

### calculateCardDamage(AbstractMonster mo) (L2298-2381)

**Calls (in order):**
- L2299: `this.applyPowersToBlock()`
- L2305: `r.atDamageModify(tmp, this)`
- L2310: `p.atDamageGive(tmp, this.damageTypeForTurn, this)`
- L2312: `player.stance.atDamageGive(tmp, this.damageTypeForTurn, this)`
- L2316: `p.atDamageReceive(tmp, this.damageTypeForTurn, this)`
- L2319: `p.atDamageFinalGive(tmp, this.damageTypeForTurn, this)`
- L2322: `p.atDamageFinalReceive(tmp, this.damageTypeForTurn, this)`
- L2327: `MathUtils.floor(tmp)`
- L2330: `MathUtils.floor(tmp)`
- L2333: `AbstractDungeon.getCurrRoom()`
- L2334: `m.size()`
- L2340: `r.atDamageModify(tmp[i], this)`
- L2345: `p.atDamageGive(tmp[i], this.damageTypeForTurn, this)`
- L2347: `player.stance.atDamageGive(tmp[i], this.damageTypeForTurn, this)`
- L2352: `m.get((int)i)`
- L2353: `m.get((int)i)`
- L2353: `m.get((int)i)`
- L2354: `p.atDamageReceive(tmp[i], this.damageTypeForTurn, this)`
- L2359: `p.atDamageFinalGive(tmp[i], this.damageTypeForTurn, this)`
- L2363: `m.get((int)i)`
- L2364: `m.get((int)i)`
- L2364: `m.get((int)i)`
- L2365: `p.atDamageFinalReceive(tmp[i], this.damageTypeForTurn, this)`
- L2374: `MathUtils.floor(tmp[i])`
- L2377: `MathUtils.floor(tmp[i])`

### updateColor() (L2417-2425)

**Calls (in order):**
- L2419: `Gdx.graphics.getDeltaTime()`

### superFlash(Color c) (L2427-2429)

**Objects created:**
- L2428: `CardFlashVfx`

### superFlash() (L2431-2433)

**Objects created:**
- L2432: `CardFlashVfx`

### flash() (L2435-2437)

**Objects created:**
- L2436: `CardFlashVfx`

### flash(Color c) (L2439-2441)

**Objects created:**
- L2440: `CardFlashVfx`

### updateTransparency() (L2461-2486)

**Calls (in order):**
- L2463: `Gdx.graphics.getDeltaTime()`
- L2468: `Gdx.graphics.getDeltaTime()`

### setAngle(float degrees) (L2488-2490)

**Calls (in order):**
- L2489: `this.setAngle(degrees, false)`

### clearPowers() (L2496-2499)

**Calls (in order):**
- L2497: `this.resetAttributes()`

### debugPrintDetailedCardDataHeader() (L2501-2503)

**Calls (in order):**
- L2502: `logger.info(AbstractCard.gameDataUploadHeader())`
- L2502: `AbstractCard.gameDataUploadHeader()`

### gameDataUploadHeader() (L2505-2526)

**Calls (in order):**
- L2507: `builder.addFieldData("name")`
- L2508: `builder.addFieldData("cardID")`
- L2509: `builder.addFieldData("rawDescription")`
- L2510: `builder.addFieldData("assetURL")`
- L2511: `builder.addFieldData("keywords")`
- L2512: `builder.addFieldData("color")`
- L2513: `builder.addFieldData("type")`
- L2514: `builder.addFieldData("rarity")`
- L2515: `builder.addFieldData("cost")`
- L2516: `builder.addFieldData("target")`
- L2517: `builder.addFieldData("damageType")`
- L2518: `builder.addFieldData("baseDamage")`
- L2519: `builder.addFieldData("baseBlock")`
- L2520: `builder.addFieldData("baseHeal")`
- L2521: `builder.addFieldData("baseDraw")`
- L2522: `builder.addFieldData("baseDiscard")`
- L2523: `builder.addFieldData("baseMagicNumber")`
- L2524: `builder.addFieldData("isMultiDamage")`
- L2525: `builder.toString()`

**Objects created:**
- L2506: `GameDataStringBuilder`

### debugPrintDetailedCardData() (L2528-2530)

**Calls (in order):**
- L2529: `logger.info(this.gameDataUploadData())`
- L2529: `this.gameDataUploadData()`

### addToBot(AbstractGameAction action) (L2532-2534)

**Calls (in order):**
- L2533: `AbstractDungeon.actionManager.addToBottom(action)`

### addToTop(AbstractGameAction action) (L2536-2538)

**Calls (in order):**
- L2537: `AbstractDungeon.actionManager.addToTop(action)`

### gameDataUploadData() (L2540-2561)

**Calls (in order):**
- L2542: `builder.addFieldData(this.name)`
- L2543: `builder.addFieldData(this.cardID)`
- L2544: `builder.addFieldData(this.rawDescription)`
- L2545: `builder.addFieldData(this.assetUrl)`
- L2546: `builder.addFieldData(Arrays.toString(this.keywords.toArray()))`
- L2546: `Arrays.toString(this.keywords.toArray())`
- L2546: `this.keywords.toArray()`
- L2547: `builder.addFieldData(this.color.name())`
- L2547: `this.color.name()`
- L2548: `builder.addFieldData(this.type.name())`
- L2548: `this.type.name()`
- L2549: `builder.addFieldData(this.rarity.name())`
- L2549: `this.rarity.name()`
- L2550: `builder.addFieldData(this.cost)`
- L2551: `builder.addFieldData(this.target.name())`
- L2551: `this.target.name()`
- L2552: `builder.addFieldData(this.damageType.name())`
- L2552: `this.damageType.name()`
- L2553: `builder.addFieldData(this.baseDamage)`
- L2554: `builder.addFieldData(this.baseBlock)`
- L2555: `builder.addFieldData(this.baseHeal)`
- L2556: `builder.addFieldData(this.baseDraw)`
- L2557: `builder.addFieldData(this.baseDiscard)`
- L2558: `builder.addFieldData(this.baseMagicNumber)`
- L2559: `builder.addFieldData(this.isMultiDamage)`
- L2560: `builder.toString()`

**Objects created:**
- L2541: `GameDataStringBuilder`

### compareTo(AbstractCard other) (L2567-2570)

**Calls (in order):**
- L2569: `this.cardID.compareTo(other.cardID)`

### setLocked() (L2572-2588)

**Calls (in order):**
- L2587: `this.initializeDescription()`

### unlock() (L2590-2596)

**Calls (in order):**
- L2592: `cardAtlas.findRegion(this.assetUrl)`
- L2594: `oldCardAtlas.findRegion(this.assetUrl)`

### getLocStrings() (L2598-2604)

**Calls (in order):**
- L2600: `this.initializeDescription()`
- L2601: `cardData.put("name", (Serializable)((Object)this.name))`
- L2602: `cardData.put("description", (Serializable)((Object)this.rawDescription))`

**Objects created:**
- L2599: `None`

## AbstractCreature
File: `core\AbstractCreature.java`

### brokeBlock() (L148-156)

**Calls (in order):**
- L151: `r.onBlockBroken(this)`
- L154: `AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - th`
- L155: `CardCrawlGame.sound.play("BLOCK_BREAK")`

**Objects created:**
- L154: `HbBlockBrokenEffect`

### decrementBlock(DamageInfo info, int damageAmount) (L158-186)

**Calls (in order):**
- L160: `CardCrawlGame.screenShake.shake(ScreenShake.ShakeIntensity.MED, ScreenShake.ShakeDur.SHORT, false)`
- L164: `AbstractDungeon.effectList.add(new BlockedNumberEffect(this.hb.cX, this.hb.cY + this.hb.height / 2.0f, Integer.toString(`
- L164: `Integer.toString(this.currentBlock)`
- L166: `this.loseBlock()`
- L167: `this.brokeBlock()`
- L170: `this.loseBlock()`
- L171: `this.brokeBlock()`
- L172: `AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, TEXT[1]))`
- L174: `CardCrawlGame.sound.play("BLOCK_ATTACK")`
- L175: `this.loseBlock(damageAmount)`
- L177: `AbstractDungeon.effectList.add(new BlockImpactLineEffect(this.hb.cX, this.hb.cY))`
- L180: `AbstractDungeon.effectList.add(new BlockedNumberEffect(this.hb.cX, this.hb.cY + this.hb.height / 2.0f, Integer.toString(`
- L180: `Integer.toString(damageAmount)`

**Objects created:**
- L164: `BlockedNumberEffect`
- L172: `BlockedWordEffect`
- L177: `BlockImpactLineEffect`
- L180: `BlockedNumberEffect`

### increaseMaxHp(int amount, boolean showEffect) (L188-198)

**Calls (in order):**
- L189: `AbstractDungeon.player.hasBlight("FullBelly")`
- L191: `logger.info("Why are we decreasing health with increaseMaxHealth()?")`
- L194: `AbstractDungeon.effectsQueue.add(new TextAboveCreatureEffect(this.hb.cX - this.animX, this.hb.cY, TEXT[2] + Integer.toSt`
- L194: `Integer.toString(amount)`
- L195: `this.heal(amount, true)`
- L196: `this.healthBarUpdatedEvent()`

**Objects created:**
- L194: `TextAboveCreatureEffect`

### decreaseMaxHealth(int amount) (L200-212)

**Calls (in order):**
- L202: `logger.info("Why are we increasing health with decreaseMaxHealth()?")`
- L211: `this.healthBarUpdatedEvent()`

### refreshHitboxLocation() (L214-217)

**Calls (in order):**
- L215: `this.hb.move(this.drawX + this.hb_x + this.animX, this.drawY + this.hb_y + this.hb_h / 2.0f)`
- L216: `this.healthHb.move(this.hb.cX, this.hb.cY - this.hb_h / 2.0f - this.healthHb.height / 2.0f)`

### updateAnimations() (L219-256)

**Calls (in order):**
- L223: `this.updateFastAttackAnimation()`
- L227: `this.updateSlowAttackAnimation()`
- L231: `this.updateFastShakeAnimation()`
- L235: `this.updateHopAnimation()`
- L239: `this.updateJumpAnimation()`
- L243: `this.updateShakeAnimation()`
- L247: `this.updateStaggerAnimation()`
- L252: `this.refreshHitboxLocation()`
- L254: `((AbstractMonster)this).refreshIntentHbLocation()`

### updateFastAttackAnimation() (L258-272)

**Calls (in order):**
- L259: `Gdx.graphics.getDeltaTime()`
- L265: `Interpolation.exp5In.apply(0.0f, targetPos, (1.0f - this.animationTimer / 1.0f) * 2.0f)`
- L270: `Interpolation.fade.apply(0.0f, targetPos, this.animationTimer / 1.0f * 2.0f)`

### updateSlowAttackAnimation() (L274-288)

**Calls (in order):**
- L275: `Gdx.graphics.getDeltaTime()`
- L281: `Interpolation.exp10In.apply(0.0f, targetPos, (1.0f - this.animationTimer / 1.0f) * 2.0f)`
- L286: `Interpolation.fade.apply(0.0f, targetPos, this.animationTimer / 1.0f * 2.0f)`

### updateFastShakeAnimation() (L290-306)

**Calls (in order):**
- L291: `Gdx.graphics.getDeltaTime()`
- L296: `Gdx.graphics.getDeltaTime()`
- L301: `Gdx.graphics.getDeltaTime()`

### updateHopAnimation() (L308-315)

**Calls (in order):**
- L310: `Gdx.graphics.getDeltaTime()`

### updateJumpAnimation() (L317-324)

**Calls (in order):**
- L319: `Gdx.graphics.getDeltaTime()`

### updateStaggerAnimation() (L326-336)

**Calls (in order):**
- L328: `Gdx.graphics.getDeltaTime()`
- L329: `Interpolation.pow2.apply(STAGGER_MOVE_SPEED, 0.0f, 1.0f - this.animationTimer / 0.3f)`
- L329: `Interpolation.pow2.apply(-STAGGER_MOVE_SPEED, 0.0f, 1.0f - this.animationTimer / 0.3f)`

### updateShakeAnimation() (L338-354)

**Calls (in order):**
- L339: `Gdx.graphics.getDeltaTime()`
- L344: `Gdx.graphics.getDeltaTime()`
- L349: `Gdx.graphics.getDeltaTime()`

### loadAnimation(String atlasUrl, String skeletonUrl, float scale) (L356-373)

**Calls (in order):**
- L357: `Gdx.files.internal(atlasUrl)`
- L360: `AbstractDungeon.player.hasRelic("PreservedInsect")`
- L360: `AbstractDungeon.getCurrRoom()`
- L363: `ModHelper.isModEnabled("MonsterHunter")`
- L367: `json.setScale(Settings.renderScale / scale)`
- L368: `json.readSkeletonData(Gdx.files.internal(skeletonUrl))`
- L368: `Gdx.files.internal(skeletonUrl)`
- L370: `this.skeleton.setColor(Color.WHITE)`

**Objects created:**
- L357: `TextureAtlas`
- L358: `SkeletonJson`
- L369: `Skeleton`
- L371: `AnimationStateData`
- L372: `AnimationState`

### heal(int healAmount, boolean showEffect) (L375-406)

**Calls (in order):**
- L376: `AbstractDungeon.player.hasBlight("FullBelly")`
- L384: `r.onPlayerHeal(healAmount)`
- L387: `p.onHeal(healAmount)`
- L396: `r2.onNotBloodied()`
- L401: `AbstractDungeon.topPanel.panelHealEffect()`
- L402: `AbstractDungeon.effectsQueue.add(new HealEffect(this.hb.cX - this.animX, this.hb.cY, healAmount))`
- L404: `this.healthBarUpdatedEvent()`

**Objects created:**
- L402: `HealEffect`

### heal(int amount) (L408-410)

**Calls (in order):**
- L409: `this.heal(amount, true)`

### addBlock(int blockAmount) (L412-451)

**Calls (in order):**
- L416: `abstractRelic.onPlayerGainedBlock(tmp)`
- L420: `abstractPower.onGainedBlock(tmp)`
- L428: `AbstractDungeon.getCurrRoom()`
- L430: `p.onPlayerGainedBlock(tmp)`
- L433: `MathUtils.floor(tmp)`
- L435: `UnlockTracker.unlockAchievement("IMPERVIOUS")`
- L441: `UnlockTracker.unlockAchievement("BARRICADED")`
- L444: `this.gainBlockAnimation()`
- L446: `Settings.GOLD_COLOR.cpy()`

### loseBlock(int amount, boolean noAnimation) (L453-472)

**Calls (in order):**
- L464: `AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - th`
- L467: `Color.SCARLET.cpy()`

**Objects created:**
- L464: `HbBlockBrokenEffect`

### loseBlock() (L474-476)

**Calls (in order):**
- L475: `this.loseBlock(this.currentBlock)`

### loseBlock(boolean noAnimation) (L478-480)

**Calls (in order):**
- L479: `this.loseBlock(this.currentBlock, noAnimation)`

### loseBlock(int amount) (L482-484)

**Calls (in order):**
- L483: `this.loseBlock(amount, false)`

### addPower(AbstractPower powerToApply) (L495-516)

**Calls (in order):**
- L498: `p.ID.equals(powerToApply.ID)`
- L499: `p.stackPower(powerToApply.amount)`
- L500: `p.updateDescription()`
- L504: `this.powers.add(powerToApply)`
- L512: `UnlockTracker.unlockAchievement("POWERFUL")`

### applyStartOfTurnPowers() (L518-522)

**Calls (in order):**
- L520: `p.atStartOfTurn()`

### applyTurnPowers() (L524-528)

**Calls (in order):**
- L526: `p.duringTurn()`

### applyStartOfTurnPostDrawPowers() (L530-534)

**Calls (in order):**
- L532: `p.atStartOfTurnPostDraw()`

### applyEndOfTurnTriggers() (L536-543)

**Calls (in order):**
- L539: `p.atEndOfTurnPreEndTurnCards(false)`
- L541: `p.atEndOfTurn(this.isPlayer)`

### updateHealthBar() (L567-573)

**Calls (in order):**
- L568: `this.updateHbHoverFade()`
- L569: `this.updateBlockAnimations()`
- L570: `this.updateHbPopInAnimation()`
- L571: `this.updateHbDamageAnimation()`
- L572: `this.updateHbAlpha()`

### updateHbHoverFade() (L575-587)

**Calls (in order):**
- L577: `Gdx.graphics.getDeltaTime()`
- L582: `Gdx.graphics.getDeltaTime()`

### updateHbAlpha() (L589-616)

**Calls (in order):**
- L591: `MathHelper.fadeLerpSnap(this.hbAlpha, 0.0f)`
- L602: `MathHelper.fadeLerpSnap(this.hbShadowColor.a, 0.0f)`
- L603: `MathHelper.fadeLerpSnap(this.hbBgColor.a, 0.0f)`
- L604: `MathHelper.fadeLerpSnap(this.hbTextColor.a, 0.0f)`
- L605: `MathHelper.fadeLerpSnap(this.blockOutlineColor.a, 0.0f)`

### updateBlockAnimations() (L624-648)

**Calls (in order):**
- L627: `Gdx.graphics.getDeltaTime()`
- L631: `Interpolation.swingOut.apply(BLOCK_OFFSET_DIST * 3.0f, 0.0f, 1.0f - this.blockAnimTimer / 0.7f)`
- L632: `Interpolation.pow3In.apply(3.0f, 1.0f, 1.0f - this.blockAnimTimer / 0.7f)`
- L633: `Interpolation.pow2Out.apply(0.0f, 1.0f, 1.0f - this.blockAnimTimer / 0.7f)`
- L634: `Interpolation.pow5In.apply(0.0f, 1.0f, 1.0f - this.blockAnimTimer / 0.7f)`
- L636: `MathHelper.scaleLerpSnap(this.blockScale, 1.0f)`
- L639: `MathHelper.slowColorLerpSnap(this.blockTextColor.r, 1.0f)`
- L642: `MathHelper.slowColorLerpSnap(this.blockTextColor.g, 1.0f)`
- L645: `MathHelper.slowColorLerpSnap(this.blockTextColor.b, 1.0f)`

### updateHbPopInAnimation() (L650-659)

**Calls (in order):**
- L652: `Gdx.graphics.getDeltaTime()`
- L656: `Interpolation.fade.apply(0.0f, 1.0f, 1.0f - this.hbShowTimer / 0.7f)`
- L657: `Interpolation.exp10Out.apply(HB_Y_OFFSET_DIST * 5.0f, 0.0f, 1.0f - this.hbShowTimer / 0.7f)`

### updateHbDamageAnimation() (L661-668)

**Calls (in order):**
- L663: `Gdx.graphics.getDeltaTime()`
- L666: `MathHelper.uiLerpSnap(this.healthBarWidth, this.targetHealthBarWidth)`

### updatePowers() (L670-674)

**Calls (in order):**
- L671: `this.powers.size()`
- L672: `this.powers.get(i).update(i)`
- L672: `this.powers.get(i)`

### initialize() (L676-679)

**Calls (in order):**
- L678: `sr.setPremultipliedAlpha(true)`

**Objects created:**
- L677: `SkeletonMeshRenderer`

### renderPowerTips(SpriteBatch sb) (L681-697)

**Calls (in order):**
- L682: `this.tips.clear()`
- L685: `this.tips.add(new PowerTip(p.name, p.description, p.region48))`
- L688: `this.tips.add(new PowerTip(p.name, p.description, p.img))`
- L690: `this.tips.isEmpty()`
- L692: `TipHelper.queuePowerTips(this.hb.cX + this.hb.width / 2.0f + TIP_OFFSET_R_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L692: `TipHelper.calculateAdditionalOffset(this.tips, this.hb.cY)`
- L694: `TipHelper.queuePowerTips(this.hb.cX - this.hb.width / 2.0f + TIP_OFFSET_L_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L694: `TipHelper.calculateAdditionalOffset(this.tips, this.hb.cY)`

**Objects created:**
- L685: `PowerTip`
- L688: `PowerTip`

### getPower(String targetID) (L753-759)

**Calls (in order):**
- L755: `p.ID.equals(targetID)`

### hasPower(String targetID) (L761-767)

**Calls (in order):**
- L763: `p.ID.equals(targetID)`

### loseGold(int goldAmount) (L782-791)

**Calls (in order):**
- L789: `logger.info("NEGATIVE MONEY???")`

### gainGold(int amount) (L793-799)

**Calls (in order):**
- L795: `logger.info("NEGATIVE MONEY???")`

### renderReticle(SpriteBatch sb) (L801-807)

**Calls (in order):**
- L803: `this.renderReticleCorner(sb, -this.hb.width / 2.0f + this.reticleOffset, this.hb.height / 2.0f - this.reticleOffset, fal`
- L804: `this.renderReticleCorner(sb, this.hb.width / 2.0f - this.reticleOffset, this.hb.height / 2.0f - this.reticleOffset, true`
- L805: `this.renderReticleCorner(sb, -this.hb.width / 2.0f + this.reticleOffset, -this.hb.height / 2.0f + this.reticleOffset, fa`
- L806: `this.renderReticleCorner(sb, this.hb.width / 2.0f - this.reticleOffset, -this.hb.height / 2.0f + this.reticleOffset, tru`

### renderReticle(SpriteBatch sb, Hitbox hb) (L809-815)

**Calls (in order):**
- L811: `this.renderReticleCorner(sb, -hb.width / 2.0f + this.reticleOffset, hb.height / 2.0f - this.reticleOffset, hb, false, fa`
- L812: `this.renderReticleCorner(sb, hb.width / 2.0f - this.reticleOffset, hb.height / 2.0f - this.reticleOffset, hb, true, fals`
- L813: `this.renderReticleCorner(sb, -hb.width / 2.0f + this.reticleOffset, -hb.height / 2.0f + this.reticleOffset, hb, false, t`
- L814: `this.renderReticleCorner(sb, hb.width / 2.0f - this.reticleOffset, -hb.height / 2.0f + this.reticleOffset, hb, true, tru`

### updateReticle() (L817-834)

**Calls (in order):**
- L820: `Gdx.graphics.getDeltaTime()`
- L824: `Gdx.graphics.getDeltaTime()`
- L828: `Interpolation.elasticOut.apply(RETICLE_OFFSET_DIST, 0.0f, this.reticleAnimTimer)`

### renderHealth(SpriteBatch sb) (L836-858)

**Calls (in order):**
- L842: `this.renderHealthBg(sb, x, y)`
- L844: `this.renderOrangeHealthBar(sb, x, y)`
- L845: `this.hasPower("Poison")`
- L846: `this.renderGreenHealthBar(sb, x, y)`
- L848: `this.renderRedHealthBar(sb, x, y)`
- L851: `this.renderBlockOutline(sb, x, y)`
- L853: `this.renderHealthText(sb, y)`
- L855: `this.renderBlockIconAndValue(sb, x, y)`
- L857: `this.renderPowerIcons(sb, x, y)`

### renderBlockOutline(SpriteBatch sb, float x, float y) (L860-867)

**Calls (in order):**
- L861: `sb.setColor(this.blockOutlineColor)`
- L862: `sb.setBlendFunction(770, 1)`
- L863: `sb.draw(ImageMaster.BLOCK_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L864: `sb.draw(ImageMaster.BLOCK_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.hb.width, HEALTH_BAR_HEIGHT)`
- L865: `sb.draw(ImageMaster.BLOCK_BAR_R, x + this.hb.width, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L866: `sb.setBlendFunction(770, 771)`

### renderBlockIconAndValue(SpriteBatch sb, float x, float y) (L869-873)

**Calls (in order):**
- L870: `sb.setColor(this.blockColor)`
- L871: `sb.draw(ImageMaster.BLOCK_ICON, x + BLOCK_ICON_X - 32.0f, y + BLOCK_ICON_Y - 32.0f + this.blockOffset, 32.0f, 32.0f, 64.`
- L872: `FontHelper.renderFontCentered(sb, FontHelper.blockInfoFont, Integer.toString(this.currentBlock), x + BLOCK_ICON_X, y - 1`
- L872: `Integer.toString(this.currentBlock)`

### renderHealthBg(SpriteBatch sb, float x, float y) (L875-886)

**Calls (in order):**
- L876: `sb.setColor(this.hbShadowColor)`
- L877: `sb.draw(ImageMaster.HB_SHADOW_L, x - HEALTH_BAR_HEIGHT, y - HEALTH_BG_OFFSET_X + 3.0f * Settings.scale, HEALTH_BAR_HEIGH`
- L878: `sb.draw(ImageMaster.HB_SHADOW_B, x, y - HEALTH_BG_OFFSET_X + 3.0f * Settings.scale, this.hb.width, HEALTH_BAR_HEIGHT)`
- L879: `sb.draw(ImageMaster.HB_SHADOW_R, x + this.hb.width, y - HEALTH_BG_OFFSET_X + 3.0f * Settings.scale, HEALTH_BAR_HEIGHT, H`
- L880: `sb.setColor(this.hbBgColor)`
- L882: `sb.draw(ImageMaster.HEALTH_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L883: `sb.draw(ImageMaster.HEALTH_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.hb.width, HEALTH_BAR_HEIGHT)`
- L884: `sb.draw(ImageMaster.HEALTH_BAR_R, x + this.hb.width, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`

### renderOrangeHealthBar(SpriteBatch sb, float x, float y) (L888-893)

**Calls (in order):**
- L889: `sb.setColor(this.orangeHbBarColor)`
- L890: `sb.draw(ImageMaster.HEALTH_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L891: `sb.draw(ImageMaster.HEALTH_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.healthBarWidth, HEALTH_BAR_HEIGHT)`
- L892: `sb.draw(ImageMaster.HEALTH_BAR_R, x + this.healthBarWidth, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT`

### renderGreenHealthBar(SpriteBatch sb, float x, float y) (L895-902)

**Calls (in order):**
- L896: `sb.setColor(this.greenHbBarColor)`
- L898: `sb.draw(ImageMaster.HEALTH_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L900: `sb.draw(ImageMaster.HEALTH_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.targetHealthBarWidth, HEALTH_BAR_HEIGHT)`
- L901: `sb.draw(ImageMaster.HEALTH_BAR_R, x + this.targetHealthBarWidth, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_`

### renderRedHealthBar(SpriteBatch sb, float x, float y) (L904-931)

**Calls (in order):**
- L906: `sb.setColor(this.blueHbBarColor)`
- L908: `sb.setColor(this.redHbBarColor)`
- L910: `this.hasPower("Poison")`
- L912: `sb.draw(ImageMaster.HEALTH_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L914: `sb.draw(ImageMaster.HEALTH_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.targetHealthBarWidth, HEALTH_BAR_HEIGHT)`
- L915: `sb.draw(ImageMaster.HEALTH_BAR_R, x + this.targetHealthBarWidth, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_`
- L917: `this.getPower((String)"Poison")`
- L918: `this.hasPower("Intangible")`
- L925: `sb.draw(ImageMaster.HEALTH_BAR_L, x - HEALTH_BAR_HEIGHT, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_BAR_HEIGHT)`
- L927: `sb.draw(ImageMaster.HEALTH_BAR_B, x, y + HEALTH_BAR_OFFSET_Y, this.targetHealthBarWidth - w, HEALTH_BAR_HEIGHT)`
- L928: `sb.draw(ImageMaster.HEALTH_BAR_R, x + this.targetHealthBarWidth - w, y + HEALTH_BAR_OFFSET_Y, HEALTH_BAR_HEIGHT, HEALTH_`

### renderHealthText(SpriteBatch sb, float y) (L933-942)

**Calls (in order):**
- L937: `FontHelper.renderFontCentered(sb, FontHelper.healthInfoFont, this.currentHealth + "/" + this.maxHealth, this.hb.cX, y + `
- L940: `FontHelper.renderFontCentered(sb, FontHelper.healthInfoFont, TEXT[0], this.hb.cX, y + HEALTH_BAR_OFFSET_Y + HEALTH_TEXT_`

### renderPowerIcons(SpriteBatch sb, float x, float y) (L944-963)

**Calls (in order):**
- L948: `p.renderIcons(sb, x + offset, y - 53.0f * Settings.scale, this.hbTextColor)`
- L950: `p.renderIcons(sb, x + offset, y - 48.0f * Settings.scale, this.hbTextColor)`
- L957: `p.renderAmount(sb, x + offset + 32.0f * Settings.scale, y - 75.0f * Settings.scale, this.hbTextColor)`
- L959: `p.renderAmount(sb, x + offset + 32.0f * Settings.scale, y - 66.0f * Settings.scale, this.hbTextColor)`

### renderReticleCorner(SpriteBatch sb, float x, float y, Hitbox hb, boolean flipX, boolean flipY) (L965-972)

**Calls (in order):**
- L967: `sb.setColor(this.reticleShadowColor)`
- L968: `sb.draw(ImageMaster.RETICLE_CORNER, hb.cX + x - 18.0f + 4.0f * Settings.scale, hb.cY + y - 18.0f - 4.0f * Settings.scale`
- L970: `sb.setColor(this.reticleColor)`
- L971: `sb.draw(ImageMaster.RETICLE_CORNER, hb.cX + x - 18.0f, hb.cY + y - 18.0f, 18.0f, 18.0f, 36.0f, 36.0f, Settings.scale, Se`

### renderReticleCorner(SpriteBatch sb, float x, float y, boolean flipX, boolean flipY) (L974-981)

**Calls (in order):**
- L976: `sb.setColor(this.reticleShadowColor)`
- L977: `sb.draw(ImageMaster.RETICLE_CORNER, this.hb.cX + x - 18.0f + 4.0f * Settings.scale, this.hb.cY + y - 18.0f - 4.0f * Sett`
- L979: `sb.setColor(this.reticleColor)`
- L980: `sb.draw(ImageMaster.RETICLE_CORNER, this.hb.cX + x - 18.0f, this.hb.cY + y - 18.0f, 18.0f, 18.0f, 36.0f, 36.0f, Settings`

## AbstractDungeon
File: `dungeons\AbstractDungeon.java`

### setBoss(String key) (L329-370)

**Calls (in order):**
- L332: `DungeonMap.boss.dispose()`
- L333: `DungeonMap.bossOutline.dispose()`
- L335: `key.equals("The Guardian")`
- L336: `ImageMaster.loadImage("images/ui/map/boss/guardian.png")`
- L337: `ImageMaster.loadImage("images/ui/map/bossOutline/guardian.png")`
- L338: `key.equals("Hexaghost")`
- L339: `ImageMaster.loadImage("images/ui/map/boss/hexaghost.png")`
- L340: `ImageMaster.loadImage("images/ui/map/bossOutline/hexaghost.png")`
- L341: `key.equals("Slime Boss")`
- L342: `ImageMaster.loadImage("images/ui/map/boss/slime.png")`
- L343: `ImageMaster.loadImage("images/ui/map/bossOutline/slime.png")`
- L344: `key.equals("Collector")`
- L345: `ImageMaster.loadImage("images/ui/map/boss/collector.png")`
- L346: `ImageMaster.loadImage("images/ui/map/bossOutline/collector.png")`
- L347: `key.equals("Automaton")`
- L348: `ImageMaster.loadImage("images/ui/map/boss/automaton.png")`
- L349: `ImageMaster.loadImage("images/ui/map/bossOutline/automaton.png")`
- L350: `key.equals("Champ")`
- L351: `ImageMaster.loadImage("images/ui/map/boss/champ.png")`
- L352: `ImageMaster.loadImage("images/ui/map/bossOutline/champ.png")`
- L353: `key.equals("Awakened One")`
- L354: `ImageMaster.loadImage("images/ui/map/boss/awakened.png")`
- L355: `ImageMaster.loadImage("images/ui/map/bossOutline/awakened.png")`
- L356: `key.equals("Time Eater")`
- L357: `ImageMaster.loadImage("images/ui/map/boss/timeeater.png")`
- L358: `ImageMaster.loadImage("images/ui/map/bossOutline/timeeater.png")`
- L359: `key.equals("Donu and Deca")`
- L360: `ImageMaster.loadImage("images/ui/map/boss/donu.png")`
- L361: `ImageMaster.loadImage("images/ui/map/bossOutline/donu.png")`
- L362: `key.equals("The Heart")`
- L363: `ImageMaster.loadImage("images/ui/map/boss/heart.png")`
- L364: `ImageMaster.loadImage("images/ui/map/bossOutline/heart.png")`
- L366: `logger.info("WARNING: UNKNOWN BOSS ICON: " + key)`
- L369: `logger.info("[BOSS] " + key)`

### generateSeeds() (L378-392)

**Calls (in order):**
- L379: `logger.info("Generating seeds: " + Settings.seed)`

**Objects created:**
- L380: `Random`
- L381: `Random`
- L382: `Random`
- L383: `Random`
- L384: `Random`
- L385: `Random`
- L386: `Random`
- L387: `Random`
- L388: `Random`
- L389: `Random`
- L390: `Random`
- L391: `Random`

### loadSeeds(SaveFile save) (L394-421)

**Calls (in order):**
- L400: `ModHelper.setTodaysMods(save.special_seed, AbstractDungeon.player.chosenClass)`
- L402: `ModHelper.setTodaysMods(save.seed, AbstractDungeon.player.chosenClass)`
- L413: `logger.info("Loading seeds: " + Settings.seed)`
- L414: `logger.info("Monster seed:  " + AbstractDungeon.monsterRng.counter)`
- L415: `logger.info("Event seed:    " + AbstractDungeon.eventRng.counter)`
- L416: `logger.info("Merchant seed: " + AbstractDungeon.merchantRng.counter)`
- L417: `logger.info("Card seed:     " + AbstractDungeon.cardRng.counter)`
- L418: `logger.info("Treasure seed: " + AbstractDungeon.treasureRng.counter)`
- L419: `logger.info("Relic seed:    " + AbstractDungeon.relicRng.counter)`
- L420: `logger.info("Potion seed:   " + AbstractDungeon.potionRng.counter)`

**Objects created:**
- L405: `Random`
- L406: `Random`
- L407: `Random`
- L408: `Random`
- L410: `Random`
- L411: `Random`
- L412: `Random`

### populatePathTaken(SaveFile saveFile) (L423-473)

**Calls (in order):**
- L425: `saveFile.current_room.equals(MonsterRoomBoss.class.getName())`
- L425: `MonsterRoomBoss.class.getName()`
- L429: `saveFile.current_room.equals(TreasureRoomBoss.class.getName())`
- L429: `TreasureRoomBoss.class.getName()`
- L438: `saveFile.current_room.equals(NeowRoom.class.getName())`
- L438: `NeowRoom.class.getName()`
- L438: `map.get(saveFile.room_y).get(saveFile.room_x)`
- L438: `map.get(saveFile.room_y)`
- L440: `pathX.size()`
- L442: `pathY.get(i)`
- L443: `map.get(pathY.get(i)).get(pathX.get(i))`
- L443: `map.get(pathY.get(i))`
- L443: `pathY.get(i)`
- L443: `pathX.get(i)`
- L444: `node2.getEdges()`
- L446: `e.markAsTaken()`
- L449: `pathY.get(i)`
- L450: `AbstractDungeon.map.get((int)AbstractDungeon.pathY.get((int)i).intValue()).get((int)AbstractDungeon.pathX.get((int)i).in`
- L450: `AbstractDungeon.map.get((int)AbstractDungeon.pathY.get((int)i).intValue())`
- L450: `AbstractDungeon.pathY.get((int)i).intValue()`
- L450: `AbstractDungeon.pathY.get((int)i)`
- L450: `AbstractDungeon.pathX.get((int)i).intValue()`
- L450: `AbstractDungeon.pathX.get((int)i)`
- L451: `node.getEdgeConnectedTo(map.get(pathY.get(i)).get(pathX.get(i)))`
- L451: `map.get(pathY.get(i)).get(pathX.get(i))`
- L451: `map.get(pathY.get(i))`
- L451: `pathY.get(i)`
- L451: `pathX.get(i)`
- L452: `connectedEdge.markAsTaken()`
- L454: `map.get(pathY.get(i)).get(pathX.get(i))`
- L454: `map.get(pathY.get(i))`
- L454: `pathY.get(i)`
- L454: `pathX.get(i)`
- L456: `this.isLoadingIntoNeow(saveFile)`
- L457: `logger.info("Loading into Neow")`
- L462: `logger.info("Loading into: " + saveFile.room_x + "," + saveFile.room_y)`
- L466: `this.nextRoomTransition(saveFile)`
- L467: `this.isLoadingIntoNeow(saveFile)`

**Objects created:**
- L426: `MapRoomNode`
- L427: `MonsterRoomBoss`
- L430: `MapRoomNode`
- L431: `TreasureRoomBoss`
- L434: `MapRoomNode`
- L435: `VictoryRoom`
- L458: `MapRoomNode`
- L459: `EmptyRoom`
- L463: `MapRoomNode`
- L464: `EmptyRoom`
- L468: `NeowRoom`
- L468: `NeowRoom`

### isLoadingIntoNeow(SaveFile saveFile) (L475-477)

**Calls (in order):**
- L476: `saveFile.current_room.equals(NeowRoom.class.getName())`
- L476: `NeowRoom.class.getName()`

### getRandomChest() (L479-488)

**Calls (in order):**
- L480: `treasureRng.random(0, 99)`

**Objects created:**
- L482: `SmallChest`
- L485: `MediumChest`
- L487: `LargeChest`

### generateMap() (L490-520)

**Calls (in order):**
- L491: `System.currentTimeMillis()`
- L496: `MapGenerator.generateDungeon(mapHeight, mapWidth, mapPathDensity, mapRng)`
- L500: `n.hasEdges()`
- L500: `map.size()`
- L504: `AbstractDungeon.generateRoomTypes(roomList, count)`
- L505: `RoomTypeAssigner.assignRowAsRoomType(map.get(map.size() - 1), RestRoom.class)`
- L505: `map.get(map.size() - 1)`
- L505: `map.size()`
- L506: `RoomTypeAssigner.assignRowAsRoomType(map.get(0), MonsterRoom.class)`
- L506: `map.get(0)`
- L507: `player.hasBlight("MimicInfestation")`
- L508: `RoomTypeAssigner.assignRowAsRoomType(map.get(8), MonsterRoomElite.class)`
- L508: `map.get(8)`
- L510: `RoomTypeAssigner.assignRowAsRoomType(map.get(8), TreasureRoom.class)`
- L510: `map.get(8)`
- L512: `RoomTypeAssigner.distributeRoomsAcrossMap(mapRng, map, roomList)`
- L513: `logger.info("Generated the following dungeon map:")`
- L514: `logger.info(MapGenerator.toString(map, true))`
- L514: `MapGenerator.toString(map, true)`
- L515: `logger.info("Game Seed: " + Settings.seed)`
- L516: `logger.info("Map generation time: " + (System.currentTimeMillis() - startTime) + "ms")`
- L516: `System.currentTimeMillis()`
- L518: `AbstractDungeon.fadeIn()`
- L519: `AbstractDungeon.setEmeraldElite()`

**Objects created:**
- L495: `None`

### setEmeraldElite() (L522-536)

**Calls (in order):**
- L525: `map.size()`
- L526: `map.get(i).size()`
- L526: `map.get(i)`
- L527: `AbstractDungeon.map.get((int)i).get((int)j)`
- L527: `AbstractDungeon.map.get((int)i)`
- L528: `eliteNodes.add(map.get(i).get(j))`
- L528: `map.get(i).get(j)`
- L528: `map.get(i)`
- L531: `eliteNodes.get(mapRng.random(0, eliteNodes.size() - 1))`
- L531: `mapRng.random(0, eliteNodes.size() - 1)`
- L531: `eliteNodes.size()`
- L533: `logger.info("[INFO] Elite nodes identified: " + eliteNodes.size())`
- L533: `eliteNodes.size()`
- L534: `logger.info("[INFO] Emerald Key  placed in: [" + chosenNode.x + "," + chosenNode.y + "]")`

**Objects created:**
- L524: `None`

### generateRoomTypes(ArrayList<AbstractRoom> roomList, int availableRoomCount) (L538-574)

**Calls (in order):**
- L541: `logger.info("Generating Room Types! There are " + availableRoomCount + " rooms:")`
- L542: `Math.round((float)availableRoomCount * shopRoomChance)`
- L543: `logger.info(" SHOP (" + AbstractDungeon.toPercentage(shopRoomChance) + "): " + shopCount)`
- L543: `AbstractDungeon.toPercentage(shopRoomChance)`
- L544: `Math.round((float)availableRoomCount * restRoomChance)`
- L545: `logger.info(" REST (" + AbstractDungeon.toPercentage(restRoomChance) + "): " + restCount)`
- L545: `AbstractDungeon.toPercentage(restRoomChance)`
- L546: `Math.round((float)availableRoomCount * treasureRoomChance)`
- L547: `logger.info(" TRSRE (" + AbstractDungeon.toPercentage(treasureRoomChance) + "): " + treasureCount)`
- L547: `AbstractDungeon.toPercentage(treasureRoomChance)`
- L548: `ModHelper.isModEnabled("Elite Swarm")`
- L549: `Math.round((float)availableRoomCount * (eliteRoomChance * 2.5f))`
- L550: `logger.info(" ELITE (" + AbstractDungeon.toPercentage(eliteRoomChance) + "): " + eliteCount)`
- L550: `AbstractDungeon.toPercentage(eliteRoomChance)`
- L552: `Math.round((float)availableRoomCount * eliteRoomChance * 1.6f)`
- L553: `logger.info(" ELITE (" + AbstractDungeon.toPercentage(eliteRoomChance) + "): " + eliteCount)`
- L553: `AbstractDungeon.toPercentage(eliteRoomChance)`
- L555: `Math.round((float)availableRoomCount * eliteRoomChance)`
- L556: `logger.info(" ELITE (" + AbstractDungeon.toPercentage(eliteRoomChance) + "): " + eliteCount)`
- L556: `AbstractDungeon.toPercentage(eliteRoomChance)`
- L558: `Math.round((float)availableRoomCount * eventRoomChance)`
- L559: `logger.info(" EVNT (" + AbstractDungeon.toPercentage(eventRoomChance) + "): " + eventCount)`
- L559: `AbstractDungeon.toPercentage(eventRoomChance)`
- L561: `logger.info(" MSTR (" + AbstractDungeon.toPercentage(1.0f - shopRoomChance - restRoomChance - treasureRoomChance - elite`
- L561: `AbstractDungeon.toPercentage(1.0f - shopRoomChance - restRoomChance - treasureRoomChance - eliteRoomChance - eventRoomCh`
- L563: `roomList.add(new ShopRoom())`
- L566: `roomList.add(new RestRoom())`
- L569: `roomList.add(new MonsterRoomElite())`
- L572: `roomList.add(new EventRoom())`

**Objects created:**
- L563: `ShopRoom`
- L566: `RestRoom`
- L569: `MonsterRoomElite`
- L572: `EventRoom`

### toPercentage(float n) (L576-578)

**Calls (in order):**
- L577: `String.format("%.0f", Float.valueOf(n * 100.0f))`
- L577: `Float.valueOf(n * 100.0f)`

### firstRoomLogic() (L580-585)

**Calls (in order):**
- L581: `AbstractDungeon.initializeFirstRoom()`
- L582: `currMapNode.leftNodeAvailable()`
- L583: `currMapNode.centerNodeAvailable()`
- L584: `currMapNode.rightNodeAvailable()`

### passesDonutCheck(ArrayList<ArrayList<MapRoomNode>> map) (L587-622)

**Calls (in order):**
- L588: `logger.info("CASEY'S DONUT CHECK: ")`
- L589: `map.get(0).size()`
- L589: `map.get(0)`
- L590: `map.size()`
- L591: `logger.info(" HEIGHT: " + height)`
- L592: `logger.info(" WIDTH:  " + width)`
- L598: `map.get(map.size() - 2)`
- L598: `map.size()`
- L600: `n.getEdges()`
- L609: `logger.info(" [FAIL] " + nodeCount + " NODES IN LAST ROW")`
- L612: `logger.info(" [SUCCESS] " + nodeCount + " NODE IN LAST ROW")`
- L620: `logger.info(" ROOM COUNT: " + roomCount)`

### getCurrRoom() (L624-626)

**Calls (in order):**
- L625: `currMapNode.getRoom()`

### setCurrMapNode(MapRoomNode currMapNode) (L632-650)

**Calls (in order):**
- L634: `AbstractDungeon.getCurrRoom()`
- L635: `AbstractDungeon.getCurrRoom().dispose()`
- L635: `AbstractDungeon.getCurrRoom()`
- L639: `logger.warn("This player loaded into a room that no longer exists (due to a new map gen?)")`
- L641: `AbstractDungeon.map.get((int)currMapNode.y).get((int)i)`
- L641: `AbstractDungeon.map.get((int)currMapNode.y)`
- L642: `map.get(currMapNode.y).get(i)`
- L642: `map.get(currMapNode.y)`
- L643: `AbstractDungeon.map.get((int)currMapNode.y).get((int)i)`
- L643: `AbstractDungeon.map.get((int)currMapNode.y)`
- L644: `AbstractDungeon.map.get((int)currMapNode.y).get((int)i)`
- L644: `AbstractDungeon.map.get((int)currMapNode.y)`

### returnRandomRelic(AbstractRelic.RelicTier tier) (L656-659)

**Calls (in order):**
- L657: `logger.info("Returning " + tier.name() + " relic")`
- L657: `tier.name()`
- L658: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier)).makeCopy()`
- L658: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier))`
- L658: `AbstractDungeon.returnRandomRelicKey(tier)`

### returnRandomScreenlessRelic(AbstractRelic.RelicTier tier) (L661-668)

**Calls (in order):**
- L662: `logger.info("Returning " + tier.name() + " relic")`
- L662: `tier.name()`
- L663: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier)).makeCopy()`
- L663: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier))`
- L663: `AbstractDungeon.returnRandomRelicKey(tier)`
- L664: `Objects.equals(tmpRelic.relicId, "Bottled Flame")`
- L664: `Objects.equals(tmpRelic.relicId, "Bottled Lightning")`
- L664: `Objects.equals(tmpRelic.relicId, "Bottled Tornado")`
- L664: `Objects.equals(tmpRelic.relicId, "Whetstone")`
- L665: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier)).makeCopy()`
- L665: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier))`
- L665: `AbstractDungeon.returnRandomRelicKey(tier)`

### returnRandomNonCampfireRelic(AbstractRelic.RelicTier tier) (L670-677)

**Calls (in order):**
- L671: `logger.info("Returning " + tier.name() + " relic")`
- L671: `tier.name()`
- L672: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier)).makeCopy()`
- L672: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier))`
- L672: `AbstractDungeon.returnRandomRelicKey(tier)`
- L673: `Objects.equals(tmpRelic.relicId, "Peace Pipe")`
- L673: `Objects.equals(tmpRelic.relicId, "Shovel")`
- L673: `Objects.equals(tmpRelic.relicId, "Girya")`
- L674: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier)).makeCopy()`
- L674: `RelicLibrary.getRelic(AbstractDungeon.returnRandomRelicKey(tier))`
- L674: `AbstractDungeon.returnRandomRelicKey(tier)`

### returnRandomRelicEnd(AbstractRelic.RelicTier tier) (L679-682)

**Calls (in order):**
- L680: `logger.info("Returning " + tier.name() + " relic")`
- L680: `tier.name()`
- L681: `RelicLibrary.getRelic(AbstractDungeon.returnEndRandomRelicKey(tier)).makeCopy()`
- L681: `RelicLibrary.getRelic(AbstractDungeon.returnEndRandomRelicKey(tier))`
- L681: `AbstractDungeon.returnEndRandomRelicKey(tier)`

### returnEndRandomRelicKey(AbstractRelic.RelicTier tier) (L684-735)

**Calls (in order):**
- L688: `commonRelicPool.isEmpty()`
- L689: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.UNCOMMON)`
- L692: `commonRelicPool.remove(commonRelicPool.size() - 1)`
- L692: `commonRelicPool.size()`
- L696: `uncommonRelicPool.isEmpty()`
- L697: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.RARE)`
- L700: `uncommonRelicPool.remove(uncommonRelicPool.size() - 1)`
- L700: `uncommonRelicPool.size()`
- L704: `rareRelicPool.isEmpty()`
- L708: `rareRelicPool.remove(rareRelicPool.size() - 1)`
- L708: `rareRelicPool.size()`
- L712: `shopRelicPool.isEmpty()`
- L713: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.UNCOMMON)`
- L716: `shopRelicPool.remove(shopRelicPool.size() - 1)`
- L716: `shopRelicPool.size()`
- L720: `bossRelicPool.isEmpty()`
- L724: `bossRelicPool.remove(0)`
- L728: `logger.info("Incorrect relic tier: " + tier.name() + " was called in returnEndRandomRelicKey()")`
- L728: `tier.name()`
- L731: `RelicLibrary.getRelic(retVal).canSpawn()`
- L731: `RelicLibrary.getRelic(retVal)`
- L732: `AbstractDungeon.returnEndRandomRelicKey(tier)`

### returnRandomRelicKey(AbstractRelic.RelicTier tier) (L737-788)

**Calls (in order):**
- L741: `commonRelicPool.isEmpty()`
- L742: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.UNCOMMON)`
- L745: `commonRelicPool.remove(0)`
- L749: `uncommonRelicPool.isEmpty()`
- L750: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.RARE)`
- L753: `uncommonRelicPool.remove(0)`
- L757: `rareRelicPool.isEmpty()`
- L761: `rareRelicPool.remove(0)`
- L765: `shopRelicPool.isEmpty()`
- L766: `AbstractDungeon.returnRandomRelicKey(AbstractRelic.RelicTier.UNCOMMON)`
- L769: `shopRelicPool.remove(0)`
- L773: `bossRelicPool.isEmpty()`
- L777: `bossRelicPool.remove(0)`
- L781: `logger.info("Incorrect relic tier: " + tier.name() + " was called in returnRandomRelicKey()")`
- L781: `tier.name()`
- L784: `RelicLibrary.getRelic(retVal).canSpawn()`
- L784: `RelicLibrary.getRelic(retVal)`
- L785: `AbstractDungeon.returnEndRandomRelicKey(tier)`

### returnRandomRelicTier() (L790-799)

**Calls (in order):**
- L791: `relicRng.random(0, 99)`

### returnTotallyRandomPotion() (L801-803)

**Calls (in order):**
- L802: `PotionHelper.getRandomPotion()`

### returnRandomPotion() (L805-807)

**Calls (in order):**
- L806: `AbstractDungeon.returnRandomPotion(false)`

### returnRandomPotion(boolean limited) (L809-818)

**Calls (in order):**
- L810: `potionRng.random(0, 99)`
- L812: `AbstractDungeon.returnRandomPotion(AbstractPotion.PotionRarity.COMMON, limited)`
- L815: `AbstractDungeon.returnRandomPotion(AbstractPotion.PotionRarity.UNCOMMON, limited)`
- L817: `AbstractDungeon.returnRandomPotion(AbstractPotion.PotionRarity.RARE, limited)`

### returnRandomPotion(AbstractPotion.PotionRarity rarity, boolean limited) (L820-830)

**Calls (in order):**
- L821: `PotionHelper.getRandomPotion()`
- L825: `PotionHelper.getRandomPotion()`

### transformCard(AbstractCard c) (L832-834)

**Calls (in order):**
- L833: `AbstractDungeon.transformCard(c, false)`

### transformCard(AbstractCard c, boolean autoUpgrade) (L836-838)

**Calls (in order):**
- L837: `AbstractDungeon.transformCard(c, autoUpgrade, new Random())`

**Objects created:**
- L837: `Random`

### transformCard(AbstractCard c, boolean autoUpgrade, Random rng) (L840-858)

**Calls (in order):**
- L843: `AbstractDungeon.returnTrulyRandomColorlessCardFromAvailable(c, rng).makeCopy()`
- L843: `AbstractDungeon.returnTrulyRandomColorlessCardFromAvailable(c, rng)`
- L847: `CardLibrary.getCurse(c, rng).makeCopy()`
- L847: `CardLibrary.getCurse(c, rng)`
- L851: `AbstractDungeon.returnTrulyRandomCardFromAvailable(c, rng).makeCopy()`
- L851: `AbstractDungeon.returnTrulyRandomCardFromAvailable(c, rng)`
- L854: `UnlockTracker.markCardAsSeen(AbstractDungeon.transformedCard.cardID)`
- L855: `transformedCard.canUpgrade()`
- L856: `transformedCard.upgrade()`

### srcTransformCard(AbstractCard c) (L860-893)

**Calls (in order):**
- L861: `logger.info("Transform using SRC pool...")`
- L864: `srcCommonCardPool.getRandomCard(false).makeCopy()`
- L864: `srcCommonCardPool.getRandomCard(false)`
- L868: `srcCommonCardPool.removeCard(c.cardID)`
- L869: `srcCommonCardPool.getRandomCard(false).makeCopy()`
- L869: `srcCommonCardPool.getRandomCard(false)`
- L870: `srcCommonCardPool.addToTop(c.makeCopy())`
- L870: `c.makeCopy()`
- L874: `srcUncommonCardPool.removeCard(c.cardID)`
- L875: `srcUncommonCardPool.getRandomCard(false).makeCopy()`
- L875: `srcUncommonCardPool.getRandomCard(false)`
- L876: `srcUncommonCardPool.addToTop(c.makeCopy())`
- L876: `c.makeCopy()`
- L880: `srcRareCardPool.removeCard(c.cardID)`
- L881: `srcRareCardPool.isEmpty()`
- L881: `srcUncommonCardPool.getRandomCard(false).makeCopy()`
- L881: `srcUncommonCardPool.getRandomCard(false)`
- L881: `srcRareCardPool.getRandomCard(false).makeCopy()`
- L881: `srcRareCardPool.getRandomCard(false)`
- L882: `srcRareCardPool.addToTop(c.makeCopy())`
- L882: `c.makeCopy()`
- L886: `srcRareCardPool.isEmpty()`
- L886: `srcRareCardPool.getRandomCard(false).makeCopy()`
- L886: `srcRareCardPool.getRandomCard(false)`
- L886: `srcUncommonCardPool.getRandomCard(false).makeCopy()`
- L886: `srcUncommonCardPool.getRandomCard(false)`
- L889: `logger.info("Transform called on a strange card type: " + c.type.name())`
- L889: `c.type.name()`
- L890: `srcCommonCardPool.getRandomCard(false).makeCopy()`
- L890: `srcCommonCardPool.getRandomCard(false)`

### getEachRare() (L895-901)

**Calls (in order):**
- L898: `everyRareCard.addToBottom(c.makeCopy())`
- L898: `c.makeCopy()`

**Objects created:**
- L896: `CardGroup`

### returnRandomCard() (L903-914)

**Calls (in order):**
- L905: `AbstractDungeon.rollRarity()`
- L906: `rarity.equals((Object)AbstractCard.CardRarity.COMMON)`
- L907: `list.addAll(AbstractDungeon.srcCommonCardPool.group)`
- L908: `rarity.equals((Object)AbstractCard.CardRarity.UNCOMMON)`
- L909: `list.addAll(AbstractDungeon.srcUncommonCardPool.group)`
- L911: `list.addAll(AbstractDungeon.srcRareCardPool.group)`
- L913: `list.get(cardRandomRng.random(list.size() - 1))`
- L913: `cardRandomRng.random(list.size() - 1)`
- L913: `list.size()`

**Objects created:**
- L904: `None`

### returnTrulyRandomCard() (L916-922)

**Calls (in order):**
- L918: `list.addAll(AbstractDungeon.srcCommonCardPool.group)`
- L919: `list.addAll(AbstractDungeon.srcUncommonCardPool.group)`
- L920: `list.addAll(AbstractDungeon.srcRareCardPool.group)`
- L921: `list.get(cardRandomRng.random(list.size() - 1))`
- L921: `cardRandomRng.random(list.size() - 1)`
- L921: `list.size()`

**Objects created:**
- L917: `None`

### returnTrulyRandomCardInCombat() (L924-942)

**Calls (in order):**
- L927: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L928: `list.add(c)`
- L929: `UnlockTracker.markCardAsSeen(c.cardID)`
- L932: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L933: `list.add(c)`
- L934: `UnlockTracker.markCardAsSeen(c.cardID)`
- L937: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L938: `list.add(c)`
- L939: `UnlockTracker.markCardAsSeen(c.cardID)`
- L941: `list.get(cardRandomRng.random(list.size() - 1))`
- L941: `cardRandomRng.random(list.size() - 1)`
- L941: `list.size()`

**Objects created:**
- L925: `None`

### returnTrulyRandomCardInCombat(AbstractCard.CardType type) (L944-959)

**Calls (in order):**
- L947: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L948: `list.add(c)`
- L951: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L952: `list.add(c)`
- L955: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L956: `list.add(c)`
- L958: `list.get(cardRandomRng.random(list.size() - 1))`
- L958: `cardRandomRng.random(list.size() - 1)`
- L958: `list.size()`

**Objects created:**
- L945: `None`

### returnTrulyRandomColorlessCardInCombat() (L961-963)

**Calls (in order):**
- L962: `AbstractDungeon.returnTrulyRandomColorlessCardInCombat(cardRandomRng)`

### returnTrulyRandomColorlessCardInCombat(String prohibitedID) (L965-967)

**Calls (in order):**
- L966: `AbstractDungeon.returnTrulyRandomColorlessCardFromAvailable(prohibitedID, cardRandomRng)`

### returnTrulyRandomColorlessCardInCombat(Random rng) (L969-976)

**Calls (in order):**
- L972: `c.hasTag(AbstractCard.CardTags.HEALING)`
- L973: `list.add(c)`
- L975: `list.get(rng.random(list.size() - 1))`
- L975: `rng.random(list.size() - 1)`
- L975: `list.size()`

**Objects created:**
- L970: `None`

### returnTrulyRandomColorlessCardFromAvailable(String prohibited, Random rng) (L978-985)

**Calls (in order):**
- L982: `list.add(c)`
- L984: `list.get(rng.random(list.size() - 1))`
- L984: `rng.random(list.size() - 1)`
- L984: `list.size()`

**Objects created:**
- L979: `None`

### returnTrulyRandomColorlessCardFromAvailable(AbstractCard prohibited, Random rng) (L987-994)

**Calls (in order):**
- L990: `Objects.equals(c.cardID, prohibited.cardID)`
- L991: `list.add(c)`
- L993: `list.get(rng.random(list.size() - 1))`
- L993: `rng.random(list.size() - 1)`
- L993: `list.size()`

**Objects created:**
- L988: `None`

### returnTrulyRandomCardFromAvailable(AbstractCard prohibited, Random rng) (L996-1025)

**Calls (in order):**
- L1001: `Objects.equals(c.cardID, prohibited.cardID)`
- L1002: `list.add(c)`
- L1007: `CardLibrary.getCurse()`
- L1011: `Objects.equals(c.cardID, prohibited.cardID)`
- L1012: `list.add(c)`
- L1015: `Objects.equals(c.cardID, prohibited.cardID)`
- L1016: `list.add(c)`
- L1019: `Objects.equals(c.cardID, prohibited.cardID)`
- L1020: `list.add(c)`
- L1024: `((AbstractCard)list.get(rng.random(list.size() - 1))).makeCopy()`
- L1024: `list.get(rng.random(list.size() - 1))`
- L1024: `rng.random(list.size() - 1)`
- L1024: `list.size()`

**Objects created:**
- L997: `None`

### returnTrulyRandomCardFromAvailable(AbstractCard prohibited) (L1027-1029)

**Calls (in order):**
- L1028: `AbstractDungeon.returnTrulyRandomCardFromAvailable(prohibited, new Random())`

**Objects created:**
- L1028: `Random`

### populateFirstStrongEnemy(ArrayList<MonsterInfo> monsters, ArrayList<String> exclusions) (L1037-1042)

**Calls (in order):**
- L1039: `exclusions.contains(m = MonsterInfo.roll(monsters, monsterRng.random()))`
- L1039: `MonsterInfo.roll(monsters, monsterRng.random())`
- L1039: `monsterRng.random()`
- L1041: `monsterList.add(m)`

### populateMonsterList(ArrayList<MonsterInfo> monsters, int numMonsters, boolean elites) (L1044-1076)

**Calls (in order):**
- L1047: `eliteMonsterList.isEmpty()`
- L1048: `eliteMonsterList.add(MonsterInfo.roll(monsters, monsterRng.random()))`
- L1048: `MonsterInfo.roll(monsters, monsterRng.random())`
- L1048: `monsterRng.random()`
- L1051: `MonsterInfo.roll(monsters, monsterRng.random())`
- L1051: `monsterRng.random()`
- L1052: `toAdd.equals(eliteMonsterList.get(eliteMonsterList.size() - 1))`
- L1052: `eliteMonsterList.get(eliteMonsterList.size() - 1)`
- L1052: `eliteMonsterList.size()`
- L1053: `eliteMonsterList.add(toAdd)`
- L1060: `monsterList.isEmpty()`
- L1061: `monsterList.add(MonsterInfo.roll(monsters, monsterRng.random()))`
- L1061: `MonsterInfo.roll(monsters, monsterRng.random())`
- L1061: `monsterRng.random()`
- L1064: `MonsterInfo.roll(monsters, monsterRng.random())`
- L1064: `monsterRng.random()`
- L1065: `toAdd.equals(monsterList.get(monsterList.size() - 1))`
- L1065: `monsterList.get(monsterList.size() - 1)`
- L1065: `monsterList.size()`
- L1066: `monsterList.size()`
- L1066: `toAdd.equals(monsterList.get(monsterList.size() - 2))`
- L1066: `monsterList.get(monsterList.size() - 2)`
- L1066: `monsterList.size()`
- L1070: `monsterList.add(toAdd)`

### returnColorlessCard(AbstractCard.CardRarity rarity) (L1080-1093)

**Calls (in order):**
- L1081: `Collections.shuffle(AbstractDungeon.colorlessCardPool.group, new java.util.Random(shuffleRng.randomLong()))`
- L1081: `shuffleRng.randomLong()`
- L1084: `c.makeCopy()`
- L1089: `c.makeCopy()`

**Objects created:**
- L1081: `None`
- L1092: `SwiftStrike`

### returnColorlessCard() (L1095-1103)

**Calls (in order):**
- L1096: `Collections.shuffle(AbstractDungeon.colorlessCardPool.group)`
- L1097: `AbstractDungeon.colorlessCardPool.group.iterator()`
- L1098: `iterator.hasNext()`
- L1099: `iterator.next()`
- L1100: `c.makeCopy()`

**Objects created:**
- L1102: `SwiftStrike`

### returnRandomCurse() (L1105-1109)

**Calls (in order):**
- L1106: `CardLibrary.getCurse().makeCopy()`
- L1106: `CardLibrary.getCurse()`
- L1107: `UnlockTracker.markCardAsSeen(c.cardID)`

### initializePotions() (L1111-1113)

**Calls (in order):**
- L1112: `PotionHelper.initialize(AbstractDungeon.player.chosenClass)`

### initializeCardPools() (L1115-1181)

**Calls (in order):**
- L1116: `logger.info("INIT CARD POOL")`
- L1117: `System.currentTimeMillis()`
- L1118: `commonCardPool.clear()`
- L1119: `uncommonCardPool.clear()`
- L1120: `rareCardPool.clear()`
- L1121: `colorlessCardPool.clear()`
- L1122: `curseCardPool.clear()`
- L1124: `ModHelper.isModEnabled("Colorless Cards")`
- L1125: `CardLibrary.addColorlessCards(tmpPool)`
- L1127: `ModHelper.isModEnabled("Diverse")`
- L1128: `CardLibrary.addRedCards(tmpPool)`
- L1129: `CardLibrary.addGreenCards(tmpPool)`
- L1130: `CardLibrary.addBlueCards(tmpPool)`
- L1131: `UnlockTracker.isCharacterLocked("Watcher")`
- L1132: `CardLibrary.addPurpleCards(tmpPool)`
- L1135: `player.getCardPool(tmpPool)`
- L1137: `this.addColorlessCards()`
- L1138: `this.addCurseCards()`
- L1142: `commonCardPool.addToTop(c)`
- L1146: `uncommonCardPool.addToTop(c)`
- L1150: `rareCardPool.addToTop(c)`
- L1154: `curseCardPool.addToTop(c)`
- L1158: `logger.info("Unspecified rarity: " + c.rarity.name() + " when creating pools! AbstractDungeon: Line 827")`
- L1158: `c.rarity.name()`
- L1166: `srcColorlessCardPool.addToBottom(c)`
- L1169: `srcCurseCardPool.addToBottom(c)`
- L1172: `srcRareCardPool.addToBottom(c)`
- L1175: `srcUncommonCardPool.addToBottom(c)`
- L1178: `srcCommonCardPool.addToBottom(c)`
- L1180: `logger.info("Cardpool load time: " + (System.currentTimeMillis() - startTime) + "ms")`
- L1180: `System.currentTimeMillis()`

**Objects created:**
- L1123: `None`
- L1160: `CardGroup`
- L1161: `CardGroup`
- L1162: `CardGroup`
- L1163: `CardGroup`
- L1164: `CardGroup`

### addColorlessCards() (L1183-1190)

**Calls (in order):**
- L1184: `CardLibrary.cards.entrySet()`
- L1185: `c.getValue()`
- L1187: `colorlessCardPool.addToTop(card)`
- L1189: `logger.info("COLORLESS CARDS: " + colorlessCardPool.size())`
- L1189: `colorlessCardPool.size()`

### addCurseCards() (L1192-1199)

**Calls (in order):**
- L1193: `CardLibrary.cards.entrySet()`
- L1194: `c.getValue()`
- L1195: `Objects.equals(card.cardID, "Necronomicurse")`
- L1195: `Objects.equals(card.cardID, "AscendersBane")`
- L1195: `Objects.equals(card.cardID, "CurseOfTheBell")`
- L1195: `Objects.equals(card.cardID, "Pride")`
- L1196: `curseCardPool.addToTop(card)`
- L1198: `logger.info("CURSE CARDS: " + curseCardPool.size())`
- L1198: `curseCardPool.size()`

### initializeRelicList() (L1201-1302)

**Calls (in order):**
- L1202: `commonRelicPool.clear()`
- L1203: `uncommonRelicPool.clear()`
- L1204: `rareRelicPool.clear()`
- L1205: `shopRelicPool.clear()`
- L1206: `bossRelicPool.clear()`
- L1207: `RelicLibrary.populateRelicPool(commonRelicPool, AbstractRelic.RelicTier.COMMON, AbstractDungeon.player.chosenClass)`
- L1208: `RelicLibrary.populateRelicPool(uncommonRelicPool, AbstractRelic.RelicTier.UNCOMMON, AbstractDungeon.player.chosenClass)`
- L1209: `RelicLibrary.populateRelicPool(rareRelicPool, AbstractRelic.RelicTier.RARE, AbstractDungeon.player.chosenClass)`
- L1210: `RelicLibrary.populateRelicPool(shopRelicPool, AbstractRelic.RelicTier.SHOP, AbstractDungeon.player.chosenClass)`
- L1211: `RelicLibrary.populateRelicPool(bossRelicPool, AbstractRelic.RelicTier.BOSS, AbstractDungeon.player.chosenClass)`
- L1214: `relicsToRemoveOnStart.add(r.relicId)`
- L1217: `Collections.shuffle(commonRelicPool, new java.util.Random(relicRng.randomLong()))`
- L1217: `relicRng.randomLong()`
- L1218: `Collections.shuffle(uncommonRelicPool, new java.util.Random(relicRng.randomLong()))`
- L1218: `relicRng.randomLong()`
- L1219: `Collections.shuffle(rareRelicPool, new java.util.Random(relicRng.randomLong()))`
- L1219: `relicRng.randomLong()`
- L1220: `Collections.shuffle(shopRelicPool, new java.util.Random(relicRng.randomLong()))`
- L1220: `relicRng.randomLong()`
- L1221: `Collections.shuffle(bossRelicPool, new java.util.Random(relicRng.randomLong()))`
- L1221: `relicRng.randomLong()`
- L1222: `ModHelper.isModEnabled("Flight")`
- L1222: `ModHelper.isModEnabled("Uncertain Future")`
- L1223: `relicsToRemoveOnStart.add("WingedGreaves")`
- L1225: `ModHelper.isModEnabled("Diverse")`
- L1226: `relicsToRemoveOnStart.add("PrismaticShard")`
- L1228: `ModHelper.isModEnabled("DeadlyEvents")`
- L1229: `relicsToRemoveOnStart.add("Juzu Bracelet")`
- L1231: `ModHelper.isModEnabled("Hoarder")`
- L1232: `relicsToRemoveOnStart.add("Smiling Mask")`
- L1234: `ModHelper.isModEnabled("Draft")`
- L1234: `ModHelper.isModEnabled("SealedDeck")`
- L1234: `ModHelper.isModEnabled("Shiny")`
- L1234: `ModHelper.isModEnabled("Insanity")`
- L1235: `relicsToRemoveOnStart.add("Pandora's Box")`
- L1239: `commonRelicPool.iterator()`
- L1240: `s.hasNext()`
- L1241: `s.next()`
- L1242: `derp.equals(remove)`
- L1243: `s.remove()`
- L1244: `logger.info(derp + " removed.")`
- L1247: `uncommonRelicPool.iterator()`
- L1248: `s.hasNext()`
- L1249: `s.next()`
- L1250: `derp.equals(remove)`
- L1251: `s.remove()`
- L1252: `logger.info(derp + " removed.")`
- L1255: `rareRelicPool.iterator()`
- L1256: `s.hasNext()`
- L1257: `s.next()`
- ... (25 more)

**Objects created:**
- L1217: `None`
- L1218: `None`
- L1219: `None`
- L1220: `None`
- L1221: `None`

### initializeSpecialOneTimeEventList() (L1320-1338)

**Calls (in order):**
- L1321: `specialOneTimeEventList.clear()`
- L1322: `specialOneTimeEventList.add("Accursed Blacksmith")`
- L1323: `specialOneTimeEventList.add("Bonfire Elementals")`
- L1324: `specialOneTimeEventList.add("Designer")`
- L1325: `specialOneTimeEventList.add("Duplicator")`
- L1326: `specialOneTimeEventList.add("FaceTrader")`
- L1327: `specialOneTimeEventList.add("Fountain of Cleansing")`
- L1328: `specialOneTimeEventList.add("Knowing Skull")`
- L1329: `specialOneTimeEventList.add("Lab")`
- L1330: `specialOneTimeEventList.add("N'loth")`
- L1331: `this.isNoteForYourselfAvailable()`
- L1332: `specialOneTimeEventList.add("NoteForYourself")`
- L1334: `specialOneTimeEventList.add("SecretPortal")`
- L1335: `specialOneTimeEventList.add("The Joust")`
- L1336: `specialOneTimeEventList.add("WeMeetAgain")`
- L1337: `specialOneTimeEventList.add("The Woman in Blue")`

### isNoteForYourselfAvailable() (L1340-1359)

**Calls (in order):**
- L1342: `logger.info("Note For Yourself is disabled due to Daily Run")`
- L1346: `logger.info("Note For Yourself is disabled beyond Ascension 15+")`
- L1350: `logger.info("Note For Yourself is enabled due to No Ascension")`
- L1353: `player.getPrefs().getInteger("ASCENSION_LEVEL")`
- L1353: `player.getPrefs()`
- L1354: `logger.info("Note For Yourself is enabled as it's less than Highest Unlocked Ascension")`
- L1357: `logger.info("Note For Yourself is disabled as requirements aren't met")`

### getColorlessRewardCards() (L1361-1401)

**Calls (in order):**
- L1365: `r.changeNumberOfCardsInReward(numCards)`
- L1367: `ModHelper.isModEnabled("Binary")`
- L1371: `AbstractDungeon.rollRareOrUncommon(colorlessRareChance)`
- L1375: `AbstractDungeon.getColorlessCardFromPool(rarity)`
- L1380: `AbstractDungeon.getColorlessCardFromPool(rarity)`
- L1384: `logger.info("WTF?")`
- L1387: `retVal.contains(card)`
- L1389: `logger.info("DUPE: " + card.originalName)`
- L1391: `AbstractDungeon.getColorlessCardFromPool(rarity)`
- L1394: `retVal.add(card)`
- L1398: `retVal2.add(c.makeCopy())`
- L1398: `c.makeCopy()`

**Objects created:**
- L1362: `None`
- L1396: `None`

### getRewardCards() (L1403-1459)

**Calls (in order):**
- L1407: `r.changeNumberOfCardsInReward(numCards)`
- L1409: `ModHelper.isModEnabled("Binary")`
- L1413: `AbstractDungeon.rollRarity()`
- L1429: `logger.info("WTF?")`
- L1435: `player.hasRelic("PrismaticShard")`
- L1435: `CardLibrary.getAnyColorCard(rarity)`
- L1435: `AbstractDungeon.getCard(rarity)`
- L1437: `c.cardID.equals(card.cardID)`
- L1443: `retVal.add(card)`
- L1447: `retVal2.add(c.makeCopy())`
- L1447: `c.makeCopy()`
- L1450: `cardRng.randomBoolean(cardUpgradedChance)`
- L1450: `c.canUpgrade()`
- L1451: `c.upgrade()`
- L1455: `r.onPreviewObtainCard(c)`

**Objects created:**
- L1404: `None`
- L1445: `None`

### getCard(AbstractCard.CardRarity rarity) (L1461-1478)

**Calls (in order):**
- L1464: `rareCardPool.getRandomCard(true)`
- L1467: `uncommonCardPool.getRandomCard(true)`
- L1470: `commonCardPool.getRandomCard(true)`
- L1473: `curseCardPool.getRandomCard(true)`
- L1476: `logger.info("No rarity on getCard in Abstract Dungeon")`

### getCard(AbstractCard.CardRarity rarity, Random rng) (L1480-1497)

**Calls (in order):**
- L1483: `rareCardPool.getRandomCard(rng)`
- L1486: `uncommonCardPool.getRandomCard(rng)`
- L1489: `commonCardPool.getRandomCard(rng)`
- L1492: `curseCardPool.getRandomCard(rng)`
- L1495: `logger.info("No rarity on getCard in Abstract Dungeon")`

### getCardWithoutRng(AbstractCard.CardRarity rarity) (L1499-1516)

**Calls (in order):**
- L1502: `rareCardPool.getRandomCard(false)`
- L1505: `uncommonCardPool.getRandomCard(false)`
- L1508: `commonCardPool.getRandomCard(false)`
- L1511: `AbstractDungeon.returnRandomCurse()`
- L1514: `logger.info("Check getCardWithoutRng")`

### getCardFromPool(AbstractCard.CardRarity rarity, AbstractCard.CardType type, boolean useRng) (L1518-1557)

**Calls (in order):**
- L1521: `rareCardPool.getRandomCard(type, useRng)`
- L1525: `logger.info("ERROR: Could not find Rare card of type: " + type.name())`
- L1525: `type.name()`
- L1528: `uncommonCardPool.getRandomCard(type, useRng)`
- L1533: `AbstractDungeon.getCardFromPool(AbstractCard.CardRarity.RARE, type, useRng)`
- L1535: `logger.info("ERROR: Could not find Uncommon card of type: " + type.name())`
- L1535: `type.name()`
- L1538: `commonCardPool.getRandomCard(type, useRng)`
- L1543: `AbstractDungeon.getCardFromPool(AbstractCard.CardRarity.UNCOMMON, type, useRng)`
- L1545: `logger.info("ERROR: Could not find Common card of type: " + type.name())`
- L1545: `type.name()`
- L1548: `curseCardPool.getRandomCard(type, useRng)`
- L1552: `logger.info("ERROR: Could not find Curse card of type: " + type.name())`
- L1552: `type.name()`
- L1555: `logger.info("ERROR: Default in getCardFromPool")`

### getColorlessCardFromPool(AbstractCard.CardRarity rarity) (L1559-1575)

**Calls (in order):**
- L1562: `colorlessCardPool.getRandomCard(true, rarity)`
- L1568: `colorlessCardPool.getRandomCard(true, rarity)`
- L1573: `logger.info("ERROR: getColorlessCardFromPool")`

### rollRarity(Random rng) (L1577-1584)

**Calls (in order):**
- L1578: `cardRng.random(99)`
- L1581: `AbstractDungeon.getCardRarityFallback(roll)`
- L1583: `AbstractDungeon.getCurrRoom().getCardRarity(roll)`
- L1583: `AbstractDungeon.getCurrRoom()`

### rollRarity() (L1597-1599)

**Calls (in order):**
- L1598: `AbstractDungeon.rollRarity(cardRng)`

### rollRareOrUncommon(float rareChance) (L1601-1606)

**Calls (in order):**
- L1602: `cardRng.randomBoolean(rareChance)`

### getRandomMonster() (L1608-1610)

**Calls (in order):**
- L1609: `AbstractDungeon.currMapNode.room.monsters.getRandomMonster(null, true, cardRandomRng)`

### getRandomMonster(AbstractMonster except) (L1612-1614)

**Calls (in order):**
- L1613: `AbstractDungeon.currMapNode.room.monsters.getRandomMonster(except, true, cardRandomRng)`

### nextRoomTransitionStart() (L1616-1623)

**Calls (in order):**
- L1617: `AbstractDungeon.fadeOut()`
- L1619: `AbstractDungeon.overlayMenu.proceedButton.hide()`
- L1620: `ModHelper.isModEnabled("Terminal")`
- L1621: `player.decreaseMaxHealth(1)`

### initializeFirstRoom() (L1625-1641)

**Calls (in order):**
- L1626: `AbstractDungeon.fadeIn()`
- L1630: `SaveHelper.shouldSave()`
- L1631: `SaveHelper.saveIfAppropriate(SaveFile.SaveType.ENTER_ROOM)`
- L1634: `metrics.setValues(false, false, null, Metrics.MetricRequestType.NONE)`
- L1635: `metrics.gatherAllDataAndSave(false, false, null)`
- L1640: `scene.nextRoom(AbstractDungeon.currMapNode.room)`

**Objects created:**
- L1633: `Metrics`

### resetPlayer() (L1643-1661)

**Calls (in order):**
- L1644: `AbstractDungeon.player.orbs.clear()`
- L1647: `player.hideHealthBar()`
- L1648: `AbstractDungeon.player.hand.clear()`
- L1649: `AbstractDungeon.player.powers.clear()`
- L1650: `AbstractDungeon.player.drawPile.clear()`
- L1651: `AbstractDungeon.player.discardPile.clear()`
- L1652: `AbstractDungeon.player.exhaustPile.clear()`
- L1653: `AbstractDungeon.player.limbo.clear()`
- L1654: `player.loseBlock(true)`
- L1656: `AbstractDungeon.player.stance.ID.equals("Neutral")`
- L1658: `player.onStanceChange("Neutral")`

**Objects created:**
- L1657: `NeutralStance`

### nextRoomTransition() (L1663-1665)

**Calls (in order):**
- L1664: `this.nextRoomTransition(null)`

### nextRoomTransition(SaveFile saveFile) (L1667-1793)

**Calls (in order):**
- L1669: `AbstractDungeon.overlayMenu.proceedButton.setLabel(TEXT[0])`
- L1670: `combatRewardScreen.clear()`
- L1672: `AbstractDungeon.nextRoom.room.rewards.clear()`
- L1674: `AbstractDungeon.getCurrRoom()`
- L1675: `eliteMonsterList.isEmpty()`
- L1676: `logger.info("Removing elite: " + eliteMonsterList.get(0) + " from monster list.")`
- L1676: `eliteMonsterList.get(0)`
- L1677: `eliteMonsterList.remove(0)`
- L1679: `this.generateElites(10)`
- L1681: `AbstractDungeon.getCurrRoom()`
- L1682: `monsterList.isEmpty()`
- L1683: `logger.info("Removing monster: " + monsterList.get(0) + " from monster list.")`
- L1683: `monsterList.get(0)`
- L1684: `monsterList.remove(0)`
- L1686: `this.generateStrongEnemies(12)`
- L1688: `AbstractDungeon.getCurrRoom()`
- L1688: `AbstractDungeon.getCurrRoom()`
- L1688: `AbstractDungeon.getCurrRoom()`
- L1689: `CardCrawlGame.playerPref.putString("NOTE_CARD", tmpCard.cardID)`
- L1690: `CardCrawlGame.playerPref.putInteger("NOTE_UPGRADE", tmpCard.timesUpgraded)`
- L1691: `CardCrawlGame.playerPref.flush()`
- L1694: `CardCrawlGame.sound.fadeOut("REST_FIRE_WET", RestRoom.lastFireSoundId)`
- L1696: `AbstractDungeon.player.stance.ID.equals("Neutral")`
- L1697: `AbstractDungeon.player.stance.stopIdleSfx()`
- L1701: `dynamicBanner.hide()`
- L1702: `dungeonMapScreen.closeInstantly()`
- L1703: `AbstractDungeon.closeCurrentScreen()`
- L1704: `topPanel.unhoverHitboxes()`
- L1705: `AbstractDungeon.fadeIn()`
- L1706: `player.resetControllerValues()`
- L1707: `effectList.clear()`
- L1708: `topLevelEffects.iterator()`
- L1709: `i.hasNext()`
- L1710: `i.next()`
- L1712: `i.remove()`
- L1714: `topLevelEffectsQueue.clear()`
- L1715: `effectsQueue.clear()`
- L1718: `AbstractDungeon.resetPlayer()`
- L1720: `this.incrementFloorBasedMetrics()`
- L1721: `TipTracker.tips.get("INTENT_TIP").booleanValue()`
- L1721: `TipTracker.tips.get("INTENT_TIP")`
- L1722: `TipTracker.neverShowAgain("INTENT_TIP")`
- L1724: `StatsScreen.incrementFloorClimbed()`
- L1725: `SaveHelper.saveIfAppropriate(SaveFile.SaveType.ENTER_ROOM)`
- L1736: `r.onEnterRoom(AbstractDungeon.nextRoom.room)`
- L1739: `AbstractDungeon.actionManager.actions.isEmpty()`
- L1740: `logger.info("[WARNING] Line:1904: Action Manager was NOT clear! Clearing")`
- L1741: `actionManager.clear()`
- L1744: `AbstractDungeon.nextRoom.room.getMapSymbol()`
- L1747: `EventHelper.roll(eventRngDuplicate)`
- ... (27 more)

**Objects created:**
- L1727: `Random`
- L1728: `Random`
- L1729: `Random`
- L1730: `Random`
- L1731: `Random`
- L1746: `Random`

### incrementFloorBasedMetrics() (L1795-1801)

**Calls (in order):**
- L1797: `CardCrawlGame.metricData.current_hp_per_floor.add(AbstractDungeon.player.currentHealth)`
- L1798: `CardCrawlGame.metricData.max_hp_per_floor.add(AbstractDungeon.player.maxHealth)`
- L1799: `CardCrawlGame.metricData.gold_per_floor.add(AbstractDungeon.player.gold)`

### generateRoom(EventHelper.RoomResult roomType) (L1803-1820)

**Calls (in order):**
- L1804: `logger.info("GENERATING ROOM: " + roomType.name())`
- L1804: `roomType.name()`

**Objects created:**
- L1807: `MonsterRoomElite`
- L1810: `MonsterRoom`
- L1813: `ShopRoom`
- L1816: `TreasureRoom`
- L1819: `EventRoom`

### getMonsters() (L1822-1824)

**Calls (in order):**
- L1823: `AbstractDungeon.getCurrRoom()`

### getMonsterForRoomCreation() (L1826-1833)

**Calls (in order):**
- L1827: `monsterList.isEmpty()`
- L1828: `this.generateStrongEnemies(12)`
- L1830: `logger.info("MONSTER: " + monsterList.get(0))`
- L1830: `monsterList.get(0)`
- L1831: `monsterList.get(0)`
- L1832: `MonsterHelper.getEncounter(monsterList.get(0))`
- L1832: `monsterList.get(0)`

### getEliteMonsterForRoomCreation() (L1835-1842)

**Calls (in order):**
- L1836: `eliteMonsterList.isEmpty()`
- L1837: `this.generateElites(10)`
- L1839: `logger.info("ELITE: " + eliteMonsterList.get(0))`
- L1839: `eliteMonsterList.get(0)`
- L1840: `eliteMonsterList.get(0)`
- L1841: `MonsterHelper.getEncounter(eliteMonsterList.get(0))`
- L1841: `eliteMonsterList.get(0)`

### generateEvent(Random rng) (L1844-1860)

**Calls (in order):**
- L1845: `rng.random(1.0f)`
- L1846: `shrineList.isEmpty()`
- L1846: `specialOneTimeEventList.isEmpty()`
- L1847: `AbstractDungeon.getShrine(rng)`
- L1849: `eventList.isEmpty()`
- L1850: `AbstractDungeon.getEvent(rng)`
- L1852: `logger.info("No events or shrines left")`
- L1855: `AbstractDungeon.getEvent(rng)`
- L1857: `AbstractDungeon.getShrine(rng)`

### getShrine(Random rng) (L1862-1922)

**Calls (in order):**
- L1864: `tmp.addAll(shrineList)`
- L1865: `specialOneTimeEventList.iterator()`
- L1866: `iterator.hasNext()`
- L1868: `iterator.next()`
- L1870: `player.isCursed()`
- L1871: `tmp.add(e)`
- L1875: `id.equals("TheCity")`
- L1875: `id.equals("TheBeyond")`
- L1876: `tmp.add(e)`
- L1880: `id.equals("TheCity")`
- L1880: `id.equals("TheBeyond")`
- L1881: `tmp.add(e)`
- L1885: `id.equals("TheCity")`
- L1885: `id.equals("Exordium")`
- L1886: `tmp.add(e)`
- L1890: `id.equals("TheCity")`
- L1891: `tmp.add(e)`
- L1895: `id.equals("TheCity")`
- L1895: `id.equals("TheCity")`
- L1895: `AbstractDungeon.player.relics.size()`
- L1896: `tmp.add(e)`
- L1900: `id.equals("TheCity")`
- L1901: `tmp.add(e)`
- L1906: `tmp.add(e)`
- L1910: `id.equals("TheBeyond")`
- L1911: `tmp.add(e)`
- L1915: `tmp.add(e)`
- L1917: `tmp.get(rng.random(tmp.size() - 1))`
- L1917: `rng.random(tmp.size() - 1)`
- L1917: `tmp.size()`
- L1918: `shrineList.remove(tmpKey)`
- L1919: `specialOneTimeEventList.remove(tmpKey)`
- L1920: `logger.info("Removed event: " + tmpKey + " from pool.")`
- L1921: `EventHelper.getEvent(tmpKey)`

**Objects created:**
- L1863: `None`

### getEvent(Random rng) (L1924-1970)

**Calls (in order):**
- L1926: `eventList.iterator()`
- L1927: `iterator.hasNext()`
- L1929: `iterator.next()`
- L1932: `tmp.add(e)`
- L1937: `tmp.add(e)`
- L1941: `player.hasRelic("Golden Idol")`
- L1942: `tmp.add(e)`
- L1947: `tmp.add(e)`
- L1952: `tmp.add(e)`
- L1956: `map.size()`
- L1957: `tmp.add(e)`
- L1961: `tmp.add(e)`
- L1963: `tmp.isEmpty()`
- L1964: `AbstractDungeon.getShrine(rng)`
- L1966: `tmp.get(rng.random(tmp.size() - 1))`
- L1966: `rng.random(tmp.size() - 1)`
- L1966: `tmp.size()`
- L1967: `eventList.remove(tmpKey)`
- L1968: `logger.info("Removed event: " + tmpKey + " from pool.")`
- L1969: `EventHelper.getEvent(tmpKey)`

**Objects created:**
- L1925: `None`

### getBoss() (L1972-1976)

**Calls (in order):**
- L1975: `MonsterHelper.getEncounter(bossKey)`

### update() (L1978-2131)

**Calls (in order):**
- L1981: `Gdx.graphics.getDeltaTime()`
- L1985: `CInputActionSet.select.unpress()`
- L1987: `topPanel.update()`
- L1988: `dynamicBanner.update()`
- L1989: `this.updateFading()`
- L1990: `AbstractDungeon.currMapNode.room.updateObjects()`
- L1992: `MathHelper.fadeLerpSnap(AbstractDungeon.topGradientColor.a, 0.25f)`
- L1993: `MathHelper.fadeLerpSnap(AbstractDungeon.botGradientColor.a, 0.25f)`
- L1995: `MathHelper.fadeLerpSnap(AbstractDungeon.topGradientColor.a, 0.1f)`
- L1996: `MathHelper.fadeLerpSnap(AbstractDungeon.botGradientColor.a, 0.1f)`
- L2001: `dungeonMapScreen.update()`
- L2002: `AbstractDungeon.currMapNode.room.update()`
- L2003: `scene.update()`
- L2004: `AbstractDungeon.currMapNode.room.eventControllerInput()`
- L2008: `ftue.update()`
- L2011: `AbstractDungeon.currMapNode.room.update()`
- L2015: `deckViewScreen.update()`
- L2019: `gameDeckViewScreen.update()`
- L2023: `discardPileViewScreen.update()`
- L2027: `exhaustPileViewScreen.update()`
- L2031: `settingsScreen.update()`
- L2035: `inputSettingsScreen.update()`
- L2039: `dungeonMapScreen.update()`
- L2043: `gridSelectScreen.update()`
- L2045: `AbstractDungeon.currMapNode.room.update()`
- L2049: `cardRewardScreen.update()`
- L2051: `AbstractDungeon.currMapNode.room.update()`
- L2055: `combatRewardScreen.update()`
- L2059: `bossRelicScreen.update()`
- L2060: `AbstractDungeon.currMapNode.room.update()`
- L2064: `handCardSelectScreen.update()`
- L2065: `AbstractDungeon.currMapNode.room.update()`
- L2069: `shopScreen.update()`
- L2073: `deathScreen.update()`
- L2077: `victoryScreen.update()`
- L2081: `unlockScreen.update()`
- L2085: `gUnlockScreen.update()`
- L2089: `creditsScreen.update()`
- L2093: `CardCrawlGame.mainMenuScreen.doorUnlockScreen.update()`
- L2097: `logger.info("ERROR: UNKNOWN SCREEN TO UPDATE: " + screen.name())`
- L2097: `screen.name()`
- L2101: `topLevelEffects.iterator()`
- L2102: `i.hasNext()`
- L2103: `i.next()`
- L2104: `e.update()`
- L2106: `i.remove()`
- L2108: `effectList.iterator()`
- L2109: `i.hasNext()`
- L2110: `i.next()`
- L2111: `e.update()`
- ... (12 more)

### render(SpriteBatch sb) (L2133-2284)

**Calls (in order):**
- L2137: `scene.renderCombatRoomBg(sb)`
- L2141: `scene.renderCampfireRoom(sb)`
- L2142: `this.renderLetterboxGradient(sb)`
- L2146: `scene.renderEventRoom(sb)`
- L2151: `e.render(sb)`
- L2153: `AbstractDungeon.currMapNode.room.render(sb)`
- L2155: `scene.renderCombatRoomFg(sb)`
- L2158: `this.renderLetterboxGradient(sb)`
- L2160: `AbstractDungeon.getCurrRoom()`
- L2161: `room.renderEventTexts(sb)`
- L2165: `e.render(sb)`
- L2167: `overlayMenu.render(sb)`
- L2168: `overlayMenu.renderBlackScreen(sb)`
- L2171: `dungeonMapScreen.render(sb)`
- L2175: `deckViewScreen.render(sb)`
- L2179: `discardPileViewScreen.render(sb)`
- L2183: `gameDeckViewScreen.render(sb)`
- L2187: `exhaustPileViewScreen.render(sb)`
- L2191: `settingsScreen.render(sb)`
- L2195: `inputSettingsScreen.render(sb)`
- L2199: `dungeonMapScreen.render(sb)`
- L2203: `gridSelectScreen.render(sb)`
- L2207: `cardRewardScreen.render(sb)`
- L2211: `combatRewardScreen.render(sb)`
- L2215: `bossRelicScreen.render(sb)`
- L2219: `handCardSelectScreen.render(sb)`
- L2223: `shopScreen.render(sb)`
- L2227: `deathScreen.render(sb)`
- L2231: `victoryScreen.render(sb)`
- L2235: `unlockScreen.render(sb)`
- L2239: `CardCrawlGame.mainMenuScreen.doorUnlockScreen.render(sb)`
- L2243: `gUnlockScreen.render(sb)`
- L2247: `creditsScreen.render(sb)`
- L2256: `logger.info("ERROR: UNKNOWN SCREEN TO RENDER: " + screen.name())`
- L2256: `screen.name()`
- L2260: `sb.setColor(topGradientColor)`
- L2262: `sb.draw(ImageMaster.SCROLL_GRADIENT, 0.0f, (float)Settings.HEIGHT - 128.0f * Settings.scale, (float)Settings.WIDTH, 64.0`
- L2264: `sb.setColor(botGradientColor)`
- L2266: `sb.draw(ImageMaster.SCROLL_GRADIENT, 0.0f, 64.0f * Settings.scale, (float)Settings.WIDTH, -64.0f * Settings.scale)`
- L2270: `ftue.render(sb)`
- L2272: `AbstractDungeon.overlayMenu.cancelButton.render(sb)`
- L2273: `dynamicBanner.render(sb)`
- L2275: `topPanel.render(sb)`
- L2277: `AbstractDungeon.currMapNode.room.renderAboveTopPanel(sb)`
- L2280: `e.render(sb)`
- L2282: `sb.setColor(fadeColor)`
- L2283: `sb.draw(ImageMaster.WHITE_SQUARE_IMG, 0.0f, 0.0f, (float)Settings.WIDTH, (float)Settings.HEIGHT)`

### updateFading() (L2289-2308)

**Calls (in order):**
- L2291: `Interpolation.fade.apply(0.0f, 1.0f, (fadeTimer -= Gdx.graphics.getDeltaTime()) / 0.8f)`
- L2291: `Gdx.graphics.getDeltaTime()`
- L2298: `Interpolation.fade.apply(1.0f, 0.0f, (fadeTimer -= Gdx.graphics.getDeltaTime()) / 0.8f)`
- L2298: `Gdx.graphics.getDeltaTime()`
- L2304: `this.nextRoomTransition()`

### closeCurrentScreen() (L2310-2436)

**Calls (in order):**
- L2317: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2318: `AbstractDungeon.genericScreenOverlayReset()`
- L2320: `c.unhover()`
- L2321: `c.untip()`
- L2326: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2327: `AbstractDungeon.genericScreenOverlayReset()`
- L2331: `c.teleportToDiscardPile()`
- L2332: `c.darken(true)`
- L2333: `c.unhover()`
- L2338: `AbstractDungeon.genericScreenOverlayReset()`
- L2342: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2343: `AbstractDungeon.genericScreenOverlayReset()`
- L2347: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2348: `AbstractDungeon.genericScreenOverlayReset()`
- L2352: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2353: `AbstractDungeon.genericScreenOverlayReset()`
- L2354: `AbstractDungeon.settingsScreen.abandonPopup.hide()`
- L2355: `AbstractDungeon.settingsScreen.exitPopup.hide()`
- L2359: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2360: `AbstractDungeon.genericScreenOverlayReset()`
- L2361: `AbstractDungeon.settingsScreen.abandonPopup.hide()`
- L2362: `AbstractDungeon.settingsScreen.exitPopup.hide()`
- L2366: `AbstractDungeon.genericScreenOverlayReset()`
- L2367: `CardCrawlGame.sound.stop("UNLOCK_SCREEN", AbstractDungeon.gUnlockScreen.id)`
- L2371: `AbstractDungeon.genericScreenOverlayReset()`
- L2372: `AbstractDungeon.combatRewardScreen.rewards.isEmpty()`
- L2377: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2378: `dynamicBanner.hide()`
- L2379: `AbstractDungeon.genericScreenOverlayReset()`
- L2381: `cardRewardScreen.onClose()`
- L2385: `dynamicBanner.hide()`
- L2386: `AbstractDungeon.genericScreenOverlayReset()`
- L2390: `AbstractDungeon.genericScreenOverlayReset()`
- L2391: `dynamicBanner.hide()`
- L2395: `AbstractDungeon.genericScreenOverlayReset()`
- L2396: `overlayMenu.showCombatPanels()`
- L2400: `AbstractDungeon.genericScreenOverlayReset()`
- L2401: `dungeonMapScreen.close()`
- L2404: `AbstractDungeon.firstRoomLogic()`
- L2408: `CardCrawlGame.sound.play("SHOP_CLOSE")`
- L2409: `AbstractDungeon.genericScreenOverlayReset()`
- L2410: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2414: `CardCrawlGame.sound.play("ATTACK_MAGIC_SLOW_1")`
- L2415: `AbstractDungeon.genericScreenOverlayReset()`
- L2416: `AbstractDungeon.overlayMenu.cancelButton.hide()`
- L2420: `logger.info("UNSPECIFIED CASE: " + screen.name())`
- L2420: `screen.name()`
- L2430: `AbstractDungeon.getCurrRoom()`
- L2434: `AbstractDungeon.openPreviousScreen(screen)`

### openPreviousScreen(CurrentScreen s) (L2438-2508)

**Calls (in order):**
- L2441: `deathScreen.reopen()`
- L2445: `victoryScreen.reopen()`
- L2449: `deckViewScreen.open()`
- L2453: `cardRewardScreen.reopen()`
- L2459: `discardPileViewScreen.reopen()`
- L2463: `exhaustPileViewScreen.reopen()`
- L2467: `gameDeckViewScreen.reopen()`
- L2471: `overlayMenu.hideBlackScreen()`
- L2472: `handCardSelectScreen.reopen()`
- L2476: `combatRewardScreen.reopen()`
- L2480: `bossRelicScreen.reopen()`
- L2484: `shopScreen.open()`
- L2488: `overlayMenu.hideBlackScreen()`
- L2490: `dynamicBanner.appear()`
- L2492: `gridSelectScreen.reopen()`
- L2496: `gUnlockScreen.reOpen()`
- L2501: `AbstractDungeon.overlayMenu.cancelButton.show(DungeonMapScreen.TEXT[1])`
- L2504: `AbstractDungeon.overlayMenu.cancelButton.hide()`

### genericScreenOverlayReset() (L2510-2522)

**Calls (in order):**
- L2516: `overlayMenu.hideBlackScreen()`
- L2519: `AbstractDungeon.getCurrRoom()`
- L2520: `overlayMenu.showCombatPanels()`

### fadeIn() (L2524-2530)

**Calls (in order):**
- L2526: `logger.info("WARNING: Attempting to fade in even though screen is not black")`

### fadeOut() (L2532-2540)

**Calls (in order):**
- L2535: `logger.info("WARNING: Attempting to fade out even though screen is not transparent")`

### dungeonTransitionSetup() (L2542-2584)

**Calls (in order):**
- L2545: `cardRng.setCounter(250)`
- L2547: `cardRng.setCounter(500)`
- L2549: `cardRng.setCounter(750)`
- L2551: `logger.info("CardRng Counter: " + AbstractDungeon.cardRng.counter)`
- L2552: `topPanel.unhoverHitboxes()`
- L2553: `pathX.clear()`
- L2554: `pathY.clear()`
- L2555: `EventHelper.resetProbabilities()`
- L2556: `eventList.clear()`
- L2557: `shrineList.clear()`
- L2558: `monsterList.clear()`
- L2559: `eliteMonsterList.clear()`
- L2560: `bossList.clear()`
- L2563: `player.heal(MathUtils.round((float)(AbstractDungeon.player.maxHealth - AbstractDungeon.player.currentHealth) * 0.75f), f`
- L2563: `MathUtils.round((float)(AbstractDungeon.player.maxHealth - AbstractDungeon.player.currentHealth) * 0.75f)`
- L2565: `player.heal(AbstractDungeon.player.maxHealth, false)`
- L2568: `topPanel.panelHealEffect()`
- L2572: `player.decreaseMaxHealth(player.getAscensionMaxHPLoss())`
- L2572: `player.getAscensionMaxHPLoss()`
- L2575: `MathUtils.round((float)AbstractDungeon.player.maxHealth * 0.9f)`
- L2578: `AbstractDungeon.player.masterDeck.addToTop(new AscendersBane())`
- L2579: `UnlockTracker.markCardAsSeen("AscendersBane")`

**Objects created:**
- L2578: `AscendersBane`

### reset() (L2586-2621)

**Calls (in order):**
- L2587: `logger.info("Resetting variables...")`
- L2588: `CardCrawlGame.resetScoreVars()`
- L2589: `ModHelper.setModsFalse()`
- L2592: `AbstractDungeon.getCurrRoom()`
- L2593: `AbstractDungeon.getCurrRoom().dispose()`
- L2593: `AbstractDungeon.getCurrRoom()`
- L2594: `AbstractDungeon.getCurrRoom()`
- L2595: `AbstractDungeon.getCurrRoom()`
- L2596: `m.dispose()`
- L2601: `shrineList.clear()`
- L2602: `relicsToRemoveOnStart.clear()`
- L2604: `actionManager.clear()`
- L2605: `actionManager.clearNextRoomCombatActions()`
- L2606: `combatRewardScreen.clear()`
- L2607: `cardRewardScreen.reset()`
- L2609: `dungeonMapScreen.closeInstantly()`
- L2611: `effectList.clear()`
- L2612: `effectsQueue.clear()`
- L2613: `topLevelEffectsQueue.clear()`
- L2614: `topLevelEffects.clear()`
- L2617: `AbstractDungeon.player.relics.clear()`
- L2620: `blightPool.clear()`

### removeRelicFromPool(ArrayList<String> pool, String name) (L2623-2631)

**Calls (in order):**
- L2624: `pool.iterator()`
- L2625: `i.hasNext()`
- L2626: `i.next()`
- L2627: `s.equals(name)`
- L2628: `i.remove()`
- L2629: `logger.info("Relic" + s + " removed from relic pool.")`

### onModifyPower() (L2633-2647)

**Calls (in order):**
- L2635: `AbstractDungeon.player.hand.applyPowers()`
- L2636: `player.hasPower("Focus")`
- L2638: `o.updateDescription()`
- L2642: `AbstractDungeon.getCurrRoom()`
- L2643: `AbstractDungeon.getCurrRoom()`
- L2644: `m.applyPowers()`

### checkForPactAchievement() (L2649-2653)

**Calls (in order):**
- L2650: `AbstractDungeon.player.exhaustPile.size()`
- L2651: `UnlockTracker.unlockAchievement("THE_PACT")`

### loadSave(SaveFile saveFile) (L2655-2681)

**Calls (in order):**
- L2659: `AbstractDungeon.loadSeeds(saveFile)`
- L2663: `this.setBoss(saveFile.boss)`
- L2674: `EventHelper.setChances(saveFile.event_chances)`
- L2679: `ModHelper.setMods(saveFile.daily_mods)`

### getBlight(String targetID) (L2683-2689)

**Calls (in order):**
- L2685: `b.blightID.equals(targetID)`

## AbstractMonster
File: `monsters\AbstractMonster.java`

### refreshIntentHbLocation() (L158-160)

**Calls (in order):**
- L159: `this.intentHb.move(this.hb.cX + this.intentOffsetX, this.hb.cY + this.hb_h / 2.0f + INTENT_HB_W / 2.0f)`

### update() (L162-173)

**Calls (in order):**
- L164: `p.updateParticles()`
- L166: `this.updateReticle()`
- L167: `this.updateHealthBar()`
- L168: `this.updateAnimations()`
- L169: `this.updateDeathAnimation()`
- L170: `this.updateEscapeAnimation()`
- L171: `this.updateIntent()`
- L172: `this.tint.update()`

### updateIntent() (L181-204)

**Calls (in order):**
- L182: `this.bobEffect.update()`
- L184: `Gdx.graphics.getDeltaTime()`
- L189: `Gdx.graphics.getDeltaTime()`
- L195: `this.updateIntentVFX()`
- L197: `this.intentVfx.iterator()`
- L198: `i.hasNext()`
- L199: `i.next()`
- L200: `e.update()`
- L202: `i.remove()`

### updateIntentVFX() (L206-240)

**Calls (in order):**
- L209: `Gdx.graphics.getDeltaTime()`
- L212: `this.intentVfx.add(new DebuffParticleEffect(this.intentHb.cX, this.intentHb.cY))`
- L215: `Gdx.graphics.getDeltaTime()`
- L218: `this.intentVfx.add(new BuffParticleEffect(this.intentHb.cX, this.intentHb.cY))`
- L221: `Gdx.graphics.getDeltaTime()`
- L224: `this.intentVfx.add(new ShieldParticleEffect(this.intentHb.cX, this.intentHb.cY))`
- L227: `Gdx.graphics.getDeltaTime()`
- L230: `this.intentVfx.add(new UnknownParticleEffect(this.intentHb.cX, this.intentHb.cY))`
- L233: `Gdx.graphics.getDeltaTime()`
- L236: `this.intentVfx.add(new StunStarEffect(this.intentHb.cX, this.intentHb.cY))`

**Objects created:**
- L212: `DebuffParticleEffect`
- L218: `BuffParticleEffect`
- L224: `ShieldParticleEffect`
- L230: `UnknownParticleEffect`
- L236: `StunStarEffect`

### renderTip(SpriteBatch sb) (L242-261)

**Calls (in order):**
- L243: `this.tips.clear()`
- L244: `AbstractDungeon.player.hasRelic("Runic Dome")`
- L245: `this.tips.add(this.intentTip)`
- L249: `this.tips.add(new PowerTip(p.name, p.description, p.region48))`
- L252: `this.tips.add(new PowerTip(p.name, p.description, p.img))`
- L254: `this.tips.isEmpty()`
- L256: `TipHelper.queuePowerTips(this.hb.cX + this.hb.width / 2.0f + TIP_OFFSET_R_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L256: `TipHelper.calculateAdditionalOffset(this.tips, this.hb.cY)`
- L258: `TipHelper.queuePowerTips(this.hb.cX - this.hb.width / 2.0f + TIP_OFFSET_L_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L258: `TipHelper.calculateAdditionalOffset(this.tips, this.hb.cY)`

**Objects created:**
- L249: `PowerTip`
- L252: `PowerTip`

### updateIntentTip() (L263-367)

**Calls (in order):**
- L268: `this.getAttackIntentTip()`

### heal(int healAmount) (L369-385)

**Calls (in order):**
- L375: `p.onHeal(healAmount)`
- L382: `AbstractDungeon.effectList.add(new HealEffect(this.hb.cX - this.animX, this.hb.cY, healAmount))`
- L383: `this.healthBarUpdatedEvent()`

**Objects created:**
- L382: `HealEffect`

### flashIntent() (L387-392)

**Calls (in order):**
- L389: `AbstractDungeon.effectList.add(new FlashIntentEffect(this.intentImg, this))`

**Objects created:**
- L389: `FlashIntentEffect`

### createIntent() (L394-415)

**Calls (in order):**
- L400: `this.calculateDamage(this.intentBaseDmg)`
- L409: `this.getIntentImg()`
- L410: `this.getIntentBg()`
- L414: `this.updateIntentTip()`

### setMove(String moveName, byte nextMove, Intent intent, int baseDamage, int multiplier, boolean isMultiDamage) (L417-423)

**Calls (in order):**
- L420: `this.moveHistory.add(nextMove)`

**Objects created:**
- L422: `EnemyMoveInfo`

### setMove(byte nextMove, Intent intent, int baseDamage, int multiplier, boolean isMultiDamage) (L425-427)

**Calls (in order):**
- L426: `this.setMove(null, nextMove, intent, baseDamage, multiplier, isMultiDamage)`

### setMove(byte nextMove, Intent intent, int baseDamage) (L429-431)

**Calls (in order):**
- L430: `this.setMove(null, nextMove, intent, baseDamage, 0, false)`

### setMove(String moveName, byte nextMove, Intent intent, int baseDamage) (L433-435)

**Calls (in order):**
- L434: `this.setMove(moveName, nextMove, intent, baseDamage, 0, false)`

### setMove(String moveName, byte nextMove, Intent intent) (L437-445)

**Calls (in order):**
- L440: `AbstractDungeon.effectsQueue.add(new TextAboveCreatureEffect(MathUtils.random((float)Settings.WIDTH * 0.25f, (float)Sett`
- L440: `MathUtils.random((float)Settings.WIDTH * 0.25f, (float)Settings.WIDTH * 0.75f)`
- L440: `MathUtils.random((float)Settings.HEIGHT * 0.25f, (float)Settings.HEIGHT * 0.75f)`
- L440: `Color.RED.cpy()`
- L442: `logger.info("ENEMY MOVE " + moveName + " IS SET INCORRECTLY! REPORT TO DEV")`
- L444: `this.setMove(moveName, nextMove, intent, -1, 0, false)`

**Objects created:**
- L440: `TextAboveCreatureEffect`

### setMove(byte nextMove, Intent intent) (L447-449)

**Calls (in order):**
- L448: `this.setMove(null, nextMove, intent, -1, 0, false)`

### rollMove() (L451-453)

**Calls (in order):**
- L452: `this.getMove(AbstractDungeon.aiRng.random(99))`
- L452: `AbstractDungeon.aiRng.random(99)`

### lastMove(byte move) (L455-460)

**Calls (in order):**
- L456: `this.moveHistory.isEmpty()`
- L459: `this.moveHistory.get(this.moveHistory.size() - 1)`
- L459: `this.moveHistory.size()`

### lastMoveBefore(byte move) (L462-470)

**Calls (in order):**
- L463: `this.moveHistory.isEmpty()`
- L466: `this.moveHistory.size()`
- L469: `this.moveHistory.get(this.moveHistory.size() - 2)`
- L469: `this.moveHistory.size()`

### lastTwoMoves(byte move) (L472-477)

**Calls (in order):**
- L473: `this.moveHistory.size()`
- L476: `this.moveHistory.get(this.moveHistory.size() - 1)`
- L476: `this.moveHistory.size()`
- L476: `this.moveHistory.get(this.moveHistory.size() - 2)`
- L476: `this.moveHistory.size()`

### getIntentImg() (L479-528)

**Calls (in order):**
- L482: `this.getAttackIntent()`
- L485: `this.getAttackIntent()`
- L488: `this.getAttackIntent()`
- L491: `this.getAttackIntent()`

### damage(DamageInfo info) (L607-696)

**Calls (in order):**
- L610: `this.hasPower("IntangiblePlayer")`
- L625: `this.decrementBlock(info, damageAmount)`
- L628: `r.onAttackToChangeDamage(info, damageAmount)`
- L633: `p.onAttackToChangeDamage(info, damageAmount)`
- L637: `p.onAttackedToChangeDamage(info, damageAmount)`
- L641: `r.onAttack(info, damageAmount, this)`
- L645: `p.wasHPLost(info, damageAmount)`
- L649: `p.onAttack(info, damageAmount, this)`
- L653: `p.onAttacked(info, damageAmount)`
- L655: `Math.min(damageAmount, this.currentHealth)`
- L659: `this.useStaggerAnimation()`
- L666: `AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, damageAmount))`
- L671: `this.healthBarUpdatedEvent()`
- L675: `AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, TEXT[30]))`
- L677: `AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, 0))`
- L680: `AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, TEXT[30]))`
- L684: `this.die()`
- L685: `AbstractDungeon.getMonsters().areMonstersBasicallyDead()`
- L685: `AbstractDungeon.getMonsters()`
- L686: `AbstractDungeon.actionManager.cleanCardQueue()`
- L687: `AbstractDungeon.effectList.add(new DeckPoofEffect(64.0f * Settings.scale, 64.0f * Settings.scale, true))`
- L688: `AbstractDungeon.effectList.add(new DeckPoofEffect((float)Settings.WIDTH - 64.0f * Settings.scale, 64.0f * Settings.scale`
- L689: `AbstractDungeon.overlayMenu.hideCombatPanels()`
- L692: `this.loseBlock()`
- L693: `AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - th`

**Objects created:**
- L666: `StrikeEffect`
- L675: `BlockedWordEffect`
- L677: `StrikeEffect`
- L680: `BlockedWordEffect`
- L687: `DeckPoofEffect`
- L688: `DeckPoofEffect`
- L693: `HbBlockBrokenEffect`

### init() (L698-701)

**Calls (in order):**
- L699: `this.rollMove()`
- L700: `this.healthBarUpdatedEvent()`

### render(SpriteBatch sb) (L705-749)

**Calls (in order):**
- L709: `sb.setColor(this.tint.color)`
- L711: `sb.draw(this.img, this.drawX - (float)this.img.getWidth() * Settings.scale / 2.0f + this.animX, this.drawY + this.animY,`
- L711: `this.img.getWidth()`
- L711: `this.img.getWidth()`
- L711: `this.img.getHeight()`
- L711: `this.img.getWidth()`
- L711: `this.img.getHeight()`
- L714: `this.state.update(Gdx.graphics.getDeltaTime())`
- L714: `Gdx.graphics.getDeltaTime()`
- L715: `this.state.apply(this.skeleton)`
- L716: `this.skeleton.updateWorldTransform()`
- L717: `this.skeleton.setPosition(this.drawX + this.animX, this.drawY + this.animY)`
- L718: `this.skeleton.setColor(this.tint.color)`
- L719: `this.skeleton.setFlip(this.flipHorizontal, this.flipVertical)`
- L720: `sb.end()`
- L721: `CardCrawlGame.psb.begin()`
- L722: `sr.draw(CardCrawlGame.psb, this.skeleton)`
- L723: `CardCrawlGame.psb.end()`
- L724: `sb.begin()`
- L725: `sb.setBlendFunction(770, 771)`
- L727: `AbstractDungeon.getCurrRoom()`
- L728: `sb.setBlendFunction(770, 1)`
- L729: `sb.setColor(new Color(1.0f, 1.0f, 1.0f, 0.1f))`
- L731: `sb.draw(this.img, this.drawX - (float)this.img.getWidth() * Settings.scale / 2.0f + this.animX, this.drawY + this.animY,`
- L731: `this.img.getWidth()`
- L731: `this.img.getWidth()`
- L731: `this.img.getHeight()`
- L731: `this.img.getWidth()`
- L731: `this.img.getHeight()`
- L732: `sb.setBlendFunction(770, 771)`
- L735: `AbstractDungeon.getCurrRoom()`
- L735: `AbstractDungeon.player.hasRelic("Runic Dome")`
- L736: `this.renderIntentVfxBehind(sb)`
- L737: `this.renderIntent(sb)`
- L738: `this.renderIntentVfxAfter(sb)`
- L739: `this.renderDamageRange(sb)`
- L741: `this.hb.render(sb)`
- L742: `this.intentHb.render(sb)`
- L743: `this.healthHb.render(sb)`
- L746: `this.renderHealth(sb)`
- L747: `this.renderName(sb)`

**Objects created:**
- L729: `Color`

### setHp(int minHp, int maxHp) (L751-761)

**Calls (in order):**
- L752: `AbstractDungeon.monsterHpRng.random(minHp, maxHp)`
- L753: `AbstractDungeon.player.hasBlight("ToughEnemies")`
- L754: `AbstractDungeon.player.getBlight("ToughEnemies").effectFloat()`
- L754: `AbstractDungeon.player.getBlight("ToughEnemies")`
- L757: `ModHelper.isModEnabled("MonsterHunter")`

### setHp(int hp) (L763-765)

**Calls (in order):**
- L764: `this.setHp(hp, hp)`

### renderDamageRange(SpriteBatch sb) (L767-775)

**Calls (in order):**
- L768: `this.intent.name().contains("ATTACK")`
- L768: `this.intent.name()`
- L770: `FontHelper.renderFontLeftTopAligned(sb, FontHelper.topPanelInfoFont, Integer.toString(this.intentDmg) + "x" + Integer.to`
- L770: `Integer.toString(this.intentDmg)`
- L770: `Integer.toString(this.intentMultiAmt)`
- L772: `FontHelper.renderFontLeftTopAligned(sb, FontHelper.topPanelInfoFont, Integer.toString(this.intentDmg), this.intentHb.cX `
- L772: `Integer.toString(this.intentDmg)`

### renderIntentVfxBehind(SpriteBatch sb) (L777-782)

**Calls (in order):**
- L780: `e.render(sb)`

### renderIntentVfxAfter(SpriteBatch sb) (L784-789)

**Calls (in order):**
- L787: `e.render(sb)`

### renderName(SpriteBatch sb) (L791-808)

**Calls (in order):**
- L792: `MathHelper.fadeLerpSnap(this.hoverTimer, 0.0f)`
- L792: `Gdx.graphics.getDeltaTime()`
- L794: `MathHelper.slowColorLerpSnap(this.nameColor.a, 0.0f)`
- L795: `Interpolation.exp5Out.apply(1.5f, 2.0f, this.hoverTimer)`
- L796: `Interpolation.fade.apply(Color.DARK_GRAY.r, Settings.CREAM_COLOR.r, this.hoverTimer * 10.0f)`
- L797: `Interpolation.fade.apply(Color.DARK_GRAY.g, Settings.CREAM_COLOR.g, this.hoverTimer * 3.0f)`
- L798: `Interpolation.fade.apply(Color.DARK_GRAY.b, Settings.CREAM_COLOR.b, this.hoverTimer * 3.0f)`
- L799: `Interpolation.exp10Out.apply(this.healthHb.cY, this.healthHb.cY - 8.0f * Settings.scale, this.nameColor.a)`
- L802: `sb.setColor(this.nameBgColor)`
- L804: `sb.draw(img, x - (float)img.packedWidth / 2.0f, y - (float)img.packedHeight / 2.0f, (float)img.packedWidth / 2.0f, (floa`
- L806: `FontHelper.renderFontCentered(sb, FontHelper.tipHeaderFont, this.name, x, y, this.nameColor)`

### renderIntent(SpriteBatch sb) (L810-830)

**Calls (in order):**
- L812: `sb.setColor(this.intentColor)`
- L814: `sb.setColor(new Color(1.0f, 1.0f, 1.0f, this.intentAlpha / 2.0f))`
- L816: `sb.draw(this.intentBg, this.intentHb.cX - 64.0f, this.intentHb.cY - 64.0f + this.bobEffect.y, 64.0f, 64.0f, 128.0f, 128.`
- L818: `sb.draw(this.intentBg, this.intentHb.cX - 64.0f, this.intentHb.cY - 64.0f + this.bobEffect.y, 64.0f, 64.0f, 128.0f, 128.`
- L822: `Gdx.graphics.getDeltaTime()`
- L823: `sb.setColor(this.intentColor)`
- L825: `sb.draw(this.intentImg, this.intentHb.cX - 64.0f, this.intentHb.cY - 64.0f + this.bobEffect.y, 64.0f, 64.0f, 128.0f, 128`
- L827: `sb.draw(this.intentImg, this.intentHb.cX - 64.0f, this.intentHb.cY - 64.0f + this.bobEffect.y, 64.0f, 64.0f, 128.0f, 128`

**Objects created:**
- L814: `Color`

### updateHitbox(float hb_x, float hb_y, float hb_w, float hb_h) (L832-841)

**Calls (in order):**
- L838: `this.hb.move(this.drawX + this.hb_x + this.animX, this.drawY + this.hb_y + this.hb_h / 2.0f)`
- L839: `this.healthHb.move(this.hb.cX, this.hb.cY - this.hb_h / 2.0f - this.healthHb.height / 2.0f)`
- L840: `this.intentHb.move(this.hb.cX + this.intentOffsetX, this.hb.cY + this.hb_h / 2.0f + 32.0f * Settings.scale)`

**Objects created:**
- L837: `Hitbox`

### updateDeathAnimation() (L845-861)

**Calls (in order):**
- L847: `Gdx.graphics.getDeltaTime()`
- L850: `this.tint.fadeOut()`
- L855: `AbstractDungeon.getMonsters().areMonstersDead()`
- L855: `AbstractDungeon.getMonsters()`
- L855: `AbstractDungeon.getCurrRoom()`
- L855: `AbstractDungeon.getCurrRoom()`
- L856: `AbstractDungeon.getCurrRoom().endBattle()`
- L856: `AbstractDungeon.getCurrRoom()`
- L858: `this.dispose()`
- L859: `this.powers.clear()`

### dispose() (L863-878)

**Calls (in order):**
- L865: `logger.info("Disposed monster img asset")`
- L866: `this.img.dispose()`
- L870: `logger.info("Disposed extra monster assets")`
- L871: `d.dispose()`
- L874: `this.atlas.dispose()`
- L876: `logger.info("Disposed Texture: " + this.name)`

### updateEscapeAnimation() (L880-892)

**Calls (in order):**
- L883: `Gdx.graphics.getDeltaTime()`
- L884: `Gdx.graphics.getDeltaTime()`
- L888: `AbstractDungeon.getMonsters().areMonstersDead()`
- L888: `AbstractDungeon.getMonsters()`
- L888: `AbstractDungeon.getCurrRoom()`
- L888: `AbstractDungeon.getCurrRoom()`
- L889: `AbstractDungeon.getCurrRoom().endBattle()`
- L889: `AbstractDungeon.getCurrRoom()`

### escape() (L901-905)

**Calls (in order):**
- L902: `this.hideHealthBar()`

### die() (L907-909)

**Calls (in order):**
- L908: `this.die(true)`

### die(boolean triggerRelics) (L911-937)

**Calls (in order):**
- L916: `p.onDeath()`
- L921: `r.onMonsterDeath(this)`
- L924: `AbstractDungeon.getMonsters().areMonstersBasicallyDead()`
- L924: `AbstractDungeon.getMonsters()`
- L925: `AbstractDungeon.overlayMenu.endTurnButton.disable()`
- L927: `AbstractDungeon.effectList.add(new ExhaustCardEffect(c))`
- L929: `AbstractDungeon.player.limbo.clear()`
- L935: `StatsScreen.incrementEnemySlain()`

**Objects created:**
- L927: `ExhaustCardEffect`

### useUniversalPreBattleAction() (L942-952)

**Calls (in order):**
- L943: `ModHelper.isModEnabled("Lethality")`
- L944: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(this, this, new StrengthPower(this, 3), 3))`
- L947: `b.onCreateEnemy(this)`
- L949: `ModHelper.isModEnabled("Time Dilation")`
- L949: `this.id.equals("GiantHead")`
- L950: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(this, this, new SlowPower(this, 0)))`

**Objects created:**
- L944: `ApplyPowerAction`
- L944: `StrengthPower`
- L950: `ApplyPowerAction`
- L950: `SlowPower`

### calculateDamage(int dmg) (L954-982)

**Calls (in order):**
- L957: `AbstractDungeon.player.hasBlight("DeadlyEnemies")`
- L958: `AbstractDungeon.player.getBlight("DeadlyEnemies").effectFloat()`
- L958: `AbstractDungeon.player.getBlight("DeadlyEnemies")`
- L962: `p.atDamageGive(tmp, DamageInfo.DamageType.NORMAL)`
- L965: `p.atDamageReceive(tmp, DamageInfo.DamageType.NORMAL)`
- L967: `AbstractDungeon.player.stance.atDamageReceive(tmp, DamageInfo.DamageType.NORMAL)`
- L968: `this.applyBackAttack()`
- L972: `p.atDamageFinalGive(tmp, DamageInfo.DamageType.NORMAL)`
- L975: `p.atDamageFinalReceive(tmp, DamageInfo.DamageType.NORMAL)`
- L977: `MathUtils.floor(tmp)`

### applyPowers() (L984-999)

**Calls (in order):**
- L985: `this.applyBackAttack()`
- L986: `this.hasPower("BackAttack")`
- L987: `AbstractDungeon.actionManager.addToTop(new ApplyPowerAction(this, null, new BackAttackPower(this)))`
- L990: `dmg.applyPowers(this, AbstractDungeon.player)`
- L995: `this.calculateDamage(this.move.baseDamage)`
- L997: `this.getIntentImg()`
- L998: `this.updateIntentTip()`

**Objects created:**
- L987: `ApplyPowerAction`
- L987: `BackAttackPower`

### applyBackAttack() (L1001-1003)

**Calls (in order):**
- L1002: `AbstractDungeon.player.hasPower("Surrounded")`

### removeSurroundedPower() (L1005-1009)

**Calls (in order):**
- L1006: `this.hasPower("BackAttack")`
- L1007: `AbstractDungeon.actionManager.addToTop(new RemoveSpecificPowerAction((AbstractCreature)this, null, "BackAttack"))`

**Objects created:**
- L1007: `RemoveSpecificPowerAction`

### addToBot(AbstractGameAction action) (L1014-1016)

**Calls (in order):**
- L1015: `AbstractDungeon.actionManager.addToBottom(action)`

### addToTop(AbstractGameAction action) (L1018-1020)

**Calls (in order):**
- L1019: `AbstractDungeon.actionManager.addToTop(action)`

### onBossVictoryLogic() (L1022-1042)

**Calls (in order):**
- L1024: `AbstractDungeon.scene.fadeInAmbiance()`
- L1025: `AbstractDungeon.getCurrRoom()`
- L1027: `StatsScreen.incrementBossSlain()`
- L1029: `UnlockTracker.unlockAchievement("YOU_ARE_NOTHING")`
- L1032: `UnlockTracker.unlockAchievement("PERFECT")`
- L1036: `CardCrawlGame.music.silenceTempBgmInstantly()`
- L1037: `CardCrawlGame.music.silenceBGMInstantly()`
- L1038: `AbstractMonster.playBossStinger()`
- L1040: `b.onBossDefeat()`

### onFinalBossVictoryLogic() (L1044-1071)

**Calls (in order):**
- L1045: `AbstractDungeon.bossList.size()`
- L1050: `UnlockTracker.unlockAchievement("SPEED_CLIMBER")`
- L1052: `AbstractDungeon.player.masterDeck.size()`
- L1053: `UnlockTracker.unlockAchievement("MINIMALIST")`
- L1062: `UnlockTracker.unlockAchievement("COMMON_SENSE")`
- L1064: `AbstractDungeon.player.relics.size()`
- L1065: `UnlockTracker.unlockAchievement("ONE_RELIC")`
- L1068: `UnlockTracker.unlockLuckyDay()`

### playBossStinger() (L1073-1100)

**Calls (in order):**
- L1074: `CardCrawlGame.sound.play("BOSS_VICTORY_STINGER")`
- L1075: `AbstractDungeon.id.equals("TheEnding")`
- L1076: `CardCrawlGame.music.playTempBgmInstantly("STS_EndingStinger_v1.ogg", false)`
- L1078: `MathUtils.random(0, 3)`
- L1080: `CardCrawlGame.music.playTempBgmInstantly("STS_BossVictoryStinger_1_v3_MUSIC.ogg", false)`
- L1084: `CardCrawlGame.music.playTempBgmInstantly("STS_BossVictoryStinger_2_v3_MUSIC.ogg", false)`
- L1088: `CardCrawlGame.music.playTempBgmInstantly("STS_BossVictoryStinger_3_v3_MUSIC.ogg", false)`
- L1092: `CardCrawlGame.music.playTempBgmInstantly("STS_BossVictoryStinger_4_v3_MUSIC.ogg", false)`
- L1096: `logger.info("[ERROR] Attempted to play boss stinger but failed.")`

### getLocStrings() (L1102-1108)

**Calls (in order):**
- L1104: `data.put("name", (Serializable)((Object)this.name))`
- L1105: `data.put("moves", (Serializable)MOVES)`
- L1106: `data.put("dialogs", (Serializable)DIALOG)`

**Objects created:**
- L1103: `None`

## AbstractPlayer
File: `characters\AbstractPlayer.java`

### getSaveFilePath() (L283-285)

**Calls (in order):**
- L284: `SaveAndContinue.getPlayerSavePath(this.chosenClass)`

### dispose() (L287-303)

**Calls (in order):**
- L289: `this.atlas.dispose()`
- L292: `this.img.dispose()`
- L295: `this.shoulderImg.dispose()`
- L298: `this.shoulder2Img.dispose()`
- L301: `this.corpseImg.dispose()`

### adjustPotionPositions() (L305-309)

**Calls (in order):**
- L306: `this.potions.size()`
- L307: `this.potions.get(i).adjustPosition(i)`
- L307: `this.potions.get(i)`

### initializeClass(String imgUrl, String shoulder2ImgUrl, String shouldImgUrl, String corpseImgUrl, CharSelectInfo info, float hb_x, float hb_y, float hb_w, float hb_h, EnergyManager energy) (L311-337)

**Calls (in order):**
- L313: `ImageMaster.loadImage(imgUrl)`
- L318: `ImageMaster.loadImage(shouldImgUrl)`
- L319: `ImageMaster.loadImage(shoulder2ImgUrl)`
- L320: `ImageMaster.loadImage(corpseImgUrl)`
- L336: `this.refreshHitboxLocation()`

**Objects created:**
- L334: `Hitbox`
- L335: `Hitbox`

### initializeStarterDeck() (L339-376)

**Calls (in order):**
- L340: `this.getStartingDeck()`
- L342: `ModHelper.isModEnabled("Draft")`
- L342: `ModHelper.isModEnabled("Chimera")`
- L342: `ModHelper.isModEnabled("SealedDeck")`
- L342: `ModHelper.isModEnabled("Shiny")`
- L342: `ModHelper.isModEnabled("Insanity")`
- L345: `ModHelper.isModEnabled("Chimera")`
- L346: `this.masterDeck.addToTop(new Bash())`
- L347: `this.masterDeck.addToTop(new Survivor())`
- L348: `this.masterDeck.addToTop(new Zap())`
- L349: `this.masterDeck.addToTop(new Eruption())`
- L350: `this.masterDeck.addToTop(new Strike_Red())`
- L351: `this.masterDeck.addToTop(new Strike_Green())`
- L352: `this.masterDeck.addToTop(new Strike_Blue())`
- L353: `this.masterDeck.addToTop(new Defend_Red())`
- L354: `this.masterDeck.addToTop(new Defend_Green())`
- L355: `this.masterDeck.addToTop(new Defend_Watcher())`
- L357: `ModHelper.isModEnabled("Insanity")`
- L359: `this.masterDeck.addToTop(AbstractDungeon.returnRandomCard().makeCopy())`
- L359: `AbstractDungeon.returnRandomCard().makeCopy()`
- L359: `AbstractDungeon.returnRandomCard()`
- L362: `ModHelper.isModEnabled("Shiny")`
- L363: `AbstractDungeon.getEachRare()`
- L365: `this.masterDeck.addToTop(c)`
- L370: `this.masterDeck.addToTop(CardLibrary.getCard(this.chosenClass, s).makeCopy())`
- L370: `CardLibrary.getCard(this.chosenClass, s).makeCopy()`
- L370: `CardLibrary.getCard(this.chosenClass, s)`
- L374: `UnlockTracker.markCardAsSeen(c.cardID)`

**Objects created:**
- L346: `Bash`
- L347: `Survivor`
- L348: `Zap`
- L349: `Eruption`
- L350: `Strike_Red`
- L351: `Strike_Green`
- L352: `Strike_Blue`
- L353: `Defend_Red`
- L354: `Defend_Green`
- L355: `Defend_Watcher`

### initializeStarterRelics(PlayerClass chosenClass) (L378-395)

**Calls (in order):**
- L379: `this.getStartingRelics()`
- L380: `ModHelper.isModEnabled("Cursed Run")`
- L381: `relics.clear()`
- L382: `relics.add("Cursed Key")`
- L383: `relics.add("Darkstone Periapt")`
- L384: `relics.add("Du-Vu Doll")`
- L386: `ModHelper.isModEnabled("ControlledChaos")`
- L387: `relics.add("Frozen Eye")`
- L391: `RelicLibrary.getRelic(s).makeCopy().instantObtain(this, index, false)`
- L391: `RelicLibrary.getRelic(s).makeCopy()`
- L391: `RelicLibrary.getRelic(s)`
- L394: `AbstractDungeon.relicsToRemoveOnStart.addAll(relics)`

### combatUpdate() (L397-410)

**Calls (in order):**
- L399: `this.cardInUse.update()`
- L401: `this.limbo.update()`
- L402: `this.exhaustPile.update()`
- L404: `p.updateParticles()`
- L407: `o.update()`
- L409: `this.stance.update()`

### update() (L412-426)

**Calls (in order):**
- L413: `this.updateControllerInput()`
- L414: `this.hb.update()`
- L415: `this.updateHealthBar()`
- L416: `this.updatePowers()`
- L417: `this.healthHb.update()`
- L418: `this.updateReticle()`
- L419: `this.tint.update()`
- L420: `AbstractDungeon.getCurrRoom()`
- L422: `o.updateAnimation()`
- L425: `this.updateEscapeAnimation()`

### updateControllerInput() (L431-577)

**Calls (in order):**
- L446: `CInputActionSet.up.isJustPressed()`
- L446: `CInputActionSet.altUp.isJustPressed()`
- L447: `CInputActionSet.up.unpress()`
- L448: `CInputActionSet.altUp.unpress()`
- L453: `this.blights.isEmpty()`
- L454: `CInputHelper.setCursor(this.blights.get((int)0).hb)`
- L454: `this.blights.get((int)0)`
- L456: `CInputHelper.setCursor(this.relics.get((int)0).hb)`
- L456: `this.relics.get((int)0)`
- L458: `CInputActionSet.left.isJustPressed()`
- L458: `CInputActionSet.altLeft.isJustPressed()`
- L459: `this.orbs.size()`
- L462: `this.orbs.get((int)orbIndex)`
- L463: `Gdx.input.setCursorPosition((int)this.orbs.get((int)orbIndex).hb.cX, Settings.HEIGHT - (int)this.orbs.get((int)orbIndex)`
- L463: `this.orbs.get((int)orbIndex)`
- L463: `this.orbs.get((int)orbIndex)`
- L464: `CInputActionSet.right.isJustPressed()`
- L464: `CInputActionSet.altRight.isJustPressed()`
- L466: `this.orbs.size()`
- L468: `this.orbs.get((int)orbIndex)`
- L469: `Gdx.input.setCursorPosition((int)this.orbs.get((int)orbIndex).hb.cX, Settings.HEIGHT - (int)this.orbs.get((int)orbIndex)`
- L469: `this.orbs.get((int)orbIndex)`
- L469: `this.orbs.get((int)orbIndex)`
- L470: `CInputActionSet.down.isJustPressed()`
- L470: `CInputActionSet.altDown.isJustPressed()`
- L473: `CInputHelper.setCursor(this.inspectHb)`
- L477: `this.hand.isEmpty()`
- L478: `this.hand.group.get(0)`
- L479: `this.hoverCardInHand(this.hoveredCard)`
- L483: `CInputActionSet.up.isJustPressed()`
- L483: `CInputActionSet.altUp.isJustPressed()`
- L483: `AbstractDungeon.getCurrRoom()`
- L484: `Gdx.input.getX()`
- L486: `AbstractDungeon.getMonsters().monsters.isEmpty()`
- L486: `AbstractDungeon.getMonsters()`
- L488: `AbstractDungeon.getMonsters()`
- L490: `hbs.add(abstractMonster.hb)`
- L492: `hbs.isEmpty()`
- L492: `hbs.get(0)`
- L496: `CInputHelper.setCursor(this.inspectHb)`
- L498: `this.releaseCard()`
- L499: `CInputActionSet.right.isJustPressed()`
- L499: `CInputActionSet.altRight.isJustPressed()`
- L499: `AbstractDungeon.getCurrRoom()`
- L501: `hbs.add(this.hb)`
- L502: `AbstractDungeon.getMonsters()`
- L504: `hbs.add(abstractMonster.hb)`
- L509: `h.update()`
- L517: `CInputHelper.setCursor((Hitbox)hbs.get(0))`
- L517: `hbs.get(0)`
- ... (36 more)

**Objects created:**
- L487: `ArrayList`
- L500: `None`
- L531: `ArrayList`

### updateViewRelicControls() (L579-676)

**Calls (in order):**
- L602: `CInputHelper.setCursor(this.relics.get((int)0).hb)`
- L602: `this.relics.get((int)0)`
- L603: `CInputActionSet.left.isJustPressed()`
- L603: `CInputActionSet.altLeft.isJustPressed()`
- L608: `this.relics.size()`
- L611: `this.relics.size()`
- L612: `AbstractDungeon.topPanel.adjustRelicHbs()`
- L614: `this.relics.size()`
- L617: `AbstractDungeon.topPanel.adjustRelicHbs()`
- L620: `CInputHelper.setCursor(this.relics.get((int)index).hb)`
- L620: `this.relics.get((int)index)`
- L623: `this.blights.size()`
- L625: `CInputHelper.setCursor(this.blights.get((int)index).hb)`
- L625: `this.blights.get((int)index)`
- L627: `CInputActionSet.right.isJustPressed()`
- L627: `CInputActionSet.altRight.isJustPressed()`
- L630: `this.relics.size()`
- L633: `AbstractDungeon.topPanel.adjustRelicHbs()`
- L636: `CInputHelper.setCursor(this.relics.get((int)index).hb)`
- L636: `this.relics.get((int)index)`
- L638: `this.blights.size()`
- L641: `CInputHelper.setCursor(this.blights.get((int)index).hb)`
- L641: `this.blights.get((int)index)`
- L643: `CInputActionSet.up.isJustPressed()`
- L643: `CInputActionSet.altUp.isJustPressed()`
- L644: `CInputActionSet.up.unpress()`
- L648: `CInputHelper.setCursor(this.potions.get((int)0).hb)`
- L648: `this.potions.get((int)0)`
- L650: `CInputHelper.setCursor(this.relics.get((int)0).hb)`
- L650: `this.relics.get((int)0)`
- L652: `CInputActionSet.cancel.isJustPressed()`
- L654: `Gdx.input.setCursorPosition(10, Settings.HEIGHT / 2)`
- L655: `CInputActionSet.down.isJustPressed()`
- L655: `CInputActionSet.altDown.isJustPressed()`
- L657: `this.blights.isEmpty()`
- L658: `CInputActionSet.down.unpress()`
- L659: `CInputActionSet.altDown.unpress()`
- L662: `this.orbs.isEmpty()`
- L662: `this.orbs.get((int)0)`
- L663: `CInputHelper.setCursor(this.inspectHb)`
- L665: `CInputHelper.setCursor(this.blights.get((int)0).hb)`
- L665: `this.blights.get((int)0)`
- L668: `CInputActionSet.down.unpress()`
- L669: `CInputActionSet.altDown.unpress()`
- L672: `this.orbs.isEmpty()`
- L672: `this.orbs.get((int)0)`
- L673: `CInputHelper.setCursor(this.inspectHb)`

### loseGold(int goldAmount) (L678-699)

**Calls (in order):**
- L680: `AbstractDungeon.getCurrRoom()`
- L682: `r.onSpendGold()`
- L685: `AbstractDungeon.getCurrRoom()`
- L685: `AbstractDungeon.getCurrRoom()`
- L686: `CardCrawlGame.sound.play("EVENT_PURCHASE")`
- L694: `r.onLoseGold()`
- L697: `logger.info("NEGATIVE MONEY???")`

### gainGold(int amount) (L701-716)

**Calls (in order):**
- L703: `this.hasRelic("Ectoplasm")`
- L704: `this.getRelic("Ectoplasm").flash()`
- L704: `this.getRelic("Ectoplasm")`
- L708: `logger.info("NEGATIVE MONEY???")`
- L713: `r.onGainGold()`

### updateInput() (L735-854)

**Calls (in order):**
- L740: `Gdx.graphics.getDeltaTime()`
- L743: `this.updateSingleTargetInput()`
- L747: `AbstractDungeon.getCurrRoom()`
- L748: `abstractMonster.hb.update()`
- L760: `this.hoveredCard.flash(Color.SKY.cpy())`
- L760: `Color.SKY.cpy()`
- L764: `o.showEvokeValue()`
- L777: `o.showEvokeValue()`
- L790: `InputHelper.moveCursorToNeutralPosition()`
- L792: `this.releaseCard()`
- L793: `CardCrawlGame.sound.play("UI_CLICK_1")`
- L795: `this.updateFullKeyboardCardSelection()`
- L798: `this.releaseCard()`
- L804: `this.hoveredCard.setAngle(0.0f, true)`
- L805: `this.hand.hoverCardPush(this.hoveredCard)`
- L813: `this.hand.getHoveredCard()`
- L819: `this.hoveredCard.setAngle(0.0f, true)`
- L820: `this.hand.hoverCardPush(this.hoveredCard)`
- L827: `this.hasRelic("Necronomicon")`
- L828: `this.getRelic("Necronomicon").stopPulse()`
- L828: `this.getRelic("Necronomicon")`
- L831: `this.clickAndDragCards()`
- L834: `this.hoveredCard.isHoveredInHand(1.0f)`
- L837: `this.hand.group.size()`
- L838: `this.hand.group.get((int)var4_11)`
- L838: `this.hand.group.get((int)(var4_11 - true)).isHoveredInHand(1.0f)`
- L838: `this.hand.group.get((int)(var4_11 - true))`
- L839: `this.hand.group.get((int)(var4_11 - true))`
- L844: `this.releaseCard()`
- L847: `this.hoveredCard.updateHoverLogic()`
- L849: `AbstractDungeon.actionManager.cardQueue.isEmpty()`

### updateSingleTargetInput() (L856-936)

**Calls (in order):**
- L858: `Gdx.input.setCursorPosition((int)MathUtils.lerp(Gdx.input.getX(), (float)Settings.WIDTH / 2.0f, Gdx.graphics.getDeltaTim`
- L858: `MathUtils.lerp(Gdx.input.getX(), (float)Settings.WIDTH / 2.0f, Gdx.graphics.getDeltaTime() * 10.0f)`
- L858: `Gdx.input.getX()`
- L858: `Gdx.graphics.getDeltaTime()`
- L858: `MathUtils.lerp(Gdx.input.getY(), (float)Settings.HEIGHT * 1.1f, Gdx.graphics.getDeltaTime() * 4.0f)`
- L858: `Gdx.input.getY()`
- L858: `Gdx.graphics.getDeltaTime()`
- L861: `InputActionSet.releaseCard.isJustPressed()`
- L861: `CInputActionSet.cancel.isJustPressed()`
- L865: `this.hoverCardInHand(card)`
- L867: `this.updateTargetArrowWithKeyboard(false)`
- L871: `AbstractDungeon.getCurrRoom()`
- L872: `m.hb.update()`
- L878: `AbstractDungeon.getCurrRoom().monsters.areMonstersBasicallyDead()`
- L878: `AbstractDungeon.getCurrRoom()`
- L880: `InputHelper.moveCursorToNeutralPosition()`
- L882: `this.releaseCard()`
- L883: `CardCrawlGame.sound.play("UI_CLICK_2")`
- L890: `InputHelper.getCardSelectedByHotkey(this.hand)`
- L891: `this.isCardQueued(cardFromHotkey)`
- L893: `this.releaseCard()`
- L899: `this.hoveredCard.setAngle(0.0f, false)`
- L904: `InputActionSet.confirm.isJustPressed()`
- L904: `CInputActionSet.select.isJustPressed()`
- L907: `CardCrawlGame.sound.play("UI_CLICK_1")`
- L910: `this.hoveredCard.canUse(this, this.hoveredMonster)`
- L911: `this.playCard()`
- L913: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, this.hoveredCard.cantUseMessage, true`
- L914: `this.energyTip(this.hoveredCard)`
- L915: `this.releaseCard()`
- L924: `this.hoveredCard.canUse(this, this.hoveredMonster)`
- L925: `this.playCard()`
- L927: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, this.hoveredCard.cantUseMessage, true`
- L928: `this.energyTip(this.hoveredCard)`
- L929: `this.releaseCard()`

**Objects created:**
- L913: `ThoughtBubble`
- L927: `ThoughtBubble`

### energyTip(AbstractCard cardToCheck) (L946-952)

**Calls (in order):**
- L948: `TipTracker.tips.get("ENERGY_USE_TIP").booleanValue()`
- L948: `TipTracker.tips.get("ENERGY_USE_TIP")`
- L950: `TipTracker.neverShowAgain("ENERGY_USE_TIP")`

**Objects created:**
- L949: `FtueTip`

### updateFullKeyboardCardSelection() (L954-991)

**Calls (in order):**
- L956: `InputActionSet.left.isJustPressed()`
- L956: `InputActionSet.right.isJustPressed()`
- L956: `InputActionSet.confirm.isJustPressed()`
- L961: `InputHelper.didMoveMouse()`
- L969: `this.hand.isEmpty()`
- L973: `InputActionSet.left.isJustPressed()`
- L973: `CInputActionSet.left.isJustPressed()`
- L973: `CInputActionSet.altLeft.isJustPressed()`
- L974: `this.hand.size()`
- L975: `InputActionSet.right.isJustPressed()`
- L975: `CInputActionSet.right.isJustPressed()`
- L975: `CInputActionSet.altRight.isJustPressed()`
- L980: `InputActionSet.left.isJustPressed()`
- L980: `CInputActionSet.left.isJustPressed()`
- L980: `CInputActionSet.altLeft.isJustPressed()`
- L982: `InputActionSet.right.isJustPressed()`
- L982: `CInputActionSet.right.isJustPressed()`
- L982: `CInputActionSet.altRight.isJustPressed()`
- L985: `this.hand.size()`
- L985: `this.hand.size()`
- L986: `this.hand.group.get(this.keyboardCardIndex)`
- L986: `Math.abs(card.current_x - card.target_x)`
- L987: `this.hoverCardInHand(card)`

### hoverCardInHand(AbstractCard card) (L993-1001)

**Calls (in order):**
- L999: `Gdx.input.setCursorPosition((int)card.hb.cX, (int)((float)Settings.HEIGHT - HOVER_CARD_Y_POSITION))`

### updateTargetArrowWithKeyboard(boolean autoTargetFirst) (L1003-1050)

**Calls (in order):**
- L1008: `InputActionSet.left.isJustPressed()`
- L1008: `CInputActionSet.left.isJustPressed()`
- L1008: `CInputActionSet.altLeft.isJustPressed()`
- L1011: `InputActionSet.right.isJustPressed()`
- L1011: `CInputActionSet.right.isJustPressed()`
- L1011: `CInputActionSet.altRight.isJustPressed()`
- L1015: `AbstractDungeon.getCurrRoom()`
- L1016: `AbstractDungeon.getCurrRoom()`
- L1019: `sortedMonsters.remove(mons)`
- L1021: `sortedMonsters.sort(AbstractMonster.sortByHitbox)`
- L1022: `sortedMonsters.isEmpty()`
- L1032: `sortedMonsters.get(0)`
- L1032: `sortedMonsters.get(sortedMonsters.size() - 1)`
- L1032: `sortedMonsters.size()`
- L1034: `sortedMonsters.indexOf(this.hoveredMonster)`
- L1036: `sortedMonsters.size()`
- L1036: `sortedMonsters.size()`
- L1037: `sortedMonsters.get(newTargetIndex)`
- L1041: `Gdx.input.setCursorPosition((int)target.cX, Settings.HEIGHT - (int)target.cY)`

**Objects created:**
- L1016: `None`

### renderCardHotKeyText(SpriteBatch sb) (L1052-1067)

**Calls (in order):**
- L1061: `Math.sin((double)(card.angle / 180.0f) * Math.PI)`
- L1063: `FontHelper.renderFontCentered(sb, FontHelper.buttonLabelFont, InputActionSet.selectCardActions[index].getKeyString(), ca`
- L1063: `InputActionSet.selectCardActions[index].getKeyString()`

### clickAndDragCards() (L1069-1235)

**Calls (in order):**
- L1071: `InputHelper.getCardSelectedByHotkey(this.hand)`
- L1072: `this.isCardQueued(cardFromHotkey)`
- L1075: `CardCrawlGame.sound.play("UI_CLICK_2")`
- L1076: `this.releaseCard()`
- L1079: `this.manuallySelectCard(cardFromHotkey)`
- L1082: `CInputActionSet.select.isJustPressed()`
- L1082: `this.isCardQueued(this.hoveredCard)`
- L1083: `this.manuallySelectCard(this.hoveredCard)`
- L1085: `this.updateTargetArrowWithKeyboard(true)`
- L1087: `InputHelper.moveCursorToNeutralPosition()`
- L1095: `CardCrawlGame.sound.play("CARD_OBTAIN")`
- L1102: `Gdx.input.setCursorPosition((int)this.hoveredCard.current_x, (int)((float)Settings.HEIGHT - AbstractCard.IMG_HEIGHT / 2.`
- L1108: `Gdx.graphics.getDeltaTime()`
- L1109: `InputActionSet.confirm.isJustPressed()`
- L1109: `CInputActionSet.select.isJustPressed()`
- L1111: `CardCrawlGame.sound.play("UI_CLICK_2")`
- L1112: `this.releaseCard()`
- L1116: `this.hoveredCard.canUse(this, null)`
- L1117: `this.playCard()`
- L1119: `CardCrawlGame.sound.play("CARD_OBTAIN")`
- L1120: `this.releaseCard()`
- L1126: `InputActionSet.releaseCard.isJustPressed()`
- L1126: `CInputActionSet.cancel.isJustPressed()`
- L1127: `this.hoverCardInHand(this.hoveredCard)`
- L1128: `InputActionSet.confirm.isJustPressed()`
- L1128: `CInputActionSet.select.isJustPressed()`
- L1129: `this.manuallySelectCard(this.hoveredCard)`
- L1131: `this.updateTargetArrowWithKeyboard(true)`
- L1133: `Gdx.input.setCursorPosition(10, Settings.HEIGHT / 2)`
- L1139: `CardCrawlGame.sound.play("UI_CLICK_2")`
- L1140: `this.releaseCard()`
- L1150: `this.hoveredCard.hasEnoughEnergy()`
- L1151: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, this.hoveredCard.cantUseMessage, true`
- L1152: `this.energyTip(this.hoveredCard)`
- L1153: `this.releaseCard()`
- L1154: `CardCrawlGame.sound.play("CARD_REJECT")`
- L1162: `this.hoveredCard.untip()`
- L1163: `this.hand.refreshHandLayout()`
- L1183: `this.hoveredCard.untip()`
- L1184: `this.hand.refreshHandLayout()`
- L1190: `this.hoveredCard.canUse(this, null)`
- L1191: `this.playCard()`
- L1194: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, this.hoveredCard.cantUseMessage, true`
- L1195: `this.energyTip(this.hoveredCard)`
- L1196: `this.releaseCard()`
- L1204: `this.releaseCard()`
- L1205: `CardCrawlGame.sound.play("CARD_OBTAIN")`
- L1210: `this.hoveredCard.hasEnoughEnergy()`
- L1210: `this.hoveredCard.canUse(this, null)`
- L1211: `this.playCard()`
- ... (4 more)

**Objects created:**
- L1151: `ThoughtBubble`
- L1194: `ThoughtBubble`

### manuallySelectCard(AbstractCard card) (L1237-1265)

**Calls (in order):**
- L1241: `this.hoveredCard.setAngle(0.0f, false)`
- L1244: `this.hoveredCard.flash(Color.SKY.cpy())`
- L1244: `Color.SKY.cpy()`
- L1248: `o.showEvokeValue()`
- L1260: `o.showEvokeValue()`

### playCard() (L1267-1284)

**Calls (in order):**
- L1270: `this.hoveredCard.unhover()`
- L1271: `this.queueContains(this.hoveredCard)`
- L1273: `this.hasPower("Surrounded")`
- L1276: `AbstractDungeon.actionManager.cardQueue.add(new CardQueueItem(this.hoveredCard, this.hoveredMonster))`
- L1278: `AbstractDungeon.actionManager.cardQueue.add(new CardQueueItem(this.hoveredCard, null))`

**Objects created:**
- L1276: `CardQueueItem`
- L1278: `CardQueueItem`

### releaseCard() (L1294-1321)

**Calls (in order):**
- L1296: `o.hideEvokeValues()`
- L1311: `this.hoveredCard.canUse(this, null)`
- L1312: `this.hoveredCard.beginGlowing()`
- L1314: `this.hoveredCard.untip()`
- L1316: `this.hoveredCard.unhover()`
- L1319: `this.hand.refreshHandLayout()`

### onCardDrawOrDiscard() (L1323-1338)

**Calls (in order):**
- L1325: `p.onDrawOrDiscard()`
- L1328: `r.onDrawOrDiscard()`
- L1330: `this.hasPower("Corruption")`
- L1333: `c.modifyCostForCombat(-9)`
- L1336: `this.hand.applyPowers()`
- L1337: `this.hand.glowCheck()`

### useCard(AbstractCard c, AbstractMonster monster, int energyOnUse) (L1340-1366)

**Calls (in order):**
- L1342: `this.useFastAttackAnimation()`
- L1344: `c.calculateCardDamage(monster)`
- L1351: `c.use(this, monster)`
- L1352: `AbstractDungeon.actionManager.addToBottom(new UseCardAction(c, monster))`
- L1354: `this.hand.triggerOnOtherCardPlayed(c)`
- L1356: `this.hand.removeCard(c)`
- L1360: `c.freeToPlay()`
- L1360: `this.hasPower("Corruption")`
- L1361: `this.energy.use(c.costForTurn)`
- L1363: `this.hand.canUseAnyCard()`

**Objects created:**
- L1352: `UseCardAction`

### damage(DamageInfo info) (L1368-1498)

**Calls (in order):**
- L1378: `this.hasPower("IntangiblePlayer")`
- L1381: `this.decrementBlock(info, damageAmount)`
- L1384: `abstractRelic.onAttackToChangeDamage(info, damageAmount)`
- L1389: `abstractPower.onAttackToChangeDamage(info, damageAmount)`
- L1393: `abstractRelic.onAttackedToChangeDamage(info, damageAmount)`
- L1396: `abstractPower.onAttackedToChangeDamage(info, damageAmount)`
- L1400: `abstractRelic.onAttack(info, damageAmount, this)`
- L1405: `abstractPower.onAttack(info, damageAmount, this)`
- L1408: `abstractPower.onAttacked(info, damageAmount)`
- L1411: `abstractRelic.onAttacked(info, damageAmount)`
- L1414: `logger.info("NO OWNER, DON'T TRIGGER POWERS")`
- L1417: `abstractRelic.onLoseHpLast(damageAmount)`
- L1419: `Math.min(damageAmount, this.currentHealth)`
- L1422: `abstractPower.onLoseHp(damageAmount)`
- L1425: `abstractRelic.onLoseHp(damageAmount)`
- L1428: `abstractPower.wasHPLost(info, damageAmount)`
- L1431: `abstractRelic.wasHPLost(damageAmount)`
- L1435: `abstractPower.onInflictDamage(info, damageAmount, this)`
- L1439: `this.useStaggerAnimation()`
- L1447: `AbstractDungeon.getCurrRoom()`
- L1448: `this.updateCardsOnDamage()`
- L1451: `AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, damageAmount))`
- L1455: `AbstractDungeon.topLevelEffects.add(new BorderFlashEffect(new Color(1.0f, 0.1f, 0.05f, 0.0f)))`
- L1457: `this.healthBarUpdatedEvent()`
- L1462: `abstractRelic.onBloodied()`
- L1466: `this.hasRelic("Mark of the Bloom")`
- L1467: `this.hasPotion("FairyPotion")`
- L1469: `abstractPotion.ID.equals("FairyPotion")`
- L1470: `abstractPotion.flash()`
- L1472: `abstractPotion.use(this)`
- L1473: `AbstractDungeon.topPanel.destroyPotion(abstractPotion.slot)`
- L1476: `this.hasRelic("Lizard Tail")`
- L1476: `this.getRelic((String)"Lizard Tail")`
- L1478: `this.getRelic("Lizard Tail").onTrigger()`
- L1478: `this.getRelic("Lizard Tail")`
- L1483: `AbstractDungeon.getMonsters()`
- L1486: `this.loseBlock()`
- L1487: `AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - th`
- L1491: `AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, AbstractPlayer.uiStrings.TEXT[0]))`
- L1493: `AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, AbstractPlayer.uiStrings.TEXT[0]))`
- L1494: `AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - th`
- L1496: `AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, 0))`

**Objects created:**
- L1451: `StrikeEffect`
- L1455: `BorderFlashEffect`
- L1455: `Color`
- L1483: `DeathScreen`
- L1487: `HbBlockBrokenEffect`
- L1491: `BlockedWordEffect`
- L1493: `BlockedWordEffect`
- L1494: `HbBlockBrokenEffect`
- L1496: `StrikeEffect`

### updateCardsOnDamage() (L1500-1512)

**Calls (in order):**
- L1501: `AbstractDungeon.getCurrRoom()`
- L1503: `c.tookDamage()`
- L1506: `c.tookDamage()`
- L1509: `c.tookDamage()`

### updateCardsOnDiscard() (L1514-1524)

**Calls (in order):**
- L1516: `c.didDiscard()`
- L1519: `c.didDiscard()`
- L1522: `c.didDiscard()`

### heal(int healAmount) (L1526-1535)

**Calls (in order):**
- L1528: `super.heal(healAmount)`
- L1532: `r.onNotBloodied()`

### gainEnergy(int e) (L1537-1540)

**Calls (in order):**
- L1538: `EnergyPanel.addEnergy(e)`
- L1539: `this.hand.glowCheck()`

### loseEnergy(int e) (L1542-1544)

**Calls (in order):**
- L1543: `EnergyPanel.useEnergy(e)`

### preBattlePrep() (L1546-1590)

**Calls (in order):**
- L1547: `TipTracker.tips.get("COMBAT_TIP").booleanValue()`
- L1547: `TipTracker.tips.get("COMBAT_TIP")`
- L1549: `TipTracker.neverShowAgain("COMBAT_TIP")`
- L1551: `AbstractDungeon.actionManager.clear()`
- L1555: `this.orbs.clear()`
- L1556: `this.increaseMaxOrbSlots(this.masterMaxOrbs, false)`
- L1566: `this.drawPile.initializeDeck(this.masterDeck)`
- L1568: `this.hand.clear()`
- L1569: `this.discardPile.clear()`
- L1570: `this.exhaustPile.clear()`
- L1571: `AbstractDungeon.player.hasRelic("SlaversCollar")`
- L1572: `((SlaversCollar)AbstractDungeon.player.getRelic("SlaversCollar")).beforeEnergyPrep()`
- L1572: `AbstractDungeon.player.getRelic("SlaversCollar")`
- L1574: `this.energy.prep()`
- L1575: `this.powers.clear()`
- L1577: `this.healthBarUpdatedEvent()`
- L1578: `ModHelper.isModEnabled("Lethality")`
- L1579: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(this, this, new StrengthPower(this, 3), 3))`
- L1581: `ModHelper.isModEnabled("Terminal")`
- L1582: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(this, this, new PlatedArmorPower(this, 5), 5))`
- L1584: `AbstractDungeon.getCurrRoom().monsters.usePreBattleAction()`
- L1584: `AbstractDungeon.getCurrRoom()`
- L1585: `AbstractDungeon.getCurrMapNode()`
- L1586: `AbstractDungeon.getCurrRoom().applyEmeraldEliteBuff()`
- L1586: `AbstractDungeon.getCurrRoom()`
- L1588: `AbstractDungeon.actionManager.addToTop(new WaitAction(1.0f))`
- L1589: `this.applyPreCombatLogic()`

**Objects created:**
- L1548: `MultiPageFtue`
- L1579: `ApplyPowerAction`
- L1579: `StrengthPower`
- L1582: `ApplyPowerAction`
- L1582: `PlatedArmorPower`
- L1588: `WaitAction`

### getRelicNames() (L1592-1598)

**Calls (in order):**
- L1595: `arr.add(relic.relicId)`

**Objects created:**
- L1593: `None`

### getCircletCount() (L1600-1612)

**Calls (in order):**
- L1604: `relic.relicId.equals("Circlet")`

### draw(int numCards) (L1614-1637)

**Calls (in order):**
- L1616: `this.drawPile.isEmpty()`
- L1617: `this.drawPile.getTopCard()`
- L1620: `c.setAngle(0.0f, true)`
- L1621: `c.lighten(false)`
- L1624: `c.triggerWhenDrawn()`
- L1625: `this.hand.addToHand(c)`
- L1626: `this.drawPile.removeTopCard()`
- L1628: `p.onCardDraw(c)`
- L1631: `r.onCardDraw(c)`
- L1635: `logger.info("ERROR: How did this happen? No cards in draw pile?? Player.java")`

### draw() (L1639-1647)

**Calls (in order):**
- L1640: `this.hand.size()`
- L1641: `this.createHandIsFullDialog()`
- L1644: `CardCrawlGame.sound.playAV("CARD_DRAW_8", -0.12f, 0.25f)`
- L1645: `this.draw(1)`
- L1646: `this.onCardDrawOrDiscard()`

### render(SpriteBatch sb) (L1649-1673)

**Calls (in order):**
- L1651: `this.stance.render(sb)`
- L1652: `AbstractDungeon.getCurrRoom()`
- L1652: `AbstractDungeon.getCurrRoom()`
- L1653: `this.renderHealth(sb)`
- L1654: `this.orbs.isEmpty()`
- L1656: `o.render(sb)`
- L1660: `AbstractDungeon.getCurrRoom()`
- L1662: `sb.setColor(Color.WHITE)`
- L1663: `sb.draw(this.img, this.drawX - (float)this.img.getWidth() * Settings.scale / 2.0f + this.animX, this.drawY, (float)this.`
- L1663: `this.img.getWidth()`
- L1663: `this.img.getWidth()`
- L1663: `this.img.getHeight()`
- L1663: `this.img.getWidth()`
- L1663: `this.img.getHeight()`
- L1665: `this.renderPlayerImage(sb)`
- L1667: `this.hb.render(sb)`
- L1668: `this.healthHb.render(sb)`
- L1670: `sb.setColor(Color.WHITE)`
- L1671: `this.renderShoulderImg(sb)`

### renderShoulderImg(SpriteBatch sb) (L1675-1681)

**Calls (in order):**
- L1677: `sb.draw(this.shoulder2Img, 0.0f, 0.0f, 1920.0f * Settings.scale, 1136.0f * Settings.scale)`
- L1679: `sb.draw(this.shoulderImg, this.animX, 0.0f, 1920.0f * Settings.scale, 1136.0f * Settings.scale)`

### renderPlayerImage(SpriteBatch sb) (L1683-1700)

**Calls (in order):**
- L1685: `this.state.update(Gdx.graphics.getDeltaTime())`
- L1685: `Gdx.graphics.getDeltaTime()`
- L1686: `this.state.apply(this.skeleton)`
- L1687: `this.skeleton.updateWorldTransform()`
- L1688: `this.skeleton.setPosition(this.drawX + this.animX, this.drawY + this.animY)`
- L1689: `this.skeleton.setColor(this.tint.color)`
- L1690: `this.skeleton.setFlip(this.flipHorizontal, this.flipVertical)`
- L1691: `sb.end()`
- L1692: `CardCrawlGame.psb.begin()`
- L1693: `sr.draw(CardCrawlGame.psb, this.skeleton)`
- L1694: `CardCrawlGame.psb.end()`
- L1695: `sb.begin()`
- L1697: `sb.setColor(Color.WHITE)`
- L1698: `sb.draw(this.img, this.drawX - (float)this.img.getWidth() * Settings.scale / 2.0f + this.animX, this.drawY, (float)this.`
- L1698: `this.img.getWidth()`
- L1698: `this.img.getWidth()`
- L1698: `this.img.getHeight()`
- L1698: `this.img.getWidth()`
- L1698: `this.img.getHeight()`

### renderPlayerBattleUi(SpriteBatch sb) (L1702-1706)

**Calls (in order):**
- L1704: `this.renderPowerTips(sb)`

### renderPowerTips(SpriteBatch sb) (L1708-1728)

**Calls (in order):**
- L1711: `this.stance.ID.equals("Neutral")`
- L1712: `tips.add(new PowerTip(this.stance.name, this.stance.description))`
- L1716: `tips.add(new PowerTip(p.name, p.description, p.region48))`
- L1719: `tips.add(new PowerTip(p.name, p.description, p.img))`
- L1721: `tips.isEmpty()`
- L1723: `TipHelper.queuePowerTips(this.hb.cX + this.hb.width / 2.0f + TIP_OFFSET_R_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L1723: `TipHelper.calculateAdditionalOffset(tips, this.hb.cY)`
- L1725: `TipHelper.queuePowerTips(this.hb.cX - this.hb.width / 2.0f + TIP_OFFSET_L_X, this.hb.cY + TipHelper.calculateAdditionalO`
- L1725: `TipHelper.calculateAdditionalOffset(tips, this.hb.cY)`

**Objects created:**
- L1710: `None`
- L1712: `PowerTip`
- L1716: `PowerTip`
- L1719: `PowerTip`

### renderHand(SpriteBatch sb) (L1730-1784)

**Calls (in order):**
- L1732: `this.renderCardHotKeyText(sb)`
- L1735: `this.renderReticle(sb, this.inspectHb)`
- L1739: `this.hand.renderHand(sb, this.hoveredCard)`
- L1740: `this.hoveredCard.renderHoverShadow(sb)`
- L1744: `AbstractDungeon.getMonsters()`
- L1750: `this.hoveredCard.calculateCardDamage(theMonster)`
- L1751: `this.hoveredCard.render(sb)`
- L1752: `this.hoveredCard.applyPowers()`
- L1754: `this.hoveredCard.render(sb)`
- L1757: `AbstractDungeon.getCurrRoom().isBattleEnding()`
- L1757: `AbstractDungeon.getCurrRoom()`
- L1758: `this.renderHoverReticle(sb)`
- L1762: `this.hoveredCard.calculateCardDamage(this.hoveredMonster)`
- L1763: `this.hoveredCard.render(sb)`
- L1764: `this.hoveredCard.applyPowers()`
- L1766: `this.hoveredCard.render(sb)`
- L1769: `this.hand.render(sb)`
- L1771: `this.hand.renderHand(sb, this.cardInUse)`
- L1774: `this.cardInUse.render(sb)`
- L1775: `AbstractDungeon.getCurrRoom()`
- L1776: `AbstractDungeon.effectList.add(new CardDisappearEffect(this.cardInUse.makeCopy(), this.cardInUse.current_x, this.cardInU`
- L1776: `this.cardInUse.makeCopy()`
- L1780: `this.limbo.render(sb)`
- L1781: `AbstractDungeon.getCurrRoom()`
- L1781: `AbstractDungeon.getCurrRoom().isBattleEnding()`
- L1781: `AbstractDungeon.getCurrRoom()`
- L1782: `this.renderTargetingUi(sb)`

**Objects created:**
- L1776: `CardDisappearEffect`

### renderTargetingUi(SpriteBatch sb) (L1786-1812)

**Calls (in order):**
- L1787: `MathHelper.mouseLerpSnap(this.arrowX, InputHelper.mX)`
- L1788: `MathHelper.mouseLerpSnap(this.arrowY, InputHelper.mY)`
- L1794: `sb.setColor(Color.WHITE)`
- L1796: `Gdx.graphics.getDeltaTime()`
- L1800: `Interpolation.elasticOut.apply(Settings.scale, Settings.scale * 1.2f, this.arrowScaleTimer)`
- L1801: `sb.setColor(ARROW_COLOR)`
- L1805: `this.arrowTmp.nor()`
- L1810: `this.drawCurvedLine(sb, this.startArrowVector, this.endArrowVector, this.controlPoint)`
- L1811: `sb.draw(ImageMaster.TARGET_UI_ARROW, this.arrowX - 128.0f, this.arrowY - 128.0f, 128.0f, 128.0f, 256.0f, 256.0f, this.ar`
- L1811: `this.arrowTmp.angle()`

### drawCurvedLine(SpriteBatch sb, Vector2 start, Vector2 end, Vector2 control) (L1814-1829)

**Calls (in order):**
- L1817: `Bezier.quadratic(this.points[i], (float)i / 20.0f, start, control, end, this.arrowTmp)`
- L1822: `sb.draw(ImageMaster.TARGET_UI_CIRCLE, this.points[i].x - 64.0f, this.points[i].y - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, `
- L1822: `this.arrowTmp.nor().angle()`
- L1822: `this.arrowTmp.nor()`
- L1827: `sb.draw(ImageMaster.TARGET_UI_CIRCLE, this.points[i].x - 64.0f, this.points[i].y - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, `
- L1827: `this.arrowTmp.nor().angle()`
- L1827: `this.arrowTmp.nor()`

### createHandIsFullDialog() (L1831-1833)

**Calls (in order):**
- L1832: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, MSG[2], true))`

**Objects created:**
- L1832: `ThoughtBubble`

### renderHoverReticle(SpriteBatch sb) (L1835-1865)

**Calls (in order):**
- L1839: `this.hoveredMonster.renderReticle(sb)`
- L1843: `AbstractDungeon.getCurrRoom().monsters.renderReticle(sb)`
- L1843: `AbstractDungeon.getCurrRoom()`
- L1847: `this.renderReticle(sb)`
- L1851: `this.renderReticle(sb)`
- L1853: `this.hoveredMonster.renderReticle(sb)`
- L1857: `this.renderReticle(sb)`
- L1858: `AbstractDungeon.getCurrRoom().monsters.renderReticle(sb)`
- L1858: `AbstractDungeon.getCurrRoom()`

### applyPreCombatLogic() (L1867-1872)

**Calls (in order):**
- L1870: `r.atPreBattle()`

### applyStartOfCombatLogic() (L1874-1883)

**Calls (in order):**
- L1877: `r.atBattleStart()`
- L1881: `b.atBattleStart()`

### applyStartOfCombatPreDrawLogic() (L1885-1890)

**Calls (in order):**
- L1888: `r.atBattleStartPreDraw()`

### applyStartOfTurnRelics() (L1892-1902)

**Calls (in order):**
- L1893: `this.stance.atStartOfTurn()`
- L1896: `r.atTurnStart()`
- L1900: `b.atTurnStart()`

### applyStartOfTurnPostDrawRelics() (L1904-1909)

**Calls (in order):**
- L1907: `r.atTurnStartPostDraw()`

### applyStartOfTurnPreDrawCards() (L1911-1916)

**Calls (in order):**
- L1914: `c.atTurnStartPreDraw()`

### applyStartOfTurnCards() (L1918-1931)

**Calls (in order):**
- L1921: `c.atTurnStart()`
- L1925: `c.atTurnStart()`
- L1929: `c.atTurnStart()`

### onVictory() (L1933-1946)

**Calls (in order):**
- L1936: `r.onVictory()`
- L1939: `b.onVictory()`
- L1942: `p.onVictory()`

### hasRelic(String targetID) (L1948-1954)

**Calls (in order):**
- L1950: `r.relicId.equals(targetID)`

### hasBlight(String targetID) (L1956-1962)

**Calls (in order):**
- L1958: `b.blightID.equals(targetID)`

### hasPotion(String id) (L1964-1970)

**Calls (in order):**
- L1966: `p.ID.equals(id)`

### loseRandomRelics(int amount) (L1980-1994)

**Calls (in order):**
- L1981: `this.relics.size()`
- L1983: `r.onUnequip()`
- L1985: `this.relics.clear()`
- L1989: `MathUtils.random(0, this.relics.size() - 1)`
- L1989: `this.relics.size()`
- L1990: `this.relics.get(index).onUnequip()`
- L1990: `this.relics.get(index)`
- L1991: `this.relics.remove(index)`
- L1993: `this.reorganizeRelics()`

### loseRelic(String targetID) (L1996-2013)

**Calls (in order):**
- L1997: `this.hasRelic(targetID)`
- L2002: `r.relicId.equals(targetID)`
- L2003: `r.onUnequip()`
- L2007: `logger.info("WHY WAS RELIC: " + this.name + " NOT FOUND???")`
- L2010: `this.relics.remove(toRemove)`
- L2011: `this.reorganizeRelics()`

### reorganizeRelics() (L2015-2023)

**Calls (in order):**
- L2016: `logger.info("Reorganizing relics")`
- L2018: `tmpRelics.addAll(this.relics)`
- L2019: `this.relics.clear()`
- L2020: `tmpRelics.size()`
- L2021: `((AbstractRelic)tmpRelics.get(i)).reorganizeObtain(this, i, false, tmpRelics.size())`
- L2021: `tmpRelics.get(i)`
- L2021: `tmpRelics.size()`

**Objects created:**
- L2017: `None`

### getRelic(String targetID) (L2025-2031)

**Calls (in order):**
- L2027: `r.relicId.equals(targetID)`

### getBlight(String targetID) (L2033-2039)

**Calls (in order):**
- L2035: `b.blightID.equals(targetID)`

### obtainPotion(int slot, AbstractPotion potionToObtain) (L2041-2047)

**Calls (in order):**
- L2045: `this.potions.set(slot, potionToObtain)`
- L2046: `potionToObtain.setAsObtained(slot)`

### obtainPotion(AbstractPotion potionToObtain) (L2049-2065)

**Calls (in order):**
- L2056: `this.potions.set(index, potionToObtain)`
- L2057: `potionToObtain.setAsObtained(index)`
- L2058: `potionToObtain.flash()`
- L2059: `AbstractPotion.playPotionSound()`
- L2062: `logger.info("NOT ENOUGH POTION SLOTS")`
- L2063: `AbstractDungeon.topPanel.flashRed()`

### renderRelics(SpriteBatch sb) (L2067-2076)

**Calls (in order):**
- L2068: `this.relics.size()`
- L2070: `this.relics.get(i).renderInTopPanel(sb)`
- L2070: `this.relics.get(i)`
- L2074: `r.renderTip(sb)`

### renderBlights(SpriteBatch sb) (L2078-2086)

**Calls (in order):**
- L2080: `b.renderInTopPanel(sb)`
- L2084: `b.renderTip(sb)`

### bottledCardUpgradeCheck(AbstractCard c) (L2088-2098)

**Calls (in order):**
- L2089: `this.hasRelic("Bottled Flame")`
- L2090: `((BottledFlame)this.getRelic("Bottled Flame")).setDescriptionAfterLoading()`
- L2090: `this.getRelic("Bottled Flame")`
- L2092: `this.hasRelic("Bottled Lightning")`
- L2093: `((BottledLightning)this.getRelic("Bottled Lightning")).setDescriptionAfterLoading()`
- L2093: `this.getRelic("Bottled Lightning")`
- L2095: `this.hasRelic("Bottled Tornado")`
- L2096: `((BottledTornado)this.getRelic("Bottled Tornado")).setDescriptionAfterLoading()`
- L2096: `this.getRelic("Bottled Tornado")`

### triggerEvokeAnimation(int slot) (L2100-2105)

**Calls (in order):**
- L2104: `this.orbs.get(slot).triggerEvokeAnimation()`
- L2104: `this.orbs.get(slot)`

### evokeOrb() (L2107-2120)

**Calls (in order):**
- L2108: `this.orbs.isEmpty()`
- L2108: `this.orbs.get(0)`
- L2110: `this.orbs.get(0).onEvoke()`
- L2110: `this.orbs.get(0)`
- L2112: `this.orbs.size()`
- L2113: `Collections.swap(this.orbs, i, i - 1)`
- L2115: `this.orbs.set(this.orbs.size() - 1, orbSlot)`
- L2115: `this.orbs.size()`
- L2116: `this.orbs.size()`
- L2117: `this.orbs.get(i).setSlot(i, this.maxOrbs)`
- L2117: `this.orbs.get(i)`

**Objects created:**
- L2111: `EmptyOrbSlot`

### evokeNewestOrb() (L2122-2126)

**Calls (in order):**
- L2123: `this.orbs.isEmpty()`
- L2123: `this.orbs.get(this.orbs.size() - 1)`
- L2123: `this.orbs.size()`
- L2124: `this.orbs.get(this.orbs.size() - 1).onEvoke()`
- L2124: `this.orbs.get(this.orbs.size() - 1)`
- L2124: `this.orbs.size()`

### evokeWithoutLosingOrb() (L2128-2132)

**Calls (in order):**
- L2129: `this.orbs.isEmpty()`
- L2129: `this.orbs.get(0)`
- L2130: `this.orbs.get(0).onEvoke()`
- L2130: `this.orbs.get(0)`

### removeNextOrb() (L2134-2146)

**Calls (in order):**
- L2135: `this.orbs.isEmpty()`
- L2135: `this.orbs.get(0)`
- L2137: `this.orbs.get((int)0)`
- L2137: `this.orbs.get((int)0)`
- L2138: `this.orbs.size()`
- L2139: `Collections.swap(this.orbs, i, i - 1)`
- L2141: `this.orbs.set(this.orbs.size() - 1, orbSlot)`
- L2141: `this.orbs.size()`
- L2142: `this.orbs.size()`
- L2143: `this.orbs.get(i).setSlot(i, this.maxOrbs)`
- L2143: `this.orbs.get(i)`

**Objects created:**
- L2137: `EmptyOrbSlot`

### hasEmptyOrb() (L2148-2157)

**Calls (in order):**
- L2149: `this.orbs.isEmpty()`

### hasOrb() (L2159-2164)

**Calls (in order):**
- L2160: `this.orbs.isEmpty()`
- L2163: `this.orbs.get(0)`

### channelOrb(AbstractOrb orbToSet) (L2175-2216)

**Calls (in order):**
- L2177: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, MSG[4], true))`
- L2181: `this.hasRelic("Dark Core")`
- L2185: `this.orbs.size()`
- L2186: `this.orbs.get(i)`
- L2191: `this.orbs.get((int)index)`
- L2192: `this.orbs.get((int)index)`
- L2193: `this.orbs.set(index, orbToSet)`
- L2194: `this.orbs.get(index).setSlot(index, this.maxOrbs)`
- L2194: `this.orbs.get(index)`
- L2195: `orbToSet.playChannelSFX()`
- L2197: `p.onChannel(orbToSet)`
- L2199: `AbstractDungeon.actionManager.orbsChanneledThisCombat.add(orbToSet)`
- L2200: `AbstractDungeon.actionManager.orbsChanneledThisTurn.add(orbToSet)`
- L2207: `UnlockTracker.unlockAchievement("NEON")`
- L2209: `orbToSet.applyFocus()`
- L2211: `AbstractDungeon.actionManager.addToTop(new ChannelAction(orbToSet))`
- L2212: `AbstractDungeon.actionManager.addToTop(new EvokeOrbAction(1))`
- L2213: `AbstractDungeon.actionManager.addToTop(new AnimateOrbAction(1))`

**Objects created:**
- L2177: `ThoughtBubble`
- L2182: `Dark`
- L2211: `ChannelAction`
- L2212: `EvokeOrbAction`
- L2213: `AnimateOrbAction`

### increaseMaxOrbSlots(int amount, boolean playSfx) (L2218-2234)

**Calls (in order):**
- L2221: `AbstractDungeon.effectList.add(new ThoughtBubble(this.dialogX, this.dialogY, 3.0f, MSG[3], true))`
- L2225: `CardCrawlGame.sound.play("ORB_SLOT_GAIN", 0.1f)`
- L2229: `this.orbs.add(new EmptyOrbSlot())`
- L2231: `this.orbs.size()`
- L2232: `this.orbs.get(i).setSlot(i, this.maxOrbs)`
- L2232: `this.orbs.get(i)`

**Objects created:**
- L2221: `ThoughtBubble`
- L2229: `EmptyOrbSlot`

### decreaseMaxOrbSlots(int amount) (L2236-2250)

**Calls (in order):**
- L2244: `this.orbs.isEmpty()`
- L2245: `this.orbs.remove(this.orbs.size() - 1)`
- L2245: `this.orbs.size()`
- L2247: `this.orbs.size()`
- L2248: `this.orbs.get(i).setSlot(i, this.maxOrbs)`
- L2248: `this.orbs.get(i)`

### applyStartOfTurnOrbs() (L2252-2261)

**Calls (in order):**
- L2253: `this.orbs.isEmpty()`
- L2255: `o.onStartOfTurn()`
- L2257: `this.hasRelic("Cables")`
- L2257: `this.orbs.get(0)`
- L2258: `this.orbs.get(0).onStartOfTurn()`
- L2258: `this.orbs.get(0)`

### updateEscapeAnimation() (L2263-2274)

**Calls (in order):**
- L2265: `Gdx.graphics.getDeltaTime()`
- L2266: `Gdx.graphics.getDeltaTime()`
- L2266: `Gdx.graphics.getDeltaTime()`
- L2269: `AbstractDungeon.getCurrRoom().endBattle()`
- L2269: `AbstractDungeon.getCurrRoom()`

### resetControllerValues() (L2284-2293)

**Calls (in order):**
- L2291: `this.hand.refreshHandLayout()`

### getRandomPotion() (L2295-2306)

**Calls (in order):**
- L2299: `list.add(p)`
- L2301: `list.isEmpty()`
- L2304: `Collections.shuffle(list, new Random(AbstractDungeon.miscRng.randomLong()))`
- L2304: `AbstractDungeon.miscRng.randomLong()`
- L2305: `list.get(0)`

**Objects created:**
- L2296: `None`
- L2304: `Random`

### removePotion(AbstractPotion potionOption) (L2308-2313)

**Calls (in order):**
- L2309: `this.potions.indexOf(potionOption)`
- L2311: `this.potions.set(slot, new PotionSlot(slot))`

**Objects created:**
- L2311: `PotionSlot`

### movePosition(float x, float y) (L2315-2323)

**Calls (in order):**
- L2322: `this.refreshHitboxLocation()`

### switchedStance() (L2325-2335)

**Calls (in order):**
- L2327: `c.switchedStance()`
- L2330: `c.switchedStance()`
- L2333: `c.switchedStance()`

## AbstractPower
File: `powers\AbstractPower.java`

### initialize() (L64-66)

**Calls (in order):**
- L65: `Gdx.files.internal("powers/powers.atlas")`

**Objects created:**
- L65: `TextureAtlas`

### loadRegion(String fileName) (L68-71)

**Calls (in order):**
- L69: `atlas.findRegion("48/" + fileName)`
- L70: `atlas.findRegion("128/" + fileName)`

### playApplyPowerSfx() (L77-97)

**Calls (in order):**
- L79: `MathUtils.random(0, 2)`
- L81: `CardCrawlGame.sound.play("BUFF_1")`
- L83: `CardCrawlGame.sound.play("BUFF_2")`
- L85: `CardCrawlGame.sound.play("BUFF_3")`
- L88: `MathUtils.random(0, 2)`
- L90: `CardCrawlGame.sound.play("DEBUFF_1")`
- L92: `CardCrawlGame.sound.play("DEBUFF_2")`
- L94: `CardCrawlGame.sound.play("DEBUFF_3")`

### update(int slot) (L102-106)

**Calls (in order):**
- L103: `this.updateFlash()`
- L104: `this.updateFontScale()`
- L105: `this.updateColor()`

### addToBot(AbstractGameAction action) (L108-110)

**Calls (in order):**
- L109: `AbstractDungeon.actionManager.addToBottom(action)`

### addToTop(AbstractGameAction action) (L112-114)

**Calls (in order):**
- L113: `AbstractDungeon.actionManager.addToTop(action)`

### updateFlash() (L116-124)

**Calls (in order):**
- L117: `this.effect.iterator()`
- L118: `i.hasNext()`
- L119: `i.next()`
- L120: `e.update()`
- L122: `i.remove()`

### updateColor() (L126-130)

**Calls (in order):**
- L128: `MathHelper.fadeLerpSnap(this.color.a, 1.0f)`

### updateFontScale() (L132-139)

**Calls (in order):**
- L134: `MathUtils.lerp(this.fontScale, 1.0f, Gdx.graphics.getDeltaTime() * 10.0f)`
- L134: `Gdx.graphics.getDeltaTime()`

### stackPower(int stackAmount) (L144-151)

**Calls (in order):**
- L146: `logger.info(this.name + " does not stack")`

### renderIcons(SpriteBatch sb, float x, float y, Color c) (L167-182)

**Calls (in order):**
- L169: `sb.setColor(c)`
- L170: `sb.draw(this.img, x - 12.0f, y - 12.0f, 16.0f, 16.0f, 32.0f, 32.0f, Settings.scale * 1.5f, Settings.scale * 1.5f, 0.0f, `
- L172: `sb.setColor(c)`
- L174: `sb.draw(this.region48, x - (float)this.region48.packedWidth / 2.0f, y - (float)this.region48.packedHeight / 2.0f, (float`
- L176: `sb.draw(this.region48, x - (float)this.region48.packedWidth / 2.0f, y - (float)this.region48.packedHeight / 2.0f, (float`
- L180: `e.render(sb, x, y)`

### renderAmount(SpriteBatch sb, float x, float y, Color c) (L184-196)

**Calls (in order):**
- L190: `FontHelper.renderFontRightTopAligned(sb, FontHelper.powerAmountFont, Integer.toString(this.amount), x, y, this.fontScale`
- L190: `Integer.toString(this.amount)`
- L194: `FontHelper.renderFontRightTopAligned(sb, FontHelper.powerAmountFont, Integer.toString(this.amount), x, y, this.fontScale`
- L194: `Integer.toString(this.amount)`

### atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) (L214-216)

**Calls (in order):**
- L215: `this.atDamageGive(damage, type)`

### atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card) (L218-220)

**Calls (in order):**
- L219: `this.atDamageFinalGive(damage, type)`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card) (L222-224)

**Calls (in order):**
- L223: `this.atDamageFinalReceive(damage, type)`

### atDamageReceive(float damage, DamageInfo.DamageType damageType, AbstractCard card) (L226-228)

**Calls (in order):**
- L227: `this.atDamageReceive(damage, damageType)`

### modifyBlock(float blockAmount, AbstractCard card) (L319-321)

**Calls (in order):**
- L320: `this.modifyBlock(blockAmount)`

### onPlayerGainedBlock(float blockAmount) (L330-332)

**Calls (in order):**
- L331: `MathUtils.floor(blockAmount)`

### flash() (L361-364)

**Calls (in order):**
- L362: `this.effect.add(new GainPowerEffect(this))`
- L363: `AbstractDungeon.effectList.add(new FlashPowerEffect(this))`

**Objects created:**
- L362: `GainPowerEffect`
- L363: `FlashPowerEffect`

### flashWithoutSound() (L366-369)

**Calls (in order):**
- L367: `this.effect.add(new SilentGainPowerEffect(this))`
- L368: `AbstractDungeon.effectList.add(new FlashPowerEffect(this))`

**Objects created:**
- L367: `SilentGainPowerEffect`
- L368: `FlashPowerEffect`

### getLocStrings() (L374-379)

**Calls (in order):**
- L376: `powerData.put("name", (Serializable)((Object)this.name))`
- L377: `powerData.put("description", (Serializable)DESCRIPTIONS)`

**Objects created:**
- L375: `None`

## AbstractRelic
File: `relics\AbstractRelic.java`

### usedUp() (L132-139)

**Calls (in order):**
- L136: `this.tips.clear()`
- L137: `this.tips.add(new PowerTip(this.name, this.description))`
- L138: `this.initializeTips()`

**Objects created:**
- L137: `PowerTip`

### spawn(float x, float y) (L141-159)

**Calls (in order):**
- L142: `AbstractDungeon.getCurrRoom()`
- L143: `AbstractDungeon.effectsQueue.add(new SmokePuffEffect(x, y))`

**Objects created:**
- L143: `SmokePuffEffect`

### reorganizeObtain(AbstractPlayer p, int slot, boolean callOnEquip, int relicAmount) (L191-205)

**Calls (in order):**
- L194: `p.relics.add(this)`
- L199: `this.hb.move(this.currentX, this.currentY)`
- L201: `this.onEquip()`
- L202: `this.relicTip()`
- L204: `UnlockTracker.markRelicAsSeen(this.relicId)`

### instantObtain(AbstractPlayer p, int slot, boolean callOnEquip) (L207-238)

**Calls (in order):**
- L208: `this.relicId.equals("Circlet")`
- L208: `p.hasRelic("Circlet")`
- L209: `p.getRelic("Circlet")`
- L211: `circ.flash()`
- L218: `p.relics.size()`
- L219: `p.relics.add(this)`
- L221: `p.relics.set(slot, this)`
- L227: `this.hb.move(this.currentX, this.currentY)`
- L229: `this.onEquip()`
- L230: `this.relicTip()`
- L232: `UnlockTracker.markRelicAsSeen(this.relicId)`
- L233: `this.getUpdatedDescription()`
- L235: `AbstractDungeon.topPanel.adjustRelicHbs()`

### instantObtain() (L240-263)

**Calls (in order):**
- L241: `AbstractDungeon.player.hasRelic("Circlet")`
- L242: `AbstractDungeon.player.getRelic("Circlet")`
- L244: `circ.flash()`
- L246: `this.playLandingSFX()`
- L249: `AbstractDungeon.player.relics.size()`
- L253: `this.flash()`
- L254: `AbstractDungeon.player.relics.add(this)`
- L255: `this.hb.move(this.currentX, this.currentY)`
- L256: `this.onEquip()`
- L257: `this.relicTip()`
- L258: `UnlockTracker.markRelicAsSeen(this.relicId)`
- L261: `AbstractDungeon.topPanel.adjustRelicHbs()`

### obtain() (L265-280)

**Calls (in order):**
- L266: `AbstractDungeon.player.hasRelic("Circlet")`
- L267: `AbstractDungeon.player.getRelic("Circlet")`
- L269: `circ.flash()`
- L273: `AbstractDungeon.player.relics.size()`
- L276: `AbstractDungeon.player.relics.add(this)`
- L277: `this.relicTip()`
- L278: `UnlockTracker.markRelicAsSeen(this.relicId)`

### getColumn() (L282-284)

**Calls (in order):**
- L283: `AbstractDungeon.player.relics.indexOf(this)`

### relicTip() (L286-291)

**Calls (in order):**
- L287: `TipTracker.tips.get("RELIC_TIP").booleanValue()`
- L287: `TipTracker.tips.get("RELIC_TIP")`
- L289: `TipTracker.neverShowAgain("RELIC_TIP")`

**Objects created:**
- L288: `FtueTip`

### update() (L297-377)

**Calls (in order):**
- L298: `this.updateFlash()`
- L301: `Gdx.graphics.getDeltaTime()`
- L304: `AbstractDungeon.effectList.add(new GlowRelicParticle(this.img, this.currentX + this.f_effect.x, this.currentY + this.f_e`
- L306: `this.f_effect.update()`
- L307: `MathHelper.scaleLerpSnap(this.scale, Settings.scale * 1.1f)`
- L309: `MathHelper.scaleLerpSnap(this.scale, Settings.scale)`
- L313: `MathUtils.lerp(this.rotation, 0.0f, Gdx.graphics.getDeltaTime() * 6.0f * 2.0f)`
- L313: `Gdx.graphics.getDeltaTime()`
- L316: `MathUtils.lerp(this.currentX, this.targetX, Gdx.graphics.getDeltaTime() * 6.0f)`
- L316: `Gdx.graphics.getDeltaTime()`
- L317: `Math.abs(this.currentX - this.targetX)`
- L322: `MathUtils.lerp(this.currentY, this.targetY, Gdx.graphics.getDeltaTime() * 6.0f)`
- L322: `Gdx.graphics.getDeltaTime()`
- L323: `Math.abs(this.currentY - this.targetY)`
- L330: `AbstractDungeon.topPanel.adjustRelicHbs()`
- L332: `this.hb.move(this.currentX, this.currentY)`
- L333: `AbstractDungeon.getCurrRoom()`
- L334: `AbstractDungeon.overlayMenu.proceedButton.show()`
- L336: `this.onEquip()`
- L341: `this.hb.update()`
- L347: `CInputActionSet.select.isJustPressed()`
- L348: `CInputActionSet.select.unpress()`
- L351: `this.bossObtainLogic()`
- L353: `AbstractDungeon.bossRelicScreen.confirmButton.show()`
- L361: `this.updateAnimation()`
- L364: `AbstractDungeon.player.relics.indexOf(this)`
- L365: `this.hb.update()`
- L371: `CardCrawlGame.cursor.changeType(GameCursor.CursorType.INSPECT)`
- L373: `MathHelper.scaleLerpSnap(this.scale, Settings.scale)`
- L375: `this.updateRelicPopupClick()`

**Objects created:**
- L304: `GlowRelicParticle`

### bossObtainLogic() (L379-386)

**Calls (in order):**
- L380: `this.relicId.equals("HolyWater")`
- L380: `this.relicId.equals("Black Blood")`
- L380: `this.relicId.equals("Ring of the Serpent")`
- L380: `this.relicId.equals("FrozenCore")`
- L381: `this.obtain()`

### updateRelicPopupClick() (L388-398)

**Calls (in order):**
- L392: `CInputActionSet.select.isJustPressed()`
- L393: `CardCrawlGame.relicPopup.open(this, AbstractDungeon.player.relics)`
- L394: `CInputActionSet.select.unpress()`

### playLandingSFX() (L407-433)

**Calls (in order):**
- L410: `CardCrawlGame.sound.play("RELIC_DROP_CLINK")`
- L414: `CardCrawlGame.sound.play("RELIC_DROP_FLAT")`
- L418: `CardCrawlGame.sound.play("RELIC_DROP_ROCKY")`
- L422: `CardCrawlGame.sound.play("RELIC_DROP_HEAVY")`
- L426: `CardCrawlGame.sound.play("RELIC_DROP_MAGICAL")`
- L430: `CardCrawlGame.sound.play("RELIC_DROP_CLINK")`

### updateAnimation() (L435-442)

**Calls (in order):**
- L437: `Gdx.graphics.getDeltaTime()`

### updateFlash() (L444-451)

**Calls (in order):**
- L446: `Gdx.graphics.getDeltaTime()`

### onPlayerGainedBlock(float blockAmount) (L526-528)

**Calls (in order):**
- L527: `MathUtils.floor(blockAmount)`

### renderInTopPanel(SpriteBatch sb) (L630-646)

**Calls (in order):**
- L634: `this.renderOutline(sb, true)`
- L636: `ShaderHelper.setShader(sb, ShaderHelper.Shader.GRAYSCALE)`
- L638: `sb.setColor(Color.WHITE)`
- L639: `sb.draw(this.img, this.currentX - 64.0f + offsetX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this`
- L641: `ShaderHelper.setShader(sb, ShaderHelper.Shader.DEFAULT)`
- L643: `this.renderCounter(sb, true)`
- L644: `this.renderFlash(sb, true)`
- L645: `this.hb.render(sb)`

### render(SpriteBatch sb) (L648-685)

**Calls (in order):**
- L652: `this.renderOutline(sb, false)`
- L655: `this.renderBossTip(sb)`
- L659: `sb.setColor(PASSIVE_OUTLINE_COLOR)`
- L660: `sb.draw(this.outlineImg, this.currentX - 64.0f + this.f_effect.x, this.currentY - 64.0f + this.f_effect.y, 64.0f, 64.0f,`
- L662: `sb.setColor(PASSIVE_OUTLINE_COLOR)`
- L663: `sb.draw(this.outlineImg, this.currentX - 64.0f + this.f_effect.x, this.currentY - 64.0f + this.f_effect.y, 64.0f, 64.0f,`
- L669: `sb.setColor(Color.WHITE)`
- L670: `sb.draw(this.img, this.currentX - 64.0f + this.f_effect.x, this.currentY - 64.0f + this.f_effect.y, 64.0f, 64.0f, 128.0f`
- L672: `sb.setColor(Color.WHITE)`
- L673: `sb.draw(this.img, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, th`
- L674: `this.renderCounter(sb, false)`
- L677: `sb.setColor(Color.WHITE)`
- L678: `sb.draw(this.img, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, th`
- L679: `this.renderCounter(sb, false)`
- L682: `this.renderFlash(sb, false)`
- L684: `this.hb.render(sb)`

### renderLock(SpriteBatch sb, Color outlineColor) (L687-717)

**Calls (in order):**
- L688: `sb.setColor(outlineColor)`
- L689: `sb.draw(ImageMaster.RELIC_LOCK_OUTLINE, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this`
- L690: `sb.setColor(Color.WHITE)`
- L691: `sb.draw(ImageMaster.RELIC_LOCK, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, `
- L693: `UnlockTracker.unlockReqs.get(this.relicId)`
- L700: `TipHelper.renderGenericTip((float)InputHelper.mX + 60.0f * Settings.scale, (float)InputHelper.mY + 100.0f * Settings.sca`
- L702: `TipHelper.renderGenericTip((float)InputHelper.mX + 60.0f * Settings.scale, (float)InputHelper.mY - 50.0f * Settings.scal`
- L705: `TipHelper.renderGenericTip((float)InputHelper.mX - 350.0f * Settings.scale, (float)InputHelper.mY - 50.0f * Settings.sca`
- L713: `sb.setColor(Color.WHITE)`
- L714: `sb.draw(ImageMaster.RELIC_LOCK, tmpX - 64.0f, tmpY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, this.r`
- L716: `this.hb.render(sb)`

### render(SpriteBatch sb, boolean renderAmount, Color outlineColor) (L719-756)

**Calls (in order):**
- L721: `this.renderOutline(outlineColor, sb, false)`
- L723: `this.renderOutline(Color.LIGHT_GRAY, sb, false)`
- L726: `sb.setColor(Color.WHITE)`
- L728: `sb.setColor(Settings.HALF_TRANSPARENT_BLACK_COLOR)`
- L730: `sb.setColor(Color.BLACK)`
- L734: `sb.draw(this.img, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, Settings.scale * 2.0f + Ma`
- L734: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L734: `System.currentTimeMillis()`
- L734: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L734: `System.currentTimeMillis()`
- L736: `sb.draw(this.largeImg, this.currentX - 128.0f, this.currentY - 128.0f, 128.0f, 128.0f, 256.0f, 256.0f, Settings.scale * `
- L736: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L736: `System.currentTimeMillis()`
- L736: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L736: `System.currentTimeMillis()`
- L739: `sb.draw(this.img, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, th`
- L740: `this.relicId.equals("Circlet")`
- L741: `this.renderCounter(sb, false)`
- L747: `TipHelper.renderGenericTip((float)InputHelper.mX + 60.0f * Settings.scale, (float)InputHelper.mY - 50.0f * Settings.scal`
- L749: `TipHelper.renderGenericTip((float)InputHelper.mX - 350.0f * Settings.scale, (float)InputHelper.mY - 50.0f * Settings.sca`
- L753: `this.renderTip(sb)`
- L755: `this.hb.render(sb)`

### renderWithoutAmount(SpriteBatch sb, Color c) (L758-774)

**Calls (in order):**
- L759: `this.renderOutline(c, sb, false)`
- L760: `sb.setColor(Color.WHITE)`
- L761: `sb.draw(this.img, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, th`
- L763: `this.renderTip(sb)`
- L770: `sb.setColor(Color.WHITE)`
- L771: `sb.draw(this.img, tmpX - 64.0f, tmpY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, this.rotation, 0, 0,`
- L773: `this.hb.render(sb)`

### renderCounter(SpriteBatch sb, boolean inTopPanel) (L776-784)

**Calls (in order):**
- L779: `FontHelper.renderFontRightTopAligned(sb, FontHelper.topPanelInfoFont, Integer.toString(this.counter), offsetX + this.cur`
- L779: `Integer.toString(this.counter)`
- L781: `FontHelper.renderFontRightTopAligned(sb, FontHelper.topPanelInfoFont, Integer.toString(this.counter), this.currentX + 30`
- L781: `Integer.toString(this.counter)`

### renderOutline(Color c, SpriteBatch sb, boolean inTopPanel) (L786-799)

**Calls (in order):**
- L787: `sb.setColor(c)`
- L789: `sb.draw(this.outlineImg, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, Settings.scale * 2.`
- L789: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L789: `System.currentTimeMillis()`
- L789: `MathUtils.cosDeg(System.currentTimeMillis() / 5L % 360L)`
- L789: `System.currentTimeMillis()`
- L791: `sb.setBlendFunction(770, 1)`
- L792: `MathUtils.cosDeg(System.currentTimeMillis() / 2L % 360L)`
- L792: `System.currentTimeMillis()`
- L793: `sb.setColor(this.goldOutlineColor)`
- L794: `sb.draw(this.outlineImg, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.sc`
- L795: `sb.setBlendFunction(770, 771)`
- L797: `sb.draw(this.outlineImg, this.currentX - 64.0f, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.sc`

### renderOutline(SpriteBatch sb, boolean inTopPanel) (L801-816)

**Calls (in order):**
- L807: `sb.setBlendFunction(770, 1)`
- L808: `MathUtils.cosDeg(System.currentTimeMillis() / 2L % 360L)`
- L808: `System.currentTimeMillis()`
- L809: `sb.setColor(this.goldOutlineColor)`
- L810: `sb.draw(this.outlineImg, tmpX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, this.rotatio`
- L811: `sb.setBlendFunction(770, 771)`
- L813: `sb.setColor(PASSIVE_OUTLINE_COLOR)`
- L814: `sb.draw(this.outlineImg, tmpX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale, this.scale, this.rotatio`

### renderFlash(SpriteBatch sb, boolean inTopPanel) (L818-831)

**Calls (in order):**
- L819: `Interpolation.exp10In.apply(0.0f, 4.0f, this.flashTimer / 2.0f)`
- L820: `sb.setBlendFunction(770, 1)`
- L822: `sb.setColor(this.flashColor)`
- L827: `sb.draw(this.img, tmpX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale + tmp, this.scale + tmp, this.ro`
- L828: `sb.draw(this.img, tmpX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale + tmp * 0.66f, this.scale + tmp `
- L829: `sb.draw(this.img, tmpX, this.currentY - 64.0f, 64.0f, 64.0f, 128.0f, 128.0f, this.scale + tmp / 3.0f, this.scale + tmp /`
- L830: `sb.setBlendFunction(770, 771)`

### renderBossTip(SpriteBatch sb) (L850-852)

**Calls (in order):**
- L851: `TipHelper.queuePowerTips((float)Settings.WIDTH * 0.63f, (float)Settings.HEIGHT * 0.63f, this.tips)`

### renderTip(SpriteBatch sb) (L854-870)

**Calls (in order):**
- L857: `TipHelper.queuePowerTips(180.0f * Settings.scale, (float)Settings.HEIGHT * 0.7f, this.tips)`
- L858: `this.tips.size()`
- L858: `AbstractDungeon.player.hasRelic(this.relicId)`
- L859: `TipHelper.queuePowerTips((float)InputHelper.mX + 60.0f * Settings.scale, (float)InputHelper.mY + 180.0f * Settings.scale`
- L860: `AbstractDungeon.player.hasRelic(this.relicId)`
- L861: `TipHelper.queuePowerTips((float)InputHelper.mX + 60.0f * Settings.scale, (float)InputHelper.mY - 30.0f * Settings.scale,`
- L863: `TipHelper.queuePowerTips(360.0f * Settings.scale, (float)InputHelper.mY + 50.0f * Settings.scale, this.tips)`
- L865: `TipHelper.queuePowerTips((float)InputHelper.mX + 50.0f * Settings.scale, (float)InputHelper.mY + 50.0f * Settings.scale,`
- L868: `TipHelper.queuePowerTips((float)InputHelper.mX - 350.0f * Settings.scale, (float)InputHelper.mY - 50.0f * Settings.scale`

### gameDataUploadHeader() (L876-887)

**Calls (in order):**
- L878: `builder.addFieldData("name")`
- L879: `builder.addFieldData("relicID")`
- L880: `builder.addFieldData("color")`
- L881: `builder.addFieldData("description")`
- L882: `builder.addFieldData("flavorText")`
- L883: `builder.addFieldData("cost")`
- L884: `builder.addFieldData("tier")`
- L885: `builder.addFieldData("assetURL")`
- L886: `builder.toString()`

**Objects created:**
- L877: `GameDataStringBuilder`

### initializeTips() (L889-912)

**Calls (in order):**
- L891: `desc.hasNext()`
- L892: `desc.next()`
- L893: `s.charAt(0)`
- L894: `s.substring(2)`
- L896: `s.replace(',', ' ')`
- L897: `s.replace('.', ' ')`
- L898: `s.trim()`
- L899: `s.toLowerCase()`
- L901: `GameDictionary.keywords.containsKey(s)`
- L902: `GameDictionary.parentWord.get(s)`
- L904: `t.header.toLowerCase().equals(s)`
- L904: `t.header.toLowerCase()`
- L909: `this.tips.add(new PowerTip(TipHelper.capitalize(s), GameDictionary.keywords.get(s)))`
- L909: `TipHelper.capitalize(s)`
- L909: `GameDictionary.keywords.get(s)`
- L911: `desc.close()`

**Objects created:**
- L890: `Scanner`
- L909: `PowerTip`

### gameDataUploadData(String color) (L914-925)

**Calls (in order):**
- L916: `builder.addFieldData(this.name)`
- L917: `builder.addFieldData(this.relicId)`
- L918: `builder.addFieldData(color)`
- L919: `builder.addFieldData(this.description)`
- L920: `builder.addFieldData(this.flavorText)`
- L921: `builder.addFieldData(this.cost)`
- L922: `builder.addFieldData(this.tier.name())`
- L922: `this.tier.name()`
- L923: `builder.addFieldData(this.assetURL)`
- L924: `builder.toString()`

**Objects created:**
- L915: `GameDataStringBuilder`

### compareTo(AbstractRelic arg0) (L933-936)

**Calls (in order):**
- L935: `this.name.compareTo(arg0.name)`

### getLocStrings() (L942-947)

**Calls (in order):**
- L944: `relicData.put("name", (Serializable)((Object)this.name))`
- L945: `relicData.put("description", (Serializable)((Object)this.description))`

**Objects created:**
- L943: `None`

### updateOffsetX() (L962-970)

**Calls (in order):**
- L964: `AbstractDungeon.player.relics.size()`
- L968: `MathHelper.uiLerpSnap(offsetX, target)`

### loadLargeImg() (L972-976)

**Calls (in order):**
- L974: `ImageMaster.loadImage(L_IMG_DIR + this.imgUrl)`

### addToBot(AbstractGameAction action) (L978-980)

**Calls (in order):**
- L979: `AbstractDungeon.actionManager.addToBottom(action)`

### addToTop(AbstractGameAction action) (L982-984)

**Calls (in order):**
- L983: `AbstractDungeon.actionManager.addToTop(action)`

## AbstractRoom
File: `rooms\AbstractRoom.java`

### playBGM(String key) (L122-124)

**Calls (in order):**
- L123: `CardCrawlGame.music.playTempBGM(key)`

### playBgmInstantly(String key) (L126-128)

**Calls (in order):**
- L127: `CardCrawlGame.music.playTempBgmInstantly(key)`

### getCardRarity(int roll) (L138-140)

**Calls (in order):**
- L139: `this.getCardRarity(roll, true)`

### getCardRarity(int roll, boolean useAlternation) (L142-167)

**Calls (in order):**
- L146: `this.alterCardRarityProbabilities()`
- L151: `r.changeRareCardRewardChance(this.baseRareCardChance)`
- L152: `r.flash()`
- L160: `r.changeUncommonCardRewardChance(this.baseUncommonCardChance)`
- L161: `r.flash()`

### alterCardRarityProbabilities() (L169-176)

**Calls (in order):**
- L171: `r.changeRareCardRewardChance(this.rareCardChance)`
- L174: `r.changeUncommonCardRewardChance(this.uncommonCardChance)`

### updateObjects() (L178-194)

**Calls (in order):**
- L179: `this.souls.update()`
- L180: `this.potions.iterator()`
- L181: `i.hasNext()`
- L182: `i.next()`
- L183: `tmpPotion.update()`
- L185: `i.remove()`
- L187: `this.relics.iterator()`
- L188: `i.hasNext()`
- L189: `i.next()`
- L190: `relic.update()`
- L192: `i.remove()`

### update() (L196-381)

**Calls (in order):**
- L198: `AbstractDungeon.settingsScreen.open()`
- L202: `AbstractDungeon.player.obtainPotion(new BlessingOfTheForge())`
- L203: `AbstractDungeon.scene.randomizeScene()`
- L205: `Gdx.input.isKeyJustPressed(49)`
- L206: `AbstractDungeon.player.increaseMaxOrbSlots(1, true)`
- L208: `DevInputActionSet.gainGold.isJustPressed()`
- L209: `AbstractDungeon.player.gainGold(100)`
- L214: `this.event.updateDialog()`
- L218: `this.monsters.update()`
- L220: `AbstractDungeon.actionManager.isEmpty()`
- L221: `AbstractDungeon.actionManager.update()`
- L223: `Gdx.graphics.getDeltaTime()`
- L228: `AbstractDungeon.topLevelEffects.add(new BattleStartEffect(false))`
- L230: `AbstractDungeon.actionManager.addToBottom(new GainEnergyAndEnableControlsAction(AbstractDungeon.player.energy.energyMast`
- L231: `AbstractDungeon.player.applyStartOfCombatPreDrawLogic()`
- L232: `AbstractDungeon.actionManager.addToBottom(new DrawCardAction(AbstractDungeon.player, AbstractDungeon.player.gameHandSize`
- L233: `AbstractDungeon.actionManager.addToBottom(new EnableEndTurnButtonAction())`
- L234: `AbstractDungeon.overlayMenu.showCombatPanels()`
- L235: `AbstractDungeon.player.applyStartOfCombatLogic()`
- L236: `ModHelper.isModEnabled("Careless")`
- L237: `Careless.modAction()`
- L239: `ModHelper.isModEnabled("ControlledChaos")`
- L240: `ControlledChaos.modAction()`
- L243: `AbstractDungeon.player.applyStartOfTurnRelics()`
- L244: `AbstractDungeon.player.applyStartOfTurnPostDrawRelics()`
- L245: `AbstractDungeon.player.applyStartOfTurnCards()`
- L246: `AbstractDungeon.player.applyStartOfTurnPowers()`
- L247: `AbstractDungeon.player.applyStartOfTurnOrbs()`
- L248: `AbstractDungeon.actionManager.useNextCombatActions()`
- L251: `DevInputActionSet.drawCard.isJustPressed()`
- L252: `AbstractDungeon.actionManager.addToTop(new DrawCardAction(AbstractDungeon.player, 1))`
- L255: `AbstractDungeon.actionManager.update()`
- L256: `this.monsters.areMonstersBasicallyDead()`
- L257: `AbstractDungeon.player.updateInput()`
- L260: `AbstractDungeon.screen.equals((Object)AbstractDungeon.CurrentScreen.HAND_SELECT)`
- L261: `AbstractDungeon.player.combatUpdate()`
- L264: `this.endTurn()`
- L267: `AbstractDungeon.actionManager.actions.isEmpty()`
- L269: `Gdx.graphics.getDeltaTime()`
- L272: `AbstractDungeon.getCurrRoom()`
- L273: `CardCrawlGame.sound.play("VICTORY")`
- L279: `this.addGoldToRewards(100)`
- L281: `AbstractDungeon.miscRng.random(-5, 5)`
- L283: `this.addGoldToRewards(MathUtils.round((float)tmp * 0.75f))`
- L283: `MathUtils.round((float)tmp * 0.75f)`
- L285: `this.addGoldToRewards(tmp)`
- L289: `ModHelper.isModEnabled("Cursed Run")`
- L290: `AbstractDungeon.effectList.add(new ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse(), (float)Settings.WIDTH / `
- L290: `AbstractDungeon.returnRandomCurse()`
- L294: `logger.info("ELITES SLAIN " + ++CardCrawlGame.elites1Slain)`
- ... (30 more)

**Objects created:**
- L202: `BlessingOfTheForge`
- L228: `BattleStartEffect`
- L230: `GainEnergyAndEnableControlsAction`
- L232: `DrawCardAction`
- L233: `EnableEndTurnButtonAction`
- L252: `DrawCardAction`
- L290: `ShowCardAndObtainEffect`
- L333: `SaveFile`
- L340: `GameSavedEffect`

### endTurn() (L383-413)

**Calls (in order):**
- L384: `AbstractDungeon.player.applyEndOfTurnTriggers()`
- L385: `AbstractDungeon.actionManager.addToBottom(new ClearCardQueueAction())`
- L386: `AbstractDungeon.actionManager.addToBottom(new DiscardAtEndOfTurnAction())`
- L388: `c.resetAttributes()`
- L391: `c.resetAttributes()`
- L394: `c.resetAttributes()`
- L397: `AbstractDungeon.player.hoveredCard.resetAttributes()`
- L399: `AbstractDungeon.actionManager.addToBottom(new AbstractGameAction(){

            @Override
            public void updat`
- L403: `this.addToBot(new EndTurnAction())`
- L404: `this.addToBot(new WaitAction(1.2f))`
- L406: `this.addToBot(new MonsterStartTurnAction())`

**Objects created:**
- L385: `ClearCardQueueAction`
- L386: `DiscardAtEndOfTurnAction`
- L399: `AbstractGameAction`
- L403: `EndTurnAction`
- L404: `WaitAction`
- L406: `MonsterStartTurnAction`

### endBattle() (L415-454)

**Calls (in order):**
- L418: `UnlockTracker.unlockAchievement("SHRUG_IT_OFF")`
- L420: `AbstractDungeon.player.hasRelic("Meat on the Bone")`
- L421: `AbstractDungeon.player.getRelic("Meat on the Bone").onTrigger()`
- L421: `AbstractDungeon.player.getRelic("Meat on the Bone")`
- L423: `AbstractDungeon.player.onVictory()`
- L436: `UnlockTracker.unlockAchievement("COME_AT_ME")`
- L444: `CardCrawlGame.metricData.addEncounterData()`
- L445: `AbstractDungeon.actionManager.clear()`
- L447: `AbstractDungeon.player.releaseCard()`
- L448: `AbstractDungeon.player.hand.refreshHandLayout()`
- L449: `AbstractDungeon.player.resetControllerValues()`
- L450: `AbstractDungeon.overlayMenu.hideCombatPanels()`
- L451: `AbstractDungeon.player.stance.ID.equals("Neutral")`
- L452: `AbstractDungeon.player.stance.stopIdleSfx()`

### render(SpriteBatch sb) (L459-486)

**Calls (in order):**
- L462: `this.event.renderRoomEventPanel(sb)`
- L464: `AbstractDungeon.player.render(sb)`
- L468: `AbstractDungeon.player.render(sb)`
- L470: `AbstractDungeon.getCurrRoom()`
- L472: `this.monsters.render(sb)`
- L475: `AbstractDungeon.player.renderPlayerBattleUi(sb)`
- L479: `i.render(sb)`
- L483: `r.render(sb)`
- L485: `this.renderTips(sb)`

### renderAboveTopPanel(SpriteBatch sb) (L488-498)

**Calls (in order):**
- L491: `i.render(sb)`
- L493: `this.souls.render(sb)`
- L495: `AbstractDungeon.player.masterDeck.size()`
- L495: `AbstractDungeon.player.drawPile.size()`
- L495: `AbstractDungeon.player.discardPile.size()`
- L495: `AbstractDungeon.player.exhaustPile.size()`
- L495: `AbstractDungeon.actionManager.phase.name()`
- L495: `CardCrawlGame.publisherIntegration.isInitialized()`
- L495: `AbstractDungeon.screen.name()`
- L495: `AbstractDungeon.effectList.size()`
- L496: `FontHelper.renderFontCenteredHeight(sb, FontHelper.tipBodyFont, msg, 30.0f, (float)Settings.HEIGHT * 0.5f, Color.WHITE)`

### spawnRelicAndObtain(float x, float y, AbstractRelic relic) (L503-517)

**Calls (in order):**
- L504: `AbstractDungeon.player.hasRelic("Circlet")`
- L505: `AbstractDungeon.player.getRelic("Circlet")`
- L507: `circ.flash()`
- L509: `relic.spawn(x, y)`
- L510: `this.relics.add(relic)`
- L511: `relic.obtain()`
- L515: `relic.flash()`

### spawnBlightAndObtain(float x, float y, AbstractBlight blight) (L519-526)

**Calls (in order):**
- L520: `blight.spawn(x, y)`
- L521: `blight.obtain()`
- L525: `blight.flash()`

### applyEndOfTurnRelics() (L528-535)

**Calls (in order):**
- L530: `r.onPlayerEndTurn()`
- L533: `b.onPlayerEndTurn()`

### applyEndOfTurnPreCardPowers() (L537-541)

**Calls (in order):**
- L539: `p.atEndOfTurnPreEndTurnCards(true)`

### addRelicToRewards(AbstractRelic.RelicTier tier) (L543-545)

**Calls (in order):**
- L544: `this.rewards.add(new RewardItem(AbstractDungeon.returnRandomRelic(tier)))`
- L544: `AbstractDungeon.returnRandomRelic(tier)`

**Objects created:**
- L544: `RewardItem`

### addSapphireKey(RewardItem item) (L547-549)

**Calls (in order):**
- L548: `this.rewards.add(new RewardItem(item, RewardItem.RewardType.SAPPHIRE_KEY))`

**Objects created:**
- L548: `RewardItem`

### removeOneRelicFromRewards() (L551-561)

**Calls (in order):**
- L552: `this.rewards.iterator()`
- L553: `i.hasNext()`
- L554: `i.next()`
- L556: `i.remove()`
- L557: `i.hasNext()`
- L557: `i.next()`
- L558: `i.remove()`

### addNoncampRelicToRewards(AbstractRelic.RelicTier tier) (L563-565)

**Calls (in order):**
- L564: `this.rewards.add(new RewardItem(AbstractDungeon.returnRandomNonCampfireRelic(tier)))`
- L564: `AbstractDungeon.returnRandomNonCampfireRelic(tier)`

**Objects created:**
- L564: `RewardItem`

### addRelicToRewards(AbstractRelic relic) (L567-569)

**Calls (in order):**
- L568: `this.rewards.add(new RewardItem(relic))`

**Objects created:**
- L568: `RewardItem`

### addPotionToRewards(AbstractPotion potion) (L571-573)

**Calls (in order):**
- L572: `this.rewards.add(new RewardItem(potion))`

**Objects created:**
- L572: `RewardItem`

### addCardToRewards() (L575-580)

**Calls (in order):**
- L577: `cardReward.cards.size()`
- L578: `this.rewards.add(cardReward)`

**Objects created:**
- L576: `RewardItem`

### addPotionToRewards() (L582-610)

**Calls (in order):**
- L588: `AbstractDungeon.getMonsters().haveMonstersEscaped()`
- L588: `AbstractDungeon.getMonsters()`
- L596: `AbstractDungeon.player.hasRelic("White Beast Statue")`
- L599: `this.rewards.size()`
- L602: `logger.info("POTION CHANCE: " + chance)`
- L603: `AbstractDungeon.potionRng.random(0, 99)`
- L604: `CardCrawlGame.metricData.potions_floor_spawned.add(AbstractDungeon.floorNum)`
- L605: `this.rewards.add(new RewardItem(AbstractDungeon.returnRandomPotion()))`
- L605: `AbstractDungeon.returnRandomPotion()`

**Objects created:**
- L605: `RewardItem`

### addGoldToRewards(int gold) (L612-619)

**Calls (in order):**
- L615: `i.incrementGold(gold)`
- L618: `this.rewards.add(new RewardItem(gold))`

**Objects created:**
- L618: `RewardItem`

### addStolenGoldToRewards(int gold) (L621-628)

**Calls (in order):**
- L624: `i.incrementGold(gold)`
- L627: `this.rewards.add(new RewardItem(gold, true))`

**Objects created:**
- L627: `RewardItem`

### isBattleEnding() (L630-638)

**Calls (in order):**
- L635: `this.monsters.areMonstersBasicallyDead()`

### renderEventTexts(SpriteBatch sb) (L640-644)

**Calls (in order):**
- L642: `this.event.renderText(sb)`

### clearEvent() (L646-651)

**Calls (in order):**
- L648: `this.event.imageEventText.clear()`
- L649: `this.event.roomEventText.clear()`

### eventControllerInput() (L653-706)

**Calls (in order):**
- L657: `AbstractDungeon.getCurrRoom()`
- L657: `AbstractDungeon.getCurrRoom()`
- L658: `RoomEventDialog.optionList.isEmpty()`
- L669: `Gdx.input.setCursorPosition((int)RoomEventDialog.optionList.get((int)0).hb.cX, Settings.HEIGHT - (int)RoomEventDialog.op`
- L669: `RoomEventDialog.optionList.get((int)0)`
- L669: `RoomEventDialog.optionList.get((int)0)`
- L670: `CInputActionSet.down.isJustPressed()`
- L670: `CInputActionSet.altDown.isJustPressed()`
- L671: `RoomEventDialog.optionList.size()`
- L674: `Gdx.input.setCursorPosition((int)RoomEventDialog.optionList.get((int)index).hb.cX, Settings.HEIGHT - (int)RoomEventDialo`
- L674: `RoomEventDialog.optionList.get((int)index)`
- L674: `RoomEventDialog.optionList.get((int)index)`
- L675: `CInputActionSet.up.isJustPressed()`
- L675: `CInputActionSet.altUp.isJustPressed()`
- L677: `RoomEventDialog.optionList.size()`
- L679: `Gdx.input.setCursorPosition((int)RoomEventDialog.optionList.get((int)index).hb.cX, Settings.HEIGHT - (int)RoomEventDialo`
- L679: `RoomEventDialog.optionList.get((int)index)`
- L679: `RoomEventDialog.optionList.get((int)index)`
- L681: `this.event.imageEventText.optionList.isEmpty()`
- L692: `Gdx.input.setCursorPosition((int)this.event.imageEventText.optionList.get((int)0).hb.cX, Settings.HEIGHT - (int)this.eve`
- L692: `this.event.imageEventText.optionList.get((int)0)`
- L692: `this.event.imageEventText.optionList.get((int)0)`
- L693: `CInputActionSet.down.isJustPressed()`
- L693: `CInputActionSet.altDown.isJustPressed()`
- L694: `this.event.imageEventText.optionList.size()`
- L697: `Gdx.input.setCursorPosition((int)this.event.imageEventText.optionList.get((int)index).hb.cX, Settings.HEIGHT - (int)this`
- L697: `this.event.imageEventText.optionList.get((int)index)`
- L697: `this.event.imageEventText.optionList.get((int)index)`
- L698: `CInputActionSet.up.isJustPressed()`
- L698: `CInputActionSet.altUp.isJustPressed()`
- L700: `this.event.imageEventText.optionList.size()`
- L702: `Gdx.input.setCursorPosition((int)this.event.imageEventText.optionList.get((int)index).hb.cX, Settings.HEIGHT - (int)this`
- L702: `this.event.imageEventText.optionList.get((int)index)`
- L702: `this.event.imageEventText.optionList.get((int)index)`

### addCardReward(RewardItem rewardItem) (L708-712)

**Calls (in order):**
- L709: `rewardItem.cards.isEmpty()`
- L710: `this.rewards.add(rewardItem)`

### dispose() (L714-724)

**Calls (in order):**
- L717: `this.event.dispose()`
- L721: `m.dispose()`

## CardGroup
File: `cards\CardGroup.java`

### getCardDeck() (L62-68)

**Calls (in order):**
- L65: `retVal.add(new CardSave(card.cardID, card.timesUpgraded, card.misc))`

**Objects created:**
- L63: `None`
- L65: `CardSave`

### getCardNames() (L70-76)

**Calls (in order):**
- L73: `retVal.add(card.cardID)`

**Objects created:**
- L71: `None`

### getCardIdsForMetrics() (L78-84)

**Calls (in order):**
- L81: `retVal.add(card.getMetricID())`
- L81: `card.getMetricID()`

**Objects created:**
- L79: `None`

### clear() (L86-88)

**Calls (in order):**
- L87: `this.group.clear()`

### contains(AbstractCard c) (L90-92)

**Calls (in order):**
- L91: `this.group.contains(c)`

### canUseAnyCard() (L94-100)

**Calls (in order):**
- L96: `c.hasEnoughEnergy()`

### fullSetCheck() (L102-122)

**Calls (in order):**
- L107: `cardIDS.add(c.cardID)`
- L111: `cardCount.containsKey(string)`
- L112: `cardCount.put(string, (Integer)cardCount.get(string) + 1)`
- L112: `cardCount.get(string)`
- L115: `cardCount.put(string, 1)`
- L117: `cardCount.entrySet()`
- L118: `entry.getValue()`

**Objects created:**
- L104: `None`
- L109: `None`

### highlanderCheck() (L141-149)

**Calls (in order):**
- L145: `cardIDS.add(c.cardID)`
- L148: `set.size()`
- L148: `cardIDS.size()`

**Objects created:**
- L142: `None`
- L147: `HashSet`

### applyPowers() (L151-155)

**Calls (in order):**
- L153: `c.applyPowers()`

### removeCard(AbstractCard c) (L157-165)

**Calls (in order):**
- L158: `this.group.remove(c)`
- L160: `c.onRemoveFromMasterDeck()`
- L162: `r.onMasterDeckChange()`

### removeCard(String targetID) (L167-176)

**Calls (in order):**
- L168: `this.group.iterator()`
- L169: `i.hasNext()`
- L170: `i.next()`
- L171: `e.cardID.equals(targetID)`
- L172: `i.remove()`

### addToHand(AbstractCard c) (L178-181)

**Calls (in order):**
- L179: `c.untip()`
- L180: `this.group.add(c)`

### refreshHandLayout() (L183-387)

**Calls (in order):**
- L184: `AbstractDungeon.getCurrRoom()`
- L184: `AbstractDungeon.getCurrRoom().monsters.areMonstersBasicallyDead()`
- L184: `AbstractDungeon.getCurrRoom()`
- L187: `AbstractDungeon.player.hasPower("Surrounded")`
- L187: `AbstractDungeon.getCurrRoom()`
- L188: `AbstractDungeon.getCurrRoom()`
- L191: `m.applyPowers()`
- L194: `m.applyPowers()`
- L195: `m.removeSurroundedPower()`
- L200: `m.applyPowers()`
- L203: `m.applyPowers()`
- L204: `m.removeSurroundedPower()`
- L208: `o.hideEvokeValues()`
- L210: `AbstractDungeon.player.hand.size()`
- L210: `AbstractDungeon.player.drawPile.size()`
- L210: `AbstractDungeon.player.discardPile.size()`
- L210: `AbstractDungeon.getCurrRoom()`
- L210: `AbstractDungeon.getCurrRoom()`
- L210: `AbstractDungeon.getCurrRoom().monsters.areMonstersBasicallyDead()`
- L210: `AbstractDungeon.getCurrRoom()`
- L211: `UnlockTracker.unlockAchievement("PURITY")`
- L214: `r.onRefreshHand()`
- L216: `this.group.size()`
- L217: `this.group.size()`
- L220: `this.group.size()`
- L221: `this.group.size()`
- L222: `this.group.size()`
- L223: `this.group.get(i).setAngle(angleRange / 2.0f - incrementAngle * (float)i - incrementAngle / 2.0f)`
- L223: `this.group.get(i)`
- L226: `this.group.size()`
- L233: `this.group.size()`
- L237: `this.group.get((int)i)`
- L242: `this.group.size()`
- L247: `this.group.get((int)0)`
- L251: `this.group.get((int)0)`
- L252: `this.group.get((int)1)`
- L256: `this.group.get((int)0)`
- L257: `this.group.get((int)1)`
- L258: `this.group.get((int)2)`
- L259: `this.group.get((int)0)`
- L260: `this.group.get((int)2)`
- L264: `this.group.get((int)0)`
- L265: `this.group.get((int)1)`
- L266: `this.group.get((int)2)`
- L267: `this.group.get((int)3)`
- L268: `this.group.get((int)1)`
- L269: `this.group.get((int)2)`
- L273: `this.group.get((int)0)`
- L274: `this.group.get((int)1)`
- L275: `this.group.get((int)2)`
- ... (72 more)

### glowCheck() (L389-398)

**Calls (in order):**
- L391: `c.canUse(AbstractDungeon.player, null)`
- L392: `c.beginGlowing()`
- L394: `c.stopGlowing()`
- L396: `c.triggerOnGlowCheck()`

### stopGlowing() (L400-404)

**Calls (in order):**
- L402: `c.stopGlowing()`

### hoverCardPush(AbstractCard c) (L406-436)

**Calls (in order):**
- L407: `this.group.size()`
- L410: `this.group.size()`
- L411: `c.equals(this.group.get(i))`
- L411: `this.group.get(i)`
- L416: `this.group.size()`
- L418: `this.group.size()`
- L418: `this.group.size()`
- L421: `this.group.size()`
- L422: `this.group.get((int)currentSlot)`
- L426: `this.group.size()`
- L428: `this.group.size()`
- L428: `this.group.size()`
- L431: `this.group.size()`
- L432: `this.group.get((int)currentSlot)`

### addToTop(AbstractCard c) (L438-440)

**Calls (in order):**
- L439: `this.group.add(c)`

### addToBottom(AbstractCard c) (L442-444)

**Calls (in order):**
- L443: `this.group.add(0, c)`

### addToRandomSpot(AbstractCard c) (L446-452)

**Calls (in order):**
- L447: `this.group.size()`
- L448: `this.group.add(c)`
- L450: `this.group.add(AbstractDungeon.cardRandomRng.random(this.group.size() - 1), c)`
- L450: `AbstractDungeon.cardRandomRng.random(this.group.size() - 1)`
- L450: `this.group.size()`

### getTopCard() (L454-456)

**Calls (in order):**
- L455: `this.group.get(this.group.size() - 1)`
- L455: `this.group.size()`

### getNCardFromTop(int num) (L458-460)

**Calls (in order):**
- L459: `this.group.get(this.group.size() - 1 - num)`
- L459: `this.group.size()`

### getBottomCard() (L462-464)

**Calls (in order):**
- L463: `this.group.get(0)`

### getHoveredCard() (L466-479)

**Calls (in order):**
- L468: `c.isHoveredInHand(0.7f)`

### getRandomCard(Random rng) (L481-483)

**Calls (in order):**
- L482: `this.group.get(rng.random(this.group.size() - 1))`
- L482: `rng.random(this.group.size() - 1)`
- L482: `this.group.size()`

### getRandomCard(boolean useRng) (L485-490)

**Calls (in order):**
- L487: `this.group.get(AbstractDungeon.cardRng.random(this.group.size() - 1))`
- L487: `AbstractDungeon.cardRng.random(this.group.size() - 1)`
- L487: `this.group.size()`
- L489: `this.group.get(MathUtils.random(this.group.size() - 1))`
- L489: `MathUtils.random(this.group.size() - 1)`
- L489: `this.group.size()`

### getRandomCard(boolean useRng, AbstractCard.CardRarity rarity) (L492-507)

**Calls (in order):**
- L496: `tmp.add(c)`
- L498: `tmp.isEmpty()`
- L499: `logger.info("ERROR: No cards left for type: " + this.type.name())`
- L499: `this.type.name()`
- L502: `Collections.sort(tmp)`
- L504: `tmp.get(AbstractDungeon.cardRng.random(tmp.size() - 1))`
- L504: `AbstractDungeon.cardRng.random(tmp.size() - 1)`
- L504: `tmp.size()`
- L506: `tmp.get(MathUtils.random(tmp.size() - 1))`
- L506: `MathUtils.random(tmp.size() - 1)`
- L506: `tmp.size()`

**Objects created:**
- L493: `None`

### getRandomCard(Random rng, AbstractCard.CardRarity rarity) (L509-521)

**Calls (in order):**
- L513: `tmp.add(c)`
- L515: `tmp.isEmpty()`
- L516: `logger.info("ERROR: No cards left for type: " + this.type.name())`
- L516: `this.type.name()`
- L519: `Collections.sort(tmp)`
- L520: `tmp.get(rng.random(tmp.size() - 1))`
- L520: `rng.random(tmp.size() - 1)`
- L520: `tmp.size()`

**Objects created:**
- L510: `None`

### getRandomCard(AbstractCard.CardType type, boolean useRng) (L523-538)

**Calls (in order):**
- L527: `tmp.add(c)`
- L529: `tmp.isEmpty()`
- L530: `logger.info("ERROR: No cards left for type: " + type.name())`
- L530: `type.name()`
- L533: `Collections.sort(tmp)`
- L535: `tmp.get(AbstractDungeon.cardRng.random(tmp.size() - 1))`
- L535: `AbstractDungeon.cardRng.random(tmp.size() - 1)`
- L535: `tmp.size()`
- L537: `tmp.get(MathUtils.random(tmp.size() - 1))`
- L537: `MathUtils.random(tmp.size() - 1)`
- L537: `tmp.size()`

**Objects created:**
- L524: `None`

### removeTopCard() (L540-542)

**Calls (in order):**
- L541: `this.group.remove(this.group.size() - 1)`
- L541: `this.group.size()`

### shuffle() (L544-546)

**Calls (in order):**
- L545: `Collections.shuffle(this.group, new java.util.Random(AbstractDungeon.shuffleRng.randomLong()))`
- L545: `AbstractDungeon.shuffleRng.randomLong()`

**Objects created:**
- L545: `None`

### shuffle(Random rng) (L548-550)

**Calls (in order):**
- L549: `Collections.shuffle(this.group, new java.util.Random(rng.randomLong()))`
- L549: `rng.randomLong()`

**Objects created:**
- L549: `None`

### toString() (L552-559)

**Calls (in order):**
- L555: `sb.append(c.cardID)`
- L556: `sb.append("\n")`
- L558: `sb.toString()`

**Objects created:**
- L553: `StringBuilder`

### update() (L561-565)

**Calls (in order):**
- L563: `c.update()`

### updateHoverLogic() (L567-571)

**Calls (in order):**
- L569: `c.updateHoverLogic()`

### render(SpriteBatch sb) (L573-577)

**Calls (in order):**
- L575: `c.render(sb)`

### renderShowBottled(SpriteBatch sb) (L579-623)

**Calls (in order):**
- L584: `c.render(sb)`
- L586: `RelicLibrary.getRelic("Bottled Flame")`
- L592: `tmp.render(sb)`
- L599: `RelicLibrary.getRelic("Bottled Lightning")`
- L605: `tmp.render(sb)`
- L612: `RelicLibrary.getRelic("Bottled Tornado")`
- L618: `tmp.render(sb)`

### renderMasterDeck(SpriteBatch sb) (L625-669)

**Calls (in order):**
- L630: `c.render(sb)`
- L632: `RelicLibrary.getRelic("Bottled Flame")`
- L638: `tmp.render(sb)`
- L645: `RelicLibrary.getRelic("Bottled Lightning")`
- L651: `tmp.render(sb)`
- L658: `RelicLibrary.getRelic("Bottled Tornado")`
- L664: `tmp.render(sb)`

### renderExceptOneCard(SpriteBatch sb, AbstractCard card) (L671-676)

**Calls (in order):**
- L673: `c.equals(card)`
- L674: `c.render(sb)`

### renderExceptOneCardShowBottled(SpriteBatch sb, AbstractCard card) (L678-723)

**Calls (in order):**
- L683: `c.equals(card)`
- L684: `c.render(sb)`
- L686: `RelicLibrary.getRelic("Bottled Flame")`
- L692: `tmp.render(sb)`
- L699: `RelicLibrary.getRelic("Bottled Lightning")`
- L705: `tmp.render(sb)`
- L712: `RelicLibrary.getRelic("Bottled Tornado")`
- L718: `tmp.render(sb)`

### renderMasterDeckExceptOneCard(SpriteBatch sb, AbstractCard card) (L725-770)

**Calls (in order):**
- L730: `c.equals(card)`
- L731: `c.render(sb)`
- L733: `RelicLibrary.getRelic("Bottled Flame")`
- L739: `tmp.render(sb)`
- L746: `RelicLibrary.getRelic("Bottled Lightning")`
- L752: `tmp.render(sb)`
- L759: `RelicLibrary.getRelic("Bottled Tornado")`
- L765: `tmp.render(sb)`

### renderHand(SpriteBatch sb, AbstractCard exceptThis) (L772-793)

**Calls (in order):**
- L777: `i.card.equals(c)`
- L778: `this.queued.add(c)`
- L783: `this.inHand.add(c)`
- L786: `c.render(sb)`
- L789: `c.render(sb)`
- L791: `this.inHand.clear()`
- L792: `this.queued.clear()`

### renderInLibrary(SpriteBatch sb) (L795-799)

**Calls (in order):**
- L797: `c.renderInLibrary(sb)`

### renderTip(SpriteBatch sb) (L801-805)

**Calls (in order):**
- L803: `c.renderCardTip(sb)`

### renderWithSelections(SpriteBatch sb) (L807-811)

**Calls (in order):**
- L809: `c.renderWithSelections(sb)`

### renderDiscardPile(SpriteBatch sb) (L813-817)

**Calls (in order):**
- L815: `c.render(sb)`

### moveToDiscardPile(AbstractCard c) (L819-825)

**Calls (in order):**
- L820: `this.resetCardBeforeMoving(c)`
- L821: `c.shrink()`
- L822: `c.darken(false)`
- L823: `AbstractDungeon.getCurrRoom().souls.discard(c)`
- L823: `AbstractDungeon.getCurrRoom()`
- L824: `AbstractDungeon.player.onCardDrawOrDiscard()`

### empower(AbstractCard c) (L827-831)

**Calls (in order):**
- L828: `this.resetCardBeforeMoving(c)`
- L829: `c.shrink()`
- L830: `AbstractDungeon.getCurrRoom().souls.empower(c)`
- L830: `AbstractDungeon.getCurrRoom()`

### moveToExhaustPile(AbstractCard c) (L833-845)

**Calls (in order):**
- L835: `r.onExhaust(c)`
- L838: `p.onExhaust(c)`
- L840: `c.triggerOnExhaust()`
- L841: `this.resetCardBeforeMoving(c)`
- L842: `AbstractDungeon.effectList.add(new ExhaustCardEffect(c))`
- L843: `AbstractDungeon.player.exhaustPile.addToTop(c)`
- L844: `AbstractDungeon.player.onCardDrawOrDiscard()`

**Objects created:**
- L842: `ExhaustCardEffect`

### moveToHand(AbstractCard c, CardGroup group) (L847-859)

**Calls (in order):**
- L848: `c.unhover()`
- L849: `c.lighten(true)`
- L850: `c.setAngle(0.0f)`
- L855: `group.removeCard(c)`
- L856: `AbstractDungeon.player.hand.addToTop(c)`
- L857: `AbstractDungeon.player.hand.refreshHandLayout()`
- L858: `AbstractDungeon.player.hand.applyPowers()`

### moveToHand(AbstractCard c) (L861-873)

**Calls (in order):**
- L862: `this.resetCardBeforeMoving(c)`
- L863: `c.unhover()`
- L864: `c.lighten(true)`
- L865: `c.setAngle(0.0f)`
- L870: `AbstractDungeon.player.hand.addToTop(c)`
- L871: `AbstractDungeon.player.hand.refreshHandLayout()`
- L872: `AbstractDungeon.player.hand.applyPowers()`

### moveToDeck(AbstractCard c, boolean randomSpot) (L875-879)

**Calls (in order):**
- L876: `this.resetCardBeforeMoving(c)`
- L877: `c.shrink()`
- L878: `AbstractDungeon.getCurrRoom().souls.onToDeck(c, randomSpot)`
- L878: `AbstractDungeon.getCurrRoom()`

### moveToBottomOfDeck(AbstractCard c) (L881-885)

**Calls (in order):**
- L882: `this.resetCardBeforeMoving(c)`
- L883: `c.shrink()`
- L884: `AbstractDungeon.getCurrRoom().souls.onToBottomOfDeck(c)`
- L884: `AbstractDungeon.getCurrRoom()`

### resetCardBeforeMoving(AbstractCard c) (L887-896)

**Calls (in order):**
- L889: `AbstractDungeon.player.releaseCard()`
- L891: `AbstractDungeon.actionManager.removeFromQueue(c)`
- L892: `c.unhover()`
- L893: `c.untip()`
- L894: `c.stopGlowing()`
- L895: `this.group.remove(c)`

### isEmpty() (L898-900)

**Calls (in order):**
- L899: `this.group.isEmpty()`

### discardAll(CardGroup discardPile) (L902-909)

**Calls (in order):**
- L906: `discardPile.addToTop(c)`
- L908: `this.group.clear()`

### initializeDeck(CardGroup masterDeck) (L911-938)

**Calls (in order):**
- L912: `this.clear()`
- L914: `copy.shuffle(AbstractDungeon.shuffleRng)`
- L918: `placeOnTop.add(c)`
- L922: `placeOnTop.add(c)`
- L929: `this.addToTop(c)`
- L932: `this.addToTop(c)`
- L934: `placeOnTop.size()`
- L935: `AbstractDungeon.actionManager.addToTurnStart(new DrawCardAction(AbstractDungeon.player, placeOnTop.size() - AbstractDung`
- L935: `placeOnTop.size()`
- L937: `placeOnTop.clear()`

**Objects created:**
- L913: `CardGroup`
- L915: `None`
- L935: `DrawCardAction`

### size() (L940-942)

**Calls (in order):**
- L941: `this.group.size()`

### getUpgradableCards() (L944-951)

**Calls (in order):**
- L947: `c.canUpgrade()`
- L948: `retVal.group.add(c)`

**Objects created:**
- L945: `CardGroup`

### hasUpgradableCards() (L953-959)

**Calls (in order):**
- L955: `c.canUpgrade()`

### getPurgeableCards() (L961-968)

**Calls (in order):**
- L964: `c.cardID.equals("Necronomicurse")`
- L964: `c.cardID.equals("CurseOfTheBell")`
- L964: `c.cardID.equals("AscendersBane")`
- L965: `retVal.group.add(c)`

**Objects created:**
- L962: `CardGroup`

### getSpecificCard(AbstractCard targetCard) (L970-975)

**Calls (in order):**
- L971: `this.group.contains(targetCard)`

### triggerOnOtherCardPlayed(AbstractCard usedCard) (L977-985)

**Calls (in order):**
- L980: `c.triggerOnOtherCardPlayed(usedCard)`
- L983: `p.onAfterCardPlayed(usedCard)`

### sortWithComparator(Comparator<AbstractCard> comp, boolean ascending) (L987-993)

**Calls (in order):**
- L989: `this.group.sort(comp)`
- L991: `this.group.sort(Collections.reverseOrder(comp))`
- L991: `Collections.reverseOrder(comp)`

### sortByRarity(boolean ascending) (L995-997)

**Calls (in order):**
- L996: `this.sortWithComparator(new CardRarityComparator(), ascending)`

**Objects created:**
- L996: `CardRarityComparator`

### sortByRarityPlusStatusCardType(boolean ascending) (L999-1002)

**Calls (in order):**
- L1000: `this.sortWithComparator(new CardRarityComparator(), ascending)`
- L1001: `this.sortWithComparator(new StatusCardsLastComparator(), true)`

**Objects created:**
- L1000: `CardRarityComparator`
- L1001: `StatusCardsLastComparator`

### sortByType(boolean ascending) (L1004-1006)

**Calls (in order):**
- L1005: `this.sortWithComparator(new CardTypeComparator(), ascending)`

**Objects created:**
- L1005: `CardTypeComparator`

### sortByStatus(boolean ascending) (L1011-1013)

**Calls (in order):**
- L1012: `this.sortWithComparator(new CardLockednessComparator(), ascending)`

**Objects created:**
- L1012: `CardLockednessComparator`

### sortAlphabetically(boolean ascending) (L1015-1017)

**Calls (in order):**
- L1016: `this.sortWithComparator(new CardNameComparator(), ascending)`

**Objects created:**
- L1016: `CardNameComparator`

### sortByCost(boolean ascending) (L1019-1021)

**Calls (in order):**
- L1020: `this.sortWithComparator(new CardCostComparator(), ascending)`

**Objects created:**
- L1020: `CardCostComparator`

### getSkills() (L1023-1025)

**Calls (in order):**
- L1024: `this.getCardsOfType(AbstractCard.CardType.SKILL)`

### getAttacks() (L1027-1029)

**Calls (in order):**
- L1028: `this.getCardsOfType(AbstractCard.CardType.ATTACK)`

### getPowers() (L1031-1033)

**Calls (in order):**
- L1032: `this.getCardsOfType(AbstractCard.CardType.POWER)`

### getCardsOfType(AbstractCard.CardType cardType) (L1035-1042)

**Calls (in order):**
- L1039: `retVal.addToBottom(card)`

**Objects created:**
- L1036: `CardGroup`

### getGroupedByColor() (L1044-1057)

**Calls (in order):**
- L1046: `AbstractCard.CardColor.values()`
- L1047: `colorGroups.add(new CardGroup(CardGroupType.UNSPECIFIED))`
- L1050: `((CardGroup)colorGroups.get(card.color.ordinal())).addToTop(card)`
- L1050: `colorGroups.get(card.color.ordinal())`
- L1050: `card.color.ordinal()`
- L1054: `retVal.group.addAll(group.group)`

**Objects created:**
- L1045: `None`
- L1047: `CardGroup`
- L1052: `CardGroup`

### findCardById(String id) (L1059-1065)

**Calls (in order):**
- L1061: `c.cardID.equals(id)`

### getGroupWithoutBottledCards(CardGroup group) (L1067-1074)

**Calls (in order):**
- L1071: `retVal.addToTop(c)`

**Objects created:**
- L1068: `CardGroup`

## GameActionManager
File: `actions\GameActionManager.java`

### addToNextCombat(AbstractGameAction action) (L74-76)

**Calls (in order):**
- L75: `this.nextCombatActions.add(action)`

### useNextCombatActions() (L78-83)

**Calls (in order):**
- L80: `this.addToBottom(a)`
- L82: `this.nextCombatActions.clear()`

### addToBottom(AbstractGameAction action) (L85-89)

**Calls (in order):**
- L86: `AbstractDungeon.getCurrRoom()`
- L87: `this.actions.add(action)`

### addCardQueueItem(CardQueueItem c, boolean inFrontOfQueue) (L91-101)

**Calls (in order):**
- L93: `AbstractDungeon.actionManager.cardQueue.isEmpty()`
- L94: `AbstractDungeon.actionManager.cardQueue.add(1, c)`
- L96: `AbstractDungeon.actionManager.cardQueue.add(c)`
- L99: `AbstractDungeon.actionManager.cardQueue.add(c)`

### addCardQueueItem(CardQueueItem c) (L103-105)

**Calls (in order):**
- L104: `this.addCardQueueItem(c, false)`

### removeFromQueue(AbstractCard c) (L107-117)

**Calls (in order):**
- L109: `this.cardQueue.size()`
- L110: `this.cardQueue.get((int)i)`
- L110: `this.cardQueue.get((int)i).card.equals(c)`
- L110: `this.cardQueue.get((int)i)`
- L115: `this.cardQueue.remove(index)`

### clearPostCombatActions() (L119-126)

**Calls (in order):**
- L120: `this.actions.iterator()`
- L121: `i.hasNext()`
- L122: `i.next()`
- L124: `i.remove()`

### addToTop(AbstractGameAction action) (L128-132)

**Calls (in order):**
- L129: `AbstractDungeon.getCurrRoom()`
- L130: `this.actions.add(0, action)`

### addToTurnStart(AbstractGameAction action) (L134-138)

**Calls (in order):**
- L135: `AbstractDungeon.getCurrRoom()`
- L136: `this.preTurnActions.add(0, action)`

### update() (L140-166)

**Calls (in order):**
- L143: `this.getNextAction()`
- L148: `this.currentAction.update()`
- L153: `this.getNextAction()`
- L154: `AbstractDungeon.getCurrRoom()`
- L156: `AbstractDungeon.player.hand.refreshHandLayout()`
- L163: `logger.info("This should never be called")`

### endTurn() (L168-172)

**Calls (in order):**
- L169: `AbstractDungeon.player.resetControllerValues()`

### getNextAction() (L174-356)

**Calls (in order):**
- L175: `this.actions.isEmpty()`
- L176: `this.actions.remove(0)`
- L179: `this.preTurnActions.isEmpty()`
- L180: `this.preTurnActions.remove(0)`
- L183: `this.cardQueue.isEmpty()`
- L186: `this.cardQueue.get((int)0)`
- L188: `this.callEndOfTurnActions()`
- L189: `c.equals(this.lastCard)`
- L190: `logger.info("Last card! " + c.name)`
- L193: `this.cardQueue.size()`
- L193: `this.cardQueue.get((int)0)`
- L193: `AbstractDungeon.player.getRelic("Unceasing Top")`
- L194: `((UnceasingTop)top).disableUntilTurnEnds()`
- L198: `this.cardQueue.get((int)0)`
- L200: `this.cardQueue.get((int)0)`
- L201: `this.cardQueue.get((int)0)`
- L201: `AbstractDungeon.getMonsters().getRandomMonster(null, true, AbstractDungeon.cardRandomRng)`
- L201: `AbstractDungeon.getMonsters()`
- L203: `this.cardQueue.get((int)0)`
- L203: `c.canUse(AbstractDungeon.player, this.cardQueue.get((int)0).monster)`
- L203: `this.cardQueue.get((int)0)`
- L203: `this.cardQueue.get((int)0)`
- L205: `c.freeToPlay()`
- L208: `this.cardQueue.get((int)0)`
- L208: `this.cardQueue.get((int)0)`
- L209: `this.cardQueue.get((int)0)`
- L209: `this.cardQueue.get((int)0)`
- L210: `this.cardQueue.get((int)0)`
- L212: `abstractPower.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L212: `this.cardQueue.get((int)0)`
- L212: `this.cardQueue.get((int)0)`
- L214: `AbstractDungeon.getMonsters()`
- L216: `p.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L216: `this.cardQueue.get((int)0)`
- L216: `this.cardQueue.get((int)0)`
- L220: `abstractRelic.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L220: `this.cardQueue.get((int)0)`
- L220: `this.cardQueue.get((int)0)`
- L222: `AbstractDungeon.player.stance.onPlayCard(this.cardQueue.get((int)0).card)`
- L222: `this.cardQueue.get((int)0)`
- L224: `abstractBlight.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L224: `this.cardQueue.get((int)0)`
- L224: `this.cardQueue.get((int)0)`
- L227: `abstractCard.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L227: `this.cardQueue.get((int)0)`
- L227: `this.cardQueue.get((int)0)`
- L230: `abstractCard.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- L230: `this.cardQueue.get((int)0)`
- L230: `this.cardQueue.get((int)0)`
- L233: `abstractCard.onPlayCard(this.cardQueue.get((int)0).card, this.cardQueue.get((int)0).monster)`
- ... (91 more)

**Objects created:**
- L260: `ExhaustCardEffect`
- L268: `ExhaustCardEffect`
- L280: `ExhaustCardEffect`
- L284: `ThoughtBubble`
- L290: `UseCardAction`
- L301: `ShowMoveNameAction`
- L302: `IntentFlashAction`
- L316: `WaitAction`
- L350: `DrawCardAction`
- L353: `EnableEndTurnButtonAction`

### callEndOfTurnActions() (L358-366)

**Calls (in order):**
- L359: `AbstractDungeon.getCurrRoom().applyEndOfTurnRelics()`
- L359: `AbstractDungeon.getCurrRoom()`
- L360: `AbstractDungeon.getCurrRoom().applyEndOfTurnPreCardPowers()`
- L360: `AbstractDungeon.getCurrRoom()`
- L361: `this.addToBottom(new TriggerEndOfTurnOrbsAction())`
- L363: `c.triggerOnEndOfTurnForPlayingCard()`
- L365: `AbstractDungeon.player.stance.onEndOfTurn()`

**Objects created:**
- L361: `TriggerEndOfTurnOrbsAction`

### callEndTurnEarlySequence() (L368-381)

**Calls (in order):**
- L372: `AbstractDungeon.actionManager.addToBottom(new UseCardAction(i.card))`
- L374: `AbstractDungeon.actionManager.cardQueue.clear()`
- L376: `AbstractDungeon.effectList.add(new ExhaustCardEffect(c))`
- L378: `AbstractDungeon.player.limbo.group.clear()`
- L379: `AbstractDungeon.player.releaseCard()`
- L380: `AbstractDungeon.overlayMenu.endTurnButton.disable(true)`

**Objects created:**
- L372: `UseCardAction`
- L376: `ExhaustCardEffect`

### cleanCardQueue() (L383-393)

**Calls (in order):**
- L384: `this.cardQueue.iterator()`
- L385: `i.hasNext()`
- L386: `i.next()`
- L387: `AbstractDungeon.player.hand.contains(cardQueueItem.card)`
- L388: `i.remove()`

### isEmpty() (L395-397)

**Calls (in order):**
- L396: `this.actions.isEmpty()`

### clearNextRoomCombatActions() (L399-401)

**Calls (in order):**
- L400: `this.nextCombatActions.clear()`

### clear() (L403-424)

**Calls (in order):**
- L404: `this.actions.clear()`
- L405: `this.preTurnActions.clear()`
- L409: `this.cardsPlayedThisCombat.clear()`
- L410: `this.cardsPlayedThisTurn.clear()`
- L411: `this.orbsChanneledThisCombat.clear()`
- L412: `this.orbsChanneledThisTurn.clear()`
- L413: `this.uniqueStancesThisCombat.clear()`
- L414: `this.cardQueue.clear()`

### incrementDiscard(boolean endOfTurn) (L426-434)

**Calls (in order):**
- L429: `AbstractDungeon.player.updateCardsOnDiscard()`
- L431: `r.onManualDiscard()`

### queueExtraCard(AbstractCard card, AbstractMonster m) (L440-478)

**Calls (in order):**
- L441: `card.makeSameInstanceOf()`
- L442: `AbstractDungeon.player.limbo.addToBottom(tmp)`
- L447: `c.card.uuid.equals(card.uuid)`
- L469: `MathUtils.random((float)Settings.WIDTH * 0.2f, (float)Settings.WIDTH * 0.8f)`
- L470: `MathUtils.random((float)Settings.HEIGHT * 0.3f, (float)Settings.HEIGHT * 0.7f)`
- L474: `tmp.calculateCardDamage(m)`
- L477: `AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true)`

**Objects created:**
- L477: `CardQueueItem`

## MonsterGroup
File: `monsters\MonsterGroup.java`

### addMonster(int newIndex, AbstractMonster m) (L30-35)

**Calls (in order):**
- L34: `this.monsters.add(newIndex, m)`

### addMonster(AbstractMonster m) (L37-40)

**Calls (in order):**
- L39: `this.monsters.add(m)`

### addSpawnedMonster(AbstractMonster m) (L42-45)

**Calls (in order):**
- L44: `this.monsters.add(0, m)`

### showIntent() (L51-55)

**Calls (in order):**
- L53: `m.createIntent()`

### init() (L57-61)

**Calls (in order):**
- L59: `m.init()`

### add(AbstractMonster m) (L63-65)

**Calls (in order):**
- L64: `this.monsters.add(m)`

### usePreBattleAction() (L67-75)

**Calls (in order):**
- L72: `m.usePreBattleAction()`
- L73: `m.useUniversalPreBattleAction()`

### applyPreTurnLogic() (L93-101)

**Calls (in order):**
- L96: `m.hasPower("Barricade")`
- L97: `m.loseBlock()`
- L99: `m.applyStartOfTurnPowers()`

### getMonster(String id) (L103-110)

**Calls (in order):**
- L105: `m.id.equals(id)`
- L108: `logger.info("MONSTER GROUP getMonster(): Could not find monster: " + id)`

### queueMonsters() (L112-117)

**Calls (in order):**
- L114: `m.isDeadOrEscaped()`
- L115: `AbstractDungeon.actionManager.monsterQueue.add(new MonsterQueueItem(m))`

**Objects created:**
- L115: `MonsterQueueItem`

### getRandomMonster() (L143-145)

**Calls (in order):**
- L144: `this.getRandomMonster(null, false)`

### getRandomMonster(boolean aliveOnly) (L147-149)

**Calls (in order):**
- L148: `this.getRandomMonster(null, aliveOnly)`

### getRandomMonster(AbstractMonster exception, boolean aliveOnly, Random rng) (L151-189)

**Calls (in order):**
- L152: `this.areMonstersBasicallyDead()`
- L160: `tmp.add(m)`
- L162: `tmp.size()`
- L165: `tmp.get(rng.random(0, tmp.size() - 1))`
- L165: `rng.random(0, tmp.size() - 1)`
- L165: `tmp.size()`
- L167: `this.monsters.get(rng.random(0, this.monsters.size() - 1))`
- L167: `rng.random(0, this.monsters.size() - 1)`
- L167: `this.monsters.size()`
- L169: `this.monsters.size()`
- L170: `this.monsters.get(0)`
- L175: `exception.equals(m)`
- L176: `tmp.add(m)`
- L178: `tmp.size()`
- L181: `tmp.get(rng.random(0, tmp.size() - 1))`
- L181: `rng.random(0, tmp.size() - 1)`
- L181: `tmp.size()`
- L185: `exception.equals(m)`
- L186: `tmp.add(m)`
- L188: `tmp.get(rng.random(0, tmp.size() - 1))`
- L188: `rng.random(0, tmp.size() - 1)`
- L188: `tmp.size()`

**Objects created:**
- L157: `None`
- L173: `None`
- L183: `None`

### getRandomMonster(AbstractMonster exception, boolean aliveOnly) (L191-229)

**Calls (in order):**
- L192: `this.areMonstersBasicallyDead()`
- L200: `tmp.add(m)`
- L202: `tmp.size()`
- L205: `tmp.get(MathUtils.random(0, tmp.size() - 1))`
- L205: `MathUtils.random(0, tmp.size() - 1)`
- L205: `tmp.size()`
- L207: `this.monsters.get(MathUtils.random(0, this.monsters.size() - 1))`
- L207: `MathUtils.random(0, this.monsters.size() - 1)`
- L207: `this.monsters.size()`
- L209: `this.monsters.size()`
- L210: `this.monsters.get(0)`
- L215: `exception.equals(m)`
- L216: `tmp.add(m)`
- L218: `tmp.size()`
- L221: `tmp.get(MathUtils.random(0, tmp.size() - 1))`
- L221: `MathUtils.random(0, tmp.size() - 1)`
- L221: `tmp.size()`
- L225: `exception.equals(m)`
- L226: `tmp.add(m)`
- L228: `tmp.get(MathUtils.random(0, tmp.size() - 1))`
- L228: `MathUtils.random(0, tmp.size() - 1)`
- L228: `tmp.size()`

**Objects created:**
- L197: `None`
- L213: `None`
- L223: `None`

### update() (L231-252)

**Calls (in order):**
- L233: `m.update()`
- L239: `m.hb.update()`
- L240: `m.intentHb.update()`
- L241: `m.healthHb.update()`

### updateAnimations() (L254-258)

**Calls (in order):**
- L256: `m.updatePowers()`

### shouldFlipVfx() (L260-262)

**Calls (in order):**
- L261: `AbstractDungeon.lastCombatMetricKey.equals("Shield and Spear")`
- L261: `this.monsters.get((int)1)`

### escape() (L264-268)

**Calls (in order):**
- L266: `m.escape()`

### unhover() (L270-274)

**Calls (in order):**
- L272: `m.unhover()`

### render(SpriteBatch sb) (L276-283)

**Calls (in order):**
- L278: `this.hoveredMonster.renderTip(sb)`
- L281: `m.render(sb)`

### applyEndOfTurnPowers() (L285-299)

**Calls (in order):**
- L288: `m.applyEndOfTurnTriggers()`
- L291: `p.atEndOfRound()`
- L296: `p.atEndOfRound()`

### renderReticle(SpriteBatch sb) (L301-306)

**Calls (in order):**
- L304: `m.renderReticle(sb)`

### getMonsterNames() (L308-314)

**Calls (in order):**
- L311: `arr.add(m.id)`

**Objects created:**
- L309: `None`

## MonsterRoomBoss
File: `rooms\MonsterRoomBoss.java`

### onPlayerEntry() (L21-32)

**Calls (in order):**
- L23: `CardCrawlGame.dungeon.getBoss()`
- L24: `logger.info("BOSSES: " + AbstractDungeon.bossList.size())`
- L24: `AbstractDungeon.bossList.size()`
- L25: `CardCrawlGame.metricData.path_taken.add("BOSS")`
- L26: `CardCrawlGame.music.silenceBGM()`
- L27: `AbstractDungeon.bossList.remove(0)`
- L29: `this.monsters.init()`

## MonsterRoomElite
File: `rooms\MonsterRoomElite.java`

### applyEmeraldEliteBuff() (L33-63)

**Calls (in order):**
- L35: `AbstractDungeon.getCurrMapNode()`
- L36: `AbstractDungeon.mapRng.random(0, 3)`
- L39: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(m, m, new StrengthPower(m, AbstractDungeon.actNum + 1), A`
- L45: `AbstractDungeon.actionManager.addToBottom(new IncreaseMaxHpAction(m, 0.25f, true))`
- L51: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(m, m, new MetallicizePower(m, AbstractDungeon.actNum * 2 `
- L57: `AbstractDungeon.actionManager.addToBottom(new ApplyPowerAction(m, m, new RegenerateMonsterPower(m, 1 + AbstractDungeon.a`

**Objects created:**
- L39: `ApplyPowerAction`
- L39: `StrengthPower`
- L45: `IncreaseMaxHpAction`
- L51: `ApplyPowerAction`
- L51: `MetallicizePower`
- L57: `ApplyPowerAction`
- L57: `RegenerateMonsterPower`

### onPlayerEntry() (L65-73)

**Calls (in order):**
- L67: `this.playBGM(null)`
- L69: `CardCrawlGame.dungeon.getEliteMonsterForRoomCreation()`
- L70: `this.monsters.init()`

### dropReward() (L75-87)

**Calls (in order):**
- L77: `this.returnRandomRelicTier()`
- L78: `AbstractDungeon.player.hasBlight("MimicInfestation")`
- L79: `AbstractDungeon.player.getBlight("MimicInfestation").flash()`
- L79: `AbstractDungeon.player.getBlight("MimicInfestation")`
- L81: `this.addRelicToRewards(tier)`
- L82: `AbstractDungeon.player.hasRelic("Black Star")`
- L83: `this.addNoncampRelicToRewards(this.returnRandomRelicTier())`
- L83: `this.returnRandomRelicTier()`
- L85: `this.addEmeraldKey()`

### addEmeraldKey() (L89-93)

**Calls (in order):**
- L90: `this.rewards.isEmpty()`
- L90: `AbstractDungeon.getCurrMapNode()`
- L91: `this.rewards.add(new RewardItem((RewardItem)this.rewards.get(this.rewards.size() - 1), RewardItem.RewardType.EMERALD_KEY`
- L91: `this.rewards.get(this.rewards.size() - 1)`
- L91: `this.rewards.size()`

**Objects created:**
- L91: `RewardItem`

### returnRandomRelicTier() (L95-107)

**Calls (in order):**
- L96: `AbstractDungeon.relicRng.random(0, 99)`
- L97: `ModHelper.isModEnabled("Elite Swarm")`

### getCardRarity(int roll) (L109-115)

**Calls (in order):**
- L111: `ModHelper.isModEnabled("Elite Swarm")`
- L114: `super.getCardRarity(roll)`

