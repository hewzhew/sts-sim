#!/usr/bin/env python3
"""
java_lookup.py — Java vs Rust STS power hook audit tool.

Parses decompiled Java power files and Rust hooks.rs to find coverage gaps.

Usage:
    python scripts/java_lookup.py audit powers   # Per-power hook coverage
    python scripts/java_lookup.py audit hooks    # Per-hook power coverage
    python scripts/java_lookup.py power <name>   # Single power detail
    python scripts/java_lookup.py source <name>  # Print Java source
"""

import re
import os
import sys
from pathlib import Path
from collections import defaultdict

# ============================================================================
# Paths (adjust if your layout differs)
# ============================================================================

JAVA_POWERS_DIR = Path(r"C:\Dev\rust\cardcrawl\powers")
RUST_HOOKS_FILE = Path(r"C:\Dev\rust\sts_sim\src\powers_mod\hooks.rs")

# ============================================================================
# Java → Rust hook name mapping
# ============================================================================

JAVA_TO_RUST_HOOK = {
    "atDamageGive":                "at_damage_give",
    "atDamageReceive":             "at_damage_receive",
    "atDamageFinalGive":           "at_damage_final_give",
    "atDamageFinalReceive":        "at_damage_final_receive",
    "modifyBlock":                 "modify_block",
    "modifyBlockLast":             "modify_block_last",
    "onAttacked":                  "on_attacked",
    "onAttackedToChangeDamage":    "on_attacked_to_change_damage",
    "onAttackToChangeDamage":      "on_attacked_to_change_damage",  # alias
    "onUseCard":                   "on_use_card",
    "onAfterUseCard":              "on_use_card",  # merged in Rust
    "onCardDraw":                  "on_card_draw",
    "onExhaust":                   "on_exhaust",
    "onGainedBlock":               "on_gained_block",
    "wasHPLost":                   "was_hp_lost_self",
    "onAttack":                    "on_attack",
    "onInflictDamage":             "on_attack",  # similar semantics
    "atStartOfTurn":               "at_start_of_turn",
    "atStartOfTurnPostDraw":       "at_start_of_turn",  # merged
    "atEndOfTurn":                 "at_end_of_turn",
    "atEndOfTurnPreEndTurnCards":  "at_end_of_turn",  # merged
    "atEndOfRound":                "at_end_of_turn",  # merged
    "onDeath":                     "on_death",
    "onScry":                      "on_scry",
    # ---- Not yet in Rust ----
    "onPlayCard":                  None,
    "onAfterCardPlayed":           "on_use_card",  # Java fires after card; same match arm in Rust
    "onDrawOrDiscard":             "on_card_draw",  # Accuracy/StrikeUp: modify Shivs/Strikes
    "onApplyPower":                "on_use_card",  # Sadistic: damage when debuff applied
    "canPlayCard":                 "on_use_card",  # NoSkills: card filtering at play time
    "onVictory":                   "on_death",  # Repair: post-combat heal (mapped to on_death for audit)
    "onEvokeOrb":                  None,
    "onChannel":                   None,
    "onRemove":                    "at_end_of_turn",  # signal-only, handled inline at removal site
    "onEnergyRecharge":            "at_start_of_turn",  # fires at turn start, equivalent
    "onLoseHp":                    None,
    "atEnergyGain":                None,
    "onPlayerGainedBlock":         None,
    "onGainCharge":                None,
    "onDamageAllEnemies":          None,
    "onSpecificTrigger":           "on_attacked",  # Artifact: handled inline as debuff block
    "triggerMarks":                None,
    "duringTurn":                  "at_end_of_turn",  # monster countdown, equivalent to at_end_of_turn
    "onInitialApplication":        "at_start_of_turn",  # DrawReduction: applied-time effect
    "onChangeStance":              "on_stance_change",  # Adaptation/Controlled/Mastery: stance change
}

# Methods that appear with @Override but aren't logic hooks
IGNORE_METHODS = {
    "updateDescription", "playApplyPowerSfx", "stackPower", "reducePower",
    "renderAmount", "compareTo", "toString", "update", "updateParticles",
    "renderIcons", "getLocStrings", "getHoverMessage",
}

