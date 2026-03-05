import json, os, re

with open("data/monsters_with_behavior.json", "r", encoding="utf-8") as f:
    data = json.load(f)

json_monsters = {}
for mon in data["monsters"]:
    moves = {}
    for m in mon.get("moves", []):
        effects = m.get("effects", [])
        moves[m["name"]] = {
            "damage": m.get("damage", 0), "hits": m.get("hits", 1),
            "block": m.get("block"), "effects": effects,
            "intent": m.get("intent", "Unknown"),
        }
    json_monsters[mon["name"]] = moves
    if mon.get("internal_id"):
        json_monsters[mon["internal_id"]] = moves

java_dir = r"C:\Dev\rust\cardcrawl\monsters"
lines = []
checked = 0
issue_count = 0

for root, dirs, files in os.walk(java_dir):
    for fname in sorted(files):
        if not fname.endswith(".java") or fname.startswith("Abstract"):
            continue
        fpath = os.path.join(root, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            content = f.read()
        
        id_match = re.search(r'public static final String ID = "(\w+)"', content)
        if not id_match:
            continue
        monster_id = id_match.group(1)
        
        name_match = re.search(r'getMonsterStrings\("(\w+)"\)', content)
        monster_name = name_match.group(1) if name_match else monster_id

        tt_match = re.search(r'public void takeTurn\(\)', content)
        if not tt_match:
            continue
        
        checked += 1
        tt_section = content[tt_match.start():]
        
        java_actions = set()
        if "GainBlockAction" in tt_section:
            java_actions.add("GainBlock")
        if "StrengthPower" in tt_section:
            java_actions.add("Strength")
        if "AngerPower" in tt_section:
            java_actions.add("Anger")
        if "HexPower" in tt_section:
            java_actions.add("Hex")
        if "VulnerablePower" in tt_section:
            java_actions.add("Vulnerable")
        if "WeakPower" in tt_section:
            java_actions.add("Weak") 
        if "FrailPower" in tt_section:
            java_actions.add("Frail")
        if "RitualPower" in tt_section:
            java_actions.add("Ritual")
        if "MetallicizePower" in tt_section:
            java_actions.add("Metallicize")
        if "ConstrictedPower" in tt_section:
            java_actions.add("Constricted")
        if "MakeTempCardInDiscardAction" in tt_section or "MakeTempCardInDrawPileAction" in tt_section:
            java_actions.add("AddCard")
        if "SpawnMonsterAction" in tt_section:
            java_actions.add("Summon")
        if "EnragePower" in tt_section:
            java_actions.add("Enrage")
        if "PoisonPower" in tt_section:
            java_actions.add("Poison")
        if "PlatedArmorPower" in tt_section:
            java_actions.add("PlatedArmor")
        if "MalleablePower" in tt_section:
            java_actions.add("Malleable")
        if "DamageAction" in tt_section:
            java_actions.add("Damage")
        
        json_moves = json_monsters.get(monster_name, json_monsters.get(monster_id, {}))
        if not json_moves:
            lines.append(f"MISSING: {monster_name} ({monster_id}) - no JSON entry at all")
            issue_count += 1
            continue
        
        all_json_effects = set()
        for mn, md in json_moves.items():
            for eff in md["effects"]:
                etype = eff.get("type", "")
                ename = eff.get("effect", eff.get("card", ""))
                all_json_effects.add(etype)
                all_json_effects.add(ename)
            if md.get("block"):
                all_json_effects.add("GainBlock")
            if md.get("damage") and md["damage"] > 0:
                all_json_effects.add("Damage")
        
        missing = []
        for ja in java_actions:
            found = False
            if ja == "GainBlock" and "GainBlock" in all_json_effects:
                found = True
            elif ja == "GainBlock" and "Block" in all_json_effects:
                found = True
            elif ja == "Damage" and "Damage" in all_json_effects:
                found = True
            elif ja == "Anger" and ("Anger" in all_json_effects or "Angry" in all_json_effects or "Enrage" in all_json_effects):
                found = True
            elif ja == "Enrage" and ("Enrage" in all_json_effects or "Anger" in all_json_effects):
                found = True
            elif ja in all_json_effects:
                found = True
            
            if not found:
                missing.append(ja)
        
        if missing:
            issue_count += 1
            lines.append(f"MISMATCH: {monster_name} ({fname})")
            lines.append(f"  Java has: {sorted(java_actions)}")
            lines.append(f"  JSON has: {sorted(all_json_effects)}")
            lines.append(f"  Missing in JSON: {sorted(missing)}")
            lines.append("")

with open("tests/audit_result.txt", "w", encoding="utf-8") as f:
    f.write(f"Checked {checked} Java monsters against JSON\n")
    f.write(f"Issues found: {issue_count}\n\n")
    for line in lines:
        f.write(line + "\n")

print(f"Done. Checked {checked}, issues: {issue_count}. See tests/audit_result.txt")
