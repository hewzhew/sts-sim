"""Run the maintained learning suite under one explicit unittest protocol."""

from __future__ import annotations

import ast
import sys
import unittest
from pathlib import Path


def _module_level_tests(test_root: Path) -> tuple[str, ...]:
    violations: list[str] = []
    for path in sorted(test_root.glob("test_*.py")):
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        for node in tree.body:
            is_test = isinstance(
                node,
                (ast.FunctionDef, ast.AsyncFunctionDef),
            ) and node.name.startswith("test_")
            if is_test:
                violations.append(f"{path.name}:{node.lineno}:{node.name}")
    return tuple(violations)


def main() -> int:
    learning_root = Path(__file__).resolve().parent
    repository_root = learning_root.parent
    test_root = learning_root / "tests"
    violations = _module_level_tests(test_root)
    if violations:
        print(
            "learning tests use unittest.TestCase only; module-level tests are not "
            "collected by the maintained runner:",
            file=sys.stderr,
        )
        for violation in violations:
            print(f"  {violation}", file=sys.stderr)
        return 2

    sys.path.insert(0, str(repository_root))
    suite = unittest.defaultTestLoader.discover(str(test_root))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
