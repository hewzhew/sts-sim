#!/usr/bin/env python3
"""
Extract power specifications from Java source files.

Parses all Power*.java files from the decompiled STS source and generates
a structured powers_spec.json with exact hook signatures, constructor params,
field mappings, and stack behavior.

Usage:
    python extract_powers.py                    # Generate powers_spec.json
    python extract_powers.py --cards            # Also extract card→power mappings
    python extract_powers.py --power Berserk    # Extract single power
"""

import json
import os
import re
import sys
from pathlib import Path

# Paths
SCRIPT_DIR = Path(__file__).resolve().parent
DATA_DIR = SCRIPT_DIR.parent
PROJECT_ROOT = DATA_DIR.parent
JAVA_POWERS_DIR = Path(r"C:\Dev\rust\cardcrawl\powers")
JAVA_WATCHER_POWERS_DIR = JAVA_POWERS_DIR / "watcher"
JAVA_CARDS_DIR = Path(r"C:\Dev\rust\cardcrawl\cards")
OUTPUT_FILE = DATA_DIR / "powers_spec.json"

# All Java hook methods from AbstractPower
KNOWN_HOOKS = [
    "atStartOfTurn",
    "atStartOfTurnPostDraw",
    "atEndOfTurn",
    "atEndOfTurnPreEndTurnCards",
    "atEndOfRound",
    "atDamageGive",
    "atDamageReceive",
    "atDamageFinalGive",
    "atDamageFinalReceive",
    "modifyBlock",
    "modifyBlockLast",
    "onAttacked",
    "onAttackedToChangeDamage",
    "wasHPLost",
    "onGainedBlock",
    "onUseCard",
    "onAfterUseCard",
    "onPlayCard",
    "onCardDraw",
    "onExhaust",
    "onDeath",
    "onApplyPower",
    "onAttack",
    "onInflictDamage",
    "onRemove",
    "onInitialApplication",
    "onSpecificTrigger",
    "onChannel",
    "onEvokeOrb",
    "triggerMarks",
    "onHeal",
    "onVictory",
    "onChangeStance",
    "onPlayerGainedBlock",
    "stackPower",
    "reducePower",
]

# Action name extraction patterns
ACTION_PATTERN = re.compile(
    r'new\s+(\w+Action)\s*\((.*?)\)',
    re.DOTALL
)

# Power creation pattern inside hooks
POWER_IN_HOOK_PATTERN = re.compile(
    r'new\s+(\w+Power)\s*\((.*?)\)',
    re.DOTALL
)


def extract_power_id(content):
    """Extract POWER_ID constant."""
    m = re.search(r'POWER_ID\s*=\s*"([^"]+)"', content)
    return m.group(1) if m else None


def extract_class_name(content):
    """Extract class name."""
    m = re.search(r'public class (\w+)\s+extends', content)
    return m.group(1) if m else None


def extract_power_type(content):
    """Extract power type (BUFF/DEBUFF)."""
    if "PowerType.DEBUFF" in content:
        return "Debuff"
    return "Buff"


def extract_constructor(content, class_name):
    """Extract constructor params and field mappings."""
    # Match constructor
    pattern = re.compile(
        rf'public\s+{class_name}\s*\((.*?)\)\s*\{{(.*?)\n\s*\}}',
        re.DOTALL
    )
    m = pattern.search(content)
    if not m:
        return None

    params_str = m.group(1).strip()
    body = m.group(2)

    # Parse params
    params = []
    field_mapping = {}
    for param in params_str.split(","):
        param = param.strip()
        if not param:
            continue
        parts = param.split()
        if len(parts) >= 2:
            param_type = parts[-2]
            param_name = parts[-1]
            params.append({"name": param_name, "type": param_type})

    # Extract field assignments
    for line in body.split("\n"):
        line = line.strip()
        # this.amount = X
        m_field = re.match(r'this\.(\w+)\s*=\s*(.+?);', line)
        if m_field:
            field = m_field.group(1)
            value = m_field.group(2).strip()
            if field in ("amount", "hpLoss", "basePower", "magicNumber"):
                # Map field to constructor param
                for p in params:
                    if p["name"] in value:
                        field_mapping[field] = p["name"]
                        break
                else:
                    # Constant value
                    try:
                        field_mapping[field] = int(value)
                    except ValueError:
                        field_mapping[field] = value

    return {
        "params": [p["name"] for p in params],
        "param_types": {p["name"]: p["type"] for p in params},
        "field_mapping": field_mapping,
    }


