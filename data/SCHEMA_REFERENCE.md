# JSON Schema Reference for monsters_with_behavior.json

## Condition Types (所有条件类型及其字段)

### 1. HpThreshold
```json
{"type": "HpThreshold", "percent": 50, "comparison": "<="}
```
- `percent`: i32
- `comparison`: String ("<=", "<", ">=", ">")

### 2. HistoryConstraint  
```json
{"type": "HistoryConstraint", "max_in_row": 2}
```
- `max_in_row`: i32

### 3. Turn
```json
{"type": "Turn", "comparison": "==", "target": "1"}
```
- `comparison`: String ("==")
- `target`: String (turn number as string!)

### 4. ChargingPattern
```json
{"type": "ChargingPattern", "max_in_row": 2}
```
- `max_in_row`: i32

### 5. AfterCharging
```json
{"type": "AfterCharging"}
```
- No additional fields

### 6. MoveNotUsedYet
```json
{"type": "MoveNotUsedYet", "target": "Entangle"}
```
- `target`: String (move name)

### 7. UntilUsed
```json
{"type": "UntilUsed", "target": "Entangle"}
```
- `target`: String (move name)

### 8. AlliesExist
```json
{"type": "AlliesExist"}
```
- No additional fields

### 9. NoAllies
```json
{"type": "NoAllies"}
```
- No additional fields

### 10. AllyAlive
```json
{"type": "AllyAlive", "target": "Mystic"}
```
- `target`: String (ally name)

### 11. EveryNTurns
```json
{"type": "EveryNTurns", "comparison": "start=4", "target": "4"}
```
- `comparison`: String (format: "start=N")
- `target`: String (every N turns as string!)

## Logic Types (所有逻辑类型)

1. **Probabilistic** - 带权重的随机选择 + 条件约束
2. **Cycle** - 固定循环序列
3. **Alternating** - 两招交替
4. **Repeat** - 始终同一招
5. **Charging** - 蓄力 N 回合后攻击
6. **EveryNTurns** - 每 N 回合使用特定招式
7. **Turn1ThenAlternate** - 第一回合特殊，之后交替
8. **Conditional** - 复杂条件逻辑
9. **MultiPhase** - 多阶段 Boss
10. **Reference** - 引用其他怪物的 AI

## BehaviorModel 结构

```json
{
  "logic_type": "Probabilistic",
  "init_sequence": ["Incantation"],  // optional
  "cycle": ["Move1", "Move2"],       // optional, for Cycle/Alternating
  "rules": [...],                    // optional
  "phases": [...],                   // optional, for MultiPhase
  "notes": "..."                     // optional
}
```

## BehaviorRule 结构

```json
{
  "move": "Dark Strike",
  "weight": 100,        // optional, for weighted random
  "priority": 10,       // optional, higher = checked first
  "conditions": [...]   // optional
}
```

## BehaviorPhase 结构 (for MultiPhase)

```json
{
  "phase": 1,
  "init_sequence": ["Slash"],
  "rules": [...],
  "transition": {
    "trigger": "HpZero",
    "uses": "Rebirth"
  }
}
```

## 关键注意事项

1. **Turn.target 和 EveryNTurns.target 是 STRING 类型，不是 i32**
2. **EveryNTurns.comparison 格式是 "start=N"，需要解析**
3. **AllyAlive 用 "target" 字段，不是 "ally"**
4. **所有 weight/priority 都是 optional**
