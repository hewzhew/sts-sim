"""
Comprehensive monster data extractor — reads EVERY Java file and produces
a clean, verified, per-move JSON with ascension breakpoints.

Schema v3:
{
  "MonsterId": {
    "name": "Display Name",
    "java_id": "JavaInternalID",
    "type": "normal|elite|boss|minion",
    "act": 1,
    "hp": {"base": [min, max], "asc7": [min, max]},
    "moves": {
      "MoveName": {
        "damage": N,           // base damage per hit
        "hits": N,             // number of hits (omit if 1)
        "block": N,            // block gained (omit if 0)
        "effects": [           // buffs/debuffs applied
          {"type": "Strength", "amount": 3, "target": "self"},
          {"type": "Vulnerable", "amount": 2, "target": "player"}
        ]
      }
    },
    "asc_scaling": {           // changes at ascension breakpoints
      "asc2": {"moves": {"MoveName": {"damage": N}}},
      "asc17": {"moves": {"MoveName": {"damage": N, "effects": [...]}}}
    },
    "pre_battle": [            // powers applied at fight start
      {"type": "Metallicize", "amount": 8}
    ]
  }
}
"""
import os, re, json, sys

JAVA_DIR = r"C:\Dev\rust\cardcrawl\monsters"

# Rust ID ← Java ID mapping
JAVA_TO_RUST = {
    "AcidSlime_L": ("AcidSlime_L", "Acid Slime (L)", 1, "normal"),
    "AcidSlime_M": ("AcidSlime_M", "Acid Slime (M)", 1, "normal"),
    "AcidSlime_S": ("AcidSlime_S", "Acid Slime (S)", 1, "normal"),
    "AwakenedOne": ("AwakenedOne", "Awakened One", 3, "boss"),
    "BanditBear": ("Bear", "Bear", 2, "normal"),
    "BanditChild": ("Pointy", "Pointy", 2, "normal"),
    "BanditLeader": ("Romeo", "Romeo", 2, "normal"),
    "BookOfStabbing": ("BookOfStabbing", "Book of Stabbing", 2, "elite"),
    "BronzeAutomaton": ("BronzeAutomaton", "Bronze Automaton", 2, "boss"),
    "BronzeOrb": ("BronzeOrb", "Bronze Orb", 2, "minion"),
    "Byrd": ("Byrd", "Byrd", 2, "normal"),
    "Centurion": ("Centurion", "Centurion", 2, "normal"),
    "Champ": ("Champ", "The Champ", 2, "boss"),
    "Chosen": ("Chosen", "Chosen", 2, "normal"),
    "CorruptHeart": ("CorruptHeart", "Corrupt Heart", 4, "boss"),
    "Cultist": ("Cultist", "Cultist", 1, "normal"),
    "Dagger": ("Dagger", "Dagger", 3, "minion"),
    "Darkling": ("Darkling", "Darkling", 3, "normal"),
    "Deca": ("Deca", "Deca", 3, "boss"),
    "Donu": ("Donu", "Donu", 3, "boss"),
    "Exploder": ("Exploder", "Exploder", 3, "normal"),
    "FungiBeast": ("FungiBeast", "Fungi Beast", 1, "normal"),
    "FuzzyLouseDefensive": ("GreenLouse", "Green Louse", 1, "normal"),
    "FuzzyLouseNormal": ("RedLouse", "Red Louse", 1, "normal"),
    "GiantHead": ("GiantHead", "Giant Head", 3, "elite"),
    "GremlinFat": ("FatGremlin", "Fat Gremlin", 1, "normal"),
    "GremlinLeader": ("GremlinLeader", "Gremlin Leader", 2, "elite"),
    "GremlinNob": ("GremlinNob", "Gremlin Nob", 1, "elite"),
    "GremlinThief": ("SneakyGremlin", "Sneaky Gremlin", 1, "normal"),
    "GremlinTsundere": ("ShieldGremlin", "Shield Gremlin", 1, "normal"),
    "GremlinWarrior": ("MadGremlin", "Mad Gremlin", 1, "normal"),
    "GremlinWizard": ("GremlinWizard", "Gremlin Wizard", 1, "normal"),
    "Healer": ("Mystic", "Mystic", 2, "normal"),
    "Hexaghost": ("Hexaghost", "Hexaghost", 1, "boss"),
    "JawWorm": ("JawWorm", "Jaw Worm", 1, "normal"),
    "Lagavulin": ("Lagavulin", "Lagavulin", 1, "elite"),
    "Looter": ("Looter", "Looter", 1, "normal"),
    "Maw": ("Maw", "The Maw", 3, "normal"),
    "Mugger": ("Mugger", "Mugger", 2, "normal"),
    "Nemesis": ("Nemesis", "Nemesis", 3, "elite"),
    "Orb Walker": ("OrbWalker", "Orb Walker", 3, "normal"),
    "Reptomancer": ("Reptomancer", "Reptomancer", 3, "elite"),
    "Repulsor": ("Repulsor", "Repulsor", 3, "normal"),
    "Sentry": ("Sentry", "Sentry", 1, "elite"),
    "Serpent": ("SpireGrowth", "Spire Growth", 3, "normal"),
    "Shelled Parasite": ("ShelledParasite", "Shelled Parasite", 2, "normal"),
    "SlaverBlue": ("BlueSlaver", "Blue Slaver", 1, "normal"),
    "SlaverBoss": ("Taskmaster", "Taskmaster", 2, "elite"),
    "SlaverRed": ("RedSlaver", "Red Slaver", 1, "normal"),
    "SlimeBoss": ("SlimeBoss", "Slime Boss", 1, "boss"),
    "SnakePlant": ("SnakePlant", "Snake Plant", 2, "normal"),
    "Snecko": ("Snecko", "Snecko", 2, "normal"),
    "SphericGuardian": ("SphericGuardian", "Spheric Guardian", 2, "normal"),
    "SpikeSlime_L": ("SpikeSlime_L", "Spike Slime (L)", 1, "normal"),
    "SpikeSlime_M": ("SpikeSlime_M", "Spike Slime (M)", 1, "normal"),
    "SpikeSlime_S": ("SpikeSlime_S", "Spike Slime (S)", 1, "normal"),
    "Spiker": ("Spiker", "Spiker", 3, "normal"),
    "SpireShield": ("SpireShield", "Spire Shield", 4, "elite"),
    "SpireSpear": ("SpireSpear", "Spire Spear", 4, "elite"),
    "TheCollector": ("Collector", "The Collector", 2, "boss"),
    "TheGuardian": ("Guardian", "The Guardian", 1, "boss"),
    "TimeEater": ("TimeEater", "Time Eater", 3, "boss"),
    "TorchHead": ("TorchHead", "Torch Head", 2, "minion"),
    "Transient": ("Transient", "Transient", 3, "normal"),
    "WrithingMass": ("WrithingMass", "Writhing Mass", 3, "normal"),
}


