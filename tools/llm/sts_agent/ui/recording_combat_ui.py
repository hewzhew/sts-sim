"""Combat-specific human-recording compact UI helpers."""

from __future__ import annotations

from typing import Any

from sts_agent.ui.recording_choice_view import (
    RecordingGroup,
    RecordingLeaf,
    RecordingView,
    parse_recording_command,
    recording_candidate_choice_option,
    recording_candidate_detail,
    recording_candidate_kind,
    recording_candidate_label,
)


def _recording_action_key(candidate: dict[str, Any]) -> str:
    payload = candidate.get("payload")
    payload_key = payload.get("action_key") if isinstance(payload, dict) else None
    return str(payload_key or candidate.get("action_key") or "")


def _parse_key_int_after(text: str, marker: str) -> int | None:
    if marker not in text:
        return None
    tail = text.split(marker, 1)[1].split("/", 1)[0]
    try:
        return int(tail)
    except Exception:
        return None


def _combat_target_slot(action_key: str) -> int | None:
    if "target:none" in action_key:
        return None
    return _parse_key_int_after(action_key, "target:monster_slot:")


def _combat_hand_index(action_key: str) -> int | None:
    return _parse_key_int_after(action_key, "hand:")


def _potion_slot(action_key: str) -> int | None:
    return _parse_key_int_after(action_key, "slot:")


def _combat_monster_entries(public_payload: dict[str, Any]) -> list[dict[str, Any]]:
    combat = public_payload.get("combat") or {}
    context = combat.get("combat_context") or {}
    monsters = context.get("monsters") if isinstance(context, dict) else None
    if not monsters:
        monsters = combat.get("monsters") or []
    out = []
    for fallback_slot, monster in enumerate(monsters):
        if not isinstance(monster, dict):
            continue
        if monster.get("alive") is False:
            continue
        slot = monster.get("slot")
        if slot is None:
            slot = fallback_slot
        out.append(
            {
                "letter": chr(ord("A") + len(out)),
                "slot": slot,
                "name": monster.get("name") or monster.get("monster_id") or f"monster{slot}",
                "hp": monster.get("hp", monster.get("current_hp")),
                "max_hp": monster.get("max_hp"),
                "block": monster.get("block"),
                "intent": monster.get("visible_intent")
                or monster.get("visible_intent_kind"),
                "intent_kind": monster.get("visible_intent_kind"),
                "intent_damage_per_hit": monster.get("visible_intent_damage_per_hit"),
                "intent_hits": monster.get("visible_intent_hits"),
                "intent_total_damage": monster.get("visible_intent_total_damage"),
                "move": monster.get("planned_move_id"),
                "lifecycle_state": monster.get("lifecycle_state"),
                "display_state": monster.get("display_state"),
                "enemy_state": monster.get("enemy_state"),
                "status": monster.get("status"),
                "phase": monster.get("phase"),
                "targetable": monster.get("targetable"),
            }
        )
    return out


def _combat_hand_entries(public_payload: dict[str, Any]) -> list[dict[str, Any]]:
    combat = public_payload.get("combat") or {}
    hand = combat.get("hand_cards") or []
    out = []
    for card in hand:
        if not isinstance(card, dict):
            continue
        hand_index = card.get("hand_index")
        if hand_index is None:
            continue
        name = str(card.get("card_id") or card.get("card_name") or f"h{hand_index}")
        if card.get("upgraded"):
            name += "+"
        cost = card.get("cost_for_turn")
        if cost is None:
            cost = card.get("cost")
        base_cost = card.get("base_cost")
        if base_cost is None:
            base_cost = card.get("base_cost_for_turn")
        out.append(
            {
                "hand_index": int(hand_index),
                "card_instance_id": card.get("card_instance_id") or card.get("uuid"),
                "card_id": card.get("card_id"),
                "name": name,
                "cost": cost,
                "base_cost": base_cost,
                "playable": card.get("playable"),
            }
        )
    return out


