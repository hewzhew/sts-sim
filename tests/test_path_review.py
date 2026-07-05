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


if __name__ == "__main__":
    unittest.main()
