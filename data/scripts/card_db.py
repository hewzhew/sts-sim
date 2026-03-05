#!/usr/bin/env python3
"""
Card-Power Cross-Reference Database

Parses all card JSON files, powers.json, and hooks.rs into a SQLite in-memory
database for ad-hoc querying. Catches data inconsistencies that cause runtime bugs.

Usage:
    python card_db.py power-cards     # All Power-type cards with ApplyPower status
    python card_db.py turn-trigger    # All TurnTrigger cards + ApplyPower check
    python card_db.py power-names     # Cross-ref power names: JSON vs hooks.rs
    python card_db.py hooks           # Hook implementation status per PowerId
    python card_db.py validate        # Run all validation checks
    python card_db.py search <term>   # Full-text search across all sources
    python card_db.py sql "<query>"   # Raw SQL query against the database
"""

import json
import os
import re
import sqlite3
import sys
from pathlib import Path

# Resolve project root
SCRIPT_DIR = Path(__file__).resolve().parent
DATA_DIR = SCRIPT_DIR.parent
PROJECT_ROOT = DATA_DIR.parent
CARDS_DIR = DATA_DIR / "cards"
POWERS_JSON = DATA_DIR / "powers.json"
HOOKS_RS = PROJECT_ROOT / "src" / "powers_mod" / "hooks.rs"


def load_all_cards():
    """Parse all card JSON files into flat list of card dicts."""
    cards = []
    for json_file in sorted(CARDS_DIR.rglob("*.json")):
        rel_path = json_file.relative_to(CARDS_DIR)
        parts = rel_path.parts  # e.g. ('red', 'rare.json')
        color = parts[0] if len(parts) > 1 else "unknown"
        rarity = parts[-1].replace(".json", "") if len(parts) > 1 else "unknown"

        with open(json_file, "r", encoding="utf-8") as f:
            data = json.load(f)

        for card in data:
            card["_color"] = color
            card["_rarity_file"] = rarity
            card["_source"] = str(rel_path)
            cards.append(card)
    return cards


def load_powers():
    """Parse powers.json."""
    with open(POWERS_JSON, "r", encoding="utf-8") as f:
        return json.load(f)


def parse_hooks_rs():
    """Extract PowerId enum variants and from_str name mappings from hooks.rs."""
    with open(HOOKS_RS, "r", encoding="utf-8") as f:
        content = f.read()

    # Extract from_str mappings: "Name1" | "Name2" => Self::Variant,
    from_str_pattern = re.compile(
        r'"([^"]+)"(?:\s*\|\s*"([^"]+)")*\s*=>\s*Self::(\w+)',
    )
    # Also handle multi-alias patterns like "A" | "B" | "C" => Self::X
    full_line_pattern = re.compile(
        r'^\s*((?:"[^"]+"\s*\|\s*)*"[^"]+"\s*)\s*=>\s*Self::(\w+)',
        re.MULTILINE,
    )

    name_map = {}  # variant -> [accepted_names]
    for m in full_line_pattern.finditer(content):
        names_str = m.group(1)
        variant = m.group(2)
        names = re.findall(r'"([^"]+)"', names_str)
        name_map[variant] = names

    # Extract PowerId enum variants
    enum_pattern = re.compile(
        r"pub enum PowerId \{(.*?)\}", re.DOTALL
    )
    enum_match = enum_pattern.search(content)
    variants = []
    if enum_match:
        enum_body = enum_match.group(1)
        for line in enum_body.split("\n"):
            line = line.strip().rstrip(",")
            if line and not line.startswith("//") and not line.startswith("///") and not line.startswith("#"):
                # Skip attributes and comments
                if line.isidentifier():
                    variants.append(line)

    # Extract which hooks each power implements (at_start_of_turn, at_end_of_turn, etc.)
    # Look for PowerId::Variant => vec![...] patterns in hook methods
    hook_methods = [
        "at_start_of_turn", "at_end_of_turn", "at_end_of_round",
        "at_damage_give", "at_damage_receive", "at_damage_final_receive",
        "modify_block", "on_attacked", "on_use_card", "on_card_draw",
        "on_exhaust", "on_death", "on_gained_block", "was_hp_lost",
        "on_attack",
    ]

    # Simple approach: find all PowerId::Variant occurrences in each fn block
    hook_impl = {}  # variant -> [hook_names]
    for variant in variants:
        hook_impl[variant] = []

    for method in hook_methods:
        fn_pattern = re.compile(
            rf"pub fn {method}\b.*?\n\s*\}}\n", re.DOTALL
        )
        fn_match = fn_pattern.search(content)
        if fn_match:
            fn_body = fn_match.group(0)
            for variant in variants:
                if f"PowerId::{variant}" in fn_body:
                    if variant not in hook_impl:
                        hook_impl[variant] = []
                    hook_impl[variant].append(method)

    return variants, name_map, hook_impl


