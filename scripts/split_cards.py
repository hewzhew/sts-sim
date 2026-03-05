#!/usr/bin/env python3
"""Split cards.json into per-color/per-rarity JSON files.

Usage:
    python scripts/split_cards.py [--source data/cards.json] [--output data/cards]

Reads the monolithic cards.json and splits into:
    data/cards/{color}/{rarity}.json

Cards within each file are sorted by: type (Attack > Skill > Power > Status > Curse), then ID.
"""

import json
import os
import sys
import argparse
from collections import defaultdict
from pathlib import Path


# Canonical sort order for card types
TYPE_ORDER = {
    "Attack": 0,
    "Skill": 1, 
    "Power": 2,
    "Status": 3,
    "Curse": 4,
}

# Map color names to directory names (lowercase)
COLOR_DIR = {
    "Red": "red",
    "Green": "green",
    "Blue": "blue",
    "Purple": "purple",
    "Colorless": "colorless",
}

# Map rarity names to file names (lowercase)
RARITY_FILE = {
    "Basic": "basic",
    "Common": "common",
    "Uncommon": "uncommon",
    "Rare": "rare",
    "Curse": "curse",
    "Special": "special",
}


def card_sort_key(card):
    """Sort key: type order, then alphabetical by ID."""
    type_idx = TYPE_ORDER.get(card.get("type", ""), 99)
    return (type_idx, card.get("id", ""))


def split_cards(source_path: str, output_dir: str, patch_path: str = None):
    """Split source JSON into directory structure."""
    
    # Load source
    print(f"Reading {source_path}...")
    with open(source_path, "r", encoding="utf-8") as f:
        cards = json.load(f)
    print(f"  Loaded {len(cards)} cards")
    
    # Optionally merge patches
    if patch_path and os.path.exists(patch_path):
        print(f"Reading patches from {patch_path}...")
        with open(patch_path, "r", encoding="utf-8") as f:
            patched = json.load(f)
        
        # Build patch lookup by ID
        patch_map = {c["id"]: c for c in patched}
        source_ids = {c["id"] for c in cards}
        
        # Override source cards with patched versions
        merged = []
        for card in cards:
            if card["id"] in patch_map:
                merged.append(patch_map[card["id"]])
            else:
                merged.append(card)
        
        # Add cards that only exist in patch
        for pid, pcard in patch_map.items():
            if pid not in source_ids:
                merged.append(pcard)
                print(f"  + Added patch-only card: {pid}")
        
        cards = merged
        print(f"  Merged total: {len(cards)} cards")
    
    # Group by color -> rarity
    groups = defaultdict(list)
    unknown = []
    
    for card in cards:
        color = card.get("color", "")
        rarity = card.get("rarity", "")
        
        color_dir = COLOR_DIR.get(color)
        rarity_file = RARITY_FILE.get(rarity)
        
        if color_dir and rarity_file:
            groups[(color_dir, rarity_file)].append(card)
        else:
            unknown.append(card)
            print(f"  ⚠ Unknown color/rarity: {card.get('id')} -> {color}/{rarity}")
    
    # Write output files
    os.makedirs(output_dir, exist_ok=True)
    total_written = 0
    file_count = 0
    
    for (color_dir, rarity_file), card_list in sorted(groups.items()):
        # Sort cards within group
        card_list.sort(key=card_sort_key)
        
        # Create directory
        dir_path = os.path.join(output_dir, color_dir)
        os.makedirs(dir_path, exist_ok=True)
        
        # Write JSON file
        file_path = os.path.join(dir_path, f"{rarity_file}.json")
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(card_list, f, indent=2, ensure_ascii=False)
            f.write("\n")
        
        total_written += len(card_list)
        file_count += 1
        print(f"  ✓ {file_path}: {len(card_list)} cards")
    
    # Handle unknown cards
    if unknown:
        unknown.sort(key=card_sort_key)
        file_path = os.path.join(output_dir, "_unknown.json")
        with open(file_path, "w", encoding="utf-8") as f:
            json.dump(unknown, f, indent=2, ensure_ascii=False)
            f.write("\n")
        total_written += len(unknown)
        file_count += 1
        print(f"  ⚠ {file_path}: {len(unknown)} unknown cards")
    
    print(f"\nDone! {total_written} cards → {file_count} files in {output_dir}/")
    
    # Verification
    if total_written != len(cards):
        print(f"  ❌ ERROR: {len(cards)} input but {total_written} output!")
        sys.exit(1)
    else:
        print(f"  ✅ All {total_written} cards accounted for")
    
    return total_written, file_count


def main():
    parser = argparse.ArgumentParser(description="Split cards.json into per-color/per-rarity files")
    parser.add_argument("--source", default="data/cards.json", help="Source cards.json path")
    parser.add_argument("--patch", default=None, help="Optional patched JSON to merge")
    parser.add_argument("--output", default="data/cards", help="Output directory")
    args = parser.parse_args()
    
    split_cards(args.source, args.output, args.patch)


if __name__ == "__main__":
    main()
