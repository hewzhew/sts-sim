# GainBlockAction

**File**: `actions\common\GainBlockAction.java`
**Category**: action
**Extends**: `AbstractGameAction`

## Method: `update()`
Lines 46–56

### Structured Logic

- **IF** `(!this.target.isDying && !this.target.isDead && this.duration == this.startDuration)`:
  - `AbstractDungeon.effectList.add(new FlashAtkImgEffect(this.target.hb.cX, this.target.hb.cY, AbstractGameAction.AttackEffect.SHIELD));`
  - `this.target.addBlock(this.amount);`
  - **FOR EACH** `AbstractCard c : AbstractDungeon.player.hand.group)`:
    - `c.applyPowers();`
- `this.tickDuration();`

### Call Chain

```
GainBlockAction.update()
  ├─ iterates: AbstractDungeon.player.hand.group → AbstractCard.applyPowers()
  │   ├─ DamageInfo (other) ✅ LIVE
  │   ├─ CardGroup (other) ✅ LIVE
  │   ├─ Normality (card) ✅ LIVE
  │   ├─ Brilliance (card) ✅ LIVE
  │   ├─ RitualDagger (card) ✅ LIVE
  │   ├─ MindBlast (card) ✅ LIVE
  │   ├─ ThunderStrike (card) ✅ LIVE
  │   ├─ Wish (card) ✅ LIVE
  │   ├─ Stack (card) ✅ LIVE
  │   ├─ PerfectedStrike (card) ✅ LIVE
  │   ├─ ... +9 more
  ├─ calls: this.target.addBlock(this.amount) L50
```

## Rust Parity

✅ `GainBlockAction` → `GainBlock`
