"""
java_parser.py — Tree-sitter Java parsing primitives.

All Java source analysis goes through this module. Zero regex for structural parsing.
"""

from pathlib import Path
from dataclasses import dataclass, field
from typing import Optional

import tree_sitter_java as tsjava
from tree_sitter import Language, Parser, Node

# ── Tree-sitter setup ─────────────────────────────────────────────────────

JAVA_LANG = Language(tsjava.language())
_parser = Parser(JAVA_LANG)

# ── Config ─────────────────────────────────────────────────────────────────

JAVA_SRC = Path(r"d:\rust\cardcrawl")

# Known abstract base classes (auto-categorization by superclass)
_CATEGORY_MAP = {
    "AbstractCard": "card",
    "AbstractPower": "power",
    "AbstractRelic": "relic",
    "AbstractGameAction": "action",
    "AbstractMonster": "monster",
    "AbstractPotion": "potion",
    "AbstractOrb": "orb",
    "AbstractPlayer": "engine",
    "AbstractRoom": "engine",
    "AbstractCreature": "engine",
    "AbstractDungeon": "engine",
}


# ── Data classes ───────────────────────────────────────────────────────────

@dataclass
class MethodInfo:
    name: str
    params: str
    return_type: str
    node: Node           # tree-sitter AST node for the full method declaration
    body_node: Node | None  # the block {} node
    start_line: int
    end_line: int


@dataclass
class ClassInfo:
    name: str
    superclass: str | None
    category: str
    file_path: Path
    string_id: str | None  # e.g., "Strength" from POWER_ID
    methods: dict[str, MethodInfo] = field(default_factory=dict)


@dataclass
class FileContext:
    """Holds source text + parsed tree for a single Java file."""
    path: Path
    source: str
    tree: object  # tree-sitter Tree


# ── Core parsing ───────────────────────────────────────────────────────────

def parse_file(path: Path) -> FileContext:
    """Parse a Java file, return FileContext with source and AST tree."""
    raw = path.read_bytes()
    tree = _parser.parse(raw)
    source = raw.decode("utf-8", errors="replace")
    return FileContext(path=path, source=source, tree=tree)


def node_text(node: Node, source: str) -> str:
    """Extract text of an AST node."""
    if node is None:
        return ""
    return source[node.start_byte:node.end_byte]


def find_descendants(node: Node, type_name: str) -> list[Node]:
    """Recursively find all descendants of a given type."""
    results = []
    for c in node.children:
        if c.type == type_name:
            results.append(c)
        results.extend(find_descendants(c, type_name))
    return results


def find_child(node: Node, type_name: str) -> Node | None:
    """Find first direct child of a given type."""
    for c in node.children:
        if c.type == type_name:
            return c
    return None


def find_children(node: Node, type_name: str) -> list[Node]:
    """Find all direct children of a given type."""
    return [c for c in node.children if c.type == type_name]


# ── Class-level extraction ─────────────────────────────────────────────────

def _categorize_by_path(rel_path: str) -> str:
    """Fallback category from file path when superclass doesn't match."""
    p = rel_path.replace("\\", "/").lower()
    for seg, cat in [
        ("/cards/", "card"), ("/powers/", "power"), ("/relics/", "relic"),
        ("/potions/", "potion"), ("/monsters/", "monster"), ("/orbs/", "orb"),
        ("/actions/", "action"),
    ]:
        if seg in p:
            return cat
    return "other"


