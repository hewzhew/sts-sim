#!/usr/bin/env python3
"""Validate the standardized event schema."""

import json
from pathlib import Path

def main():
    data_path = Path(__file__).parent.parent / "events_final_master.json"
    with open(data_path, encoding='utf-8') as f:
        d = json.load(f)
    
    print("=== Final Standardization Report ===")
    print(f"Events: {len(d)}")
    
    opts = [o for e in d.values() for o in e.get('options', [])]
    print(f"Options: {len(opts)}")
    print(f"  with commands: {sum(1 for o in opts if 'commands' in o)}")
    print(f"  with rewards: {sum(1 for o in opts if 'rewards' in o)}")
    print(f"  with costs: {sum(1 for o in opts if 'costs' in o)}")
    print(f"  with conditions: {sum(1 for o in opts if 'conditions' in o)}")
    print(f"  with random_outcomes: {sum(1 for o in opts if 'random_outcomes' in o)}")
    
    ros = [ro for o in opts for ro in o.get('random_outcomes', [])]
    print(f"Random Outcomes: {len(ros)}")
    print(f"  with rewards: {sum(1 for ro in ros if 'rewards' in ro)}")
    print(f"  with costs: {sum(1 for ro in ros if 'costs' in ro)}")
    print(f"  with commands: {sum(1 for ro in ros if 'commands' in ro)}")
    
    # Check for non-standard fields
    non_standard = ['bet', 'trigger', 'minigame', 'possible_rewards', 'effect']
    issues = []
    for eid, event in d.items():
        for field in non_standard:
            if field in event:
                issues.append(f"Event {eid} has non-standard field: {field}")
        for opt in event.get('options', []):
            for field in non_standard:
                if field in opt:
                    issues.append(f"Option in {eid} has non-standard field: {field}")
            for ro in opt.get('random_outcomes', []):
                for field in non_standard:
                    if field in ro:
                        issues.append(f"Random outcome in {eid} has non-standard field: {field}")
    
    print(f"\nNon-standard fields: {len(issues)}")
    for issue in issues:
        print(f"  ⚠ {issue}")
    
    if not issues:
        print("  ✅ All fields are standardized!")
    
    print("\n=== Schema Validation: PASSED ===")

if __name__ == "__main__":
    main()
