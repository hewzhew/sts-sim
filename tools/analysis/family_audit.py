from __future__ import annotations

import json
from pathlib import Path

try:
    from tools.analysis.cache_builder import load_cache
    from tools.analysis.common import ANALYSIS_CACHE_DIR, FAMILY_AUDIT_DIR, ensure_dir, normalize_alias, now_iso
except ImportError:
    from analysis.cache_builder import load_cache  # type: ignore
    from analysis.common import ANALYSIS_CACHE_DIR, FAMILY_AUDIT_DIR, ensure_dir, normalize_alias, now_iso  # type: ignore


FAMILY_CONFIG = {
    "exhaust": {
        "description": "Exhaust card movement and on_exhaust hook fan-out.",
        "targets": {
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
        "targets": {
            "TheGuardian": ["damage", "changeState", "takeTurn", "getMove"],
            "ModeShiftPower": ["onAttacked", "atStartOfTurn"],
        },
        "hook_names": ["onAttacked", "atStartOfTurn"],
        "rust_entries": [
            "src/content/monsters/exordium/the_guardian.rs",
            "src/diff/state_sync/mod.rs",
            "src/cli/live_comm/mod.rs",
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
        "targets": {
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
    "apply_power": {
        "description": "ApplyPowerAction and power application fan-out.",
        "targets": {
            "ApplyPowerAction": ["update"],
        },
        "hook_names": ["onApplyPower", "onReceivePower", "onApplyPowerStacks"],
        "rust_entries": [
            "src/engine/action_handlers/powers.rs",
            "src/content/powers/mod.rs",
            "src/content/relics/hooks.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Artifact / debuff gate drift.",
            "Sentinel amount normalization drift during application.",
            "applyPower fan-out ordering missing relic/power hooks.",
        ],
    },
    "remove_power": {
        "description": "RemoveSpecificPowerAction, RemoveDebuffsAction, and power teardown.",
        "targets": {
            "RemoveSpecificPowerAction": ["update"],
            "RemoveDebuffsAction": ["update"],
        },
        "hook_names": ["onRemove"],
        "rust_entries": [
            "src/engine/action_handlers/powers.rs",
            "src/content/powers/mod.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Cleanup paths deleting sentinel/negative amount powers incorrectly.",
            "RemoveDebuffs parity drift for PowerType.DEBUFF.",
        ],
    },
    "debuff_gate": {
        "description": "Artifact / DEBUFF gate family across apply paths.",
        "method_tags": ["uses_debuff_gate", "checks_artifact"],
        "hook_names": ["onApplyPower", "onReceivePower"],
        "rust_entries": [
            "src/content/powers/mod.rs",
            "src/engine/action_handlers/powers.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Debuff classification drift causing Artifact to over-block or under-block.",
        ],
    },
    "sentinel_amount": {
        "description": "Sentinel `amount=-1` powers and lifecycle handling.",
        "entity_filter": "sentinel_power",
        "hook_names": [],
        "rust_entries": [
            "src/content/powers/mod.rs",
            "src/engine/action_handlers/powers.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Sentinel powers dropped by generic cleanup or parser assumptions.",
        ],
    },
    "queue_insertion": {
        "description": "Java method queue insertion modes for action creation.",
        "method_tags": ["queues_top", "queues_bottom"],
        "hook_names": [],
        "rust_entries": [
            "src/engine/action_handlers/mod.rs",
            "src/action.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "addToTop/addToBot ordering drift in translated action pipelines.",
        ],
    },
    "power_lifecycle": {
        "description": "PowerType, sentinel amount, end-of-turn removal, and Artifact gate lifecycle.",
        "hook_names": ["onApplyPower", "onReceivePower", "onRemove"],
        "rust_entries": [
            "src/content/powers/mod.rs",
            "src/engine/action_handlers/powers.rs",
            "src/content/powers/ironclad/double_tap.rs",
            "src/content/powers/silent/burst.rs",
            "src/content/powers/core/duplication_power.rs",
        ],
        "internal_state": [],
        "parity_risks": [
            "Artifact/debuff classification drift.",
            "Sentinel amount powers incorrectly cleaned up.",
            "Power removal semantics bypassing shared lifecycle helpers.",
        ],
    },
}


def _method_index(cache: dict) -> dict[tuple[str, str], dict]:
    return {
        (method["class_name"], method["name"]): method
        for method in cache["java_methods"]["methods"]
    }


def _entity_index(cache: dict) -> dict[str, dict]:
    return {
        entity["class_name"]: entity
        for entity in cache["java_entities"]["entities"]
    }


def _collect_target_methods(cache: dict, targets: dict[str, list[str]]) -> list[dict]:
    index = _method_index(cache)
    records = []
    for class_name, methods in targets.items():
        for method_name in methods:
            record = index.get((class_name, method_name))
            if record:
                records.append(record)
    return records


def _collect_tagged_methods(cache: dict, tags: list[str]) -> list[dict]:
    tag_set = set(tags)
    return [
        method
        for method in cache["java_methods"]["methods"]
        if tag_set.intersection(method.get("normalized_tags", []))
    ]


def _collect_sentinel_entities(cache: dict) -> list[dict]:
    return [
        entity
        for entity in cache["java_entities"]["entities"]
        if entity.get("category") == "power" and entity.get("analysis", {}).get("uses_sentinel_amount")
    ]


def _build_hooks_section(cache: dict, hook_names: list[str]) -> dict:
    hook_data = cache["java_hooks"]["hooks"]
    callsites = cache["java_callsites"]["callsites"]
    return {
        hook_name: {
            "base_definitions": hook_data.get(hook_name, {}).get("base_definitions", []),
            "overrides": hook_data.get(hook_name, {}).get("overrides", []),
            "callsites": callsites.get(hook_name, []),
        }
        for hook_name in hook_names
    }


def _power_lifecycle_payload(cache: dict, config: dict) -> dict:
    entity_index = _entity_index(cache)
    methods = cache["java_methods"]["methods"]
    rust_dispatch = cache["rust_dispatch"]

    artifact_gate_sites = [
        {
            "class_name": method["class_name"],
            "method": method["name"],
            "file": method["file"],
            "line": method["start_line"],
            "checks": [
                check for check in method["hardcoded_checks"]
                if check["type"] == "hasPower" and normalize_alias(check["id"]) == "artifact"
            ],
        }
        for method in methods
        if any(check["type"] == "hasPower" and normalize_alias(check["id"]) == "artifact" for check in method["hardcoded_checks"])
    ]

    powers = []
    for entity in cache["java_entities"]["entities"]:
        if entity.get("category") != "power":
            continue
        class_methods = [
            method
            for method in methods
            if method["class_name"] == entity["class_name"]
        ]
        rust_variant = entity.get("rust_variant")
        dispatch = sorted(
            fn_name
            for fn_name, variants in rust_dispatch.get("power_dispatch", {}).items()
            if rust_variant in variants
        )
        powers.append(
            {
                "class_name": entity["class_name"],
                "string_id": entity.get("string_id"),
                "file": entity["file"],
                "live": entity.get("live", True),
                "power_type": entity.get("analysis", {}).get("power_type"),
                "uses_sentinel_amount": entity.get("analysis", {}).get("uses_sentinel_amount", False),
                "removes_at_end_of_turn": entity.get("analysis", {}).get("removes_at_end_of_turn", False),
                "has_on_specific_trigger": entity.get("analysis", {}).get("has_on_specific_trigger", False),
                "has_stack_power": entity.get("analysis", {}).get("has_stack_power", False),
                "checks_artifact": any(
                    method["class_name"] == entity["class_name"]
                    and any(check["type"] == "hasPower" and normalize_alias(check["id"]) == "artifact" for check in method["hardcoded_checks"])
                    for method in class_methods
                ),
                "rust_variant": rust_variant,
                "rust_dispatch": dispatch,
                "status": "implemented" if dispatch else "missing",
                "methods": [method["name"] for method in class_methods],
            }
        )

    representative = {}
    for key in ("NoDrawPower", "EntangledPower", "NoSkillsPower", "ConfusionPower", "BarricadePower"):
        if key in entity_index:
            representative[key] = next(power for power in powers if power["class_name"] == key)

    return {
        "generated_at": now_iso(),
        "family": "power_lifecycle",
        "description": config["description"],
        "representative_powers": representative,
        "powers": sorted(powers, key=lambda item: item["class_name"]),
        "artifact_gate_sites": artifact_gate_sites,
        "hooks": _build_hooks_section(cache, config["hook_names"]),
        "rust": {
            "entries": config["rust_entries"],
            "power_dispatch": cache["rust_dispatch"].get("power_dispatch", {}),
        },
        "known_parity_risks": config["parity_risks"],
    }


def _standard_family_payload(family: str, cache: dict, config: dict) -> dict:
    selected_methods: list[dict] = []
    selected_entities: list[dict] = []
    if "targets" in config:
        selected_methods = _collect_target_methods(cache, config["targets"])
    if "method_tags" in config:
        selected_methods = _collect_tagged_methods(cache, config["method_tags"])
    if config.get("entity_filter") == "sentinel_power":
        selected_entities = _collect_sentinel_entities(cache)

    return {
        "generated_at": now_iso(),
        "family": family,
        "description": config["description"],
        "selected_methods": selected_methods,
        "selected_entities": selected_entities,
        "hooks": _build_hooks_section(cache, config["hook_names"]),
        "rust": {
            "entries": config["rust_entries"],
            "power_dispatch": cache["rust_dispatch"].get("power_dispatch", {}),
            "relic_dispatch": cache["rust_dispatch"].get("relic_dispatch", {}),
        },
        "internal_state_dependencies": config["internal_state"],
        "known_parity_risks": config["parity_risks"],
    }


def _render_standard_markdown(audit: dict, config: dict) -> str:
    lines = [f"# Family Audit: `{audit['family']}`", "", config["description"], ""]
    if audit.get("selected_methods"):
        lines.append("## Selected Methods")
        lines.append("")
        for method in audit["selected_methods"]:
            lines.append(f"### `{method['class_name']}.{method['name']}()`")
            lines.append("")
            lines.append(f"File: `{method['file']}`")
            lines.append("")
            if method["call_chain"]["formatted"]:
                lines.append("```")
                lines.append(method["call_chain"]["formatted"])
                lines.append("```")
                lines.append("")
            lines.append(f"Tags: `{', '.join(method.get('normalized_tags', [])) or 'none'}`")
            lines.append("")
    if audit.get("selected_entities"):
        lines.append("## Selected Entities")
        lines.append("")
        for entity in audit["selected_entities"]:
            lines.append(
                f"- `{entity['class_name']}` power_type=`{entity.get('analysis', {}).get('power_type')}` "
                f"sentinel=`{entity.get('analysis', {}).get('uses_sentinel_amount', False)}`"
            )
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
    lines.append("## Known Parity Risks")
    lines.append("")
    for risk in config["parity_risks"]:
        lines.append(f"- {risk}")
    lines.append("")
    return "\n".join(lines)


def _render_power_lifecycle_markdown(audit: dict, config: dict) -> str:
    lines = [f"# Family Audit: `{audit['family']}`", "", config["description"], ""]
    lines.append("## Representative Powers")
    lines.append("")
    for class_name, row in audit["representative_powers"].items():
        lines.append(
            f"- `{class_name}`: type=`{row['power_type']}` sentinel=`{row['uses_sentinel_amount']}` "
            f"end_of_turn_remove=`{row['removes_at_end_of_turn']}` stack=`{row['has_stack_power']}` "
            f"onSpecificTrigger=`{row['has_on_specific_trigger']}` rust=`{row['status']}`"
        )
    lines.append("")
    lines.append("## Artifact Gate Sites")
    lines.append("")
    for site in audit["artifact_gate_sites"][:20]:
        lines.append(f"- `{site['class_name']}.{site['method']}` — `{site['file']}` L{site['line']}")
    lines.append("")
    lines.append("## Hook Coverage")
    lines.append("")
    for hook_name, payload in audit["hooks"].items():
        lines.append(
            f"- `{hook_name}`: {len(payload['base_definitions'])} base definitions, "
            f"{len(payload['overrides'])} overrides, {len(payload['callsites'])} callsites"
        )
    lines.append("")
    lines.append("## Known Parity Risks")
    lines.append("")
    for risk in config["parity_risks"]:
        lines.append(f"- {risk}")
    lines.append("")
    return "\n".join(lines)


def build_family_audit(
    family: str,
    cache_dir: Path = ANALYSIS_CACHE_DIR,
    java_dir: Path | None = None,
    rust_dir: Path | None = None,
) -> tuple[Path, Path]:
    if family not in FAMILY_CONFIG:
        raise SystemExit(f"Unknown family '{family}'. Expected one of: {', '.join(sorted(FAMILY_CONFIG))}")

    cache = load_cache(cache_dir)
    config = FAMILY_CONFIG[family]
    if family == "power_lifecycle":
        audit = _power_lifecycle_payload(cache, config)
        markdown = _render_power_lifecycle_markdown(audit, config)
    else:
        audit = _standard_family_payload(family, cache, config)
        markdown = _render_standard_markdown(audit, config)

    out_dir = ensure_dir(FAMILY_AUDIT_DIR)
    json_path = out_dir / f"{family}.json"
    md_path = out_dir / f"{family}.md"
    json_path.write_text(json.dumps(audit, indent=2, ensure_ascii=False), encoding="utf-8")
    md_path.write_text(markdown, encoding="utf-8")
    return json_path, md_path
