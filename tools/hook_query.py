#!/usr/bin/env python3
"""
Hook Query Tool — focused dependency extraction for a single Java hook.

Usage:
    python hook_query.py <hook_name> [--java-dir PATH] [--rust-dir PATH]

Example:
    python hook_query.py onApplyPower
    python hook_query.py atDamageFinalGive

Output:
    hook_query_output/<hook_name>.md
"""

import sys
import re
import subprocess
from pathlib import Path
from collections import defaultdict

import tree_sitter_java as tsjava
from tree_sitter import Language, Parser

# ── Config ─────────────────────────────────────────────────────────────────

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent
DEFAULT_JAVA_DIR = Path("d:/rust/cardcrawl")
DEFAULT_RUST_DIR = PROJECT_ROOT / "src"
OUTPUT_DIR = SCRIPT_DIR / "hook_query_output"

# Known base classes whose methods define hooks (not overrides)
BASE_CLASSES = {
    "AbstractPower", "AbstractRelic", "AbstractCard", "AbstractAction",
    "AbstractMonster", "AbstractCreature", "AbstractPotion", "AbstractOrb",
    "AbstractPlayer", "AbstractRoom", "AbstractDungeon",
}

# Cache for liveness checks
_instantiation_cache: dict[str, bool] = {}

# Java class name → Rust PowerId variant (explicit exceptions only)
# Most Java classes map by stripping "Power" suffix: "VulnerablePower" → "Vulnerable"
# Only list cases where the mapping is non-obvious.
JAVA_TO_RUST_POWER: dict[str, str] = {
    "IntangiblePlayerPower": "Intangible",
    "IntangiblePower": "Intangible",
    "NoDrawPower": "NoDraw",
    "LoseStrengthPower": "LoseStrength",
    "LoseDexterityPower": "DexterityDown",
    "GainStrengthPower": "GainStrength",  # "Shackled" — may not exist in Rust yet
    "DrawReductionPower": "DrawReduction",
    "DrawPower": "DrawPower",
    "StrengthPower": "Strength",
    "DexterityPower": "Dexterity",
    "PoisonPower": "Poison",
    "VulnerablePower": "Vulnerable",
    "WeakPower": "Weak",
    "FrailPower": "Frail",
    "ArtifactPower": "Artifact",
    "RegenPower": "Regen",
    "RitualPower": "Ritual",
    "ThornsPower": "Thorns",
    "EntanglePower": "Entangle",
    "ConfusionPower": "Confusion",
    "HexPower": "Hex",
    "SlowPower": "Slow",
    "FlightPower": "Flight",
    "MalleablePower": "Malleable",
    "PlatedArmorPower": "PlatedArmor",
    "ConstrictedPower": "Constricted",
    "CurlUpPower": "CurlUp",
    "SporeCloudPower": "SporeCloud",
    "SplitPower": "Split",
    "AngryPower": "Angry",
    "AngerPower": "Anger",
    "SharpHidePower": "SharpHide",
    "ModeShiftPower": "ModeShift",
    "BeatOfDeathPower": "BeatOfDeath",
    "TimeWarpPower": "TimeWarp",
    "InvinciblePower": "Invincible",
    "CuriosityPower": "Curiosity",
    "ReactivePower": "Reactive",
    "RegrowPower": "Regrow",
    "FadingPower": "Fading",
    "ShiftingPower": "Shifting",
    "ExplosivePower": "Explosive",
    "StasisPower": "Stasis",
    "ThieveryPower": "Thievery",
    "PainfulStabsPower": "PainfulStabs",
    "MinionPower": "Minion",
    "BufferPower": "Buffer",
    "VigorPower": "Vigor",
    "MantraPower": "Mantra",
    "FocusPower": "Focus",
    "ElectroPower": "Electro",
    "PenNibPower": "PenNibPower",
    "NextTurnBlockPower": "NextTurnBlock",
    "EnergizedPower": "Energized",
    "MetallicizePower": "Metallicize",
    "DemonFormPower": "DemonForm",
    "CorruptionPower": "Corruption",
    "DoubleTapPower": "DoubleTap",
    "FeelNoPainPower": "FeelNoPain",
    "DarkEmbracePower": "DarkEmbrace",
    "RupturePower": "Rupture",
    "CombustPower": "Combust",
    "BrutalityPower": "Brutality",
    "BarricadePower": "Barricade",
    "JuggernautPower": "Juggernaut",
    "FlameBarrierPower": "FlameBarrier",
    "EvolvePower": "Evolve",
    "FireBreathingPower": "FireBreathing",
    "RagePower": "Rage",
    "BerserkPower": "Berserk",
    "NoSkillsPower": "NoSkills",
    # Powers where Rust keeps the "Power" suffix
    "SadisticPower": "SadisticPower",
    "DuplicationPower": "DuplicationPower",
    "MagnetismPower": "MagnetismPower",
    "MayhemPower": "MayhemPower",
    "PanachePower": "PanachePower",
    "TheBombPower": "TheBombPower",
}


