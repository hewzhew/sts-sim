# StS Damage Pipeline

Extracted from AbstractCreature.damage(), AbstractCard.calculateCardDamage(),
and related methods. Check call order carefully for modifier application.

## AbstractCard
File: `cards\AbstractCard.java`

### update() (L913-951)

```java
public void update() {
        this.updateFlashVfx();
        if (this.hoverTimer != 0.0f) {
            this.hoverTimer -= Gdx.graphics.getDeltaTime();
            if (this.hoverTimer < 0.0f) {
                this.hoverTimer = 0.0f;
            }
        }
        if (AbstractDungeon.player != null && AbstractDungeon.player.isDraggingCard && this == AbstractDungeon.player.hoveredCard) {
            this.current_x = MathHelper.cardLerpSnap(this.current_x, this.target_x);
            this.current_y = MathHelper.cardLerpSnap(this.current_y, this.target_y);
            if (AbstractDungeon.player.hasRelic("Necronomicon")) {
                if (this.cost >= 2 && this.type == CardType.ATTACK && AbstractDungeon.player.getRelic("Necronomicon").checkTrigger()) {
                    AbstractDungeon.player.getRelic("Necronomicon").beginLongPulse();
                } else {
                    AbstractDungeon.player.getRelic("Necronomicon").stopPulse();
                }
            }
        }
        if (Settings.FAST_MODE) {
            this.current_x = MathHelper.cardLerpSnap(this.current_x, this.target_x);
            this.current_y = MathHelper.cardLerpSnap(this.current_y, this.target_y);
        }
        this.current_x = MathHelper.cardLerpSnap(this.current_x, this.target_x);
        this.current_y = MathHelper.cardLerpSnap(this.current_y, this.target_y);
        this.hb.move(this.current_x, this.current_y);
        this.hb.resize(HB_W * this.drawScale, HB_H * this.drawScale);
        if (this.hb.clickStarted && this.hb.hovered) {
            this.drawScale = MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale * 0.9f);
            this.drawScale = MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale * 0.9f);
        } else {
            this.drawScale = MathHelper.cardScaleLerpSnap(this.drawScale, this.targetDrawScale);
        }
        if (this.angle != this.targetAngle) {
            this.angle = MathHelper.angleLerpSnap(this.angle, this.targetAngle);
        }
        this.updateTransparency();
        this.updateColor();
    }
```

### applyPowers() (L2209-2274)

```java
public void applyPowers() {
        this.applyPowersToBlock();
        AbstractPlayer player = AbstractDungeon.player;
        this.isDamageModified = false;
        if (!this.isMultiDamage) {
            float tmp = this.baseDamage;
            for (AbstractRelic r : player.relics) {
                tmp = r.atDamageModify(tmp, this);
                if (this.baseDamage == (int)tmp) continue;
                this.isDamageModified = true;
            }
            for (AbstractPower p : player.powers) {
                tmp = p.atDamageGive(tmp, this.damageTypeForTurn, this);
            }
            if (this.baseDamage != (int)(tmp = player.stance.atDamageGive(tmp, this.damageTypeForTurn, this))) {
                this.isDamageModified = true;
            }
            for (AbstractPower p : player.powers) {
                tmp = p.atDamageFinalGive(tmp, this.damageTypeForTurn, this);
            }
            if (tmp < 0.0f) {
                tmp = 0.0f;
            }
            if (this.baseDamage != MathUtils.floor(tmp)) {
                this.isDamageModified = true;
            }
            this.damage = MathUtils.floor(tmp);
        } else {
            int i;
            ArrayList<AbstractMonster> m = AbstractDungeon.getCurrRoom().monsters.monsters;
            float[] tmp = new float[m.size()];
            for (i = 0; i < tmp.length; ++i) {
                tmp[i] = this.baseDamage;
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractRelic r : player.relics) {
                    tmp[i] = r.atDamageModify(tmp[i], this);
                    if (this.baseDamage == (int)tmp[i]) continue;
                    this.isDamageModified = true;
                }
                for (AbstractPower p : player.powers) {
                    tmp[i] = p.atDamageGive(tmp[i], this.damageTypeForTurn, this);
                }
                tmp[i] = player.stance.atDamageGive(tmp[i], this.damageTypeForTurn, this);
                if (this.baseDamage == (int)tmp[i]) continue;
                this.isDamageModified = true;
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractPower p : player.powers) {
                    tmp[i] = p.atDamageFinalGive(tmp[i], this.damageTypeForTurn, this);
                }
            }
            for (i = 0; i < tmp.length; ++i) {
                if (!(tmp[i] < 0.0f)) continue;
                tmp[i] = 0.0f;
            }
            this.multiDamage = new int[tmp.length];
            for (i = 0; i < tmp.length; ++i) {
                if (this.baseDamage != (int)tmp[i]) {
                    this.isDamageModified = true;
                }
                this.multiDamage[i] = MathUtils.floor(tmp[i]);
            }
            this.damage = this.multiDamage[0];
        }
    }
```

### applyPowersToBlock() (L2276-2292)

```java
protected void applyPowersToBlock() {
        this.isBlockModified = false;
        float tmp = this.baseBlock;
        for (AbstractPower p : AbstractDungeon.player.powers) {
            tmp = p.modifyBlock(tmp, this);
        }
        for (AbstractPower p : AbstractDungeon.player.powers) {
            tmp = p.modifyBlockLast(tmp);
        }
        if (this.baseBlock != MathUtils.floor(tmp)) {
            this.isBlockModified = true;
        }
        if (tmp < 0.0f) {
            tmp = 0.0f;
        }
        this.block = MathUtils.floor(tmp);
    }
```

### calculateCardDamage(AbstractMonster mo) (L2298-2381)

