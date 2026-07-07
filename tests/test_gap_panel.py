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
    def test_default_run_delegates_to_branch_panel_smoke(self):
        gap_panel = load_gap_panel()
        commands = []

        def fake_run_command(command, stdout_path, _stderr_path, append=False):
            self.assertFalse(append)
            commands.append(command)
            summary = stdout_path.parent / "panel_summary.json"
            summary.write_text(json.dumps({"schema": "branch_panel_summary_v0"}), encoding="utf-8")
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
        self.assertEqual(len(commands), 1)
        command = commands[0]
        self.assertEqual(command[:5], ["cargo", "run", "--bin", "branch_panel", "--"])
        self.assertIn("smoke", command)
        self.assertIn("--slice-ms", command)
        self.assertNotIn("branch_tiny", command)

    def test_continue_soft_wall_delegates_to_branch_panel_drain(self):
        gap_panel = load_gap_panel()
        commands = []

        def fake_run_command(command, stdout_path, _stderr_path, append=False):
            commands.append(command)
            summary = stdout_path.parent / "panel_summary.json"
            summary.write_text(json.dumps({"schema": "branch_panel_summary_v0"}), encoding="utf-8")
            return 0

        with tempfile.TemporaryDirectory() as tmp:
            argv = [
                "gap_panel.py",
                "--seeds",
                "1",
                "--capsule-root",
                str(Path(tmp) / "capsules"),
                "--continue-soft-wall",
                "2",
            ]
            with mock.patch.object(sys, "argv", argv), mock.patch.object(
                gap_panel, "run_command", fake_run_command
            ):
                exit_code = gap_panel.main()

        self.assertEqual(exit_code, 0)
        command = commands[0]
        self.assertIn("drain", command)
        self.assertIn("--max-slices", command)
        self.assertIn("3", command)


if __name__ == "__main__":
    unittest.main()
