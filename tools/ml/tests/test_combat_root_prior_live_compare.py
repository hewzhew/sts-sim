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
        self.assertEqual(len(summary["case_deltas"]), 1)
        self.assertEqual(summary["case_deltas"][0]["case_id"], "case-a")
        self.assertEqual(
            summary["case_deltas"][0]["baseline_best_complete_first_action"],
            "combat/play_card/bash",
        )
        self.assertEqual(
            summary["case_deltas"][0]["prior_best_complete_first_action"],
            "combat/play_card/strike",
        )
        self.assertEqual(summary["case_deltas"][0]["prior_scored_actions"], 3)

    def test_live_prior_decision_rejects_when_hits_have_no_positive_effect(self):
        compare = load_module()
        summary = {
            "case_count": 30,
            "prior_scored_states": 30,
            "prior_scored_actions": 95,
            "complete_found_delta": 0,
            "frontier_hp_delta": 0,
            "nodes_expanded_delta": 13,
            "best_complete_first_action_changed": 2,
        }

        decision = compare.live_prior_effect_decision(summary)

        self.assertEqual(decision["recommendation"], "do_not_enable_live_prior_yet")
        self.assertIn("prior_hits_without_outcome_gain", decision["evidence"])
        self.assertIn("prior_increased_nodes", decision["limitations"])


if __name__ == "__main__":
    unittest.main()
