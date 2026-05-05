"""Legacy parser for hooks.md.

Normal coverage paths should consume analysis_cache/source_extractor JSON facts first.
This module remains only as historical compatibility code.
"""
import re
from pathlib import Path
from ..models import JavaEntity, JavaHook, EntityCategory, SKIPPABLE_HOOKS, HookStatus


def parse_hooks_md(hooks_path: Path) -> dict[str, JavaEntity]:
    """Parse hooks.md → dict keyed by Java class name.

    Format expected:
        ## POWER Hooks

        ### AngerPower
        File: `powers\\AngerPower.java`

        - `onUseCard`
        - `updateDescription`
    """
    entities: dict[str, JavaEntity] = {}
    if not hooks_path.exists():
        return entities

    text = hooks_path.read_text(encoding="utf-8-sig")
    current_category = None
    current_class = None

    for line in text.splitlines():
        line = line.strip()

        # Detect category headers like "## POWER Hooks"
        if line.startswith("## ") and "Hooks" in line:
            cat_str = line[3:].split()[0].upper()
            cat_map = {"POWER": EntityCategory.POWER, "RELIC": EntityCategory.RELIC,
                       "CARD": EntityCategory.CARD}
            current_category = cat_map.get(cat_str)
            continue

        # Detect entity headers like "### AngerPower"
        if line.startswith("### "):
            class_name = line[4:].strip()
            if current_category:
                current_class = class_name
                if class_name not in entities:
                    entities[class_name] = JavaEntity(
                        class_name=class_name,
                        java_id=class_name,  # Will be refined by schema
                        category=current_category,
                    )
            continue

        # Detect file line like "File: `powers\\AngerPower.java`"
        if line.startswith("File:") and current_class and current_class in entities:
            m = re.search(r"`(.+?)`", line)
            if m:
                entities[current_class].java_file = m.group(1)
            continue

        # Detect hook lines like "- `onUseCard`"
        if line.startswith("- `") and current_class and current_class in entities:
            m = re.search(r"`(\w+)`", line)
            if m:
                hook_name = m.group(1)
                status = HookStatus.SKIPPED if hook_name in SKIPPABLE_HOOKS else HookStatus.MISSING
                entities[current_class].hooks.append(JavaHook(name=hook_name, status=status))

    return entities
