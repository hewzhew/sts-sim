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

    def test_prior_hint_records_aggregate_candidate_scores_by_first_action(self):
        baseline = load_module()
        groups = {
            "root-a": [
                {
                    "schema_name": "CombatTurnPlanProbeSampleV1",
                    "source": {"source_file": "case-a.json"},
                    "root_context": {
                        "enumeration": {"root_exact_state_hash": "abc123"},
                    },
                    "plan": {
                        "plan_index": 0,
                        "first_action_key": "combat/play_card/hand:0/card:Strike_R+0#1/target:monster_slot:0",
                    },
                    "target": {
                        "terminal": "win",
                        "complete_win": True,
                        "final_hp": 40,
                    },
                },
                {
                    "schema_name": "CombatTurnPlanProbeSampleV1",
                    "source": {"source_file": "case-a.json"},
                    "root_context": {
                        "enumeration": {"root_exact_state_hash": "abc123"},
                    },
                    "plan": {
                        "plan_index": 1,
                        "first_action_key": "combat/end_turn",
                    },
                    "target": {
                        "terminal": "win",
                        "complete_win": True,
                        "final_hp": 35,
                    },
                },
                {
                    "schema_name": "CombatTurnPlanProbeSampleV1",
                    "source": {"source_file": "case-a.json"},
                    "root_context": {
                        "enumeration": {"root_exact_state_hash": "abc123"},
                    },
                    "plan": {
                        "plan_index": 2,
                        "first_action_key": "combat/play_card/hand:0/card:Strike_R+0#1/target:monster_slot:0",
                    },
                    "target": {
                        "terminal": "win",
                        "complete_win": True,
                        "final_hp": 42,
                    },
                },
            ]
        }
        scores = {"root-a": [0.2, -1.0, 0.7]}

        records = baseline.prior_hint_records_from_scores(
            groups,
            scores,
            target_mode="equivalent-hp-outcome",
            model_id="unit-test-prior",
        )

        self.assertEqual(len(records), 1)
        self.assertEqual(records[0]["schema_name"], "CombatRootActionPriorHintV0")
        self.assertEqual(records[0]["root_exact_state_hash"], "abc123")
        self.assertEqual(records[0]["action_prior_hints"][0]["action_key"], "combat/play_card/hand:0/card:Strike_R+0#1/target:monster_slot:0")
        self.assertEqual(records[0]["action_prior_hints"][0]["score"], 0.7)
        self.assertEqual(records[0]["action_prior_hints"][0]["candidate_count"], 2)
        self.assertEqual(records[0]["action_prior_hints"][1]["action_key"], "combat/end_turn")

    def test_tactical_utility_target_does_not_treat_same_hp_dirty_plan_as_equivalent(self):
        baseline = load_module()
        clean_same_hp = {
            "schema_name": "CombatTurnPlanProbeSampleV1",
            "plan": {
                "plan_index": 0,
                "plan_summary": {
                    "hp_lost_to_plan_boundary": 0,
                    "enemy_hp_removed_to_plan_boundary": 18,
                    "enemy_kill_count_to_plan_boundary": 0,
                    "potion_actions": 0,
                    "cards_played": 2,
                },
            },
            "target": {
                "terminal": "win",
                "complete_win": True,
                "final_hp": 40,
                "child_search_hp_loss": 0,
                "nodes_expanded": 40,
                "is_best_target_plan": True,
                "is_equivalent_hp_outcome_target_plan": True,
            },
        }
        dirty_same_hp = {
            "schema_name": "CombatTurnPlanProbeSampleV1",
            "plan": {
                "plan_index": 1,
                "plan_summary": {
                    "hp_lost_to_plan_boundary": 0,
                    "enemy_hp_removed_to_plan_boundary": 18,
                    "enemy_kill_count_to_plan_boundary": 0,
                    "potion_actions": 1,
                    "cards_played": 5,
                },
            },
            "target": {
                "terminal": "win",
                "complete_win": True,
                "final_hp": 40,
                "child_search_hp_loss": 0,
                "nodes_expanded": 200,
                "is_best_target_plan": False,
                "is_equivalent_hp_outcome_target_plan": True,
            },
        }
        worse_hp = {
            "schema_name": "CombatTurnPlanProbeSampleV1",
            "plan": {
                "plan_index": 2,
                "plan_summary": {
                    "hp_lost_to_plan_boundary": 2,
                    "enemy_hp_removed_to_plan_boundary": 25,
                    "enemy_kill_count_to_plan_boundary": 1,
                    "potion_actions": 0,
                    "cards_played": 2,
                },
            },
            "target": {
                "terminal": "win",
                "complete_win": True,
                "final_hp": 38,
                "child_search_hp_loss": 2,
                "nodes_expanded": 20,
                "is_best_target_plan": False,
                "is_equivalent_hp_outcome_target_plan": False,
            },
        }
        group = [clean_same_hp, dirty_same_hp, worse_hp]

        self.assertEqual(
            baseline.positive_target_indices(group, "equivalent-hp-outcome"),
            [0, 1],
        )
        self.assertEqual(
            baseline.positive_target_indices(group, "tactical-utility"),
            [0],
        )


if __name__ == "__main__":
    unittest.main()