def _parse_uuid_list_from_action_key(action_key: str) -> tuple[int, ...]:
    marker = "uuids:"
    if marker not in action_key:
        return ()
    tail = action_key.split(marker, 1)[1].split("/", 1)[0]
    out = []
    for part in tail.split(","):
        try:
            out.append(int(part))
        except Exception:
            return ()
    return tuple(out)


def _hand_card_by_uuid(hand: list[dict[str, Any]]) -> dict[int, dict[str, Any]]:
    out = {}
    for card in hand:
        uuid = card.get("card_instance_id")
        try:
            out[int(uuid)] = card
        except Exception:
            continue
    return out


def _pending_choice_option_by_uuid(pending_choice: dict[str, Any]) -> dict[int, dict[str, Any]]:
    out = {}
    for option in pending_choice.get("options") or []:
        if not isinstance(option, dict):
            continue
        uuid = option.get("card_uuid")
        if uuid is None:
            uuids = option.get("selection_uuids") or []
            uuid = uuids[0] if len(uuids) == 1 else None
        try:
            out[int(uuid)] = option
        except Exception:
            continue
    return out


def _pending_option_preview_text(
    option: dict[str, Any] | None,
    reason: str,
    candidate_option: dict[str, Any] | None = None,
) -> str:
    preview = option or candidate_option
    if not preview:
        if reason.lower() == "upgrade":
            return "[upgrade preview unavailable]"
        return "[choice preview unavailable]"
    delta = preview.get("delta_summary")
    after = preview.get("after_summary")
    before = preview.get("before_summary")
    status = str(preview.get("preview_status") or "")
    if delta and after:
        return f"-> {after} [{delta}]"
    if delta:
        return f"[{delta}]"
    if after:
        return f"-> {after}"
    if before:
        return f"[{before}]"
    if reason.lower() == "upgrade":
        return "[upgrade preview unavailable]"
    if status.startswith("unavailable"):
        return f"[preview {status}]"
    return ""


def _monster_hp_leq_zero(monster: dict[str, Any]) -> bool:
    hp = monster.get("hp")
    if hp is None:
        hp = monster.get("current_hp")
    try:
        return int(hp) <= 0
    except Exception:
        return False


def _enemy_lifecycle_display_state(monster: dict[str, Any]) -> str | None:
    # Display-only state. Engine lifecycle/replay must remain based on
    # simulator fields and legal actions, not this UI fallback.
    for key in ("lifecycle_state", "display_state", "enemy_state", "status", "phase"):
        value = monster.get(key)
        if value is None:
            continue
        normalized = str(value).strip().lower()
        if normalized in {"splitting", "split", "split_pending"}:
            return "splitting"
        if normalized in {"retreated", "escaped", "fled"}:
            return "retreated"
        if normalized in {"defeated", "dead", "dying"}:
            return "defeated"
        if normalized in {"spawning", "summoning"}:
            return "spawning"
        if normalized in {"inactive", "not_targetable"}:
            return "inactive"
    return None


def _enemy_display_state(monster: dict[str, Any], targetable_slots: set[int]) -> str | None:
    lifecycle_state = _enemy_lifecycle_display_state(monster)
    if lifecycle_state:
        return lifecycle_state
    slot = monster.get("slot")
    try:
        slot_id = int(slot)
    except Exception:
        slot_id = slot
    if _monster_hp_leq_zero(monster) and slot_id not in targetable_slots:
        return "defeated"
    if monster.get("targetable") is False:
        return "inactive"
    return None


def _format_cost_text(card: dict[str, Any]) -> str:
    cost = card.get("cost")
    base_cost = card.get("base_cost")
    if cost is None:
        return "[cost ?]"
    if base_cost is not None and base_cost != cost:
        return f"[cost {cost}<-base{base_cost}]"
    return f"[cost {cost}]"


def _resources_lines(combat: dict[str, Any]) -> list[str]:
    energy = combat.get("energy")
    max_energy = combat.get("max_energy")
    if max_energy is None:
        max_energy = combat.get("player_max_energy")
    energy_text = f"{energy}/{max_energy}" if max_energy is not None else str(energy)
    return [
        "Resources:",
        (
            f"  Energy {energy_text} | Block {combat.get('player_block')} | "
            f"Incoming {combat.get('visible_incoming_damage')} | Turn {combat.get('turn_count')}"
        ),
    ]


