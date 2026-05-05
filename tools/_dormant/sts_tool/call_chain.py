"""
call_chain.py — Layer 2: Cross-file call chain resolution.

Given a Java method, extract all outgoing dependencies:
  - new XxxAction(...) → creates
  - p.hookName() in for-each loop → virtual dispatch
  - hasPower("X") / hasRelic("X") → entity checks
  - addToTop / addToBot → queue insertion mode

Then resolve virtual dispatch targets across the codebase using
ripgrep (fast file filtering) + tree-sitter (precise AST matching).
"""

import subprocess
from pathlib import Path
from dataclasses import dataclass, field

from .java_parser import (
    Node, node_text, find_descendants, find_child, find_children,
    parse_file, extract_class, JAVA_SRC,
)


@dataclass
class ActionCreation:
    """A `new XxxAction(...)` found in method body."""
    class_name: str
    full_text: str
    line: int
    insertion: str  # "addToTop" | "addToBot" | "unknown"


@dataclass
class VirtualDispatch:
    """A virtual method call on a collection (e.g., for (p : powers) p.hookName())."""
    method_name: str
    iterable: str          # e.g., "source.powers", "player.relics"
    receiver_type: str     # e.g., "AbstractPower", "AbstractRelic"
    line: int
    overrides: list[dict] = field(default_factory=list)  # [{class, file, live}, ...]


@dataclass
class EntityCheck:
    """A hasPower("X") or hasRelic("X") check."""
    check_type: str    # "hasPower" | "hasRelic" | "hasPower" | "hasOrb"
    entity_id: str     # The string argument, e.g., "Artifact"
    line: int


@dataclass 
class MethodCall:
    """A direct method call."""
    receiver: str     # e.g., "this", "target", "AbstractDungeon"
    method_name: str
    full_text: str
    line: int


@dataclass
class CallChainResult:
    """Complete analysis of a method's outgoing dependencies."""
    class_name: str
    method_name: str
    file_path: str
    creates: list[ActionCreation] = field(default_factory=list)
    virtual_dispatches: list[VirtualDispatch] = field(default_factory=list)
    entity_checks: list[EntityCheck] = field(default_factory=list)
    significant_calls: list[MethodCall] = field(default_factory=list)
    guards: list[str] = field(default_factory=list)


# ── Method body analysis ───────────────────────────────────────────────────

def analyze_method(method_body: Node, source: str) -> CallChainResult:
    """Analyze a method body for all outgoing dependencies."""
    result = CallChainResult(class_name="", method_name="", file_path="")

    # 1. Find all object creations (new Xxx(...))
    for creation in find_descendants(method_body, "object_creation_expression"):
        type_node = find_child(creation, "type_identifier")
        if not type_node:
            continue
        type_name = node_text(type_node, source)
        # Skip presentation-only effects
        _PRES_EFFECTS = {
            "FlashAtkImgEffect", "PowerBuffEffect", "PowerDebuffEffect",
            "DamageNumberEffect", "FireBurstParticleEffect", "TextAboveCreatureAction",
            "SFXAction", "VFXAction", "WaitAction", "TalkAction",
        }
        if type_name in _PRES_EFFECTS:
            continue
        if not (type_name.endswith("Action") or type_name.endswith("Power")):
            continue

        full_text = node_text(creation, source).replace('\n', ' ')
        if len(full_text) > 150:
            full_text = full_text[:147] + "..."

        # Detect insertion mode by looking at enclosing method invocation
        insertion = _detect_insertion_mode(creation, source)

        result.creates.append(ActionCreation(
            class_name=type_name,
            full_text=full_text,
            line=creation.start_point[0] + 1,
            insertion=insertion,
        ))

    # 2. Find virtual dispatch in for-each loops
    for for_each in find_descendants(method_body, "enhanced_for_statement"):
        _analyze_for_each_dispatch(for_each, source, result)

    # 3. Find entity checks
    for invocation in find_descendants(method_body, "method_invocation"):
        text = node_text(invocation, source)
        for check_fn in ("hasPower", "hasRelic", "hasPotion", "hasOrb"):
            if f".{check_fn}(" in text or text.startswith(f"{check_fn}("):
                # Extract string argument
                args = find_child(invocation, "argument_list")
                if args:
                    str_lit = find_child(args, "string_literal")
                    if str_lit:
                        entity_id = node_text(str_lit, source).strip('"')
                        result.entity_checks.append(EntityCheck(
                            check_type=check_fn,
                            entity_id=entity_id,
                            line=invocation.start_point[0] + 1,
                        ))

    # 4. Detect guards (early returns at method start)
    _detect_guards(method_body, source, result)

    # 5. Find significant direct calls
    _find_significant_calls(method_body, source, result)

    return result


