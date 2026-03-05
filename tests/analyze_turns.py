"""Analyze JSONL turn boundaries to understand the full turn cycle."""
import json

with open(r"c:\Dev\rust\sts_sim\tests\fixtures\real_game_floor51.jsonl", 'r', encoding='utf-8') as f:
    lines = [json.loads(line) for line in f if line.strip()]

# Show the first combat's complete turn cycle 
# Focus on the Byrd combat where we had debuff decay issues
# Steps around 335-350

print("=== BYRD COMBAT TURN BOUNDARY ===")
for e in lines:
    step = e.get('step', -1)
    if 335 <= step <= 350:
        cmd = e.get('command', '')
        cs = e.get('state', {}).get('game_state', {}).get('combat_state')
        screen = e.get('state', {}).get('game_state', {}).get('screen_type', '?')
        
        if cs:
            player = cs.get('player', {})
            turn = cs.get('turn', '?')
            monsters_summary = []
            for j, m in enumerate(cs.get('monsters', [])):
                if not m.get('is_gone'):
                    powers = {p['id']: p['amount'] for p in m.get('powers', [])}
                    intent = m.get('intent', '?')
                    monsters_summary.append(f"e[{j}]({m['name']}) hp={m['current_hp']} blk={m.get('block',0)} pwr={powers} int={intent}")
            
            print(f"\nStep {step}: cmd='{cmd}' turn={turn} screen={screen}")
            print(f"  Player: hp={player.get('current_hp')} blk={player.get('block')} energy={player.get('energy')}")
            pp = {p['id']: p['amount'] for p in player.get('powers', [])}
            if pp: print(f"  Player powers: {pp}")
            for m in monsters_summary:
                print(f"  {m}")
        else:
            print(f"\nStep {step}: cmd='{cmd}' [NO COMBAT] screen={screen}")

# Also show the Slaver combat for another example
print("\n\n=== SLAVER COMBAT TURN BOUNDARY ===")
for e in lines:
    step = e.get('step', -1)
    if 514 <= step <= 530:
        cmd = e.get('command', '')
        cs = e.get('state', {}).get('game_state', {}).get('combat_state')
        screen = e.get('state', {}).get('game_state', {}).get('screen_type', '?')
        
        if cs:
            player = cs.get('player', {})
            turn = cs.get('turn', '?')
            monsters_summary = []
            for j, m in enumerate(cs.get('monsters', [])):
                powers = {p['id']: p['amount'] for p in m.get('powers', [])}
                gone = " GONE" if m.get('is_gone') else ""
                monsters_summary.append(f"e[{j}]({m['name']}) hp={m['current_hp']}{gone} pwr={powers}")
            
            print(f"\nStep {step}: cmd='{cmd}' turn={turn}")
            print(f"  Player: hp={player.get('current_hp')} blk={player.get('block')} energy={player.get('energy')}")
            pp = {p['id']: p['amount'] for p in player.get('powers', [])}
            if pp: print(f"  Player powers: {pp}")
            for m in monsters_summary:
                print(f"  {m}")
        else:
            print(f"\nStep {step}: cmd='{cmd}' [NO COMBAT] screen={screen}")