def _effect_name(effect: dict[str, Any]) -> str:
    return str(
        effect.get("name")
        or effect.get("power_id")
        or effect.get("id")
        or effect.get("effect_id")
        or effect.get("kind")
        or "effect"
    )


def _effect_description(effect: dict[str, Any]) -> str:
    parts = []
    amount = effect.get("amount")
    if amount is not None:
        parts.append(f"amount={amount}")
    description = effect.get("description") or effect.get("text")
    if description:
        parts.append(str(description))
    source = effect.get("source")
    if source:
        parts.append(f"source={source}")
    return "; ".join(parts)


def _player_effect_entries(public_payload: dict[str, Any]) -> list[str] | None:
    combat = public_payload.get("combat") or {}
    context = combat.get("combat_context") or {}
    candidates = [
        combat.get("player_effects"),
        combat.get("player_powers"),
        combat.get("player_debuffs"),
        combat.get("powers"),
    ]
    if isinstance(context, dict):
        candidates.extend(
            [
                context.get("player_effects"),
                context.get("player_powers"),
                context.get("player_debuffs"),
            ]
        )
    for effects in candidates:
        if effects is None:
            continue
        if not effects:
            return []
        out = []
        for effect in effects:
            if isinstance(effect, dict):
                desc = _effect_description(effect)
                out.append(_effect_name(effect) + (f" [{desc}]" if desc else ""))
            else:
                out.append(str(effect))
        return out
    return None


def _player_effect_lines(public_payload: dict[str, Any]) -> list[str]:
    effects = _player_effect_entries(public_payload)
    if effects is None:
        return ["Player effects: unknown/not provided"]
    if not effects:
        return ["Player effects: none"]
    return ["Player effects:"] + [f"  {effect}" for effect in effects]


def _safe_intent_label(intent: Any) -> str:
    if not intent:
        return "Unknown"
    text = str(intent)
    if "{" in text or "(" in text:
        return "Known"
    return text


def _combat_state_lines(public_payload: dict[str, Any], targetable_slots: set[int] | None = None) -> list[str]:
    targetable_slots = targetable_slots or set()
    lines: list[str] = []
    combat = public_payload.get("combat") or {}
    lines.extend(_resources_lines(combat))
    lines.extend(_player_effect_lines(public_payload))
    monsters = _combat_monster_entries(public_payload)
    if monsters:
        lines.append("Enemies:")
        for monster in monsters:
            display_state = _enemy_display_state(monster, targetable_slots)
            if display_state:
                lines.append(f"  {monster['letter']} | slot{monster['slot']} {monster['name']} {display_state}")
                continue
            intent_text = f" intent={_safe_intent_label(monster.get('intent'))}"
            lines.append(
                f"  {monster['letter']} | slot{monster['slot']} {monster['name']} "
                f"hp={monster.get('hp')}/{monster.get('max_hp')} block={monster.get('block')}"
                + intent_text
            )
    return lines


def _card_name_from_recording_label(candidate: dict[str, Any], hand_index: int) -> str | None:
    label, resolved = recording_candidate_label(candidate, {})
    if not resolved:
        return None
    prefix = "Play "
    marker = f" h{hand_index}"
    if label.startswith(prefix) and marker in label:
        return label[len(prefix): label.index(marker)]
    return None


def build_action_id_display_command_map(view: RecordingView) -> dict[int, str]:
    out: dict[int, str] = {}
    for command, leaf in view.leaves_by_command.items():
        if command.startswith("@"):
            continue
        if not isinstance(leaf.action_id, int):
            continue
        current = out.get(leaf.action_id)
        display_command = command if command in {"e", "end"} else command.upper()
        if current is None or (current in {"e", "end"} and command not in {"e", "end"}):
            out[leaf.action_id] = display_command
    for command, leaf in (view.raw_leaves_by_command or {}).items():
        if not isinstance(leaf.action_id, int):
            continue
        out.setdefault(leaf.action_id, command)
    return out


