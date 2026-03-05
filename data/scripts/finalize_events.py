#!/usr/bin/env python3
"""
Phase 7.3 Step 4: Finalize Events
合并、强类型化、标记需要人工审核的复杂逻辑

输入: data/events_structured/*.json
输出: data/events_master.json
"""

import json
import re
import os
from pathlib import Path

# 配置
SCRIPT_DIR = Path(__file__).parent
DATA_DIR = SCRIPT_DIR.parent
INPUT_DIR = DATA_DIR / 'events_structured'
OUTPUT_FILE = DATA_DIR / 'events_master.json'

INPUT_FILES = ['act1.json', 'act2.json', 'act3.json', 'shrines.json']


def load_json(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        return json.load(f)


def parse_percentage(text):
    """尝试从文本中提取百分比逻辑"""
    # 匹配 "25% (A15+: 35%)"
    match = re.search(r'(\d+)%\s*\(A15\+:\s*(\d+)%\)', text)
    if match:
        return {
            "base": float(match.group(1)) / 100.0,
            "ascension_15": float(match.group(2)) / 100.0
        }
    
    # 匹配单纯的 "25%"
    match = re.search(r'(\d+)%', text)
    if match:
        return {"base": float(match.group(1)) / 100.0}
    return None


def parse_gold_range(text):
    """解析金币范围，如 '20 - 50' 或 '25 to 35'"""
    patterns = [
        r'(\d+)\s*[-–]\s*(\d+)',  # 20-50 or 20 - 50
        r'(\d+)\s+to\s+(\d+)',    # 20 to 50
        r'between\s+(\d+)\s+and\s+(\d+)',  # between 20 and 50
    ]
    for pattern in patterns:
        match = re.search(pattern, text, re.IGNORECASE)
        if match:
            return int(match.group(1)), int(match.group(2))
    return None, None


def normalize_logic(option):
    """
    将剩余的自然语言转化为逻辑结构
    重点修补那些 AI 没提取干净的字段
    """
    desc = option.get('description', '') + " " + " ".join(option.get('effects', []))
    
    # 1. 补全 Costs (如果 AI 漏了)
    if 'costs' not in option:
        option['costs'] = {}
    
    # HP Loss with Ascension
    hp_match = re.search(r'[Ll]ose\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*HP', desc)
    if hp_match:
        option['costs']['hp'] = int(hp_match.group(1))
        option['costs']['hp_ascension'] = int(hp_match.group(2))
    elif 'Lose' in desc and 'HP' in desc:
        simple_hp = re.search(r'[Ll]ose\s+(\d+)\s*HP', desc)
        if simple_hp and 'hp' not in option['costs']:
            option['costs']['hp'] = int(simple_hp.group(1))
    
    # HP Loss as percentage
    hp_pct = re.search(r'[Ll]ose\s+(\d+)%\s*\(A15\+:\s*(\d+)%\)', desc)
    if hp_pct:
        option['costs']['hp_percent'] = int(hp_pct.group(1))
        option['costs']['hp_percent_ascension'] = int(hp_pct.group(2))
    
    # Gold Loss with Ascension
    gold_loss = re.search(r'[Ll]ose\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*[Gg]old', desc)
    if gold_loss:
        option['costs']['gold'] = int(gold_loss.group(1))
        option['costs']['gold_ascension'] = int(gold_loss.group(2))
    elif re.search(r'[Ll]ose.*[Gg]old', desc):
        simple_gold = re.search(r'[Ll]ose\s+(\d+)\s*[Gg]old', desc)
        if simple_gold and 'gold' not in option['costs']:
            option['costs']['gold'] = int(simple_gold.group(1))
    
    # Lose ALL Gold
    if re.search(r'[Ll]ose\s+ALL\s+(?:of\s+)?(?:your\s+)?[Gg]old', desc):
        option['costs']['gold'] = 'all'
    
    # Max HP Loss
    max_hp = re.search(r'[Ll]ose\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*[Mm]ax\s*HP', desc)
    if max_hp:
        option['costs']['max_hp'] = int(max_hp.group(1))
        option['costs']['max_hp_ascension'] = int(max_hp.group(2))
    elif 'Lose' in desc and 'Max HP' in desc:
        simple_max = re.search(r'[Ll]ose\s+(\d+)\s*[Mm]ax\s*HP', desc)
        if simple_max and 'max_hp' not in option['costs']:
            option['costs']['max_hp'] = int(simple_max.group(1))

    # 2. 补全 Rewards
    if 'rewards' not in option:
        option['rewards'] = {}
    
    # Gold Gain with Ascension
    gold_gain = re.search(r'[Gg]ain\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*[Gg]old', desc)
    if gold_gain:
        option['rewards']['gold'] = int(gold_gain.group(1))
        option['rewards']['gold_ascension'] = int(gold_gain.group(2))
    elif 'Gain' in desc and 'Gold' in desc:
        simple_gold = re.search(r'[Gg]ain\s+(\d+)\s*[Gg]old', desc)
        if simple_gold and 'gold' not in option['rewards']:
            option['rewards']['gold'] = int(simple_gold.group(1))
    
    # Gold Range (random)
    if 'random' in desc.lower() and 'gold' in desc.lower():
        min_g, max_g = parse_gold_range(desc)
        if min_g and max_g:
            option['rewards']['gold'] = {"type": "random", "min": min_g, "max": max_g}
    
    # HP Gain
    hp_gain = re.search(r'[Gg]ain\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*HP(?!\s*[Mm]ax)', desc)
    if hp_gain:
        option['rewards']['hp'] = int(hp_gain.group(1))
        option['rewards']['hp_ascension'] = int(hp_gain.group(2))
    elif re.search(r'[Gg]ain.*HP(?!\s*[Mm]ax)', desc):
        simple_hp = re.search(r'[Gg]ain\s+(\d+)\s*HP', desc)
        if simple_hp and 'hp' not in option['rewards']:
            option['rewards']['hp'] = int(simple_hp.group(1))
    
    # Heal
    heal = re.search(r'[Hh]eal\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*HP', desc)
    if heal:
        option['rewards']['heal'] = int(heal.group(1))
        option['rewards']['heal_ascension'] = int(heal.group(2))
    elif 'Heal' in desc:
        simple_heal = re.search(r'[Hh]eal\s+(\d+)', desc)
        if simple_heal and 'heal' not in option['rewards']:
            option['rewards']['heal'] = int(simple_heal.group(1))
    
    # Max HP Gain
    max_hp_gain = re.search(r'[Gg]ain\s+(\d+)\s*\(A15\+:\s*(\d+)\)\s*[Mm]ax\s*HP', desc)
    if max_hp_gain:
        option['rewards']['max_hp'] = int(max_hp_gain.group(1))
        option['rewards']['max_hp_ascension'] = int(max_hp_gain.group(2))
    elif 'Gain' in desc and 'Max HP' in desc:
        simple_max = re.search(r'[Gg]ain\s+(\d+)\s*[Mm]ax\s*HP', desc)
        if simple_max and 'max_hp' not in option['rewards']:
            option['rewards']['max_hp'] = int(simple_max.group(1))

    # 3. 清理空的 costs/rewards
    if not option['costs']:
        del option['costs']
    if not option['rewards']:
        del option['rewards']

    # 4. 标记复杂变量逻辑 (X)
    has_variable_x = bool(re.search(r'\bX\b', desc))
    has_formula = 'hp_formula' in str(option.get('costs', '')) or 'formula' in str(option)
    
    if has_variable_x and not has_formula:
        option['manual_review_needed'] = True
        option['review_note'] = "Variable X logic detected but not parsed."

    # 5. 规范化 Conditions
    if 'conditions' in option and isinstance(option['conditions'], list):
        new_conditions = []
        for cond in option['conditions']:
            if isinstance(cond, str):
                structured = parse_condition_string(cond)
                new_conditions.append(structured)
            else:
                new_conditions.append(cond)
        option['conditions'] = new_conditions
        
        # 如果全是结构化的，去掉 conditions key（已转入 requirements）
        if all(isinstance(c, dict) and c.get('type') != 'TODO_PARSE' for c in new_conditions):
            if 'requirements' not in option:
                option['requirements'] = {}
            for c in new_conditions:
                if c.get('type') == 'HasRelic':
                    option['requirements']['has_relic'] = c.get('id')
                elif c.get('type') == 'HasGold':
                    option['requirements']['min_gold'] = c.get('amount')
                elif c.get('type') == 'HasHP':
                    option['requirements']['min_hp'] = c.get('amount')
            del option['conditions']

    return option


def parse_condition_string(cond: str) -> dict:
    """将条件字符串转为结构化对象"""
    lower = cond.lower()
    
    # Relic checks
    relic_patterns = [
        (r'golden\s*idol', 'Golden Idol'),
        (r'blood\s*vial', 'Blood Vial'),
        (r'red\s*mask', 'Red Mask'),
        (r'necronomicon', 'Necronomicon'),
        (r'neow.*lament', "Neow's Lament"),
        (r'circlet', 'Circlet'),
    ]
    
    for pattern, relic_id in relic_patterns:
        if re.search(pattern, lower):
            return {"type": "HasRelic", "id": relic_id}
    
    # Gold check
    gold_match = re.search(r'(?:at\s+least|have|requires?)\s+(\d+)\s*gold', lower)
    if gold_match:
        return {"type": "HasGold", "amount": int(gold_match.group(1))}
    
    # HP check
    hp_match = re.search(r'(?:at\s+least|have|requires?)\s+(\d+)\s*hp', lower)
    if hp_match:
        return {"type": "HasHP", "amount": int(hp_match.group(1))}
    
    # Card type check
    card_patterns = [
        (r'have\s+(?:a\s+)?skill\s+card', 'Skill'),
        (r'have\s+(?:a\s+)?attack\s+card', 'Attack'),
        (r'have\s+(?:a\s+)?curse', 'Curse'),
    ]
    for pattern, card_type in card_patterns:
        if re.search(pattern, lower):
            return {"type": "HasCardType", "card_type": card_type}
    
    # Fallback: mark for manual review
    return {"type": "TODO_PARSE", "text": cond}


def extract_random_outcome_probabilities(event):
    """从 random_outcomes 中提取概率结构"""
    for opt in event.get('options', []):
        if 'random_outcomes' not in opt:
            continue
        
        outcomes = opt['random_outcomes']
        for outcome in outcomes:
            # 跳过非字典类型的 outcome
            if not isinstance(outcome, dict):
                continue
            
            # 尝试从 'probability' 或 'chance' 字段获取概率文本
            prob_text = outcome.get('probability', '') or outcome.get('chance', '')
            if not prob_text:
                continue
            
            # 解析概率
            prob_data = parse_percentage(prob_text)
            if prob_data:
                outcome['probability_parsed'] = prob_data
            
            # 检查是否有递增逻辑
            if 'per search' in prob_text.lower() or 'cumulative' in prob_text.lower():
                incr_match = re.search(r'\+(\d+)%', prob_text)
                if incr_match:
                    outcome['probability_increment'] = float(incr_match.group(1)) / 100.0
    
    return event


def clean_event(event):
    """清理事件，移除不需要的字段"""
    # 清理不需要的 Wiki 原文字段
    keys_to_remove = ['raw_options', 'raw_dialogue']
    for k in keys_to_remove:
        if k in event:
            del event[k]
    
    # 处理 options
    if 'options' in event:
        for opt in event['options']:
            normalize_logic(opt)
            
            # 清理空的 effects 数组
            if 'effects' in opt and not opt['effects']:
                del opt['effects']
    
    # 提取随机结果概率
    extract_random_outcome_probabilities(event)
    
    return event


def generate_event_id(event):
    """生成事件 ID (snake_case)"""
    wiki_id = event.get('wiki_id', event.get('name', 'unknown'))
    # 转为 snake_case
    event_id = wiki_id.replace(' ', '_').replace("'", "").replace("-", "_")
    event_id = re.sub(r'[^a-zA-Z0-9_]', '', event_id)
    return event_id.lower()


def main():
    master_list = {}
    stats = {
        'total_events': 0,
        'total_options': 0,
        'manual_review_needed': 0,
        'costs_extracted': 0,
        'rewards_extracted': 0,
    }
    
    for fname in INPUT_FILES:
        filepath = INPUT_DIR / fname
        if not filepath.exists():
            print(f"⚠️ Skipping {fname} (not found)")
            continue
        
        print(f"Processing {fname}...")
        data = load_json(filepath)
        
        for event in data:
            event_id = generate_event_id(event)
            cleaned = clean_event(event)
            
            # 统计
            stats['total_events'] += 1
            for opt in cleaned.get('options', []):
                stats['total_options'] += 1
                if opt.get('manual_review_needed'):
                    stats['manual_review_needed'] += 1
                if 'costs' in opt:
                    stats['costs_extracted'] += 1
                if 'rewards' in opt:
                    stats['rewards_extracted'] += 1
            
            master_list[event_id] = cleaned
    
    # 保存
    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with open(OUTPUT_FILE, 'w', encoding='utf-8') as f:
        json.dump(master_list, f, indent=2, ensure_ascii=False)
    
    # 打印统计
    print("\n" + "="*50)
    print("✅ Finalization Complete!")
    print("="*50)
    print(f"  Events merged: {stats['total_events']}")
    print(f"  Options processed: {stats['total_options']}")
    print(f"  Costs extracted: {stats['costs_extracted']}")
    print(f"  Rewards extracted: {stats['rewards_extracted']}")
    print(f"  ⚠️ Manual review needed: {stats['manual_review_needed']}")
    print(f"\n📁 Output: {OUTPUT_FILE}")
    
    if stats['manual_review_needed'] > 0:
        print(f"\n👉 Search for 'manual_review_needed' in the JSON to find items needing attention.")
    
    # 列出需要人工审核的事件
    print("\n📋 Events needing manual review:")
    for event_id, event in master_list.items():
        for i, opt in enumerate(event.get('options', [])):
            if opt.get('manual_review_needed'):
                print(f"  - {event_id} / Option {i}: {opt.get('label', 'N/A')}")
                print(f"    Note: {opt.get('review_note', 'N/A')}")


if __name__ == "__main__":
    main()
