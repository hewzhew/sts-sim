"""Verify v5 schema structural rules for ALL entries."""
import json

with open("data/monsters_verified.json", "r", encoding="utf-8") as f:
    data = json.load(f)

checks = []
def ok(monster, field, expected, actual):
    status = "OK" if expected == actual else "MISMATCH"
    checks.append((status, f"{monster}.{field}", expected, actual))

monsters = {k: v for k, v in data.items() if not k.startswith("_")}

for mid, m in monsters.items():
    # R9: Must have id field matching key
    ok(mid, "has_id", True, "id" in m and m["id"] == mid)
    
    # Must have name, java_id, type, act
    ok(mid, "has_name", True, "name" in m)
    ok(mid, "has_java_id", True, "java_id" in m)
    ok(mid, "has_type", True, m.get("type") in ("normal", "elite", "boss", "minion"))
    ok(mid, "has_act", True, "act" in m)
    
    # R3: HP must be {min, max}
    hp = m["hp"]
    ok(mid, "hp_format", True, isinstance(hp, dict) and "min" in hp and "max" in hp)
    
    # No bare "notes" (must be _notes)
    ok(mid, "no_bare_notes", True, "notes" not in m)
    
    # Move checks
    for mk, mv in m.get("moves", {}).items():
        ok(mid, f"m{mk}_is_digit", True, mk.isdigit())
        ok(mid, f"m{mk}_has_name", True, "name" in mv)
        ok(mid, f"m{mk}_no_notes", True, "notes" not in mv)
        
        if "damage" in mv:
            d = mv["damage"]
            ok(mid, f"m{mk}_dmg_type", True,
               isinstance(d, int) or (isinstance(d, dict) and "min" in d))
        
        for i, e in enumerate(mv.get("effects", [])):
            ok(mid, f"m{mk}_e{i}_id", True, "id" in e and "power" not in e)
        
        for i, c in enumerate(mv.get("cards", [])):
            ok(mid, f"m{mk}_c{i}_id", True, "id" in c)
            ok(mid, f"m{mk}_c{i}_amt", True, "amount" in c and "count" not in c)
            ok(mid, f"m{mk}_c{i}_dest", True, "destination" in c and "dest" not in c)
    
    # Pre-battle checks
    for i, pb in enumerate(m.get("pre_battle", [])):
        ok(mid, f"pb{i}_id", True, "id" in pb)
        if "amount" in pb:
            a = pb["amount"]
            ok(mid, f"pb{i}_amt_type", True,
               isinstance(a, int) or (isinstance(a, dict) and "min" in a))
    
    # Ascension checks
    for ak, av in m.get("ascension", {}).items():
        ok(mid, f"asc{ak}_digit", True, ak.isdigit())
        if "hp" in av:
            ok(mid, f"asc{ak}_hp_fmt", True,
               isinstance(av["hp"], dict) and "min" in av["hp"])
        for mk, mv in av.get("moves", {}).items():
            ok(mid, f"asc{ak}_m{mk}_no_notes", True, "notes" not in mv)
            for i, c in enumerate(mv.get("cards", [])):
                ok(mid, f"asc{ak}_m{mk}_c{i}_amt", True, "amount" in c and "count" not in c)
                ok(mid, f"asc{ak}_m{mk}_c{i}_dest", True, "destination" in c and "dest" not in c)

passed = sum(1 for s, *_ in checks if s == "OK")
total = len(checks)
print(f"=== v5 Verification: {passed}/{total} ({len(monsters)} monsters) ===")
for s, f, e, a in checks:
    if s != "OK":
        print(f"  MISMATCH {f}: expected={e}, got={a}")
if passed == total:
    print("ALL CHECKS PASSED!")
