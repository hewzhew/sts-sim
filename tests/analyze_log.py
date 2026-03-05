"""
Feasibility probe: Check real JSONL data fields against what the Rust parser expects.
Identifies card ID mismatches, unknown power IDs, missing fields, etc.
"""
import json
from pathlib import Path
from collections import Counter

f = Path(r'c:\Dev\rust\sts_sim\tests\fixtures\real_game_floor51.jsonl')
lines = f.read_text(encoding='utf-8').strip().split('\n')

# Collect all unique values for each field type
card_ids = Counter()
power_ids = Counter()
relic_ids = Counter()
monster_ids = Counter()
orb_ids = Counter()
screen_types = Counter()
commands = Counter()
intents = Counter()

# Track field presence
field_issues = []
combat_count = 0
non_combat_count = 0

for i, line in enumerate(lines):
    e = json.loads(line)
    gs = e['state'].get('game_state')
    if not gs:
        non_combat_count += 1
        continue
    
    cmd = e.get('command', '')
    cmd_type = cmd.split()[0] if cmd else 'empty'
    commands[cmd_type] += 1
    screen_types[gs.get('screen_type', 'MISSING')] += 1
    
    # Relics
    for r in gs.get('relics', []):
        relic_ids[r.get('id', 'MISSING')] += 1
    
    cs = gs.get('combat_state')
    if not cs:
        non_combat_count += 1
        continue
    combat_count += 1
    
    # Cards in all piles
    for pile_name in ['hand', 'draw_pile', 'discard_pile', 'exhaust_pile']:
        for c in cs.get(pile_name, []):
            cid = c.get('id', 'MISSING')
            card_ids[cid] += 1
    
    # Player powers
    for p in cs.get('player', {}).get('powers', []):
        power_ids[p.get('id', 'MISSING')] += 1
    
    # Monster data
    for m in cs.get('monsters', []):
        monster_ids[m.get('id', 'MISSING')] += 1
        intents[m.get('intent', 'MISSING')] += 1
        for p in m.get('powers', []):
            power_ids[p.get('id', 'MISSING')] += 1
    
    # Orbs
    for o in cs.get('player', {}).get('orbs', []):
        orb_ids[o.get('id', 'MISSING')] += 1
    
    # Check for unexpected fields or missing expected fields
    player = cs.get('player', {})
    expected_player_fields = {'current_hp', 'max_hp', 'block', 'energy', 'powers', 'orbs'}
    actual_player_fields = set(player.keys())
    extra = actual_player_fields - expected_player_fields
    missing = expected_player_fields - actual_player_fields
    if extra or missing:
        field_issues.append(f'Step {i}: player extra={extra} missing={missing}')

print(f'=== FEASIBILITY ANALYSIS: Floor 51 Run ===\n')
print(f'Total lines: {len(lines)}')
print(f'Combat steps: {combat_count}, Non-combat: {non_combat_count}')

print(f'\n--- CARD IDs ({len(card_ids)} unique) ---')
for cid, count in card_ids.most_common():
    # Check if it has class suffix
    suffix = ''
    if cid.endswith(('_R', '_G', '_B', '_P')):
        suffix = ' [HAS SUFFIX]'
    print(f'  {cid}: {count}{suffix}')

print(f'\n--- POWER IDs ({len(power_ids)} unique) ---')
for pid, count in power_ids.most_common():
    print(f'  {pid}: {count}')

print(f'\n--- MONSTER IDs ({len(monster_ids)} unique) ---')
for mid, count in monster_ids.most_common():
    print(f'  {mid}: {count}')

print(f'\n--- INTENTS ({len(intents)} unique) ---')
for intent, count in intents.most_common():
    print(f'  {intent}: {count}')

print(f'\n--- RELIC IDs ({len(relic_ids)} unique) ---')
for rid, count in relic_ids.most_common(15):
    print(f'  {rid}: {count}')

print(f'\n--- COMMANDS ({len(commands)} unique) ---')
for cmd, count in commands.most_common():
    print(f'  {cmd}: {count}')

print(f'\n--- SCREEN TYPES ({len(screen_types)} unique) ---')
for st, count in screen_types.most_common():
    print(f'  {st}: {count}')

if orb_ids:
    print(f'\n--- ORB IDs ({len(orb_ids)} unique) ---')
    for oid, count in orb_ids.most_common():
        print(f'  {oid}: {count}')

if field_issues:
    print(f'\n--- FIELD ISSUES ({len(field_issues)}) ---')
    for issue in field_issues[:10]:
        print(f'  {issue}')

# Check specific mapping concerns
print(f'\n\n=== KEY MAPPING CONCERNS ===')
# 1. Card IDs with spaces (CommunicationMod uses spaces, Rust may use different format)
space_cards = [c for c in card_ids if ' ' in c]
print(f'\nCards with spaces in ID ({len(space_cards)}):')
for c in sorted(space_cards):
    print(f'  "{c}"')

# 2. Cards with + suffix (upgraded)
plus_cards = [c for c in card_ids if c.endswith('+')]
print(f'\nUpgraded cards (+ suffix) ({len(plus_cards)}):')
for c in sorted(plus_cards):
    print(f'  "{c}"')

# 3. Check if 'upgrades' field uses 0/1 or the ID has + 
print(f'\n--- UPGRADE FORMAT CHECK ---')
sample_line = json.loads(lines[10])
sample_gs = sample_line['state'].get('game_state', {})
sample_cs = sample_gs.get('combat_state')
if sample_cs:
    for c in sample_cs.get('hand', [])[:3]:
        print(f'  Card: id="{c.get("id")}" name="{c.get("name")}" upgrades={c.get("upgrades")} cost={c.get("cost")}')
