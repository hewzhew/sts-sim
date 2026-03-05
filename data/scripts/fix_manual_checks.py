#!/usr/bin/env python3
"""
Phase 7.3 Final: Fix Manual Check Items
修复剩余的 ManualCheck 条件，将它们转为真正的游戏逻辑

这些是脚本无法自动处理的复杂条件，需要人工理解后硬编码
"""

import json
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent
MASTER_FILE = DATA_DIR / 'events_final_master.json'

# 手动定义的条件修复
# 格式: event_id -> option_index -> new_conditions
MANUAL_FIXES = {
    "big_fish": {
        0: {  # [Banana]
            "remove_conditions": True,
            "add_notes": ["Healing rounds down if Max HP not divisible by 3"],
            "rewards": {"heal_percent": 33.33, "round": "down"}
        },
        1: {  # [Donut]
            "remove_conditions": True,
            "add_notes": ["Gaining Max HP also heals for that amount"],
            "rewards": {"max_hp": 5}
        }
    },
    
    "dead_adventurer": {
        1: {  # [Continue]
            "conditions": [
                {"type": "EventState", "state": "search_completed", "value": True}
            ],
            "visibility": "conditional"
        }
    },
    
    "scrap_ooze": {
        0: {  # [Reach Inside]
            "remove_conditions": True,
            "add_notes": ["Can retry until Relic found or player leaves"],
            "loop_mechanic": {
                "type": "retry_until_success",
                "success_chance": {"base": 0.25, "cumulative": True},
                "on_fail": {"hp_loss": 3, "hp_loss_ascension": 5}
            }
        }
    },
    
    "wing_statue": {
        1: {  # [Destroy]
            "conditions": [
                {"type": "HasCardCost", "min_cost": 1}
            ],
            "locked_if_fail": True
        }
    },
    
    "cursed_tome": {
        # 这是一个多阶段事件，用 phases 处理
        "event_type": "multi_phase",
        "phases": {
            "start": {
                "options": ["[Read]", "[Leave]"]
            },
            "reading_1": {
                "trigger": "[Read]",
                "options": ["[Continue]", "[Stop]"]
            },
            "reading_2": {
                "trigger": "[Continue] (from reading_1)",
                "options": ["[Continue]", "[Stop]"]
            },
            "reading_3": {
                "trigger": "[Continue] (from reading_2)",
                "options": ["[Continue]", "[Stop]"]
            },
            "final": {
                "trigger": "[Continue] (from reading_3)",
                "options": ["[Take]", "[Stop]"]
            }
        }
    },
    
    "pleading_vagrant": {
        0: {  # [Offer Gold]
            "conditions": [
                {"type": "HasGold", "amount": 85}
            ],
            "locked_if_fail": True,
            "locked_text": "Not enough Gold"
        }
    },
    
    "falling": {
        # 特殊选项 [Land]
        3: {  # 假设是第4个选项 [Land]
            "conditions": [
                {"type": "NoCardOfType", "types": ["Skill", "Power", "Attack"]}
            ],
            "visibility": "conditional",
            "note": "Only appears if deck has no removable cards of any type"
        }
    },
    
    "mind_bloom": {
        0: {  # [I am Rich]
            "conditions": [
                {"type": "FloorRange", "min": 35, "max": 40}
            ],
            "visibility": "conditional"
        },
        1: {  # [I am Healthy]
            "conditions": [
                {"type": "FloorRange", "min": 41, "max": 999}
            ],
            "visibility": "conditional"
        }
    },
    
    "sensory_stone": {
        # 根据 Act 决定 Recall 次数
        "act_scaling": {
            "act1": {"recalls": 1},
            "act2": {"recalls": 2},
            "act3": {"recalls": 3}
        }
    }
}


def apply_option_fix(option: dict, fix: dict) -> dict:
    """应用单个选项的修复"""
    
    # 移除旧条件
    if fix.get("remove_conditions"):
        if "conditions" in option:
            del option["conditions"]
    
    # 添加新条件
    if "conditions" in fix:
        option["conditions"] = fix["conditions"]
    
    # 添加 notes
    if "add_notes" in fix:
        if "notes" not in option:
            option["notes"] = []
        option["notes"].extend(fix["add_notes"])
    
    # 更新 rewards
    if "rewards" in fix:
        if "rewards" not in option:
            option["rewards"] = {}
        option["rewards"].update(fix["rewards"])
    
    # 添加 loop_mechanic
    if "loop_mechanic" in fix:
        option["loop_mechanic"] = fix["loop_mechanic"]
    
    # 添加可见性控制
    if "visibility" in fix:
        option["visibility"] = fix["visibility"]
    
    if "locked_if_fail" in fix:
        option["locked_if_fail"] = fix["locked_if_fail"]
    
    if "locked_text" in fix:
        option["locked_text"] = fix["locked_text"]
    
    return option


def apply_event_fix(event: dict, event_id: str, fix: dict) -> dict:
    """应用事件级别的修复"""
    
    # 事件类型
    if "event_type" in fix:
        event["event_type"] = fix["event_type"]
    
    # 阶段定义
    if "phases" in fix:
        event["phases"] = fix["phases"]
    
    # Act 缩放
    if "act_scaling" in fix:
        event["act_scaling"] = fix["act_scaling"]
    
    # 选项级修复
    options = event.get("options", [])
    for opt_idx, opt_fix in fix.items():
        if isinstance(opt_idx, int) and opt_idx < len(options):
            options[opt_idx] = apply_option_fix(options[opt_idx], opt_fix)
    
    return event


def clean_manual_checks(event: dict) -> dict:
    """清理剩余的 ManualCheck，转为 notes"""
    for opt in event.get("options", []):
        if "conditions" not in opt:
            continue
        
        new_conditions = []
        for cond in opt["conditions"]:
            if isinstance(cond, dict) and cond.get("type") == "ManualCheck":
                # 转为 note
                text = cond.get("text", "")
                if text:
                    if "notes" not in opt:
                        opt["notes"] = []
                    opt["notes"].append(f"[UNSTRUCTURED] {text}")
            else:
                new_conditions.append(cond)
        
        if new_conditions:
            opt["conditions"] = new_conditions
        elif "conditions" in opt:
            del opt["conditions"]
    
    return event


def main():
    print("Loading events_final_master.json...")
    with open(MASTER_FILE, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    fixes_applied = 0
    
    print("\nApplying manual fixes...")
    for event_id, fix in MANUAL_FIXES.items():
        if event_id not in data:
            print(f"  ⚠️ Event '{event_id}' not found")
            continue
        
        data[event_id] = apply_event_fix(data[event_id], event_id, fix)
        print(f"  ✅ Fixed: {event_id}")
        fixes_applied += 1
    
    print("\nCleaning remaining ManualCheck items...")
    cleaned = 0
    for event_id, event in data.items():
        before = str(event)
        event = clean_manual_checks(event)
        if str(event) != before:
            cleaned += 1
    
    # 保存
    with open(MASTER_FILE, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    
    # 统计剩余的 ManualCheck
    remaining = 0
    for event in data.values():
        for opt in event.get("options", []):
            for cond in opt.get("conditions", []):
                if isinstance(cond, dict) and cond.get("type") == "ManualCheck":
                    remaining += 1
    
    print("\n" + "="*50)
    print("✅ Manual Fixes Applied!")
    print("="*50)
    print(f"  Events fixed: {fixes_applied}")
    print(f"  Items cleaned: {cleaned}")
    print(f"  Remaining ManualCheck: {remaining}")
    print(f"\n📁 Updated: {MASTER_FILE}")


if __name__ == "__main__":
    main()
