#!/usr/bin/env python3
import sys
import json
from pathlib import Path

def main():
    if len(sys.argv) < 3:
        print("Usage: python schema_compiler.py <matched.json> <final_schema.json>")
        sys.exit(1)
        
    matched_file = Path(sys.argv[1])
    out_file = Path(sys.argv[2])
    
    schema = json.loads(matched_file.read_text(encoding="utf-8"))
    
    anomalies = []
    
    for enum_key, enum_data in schema.get("enums", {}).items():
        entries = enum_data.get("entries", {})
        for rust_id, data in entries.items():
            if data.get("_match_confidence") == "failure":
                anomalies.append(f"[{enum_key}] {rust_id} -> UNKNOWN JAVA ID")
            
            # Clean up our internal confidence keys so the final schema is pristine
            if "_match_confidence" in data:
                del data["_match_confidence"]

    out_file.write_text(json.dumps(schema, indent=2), encoding="utf-8")
    
    if anomalies:
        print("WARNING: Automatically compiled schema contains unassociated Rust entities:")
        for a in anomalies:
            print(f"  - {a}")
        print("\nPlease update the baseline protocol_schema.json with the correct Java ID.")
    else:
        print(f"Success! Schema uniquely compiled and ready for build.rs at {out_file}")

if __name__ == '__main__':
    main()
