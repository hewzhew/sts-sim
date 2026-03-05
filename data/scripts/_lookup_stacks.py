import json, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

d = json.load(open('data/powers_spec.json', 'r', encoding='utf-8'))

# Find power specs for our target cards
# Map card JSON id -> Java power ID
card_to_power = {
    'Biased_Cognition': ['Biased Cognition', 'Bias'],
    'Creative_AI': ['Creative AI'],
    'Hello_World': ['Hello'],
    'Loop': ['Loop'],
    'Machine_Learning': ['Machine Learning'],
    'Noxious_Fumes': ['Noxious Fumes'],
    'Infinite_Blades': ['Infinite Blades'],
    'Tools_of_the_Trade': ['Tools of the Trade'],
    'Wraith_Form': ['Wraith Form v2'],
    'Well-Laid_Plans': ['Retain'],
    'Battle_Hymn': ['Battle Hymn'],
    'Deva_Form': ['Deva Form'],
    'Devotion': ['Devotion'],
    'Fasting': ['Fasting'],
    'Foresight': ['Foresight'],
    'Study': ['Study'],
    'Magnetism': ['Magnetism'],
    'Mayhem': ['Mayhem'],
}

for card_id, power_ids in sorted(card_to_power.items()):
    for p in d:
        if p['id'] in power_ids:
            cards = p.get('card_creates_with', [])
            if cards:
                c = cards[0]
                print(f"{card_id:25s} -> power_id=\"{p['id']}\", card_args=\"{c['constructor_args']}\", base={c.get('base_value')}, upgrade_delta={c.get('upgrade_delta')}")
            else:
                print(f"{card_id:25s} -> power_id=\"{p['id']}\", NO CARD MAPPING")
            break
    else:
        print(f"{card_id:25s} -> NOT FOUND IN SPEC")