def is_class_instantiated(class_name: str, java_dir: Path) -> bool:
    """Check if `new ClassName(` appears anywhere in the Java codebase."""
    if class_name in _instantiation_cache:
        return _instantiation_cache[class_name]
    try:
        out = subprocess.run(
            ["rg", "-F", "-l", f"new {class_name}(", str(java_dir), "-g", "*.java"],
            capture_output=True, text=True, timeout=10, encoding="utf-8",
        )
        # Exclude self-references (the class's own definition file)
        files = [f for f in out.stdout.strip().split("\n") if f.strip() and Path(f).stem != class_name]
        found = len(files) > 0
    except Exception:
        found = True  # Assume live on error
    _instantiation_cache[class_name] = found
    return found

# ── Tree-sitter setup ─────────────────────────────────────────────────────

JAVA_LANG = Language(tsjava.language())
_parser = Parser(JAVA_LANG)


def parse_java(path: Path):
    source = path.read_bytes()
    tree = _parser.parse(source)
    return source.decode("utf-8", errors="replace"), tree


def node_text(node, source: str) -> str:
    return source[node.start_byte:node.end_byte]


def find_descendants(node, type_name: str):
    results = []
    for c in node.children:
        if c.type == type_name:
            results.append(c)
        results.extend(find_descendants(c, type_name))
    return results


def get_class_name(class_node, source: str):
    for c in class_node.children:
        if c.type == "identifier":
            return node_text(c, source)
    return None


def get_superclass(class_node, source: str):
    for c in class_node.children:
        if c.type == "superclass":
            for sc in c.children:
                if sc.type == "type_identifier":
                    return node_text(sc, source)
    return None


def get_methods_named(class_node, source: str, method_name: str):
    """Find methods with a specific name in a class body."""
    body = None
    for c in class_node.children:
        if c.type == "class_body":
            body = c
            break
    if not body:
        return []

    results = []
    for member in body.children:
        if member.type == "method_declaration":
            for c in member.children:
                if c.type == "identifier" and node_text(c, source) == method_name:
                    params = ""
                    ret_type = ""
                    for c2 in member.children:
                        if c2.type == "formal_parameters":
                            params = node_text(c2, source)
                        if c2.type in ("type_identifier", "void_type", "integral_type",
                                       "floating_point_type", "boolean_type", "generic_type"):
                            ret_type = node_text(c2, source)
                    results.append({
                        "params": params,
                        "return_type": ret_type,
                        "body": node_text(member, source),
                        "start_line": member.start_point[0] + 1,
                        "end_line": member.end_point[0] + 1,
                        "side_effects": detect_side_effects(member, source),
                    })
    return results


