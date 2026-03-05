#!/usr/bin/env python3
"""
事件数据预处理脚本

将 Wiki 爬取的原始数据预处理为结构化格式，方便后续 AI 校对。
分批处理避免一次处理过多数据导致错乱。
"""

import json
import re
from pathlib import Path
from dataclasses import dataclass, asdict
from typing import Optional

# 路径配置
DATA_DIR = Path(__file__).parent.parent / "data"
RAW_FILE = DATA_DIR / "sts_events_2026-01-23.json"
OUTPUT_DIR = DATA_DIR / "events_preprocessed"
EXISTING_LOGIC = DATA_DIR / "events_logic.json"

# 事件分类（用于分批处理）
EVENT_CATEGORIES = {
    "shrines": [
        "A_Note_For_Yourself", "Bonfire_Spirits", "Duplicator", "Golden_Shrine",
        "Lab", "Match_and_Keep", "Ominous_Forge", "Purifier", "The_Divine_Fountain",
        "The_Woman_in_Blue", "Transmogrifier", "Upgrade_Shrine", "We_Meet_Again!",
        "Wheel_of_Change"
    ],
    "act1": [
        "Big_Fish", "Dead_Adventurer", "Face_Trader", "Golden_Idol_(Event)",
        "Hypnotizing_Colored_Mushrooms", "Living_Wall", "Scrap_Ooze", "Shining_Light",
        "The_Cleric", "The_Ssssserpent", "Wing_Statue", "World_of_Goop"
    ],
    "act2": [
        "Ancient_Writing", "Augmenter", "Council_of_Ghosts", "Cursed_Tome",
        "Designer_In-Spire", "Forgotten_Altar", "Knowing_Skull", "Masked_Bandits",
        "N'loth", "Old_Beggar", "Pleading_Vagrant", "The_Colosseum", "The_Joust",
        "The_Library", "The_Mausoleum", "The_Nest", "Vampires"
    ],
    "act3": [
        "Falling", "Mind_Bloom", "Mysterious_Sphere", "Secret_Portal",
        "Sensory_Stone", "The_Moai_Head", "Tomb_of_Lord_Red_Mask", "Winding_Halls"
    ]
}


@dataclass
class EventOption:
    """事件选项"""
    label: str                    # 选项显示文本，如 "[Banana]"
    description: str              # 选项效果描述
    conditions: list[str]         # 出现条件
    effects: list[str]            # 效果列表


@dataclass  
class PreprocessedEvent:
    """预处理后的事件数据"""
    wiki_id: str
    name: str
    category: str                 # shrines/act1/act2/act3
    options: list[EventOption]    # 解析后的选项
    notes: list[str]              # 注意事项
    raw_options: str              # 原始文本（用于对照）
    raw_dialogue: str             # 对话文本


def parse_options_text(raw_options: str) -> list[EventOption]:
    """
    解析原始 options 文本为结构化数据
    
    示例输入:
    "[Banana] Heal 1/3 of your max HP.
    • If the player's HP is not divisble by 3, the HP gain is rounded down.
    [Donut] Max HP +5.
    • Like all HP gain, the extra hit points are healed when you get them."
    """
    options = []
    
    # 按 [选项名] 分割
    pattern = r'\[([^\]]+)\]'
    parts = re.split(pattern, raw_options)
    
    # parts 格式: ['前置文本', '选项1名', '选项1内容', '选项2名', '选项2内容', ...]
    i = 1
    while i < len(parts) - 1:
        label = f"[{parts[i]}]"
        content = parts[i + 1].strip() if i + 1 < len(parts) else ""
        
        # 分离描述和条件/注释
        lines = content.split('\n')
        description = ""
        conditions = []
        effects = []
        
        for line in lines:
            line = line.strip()
            if not line:
                continue
            
            # 条件行（以 • 开头且包含条件关键词）
            if line.startswith('•'):
                note = line[1:].strip()
                if any(kw in note.lower() for kw in ['if ', 'only ', 'when ', 'requires']):
                    conditions.append(note)
                else:
                    effects.append(note)
            elif not description:
                description = line
            else:
                effects.append(line)
        
        options.append(EventOption(
            label=label,
            description=description,
            conditions=conditions,
            effects=effects
        ))
        
        i += 2
    
    return options


def preprocess_event(wiki_id: str, data: dict, category: str) -> PreprocessedEvent:
    """预处理单个事件"""
    raw_options = data.get('raw_options', '')
    raw_dialogue = data.get('raw_dialogue', '')
    raw_notes = data.get('raw_notes', '')
    
    # 解析选项
    options = parse_options_text(raw_options)
    
    # 提取注意事项
    notes = []
    if raw_notes:
        for line in raw_notes.split('\n'):
            line = line.strip()
            if line.startswith('•'):
                notes.append(line[1:].strip())
            elif line:
                notes.append(line)
    
    return PreprocessedEvent(
        wiki_id=wiki_id,
        name=data.get('name', wiki_id),
        category=category,
        options=options,
        notes=notes,
        raw_options=raw_options,
        raw_dialogue=raw_dialogue
    )


def load_existing_logic() -> dict:
    """加载现有的 events_logic.json"""
    if EXISTING_LOGIC.exists():
        with open(EXISTING_LOGIC, 'r', encoding='utf-8') as f:
            return json.load(f)
    return {}


def preprocess_category(category: str, raw_events: dict) -> list[dict]:
    """预处理一个分类的所有事件"""
    event_ids = EVENT_CATEGORIES.get(category, [])
    results = []
    
    for wiki_id in event_ids:
        if wiki_id in raw_events:
            event = preprocess_event(wiki_id, raw_events[wiki_id], category)
            results.append(asdict(event))
        else:
            print(f"  [跳过] {wiki_id} - 未找到数据")
    
    return results


def main():
    """主函数"""
    print("=" * 60)
    print("事件数据预处理")
    print("=" * 60)
    
    # 加载原始数据
    with open(RAW_FILE, 'r', encoding='utf-8') as f:
        raw_events = json.load(f)
    print(f"\n加载了 {len(raw_events)} 个原始事件")
    
    # 创建输出目录
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    
    # 分批处理每个分类
    for category, event_ids in EVENT_CATEGORIES.items():
        print(f"\n处理 {category} ({len(event_ids)} 个事件)...")
        
        results = preprocess_category(category, raw_events)
        
        # 保存到单独文件
        output_file = OUTPUT_DIR / f"{category}.json"
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, ensure_ascii=False)
        
        print(f"  保存到: {output_file.name}")
        print(f"  成功处理: {len(results)} 个事件")
        
        # 显示简要摘要
        for event in results[:2]:  # 只显示前2个作为预览
            opt_count = len(event['options'])
            print(f"    - {event['name']}: {opt_count} 个选项")
    
    print("\n" + "=" * 60)
    print("预处理完成！")
    print("=" * 60)
    print(f"\n输出文件位置: {OUTPUT_DIR}")
    print("\n下一步: 使用 compare_events.py 与现有 events_logic.json 对比")


if __name__ == "__main__":
    main()
