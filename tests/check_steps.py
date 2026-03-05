import json

lines = open('tests/fixtures/real_game_floor51.jsonl').readlines()

out = []
for l in lines:
    e = json.loads(l)
    if 7 <= e['step'] <= 16:
        gs = e.get('state', {}).get('game_state', {})
        cs = gs.get('combat_state')
        if cs:
            p = cs.get('player', {})
            hand_ids = [c['id'] for c in cs.get('hand', [])]
            m0 = cs.get('monsters', [{}])[0]
            out.append(f"Step {e['step']:2d} | cmd='{e['command']}' | E={p.get('energy','?')} B={p.get('block',0)} | hand({len(hand_ids)})={hand_ids} | enemy_hp={m0.get('current_hp','?')}")
        else:
            out.append(f"Step {e['step']:2d} | cmd='{e['command']}' | (no combat)")

with open('tests/step_analysis.txt', 'w') as f:
    f.write('\n'.join(out))

print("Done! Check tests/step_analysis.txt")
