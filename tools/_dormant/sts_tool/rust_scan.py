"""
rust_scan.py — Layer 3: Scan Rust source for parity checking.

Auto-discovers enum variants and dispatch function match arms.
Zero hardcoded name mappings — uses suffix-stripping and normalization.
"""

import re
from pathlib import Path
from dataclasses import dataclass, field

RUST_SRC = Path(r"d:\rust\sts_simulator\src")


@dataclass
class RustParity:
    """Parity status for a Java entity in Rust."""
    java_class: str
    rust_variant: str | None
    variant_exists: bool
    dispatch_fns: list[str] = field(default_factory=list)  # functions that handle this variant
    notes: list[str] = field(default_factory=list)


# ── Enum scanning ──────────────────────────────────────────────────────────

def scan_enum_variants(file_path: Path, enum_name: str) -> set[str]:
    """Extract all variants from a Rust enum definition."""
    if not file_path.exists():
        return set()
    src = file_path.read_text(errors="replace")
    
    variants = set()
    in_enum = False
    brace_depth = 0
    
    for line in src.split("\n"):
        stripped = line.strip()
        if f"enum {enum_name}" in stripped and "pub" in stripped:
            in_enum = True
            brace_depth = 0
        if in_enum:
            brace_depth += line.count("{") - line.count("}")
            if brace_depth <= 0 and in_enum and "{" in src[:src.find(stripped) + len(stripped)]:
                if brace_depth == 0 and stripped == "}":
                    break
            # Extract variant name
            v = stripped.split("//")[0].strip().rstrip(",")
            v = v.split("{")[0].split("(")[0].strip()
            if v and v[0].isupper() and v != enum_name and "enum" not in v.lower():
                variants.add(v)
    
    return variants


def scan_action_variants(rust_src: Path = RUST_SRC) -> set[str]:
    return scan_enum_variants(rust_src / "action.rs", "Action")


def scan_power_variants(rust_src: Path = RUST_SRC) -> set[str]:
    return scan_enum_variants(rust_src / "content" / "powers" / "mod.rs", "PowerId")


def scan_relic_variants(rust_src: Path = RUST_SRC) -> set[str]:
    return scan_enum_variants(rust_src / "content" / "relics" / "mod.rs", "RelicId")


# ── Dispatch function scanning ─────────────────────────────────────────────

def scan_dispatch_arms(file_path: Path, fn_prefix: str) -> dict[str, set[str]]:
    """Find all dispatch functions matching fn_prefix and their match arms.
    
    Returns: {fn_name: {variant1, variant2, ...}}
    """
    if not file_path.exists():
        return {}
    
    src = file_path.read_text(errors="replace")
    result = {}
    
    current_fn = None
    brace_depth = 0
    fn_start_depth = 0
    
    for line in src.split("\n"):
        # Detect function start
        fn_match = re.search(rf'pub\s+fn\s+({fn_prefix}\w*)\s*\(', line)
        if fn_match and current_fn is None:
            current_fn = fn_match.group(1)
            result[current_fn] = set()
            fn_start_depth = brace_depth
        
        brace_depth += line.count("{") - line.count("}")
        
        if current_fn:
            # Extract enum variant references
            for m in re.finditer(r'(?:PowerId|RelicId|CardId|Action)::(\w+)', line):
                result[current_fn].add(m.group(1))
            
            # Detect function end
            if brace_depth <= fn_start_depth and brace_depth >= 0:
                current_fn = None
    
    return result


# ── Name matching (zero hardcoding) ────────────────────────────────────────

def java_to_rust_name(java_class: str, category: str) -> str:
    """Auto-map Java class name to Rust variant name.
    
    Rules (applied in order):
      1. Strip category suffix: StrengthPower → Strength, DamageAction → Damage
      2. Normalize: spaces removed
      3. Keep as-is if no suffix matches
    """
    suffixes = {
        "power": "Power",
        "action": "Action",
        "relic": "Relic",
        "card": "",  # Cards don't have a consistent suffix
        "potion": "Potion",
    }
    suffix = suffixes.get(category, "")
    if suffix and java_class.endswith(suffix) and len(java_class) > len(suffix):
        return java_class[:-len(suffix)]
    return java_class.replace(" ", "")


def check_rust_parity(java_class: str, category: str, rust_src: Path = RUST_SRC) -> RustParity:
    """Check if a Java class has a corresponding Rust implementation."""
    rust_name = java_to_rust_name(java_class, category)
    
    # Get the appropriate variants set
    if category == "action":
        variants = scan_action_variants(rust_src)
    elif category == "power":
        variants = scan_power_variants(rust_src)
    elif category == "relic":
        variants = scan_relic_variants(rust_src)
    else:
        variants = set()
    
    exists = rust_name in variants
    
    # Check dispatch functions
    dispatch_fns = []
    if category == "power":
        dispatches = scan_dispatch_arms(
            rust_src / "content" / "powers" / "mod.rs",
            "resolve_power_"
        )
        for fn_name, arms in dispatches.items():
            if rust_name in arms:
                dispatch_fns.append(fn_name)
    
    return RustParity(
        java_class=java_class,
        rust_variant=rust_name if exists else None,
        variant_exists=exists,
        dispatch_fns=dispatch_fns,
    )


def format_rust_parity(parity: RustParity) -> str:
    """Format parity status as readable text."""
    if not parity.variant_exists:
        return f"❌ `{parity.java_class}` → no Rust variant found"
    
    status = f"✅ `{parity.java_class}` → `{parity.rust_variant}`"
    if parity.dispatch_fns:
        fns = ", ".join(f"`{f}`" for f in parity.dispatch_fns)
        status += f" (dispatched in: {fns})"
    return status
