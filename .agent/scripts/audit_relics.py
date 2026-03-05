#!/usr/bin/env python3
"""
Relic Implementation Audit Script for sts_sim.

Scans Java source files and cross-references with Rust implementation
to produce a comprehensive status report of all relics.

Usage:
    python .agent/scripts/audit_relics.py [--json] [--missing-only] [--combat-only]

Output: Markdown table showing implementation status of every relic.
"""

import os
import re
import io
import sys
import json
import argparse
from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional

# === Paths ===
JAVA_RELICS_DIR = Path(r"c:\Dev\rust\cardcrawl\relics")
RUST_SRC_DIR = Path(r"c:\Dev\rust\sts_sim\src")
RELICS_JSON = Path(r"c:\Dev\rust\sts_sim\data\relics_patched.json")

# === Java Hooks that indicate combat relevance ===
COMBAT_HOOKS = {
    'atBattleStart', 'atPreBattle', 'atTurnStart', 'atTurnStartPostDraw',
    'onPlayerEndTurn', 'onUseCard', 'onPlayCard', 'wasHPLost',
    'onAttack', 'onAttacked', 'onLoseHp', 'onMonsterDeath',
    'onObtainCard', 'onShuffle', 'onExhaust', 'onManualDiscard',
    'onUsePotion', 'onVictory', 'onBloodied', 'onNotBloodied',
    'onEquip', 'onUnequip', 'onTrigger', 'onPlayerGainedBlock',
    'onAttackToChangeDamage', 'onRefreshHand', 'onSpawnMonster',
}

# Java hooks that are purely out-of-combat / meta
NON_COMBAT_HOOKS = {
    'justEnteredRoom', 'onEnterRoom', 'canSpawn', 'onChestOpen',
    'addCampfireOption', 'canUseCampfireOption', 'onMasterDeckChange',
}

# Developer debug relics — not obtainable in normal gameplay, excluded from audit
SKIP_RELICS = {'Test 1', 'Test 2', 'Test 3', 'Test 4', 'Test 5', 'Test 6',
               'Test1', 'Test2', 'Test3', 'Test4', 'Test5', 'Test6'}


@dataclass
class JavaRelicInfo:
    """Information extracted from a Java relic source file."""
    class_name: str
    relic_id: str  # The ID string from Java (may have spaces)
    tier: str = "Unknown"
    hooks: list = field(default_factory=list)
    is_combat_relevant: bool = False
    character_specific: Optional[str] = None


@dataclass
class RustImplStatus:
    """Implementation status in Rust."""
    found_in_hardcoded: bool = False      # In trigger_hardcoded_relic_standalone
    found_in_preloop: bool = False        # In trigger_relics pre-loop
    found_inline: bool = False            # Inline in state.rs, commands.rs, etc.
    found_in_data_json: bool = False      # Has JSON hooks in relics_patched.json
    found_anywhere: bool = False          # Referenced anywhere in Rust code
    locations: list = field(default_factory=list)  # File:line references


def parse_java_relic(filepath: Path) -> Optional[JavaRelicInfo]:
    """Parse a Java relic source file to extract key info."""
    try:
        content = filepath.read_text(encoding='utf-8', errors='replace')
    except:
        return None

    class_name = filepath.stem
    if class_name == 'AbstractRelic':
        return None

    info = JavaRelicInfo(class_name=class_name, relic_id=class_name)

    # Extract ID
    id_match = re.search(r'public static final String ID\s*=\s*"([^"]+)"', content)
    if id_match:
        info.relic_id = id_match.group(1)

    # Extract tier
    tier_match = re.search(r'RelicTier\.(\w+)', content)
    if tier_match:
        info.tier = tier_match.group(1).capitalize()

    # Extract hooks (overridden methods)
    hook_matches = re.findall(r'@Override\s+public\s+\w+\s+(\w+)\s*\(', content)
    info.hooks = [h for h in hook_matches if h not in ('getUpdatedDescription', 'makeCopy', 'updateDescription')]

    # Determine combat relevance
    info.is_combat_relevant = any(h in COMBAT_HOOKS for h in info.hooks)

    return info


