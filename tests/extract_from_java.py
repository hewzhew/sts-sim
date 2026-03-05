"""
Extract monster static data from Java source to generate minimal JSON.

Extracts:
- Monster ID and display name
- HP ranges (base + ascension breakpoints)
- Move damage, block, hits values (base + ascension scaling)
- Initial powers (usePreBattleAction)

Output: data/monsters_verified.json
"""
import os, re, json, sys

JAVA_DIR = r"C:\Dev\rust\cardcrawl\monsters"

# Rust canonical ID mapping (display_name -> rust_id, java_id)
# From implementation_plan.md
MONSTER_MAP = {
    "AcidSlime_L":        ("AcidSlime_L",     "AcidSlime_L",     "Acid Slime (L)",    1),
    "AcidSlime_M":        ("AcidSlime_M",     "AcidSlime_M",     "Acid Slime (M)",    1),
    "AcidSlime_S":        ("AcidSlime_S",     "AcidSlime_S",     "Acid Slime (S)",    1),
    "Apology Slime":      ("ApologySlime",    "Apology Slime",   "Apology Slime",     1),
    "AwakenedOne":        ("AwakenedOne",     "AwakenedOne",     "Awakened One",      3),
    "BanditBear":         ("Bear",            "BanditBear",      "Bear",              2),
    "BanditChild":        ("Pointy",          "BanditChild",     "Pointy",            2),
    "BanditLeader":       ("Romeo",           "BanditLeader",    "Romeo",             2),
    "BookOfStabbing":     ("BookOfStabbing",  "BookOfStabbing",  "Book of Stabbing",  2),
    "BronzeAutomaton":    ("BronzeAutomaton", "BronzeAutomaton", "Bronze Automaton",  2),
    "BronzeOrb":          ("BronzeOrb",       "BronzeOrb",       "Bronze Orb",        2),
    "Byrd":               ("Byrd",            "Byrd",            "Byrd",              2),
    "Centurion":          ("Centurion",       "Centurion",       "Centurion",         2),
    "Champ":              ("Champ",           "Champ",           "The Champ",         2),
    "Chosen":             ("Chosen",          "Chosen",          "Chosen",            2),
    "CorruptHeart":       ("CorruptHeart",    "CorruptHeart",    "Corrupt Heart",     4),
    "Cultist":            ("Cultist",         "Cultist",         "Cultist",           1),
    "Dagger":             ("Dagger",          "Dagger",          "Dagger",            3),
    "Darkling":           ("Darkling",        "Darkling",        "Darkling",          3),
    "Deca":               ("Deca",            "Deca",            "Deca",              3),
    "Donu":               ("Donu",            "Donu",            "Donu",              3),
    "Exploder":           ("Exploder",        "Exploder",        "Exploder",          3),
    "FungiBeast":         ("FungiBeast",      "FungiBeast",      "Fungi Beast",       1),
    "FuzzyLouseDefensive":("GreenLouse",      "FuzzyLouseDefensive","Green Louse",    1),
    "FuzzyLouseNormal":   ("RedLouse",        "FuzzyLouseNormal","Red Louse",         1),
    "GiantHead":          ("GiantHead",       "GiantHead",       "Giant Head",        3),
    "GremlinFat":         ("FatGremlin",      "GremlinFat",      "Fat Gremlin",       1),
    "GremlinLeader":      ("GremlinLeader",   "GremlinLeader",   "Gremlin Leader",    2),
    "GremlinNob":         ("GremlinNob",      "GremlinNob",      "Gremlin Nob",       1),
    "GremlinThief":       ("SneakyGremlin",   "GremlinThief",    "Sneaky Gremlin",    1),
    "GremlinTsundere":    ("ShieldGremlin",   "GremlinTsundere", "Shield Gremlin",    1),
    "GremlinWarrior":     ("MadGremlin",      "GremlinWarrior",  "Mad Gremlin",       1),
    "GremlinWizard":      ("GremlinWizard",   "GremlinWizard",   "Gremlin Wizard",    1),
    "Healer":             ("Mystic",          "Healer",          "Mystic",            2),
    "Hexaghost":          ("Hexaghost",       "Hexaghost",       "Hexaghost",         1),
    "JawWorm":            ("JawWorm",         "JawWorm",         "Jaw Worm",          1),
    "Lagavulin":          ("Lagavulin",       "Lagavulin",       "Lagavulin",         1),
    "Looter":             ("Looter",          "Looter",          "Looter",            1),
    "Maw":                ("Maw",             "Maw",             "The Maw",           3),
    "Mugger":             ("Mugger",          "Mugger",          "Mugger",            2),
    "Nemesis":            ("Nemesis",         "Nemesis",         "Nemesis",           3),
    "Orb Walker":         ("OrbWalker",       "Orb Walker",      "Orb Walker",        3),
    "Reptomancer":        ("Reptomancer",     "Reptomancer",     "Reptomancer",       3),
    "Repulsor":           ("Repulsor",        "Repulsor",        "Repulsor",          3),
    "Sentry":             ("Sentry",          "Sentry",          "Sentry",            1),
    "Serpent":             ("SpireGrowth",     "Serpent",         "Spire Growth",      3),
    "Shelled Parasite":   ("ShelledParasite", "Shelled Parasite","Shelled Parasite",  2),
    "SlaverBlue":         ("BlueSlaver",      "SlaverBlue",      "Blue Slaver",       1),
    "SlaverBoss":         ("Taskmaster",      "SlaverBoss",      "Taskmaster",        2),
    "SlaverRed":          ("RedSlaver",       "SlaverRed",       "Red Slaver",        1),
    "SlimeBoss":          ("SlimeBoss",       "SlimeBoss",       "Slime Boss",        1),
    "SnakePlant":         ("SnakePlant",      "SnakePlant",      "Snake Plant",       2),
    "Snecko":             ("Snecko",          "Snecko",          "Snecko",            2),
    "SphericGuardian":    ("SphericGuardian", "SphericGuardian", "Spheric Guardian",  2),
    "SpikeSlime_L":       ("SpikeSlime_L",    "SpikeSlime_L",    "Spike Slime (L)",   1),
    "SpikeSlime_M":       ("SpikeSlime_M",    "SpikeSlime_M",    "Spike Slime (M)",   1),
    "SpikeSlime_S":       ("SpikeSlime_S",    "SpikeSlime_S",    "Spike Slime (S)",   1),
    "Spiker":             ("Spiker",          "Spiker",          "Spiker",            3),
    "SpireShield":        ("SpireShield",     "SpireShield",     "Spire Shield",      4),
    "SpireSpear":         ("SpireSpear",      "SpireSpear",      "Spire Spear",       4),
    "TheCollector":       ("Collector",       "TheCollector",    "The Collector",     2),
    "TheGuardian":        ("Guardian",        "TheGuardian",     "The Guardian",      1),
    "TimeEater":          ("TimeEater",       "TimeEater",       "Time Eater",        3),
    "TorchHead":          ("TorchHead",       "TorchHead",       "Torch Head",        2),
    "Transient":          ("Transient",       "Transient",       "Transient",         3),
    "WrithingMass":       ("WrithingMass",    "WrithingMass",    "Writhing Mass",     3),
}

