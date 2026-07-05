import importlib.util
import tempfile
import unittest
from pathlib import Path


def load_path_review():
    module_path = Path(__file__).resolve().parents[1] / "tools" / "path_review.py"
    spec = importlib.util.spec_from_file_location("path_review", module_path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PathReviewTests(unittest.TestCase):
    def test_renders_selected_choice_and_candidate_pool_reasons(self):
        path_review = load_path_review()
        payload = {
            "schema": "branch_tiny_run_path",
            "branch_id": 7,
            "steps": [
                {
                    "step": 3,
                    "state_before": {
                        "act": 2,
                        "floor": 17,
                        "hp": 44,
                        "max_hp": 80,
                        "gold": 72,
                        "deck_size": 18,
                        "boundary": "Shop",
                    },
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "rank": 1,
                            "selected": True,
                            "auto_expand": True,
                            "inspect_only": None,
                            "label": "Leave shop",
                            "annotation": {
                                "kind": "candidate",
                                "candidate": {"kind": "shop_leave"},
                                "lane": "mainline",
                                "score": 0,
                            },
                        },
                        {
                            "rank": 2,
                            "selected": False,
                            "auto_expand": False,
                            "inspect_only": "shop card would spend purge reserve despite hard gap",
                            "label": "Shrug It Off | 51 gold",
                            "annotation": {
                                "kind": "candidate",
                                "candidate": {
                                    "kind": "shop_buy_card",
                                    "card": "ShrugItOff",
                                    "upgrades": 0,
                                    "price": 51,
                                },
                                "lane": "reject",
                                "score": 42,
                            },
                        },
                    ],
                }
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "path.json"
            path.write_text(path_review.dumps_json(payload), encoding="utf-8")
            output = path_review.render_source(path)

        self.assertIn("A2F17 hp=44/80 gold=72 deck=18 boundary=Shop", output)
        self.assertIn("selected: Leave shop", output)
        self.assertIn("[x] 1. Leave shop", output)
        self.assertIn("[ ] 2. Shrug It Off | 51 gold", output)
        self.assertIn(
            "inspect=shop card would spend purge reserve despite hard gap", output
        )
        self.assertIn("shop_buy_card ShrugItOff+0 51g", output)

    def test_can_filter_to_interesting_shop_steps_with_summary(self):
        path_review = load_path_review()
        payload = {
            "schema": "branch_tiny_run_path",
            "branch_id": 7,
            "steps": [
                {
                    "state_before": {
                        "act": 1,
                        "floor": 4,
                        "hp": 80,
                        "max_hp": 80,
                        "gold": 99,
                        "deck_size": 12,
                        "boundary": "Card Reward",
                    },
                    "label": "Headbutt",
                    "candidate_pool": [
                        {
                            "rank": 1,
                            "selected": True,
                            "auto_expand": True,
                            "label": "Headbutt",
                            "annotation": {"kind": "candidate"},
                        }
                    ],
                },
                {
                    "state_before": {
                        "act": 2,
                        "floor": 17,
                        "hp": 44,
                        "max_hp": 80,
                        "gold": 72,
                        "deck_size": 18,
                        "boundary": "Shop",
                    },
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "rank": 1,
                            "selected": True,
                            "auto_expand": True,
                            "label": "Leave shop",
                            "annotation": {"kind": "candidate"},
                        },
                        {
                            "rank": 2,
                            "selected": False,
                            "auto_expand": False,
                            "inspect_only": "shop card has no acquisition policy support",
                            "label": "Combust | 28 gold",
                            "annotation": {
                                "kind": "candidate",
                                "candidate": {
                                    "kind": "shop_buy_card",
                                    "card": "Combust",
                                    "upgrades": 0,
                                    "price": 28,
                                },
                            },
                        },
                    ],
                },
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "path.json"
            path.write_text(path_review.dumps_json(payload), encoding="utf-8")
            output = path_review.render_source(
                path,
                boundaries={"shop"},
                interesting=True,
                show_summary=True,
            )

        self.assertIn("summary: paths=1 steps=2 shown=1 inspect_reasons=1", output)
        self.assertIn("boundary=Shop", output)
        self.assertIn("Combust | 28 gold", output)
        self.assertNotIn("boundary=Card Reward", output)

    def test_can_filter_steps_by_candidate_text(self):
        path_review = load_path_review()
        payload = {
            "schema": "branch_tiny_run_path",
            "branch_id": 7,
            "steps": [
                {
                    "state_before": {"act": 2, "floor": 17, "boundary": "Shop"},
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "rank": 1,
                            "selected": False,
                            "auto_expand": False,
                            "inspect_only": "shop card has no acquisition policy support",
                            "label": "Combust | 28 gold",
                            "annotation": {
                                "kind": "candidate",
                                "candidate": {
                                    "kind": "shop_buy_card",
                                    "card": "Combust",
                                    "upgrades": 0,
                                    "price": 28,
                                },
                            },
                        }
                    ],
                },
                {
                    "state_before": {"act": 2, "floor": 18, "boundary": "Shop"},
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "rank": 1,
                            "selected": False,
                            "auto_expand": False,
                            "inspect_only": "shop card would spend purge reserve despite hard gap",
                            "label": "Shrug It Off | 51 gold",
                            "annotation": {
                                "kind": "candidate",
                                "candidate": {
                                    "kind": "shop_buy_card",
                                    "card": "ShrugItOff",
                                    "upgrades": 0,
                                    "price": 51,
                                },
                            },
                        }
                    ],
                },
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "path.json"
            path.write_text(path_review.dumps_json(payload), encoding="utf-8")
            output = path_review.render_source(path, contains=["purge reserve"])

        self.assertIn("A2F18", output)
        self.assertIn("Shrug It Off | 51 gold", output)
        self.assertNotIn("Combust | 28 gold", output)

    def test_can_group_inspect_reasons(self):
        path_review = load_path_review()
        payload = {
            "schema": "branch_tiny_run_path",
            "branch_id": 7,
            "steps": [
                {
                    "state_before": {"act": 1, "floor": 7, "boundary": "Shop"},
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "label": "Shrug It Off | 51 gold",
                            "inspect_only": "shop card would spend purge reserve",
                        },
                        {
                            "label": "Combust | 28 gold",
                            "inspect_only": "shop card has no acquisition policy support",
                        },
                    ],
                },
                {
                    "state_before": {"act": 2, "floor": 18, "boundary": "Shop"},
                    "label": "Leave shop",
                    "candidate_pool": [
                        {
                            "label": "Iron Wave | 47 gold",
                            "inspect_only": "shop card would spend purge reserve",
                        }
                    ],
                },
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "path.json"
            path.write_text(path_review.dumps_json(payload), encoding="utf-8")
            output = path_review.render_source(path, inspect_summary=True)

        self.assertIn("inspect_summary: paths=1 steps=2 shown=2 inspect_reasons=3", output)
        self.assertIn("- 2x shop card would spend purge reserve", output)
        self.assertIn("- 1x shop card has no acquisition policy support", output)
        self.assertIn("path.json#00 A1F7 Shop", output)
        self.assertIn("candidate='Shrug It Off | 51 gold'", output)

    def test_flags_hidden_auto_transition_between_path_steps(self):
        path_review = load_path_review()
        payload = {
            "schema": "branch_tiny_run_path",
            "branch_id": 7,
            "steps": [
                {
                    "state_before": {
                        "act": 3,
                        "floor": 36,
                        "hp": 103,
                        "max_hp": 103,
                        "gold": 315,
                        "deck_size": 17,
                        "boundary": "Shop",
                    },
                    "label": "Leave shop",
                    "candidate_pool": [],
                },
                {
                    "state_before": {
                        "act": 3,
                        "floor": 39,
                        "hp": 63,
                        "max_hp": 103,
                        "gold": 342,
                        "deck_size": 16,
                        "boundary": "Card Reward",
                    },
                    "label": "Shrug It Off+1",
                    "candidate_pool": [],
                },
            ],
        }
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "path.json"
            path.write_text(path_review.dumps_json(payload), encoding="utf-8")
            output = path_review.render_source(path)

        self.assertIn(
            "auto transition: A3F36 -> A3F39 hp 103->63 gold 315->342 deck 17->16",
            output,
        )


if __name__ == "__main__":
    unittest.main()
