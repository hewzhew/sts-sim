#!/usr/bin/env python3
"""
Update events_final_master.json with act_pool, pool_type, and pool_conditions.
Based on act_1_exo.txt, act_2_city.txt, act_3_beyond.txt
"""

import json
from pathlib import Path

# Event distribution by act (from the txt files)
# Format: event_id -> (pool_type, [acts], pool_conditions or None)

EVENT_DISTRIBUTION = {
    # ========================================================================
    # ACT 1 REGULAR EVENTS
    # ========================================================================
    "big_fish": ("regular", ["act1"], None),
    "the_cleric": ("regular", ["act1"], {"type": "HasGold", "amount": 35}),
    "dead_adventurer": ("regular", ["act1"], {"type": "FloorMin", "floor": 7}),
    "golden_idol_event": ("regular", ["act1"], None),
    "living_wall": ("regular", ["act1"], None),
    "hypnotizing_colored_mushrooms": ("regular", ["act1"], {"type": "FloorMin", "floor": 7}),
    "scrap_ooze": ("regular", ["act1"], None),
    "shining_light": ("regular", ["act1"], None),
    "the_ssssserpent": ("regular", ["act1"], None),
    "wing_statue": ("regular", ["act1"], None),
    "world_of_goop": ("regular", ["act1"], None),
    
    # ========================================================================
    # ACT 2 REGULAR EVENTS
    # ========================================================================
    "ancient_writing": ("regular", ["act2"], None),
    "augmenter": ("regular", ["act2"], None),
    "the_colosseum": ("regular", ["act2"], {"type": "AboveChestFloor"}),
    "council_of_ghosts": ("regular", ["act2"], None),
    "cursed_tome": ("regular", ["act2"], None),
    "forgotten_altar": ("regular", ["act2"], None),
    "the_library": ("regular", ["act2"], None),
    "masked_bandits": ("regular", ["act2"], None),
    "the_mausoleum": ("regular", ["act2"], None),
    "the_nest": ("regular", ["act2"], None),
    "pleading_vagrant": ("regular", ["act2"], None),
    "old_beggar": ("regular", ["act2"], {"type": "HasGold", "amount": 75}),
    "vampires": ("regular", ["act2"], None),
    
    # ========================================================================
    # ACT 3 REGULAR EVENTS
    # ========================================================================
    "falling": ("regular", ["act3"], None),
    "mind_bloom": ("regular", ["act3"], None),
    "the_moai_head": ("regular", ["act3"], {
        "type": "Or",
        "conditions": [
            {"type": "HasRelic", "relic_id": "Golden Idol"},
            {"type": "HpPercent", "max_percent": 50}
        ]
    }),
    "mysterious_sphere": ("regular", ["act3"], None),
    "sensory_stone": ("regular", ["act3"], None),
    "tomb_of_lord_red_mask": ("regular", ["act3"], None),
    "winding_halls": ("regular", ["act3"], None),
    
    # ========================================================================
    # SHRINE EVENTS (appear in multiple acts)
    # ========================================================================
    # Act 1, 2, 3 shrines
    "bonfire_spirits": ("shrine", ["act1", "act2", "act3"], None),
    "the_divine_fountain": ("shrine", ["act1", "act2", "act3"], {"type": "HasCurse"}),
    "golden_shrine": ("shrine", ["act1", "act2", "act3"], None),
    "lab": ("shrine", ["act1", "act2", "act3"], None),
    "match_and_keep": ("shrine", ["act1", "act2", "act3"], None),
    "a_note_for_yourself": ("shrine", ["act1", "act2", "act3"], {"type": "AscensionBelow", "level": 15}),
    "ominous_forge": ("shrine", ["act1", "act2", "act3"], None),
    "purifier": ("shrine", ["act1", "act2", "act3"], None),
    "transmogrifier": ("shrine", ["act1", "act2", "act3"], None),
    "upgrade_shrine": ("shrine", ["act1", "act2", "act3"], None),
    "we_meet_again": ("shrine", ["act1", "act2", "act3"], None),
    "wheel_of_change": ("shrine", ["act1", "act2", "act3"], None),
    "the_woman_in_blue": ("shrine", ["act1", "act2", "act3"], {"type": "HasGold", "amount": 50}),
    
    # Act 1, 2 shrines only
    "face_trader": ("shrine", ["act1", "act2"], None),
    
    # Act 2, 3 shrines only
    "designer_in_spire": ("shrine", ["act2", "act3"], {"type": "HasGold", "amount": 75}),
    "duplicator": ("shrine", ["act2", "act3"], None),
    
    # Act 2 only shrines
    "the_joust": ("shrine", ["act2"], {"type": "HasGold", "amount": 50}),
    "knowing_skull": ("shrine", ["act2"], {"type": "HpAbove", "amount": 12}),
    "nloth": ("shrine", ["act2"], {"type": "RelicCount", "min": 2}),
    
    # Act 3 only shrines
    "secret_portal": ("shrine", ["act3"], {"type": "TimeElapsed", "seconds": 800}),
}

def main():
    # Load existing JSON
    json_path = Path(__file__).parent.parent / "events_final_master.json"
    with open(json_path, 'r', encoding='utf-8') as f:
        events = json.load(f)
    
    # Track what we found and what's missing
    found = set()
    missing_in_dist = []
    
    # Update each event
    for event_id, event_data in events.items():
        if event_id in EVENT_DISTRIBUTION:
            pool_type, act_pool, pool_conditions = EVENT_DISTRIBUTION[event_id]
            
            # Add new fields right after 'category'
            event_data["pool_type"] = pool_type
            event_data["act_pool"] = act_pool
            if pool_conditions:
                event_data["pool_conditions"] = [pool_conditions] if not isinstance(pool_conditions, list) else pool_conditions
            
            found.add(event_id)
        else:
            missing_in_dist.append(event_id)
    
    # Report missing
    missing_in_json = set(EVENT_DISTRIBUTION.keys()) - found
    
    if missing_in_dist:
        print(f"⚠️  Events in JSON but not in distribution mapping:")
        for e in missing_in_dist:
            print(f"    - {e}")
    
    if missing_in_json:
        print(f"⚠️  Events in distribution but not in JSON:")
        for e in missing_in_json:
            print(f"    - {e}")
    
    # Save updated JSON
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(events, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ Updated {len(found)} events with pool information")
    print(f"   - Regular events: {sum(1 for v in EVENT_DISTRIBUTION.values() if v[0] == 'regular')}")
    print(f"   - Shrine events: {sum(1 for v in EVENT_DISTRIBUTION.values() if v[0] == 'shrine')}")

if __name__ == "__main__":
    main()