def scan_rust_for_relic(relic_id: str, rust_id_variants: list, rust_code_map: dict) -> RustImplStatus:
    """Check if a relic is referenced in Rust code."""
    status = RustImplStatus()

    for variant in rust_id_variants:
        for filepath, content in rust_code_map.items():
            if variant in content:
                status.found_anywhere = True

                rel_path = str(filepath)
                if 'relics.rs' in rel_path:
                    # Check which function section the reference is in
                    lines = content.split('\n')
                    # Build a section map: line_number -> section_name
                    current_section = 'other'
                    for i, line in enumerate(lines):
                        if 'fn trigger_relics(' in line:
                            current_section = 'preloop'
                        elif 'fn trigger_hardcoded_relic_standalone(' in line:
                            current_section = 'hardcoded'
                        elif 'fn on_relic_equip(' in line:
                            current_section = 'hardcoded'
                        elif 'fn trigger_relics_with_library(' in line:
                            current_section = 'other'
                        elif 'fn apply_relic_results(' in line:
                            current_section = 'other'
                        elif 'fn execute_command(' in line:
                            current_section = 'other'
                        elif 'fn check_condition' in line:
                            current_section = 'other'
                        if variant in line and '"' in line:
                            if current_section == 'hardcoded':
                                status.found_in_hardcoded = True
                                status.locations.append(f"relics.rs:L{i+1} (hardcoded)")
                            elif current_section == 'preloop':
                                status.found_in_preloop = True
                                status.locations.append(f"relics.rs:L{i+1} (pre-loop)")
                            else:
                                status.locations.append(f"relics.rs:L{i+1}")
                elif any(f in rel_path for f in ['state.rs', 'commands.rs', 'combat.rs', 'card_overrides.rs', 'potions_use.rs']):
                    status.found_inline = True
                    # Find line number
                    lines = content.split('\n')
                    for i, line in enumerate(lines):
                        if variant in line:
                            short_path = Path(rel_path).name
                            status.locations.append(f"{short_path}:L{i+1} (inline)")
                            break

    return status


def check_json_data(relic_id: str, json_data: list) -> bool:
    """Check if relic has hooks defined in JSON data."""
    for r in json_data:
        if r.get('id') == relic_id:
            hooks = r.get('logic', {}).get('hooks', [])
            return len(hooks) > 0
    return False


def generate_rust_id_variants(java_id: str, class_name: str) -> list:
    """Generate possible Rust ID variants for a Java relic."""
    variants = set()
    variants.add(java_id)
    variants.add(class_name)

    # Common transformations
    # "Burning Blood" -> "BurningBlood"
    no_spaces = java_id.replace(" ", "")
    variants.add(no_spaces)

    # "Philosopher's Stone" -> "PhilosophersStone" or "PhilosopherStone"
    no_apos = java_id.replace("'s ", "s").replace("' ", "").replace("'", "")
    variants.add(no_apos)
    variants.add(no_apos.replace(" ", ""))

    # Du-Vu Doll special cases
    variants.add(java_id.replace(" ", "-"))
    variants.add(java_id.replace(" ", "_"))

    return list(variants)


