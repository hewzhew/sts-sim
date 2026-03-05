"""List all cards that have conditions, showing card ID, type, and conditions."""
import json, os

for color in ['red', 'colorless']:
    d = f'data/cards/{color}'
    for f in sorted(os.listdir(d)):
        if not f.endswith('.json'):
            continue
        for card in json.load(open(f'{d}/{f}', encoding='utf-8')):
            conds = card.get('logic', {}).get('conditions', [])
            if conds:
                cid = card['id']
                ctype = card['type']
                cmds = [c['type'] for c in card.get('logic', {}).get('commands', [])]
                print(f"{cid:25s} {ctype:8s} cmds={cmds}")
                for c in conds:
                    print(f"  condition: {c}")
                print()
