from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def load_trace(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def render_trace_markdown(trace: dict[str, Any]) -> str:
    lines: list[str] = []
    lines.append("# Combat Lab Trace")
    lines.append("")
    lines.append(f"- Fixture: `{trace.get('fixture_name', '<unknown>')}`")
    lines.append(f"- Episode: `{trace.get('episode_id', '<unknown>')}`")
    lines.append(f"- Variant: `{trace.get('variant_mode', '<unknown>')}`")
    lines.append(f"- Seed: `{trace.get('seed', '<unknown>')}`")
    lines.append(f"- Outcome: `{trace.get('outcome', '<unknown>')}`")
    lines.append(f"- Final player HP: `{trace.get('final_player_hp', '<unknown>')}`")
    lines.append(f"- Final monster HP: `{trace.get('final_monster_hp', '<unknown>')}`")
    lines.append(f"- Turns: `{trace.get('turns', '<unknown>')}`")
    path_score = trace.get("path_score")
    if isinstance(path_score, (int, float)):
        lines.append(f"- Path score: `{path_score:.2f}`")
    else:
        lines.append(f"- Path score: `{path_score}`")
    lines.append("")

    for step in trace.get("steps", []):
        lines.append(f"## Turn {step.get('turn_index', '?')} Step {step.get('step_index', '?')}")
        lines.append("")
        lines.append(
            "- Before: "
            f"hp=`{step.get('player_hp_before', '?')}` "
            f"block=`{step.get('player_block_before', '?')}` "
            f"energy=`{step.get('energy_before', '?')}` "
            f"hand_size=`{step.get('hand_size_before', '?')}` "
            f"draw=`{step.get('draw_size_before', '?')}` "
            f"discard=`{step.get('discard_size_before', '?')}` "
            f"eval=`{format_float(step.get('evaluate_state_before'))}`"
        )
        lines.append(
            f"- Hand before: {', '.join(step.get('hand_before', [])) or '<empty>'}"
        )
        monsters_before = step.get("monsters_before", [])
        lines.append(f"- Monsters before: {render_monsters(monsters_before)}")
        lines.append(
            f"- Action: `{step.get('chosen_action', '<unknown>')}` "
            f"kind=`{step.get('action_kind', '<unknown>')}` "
            f"payload=`{json.dumps(step.get('action_payload', {}), ensure_ascii=False)}`"
        )
        lines.append(
            "- After: "
            f"hp=`{step.get('player_hp_after', '?')}` "
            f"block=`{step.get('player_block_after', '?')}` "
            f"energy=`{step.get('energy_after', '?')}` "
            f"hand_size=`{step.get('hand_size_after', '?')}` "
            f"draw=`{step.get('draw_size_after', '?')}` "
            f"discard=`{step.get('discard_size_after', '?')}` "
            f"eval=`{format_float(step.get('evaluate_state_after'))}` "
            f"remaining_monster_hp=`{step.get('remaining_monster_hp_after_step', '?')}`"
        )
        lines.append(
            f"- Hand after: {', '.join(step.get('hand_after', [])) or '<empty>'}"
        )
        preview = step.get("state_features_preview", {})
        if isinstance(preview, dict) and preview:
            ordered = ", ".join(
                f"{key}={format_float(value)}" for key, value in sorted(preview.items())
            )
            lines.append(f"- Preview: {ordered}")
        lines.append("")

    return "\n".join(lines) + "\n"


def render_monsters(monsters: list[dict[str, Any]]) -> str:
    if not monsters:
        return "<none>"
    rendered: list[str] = []
    for monster in monsters:
        rendered.append(
            f"{monster.get('id', '<unknown>')} "
            f"hp={monster.get('hp', '?')}/{monster.get('max_hp', '?')} "
            f"block={monster.get('block', '?')} "
            f"intent={monster.get('intent', '<unknown>')} "
            f"powers=[{', '.join(monster.get('key_powers', []))}]"
        )
    return " | ".join(rendered)


def format_float(value: Any) -> str:
    if isinstance(value, (int, float)):
        return f"{value:.2f}"
    return str(value)


def main() -> int:
    parser = argparse.ArgumentParser(description="Render a combat_lab trace JSON as Markdown.")
    parser.add_argument("--trace", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()

    trace = load_trace(args.trace)
    markdown = render_trace_markdown(trace)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(markdown, encoding="utf-8")
    print(f"wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
