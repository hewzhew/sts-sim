"""
Extract move data (damage, block, buffs, debuffs, ascension scaling) from Java source.
Generates a comprehensive monsters_verified.json with full move data.

Strategy: Since Java constructors use varied patterns (member vars, constants, inline values),
we take a semi-automated approach:
1. Auto-extract what we can (HP, damage constants, member var assignments)
2. Parse damage.add() indices to map damage values to slot indices
3. Parse takeTurn() to connect move byte IDs to damage slots and effects
"""
import os, re, json

JAVA_DIR = r"C:\Dev\rust\cardcrawl\monsters"

# Load our monster map from the existing extraction
MONSTER_MAP = {
    "AcidSlime_L":        ("AcidSlime_L",     "AcidSlime_L",     "Acid Slime (L)",    1),
    "AcidSlime_M":        ("AcidSlime_M",     "AcidSlime_M",     "Acid Slime (M)",    1),
    "AcidSlime_S":        ("AcidSlime_S",     "AcidSlime_S",     "Acid Slime (S)",    1),
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
    "Serpent":            ("SpireGrowth",     "Serpent",         "Spire Growth",      3),
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
    hp_calls_range = re.findall(r'this\.setHp\((\d+),\s*(\d+)\)', content)
    hp_calls_fixed = re.findall(r'this\.setHp\((\d+)\)', content)
    super_hp = re.findall(r'super\([^,]+,\s*[^,]+,\s*(\d+)', content)
    
    const_hp = {}
    for m in re.finditer(r'(?:public|private)\s+static\s+final\s+int\s+([\w_]*HP[\w_]*|MAX_HEALTH)\s*=\s*(\d+)', content):
        const_hp[m.group(1)] = int(m.group(2))
    
    asc_hp_range = re.findall(r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+),\s*(\d+)\)', content)
    asc_hp_fixed = re.findall(r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+)\)', content)
    
    if asc_hp_range:
        for asc_level, hp_min, hp_max in asc_hp_range:
            hp_data[f"asc{asc_level}"] = [int(hp_min), int(hp_max)]
    if hp_calls_range:
        if len(hp_calls_range) > 1 and asc_hp_range:
            hp_data["base"] = [int(hp_calls_range[-1][0]), int(hp_calls_range[-1][1])]
        else:
            hp_data["base"] = [int(hp_calls_range[0][0]), int(hp_calls_range[0][1])]
    
    if not hp_data:
        if asc_hp_fixed:
            for asc_level, hp_val in asc_hp_fixed:
                hp_data[f"asc{asc_level}"] = [int(hp_val), int(hp_val)]
        if hp_calls_fixed:
            if len(hp_calls_fixed) > 1 and asc_hp_fixed:
                hp_data["base"] = [int(hp_calls_fixed[-1]), int(hp_calls_fixed[-1])]
            else:
                hp_data["base"] = [int(hp_calls_fixed[0]), int(hp_calls_fixed[0])]
    
    if not hp_data:
        if const_hp:
            hp_val = const_hp.get("HP", const_hp.get("MAX_HEALTH", list(const_hp.values())[0]))
            hp_data["base"] = [hp_val, hp_val]
            for k, v in const_hp.items():
                if "A_" in k and v != hp_val:
                    asc_match = re.search(r'A_(\d+)', k)
                    if asc_match:
                        hp_data[f"asc{asc_match.group(1)}"] = [v, v]
        elif super_hp:
            hp_data["base"] = [int(super_hp[0]), int(super_hp[0])]
    
    return hp_data

def extract_damage_slots(content):
    """Extract damage slot values from this.damage.add() calls.
    Returns list of {slot_index, value_expr, ascension_tiers}.
    """
    slots = []
    
    # Find all damage.add calls in constructor
    # Handles both DamageInfo(this, VALUE) and DamageInfo(this, VALUE, DamageType)
    damage_adds = re.findall(
        r'this\.damage\.add\(new DamageInfo\(this,\s*(?:this\.)?(\w+)(?:\s*,\s*DamageInfo\.DamageType\.\w+)?\)',
        content
    )
    
    return damage_adds

def extract_all_constants(content):
    """Extract all named constants (public/private static final int)."""
    constants = {}
    for m in re.finditer(r'(?:public|private)\s+static\s+final\s+int\s+(\w+)\s*=\s*(\d+)', content):
        constants[m.group(1)] = int(m.group(2))
    return constants

