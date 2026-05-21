from __future__ import annotations

import io
import sys
import unittest
from contextlib import redirect_stdout
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
LLM_DIR = ROOT / "tools" / "llm"
if str(LLM_DIR) not in sys.path:
    sys.path.insert(0, str(LLM_DIR))

from sts_agent.ui.recording_choice_view import (  # noqa: E402
    RecordingView,
    parse_recording_command,
    parse_recording_raw_action_token,
    print_recording_raw_actions,
    validate_recording_view,
    watch_candidate_recording_lines,
)
from sts_agent.ui.recording_combat_ui import (  # noqa: E402
    build_action_id_display_command_map,
    build_combat_recording_view,
    build_combat_target_picker_view,
    combat_target_picker_prompt,
    combat_target_picker_unknown_message,
    parse_combat_target_picker_command,
    parse_recording_combat_command,
    recording_combat_command_error,
)


def _combat_payload() -> dict:
    return {
        "decision_type": "combat",
        "combat": {
            "energy": 3,
            "player_block": 0,
            "visible_incoming_damage": 7,
            "turn_count": 0,
            "hand_cards": [
                {
                    "hand_index": 0,
                    "card_instance_id": 1000,
                    "card_id": "Defend",
                    "cost_for_turn": 3,
                    "playable": True,
                },
                {
                    "hand_index": 1,
                    "card_instance_id": 1001,
                    "card_id": "Strike",
                    "cost_for_turn": 1,
                    "playable": True,
                },
                {
                    "hand_index": 6,
                    "card_instance_id": 1006,
                    "card_id": "Bash",
                    "cost_for_turn": 1,
                    "playable": True,
                },
            ],
            "combat_context": {
                "monsters": [
                    {
                        "slot": 0,
                        "name": "Louse",
                        "hp": 15,
                        "max_hp": 15,
                        "block": 0,
                        "visible_intent": "Unknown",
                        "planned_move_id": 4,
                    },
                    {
                        "slot": 1,
                        "name": "Louse",
                        "hp": 15,
                        "max_hp": 15,
                        "block": 0,
                        "visible_intent": "Unknown",
                        "planned_move_id": 3,
                    },
                ]
            },
        },
    }


def _candidate(
    action_id: int,
    action_key: str,
    label: str,
    kind: str,
    detail: str | None = None,
) -> dict:
    out = {
        "id": action_id,
        "action_key": action_key,
        "recording_label": label,
        "recording_kind": kind,
    }
    if detail:
        out["recording_detail"] = detail
    return out


def _combat_candidates() -> list[dict]:
    return [
        _candidate(0, "combat/end_turn", "End turn", "combat_end_turn"),
        _candidate(
            1,
            "combat/play_card/card:Defend/hand:0/target:none",
            "Play Defend h0 -> none",
            "combat_play_card",
        ),
        _candidate(
            2,
            "combat/play_card/card:Strike/hand:1/target:monster_slot:0",
            "Play Strike h1 -> monster_slot:0 Louse hp=15/15",
            "combat_play_card",
        ),
        _candidate(
            3,
            "combat/play_card/card:Strike/hand:1/target:monster_slot:1",
            "Play Strike h1 -> monster_slot:1 Louse hp=15/15",
            "combat_play_card",
        ),
        _candidate(
            4,
            "combat/play_card/card:Bash/hand:6/target:monster_slot:0",
            "Play Bash h6 -> monster_slot:0 Louse hp=15/15",
            "combat_play_card",
        ),
        _candidate(
            5,
            "combat/play_card/card:Bash/hand:6/target:monster_slot:1",
            "Play Bash h6 -> monster_slot:1 Louse hp=15/15",
            "combat_play_card",
        ),
    ]


def _combat_candidates_targeting_b_only() -> list[dict]:
    return [
        candidate
        for candidate in _combat_candidates()
        if "target:monster_slot:0" not in candidate.get("action_key", "")
    ]


def _hand_select_candidates() -> list[dict]:
    return [
        _candidate(
            20,
            "combat/hand_select/uuids:1001",
            "Select hand Strike",
            "pending_hand_select",
        ),
        _candidate(
            21,
            "combat/hand_select/uuids:1000",
            "Select hand Defend",
            "pending_hand_select",
        ),
    ]