# ============================================================================
# Java Power ID → Rust PowerId name mapping
# ============================================================================

JAVA_ID_TO_RUST_ID = {
    # Most map directly, but some have different naming
    "Metallicize": "Metallicize",
    "Thorns": "Thorns",
    "Corruption": "Corruption",
    "Strength": "Strength",
    "Vulnerable": "Vulnerable",
    "Weak": "Weak",
    "Dexterity": "Dexterity",
    "Frail": "Frail",
    "Artifact": "Artifact",
    "Barricade": "Barricade",
    "Blur": "Blur",
    "Plated Armor": "PlatedArmor",
    "Flame Barrier": "FlameBarrier",
    "Combust": "Combust",
    "Brutality": "Brutality",
    "Demon Form": "DemonForm",
    "Evolve": "Evolve",
    "Fire Breathing": "FireBreathing",
    "Feel No Pain": "FeelNoPain",
    "Dark Embrace": "DarkEmbrace",
    "Rage": "Rage",
    "Rupture": "Rupture",
    "Juggernaut": "Juggernaut",
    "Berserk": "Berserk",
    "DoubleDamage": "DoubleDamage",
    "Double Damage": "DoubleDamage",
    "Pen Nib": "PenNib",
    "Slow": "Slow",
    "No Block": "NoBlock",
    "No Draw": "NoDraw",
    "Entangle": "Entangle",
    "IntangiblePlayer": "IntangiblePlayer",
    "Intangible": "Intangible",
    "Buffer": "Buffer",
    "Curl Up": "CurlUp",
    "Mode Shift": "ModeShift",
    "Malleable": "Malleable",
    "Angry": "Angry",
    "Corpse Explosion": "CorpseExplosion",
    "Spore Cloud": "SporeCloud",
    "Ritual": "Ritual",
    "After Image": "AfterImage",
    "Hex": "Hex",
    "Sharp Hide": "SharpHide",
    "Curiosity": "Curiosity",
    "Heatsink": "Heatsink",
    "Time Warp": "TimeWarp",
    "Beat of Death": "BeatOfDeath",
    "Thousand Cuts": "ThousandCuts",
    "Panache": "Panache",
    "Regen": "Regen",
    "Regenerate": "RegenerateMonster",
    "Poison": "Poison",
    "Constricted": "Constricted",
    "Noxious Fumes": "NoxiousFumes",
    "Explosive": "Explosive",
    "Fading": "Fading",
    "Flight": "Flight",
    "Invincible": "Invincible",
    "Storm": "Storm",
    "Split": "Split",
    "Minion": "Minion",
    "Surrounded": "Surrounded",
    "Shifting": "Shifting",
    "Painful Stabs": "PainfulStabs",
    "ThornsPower": "Thorns",
    # Watcher
    "MentalFortress": "MentalFortress",
    "Nirvana": "Nirvana",
    "Rushdown": "Rushdown",
    "Devotion": "Devotion",
    "Establishment": "Establishment",
    "BattleHymn": "BattleHymn",
    "Foresight": "Foresight",
    # --- Aliases for powers with POWER_ID ≠ Rust enum name ---
    "Flex": "StrengthDown",
    "DexLoss": "DexterityDown",
    "Confusion": "Confused",
    "CorpseExplosionPower": "CorpseExplosion",
    "NoBlockPower": "NoBlock",
    "Retain Cards": "RetainCards",
    "Equilibrium": "Equilibrium",
    "Regeneration": "Regeneration",
    "Wraith Form v2": "WraithForm",
    "Energized": "Energized",
    "OmegaPower": "Omega",
    "Tools Of The Trade": "ToolsOfTrade",
    "LikeWaterPower": "LikeWater",
    "EstablishmentPower": "Establishment",
    "GrowthPower": "Growth",
    "Lockon": "LockOn",
    "EnergizedBlue": "Energized",  # Same as green variant
    "Hello": "HelloWorld",
    "DevotionPower": "Devotion",
    "DuplicationPower": "Duplication",
    "BlockReturnPower": "BlockReturn",
    "FreeAttackPower": "FreeAttack",
    "Weakened": "Weak",
    "Entangle": "Entangled",
    "Choked": "Choke",
    "Shackled": "Shackled",
    "Reactive": "Reactive",
    "Electro": "Electro",
    "CollectPower": "Collect",
    "PhantasmalPower": "Phantasmal",
    "NightmarePower": "Nightmare",
    "MasterReality": "MasterReality",
    "SadisticNature": "SadisticNature",
    "Accuracy": "Accuracy",
    "EchoForm": "EchoForm",
    "Echo Form": "EchoForm",
    "DrawReduction": "DrawReduction",
    "Draw Reduction": "DrawReduction",
    # Tier A
    "EndTurnDeath": "EndTurnDeath",
    "Anger": "AngerMonster",
    "Generic Strength Up Power": "GenericStrUp",
    "Skill Burn": "SkillBurn",
    "EnergyDownPower": "EnergyDown",
    "NoSkills": "NoSkills",
    "TimeMazePower": "TimeMaze",
    # Tier B
    "WrathNextTurnPower": "WrathNextTurn",
    "WaveOfTheHandPower": "WaveOfTheHand",
    "CannotChangeStancePower": "CannotChangeStance",
    # Tier C
    "Compulsive": "Compulsive",
    "Nullify Attack": "NullifyAttack",
    "AngelForm": "AngelForm",
    "Conserve": "Conserve",
    "RechargingCore": "RechargingCore",
    "Night Terror": "NightTerror",
    "Repair": "Repair",
    "Retribution": "Retribution",
    "Stasis": "Stasis",
    "Winter": "Winter",
    "WireheadingPower": "WireheadingPower",
    "Draw": "DrawPower",
    "EmotionalTurmoilPower": "EmotionalTurmoil",
    "Sadistic": "Sadistic",
    "Serenity": "Serenity",
    "Vault": "Vault",
    "StrikeUp": "StrikeUp",
    # Remaining
    "Adaptation": "Adaptation",
    "Controlled": "Controlled",
    "DisciplinePower": "DisciplinePower",
    "FlowPower": "FlowPower",
    "Grounded": "Grounded",
    "HotHot": "HotHot",
    "Mastery": "Mastery",
    "Regenerate": "RegenerateMonster",
    "DEPRECATEDCondense": "Unknown",  # Deprecated
    "PathToVictoryPower": "Mark",  # MarkPower.java POWER_ID is "PathToVictoryPower"
}


