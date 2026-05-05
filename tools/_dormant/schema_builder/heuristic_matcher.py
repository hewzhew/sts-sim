#!/usr/bin/env python3
import difflib
import json
import re
import sys
from pathlib import Path


CATEGORY_TO_ENUM = {
    "relic": "relic_id",
    "monster": "monster_id",
    "card": "card_id",
    "power": "power_id",
    "potion": "potion_id",
}

CARD_CLASS_SUFFIXES = ("_r", "_g", "_b", "_p", "_c")
TOKEN_TRANSFORMS = {
    "nodraw": {"nodraw", "no draw"},
    "modeshift": {"modeshift", "mode shift"},
    "sharphide": {"sharphide", "sharp hide"},
    "selfformingclay": {"selfformingclay", "self forming clay"},
    "jax": {"jax", "j.a.x."},
    "handofgreed": {"handofgreed", "hand of greed"},
}


def load_json(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def base_normalize(value: str) -> str:
    value = value.strip()
    value = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", value)
    value = re.sub(r"[^A-Za-z0-9]+", "", value).lower()
    return value


def expand_normalized_forms(value: str, category: str):
    normalized = base_normalize(value)
    forms = {normalized}

    if category == "card":
        for suffix in CARD_CLASS_SUFFIXES:
            if normalized.endswith(suffix[1:]):
                forms.add(normalized[: -len(suffix) + 1])
        for suffix in ("red", "green", "blue", "purple", "colorless", "curse", "status"):
            if normalized.endswith(suffix):
                forms.add(normalized[: -len(suffix)])

    if normalized in TOKEN_TRANSFORMS:
        forms.update(base_normalize(v) for v in TOKEN_TRANSFORMS[normalized])
    for canonical, variants in TOKEN_TRANSFORMS.items():
        if normalized in {base_normalize(v) for v in variants}:
            forms.add(canonical)
            forms.update(base_normalize(v) for v in variants)

    return {form for form in forms if form}


def confidence_for_rule(rule: str, similarity: float = 0.0, observed=False):
    base = {
        "override": 1.0,
        "exact_name": 0.99,
        "normalized_name": 0.96,
        "token_transform": 0.93,
        "class_name": 0.90,
        "fuzzy": max(0.70, min(0.89, similarity)),
    }[rule]
    if observed and rule != "override":
        base = min(1.0, base + 0.02)
    return round(base, 3)


def build_java_candidates(category: str, java_dict: dict, observed_java_ids: set[str]):
    candidates = []
    for java_id, meta in java_dict.items():
        class_name = meta.get("class_name", "")
        observed = java_id in observed_java_ids
        candidates.append(
            {
                "java_id": java_id,
                "class_name": class_name,
                "normalized_java": expand_normalized_forms(java_id, category),
                "normalized_class": expand_normalized_forms(class_name, category),
                "observed": observed,
            }
        )
    return candidates


def exact_match(rust_id: str, candidates):
    lowered = rust_id.lower()
    for candidate in candidates:
        if candidate["java_id"].lower() == lowered:
            return candidate, "exact_name", 1.0
        if candidate["class_name"].lower() == lowered:
            return candidate, "exact_name", 1.0
    return None


def normalized_match(rust_id: str, category: str, candidates):
    rust_forms = expand_normalized_forms(rust_id, category)
    for candidate in candidates:
        if rust_forms & candidate["normalized_java"]:
            return candidate, "normalized_name"
    return None


def token_transform_match(rust_id: str, category: str, candidates):
    rust_forms = expand_normalized_forms(rust_id, category)
    for candidate in candidates:
        if rust_forms & candidate["normalized_class"]:
            return candidate, "token_transform"
    return None


def class_name_match(rust_id: str, category: str, candidates):
    rust_forms = expand_normalized_forms(rust_id, category)
    for candidate in candidates:
        if candidate["normalized_class"] and rust_forms & candidate["normalized_class"]:
            return candidate, "class_name"
    return None


def fuzzy_candidates(rust_id: str, category: str, candidates):
    rust_key = sorted(expand_normalized_forms(rust_id, category))[0]
    scored = []
    for candidate in candidates:
        java_key = sorted(candidate["normalized_java"] or {""})[0]
        class_key = sorted(candidate["normalized_class"] or {""})[0]
        similarity = max(
            difflib.SequenceMatcher(None, rust_key, java_key).ratio(),
            difflib.SequenceMatcher(None, rust_key, class_key).ratio(),
        )
        if similarity >= 0.75:
            scored.append((candidate, similarity))
    scored.sort(key=lambda item: item[1], reverse=True)
    return scored[:3]


def main():
    if len(sys.argv) < 7:
        print(
            "Usage: python heuristic_matcher.py <java.json> <rust.json> <skeleton.json> <overrides.json> <observed.json> <output.json>"
        )
        sys.exit(1)

    java_file, rust_file, skeleton_file, overrides_file, observed_file, out_file = [
        Path(p) for p in sys.argv[1:7]
    ]

    java_nodes = load_json(java_file)
    rust_nodes = load_json(rust_file)
    final_schema = load_json(skeleton_file)
    overrides = load_json(overrides_file)
    observed = load_json(observed_file)
    observed_categories = observed.get("categories", {})

    for category, rust_list in rust_nodes.items():
        enum_key = CATEGORY_TO_ENUM[category]
        java_dict = java_nodes.get(category, {})
        overrides_dict = overrides.get(enum_key, {})
        observed_java_ids = set(observed_categories.get(category, {}).keys())
        entries = final_schema["enums"][enum_key]["entries"]
        candidates = build_java_candidates(category, java_dict, observed_java_ids)

        for rust_id in sorted(set(rust_list)):
            override_data = dict(overrides_dict.get(rust_id, {}))
            explicit_java = override_data.get("java")
            if isinstance(explicit_java, str):
                explicit_java = [explicit_java]

            entry = {"rust": rust_id}
            for key, value in override_data.items():
                entry[key] = value

            if explicit_java:
                entry["java"] = explicit_java
                entry["_match_rule"] = "override"
                entry["_match_confidence"] = 1.0
                entry["_sources"] = ["override_rules"]
                entry["_candidates"] = []
                entries[rust_id] = entry
                continue

            selected = exact_match(rust_id, candidates)
            if selected is None:
                normalized = normalized_match(rust_id, category, candidates)
                selected = (
                    (normalized[0], normalized[1], None) if normalized is not None else None
                )
            if selected is None:
                token = token_transform_match(rust_id, category, candidates)
                selected = ((token[0], token[1], None) if token is not None else None)
            if selected is None:
                structural = class_name_match(rust_id, category, candidates)
                selected = (
                    (structural[0], structural[1], None) if structural is not None else None
                )

            scored_candidates = []
            if selected is None:
                for candidate, similarity in fuzzy_candidates(rust_id, category, candidates):
                    scored_candidates.append(
                        {
                            "java": candidate["java_id"],
                            "class_name": candidate["class_name"],
                            "rule": "fuzzy",
                            "confidence": confidence_for_rule(
                                "fuzzy", similarity, candidate["observed"]
                            ),
                            "observed": candidate["observed"],
                        }
                    )
                if scored_candidates:
                    best = scored_candidates[0]
                    entry["java"] = [best["java"]]
                    entry["_match_rule"] = "fuzzy"
                    entry["_match_confidence"] = best["confidence"]
                    entry["_sources"] = ["java_crawler", "rust_crawler"]
                    if best["observed"]:
                        entry["_sources"].append("observed_ids")
                    entry["_candidates"] = scored_candidates
                else:
                    entry["_match_rule"] = "unresolved"
                    entry["_match_confidence"] = 0.0
                    entry["_sources"] = ["rust_crawler"]
                    entry["_candidates"] = []
                entries[rust_id] = entry
                continue

            candidate, rule, similarity = selected
            entry["java"] = [candidate["java_id"]]
            entry["_match_rule"] = rule
            entry["_match_confidence"] = confidence_for_rule(
                rule, similarity or 0.0, candidate["observed"]
            )
            entry["_sources"] = ["java_crawler", "rust_crawler"]
            if candidate["observed"]:
                entry["_sources"].append("observed_ids")
            entry["_candidates"] = [
                {
                    "java": candidate["java_id"],
                    "class_name": candidate["class_name"],
                    "rule": rule,
                    "confidence": entry["_match_confidence"],
                    "observed": candidate["observed"],
                }
            ]
            entries[rust_id] = entry

    out_file.write_text(json.dumps(final_schema, indent=2), encoding="utf-8")
    print(f"Matcher complete. Proposal schema written to {out_file}")


if __name__ == "__main__":
    main()
