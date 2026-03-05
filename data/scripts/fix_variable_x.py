#!/usr/bin/env python3
"""
Phase 7.3 Step 5: Fix Variable X Logic
修复 events_master.json 中标记为 manual_review_needed 的变量 X 逻辑

根据 StS Wiki 的准确公式定义每个变量
"""

import json
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent
MASTER_FILE = DATA_DIR / 'events_master.json'

# 变量 X 的精确定义
# 格式: event_id -> option_index -> { formula, 其他属性 }
VARIABLE_X_DEFINITIONS = {
    "face_trader": {
        0: {  # [Touch]
            "formula": {
                "type": "calculate",
                "expression": "floor(current_hp * 0.10)",
                "description": "X = 10% of current HP"
            },
            "costs": {"hp_percent_current": 10},
            "rewards": {"gold": 75, "gold_ascension": 50}
        }
    },
    
    "golden_idol_event": {
        2: {  # [Smash]
            "formula": {
                "type": "calculate",
                "expression": "floor(max_hp * 0.25) or floor(max_hp * 0.35) on A15+",
                "description": "X = 25% (A15+: 35%) of Max HP"
            },
            "costs": {"max_hp_percent": 25, "max_hp_percent_ascension": 35}
        },
        3: {  # [Hide]
            "formula": {
                "type": "calculate",
                "expression": "floor(max_hp * 0.08) or floor(max_hp * 0.10) on A15+",
                "description": "X = 8% (A15+: 10%) of Max HP, min 5 HP, max 40 HP"
            },
            "costs": {"damage_percent": 8, "damage_percent_ascension": 10, "damage_min": 5, "damage_max": 40}
        }
    },
    
    "hypnotizing_colored_mushrooms": {
        1: {  # [Eat]
            "formula": {
                "type": "calculate",
                "expression": "random(1, 5)",
                "description": "X = random 1-5 cards upgraded"
            },
            "rewards": {"upgrade_cards": {"type": "random", "min": 1, "max": 5}}
        }
    },
    
    "shining_light": {
        0: {  # [Enter]
            "formula": {
                "type": "calculate",
                "expression": "floor(max_hp * 0.20) or floor(max_hp * 0.30) on A15+",
                "description": "X = 20% (A15+: 30%) of Max HP"
            },
            "costs": {"hp_percent": 20, "hp_percent_ascension": 30},
            "rewards": {"upgrade_cards": 2}
        }
    },
    
    "the_cleric": {
        0: {  # [Heal]
            "formula": {
                "type": "fixed",
                "value": {"base": 35, "ascension": 50},
                "description": "Heal costs 35 (A15+: 50) Gold"
            },
            "costs": {"gold": 35, "gold_ascension": 50},
            "rewards": {"heal_percent": 25}
        }
    },
    
    "world_of_goop": {
        1: {  # [Leave It]
            "formula": {
                "type": "random",
                "range": {"min": 20, "max": 50},
                "description": "X = random 20-50 Gold lost"
            },
            "costs": {"gold": {"type": "random", "min": 20, "max": 50}}
        }
    },
    
    "augmenter": {
        0: {  # [Test J.A.X.]
            # J.A.X. card has variable X in its effect, not the event
            "formula": {
                "type": "card_effect",
                "description": "J.A.X. card adds X Strength where X is its upgrade level"
            },
            "rewards": {"card": "J.A.X."}
        }
    },
    
    "council_of_ghosts": {
        0: {  # [Accept]
            "formula": {
                "type": "calculate",
                "expression": "ceil(max_hp * 0.50)",
                "description": "X = 50% of Max HP (rounded up)"
            },
            "costs": {"max_hp_percent": 50, "round": "ceil"},
            "rewards": {"cards": {"card_id": "Apparition", "count": 5, "count_ascension": 3}}
        }
    },
    
    "forgotten_altar": {
        1: {  # [Sacrifice]
            "formula": {
                "type": "calculate",
                "expression": "floor(max_hp * 0.25) or floor(max_hp * 0.35) on A15+",
                "description": "X = 25% (A15+: 35%) of Max HP"
            },
            "costs": {"hp_percent": 25, "hp_percent_ascension": 35},
            "rewards": {"max_hp": 5}
        }
    },
    
    "the_library": {
        1: {  # [Sleep]
            "formula": {
                "type": "calculate",
                "expression": "ceil(max_hp * 0.25) or ceil(max_hp * 0.35) on A15+",
                "description": "X = 25% (A15+: 35%) of Max HP (rounded up)"
            },
            "rewards": {"heal_percent": 25, "heal_percent_ascension": 35, "round": "ceil"}
        }
    },
    
    "vampires": {
        1: {  # [Accept]
            "formula": {
                "type": "calculate", 
                "expression": "floor(max_hp * 0.30)",
                "description": "X = 30% of Max HP"
            },
            "costs": {"max_hp_percent": 30},
            "rewards": {
                "remove_all": "Strike",
                "cards": {"card_id": "Bite", "count": 5}
            }
        }
    },
    
    "the_moai_head": {
        0: {  # [Jump Inside]
            "formula": {
                "type": "calculate",
                "expression": "clamp(floor(max_hp * 0.125), 5, 50)",
                "description": "X = 12.5% of Max HP, minimum 5, maximum 50"
            },
            "costs": {"hp_percent": 12.5, "hp_min": 5, "hp_max": 50}
        }
    },
    
    "tomb_of_lord_red_mask": {
        1: {  # [Offer: X Gold]
            "formula": {
                "type": "fixed",
                "value": {"amount": 0},  # If player doesn't have Red Mask, costs all gold
                "description": "X = All Gold (if no Red Mask)"
            },
            "costs": {"gold": "all"},
            "requirements": {"not_has_relic": "Red Mask"}
        }
    },
    
    "winding_halls": {
        0: {  # [Embrace Madness]
            "formula": {
                "type": "calculate",
                "expression": "floor(current_hp * 0.10) or floor(current_hp * 0.15) on A15+",
                "description": "X = 10% (A15+: 15%) of current HP"
            },
            "costs": {"hp_percent_current": 10, "hp_percent_current_ascension": 15},
            "rewards": {"card": {"card_id": "Madness", "count": 2}}
        },
        1: {  # [Focus]
            "formula": {
                "type": "calculate",
                "expression": "floor(current_hp * 0.10) or floor(current_hp * 0.15) on A15+",
                "description": "X = 10% (A15+: 15%) of current HP"
            },
            "costs": {"hp_percent_current": 10, "hp_percent_current_ascension": 15},
            "rewards": {"upgrade_cards": 2}
        },
        2: {  # [Retrace Your Steps]
            "formula": {
                "type": "calculate",
                "expression": "floor(current_hp * 0.10) or floor(current_hp * 0.15) on A15+",
                "description": "X = 10% (A15+: 15%) of current HP"
            },
            "costs": {"hp_percent_current": 10, "hp_percent_current_ascension": 15}
        }
    }
}