```java
public void calculateCardDamage(AbstractMonster mo) {
        this.applyPowersToBlock();
        AbstractPlayer player = AbstractDungeon.player;
        this.isDamageModified = false;
        if (!this.isMultiDamage && mo != null) {
            float tmp = this.baseDamage;
            for (AbstractRelic r : player.relics) {
                tmp = r.atDamageModify(tmp, this);
                if (this.baseDamage == (int)tmp) continue;
                this.isDamageModified = true;
            }
            for (AbstractPower p : player.powers) {
                tmp = p.atDamageGive(tmp, this.damageTypeForTurn, this);
            }
            if (this.baseDamage != (int)(tmp = player.stance.atDamageGive(tmp, this.damageTypeForTurn, this))) {
                this.isDamageModified = true;
            }
            for (AbstractPower p : mo.powers) {
                tmp = p.atDamageReceive(tmp, this.damageTypeForTurn, this);
            }
            for (AbstractPower p : player.powers) {
                tmp = p.atDamageFinalGive(tmp, this.damageTypeForTurn, this);
            }
            for (AbstractPower p : mo.powers) {
                tmp = p.atDamageFinalReceive(tmp, this.damageTypeForTurn, this);
            }
            if (tmp < 0.0f) {
                tmp = 0.0f;
            }
            if (this.baseDamage != MathUtils.floor(tmp)) {
                this.isDamageModified = true;
            }
            this.damage = MathUtils.floor(tmp);
        } else {
            int i;
            ArrayList<AbstractMonster> m = AbstractDungeon.getCurrRoom().monsters.monsters;
            float[] tmp = new float[m.size()];
            for (i = 0; i < tmp.length; ++i) {
                tmp[i] = this.baseDamage;
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractRelic r : player.relics) {
                    tmp[i] = r.atDamageModify(tmp[i], this);
                    if (this.baseDamage == (int)tmp[i]) continue;
                    this.isDamageModified = true;
                }
                for (AbstractPower p : player.powers) {
                    tmp[i] = p.atDamageGive(tmp[i], this.damageTypeForTurn, this);
                }
                tmp[i] = player.stance.atDamageGive(tmp[i], this.damageTypeForTurn, this);
                if (this.baseDamage == (int)tmp[i]) continue;
                this.isDamageModified = true;
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractPower p : m.get((int)i).powers) {
                    if (m.get((int)i).isDying || m.get((int)i).isEscaping) continue;
                    tmp[i] = p.atDamageReceive(tmp[i], this.damageTypeForTurn, this);
                }
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractPower p : player.powers) {
                    tmp[i] = p.atDamageFinalGive(tmp[i], this.damageTypeForTurn, this);
                }
            }
            for (i = 0; i < tmp.length; ++i) {
                for (AbstractPower p : m.get((int)i).powers) {
                    if (m.get((int)i).isDying || m.get((int)i).isEscaping) continue;
                    tmp[i] = p.atDamageFinalReceive(tmp[i], this.damageTypeForTurn, this);
                }
            }
            for (i = 0; i < tmp.length; ++i) {
                if (!(tmp[i] < 0.0f)) continue;
                tmp[i] = 0.0f;
            }
            this.multiDamage = new int[tmp.length];
            for (i = 0; i < tmp.length; ++i) {
                if (this.baseDamage != MathUtils.floor(tmp[i])) {
                    this.isDamageModified = true;
                }
                this.multiDamage[i] = MathUtils.floor(tmp[i]);
            }
            this.damage = this.multiDamage[0];
        }
    }
```

## AbstractCreature
File: `core\AbstractCreature.java`

### damage(DamageInfo var1) (L146-146)

```java
public abstract void damage(DamageInfo var1);
```

## AbstractMonster
File: `monsters\AbstractMonster.java`

### update() (L162-173)

```java
public void update() {
        for (AbstractPower p : this.powers) {
            p.updateParticles();
        }
        this.updateReticle();
        this.updateHealthBar();
        this.updateAnimations();
        this.updateDeathAnimation();
        this.updateEscapeAnimation();
        this.updateIntent();
        this.tint.update();
    }
```

### damage(DamageInfo info) (L607-696)

```java
@Override
    public void damage(DamageInfo info) {
        boolean probablyInstantKill;
        if (info.output > 0 && this.hasPower("IntangiblePlayer")) {
            info.output = 1;
        }
        int damageAmount = info.output;
        if (this.isDying || this.isEscaping) {
            return;
        }
        if (damageAmount < 0) {
            damageAmount = 0;
        }
        boolean hadBlock = true;
        if (this.currentBlock == 0) {
            hadBlock = false;
        }
        boolean weakenedToZero = damageAmount == 0;
        damageAmount = this.decrementBlock(info, damageAmount);
        if (info.owner == AbstractDungeon.player) {
            for (AbstractRelic r : AbstractDungeon.player.relics) {
                damageAmount = r.onAttackToChangeDamage(info, damageAmount);
            }
        }
        if (info.owner != null) {
            for (AbstractPower p : info.owner.powers) {
                damageAmount = p.onAttackToChangeDamage(info, damageAmount);
            }
        }
        for (AbstractPower p : this.powers) {
            damageAmount = p.onAttackedToChangeDamage(info, damageAmount);
        }
        if (info.owner == AbstractDungeon.player) {
            for (AbstractRelic r : AbstractDungeon.player.relics) {
                r.onAttack(info, damageAmount, this);
            }
        }
        for (AbstractPower p : this.powers) {
            p.wasHPLost(info, damageAmount);
        }
        if (info.owner != null) {
            for (AbstractPower p : info.owner.powers) {
                p.onAttack(info, damageAmount, this);
            }
        }
        for (AbstractPower p : this.powers) {
            damageAmount = p.onAttacked(info, damageAmount);
        }
        this.lastDamageTaken = Math.min(damageAmount, this.currentHealth);
        boolean bl = probablyInstantKill = this.currentHealth == 0;
        if (damageAmount > 0) {
            if (info.owner != this) {
                this.useStaggerAnimation();
            }
            if (damageAmount >= 99 && !CardCrawlGame.overkill) {
                CardCrawlGame.overkill = true;
            }
            this.currentHealth -= damageAmount;
            if (!probablyInstantKill) {
                AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, damageAmount));
            }
            if (this.currentHealth < 0) {
                this.currentHealth = 0;
            }
            this.healthBarUpdatedEvent();
        } else if (!probablyInstantKill) {
            if (weakenedToZero && this.currentBlock == 0) {
                if (hadBlock) {
                    AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, TEXT[30]));
                } else {
                    AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, 0));
                }
            } else if (Settings.SHOW_DMG_BLOCK) {
                AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, TEXT[30]));
            }
        }
        if (this.currentHealth <= 0) {
            this.die();
            if (AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
                AbstractDungeon.actionManager.cleanCardQueue();
                AbstractDungeon.effectList.add(new DeckPoofEffect(64.0f * Settings.scale, 64.0f * Settings.scale, true));
                AbstractDungeon.effectList.add(new DeckPoofEffect((float)Settings.WIDTH - 64.0f * Settings.scale, 64.0f * Settings.scale, false));
                AbstractDungeon.overlayMenu.hideCombatPanels();
            }
            if (this.currentBlock > 0) {
                this.loseBlock();
                AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - this.hb.height / 2.0f + BLOCK_ICON_Y));
            }
        }
    }
```

### applyPowers() (L984-999)

```java
public void applyPowers() {
        boolean applyBackAttack = this.applyBackAttack();
        if (applyBackAttack && !this.hasPower("BackAttack")) {
            AbstractDungeon.actionManager.addToTop(new ApplyPowerAction(this, null, new BackAttackPower(this)));
        }
        for (DamageInfo dmg : this.damage) {
            dmg.applyPowers(this, AbstractDungeon.player);
            if (!applyBackAttack) continue;
            dmg.output = (int)((float)dmg.output * 1.5f);
        }
        if (this.move.baseDamage > -1) {
            this.calculateDamage(this.move.baseDamage);
        }
        this.intentImg = this.getIntentImg();
        this.updateIntentTip();
    }
```

## AbstractPlayer
File: `characters\AbstractPlayer.java`

### update() (L412-426)

