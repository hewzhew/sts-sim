#!/usr/bin/env python3
"""Scan Java power files and extract which hooks each power overrides.

Outputs a coverage report showing:
1. All 162 powers with their hooks
2. Hook frequency (which hooks are used most)
3. Powers grouped by hook pattern
"""

import os
import re
from collections import defaultdict
from pathlib import Path

JAVA_POWERS_DIR = Path(r"C:\Dev\rust\cardcrawl\powers")
RUST_POWERS_RS = Path(r"C:\Dev\rust\sts_sim\src\powers.rs")

# All 35 hook methods from AbstractPower.java (excluding render/UI/utility)
HOOKS = [
    "atDamageGive",
    "atDamageFinalGive",
    "atDamageReceive",
    "atDamageFinalReceive",
    "modifyBlock",
    "modifyBlockLast",
    "atStartOfTurn",
    "atStartOfTurnPostDraw",
    "duringTurn",
    "atEndOfTurn",
    "atEndOfTurnPreEndTurnCards",
    "atEndOfRound",
    "onAttacked",
    "onAttackedToChangeDamage",
    "onAttack",
    "onAttackToChangeDamage",
    "onInflictDamage",
    "onHeal",
    "onLoseHp",
    "wasHPLost",
    "onPlayCard",
    "onUseCard",
    "onAfterUseCard",
    "onAfterCardPlayed",
    "onCardDraw",
    "onExhaust",
    "onDeath",
    "onApplyPower",
    "onGainedBlock",
    "onPlayerGainedBlock",
    "onDrawOrDiscard",
    "onEnergyRecharge",
    "onInitialApplication",
    "onRemove",
    "onSpecificTrigger",
    "onScry",
    "onChangeStance",
    "onEvokeOrb",
    "onChannel",
    "onGainCharge",
    "onDamageAllEnemies",
    "canPlayCard",
    "stackPower",
    "reducePower",
    "atEnergyGain",
    "onVictory",
    "triggerMarks",
]

def extract_power_info(filepath: Path) -> dict:
    """Extract power ID and overridden hooks from a Java power file."""
    content = filepath.read_text(encoding="utf-8", errors="replace")
    
    # Extract class name
    class_match = re.search(r"public class (\w+)\s+extends\s+AbstractPower", content)
    if not class_match:
        return None
    
    class_name = class_match.group(1)
    
    # Extract power ID from constructor
    id_match = re.search(r'this\.ID\s*=\s*"([^"]+)"', content)
    power_id = id_match.group(1) if id_match else class_name.replace("Power", "")
    
    # Extract power type
    type_match = re.search(r'this\.type\s*=\s*PowerType\.(\w+)', content)
    power_type = type_match.group(1) if type_match else "UNKNOWN"
    
    # Check which hooks are overridden (look for @Override + method name or just method signature)
    overridden = []
    for hook in HOOKS:
        # Match method definitions like "public float atDamageGive(" or "public void onAttacked("
        pattern = rf"public\s+\w+\s+{hook}\s*\("
        if re.search(pattern, content):
            overridden.append(hook)
    
    return {
        "class": class_name,
        "id": power_id,
        "type": power_type,
        "hooks": overridden,
        "file": filepath.name,
    }

def check_rust_coverage(rust_file: Path) -> set:
    """Check which power IDs are defined in the Rust power_ids module."""
    content = rust_file.read_text(encoding="utf-8")
    # Match const definitions like: pub const STRENGTH: &str = "Strength";
    matches = re.findall(r'pub const \w+:\s*&str\s*=\s*"([^"]+)"', content)
    return set(matches)

def main():
    if not JAVA_POWERS_DIR.exists():
        print(f"ERROR: Java powers directory not found: {JAVA_POWERS_DIR}")
        return
    
    # Scan all Java power files
    powers = []
    for f in sorted(JAVA_POWERS_DIR.glob("*.java")):
        if f.name == "AbstractPower.java":
            continue
        info = extract_power_info(f)
        if info:
            powers.append(info)
    
    # Get Rust coverage
    rust_ids = check_rust_coverage(RUST_POWERS_RS) if RUST_POWERS_RS.exists() else set()
    
    # === Report 1: Hook frequency ===
    print("=" * 70)
    print("HOOK FREQUENCY (how many powers use each hook)")
    print("=" * 70)
    hook_counts = defaultdict(int)
    hook_powers = defaultdict(list)
    for p in powers:
        for h in p["hooks"]:
            hook_counts[h] += 1
            hook_powers[h].append(p["id"])
    
    for hook, count in sorted(hook_counts.items(), key=lambda x: -x[1]):
        in_rust = "[Y]" if hook in [
            "atDamageGive", "atDamageReceive", "modifyBlock",
            "atDamageFinalReceive",
        ] else "[ ]"
        print(f"  {in_rust} {hook:35s} {count:3d} powers")
    
    # === Report 2: All powers with hooks ===
    print("\n" + "=" * 70)
    print("ALL POWERS AND THEIR HOOKS")
    print("=" * 70)
    
    buffs = [p for p in powers if p["type"] == "BUFF"]
    debuffs = [p for p in powers if p["type"] == "DEBUFF"]
    unknown = [p for p in powers if p["type"] == "UNKNOWN"]
    
    for label, group in [("BUFFS", buffs), ("DEBUFFS", debuffs), ("UNKNOWN TYPE", unknown)]:
        print(f"\n--- {label} ({len(group)}) ---")
        for p in sorted(group, key=lambda x: -len(x["hooks"])):
            in_rust = "[Y]" if p["id"] in rust_ids else "[N]"
            hooks_str = ", ".join(p["hooks"]) if p["hooks"] else "(no hooks)"
            print(f"  {in_rust} {p['id']:35s} [{len(p['hooks'])}] {hooks_str}")
    
    # === Report 3: Priority list (most hooks = most impact) ===
    print("\n" + "=" * 70)
    print("PRIORITY: Powers with most hooks (complex behavior)")
    print("=" * 70)
    for p in sorted(powers, key=lambda x: -len(x["hooks"]))[:30]:
        in_rust = "[Y]" if p["id"] in rust_ids else "[N]"
        print(f"  {in_rust} {p['id']:35s} {p['type']:7s} hooks: {', '.join(p['hooks'])}")
    
    # === Report 4: Combat-critical hooks ===
    print("\n" + "=" * 70)
    print("COMBAT-CRITICAL HOOKS (damage/block pipeline)")
    print("=" * 70)
    critical_hooks = [
        "atDamageGive", "atDamageFinalGive", 
        "atDamageReceive", "atDamageFinalReceive",
        "modifyBlock", "modifyBlockLast",
        "onAttacked", "onAttackedToChangeDamage",
    ]
    for hook in critical_hooks:
        powers_using = hook_powers.get(hook, [])
        print(f"\n  {hook} ({len(powers_using)} powers):")
        for pid in powers_using:
            in_rust = "[Y]" if pid in rust_ids else "[N]"
            print(f"    {in_rust} {pid}")
    
    # === Summary stats ===
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    total = len(powers)
    in_rust_count = sum(1 for p in powers if p["id"] in rust_ids)
    print(f"  Total Java powers:      {total}")
    print(f"  In Rust power_ids:      {in_rust_count} ({100*in_rust_count/total:.0f}%)")
    print(f"  Missing from Rust:      {total - in_rust_count}")
    print(f"  Total unique hooks:     {len(hook_counts)}")
    print(f"  Hooks with >5 powers:   {sum(1 for c in hook_counts.values() if c > 5)}")

if __name__ == "__main__":
    main()
