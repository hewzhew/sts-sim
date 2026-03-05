"""Clean up duplicate card definitions in red/common.json.

For duplicate IDs, keeps the LAST occurrence (which is the corrected version).
Also validates that the kept version has correct target_type.
"""
import json

path = 'data/cards/red/common.json'
with open(path, 'r', encoding='utf-8') as f:
    cards = json.load(f)

print(f"Before: {len(cards)} entries")

# Find duplicates
from collections import Counter
id_counts = Counter(c['id'] for c in cards)
dupes = {k: v for k, v in id_counts.items() if v > 1}
print(f"Duplicates: {dupes}")

# Keep last occurrence of each ID (corrected version)
seen = {}
for i, c in enumerate(cards):
    seen[c['id']] = i  # last index wins

# Build cleaned list preserving order of first occurrence
first_seen_order = {}
for i, c in enumerate(cards):
    if c['id'] not in first_seen_order:
        first_seen_order[c['id']] = i

# Sort by first occurrence position, but use the last-seen card data
result = []
for card_id in sorted(first_seen_order, key=first_seen_order.get):
    result.append(cards[seen[card_id]])

print(f"After: {len(result)} entries")
for c in result:
    cmds = [cmd['type'] for cmd in c['logic']['commands']]
    print(f"  {c['id']:30s} target={c['logic']['target_type']:15s} cmds={cmds}")

with open(path, 'w', encoding='utf-8') as f:
    json.dump(result, f, indent=2, ensure_ascii=False)
    f.write('\n')

print(f"\nWritten {len(result)} cards to {path}")