def _detect_insertion_mode(creation_node: Node, source: str) -> str:
    """Walk up the AST to find if this creation is inside addToTop() or addToBot()."""
    current = creation_node.parent
    while current:
        if current.type == "method_invocation":
            text = node_text(current, source)
            if "addToTop(" in text or ".addToTop(" in text:
                return "addToTop"
            if "addToBot(" in text or ".addToBot(" in text:
                return "addToBot"
            if "addToBottom(" in text or ".addToBottom(" in text:
                return "addToBot"
        if current.type in ("method_declaration", "class_body"):
            break
        current = current.parent
    return "unknown"


def _analyze_for_each_dispatch(for_each: Node, source: str, result: CallChainResult):
    """Detect virtual dispatch patterns in for-each loops.
    
    Pattern: for (SomeType var : collection) { var.method(...); }
    """
    # Extract loop variable and type
    var_type = None
    var_name = None
    iterable_text = None

    children = list(for_each.children)
    for i, c in enumerate(children):
        if c.type == "type_identifier":
            var_type = node_text(c, source)
        elif c.type == "identifier" and var_type and var_name is None:
            var_name = node_text(c, source)
        elif c.type == ":" and i + 1 < len(children):
            # Next non-trivial child is the iterable
            for j in range(i + 1, len(children)):
                next_c = children[j]
                if next_c.type not in ("{", "}", "block", ";"):
                    iterable_text = node_text(next_c, source).strip()
                    break

    if not var_name or not iterable_text:
        return

    # Find method invocations on the loop variable inside the body
    body = find_child(for_each, "block")
    if not body:
        return

    # Methods that are noise (presentation/utility, not game logic)
    _TRIVIAL_METHODS = {
        "flash", "flashWithoutSound", "updateDescription", "equals",
        "hashCode", "toString", "compareTo", "sort", "add", "remove",
        "size", "get", "iterator", "hasNext", "next", "clear",
        "contains", "isEmpty", "indexOf", "set",
        "useFastShakeAnimation", "render", "update",
    }

    seen_dispatches = set()  # Deduplicate by method name

    for invocation in find_descendants(body, "method_invocation"):
        inv_text = node_text(invocation, source)
        # Check if the invocation starts with our loop variable
        if inv_text.startswith(f"{var_name}."):
            # Extract method name
            method_name = None
            for c in invocation.children:
                if c.type == "identifier":
                    candidate = node_text(c, source)
                    if candidate != var_name:
                        method_name = candidate
                        break
            if method_name and method_name not in _TRIVIAL_METHODS and method_name not in seen_dispatches:
                seen_dispatches.add(method_name)
                result.virtual_dispatches.append(VirtualDispatch(
                    method_name=method_name,
                    iterable=iterable_text,
                    receiver_type=var_type or "?",
                    line=invocation.start_point[0] + 1,
                ))


def _detect_guards(body: Node, source: str, result: CallChainResult):
    """Detect early-return guards at the start of a method body."""
    for c in body.children:
        if c.type == "if_statement":
            # Check if consequence contains a return statement
            has_return = bool(find_descendants(c, "return_statement"))
            if has_return:
                cond = find_child(c, "parenthesized_expression")
                if cond:
                    result.guards.append(node_text(cond, source))
            # Only check first few statements as guards
            if len(result.guards) >= 3:
                break
        elif c.type not in ("{", "}", "line_comment", "block_comment"):
            break  # Stop at first non-if statement


def _find_significant_calls(body: Node, source: str, result: CallChainResult):
    """Find significant method calls (state mutations, not just getters)."""
    significant_methods = {
        "stackPower", "onInitialApplication", "onRemove", "reducePower",
        "heal", "damage", "loseHp", "addBlock", "loseBlock",
        "drawCards", "moveToDiscard", "moveToExhaustPile",
        "flash", "triggerMarks",
    }
    for invocation in find_descendants(body, "method_invocation"):
        for c in invocation.children:
            if c.type == "identifier":
                method = node_text(c, source)
                if method in significant_methods:
                    full = node_text(invocation, source).replace('\n', ' ')
                    if len(full) > 120:
                        full = full[:117] + "..."
                    # Infer receiver
                    receiver = ""
                    for r in invocation.children:
                        if r.type in ("identifier", "field_access", "this"):
                            candidate = node_text(r, source)
                            if candidate != method:
                                receiver = candidate
                                break
                    result.significant_calls.append(MethodCall(
                        receiver=receiver,
                        method_name=method,
                        full_text=full,
                        line=invocation.start_point[0] + 1,
                    ))
                break


