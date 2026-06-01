"""Interactive watch UI and decision-case capture helpers."""

from __future__ import annotations

from typing import Any

from sts_agent.context.action_candidates import build_action_descriptor_v1
from sts_agent.evidence.combat_search_view import watch_search_summary_lines, watch_shadow_opinion_lines
from sts_agent.ui.case_capture import save_decision_case_v1
from sts_agent.ui.recording_choice_view import (
    parse_recording_raw_action_token,
    print_recording_raw_actions,
    recording_candidate_confirmation_required,
    watch_candidate_recording_lines,
)
from sts_agent.ui.recording_combat_ui import (
    combat_target_picker_for_command,
    combat_target_picker_prompt,
    combat_target_picker_unknown_message,
    parse_combat_target_picker_command,
    parse_recording_combat_command,
    print_recording_potion_actions,
    recording_combat_choice_tree,
    recording_combat_command_error,
)
from sts_agent.briefs.decision_brief import (
    decision_lens_lines,
    llm_decision_summary_lines,
    print_decision_brief_prompt_section,
    print_full_decision_brief,
    print_full_llm_json,
)
from sts_agent.utils.llm_utils import (
    compact_json,
    find_candidate,
    map_room_label,
    map_route_context_lines,
    short_action_label,
)
from sts_agent.evidence.reward_metrics import reward_candidate_metrics_v1




def watch_state_lines(payload: dict[str, Any]) -> list[str]:
    lines = [
        (
            f"State: decision={payload.get('decision_type')} "
            f"engine={payload.get('engine_state')} room={payload.get('current_room')} "
            f"boss={payload.get('act_boss')}"
        ),
        (
            f"Run: act={payload.get('act')} floor={payload.get('floor')} "
            f"hp={payload.get('current_hp')}/{payload.get('max_hp')} "
            f"gold={payload.get('gold')} deck={payload.get('deck_size')}"
        ),
    ]
    deck = payload.get("deck") or {}
    if deck:
        lines.append(
            "Deck: "
            + f"atk={deck.get('attack_count')} skill={deck.get('skill_count')} "
            + f"power={deck.get('power_count')} draw={deck.get('draw_card_count')} "
            + f"scale={deck.get('scaling_card_count')} basic={deck.get('starter_basic_count')} "
            + f"avg_cost_milli={deck.get('average_cost_milli')}"
        )
    combat = payload.get("combat")
    if isinstance(combat, dict):
        context = combat.get("combat_context") or {}
        if isinstance(context, dict):
            lines.append(
                "Encounter: "
                + f"id={context.get('encounter_id')} level={context.get('context_level')} "
                + f"hint_source={context.get('hint_source')}"
            )
        lines.append(
            "Combat: "
            + f"hp={combat.get('player_hp')} block={combat.get('player_block')} "
            + f"energy={combat.get('energy')} turn={combat.get('turn_count')} "
            + f"incoming={combat.get('visible_incoming_damage')} "
            + f"monster_hp={combat.get('total_monster_hp')} "
            + f"draw={combat.get('draw_count')} discard={combat.get('discard_count')}"
        )
        monsters = (
            context.get("monsters")
            if isinstance(context, dict) and context.get("monsters") is not None
            else combat.get("monsters")
        ) or []
        for monster in monsters[:4]:
            if not isinstance(monster, dict):
                continue
            powers = monster.get("powers") or []
            power_text = ",".join(
                f"{power.get('power_id')}:{power.get('amount')}"
                for power in powers[:4]
                if isinstance(power, dict)
            )
            lines.append(
                "Monster: "
                + f"{monster.get('name') or monster.get('monster_id')} "
                + f"hp={monster.get('hp', monster.get('current_hp'))}/{monster.get('max_hp')} "
                + f"block={monster.get('block')} move={monster.get('planned_move_id')} "
                + f"intent={monster.get('visible_intent')}"
                + (f" powers={power_text}" if power_text else "")
            )
            hints = monster.get("mechanic_hints") or []
            for hint in hints[:2]:
                lines.append(f"Hint: {hint}")
        encounter_hints = (
            context.get("encounter_hints")
            if isinstance(context, dict) and context.get("encounter_hints") is not None
            else combat.get("encounter_hints")
        ) or []
        for hint in encounter_hints[:3]:
            lines.append(f"Hint: {hint}")
        hand = combat.get("hand_cards") or []
        if hand:
            lines.append("Hand:")
            for card in hand[:12]:
                tags = ",".join(card.get("base_semantics") or [])
                lines.append(
                    "  "
                    + f"h{card.get('hand_index')} {card.get('card_id')}"
                    + ("+" if card.get("upgraded") else "")
                    + f" cost={card.get('cost_for_turn')} playable={card.get('playable')}"
                    + (f" tags={tags}" if tags else "")
                )
        return lines
    screen = payload.get("screen") or {}
    active_counts = {
        key: value
        for key, value in screen.items()
        if isinstance(value, int) and value > 0
    }
    if active_counts:
        lines.append("Screen: " + ", ".join(f"{k}={v}" for k, v in active_counts.items()))
    route_lines = map_route_context_lines(payload.get("map_route_context"))
    if route_lines:
        lines.extend(route_lines)
    elif payload.get("next_nodes"):
        lines.append("Map choices:")
        for node in (payload.get("next_nodes") or [])[:8]:
            lines.append(
                "  "
                + f"x={node.get('x')} y={node.get('y')} "
                + map_room_label(node.get("room_type"), bool(node.get("has_emerald_key")))
            )
    rewards = screen.get("reward_items") or []
    if rewards:
        lines.append("Rewards:")
        for item in rewards[:6]:
            lines.append("  " + compact_json(item, limit=220))
    reward_card_choices = screen.get("reward_card_choices") or []
    if reward_card_choices:
        lines.append("Reward card choices:")
        for option in reward_card_choices[:6]:
            if not isinstance(option, dict):
                continue
            descriptor = option.get("semantic_descriptor") or {}
            label = descriptor.get("label") or option.get("card_name") or option.get("card_id")
            hints = ",".join(str(tag) for tag in (option.get("base_semantics") or [])[:8])
            metrics = reward_candidate_metrics_v1(payload, option)
            clash_metrics = (metrics.get("computed_metrics") or {}).get("clash_activation_cost")
            metric_text = ""
            if clash_metrics:
                values = clash_metrics.get("metrics") or {}
                metric_text = (
                    " | ClashActivation "
                    + f"clean={float(values.get('clean_hand_probability') or 0):.1%} "
                    + f"clearable={float(values.get('clearable_hand_probability_under_simple_energy_model') or 0):.1%} "
                    + f"E_cost={float(values.get('expected_clear_energy_cost_given_clearable') or 0):.2f} "
                    + f"E_skills={float(values.get('expected_skill_clear_count_given_clearable') or 0):.2f} "
                    + f"nob={(clash_metrics.get('context_penalties') or {}).get('gremlin_nob_skill_punish')}"
                )
            lines.append(
                "  "
                + f"{option.get('option_index')} {label} "
                + f"{option.get('card_type')} {option.get('rarity')} cost={option.get('cost')} "
                + f"hints={hints}{metric_text}"
            )
        lines.append("  note: hints are low-level handwritten hints; heuristic plan_delta is hidden from this view")
    return lines

