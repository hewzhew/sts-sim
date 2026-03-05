#!/usr/bin/env python3
"""
合并所有手动下载的事件 JSON 文件
"""

import json
from pathlib import Path

DOWNLOADED_DIR = Path(__file__).parent / "downloaded"
OUTPUT_DIR = Path(__file__).parent.parent.parent / "data" / "wiki_scrape"

def merge_events():
    """合并所有下载的 JSON 文件"""
    
    # 确保输出目录存在
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    
    # 查找所有 JSON 文件
    json_files = list(DOWNLOADED_DIR.glob("*.json"))
    
    if not json_files:
        print(f"❌ 在 {DOWNLOADED_DIR} 中没有找到 JSON 文件")
        print(f"   请将下载的文件放到该文件夹中")
        return
    
    print(f"找到 {len(json_files)} 个 JSON 文件")
    
    # 合并所有数据
    all_events = {}
    errors = []
    
    for json_file in sorted(json_files):
        try:
            with open(json_file, "r", encoding="utf-8") as f:
                data = json.load(f)
            
            wiki_id = data.get("wiki_id", json_file.stem)
            all_events[wiki_id] = data
            
            # 检查数据质量
            has_options = bool(data.get("raw_options", "").strip())
            has_dialogue = bool(data.get("raw_dialogue", "").strip())
            
            status = "✓" if (has_options or has_dialogue) else "⚠"
            print(f"  {status} {wiki_id}: options={has_options}, dialogue={has_dialogue}")
            
        except Exception as e:
            errors.append((json_file.name, str(e)))
            print(f"  ✗ {json_file.name}: {e}")
    
    # 保存合并后的文件
    output_file = OUTPUT_DIR / "all_events_raw.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(all_events, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ 已保存合并数据到: {output_file}")
    print(f"   总计: {len(all_events)} 个事件")
    
    if errors:
        print(f"\n⚠ 有 {len(errors)} 个文件处理失败:")
        for name, err in errors:
            print(f"   - {name}: {err}")
    
    # 统计
    with_options = sum(1 for e in all_events.values() if e.get("raw_options", "").strip())
    with_dialogue = sum(1 for e in all_events.values() if e.get("raw_dialogue", "").strip())
    
    print(f"\n📊 数据统计:")
    print(f"   有 Options 数据: {with_options}/{len(all_events)}")
    print(f"   有 Dialogue 数据: {with_dialogue}/{len(all_events)}")


def list_missing():
    """列出还没有下载的事件"""
    
    # 预期的事件列表
    expected = [
        "A_Note_For_Yourself", "Bonfire_Spirits", "Duplicator", "Golden_Shrine",
        "Lab", "Match_and_Keep", "Ominous_Forge", "Purifier", "The_Divine_Fountain",
        "The_Woman_in_Blue", "Transmogrifier", "Upgrade_Shrine", "We_Meet_Again!",
        "Wheel_of_Change", "Big_Fish", "Dead_Adventurer", "Face_Trader", 
        "Golden_Idol_(Event)", "Hypnotizing_Colored_Mushrooms", "Living_Wall",
        "Scrap_Ooze", "Shining_Light", "The_Cleric", "The_Ssssserpent",
        "Wing_Statue", "World_of_Goop", "Ancient_Writing", "Augmenter",
        "Council_of_Ghosts", "Cursed_Tome", "Designer_In-Spire", "Forgotten_Altar",
        "Knowing_Skull", "Masked_Bandits", "N%27loth", "Old_Beggar",
        "Pleading_Vagrant", "The_Colosseum", "The_Joust", "The_Library",
        "The_Mausoleum", "The_Nest", "Vampires", "Falling", "Mind_Bloom",
        "Mysterious_Sphere", "Secret_Portal", "Sensory_Stone", "The_Moai_Head",
        "Tomb_of_Lord_Red_Mask", "Winding_Halls"
    ]
    
    # 已下载的文件
    downloaded = {f.stem for f in DOWNLOADED_DIR.glob("*.json")}
    
    missing = [e for e in expected if e not in downloaded]
    
    if missing:
        print(f"还有 {len(missing)} 个事件未下载:")
        for e in missing:
            print(f"  - https://slaythespire.wiki.gg/wiki/{e}")
    else:
        print("✅ 所有事件都已下载!")


if __name__ == "__main__":
    import sys
    
    # 确保 downloaded 文件夹存在
    DOWNLOADED_DIR.mkdir(exist_ok=True)
    
    if len(sys.argv) > 1 and sys.argv[1] == "missing":
        list_missing()
    else:
        merge_events()
        print("\n提示: 运行 'python merge_events.py missing' 查看缺失的事件")
