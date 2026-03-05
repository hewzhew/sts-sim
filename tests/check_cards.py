import json
cards = json.load(open('data/cards.json'))
for c in cards:
    if c['id'] in ['Demon_Form', 'Feel_No_Pain', 'Shockwave']:
        print(f"\n=== {c['id']} ===")
        logic = c.get('logic', {})
        print(f"  target_type: {logic.get('target_type')}")
        print(f"  commands: {json.dumps(logic.get('commands', []))}")
