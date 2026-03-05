"""Extract key data from ALL Java monster files to accelerate JSON creation.
Outputs a structured summary for each monster that I can verify and convert to JSON."""
import re, os, json

JAVA_DIR = r"c:\Dev\rust\cardcrawl\monsters"
# Map of Java filename → Rust ID (from monster_id.rs)
JAVA_TO_RUST = {
    "AcidSlime_L": "AcidSlime_L",
    "AcidSlime_M": "AcidSlime_M", 
    "AcidSlime_S": "AcidSlime_S",
    "Cultist": "Cultist",
    "GremlinFat": "FatGremlin",
    "FungiBeast": "FungiBeast",
    "LouseDefensive": "GreenLouse",
    "JawWorm": "JawWorm",
    "Looter": "Looter",
    "GremlinWarrior": "MadGremlin",
    "LouseNormal": "RedLouse",
    "GremlinThief": "SneakyGremlin",
    "GremlinTsundere": "ShieldGremlin",
    "GremlinWizard": "GremlinWizard",
    "SpikeSlime_L": "SpikeSlime_L",
    "SpikeSlime_M": "SpikeSlime_M",
    "SpikeSlime_S": "SpikeSlime_S",
    "SlaverBlue": "BlueSlaver",
    "SlaverRed": "RedSlaver",
    "GremlinNob": "GremlinNob",
    "Lagavulin": "Lagavulin",
    "Sentry": "Sentry",
    "TheGuardian": "Guardian",
    "Hexaghost": "Hexaghost",
    "SlimeBoss": "SlimeBoss",
    "BanditBear": "Bear",
    "BookOfStabbing": "BookOfStabbing",
    "BronzeAutomaton": "BronzeAutomaton",
    "BronzeOrb": "BronzeOrb",
    "Byrd": "Byrd",
    "Centurion": "Centurion",
    "Chosen": "Chosen",
    "Mugger": "Mugger",
    "Healer": "Mystic",
    "BanditPointy": "Pointy",
    "BanditLeader": "Romeo",
    "ShelledParasite": "ShelledParasite",
    "SnakePlant": "SnakePlant",
    "Snecko": "Snecko",
    "SphericGuardian": "SphericGuardian",
    "Taskmaster": "Taskmaster",
    "TheCollector": "Collector",
    "TorchHead": "TorchHead",
    "GremlinLeader": "GremlinLeader",
    "Champ": "Champ",
    "Darkling": "Darkling",
    "Exploder": "Exploder",
    "Maw": "Maw",
    "OrbWalker": "OrbWalker",
    "Repulsor": "Repulsor",
    "Spiker": "Spiker",
    "SpireGrowth": "SpireGrowth",
    "Transient": "Transient",
    "WrithingMass": "WrithingMass",
    "GiantHead": "GiantHead",
    "Nemesis": "Nemesis",
    "Reptomancer": "Reptomancer",
    "AwakenedOne": "AwakenedOne",
    "Deca": "Deca",
    "Donu": "Donu",
    "TimeEater": "TimeEater",
    "CorruptHeart": "CorruptHeart",
    "SpireShield": "SpireShield",
    "SpireSpear": "SpireSpear",
    "SnakeDagger": "Dagger",
}