def detect_side_effects(method_node, source: str) -> list[str]:
    """Scan a method body for side effects that affect Rust signature design."""
    effects = []
    body_text = node_text(method_node, source)

    # 1. Action queuing
    if "addToBot" in body_text:
        effects.append("QUEUES_ACTIONS(addToBot)")
    if "addToTop" in body_text:
        effects.append("QUEUES_ACTIONS(addToTop)")

    # 2. Member variable mutation (this.xxx = ...)
    for assign in find_descendants(method_node, "assignment_expression"):
        if assign.children:
            lhs = node_text(assign.children[0], source)
            if lhs.startswith("this."):
                effects.append(f"MUTATES({lhs})")

    # 3. Calls on owner object
    for inv in find_descendants(method_node, "method_invocation"):
        text = node_text(inv, source)
        if text.startswith("this.owner.") or text.startswith("owner."):
            # Extract method name
            m = re.match(r"(?:this\.)?owner\.(\w+)", text)
            if m:
                effects.append(f"CALLS_OWNER({m.group(1)})")

    # Deduplicate while preserving order
    seen = set()
    unique = []
    for e in effects:
        if e not in seen:
            seen.add(e)
            unique.append(e)
    return unique


def extract_hardcoded_checks(method_node, source: str) -> list[dict]:
    """Find hasPower/hasRelic checks in a method body — engine-side hardcoded logic."""
    checks = []
    for inv in find_descendants(method_node, "method_invocation"):
        text = node_text(inv, source)
        if ".hasPower(" in text or ".hasRelic(" in text:
            check_type = "hasPower" if ".hasPower(" in text else "hasRelic"
            # Extract string literal argument
            for arg in find_descendants(inv, "string_literal"):
                id_str = node_text(arg, source).strip('"')
                checks.append({
                    "type": check_type,
                    "id": id_str,
                    "line": inv.start_point[0] + 1,
                })
    return checks


def get_enclosing_method(node, source: str):
    curr = node.parent
    while curr:
        if curr.type == "method_declaration":
            for c in curr.children:
                if c.type == "identifier":
                    return node_text(c, source)
        curr = curr.parent
    return "<unknown>"


def get_enclosing_method_node(node):
    """Walk up AST to find the enclosing method_declaration node."""
    curr = node.parent
    while curr:
        if curr.type == "method_declaration":
            return curr
        curr = curr.parent
    return None


def get_enclosing_for_iterable(node, source: str) -> str | None:
    """Walk up AST to find enclosing enhanced_for_statement and extract its iterable."""
    curr = node.parent
    while curr:
        if curr.type == "enhanced_for_statement":
            # Structure: for (Type var : iterable) { ... }
            # The iterable is typically the last expression before the body
            for i, child in enumerate(curr.children):
                if child.type == ":" and i + 1 < len(curr.children):
                    # Next non-body child is the iterable
                    for j in range(i + 1, len(curr.children)):
                        if curr.children[j].type not in ("{", "}", "block"):
                            return node_text(curr.children[j], source).strip()
            break
        if curr.type == "method_declaration":
            break  # Don't search beyond method boundary
        curr = curr.parent
    return None


def find_invocations_of(root_node, source: str, method_name: str):
    """Find all method_invocation nodes calling a specific method name."""
    results = []
    for inv in find_descendants(root_node, "method_invocation"):
        for c in inv.children:
            if c.type == "identifier" and node_text(c, source) == method_name:
                results.append({
                    "text": node_text(inv, source),
                    "line": inv.start_point[0] + 1,
                    "enclosing": get_enclosing_method(inv, source),
                    "enclosing_node": get_enclosing_method_node(inv),
                    "iterable": get_enclosing_for_iterable(inv, source),
                })
                break
    return results


# ── Scanning ───────────────────────────────────────────────────────────────

