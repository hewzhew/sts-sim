"""Cross-reference Java hooks with Rust implementations to build coverage entries."""
from pathlib import Path
from .models import (
    CoverageEntry, CategorySummary, JavaEntity, JavaHook, RustEntity,
    EntityCategory, HookStatus, SKIPPABLE_HOOKS,
)
from .parsers.java_hooks import parse_hooks_md
from .parsers.schema import parse_schema_dump, build_class_to_id_map, build_class_to_file_map
from .parsers.scattered import check_scattered
from .analyzers.rust_powers import analyze_power_dispatch, POWER_DISPATCH_MAP
from .analyzers.rust_relics import analyze_relic_dispatch, RELIC_DISPATCH_MAP
from .analyzers.rust_files import scan_rust_files, to_snake_case


class CoverageAnalyzer:
    """Orchestrates the full Java ↔ Rust coverage analysis."""

    def __init__(self, project_root: Path):
        self.project_root = project_root
        self.extractor_output = project_root / "tools" / "source_extractor" / "output"
        self.rust_src = project_root / "src" / "content"

    def analyze_powers(self) -> CategorySummary:
        """Build coverage data for all Powers."""
        # 1. Parse Java hooks
        java_entities = parse_hooks_md(self.extractor_output / "hooks.md")

        # 2. Parse schema for ID mapping
        schema_entities = parse_schema_dump(self.extractor_output / "schema_dump.json")
        class_to_id = build_class_to_id_map(schema_entities, "power")
        class_to_file = build_class_to_file_map(schema_entities, "power")

        # 3. Analyze Rust dispatch
        power_dispatch = analyze_power_dispatch(self.rust_src / "powers" / "mod.rs")

        # 4. Scan Rust files
        rust_files = scan_rust_files(self.rust_src / "powers", ["core", "ironclad"])

        # 5. Load scattered logic
        scattered_path = self.extractor_output / "scattered_logic.md"
        scattered_text = ""
        if scattered_path.exists():
            scattered_text = scattered_path.read_text(encoding="utf-8-sig").lower()

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
            java_ent.has_scattered_logic = check_scattered(scattered_text, java_id)

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
        java_entities = parse_hooks_md(self.extractor_output / "hooks.md")

        # 2. Schema
        schema_entities = parse_schema_dump(self.extractor_output / "schema_dump.json")
        class_to_id = build_class_to_id_map(schema_entities, "relic")

        # 3. Rust dispatch from hooks.rs
        relic_dispatch = analyze_relic_dispatch(self.rust_src / "relics" / "hooks.rs")

        # 4. Rust files
        rust_files = scan_rust_files(self.rust_src / "relics")

        # 5. Scattered logic
        scattered_path = self.extractor_output / "scattered_logic.md"
        scattered_text = ""
        if scattered_path.exists():
            scattered_text = scattered_path.read_text(encoding="utf-8-sig").lower()

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
            java_ent.has_scattered_logic = check_scattered(scattered_text, java_id)

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
