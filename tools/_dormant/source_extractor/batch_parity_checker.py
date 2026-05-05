#!/usr/bin/env python3
"""
batch_parity_checker.py - Automated Exordium Monster Batch Audit

1. Uses tree-sitter AST traversal to extract invoked Actions and Powers from Java.
2. Reads Rust action and power definitions to ensure the backend supports them.
3. Scans Rust implementation files to check if the translated monster calls them.
"""

import sys
import argparse
import re
import json
from pathlib import Path

# Important: Imports the AST logic we built in monster_ast.py
try:
    import monster_ast as mast
except ImportError:
    import source_extractor.monster_ast as mast

# Maps Java Slay The Spire class names to our Rust Enum variants.
# This avoids false positives and translates Java concepts to Rust concepts.
JAVA_TO_RUST_ACTION = {
    "DamageAction": "Damage",
    "ApplyPowerAction": "ApplyPower",
    "MakeTempCardInDrawPileAction": "MakeTempCardInDrawPile",
    "MakeTempCardInDiscardAction": "MakeTempCardInDiscard",
    "SuicideAction": "Suicide",
    "HealAction": "Heal",
    "GainBlockAction": "GainBlock",
    "RollMoveAction": "RollMonsterMove", # Not strictly mapped but functionally matches SetMonsterMove sometimes
    "SpawnMonsterAction": "SpawnMonsterSmart", # Rust unified spawner
    "SetMoveAction": "SetMonsterMove",
    "ChangeStateAction": "ChangeState",
    # Ignored UI actions
    "TalkAction": None,
    "SFXAction": None,
    "AnimateSlowAttackAction": None,
    "AnimateFastAttackAction": None,
    "AnimateShakeAction": None,
    "AnimateHopAction": None,
    "AnimateJumpAction": None,
    "WaitAction": None,
    "HideHealthBarAction": None,
    "CannotLoseAction": None,
    "CanLoseAction": None,
    "TextAboveCreatureAction": None,
    "VFXAction": None,
    "ShakeScreenAction": None,
    "FastShakeAction": None,
    "SetAnimationAction": None,
    "ShoutAction": None,
    "ChangeStateAction": None,
    "RemoveAllBlockAction": "RemoveAllBlock",
    "LoseBlockAction": "LoseBlock",
    "GainBlockRandomMonsterAction": "GainBlockRandomMonster",
    "ApplyPowerToRandomEnemyAction": "ApplyPower", # Rust handles randomness via engine? Or missing
    "DamageCallbackAction": "Damage",
    "EscapeAction": "Escape",
    "SummonGremlinAction": "SpawnMonsterSmart",
    "RemoveDebuffsAction": "RemoveAllDebuffs", # or RemoveDebuffs? We'll see
    "ApplyStasisAction": "ApplyStasis", # check if engine has it
    "VampireDamageAction": "VampireDamage",
    "BurnIncreaseAction": "UpgradeAllBurns",
    "ReducePowerAction": "RemovePower", # Usually generic fallback
    "RemoveSpecificPowerAction": "RemovePower",
    "ClearCardQueueAction": "ClearCardQueue",
    "MakeTempCardInDiscardAndDeckAction": "MakeTempCardInDiscardAndDeck",
    "LoseHPAction": "LoseHp",
    "AddCardToDeckAction": "AddCardToMasterDeck",
}

def load_schema_mappings():
    schema_path = Path(__file__).resolve().parent.parent / "compiled_protocol_schema.json"
    if not schema_path.exists():
        return {}, {}
    
    with open(schema_path, "r", encoding="utf-8") as f:
        schema = json.load(f)
        
    power_mapping = {}
    if "power_id" in schema.get("enums", {}):
        for rust_variant, data in schema["enums"]["power_id"].get("entries", {}).items():
            for j_name in data.get("java", []):
                # Ensure Power suffix is mapped correctly if AST adds it
                # Slay the Spire AST extracts "StrengthPower"
                if not j_name.endswith("Power"):
                    power_mapping[j_name + "Power"] = rust_variant
                power_mapping[j_name] = rust_variant

    monster_mapping = {}
    if "monster_id" in schema.get("enums", {}):
        for rust_variant, data in schema["enums"]["monster_id"].get("entries", {}).items():
            for j_name in data.get("java", []):
                monster_mapping[j_name] = rust_variant
                
    return power_mapping, monster_mapping