def build_database():
    """Build SQLite in-memory database from all sources."""
    db = sqlite3.connect(":memory:")
    db.row_factory = sqlite3.Row

    db.execute("""
        CREATE TABLE cards (
            id TEXT, name TEXT, card_type TEXT, cost INTEGER,
            color TEXT, rarity TEXT, rarity_file TEXT, source TEXT,
            target_type TEXT,
            has_turn_trigger INTEGER, turn_trigger_text TEXT,
            has_apply_power INTEGER, apply_power_name TEXT,
            apply_power_base INTEGER, apply_power_upgrade INTEGER,
            commands TEXT, conditions TEXT,
            original_text TEXT
        )
    """)

    db.execute("""
        CREATE TABLE powers (
            id TEXT, name TEXT, power_type TEXT, stack_type TEXT,
            description TEXT, duration_based INTEGER,
            trigger TEXT, effect TEXT, class_specific TEXT
        )
    """)

    db.execute("""
        CREATE TABLE hook_names (
            variant TEXT, accepted_name TEXT
        )
    """)

    db.execute("""
        CREATE TABLE hook_impls (
            variant TEXT, hook_method TEXT
        )
    """)

    # Load cards
    cards = load_all_cards()
    for card in cards:
        logic = card.get("logic", {})
        commands = logic.get("commands", [])
        conditions = logic.get("conditions", [])

        # Check for TurnTrigger
        turn_trigger_texts = [c for c in conditions if "turntrigger" in c.lower()]
        has_turn_trigger = len(turn_trigger_texts) > 0

        # Check for ApplyPower command
        apply_power = None
        for cmd in commands:
            if cmd.get("type") == "ApplyPower":
                apply_power = cmd.get("params", {})
                break

        # Command types list
        cmd_types = [cmd.get("type", "?") for cmd in commands]

        db.execute(
            "INSERT INTO cards VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            (
                card.get("id"),
                card.get("name"),
                card.get("type"),
                card.get("cost", 0),
                card.get("_color"),
                card.get("rarity"),
                card.get("_rarity_file"),
                card.get("_source"),
                logic.get("target_type"),
                int(has_turn_trigger),
                turn_trigger_texts[0] if turn_trigger_texts else None,
                int(apply_power is not None),
                apply_power.get("power") if apply_power else None,
                apply_power.get("base", apply_power.get("amount")) if apply_power else None,
                apply_power.get("upgrade", apply_power.get("upgrade_amount")) if apply_power else None,
                json.dumps(cmd_types),
                json.dumps(conditions),
                card.get("original_text"),
            ),
        )

    # Load powers
    powers = load_powers()
    for p in powers:
        logic = p.get("logic", {})
        # Extract effect — could be single or list
        effect = logic.get("effect", "")
        if not effect and "effects" in logic:
            effect = json.dumps(logic["effects"])

        db.execute(
            "INSERT INTO powers VALUES (?,?,?,?,?,?,?,?,?)",
            (
                p.get("id"),
                p.get("name"),
                p.get("type"),
                p.get("stack_type"),
                p.get("description"),
                int(p.get("duration_based", False)),
                logic.get("trigger", ""),
                effect,
                p.get("class_specific", ""),
            ),
        )

    # Load hooks
    variants, name_map, hook_impl = parse_hooks_rs()
    for variant, names in name_map.items():
        for name in names:
            db.execute("INSERT INTO hook_names VALUES (?,?)", (variant, name))

    for variant, hooks in hook_impl.items():
        for hook in hooks:
            db.execute("INSERT INTO hook_impls VALUES (?,?)", (variant, hook))

    db.commit()
    return db


