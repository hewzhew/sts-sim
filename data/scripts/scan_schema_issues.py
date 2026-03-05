#!/usr/bin/env python3
"""Scan events JSON for schema inconsistencies."""

import json

def main():
    with open('../events_final_master.json', encoding='utf-8') as f:
        d = json.load(f)

    print('=== CardSelect without action ===')
    for eid, event in d.items():
        for opt in event.get('options', []):
            for cmd in opt.get('commands', []):
                if cmd.get('type') == 'CardSelect' and 'action' not in cmd:
                    print(f"  {eid}: {opt.get('label')} -> {cmd}")

    print('\n=== HasRelic with "id" instead of "relic_id" ===')
    for eid, event in d.items():
        for opt in event.get('options', []):
            for cond in opt.get('conditions', []):
                if cond.get('type') == 'HasRelic' and 'id' in cond:
                    print(f"  {eid}: {opt.get('label')} -> {cond}")
                if cond.get('type') == 'NoRelic' and 'id' in cond:
                    print(f"  {eid}: {opt.get('label')} -> {cond}")

    print('\n=== hp_dynamic in costs ===')
    for eid, event in d.items():
        for opt in event.get('options', []):
            if opt.get('costs', {}).get('hp_dynamic'):
                print(f"  {eid}: {opt.get('label')}")

    print('\n=== SetEventPhase commands ===')
    for eid, event in d.items():
        for opt in event.get('options', []):
            for cmd in opt.get('commands', []):
                if cmd.get('type') == 'SetEventPhase':
                    print(f"  {eid}: {opt.get('label')} -> {cmd}")

    print('\n=== Complex potion/card rewards (object instead of string) ===')
    for eid, event in d.items():
        for opt in event.get('options', []):
            rewards = opt.get('rewards', {})
            if isinstance(rewards.get('potion'), dict):
                print(f"  {eid}: rewards.potion is object -> {rewards.get('potion')}")
            if isinstance(rewards.get('card'), dict):
                print(f"  {eid}: rewards.card is object -> {rewards.get('card')}")

if __name__ == '__main__':
    main()