```java
public void update() {
        this.updateControllerInput();
        this.hb.update();
        this.updateHealthBar();
        this.updatePowers();
        this.healthHb.update();
        this.updateReticle();
        this.tint.update();
        if (AbstractDungeon.getCurrRoom().phase != AbstractRoom.RoomPhase.EVENT) {
            for (AbstractOrb o : this.orbs) {
                o.updateAnimation();
            }
        }
        this.updateEscapeAnimation();
    }
```

### damage(DamageInfo info) (L1368-1498)

```java
@Override
    public void damage(DamageInfo info) {
        int damageAmount = info.output;
        boolean hadBlock = true;
        if (this.currentBlock == 0) {
            hadBlock = false;
        }
        if (damageAmount < 0) {
            damageAmount = 0;
        }
        if (damageAmount > 1 && this.hasPower("IntangiblePlayer")) {
            damageAmount = 1;
        }
        damageAmount = this.decrementBlock(info, damageAmount);
        if (info.owner == this) {
            for (AbstractRelic abstractRelic : this.relics) {
                damageAmount = abstractRelic.onAttackToChangeDamage(info, damageAmount);
            }
        }
        if (info.owner != null) {
            for (AbstractPower abstractPower : info.owner.powers) {
                damageAmount = abstractPower.onAttackToChangeDamage(info, damageAmount);
            }
        }
        for (AbstractRelic abstractRelic : this.relics) {
            damageAmount = abstractRelic.onAttackedToChangeDamage(info, damageAmount);
        }
        for (AbstractPower abstractPower : this.powers) {
            damageAmount = abstractPower.onAttackedToChangeDamage(info, damageAmount);
        }
        if (info.owner == this) {
            for (AbstractRelic abstractRelic : this.relics) {
                abstractRelic.onAttack(info, damageAmount, this);
            }
        }
        if (info.owner != null) {
            for (AbstractPower abstractPower : info.owner.powers) {
                abstractPower.onAttack(info, damageAmount, this);
            }
            for (AbstractPower abstractPower : this.powers) {
                damageAmount = abstractPower.onAttacked(info, damageAmount);
            }
            for (AbstractRelic abstractRelic : this.relics) {
                damageAmount = abstractRelic.onAttacked(info, damageAmount);
            }
        } else {
            logger.info("NO OWNER, DON'T TRIGGER POWERS");
        }
        for (AbstractRelic abstractRelic : this.relics) {
            damageAmount = abstractRelic.onLoseHpLast(damageAmount);
        }
        this.lastDamageTaken = Math.min(damageAmount, this.currentHealth);
        if (damageAmount > 0) {
            for (AbstractPower abstractPower : this.powers) {
                damageAmount = abstractPower.onLoseHp(damageAmount);
            }
            for (AbstractRelic abstractRelic : this.relics) {
                abstractRelic.onLoseHp(damageAmount);
            }
            for (AbstractPower abstractPower : this.powers) {
                abstractPower.wasHPLost(info, damageAmount);
            }
            for (AbstractRelic abstractRelic : this.relics) {
                abstractRelic.wasHPLost(damageAmount);
            }
            if (info.owner != null) {
                for (AbstractPower abstractPower : info.owner.powers) {
                    abstractPower.onInflictDamage(info, damageAmount, this);
                }
            }
            if (info.owner != this) {
                this.useStaggerAnimation();
            }
            if (info.type == DamageInfo.DamageType.HP_LOSS) {
                GameActionManager.hpLossThisCombat += damageAmount;
            }
            GameActionManager.damageReceivedThisTurn += damageAmount;
            GameActionManager.damageReceivedThisCombat += damageAmount;
            this.currentHealth -= damageAmount;
            if (damageAmount > 0 && AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT) {
                this.updateCardsOnDamage();
                ++this.damagedThisCombat;
            }
            AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, damageAmount));
            if (this.currentHealth < 0) {
                this.currentHealth = 0;
            } else if (this.currentHealth < this.maxHealth / 4) {
                AbstractDungeon.topLevelEffects.add(new BorderFlashEffect(new Color(1.0f, 0.1f, 0.05f, 0.0f)));
            }
            this.healthBarUpdatedEvent();
            if ((float)this.currentHealth <= (float)this.maxHealth / 2.0f && !this.isBloodied) {
                this.isBloodied = true;
                for (AbstractRelic abstractRelic : this.relics) {
                    if (abstractRelic == null) continue;
                    abstractRelic.onBloodied();
                }
            }
            if (this.currentHealth < 1) {
                if (!this.hasRelic("Mark of the Bloom")) {
                    if (this.hasPotion("FairyPotion")) {
                        for (AbstractPotion abstractPotion : this.potions) {
                            if (!abstractPotion.ID.equals("FairyPotion")) continue;
                            abstractPotion.flash();
                            this.currentHealth = 0;
                            abstractPotion.use(this);
                            AbstractDungeon.topPanel.destroyPotion(abstractPotion.slot);
                            return;
                        }
                    } else if (this.hasRelic("Lizard Tail") && ((LizardTail)this.getRelic((String)"Lizard Tail")).counter == -1) {
                        this.currentHealth = 0;
                        this.getRelic("Lizard Tail").onTrigger();
                        return;
                    }
                }
                this.isDead = true;
                AbstractDungeon.deathScreen = new DeathScreen(AbstractDungeon.getMonsters());
                this.currentHealth = 0;
                if (this.currentBlock > 0) {
                    this.loseBlock();
                    AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - this.hb.height / 2.0f + BLOCK_ICON_Y));
                }
            }
        } else if (this.currentBlock > 0) {
            AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, AbstractPlayer.uiStrings.TEXT[0]));
        } else if (hadBlock) {
            AbstractDungeon.effectList.add(new BlockedWordEffect(this, this.hb.cX, this.hb.cY, AbstractPlayer.uiStrings.TEXT[0]));
            AbstractDungeon.effectList.add(new HbBlockBrokenEffect(this.hb.cX - this.hb.width / 2.0f + BLOCK_ICON_X, this.hb.cY - this.hb.height / 2.0f + BLOCK_ICON_Y));
        } else {
            AbstractDungeon.effectList.add(new StrikeEffect((AbstractCreature)this, this.hb.cX, this.hb.cY, 0));
        }
    }
```

## AbstractPower
File: `powers\AbstractPower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L198-200)

```java
public float atDamageGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

### atDamageFinalGive(float damage, DamageInfo.DamageType type) (L202-204)

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

### atDamageFinalReceive(float damage, DamageInfo.DamageType type) (L206-208)

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

### atDamageReceive(float damage, DamageInfo.DamageType damageType) (L210-212)

```java
public float atDamageReceive(float damage, DamageInfo.DamageType damageType) {
        return damage;
    }
```

### atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) (L214-216)

```java
public float atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageGive(damage, type);
    }
```

### atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card) (L218-220)

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalGive(damage, type);
    }
```

### atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card) (L222-224)

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalReceive(damage, type);
    }
```

### atDamageReceive(float damage, DamageInfo.DamageType damageType, AbstractCard card) (L226-228)

```java
public float atDamageReceive(float damage, DamageInfo.DamageType damageType, AbstractCard card) {
        return this.atDamageReceive(damage, damageType);
    }
```

### onAttacked(DamageInfo info, int damageAmount) (L258-260)

