"""Cross-reference Java hooks with Rust implementations to build coverage entries."""
import json
from pathlib import Path
from .models import (
    CoverageEntry, CategorySummary, JavaEntity, JavaHook, RustEntity,
    EntityCategory, HookStatus, SKIPPABLE_HOOKS,
)
from .parsers.schema import parse_schema_dump, build_class_to_id_map, build_class_to_file_map
from .analyzers.rust_powers import analyze_power_dispatch, POWER_DISPATCH_MAP
from .analyzers.rust_relics import analyze_relic_dispatch, RELIC_DISPATCH_MAP
from .analyzers.rust_files import scan_rust_files, to_snake_case


class CoverageAnalyzer:
    """Orchestrates the full Java ↔ Rust coverage analysis."""

    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.extractor_output = project_root / "tools" / "source_extractor" / "output"
        self.analysis_cache = project_root / "tools" / "analysis_cache"
        self.rust_src = project_root / "src" / "content"

    @staticmethod
    def _structured_entities_to_java_entities(data: dict) -> dict[str, JavaEntity]:
        entities: dict[str, JavaEntity] = {}
        for category_name, rows in data.get("entities", {}).items():
            try:
                category = EntityCategory(category_name.lower())
            except ValueError:
                continue
            for row in rows:
                class_name = row["class_name"]
                entity = entities.setdefault(
                    class_name,
                    JavaEntity(
                        class_name=class_name,
                        java_id=row.get("string_id") or class_name,
                        category=category,
                        java_file=row.get("file", ""),
                    ),
                )
                for hook in row.get("hooks", []):
                    hook_name = hook["name"]
                    if hook_name in {existing.name for existing in entity.hooks}:
                        continue
                    status = HookStatus.SKIPPED if hook_name in SKIPPABLE_HOOKS else HookStatus.MISSING
                    entity.hooks.append(JavaHook(name=hook_name, status=status))
        return entities

    @staticmethod
    def _extract_scattered_ids(data: dict) -> set[str]:
        ids: set[str] = set()
        for category_entries in data.get("entities", {}).values():
            for entry in category_entries:
                for key in ("entity", "normalized_entity"):
                    value = entry.get(key)
                    if value:
                        ids.add(str(value).lower())
                        ids.add(str(value).replace(" ", "").lower())
        return ids

    @staticmethod
    def _has_scattered(scattered_lookup: set[str] | str, java_id: str) -> bool:
        candidates = {java_id.lower(), java_id.replace(" ", "").lower()}
        if isinstance(scattered_lookup, set):
            return any(candidate in scattered_lookup for candidate in candidates)
        return any(candidate in scattered_lookup for candidate in candidates)

    def _load_java_entities(self) -> dict[str, JavaEntity]:
        cache_path = self.analysis_cache / "java_hooks.json"
        entities_path = self.analysis_cache / "java_entities.json"
        extractor_hooks_path = self.extractor_output / "hooks.json"
        if (not cache_path.exists() or not entities_path.exists()) and extractor_hooks_path.exists():
            data = json.loads(extractor_hooks_path.read_text(encoding="utf-8"))
            return self._structured_entities_to_java_entities(data)
        if not cache_path.exists() or not entities_path.exists():
            return {}

        hook_data = json.loads(cache_path.read_text(encoding="utf-8"))
        entity_data = json.loads(entities_path.read_text(encoding="utf-8"))
        class_meta = {
            entity["class_name"]: entity
            for entity in entity_data.get("entities", [])
        }
        entities: dict[str, JavaEntity] = {}

        for hook_name, payload in hook_data.get("hooks", {}).items():
            all_rows = payload.get("base_definitions", []) + payload.get("overrides", [])
            for row in all_rows:
                class_name = row["class"]
                meta = class_meta.get(class_name, {})
                category_str = (row.get("category") or meta.get("category") or "card").lower()
                try:
                    category = EntityCategory(category_str)
                except ValueError:
                    continue
                entity = entities.setdefault(
                    class_name,
                    JavaEntity(
                        class_name=class_name,
                        java_id=meta.get("string_id") or row.get("string_id") or class_name,
                        category=category,
                        java_file=meta.get("file", row.get("file", "")),
                    ),
                )
                if hook_name not in {hook.name for hook in entity.hooks}:
                    status = HookStatus.SKIPPED if hook_name in SKIPPABLE_HOOKS else HookStatus.MISSING
                    entity.hooks.append(JavaHook(name=hook_name, status=status))

        return entities

    def _build_class_maps(self, category: str) -> tuple[dict[str, str], dict[str, str]]:
        entities_path = self.analysis_cache / "java_entities.json"
        if not entities_path.exists():
            schema_entities = parse_schema_dump(self.extractor_output / "schema_dump.json")
            return (
                build_class_to_id_map(schema_entities, category),
                build_class_to_file_map(schema_entities, category),
            )

        data = json.loads(entities_path.read_text(encoding="utf-8"))
        class_to_id: dict[str, str] = {}
        class_to_file: dict[str, str] = {}
        for entity in data.get("entities", []):
            if entity.get("category", "").lower() != category.lower():
                continue
            class_to_id[entity["class_name"]] = entity.get("string_id") or entity["class_name"]
            class_to_file[entity["class_name"]] = entity.get("file", "")
        return class_to_id, class_to_file

    def _load_scattered_lookup(self) -> set[str] | str:
        cache_path = self.analysis_cache / "java_callsites.json"
        if cache_path.exists():
            data = json.loads(cache_path.read_text(encoding="utf-8"))
            tokens: set[str] = set()
            for callsites in data.get("callsites", {}).values():
                for site in callsites:
                    for check in site.get("hardcoded_checks", []):
                        check_id = check.get("id")
                        if check_id:
                            tokens.add(str(check_id).lower())
                            tokens.add(str(check_id).replace(" ", "").lower())
            return tokens

        scattered_json_path = self.extractor_output / "scattered_logic.json"
        if scattered_json_path.exists():
            data = json.loads(scattered_json_path.read_text(encoding="utf-8"))
            return self._extract_scattered_ids(data)
        return set()

    def analyze_powers(self) -> CategorySummary:
        """Build coverage data for all Powers."""
        # 1. Parse Java hooks
        java_entities = self._load_java_entities()

        # 2. Parse schema for ID mapping
        class_to_id, class_to_file = self._build_class_maps("power")

        # 3. Analyze Rust dispatch
        power_dispatch = analyze_power_dispatch(self.rust_src / "powers" / "mod.rs")

        # 4. Scan Rust files
        rust_files = scan_rust_files(self.rust_src / "powers", ["core", "ironclad"])

        # 5. Load scattered logic
        scattered_text = self._load_scattered_lookup()

        # 6. Build coverage entries
        summary = CategorySummary(category=EntityCategory.POWER)

        # Filter to only POWER entities from hooks.md
        power_entities = {k: v for k, v in java_entities.items()
                         if v.category == EntityCategory.POWER}

        for class_name, java_ent in sorted(power_entities.items()):
            # Skip deprecated
            if class_name.startswith("DEPRECATED"):
                continue

            # Resolve Java ID
            java_id = class_to_id.get(class_name, class_name)
            java_ent.java_id = java_id
            if class_name in class_to_file:
                java_ent.java_file = class_to_file[class_name]

            # Check scattered logic
            java_ent.has_scattered_logic = self._has_scattered(scattered_text, java_id)

            # Find Rust entity
            snake = to_snake_case(class_name.replace("Power", ""))
            rust_file = rust_files.get(snake)

            # Try variant name matching in dispatch
            # The Rust PowerId variant often matches the java_id or class_name minus "Power"
            variant_candidates = [
                class_name.replace("Power", ""),
                java_id,
                java_id.replace(" ", ""),
            ]

            matched_dispatch = {}
            for vc in variant_candidates:
                if vc in power_dispatch:
                    matched_dispatch = {vc: power_dispatch[vc]}
                    break

            rust_ent = RustEntity(
                enum_variant=variant_candidates[0],
                file_path=str(rust_file) if rust_file else None,
                file_exists=rust_file is not None,
                matched_hooks=[fn for fns in matched_dispatch.values() for fn in fns],
            )

            # 7. Build hook details
            hook_details = []
            for jh in java_ent.hooks:
                if jh.name in SKIPPABLE_HOOKS:
                    hook_details.append(JavaHook(jh.name, HookStatus.SKIPPED))
                    continue

                # Check if this Java hook has a corresponding Rust dispatch
                implemented = False
                rust_fn = None
                for dispatch_fn, java_hook_name in POWER_DISPATCH_MAP.items():
                    if java_hook_name == jh.name:
                        # Check if our variant is in this function
                        for vc in variant_candidates:
                            fns = power_dispatch.get(vc, [])
                            if dispatch_fn in fns:
                                implemented = True
                                rust_fn = dispatch_fn
                                break
                    if implemented:
                        break

                status = HookStatus.IMPLEMENTED if implemented else HookStatus.MISSING
                hook_details.append(JavaHook(jh.name, status, rust_fn))

            entry = CoverageEntry(java=java_ent, rust=rust_ent, hook_details=hook_details)
            summary.entries.append(entry)
            summary.total_java += 1
            if rust_ent.file_exists:
                summary.has_rust_file += 1
            if entry.coverage_pct >= 100:
                summary.fully_covered += 1
            elif entry.coverage_pct > 0:
                summary.partially_covered += 1
            else:
                summary.not_covered += 1

        return summary

    def analyze_relics(self) -> CategorySummary:
        """Build coverage data for all Relics."""
        # 1. Parse Java hooks (relic section)
        java_entities = self._load_java_entities()

        # 2. Schema
        class_to_id, _ = self._build_class_maps("relic")

        # 3. Rust dispatch from hooks.rs
        relic_dispatch = analyze_relic_dispatch(self.rust_src / "relics" / "hooks.rs")

        # 4. Rust files
        rust_files = scan_rust_files(self.rust_src / "relics")

        # 5. Scattered logic
        scattered_text = self._load_scattered_lookup()

        summary = CategorySummary(category=EntityCategory.RELIC)

        relic_entities = {k: v for k, v in java_entities.items()
                         if v.category == EntityCategory.RELIC}

        for class_name, java_ent in sorted(relic_entities.items()):
            if class_name.startswith("DEPRECATED") or class_name.startswith("Test"):
                continue
            if class_name == "AbstractRelic":
                continue

            java_id = class_to_id.get(class_name, class_name)
            java_ent.java_id = java_id
            java_ent.has_scattered_logic = self._has_scattered(scattered_text, java_id)

            snake = to_snake_case(class_name)
            rust_file = rust_files.get(snake)

            # Check dispatch
            variant_candidates = [class_name, java_id.replace(" ", "")]
            matched_fns = []
            for vc in variant_candidates:
                if vc in relic_dispatch:
                    matched_fns = relic_dispatch[vc]
                    break

            rust_ent = RustEntity(
                enum_variant=class_name,
                file_path=str(rust_file) if rust_file else None,
                file_exists=rust_file is not None,
                matched_hooks=matched_fns,
            )

            hook_details = []
            for jh in java_ent.hooks:
                if jh.name in SKIPPABLE_HOOKS:
                    hook_details.append(JavaHook(jh.name, HookStatus.SKIPPED))
                    continue

                # Check if this relic appears in a relevant dispatch function
                implemented = False
                rust_fn = None
                for dispatch_fn_name in matched_fns:
                    java_hook = RELIC_DISPATCH_MAP.get(dispatch_fn_name)
                    if java_hook and java_hook == jh.name:
                        implemented = True
                        rust_fn = dispatch_fn_name
                        break

                # Also count file existence as partial evidence
                if not implemented and rust_file and len(java_ent.hooks) <= 2:
                    # Simple relics with only makeCopy + 1 hook usually work
                    implemented = rust_ent.file_exists

                status = HookStatus.IMPLEMENTED if implemented else HookStatus.MISSING
                hook_details.append(JavaHook(jh.name, status, rust_fn))

            entry = CoverageEntry(java=java_ent, rust=rust_ent, hook_details=hook_details)
            summary.entries.append(entry)
            summary.total_java += 1
            if rust_ent.file_exists:
                summary.has_rust_file += 1
            if entry.coverage_pct >= 100:
                summary.fully_covered += 1
            elif entry.coverage_pct > 0:
                summary.partially_covered += 1
            else:
                summary.not_covered += 1

        return summary

    def analyze_all(self) -> dict[EntityCategory, CategorySummary]:
        """Run all analyzers and return results."""
        return {
            EntityCategory.POWER: self.analyze_powers(),
            EntityCategory.RELIC: self.analyze_relics(),
        }