def extract_hp(content):
    """Extract HP ranges from Java constructor."""
    hp_data = {}
    
    # Pattern 1: this.setHp(min, max) — normal monsters
    hp_calls_range = re.findall(r'this\.setHp\((\d+),\s*(\d+)\)', content)
    
    # Pattern 2: this.setHp(N) — bosses with fixed HP
    hp_calls_fixed = re.findall(r'this\.setHp\((\d+)\)', content)
    
    # Pattern 3: super(..., N, ...) where N is in the 3rd position (maxHealth in AbstractMonster)
    # super(NAME, ID, 420, ...)
    super_hp = re.findall(r'super\([^,]+,\s*[^,]+,\s*(\d+)', content)
    
    # Pattern 4: public static final int HP = N
    const_hp = {}
    for m in re.finditer(r'(?:public|private)\s+static\s+final\s+int\s+([\w_]*HP[\w_]*|MAX_HEALTH)\s*=\s*(\d+)', content):
        const_hp[m.group(1)] = int(m.group(2))
    
    # Look for ascension-gated setHp (range version)
    asc_hp_range = re.findall(
        r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+),\s*(\d+)\)',
        content
    )
    
    # Look for ascension-gated setHp (fixed version)
    asc_hp_fixed = re.findall(
        r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+)\)',
        content
    )
    
    # Process range calls
    if asc_hp_range:
        for asc_level, hp_min, hp_max in asc_hp_range:
            hp_data[f"asc{asc_level}"] = [int(hp_min), int(hp_max)]
    
    if hp_calls_range:
        if len(hp_calls_range) > 1 and asc_hp_range:
            hp_data["base"] = [int(hp_calls_range[-1][0]), int(hp_calls_range[-1][1])]
        else:
            hp_data["base"] = [int(hp_calls_range[0][0]), int(hp_calls_range[0][1])]
    
    # Process fixed calls (only if no range calls found)
    if not hp_data:
        if asc_hp_fixed:
            for asc_level, hp_val in asc_hp_fixed:
                hp_data[f"asc{asc_level}"] = [int(hp_val), int(hp_val)]
        
        if hp_calls_fixed:
            if len(hp_calls_fixed) > 1 and asc_hp_fixed:
                hp_data["base"] = [int(hp_calls_fixed[-1]), int(hp_calls_fixed[-1])]
            else:
                hp_data["base"] = [int(hp_calls_fixed[0]), int(hp_calls_fixed[0])]
    
    # Fallback to super() constructor or constants
    if not hp_data:
        if const_hp:
            # Use the most basic HP constant
            hp_val = const_hp.get("HP", const_hp.get("MAX_HEALTH", list(const_hp.values())[0]))
            hp_data["base"] = [hp_val, hp_val]
            # Look for ascension HP constant
            for k, v in const_hp.items():
                if "A_" in k and v != hp_val:
                    # Extract asc level from name like A_9_HP
                    asc_match = re.search(r'A_(\d+)', k)
                    if asc_match:
                        hp_data[f"asc{asc_match.group(1)}"] = [v, v]
        elif super_hp:
            hp_data["base"] = [int(super_hp[0]), int(super_hp[0])]
    
    return hp_data

