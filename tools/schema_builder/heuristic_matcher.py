#!/usr/bin/env python3
import sys
import json
import re
from pathlib import Path

CATEGORY_TO_ENUM = {
    "relic": "relic_id",
    "monster": "monster_id",
    "card": "card_id",
    "power": "power_id",
    "potion": "potion_id"
}

def load_json(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def to_snake_case(name: str) -> str:
    name = name.replace(" ", "_").replace("-", "_").replace(":", "_").replace("'", "")
    s1 = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    result = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s1).lower()
    return re.sub(r"_+", "_", result)

def main():
    if len(sys.argv) < 6:
        print("Usage: python heuristic_matcher.py <java.json> <rust.json> <skeleton.json> <overrides.json> <output.json>")
        sys.exit(1)
        
    java_file, rust_file, skeleton_file, overrides_file, out_file = [Path(p) for p in sys.argv[1:6]]
    
    java_nodes = load_json(java_file)
    rust_nodes = load_json(rust_file)
    final_schema = load_json(skeleton_file)
    overrides = load_json(overrides_file)
    
    # Process each enum type
    for category, rust_list in rust_nodes.items():
        enum_key = CATEGORY_TO_ENUM[category]
        java_dict = java_nodes.get(category, {})
        overrides_dict = overrides.get(enum_key, {})
        
        entries = final_schema["enums"][enum_key]["entries"]
        
        for rust_id in rust_list:
            override_data = overrides_dict.get(rust_id, {})
            
            entry = {
                "rust": rust_id
            }
            
            # Scope rules
            if category in ["relic", "card"]:
                entry["scope"] = "combat"
            elif category == "power":
                if "owner" not in override_data:
                    entry["scope"] = "combat"
            
            # Mixin overrides
            for k, v in override_data.items():
                entry[k] = v
                
            # If java is not overridden, try heuristic match
            if "java" not in entry:
                match_found = False
                
                # 1. Exact Name/ID match
                if rust_id in java_dict:
                    entry["java"] = [rust_id]
                    match_found = True
                
                # 2. Heuristic Java ID Match
                if not match_found:
                    rust_snake = to_snake_case(rust_id)
                    for j_id, j_data in java_dict.items():
                        if to_snake_case(j_id) == rust_snake or to_snake_case(j_data["class_name"]) == rust_snake:
                            entry["java"] = [j_id]
                            match_found = True
                            break
                            
                if not match_found:
                    entry["java"] = [rust_id] # Default fallback, though it won't map
            
            entries[rust_id] = entry
            
    out_file.write_text(json.dumps(final_schema, indent=4), encoding="utf-8")
    print(f"Matcher complete. Pure schema built into {out_file}")

if __name__ == '__main__':
    main()
