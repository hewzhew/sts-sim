#!/usr/bin/env python3
"""Monster behavior audit: cross-reference Java sources with JSON behavior models."""

import json
import os
import sys
import re

JAVA_BASE = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'cardcrawl', 'monsters')
JSON_PATH = os.path.join(os.path.dirname(__file__), '..', '..', 'data', 'monsters_with_behavior.json')
RUST_ENEMY = os.path.join(os.path.dirname(__file__), '..', '..', 'src', 'monsters', 'enemy.rs')

def load_monsters():
    with open(JSON_PATH, encoding='utf-8') as f:
        data = json.load(f)
    return data.get('monsters', data) if isinstance(data, dict) else data

def find_java_file(internal_id):
    """Find the Java source file for a monster."""
    for subdir in ['exordium', 'city', 'beyond', 'ending']:
        path = os.path.join(JAVA_BASE, subdir, f'{internal_id}.java')
        if os.path.exists(path):
            return path, subdir
    return None, None

def analyze_java_getmove(path):
    """Extract key patterns from Java getMove() method."""
    with open(path, encoding='utf-8') as f:
        content = f.read()
    
    info = {}
    # Check for firstMove flag
    info['has_first_move'] = 'firstMove' in content
    # Check for lastMove/lastTwoMoves  
    info['uses_lastMove'] = 'lastMove(' in content
    info['uses_lastTwoMoves'] = 'lastTwoMoves(' in content
    info['uses_lastMoveBefore'] = 'lastMoveBefore(' in content
    # Check for HP threshold
    info['has_hp_check'] = 'currentHealth' in content or 'maxHealth' in content
    # Check for usePreBattleAction
    info['has_pre_battle'] = 'usePreBattleAction' in content
    # Count move IDs (byte constants)
    byte_constants = re.findall(r'private static final byte (\w+)\s*=\s*(\d+)', content)
    info['move_count'] = len(byte_constants)
    info['moves'] = {name: int(val) for name, val in byte_constants}
    # Check for multi-phase
    info['is_multi_phase'] = 'halfDead' in content or 'phase' in content.lower()
    # Check for ascension branching
    info['has_ascension_ai'] = 'ascensionLevel' in content and 'getMove' in content.split('ascensionLevel')[-1][:200] if 'ascensionLevel' in content else False
    
    return info

def analyze_json_behavior(monster):
    """Analyze the JSON behavior model for completeness."""
    bm = monster.get('behavior_model', {})
    if not bm:
        return {'status': 'missing', 'logic_type': None}
    
    info = {
        'logic_type': bm.get('logic_type', 'Unknown'),
        'has_init_seq': bool(bm.get('init_sequence')),
        'rule_count': len(bm.get('rules', [])),
        'has_cycle': bool(bm.get('cycle')),
        'has_phases': bool(bm.get('phases')),
        'has_asc20': bool(monster.get('behavior_model_asc20')),
    }
    
    # Check for issues
    issues = []
    if info['logic_type'] == 'Reference':
        issues.append('Uses Reference (needs resolution)')
    if info['logic_type'] == 'Unknown':
        issues.append('Unknown logic type')
    
    # Check move name consistency
    move_names = {m['name'] for m in monster.get('moves', [])}
    rule_moves = set()
    for r in bm.get('rules', []):
        rule_moves.add(r.get('move', ''))
    
    mismatched = rule_moves - move_names - {''}
    if mismatched:
        issues.append(f'Rule moves not in moves list: {mismatched}')
    
    info['issues'] = issues
    info['status'] = 'ok' if not issues else 'issues'
    return info

def main():
    monsters = load_monsters()
    
    # Organize by act
    acts = {}
    for m in monsters:
        act = m.get('act', 'Unknown')
        acts.setdefault(act, []).append(m)
    
    print(f"# Monster Behavior Audit\n")
    print(f"**Total monsters in JSON**: {len(monsters)}\n")
    
    total_ok = 0
    total_issues = 0
    total_missing_java = 0
    total_reference = 0
    all_issues = []
    
    for act_name in sorted(acts.keys()):
        act_monsters = acts[act_name]
        print(f"\n## {act_name} ({len(act_monsters)} monsters)\n")
        print("| Monster | Internal ID | Java | Logic Type | Moves | Issues |")
        print("|---------|------------|------|------------|-------|--------|")
        
        for m in sorted(act_monsters, key=lambda x: x.get('type', '')):
            internal_id = m.get('internal_id', m['id'].replace(' ', ''))
            java_path, java_dir = find_java_file(internal_id)
            java_status = f"✅ {java_dir}" if java_path else "❌ missing"
            if not java_path:
                total_missing_java += 1
            
            json_info = analyze_json_behavior(m)
            logic = json_info['logic_type'] or '—'
            move_count = len(m.get('moves', []))
            
            # Java analysis
            java_info = {}
            if java_path:
                java_info = analyze_java_getmove(java_path)
            
            # Determine issues
            issues_str = ''
            if json_info['issues']:
                issues_str = '; '.join(json_info['issues'])
                total_issues += 1
                all_issues.append((m['id'], issues_str))
            elif json_info['logic_type'] == 'Reference':
                issues_str = 'Reference'
                total_reference += 1
            else:
                total_ok += 1
            
            # Flag Java vs JSON discrepancies
            if java_info:
                if java_info['has_first_move'] and not json_info.get('has_init_seq'):
                    if issues_str: issues_str += '; '
                    issues_str += '⚠️ Java has firstMove but no init_sequence'
                if java_info['uses_lastTwoMoves'] and json_info['logic_type'] == 'Repeat':
                    if issues_str: issues_str += '; '
                    issues_str += '⚠️ Java uses lastTwoMoves but Rust is Repeat'
            
            monster_type = m.get('type', '?')
            name = m['id']
            if len(name) > 20:
                name = name[:18] + '..'
            
            print(f"| {name} | {internal_id} | {java_status} | {logic} | {move_count} | {issues_str or '✅'} |")
    
    # Summary
    print(f"\n## Summary\n")
    print(f"| Metric | Count |")
    print(f"|--------|-------|")
    print(f"| Total monsters | {len(monsters)} |")
    print(f"| Behavior OK | {total_ok} |")
    print(f"| Has issues | {total_issues} |")
    print(f"| Reference (needs resolution) | {total_reference} |")
    print(f"| Missing Java source | {total_missing_java} |")
    
    if all_issues:
        print(f"\n## Issues to Fix\n")
        for name, issue in all_issues:
            print(f"- **{name}**: {issue}")
    
    return 0 if total_issues == 0 else 1

if __name__ == '__main__':
    sys.exit(main())
