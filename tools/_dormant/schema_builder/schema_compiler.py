#!/usr/bin/env python3
import json
import sys
from collections import defaultdict
from pathlib import Path


STANDARD_FIELDS = {
    "java",
    "rust",
    "status",
    "confidence",
    "match_rule",
    "sources",
    "candidates",
}

ENUM_DEFAULTS = {
    "power_id": {"rust_type": "PowerId", "fn_name": "power_id_from_java"},
    "monster_id": {"rust_type": "EnemyId", "fn_name": "monster_id_from_java"},
    "relic_id": {"rust_type": "RelicId", "fn_name": "relic_id_from_java"},
    "card_id": {"rust_type": "CardId", "fn_name": "card_id_from_java"},
    "potion_id": {"rust_type": "PotionId", "fn_name": "java_potion_id_to_rust"},
}


def load_json(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def dedupe_preserve(values):
    seen = set()
    ordered = []
    for value in values:
        if value not in seen:
            seen.add(value)
            ordered.append(value)
    return ordered


def sanitize_java_aliases(values):
    return [value for value in values if value != "UNKNOWN_PLEASE_MAP_MANUALLY"]


def baseline_rust_variant(rust_variant: str, entry: dict):
    explicit = entry.get("rust")
    if isinstance(explicit, str):
        return explicit
    if explicit is None and "rust" in entry:
        return None
    return rust_variant


def merge_metadata(target: dict, source: dict):
    for key, value in source.items():
        if key.startswith("_"):
            continue
        if key in STANDARD_FIELDS:
            continue
        target[key] = value


def build_baseline_indexes(entries: dict):
    by_key = {}
    by_rust = defaultdict(list)
    by_java = defaultdict(list)
    for key, entry in entries.items():
        copied = dict(entry)
        copied["_baseline_key"] = key
        by_key[key] = copied
        rust_variant = baseline_rust_variant(key, copied)
        if rust_variant:
            by_rust[rust_variant].append(copied)
        for java_id in copied.get("java", []):
            by_java[java_id].append(copied)
    return by_key, by_rust, by_java


def compile_enum(enum_key: str, baseline_enum: dict, matched_enum: dict):
    baseline_entries = baseline_enum.get("entries", {})
    matched_entries = matched_enum.get("entries", {})
    _, baseline_by_rust, baseline_by_java = build_baseline_indexes(baseline_entries)

    compiled_entries = {}
    audit = {
        "new_mappings": [],
        "low_confidence": [],
        "unresolved": [],
        "unsupported": [],
        "baseline_diffs": [],
    }
    consumed_baseline_keys = set()

    matched_rust_ids = sorted(matched_entries.keys())
    for rust_id in matched_rust_ids:
        proposal = matched_entries[rust_id]
        baseline_matches = list(baseline_by_rust.get(rust_id, []))
        for java_id in proposal.get("java", []):
            baseline_matches.extend(baseline_by_java.get(java_id, []))

        entry = {
            "rust": rust_id,
            "java": [],
            "sources": [],
            "candidates": proposal.get("_candidates", []),
        }

        baseline_aliases = []
        for matched in baseline_matches:
            baseline_aliases.extend(matched.get("java", []))
            merge_metadata(entry, matched)
            consumed_baseline_keys.add(matched["_baseline_key"])

        proposal_aliases = proposal.get("java", [])
        entry["java"] = sanitize_java_aliases(
            dedupe_preserve(baseline_aliases + proposal_aliases)
        )
        merge_metadata(entry, proposal)

        sources = []
        if baseline_matches:
            sources.append("baseline")
        sources.extend(proposal.get("_sources", []))
        entry["sources"] = dedupe_preserve(sources)
        entry["match_rule"] = (
            "baseline" if baseline_matches and not proposal_aliases else proposal.get("_match_rule")
        )
        entry["confidence"] = max(
            1.0 if baseline_matches else 0.0,
            float(proposal.get("_match_confidence", 0.0)),
        )

        if entry["java"]:
            entry["status"] = "mapped"
            if not baseline_matches:
                audit["new_mappings"].append(
                    {
                        "enum": enum_key,
                        "rust": rust_id,
                        "java": entry["java"],
                        "rule": entry["match_rule"],
                        "confidence": entry["confidence"],
                    }
                )
        else:
            entry["status"] = (
                "unsupported"
                if "UNKNOWN_PLEASE_MAP_MANUALLY" in baseline_aliases + proposal_aliases
                else "unresolved"
            )
            target_bucket = "unsupported" if entry["status"] == "unsupported" else "unresolved"
            audit[target_bucket].append(
                {
                    "enum": enum_key,
                    "rust": rust_id,
                    "rule": proposal.get("_match_rule", "unresolved"),
                }
            )

        if entry["status"] == "mapped" and entry["confidence"] < 0.90:
            audit["low_confidence"].append(
                {
                    "enum": enum_key,
                    "rust": rust_id,
                    "java": entry["java"],
                    "rule": entry["match_rule"],
                    "confidence": entry["confidence"],
                    "candidates": entry["candidates"],
                }
            )

        compiled_entries[rust_id] = entry

    for key, baseline_entry in baseline_entries.items():
        if key in consumed_baseline_keys:
            continue
        rust_variant = baseline_rust_variant(key, baseline_entry)
        if rust_variant and rust_variant in compiled_entries:
            compiled = compiled_entries[rust_variant]
            base_java = set(baseline_entry.get("java", []))
            compiled_java = set(compiled.get("java", []))
            if not base_java.issubset(compiled_java):
                audit["baseline_diffs"].append(
                    {
                        "enum": enum_key,
                        "rust": rust_variant,
                        "baseline_java": sorted(base_java),
                        "compiled_java": sorted(compiled_java),
                    }
                )
            continue

        entry = dict(baseline_entry)
        entry["rust"] = rust_variant
        entry["java"] = sanitize_java_aliases(entry.get("java", []))
        entry["sources"] = ["baseline"]
        entry["candidates"] = []
        entry["confidence"] = 1.0
        entry["match_rule"] = "baseline"
        if "rust" in baseline_entry and baseline_entry.get("rust") is None:
            entry["status"] = baseline_entry.get("status", "unsupported")
            audit["unsupported"].append(
                {
                    "enum": enum_key,
                    "key": key,
                    "java": entry.get("java", []),
                    "status": entry["status"],
                }
            )
            compiled_entries[key] = entry
        elif rust_variant and rust_variant not in compiled_entries:
            if entry.get("java"):
                entry["status"] = "mapped"
            elif "UNKNOWN_PLEASE_MAP_MANUALLY" in baseline_entry.get("java", []):
                entry["status"] = "unsupported"
            else:
                entry["status"] = "unresolved"
            if entry["status"] == "unresolved":
                audit["unresolved"].append(
                    {"enum": enum_key, "rust": rust_variant, "rule": "baseline_missing_java"}
                )
            compiled_entries[rust_variant] = entry
        else:
            entry["status"] = "unresolved"
            audit["unresolved"].append(
                {"enum": enum_key, "rust": key, "rule": "baseline_unresolved"}
            )
            compiled_entries[key] = entry

    return compiled_entries, audit


def observed_coverage_report(compiled_schema: dict, observed: dict):
    alias_status = defaultdict(dict)
    for enum_key, enum_def in compiled_schema.get("enums", {}).items():
        category = enum_key.replace("_id", "")
        for _, entry in enum_def.get("entries", {}).items():
            status = entry.get("status", "mapped")
            for java_id in entry.get("java", []):
                alias_status[category][java_id] = status

    coverage = {
        "covered": [],
        "unsupported": [],
        "unresolved": [],
    }
    for category, values in observed.get("categories", {}).items():
        for java_id in values.keys():
            status = alias_status.get(category, {}).get(java_id)
            payload = {"category": category, "java": java_id, "status": status}
            if status == "mapped":
                coverage["covered"].append(payload)
            elif status in {"unsupported", "internal_only"}:
                coverage["unsupported"].append(payload)
            else:
                coverage["unresolved"].append(payload)
    return coverage


def main():
    if len(sys.argv) < 6:
        print(
            "Usage: python schema_compiler.py <matched.json> <baseline.json> <observed.json> <compiled.json> <audit.json>"
        )
        sys.exit(1)

    matched_file, baseline_file, observed_file, compiled_file, audit_file = [
        Path(arg) for arg in sys.argv[1:6]
    ]

    matched = load_json(matched_file)
    baseline = load_json(baseline_file)
    observed = load_json(observed_file)

    compiled = {
        "version": "4.0",
        "description": "Compiled protocol schema for runtime consumption",
        "inputs": {
            "baseline": str(baseline_file),
            "matched": str(matched_file),
            "observed": str(observed_file),
        },
        "enums": {},
    }
    audit = {
        "summary": {},
        "new_mappings": [],
        "low_confidence": [],
        "unresolved": [],
        "unsupported": [],
        "baseline_diffs": [],
        "observed_coverage": {},
    }

    for enum_key, base_enum in baseline.get("enums", {}).items():
        matched_enum = matched.get("enums", {}).get(enum_key, {})
        compiled_entries, enum_audit = compile_enum(enum_key, base_enum, matched_enum)

        compiled_enum = dict(base_enum)
        for key, value in ENUM_DEFAULTS.get(enum_key, {}).items():
            compiled_enum.setdefault(key, value)
        compiled_enum["entries"] = compiled_entries
        compiled["enums"][enum_key] = compiled_enum

        audit["new_mappings"].extend(enum_audit["new_mappings"])
        audit["low_confidence"].extend(enum_audit["low_confidence"])
        audit["unresolved"].extend(enum_audit["unresolved"])
        audit["unsupported"].extend(enum_audit["unsupported"])
        audit["baseline_diffs"].extend(enum_audit["baseline_diffs"])

    coverage = observed_coverage_report(compiled, observed)
    audit["observed_coverage"] = {
        "covered": len(coverage["covered"]),
        "unsupported": len(coverage["unsupported"]),
        "unresolved": len(coverage["unresolved"]),
        "details": coverage,
    }
    audit["summary"] = {
        "new_mappings": len(audit["new_mappings"]),
        "low_confidence": len(audit["low_confidence"]),
        "unresolved": len(audit["unresolved"]),
        "unsupported": len(audit["unsupported"]),
        "baseline_diffs": len(audit["baseline_diffs"]),
    }

    compiled_file.write_text(json.dumps(compiled, indent=2), encoding="utf-8")
    audit_file.write_text(json.dumps(audit, indent=2), encoding="utf-8")

    print(f"Compiled schema written to {compiled_file}")
    print(f"Audit report written to {audit_file}")
    print(
        "Summary:",
        json.dumps(
            {
                **audit["summary"],
                "observed_unresolved": audit["observed_coverage"]["unresolved"],
            }
        ),
    )


if __name__ == "__main__":
    main()
