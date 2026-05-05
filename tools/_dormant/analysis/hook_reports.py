from __future__ import annotations

from pathlib import Path

try:
    from tools.analysis.cache_builder import load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, ensure_dir
except ImportError:
    from analysis.cache_builder import load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, ensure_dir  # type: ignore


JAVA_TO_RUST_POWER = {
    "IntangiblePlayerPower": "Intangible",
    "IntangiblePower": "Intangible",
    "NoDrawPower": "NoDraw",
    "LoseStrengthPower": "LoseStrength",
    "LoseDexterityPower": "DexterityDown",
    "GainStrengthPower": "GainStrength",
    "DrawReductionPower": "DrawReduction",
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
    "PainfulStabsPower": "PainfulStabs",
    "MinionPower": "Minion",
    "BufferPower": "Buffer",
    "VigorPower": "Vigor",
    "PenNibPower": "PenNibPower",
    "NextTurnBlockPower": "NextTurnBlock",
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
}


def java_class_to_rust_power(java_class: str) -> str:
    if java_class in JAVA_TO_RUST_POWER:
        return JAVA_TO_RUST_POWER[java_class]
    if java_class.endswith("Power"):
        return java_class[:-5]
    return java_class


def java_class_to_rust_variant(java_class: str, category: str) -> str:
    if category == "power":
        return java_class_to_rust_power(java_class)
    if category == "relic" and java_class.endswith("Relic"):
        return java_class[:-5]
    if category == "action" and java_class.endswith("Action"):
        return java_class[:-6]
    return java_class


def render_hook_report(
    hook_name: str,
    output_path: Path,
    cache_dir: Path = ANALYSIS_CACHE_DIR,
) -> Path:
    cache = load_cache(cache_dir)
    hooks = cache["java_hooks"]["hooks"]
    callsites = cache["java_callsites"]["callsites"]
    rust_dispatch = cache["rust_dispatch"]
    entry = hooks.get(hook_name, {"base_definitions": [], "overrides": []})
    base_defs = entry["base_definitions"]
    overrides = entry["overrides"]
    hook_callsites = callsites.get(hook_name, [])
    power_dispatch = rust_dispatch.get("power_dispatch", {})
    relic_dispatch = rust_dispatch.get("relic_dispatch", {})
    power_variants = set(rust_dispatch.get("power_variants", []))
    relic_variants = set(rust_dispatch.get("relic_variants", []))

    lines: list[str] = [f"# Hook Query: `{hook_name}`", ""]
    lines.append(f"## 1. Base Class Definition ({len(base_defs)} signatures)")
    lines.append("")
    if base_defs:
        for bd in base_defs:
            lines.append(f"**Class**: `{bd['class']}` — `{bd['file']}` L{bd['start_line']}")
            lines.append("")
            lines.append("```java")
            lines.append(bd["body"])
            lines.append("```")
            lines.append("")
    else:
        lines.append("*Not found in any known base class.*")
        lines.append("")

    lines.append(f"## 2. Engine Call Sites ({len(hook_callsites)})")
    lines.append("")
    if hook_callsites:
        grouped: dict[tuple[str, str, str], list[dict]] = {}
        for cs in hook_callsites:
            key = (cs["class"], cs["file"], cs["enclosing"])
            grouped.setdefault(key, []).append(cs)
        for (cls_name, file_path, enclosing), invs in sorted(grouped.items()):
            lines.append(f"### `{cls_name}.{enclosing}()`")
            lines.append("")
            lines.append(f"File: `{file_path}`")
            lines.append("")
            lines.append("```java")
            for inv in sorted(invs, key=lambda item: item["line"]):
                lines.append(f"// --- Line {inv['line']} ---")
                lines.append(inv["context"])
            lines.append("```")
            lines.append("")
            iterables = sorted({inv.get("iterable") for inv in invs if inv.get("iterable")})
            for iterable in iterables:
                lines.append(f"**Iterates**: `{iterable}` (ordered — sensitive to iteration order)")
                lines.append("")
            checks = invs[0].get("hardcoded_checks", [])
            if checks:
                lines.append("**Hardcoded checks in this method:**")
                lines.append("")
                for check in checks:
                    lines.append(f"- L{check['line']}: `{check['type']}(\"{check['id']}\")`")
                lines.append("")
    else:
        lines.append("*No call sites found.*")
        lines.append("")

    lines.append(f"## 3. Subclass Overrides ({len(overrides)})")
    lines.append("")
    if overrides:
        lines.append("| Class | Superclass | File | Lines | Status | Side Effects |")
        lines.append("|-------|-----------|------|-------|--------|-------------|")
        for ov in sorted(overrides, key=lambda item: item["class"]):
            tag = "✅" if ov.get("live", True) else "⚠️ DEAD"
            effects = ", ".join(ov.get("side_effects", [])) or "pure"
            lines.append(
                f"| {ov['class']} | {ov.get('superclass', '?')} | `{ov['file']}` | "
                f"{ov['start_line']}-{ov['end_line']} | {tag} | {effects} |"
            )
        lines.append("")
    else:
        lines.append("*No subclass overrides found.*")
        lines.append("")

    lines.append("## 4. Rust Parity")
    lines.append("")
    if overrides:
        lines.append("| Java Class | Rust PowerId | Dispatch | Status |")
        lines.append("|------------|--------------|----------|--------|")
        for ov in sorted(overrides, key=lambda item: item["class"]):
            rust_id = java_class_to_rust_variant(ov["class"], ov.get("category", "power"))
            if ov.get("category") == "relic":
                dispatch_source = relic_dispatch
                variant_set = relic_variants
            else:
                dispatch_source = power_dispatch
                variant_set = power_variants
            dispatch_fns = sorted(
                fn for fn, arms in dispatch_source.items() if rust_id in arms
            )
            if not ov.get("live", True):
                status = "⚠️ DEAD"
            elif dispatch_fns:
                status = "✅ IMPLEMENTED"
            elif rust_id in variant_set:
                status = "❌ MISSING (variant exists)"
            else:
                status = "❌ MISSING (no variant)"
            lines.append(
                f"| {ov['class']} | {rust_id} | "
                f"{', '.join(dispatch_fns) if dispatch_fns else '-'} | {status} |"
            )
        lines.append("")
    else:
        lines.append("*No override classes to parity-check.*")
        lines.append("")

    ensure_dir(output_path.parent)
    output_path.write_text("\n".join(lines), encoding="utf-8")
    return output_path