```java
public int onAttacked(DamageInfo info, int damageAmount) {
        return damageAmount;
    }
```

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target) (L262-263)

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

## AbstractRelic
File: `relics\AbstractRelic.java`

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target) (L565-566)

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

### onAttacked(DamageInfo info, int damageAmount) (L568-570)

```java
public int onAttacked(DamageInfo info, int damageAmount) {
        return damageAmount;
    }
```

## AbstractStance
File: `stances\AbstractStance.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L49-51)

```java
public float atDamageGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

### atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) (L53-55)

```java
public float atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageGive(damage, type);
    }
```

### atDamageReceive(float damage, DamageInfo.DamageType damageType) (L57-59)

```java
public float atDamageReceive(float damage, DamageInfo.DamageType damageType) {
        return damage;
    }
```

## AcidSlime_L
File: `monsters\exordium\AcidSlime_L.java`

### damage(DamageInfo info) (L136-146)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (!this.isDying && (float)this.currentHealth <= (float)this.maxHealth / 2.0f && this.nextMove != 3 && !this.splitTriggered) {
            this.setMove(SPLIT_NAME, (byte)3, AbstractMonster.Intent.UNKNOWN);
            this.createIntent();
            AbstractDungeon.actionManager.addToBottom(new TextAboveCreatureAction((AbstractCreature)this, TextAboveCreatureAction.TextType.INTERRUPTED));
            AbstractDungeon.actionManager.addToBottom(new SetMoveAction((AbstractMonster)this, SPLIT_NAME, 3, AbstractMonster.Intent.UNKNOWN));
            this.splitTriggered = true;
        }
    }
```

## AngryPower
File: `powers\AngryPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L31-38)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && damageAmount > 0 && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS) {
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
            this.flash();
        }
        return damageAmount;
    }
```

## AwakenedOne
File: `monsters\beyond\AwakenedOne.java`

### damage(DamageInfo info) (L269-309)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            if (this.form1) {
                this.state.addAnimation(0, "Idle_1", true, 0.0f);
            } else {
                this.state.addAnimation(0, "Idle_2", true, 0.0f);
            }
        }
        if (this.currentHealth <= 0 && !this.halfDead) {
            if (AbstractDungeon.getCurrRoom().cannotLose) {
                this.halfDead = true;
            }
            for (AbstractPower p : this.powers) {
                p.onDeath();
            }
            for (AbstractRelic r : AbstractDungeon.player.relics) {
                r.onMonsterDeath(this);
            }
            this.addToTop(new ClearCardQueueAction());
            Iterator s = this.powers.iterator();
            while (s.hasNext()) {
                AbstractPower p;
                p = (AbstractPower)s.next();
                if (p.type != AbstractPower.PowerType.DEBUFF && !p.ID.equals("Curiosity") && !p.ID.equals("Unawakened") && !p.ID.equals("Shackled")) continue;
                s.remove();
            }
            this.setMove((byte)3, AbstractMonster.Intent.UNKNOWN);
            this.createIntent();
            AbstractDungeon.actionManager.addToBottom(new ShoutAction(this, DIALOG[0]));
            AbstractDungeon.actionManager.addToBottom(new SetMoveAction(this, 3, AbstractMonster.Intent.UNKNOWN));
            this.applyPowers();
            this.firstTurn = true;
            this.form1 = false;
            if (GameActionManager.turn <= 1) {
                UnlockTracker.unlockAchievement("YOU_ARE_NOTHING");
            }
        }
    }
```

## BanditBear
File: `monsters\city\BanditBear.java`

### damage(DamageInfo info) (L110-118)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(1.0f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## BanditLeader
File: `monsters\city\BanditLeader.java`

### damage(DamageInfo info) (L133-141)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(0.8f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## BanditPointy
File: `monsters\city\BanditPointy.java`

### damage(DamageInfo info) (L81-89)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(1.0f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Blizzard
File: `cards\blue\Blizzard.java`

### applyPowers() (L49-62)

```java
@Override
    public void applyPowers() {
        int frostCount = 0;
        for (AbstractOrb o : AbstractDungeon.actionManager.orbsChanneledThisCombat) {
            if (!(o instanceof Frost)) continue;
            ++frostCount;
        }
        if (frostCount > 0) {
            this.baseDamage = frostCount * this.magicNumber;
            super.applyPowers();
            this.rawDescription = Blizzard.cardStrings.DESCRIPTION + Blizzard.cardStrings.EXTENDED_DESCRIPTION[0];
            this.initializeDescription();
        }
    }
```

### calculateCardDamage(AbstractMonster mo) (L70-76)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        super.calculateCardDamage(mo);
        this.rawDescription = Blizzard.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + Blizzard.cardStrings.EXTENDED_DESCRIPTION[0];
        this.initializeDescription();
    }
```

## BlockReturnPower
File: `powers\watcher\BlockReturnPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L37-44)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner) {
            this.flash();
            this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, this.amount, Settings.FAST_MODE));
        }
        return damageAmount;
    }
```

## BodySlam
File: `cards\red\BodySlam.java`

### applyPowers() (L36-43)

```java
@Override
    public void applyPowers() {
        this.baseDamage = AbstractDungeon.player.currentBlock;
        super.applyPowers();
        this.rawDescription = BodySlam.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + BodySlam.cardStrings.UPGRADE_DESCRIPTION;
        this.initializeDescription();
    }
```

### calculateCardDamage(AbstractMonster mo) (L51-57)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        super.calculateCardDamage(mo);
        this.rawDescription = BodySlam.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + BodySlam.cardStrings.UPGRADE_DESCRIPTION;
        this.initializeDescription();
    }
```

## BookOfStabbing
File: `monsters\city\BookOfStabbing.java`

### damage(DamageInfo info) (L113-120)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Brilliance
File: `cards\purple\Brilliance.java`

### applyPowers() (L27-35)

```java
@Override
    public void applyPowers() {
        int realBaseDamage = this.baseDamage;
        this.baseMagicNumber = AbstractDungeon.actionManager.mantraGained;
        this.baseDamage += this.baseMagicNumber;
        super.applyPowers();
        this.baseDamage = realBaseDamage;
        this.isDamageModified = this.damage != this.baseDamage;
    }
```

### calculateCardDamage(AbstractMonster mo) (L37-45)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        this.baseMagicNumber = AbstractDungeon.actionManager.mantraGained;
        int realBaseDamage = this.baseDamage;
        this.baseDamage += this.baseMagicNumber;
        super.calculateCardDamage(mo);
        this.baseDamage = realBaseDamage;
        this.isDamageModified = this.damage != this.baseDamage;
    }
```

## CardGroup
File: `cards\CardGroup.java`

### applyPowers() (L151-155)

```java
public void applyPowers() {
        for (AbstractCard c : this.group) {
            c.applyPowers();
        }
    }
