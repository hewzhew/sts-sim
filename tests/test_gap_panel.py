import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


def load_gap_panel():
    module_path = Path(__file__).resolve().parents[1] / "tools" / "gap_panel.py"
    spec = importlib.util.spec_from_file_location("gap_panel", module_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class GapPanelTests(unittest.TestCase):
    def test_default_run_builds_branch_tiny_before_using_target_exe(self):
        gap_panel = load_gap_panel()
        commands = []

        def fake_run_command(command, stdout_path, _stderr_path, append=False):
            self.assertFalse(append)
            commands.append(command)
            if command == ["cargo", "build", "--bin", "branch_tiny"]:
                return 0
            summary = stdout_path.parent / "summary.json"
            summary.write_text(
                json.dumps(
                    {
                        "schema": "branch_tiny_capsule_summary",
                        "blocker_kind": "terminal",
                    }
                ),
                encoding="utf-8",
            )
            return 0

        with tempfile.TemporaryDirectory() as tmp:
            argv = [
                "gap_panel.py",
                "--seeds",
                "1",
                "--capsule-root",
                str(Path(tmp) / "capsules"),
            ]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(
                gap_panel, "run_command", fake_run_command
            ):
                exit_code = gap_panel.main()

        self.assertEqual(exit_code, 0)
        self.assertGreaterEqual(len(commands), 2)
        self.assertEqual(commands[0], ["cargo", "build", "--bin", "branch_tiny"])
        self.assertEqual(commands[1][0], str(gap_panel.default_branch_tiny()))


if __name__ == "__main__":
    unittest.main()