def watch_candidate_lines(candidates: list[dict[str, Any]]) -> list[str]:
    lines = []
    for candidate in candidates:
        descriptor = candidate.get("action_descriptor") or build_action_descriptor_v1(
            candidate,
            {},
        )
        label = descriptor.get("label") or short_action_label(candidate)
        status = descriptor.get("semantic_status")
        unknown_fields = descriptor.get("unknown_fields") or []
        transition = str(descriptor.get("transition") or "")
        if status == "partial":
            reason = "exact outcome unresolved"
            label_lower = str(label or "").lower()
            if (
                "choose" in label_lower
                or any("followup" in str(field) or "selected" in str(field) for field in unknown_fields)
                or "Selection" in transition
            ):
                reason = "opens follow-up choice; exact offered/selected result unknown"
            elif any("relic" in str(field) for field in unknown_fields):
                reason = "exact relic unknown"
            elif any("card" in str(field) for field in unknown_fields):
                reason = "exact card unknown"
            suffix = f" [partial: {reason}]"
        elif status and status != "described":
            suffix = f" [{status}]"
        else:
            suffix = ""
        key = candidate.get("action_key")
        if key and key != label:
            lines.append(f"{candidate.get('id'):>3} | {label}{suffix} | {key}")
        else:
            lines.append(f"{candidate.get('id'):>3} | {label}{suffix}")
        costs = descriptor.get("costs") or []
        effects = descriptor.get("effects") or []
        if costs or effects:
            detail = []
            if costs:
                detail.append("costs=" + compact_json(costs, limit=160))
            if effects:
                detail.append("effects=" + compact_json(effects, limit=220))
            lines.append("      " + " ".join(detail))
    return lines