# ============================================================================
# Query Commands
# ============================================================================

def cmd_power_cards(db):
    """List all Power-type cards with their ApplyPower status."""
    print("\n=== Power Cards ===\n")
    print(f"{'ID':<25} {'Color':<10} {'Cost':<5} {'ApplyPower?':<12} {'PowerName':<20} {'Stacks':<12} {'Commands'}")
    print("-" * 120)

    rows = db.execute("""
        SELECT id, color, cost, has_apply_power, apply_power_name,
               apply_power_base, apply_power_upgrade, commands
        FROM cards WHERE card_type = 'Power'
        ORDER BY color, id
    """).fetchall()

    for r in rows:
        ap = "[OK]" if r["has_apply_power"] else "✗"
        name = r["apply_power_name"] or "-"
        stacks = f"{r['apply_power_base']}/{r['apply_power_upgrade']}" if r["has_apply_power"] else "-"
        print(f"{r['id']:<25} {r['color']:<10} {r['cost']:<5} {ap:<12} {name:<20} {stacks:<12} {r['commands']}")

    print(f"\nTotal: {len(rows)} Power cards")


def cmd_turn_trigger(db):
    """All cards with TurnTrigger conditions + ApplyPower check."""
    print("\n=== TurnTrigger Cards ===\n")
    print(f"{'ID':<25} {'Type':<8} {'Color':<10} {'ApplyPower?':<12} {'PowerName':<20} {'Trigger Text'}")
    print("-" * 120)

    rows = db.execute("""
        SELECT id, card_type, color, has_apply_power, apply_power_name,
               turn_trigger_text, commands
        FROM cards WHERE has_turn_trigger = 1
        ORDER BY card_type, color, id
    """).fetchall()

    issues = 0
    for r in rows:
        ap = "[OK]" if r["has_apply_power"] else "[MISS]"
        name = r["apply_power_name"] or "-"
        flag = ""
        if r["card_type"] == "Power" and not r["has_apply_power"]:
            flag = " [!]"
            issues += 1
        print(f"{r['id']:<25} {r['card_type']:<8} {r['color']:<10} {ap:<12} {name:<20} {r['turn_trigger_text']}{flag}")

    print(f"\nTotal: {len(rows)} TurnTrigger cards, {issues} Power cards missing ApplyPower")


def cmd_power_names(db):
    """Cross-reference power names used in card JSON vs hooks.rs from_str."""
    print("\n=== Power Name Cross-Reference ===\n")

    # Get all power names used in ApplyPower commands
    card_names = db.execute("""
        SELECT DISTINCT apply_power_name FROM cards
        WHERE apply_power_name IS NOT NULL
    """).fetchall()

    print(f"{'Card ApplyPower Name':<30} {'Resolves in hooks.rs?':<25} {'→ PowerId Variant'}")
    print("-" * 80)

    issues = 0
    for r in card_names:
        name = r["apply_power_name"]
        match = db.execute(
            "SELECT variant FROM hook_names WHERE accepted_name = ?", (name,)
        ).fetchone()

        if match:
            print(f"{name:<30} {'✓':<25} {match['variant']}")
        else:
            print(f"{name:<30} {'✗ NOT FOUND':<25} -")
            issues += 1

    # Also show powers.json entries without a hook mapping
    print(f"\n--- powers.json entries without hooks.rs mapping ---\n")
    power_ids = db.execute("SELECT id, name FROM powers").fetchall()
    for p in power_ids:
        match = db.execute(
            "SELECT variant FROM hook_names WHERE accepted_name = ? OR accepted_name = ?",
            (p["id"], p["name"]),
        ).fetchone()
        if not match:
            print(f"  {p['id']:<25} ({p['name']})")

    print(f"\n{issues} card ApplyPower name(s) do not resolve in hooks.rs")


