"""Check for no-op ascension overrides and other issues."""
import json

with open("data/monsters_verified.json", "r", encoding="utf-8") as f:
    data = json.load(f)

print("=== 1. Checking for no-op ascension move overrides ===")
for k, v in data.items():
    if k.startswith("_"):
        continue
    base_moves = v.get("moves", {})
    for ak, av in v.get("ascension", {}).items():
        for mk, mv in av.get("moves", {}).items():
            if mk in base_moves:
                # Check if the override values match base  
                for field, val in mv.items():
                    if field in base_moves[mk] and base_moves[mk][field] == val:
                        print(f"  NO-OP: {k}.asc{ak}.m{mk}.{field} = {val} (same as base)")

print("\n=== 2. Monsters with NO ascensionLevel checks in Java ===")
# Dagger has no asc checks. Let me check what we put:
for k, v in data.items():
    if k.startswith("_"):
        continue
    asc = v.get("ascension", {})
    if asc:
        for ak, av in asc.items():
            all_noop = True
            for mk, mv in av.get("moves", {}).items():
                if mk in v.get("moves", {}):
                    for field, val in mv.items():
                        if field not in v["moves"].get(mk, {}) or v["moves"][mk][field] != val:
                            all_noop = False
                else:
                    all_noop = False
            if "hp" not in av and "pre_battle" not in av and "end_turn_effects" not in av:
                if all_noop and av.get("moves"):
                    print(f"  SUSPICIOUS: {k}.asc{ak} - overrides match base values exactly")
            elif not av.get("moves") and "hp" not in av and "pre_battle" not in av:
                print(f"  EMPTY: {k}.asc{ak} - no meaningful overrides")

print("\n=== 3. Monsters with 'hits' field that might be dynamic ===")
for k, v in data.items():
    if k.startswith("_"):
        continue
    for mk, mv in v.get("moves", {}).items():
        if "hits" in mv:
            print(f"  {k}.m{mk} ({mv.get('name','?')}): hits={mv['hits']}")

print("\n=== 4. Monsters with fixed HP (min==max) ===")
for k, v in data.items():
    if k.startswith("_"):
        continue
    hp = v.get("hp", {})
    if hp.get("min") == hp.get("max"):
        print(f"  {k}: HP={hp['min']}")