def scan_java_for_hook(java_dir: Path, hook_name: str):
    """Scan all Java files for a hook: find overrides and call sites."""
    overrides = []      # (class_name, superclass, file_path, method_info)
    base_defs = []      # The definitions in the base class
    call_sites = []     # (file_path, class_name, method_name, line, context)

    for fpath in java_dir.rglob("*.java"):
        try:
            source, tree = parse_java(fpath)
        except Exception:
            continue

        for class_node in find_descendants(tree.root_node, "class_declaration"):
            cname = get_class_name(class_node, source)
            sclass = get_superclass(class_node, source)
            if not cname:
                continue

            rel_path = str(fpath.relative_to(java_dir))

            # Check if this class has our hook method
            methods = get_methods_named(class_node, source, hook_name)
            for m in methods:
                if cname in BASE_CLASSES:
                    base_defs.append({
                        "class": cname,
                        "file": rel_path,
                        **m,
                    })
                else:
                    overrides.append({
                        "class": cname,
                        "superclass": sclass,
                        "file": rel_path,
                        **m,
                    })

            # Check for call sites: any invocation of hook_name in any method
            invocations = find_invocations_of(class_node, source, hook_name)
            # Only exclude classes that DEFINE the hook (have it as a base method), not all BASE_CLASSES
            has_hook_def = any(m for m in methods if cname in BASE_CLASSES)
            if invocations and not has_hook_def:
                # Get surrounding context for each invocation
                lines = source.split("\n")
                # Phase 2: extract hardcoded checks per enclosing method (cache to avoid re-scan)
                _method_checks_cache: dict[str, list[dict]] = {}
                for inv in invocations:
                    line_idx = inv["line"] - 1
                    ctx_start = max(0, line_idx - 3)
                    ctx_end = min(len(lines), line_idx + 4)
                    context = "\n".join(
                        (">>> " if i == line_idx else "    ") + lines[i]
                        for i in range(ctx_start, ctx_end)
                    )
                    enc = inv["enclosing"]
                    if enc not in _method_checks_cache:
                        enc_node = inv.get("enclosing_node")
                        _method_checks_cache[enc] = extract_hardcoded_checks(enc_node, source) if enc_node else []
                    call_sites.append({
                        "class": cname,
                        "file": rel_path,
                        "line": inv["line"],
                        "call_text": inv["text"],
                        "context": context,
                        "enclosing": enc,
                        "hardcoded_checks": _method_checks_cache[enc],
                        "enclosing_node": inv.get("enclosing_node"),
                        "iterable": inv.get("iterable"),
                    })

    return base_defs, overrides, call_sites


def scan_rust_for_hook(rust_dir: Path, hook_name: str):
    """Use ripgrep to find references to the hook in Rust source."""
    # Convert Java camelCase to Rust snake_case
    snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", hook_name).lower()

    results = []
    for pattern in [snake, hook_name]:
        try:
            out = subprocess.run(
                ["rg", "-n", "--no-heading", pattern, str(rust_dir), "-g", "*.rs"],
                capture_output=True, text=True, timeout=10, encoding="utf-8",
            )
            for line in out.stdout.strip().split("\n"):
                if line.strip():
                    results.append(line)
        except Exception:
            pass

    # Deduplicate
    return list(dict.fromkeys(results))


def java_class_to_rust_power_id(java_class: str) -> str | None:
    """Map a Java power class name to its Rust PowerId variant.
    Uses explicit table first, then tries stripping 'Power' suffix."""
    if java_class in JAVA_TO_RUST_POWER:
        return JAVA_TO_RUST_POWER[java_class]
    # Fallback: strip "Power" suffix
    if java_class.endswith("Power"):
        return java_class[:-5]  # e.g. "ForcefieldPower" → "Forcefield"
    return java_class


def extract_rust_power_ids(rust_dir: Path) -> set[str]:
    """Extract all PowerId::Xxx variants from the Rust enum definition."""
    powers_mod = rust_dir / "content" / "powers" / "mod.rs"
    if not powers_mod.exists():
        return set()
    variants = set()
    in_enum = False
    for line in powers_mod.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if "enum PowerId" in stripped:
            in_enum = True
            continue
        if in_enum:
            if stripped == "}":
                break
            # Extract variant name: "Strength," or "Strength" or "// comment"
            m = re.match(r"^(\w+),?", stripped)
            if m:
                variants.add(m.group(1))
    return variants