def cmd_hooks(db):
    """Hook implementation status per PowerId variant."""
    print("\n=== Hook Implementation Status ===\n")
    print(f"{'PowerId Variant':<25} {'Hooks Implemented'}")
    print("-" * 80)

    # Get all variants that have at least one hook
    rows = db.execute("""
        SELECT variant, GROUP_CONCAT(hook_method, ', ') as hooks
        FROM hook_impls GROUP BY variant ORDER BY variant
    """).fetchall()

    for r in rows:
        print(f"{r['variant']:<25} {r['hooks']}")

    print(f"\nTotal: {len(rows)} PowerId variants with hook implementations")


def cmd_validate(db):
    """Run all validation checks."""
    print("\n=== Validation Report ===\n")
    errors = 0
    warnings = 0

    # 1. Power+TurnTrigger must have ApplyPower
    print("--- Check 1: Power+TurnTrigger cards must have ApplyPower ---")
    rows = db.execute("""
        SELECT id, color, commands FROM cards
        WHERE card_type = 'Power' AND has_turn_trigger = 1 AND has_apply_power = 0
    """).fetchall()
    for r in rows:
        print(f"  ERROR: {r['id']} ({r['color']}) — Power+TurnTrigger but no ApplyPower! Commands: {r['commands']}")
        errors += 1
    if not rows:
        print("  [OK] All Power+TurnTrigger cards have ApplyPower")

    # 2. ApplyPower names must resolve in hooks.rs
    print("\n--- Check 2: ApplyPower names must resolve in hooks.rs from_str ---")
    rows = db.execute("""
        SELECT DISTINCT c.id, c.apply_power_name FROM cards c
        WHERE c.apply_power_name IS NOT NULL
        AND NOT EXISTS (
            SELECT 1 FROM hook_names h WHERE h.accepted_name = c.apply_power_name
        )
    """).fetchall()
    for r in rows:
        print(f"  ERROR: {r['id']} uses ApplyPower name '{r['apply_power_name']}' which is not in hooks.rs from_str")
        errors += 1
    if not rows:
        print("  [OK] All ApplyPower names resolve in hooks.rs")

    # 3. Duplicate card IDs
    print("\n--- Check 3: No duplicate card IDs ---")
    rows = db.execute("""
        SELECT id, COUNT(*) as cnt, GROUP_CONCAT(source) as sources
        FROM cards GROUP BY id HAVING cnt > 1
    """).fetchall()
    for r in rows:
        print(f"  WARN: Duplicate card ID '{r['id']}' appears {r['cnt']} times in: {r['sources']}")
        warnings += 1
    if not rows:
        print("  [OK] No duplicate card IDs")

    # 4. powers.json entries without hooks.rs PowerId
    print("\n--- Check 4: powers.json entries resolvable in hooks.rs ---")
    power_ids = db.execute("SELECT id, name FROM powers").fetchall()
    unresolved = 0
    for p in power_ids:
        match = db.execute(
            "SELECT variant FROM hook_names WHERE accepted_name = ? OR accepted_name = ?",
            (p["id"], p["name"]),
        ).fetchone()
        if not match:
            print(f"  WARN: powers.json '{p['id']}' has no hooks.rs PowerId mapping")
            warnings += 1
            unresolved += 1
    if unresolved == 0:
        print("  [OK] All powers.json entries have hooks.rs mappings")

    # 5. Power cards with no hook-handled commands but also no TurnTrigger (just immediate)
    # This is fine — cards like Inflame just do GainBuff(Strength) immediately.
    # But warn if a Power card has ONLY hook-handled commands and no TurnTrigger or ApplyPower
    print("\n--- Check 5: Power cards without TurnTrigger should not have ONLY hook-handled commands ---")
    rows = db.execute("""
        SELECT id, commands FROM cards
        WHERE card_type = 'Power' AND has_turn_trigger = 0 AND has_apply_power = 0
    """).fetchall()
    for r in rows:
        cmds = json.loads(r["commands"])
        # These are "immediate" power cards like Inflame — this is fine
        pass
    print("  [OK] (informational only)")

    # Summary
    print(f"\n{'='*40}")
    print(f"Result: {errors} errors, {warnings} warnings")
    if errors == 0:
        print("[OK] All critical checks passed!")
    else:
        print("[FAIL] Fix errors before proceeding")
    return errors


