#!/usr/bin/env python3
"""Analyze trigger distribution in patched relics."""
import json
from collections import Counter

with open('data/relics_patched.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

triggers = Counter()
commands = Counter()

for relic in data:
    hooks = relic.get("logic", {}).get("hooks", [])
    for hook in hooks:
        trigger = hook.get("trigger", "Unknown")
        triggers[trigger] += 1
        
        for cmd in hook.get("commands", []):
            cmd_type = cmd.get("type", "Unknown")
            commands[cmd_type] += 1
        
        # Also count effect types for passive relics
        effect = hook.get("effect")
        if effect:
            if isinstance(effect, dict):
                commands[f"Effect:{effect.get('type', 'Unknown')}"] += 1
            elif isinstance(effect, list):
                for e in effect:
                    commands[f"Effect:{e.get('type', 'Unknown')}"] += 1

print("=== Trigger Distribution ===")
for trigger, count in triggers.most_common():
    print(f"  {trigger}: {count}")

print("\n=== Command Distribution (Top 20) ===")
for cmd, count in commands.most_common(20):
    print(f"  {cmd}: {count}")