def extract_monster_data(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()
    
    data = {}
    
    # Extract HP setHp calls
    hp_pattern = r'this\.setHp\((\d+)(?:,\s*(\d+))?\)'
    hp_matches = re.findall(hp_pattern, content)
    if hp_matches:
        data['hp_calls'] = [(int(a), int(b) if b else int(a)) for a, b in hp_matches]
    
    # Extract damage.add calls  
    dmg_pattern = r'new DamageInfo\(this,\s*(\d+)\)'
    dmg_matches = re.findall(dmg_pattern, content)
    if dmg_matches:
        data['damage_slots'] = [int(d) for d in dmg_matches]
    
    # Extract byte constants (move IDs)
    byte_pattern = r'private static final byte (\w+)\s*=\s*(\d+);'
    byte_matches = re.findall(byte_pattern, content)
    if byte_matches:
        data['move_bytes'] = {name: int(val) for name, val in byte_matches}
    
    # Extract ascension level checks
    asc_pattern = r'ascensionLevel\s*>=\s*(\d+)'
    asc_matches = re.findall(asc_pattern, content)
    if asc_matches:
        data['asc_levels'] = sorted(set(int(a) for a in asc_matches))
    
    # Extract ApplyPowerAction calls
    power_pattern = r'new (\w+Power)\((?:this|AbstractDungeon\.player),\s*(?:this|AbstractDungeon\.player)?,?\s*(\d+)?\)'
    # Simpler: look for Power class names
    power_names = re.findall(r'import.*powers\.(\w+Power);', content)
    if power_names:
        data['powers_used'] = power_names
    
    # Extract card actions
    card_pattern = r'MakeTempCardIn(\w+)Action\(.*?new (\w+)\(\),?\s*(\d+)?\)'
    card_matches = re.findall(card_pattern, content)
    if card_matches:
        data['card_actions'] = [(dest, card, int(amt) if amt else 1) for dest, card, amt in card_matches]
    
    # Check for usePreBattleAction
    if 'usePreBattleAction' in content:
        data['has_pre_battle'] = True
    
    # Extract Java ID
    id_pattern = r'public static final String ID = "(\w+)"'
    id_match = re.search(id_pattern, content)
    if id_match:
        data['java_id'] = id_match.group(1)
    
    # Extract type
    if 'EnemyType.ELITE' in content:
        data['type'] = 'elite'
    elif 'EnemyType.BOSS' in content:
        data['type'] = 'boss'
    else:
        data['type'] = 'normal'
    
    return data

# Collect all data
results = {}
for subdir in ['exordium', 'city', 'beyond', 'ending']:
    dirpath = os.path.join(JAVA_DIR, subdir)
    if not os.path.isdir(dirpath):
        continue
    for fname in sorted(os.listdir(dirpath)):
        if not fname.endswith('.java'):
            continue
        basename = fname[:-5]
        if basename in JAVA_TO_RUST:
            rust_id = JAVA_TO_RUST[basename]
            filepath = os.path.join(dirpath, fname)
            data = extract_monster_data(filepath)
            data['java_file'] = basename
            data['rust_id'] = rust_id
            results[rust_id] = data

# Print organized output
ACT_ORDER = {
    1: {'normal': [], 'elite': [], 'boss': []},
    2: {'normal': [], 'elite': [], 'boss': []},
    3: {'normal': [], 'elite': [], 'boss': []},
    4: {'normal': [], 'elite': [], 'boss': []},
}

# Assign acts based on directory
ACT_MAP = {
    'AcidSlime_L': 1, 'AcidSlime_M': 1, 'AcidSlime_S': 1, 'Cultist': 1,
    'FatGremlin': 1, 'FungiBeast': 1, 'GreenLouse': 1, 'JawWorm': 1,
    'Looter': 1, 'MadGremlin': 1, 'RedLouse': 1, 'SneakyGremlin': 1,
    'ShieldGremlin': 1, 'GremlinWizard': 1, 'SpikeSlime_L': 1, 'SpikeSlime_M': 1,
    'SpikeSlime_S': 1, 'BlueSlaver': 1, 'RedSlaver': 1,
    'GremlinNob': 1, 'Lagavulin': 1, 'Sentry': 1,
    'Guardian': 1, 'Hexaghost': 1, 'SlimeBoss': 1,
    'Bear': 2, 'BookOfStabbing': 2, 'BronzeAutomaton': 2, 'BronzeOrb': 2,
    'Byrd': 2, 'Centurion': 2, 'Chosen': 2, 'Mugger': 2, 'Mystic': 2,
    'Pointy': 2, 'Romeo': 2, 'ShelledParasite': 2, 'SnakePlant': 2,
    'Snecko': 2, 'SphericGuardian': 2, 'Taskmaster': 2, 'TorchHead': 2,
    'GremlinLeader': 2, 'Champ': 2, 'Collector': 2,
    'Darkling': 3, 'Exploder': 3, 'Maw': 3, 'OrbWalker': 3, 'Repulsor': 3,
    'Spiker': 3, 'SpireGrowth': 3, 'Transient': 3, 'WrithingMass': 3,
    'GiantHead': 3, 'Nemesis': 3, 'Reptomancer': 3,
    'AwakenedOne': 3, 'Deca': 3, 'Donu': 3, 'TimeEater': 3,
    'CorruptHeart': 4, 'SpireShield': 4, 'SpireSpear': 4, 'Dagger': 3,
}

for rust_id, data in sorted(results.items(), key=lambda x: (ACT_MAP.get(x[0], 9), x[0])):
    act = ACT_MAP.get(rust_id, '?')
    tp = data.get('type', 'normal')
    print(f"\n{'='*60}")  
    print(f"  {rust_id} (java: {data.get('java_id','?')}) | Act {act} | {tp}")
    print(f"{'='*60}")
    if 'hp_calls' in data:
        print(f"  HP: {data['hp_calls']}")
    if 'damage_slots' in data:
        print(f"  Damage slots: {data['damage_slots']}")
    if 'move_bytes' in data:
        print(f"  Move bytes: {data['move_bytes']}")
    if 'asc_levels' in data:
        print(f"  Asc levels: {data['asc_levels']}")
    if 'powers_used' in data:
        print(f"  Powers: {data['powers_used']}")
    if 'card_actions' in data:
        print(f"  Card adds: {data['card_actions']}")
    if data.get('has_pre_battle'):
        print(f"  Has pre_battle: YES")

print(f"\n\nTotal monsters extracted: {len(results)}")
print(f"Missing: {set(JAVA_TO_RUST.values()) - set(results.keys())}")
