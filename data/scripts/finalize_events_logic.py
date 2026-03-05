#!/usr/bin/env python3
"""
Phase 7.3 Final: Deep Logic Cleaner
将所有自然语言条件/效果转换为机器可执行的逻辑对象

问题：
1. conditions 是字符串 → 需要 {type, value} 对象
2. effects 里的数学公式 → 需要提取到 costs/rewards
3. random_outcomes 是字符串列表 → 需要结构化
4. 复杂交互（下注等）→ 需要专门处理
"""

import json
import re
import os
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent
INPUT_DIR = DATA_DIR / 'events_structured'
OUTPUT_DIR = DATA_DIR / 'events_final'
MASTER_FILE = DATA_DIR / 'events_master.json'

INPUT_FILES = ['act1.json', 'act2.json', 'act3.json', 'shrines.json']

# ============================================================================
# 1. 条件解析器
# ============================================================================

def clean_condition_text(cond_text: str) -> dict:
    """将自然语言条件转换为逻辑对象"""
    text = cond_text.lower()
    original = cond_text
    
    # 1. 金币检查: "Requires: 50 Gold" / "At least 75 Gold" / "Have 35 Gold"
    gold_patterns = [
        r'(?:requires?|at\s+least|have)\s*:?\s*(\d+)\s*gold',
        r'(\d+)\s*gold\s*(?:required|needed)',
    ]
    for pattern in gold_patterns:
        match = re.search(pattern, text)
        if match:
            return {"type": "HasGold", "amount": int(match.group(1))}
    
    # 2. 遗物检查: "Requires: Golden Idol" / "If player has Blood Vial"
    relic_patterns = [
        (r'golden\s*idol', 'Golden Idol'),
        (r'blood\s*vial', 'Blood Vial'),
        (r'red\s*mask', 'Red Mask'),
        (r'necronomicon', 'Necronomicon'),
        (r'circlet', 'Circlet'),
        (r'ectoplasm', 'Ectoplasm'),
        (r'tiny\s*house', 'Tiny House'),
        (r'singing\s*bowl', 'Singing Bowl'),
        (r'ceramic\s*fish', 'Ceramic Fish'),
        (r'shovel', 'Shovel'),
    ]
    for pattern, relic_id in relic_patterns:
        if re.search(pattern, text):
            # 检查是否是"没有"的情况
            if 'not have' in text or 'without' in text or "doesn't have" in text:
                return {"type": "NotHasRelic", "id": relic_id}
            return {"type": "HasRelic", "id": relic_id}
    
    # 3. 卡牌类型检查 (Falling 事件等)
    card_type_patterns = [
        (r'(?:at\s+least\s+one\s+)?skill\s+card', 'Skill'),
        (r'(?:at\s+least\s+one\s+)?power\s+card', 'Power'),
        (r'(?:at\s+least\s+one\s+)?attack\s+card', 'Attack'),
        (r'(?:at\s+least\s+one\s+)?curse', 'Curse'),
    ]
    for pattern, card_type in card_type_patterns:
        if re.search(pattern, text):
            if 'remove' in text or 'lose' in text:
                return {"type": "HasCardType", "card_type": card_type, "action": "remove"}
            return {"type": "HasCardType", "card_type": card_type}
    
    # 4. HP 检查
    hp_patterns = [
        r'(?:requires?|at\s+least|have)\s*:?\s*(\d+)\s*hp',
        r'hp\s*(?:>=?|above|over)\s*(\d+)',
    ]
    for pattern in hp_patterns:
        match = re.search(pattern, text)
        if match:
            return {"type": "HasHP", "amount": int(match.group(1))}
    
    # 5. 楼层检查
    floor_match = re.search(r'floor\s*(\d+)', text)
    if floor_match:
        return {"type": "FloorNum", "value": int(floor_match.group(1))}
    
    # 6. 特殊条件
    if 'potion slot' in text and ('full' in text or 'no' in text):
        return {"type": "PotionSlotsFull", "value": True}
    
    if 'empty' in text and 'potion' in text:
        return {"type": "HasEmptyPotionSlot", "value": True}
    
    # 默认：无法解析，标记为待人工处理
    return {"type": "ManualCheck", "text": original}


def parse_conditions(conditions: List) -> List[dict]:
    """处理整个 conditions 列表"""
    result = []
    for cond in conditions:
        if isinstance(cond, str):
            parsed = clean_condition_text(cond)
            result.append(parsed)
        elif isinstance(cond, dict):
            # 已经是结构化的，检查是否需要进一步处理
            if cond.get('type') == 'TODO_PARSE':
                parsed = clean_condition_text(cond.get('text', ''))
                result.append(parsed)
            else:
                result.append(cond)
    return result