def extract_hp(content):
    """Extract HP ranges — handles both setHp(min,max) and setHp(N) patterns."""
    hp = {}
    
    # Range form
    asc_range = re.findall(r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+),\s*(\d+)\)', content)
    range_calls = re.findall(r'this\.setHp\((\d+),\s*(\d+)\)', content)
    
    if asc_range:
        for lv, lo, hi in asc_range:
            hp[f"asc{lv}"] = [int(lo), int(hi)]
    if range_calls:
        base = range_calls[-1] if (len(range_calls) > 1 and asc_range) else range_calls[0]
        hp["base"] = [int(base[0]), int(base[1])]
        return hp
    
    # Fixed form
    asc_fixed = re.findall(r'ascensionLevel\s*>=\s*(\d+)\)\s*\{[^}]*?setHp\((\d+)\)', content)
    fixed_calls = re.findall(r'this\.setHp\((\d+)\)', content)
    
    if asc_fixed:
        for lv, v in asc_fixed:
            hp[f"asc{lv}"] = [int(v), int(v)]
    if fixed_calls:
        base = fixed_calls[-1] if (len(fixed_calls) > 1 and asc_fixed) else fixed_calls[0]
        hp["base"] = [int(base), int(base)]
        return hp
    
    # Constants fallback
    const_hp = {}
    for m in re.finditer(r'(?:public|private)\s+static\s+final\s+int\s+([\w_]*HP[\w_]*)\s*=\s*(\d+)', content):
        const_hp[m.group(1)] = int(m.group(2))
    if const_hp:
        base = const_hp.get("HP", const_hp.get("HP_MIN", list(const_hp.values())[0]))
        hp["base"] = [base, base]
        for k, v in const_hp.items():
            if "A_" in k:
                am = re.search(r'A_(\d+)', k)
                if am:
                    hp[f"asc{am.group(1)}"] = [v, v]
        return hp
    
    # super() fallback
    s = re.findall(r'super\([^,]+,\s*[^,]+,\s*(\d+)', content)
    if s:
        hp["base"] = [int(s[0]), int(s[0])]
    
    return hp