def extract_rust_dispatch_arms(rust_dir: Path, hook_name: str) -> set[str]:
    """Find PowerId::Xxx match arms in resolve_power_<hook> functions."""
    snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", hook_name).lower()
    fn_name = f"resolve_power_{snake}"
    arms = set()
    try:
        out = subprocess.run(
            ["rg", "-n", "--no-heading", f"PowerId::", str(rust_dir / "content" / "powers"), "-g", "*.rs"],
            capture_output=True, text=True, timeout=10, encoding="utf-8",
        )
        # Find lines that are inside the dispatch function — simplified: just extract all PowerId::Xxx
        # from powers/mod.rs that appear after fn_name
        in_fn = False
        powers_mod = rust_dir / "content" / "powers" / "mod.rs"
        if powers_mod.exists():
            for line in powers_mod.read_text(encoding="utf-8").splitlines():
                if fn_name in line and "fn " in line:
                    in_fn = True
                    continue
                if in_fn:
                    if line.startswith("}") or (line.startswith("pub ") and "fn " in line):
                        break
                    for m in re.finditer(r"PowerId::(\w+)", line):
                        arms.add(m.group(1))
    except Exception:
        pass
    return arms


def build_parity_table(overrides, rust_dir: Path, hook_name: str, java_dir: Path) -> list[dict]:
    """Build a Java↔Rust parity mapping for this hook's overrides."""
    rust_power_ids = extract_rust_power_ids(rust_dir)
    rust_arms = extract_rust_dispatch_arms(rust_dir, hook_name)
    
    rows = []
    for ov in overrides:
        java_class = ov["class"]
        live = is_class_instantiated(java_class, java_dir)
        rust_id = java_class_to_rust_power_id(java_class)
        
        if not live:
            status = "⚠️ DEAD"
        elif rust_id and rust_id in rust_arms:
            status = "✅ IMPLEMENTED"
        elif rust_id and rust_id in rust_power_ids:
            status = "❌ MISSING (PowerId exists)"
        elif rust_id:
            status = "❌ MISSING (no PowerId)"
        else:
            status = "❓ UNMAPPED"
        
        rows.append({
            "java_class": java_class,
            "rust_id": rust_id or "?",
            "status": status,
        })
    return rows


# ── Output ─────────────────────────────────────────────────────────────────

