import json

with open("data/monsters_with_behavior.json", "r", encoding="utf-8") as f:
    data = json.load(f)

monsters = data["monsters"]
total_moves = 0
all_top_keys = set()
all_move_keys = set()
all_effect_keys = set()
all_effect_types = set()
all_logic_types = set()

for mon in monsters:
    all_top_keys.update(mon.keys())
    for m in mon.get("moves", []):
        total_moves += 1
        all_move_keys.update(m.keys())
        for e in m.get("effects", []):
            all_effect_keys.update(e.keys())
            t = e.get("type", e.get("effect_type", ""))
            if t:
                all_effect_types.add(t)
    bm = mon.get("behavior_model", {})
    if bm and isinstance(bm, dict):
        lt = bm.get("logic_type", "")
        if lt:
            all_logic_types.add(lt)

out = open("tests/schema_analysis.txt", "w", encoding="utf-8")
out.write(f"Total monsters: {len(monsters)}, Total moves: {total_moves}\n\n")

out.write("=== Monster top-level keys ===\n")
for k in sorted(all_top_keys):
    out.write(f"  {k}\n")

out.write("\n=== Move keys ===\n")
for k in sorted(all_move_keys):
    out.write(f"  {k}\n")

out.write("\n=== Effect keys ===\n")
for k in sorted(all_effect_keys):
    out.write(f"  {k}\n")

out.write("\n=== Effect types used ===\n")
for k in sorted(all_effect_types):
    out.write(f"  {k}\n")

out.write("\n=== Behavior logic types ===\n")
for k in sorted(all_logic_types):
    out.write(f"  {k}\n")

# Move field coverage
out.write(f"\n=== Move field coverage (across {total_moves} moves) ===\n")
fc = {}
for mon in monsters:
    for m in mon.get("moves", []):
        for k in m:
            fc[k] = fc.get(k, 0) + 1
for k, v in sorted(fc.items(), key=lambda x: -x[1]):
    out.write(f"  {k}: {v}/{total_moves}\n")

# Non-standard effect types
std = {"Buff", "Debuff", "Block", "AddCard", "Summon", "Split", "ApplyPower", "Damage"}
nonstd = {}
for mon in monsters:
    for m in mon.get("moves", []):
        for e in m.get("effects", []):
            t = e.get("type", "")
            if t and t not in std:
                nonstd[t] = nonstd.get(t, 0) + 1

out.write("\n=== Non-standard effect types ===\n")
for k, v in sorted(nonstd.items(), key=lambda x: -x[1]):
    out.write(f"  {k}: {v}\n")

# Block location analysis
top_block = 0
eff_block = 0
both_block = 0
for mon in monsters:
    for m in mon.get("moves", []):
        has_top = m.get("block") and m["block"] > 0
        has_eff = any(e.get("type") == "Block" for e in m.get("effects", []))
        if has_top:
            top_block += 1
        if has_eff:
            eff_block += 1
        if has_top and has_eff:
            both_block += 1

out.write(f"\n=== Block location ===\n")
out.write(f"  Top-level 'block' field: {top_block}\n")
out.write(f"  In effects[]: {eff_block}\n")
out.write(f"  Both: {both_block}\n")

# Damage location
damage_top = 0
for mon in monsters:
    for m in mon.get("moves", []):
        if m.get("damage") and m["damage"] > 0:
            damage_top += 1
out.write(f"\n=== Damage info ===\n")
out.write(f"  Moves with damage > 0: {damage_top}\n")
out.write(f"  Moves with damage == 0: {total_moves - damage_top}\n")

# Ascension scaling analysis
has_scaling = 0
scaling_fields = set()
for mon in monsters:
    for m in mon.get("moves", []):
        sc = m.get("ascension_scaling", [])
        if sc:
            has_scaling += 1
            for s in sc:
                scaling_fields.update(s.keys())

out.write(f"\n=== Ascension scaling ===\n")
out.write(f"  Moves with scaling: {has_scaling}/{total_moves}\n")
out.write(f"  Scaling fields: {sorted(scaling_fields)}\n")

# Check intent field
intent_values = set()
for mon in monsters:
    for m in mon.get("moves", []):
        intent_values.add(m.get("intent", ""))

out.write(f"\n=== Intent values ===\n")
for v in sorted(intent_values):
    out.write(f"  {v}\n")

# How Rust loads this
out.write("\n=== Rust Deserialization Notes ===\n")
out.write("  Need to check: MoveEffect struct in enemy.rs\n")
out.write("  Need to check: MonsterDefinition struct\n")
out.write("  Need to check: loader.rs\n")

out.close()
print("Done. See tests/schema_analysis.txt")
