#!/usr/bin/env python3
"""Check relics that need logic patching."""
import json

with open('data/relics.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

needs_review = [r for r in data if r.get('manual_review_needed')]
print(f"Total needing review: {len(needs_review)}\n")

for r in needs_review:
    desc = r.get('description', '(no desc)')
    if desc:
        desc = desc[:100]
    print(f"{r['id']}: {desc}")
