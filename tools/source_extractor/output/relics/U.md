# Relics: U

1 relics

## UnceasingTop
File: `relics\UnceasingTop.java`

### atPreBattle()

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.canDraw = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `UnceasingTop` — `new UnceasingTop()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new UnceasingTop();
    }
```

</details>