class RecordingCommandProtocolTest(unittest.TestCase):
    def setUp(self) -> None:
        self.payload = _combat_payload()
        self.candidates = _combat_candidates()

    def parse_compact(self, command: str) -> dict | None:
        return parse_recording_combat_command(command, self.payload, self.candidates, {}, None)

    def build_view(self):
        view = build_combat_recording_view(self.payload, self.candidates, {}, None)
        self.assertIsNotNone(view)
        return view

    def test_recording_view_invariants(self) -> None:
        view = self.build_view()
        self.assertEqual(validate_recording_view(view), [])
        self.assertIn("6a", view.leaves_by_command)
        self.assertIn("0", view.leaves_by_command)
        self.assertIn("@1", view.raw_leaves_by_command or {})
        self.assertIn("1", view.groups_by_stem)

    def test_action_id_display_command_map_prefers_main_view_commands(self) -> None:
        view = self.build_view()
        display = build_action_id_display_command_map(view)
        self.assertEqual(display[2], "1A")
        self.assertEqual(display[3], "1B")
        self.assertEqual(display[4], "6A")
        self.assertEqual(display[1], "0")
        self.assertNotEqual(display[2], "A")
        self.assertNotEqual(display[2], "@2")
        self.assertEqual(display[0], "e")

    def test_other_section_explains_potion_command_when_empty(self) -> None:
        view = self.build_view()
        text = "\n".join(view.lines)
        self.assertIn("p | Potions none", text)

    def test_shadow_frontier_uses_display_commands_not_action_keys(self) -> None:
        view = build_combat_recording_view(
            self.payload,
            self.candidates,
            {},
            {"frontier_action_ids": [2, 3, 4], "unresolved_frontier": {"unresolved_count": 1}},
        )
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("frontier=1A / 1B / 6A", text)
        self.assertNotIn("combat/play_card", text)

    def test_shadow_frontier_falls_back_to_raw_command(self) -> None:
        view = build_combat_recording_view(
            self.payload,
            self.candidates,
            {},
            {"frontier_action_ids": [999]},
        )
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("frontier=@999", text)

    def test_combat_hud_renders_resources_effects_and_explicit_costs(self) -> None:
        view = self.build_view()
        text = "\n".join(view.lines)
        self.assertIn("Resources:", text)
        self.assertIn("Energy 3 | Block 0 | Incoming 7 | Turn 0", text)
        self.assertIn("Player effects: unknown/not provided", text)
        self.assertIn("0 | [cost 3] Defend play: 0", text)
        self.assertIn("1 | [cost 1] Strike play: 1A / 1B", text)
        self.assertNotIn(" c1 play:", text)
        self.assertNotIn(" c3 play:", text)

    def test_enemy_intent_summary_is_rendered_without_raw_move_id(self) -> None:
        payload = _combat_payload()
        payload["combat"]["combat_context"]["monsters"][0]["visible_intent"] = "Attack 7"
        payload["combat"]["combat_context"]["monsters"][0]["visible_intent_kind"] = "Attack"
        view = build_combat_recording_view(payload, self.candidates, {}, None)
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("A | slot0 Louse hp=15/15 block=0 intent=Attack 7", text)
        self.assertNotIn("move=4", text)

    def test_combat_hud_renders_player_effects_when_observation_provides_them(self) -> None:
        payload = _combat_payload()
        payload["combat"]["player_effects"] = [
            {
                "name": "Confused",
                "description": "drawn card costs randomized 0-3",
                "source": "SneckoEye",
            }
        ]
        view = build_combat_recording_view(payload, self.candidates, {}, None)
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("Player effects:", text)
        self.assertIn("Confused [drawn card costs randomized 0-3; source=SneckoEye]", text)

    def test_compact_target_command_executes_atomic_action(self) -> None:
        chosen = self.parse_compact("6A")
        self.assertIsNotNone(chosen)
        self.assertEqual(chosen["id"], 4)

    def test_targetless_card_uses_display_command_not_raw_action_zero(self) -> None:
        chosen = self.parse_compact("0")
        self.assertIsNotNone(chosen)
        self.assertEqual(chosen["id"], 1)
        self.assertNotEqual(chosen["id"], 0)

    def test_bare_targeted_hand_index_does_not_fallback_to_raw_action(self) -> None:
        self.assertIsNone(self.parse_compact("1"))
        view = self.build_view()
        result = parse_recording_command(view, "1")
        self.assertEqual(result.kind, "needs_target")
        self.assertEqual(result.group.stem if result.group else None, "1")

    def test_target_picker_executes_same_actions_as_shortcuts(self) -> None:
        parent = self.build_view()
        picker = build_combat_target_picker_view(parent, "1")
        self.assertIsNotNone(picker)
        self.assertEqual(validate_recording_view(picker), [])
        shortcut_a = parse_recording_command(parent, "1A")
        shortcut_b = parse_recording_command(parent, "1B")
        target_a = parse_recording_command(picker, "A")
        target_b = parse_recording_command(picker, "B")
        self.assertEqual(target_a.action_id, shortcut_a.action_id)
        self.assertEqual(target_b.action_id, shortcut_b.action_id)

    def test_target_picker_prompt_uses_current_view_targets_only(self) -> None:
        parent = self.build_view()
        picker = build_combat_target_picker_view(parent, "1")
        self.assertIsNotNone(picker)
        self.assertEqual(combat_target_picker_prompt(picker), "target=A/B|back|@id|d|q > ")
        b_only = RecordingView(
            mode=picker.mode,
            lines=picker.lines,
            leaves_by_command={k: v for k, v in picker.leaves_by_command.items() if k == "b"},
            groups_by_stem=picker.groups_by_stem,
            global_commands=picker.global_commands,
            raw_leaves_by_command=picker.raw_leaves_by_command,
            extras=picker.extras,
        )
        self.assertEqual(combat_target_picker_prompt(b_only), "target=B|back|@id|d|q > ")
        self.assertIn("Use B, back", combat_target_picker_unknown_message(b_only))

    def test_target_picker_unknown_target_does_not_fallback(self) -> None:
        parent = self.build_view()
        picker = build_combat_target_picker_view(parent, "1")
        self.assertIsNotNone(picker)
        self.assertEqual(parse_recording_command(picker, "C").kind, "unknown")

    def test_defeated_enemy_display_hides_intent_and_move(self) -> None:
        payload = _combat_payload()
        payload["combat"]["combat_context"]["monsters"][0]["hp"] = 0
        view = build_combat_recording_view(payload, _combat_candidates_targeting_b_only(), {}, None)
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("A | slot0 Louse defeated", text)
        self.assertNotIn("A | slot0 Louse hp=0/15", text)
        self.assertNotIn("A | slot0 Louse defeated block=", text)

    def test_same_slot_new_alive_enemy_does_not_inherit_defeated_display(self) -> None:
        payload = _combat_payload()
        payload["combat"]["combat_context"]["monsters"][0]["name"] = "Gremlin Wizard"
        payload["combat"]["combat_context"]["monsters"][0]["hp"] = 20
        view = build_combat_recording_view(payload, self.candidates, {}, None)
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("A | slot0 Gremlin Wizard hp=20/15", text)
        self.assertNotIn("A | slot0 Gremlin Wizard defeated", text)

    def test_lifecycle_state_takes_precedence_over_hp_fallback(self) -> None:
        payload = _combat_payload()
        payload["combat"]["combat_context"]["monsters"][0]["hp"] = 0
        payload["combat"]["combat_context"]["monsters"][0]["name"] = "SlimeBoss"
        payload["combat"]["combat_context"]["monsters"][0]["lifecycle_state"] = "splitting"
        view = build_combat_recording_view(payload, _combat_candidates_targeting_b_only(), {}, None)
        self.assertIsNotNone(view)
        text = "\n".join(view.lines)
        self.assertIn("A | slot0 SlimeBoss splitting", text)
        self.assertNotIn("A | slot0 SlimeBoss defeated", text)

    def test_combat_hand_select_uses_combat_pending_selection_view(self) -> None:
        payload = _combat_payload()
        payload["decision_type"] = "combat_hand_select"
        payload["combat"]["pending_choice"] = {
            "kind": "hand_select",
            "min_select": 1,
            "max_select": 1,
            "reason": "Upgrade",
            "options": [
                {
                    "option_index": 0,
                    "label": "Strike",
                    "card_uuid": 1001,
                    "selection_uuids": [1001],
                    "before_summary": "Strike [dmg 6]",
                    "after_summary": "Strike+1 [dmg 9]",
                    "delta_summary": "upgrade: dmg 6 -> 9",
                    "preview_status": "available",
                },
                {
                    "option_index": 1,
                    "label": "Defend",
                    "card_uuid": 1000,
                    "selection_uuids": [1000],
                    "before_summary": "Defend [block 5]",
                    "after_summary": "Defend+1 [block 8]",
                    "delta_summary": "upgrade: block 5 -> 8",
                    "preview_status": "available",
                },
            ],
        }
        view = build_combat_recording_view(payload, self.candidates, {}, None)
        self.assertIsNone(view)
        from sts_agent.ui.recording_combat_ui import recording_combat_choice_tree

        tree = recording_combat_choice_tree(payload, _hand_select_candidates(), {}, None)
        self.assertIsNotNone(tree)
        text = "\n".join(tree["lines"])
        self.assertIn("Resources:", text)
        self.assertIn("Pending hand selection: Upgrade; choose 1 card(s)", text)
        self.assertIn(
            "0 | [cost 1] Strike h1 -> Strike+1 [dmg 9] [upgrade: dmg 6 -> 9] select: 0",
            text,
        )
        self.assertIn(
            "1 | [cost 3] Defend h0 -> Defend+1 [block 8] [upgrade: block 5 -> 8] select: 1",
            text,
        )
        self.assertEqual(tree["command_to_action_id"]["0"], 20)
        self.assertEqual(tree["command_to_action_id"]["1"], 21)

    def test_target_picker_back_and_raw_namespace_are_explicit(self) -> None:
        parent = self.build_view()
        picker = build_combat_target_picker_view(parent, "1")
        self.assertIsNotNone(picker)
        self.assertEqual(parse_combat_target_picker_command(picker, "back"), "back")
        self.assertEqual(parse_combat_target_picker_command(picker, "@1"), 1)

    def test_invalid_display_command_is_not_interpreted_as_action(self) -> None:
        self.assertIsNone(self.parse_compact("2C"))
        self.assertIsNone(parse_recording_raw_action_token("2C", self.candidates))

    def test_parser_uses_view_command_map_without_action_key_fallback(self) -> None:
        view = self.build_view()
        stripped = RecordingView(
            mode=view.mode,
            lines=view.lines,
            leaves_by_command={k: v for k, v in view.leaves_by_command.items() if k != "6a"},
            groups_by_stem=view.groups_by_stem,
            global_commands=view.global_commands,
            raw_leaves_by_command=view.raw_leaves_by_command,
            extras=view.extras,
        )
        result = parse_recording_command(stripped, "6A")
        self.assertEqual(result.kind, "unknown")

    def test_target_picker_uses_own_command_map_without_parent_fallback(self) -> None:
        parent = self.build_view()
        picker = build_combat_target_picker_view(parent, "1")
        self.assertIsNotNone(picker)
        stripped = RecordingView(
            mode=picker.mode,
            lines=picker.lines,
            leaves_by_command={k: v for k, v in picker.leaves_by_command.items() if k != "a"},
            groups_by_stem=picker.groups_by_stem,
            global_commands=picker.global_commands,
            raw_leaves_by_command=picker.raw_leaves_by_command,
            extras=picker.extras,
        )
        self.assertEqual(parse_recording_command(stripped, "A").kind, "unknown")

    def test_raw_action_namespace_requires_at_prefix(self) -> None:
        self.assertIsNone(parse_recording_raw_action_token("1", self.candidates))
        chosen = parse_recording_raw_action_token("@1", self.candidates)
        self.assertIsNotNone(chosen)
        self.assertEqual(chosen["id"], 1)

    def test_invalid_raw_action_namespace_does_not_fallback(self) -> None:
        self.assertIsNone(parse_recording_raw_action_token("@999", self.candidates))
        self.assertIsNone(parse_recording_raw_action_token("@bad", self.candidates))

    def test_raw_action_list_displays_at_namespace(self) -> None:
        stream = io.StringIO()
        with redirect_stdout(stream):
            print_recording_raw_actions(self.candidates, self.payload, {})
        text = stream.getvalue()
        self.assertIn("@0 | End turn", text)
        self.assertIn("@1 | Play Defend h0 -> none", text)
        self.assertNotIn("  0 | End turn", text)
        self.assertNotIn("  1 | Play Defend h0 -> none", text)

    def test_non_combat_recording_menu_still_uses_visible_menu_numbers(self) -> None:
        candidates = [
            _candidate(10, "event/choice/0", "First event choice", "event_choice"),
            _candidate(11, "event/choice/1", "Second event choice", "event_choice"),
        ]
        lines = watch_candidate_recording_lines(candidates, {}, {})
        self.assertIn("  0 | First event choice [id=10]", lines)
        self.assertIn("  1 | Second event choice [id=11]", lines)


if __name__ == "__main__":
    unittest.main()
