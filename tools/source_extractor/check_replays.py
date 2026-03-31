import json, os

replay_dir = r"d:\rust\sts_simulator\tools\replays"
for fname in sorted(os.listdir(replay_dir)):
    if not fname.endswith(".jsonl"):
        continue
    if not fname.startswith("2026-03-2"):
        continue
    path = os.path.join(replay_dir, fname)
    with open(path, encoding="utf-8") as f:
        lines = f.readlines()
    
    plays = 0
    potions = 0
    turns = 0
    combats = 0
    potion_cmds = []
    
    for line in lines:
        try:
            obj = json.loads(line)
        except:
            continue
        t = obj.get("type", "")
        if t == "play":
            plays += 1
        elif t == "potion":
            potions += 1
            cmd = obj.get("command", "?")
            potion_cmds.append(cmd)
        elif t == "end_turn":
            turns += 1
        elif t == "combat_start":
            combats += 1
    
    print(f"{fname}")
    print(f"  {len(lines)} lines, {combats} combats, {plays} plays, {turns} turns, {potions} potions")
    if potion_cmds:
        print(f"  Potion commands: {potion_cmds}")
    print()
