"""
Cross-reference: which individual monsters appear in encounters.rs spawn_encounter()?
Compare with our JSON roster to find missing or spurious monsters.
"""
import re

# Extract all monster names from encounters.rs spawn_encounter patterns
with open("src/monsters/encounters.rs", "r", encoding="utf-8") as f:
    content = f.read()

# Find all string literals in MonsterSpawn::new("...") 
spawn_names = set()
for m in re.finditer(r'MonsterSpawn::(?:new|with_hp)\("([^"]+)"', content):
    spawn_names.add(m.group(1))

# Find hardcoded_ai.rs monster names
with open("src/monsters/hardcoded_ai.rs", "r", encoding="utf-8") as f:
    ai_content = f.read()

# Find names in match patterns: "Monster Name" | "AltName" =>
ai_names = set()
for m in re.finditer(r'"([^"]+?)"\s*(?:\|[^=]*)?=>\s*(?:Some\(self\.|{)', ai_content):
    name = m.group(1)
    if len(name) > 2 and name[0].isupper():
        ai_names.add(name)

# Compare with our verified JSON
import json
with open("data/monsters_verified.json", "r", encoding="utf-8") as f:
    data = json.load(f)
json_displaynames = {m["display_name"] for m in data["monsters"]}
json_ids = {m["id"] for m in data["monsters"]}

# Report
lines = []
lines.append("=== Monster Roster Verification ===\n")

lines.append(f"Spawn names in encounters.rs: {len(spawn_names)}")
for n in sorted(spawn_names):
    in_json = "✅" if n in json_displaynames else "❌ MISSING"
    lines.append(f"  {n:25s} {in_json}")

lines.append(f"\nJSON monsters NOT in encounters.rs spawns ({len(json_displaynames - spawn_names)}):")
for n in sorted(json_displaynames - spawn_names):
    # Check if it could be a minion or special
    lines.append(f"  {n}")

lines.append(f"\nHardcoded AI names not in JSON ({len(ai_names - json_displaynames - json_ids)}):")
for n in sorted(ai_names - json_displaynames - json_ids):
    lines.append(f"  {n}")

with open("tests/roster_check.txt", "w", encoding="utf-8") as f:
    f.write("\n".join(lines))

print(f"Spawn names: {len(spawn_names)}, JSON: {len(json_displaynames)}")
print(f"Missing from JSON: {spawn_names - json_displaynames}")
print(f"See tests/roster_check.txt")