def parse_full(java_id, content):
    """Parse a single monster's complete data from Java source."""
    if java_id not in JAVA_TO_RUST:
        return None
    
    rust_id, display_name, act, mtype = JAVA_TO_RUST[java_id]
    hp = extract_hp(content)
    
    # ── Extract all named constants ──
    all_consts = {}
    for m in re.finditer(r'(?:public|private)\s+static\s+final\s+int\s+(\w+)\s*=\s*(\d+)', content):
        all_consts[m.group(1)] = int(m.group(2))
    
    # ── Extract damage slot values per ascension tier ──
    # Find ascension-gated constructor blocks
    tiers = []  # list of (asc_level, block_content)
    for m in re.finditer(
        r'ascensionLevel\s*>=\s*(\d+)\)\s*\{((?:[^{}]|\{[^{}]*\})*)\}',
        content):
        tiers.append((int(m.group(1)), m.group(2)))
    
    # Collect member var assignments per tier
    asc_vars = {}
    for lv, block in tiers:
        assigns = {}
        for am in re.finditer(r'this\.(\w+)\s*=\s*(\d+)\s*;', block):
            assigns[am.group(1)] = int(am.group(2))
        if assigns:
            asc_vars[lv] = assigns
    
    # Base (else block) — find the LAST set of assignments
    # Simple: collect all this.X = N from constructor
    base_vars = {}
    for am in re.finditer(r'this\.(\w+)\s*=\s*(\d+)\s*;', content):
        base_vars[am.group(1)] = int(am.group(2))
    
    # ── Extract damage.add() slots ──
    damage_adds = re.findall(
        r'this\.damage\.add\(new DamageInfo\(this,\s*(?:this\.)?(\w+)(?:\s*,\s*DamageInfo\.DamageType\.\w+)?\)',
        content)
    
    # Resolve slot values
    slots_base = []
    for expr in damage_adds:
        try:
            slots_base.append(int(expr))
        except ValueError:
            if expr in base_vars:
                slots_base.append(base_vars[expr])
            elif expr in all_consts:
                slots_base.append(all_consts[expr])
            else:
                slots_base.append(f"?{expr}")
    
    # ── Extract pre-battle powers ──
    pre_battle = []
    prebattle_match = re.search(
        r'usePreBattleAction\(\)\s*\{((?:[^{}]|\{[^{}]*\})*)\}', content)
    if prebattle_match:
        pb = prebattle_match.group(1)
        for pm in re.finditer(
            r'ApplyPowerAction\([^,]*,\s*[^,]*,\s*new (\w+Power)\((?:this|[^,)]+),?\s*(\d+)?\)',
            pb):
            power_name = pm.group(1).replace("Power", "")
            amt = int(pm.group(2)) if pm.group(2) else None
            entry = {"type": power_name}
            if amt is not None:
                entry["amount"] = amt
            pre_battle.append(entry)
    
    # ── Extract takeTurn move-to-slot mapping ──
    # Parse: case N: { ... damage.get(M) ... }
    move_slots = {}  # byte_id -> [slot_indices used]
    move_effects = {}  # byte_id -> [effects]
    move_blocks = {}  # byte_id -> block amount
    move_hits = {}  # byte_id -> hit count
    
    take_turn = re.search(
        r'takeTurn\(\)\s*\{((?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*)\}', content)
    if take_turn:
        tt = take_turn.group(1)
        # Split by case blocks
        cases = re.finditer(r'case\s+(\d+)\s*:\s*\{((?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*)\}', tt)
        for cm in cases:
            byte_id = int(cm.group(1))
            case_body = cm.group(2)
            
            # Damage slots used
            slot_refs = re.findall(r'damage\.get\(\(?(\d+)\)?\)', case_body)
            if slot_refs:
                move_slots[byte_id] = [int(s) for s in slot_refs]
            
            # Hit count (for loops)
            hit_match = re.search(r'for\s*\(\s*int\s+\w+\s*=\s*0;\s*\w+\s*<\s*(\d+)', case_body)
            if hit_match:
                move_hits[byte_id] = int(hit_match.group(1))
            
            # Block
            block_match = re.findall(
                r'GainBlockAction\([^,]*,\s*[^,]*,\s*(?:this\.)?(\w+|\d+)\)', case_body)
            for bm in block_match:
                try:
                    move_blocks[byte_id] = int(bm)
                except ValueError:
                    if bm in base_vars:
                        move_blocks[byte_id] = base_vars[bm]
                    elif bm in all_consts:
                        move_blocks[byte_id] = all_consts[bm]
            
            # Effects (ApplyPowerAction)
            effects = []
            for em in re.finditer(
                r'ApplyPowerAction\(\s*(AbstractDungeon\.player|this)[^,]*,\s*[^,]*,\s*new (\w+Power)\([^,]*,?\s*(\d+)?',
                case_body):
                target = "player" if "player" in em.group(1) else "self"
                power = em.group(2).replace("Power", "")
                amt = int(em.group(3)) if em.group(3) else None
                e = {"type": power, "target": target}
                if amt:
                    e["amount"] = amt
                effects.append(e)
            if effects:
                move_effects[byte_id] = effects
    
    # ── Extract move byte ID → name mapping from getMove() ──
    byte_names = {}  # byte_id -> move_name
    get_move = re.search(
        r'getMove\(\s*int\s+\w+\s*\)\s*\{((?:[^{}]|\{(?:[^{}]|\{(?:[^{}]|\{[^{}]*\})*\})*\})*)\}',
        content)
    if get_move:
        gm = get_move.group(1)
        # Pattern: setMove(MOVES[n], (byte)N, Intent.XXX, damage, hits, isMulti)
        for sm in re.finditer(
            r'setMove\(\s*(?:MOVES\[(\d+)\]|"([^"]*)")\s*,\s*\(byte\)\s*(\d+)',
            gm):
            name = sm.group(2) if sm.group(2) else f"MOVES[{sm.group(1)}]"
            byte_id = int(sm.group(3))
            byte_names[byte_id] = name
        
        # Also: setMove((byte)N, Intent.XXX) without name
        for sm in re.finditer(
            r'setMove\(\s*\(byte\)\s*(\d+)\s*,\s*AbstractMonster\.Intent\.(\w+)',
            gm):
            byte_id = int(sm.group(1))
            intent = sm.group(2)
            if byte_id not in byte_names:
                byte_names[byte_id] = f"_byte{byte_id}_{intent}"
    
    # Also get byte ID constant names
    byte_const_names = {}
    for m in re.finditer(r'private\s+static\s+final\s+byte\s+(\w+)\s*=\s*(\d+)', content):
        byte_const_names[int(m.group(2))] = m.group(1)
    
    # ── Merge into final move table ──
    moves = {}
    all_byte_ids = set(list(move_slots.keys()) + list(byte_names.keys()) + list(byte_const_names.keys()))
    
    for bid in sorted(all_byte_ids):
        # Determine move name
        name = byte_names.get(bid)
        if not name or name.startswith("_byte"):
            const_name = byte_const_names.get(bid, f"move_{bid}")
            # Try to make it readable
            name = const_name.replace("_", " ").title()
        
        # Skip names that are just MOVES[n] references — use the constant name instead
        if name.startswith("MOVES["):
            const_name = byte_const_names.get(bid, f"move_{bid}")
            name = const_name.replace("_", " ").title()
        
        move = {}
        
        # Damage
        if bid in move_slots:
            slot_idx = move_slots[bid][0]  # primary damage slot
            if slot_idx < len(slots_base):
                move["damage"] = slots_base[slot_idx]
        
        # Hits
        if bid in move_hits:
            move["hits"] = move_hits[bid]
        elif bid in move_slots and len(move_slots[bid]) > 1:
            # Multiple damage.get() calls = multi-hit
            if all(s == move_slots[bid][0] for s in move_slots[bid]):
                move["hits"] = len(move_slots[bid])
        
        # Block
        if bid in move_blocks:
            move["block"] = move_blocks[bid]
        
        # Effects
        if bid in move_effects:
            move["effects"] = move_effects[bid]
        
        if move:  # only add moves with actual data
            moves[name] = move
    
    # ── Build ascension scaling ──
    asc_scaling = {}
    for lv in sorted(asc_vars.keys()):
        tier = {}
        tier_moves = {}
        for var_name, var_val in asc_vars[lv].items():
            # Try to map this var to a damage slot
            for i, expr in enumerate(damage_adds):
                if expr == var_name:
                    # Find which move uses slot i
                    for bid, slot_list in move_slots.items():
                        if i in slot_list:
                            mname = byte_names.get(bid, byte_const_names.get(bid, f"move_{bid}"))
                            if mname.startswith("MOVES[") or mname.startswith("_byte"):
                                mname = byte_const_names.get(bid, f"move_{bid}").replace("_", " ").title()
                            if mname not in tier_moves:
                                tier_moves[mname] = {}
                            tier_moves[mname]["damage"] = var_val
            # Also store raw var changes for non-damage vars
            var_lower = var_name.lower()
            if "block" in var_lower:
                tier[f"{var_name}"] = var_val
            elif "str" in var_lower or "amt" in var_lower:
                tier[f"{var_name}"] = var_val
        
        if tier_moves:
            tier["moves"] = tier_moves
        if tier:
            asc_scaling[f"asc{lv}"] = tier
    
    result = {
        "name": display_name,
        "java_id": java_id,
        "type": mtype,
        "act": act,
        "hp": hp,
    }
    
    if moves:
        result["moves"] = moves
    if asc_scaling:
        result["asc_scaling"] = asc_scaling
    if pre_battle:
        result["pre_battle"] = pre_battle
    
    # Add raw data for verification
    result["_raw"] = {
        "damage_slots": slots_base,
        "byte_names": {str(k): v for k, v in byte_names.items()},
        "byte_consts": {str(k): v for k, v in byte_const_names.items()},
        "all_consts": all_consts,
    }
    
    return rust_id, result


