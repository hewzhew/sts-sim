import json

with open("data/monsters_with_behavior.json", "r", encoding="utf-8") as f:
    data = json.load(f)

targets = [
    "Jaw Worm", "Gremlin Nob", "Hexaghost", "Deca", "Donu",
    "Shield Gremlin", "Shelled Parasite", "Orb Walker", "The Champ",
    "Chosen", "Snake Plant", "Fungi Beast", "Cultist", "Spire Growth"
]

for mon in data["monsters"]:
    if mon["name"] in targets:
        print(f'=== {mon["name"]} ===')
        for m in mon.get("moves", []):
            block = m.get("block", "-")
            effs = m.get("effects", [])
            eff_strs = []
            for e in effs:
                t = e.get("type", e.get("effect_type", "?"))
                name = e.get("effect", e.get("card", ""))
                amt = e.get("amount", "")
                eff_strs.append(f"{t}:{name}={amt}")
            print(f'  {m["name"]:20s} dmg={m.get("damage",0):3d} block={str(block):5s} effects=[{", ".join(eff_strs)}]')
        print()