JAVA_TO_RUST_POWER, JAVA_TO_RUST_MONSTER = load_schema_mappings()

def extract_rust_enum_variants(filepath, enum_name):
    code = filepath.read_text(encoding='utf-8')
    # More robust logic: find the enum definition line, then read until we hit a non-indented `}`
    variants = set()
    in_enum = False
    for line in code.split('\n'):
        if f'pub enum {enum_name} {{' in line:
            in_enum = True
            continue
        if in_enum and line.startswith('}'):
            break
        if in_enum:
            stripped = line.split('//')[0].strip()
            if not stripped: continue
            
            # Action variants usually match CapitalizedWord
            v_match = re.match(r'^([A-Z][A-Za-z0-9_]*)', stripped)
            if v_match:
                variants.add(v_match.group(1))
    return variants

def parse_java_power_action(java_statement: str):
    """
    Returns (action_type, [power_types]) from a java statement using crude regex text scanning
    on the AST dumped node text.
    """
    action_type_match = re.search(r'new\s+([A-Za-z0-9_]+Action)\(', java_statement)
    action_type = action_type_match.group(1) if action_type_match else None

    # Search for nested new XXXPower
    power_matches = re.finditer(r'new\s+([A-Za-z0-9_]+Power)\(', java_statement)
    powers = [m.group(1) for m in power_matches]
    
    return action_type, powers

def analyze_java_file(fpath: Path):
    info = mast.analyze_monster(fpath)
    if not info: return None

    actions_needed = set()
    powers_needed = set()

    for _, actions in info.get("take_turn", {}).items():
        for act_str in actions:
            a, p_list = parse_java_power_action(act_str)
            if a: actions_needed.add(a)
            powers_needed.update(p_list)
            
    # Also check usePreBattleAction if mast updated to extract it (it currently skips prebattle)
    # We will do a generic regex over the file just to perfectly capture any hidden new Power(...) or new Action(...)
    src, _ = mast.read_source(fpath)
    # Fallback sweeping:
    for m in re.finditer(r'new\s+([A-Za-z0-9_]+Action)\(', src):
        actions_needed.add(m.group(1))
    for m in re.finditer(r'new\s+([A-Za-z0-9_]+Power)\(', src):
        powers_needed.add(m.group(1))

    return {
        "class_name": info["class_name"],
        "actions_needed": actions_needed,
        "powers_needed": powers_needed
    }

