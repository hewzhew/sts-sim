import importlib.util
import unittest
from pathlib import Path


def load_module():
    root = Path(__file__).resolve().parents[3]
    module_path = root / "tools" / "ml" / "combat_first_action_ranking_baseline.py"
    spec = importlib.util.spec_from_file_location("combat_first_action_ranking_baseline", module_path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)
    return module


class CombatFirstActionRankingBaselineTests(unittest.TestCase):
    def test_prior_effect_decision_recommends_ordering_only_when_coverage_is_low(self):
        baseline = load_module()
        summary = {
            "usable_group_count": 89,
            "root_action_mask_coverage": {
                "candidate_first_action_coverage_ratio": 0.30,
            },
            "metrics": {
                "ordered_index_all": {
                    "avg_hp_regret_to_target": 2.16,
                },
                "logistic_source_cv": {
                    "groups": 89,
                    "avg_hp_gain_vs_ordered": 1.46,
                    "avg_hp_regret_to_target": 0.70,
                    "target_outcome_match_rate": 0.831,
                    "negative_hp_gain": 3,
                },
            },
            "best_target_mode_by_hp_regret": {
                "target_mode": "equivalent-hp-outcome",
                "avg_hp_regret_to_target": 0.26,
            },
        }

        decision = baseline.prior_effect_decision(summary)

        self.assertEqual(decision["schema_name"], "CombatPriorEffectDecisionV0")
        self.assertEqual(decision["recommendation"], "try_root_ordering_only")
        self.assertIn("do_not_prune_low_root_action_coverage", decision["constraints"])
        self.assertEqual(decision["best_target_mode"], "equivalent-hp-outcome")
        self.assertEqual(decision["observed"]["model_hp_gain_vs_ordered"], 1.46)


if __name__ == "__main__":
    unittest.main()