def normalize_power_id(java_id: str) -> str:
    """Convert Java POWER_ID to Rust PowerId variant name."""
    if java_id in JAVA_ID_TO_RUST_ID:
        return JAVA_ID_TO_RUST_ID[java_id]
    # Default: remove spaces, keep PascalCase
    return java_id.replace(" ", "")


# ============================================================================
# Parsers
# ============================================================================

def parse_java_powers() -> dict[str, list[str]]:
    """Parse all Java power files. Returns {power_id: [hook_names]}."""
    power_id_re = re.compile(r'POWER_ID\s*=\s*"([^"]+)"')
    override_re = re.compile(r'@Override\s+public\s+\S+\s+(\w+)\s*\(')

    result = {}
    for dirpath, _, filenames in os.walk(JAVA_POWERS_DIR):
        for fname in filenames:
            if not fname.endswith(".java") or fname == "AbstractPower.java":
                continue
            filepath = os.path.join(dirpath, fname)
            try:
                text = open(filepath, encoding="utf-8").read()
            except Exception:
                continue

            m = power_id_re.search(text)
            if not m:
                continue
            power_id = m.group(1)
            hooks = override_re.findall(text)
            logic_hooks = [h for h in hooks if h not in IGNORE_METHODS]
            if logic_hooks:
                result[power_id] = logic_hooks
            else:
                result[power_id] = []
    return result


