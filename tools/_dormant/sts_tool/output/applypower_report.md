# ApplyPowerAction

**File**: `actions\common\ApplyPowerAction.java`
**Category**: action
**Extends**: `AbstractGameAction`

## Method: `update()`
Lines 89–186

### Structured Logic

- **IF** `(this.target == null || this.target.isDeadOrEscaped())`:
  - `this.isDone = true;`
  - **RETURN** `return;`
- **IF** `(this.duration == this.startingDuration)`:
  - **IF** `(this.powerToApply instanceof NoDrawPower && this.target.hasPower(this.powerToApply.ID))`:
    - `this.isDone = true;`
    - **RETURN** `return;`
  - **IF** `(this.source != null)`:
    - **FOR EACH** `AbstractPower abstractPower : this.source.powers)`:
      - `abstractPower.onApplyPower(this.powerToApply, this.target, this.source);`
  - **IF** `(AbstractDungeon.player.hasRelic("Champion Belt") && this.source != null && this.source.isPlayer && this.target != this.source && this.powerToApply.ID.equals("Vulnerable") && !this.target.hasPower("Artifact"))`:
    - `AbstractDungeon.player.getRelic("Champion Belt").onTrigger(this.target);`
  - **IF** `(this.target instanceof AbstractMonster && this.target.isDeadOrEscaped())`:
    - `this.duration = 0.0f;`
    - `this.isDone = true;`
    - **RETURN** `return;`
  - **IF** `(AbstractDungeon.player.hasRelic("Ginger") && this.target.isPlayer && this.powerToApply.ID.equals("Weakened"))`:
    - `AbstractDungeon.player.getRelic("Ginger").flash();`
    - `this.addToTop(new TextAboveCreatureAction(this.target, TEXT[1]));`
    - `this.duration -= Gdx.graphics.getDeltaTime();`
    - **RETURN** `return;`
  - **IF** `(AbstractDungeon.player.hasRelic("Turnip") && this.target.isPlayer && this.powerToApply.ID.equals("Frail"))`:
    - `AbstractDungeon.player.getRelic("Turnip").flash();`
    - `this.addToTop(new TextAboveCreatureAction(this.target, TEXT[1]));`
    - `this.duration -= Gdx.graphics.getDeltaTime();`
    - **RETURN** `return;`
  - **IF** `(this.target.hasPower("Artifact") && this.powerToApply.type == AbstractPower.PowerType.DEBUFF)`:
    - `this.addToTop(new TextAboveCreatureAction(this.target, TEXT[0]));`
    - `this.duration -= Gdx.graphics.getDeltaTime();`
    - `CardCrawlGame.sound.play("NULLIFY_SFX");`
    - `this.target.getPower("Artifact").flashWithoutSound();`
    - `this.target.getPower("Artifact").onSpecificTrigger();`
    - **RETURN** `return;`
  - `AbstractDungeon.effectList.add(new FlashAtkImgEffect(this.target.hb.cX, this.target.hb.cY, this.attackEffect));`
  - **VAR** `boolean hasBuffAlready = false;`
  - **FOR EACH** `AbstractPower p : this.target.powers)`:
    - **IF** `(!p.ID.equals(this.powerToApply.ID) || p.ID.equals("Night Terror"))`:
      - **CONTINUE**
    - `p.stackPower(this.amount);`
    - `p.flash();`
    - **IF** `((p instanceof StrengthPower || p instanceof DexterityPower) && this.amount <= 0)`:
      - `AbstractDungeon.effectList.add(new PowerDebuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name + TEXT[3]));`
    - **ELSE IF** `(this.amount > 0)`:
      - **IF** `(p.type == AbstractPower.PowerType.BUFF || p instanceof StrengthPower || p instanceof DexterityPower)`:
        - `AbstractDungeon.effectList.add(new PowerBuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, "+" + Integer.toString(this.amount) + " " + this.powerToApply.name));`
      - **ELSE**:
        - `AbstractDungeon.effectList.add(new PowerDebuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, "+" + Integer.toString(this.amount) + " " + this.powerToApply.name));`
    - **ELSE IF** `(p.type == AbstractPower.PowerType.BUFF)`:
      - `AbstractDungeon.effectList.add(new PowerBuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name + TEXT[3]));`
    - **ELSE**:
      - `AbstractDungeon.effectList.add(new PowerDebuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name + TEXT[3]));`
    - `p.updateDescription();`
    - `hasBuffAlready = true;`
    - `AbstractDungeon.onModifyPower();`
  - **IF** `(this.powerToApply.type == AbstractPower.PowerType.DEBUFF)`:
    - `this.target.useFastShakeAnimation(0.5f);`
  - **IF** `(!hasBuffAlready)`:
    - `this.target.powers.add(this.powerToApply);`
    - `Collections.sort(this.target.powers);`
    - `this.powerToApply.onInitialApplication();`
    - `this.powerToApply.flash();`
    - **IF** `(this.amount < 0 && (this.powerToApply.ID.equals("Strength") || this.powerToApply.ID.equals("Dexterity") || this.powerToApply.ID.equals("Focus")))`:
      - `AbstractDungeon.effectList.add(new PowerDebuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name + TEXT[3]));`
    - **ELSE IF** `(this.powerToApply.type == AbstractPower.PowerType.BUFF)`:
      - `AbstractDungeon.effectList.add(new PowerBuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name));`
    - **ELSE**:
      - `AbstractDungeon.effectList.add(new PowerDebuffEffect(this.target.hb.cX - this.target.animX, this.target.hb.cY + this.target.hb.height / 2.0f, this.powerToApply.name));`
    - `AbstractDungeon.onModifyPower();`
    - **IF** `(this.target.isPlayer)`:
      - **VAR** `void var2_6;`
      - **VAR** `boolean bl = false;`
      - **FOR EACH** `AbstractPower p : this.target.powers)`:
        - **IF** `(p.type != AbstractPower.PowerType.BUFF)`:
          - **CONTINUE**
        - `++var2_6;`
      - **IF** `(var2_6 >= 10)`:
        - `UnlockTracker.unlockAchievement("POWERFUL");`
