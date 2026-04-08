from __future__ import annotations

import json
from pathlib import Path

try:
    from tools.analysis.cache_builder import load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, FAMILY_AUDIT_DIR, ensure_dir, now_iso
    from tools.sts_tool.call_chain import analyze_method, format_call_chain
    from tools.sts_tool.java_parser import JAVA_SRC, extract_class, find_java_files, parse_file
    from tools.sts_tool.rust_scan import RUST_SRC
except ImportError:
    from analysis.cache_builder import load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, FAMILY_AUDIT_DIR, ensure_dir, now_iso  # type: ignore
    from sts_tool.call_chain import analyze_method, format_call_chain  # type: ignore
    from sts_tool.java_parser import JAVA_SRC, extract_class, find_java_files, parse_file  # type: ignore
    from sts_tool.rust_scan import RUST_SRC  # type: ignore


FAMILY_CONFIG = {
    "exhaust": {
        "description": "Exhaust card movement and on_exhaust hook fan-out.",
        "java_targets": {
            "TrueGrit": ["use"],
            "ExhaustAction": ["update"],
            "ExhaustSpecificCardAction": ["update"],
            "ExhaustAllNonAttackAction": ["update"],
            "DarkEmbracePower": ["onExhaust"],
            "FeelNoPainPower": ["onExhaust"],
            "CorruptionPower": ["onUseCard"],
        },
        "hook_names": ["onExhaust"],
        "rust_entries": [
            "src/engine/action_handlers/cards.rs",
            "src/engine/action_handlers/damage.rs",
            "src/content/powers/mod.rs",
            "src/content/relics/hooks.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "ExhaustAction variants that bypass the shared exhaust hook pipeline.",
            "Corruption-triggered exhaust timing missing Dark Embrace/Feel No Pain fan-out.",
            "Random exhaust actions losing relic/power ordering.",
        ],
    },
    "guardian": {
        "description": "Guardian internal threshold state, Mode Shift lifecycle, and form changes.",
        "java_targets": {
            "TheGuardian": ["damage", "changeState", "takeTurn", "getMove"],
            "ModeShiftPower": ["onAttacked", "atStartOfTurn"],
        },
        "hook_names": ["onAttacked", "atStartOfTurn"],
        "rust_entries": [
            "src/content/monsters/exordium/the_guardian.rs",
            "src/diff/state_sync.rs",
            "src/cli/live_comm.rs",
        ],
        "internal_state": ["GuardianThreshold"],
        "parity_risks": [
            "Live snapshot rebuilds dropping internal GuardianThreshold state.",
            "Mode Shift amount restored visually but internal threshold not preserved.",
            "Form change timing affecting hp/block/hand-size parity in the same turn.",
        ],
    },
    "vulnerable": {
        "description": "Vulnerable damage multiplier family including Paper Frog and Odd Mushroom.",
        "java_targets": {
            "VulnerablePower": ["atDamageReceive", "atDamageFinalReceive"],
            "PaperFrog": ["onEquip"],
            "OddMushroom": ["onEquip"],
        },
        "hook_names": ["atDamageReceive", "atDamageFinalReceive"],
        "rust_entries": [
            "src/content/powers/core/vulnerable.rs",
            "src/content/relics/hooks.rs",
            "src/engine/action_handlers/damage.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Player vs monster vulnerable multiplier split missing relic overrides.",
            "Rounding drift from applying multiplier at the wrong stage.",
            "Paper Frog/Odd Mushroom modifier path not flowing through the shared damage pipeline.",
        ],
    },
}