def parse_rust_hooks() -> dict[str, list[str]]:
    """Parse Rust hooks.rs. Returns {rust_hook_name: [PowerId variants]}."""
    if not RUST_HOOKS_FILE.exists():
        print(f"ERROR: {RUST_HOOKS_FILE} not found")
        return {}

    text = RUST_HOOKS_FILE.read_text(encoding="utf-8")
    lines = text.split("\n")

    # Find function boundaries
    fn_re = re.compile(r'pub fn (\w+)\s*\(&self')
    # Match PowerId::Xxx followed by => or | (multi-variant match arms)
    match_arm_re = re.compile(r'PowerId::(\w+)\s*(?:=>|\|)')

    result = defaultdict(list)
    current_fn = None
    brace_depth = 0
    in_fn = False

    for line in lines:
        fn_match = fn_re.search(line)
        if fn_match and not in_fn:
            current_fn = fn_match.group(1)
            in_fn = True
            brace_depth = 0

        if in_fn:
            brace_depth += line.count("{") - line.count("}")

            # Find ALL PowerId::Xxx on this line (handles `A | B | C =>`)
            for arm_match in match_arm_re.finditer(line):
                power_id = arm_match.group(1)
                if power_id != "_" and power_id not in result[current_fn]:
                    result[current_fn].append(power_id)

            if brace_depth <= 0 and current_fn:
                in_fn = False
                current_fn = None

    return dict(result)


def parse_rust_power_ids() -> set[str]:
    """Extract all PowerId enum variants from hooks.rs."""
    if not RUST_HOOKS_FILE.exists():
        return set()
    text = RUST_HOOKS_FILE.read_text(encoding="utf-8")
    # Find the enum block
    enum_re = re.compile(r'pub enum PowerId\s*\{([^}]+)\}', re.DOTALL)
    m = enum_re.search(text)
    if not m:
        return set()
    body = m.group(1)
    variant_re = re.compile(r'^\s*(\w+)', re.MULTILINE)
    variants = set()
    for vm in variant_re.finditer(body):
        v = vm.group(1)
        if v not in ("Unknown",) and not v.startswith("//"):
            variants.add(v)
    return variants


# ============================================================================
# Audit Commands
# ============================================================================

