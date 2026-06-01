"""Scan Rust source tree for .rs file existence per entity."""
import re
from pathlib import Path


def to_snake_case(name: str) -> str:
    """Convert PascalCase/camelCase to snake_case."""
    name = name.replace(" ", "_").replace("-", "_")
    s1 = re.sub(r"(.)([A-Z][a-z]+)", r"\1_\2", name)
    result = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", s1).lower()
    return re.sub(r"_+", "_", result)


def scan_rust_files(content_dir: Path, subdirs: list[str] | None = None) -> dict[str, Path]:
    """Scan content_dir for .rs files → {snake_name: full_path}.

    If subdirs is given, only scan those subdirectories recursively.
    """
    result: dict[str, Path] = {}
    if not content_dir.exists():
        return result

    search_dirs = [content_dir]
    if subdirs:
        search_dirs = [content_dir / sd for sd in subdirs if (content_dir / sd).exists()]

    for d in search_dirs:
        for rs_file in d.rglob("*.rs"):
            if rs_file.name == "mod.rs" or rs_file.name == "hooks.rs":
                continue
            stem = rs_file.stem  # e.g. "anger" from "anger.rs"
            result[stem] = rs_file

    return result


def find_rust_file_for_entity(snake_name: str, file_map: dict[str, Path]) -> Path | None:
    """Look up a .rs file by snake_case name."""
    return file_map.get(snake_name)