# ============================================================================
# 2. 数值/百分比解析器
# ============================================================================

def parse_percentage_from_text(text: str) -> Optional[Dict[str, float]]:
    """从文本中提取百分比数值"""
    # 匹配 "25% (A15+: 35%)"
    match = re.search(r'(\d+(?:\.\d+)?)%\s*\(A15\+:\s*(\d+(?:\.\d+)?)%\)', text)
    if match:
        return {
            'base': float(match.group(1)),
            'ascension_15': float(match.group(2))
        }
    
    # 匹配单纯的 "25%"
    match = re.search(r'(\d+(?:\.\d+)?)%', text)
    if match:
        return {'base': float(match.group(1))}
    
    return None


def parse_gold_range(text: str) -> Optional[Dict[str, int]]:
    """解析金币范围"""
    # "50 - 80 Gold" 或 "between 20 and 50"
    patterns = [
        r'(\d+)\s*[-–]\s*(\d+)\s*gold',
        r'between\s*(\d+)\s*and\s*(\d+)',
        r'(\d+)\s*to\s*(\d+)\s*gold',
    ]
    for pattern in patterns:
        match = re.search(pattern, text, re.IGNORECASE)
        if match:
            return {'min': int(match.group(1)), 'max': int(match.group(2))}
    return None


def extract_math_from_effects(effects: List[str], opt: dict) -> dict:
    """从 effects 列表中提取数学逻辑并注入到 costs/rewards"""
    if not effects:
        return opt
    
    full_text = ' '.join(effects)
    
    # 初始化
    if 'costs' not in opt:
        opt['costs'] = {}
    if 'rewards' not in opt:
        opt['rewards'] = {}
    
    # 1. HP 百分比损失
    hp_loss_patterns = [
        r'(?:lose|take)\s+(?:X\s+)?(?:HP|damage).*?(\d+(?:\.\d+)?)%\s*\(A15\+:\s*(\d+(?:\.\d+)?)%\)',
        r'X\s*(?:is\s+)?(?:equal\s+to\s+)?(\d+(?:\.\d+)?)%\s*\(A15\+:\s*(\d+(?:\.\d+)?)%\)',
    ]
    for pattern in hp_loss_patterns:
        match = re.search(pattern, full_text, re.IGNORECASE)
        if match:
            opt['costs']['hp_percent'] = float(match.group(1))
            opt['costs']['hp_percent_ascension'] = float(match.group(2))
            # 移除占位符
            if 'hp' in opt['costs'] and opt['costs']['hp'] == 'X':
                del opt['costs']['hp']
            break
    
    # 2. 单纯百分比（无 A15）
    if 'hp_percent' not in opt['costs']:
        simple_match = re.search(r'(\d+(?:\.\d+)?)%\s*of\s*(?:the\s+player\'?s?\s+)?(?:max\s+)?hp', full_text, re.IGNORECASE)
        if simple_match:
            opt['costs']['hp_percent'] = float(simple_match.group(1))
    
    # 3. 金币范围
    gold_range = parse_gold_range(full_text)
    if gold_range:
        opt['rewards']['gold'] = gold_range
    
    # 4. 回血百分比
    heal_match = re.search(r'heal\s+(\d+(?:\.\d+)?)%', full_text, re.IGNORECASE)
    if heal_match:
        opt['rewards']['heal_percent'] = float(heal_match.group(1))
    
    # 清理空对象
    if not opt['costs']:
        del opt['costs']
    if not opt['rewards']:
        del opt['rewards']
    
    return opt


# ============================================================================
# 3. 随机结果解析器
# ============================================================================