- `this.tickDuration();`

### Call Chain

```
ApplyPowerAction.update()
  ├─ guard: (this.target == null || this.target.isDeadOrEscaped())
  ├─ guard: (this.duration == this.startingDuration)
  ├─ checks: hasRelic("Champion Belt") L105
  ├─ checks: hasPower("Artifact") L105
  ├─ checks: hasRelic("Ginger") L113
  ├─ checks: hasRelic("Turnip") L119
  ├─ checks: hasPower("Artifact") L125
  ├─ iterates: this.source.powers → AbstractPower.onApplyPower()
  │   ├─ SadisticPower (power) ✅ LIVE
  ├─ iterates: this.target.powers → AbstractPower.stackPower()
  │   ├─ DexterityPower (power) ✅ LIVE
  │   ├─ CollectPower (power) ✅ LIVE
  │   ├─ DEPRECATEDFlickedPower (power) ✅ LIVE
  │   ├─ AccuracyPower (power) ✅ LIVE
  │   ├─ DEPRECATEDCondensePower (power) ✅ LIVE
  │   ├─ BiasPower (power) ✅ LIVE
  │   ├─ BufferPower (power) ✅ LIVE
  │   ├─ CombustPower (power) ✅ LIVE
  │   ├─ RegenPower (power) ✅ LIVE
  │   ├─ PanachePower (power) ✅ LIVE
  │   ├─ ... +22 more
  │   ├─ StrikeUpPower ⚠️ DEAD
  │   ├─ WinterPower ⚠️ DEAD
  ├─ calls: AbstractDungeon.player.getRelic("Ginger").flash() L114
  ├─ calls: AbstractDungeon.player.getRelic("Turnip").flash() L120
  ├─ calls: this.powerToApply.onInitialApplication() L162
  ├─ calls: this.powerToApply.flash() L163
```

## Rust Parity

✅ `ApplyPowerAction` → `ApplyPower`
