#!/usr/bin/env python3
"""
事件名称映射表

Wiki 名称 <-> events_logic.json 内部 key 的对应关系
"""

# Wiki wiki_id -> events_logic.json key
WIKI_TO_LOGIC = {
    # Shrines
    "A_Note_For_Yourself": None,  # 不在 logic 中
    "Bonfire_Spirits": "Bonfire Elementals",
    "Duplicator": "Duplicator",
    "Golden_Shrine": "Golden Shrine",
    "Lab": "Lab",
    "Match_and_Keep": "Match and Keep!",
    "Ominous_Forge": "Accursed Blacksmith",
    "Purifier": "Purifier",
    "The_Divine_Fountain": "Fountain of Cleansing",
    "The_Woman_in_Blue": "The Woman in Blue",
    "Transmogrifier": "Transmorgrifier",  # 注意拼写差异
    "Upgrade_Shrine": "Upgrade Shrine",
    "We_Meet_Again!": None,  # 不在 logic 中
    "Wheel_of_Change": "Wheel of Change",
    
    # Act 1
    "Big_Fish": "Big Fish",
    "Dead_Adventurer": "Dead Adventurer",
    "Face_Trader": "FaceTrader",
    "Golden_Idol_(Event)": "Golden Idol",
    "Hypnotizing_Colored_Mushrooms": "Mushrooms",
    "Living_Wall": "Living Wall",
    "Scrap_Ooze": "Scrap Ooze",
    "Shining_Light": "Shining Light",
    "The_Cleric": "The Cleric",
    "The_Ssssserpent": "Liars Game",
    "Wing_Statue": "Golden Wing",
    "World_of_Goop": "World of Goop",
    
    # Act 2
    "Ancient_Writing": "Back to Basics",
    "Augmenter": "Drug Dealer",  # Augmenter 就是 J.A.X. Dealer
    "Council_of_Ghosts": "Ghosts",
    "Cursed_Tome": "Cursed Tome",
    "Designer_In-Spire": None,  # 不在 logic 中
    "Forgotten_Altar": "Forgotten Altar",
    "Knowing_Skull": "Knowing Skull",
    "Masked_Bandits": "Masked Bandits",
    "N'loth": "N'loth",
    "Old_Beggar": "Beggar",
    "Pleading_Vagrant": "Addict",  # Pleading Vagrant = Addicted
    "The_Colosseum": None,  # 不在 logic 中
    "The_Joust": "The Joust",
    "The_Library": "The Library",
    "The_Mausoleum": "The Mausoleum",
    "The_Nest": "Nest",
    "Vampires": "Vampires",
    
    # Act 3
    "Falling": "Falling",
    "Mind_Bloom": "MindBloom",
    "Mysterious_Sphere": "Mysterious Sphere",
    "Secret_Portal": "SecretPortal",
    "Sensory_Stone": "SensoryStone",
    "The_Moai_Head": "The Moai Head",
    "Tomb_of_Lord_Red_Mask": "Tomb of Lord Red Mask",
    "Winding_Halls": "Winding Halls",
}

# 反向映射
LOGIC_TO_WIKI = {v: k for k, v in WIKI_TO_LOGIC.items() if v is not None}

# 不在 events_logic.json 中的事件
MISSING_FROM_LOGIC = [k for k, v in WIKI_TO_LOGIC.items() if v is None]

# 不确定的映射（需要人工确认）
UNCERTAIN_MAPPINGS = {
    # 所有映射已确认
}

# events_logic.json 中有但 Wiki 没覆盖的（可能是旧版/特殊事件）
LOGIC_ONLY = [
    "Thinking with Puddles",  # 不确定这是什么事件
]

if __name__ == "__main__":
    print(f"Wiki 事件: {len(WIKI_TO_LOGIC)}")
    print(f"有映射: {len([v for v in WIKI_TO_LOGIC.values() if v])}")
    print(f"无映射: {len(MISSING_FROM_LOGIC)}")
    print(f"\n不在 events_logic.json 中的事件:")
    for e in MISSING_FROM_LOGIC:
        note = UNCERTAIN_MAPPINGS.get(e, "")
        print(f"  - {e} {note}")