```

## Centurion
File: `monsters\city\Centurion.java`

### damage(DamageInfo info) (L156-164)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(0.8f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Champ
File: `monsters\city\Champ.java`

### damage(DamageInfo info) (L221-228)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Chosen
File: `monsters\city\Chosen.java`

### damage(DamageInfo info) (L193-201)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(0.8f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## CurlUpPower
File: `powers\CurlUpPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L33-43)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (!this.triggered && damageAmount < this.owner.currentHealth && damageAmount > 0 && info.owner != null && info.type == DamageInfo.DamageType.NORMAL) {
            this.flash();
            this.triggered = true;
            this.addToBot(new ChangeStateAction((AbstractMonster)this.owner, "CLOSED"));
            this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
        return damageAmount;
    }
```

## DEPRECATEDDodecahedron
File: `relics\deprecated\DEPRECATEDDodecahedron.java`

### onAttacked(DamageInfo info, int damageAmount) (L76-82)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.stopPulse();
        }
        return super.onAttacked(info, damageAmount);
    }
```

## DEPRECATEDHotHotPower
File: `powers\deprecated\DEPRECATEDHotHotPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L36-43)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner && damageAmount > 0 && !this.owner.hasPower("Buffer")) {
            this.flash();
            AbstractDungeon.actionManager.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE, true));
        }
        return damageAmount;
    }
```

## DEPRECATEDRetributionPower
File: `powers\deprecated\DEPRECATEDRetributionPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L28-35)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new VigorPower(this.owner, this.amount), this.amount));
        }
        return damageAmount;
    }
```

## DEPRECATEDSerenityPower
File: `powers\deprecated\DEPRECATEDSerenityPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L39-48)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0 && ((AbstractPlayer)this.owner).stance.ID.equals("Calm")) {
            this.flash();
            if ((damageAmount -= this.amount) < this.amount) {
                damageAmount = 0;
            }
        }
        return damageAmount;
    }
```

## DamageAction
File: `actions\common\DamageAction.java`

### update() (L58-91)

```java
@Override
    public void update() {
        if (this.shouldCancelAction() && this.info.type != DamageInfo.DamageType.THORNS) {
            this.isDone = true;
            return;
        }
        if (this.duration == 0.1f) {
            if (this.info.type != DamageInfo.DamageType.THORNS && (this.info.owner.isDying || this.info.owner.halfDead)) {
                this.isDone = true;
                return;
            }
            AbstractDungeon.effectList.add(new FlashAtkImgEffect(this.target.hb.cX, this.target.hb.cY, this.attackEffect, this.muteSfx));
            if (this.goldAmount != 0) {
                this.stealGold();
            }
        }
        this.tickDuration();
        if (this.isDone) {
            if (this.attackEffect == AbstractGameAction.AttackEffect.POISON) {
                this.target.tint.color.set(Color.CHARTREUSE.cpy());
                this.target.tint.changeColor(Color.WHITE.cpy());
            } else if (this.attackEffect == AbstractGameAction.AttackEffect.FIRE) {
                this.target.tint.color.set(Color.RED);
                this.target.tint.changeColor(Color.WHITE.cpy());
            }
            this.target.damage(this.info);
            if (AbstractDungeon.getCurrRoom().monsters.areMonstersBasicallyDead()) {
                AbstractDungeon.actionManager.clearPostCombatActions();
            }
            if (!this.skipWait && !Settings.FAST_MODE) {
                this.addToTop(new WaitAction(0.1f));
            }
        }
    }
```

## DamageInfo
File: `cards\DamageInfo.java`

### applyPowers(AbstractCreature owner, AbstractCreature target) (L31-96)

```java
public void applyPowers(AbstractCreature owner, AbstractCreature target) {
        this.output = this.base;
        this.isModified = false;
        float tmp = this.output;
        if (!owner.isPlayer) {
            float mod;
            if (Settings.isEndless && AbstractDungeon.player.hasBlight("DeadlyEnemies") && this.base != (int)(tmp *= (mod = AbstractDungeon.player.getBlight("DeadlyEnemies").effectFloat()))) {
                this.isModified = true;
            }
            for (AbstractPower p : owner.powers) {
                tmp = p.atDamageGive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            for (AbstractPower p : target.powers) {
                tmp = p.atDamageReceive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            if (this.base != (int)(tmp = AbstractDungeon.player.stance.atDamageReceive(tmp, this.type))) {
                this.isModified = true;
            }
            for (AbstractPower p : owner.powers) {
                tmp = p.atDamageFinalGive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            for (AbstractPower p : target.powers) {
                tmp = p.atDamageFinalReceive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            this.output = MathUtils.floor(tmp);
            if (this.output < 0) {
                this.output = 0;
            }
        } else {
            for (AbstractPower p : owner.powers) {
                tmp = p.atDamageGive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            if (this.base != (int)(tmp = AbstractDungeon.player.stance.atDamageGive(tmp, this.type))) {
                this.isModified = true;
            }
            for (AbstractPower p : target.powers) {
                tmp = p.atDamageReceive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            for (AbstractPower p : owner.powers) {
                tmp = p.atDamageFinalGive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            for (AbstractPower p : target.powers) {
                tmp = p.atDamageFinalReceive(tmp, this.type);
                if (this.base == (int)tmp) continue;
                this.isModified = true;
            }
            this.output = MathUtils.floor(tmp);
            if (this.output < 0) {
                this.output = 0;
            }
        }
    }
```

## Darkling
File: `monsters\beyond\Darkling.java`

### damage(DamageInfo info) (L192-228)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (this.currentHealth <= 0 && !this.halfDead) {
            this.halfDead = true;
            for (AbstractPower p : this.powers) {
                p.onDeath();
            }
            for (AbstractRelic r : AbstractDungeon.player.relics) {
                r.onMonsterDeath(this);
            }
            this.powers.clear();
            logger.info("This monster is now half dead.");
            boolean allDead = true;
            for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
                if (!m.id.equals(ID) || m.halfDead) continue;
                allDead = false;
            }
            logger.info("All dead: " + allDead);
            if (!allDead) {
                if (this.nextMove != 4) {
                    this.setMove((byte)4, AbstractMonster.Intent.UNKNOWN);
                    this.createIntent();
                    AbstractDungeon.actionManager.addToBottom(new SetMoveAction(this, 4, AbstractMonster.Intent.UNKNOWN));
                }
            } else {
                AbstractDungeon.getCurrRoom().cannotLose = false;
                this.halfDead = false;
                for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
                    m.die();
                }
            }
        } else if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Deca
File: `monsters\beyond\Deca.java`

### damage(DamageInfo info) (L80-87)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Defect
File: `characters\Defect.java`

### damage(DamageInfo info) (L275-283)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output - this.currentBlock > 0) {
            AnimationState.TrackEntry e = this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTime(0.9f);
        }
        super.damage(info);
    }
```

## DivinityStance
File: `stances\DivinityStance.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L55-61)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 3.0f;
        }
        return damage;
    }
```

## Donu
File: `monsters\beyond\Donu.java`

### damage(DamageInfo info) (L76-83)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## DoubleDamagePower
File: `powers\DoubleDamagePower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L53-59)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

## EnvenomPower
File: `powers\EnvenomPower.java`

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target) (L35-41)