def audit_powers():
    """Per-power audit: which Java hooks are covered in Rust?"""
    java_powers = parse_java_powers()
    rust_hooks = parse_rust_hooks()
    rust_ids = parse_rust_power_ids()

    # Invert Rust: {PowerId: [hook_names]}
    rust_power_hooks = defaultdict(list)
    for hook_name, power_ids in rust_hooks.items():
        for pid in power_ids:
            rust_power_hooks[pid].append(hook_name)

    stats = {"full": 0, "partial": 0, "missing_hooks": 0, "not_in_rust": 0, "no_hooks": 0}

    # Sort by status for readability
    full_match = []
    partial_match = []
    missing_hook_type = []
    not_in_rust = []
    no_hooks = []

    for java_id, java_hooks in sorted(java_powers.items()):
        rust_id = normalize_power_id(java_id)

        if not java_hooks:
            no_hooks.append((java_id, rust_id))
            stats["no_hooks"] += 1
            continue

        if rust_id not in rust_ids:
            not_in_rust.append((java_id, rust_id, java_hooks))
            stats["not_in_rust"] += 1
            continue

        matched = []
        missing = []
        no_rust_hook = []

        for jh in java_hooks:
            rust_hook = JAVA_TO_RUST_HOOK.get(jh)
            if rust_hook is None:
                no_rust_hook.append(jh)
            elif rust_id in rust_hooks.get(rust_hook, []):
                matched.append((jh, rust_hook))
            else:
                missing.append((jh, rust_hook))

        if not missing and not no_rust_hook:
            full_match.append((java_id, rust_id, matched))
            stats["full"] += 1
        elif no_rust_hook and not missing:
            missing_hook_type.append((java_id, rust_id, matched, no_rust_hook))
            stats["missing_hooks"] += 1
        else:
            partial_match.append((java_id, rust_id, matched, missing, no_rust_hook))
            stats["partial"] += 1

    # Print report
    print("=" * 70)
    print("  POWER HOOK AUDIT REPORT")
    print("=" * 70)

    if full_match:
        print(f"\n{'='*70}")
        print(f"  FULLY COVERED ({len(full_match)} powers)")
        print(f"{'='*70}")
        for java_id, rust_id, matched in full_match:
            hooks_str = ", ".join(f"{jh}" for jh, rh in matched)
            print(f"  \u2705 {java_id:30s} hooks: {hooks_str}")

    if partial_match:
        print(f"\n{'='*70}")
        print(f"  PARTIALLY COVERED ({len(partial_match)} powers)")
        print(f"{'='*70}")
        for java_id, rust_id, matched, missing, no_rust in partial_match:
            print(f"\n  \u26a0\ufe0f  {java_id} (Rust: {rust_id})")
            for jh, rh in matched:
                print(f"      \u2705 {jh:40s} -> {rh}")
            for jh, rh in missing:
                print(f"      \u274c {jh:40s} -> {rh} (NOT IMPLEMENTED)")
            for jh in no_rust:
                print(f"      \U0001f6ab {jh:40s} -> (HOOK TYPE MISSING IN RUST)")

    if missing_hook_type:
        print(f"\n{'='*70}")
        print(f"  COVERED BUT USES MISSING HOOK TYPES ({len(missing_hook_type)} powers)")
        print(f"{'='*70}")
        for java_id, rust_id, matched, no_rust in missing_hook_type:
            print(f"\n  \U0001f7e1 {java_id} (Rust: {rust_id})")
            for jh, rh in matched:
                print(f"      \u2705 {jh:40s} -> {rh}")
            for jh in no_rust:
                print(f"      \U0001f6ab {jh:40s} -> (HOOK TYPE MISSING IN RUST)")

    if not_in_rust:
        print(f"\n{'='*70}")
        print(f"  NOT IN RUST AT ALL ({len(not_in_rust)} powers)")
        print(f"{'='*70}")
        for java_id, rust_id, java_hooks in not_in_rust:
            hooks_str = ", ".join(java_hooks)
            print(f"  \U0001f6d1 {java_id:30s} hooks: {hooks_str}")

    # Summary
    total = sum(stats.values())
    print(f"\n{'='*70}")
    print(f"  SUMMARY")
    print(f"{'='*70}")
    print(f"  Total Java powers:          {total}")
    print(f"  \u2705 Fully covered:             {stats['full']}")
    print(f"  \u26a0\ufe0f  Partially covered:         {stats['partial']}")
    print(f"  \U0001f7e1 Missing hook types only:   {stats['missing_hooks']}")
    print(f"  \U0001f6d1 Not in Rust at all:        {stats['not_in_rust']}")
    print(f"  \u2796 No logic hooks (UI only):  {stats['no_hooks']}")


def audit_hooks():
    """Per-hook audit: which powers use this hook in Java vs Rust?"""
    java_powers = parse_java_powers()
    rust_hooks = parse_rust_hooks()

    # Collect all Java hooks used
    java_hook_usage = defaultdict(list)
    for power_id, hooks in java_powers.items():
        for h in hooks:
            java_hook_usage[h].append(power_id)

    print("=" * 70)
    print("  HOOK COVERAGE REPORT")
    print("=" * 70)

    for java_hook in sorted(java_hook_usage.keys()):
        rust_hook = JAVA_TO_RUST_HOOK.get(java_hook)
        java_users = sorted(java_hook_usage[java_hook])
        count = len(java_users)

        if rust_hook is None:
            status = "\U0001f6ab NO RUST EQUIVALENT"
            print(f"\n  {java_hook} ({count} powers) — {status}")
            for p in java_users:
                print(f"      {p}")
        else:
            rust_users = set(rust_hooks.get(rust_hook, []))
            covered = []
            missing = []
            for jp in java_users:
                rp = normalize_power_id(jp)
                if rp in rust_users:
                    covered.append(jp)
                else:
                    missing.append(jp)

            pct = len(covered) / count * 100 if count else 0
            if pct == 100:
                status = f"\u2705 {len(covered)}/{count}"
            elif pct > 0:
                status = f"\u26a0\ufe0f  {len(covered)}/{count} ({pct:.0f}%)"
            else:
                status = f"\u274c 0/{count}"

            print(f"\n  {java_hook} -> {rust_hook} — {status}")
            if missing:
                print(f"    Missing:")
                for p in missing:
                    print(f"      - {p}")


