#!/usr/bin/env python3
import sys
import json
import re
from pathlib import Path

# A simple regex-based extractor for Rust Enums.
# For full AST fidelity we would use tree-sitter-rust, but regex is robust enough for well-formatted Enums.

ENUM_MAP = {
    "RelicId": "relic",
    "EnemyId": "monster",
    "CardId": "card",
    "PowerId": "power",
    "PotionId": "potion"
}

def extract_enum_variants(source: str, enum_name: str):
    variants = []
    # Match pub enum EnumName { ... }
    pattern = r"pub\s+enum\s+" + enum_name + r"\s*\{([^}]*)\}"
    match = re.search(pattern, source, re.MULTILINE | re.DOTALL)
    if not match:
        return variants
        
    block = match.group(1)
    
    # Extract line by line ignoring comments
    for line in block.split('\n'):
        # strip comments
        line = line.split('//')[0].strip()
        if not line:
            continue
        
        # Variants could be `Name,` or `Name(T),` or `Name { ... },`
        # We only want the base identifier
        match_id = re.match(r"^([A-Za-z0-9_]+)", line)
        if match_id:
            variants.append(match_id.group(1))
            
    return variants

def main():
    if len(sys.argv) < 3:
        print("Usage: python rust_crawler.py <rust_src_dir> <output.json>")
        sys.exit(1)
        
    src_dir = Path(sys.argv[1])
    out_file = Path(sys.argv[2])
    
    if not src_dir.exists() or not src_dir.is_dir():
        print(f"Error: {src_dir} does not exist.")
        sys.exit(1)

    entities = { category: [] for category in ENUM_MAP.values() }
    
    for fpath in src_dir.rglob("*.rs"):
        source = fpath.read_text(encoding="utf-8", errors="replace")
        
        for enum_name, category in ENUM_MAP.items():
            if f"pub enum {enum_name}" in source:
                variants = extract_enum_variants(source, enum_name)
                if variants:
                    entities[category].extend(variants)

    out_file.write_text(json.dumps(entities, indent=2), encoding="utf-8")
    print(f"Extraction complete! Found {sum(len(v) for v in entities.values())} Rust variants. Wrote to {out_file}")

if __name__ == '__main__':
    main()