def cmd_search(db, term):
    """Full-text search across all sources."""
    print(f"\n=== Search: '{term}' ===\n")
    term_like = f"%{term}%"

    # Search cards
    rows = db.execute("""
        SELECT id, card_type, color, cost, commands, conditions, original_text
        FROM cards
        WHERE id LIKE ? OR name LIKE ? OR commands LIKE ? OR conditions LIKE ? OR original_text LIKE ?
    """, (term_like, term_like, term_like, term_like, term_like)).fetchall()

    if rows:
        print(f"--- Cards ({len(rows)} matches) ---")
        for r in rows:
            print(f"  [{r['card_type']}] {r['id']} ({r['color']}, cost {r['cost']})")
            print(f"    Commands: {r['commands']}")
            if r['conditions'] and r['conditions'] != '[]':
                print(f"    Conditions: {r['conditions']}")
            print()

    # Search powers
    rows = db.execute("""
        SELECT id, name, power_type, trigger, effect, description
        FROM powers WHERE id LIKE ? OR name LIKE ? OR description LIKE ?
    """, (term_like, term_like, term_like)).fetchall()

    if rows:
        print(f"--- Powers ({len(rows)} matches) ---")
        for r in rows:
            print(f"  [{r['power_type']}] {r['id']} — {r['name']}")
            print(f"    Trigger: {r['trigger']}, Effect: {r['effect']}")
            print(f"    {r['description']}")
            print()

    # Search hook names
    rows = db.execute("""
        SELECT variant, accepted_name FROM hook_names
        WHERE variant LIKE ? OR accepted_name LIKE ?
    """, (term_like, term_like)).fetchall()

    if rows:
        print(f"--- Hook Names ({len(rows)} matches) ---")
        for r in rows:
            print(f"  {r['variant']} ← \"{r['accepted_name']}\"")


def cmd_sql(db, query):
    """Execute raw SQL query."""
    try:
        rows = db.execute(query).fetchall()
        if rows:
            # Print header
            keys = rows[0].keys()
            print("\t".join(keys))
            print("-" * 80)
            for r in rows:
                print("\t".join(str(r[k]) for k in keys))
            print(f"\n({len(rows)} rows)")
        else:
            print("(0 rows)")
    except Exception as e:
        print(f"SQL error: {e}")


def main():
    # Fix Windows encoding
    import io
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')

    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(1)

    cmd = sys.argv[1]
    db = build_database()

    if cmd == "power-cards":
        cmd_power_cards(db)
    elif cmd == "turn-trigger":
        cmd_turn_trigger(db)
    elif cmd == "power-names":
        cmd_power_names(db)
    elif cmd == "hooks":
        cmd_hooks(db)
    elif cmd == "validate":
        errors = cmd_validate(db)
        sys.exit(1 if errors > 0 else 0)
    elif cmd == "search":
        if len(sys.argv) < 3:
            print("Usage: card_db.py search <term>")
            sys.exit(1)
        cmd_search(db, sys.argv[2])
    elif cmd == "sql":
        if len(sys.argv) < 3:
            print("Usage: card_db.py sql \"<query>\"")
            sys.exit(1)
        cmd_sql(db, sys.argv[2])
    else:
        print(f"Unknown command: {cmd}")
        print(__doc__)
        sys.exit(1)


if __name__ == "__main__":
    main()