```java
@Override
    public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
        if (damageAmount > 0 && target != this.owner && info.type == DamageInfo.DamageType.NORMAL) {
            this.flash();
            this.addToTop(new ApplyPowerAction(target, this.owner, (AbstractPower)new PoisonPower(target, this.owner, this.amount), this.amount, true));
        }
    }
```

## FTL
File: `cards\blue\FTL.java`

### applyPowers() (L33-41)

```java
@Override
    public void applyPowers() {
        super.applyPowers();
        int count = AbstractDungeon.actionManager.cardsPlayedThisTurn.size();
        this.rawDescription = FTL.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + FTL.cardStrings.EXTENDED_DESCRIPTION[0] + count;
        this.rawDescription = count == 1 ? this.rawDescription + FTL.cardStrings.EXTENDED_DESCRIPTION[1] : this.rawDescription + FTL.cardStrings.EXTENDED_DESCRIPTION[2];
        this.initializeDescription();
    }
```

## Finisher
File: `cards\green\Finisher.java`

### applyPowers() (L33-45)

```java
@Override
    public void applyPowers() {
        super.applyPowers();
        int count = 0;
        for (AbstractCard c : AbstractDungeon.actionManager.cardsPlayedThisTurn) {
            if (c.type != AbstractCard.CardType.ATTACK) continue;
            ++count;
        }
        this.rawDescription = Finisher.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + Finisher.cardStrings.EXTENDED_DESCRIPTION[0] + count;
        this.rawDescription = count == 1 ? this.rawDescription + Finisher.cardStrings.EXTENDED_DESCRIPTION[1] : this.rawDescription + Finisher.cardStrings.EXTENDED_DESCRIPTION[2];
        this.initializeDescription();
    }
```

## FlameBarrierPower
File: `powers\FlameBarrierPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L46-53)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != this.owner) {
            this.flash();
            this.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
        return damageAmount;
    }
```

## Flechettes
File: `cards\green\Flechettes.java`

### applyPowers() (L32-44)

```java
@Override
    public void applyPowers() {
        super.applyPowers();
        int count = 0;
        for (AbstractCard c : AbstractDungeon.player.hand.group) {
            if (c.type != AbstractCard.CardType.SKILL) continue;
            ++count;
        }
        this.rawDescription = Flechettes.cardStrings.DESCRIPTION;
        this.rawDescription = this.rawDescription + Flechettes.cardStrings.EXTENDED_DESCRIPTION[0] + count;
        this.rawDescription = count == 1 ? this.rawDescription + Flechettes.cardStrings.EXTENDED_DESCRIPTION[1] : this.rawDescription + Flechettes.cardStrings.EXTENDED_DESCRIPTION[2];
        this.initializeDescription();
    }
```

## FlightPower
File: `powers\FlightPower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type) (L50-53)

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return this.calculateDamageTakenAmount(damage, type);
    }
```

### onAttacked(DamageInfo info, int damageAmount) (L62-70)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        Boolean willLive = this.calculateDamageTakenAmount(damageAmount, info.type) < (float)this.owner.currentHealth;
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 0 && willLive.booleanValue()) {
            this.flash();
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
        return damageAmount;
    }
```

## ForcefieldPower
File: `powers\ForcefieldPower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type) (L33-39)

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 0.0f && type != DamageInfo.DamageType.HP_LOSS && type != DamageInfo.DamageType.THORNS) {
            return 0.0f;
        }
        return damage;
    }
```

## FungiBeast
File: `monsters\exordium\FungiBeast.java`

### damage(DamageInfo info) (L120-127)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## GeneticAlgorithm
File: `cards\blue\GeneticAlgorithm.java`

### applyPowers() (L34-39)

```java
@Override
    public void applyPowers() {
        this.baseBlock = this.misc;
        super.applyPowers();
        this.initializeDescription();
    }
```

## GremlinLeader
File: `monsters\city\GremlinLeader.java`

### damage(DamageInfo info) (L208-215)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Halt
File: `cards\purple\Halt.java`

### applyPowers() (L36-45)

```java
@Override
    public void applyPowers() {
        this.baseBlock += 6 + this.timesUpgraded * 4;
        this.baseMagicNumber = this.baseBlock;
        super.applyPowers();
        this.magicNumber = this.block;
        this.isMagicNumberModified = this.isBlockModified;
        this.baseBlock -= 6 + this.timesUpgraded * 4;
        super.applyPowers();
    }
```

## Healer
File: `monsters\city\Healer.java`

### damage(DamageInfo info) (L176-184)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(0.8f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## HeavyBlade
File: `cards\red\HeavyBlade.java`

### applyPowers() (L39-49)

```java
@Override
    public void applyPowers() {
        AbstractPower strength = AbstractDungeon.player.getPower("Strength");
        if (strength != null) {
            strength.amount *= this.magicNumber;
        }
        super.applyPowers();
        if (strength != null) {
            strength.amount /= this.magicNumber;
        }
    }
```

### calculateCardDamage(AbstractMonster mo) (L51-61)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        AbstractPower strength = AbstractDungeon.player.getPower("Strength");
        if (strength != null) {
            strength.amount *= this.magicNumber;
        }
        super.calculateCardDamage(mo);
        if (strength != null) {
            strength.amount /= this.magicNumber;
        }
    }
```

## IntangiblePlayerPower
File: `powers\IntangiblePlayerPower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type) (L36-42)

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

## IntangiblePower
File: `powers\IntangiblePower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type) (L38-44)

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

## Ironclad
File: `characters\Ironclad.java`

### damage(DamageInfo info) (L270-278)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output - this.currentBlock > 0) {
            AnimationState.TrackEntry e = this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTimeScale(0.6f);
        }
        super.damage(info);
    }
```

## Lagavulin
File: `monsters\exordium\Lagavulin.java`

### damage(DamageInfo info) (L187-200)

```java
@Override
    public void damage(DamageInfo info) {
        int previousHealth = this.currentHealth;
        super.damage(info);
        if (this.currentHealth != previousHealth && !this.isOutTriggered) {
            this.setMove((byte)4, AbstractMonster.Intent.STUN);
            this.createIntent();
            this.isOutTriggered = true;
            AbstractDungeon.actionManager.addToBottom(new ChangeStateAction(this, "OPEN"));
        } else if (this.isOutTriggered && info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle_2", true, 0.0f);
        }
    }
```

## MalleablePower
File: `powers\MalleablePower.java`

### onAttacked(DamageInfo info, int damageAmount) (L59-72)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount < this.owner.currentHealth && damageAmount > 0 && info.owner != null && info.type == DamageInfo.DamageType.NORMAL && info.type != DamageInfo.DamageType.HP_LOSS) {
            this.flash();
            if (this.owner.isPlayer) {
                this.addToTop(new GainBlockAction(this.owner, this.owner, this.amount));
            } else {
                this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
            }
            ++this.amount;
            this.updateDescription();
        }
        return damageAmount;
    }
```

## MindBlast
File: `cards\colorless\MindBlast.java`

### applyPowers() (L38-44)