def write_report(hook_name: str, base_defs, overrides, call_sites, rust_refs, output_path: Path,
                 java_dir: Path = DEFAULT_JAVA_DIR, rust_dir: Path = DEFAULT_RUST_DIR):
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(f"# Hook Query: `{hook_name}`\n\n")

        # 1. Base class signature
        f.write(f"## 1. Base Class Definition ({len(base_defs)} signatures)\n\n")
        if base_defs:
            for bd in base_defs:
                f.write(f"**Class**: `{bd['class']}` — `{bd['file']}` L{bd['start_line']}\n\n")
                f.write("```java\n")
                f.write(bd["body"])
                f.write("\n```\n\n")
        else:
            f.write("*Not found in any known base class.*\n\n")

        # 2. Engine call sites
        f.write(f"## 2. Engine Call Sites ({len(call_sites)})\n\n")
        if call_sites:
            # Group by class + file + enclosing method
            grouped = defaultdict(list)
            for cs in call_sites:
                key = (cs["class"], cs["file"], cs["enclosing"])
                grouped[key].append(cs)

            for key in sorted(grouped.keys()):
                cls_name, file_path, enclosing = key
                invs = grouped[key]
                f.write(f"### `{cls_name}.{enclosing}()`\n\n")
                f.write(f"File: `{file_path}`\n\n")
                f.write("```java\n")
                for inv in sorted(invs, key=lambda x: x["line"]):
                    f.write(f"// --- Line {inv['line']} ---\n")
                    f.write(inv["context"])
                    f.write("\n")
                f.write("```\n\n")
                # Phase 4: show iterable if detected
                iterables = set(inv.get("iterable") for inv in invs if inv.get("iterable"))
                if iterables:
                    for it in sorted(iterables):
                        f.write(f"**Iterates**: `{it}` (ordered — sensitive to iteration order)\n\n")
                # Phase 2: show hardcoded checks in this enclosing method
                checks = invs[0].get("hardcoded_checks", [])
                if checks:
                    f.write("**Hardcoded checks in this method:**\n\n")
                    for chk in checks:
                        f.write(f"- L{chk['line']}: `{chk['type']}(\"{chk['id']}\")`\n")
                    f.write("\n")
        else:
            f.write("*No call sites found outside base classes.*\n\n")

        # 3. Overrides
        f.write(f"## 3. Subclass Overrides ({len(overrides)})\n\n")
        if overrides:
            # Summary table
            f.write("| Class | Superclass | File | Lines | Status | Side Effects |\n")
            f.write("|-------|-----------|------|-------|--------|-------------|\n")
            for ov in sorted(overrides, key=lambda x: x["class"]):
                live = is_class_instantiated(ov["class"], java_dir)
                tag = "✅" if live else "⚠️ DEAD"
                effects = ov.get("side_effects", [])
                effects_str = ", ".join(effects) if effects else "pure"
                f.write(f"| {ov['class']} | {ov.get('superclass', '?')} | `{ov['file']}` | {ov['start_line']}-{ov['end_line']} | {tag} | {effects_str} |\n")
            f.write("\n")

            # Full bodies
            for ov in sorted(overrides, key=lambda x: x["class"]):
                params = ov.get("params", "").replace("\n", " ").strip()
                effects = ov.get("side_effects", [])
                effects_tag = " ⚠️ " + ", ".join(effects) if effects else ""
                f.write(f"### {ov['class']} `({params})`{effects_tag}\n\n")
                f.write(f"File: `{ov['file']}` L{ov['start_line']}-{ov['end_line']}\n\n")
                f.write("```java\n")
                f.write(ov["body"])
                f.write("\n```\n\n")
        else:
            f.write("*No subclass overrides found.*\n\n")

        # 4. Rust references
        f.write(f"## 4. Rust Current Status ({len(rust_refs)} refs)\n\n")
        if rust_refs:
            for ref in rust_refs:
                f.write(f"- `{ref}`\n")
            f.write("\n")
        else:
            snake = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", hook_name).lower()
            f.write(f"*No references to `{snake}` or `{hook_name}` found in Rust source.*\n\n")

        # 5. Phase 3: Parity table
        if overrides:
            parity = build_parity_table(overrides, rust_dir, hook_name, java_dir)
            f.write(f"## 5. Parity Status\n\n")
            f.write("| Java Class | Rust PowerId | Status |\n")
            f.write("|------------|-------------|--------|\n")
            for row in parity:
                f.write(f"| {row['java_class']} | {row['rust_id']} | {row['status']} |\n")
            f.write("\n")

    return output_path


# ── Main ───────────────────────────────────────────────────────────────────

def main():
    if len(sys.argv) < 2:
        print("Usage: python hook_query.py <hook_name> [--java-dir PATH] [--rust-dir PATH]")
        print("Example: python hook_query.py onApplyPower")
        sys.exit(1)

    hook_name = sys.argv[1]

    # Parse optional args
    java_dir = DEFAULT_JAVA_DIR
    rust_dir = DEFAULT_RUST_DIR
    args = sys.argv[2:]
    for i, arg in enumerate(args):
        if arg == "--java-dir" and i + 1 < len(args):
            java_dir = Path(args[i + 1])
        elif arg == "--rust-dir" and i + 1 < len(args):
            rust_dir = Path(args[i + 1])

    print(f"Hook:     {hook_name}")
    print(f"Java dir: {java_dir}")
    print(f"Rust dir: {rust_dir}")
    print()

    # Scan
    print("Scanning Java (tree-sitter)...")
    base_defs, overrides, call_sites = scan_java_for_hook(java_dir, hook_name)
    print(f"  Base defs: {len(base_defs)}")
    print(f"  Overrides: {len(overrides)}")
    print(f"  Call sites: {len(call_sites)}")

    print("Scanning Rust (ripgrep)...")
    rust_refs = scan_rust_for_hook(rust_dir, hook_name)
    print(f"  Rust refs: {len(rust_refs)}")

    # Write
    output_path = OUTPUT_DIR / f"{hook_name}.md"
    write_report(hook_name, base_defs, overrides, call_sites, rust_refs, output_path, java_dir, rust_dir)
    print(f"\n✅ Report: {output_path}")


if __name__ == "__main__":
    main()
