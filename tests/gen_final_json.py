"""
Generate the final monsters_verified.json with comprehensive move data.
Includes: HP, damage slots, all constants, base+ascension member vars.
Also adds JawWorm_Hard manually (same Java class, different stats triggered by hardMode).
"""
import json

with open("tests/java_full_extraction.json", "r", encoding="utf-8") as f:
    data = json.load(f)

monsters = []
for m in data:
    # Clean up: remove unresolvable damage values, keep only numbers
    damage_clean = []
    for d in m["damage_slots"]:
        if isinstance(d, int):
            damage_clean.append(d)
        elif isinstance(d, str) and d.startswith("$"):
            damage_clean.append({"var": d[1:]})  # unresolved var
        else:
            try:
                damage_clean.append(int(d))
            except:
                damage_clean.append({"var": str(d)})
    
    # Filter constants to only damage/block/status relevant ones
    relevant_consts = {}
    for k, v in m["constants"].items():
        k_lower = k.lower()
        if any(kw in k_lower for kw in [
            'dmg', 'damage', 'block', 'str', 'amt', 'count', 'heal',
            'shield', 'buff', 'weak', 'vuln', 'frail', 'poison',
            'thorns', 'metallicize', 'forge', 'exec', 'hit',
            'multi', 'bite', 'slam', 'stab', 'sear', 'fire',
        ]):
            relevant_consts[k] = v
    
    # Filter base vars similarly
    relevant_vars = {}
    for k, v in m["base_vars"].items():
        relevant_vars[k] = v
    
    entry = {
        "id": m["id"],
        "java_id": m["java_id"],
        "display_name": m["display_name"],
        "act": m["act"],
        "hp": m["hp"],
        "damage_slots": damage_clean,
    }
    
    # Add constants if any
    if relevant_consts:
        entry["constants"] = relevant_consts
    
    # Add base vars if any
    if relevant_vars:
        entry["base_vars"] = relevant_vars
    
    # Add ascension vars if any
    if m["asc_vars"]:
        entry["asc_vars"] = m["asc_vars"]
    
    monsters.append(entry)

# Add JawWorm_Hard manually
# Same Java class as JawWorm but spawned with hardMode=true
# In Java: hardMode increases base damage by 1 and changes HP range
jaw_worm = next(m for m in monsters if m["id"] == "JawWorm")
jaw_worm_hard = {
    "id": "JawWorm_Hard",
    "java_id": "JawWorm",
    "display_name": "Jaw Worm",
    "act": 3,
    "hp": jaw_worm["hp"].copy(),  # Same HP ranges
    "_note": "Act 3 Jaw Worm Horde variant. Same AI as JawWorm but spawned with hardMode flag.",
}
# Insert after JawWorm
idx = next(i for i, m in enumerate(monsters) if m["id"] == "JawWorm") + 1
monsters.insert(idx, jaw_worm_hard)

# Re-sort by act, then name
monsters.sort(key=lambda m: (m["act"], m["id"]))

output = {"_schema": "monsters_verified v2", "_count": len(monsters), "monsters": monsters}

with open("data/monsters_verified.json", "w", encoding="utf-8") as f:
    json.dump(output, f, indent=2, ensure_ascii=False)

print(f"Generated {len(monsters)} monsters")
print(f"With damage_slots: {sum(1 for m in monsters if m.get('damage_slots'))}")
print(f"With constants: {sum(1 for m in monsters if m.get('constants'))}")
print(f"With asc_vars: {sum(1 for m in monsters if m.get('asc_vars'))}")

# Print a few examples
for m in monsters:
    if m["id"] in ["Cultist", "JawWorm", "JawWorm_Hard", "Champ", "GremlinNob", "CorruptHeart"]:
        print(f"\n=== {m['id']} ({m['display_name']}) ===")
        print(f"  HP: {m['hp']}")
        print(f"  Damage slots: {m.get('damage_slots', [])}")
        if m.get('constants'):
            print(f"  Constants: {m['constants']}")
        if m.get('base_vars'):
            print(f"  Base vars: {m['base_vars']}")
        if m.get('asc_vars'):
            print(f"  Asc vars: {m['asc_vars']}")
