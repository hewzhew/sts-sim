#!/usr/bin/env python3
from __future__ import annotations

import argparse

try:
    from tools.analysis.family_audit import FAMILY_CONFIG, build_family_audit
except ImportError:
    from analysis.family_audit import FAMILY_CONFIG, build_family_audit  # type: ignore


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate structured bug-family audit reports.")
    parser.add_argument("family", choices=sorted(FAMILY_CONFIG))
    args = parser.parse_args()
    json_path, md_path = build_family_audit(args.family)
    print(f"JSON: {json_path}")
    print(f"Markdown: {md_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
