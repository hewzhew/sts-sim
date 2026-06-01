"""Generic human-recording command helpers.

This module owns UI command namespaces that are not specific to a
particular Slay the Spire screen. It does not interpret game semantics.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class RecordingLeaf:
    command: str
    action_id: int
    label: str
    action_key: str | None = None


@dataclass(frozen=True)
class RecordingGroup:
    stem: str
    label: str
    choices: tuple[str, ...]
    behavior: str = "needs_target"


@dataclass(frozen=True)
class RecordingParseResult:
    kind: str
    action_id: int | None = None
    command: str | None = None
    message: str | None = None
    group: RecordingGroup | None = None
    view: "RecordingView | None" = None


@dataclass(frozen=True)
class RecordingView:
    mode: str
    lines: tuple[str, ...]
    leaves_by_command: dict[str, RecordingLeaf]
    groups_by_stem: dict[str, RecordingGroup]
    global_commands: frozenset[str] = frozenset()
    raw_leaves_by_command: dict[str, RecordingLeaf] | None = None
    extras: dict[str, Any] | None = None

    def all_leaves_by_command(self) -> dict[str, RecordingLeaf]:
        out = dict(self.leaves_by_command)
        out.update(self.raw_leaves_by_command or {})
        return out


def normalize_recording_command(command: str) -> str:
    return command.strip().lower()


def parse_recording_command(view: RecordingView, command: str) -> RecordingParseResult:
    token = normalize_recording_command(command)
    leaf = view.leaves_by_command.get(token)
    if leaf:
        return RecordingParseResult(kind="execute", action_id=leaf.action_id, command=leaf.command)
    raw_leaf = (view.raw_leaves_by_command or {}).get(token)
    if raw_leaf:
        return RecordingParseResult(kind="execute", action_id=raw_leaf.action_id, command=raw_leaf.command)
    group = view.groups_by_stem.get(token)
    if group:
        choices = " / ".join(group.choices)
        return RecordingParseResult(
            kind=group.behavior,
            command=token,
            message=f"{group.label} needs a target: use {choices}.",
            group=group,
        )
    if token in view.global_commands:
        return RecordingParseResult(kind="global", command=token)
    return RecordingParseResult(kind="unknown", command=token)


def validate_recording_view(view: RecordingView) -> list[str]:
    errors: list[str] = []
    seen: set[str] = set()
    for namespace, leaves in (
        ("leaf", view.leaves_by_command),
        ("raw", view.raw_leaves_by_command or {}),
    ):
        for command, leaf in leaves.items():
            if not command:
                errors.append(f"{namespace} has empty command")
            if command != normalize_recording_command(command):
                errors.append(f"{namespace} command is not normalized: {command}")
            if command in seen:
                errors.append(f"duplicate command: {command}")
            seen.add(command)
            if leaf.command != command:
                errors.append(f"leaf command mismatch: {command} != {leaf.command}")
            if not isinstance(leaf.action_id, int):
                errors.append(f"leaf has no action_id: {command}")
    for stem, group in view.groups_by_stem.items():
        if stem in seen:
            errors.append(f"group stem conflicts with executable command: {stem}")
        if group.stem != stem:
            errors.append(f"group stem mismatch: {stem} != {group.stem}")
        for choice in group.choices:
            if normalize_recording_command(choice) not in view.leaves_by_command:
                errors.append(f"group choice is not executable leaf: {stem}->{choice}")
    return errors


def _candidate_recording_value(candidate: dict[str, Any], field: str) -> Any:
    value = candidate.get(field)
    if value is not None:
        return value
    payload = candidate.get("payload")
    if isinstance(payload, dict):
        return payload.get(field)
    return None


def recording_candidate_label(candidate: dict[str, Any], public_payload: dict[str, Any]) -> tuple[str, bool]:
    label = _candidate_recording_value(candidate, "recording_label")
    if label:
        return str(label), True
    key = _candidate_recording_value(candidate, "action_key") or candidate.get("action_key") or candidate.get("id")
    return f"MISSING_RUST_RECORDING_LABEL: {key}", False


def recording_candidate_detail(candidate: dict[str, Any]) -> str | None:
    detail = _candidate_recording_value(candidate, "recording_detail")
    if detail:
        return str(detail)
    return None


def recording_candidate_kind(candidate: dict[str, Any]) -> str:
    return str(_candidate_recording_value(candidate, "recording_kind") or "")


def recording_candidate_choice_option(candidate: dict[str, Any]) -> dict[str, Any]:
    value = _candidate_recording_value(candidate, "choice_option")
    return value if isinstance(value, dict) else {}


def recording_candidate_confirmation_required(candidate: dict[str, Any]) -> bool:
    option = recording_candidate_choice_option(candidate)
    return bool(option.get("requires_confirmation"))


def recording_candidate_decision_suffix(candidate: dict[str, Any], display_command: str) -> str:
    option = recording_candidate_choice_option(candidate)
    if not option:
        return ""
    suffix = ""
    danger_flags = option.get("danger_flags") or []
    if danger_flags:
        suffix += f" [DANGER: {','.join(map(str, danger_flags))}]"
    if option.get("requires_confirmation"):
        suffix += f" [confirm: type !{display_command}]"
    return suffix


def recording_candidate_is_utility(candidate: dict[str, Any]) -> bool:
    kind = recording_candidate_kind(candidate)
    return kind in {
        "potion_discard",
        "potion_use",
        "cancel",
    }


def parse_recording_raw_action_token(
    command: str,
    candidates: list[dict[str, Any]],
) -> dict[str, Any] | None:
    token = command.strip()
    if not token.startswith("@"):
        return None
    try:
        action_id = int(token[1:])
    except ValueError:
        return None
    for candidate in candidates:
        if candidate.get("id") == action_id:
            return candidate
    return None


def print_recording_raw_actions(
    candidates: list[dict[str, Any]],
    public_payload: dict[str, Any],
    action_candidate_policy: dict[str, Any] | None,
) -> None:
    policy = action_candidate_policy or {}
    discouraged_actions = policy.get("dominated_or_discouraged_actions") or []
    discouraged_reasons: dict[Any, str] = {}
    for item in discouraged_actions:
        action = item.get("action") or {}
        discouraged_reasons[action.get("id")] = str(item.get("reason") or item.get("severity") or "discouraged")
    print("Raw legal action list:", flush=True)
    for candidate in candidates:
        label, _resolved_from_rust = recording_candidate_label(candidate, public_payload)
        suffix = ""
        if candidate.get("id") in discouraged_reasons:
            suffix += f" [discouraged: {discouraged_reasons.get(candidate.get('id'))}]"
        option = recording_candidate_choice_option(candidate)
        danger_flags = option.get("danger_flags") or []
        if danger_flags:
            suffix += f" [DANGER: {','.join(map(str, danger_flags))}]"
        print(f"  @{candidate.get('id')} | {label}{suffix}", flush=True)
        detail = recording_candidate_detail(candidate)
        if detail:
            print(f"       {detail}", flush=True)


def watch_candidate_recording_lines(
    candidates: list[dict[str, Any]],
    public_payload: dict[str, Any],
    discouraged_reasons: dict[Any, str] | None = None,
) -> list[str]:
    lines = []
    discouraged_reasons = discouraged_reasons or {}
    indexed = list(enumerate(candidates))
    groups = [
        ("Main choices:", [(idx, cand) for idx, cand in indexed if not recording_candidate_is_utility(cand)]),
        ("Utility actions:", [(idx, cand) for idx, cand in indexed if recording_candidate_is_utility(cand)]),
    ]
    for group_label, group_candidates in groups:
        if not group_candidates:
            continue
        if len(groups[0][1]) > 0 and len(groups[1][1]) > 0:
            lines.append(group_label)
        for menu_index, candidate in group_candidates:
            label, _resolved_from_rust = recording_candidate_label(candidate, public_payload)
            suffix = ""
            if candidate.get("id") in discouraged_reasons:
                suffix += f" [discouraged: {discouraged_reasons.get(candidate.get('id'))}]"
            suffix += recording_candidate_decision_suffix(candidate, str(menu_index))
            raw_id = candidate.get("id")
            raw_suffix = f" [id={raw_id}]" if raw_id != menu_index else ""
            lines.append(f"{menu_index:>3} | {label}{suffix}{raw_suffix}")
            detail = recording_candidate_detail(candidate)
            if detail:
                lines.append(f"      {detail}")
    return lines