```java
@Override
    public void applyPowers() {
        this.baseDamage = AbstractDungeon.player.drawPile.size();
        super.applyPowers();
        this.rawDescription = MindBlast.cardStrings.DESCRIPTION + MindBlast.cardStrings.EXTENDED_DESCRIPTION[0];
        this.initializeDescription();
    }
```

### calculateCardDamage(AbstractMonster mo) (L46-51)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        super.calculateCardDamage(mo);
        this.rawDescription = MindBlast.cardStrings.DESCRIPTION + MindBlast.cardStrings.EXTENDED_DESCRIPTION[0];
        this.initializeDescription();
    }
```

## Nemesis
File: `monsters\beyond\Nemesis.java`

### damage(DamageInfo info) (L112-123)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.output > 0 && this.hasPower("Intangible")) {
            info.output = 1;
        }
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            AnimationState.TrackEntry e = this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTimeScale(0.8f);
        }
        super.damage(info);
    }
```

## Normality
File: `cards\curses\Normality.java`

### applyPowers() (L36-41)

```java
@Override
    public void applyPowers() {
        super.applyPowers();
        this.rawDescription = AbstractDungeon.actionManager.cardsPlayedThisTurn.size() == 0 ? Normality.cardStrings.EXTENDED_DESCRIPTION[1] + 3 + Normality.cardStrings.EXTENDED_DESCRIPTION[2] : (AbstractDungeon.actionManager.cardsPlayedThisTurn.size() == 1 ? Normality.cardStrings.EXTENDED_DESCRIPTION[1] + 3 + Normality.cardStrings.EXTENDED_DESCRIPTION[3] + AbstractDungeon.actionManager.cardsPlayedThisTurn.size() + Normality.cardStrings.EXTENDED_DESCRIPTION[4] : Normality.cardStrings.EXTENDED_DESCRIPTION[1] + 3 + Normality.cardStrings.EXTENDED_DESCRIPTION[3] + AbstractDungeon.actionManager.cardsPlayedThisTurn.size() + Normality.cardStrings.EXTENDED_DESCRIPTION[5]);
        this.initializeDescription();
    }
```

## OrbWalker
File: `monsters\beyond\OrbWalker.java`

### damage(DamageInfo info) (L109-116)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## PenNibPower
File: `powers\PenNibPower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L46-52)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

## PerfectedStrike
File: `cards\red\PerfectedStrike.java`

### calculateCardDamage(AbstractMonster mo) (L55-62)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        int realBaseDamage = this.baseDamage;
        this.baseDamage += this.magicNumber * PerfectedStrike.countCards();
        super.calculateCardDamage(mo);
        this.baseDamage = realBaseDamage;
        this.isDamageModified = this.damage != this.baseDamage;
    }
```

### applyPowers() (L64-71)

```java
@Override
    public void applyPowers() {
        int realBaseDamage = this.baseDamage;
        this.baseDamage += this.magicNumber * PerfectedStrike.countCards();
        super.applyPowers();
        this.baseDamage = realBaseDamage;
        this.isDamageModified = this.damage != this.baseDamage;
    }
```

## ReactivePower
File: `powers\ReactivePower.java`

### onAttacked(DamageInfo info, int damageAmount) (L35-42)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 0 && damageAmount < this.owner.currentHealth) {
            this.flash();
            this.addToBot(new RollMoveAction((AbstractMonster)this.owner));
        }
        return damageAmount;
    }
```

## Reptomancer
File: `monsters\beyond\Reptomancer.java`

### damage(DamageInfo info) (L141-148)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hurt", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## RitualDagger
File: `cards\colorless\RitualDagger.java`

### applyPowers() (L32-37)

```java
@Override
    public void applyPowers() {
        this.baseBlock = this.misc;
        super.applyPowers();
        this.initializeDescription();
    }
```

## Sentry
File: `monsters\exordium\Sentry.java`

### damage(DamageInfo info) (L107-114)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "hit", false);
            this.state.addAnimation(0, "idle", true, 0.0f);
        }
    }
```

## ShelledParasite
File: `monsters\city\ShelledParasite.java`

### damage(DamageInfo info) (L156-163)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## ShiftingPower
File: `powers\ShiftingPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L31-41)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, -damageAmount), -damageAmount));
            if (!this.owner.hasPower("Artifact")) {
                this.addToTop(new ApplyPowerAction(this.owner, this.owner, new GainStrengthPower(this.owner, damageAmount), damageAmount));
            }
            this.flash();
        }
        return damageAmount;
    }
```

## SlimeBoss
File: `monsters\exordium\SlimeBoss.java`

### damage(DamageInfo info) (L161-171)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (!this.isDying && (float)this.currentHealth <= (float)this.maxHealth / 2.0f && this.nextMove != 3) {
            logger.info("SPLIT");
            this.setMove(SPLIT_NAME, (byte)3, AbstractMonster.Intent.UNKNOWN);
            this.createIntent();
            AbstractDungeon.actionManager.addToBottom(new TextAboveCreatureAction((AbstractCreature)this, TextAboveCreatureAction.TextType.INTERRUPTED));
            AbstractDungeon.actionManager.addToBottom(new SetMoveAction((AbstractMonster)this, SPLIT_NAME, 3, AbstractMonster.Intent.UNKNOWN));
        }
    }
```

## SlowPower
File: `powers\SlowPower.java`

### atDamageReceive(float damage, DamageInfo.DamageType type) (L52-58)

```java
@Override
    public float atDamageReceive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * (1.0f + (float)this.amount * 0.1f);
        }
        return damage;
    }
```

## SnakeDagger
File: `monsters\beyond\SnakeDagger.java`

### damage(DamageInfo info) (L73-82)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hurt", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            this.stateData.setMix("Hurt", "Idle", 0.1f);
            this.stateData.setMix("Idle", "Hurt", 0.1f);
        }
    }
```

## SnakePlant
File: `monsters\city\SnakePlant.java`

### damage(DamageInfo info) (L77-84)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Snecko
File: `monsters\city\Snecko.java`

### damage(DamageInfo info) (L128-135)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## SphericGuardian
File: `monsters\city\SphericGuardian.java`

### damage(DamageInfo info) (L129-137)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.setTimeScale(0.8f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## SpikeSlime_L
File: `monsters\exordium\SpikeSlime_L.java`

### damage(DamageInfo info) (L124-134)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (!this.isDying && (float)this.currentHealth <= (float)this.maxHealth / 2.0f && this.nextMove != 3 && !this.splitTriggered) {
            this.setMove(SPLIT_NAME, (byte)3, AbstractMonster.Intent.UNKNOWN);
            this.createIntent();
            AbstractDungeon.actionManager.addToBottom(new TextAboveCreatureAction((AbstractCreature)this, TextAboveCreatureAction.TextType.INTERRUPTED));
            AbstractDungeon.actionManager.addToBottom(new SetMoveAction((AbstractMonster)this, SPLIT_NAME, 3, AbstractMonster.Intent.UNKNOWN));
            this.splitTriggered = true;
        }
    }