def parse_random_outcome(outcome) -> dict:
    """将随机结果转换为结构化对象"""
    if isinstance(outcome, dict):
        # 已经是结构化的，增强它
        result = outcome.copy()
        
        # 解析 effect 字段中的数值
        effect_text = result.get('effect', '')
        
        # 金币奖励
        gold_match = re.search(r'(\d+)\s*gold', effect_text, re.IGNORECASE)
        if gold_match and 'reward' not in result:
            result['reward'] = {'type': 'Gold', 'amount': int(gold_match.group(1))}
        
        # 遗物奖励
        if 'relic' in effect_text.lower() and 'reward' not in result:
            result['reward'] = {'type': 'Relic', 'rarity': 'random'}
        
        # 回血
        if 'heal' in effect_text.lower():
            if 'full' in effect_text.lower():
                result['reward'] = {'type': 'HealFull'}
            else:
                heal_match = re.search(r'heal\s+(\d+)', effect_text, re.IGNORECASE)
                if heal_match:
                    result['reward'] = {'type': 'Heal', 'amount': int(heal_match.group(1))}
        
        # 战斗
        if 'fight' in effect_text.lower() or 'combat' in effect_text.lower():
            result['trigger'] = {'type': 'Combat'}
        
        return result
    
    elif isinstance(outcome, str):
        # 字符串结果，转换为对象
        text = outcome.lower()
        
        if 'gold' in text:
            gold_match = re.search(r'(\d+)', outcome)
            if gold_match:
                return {'type': 'Gold', 'amount': int(gold_match.group(1))}
            return {'type': 'Gold', 'amount': 'variable'}
        
        if 'heal' in text:
            if 'full' in text:
                return {'type': 'HealFull'}
            heal_match = re.search(r'(\d+)', outcome)
            if heal_match:
                return {'type': 'Heal', 'amount': int(heal_match.group(1))}
            return {'type': 'Heal', 'amount': 'variable'}
        
        if 'relic' in text:
            return {'type': 'Relic', 'rarity': 'random'}
        
        if 'card' in text:
            return {'type': 'Card', 'rarity': 'random'}
        
        if 'damage' in text or 'hp' in text:
            dmg_match = re.search(r'(\d+)', outcome)
            if dmg_match:
                return {'type': 'Damage', 'amount': int(dmg_match.group(1))}
            return {'type': 'Damage', 'amount': 'variable'}
        
        if 'curse' in text:
            return {'type': 'Curse', 'card': 'random'}
        
        if 'nothing' in text:
            return {'type': 'Nothing'}
        
        return {'type': 'ManualCheck', 'text': outcome}
    
    return outcome


# ============================================================================
# 4. 特殊事件处理器
# ============================================================================

SPECIAL_EVENT_FIXES = {
    'the_joust': {
        'fix': lambda event: fix_joust_event(event)
    },
    'wheel_of_change': {
        'fix': lambda event: fix_wheel_of_change(event)
    },
    'falling': {
        'fix': lambda event: fix_falling_event(event)
    },
    'knowing_skull': {
        'fix': lambda event: fix_knowing_skull(event)
    },
}


def fix_joust_event(event: dict) -> dict:
    """修复 The Joust 下注逻辑"""
    for opt in event.get('options', []):
        label = opt.get('label', '').lower()
        
        if 'owner' in label:
            opt['bet'] = {
                'amount': 50,
                'win_chance': 0.70,
                'win_reward': {'gold': 100},
                'lose_cost': {'gold': 50}
            }
        elif 'murderer' in label:
            opt['bet'] = {
                'amount': 50,
                'win_chance': 0.30,
                'win_reward': {'gold': 250},
                'lose_cost': {'gold': 50}
            }
    return event


def fix_wheel_of_change(event: dict) -> dict:
    """修复 Wheel of Change 随机结果"""
    # 定义标准轮盘结果
    wheel_outcomes = [
        {'outcome': 'Gold', 'reward': {'type': 'Gold', 'amount': 100}, 'weight': 1},
        {'outcome': 'Damage', 'cost': {'type': 'Damage', 'percent': 10}, 'weight': 1},
        {'outcome': 'Curse', 'reward': {'type': 'Curse', 'card': 'Decay'}, 'weight': 1},
        {'outcome': 'Card', 'reward': {'type': 'Card', 'rarity': 'any'}, 'weight': 1},
        {'outcome': 'HealFull', 'reward': {'type': 'HealFull'}, 'weight': 1},
        {'outcome': 'Relic', 'reward': {'type': 'Relic', 'rarity': 'random'}, 'weight': 1},
    ]
    
    for opt in event.get('options', []):
        if 'spin' in opt.get('label', '').lower():
            opt['wheel_outcomes'] = wheel_outcomes
    
    return event


def fix_falling_event(event: dict) -> dict:
    """修复 Falling 事件的卡牌类型条件"""
    card_type_map = {
        'skill': 'Skill',
        'power': 'Power',
        'attack': 'Attack',
    }
    
    for opt in event.get('options', []):
        label = opt.get('label', '').lower()
        desc = opt.get('description', '').lower()
        
        for keyword, card_type in card_type_map.items():
            if keyword in label or keyword in desc:
                opt['requirements'] = {'has_card_type': card_type}
                opt['costs'] = {'remove_card_type': card_type}
                break
    
    return event


def fix_knowing_skull(event: dict) -> dict:
    """修复 Knowing Skull 循环逻辑"""
    event['event_type'] = 'loop'
    event['loop_mechanic'] = {
        'description': 'Event repeats until [How do I leave?] is chosen',
        'cost_formula': 'max(6, floor(max_hp * 0.10)) + selection_count',
        'cost_per_selection': 1,
        'tracked_per_option': True
    }
    return event