def recording_selectable_candidates(
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
) -> list[dict[str, Any]]:
    # Human recording must expose every legal action. Search/LLM policy may
    # discourage actions, but hiding them would corrupt a human demonstration.
    return candidates

def watch_decision_title(public_payload: dict[str, Any], decision_type: str | None) -> str:
    screen = public_payload.get("screen") or {}
    event_options = screen.get("event_options") or []
    if event_options:
        descriptor = (event_options[0] or {}).get("semantic_descriptor") or {}
        event_name = descriptor.get("event_name") or descriptor.get("event_id")
        if event_name and str(event_name).lower() == "neow":
            return "Neow start bonus"
        if event_name:
            return f"Event choice: {event_name}"
    if decision_type == "reward_card_choice":
        return "Reward card choice"
    if decision_type == "map":
        return "Map route choice"
    if decision_type == "combat":
        return "Combat action"
    return str(decision_type or "decision")

def watch_context_lines(public_payload: dict[str, Any]) -> list[str]:
    lines: list[str] = []
    deck = public_payload.get("deck") or {}
    if deck:
        avg = deck.get("average_cost_milli")
        try:
            avg_cost = f"{int(avg) / 1000:.2f}"
        except Exception:
            avg_cost = str(avg)
        lines.append(
            "Deck: "
            + f"cards={public_payload.get('deck_size')} "
            + f"attacks={deck.get('attack_count')} skills={deck.get('skill_count')} "
            + f"powers={deck.get('power_count')} draw={deck.get('draw_card_count')} "
            + f"scaling={deck.get('scaling_card_count')} basic={deck.get('starter_basic_count')} "
            + f"avg_cost={avg_cost}"
        )
    recording_view = public_payload.get("recording_view") or {}
    for line in (recording_view.get("state_lines") or [])[1:]:
        lines.append(str(line))
    route_lines = map_route_context_lines(public_payload.get("map_route_context"))
    if route_lines:
        lines.append("Strategic context:")
        lines.extend("  " + line for line in route_lines)
    elif public_payload.get("next_nodes"):
        lines.append("Map choices:")
        for node in (public_payload.get("next_nodes") or [])[:8]:
            lines.append(
                "  "
                + f"x={node.get('x')} y={node.get('y')} "
                + map_room_label(node.get("room_type"), bool(node.get("has_emerald_key")))
            )
    return lines