def combat_target_picker_prompt(view: RecordingView) -> str:
    targets = "/".join(command.upper() for command in sorted(view.leaves_by_command))
    target_text = targets if targets else "?"
    return f"target={target_text}|back|@id|d|q > "


def combat_target_picker_unknown_message(view: RecordingView) -> str:
    targets = "/".join(command.upper() for command in sorted(view.leaves_by_command))
    target_text = targets if targets else "listed target"
    return f"Unknown target command. Use {target_text}, back, @id, d, or q."


def build_combat_recording_view(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    discouraged_reasons: dict[Any, str] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> RecordingView | None:
    if public_payload.get("decision_type") != "combat":
        return None
    monsters = _combat_monster_entries(public_payload)
    slot_to_letter = {monster["slot"]: monster["letter"] for monster in monsters}
    hand = _combat_hand_entries(public_payload)
    if not hand:
        return None

    by_hand: dict[int, dict[str, Any]] = {}
    potion_actions: list[tuple[str, dict[str, Any]]] = []
    end_turn: dict[str, Any] | None = None
    leaves_by_command: dict[str, RecordingLeaf] = {}
    raw_leaves_by_command: dict[str, RecordingLeaf] = {}
    groups_by_stem: dict[str, RecordingGroup] = {}
    frontier_ids = set((search_shadow_opinion or {}).get("frontier_action_ids") or [])
    shadow_by_hand: dict[int, set[str]] = {}
    targetable_slots: set[int] = set()

    for candidate in candidates:
        action_key = _recording_action_key(candidate)
        label, _ = recording_candidate_label(candidate, public_payload)
        raw_command = f"@{candidate.get('id')}".lower()
        raw_leaves_by_command[raw_command] = RecordingLeaf(
            command=raw_command,
            action_id=candidate.get("id"),
            label=label,
            action_key=action_key,
        )
        kind = recording_candidate_kind(candidate)
        if kind == "combat_end_turn" or action_key == "combat/end_turn":
            end_turn = candidate
            leaves_by_command["e"] = RecordingLeaf("e", candidate.get("id"), "End turn", action_key)
            leaves_by_command["end"] = RecordingLeaf("end", candidate.get("id"), "End turn", action_key)
            continue
        if kind == "combat_play_card" or action_key.startswith("combat/play_card/"):
            hand_index = _combat_hand_index(action_key)
            if hand_index is None:
                continue
            target_slot = _combat_target_slot(action_key)
            row = by_hand.setdefault(hand_index, {"none": None, "targets": {}, "candidates": []})
            row["candidates"].append(candidate)
            if target_slot is None:
                row["none"] = candidate
                command = str(hand_index).lower()
                leaves_by_command[command] = RecordingLeaf(command, candidate.get("id"), label, action_key)
                if candidate.get("id") in frontier_ids:
                    shadow_by_hand.setdefault(hand_index, set()).add("self")
            else:
                letter = slot_to_letter.get(target_slot)
                if letter:
                    targetable_slots.add(target_slot)
                    row["targets"][letter] = candidate
                    command = f"{hand_index}{letter}".lower()
                    leaves_by_command[command] = RecordingLeaf(command, candidate.get("id"), label, action_key)
                    if candidate.get("id") in frontier_ids:
                        shadow_by_hand.setdefault(hand_index, set()).add(letter)
            continue
        if kind == "potion_use" or action_key.startswith("potion/use/"):
            slot = _potion_slot(action_key)
            if slot is None:
                continue
            target_slot = _combat_target_slot(action_key)
            if target_slot is None:
                command = f"p{slot}"
            else:
                letter = slot_to_letter.get(target_slot)
                if not letter:
                    continue
                targetable_slots.add(target_slot)
                command = f"p{slot}{letter}"
            command = command.lower()
            leaves_by_command[command] = RecordingLeaf(command, candidate.get("id"), label, action_key)
            potion_actions.append((command, candidate))
            continue
        if kind == "potion_discard" or action_key.startswith("potion/discard/"):
            slot = _potion_slot(action_key)
            if slot is not None:
                command = f"x{slot}".lower()
                leaves_by_command[command] = RecordingLeaf(command, candidate.get("id"), label, action_key)
                potion_actions.append((command, candidate))

    lines: list[str] = []
    lines.extend(_combat_state_lines(public_payload, targetable_slots))
    lines.append("Hand:")
    for card in hand:
        hand_index = card["hand_index"]
        row = by_hand.get(hand_index, {})
        candidates_for_card = row.get("candidates") or []
        display_name = card["name"]
        if candidates_for_card:
            display_name = _card_name_from_recording_label(candidates_for_card[0], hand_index) or display_name
        commands = []
        targets = row.get("targets") or {}
        if targets:
            commands.extend(f"{hand_index}{letter}" for letter in sorted(targets.keys()))
        if row.get("none") is not None:
            commands.append(str(hand_index))
        if targets and row.get("none") is None:
            stem = str(hand_index).lower()
            groups_by_stem[stem] = RecordingGroup(
                stem=stem,
                label=f"{display_name} h{hand_index}",
                choices=tuple(f"{hand_index}{letter}" for letter in sorted(targets.keys())),
            )
        if not commands:
            commands.append("-")
        shadow_targets = shadow_by_hand.get(hand_index) or set()
        shadow = ""
        if shadow_targets:
            shown = "/".join(sorted(shadow_targets))
            shadow = f" [shadow:{shown}]"
        cost_text = _format_cost_text(card)
        playable = "" if card.get("playable") is not False else " [unplayable]"
        lines.append(
            f"  {hand_index} | {cost_text} {display_name} play: {' / '.join(commands)}{shadow}{playable}"
        )
    lines.append("Other:")
    if end_turn is not None:
        suffix = ""
        if discouraged_reasons and end_turn.get("id") in discouraged_reasons:
            suffix = f" [discouraged: {discouraged_reasons.get(end_turn.get('id'))}]"
        lines.append(f"  e | End turn{suffix}")
    if potion_actions:
        lines.append(f"  p | Potions collapsed; {len(potion_actions)} action(s). Type p to list.")
    else:
        lines.append("  p | Potions none")
    lines.append("  r | Raw legal action list")
    shadow = search_shadow_opinion or {}
    if shadow:
        temp_view = RecordingView(
            mode="combat",
            lines=tuple(lines),
            leaves_by_command=leaves_by_command,
            groups_by_stem=groups_by_stem,
            raw_leaves_by_command=raw_leaves_by_command,
        )
        command_by_action_id = build_action_id_display_command_map(temp_view)
        frontier_ids = shadow.get("frontier_action_ids") or []
        frontier_commands = [
            command_by_action_id.get(action_id, f"@{action_id}")
            for action_id in frontier_ids
            if action_id is not None
        ]
        if not frontier_commands:
            primary_id = shadow.get("primary_action_id")
            if primary_id is not None:
                frontier_commands = [command_by_action_id.get(primary_id, f"@{primary_id}")]
        frontier = " / ".join(frontier_commands) if frontier_commands else "none"
        unresolved = (shadow.get("unresolved_frontier") or {}).get("unresolved_count")
        lines.append(
            "Search shadow: "
            + f"frontier={frontier} "
            + (f"unresolved={unresolved} " if unresolved is not None else "")
            + "(record only; whole-combat comparison later)"
        )
    return RecordingView(
        mode="combat",
        lines=tuple(lines),
        leaves_by_command=leaves_by_command,
        groups_by_stem=groups_by_stem,
        global_commands=frozenset({"d", "detail", "details", "p", "potion", "potions", "r", "raw", "s", "save", "case", "q", "quit", "stop"}),
        raw_leaves_by_command=raw_leaves_by_command,
        extras={"potion_actions": potion_actions},
    )


def build_combat_pending_hand_select_recording_view(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    discouraged_reasons: dict[Any, str] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> RecordingView | None:
    combat = public_payload.get("combat") or {}
    pending_choice = combat.get("pending_choice") or {}
    if public_payload.get("decision_type") != "combat_hand_select":
        return None
    if pending_choice.get("kind") not in {None, "hand_select"}:
        return None
    hand = _combat_hand_entries(public_payload)
    if not hand:
        return None
    card_by_uuid = _hand_card_by_uuid(hand)
    option_by_uuid = _pending_choice_option_by_uuid(pending_choice)
    leaves_by_command: dict[str, RecordingLeaf] = {}
    raw_leaves_by_command: dict[str, RecordingLeaf] = {}
    lines: list[str] = []
    lines.extend(_combat_state_lines(public_payload, set()))
    reason = pending_choice.get("reason") or "hand_select"
    min_select = pending_choice.get("min_select")
    max_select = pending_choice.get("max_select")
    select_range = (
        f"{min_select}" if min_select == max_select else f"{min_select}-{max_select}"
    )
    lines.append(f"Pending hand selection: {reason}; choose {select_range} card(s)")
    lines.append("Choices:")
    discouraged_reasons = discouraged_reasons or {}
    for menu_index, candidate in enumerate(candidates):
        action_key = _recording_action_key(candidate)
        label, _ = recording_candidate_label(candidate, public_payload)
        command = str(menu_index).lower()
        raw_command = f"@{candidate.get('id')}".lower()
        raw_leaves_by_command[raw_command] = RecordingLeaf(
            command=raw_command,
            action_id=candidate.get("id"),
            label=label,
            action_key=action_key,
        )
        leaves_by_command[command] = RecordingLeaf(
            command=command,
            action_id=candidate.get("id"),
            label=label,
            action_key=action_key,
        )
        uuid_list = _parse_uuid_list_from_action_key(action_key)
        card = card_by_uuid.get(uuid_list[0]) if len(uuid_list) == 1 else None
        option = option_by_uuid.get(uuid_list[0]) if len(uuid_list) == 1 else None
        if card:
            card_label = (
                f"{_format_cost_text(card)} {card['name']} h{card['hand_index']}"
            )
        else:
            card_label = label
        preview = _pending_option_preview_text(
            option,
            str(reason),
            recording_candidate_choice_option(candidate),
        )
        if preview:
            card_label = f"{card_label} {preview}"
        suffix = ""
        if candidate.get("id") in discouraged_reasons:
            suffix = f" [discouraged: {discouraged_reasons.get(candidate.get('id'))}]"
        lines.append(f"  {menu_index} | {card_label} select: {menu_index}{suffix}")
    lines.append("Other:")
    lines.append("  r | Raw legal action list")
    if search_shadow_opinion:
        lines.append("Search shadow: available (record only; whole-combat comparison later)")
    return RecordingView(
        mode="combat_hand_select",
        lines=tuple(lines),
        leaves_by_command=leaves_by_command,
        groups_by_stem={},
        global_commands=frozenset({"d", "detail", "details", "r", "raw", "s", "save", "case", "q", "quit", "stop"}),
        raw_leaves_by_command=raw_leaves_by_command,
    )


def build_combat_like_recording_view(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    discouraged_reasons: dict[Any, str] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> RecordingView | None:
    return build_combat_recording_view(
        public_payload,
        candidates,
        discouraged_reasons,
        search_shadow_opinion,
    ) or build_combat_pending_hand_select_recording_view(
        public_payload,
        candidates,
        discouraged_reasons,
        search_shadow_opinion,
    )


def recording_combat_choice_tree(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    discouraged_reasons: dict[Any, str] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> dict[str, Any] | None:
    view = build_combat_like_recording_view(public_payload, candidates, discouraged_reasons, search_shadow_opinion)
    if not view:
        return None
    return {
        "lines": list(view.lines),
        "command_to_action_id": {
            command: leaf.action_id
            for command, leaf in view.leaves_by_command.items()
        },
        "groups_by_stem": {
            stem: {"label": group.label, "choices": list(group.choices), "behavior": group.behavior}
            for stem, group in view.groups_by_stem.items()
        },
        "potion_actions": (view.extras or {}).get("potion_actions") or [],
        "view": view,
    }


def build_combat_target_picker_view(parent_view: RecordingView, stem: str) -> RecordingView | None:
    token = stem.strip().lower()
    group = parent_view.groups_by_stem.get(token)
    if not group:
        return None
    leaves: dict[str, RecordingLeaf] = {}
    lines = [f"Target for {group.label}:"]
    for choice in group.choices:
        choice_key = choice.lower()
        parent_leaf = parent_view.leaves_by_command.get(choice_key)
        if not parent_leaf:
            continue
        target_command = choice[len(group.stem):].lower()
        if not target_command:
            continue
        leaves[target_command] = RecordingLeaf(
            command=target_command,
            action_id=parent_leaf.action_id,
            label=parent_leaf.label,
            action_key=parent_leaf.action_key,
        )
        label = parent_leaf.label
        marker = " -> "
        if marker in label:
            label = label.split(marker, 1)[1]
        lines.append(f"  {target_command.upper()} | {label}")
    lines.append("  back | Return to combat command view")
    return RecordingView(
        mode="combat_target_picker",
        lines=tuple(lines),
        leaves_by_command=leaves,
        groups_by_stem={},
        global_commands=frozenset({"back", "b", "d", "detail", "details", "q", "quit", "stop"}),
        raw_leaves_by_command=parent_view.raw_leaves_by_command,
        extras={"parent_stem": group.stem, "parent_label": group.label},
    )


def combat_target_picker_for_command(
    command: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> RecordingView | None:
    policy = action_candidate_policy or {}
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    parent_view = build_combat_recording_view(public_payload, candidates, discouraged_reasons, search_shadow_opinion)
    if not parent_view:
        return None
    result = parse_recording_command(parent_view, command)
    if result.kind != "needs_target" or not result.group:
        return None
    return build_combat_target_picker_view(parent_view, result.group.stem)


def parse_recording_combat_command(
    command: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> dict[str, Any] | None:
    policy = action_candidate_policy or {}
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    view = build_combat_like_recording_view(public_payload, candidates, discouraged_reasons, search_shadow_opinion)
    if not view:
        return None
    result = parse_recording_command(view, command)
    if result.kind != "execute" or result.action_id is None:
        return None
    for candidate in candidates:
        if candidate.get("id") == result.action_id:
            return candidate
    return None


def parse_combat_target_picker_command(view: RecordingView, command: str) -> int | str | None:
    result = parse_recording_command(view, command)
    if result.kind == "execute":
        return result.action_id
    if result.kind == "global" and result.command in {"back", "b"}:
        return "back"
    if result.kind == "global" and result.command in {"q", "quit", "stop"}:
        return "stop"
    return None


def recording_combat_command_error(
    command: str,
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> str | None:
    policy = action_candidate_policy or {}
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    view = build_combat_like_recording_view(public_payload, candidates, discouraged_reasons, search_shadow_opinion)
    if not view:
        return None
    result = parse_recording_command(view, command)
    if result.kind == "needs_target":
        return result.message
    if (result.command or "").isdigit():
        if view.mode != "combat":
            return f"Choice {result.command} is not shown in this decision frame."
        return (
            "Bare numbers in combat compact view are hand commands only. "
            "Use a shown command like 1A, e, p, r, d, s, q, or raw @id."
        )
    return None


def print_recording_potion_actions(
    public_payload: dict[str, Any],
    candidates: list[dict[str, Any]],
    action_candidate_policy: dict[str, Any] | None,
    search_shadow_opinion: dict[str, Any] | None,
) -> None:
    policy = action_candidate_policy or {}
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    view = build_combat_recording_view(public_payload, candidates, discouraged_reasons, search_shadow_opinion)
    potion_actions = ((view.extras if view else {}) or {}).get("potion_actions") or []
    if not potion_actions:
        print("No potion actions available.", flush=True)
        return
    print("Potion actions:", flush=True)
    for command, candidate in potion_actions:
        label, _ = recording_candidate_label(candidate, public_payload)
        print(f"  {command} | {label}", flush=True)
