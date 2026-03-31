"""
Convert old CommunicationMod interaction logs to diff_driver JSONL format.

KEY FINDINGS:
1. Old CommunicationMod uses 1-INDEXED card positions (play 4 = hand[3])
2. Response to each command IS the post-action state (no timing shift needed)
3. Target 0 is always present even for non-targeted cards

Usage: python convert_logs.py [input.log] [output.jsonl]
       python convert_logs.py  (batch all d:/rust/runs/*.log)
"""

import json
import sys
import os


def convert_combat_state(cs, gs):
    """Convert old combat_state structure to diff_driver result format."""
    result = {}
    
    p = cs['player']
    result['player'] = {
        'hp': p['current_hp'],
        'max_hp': p['max_hp'],
        'block': p.get('block', 0),
        'energy': p.get('energy', 3),
        'powers': p.get('powers', []),
    }
    
    result['monsters'] = []
    for m in cs.get('monsters', []):
        result['monsters'].append({
            'id': m.get('id', m.get('name', 'Unknown')),
            'name': m.get('name', ''),
            'hp': m['current_hp'],
            'max_hp': m['max_hp'],
            'block': m.get('block', 0),
            'powers': m.get('powers', []),
            'intent': m.get('intent', 'UNKNOWN'),
            'move_id': m.get('move_id', -1),
            'is_gone': m.get('is_gone', False),
            'move_base_damage': m.get('move_base_damage', -1),
            'move_hits': m.get('move_hits', 1),
            'move_adjusted_damage': m.get('move_adjusted_damage', -1),
        })
    
    result['hand'] = []
    for c in cs.get('hand', []):
        result['hand'].append({
            'id': c['id'],
            'cost': c.get('cost', 0),
            'upgrades': c.get('upgrades', 0),
            'uuid': c.get('uuid', ''),
            'name': c.get('name', c['id']),
        })
    result['hand_size'] = len(result['hand'])
    
    draw_pile = cs.get('draw_pile', [])
    result['draw_pile_ids'] = [c['id'] for c in draw_pile]
    result['draw_pile_size'] = len(draw_pile)
    
    discard_pile = cs.get('discard_pile', [])
    result['discard_pile_ids'] = [c['id'] for c in discard_pile]
    result['discard_pile_size'] = len(discard_pile)
    
    exhaust_pile = cs.get('exhaust_pile', [])
    result['exhaust_pile_ids'] = [c['id'] for c in exhaust_pile]
    result['exhaust_pile_size'] = len(exhaust_pile)
    
    result['turn'] = cs.get('turn', 1)
    
    # Pass through RNG state if present (from modified CommunicationMod)
    if 'rng_state' in cs:
        result['rng_state'] = cs['rng_state']
    
    # Pass through per-monster-turn intermediate snapshots (P0 upgrade)
    if 'monster_turn_log' in cs:
        result['monster_turn_log'] = cs['monster_turn_log']
    
    # Potions: exported at gs (game_state) level, not inside combat_state
    # Communication Mod: {id, name, can_use, can_discard, requires_target}
    # PotionSlot entries (empty slots) have id="Potion Slot" — we store None for those.
    potions = []
    for p in gs.get('potions', []):
        pid = p.get('id', '')
        if pid == 'Potion Slot' or not pid:
            potions.append(None)
        else:
            potions.append({'id': pid, 'can_use': p.get('can_use', False),
                          'requires_target': p.get('requires_target', False)})
    if potions:
        result['potions'] = potions
    
    # Relics: track per-combat so diff_driver can initialize them
    relics = []
    for r in gs.get('relics', []):
        relic_entry = {'id': r.get('id', r.get('name', ''))}
        if 'counter' in r:
            relic_entry['counter'] = r['counter']
        relics.append(relic_entry)
    if relics:
        result['relics'] = relics
    
    return result


def parse_pairs(lines):
    """Extract (command, response_json) pairs from log lines."""
    pairs = []
    i = 0
    while i < len(lines):
        line = lines[i].strip()
        if line.startswith('Sending message:'):
            cmd = line[len('Sending message: '):]
            for j in range(i+1, min(i+5, len(lines))):
                if lines[j].startswith('Response:'):
                    try:
                        resp = json.loads(lines[j].strip()[len('Response: '):])
                        pairs.append((cmd, resp))
                    except json.JSONDecodeError:
                        pass
                    break
        i += 1
    return pairs