# ── Cross-file override resolution ─────────────────────────────────────────

def resolve_overrides(method_name: str, java_dir: Path = JAVA_SRC) -> list[dict]:
    """Find all classes that override a given method name.
    
    Uses ripgrep for fast file filtering, then tree-sitter for precise matching.
    Returns: [{class_name, superclass, file, start_line, end_line, live}, ...]
    """
    # Step 1: ripgrep to find candidate files
    try:
        rg = subprocess.run(
            ["rg", "-l", "--no-heading", f"\\b{method_name}\\b", str(java_dir), "-g", "*.java"],
            capture_output=True, text=True, timeout=10, encoding="utf-8",
        )
        candidates = [Path(f.strip()) for f in rg.stdout.strip().split("\n") if f.strip()]
    except Exception:
        candidates = list(java_dir.rglob("*.java"))

    # Step 2: tree-sitter parse each candidate to confirm the override
    overrides = []
    for fpath in candidates:
        try:
            ctx = parse_file(fpath)
            cls = extract_class(ctx)
            if not cls:
                continue
            # Skip abstract base classes (they define hooks, not override them)
            if cls.name.startswith("Abstract"):
                continue
            if method_name in cls.methods:
                mi = cls.methods[method_name]
                overrides.append({
                    "class_name": cls.name,
                    "superclass": cls.superclass,
                    "category": cls.category,
                    "file": str(fpath.relative_to(java_dir)),
                    "start_line": mi.start_line,
                    "end_line": mi.end_line,
                    "string_id": cls.string_id,
                })
        except Exception:
            continue

    return overrides


def check_instantiated(class_name: str, java_dir: Path = JAVA_SRC) -> bool:
    """Check if `new ClassName(` appears anywhere (i.e., the class is live/used)."""
    try:
        rg = subprocess.run(
            ["rg", "-F", "-l", f"new {class_name}(", str(java_dir), "-g", "*.java"],
            capture_output=True, text=True, timeout=10, encoding="utf-8",
        )
        files = [f for f in rg.stdout.strip().split("\n")
                 if f.strip() and Path(f.strip()).stem != class_name]
        return len(files) > 0
    except Exception:
        return True  # Assume live on error


# ── Call chain formatting ──────────────────────────────────────────────────

def format_call_chain(result: CallChainResult, java_dir: Path = JAVA_SRC) -> str:
    """Format a CallChainResult as a tree-style text report."""
    lines = []
    header = f"{result.class_name}.{result.method_name}()"
    lines.append(header)

    # Guards
    for guard in result.guards:
        lines.append(f"  ├─ guard: {guard}")

    # Entity checks
    for check in result.entity_checks:
        lines.append(f"  ├─ checks: {check.check_type}(\"{check.entity_id}\") L{check.line}")

    # Creates
    for cr in result.creates:
        ins = f" [{cr.insertion}]" if cr.insertion != "unknown" else ""
        lines.append(f"  ├─ creates: {cr.class_name}{ins} L{cr.line}")

    # Virtual dispatches (with override resolution)
    for vd in result.virtual_dispatches:
        lines.append(f"  ├─ iterates: {vd.iterable} → {vd.receiver_type}.{vd.method_name}()")
        # Resolve overrides
        overrides = resolve_overrides(vd.method_name, java_dir)
        live_overrides = []
        dead_overrides = []
        for ov in overrides:
            is_live = check_instantiated(ov["class_name"], java_dir)
            if is_live:
                live_overrides.append(ov)
            else:
                dead_overrides.append(ov)
        for ov in live_overrides[:10]:
            lines.append(f"  │   ├─ {ov['class_name']} ({ov['category']}) ✅ LIVE")
        if len(live_overrides) > 10:
            lines.append(f"  │   ├─ ... +{len(live_overrides) - 10} more")
        for ov in dead_overrides[:3]:
            lines.append(f"  │   ├─ {ov['class_name']} ⚠️ DEAD")
        if dead_overrides and len(dead_overrides) > 3:
            lines.append(f"  │   └─ ... +{len(dead_overrides) - 3} more dead")
        vd.overrides = live_overrides  # Store for later use

    # Significant calls
    for call in result.significant_calls:
        lines.append(f"  ├─ calls: {call.full_text} L{call.line}")

    return "\n".join(lines)