def main():
    # Force UTF-8 output on Windows
    if sys.platform == 'win32':
        sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

    parser = argparse.ArgumentParser(description='Audit relic implementation status')
    parser.add_argument('--json', action='store_true', help='Output as JSON')
    parser.add_argument('--missing-only', action='store_true', help='Show only unimplemented relics')
    parser.add_argument('--combat-only', action='store_true', help='Show only combat-relevant relics')
    parser.add_argument('--tier', type=str, help='Filter by tier (Common, Uncommon, Rare, Boss, Shop, Starter, Event, Special)')
    args = parser.parse_args()

    # 1. Parse all Java relics
    java_relics = []
    for java_file in sorted(JAVA_RELICS_DIR.glob("*.java")):
        info = parse_java_relic(java_file)
        if info:
            java_relics.append(info)

    # 2. Load all Rust source code
    rust_code_map = {}
    for rs_file in RUST_SRC_DIR.rglob("*.rs"):
        try:
            rust_code_map[rs_file] = rs_file.read_text(encoding='utf-8', errors='replace')
        except:
            pass

    # 3. Load JSON data
    json_data = []
    if RELICS_JSON.exists():
        try:
            json_data = json.loads(RELICS_JSON.read_text(encoding='utf-8'))
        except:
            pass

    # 4. Cross-reference
    results = []
    for java_relic in java_relics:
        variants = generate_rust_id_variants(java_relic.relic_id, java_relic.class_name)
        rust_status = scan_rust_for_relic(java_relic.relic_id, variants, rust_code_map)
        rust_status.found_in_data_json = check_json_data(java_relic.relic_id, json_data)

        # Skip developer test relics
        if java_relic.relic_id in SKIP_RELICS or java_relic.class_name in SKIP_RELICS:
            continue

        # Apply filters
        if args.combat_only and not java_relic.is_combat_relevant:
            continue
        if args.tier and java_relic.tier.lower() != args.tier.lower():
            continue

        is_implemented = (rust_status.found_in_hardcoded or
                          rust_status.found_in_preloop or
                          rust_status.found_inline or
                          rust_status.found_in_data_json)

        if args.missing_only and is_implemented:
            continue

        results.append((java_relic, rust_status, is_implemented))

    # 5. Output
    if args.json:
        output = []
        for java_relic, rust_status, is_impl in results:
            output.append({
                'id': java_relic.relic_id,
                'class': java_relic.class_name,
                'tier': java_relic.tier,
                'hooks': java_relic.hooks,
                'combat_relevant': java_relic.is_combat_relevant,
                'implemented': is_impl,
                'impl_type': (
                    'hardcoded' if rust_status.found_in_hardcoded else
                    'pre-loop' if rust_status.found_in_preloop else
                    'inline' if rust_status.found_inline else
                    'json-data' if rust_status.found_in_data_json else
                    'referenced' if rust_status.found_anywhere else
                    'missing'
                ),
                'locations': rust_status.locations[:3],
            })
        print(json.dumps(output, indent=2))
        return

    # Markdown output
    # Count stats
    total = len(results)
    implemented = sum(1 for _, _, i in results if i)
    missing = total - implemented

    print(f"# Relic Implementation Audit")
    print(f"")
    print(f"**Total**: {total} | **Implemented**: {implemented} | **Missing**: {missing}")
    print(f"")

    # Group by tier
    tiers = {}
    for java_relic, rust_status, is_impl in results:
        tier = java_relic.tier
        if tier not in tiers:
            tiers[tier] = []
        tiers[tier].append((java_relic, rust_status, is_impl))

    tier_order = ['Starter', 'Common', 'Uncommon', 'Rare', 'Boss', 'Shop', 'Event', 'Special', 'Unknown']

    for tier in tier_order:
        if tier not in tiers:
            continue
        items = tiers[tier]
        tier_impl = sum(1 for _, _, i in items if i)
        print(f"\n## {tier} ({tier_impl}/{len(items)})")
        print(f"")
        print(f"| Relic | Java ID | Hooks | Status | Location |")
        print(f"|-------|---------|-------|--------|----------|")

        for java_relic, rust_status, is_impl in sorted(items, key=lambda x: (x[2], x[0].class_name)):
            hooks_str = ', '.join(java_relic.hooks[:3])
            if len(java_relic.hooks) > 3:
                hooks_str += f' +{len(java_relic.hooks)-3}'

            if rust_status.found_in_hardcoded:
                status = "✅ hardcoded"
            elif rust_status.found_in_preloop:
                status = "✅ pre-loop"
            elif rust_status.found_inline:
                status = "✅ inline"
            elif rust_status.found_in_data_json:
                status = "✅ json-data"
            elif rust_status.found_anywhere:
                status = "⚠️ ref-only"
            else:
                status = "❌ missing"

            loc = rust_status.locations[0] if rust_status.locations else "-"
            if len(loc) > 35:
                loc = loc[:32] + "..."

            print(f"| {java_relic.class_name} | {java_relic.relic_id} | {hooks_str} | {status} | {loc} |")

    # Summary: Missing combat relics
    missing_combat = [(j, r, i) for j, r, i in results if not i and j.is_combat_relevant]
    if missing_combat:
        print(f"\n## Missing Combat Relics ({len(missing_combat)})")
        print(f"")
        print(f"| Relic | Tier | Java Hooks | Difficulty |")
        print(f"|-------|------|------------|------------|")
        for java_relic, _, _ in sorted(missing_combat, key=lambda x: x[0].tier):
            hooks_str = ', '.join(java_relic.hooks[:4])
            # Estimate difficulty
            simple_hooks = {'atBattleStart', 'atTurnStart', 'onVictory', 'onEquip'}
            if all(h in simple_hooks or h in NON_COMBAT_HOOKS for h in java_relic.hooks):
                diff = "🟢 Easy"
            elif any(h in {'onUseCard', 'wasHPLost', 'onPlayerEndTurn', 'onBloodied'} for h in java_relic.hooks):
                diff = "🟡 Medium"
            else:
                diff = "🔴 Hard"
            print(f"| {java_relic.class_name} | {java_relic.tier} | {hooks_str} | {diff} |")


if __name__ == '__main__':
    main()