# ── Process all Java files ──
monsters = {}
for root, dirs, files in os.walk(JAVA_DIR):
    for fname in sorted(files):
        if not fname.endswith(".java") or fname.startswith("Abstract"):
            continue
        fpath = os.path.join(root, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        
        id_match = re.search(r'public static final String ID = "([^"]+)"', content)
        if not id_match:
            continue
        java_id = id_match.group(1)
        
        result = parse_full(java_id, content)
        if result:
            rust_id, data = result
            monsters[rust_id] = data

# Add JawWorm_Hard
if "JawWorm" in monsters:
    jw = monsters["JawWorm"]
    monsters["JawWorm_Hard"] = {
        "name": "Jaw Worm",
        "java_id": "JawWorm",
        "type": "normal",
        "act": 3,
        "hp": dict(jw["hp"]),
        "moves": dict(jw.get("moves", {})),
        "_note": "Same class as JawWorm with hardMode=true. firstMove=false so no guaranteed Chomp turn 1.",
    }

# Sort by act then name
sorted_monsters = dict(sorted(monsters.items(), key=lambda kv: (kv[1]["act"], kv[0])))

with open("data/monsters_verified.json", "w", encoding="utf-8") as f:
    json.dump(sorted_monsters, f, indent=2, ensure_ascii=False)

# ── Summary ──
print(f"Total: {len(sorted_monsters)} monsters")
print(f"With moves: {sum(1 for m in sorted_monsters.values() if m.get('moves'))}")
print(f"With asc_scaling: {sum(1 for m in sorted_monsters.values() if m.get('asc_scaling'))}")
print(f"With pre_battle: {sum(1 for m in sorted_monsters.values() if m.get('pre_battle'))}")

# Check for monsters with empty moves
no_moves = [k for k, v in sorted_monsters.items() if not v.get("moves")]
if no_moves:
    print(f"\nNo moves extracted: {no_moves}")

# Show a sample
for mid in ["JawWorm", "GremlinNob", "Champ"]:
    if mid in sorted_monsters:
        m = sorted_monsters[mid]
        print(f"\n=== {mid} ===")
        print(f"  HP: {m['hp']}")
        if m.get("moves"):
            for mn, md in m["moves"].items():
                print(f"  {mn}: {md}")
        if m.get("asc_scaling"):
            print(f"  Asc: {m['asc_scaling']}")