def pascal_to_snake(name: str):
    # AcidSlime_L -> acid_slime_l; Spine... ignore
    name = re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()
    return name.replace("__", "_").replace("__", "_")

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--src", type=str, required=True, help="Path to Java monster directory")
    parser.add_argument("--out", type=str, required=True, help="Output markdown file path")
    args = parser.parse_args()

    java_root = Path(args.src)
    layer = java_root.name # 'exordium', 'city', or 'beyond'
    
    rust_root = Path(r"d:\rust\sts_simulator\src")
    rust_monster_dir = rust_root / "content" / "monsters" / layer
    
    # 1. Load Rust Engines capabilities
    action_enum_file = rust_root / "action.rs"
    power_enum_file = rust_root / "content" / "powers" / "mod.rs"
    
    supported_actions = extract_rust_enum_variants(action_enum_file, "Action")
    supported_powers = extract_rust_enum_variants(power_enum_file, "PowerId")

    # Override/Add specific rust action mechanics that map generically
    supported_actions.add("SetMonsterMove")
    supported_actions.add("RollMonsterMove")

    print(f"Loaded {len(supported_actions)} Rust Actions and {len(supported_powers)} Rust Powers.")
    
    report_lines = [
        f"# {layer.capitalize()} Batch Parity Report",
        f"**Engine Capabilities**: {len(supported_actions)} Supported Actions, {len(supported_powers)} Supported Powers",
        "---"
    ]

    java_files = list(java_root.rglob("*.java"))
    
    for j_path in java_files:
        stats = analyze_java_file(j_path)
        if not stats: continue
        
        cname = stats["class_name"]
        
        # 1) Map Java Class to Rust Enum Variant using schema
        rust_variant = JAVA_TO_RUST_MONSTER.get(cname, cname)
        
        # 2) Determine Rust file name
        snake_name = pascal_to_snake(rust_variant)
        
        # Fallback hacks for grouped files
        if "slime" in snake_name.lower():
            rpath = rust_monster_dir / f"{snake_name}.rs"
            if not rpath.exists():
                rpath = rust_monster_dir / "acid_slime.rs" # fallback for S/M
                if "spike" in snake_name: rpath = rust_monster_dir / "spike_slime.rs"
        else:
            rpath = rust_monster_dir / f"{snake_name}.rs"

        if not rpath.exists():
            report_lines.append(f"\n### :x: `{cname}`")
            report_lines.append(f"- **Error**: Expected Rust file `{rpath.name}` not found!")
            continue

        report_lines.append(f"\n### `{cname}` -> `{rpath.name}`")
        rust_src = rpath.read_text(encoding='utf-8')
        
        missing_backend = []
        missing_in_rust_file = []
        
        # Check Actions
        for j_act in stats["actions_needed"]:
            r_act = JAVA_TO_RUST_ACTION.get(j_act, j_act) # generic fallback
            if r_act is None: continue # Ignored visual action
            
            if r_act not in supported_actions and r_act != j_act: # If it's a known mapped action but missing in enum
                 missing_backend.append(f"Action: {r_act} (from {j_act})")
            elif r_act == j_act and "Action" in j_act:
                 missing_backend.append(f"Action: Unknown Mapping for {j_act}")
            
            # Check if rust file calls it
            if r_act and r_act in supported_actions:
                 if f"Action::{r_act}" not in rust_src and f"{r_act}" not in rust_src:
                     # False positive risk for generic ones (like Attack vs Damage) but good indicator
                     if r_act not in ["SetMonsterMove", "RollMonsterMove"]: # We handle these differently manually
                        missing_in_rust_file.append(f"Action::{r_act}")

        # Check Powers
        for j_pow in stats["powers_needed"]:
            r_pow = JAVA_TO_RUST_POWER.get(j_pow, j_pow)
            if r_pow is None: continue
            
            if r_pow not in supported_powers:
                missing_backend.append(f"Power: {r_pow} (from {j_pow})")
            
            if r_pow in supported_powers:
                if f"PowerId::{r_pow}" not in rust_src:
                    missing_in_rust_file.append(f"PowerId::{r_pow}")
                    
        if not missing_backend and not missing_in_rust_file:
            report_lines.append("- :white_check_mark: **fully supported and implemented**.")
        else:
            if missing_backend:
                report_lines.append("- **MISSING ENGINE BACKEND**:")
                for m in missing_backend: report_lines.append(f"  - [ ] `{m}`")
            if missing_in_rust_file:
                report_lines.append("- **POTENTIALLY MISSING IN RUST SOURCE** (Needs manual check):")
                for m in missing_in_rust_file: report_lines.append(f"  - [ ] `{m}`")

    outpath = Path(args.out)
    outpath.write_text("\n".join(report_lines), encoding='utf-8')
    print(f"Wrote report to {outpath}")

if __name__ == "__main__":
    main()
