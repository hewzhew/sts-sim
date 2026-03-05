"""Check which Ironclad cards have tests vs total in JSON."""
import json, re, os

# Load all red cards
red_dir = "data/cards/red"
all_cards = {}
for f in sorted(os.listdir(red_dir)):
    if f.endswith(".json"):
        rarity = f.replace(".json","")
        cards = json.load(open(os.path.join(red_dir, f), encoding="utf-8"))
        for c in cards:
            all_cards[c["id"]] = rarity

# Parse test function names
with open("src/card_tests/ironclad.rs", encoding="utf-8") as f:
    content = f.read()

tested = set()
for m in re.finditer(r'play_card_by_id\(&mut state, "(\w+)"', content):
    tested.add(m.group(1))

print(f"Red cards in JSON: {len(all_cards)}")
print(f"Cards with tests:  {len(tested)}")

missing = sorted(set(all_cards.keys()) - tested)
extra = sorted(tested - set(all_cards.keys()))

if missing:
    print(f"\nMISSING ({len(missing)}):")
    for m in missing:
        print(f"  {m} ({all_cards[m]})")
else:
    print("\nAll red cards have tests!")

if extra:
    print(f"\nExtra (not in red): {extra}")