# ============================================================================
# 5. 主处理流程
# ============================================================================

def process_option(opt: dict) -> dict:
    """处理单个选项"""
    # 1. 处理 conditions
    if 'conditions' in opt:
        opt['conditions'] = parse_conditions(opt['conditions'])
        # 如果全部解析成功，可以移除空条件
        if all(c.get('type') != 'ManualCheck' for c in opt['conditions']):
            # 保留结构化条件
            pass
        # 移除空列表
        if not opt['conditions']:
            del opt['conditions']
    
    # 2. 提取 effects 中的数学逻辑
    if 'effects' in opt:
        opt = extract_math_from_effects(opt['effects'], opt)
    
    # 3. 处理 random_outcomes
    if 'random_outcomes' in opt:
        opt['random_outcomes'] = [parse_random_outcome(o) for o in opt['random_outcomes']]
    
    # 4. 清理 notes 中的 ManualCheck（来自之前的 TODO_PARSE）
    if 'notes' in opt:
        # 保留 notes，它们是给人看的
        pass
    
    return opt


def process_event(event: dict) -> dict:
    """处理单个事件"""
    event_id = event.get('name', '').lower().replace(' ', '_').replace("'", "")
    
    # 1. 应用特殊修复
    if event_id in SPECIAL_EVENT_FIXES:
        event = SPECIAL_EVENT_FIXES[event_id]['fix'](event)
    
    # 2. 处理所有选项
    if 'options' in event:
        event['options'] = [process_option(opt) for opt in event['options']]
    
    # 3. 清理不需要的字段
    fields_to_remove = ['raw_options', 'raw_dialogue']
    for field in fields_to_remove:
        if field in event:
            del event[field]
    
    return event


def main():
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    
    stats = {
        'total_events': 0,
        'total_options': 0,
        'conditions_parsed': 0,
        'manual_checks': 0,
    }
    
    all_events = {}
    
    for fname in INPUT_FILES:
        filepath = INPUT_DIR / fname
        if not filepath.exists():
            print(f"⚠️ Skipping {fname} (not found)")
            continue
        
        print(f"Processing {fname}...")
        
        with open(filepath, 'r', encoding='utf-8') as f:
            data = json.load(f)
        
        processed = []
        for event in data:
            processed_event = process_event(event)
            processed.append(processed_event)
            
            # 统计
            stats['total_events'] += 1
            for opt in processed_event.get('options', []):
                stats['total_options'] += 1
                if 'conditions' in opt:
                    for cond in opt['conditions']:
                        if cond.get('type') == 'ManualCheck':
                            stats['manual_checks'] += 1
                        else:
                            stats['conditions_parsed'] += 1
            
            # 添加到合并字典
            event_id = processed_event.get('wiki_id', processed_event.get('name', 'unknown'))
            event_id = event_id.lower().replace(' ', '_').replace("'", "").replace("-", "_")
            event_id = re.sub(r'[^a-zA-Z0-9_]', '', event_id)
            all_events[event_id] = processed_event
        
        # 保存单独文件
        out_path = OUTPUT_DIR / fname
        with open(out_path, 'w', encoding='utf-8') as f:
            json.dump(processed, f, indent=2, ensure_ascii=False)
    
    # 保存合并文件
    master_path = DATA_DIR / 'events_final_master.json'
    with open(master_path, 'w', encoding='utf-8') as f:
        json.dump(all_events, f, indent=2, ensure_ascii=False)
    
    # 打印统计
    print("\n" + "="*50)
    print("✅ Logic Deep Clean Complete!")
    print("="*50)
    print(f"  Total events: {stats['total_events']}")
    print(f"  Total options: {stats['total_options']}")
    print(f"  Conditions parsed: {stats['conditions_parsed']}")
    print(f"  ⚠️ Manual checks needed: {stats['manual_checks']}")
    print(f"\n📁 Output files:")
    print(f"  - Individual: {OUTPUT_DIR}/")
    print(f"  - Master: {master_path}")
    
    if stats['manual_checks'] > 0:
        print(f"\n👉 Search for '\"type\": \"ManualCheck\"' to find items needing attention.")
        
        # 列出需要人工审核的项目
        print("\n📋 Items needing manual review:")
        for event_id, event in all_events.items():
            for i, opt in enumerate(event.get('options', [])):
                if 'conditions' in opt:
                    for cond in opt['conditions']:
                        if cond.get('type') == 'ManualCheck':
                            print(f"  - {event_id} / {opt.get('label', f'Option {i}')}")
                            print(f"    Text: {cond.get('text', 'N/A')[:60]}...")


if __name__ == "__main__":
    main()
