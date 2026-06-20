import importlib.util
import unittest
from pathlib import Path


def load_module():
    root = Path(__file__).resolve().parents[3]
    module_path = root / "tools" / "ml" / "combat_root_prior_live_compare.py"
    spec = importlib.util.spec_from_file_location("combat_root_prior_live_compare", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class CombatRootPriorLiveCompareTests(unittest.TestCase):
    def test_summarize_driver_reports_compares_prior_hits_and_search_outcomes(self):
        compare = load_module()
        baseline = {
            "cases": [
                {
                    "id": "case-a",
                    "outcome": {"complete_trajectory_found": False},
                    "best_complete_trajectory": {
                        "actions": [{"action_key": "combat/play_card/bash"}]
                    },
                    "stats": {"nodes_expanded": 10},
                    "best_frontier_value": {"terminal": "Unresolved", "player_hp": 30},
                    "diagnostics": {
                        "ordering": {
                            "root_action_prior_scored_states": 0,
                            "root_action_prior_scored_actions": 0,
                            "largest_reorders": [
                                {"first_action_key": "combat/play_card/bash"}
                            ],
                        }
                    },
                }
            ]
        }
        prior = {
            "cases": [
                {
                    "id": "case-a",
                    "outcome": {"complete_trajectory_found": True},
                    "best_complete_trajectory": {
                        "actions": [{"action_key": "combat/play_card/strike"}]
                    },
                    "stats": {"nodes_expanded": 8},
                    "best_frontier_value": {"terminal": "Win", "player_hp": 35},
                    "diagnostics": {
                        "ordering": {
                            "root_action_prior_scored_states": 1,
                            "root_action_prior_scored_actions": 3,
                            "largest_reorders": [
                                {"first_action_key": "combat/play_card/strike"}
                            ],
                        }
                    },
                }
            ]
        }

        summary = compare.summarize_driver_report_pair("bench-a", baseline, prior)

        self.assertEqual(summary["benchmark_name"], "bench-a")
        self.assertEqual(summary["case_count"], 1)
        self.assertEqual(summary["prior_scored_states"], 1)
        self.assertEqual(summary["prior_scored_actions"], 3)
        self.assertEqual(summary["complete_found_delta"], 1)
        self.assertEqual(summary["best_complete_first_action_changed"], 1)
        self.assertEqual(summary["ordering_first_reorder_sample_changed"], 1)
        self.assertEqual(summary["nodes_expanded_delta"], -2)
        self.assertEqual(summary["frontier_hp_delta"], 5)


if __name__ == "__main__":
    unittest.main()
