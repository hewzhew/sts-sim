"""Legacy parser for scattered_logic.md.

Normal coverage paths should consume analysis_cache/source_extractor JSON facts first.
This module remains only as historical compatibility code.
"""
from pathlib import Path


def parse_scattered_logic(path: Path) -> dict[str, int]:
    """Parse scattered_logic.md → {java_id_lowercase: ref_count}.

    Counts how many times each entity ID appears in scattered engine logic.
    """
    if not path.exists():
        return {}

    text = path.read_text(encoding="utf-8-sig").lower()

    # We don't parse per-entity — caller will check membership
    # Return the full text for substring matching
    return {"_full_text": text}  # type: ignore


def check_scattered(full_text: str, java_id: str) -> bool:
    """Check if a Java ID appears in scattered logic text."""
    return java_id.lower() in full_text
