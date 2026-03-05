#!/usr/bin/env python3
"""
生成事件校对任务

将 Wiki 数据与现有 events_logic.json 对比，
按分类生成校对任务文件，每个文件只处理几个事件，避免一次处理过多。
"""

import json
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding='utf-8')

DATA_DIR = Path(__file__).parent.parent / "data"
PREPROCESSED_DIR = DATA_DIR / "events_preprocessed"
TASKS_DIR = DATA_DIR / "correction_tasks"
EXISTING_LOGIC = DATA_DIR / "events_logic.json"

# 名称映射
from event_name_mapping import WIKI_TO_LOGIC, MISSING_FROM_LOGIC


def load_data():
    """加载所有数据"""
    # 加载现有 logic
    with open(EXISTING_LOGIC, 'r', encoding='utf-8') as f:
        logic = json.load(f)
    
    # 加载预处理的 Wiki 数据
    wiki_data = {}
    for cat in ["shrines", "act1", "act2", "act3"]:
        cat_file = PREPROCESSED_DIR / f"{cat}.json"
        if cat_file.exists():
            with open(cat_file, 'r', encoding='utf-8') as f:
                for event in json.load(f):
                    wiki_data[event['wiki_id']] = event
    
    return logic, wiki_data


def generate_task(wiki_id: str, wiki_event: dict, logic_event: dict, logic_key: str) -> dict:
    """生成单个事件的校对任务"""
    task = {
        "wiki_id": wiki_id,
        "wiki_name": wiki_event['name'],
        "logic_key": logic_key,
        "status": "pending",  # pending / reviewed / corrected
        "issues": [],
        "wiki_data": {
            "options": wiki_event['options'],
            "notes": wiki_event['notes']
        },
        "existing_data": {
            "name": logic_event.get('name') if logic_event else None,
            "options": logic_event.get('options', []) if logic_event else []
        },
        "corrections": []  # 需要的修正
    }
    
    # 分析问题
    if not logic_event:
        task["issues"].append("NOT_IN_LOGIC: 事件在 events_logic.json 中不存在，需要新增")
    else:
        wiki_opts = len(wiki_event['options'])
        logic_opts = len(logic_event.get('options', []))
        
        if wiki_opts != logic_opts:
            task["issues"].append(f"OPTION_COUNT_MISMATCH: Wiki有{wiki_opts}个选项, Logic有{logic_opts}个")
        
        # 比较选项标签
        wiki_labels = set(opt['label'].lower() for opt in wiki_event['options'])
        logic_labels = set(opt.get('label', '').lower() for opt in logic_event.get('options', []))
        
        missing = wiki_labels - logic_labels
        extra = logic_labels - wiki_labels
        
        if missing:
            task["issues"].append(f"MISSING_OPTIONS: Logic缺少选项 {missing}")
        if extra:
            task["issues"].append(f"EXTRA_OPTIONS: Logic多余选项 {extra}")
    
    return task


def main():
    """主函数"""
    print("=" * 60)
    print("生成事件校对任务")
    print("=" * 60)
    
    TASKS_DIR.mkdir(parents=True, exist_ok=True)
    
    logic, wiki_data = load_data()
    
    # 按分类生成任务
    categories = {
        "shrines": ["A_Note_For_Yourself", "Bonfire_Spirits", "Duplicator", "Golden_Shrine",
                    "Lab", "Match_and_Keep", "Ominous_Forge", "Purifier", "The_Divine_Fountain",
                    "The_Woman_in_Blue", "Transmogrifier", "Upgrade_Shrine", "We_Meet_Again!",
                    "Wheel_of_Change"],
        "act1": ["Big_Fish", "Dead_Adventurer", "Face_Trader", "Golden_Idol_(Event)",
                 "Hypnotizing_Colored_Mushrooms", "Living_Wall", "Scrap_Ooze", "Shining_Light",
                 "The_Cleric", "The_Ssssserpent", "Wing_Statue", "World_of_Goop"],
        "act2": ["Ancient_Writing", "Augmenter", "Council_of_Ghosts", "Cursed_Tome",
                 "Designer_In-Spire", "Forgotten_Altar", "Knowing_Skull", "Masked_Bandits",
                 "N'loth", "Old_Beggar", "Pleading_Vagrant", "The_Colosseum", "The_Joust",
                 "The_Library", "The_Mausoleum", "The_Nest", "Vampires"],
        "act3": ["Falling", "Mind_Bloom", "Mysterious_Sphere", "Secret_Portal",
                 "Sensory_Stone", "The_Moai_Head", "Tomb_of_Lord_Red_Mask", "Winding_Halls"]
    }
    
    summary = {"total": 0, "with_issues": 0, "categories": {}}
    
    for cat, event_ids in categories.items():
        print(f"\n处理 {cat}...")
        tasks = []
        
        for wiki_id in event_ids:
            if wiki_id not in wiki_data:
                print(f"  [跳过] {wiki_id} - Wiki数据不存在")
                continue
            
            wiki_event = wiki_data[wiki_id]
            logic_key = WIKI_TO_LOGIC.get(wiki_id)
            logic_event = logic.get(logic_key) if logic_key else None
            
            task = generate_task(wiki_id, wiki_event, logic_event, logic_key)
            tasks.append(task)
            
            summary["total"] += 1
            if task["issues"]:
                summary["with_issues"] += 1
        
        # 分成小批次（每批最多5个事件）
        batch_size = 5
        for i in range(0, len(tasks), batch_size):
            batch = tasks[i:i+batch_size]
            batch_num = i // batch_size + 1
            
            output_file = TASKS_DIR / f"{cat}_batch{batch_num}.json"
            with open(output_file, 'w', encoding='utf-8') as f:
                json.dump({
                    "category": cat,
                    "batch": batch_num,
                    "events": batch,
                    "instructions": """
校对说明:
1. 查看每个事件的 issues 列表
2. 对比 wiki_data 和 existing_data 的差异
3. 在 corrections 中填写需要的修正
4. 完成后将 status 改为 "reviewed"

corrections 格式示例:
- {"action": "add_option", "label": "[新选项]", "data": {...}}
- {"action": "fix_label", "old": "[错误]", "new": "[正确]"}
- {"action": "add_event", "data": {...}}
"""
                }, f, indent=2, ensure_ascii=False)
            
            issues_in_batch = len([t for t in batch if t["issues"]])
            print(f"  生成: {output_file.name} ({len(batch)}个事件, {issues_in_batch}个有问题)")
        
        summary["categories"][cat] = {
            "total": len(tasks),
            "with_issues": len([t for t in tasks if t["issues"]])
        }
    
    # 保存汇总
    summary_file = TASKS_DIR / "summary.json"
    with open(summary_file, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)
    
    print("\n" + "=" * 60)
    print("任务生成完成！")
    print("=" * 60)
    print(f"\n总事件数: {summary['total']}")
    print(f"有问题: {summary['with_issues']}")
    print(f"\n任务文件位置: {TASKS_DIR}")
    print("\n建议校对顺序:")
    print("  1. 先处理 shrines (简单的神龛事件)")
    print("  2. 然后 act1 (第一幕事件)")
    print("  3. 接着 act2 和 act3")
    print("\n每个 batch 文件最多5个事件，避免一次处理过多")


if __name__ == "__main__":
    main()