def convert_log(input_path, output_path):
    with open(input_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()
    
    pairs = parse_pairs(lines)
    records = []
    combat_idx = 0
    in_combat = False
    current_floor = 0
    init_written = False
    
    for idx, (cmd, resp) in enumerate(pairs):
        gs = resp.get('game_state', {})
        cs = gs.get('combat_state')
        screen = gs.get('screen_type', '')
        floor = gs.get('floor', 0)
        
        # Init record
        if not init_written and gs.get('seed'):
            records.append({
                'type': 'init',
                'format_version': 2,
                'capabilities': ['rng_state', 'potions', 'pile_ids'],
                'producer': {
                    'converter': 'convert_logs.py v2',
                },
                'seed': gs.get('seed', 0),
                'class': gs.get('class', 'IRONCLAD'),
                'ascension': gs.get('ascension_level', 0),
                'relics': [r['id'] for r in gs.get('relics', [])],
                'deck': [c['id'] for c in gs.get('deck', [])],
            })
            init_written = True
        
        # Combat start: first Response with combat_state (before any play)
        if cs and cs.get('monsters') and not in_combat:
            in_combat = True
            combat_idx += 1
            current_floor = floor
            snapshot = convert_combat_state(cs, gs)
            records.append({
                'type': 'combat_start',
                'combat_idx': combat_idx,
                'floor': current_floor,
                'room_type': gs.get('room_type', 'MonsterRoom'),
                'monsters': snapshot['monsters'],
                'snapshot': snapshot,
            })
        
        # Play action: Response IS the post-action resolved state
        # Old CommunicationMod uses 1-INDEXED card positions!
        if cmd.startswith('play ') and in_combat and cs and cs.get('player'):
            parts = cmd.split()
            card_index = int(parts[1]) - 1 if len(parts) > 1 else 0  # 1-indexed → 0-indexed
            target = None
            if len(parts) > 2 and parts[2].isdigit():
                target = int(parts[2])
            
            result = convert_combat_state(cs, gs)
            records.append({
                'type': 'play',
                'combat_idx': combat_idx,
                'floor': current_floor,
                'card_index': card_index,
                'target': target,
                'result': result,
            })
        
        # End turn
        elif cmd.startswith('end') and in_combat and cs and cs.get('player'):
            result = convert_combat_state(cs, gs)
            records.append({
                'type': 'end_turn',
                'combat_idx': combat_idx,
                'floor': current_floor,
                'result': result,
            })
        
        # Potion
        elif cmd.startswith('potion ') and in_combat and cs and cs.get('player'):
            result = convert_combat_state(cs, gs)
            records.append({
                'type': 'potion',
                'combat_idx': combat_idx,
                'floor': current_floor,
                'command': cmd,
                'result': result,
            })
        
        # Combat end
        if in_combat and screen in ('COMBAT_REWARD', 'BOSS_REWARD', 'GAME_OVER', 'DEATH'):
            records.append({'type': 'combat_end', 'combat_idx': combat_idx, 'floor': current_floor})
            in_combat = False
    
    if in_combat:
        records.append({'type': 'combat_end', 'combat_idx': combat_idx, 'floor': current_floor})
    
    with open(output_path, 'w', encoding='utf-8') as f:
        for rec in records:
            f.write(json.dumps(rec, ensure_ascii=False) + '\n')
    
    plays = sum(1 for r in records if r['type'] == 'play')
    combats = sum(1 for r in records if r['type'] == 'combat_start')
    ends = sum(1 for r in records if r['type'] == 'end_turn')
    print(f"Converted {os.path.basename(input_path)}: {combats} combats, {plays} plays, {ends} end_turns")
    return combats, plays


if __name__ == '__main__':
    if len(sys.argv) < 2:
        runs_dir = 'd:/rust/runs'
        out_dir = 'd:/rust/sts_simulator/tools/replays'
        os.makedirs(out_dir, exist_ok=True)
        total_combats = total_plays = 0
        for fname in sorted(os.listdir(runs_dir)):
            if fname.endswith('.log'):
                inp = os.path.join(runs_dir, fname)
                outp = os.path.join(out_dir, fname.replace('.log', '.jsonl'))
                try:
                    c, p = convert_log(inp, outp)
                    total_combats += c
                    total_plays += p
                except Exception as e:
                    print(f"ERROR {fname}: {e}")
        print(f"\nTotal: {total_combats} combats, {total_plays} plays")
    else:
        inp = sys.argv[1]
        outp = sys.argv[2] if len(sys.argv) > 2 else inp.replace('.log', '.jsonl')
        convert_log(inp, outp)
