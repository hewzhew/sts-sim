"""
Schema Lookup, Batch Audit & Insertion-Mode Checker for STS Java-to-Rust Porting.

Modes:
  query            — Translate between Rust enums, filenames, and Java IDs.
  audit            — Batch-check a range of relics: hooks, .rs file, scattered logic.
  check-insertion  — Auto-detect addToBot/addToTop mismatches between Java and Rust.

Usage:
  python query_relics.py query <search_term> [--type relic|card|power|monster|potion]
  python query_relics.py audit <prefix> [--type relic]
  python query_relics.py check-insertion <prefix>
"""

import sys
import json
import re
import os
import argparse
import glob
from collections import defaultdict

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SCHEMA_PATH = os.path.join(SCRIPT_DIR, "protocol_schema.json")
SCATTERED_LOGIC_PATH = os.path.join(
    SCRIPT_DIR, "source_extractor", "output", "scattered_logic.md"
)
RELICS_DIR = os.path.join(
    SCRIPT_DIR, "source_extractor", "output", "relics"
)
# Fallback: monolithic relics.md
RELICS_MD_PATH = os.path.join(
    SCRIPT_DIR, "source_extractor", "output", "relics.md"
)
HOOKS_MD_PATH = os.path.join(
    SCRIPT_DIR, "source_extractor", "output", "hooks.md"
)
JAVA_SRC_ROOT = os.path.normpath(os.path.join(SCRIPT_DIR, "..", "..", "cardcrawl"))
RUST_RELICS_DIR = os.path.normpath(
    os.path.join(SCRIPT_DIR, "..", "src", "content", "relics")
)

ENUM_KEY_MAP = {
    "relic":   "relic_id",
    "card":    "card_id",
    "power":   "power_id",
    "monster": "monster_id",
    "potion":  "potion_id",
}

JAVA_SUBDIR_MAP = {
    "relic":   "relics",
    "card":    "cards",
    "power":   "powers",
    "monster": "monsters",
    "potion":  "potions",
}


def to_snake_case(name: str) -> str:
    name = name.replace(" ", "_").replace("-", "_")
    s1 = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    result = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s1).lower()
    return re.sub(r"_+", "_", result)


def load_schema():
    for enc in ("utf-8", "utf-8-sig", "utf-16", "latin-1"):
        try:
            with open(SCHEMA_PATH, "r", encoding=enc) as f:
                content = f.read()
            if content.startswith("\ufeff"):
                content = content[1:]
            return json.loads(content)
        except (UnicodeDecodeError, UnicodeError):
            continue
    raise RuntimeError(f"Cannot decode {SCHEMA_PATH}")


def load_scattered_logic():
    if not os.path.isfile(SCATTERED_LOGIC_PATH):
        return ""
    for enc in ("utf-8", "utf-8-sig", "utf-16", "latin-1"):
        try:
            with open(SCATTERED_LOGIC_PATH, "r", encoding=enc) as f:
                return f.read()
        except (UnicodeDecodeError, UnicodeError):
            continue
    return ""


def get_java_ids(data):
    java_ids = data.get("java", "UNKNOWN")
    if isinstance(java_ids, str):
        return [java_ids]
    elif isinstance(java_ids, list):
        return [str(j) for j in java_ids]
    return [str(java_ids)]


def search_entries(entries: dict, term: str):
    results = []
    for rust_enum, data in entries.items():
        java_ids = get_java_ids(data)
        snake = to_snake_case(rust_enum)
        matched = term in rust_enum.lower() or term in snake
        if not matched:
            for jid in java_ids:
                if term in jid.lower():
                    matched = True
                    break
        if matched:
            results.append((rust_enum, snake, java_ids[0]))
    results.sort(key=lambda x: x[0])
    return results


def print_results(results, entity_type: str):
    if not results:
        return
    print(f"\n  [{entity_type.upper()}] ({len(results)} matches)")
    print(f"  {'Rust Enum':<30} {'File (.rs)':<28} Java ID")
    print("  " + "-" * 80)
    for rust_enum, snake, java_id in results:
        print(f"  {rust_enum:<30} {snake + '.rs':<28} {java_id}")


