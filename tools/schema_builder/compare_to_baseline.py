#!/usr/bin/env python3
import sys
import json
from pathlib import Path

def main():
    if len(sys.argv) < 3:
        print("Usage: python compare_to_baseline.py <generated.json> <baseline.json>")
        sys.exit(1)
        
    generated = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    baseline = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
    
    diffs_found = 0
    
    # 1. Check top level keys
    for k, v in baseline.items():
        if k != "enums":
            if k not in generated:
                print(f"[MISSING TOP-LEVEL] {k}")
                diffs_found += 1
            elif generated[k] != v:
                print(f"[MISMATCH TOP-LEVEL] {k}: expected {v}, got {generated[k]}")
                diffs_found += 1
                
    # 2. Check enums
    for enum_key, base_data in baseline.get("enums", {}).items():
        gen_data = generated.get("enums", {}).get(enum_key, {})
        if not gen_data:
            print(f"[MISSING ENUM CATEGORY] {enum_key} entirely missing.")
            diffs_found += 1
            continue
            
        # check enum metadata
        for k, v in base_data.items():
            if k != "entries":
                if k not in gen_data:
                    print(f"[MISSING ENUM META] {enum_key}.{k}")
                    diffs_found += 1
                elif gen_data[k] != v:
                    print(f"[MISMATCH ENUM META] {enum_key}.{k}: expected {v}, got {gen_data[k]}")
                    diffs_found += 1
        
        base_entries = base_data.get("entries", {})
        gen_entries = gen_data.get("entries", {})
        
        # We enforce that ALL base entries must exist in gen entries and be EQUAL or BETTER
        for rust_id, b_entry in base_entries.items():
            if rust_id not in gen_entries:
                print(f"[MISSING RUST VARIANT] {enum_key}.{rust_id} lost completely!")
                diffs_found += 1
                continue
                
            g_entry = gen_entries[rust_id]
            
            # Check 'java' array (must be a superset at least, or exact match)
            b_java = set(b_entry.get("java", []))
            g_java = set(g_entry.get("java", []))
            if not b_java.issubset(g_java):
                print(f"[JAVA ARRAY REGRESSION] {enum_key}.{rust_id}: Baseline had {b_java}, Generated only has {g_java}")
                diffs_found += 1
            elif b_java != g_java:
                # Superset is good, but log it as an upgrade
                print(f"[UPGRADE] {enum_key}.{rust_id} java array grew from {b_java} to {g_java}")
                
            # Check other attrs (scope, index, owner, class)
            for k, v in b_entry.items():
                if k not in ["java", "rust"]:
                    if k not in g_entry:
                        print(f"[MISSING ATTR] {enum_key}.{rust_id}.{k} (was {v})")
                        diffs_found += 1
                    elif g_entry[k] != v:
                        print(f"[MISMATCH ATTR] {enum_key}.{rust_id}.{k} expected {v}, got {g_entry[k]}")
                        diffs_found += 1

    if diffs_found == 0:
        print("\nSUCCESS! Generated schema is PERFECTLY PARITY or BLOWS PAST baseline!")
        sys.exit(0)
    else:
        print(f"\nFailed: {diffs_found} regressions against baseline.")
        sys.exit(1)

if __name__ == '__main__':
    main()