def extract_member_assignments(content):
    """Extract this.xxx = N assignments from constructor, grouped by ascension tier."""
    # Find ascension-gated blocks
    tiers = {}
    
    # Find base (else block or ungated) assignments
    base_assigns = {}
    asc_assigns = {}
    
    # Simple approach: find all this.X = N in the constructor 
    constructor = content
    
    # Find all ascension blocks: if (ascensionLevel >= N) { ... }
    asc_blocks = re.finditer(
        r'ascensionLevel\s*>=\s*(\d+)\)\s*\{([^{}]*(?:\{[^{}]*\}[^{}]*)*)\}',
        constructor
    )
    
    for m in asc_blocks:
        asc_level = int(m.group(1))
        block_content = m.group(2)
        assigns = {}
        for am in re.finditer(r'this\.(\w+)\s*=\s*(\d+)\s*;', block_content):
            assigns[am.group(1)] = int(am.group(2))
        if assigns:
            asc_assigns[f"asc{asc_level}"] = assigns
    
    # Base: assignments not in any ascension block (crude: all assignments)
    for am in re.finditer(r'this\.(\w+)\s*=\s*(\d+)\s*;', constructor):
        name = am.group(1)
        val = int(am.group(2))
        # Only track damage/block-like vars
        if any(kw in name.lower() for kw in ['dmg', 'damage', 'block', 'str', 'amt', 'count', 'forge']):
            base_assigns[name] = val
    
    return base_assigns, asc_assigns

def parse_monster(java_id, filepath):
    """Parse a single Java monster file — full extraction."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    
    if java_id not in MONSTER_MAP:
        return None
    
    rust_id, _, display_name, act = MONSTER_MAP[java_id]
    
    hp = extract_hp(content)
    constants = extract_all_constants(content)
    damage_slots = extract_damage_slots(content)
    base_assigns, asc_assigns = extract_member_assignments(content)
    
    # Build damage slot table
    # Each slot is a value - could be a constant name, member var, or inline int
    damage_values = []
    for slot_expr in damage_slots:
        # Try to resolve to a number
        try:
            val = int(slot_expr)
            damage_values.append(val)
        except ValueError:
            # It's a member variable name — look up in assignments
            if slot_expr in base_assigns:
                damage_values.append(base_assigns[slot_expr])
            elif slot_expr in constants:
                damage_values.append(constants[slot_expr])
            else:
                damage_values.append(f"${slot_expr}")  # unresolved
    
    # Build ascension damage tiers
    asc_damage_tiers = {}
    for tier_name, assigns in asc_assigns.items():
        tier_dmgs = {}
        for var_name, var_val in assigns.items():
            # Map var names to slot indices via damage_slots
            for i, slot_expr in enumerate(damage_slots):
                if slot_expr == var_name:
                    tier_dmgs[f"slot{i}"] = var_val
        if tier_dmgs:
            asc_damage_tiers[tier_name] = tier_dmgs
    
    return {
        "id": rust_id,
        "java_id": java_id,
        "display_name": display_name,
        "act": act,
        "hp": hp,
        "damage_slots": damage_values,
        "constants": constants,
        "base_vars": base_assigns,
        "asc_vars": asc_assigns,
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

# Generate compact JSON output
with open("tests/java_full_extraction.json", "w", encoding="utf-8") as f:
    json.dump(results, f, indent=2, ensure_ascii=False)

# Summary stats
print(f"Extracted {len(results)} monsters")
has_dmg = sum(1 for r in results if r["damage_slots"])
has_asc = sum(1 for r in results if r["asc_vars"])
has_consts = sum(1 for r in results if r["constants"])
print(f"Has damage slots: {has_dmg}")
print(f"Has ascension vars: {has_asc}")
print(f"Has constants: {has_consts}")

# Show a few interesting ones
for r in results:
    if r["id"] in ["JawWorm", "Champ", "Cultist", "GremlinNob"]:
        print(f"\n{r['id']}:")
        print(f"  HP: {r['hp']}")
        print(f"  Damage slots: {r['damage_slots']}")
        print(f"  Base vars: {r['base_vars']}")
        print(f"  Asc vars: {r['asc_vars']}")
        print(f"  Constants: {r['constants']}")