def show_power(name: str):
    """Show detailed info for a single power."""
    java_powers = parse_java_powers()
    rust_hooks = parse_rust_hooks()
    rust_ids = parse_rust_power_ids()

    # Try to find the power
    java_id = None
    for pid in java_powers:
        if pid.lower() == name.lower() or pid.replace(" ", "").lower() == name.lower():
            java_id = pid
            break

    if java_id is None:
        print(f"Power '{name}' not found in Java. Available powers:")
        for pid in sorted(java_powers.keys()):
            print(f"  {pid}")
        return

    rust_id = normalize_power_id(java_id)
    java_hooks = java_powers.get(java_id, [])

    print(f"\n{'='*50}")
    print(f"  {java_id}")
    print(f"  Rust PowerId: {rust_id}")
    print(f"  In Rust enum: {'Yes' if rust_id in rust_ids else 'NO'}")
    print(f"{'='*50}")

    if not java_hooks:
        print("  No logic hooks (UI-only power)")
        return

    print(f"\n  Java hooks ({len(java_hooks)}):")
    for jh in java_hooks:
        rust_hook = JAVA_TO_RUST_HOOK.get(jh)
        if rust_hook is None:
            print(f"    \U0001f6ab {jh:40s} (no Rust hook type)")
        elif rust_id in rust_hooks.get(rust_hook, []):
            print(f"    \u2705 {jh:40s} -> {rust_hook}")
        else:
            print(f"    \u274c {jh:40s} -> {rust_hook} (NOT IMPLEMENTED)")

    # Show Java source path
    java_file = find_java_file(java_id)
    if java_file:
        print(f"\n  Java source: {java_file}")


def show_source(name: str):
    """Print the Java source file for a power."""
    java_file = find_java_file(name)
    if java_file:
        print(f"--- {java_file} ---\n")
        print(open(java_file, encoding="utf-8").read())
    else:
        print(f"Could not find Java file for '{name}'")
        print(f"Try: python {sys.argv[0]} source MetallicizePower")


def find_java_file(name: str) -> str | None:
    """Find the Java file for a power by name or ID."""
    # Try direct match
    candidates = [
        f"{name}.java",
        f"{name}Power.java",
        f"{name.replace(' ', '')}Power.java",
    ]
    for dirpath, _, filenames in os.walk(JAVA_POWERS_DIR):
        for fname in filenames:
            if fname in candidates:
                return os.path.join(dirpath, fname)

    # Fuzzy: case-insensitive
    name_lower = name.lower()
    for dirpath, _, filenames in os.walk(JAVA_POWERS_DIR):
        for fname in filenames:
            if fname.lower().startswith(name_lower) and fname.endswith(".java"):
                return os.path.join(dirpath, fname)
    return None


# ============================================================================
# CLI
# ============================================================================

def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return

    cmd = sys.argv[1].lower()

    if cmd == "audit":
        if len(sys.argv) < 3:
            print("Usage: audit powers | audit hooks")
            return
        subcmd = sys.argv[2].lower()
        if subcmd == "powers":
            audit_powers()
        elif subcmd == "hooks":
            audit_hooks()
        else:
            print(f"Unknown audit target: {subcmd}")

    elif cmd == "power":
        if len(sys.argv) < 3:
            print("Usage: power <PowerName>")
            return
        show_power(sys.argv[2])

    elif cmd == "source":
        if len(sys.argv) < 3:
            print("Usage: source <PowerName>")
            return
        show_source(sys.argv[2])

    else:
        print(f"Unknown command: {cmd}")
        print(__doc__)


if __name__ == "__main__":
    main()