def _gather_java_targets(java_targets: dict[str, list[str]], java_dir: Path) -> list[dict]:
    records: list[dict] = []
    for class_name, methods in java_targets.items():
        for fpath in find_java_files(class_name, java_dir):
            ctx = parse_file(fpath)
            cls = extract_class(ctx)
            if not cls:
                continue
            for method_name in methods:
                method = cls.methods.get(method_name)
                if not method or not method.body_node:
                    continue
                chain = analyze_method(method.body_node, ctx.source)
                chain.class_name = cls.name
                chain.method_name = method_name
                chain.file_path = str(fpath.relative_to(java_dir))
                records.append(
                    {
                        "class_name": cls.name,
                        "category": cls.category,
                        "file": str(fpath.relative_to(java_dir)),
                        "method": method_name,
                        "call_chain": {
                            "creates": [creation.__dict__ for creation in chain.creates],
                            "virtual_dispatches": [
                                {
                                    "method_name": vd.method_name,
                                    "iterable": vd.iterable,
                                    "receiver_type": vd.receiver_type,
                                    "line": vd.line,
                                }
                                for vd in chain.virtual_dispatches
                            ],
                            "entity_checks": [check.__dict__ for check in chain.entity_checks],
                            "significant_calls": [call.__dict__ for call in chain.significant_calls],
                            "guards": chain.guards,
                            "formatted": format_call_chain(chain, java_dir),
                        },
                    }
                )
    return records


def build_family_audit(
    family: str,
    cache_dir: Path = ANALYSIS_CACHE_DIR,
    java_dir: Path = JAVA_SRC,
    rust_dir: Path = RUST_SRC,
) -> tuple[Path, Path]:
    if family not in FAMILY_CONFIG:
        raise SystemExit(f"Unknown family '{family}'. Expected one of: {', '.join(sorted(FAMILY_CONFIG))}")

    cache = load_cache(cache_dir)
    config = FAMILY_CONFIG[family]
    hook_data = cache["java_hooks"]["hooks"]
    callsites = cache["java_callsites"]["callsites"]
    rust_dispatch = cache["rust_dispatch"]

    audit = {
        "generated_at": now_iso(),
        "family": family,
        "description": config["description"],
        "hook_names": config["hook_names"],
        "java_targets": _gather_java_targets(config["java_targets"], java_dir),
        "hooks": {
            hook_name: {
                "base_definitions": hook_data.get(hook_name, {}).get("base_definitions", []),
                "overrides": hook_data.get(hook_name, {}).get("overrides", []),
                "callsites": callsites.get(hook_name, []),
            }
            for hook_name in config["hook_names"]
        },
        "rust": {
            "entries": config["rust_entries"],
            "power_dispatch": rust_dispatch.get("power_dispatch", {}),
            "relic_dispatch": rust_dispatch.get("relic_dispatch", {}),
        },
        "internal_state_dependencies": config["internal_state"],
        "known_parity_risks": config["parity_risks"],
    }

    out_dir = ensure_dir(FAMILY_AUDIT_DIR)
    json_path = out_dir / f"{family}.json"
    md_path = out_dir / f"{family}.md"
    json_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False), encoding="utf-8")

    lines = [f"# Family Audit: `{family}`", "", config["description"], ""]
    lines.append("## Java Targets")
    lines.append("")
    for target in audit["java_targets"]:
        lines.append(f"### `{target['class_name']}.{target['method']}()`")
        lines.append("")
        lines.append(f"File: `{target['file']}`")
        lines.append("")
        lines.append("```")
        lines.append(target["call_chain"]["formatted"])
        lines.append("```")
        lines.append("")
    lines.append("## Hooks")
    lines.append("")
    for hook_name, payload in audit["hooks"].items():
        lines.append(
            f"- `{hook_name}`: {len(payload['base_definitions'])} base definitions, "
            f"{len(payload['overrides'])} overrides, {len(payload['callsites'])} callsites"
        )
    lines.append("")
    lines.append("## Rust Entry Points")
    lines.append("")
    for entry in config["rust_entries"]:
        lines.append(f"- `{entry}`")
    lines.append("")
    lines.append("## Internal-only State")
    lines.append("")
    if config["internal_state"]:
        for item in config["internal_state"]:
            lines.append(f"- `{item}`")
    else:
        lines.append("- None")
    lines.append("")
    lines.append("## Known Parity Risks")
    lines.append("")
    for risk in config["parity_risks"]:
        lines.append(f"- {risk}")
    lines.append("")
    md_path.write_text("\n".join(lines), encoding="utf-8")
    return json_path, md_path