def extract_damage_constants(content):
    """Extract named damage/block/buff constants."""
    constants = {}
    for m in re.finditer(r'private static final int (\w+)\s*=\s*(\d+)', content):
        constants[m.group(1)] = int(m.group(2))
    return constants

def extract_move_data(content, constants):
    """Extract move damage assignments from constructor."""
    moves = {}
    
    # Pattern: this.damage.add(new DamageInfo(this, this.xxxDmg))
    # or: this.damage.add(new DamageInfo(this, N))
    damage_adds = re.findall(
        r'this\.damage\.add\(new DamageInfo\(this,\s*(?:this\.)?(\w+)\)',
        content
    )
    
    # Find member variable assignments for damage
    # Pattern: this.xxxDmg = N;
    member_vars = {}
    for m in re.finditer(r'this\.(\w+)\s*=\s*(\d+)\s*;', content):
        member_vars[m.group(1)] = int(m.group(2))
    
    return damage_adds, member_vars

def parse_monster(java_id, filepath):
    """Parse a single Java monster file."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    
    if java_id not in MONSTER_MAP:
        return None
    
    rust_id, _, display_name, act = MONSTER_MAP[java_id]
    
    hp = extract_hp(content)
    constants = extract_damage_constants(content)
    damage_adds, member_vars = extract_move_data(content, constants)
    
    return {
        "id": rust_id,
        "java_id": java_id,
        "display_name": display_name,
        "act": act,
        "hp": hp,
        "_constants": constants,
        "_damage_indices": damage_adds,
        "_member_vars": member_vars,
        "_source": os.path.basename(filepath),
    }

# Process all Java files
results = []
for root, dirs, files in os.walk(JAVA_DIR):
    for fname in sorted(files):
        if not fname.endswith(".java") or fname.startswith("Abstract") or fname.startswith("Monster"):
            continue
        fpath = os.path.join(root, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        
        id_match = re.search(r'public static final String ID = "([^"]+)"', content)
        if not id_match:
            continue
        java_id = id_match.group(1)
        
        result = parse_monster(java_id, fpath)
        if result:
            results.append(result)

# Sort by act, then name
results.sort(key=lambda r: (r["act"], r["id"]))

# Output
with open("tests/java_extraction.json", "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

# Summary
print(f"Extracted {len(results)} monsters")
hp_ok = sum(1 for r in results if r["hp"])
print(f"HP extracted: {hp_ok}/{len(results)}")

# Show a few examples
for r in results[:5]:
    print(f"  {r['id']:20s} HP={r['hp']}  consts={len(r['_constants'])} dmg_idx={r['_damage_indices']}")

print(f"\nSee tests/java_extraction.json for full data")