```

## SpireGrowth
File: `monsters\beyond\SpireGrowth.java`

### damage(DamageInfo info) (L115-123)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hurt", false);
            this.state.setTimeScale(1.3f);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## SpireShield
File: `monsters\ending\SpireShield.java`

### damage(DamageInfo info) (L148-155)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## SpireSpear
File: `monsters\ending\SpireSpear.java`

### damage(DamageInfo info) (L154-162)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            AnimationState.TrackEntry e = this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTimeScale(0.7f);
        }
    }
```

## SpiritShield
File: `cards\purple\SpiritShield.java`

### applyPowers() (L31-42)

```java
@Override
    public void applyPowers() {
        int count = 0;
        for (AbstractCard c : AbstractDungeon.player.hand.group) {
            if (c == this) continue;
            ++count;
        }
        this.baseBlock = count * this.magicNumber;
        super.applyPowers();
        this.rawDescription = SpiritShield.cardStrings.DESCRIPTION + SpiritShield.cardStrings.EXTENDED_DESCRIPTION[0];
        this.initializeDescription();
    }
```

## Stack
File: `cards\blue\Stack.java`

### applyPowers() (L32-42)

```java
@Override
    public void applyPowers() {
        this.baseBlock = AbstractDungeon.player.discardPile.size();
        if (this.upgraded) {
            this.baseBlock += 3;
        }
        super.applyPowers();
        this.rawDescription = !this.upgraded ? Stack.cardStrings.DESCRIPTION : Stack.cardStrings.UPGRADE_DESCRIPTION;
        this.rawDescription = this.rawDescription + Stack.cardStrings.EXTENDED_DESCRIPTION[0];
        this.initializeDescription();
    }
```

## StaticDischargePower
File: `powers\StaticDischargePower.java`

### onAttacked(DamageInfo info, int damageAmount) (L28-37)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner && damageAmount > 0) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                this.addToTop(new ChannelAction(new Lightning()));
            }
        }
        return damageAmount;
    }
```

## StrengthPower
File: `powers\StrengthPower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L88-94)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage + (float)this.amount;
        }
        return damage;
    }
```

## TheGuardian
File: `monsters\exordium\TheGuardian.java`

### damage(DamageInfo info) (L268-285)

```java
@Override
    public void damage(DamageInfo info) {
        int tmpHealth = this.currentHealth;
        super.damage(info);
        if (this.isOpen && !this.closeUpTriggered && tmpHealth > this.currentHealth && !this.isDying) {
            this.dmgTaken += tmpHealth - this.currentHealth;
            if (this.getPower("Mode Shift") != null) {
                this.getPower((String)"Mode Shift").amount -= tmpHealth - this.currentHealth;
                this.getPower("Mode Shift").updateDescription();
            }
            if (this.dmgTaken >= this.dmgThreshold) {
                this.dmgTaken = 0;
                AbstractDungeon.actionManager.addToBottom(new VFXAction(this, new IntenseZoomEffect(this.hb.cX, this.hb.cY, false), 0.05f, true));
                AbstractDungeon.actionManager.addToBottom(new ChangeStateAction(this, DEFENSIVE_MODE));
                this.closeUpTriggered = true;
            }
        }
    }
```

## TheSilent
File: `characters\TheSilent.java`

### damage(DamageInfo info) (L278-286)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output - this.currentBlock > 0) {
            AnimationState.TrackEntry e = this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTimeScale(0.9f);
        }
        super.damage(info);
    }
```

## ThornsPower
File: `powers\ThornsPower.java`

### onAttacked(DamageInfo info, int damageAmount) (L44-51)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner) {
            this.flash();
            this.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true));
        }
        return damageAmount;
    }
```

## ThunderStrike
File: `cards\blue\ThunderStrike.java`

### applyPowers() (L42-55)

```java
@Override
    public void applyPowers() {
        super.applyPowers();
        this.baseMagicNumber = 0;
        this.magicNumber = 0;
        for (AbstractOrb o : AbstractDungeon.actionManager.orbsChanneledThisCombat) {
            if (!(o instanceof Lightning)) continue;
            ++this.baseMagicNumber;
        }
        if (this.baseMagicNumber > 0) {
            this.rawDescription = ThunderStrike.cardStrings.DESCRIPTION + ThunderStrike.cardStrings.EXTENDED_DESCRIPTION[0];
            this.initializeDescription();
        }
    }
```

### calculateCardDamage(AbstractMonster mo) (L63-70)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
        super.calculateCardDamage(mo);
        if (this.baseMagicNumber > 0) {
            this.rawDescription = ThunderStrike.cardStrings.DESCRIPTION + ThunderStrike.cardStrings.EXTENDED_DESCRIPTION[0];
        }
        this.initializeDescription();
    }
```

## TimeEater
File: `monsters\beyond\TimeEater.java`

### damage(DamageInfo info) (L158-165)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## Torii
File: `relics\Torii.java`

### onAttacked(DamageInfo info, int damageAmount) (L25-33)

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 1 && damageAmount <= 5) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            return 1;
        }
        return damageAmount;
    }
```

## Transient
File: `monsters\beyond\Transient.java`

### damage(DamageInfo info) (L80-87)

```java
@Override
    public void damage(DamageInfo info) {
        super.damage(info);
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hurt", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
    }
```

## VigorPower
File: `powers\watcher\VigorPower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L36-42)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage += (float)this.amount;
        }
        return damage;
    }
```

## VulnerablePower
File: `powers\VulnerablePower.java`

### atDamageReceive(float damage, DamageInfo.DamageType type) (L57-69)

```java
@Override
    public float atDamageReceive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            if (this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom")) {
                return damage * 1.25f;
            }
            if (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Frog")) {
                return damage * 1.75f;
            }
            return damage * 1.5f;
        }
        return damage;
    }
```

## Watcher
File: `characters\Watcher.java`

### damage(DamageInfo info) (L312-320)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output - this.currentBlock > 0 && this.atlas != null) {
            AnimationState.TrackEntry e = this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
            e.setTime(0.9f);
        }
        super.damage(info);
    }
```

## WeakPower
File: `powers\WeakPower.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L57-66)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            if (!this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane")) {
                return damage * 0.6f;
            }
            return damage * 0.75f;
        }
        return damage;
    }
```

## Wish
File: `cards\purple\Wish.java`

### applyPowers() (L46-48)

```java
@Override
    public void applyPowers() {
    }
```

### calculateCardDamage(AbstractMonster mo) (L50-52)

```java
@Override
    public void calculateCardDamage(AbstractMonster mo) {
    }
```

## WrathStance
File: `stances\WrathStance.java`

### atDamageGive(float damage, DamageInfo.DamageType type) (L32-38)

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

### atDamageReceive(float damage, DamageInfo.DamageType type) (L40-46)

```java
@Override
    public float atDamageReceive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

## WrithingMass
File: `monsters\beyond\WrithingMass.java`

### damage(DamageInfo info) (L119-126)

```java
@Override
    public void damage(DamageInfo info) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.output > 0) {
            this.state.setAnimation(0, "Hit", false);
            this.state.addAnimation(0, "Idle", true, 0.0f);
        }
        super.damage(info);
    }
```

