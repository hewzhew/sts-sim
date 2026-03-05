#!/usr/bin/env python3
"""
Patch potions.json to fix missing/ambiguous data.

Issues addressed:
1. Discovery potions missing potency and descriptions
2. Fruit Juice using Heal instead of GainMaxHP
3. Snecko Oil missing special command
4. Blood Potion needs HealPercent command
5. Bottled Miracle missing potency
6. Cunning Potion missing potency
7. Various other missing upgraded values
"""

import json
from pathlib import Path

FILE_PATH = Path(__file__).parent.parent / 'data' / 'potions.json'

def patch_data():
    with open(FILE_PATH, 'r', encoding='utf-8') as f:
        data = json.load(f)

    patched_count = 0
    
    for p in data:
        pid = p['id']
        original = p.copy()
        
        # ============================================================
        # 1. Discovery Potions (AttackPotion, SkillPotion, PowerPotion, ColorlessPotion)
        # ============================================================
        if pid == 'AttackPotion':
            p['description'] = "Choose 1 of 3 random Attack cards to add to your hand."
            p['potency'] = 1
            p['potency_upgraded'] = 2
            
        elif pid == 'SkillPotion':
            p['description'] = "Choose 1 of 3 random Skill cards to add to your hand."
            p['potency'] = 1
            p['potency_upgraded'] = 2
            
        elif pid == 'PowerPotion':
            p['description'] = "Choose 1 of 3 random Power cards to add to your hand."
            p['potency'] = 1
            p['potency_upgraded'] = 2
            
        elif pid == 'ColorlessPotion':
            p['description'] = "Choose 1 of 3 random Colorless cards to add to your hand."
            p['potency'] = 1
            p['potency_upgraded'] = 2
        
        # ============================================================
        # 2. Bottled Miracle (Watcher - adds Miracles)
        # ============================================================
        elif pid == 'BottledMiracle':
            p['description'] = "Add 2 Miracles to your hand."
            p['potency'] = 2
            p['potency_upgraded'] = 4  # Sacred Bark doubles
            
        # ============================================================
        # 3. Cunning Potion (Silent - adds Shivs)
        # ============================================================
        elif pid == 'CunningPotion':
            p['description'] = "Add 3 Shivs to your hand."
            p['potency'] = 3
            p['potency_upgraded'] = 6  # Sacred Bark doubles
            
        # ============================================================
        # 4. Fruit Juice - GainMaxHP, NOT Heal
        # ============================================================
        elif pid == 'FruitJuice':
            p['command_hint'] = "GainMaxHP"
            p['description'] = "Gain 5 Max HP."
            # potency already 5, potency_upgraded already 10
            
        # ============================================================
        # 5. Snecko Oil - Special combined effect
        # ============================================================
        elif pid == 'SneckoOil':
            p['command_hint'] = "SneckoEffect"
            p['description'] = "Draw 5 cards. Randomize the costs of all cards in your hand for the rest of combat."
            # potency already 5 (draw count), potency_upgraded already 10
            
        # ============================================================
        # 6. Blood Potion - Percent heal (distinct from flat heal)
        # ============================================================
        elif pid == 'BloodPotion':
            p['command_hint'] = "HealPercent"
            # potency_percent already 20, potency_percent_upgraded already 40
            
        # ============================================================
        # 7. Liquid Memories - needs potency for "costs 0" duration
        # ============================================================
        elif pid == 'LiquidMemories':
            p['potency'] = 1  # Recall 1 card
            p['potency_upgraded'] = 2  # Sacred Bark: recall 2 cards? (Actually still 1, but cost 0)
            # Note: Sacred Bark doesn't change the card count, but we keep this for consistency
            
        # ============================================================
        # 8. Distilled Chaos - clarify it PLAYS cards, not just draws
        # ============================================================
        elif pid == 'DistilledChaos':
            p['command_hint'] = "PlayFromDraw"
            p['description'] = "Play the top 3 cards of your draw pile."
            # potency 3, potency_upgraded 6
            
        # ============================================================
        # 9. Gambler's Brew - needs special command (discard then draw)
        # ============================================================
        elif pid == "Gambler'sBrew":
            p['command_hint'] = "GamblerDraw"
            p['description'] = "Discard any number of cards, then draw that many."
            p['potency'] = 0  # Variable
            p['potency_upgraded'] = 0
            
        # ============================================================
        # 10. Elixir - exhaust any number
        # ============================================================
        elif pid == 'Elixir':
            p['potency'] = 0  # Variable (any number)
            p['potency_upgraded'] = 0
            
        # ============================================================
        # 11. Smoke Bomb - escape combat
        # ============================================================
        elif pid == 'SmokeBomb':
            p['potency'] = 1  # Flag: can escape
            p['potency_upgraded'] = 1
            
        # ============================================================
        # 12. Entropic Brew - fill potion slots
        # ============================================================
        elif pid == 'EntropicBrew':
            p['potency'] = 0  # Variable (fills empty slots)
            p['potency_upgraded'] = 0
            
        # ============================================================
        # 13. Duplication Potion - next card played twice
        # ============================================================
        elif pid == 'DuplicationPotion':
            p['potency'] = 1  # 1 duplication
            p['potency_upgraded'] = 2  # 2 duplications with Sacred Bark
            
        # ============================================================
        # 14. Stance Potion - enter Calm or Wrath
        # ============================================================
        elif pid == 'StancePotion':
            p['potency'] = 1  # Choose 1 stance
            p['potency_upgraded'] = 1
            
        # ============================================================
        # 15. Ambrosia - enter Divinity
        # ============================================================
        elif pid == 'Ambrosia':
            p['potency'] = 1  # Enter divinity
            p['potency_upgraded'] = 1
            
        # ============================================================
        # 16. Essence of Darkness - channel Dark for each orb slot
        # ============================================================
        elif pid == 'EssenceofDarkness':
            p['command_hint'] = "ChannelDark"
            p['description'] = "Channel 1 Dark for each Orb slot."
            # potency 1 per slot, upgraded 2 per slot
            
        # Track changes
        if p != original:
            patched_count += 1
            print(f"  Patched: {pid}")

    # Write back
    with open(FILE_PATH, 'w', encoding='utf-8') as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ Patched {patched_count} potions in {FILE_PATH}")
    print(f"   Total potions: {len(data)}")

if __name__ == "__main__":
    patch_data()
