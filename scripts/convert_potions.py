#!/usr/bin/env python3
"""
Convert Lua potion data (from Wiki) to structured JSON for the Rust simulator.

Usage:
    python scripts/convert_potions.py data/potions.txt data/potions.json
"""

import re
import json
import sys
from pathlib import Path
from typing import Optional, Dict, Any, List, Tuple


def parse_lua_table(content: str) -> Dict[str, Dict[str, str]]:
    """Parse a Lua table format into a Python dictionary."""
    potions = {}
    
    # Pattern to match each potion entry: ["Name"] = { ... }
    # We need to handle multi-line entries
    entry_pattern = re.compile(
        r'\["([^"]+)"\]\s*=\s*\{([^}]+)\}',
        re.MULTILINE | re.DOTALL
    )
    
    for match in entry_pattern.finditer(content):
        name = match.group(1)
        fields_str = match.group(2)
        
        # Parse individual fields within the entry
        fields = {}
        
        # Match field = "value" or field = value patterns
        field_pattern = re.compile(r'(\w+)\s*=\s*"([^"]*)"')
        for field_match in field_pattern.finditer(fields_str):
            field_name = field_match.group(1)
            field_value = field_match.group(2)
            fields[field_name] = field_value
        
        potions[name] = fields
    
    return potions


def extract_potency(text: str) -> Tuple[Optional[int], Optional[int]]:
    """
    Extract potency values from text like '<6:12>' or '<20%:40%>'.
    Returns (base_value, upgraded_value) or (None, None) if not found.
    """
    # Match patterns like <6:12> or <20%:40%>
    match = re.search(r'<(\d+)%?:(\d+)%?>', text)
    if match:
        return int(match.group(1)), int(match.group(2))
    
    # Single value pattern <5>
    match = re.search(r'<(\d+)>', text)
    if match:
        val = int(match.group(1))
        return val, val
    
    return None, None


def infer_target(text: str) -> str:
    """Infer target type from description text."""
    text_lower = text.lower()
    
    if "target enemy" in text_lower:
        return "Enemy"
    if "all enemies" in text_lower:
        return "AllEnemies"
    if any(kw in text_lower for kw in ["gain", "draw", "heal", "your hand", "enter", "exhaust", "upgrade"]):
        return "Self"
    if "orb" in text_lower:
        return "Self"
    
    # Default
    return "Self"


def infer_command(text: str, name: str) -> str:
    """Infer the command type based on description keywords."""
    text_lower = text.lower()
    name_lower = name.lower()
    
    # Damage effects
    if "deal" in text_lower and "damage" in text_lower:
        if "all enemies" in text_lower:
            return "DealDamageAll"
        return "DealDamage"
    
    # Status effects
    if "#poison" in text_lower or "poison" in name_lower:
        return "ApplyPoison"
    if "#vulnerable" in text_lower or "fear" in name_lower:
        return "ApplyVulnerable"
    if "#weak" in text_lower or "weak" in name_lower:
        return "ApplyWeak"
    
    # Buffs
    if "#strength" in text_lower or "strength" in name_lower:
        return "GainStrength"
    if "#dexterity" in text_lower or "dexterity" in name_lower:
        return "GainDexterity"
    if "#block" in text_lower or "block" in name_lower:
        return "GainBlock"
    if "#energy" in text_lower or "energy" in name_lower:
        return "GainEnergy"
    if "#focus" in text_lower or "focus" in name_lower:
        return "GainFocus"
    if "#artifact" in text_lower or "ancient" in name_lower:
        return "GainArtifact"
    if "#intangible" in text_lower or "ghost" in name_lower:
        return "GainIntangible"
    if "#thorns" in text_lower or "bronze" in name_lower:
        return "GainThorns"
    if "#plated armor" in text_lower or "steel" in name_lower:
        return "GainPlatedArmor"
    if "#metallicize" in text_lower or "heart of iron" in name_lower:
        return "GainMetallicize"
    if "#regeneration" in text_lower or "regen" in name_lower:
        return "GainRegeneration"
    if "#ritual" in text_lower or "cultist" in name_lower:
        return "GainRitual"
    
    # Card manipulation
    if "draw" in text_lower:
        return "DrawCards"
    if "discard" in text_lower:
        return "DiscardCards"
    if "exhaust" in text_lower:
        return "ExhaustCards"
    if "upgrade" in text_lower:
        return "UpgradeCards"
    if "add" in text_lower and ("hand" in text_lower or "card" in text_lower):
        return "AddCardToHand"
    if "play the top" in text_lower:
        return "PlayTopCards"
    
    # Stance (Watcher)
    if "#calm" in text_lower or "#wrath" in text_lower or "#divinity" in text_lower:
        return "EnterStance"
    if "stance" in name_lower:
        return "EnterStance"
    
    # Orbs (Defect)
    if "orb slot" in text_lower or "capacity" in name_lower:
        return "GainOrbSlots"
    if "channel" in text_lower and "#dark" in text_lower:
        return "ChannelDark"
    
    # Healing
    if "heal" in text_lower or "max hp" in text_lower:
        return "Heal"
    if "fairy" in name_lower:
        return "FairyRevive"
    if "liquid memories" in name_lower:
        return "RecallFromDiscard"
    
    # Discard related
    if "discard" in text_lower and "return" not in text_lower:
        return "DiscardCards"
    
    # Special
    if "escape" in text_lower or "smoke" in name_lower:
        return "Escape"
    if "played twice" in text_lower or "duplication" in name_lower:
        return "DoubleTap"
    if "fill all" in text_lower and "potion" in text_lower:
        return "FillPotions"
    if "randomize the cost" in text_lower:
        return "RandomizeCosts"
    if "choose" in text_lower and "card" in text_lower:
        return "DiscoverCard"
    if "return it to your hand" in text_lower:
        return "RecallFromDiscard"
    if "miracle" in text_lower:
        return "AddMiracles"
    if "shiv" in text_lower:
        return "AddShivs"
    
    # Card type potions (Attack/Skill/Power/Colorless Potion)
    if "attack" in name_lower and "potion" in name_lower:
        return "DiscoverAttack"
    if "skill" in name_lower and "potion" in name_lower:
        return "DiscoverSkill"
    if "power" in name_lower and "potion" in name_lower:
        return "DiscoverPower"
    if "colorless" in name_lower and "potion" in name_lower:
        return "DiscoverColorless"
    if "cunning" in name_lower:
        return "AddShivs"
    if "bottled" in name_lower and "miracle" in name_lower:
        return "AddMiracles"
    if "bottled" in name_lower and "miracle" in text_lower:
        return "AddMiracles"
    
    return "Unknown"


