"""Extract Flight transitions from Byrd combats in real_game_floor51.jsonl."""
import json

with open(r"c:\Dev\rust\sts_sim\tests\fixtures\real_game_floor51.jsonl", 'r', encoding='utf-8') as f:
    lines = [json.loads(line) for line in f if line.strip()]

print(f"Total entries: {len(lines)}")

# Find all entries with Flight powers
for i, entry in enumerate(lines):
    gs = entry.get("game_state", {})
    cs = gs.get("combat_state")
    if not cs:
        continue
    
    monsters = cs.get("monsters", [])
    has_flight = False
    for m in monsters:
        for p in m.get("powers", []):
            if p.get("id") == "Flight":
                has_flight = True
                break
    
    if not has_flight:
        continue
    
    # Print this state's Flight info
    cmds = entry.get("available_commands", [])
    cmd_str = ""
    if isinstance(cmds, list) and cmds:
        cmd_str = str(cmds[:2])
    elif isinstance(cmds, dict):
        cmd_str = str(list(cmds.keys())[:3])
    
    hand = cs.get("hand", [])
    hand_names = [c.get("name", "?") for c in hand[:6]]
    
    print(f"\n--- Entry {i} (turn={cs.get('turn')}) ---")
    print(f"  Hand: {hand_names}")
    for j, m in enumerate(monsters):
        flight_powers = [p for p in m.get("powers", []) if p.get("id") == "Flight"]
        if flight_powers:
            fp = flight_powers[0]
            print(f"  enemy[{j}] {m['name']} hp={m['current_hp']}: Flight amount={fp['amount']}, misc={fp.get('misc', '?')}")
        elif "Byrd" in m.get("name", ""):
            print(f"  enemy[{j}] {m['name']} hp={m['current_hp']}: NO Flight (gone={m.get('is_gone')})")
