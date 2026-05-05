from __future__ import annotations

import re
import subprocess
from pathlib import Path

try:
    from tools.sts_tool.java_parser import (
        JAVA_SRC,
        extract_class,
        find_descendants,
        node_text,
        parse_file,
    )
except ImportError:
    from sts_tool.java_parser import (  # type: ignore
        JAVA_SRC,
        extract_class,
        find_descendants,
        node_text,
        parse_file,
    )


BASE_CLASSES = {
    "AbstractPower",
    "AbstractRelic",
    "AbstractCard",
    "AbstractAction",
    "AbstractMonster",
    "AbstractCreature",
    "AbstractPotion",
    "AbstractOrb",
    "AbstractPlayer",
    "AbstractRoom",
    "AbstractDungeon",
}


def is_class_instantiated(class_name: str, java_dir: Path = JAVA_SRC) -> bool:
    try:
        out = subprocess.run(
            ["rg", "-F", "-l", f"new {class_name}(", str(java_dir), "-g", "*.java"],
            capture_output=True,
            text=True,
            timeout=10,
            encoding="utf-8",
        )
        files = [
            f
            for f in out.stdout.strip().splitlines()
            if f.strip() and Path(f).stem != class_name
        ]
        return bool(files)
    except Exception:
        return True


def detect_side_effects(method_node, source: str) -> list[str]:
    effects: list[str] = []
    body_text = node_text(method_node, source)
    if "addToBot" in body_text:
        effects.append("QUEUES_ACTIONS(addToBot)")
    if "addToTop" in body_text:
        effects.append("QUEUES_ACTIONS(addToTop)")

    for assign in find_descendants(method_node, "assignment_expression"):
        if assign.children:
            lhs = node_text(assign.children[0], source)
            if lhs.startswith("this."):
                effects.append(f"MUTATES({lhs})")

    for inv in find_descendants(method_node, "method_invocation"):
        text = node_text(inv, source)
        if text.startswith("this.owner.") or text.startswith("owner."):
            match = re.match(r"(?:this\.)?owner\.(\w+)", text)
            if match:
                effects.append(f"CALLS_OWNER({match.group(1)})")

    return list(dict.fromkeys(effects))


def extract_hardcoded_checks(method_node, source: str) -> list[dict]:
    checks: list[dict] = []
    if method_node is None:
        return checks
    for inv in find_descendants(method_node, "method_invocation"):
        text = node_text(inv, source)
        if ".hasPower(" in text or ".hasRelic(" in text:
            check_type = "hasPower" if ".hasPower(" in text else "hasRelic"
            for arg in find_descendants(inv, "string_literal"):
                checks.append(
                    {
                        "type": check_type,
                        "id": node_text(arg, source).strip('"'),
                        "line": inv.start_point[0] + 1,
                    }
                )
    return checks


def get_enclosing_method(node, source: str) -> str:
    current = node.parent
    while current:
        if current.type == "method_declaration":
            for child in current.children:
                if child.type == "identifier":
                    return node_text(child, source)
        current = current.parent
    return "<unknown>"


def get_enclosing_method_node(node):
    current = node.parent
    while current:
        if current.type == "method_declaration":
            return current
        current = current.parent
    return None


def get_enclosing_for_iterable(node, source: str) -> str | None:
    current = node.parent
    while current:
        if current.type == "enhanced_for_statement":
            for i, child in enumerate(current.children):
                if child.type == ":" and i + 1 < len(current.children):
                    for j in range(i + 1, len(current.children)):
                        nxt = current.children[j]
                        if nxt.type not in ("{", "}", "block"):
                            return node_text(nxt, source).strip()
            break
        if current.type == "method_declaration":
            break
        current = current.parent
    return None


def find_invocations_of(root_node, source: str, method_name: str) -> list[dict]:
    results: list[dict] = []
    for inv in find_descendants(root_node, "method_invocation"):
        for child in inv.children:
            if child.type == "identifier" and node_text(child, source) == method_name:
                results.append(
                    {
                        "text": node_text(inv, source),
                        "line": inv.start_point[0] + 1,
                        "enclosing": get_enclosing_method(inv, source),
                        "enclosing_node": get_enclosing_method_node(inv),
                        "iterable": get_enclosing_for_iterable(inv, source),
                    }
                )
                break
    return results


def iter_java_classes(java_dir: Path = JAVA_SRC):
    for fpath in java_dir.rglob("*.java"):
        try:
            ctx = parse_file(fpath)
            cls = extract_class(ctx)
        except Exception:
            continue
        if cls:
            yield fpath, ctx, cls