def get_relic_hooks_from_md(java_class_name: str) -> list[str]:
    """Extract hook names for a relic from per-letter .md files or monolithic relics.md."""
    letter = java_class_name[0].upper() if java_class_name else ""
    # Try per-letter file first
    letter_file = os.path.join(RELICS_DIR, f"{letter}.md")
    if os.path.isfile(letter_file):
        target_path = letter_file
    elif os.path.isfile(RELICS_MD_PATH):
        target_path = RELICS_MD_PATH
    else:
        return []

    hooks = []
    in_relic = False
    try:
        with open(target_path, "r", encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if line.startswith("## ") and not line.startswith("### "):
                    in_relic = line[3:].strip() == java_class_name
                elif in_relic and line.startswith("### "):
                    # Extract hook name from "### hookName(params)"
                    hook = line[4:].split("(")[0].strip()
                    if hook and hook not in ("makeCopy", "getUpdatedDescription",
                                             "updateDescription", "setDescription"):
                        hooks.append(hook)
    except Exception:
        pass
    return hooks


# Cosmetic-only Java actions that have no game logic — skip in insertion mode counting
COSMETIC_ACTIONS = {
    "RelicAboveCreatureAction",
    "SFXAction",
    "VFXAction",
    "WaitAction",
    "TextAboveCreatureAction",
    "TalkAction",
    "FlashAtkImgEffect",
}


def _is_cosmetic_line(line: str) -> bool:
    """Check if an addToBot/addToTop line only contains cosmetic actions."""
    return any(action in line for action in COSMETIC_ACTIONS)


def get_java_insertion_modes(java_class_name: str) -> dict[str, list[str]]:
    """Extract addToBot/addToTop calls per hook from per-letter .md files.
    Filters out cosmetic-only actions (RelicAboveCreatureAction etc.)."""
    letter = java_class_name[0].upper() if java_class_name else ""
    letter_file = os.path.join(RELICS_DIR, f"{letter}.md")
    if os.path.isfile(letter_file):
        target_path = letter_file
    elif os.path.isfile(RELICS_MD_PATH):
        target_path = RELICS_MD_PATH
    else:
        return {}

    result = {}  # hook_name -> list of "BOT" or "TOP"
    in_relic = False
    current_hook = None
    in_code_block = False
    try:
        with open(target_path, "r", encoding="utf-8") as f:
            for line in f:
                stripped = line.strip()
                if stripped.startswith("## ") and not stripped.startswith("### "):
                    in_relic = stripped[3:].strip() == java_class_name
                    current_hook = None
                elif in_relic and stripped.startswith("### "):
                    current_hook = stripped[4:].split("(")[0].strip()
                    result[current_hook] = []
                elif in_relic and current_hook:
                    if stripped == "```java":
                        in_code_block = True
                    elif stripped == "```" and in_code_block:
                        in_code_block = False
                    elif in_code_block:
                        # Skip cosmetic-only lines
                        if _is_cosmetic_line(stripped):
                            continue
                        if "addToBot(" in stripped or "this.addToBot(" in stripped:
                            result[current_hook].append("BOT")
                        if "addToTop(" in stripped or "this.addToTop(" in stripped:
                            result[current_hook].append("TOP")
    except Exception:
        pass
    return result


# Rust actions that are internal bookkeeping with no Java equivalent — skip in counting
RUST_BOOKKEEPING_ACTIONS = {
    "UpdateRelicCounter",
    "UpdateRelicState",
}


def get_rust_insertion_modes(snake_name: str) -> list[str]:
    """Extract AddTo::Top/AddTo::Bottom from a Rust .rs file.
    Filters out internal bookkeeping actions (UpdateRelicCounter etc.)
    since Java does counter mutation inline, not as queued actions."""
    rs_path = os.path.join(RUST_RELICS_DIR, f"{snake_name}.rs")
    if not os.path.isfile(rs_path):
        return []

    modes = []
    lines = []
    try:
        with open(rs_path, "r", encoding="utf-8") as f:
            lines = f.readlines()
    except Exception:
        return []

    # Scan for AddTo:: lines, but check nearby context for bookkeeping actions
    for i, line in enumerate(lines):
        if "AddTo::Top" not in line and "AddTo::Bottom" not in line:
            continue
        # Look back up to 5 lines for the action type
        context = "".join(lines[max(0, i-5):i+1])
        if any(action in context for action in RUST_BOOKKEEPING_ACTIONS):
            continue  # Skip bookkeeping
        if "AddTo::Top" in line:
            modes.append("TOP")
        elif "AddTo::Bottom" in line:
            modes.append("BOT")
    return modes


def cmd_query(args, schema):
    term = args.search.lower()
    enums = schema.get("enums", {})
    types_to_search = (
        {args.type: ENUM_KEY_MAP[args.type]} if args.type
        else ENUM_KEY_MAP
    )
    total = 0
    for label, key in types_to_search.items():
        entries = enums.get(key, {}).get("entries", {})
        results = search_entries(entries, term)
        if results:
            print_results(results, label)
            total += len(results)
    if total == 0:
        print(f"No entries found matching '{args.search}'.")
    else:
        print(f"\n  Total: {total} match(es).")


def cmd_audit(args, schema):
    """Enhanced batch audit: list relics with hooks, .rs file status, and scattered logic."""
    prefix = args.search.lower()
    entity_type = args.type or "relic"
    enum_key = ENUM_KEY_MAP[entity_type]
    entries = schema.get("enums", {}).get(enum_key, {}).get("entries", {})

    matches = []
    for rust_enum, data in entries.items():
        if rust_enum.lower().startswith(prefix):
            java_ids = get_java_ids(data)
            snake = to_snake_case(rust_enum)
            matches.append((rust_enum, snake, java_ids[0]))
    matches.sort(key=lambda x: x[0])

    if not matches:
        print(f"No {entity_type} entries starting with '{args.search}'.")
        return

    scattered = load_scattered_logic()
    scattered_lower = scattered.lower()

    print(f"\n  Batch audit: {len(matches)} {entity_type}(s) matching '{args.search}'")
    print(f"  {'Rust Enum':<22} {'Java ID':<22} {'.rs?':<6} {'Hooks':<40} {'Scattered'}")
    print("  " + "-" * 110)

    needs_attention = []
    missing_rs = []
    clean = []

    for rust_enum, snake, java_id in matches:
        # Check .rs file existence
        rs_path = os.path.join(RUST_RELICS_DIR, f"{snake}.rs")
        has_rs = os.path.isfile(rs_path)
        rs_marker = "  ✓ " if has_rs else " ✗  "

        # Get hooks from extracted Java
        java_class = java_id.replace(" ", "")
        hooks = get_relic_hooks_from_md(java_class)
        hooks_str = ", ".join(hooks) if hooks else "(none)"
        if len(hooks_str) > 38:
            hooks_str = hooks_str[:35] + "..."

        # Check scattered logic
        found = java_id.lower() in scattered_lower
        scattered_marker = ">>> YES" if found else "    no"

        print(f"  {rust_enum:<22} {java_id:<22} {rs_marker:<6} {hooks_str:<40} {scattered_marker}")

        if found:
            needs_attention.append((rust_enum, snake, java_id))
        elif not has_rs and hooks:
            missing_rs.append((rust_enum, snake, java_id, hooks))
        else:
            clean.append((rust_enum, snake, java_id))

    # Summary
    print(f"\n  ---")
    print(f"  Clean (local hooks only):  {len(clean)}")
    print(f"  Has scattered logic:       {len(needs_attention)}")
    print(f"  Missing .rs (has hooks):   {len(missing_rs)}")

    if needs_attention:
        print(f"\n  ⚠ Entities requiring engine-level review:")
        for rust_enum, snake, java_id in needs_attention:
            print(f"    - {rust_enum} ({java_id})")

    if missing_rs:
        print(f"\n  ✗ Missing Rust implementation (have Java hooks):")
        for rust_enum, snake, java_id, hooks in missing_rs:
            print(f"    - {rust_enum} ({snake}.rs) — hooks: {', '.join(hooks)}")


def cmd_check_insertion(args, schema):
    """Auto-detect addToBot/addToTop mismatches between Java and Rust relic files."""
    prefix = args.search.lower()
    entries = schema.get("enums", {}).get("relic_id", {}).get("entries", {})

    matches = []
    for rust_enum, data in entries.items():
        if rust_enum.lower().startswith(prefix):
            java_ids = get_java_ids(data)
            snake = to_snake_case(rust_enum)
            matches.append((rust_enum, snake, java_ids[0]))
    matches.sort(key=lambda x: x[0])

    if not matches:
        print(f"No relics starting with '{args.search}'.")
        return

    print(f"\n  Insertion mode check: {len(matches)} relic(s) matching '{args.search}'")
    print(f"  {'Relic':<22} {'Hook':<22} {'Java':<20} {'Rust':<20} {'Match?'}")
    print("  " + "-" * 100)

    issues = []
    checked = 0

    for rust_enum, snake, java_id in matches:
        java_class = java_id.replace(" ", "")
        java_modes = get_java_insertion_modes(java_class)
        rust_modes = get_rust_insertion_modes(snake)

        if not java_modes or not rust_modes:
            continue

        # Flatten Java modes for comparison
        java_flat = []
        for hook, modes in java_modes.items():
            for m in modes:
                java_flat.append(m)

        # Compare counts
        java_tops = java_flat.count("TOP")
        java_bots = java_flat.count("BOT")
        rust_tops = rust_modes.count("TOP")
        rust_bots = rust_modes.count("BOT")

        match = (java_tops == rust_tops and java_bots == rust_bots)
        marker = "  ✓" if match else "  ✗ MISMATCH"

        java_str = f"TOP:{java_tops} BOT:{java_bots}"
        rust_str = f"TOP:{rust_tops} BOT:{rust_bots}"

        hooks_str = ", ".join(java_modes.keys())
        if len(hooks_str) > 20:
            hooks_str = hooks_str[:17] + "..."

        print(f"  {rust_enum:<22} {hooks_str:<22} {java_str:<20} {rust_str:<20} {marker}")
        checked += 1

        if not match:
            issues.append((rust_enum, snake, java_modes, rust_modes))

    print(f"\n  ---")
    print(f"  Checked: {checked}")
    print(f"  Mismatches: {len(issues)}")

    if issues:
        print(f"\n  ⚠ Insertion mode mismatches:")
        for rust_enum, snake, java_modes, rust_modes in issues:
            print(f"\n    {rust_enum} ({snake}.rs):")
            for hook, modes in java_modes.items():
                if modes:
                    print(f"      Java {hook}: {' '.join(modes)}")
            print(f"      Rust file: {' '.join(rust_modes)}")


def main():
    parser = argparse.ArgumentParser(
        description="Schema lookup, batch audit & insertion-mode checker for STS porting."
    )
    subparsers = parser.add_subparsers(dest="command")

    # query subcommand
    q = subparsers.add_parser("query", help="Search schema entries.")
    q.add_argument("search", help="Substring to match (case-insensitive).")
    q.add_argument("--type", "-t", choices=list(ENUM_KEY_MAP.keys()), default=None)

    # audit subcommand
    a = subparsers.add_parser("audit", help="Batch audit with hooks, .rs file, scattered logic.")
    a.add_argument("search", help="Prefix to match Rust enum name.")
    a.add_argument("--type", "-t", choices=list(ENUM_KEY_MAP.keys()), default="relic")

    # check-insertion subcommand
    ci = subparsers.add_parser("check-insertion", help="Auto-detect addToBot/addToTop mismatches.")
    ci.add_argument("search", help="Prefix to match Rust enum name.")

    args = parser.parse_args()

    # Backward compatibility: bare argument treated as query
    if args.command is None:
        if len(sys.argv) >= 2 and sys.argv[1] not in ("query", "audit", "check-insertion", "-h", "--help"):
            sys.argv.insert(1, "query")
            args = parser.parse_args()
        else:
            parser.print_help()
            return

    schema = load_schema()

    if args.command == "query":
        cmd_query(args, schema)
    elif args.command == "audit":
        cmd_audit(args, schema)
    elif args.command == "check-insertion":
        cmd_check_insertion(args, schema)


if __name__ == "__main__":
    main()