def clean_description(text: str) -> str:
    """Clean up Wiki markup from description."""
    # Remove {{QueryLink|...|...|Display}} -> Display
    text = re.sub(r'\{\{QueryLink\|[^|]+\|[^|]+\|([^}]+)\}\}', r'\1', text)
    # Remove {{C|...|Display}} -> Display
    text = re.sub(r'\{\{C\|[^|]+\|([^}]+)\}\}', r'\1', text)
    # Remove [[...]] links
    text = re.sub(r'\[\[([^\]]+)\]\]', r'\1', text)
    # Remove # prefixes (wiki keyword markers)
    text = re.sub(r'#(\w+)', r'\1', text)
    # Clean up <:...> patterns (alternative text)
    text = re.sub(r'<:([^>]+)>', r'', text)
    text = re.sub(r'<([^:>]+):([^>]+)>', r'\1', text)
    return text.strip()


def convert_potion(name: str, data: Dict[str, str]) -> Dict[str, Any]:
    """Convert a single potion entry to our JSON schema."""
    text = data.get("Text", "")
    
    # Extract potency
    base_potency, upgraded_potency = extract_potency(text)
    
    # Check for percentage-based effects
    is_percent = "%" in text and base_potency is not None
    
    potion = {
        "id": name.replace(" ", ""),  # e.g., "AttackPotion"
        "name": name,
        "rarity": data.get("Rarity", "Common"),
        "class": data.get("Character", "Any"),
        "target": infer_target(text),
        "description": clean_description(text),
        "command_hint": infer_command(text, name),
    }
    
    # Add potency if found
    if base_potency is not None:
        if is_percent:
            potion["potency_percent"] = base_potency
            if upgraded_potency and upgraded_potency != base_potency:
                potion["potency_percent_upgraded"] = upgraded_potency
        else:
            potion["potency"] = base_potency
            if upgraded_potency and upgraded_potency != base_potency:
                potion["potency_upgraded"] = upgraded_potency
    
    return potion


def main():
    # Default paths
    input_path = Path("data/potions.txt")
    output_path = Path("data/potions.json")
    
    # Allow command line overrides
    if len(sys.argv) >= 2:
        input_path = Path(sys.argv[1])
    if len(sys.argv) >= 3:
        output_path = Path(sys.argv[2])
    
    print(f"Reading from: {input_path}")
    
    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}")
        sys.exit(1)
    
    # Read and parse Lua data
    with open(input_path, 'r', encoding='utf-8') as f:
        lua_content = f.read()
    
    potions_lua = parse_lua_table(lua_content)
    print(f"Parsed {len(potions_lua)} potions from Lua")
    
    # Convert to our schema
    potions_json = []
    for name, data in potions_lua.items():
        potion = convert_potion(name, data)
        potions_json.append(potion)
        print(f"  [OK] {name} -> {potion['command_hint']}")
    
    # Sort by name for consistent output
    potions_json.sort(key=lambda p: p["name"])
    
    # Write JSON
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(potions_json, f, indent=2, ensure_ascii=False)
    
    print(f"\n✅ Wrote {len(potions_json)} potions to: {output_path}")
    
    # Summary statistics
    commands = {}
    for p in potions_json:
        cmd = p["command_hint"]
        commands[cmd] = commands.get(cmd, 0) + 1
    
    print("\n📊 Command distribution:")
    for cmd, count in sorted(commands.items(), key=lambda x: -x[1]):
        print(f"  {cmd}: {count}")
    
    unknown = [p["name"] for p in potions_json if p["command_hint"] == "Unknown"]
    if unknown:
        print(f"\n⚠️ Unknown commands ({len(unknown)}):")
        for name in unknown:
            print(f"  - {name}")


if __name__ == "__main__":
    main()