def apply_variable_fixes(data):
    """应用变量 X 的精确定义"""
    fixes_applied = 0
    
    for event_id, fixes in VARIABLE_X_DEFINITIONS.items():
        if event_id not in data:
            print(f"  ⚠️ Event '{event_id}' not found in master file")
            continue
        
        event = data[event_id]
        options = event.get('options', [])
        
        for opt_index, fix_data in fixes.items():
            if opt_index >= len(options):
                print(f"  ⚠️ Option {opt_index} not found in '{event_id}'")
                continue
            
            option = options[opt_index]
            
            # 添加 formula 定义
            if 'formula' in fix_data:
                option['variable_x'] = fix_data['formula']
            
            # 更新 costs
            if 'costs' in fix_data:
                if 'costs' not in option:
                    option['costs'] = {}
                option['costs'].update(fix_data['costs'])
            
            # 更新 rewards
            if 'rewards' in fix_data:
                if 'rewards' not in option:
                    option['rewards'] = {}
                option['rewards'].update(fix_data['rewards'])
            
            # 更新 requirements
            if 'requirements' in fix_data:
                if 'requirements' not in option:
                    option['requirements'] = {}
                option['requirements'].update(fix_data['requirements'])
            
            # 移除 manual_review_needed 标记
            if 'manual_review_needed' in option:
                del option['manual_review_needed']
            if 'review_note' in option:
                del option['review_note']
            
            fixes_applied += 1
            print(f"  ✅ Fixed: {event_id} / {option.get('label', f'Option {opt_index}')}")
    
    return data, fixes_applied


def clean_todo_conditions(data):
    """清理 TODO_PARSE 条件，保留有意义的文本作为 notes"""
    cleaned = 0
    
    for event_id, event in data.items():
        for option in event.get('options', []):
            if 'conditions' not in option:
                continue
            
            new_conditions = []
            notes = []
            
            for cond in option['conditions']:
                if isinstance(cond, dict) and cond.get('type') == 'TODO_PARSE':
                    # 将无法解析的条件转为 notes
                    text = cond.get('text', '')
                    if text:
                        notes.append(text)
                    cleaned += 1
                else:
                    new_conditions.append(cond)
            
            if new_conditions:
                option['conditions'] = new_conditions
            else:
                del option['conditions']
            
            if notes:
                if 'notes' not in option:
                    option['notes'] = []
                option['notes'].extend(notes)
    
    return data, cleaned


def main():
    print("Loading events_master.json...")
    with open(MASTER_FILE, 'r', encoding='utf-8') as f:
        data = json.load(f)
    
    print(f"\nApplying Variable X fixes to {len(VARIABLE_X_DEFINITIONS)} events...")
    data, fixes_applied = apply_variable_fixes(data)
    
    print(f"\nCleaning TODO_PARSE conditions...")
    data, cleaned = clean_todo_conditions(data)
    
    # 统计剩余的 manual_review_needed
    remaining_reviews = 0
    for event in data.values():
        for opt in event.get('options', []):
            if opt.get('manual_review_needed'):
                remaining_reviews += 1
    
    # 保存
    with open(MASTER_FILE, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    
    print("\n" + "="*50)
    print("✅ Variable X Logic Fixed!")
    print("="*50)
    print(f"  Fixes applied: {fixes_applied}")
    print(f"  TODO conditions cleaned: {cleaned}")
    print(f"  Remaining manual reviews: {remaining_reviews}")
    print(f"\n📁 Updated: {MASTER_FILE}")


if __name__ == "__main__":
    main()