def watch_recording_context_lines(
    public_payload: dict[str, Any],
    combat_search_report: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> list[str]:
    lines: list[str] = []
    recording_view = public_payload.get("recording_view") or {}
    if not recording_view:
        lines.append("MISSING_RUST_RECORDING_VIEW")
    for line in recording_view.get("state_lines") or []:
        lines.append(str(line))
    for line in recording_view.get("context_lines") or []:
        lines.append(str(line))
    for line in recording_view.get("warning_lines") or []:
        lines.append("Warning: " + str(line))
    shadow = search_shadow_opinion or {}
    if shadow:
        primary = shadow.get("primary_action_key") or shadow.get("primary_action_id")
        frontier = shadow.get("frontier_action_ids") or []
        unresolved = (shadow.get("unresolved_frontier") or {}).get("unresolved_count")
        lines.append(
            "Search shadow: "
            + f"frontier={primary} "
            + (f"frontier_ids={frontier} " if frontier else "")
            + (f"unresolved={unresolved} " if unresolved is not None else "")
            + "(record only; whole-combat comparison later)"
        )
    elif combat_search_report:
        lines.append("Search shadow: available (record only; whole-combat comparison later)")
    return lines

def print_recording_watch_panel(
    *,
    step_index: int,
    decision_type: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    max_candidates: int,
    combat_search_report: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> None:
    width = 104
    title = watch_decision_title(public_payload, decision_type)
    print("\n" + "=" * width, flush=True)
    print(
        f"RECORD STEP {step_index} | {title} | Act {public_payload.get('act')} Floor {public_payload.get('floor')} | "
        f"HP {public_payload.get('current_hp')}/{public_payload.get('max_hp')} | "
        f"Gold {public_payload.get('gold')} | Boss {public_payload.get('act_boss')}",
        flush=True,
    )
    print("-" * width, flush=True)
    policy = action_candidate_policy or {}
    decision_candidates = recording_selectable_candidates(candidates, action_candidate_policy)
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    combat_tree = recording_combat_choice_tree(
        public_payload,
        decision_candidates,
        discouraged_reasons,
        search_shadow_opinion,
    )
    decision_frame = public_payload.get("decision_frame") or {}
    if decision_frame:
        prompt = decision_frame.get("prompt")
        if prompt:
            print(f"DecisionFrame: {prompt}", flush=True)
        source = decision_frame.get("source") or {}
        if isinstance(source, dict) and source.get("label"):
            print(f"Source: {source.get('label')}", flush=True)
        for warning in decision_frame.get("warnings") or []:
            print(f"Frame warning: {warning}", flush=True)
    if combat_tree:
        for line in (public_payload.get("recording_view") or {}).get("state_lines") or []:
            print(str(line), flush=True)
        for line in combat_tree.get("lines") or []:
            print(line, flush=True)
    else:
        print("Choose menu number:", flush=True)
        for line in watch_candidate_recording_lines(
            decision_candidates[:max_candidates],
            public_payload,
            discouraged_reasons=discouraged_reasons,
        ):
            print(line, flush=True)
        if len(decision_candidates) > max_candidates:
            print(f"... {len(decision_candidates) - max_candidates} more candidates hidden", flush=True)
        for line in watch_recording_context_lines(public_payload, combat_search_report, search_shadow_opinion):
            print(line, flush=True)
    print("-" * width, flush=True)

def print_watch_panel(
    *,
    step_index: int,
    decision_type: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    max_candidates: int,
    parsed: dict[str, Any] | None = None,
    action_id: int | None = None,
    validation: dict[str, Any] | None = None,
    combat_search_report: dict[str, Any] | None = None,
    search_shadow_opinion: dict[str, Any] | None = None,
    decision_brief: dict[str, Any] | None = None,
    phase: str = "post_llm",
    recording_mode: bool = False,
) -> None:
    if recording_mode:
        print_recording_watch_panel(
            step_index=step_index,
            decision_type=decision_type,
            public_payload=public_payload,
            candidates=candidates,
            action_candidate_policy=action_candidate_policy,
            max_candidates=max_candidates,
            combat_search_report=combat_search_report,
            search_shadow_opinion=search_shadow_opinion,
        )
        return
    width = 104
    title = watch_decision_title(public_payload, decision_type)
    print("\n" + "=" * width, flush=True)
    print(
        f"STEP {step_index} | {title} | Act {public_payload.get('act')} Floor {public_payload.get('floor')} | "
        f"HP {public_payload.get('current_hp')}/{public_payload.get('max_hp')} | "
        f"Gold {public_payload.get('gold')} | Boss {public_payload.get('act_boss')}",
        flush=True,
    )
    print("-" * width, flush=True)
    print(f"CURRENT DECISION: {title}", flush=True)
    policy = action_candidate_policy or {}
    decision_candidates = policy.get("decision_candidates") or candidates
    locked_actions = policy.get("locked_actions") or []
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    print(
        "Decision candidates: "
        + f"{len(decision_candidates)} shown / {len(candidates)} legal total",
        flush=True,
    )
    for line in watch_candidate_lines(decision_candidates[:max_candidates]):
        print(line, flush=True)
    if len(decision_candidates) > max_candidates:
        print(f"... {len(decision_candidates) - max_candidates} more decision candidates hidden", flush=True)
    if discouraged_actions:
        print("Discouraged legal actions:", flush=True)
        for item in discouraged_actions[:8]:
            action = item.get("action") or {}
            label = (action.get("action_descriptor") or {}).get("label") or short_action_label(action)
            print(
                f"{action.get('id'):>3} | {label} | {item.get('severity')} | {item.get('reason')}",
                flush=True,
            )
    if locked_actions:
        print("Locked legal actions:", flush=True)
        for item in locked_actions[:8]:
            action = item.get("action") or {}
            label = (action.get("action_descriptor") or {}).get("label") or short_action_label(action)
            print(
                f"{action.get('id'):>3} | {label} | locked: {item.get('reason')}",
                flush=True,
            )
    print("-" * width, flush=True)
    for line in watch_context_lines(public_payload):
        print(line, flush=True)
    if decision_type == "combat":
        for line in watch_search_summary_lines(combat_search_report):
            print(line, flush=True)
        for line in watch_shadow_opinion_lines(search_shadow_opinion):
            print(line, flush=True)
    lens_lines = decision_lens_lines(decision_brief)
    if lens_lines:
        print("Decision lens:", flush=True)
        for line in lens_lines:
            print("  " + line, flush=True)
    print("-" * width, flush=True)
    if phase == "pre_llm":
        print("LLM: not called yet", flush=True)
    else:
        for line in llm_decision_summary_lines(parsed, action_id):
            print(line, flush=True)
        selected = find_candidate(candidates, action_id) if action_id is not None else None
        validation_payload = validation if isinstance(validation, dict) else {}
        if not validation_payload.get("is_legal", bool(selected)):
            print(
                "Selected: "
                + f"id={action_id} key={(selected or {}).get('action_key')} "
                + f"valid={validation_payload.get('is_legal', bool(selected))}",
                flush=True,
            )

def watch_decision_interactively(
    *,
    step_index: int,
    decision_type: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    parsed: dict[str, Any],
    action_id: int,
    validation: dict[str, Any],
    combat_search_report: dict[str, Any] | None,
    action_candidate_policy: dict[str, Any] | None,
    max_candidates: int,
    search_shadow_opinion: dict[str, Any] | None = None,
    case_metadata: dict[str, Any] | None = None,
    llm_raw_text: str | None = None,
    decision_brief: dict[str, Any] | None = None,
    recording_mode: bool = False,
) -> dict[str, Any]:
    print_watch_panel(
        step_index=step_index,
        decision_type=decision_type,
        public_payload=public_payload,
        candidates=candidates,
        parsed=parsed,
        action_id=action_id,
        validation=validation,
        combat_search_report=combat_search_report,
        search_shadow_opinion=search_shadow_opinion,
        decision_brief=decision_brief,
        action_candidate_policy=action_candidate_policy,
        max_candidates=max_candidates,
        phase="post_llm",
        recording_mode=recording_mode,
    )
    validation_payload = validation if isinstance(validation, dict) else {}
    while True:
        try:
            command = input(
                "Enter/c=execute | v=brief | p=prompt brief | j=json | s=save case | q=stop | action_id=override > "
            ).strip()
        except EOFError:
            command = ""
        if command == "" or command.lower() == "c":
            return {"stop": False, "action_id": action_id, "override": None}
        if command.lower() in {"v", "view", "brief"}:
            print_full_decision_brief(decision_brief)
            continue
        if command.lower() in {"p", "prompt"}:
            print_decision_brief_prompt_section(decision_brief)
            continue
        if command.lower() in {"j", "json"}:
            print_full_llm_json(parsed)
            continue
        if command.lower() in {"s", "save", "case"}:
            try:
                human_note = input("case note (optional) > ").strip()
            except EOFError:
                human_note = ""
            try:
                suspected_issue = input("suspected issue (optional) > ").strip()
            except EOFError:
                suspected_issue = ""
            path = save_decision_case_v1(
                step_index=step_index,
                decision_type=decision_type,
                public_payload=public_payload,
                candidates=candidates,
                parsed=parsed if isinstance(parsed, dict) else {},
                action_id=action_id,
                validation=validation_payload,
                action_candidate_policy=action_candidate_policy,
                combat_search_report=combat_search_report,
                decision_brief=decision_brief,
                case_metadata=case_metadata,
                llm_raw_text=llm_raw_text,
                human_note=human_note,
                suspected_issue=suspected_issue,
            )
            print(f"saved case: {path}", flush=True)
            continue
        if command.lower() in {"q", "quit", "stop"}:
            return {"stop": True, "action_id": action_id, "override": None}
        try:
            override_id = int(command)
        except ValueError:
            print("Unrecognized command. Use Enter, q, or an action_id number.", flush=True)
            continue
        override_candidate = find_candidate(candidates, override_id)
        if not override_candidate:
            print(f"action_id {override_id} is not legal in this decision frame.", flush=True)
            continue
        return {
            "stop": False,
            "action_id": override_id,
            "override": {
                "source": "human_watch_window",
                "original_action_id": action_id,
                "original_action_key": (find_candidate(candidates, action_id) or {}).get("action_key"),
                "override_action_id": override_id,
                "override_action_key": override_candidate.get("action_key"),
            },
        }

def watch_before_llm_interactively(
    *,
    step_index: int,
    decision_type: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    combat_search_report: dict[str, Any] | None,
    action_candidate_policy: dict[str, Any] | None,
    max_candidates: int,
    search_shadow_opinion: dict[str, Any] | None = None,
    case_metadata: dict[str, Any] | None = None,
    decision_brief: dict[str, Any] | None = None,
    recording_mode: bool = False,
) -> dict[str, Any]:
    recording_candidates = recording_selectable_candidates(candidates, action_candidate_policy)
    print_watch_panel(
        step_index=step_index,
        decision_type=decision_type,
        public_payload=public_payload,
        candidates=candidates,
        parsed=None,
        action_id=None,
        validation=None,
        combat_search_report=combat_search_report,
        search_shadow_opinion=search_shadow_opinion,
        action_candidate_policy=action_candidate_policy,
        decision_brief=decision_brief,
        max_candidates=max_candidates,
        phase="pre_llm",
        recording_mode=recording_mode,
    )
    while True:
        try:
            if recording_mode:
                if public_payload.get("decision_type") == "combat":
                    command = input("command=<card>[target]|@id|e|p|r|d|s|q > ").strip()
                else:
                    command = input("choice=execute | d=details | s=save case | q=stop > ").strip()
            else:
                command = input(
                    "l=ask LLM | v=brief | p=prompt brief | s=save case | q=stop | action_id=manual execute > "
                ).strip()
        except EOFError:
            command = "q" if recording_mode else "l"
        if not recording_mode and command.lower() in {"", "l", "llm", "ask"}:
            return {"stop": False, "ask_llm": True, "action_id": None, "override": None}
        if recording_mode and command == "" and len(recording_candidates) == 1:
            only = recording_candidates[0]
            return {
                "stop": False,
                "ask_llm": False,
                "action_id": only.get("id"),
                "override": {
                    "source": "human_recording_single_candidate_enter",
                    "original_action_id": None,
                    "original_action_key": None,
                    "override_action_id": only.get("id"),
                    "override_action_key": only.get("action_key"),
                },
            }
        if recording_mode and command.lower() in {"d", "detail", "details"}:
            print_watch_panel(
                step_index=step_index,
                decision_type=decision_type,
                public_payload=public_payload,
                candidates=candidates,
                parsed=None,
                action_id=None,
                validation=None,
                combat_search_report=combat_search_report,
                search_shadow_opinion=search_shadow_opinion,
                action_candidate_policy=action_candidate_policy,
                decision_brief=decision_brief,
                max_candidates=max_candidates,
                phase="pre_llm",
                recording_mode=False,
            )
            continue
        if recording_mode and command.lower() in {"r", "raw"}:
            print_recording_raw_actions(recording_candidates, public_payload, action_candidate_policy)
            continue
        if recording_mode and command.lower() in {"p", "potion", "potions"}:
            print_recording_potion_actions(
                public_payload,
                recording_candidates,
                action_candidate_policy,
                search_shadow_opinion,
            )
            continue
        if command.lower() in {"v", "view", "brief"}:
            print_full_decision_brief(decision_brief)
            continue
        if command.lower() in {"p", "prompt"}:
            print_decision_brief_prompt_section(decision_brief)
            continue
        if command.lower() in {"q", "quit", "stop"}:
            return {"stop": True, "ask_llm": False, "action_id": None, "override": None}
        if command.lower() in {"s", "save", "case"}:
            try:
                human_note = input("case note (optional) > ").strip()
            except EOFError:
                human_note = ""
            try:
                suspected_issue = input("suspected issue (optional) > ").strip()
            except EOFError:
                suspected_issue = ""
            path = save_decision_case_v1(
                step_index=step_index,
                decision_type=decision_type,
                public_payload=public_payload,
                candidates=candidates,
                parsed={"status": "pre_llm_not_called"},
                action_id=None,
                validation={"is_legal": None, "reason": "pre_llm_case_capture"},
                action_candidate_policy=action_candidate_policy,
                combat_search_report=combat_search_report,
                decision_brief=decision_brief,
                case_metadata=case_metadata,
                llm_raw_text=None,
                human_note=human_note,
                suspected_issue=suspected_issue,
            )
            print(f"saved case: {path}", flush=True)
            continue
        if recording_mode:
            raw_candidate = parse_recording_raw_action_token(command, recording_candidates)
            if raw_candidate:
                raw_action_id = raw_candidate.get("id")
                return {
                    "stop": False,
                    "ask_llm": False,
                    "action_id": raw_action_id,
                    "override": {
                        "source": "human_recording_raw_token",
                        "menu_index": None,
                        "command": command,
                        "override_action_id": raw_action_id,
                        "override_action_key": raw_candidate.get("action_key"),
                    },
                }
            combat_tree = recording_combat_choice_tree(
                public_payload,
                recording_candidates,
                {},
                search_shadow_opinion,
            )
            if combat_tree:
                manual_candidate = parse_recording_combat_command(
                    command,
                    public_payload,
                    recording_candidates,
                    action_candidate_policy,
                    search_shadow_opinion,
                )
                if manual_candidate:
                    raw_action_id = manual_candidate.get("id")
                    return {
                        "stop": False,
                        "ask_llm": False,
                        "action_id": raw_action_id,
                        "override": {
                            "source": "human_recording_compact_command",
                            "menu_index": None,
                            "command": command,
                            "override_action_id": raw_action_id,
                            "override_action_key": manual_candidate.get("action_key"),
                        },
                    }
                target_view = combat_target_picker_for_command(
                    command,
                    public_payload,
                    recording_candidates,
                    action_candidate_policy,
                    search_shadow_opinion,
                )
                if target_view:
                    for line in target_view.lines:
                        print(line, flush=True)
                    while True:
                        try:
                            target_command = input(combat_target_picker_prompt(target_view)).strip()
                        except EOFError:
                            target_command = "back"
                        if target_command.lower() in {"d", "detail", "details"}:
                            for line in target_view.lines:
                                print(line, flush=True)
                            continue
                        target_result = parse_combat_target_picker_command(target_view, target_command)
                        if target_result == "back":
                            break
                        if target_result == "stop":
                            return {"stop": True, "ask_llm": False, "action_id": None, "override": None}
                        if isinstance(target_result, int):
                            manual_candidate = find_candidate(recording_candidates, target_result)
                            if not manual_candidate:
                                print(f"action_id {target_result} is not legal in this decision frame.", flush=True)
                                continue
                            return {
                                "stop": False,
                                "ask_llm": False,
                                "action_id": target_result,
                                "override": {
                                    "source": "human_recording_target_picker",
                                    "menu_index": None,
                                    "command": f"{command} {target_command}",
                                    "override_action_id": target_result,
                                    "override_action_key": manual_candidate.get("action_key"),
                                },
                            }
                        print(combat_target_picker_unknown_message(target_view), flush=True)
                    continue
                error = recording_combat_command_error(
                    command,
                    public_payload,
                    recording_candidates,
                    action_candidate_policy,
                    search_shadow_opinion,
                )
                print(
                    error
                    or "Unknown command in combat compact view. Use a shown command, @id, p, r, d, s, or q.",
                    flush=True,
                )
                continue
        confirmed_menu_command = False
        numeric_command = command
        if recording_mode and command.startswith("!"):
            confirmed_menu_command = True
            numeric_command = command[1:]
        try:
            manual_id = int(numeric_command)
        except ValueError:
            if recording_mode:
                print("Unrecognized command. Use a shown command/menu number, p, r, d, s, or q.", flush=True)
            else:
                print("Unrecognized command. Use l, s, q, or an action_id number.", flush=True)
            continue
        if recording_mode:
            if manual_id < 0 or manual_id >= len(recording_candidates):
                print(f"choice {manual_id} is not shown in this decision frame.", flush=True)
                continue
            manual_candidate = recording_candidates[manual_id]
            raw_action_id = manual_candidate.get("id")
        else:
            manual_candidate = find_candidate(candidates, manual_id)
            raw_action_id = manual_id
        if not manual_candidate:
            print(f"action_id {raw_action_id} is not legal in this decision frame.", flush=True)
            continue
        if (
            recording_mode
            and recording_candidate_confirmation_required(manual_candidate)
            and not confirmed_menu_command
        ):
            print(
                f"choice {manual_id} requires confirmation; type !{manual_id} to execute.",
                flush=True,
            )
            continue
        return {
            "stop": False,
            "ask_llm": False,
            "action_id": raw_action_id,
            "override": {
                "source": "human_recording_menu" if recording_mode else "human_pre_llm_watch_window",
                "menu_index": manual_id if recording_mode else None,
                "override_action_id": raw_action_id,
                "override_action_key": manual_candidate.get("action_key"),
            },
        }