def extract_hooks(content):
    """Extract all overridden hook methods and their action bodies."""
    hooks = []

    for hook_name in KNOWN_HOOKS:
        # Find @Override followed by the method
        # Match the method signature and body
        pattern = re.compile(
            rf'@Override\s+public\s+\w+\s+{hook_name}\s*\((.*?)\)\s*\{{(.*?)\n\s*\}}',
            re.DOTALL
        )
        m = pattern.search(content)
        if not m:
            continue

        params_str = m.group(1).strip()
        body = m.group(2)

        # Extract guard conditions
        guards = []
        guard_patterns = [
            (r'if\s*\(!?\s*AbstractDungeon\.getMonsters\(\)\.areMonstersBasicallyDead\(\)', "monsters_alive"),
            (r'if\s*\(card\.type\s*==\s*AbstractCard\.CardType\.(\w+)\)', "card_type_check"),
            (r'if\s*\(info\.owner\s*!=\s*null', "attacker_exists"),
            (r'if\s*\(info\.type\s*!=\s*DamageInfo\.DamageType\.HP_LOSS', "not_hp_loss"),
            (r'if\s*\(damageAmount\s*>\s*0\)', "damage_positive"),
        ]
        for guard_re, guard_name in guard_patterns:
            if re.search(guard_re, body):
                guards.append(guard_name)

        # Extract actions (addToBot calls)
        actions = []
        for action_match in re.finditer(r'this\.addToBot\s*\(\s*new\s+(\w+)\s*\((.*?)\)\s*\)', body, re.DOTALL):
            action_class = action_match.group(1)
            action_args_raw = action_match.group(2).strip()
            # Simplify args
            action_args = simplify_action_args(action_args_raw)
            actions.append({
                "action": action_class,
                "args": action_args,
                "raw": f"new {action_class}({action_args_raw})",
            })

        # Extract return value for modifier hooks
        return_value = None
        return_match = re.search(r'return\s+(.+?);', body)
        if return_match:
            return_value = return_match.group(1).strip()

        # Extract direct field modifications
        field_mods = []
        for mod_match in re.finditer(r'this\.(\w+)\s*([\+\-\*]=|=)\s*(.+?);', body):
            field = mod_match.group(1)
            op = mod_match.group(2)
            value = mod_match.group(3).strip()
            if field not in ("fontScale", "flash", "description"):
                field_mods.append({"field": field, "op": op, "value": value})

        # For modifier hooks, extract the formula
        hook_info = {
            "method": hook_name,
            "params": params_str if params_str else None,
        }
        if guards:
            hook_info["guards"] = guards
        if actions:
            hook_info["actions"] = [{"action": a["action"], "args": a["args"]} for a in actions]
        if return_value:
            hook_info["return_value"] = return_value
        if field_mods:
            hook_info["field_modifications"] = field_mods

        hooks.append(hook_info)

    return hooks


def simplify_action_args(raw_args):
    """Simplify Java action constructor args to readable form."""
    # Replace common patterns
    s = raw_args
    s = re.sub(r'this\.owner', 'owner', s)
    s = re.sub(r'this\.amount', 'amount', s)
    s = re.sub(r'this\.hpLoss', 'hpLoss', s)
    s = re.sub(r'AbstractDungeon\.player', 'player', s)
    s = re.sub(r'AbstractGameAction\.AttackEffect\.\w+', lambda m: m.group(0).split('.')[-1], s)
    s = re.sub(r'DamageInfo\.DamageType\.\w+', lambda m: m.group(0).split('.')[-1], s)
    s = re.sub(r'DamageInfo\.createDamageMatrix\((.*?),\s*true\)', r'damageMatrix(\1, ignoreThorns)', s)
    s = re.sub(r'\s+', ' ', s).strip()
    return s


def extract_stack_behavior(content):
    """Extract custom stackPower behavior."""
    pattern = re.compile(
        r'@Override\s+public\s+void\s+stackPower\s*\(int\s+stackAmount\)\s*\{(.*?)\n\s*\}',
        re.DOTALL
    )
    m = pattern.search(content)
    if not m:
        return {"method": "default", "code": "amount += stackAmount"}

    body = m.group(1)
    # Extract meaningful lines (skip fontScale)
    lines = []
    for line in body.strip().split("\n"):
        line = line.strip().rstrip(";")
        if not line or "fontScale" in line or "updateDescription" in line:
            continue
        lines.append(line + ";")

    return {
        "method": "custom" if len(lines) > 1 else "default",
        "code": " ".join(lines) if lines else "amount += stackAmount;",
    }


def extract_can_go_negative(content):
    """Check if power can go negative."""
    return "canGoNegative = true" in content


def extract_no_stack(content):
    """Check if power uses amount = -1 (no stack)."""
    return "this.amount = -1" in content


