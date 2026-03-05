#!/usr/bin/env python3
"""Verify patched relics."""
import json

with open('data/relics_patched.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

total = len(data)
with_hooks = sum(1 for r in data if r.get("logic", {}).get("hooks"))
empty_hooks = sum(1 for r in data if not r.get("logic", {}).get("hooks"))
needs_review = sum(1 for r in data if r.get("manual_review_needed"))

print(f"Total relics: {total}")
print(f"With hooks: {with_hooks}")
print(f"Empty hooks (flavor only): {empty_hooks}")
print(f"Needs review: {needs_review}")

# Sample a few relics
print("\n--- Sample Relics ---")
samples = ["Akabeko", "PenNib", "DeadBranch", "IceCream", "Necronomicon"]
for sample_id in samples:
    relic = next((r for r in data if r["id"] == sample_id), None)
    if relic:
        print(f"\n{relic['id']}:")
        print(json.dumps(relic["logic"], indent=2))
