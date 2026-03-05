import json

with open("tests/java_extraction.json", "r", encoding="utf-8") as f:
    data = json.load(f)

missing_hp = [d for d in data if not d["hp"]]
have_hp = [d for d in data if d["hp"]]

out = open("tests/missing_hp.txt", "w", encoding="utf-8")
out.write(f"Monsters with missing HP ({len(missing_hp)}/{len(data)}):\n")
for m in missing_hp:
    out.write(f"  {m['id']:20s}  java={m['java_id']:20s}  src={m['_source']}\n")

out.write(f"\nMonsters with HP ({len(have_hp)}):\n")
for m in have_hp:
    out.write(f"  {m['id']:20s}  HP={json.dumps(m['hp'])}\n")

out.close()

# Now generate the clean JSON
monsters_out = []
for m in data:
    entry = {
        "id": m["id"],
        "java_id": m["java_id"],
        "display_name": m["display_name"],
        "act": m["act"],
        "hp": m["hp"] if m["hp"] else {"base": [0, 0], "_TODO": "extract manually"},
    }
    monsters_out.append(entry)

with open("data/monsters_verified.json", "w", encoding="utf-8") as f:
    json.dump({"monsters": monsters_out}, f, indent=2, ensure_ascii=False)

print(f"Missing HP: {len(missing_hp)}/{len(data)}")
print(f"Output: data/monsters_verified.json ({len(monsters_out)} monsters)")
print(f"See tests/missing_hp.txt for details")
