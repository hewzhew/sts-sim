"""Auto-fix deprecated parameter names in card JSON files.

Fixes the 7 cards using old damage_base/hits_base format:
  damage_base → base
  damage_upgrade → upgrade
  hits_base → times
  hits_upgrade → times_upgrade

These are MultiHit/DealDamageRandom commands that should use the standard format.
"""
import json
import os

RENAMES = {
    "damage_base": "base",
    "damage_upgrade": "upgrade",
    "hits_base": "times",
    "hits_upgrade": "times_upgrade",
}

DATA_DIR = "data/cards"
fixed_count = 0

for root, _, files in os.walk(DATA_DIR):
    for fname in sorted(files):
        if not fname.endswith('.json'):
            continue
        
        path = os.path.join(root, fname)
        with open(path, 'r', encoding='utf-8') as f:
            cards = json.load(f)
        
        modified = False
        for card in cards:
            for cmd in card.get('logic', {}).get('commands', []):
                params = cmd.get('params', {})
                for old_name, new_name in RENAMES.items():
                    if old_name in params:
                        params[new_name] = params.pop(old_name)
                        modified = True
                        fixed_count += 1
                        print(f"  Fixed [{card['id']}] {old_name} → {new_name} in {fname}")
        
        if modified:
            with open(path, 'w', encoding='utf-8') as f:
                json.dump(cards, f, indent=2, ensure_ascii=False)
                f.write('\n')

print(f"\nTotal fixes: {fixed_count}")