def extract_class(ctx: FileContext) -> ClassInfo | None:
    """Extract the primary class declaration from a parsed file."""
    class_nodes = find_descendants(ctx.tree.root_node, "class_declaration")
    if not class_nodes:
        return None

    class_node = class_nodes[0]  # Primary class

    # Class name
    name_node = find_child(class_node, "identifier")
    if not name_node:
        return None
    name = node_text(name_node, ctx.source)

    # Superclass
    superclass = None
    sc_node = find_child(class_node, "superclass")
    if sc_node:
        ti = find_child(sc_node, "type_identifier")
        if ti:
            superclass = node_text(ti, ctx.source)

    # Category
    category = _CATEGORY_MAP.get(superclass, _categorize_by_path(
        str(ctx.path.relative_to(JAVA_SRC)) if ctx.path.is_relative_to(JAVA_SRC) else str(ctx.path)
    ))

    # String ID (e.g., POWER_ID, ID, POTION_ID)
    string_id = _extract_string_id(class_node, ctx.source)

    # Methods
    methods = {}
    body_node = find_child(class_node, "class_body")
    if body_node:
        for member in body_node.children:
            if member.type == "method_declaration":
                mi = _extract_method(member, ctx.source)
                if mi:
                    methods[mi.name] = mi

    return ClassInfo(
        name=name,
        superclass=superclass,
        category=category,
        file_path=ctx.path,
        string_id=string_id,
        methods=methods,
    )


def _extract_string_id(class_node: Node, source: str) -> str | None:
    """Extract the string ID from static final declarations like:
    public static final String ID = "Strength";
    public static final String POWER_ID = "Strength";
    """
    for field_decl in find_descendants(class_node, "field_declaration"):
        text = node_text(field_decl, source)
        # Look for static final String XX_ID patterns
        if "static" in text and "final" in text and "String" in text:
            for decl in find_descendants(field_decl, "variable_declarator"):
                var_name_node = find_child(decl, "identifier")
                if var_name_node:
                    var_name = node_text(var_name_node, source)
                    if var_name in ("ID", "POWER_ID", "POTION_ID"):
                        # Get the string literal value
                        str_lit = find_child(decl, "string_literal")
                        if str_lit:
                            return node_text(str_lit, source).strip('"')
    return None


def _extract_method(method_node: Node, source: str) -> MethodInfo | None:
    """Extract method info from a method_declaration node."""
    name = None
    params = ""
    return_type = ""
    body = None

    for c in method_node.children:
        if c.type == "identifier":
            name = node_text(c, source)
        elif c.type == "formal_parameters":
            params = node_text(c, source)
        elif c.type == "block":
            body = c
        elif c.type in ("type_identifier", "void_type", "integral_type",
                         "floating_point_type", "boolean_type", "generic_type",
                         "array_type"):
            return_type = node_text(c, source)

    if not name:
        return None

    return MethodInfo(
        name=name,
        params=params,
        return_type=return_type,
        node=method_node,
        body_node=body,
        start_line=method_node.start_point[0] + 1,
        end_line=method_node.end_point[0] + 1,
    )


# ── File discovery ─────────────────────────────────────────────────────────

def find_java_files(name: str, java_dir: Path = JAVA_SRC) -> list[Path]:
    """Find Java files matching a name.

    Search strategy:
      1. Exact: {name}Action.java / {name}Power.java / {name}Relic.java / {name}.java
      2. Fuzzy: any .java file whose stem contains the query (case-insensitive)
    """
    suffixes = ["Action", "Power", "Relic", "Potion", ""]
    exact_names = [f"{name}{s}.java" for s in suffixes]

    # Exact match
    for exact in exact_names:
        matches = list(java_dir.rglob(exact))
        if matches:
            return matches

    # Fuzzy match
    query_lower = name.lower()
    fuzzy = [f for f in java_dir.rglob("*.java")
             if query_lower in f.stem.lower()
             and not f.stem.startswith("Abstract")]
    return fuzzy


def find_action_file(name: str, java_dir: Path = JAVA_SRC) -> Path | None:
    """Find a specific Action Java file by Rust action name."""
    exact = f"{name}Action.java"
    matches = list(java_dir.rglob(exact))
    if matches:
        return matches[0]
    # Try without Action suffix
    matches = list(java_dir.rglob(f"{name}.java"))
    return matches[0] if matches else None


def get_method(ctx: FileContext, method_name: str) -> MethodInfo | None:
    """Get a specific method from a parsed file."""
    cls = extract_class(ctx)
    if cls and method_name in cls.methods:
        return cls.methods[method_name]
    return None