def find_card_usage(power_id, power_class_name):
    """Find which card(s) create this power and extract initial amounts."""
    usages = []
    for card_file in JAVA_CARDS_DIR.rglob("*.java"):
        try:
            with open(card_file, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception:
            continue

        if power_class_name not in content:
            continue

        # Find ApplyPowerAction calls with this power
        pattern = re.compile(
            rf'new\s+ApplyPowerAction\s*\(.*?new\s+{power_class_name}\s*\((.*?)\).*?\)',
            re.DOTALL
        )
        for m in pattern.finditer(content):
            args = m.group(1).strip()

            # Extract card class name
            card_class = re.search(r'public class (\w+)\s+extends', content)
            card_name = card_class.group(1) if card_class else card_file.stem

            # Extract base/upgrade magic numbers
            base_magic = None
            upgrade_magic = None

            base_match = re.search(r'this\.(?:magicNumber|baseMagicNumber|damage|baseDamage|block|baseBlock)\s*=\s*(?:this\.\w+\s*=\s*)?(\d+)', content)
            if base_match:
                base_magic = int(base_match.group(1))

            upgrade_match = re.search(r'this\.upgrade(?:MagicNumber|Damage|Block|BaseCost)\s*\((\-?\d+)\)', content)
            if upgrade_match:
                upgrade_magic = int(upgrade_match.group(1))

            # Determine color from path
            color = "unknown"
            for part in card_file.parts:
                if part in ("red", "green", "blue", "purple", "colorless", "tempCards", "curses", "status"):
                    color = part
                    break

            usages.append({
                "card": card_name,
                "color": color,
                "constructor_args": simplify_action_args(args),
                "base_value": base_magic,
                "upgrade_delta": upgrade_magic,
            })

    return usages


def process_power_file(filepath):
    """Process a single Java power file."""
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()

    class_name = extract_class_name(content)
    if not class_name:
        return None

    power_id = extract_power_id(content)
    if not power_id:
        return None

    constructor = extract_constructor(content, class_name)
    hooks = extract_hooks(content)
    stack_behavior = extract_stack_behavior(content)
    power_type = extract_power_type(content)
    can_go_negative = extract_can_go_negative(content)
    no_stack = extract_no_stack(content)

    spec = {
        "id": power_id,
        "java_class": class_name,
        "power_type": power_type,
    }

    if no_stack:
        spec["no_stack"] = True
    if can_go_negative:
        spec["can_go_negative"] = True

    if constructor:
        spec["constructor"] = constructor

    if hooks:
        spec["hooks"] = hooks

    if stack_behavior["method"] == "custom":
        spec["stack_behavior"] = stack_behavior

    return spec


def main():
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

    single_power = None
    include_cards = False

    for arg in sys.argv[1:]:
        if arg == "--cards":
            include_cards = True
        elif arg == "--power":
            idx = sys.argv.index("--power")
            if idx + 1 < len(sys.argv):
                single_power = sys.argv[idx + 1]

    # Collect all power Java files
    power_files = []
    for f in sorted(JAVA_POWERS_DIR.glob("*Power.java")):
        if f.name == "AbstractPower.java":
            continue
        power_files.append(f)
    # Also include watcher powers
    if JAVA_WATCHER_POWERS_DIR.exists():
        for f in sorted(JAVA_WATCHER_POWERS_DIR.glob("*Power.java")):
            power_files.append(f)

    specs = []
    errors = []

    for filepath in power_files:
        if single_power and single_power not in filepath.stem:
            continue

        try:
            spec = process_power_file(filepath)
            if spec:
                # Find card usage if requested
                if include_cards or single_power:
                    usages = find_card_usage(spec["id"], spec["java_class"])
                    if usages:
                        spec["card_creates_with"] = usages

                specs.append(spec)
            else:
                errors.append(f"Could not parse: {filepath.name}")
        except Exception as e:
            errors.append(f"Error parsing {filepath.name}: {e}")

    # Sort by id
    specs.sort(key=lambda s: s["id"])

    # Output
    if single_power:
        for spec in specs:
            print(json.dumps(spec, indent=2))
    else:
        with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
            json.dump(specs, f, indent=2, ensure_ascii=False)
        print(f"Extracted {len(specs)} power specs to {OUTPUT_FILE}")
        if errors:
            print(f"\n{len(errors)} errors:")
            for e in errors:
                print(f"  - {e}")

    # Summary stats
    if not single_power:
        hook_counts = {}
        for spec in specs:
            for hook in spec.get("hooks", []):
                method = hook["method"]
                hook_counts[method] = hook_counts.get(method, 0) + 1

        print(f"\nHook method distribution:")
        for method, count in sorted(hook_counts.items(), key=lambda x: -x[1]):
            print(f"  {method}: {count} powers")


if __name__ == "__main__":
    main()
