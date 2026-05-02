#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Render one combat turn plan-probe JSON into a compact human review report. "
            "This is a diagnostic viewer, not a training-data builder."
        )
    )
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    parser.add_argument("--top-sequences", type=int, default=12)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def esc(value: Any) -> str:
    return html.escape(str(value if value is not None else ""))


def action_label(action_key: Any) -> str:
    key = str(action_key or "")
    if key == "combat/end_turn":
        return "EndTurn"
    if key.startswith("combat/play_card/card:"):
        card = key.split("card:", 1)[1].split("/", 1)[0]
        target = key.split("target:", 1)[1] if "target:" in key else "none"
        if target in {"none", ""}:
            return f"{card}"
        return f"{card} -> {target}"
    if key.startswith("combat/hand_select/"):
        return "Hand select " + key.rsplit(":", 1)[-1]
    if key.startswith("combat/grid_select/"):
        return "Grid select " + key.rsplit(":", 1)[-1]
    return key


def action_sequence_label(keys: list[Any]) -> str:
    return " -> ".join(action_label(key) for key in keys)


def score_value(score: dict[str, Any] | None, key: str) -> int:
    if not score:
        return 0
    try:
        return int(score.get(key) or 0)
    except (TypeError, ValueError):
        return 0


def score_class(value: int) -> str:
    if value > 0:
        return "pos"
    if value < 0:
        return "neg"
    return "zero"


def chip(text: Any, cls: str = "") -> str:
    klass = f"chip {cls}".strip()
    return f'<span class="{klass}">{esc(text)}</span>'


def stat(label: str, value: Any, detail: str = "") -> str:
    return (
        '<div class="stat">'
        f'<div class="stat-label">{esc(label)}</div>'
        f'<div class="stat-value">{esc(value)}</div>'
        f'<div class="stat-detail">{esc(detail)}</div>'
        "</div>"
    )


def first_action(keys: list[Any]) -> str:
    return str(keys[0]) if keys else ""


def chosen_context(report: dict[str, Any]) -> dict[str, Any]:
    chosen = str((report.get("source_trace") or {}).get("trace_step_chosen_action_key") or "")
    plans = report.get("plans") or []
    sequences = report.get("sequence_classes") or []
    best_plan_names = [
        str(plan.get("plan_name"))
        for plan in plans
        if first_action(plan.get("best_action_keys") or []) == chosen
    ]
    top_sequence_hits = [
        idx + 1
        for idx, sequence in enumerate(sequences[:20])
        if first_action(sequence.get("action_keys") or []) == chosen
    ]
    return {
        "chosen": chosen,
        "chosen_label": action_label(chosen),
        "best_plan_names": best_plan_names,
        "top_sequence_hits": top_sequence_hits,
    }


def render_score_breakdown(score: dict[str, Any] | None) -> str:
    keys = [
        "total_score",
        "lethal_score",
        "block_score",
        "hp_loss_score",
        "enemy_death_score",
        "damage_score",
        "setup_score",
        "exhaust_value",
        "key_card_risk",
        "random_risk",
        "future_hand_penalty",
    ]
    cells = []
    for key in keys:
        value = score_value(score, key)
        cells.append(
            f'<div class="score-cell {score_class(value)}">'
            f'<div class="score-key">{esc(key)}</div>'
            f'<div class="score-val">{esc(value)}</div>'
            "</div>"
        )
    return '<div class="score-grid">' + "".join(cells) + "</div>"


def render_state(report: dict[str, Any]) -> str:
    state = report.get("state_summary") or {}
    source = report.get("source_trace") or {}
    chosen = chosen_context(report)
    plan_hits = chosen["best_plan_names"]
    hit_text = ", ".join(plan_hits) if plan_hits else "not a top plan first action"
    sequence_hits = chosen["top_sequence_hits"]
    seq_text = ", ".join(f"#{idx}" for idx in sequence_hits) if sequence_hits else "not in top 20"
    stats = [
        stat("Seed / Step", f"{source.get('seed')} / {source.get('step_index')}", source.get("trace_file")),
        stat("HP / Block", f"{state.get('player_hp')} / {state.get('player_block')}", f"incoming {state.get('visible_incoming_damage')}"),
        stat("Energy", state.get("energy"), f"turn {state.get('turn_count')}"),
        stat("Monsters", f"{state.get('alive_monster_count')}", f"total HP {state.get('total_monster_hp')}"),
        stat("Hand / Draw", f"{state.get('hand_count')} / {state.get('draw_count')}", f"discard {state.get('discard_count')}, exhaust {state.get('exhaust_count')}"),
        stat("Chosen", chosen["chosen_label"], f"best plans: {hit_text}; top sequences: {seq_text}"),
    ]
    return '<section class="panel"><h2>State</h2><div class="stats">' + "".join(stats) + "</div></section>"


def render_hand(report: dict[str, Any]) -> str:
    cards = report.get("hand_cards") or []
    rows = []
    for card in cards:
        semantics = " ".join(chip(tag, "semantic") for tag in card.get("base_semantics") or [])
        transient = " ".join(chip(tag, "transient") for tag in card.get("transient_tags") or [])
        rows.append(
            "<tr>"
            f"<td>{esc(card.get('hand_index'))}</td>"
            f"<td><strong>{esc(card.get('card_id'))}</strong><div class='muted'>uuid {esc(card.get('card_instance_id'))}</div></td>"
            f"<td>{esc(card.get('cost_for_turn'))}</td>"
            f"<td>{esc(card.get('playable'))}</td>"
            f"<td>{semantics}{transient}</td>"
            "</tr>"
        )
    body = "\n".join(rows) or "<tr><td colspan='5'>No hand cards</td></tr>"
    return f"""
    <section class="panel">
      <h2>Hand Cards</h2>
      <table>
        <thead><tr><th>Slot</th><th>Card</th><th>Cost</th><th>Playable</th><th>Semantics</th></tr></thead>
        <tbody>{body}</tbody>
      </table>
    </section>
    """


def render_risk_notes(report: dict[str, Any]) -> str:
    notes = report.get("risk_notes") or []
    if not notes:
        return "<section class='panel'><h2>Risk Notes</h2><p class='muted'>No explicit random/selection risk notes in this probe.</p></section>"
    rows = []
    for note in notes[:20]:
        affected = " ".join(chip(card, "affected") for card in note.get("affected_cards") or [])
        badges = [
            chip("exact_rng" if note.get("exact_rng_branches") else "not_exact_rng", "risk"),
            chip("overlay" if note.get("risk_is_overlay_only") else "engine_branch", "risk"),
        ]
        if note.get("chance_model"):
            badges.append(chip(note.get("chance_model"), "risk"))
        if note.get("bad_branch_probability_milli") is not None:
            badges.append(chip(f"bad {note.get('bad_branch_probability_milli')}/1000", "risk"))
        if note.get("good_branch_probability_milli") is not None:
            badges.append(chip(f"good {note.get('good_branch_probability_milli')}/1000", "risk"))
        rows.append(
            "<article class='risk-note'>"
            f"<h3>{esc(note.get('kind'))}</h3>"
            f"<div class='muted'>{esc(action_label(note.get('action_key')))} · action index {esc(note.get('sequence_action_index'))}</div>"
            f"<p>{esc(note.get('message'))}</p>"
            f"<div>{''.join(badges)} {affected}</div>"
            "</article>"
        )
    return "<section class='panel'><h2>Risk Notes</h2>" + "\n".join(rows) + "</section>"


def render_plans(report: dict[str, Any]) -> str:
    chosen = chosen_context(report)["chosen"]
    cards = []
    for plan in report.get("plans") or []:
        keys = plan.get("best_action_keys") or []
        score = plan.get("best_score") or {}
        first = first_action(keys)
        classes = ["plan-card"]
        if first == chosen:
            classes.append("chosen-plan")
        sequence = " -> ".join(action_label(key) for key in keys) or "no sequence"
        cards.append(
            f"<article class='{' '.join(classes)}'>"
            f"<header><h3>{esc(plan.get('plan_name'))}</h3>{chip('chosen-first', 'chosen') if first == chosen else ''}</header>"
            f"<div class='sequence'>{esc(sequence)}</div>"
            f"<p class='muted'>{esc(plan.get('explanation'))}</p>"
            f"{render_score_breakdown(score)}"
            "</article>"
        )
    return "<section class='panel'><h2>Plan Tops</h2><div class='plan-grid'>" + "\n".join(cards) + "</div></section>"


def render_sequences(report: dict[str, Any], top_n: int) -> str:
    chosen = chosen_context(report)["chosen"]
    rows = []
    compact: list[tuple[dict[str, Any], int]] = []
    seen: dict[str, int] = {}
    for sequence in report.get("sequence_classes") or []:
        label = action_sequence_label(sequence.get("action_keys") or [])
        if label in seen:
            existing_idx = seen[label]
            existing, count = compact[existing_idx]
            existing_score = score_value(existing.get("diagnostics") or {}, "total_score")
            current_score = score_value(sequence.get("diagnostics") or {}, "total_score")
            if current_score > existing_score:
                compact[existing_idx] = (sequence, count + 1)
            else:
                compact[existing_idx] = (existing, count + 1)
            continue
        seen[label] = len(compact)
        compact.append((sequence, 1))

    for idx, (sequence, variant_count) in enumerate(compact[:top_n], start=1):
        keys = sequence.get("action_keys") or []
        score = sequence.get("diagnostics") or {}
        reasons = " ".join(chip(reason, "reason") for reason in sequence.get("order_sensitive_reasons") or [])
        classes = []
        if first_action(keys) == chosen:
            classes.append("chosen-row")
        variant = f"<div class='muted'>variants collapsed: {variant_count}</div>" if variant_count > 1 else ""
        rows.append(
            f"<tr class='{' '.join(classes)}'>"
            f"<td>{idx}</td>"
            f"<td>{esc(action_sequence_label(keys))}{variant}<div class='muted mono'>{esc(sequence.get('sequence_equivalence_key'))}</div></td>"
            f"<td>{esc(score.get('total_score'))}</td>"
            f"<td>{esc(score.get('block_score'))}</td>"
            f"<td>{esc(score.get('damage_score'))}</td>"
            f"<td>{esc(score.get('setup_score'))}</td>"
            f"<td>{esc(score.get('exhaust_value'))}</td>"
            f"<td>{esc(score.get('key_card_risk'))} / {esc(score.get('random_risk'))}</td>"
            f"<td>{reasons}</td>"
            "</tr>"
        )
    body = "\n".join(rows) or "<tr><td colspan='9'>No sequences</td></tr>"
    return f"""
    <section class="panel">
      <h2>Top Sequence Classes</h2>
      <table>
        <thead>
          <tr><th>#</th><th>Sequence</th><th>Total</th><th>Block</th><th>Damage</th><th>Setup</th><th>Exhaust</th><th>Key/Random Risk</th><th>Order reasons</th></tr>
        </thead>
        <tbody>{body}</tbody>
      </table>
    </section>
    """


def render_warnings(report: dict[str, Any]) -> str:
    warnings = " ".join(chip(warning, "warning") for warning in report.get("truth_warnings") or [])
    limits = report.get("probe_limits") or {}
    limit_stats = [
        stat("Nodes", limits.get("nodes_expanded"), f"max {limits.get('max_nodes')}"),
        stat("Classes kept", limits.get("sequence_classes_kept"), ""),
        stat("Equivalent prunes", limits.get("pruned_as_equivalent"), ""),
        stat("Budget prunes", limits.get("pruned_by_budget"), ""),
    ]
    return (
        "<section class='panel'><h2>Probe Contract</h2>"
        f"<div class='warnings'>{warnings}</div>"
        f"<div class='stats'>{''.join(limit_stats)}</div>"
        "</section>"
    )


def render_html(report: dict[str, Any], report_path: Path, top_sequences: int) -> str:
    css = """
    body { margin: 0; font-family: Segoe UI, Arial, sans-serif; color: #111827; background: #f5f7fb; }
    main { max-width: 1320px; margin: 0 auto; padding: 24px; }
    h1 { margin: 0 0 8px; font-size: 28px; }
    h2 { margin: 0 0 14px; font-size: 18px; }
    h3 { margin: 0; font-size: 15px; }
    .intro { color: #4b5563; line-height: 1.45; margin: 0 0 16px; }
    .panel { background: #fff; border: 1px solid #dbe3ef; border-radius: 8px; padding: 16px; margin: 14px 0; }
    .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 12px; }
    .stat { background: #f8fafc; border: 1px solid #e5e7eb; border-radius: 6px; padding: 10px; }
    .stat-label { color: #64748b; font-size: 12px; }
    .stat-value { font-size: 20px; font-weight: 700; margin: 3px 0; overflow-wrap: anywhere; }
    .stat-detail { color: #64748b; font-size: 12px; overflow-wrap: anywhere; }
    .plan-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: 12px; }
    .plan-card { border: 1px solid #e5e7eb; background: #fbfdff; border-radius: 8px; padding: 12px; }
    .chosen-plan { border-color: #2563eb; background: #eff6ff; }
    .plan-card header { display: flex; justify-content: space-between; gap: 8px; align-items: center; }
    .sequence { font-weight: 700; margin: 8px 0; line-height: 1.35; overflow-wrap: anywhere; }
    .score-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(92px, 1fr)); gap: 6px; margin-top: 10px; }
    .score-cell { border-radius: 6px; padding: 6px; background: #f8fafc; border: 1px solid #e5e7eb; }
    .score-key { font-size: 11px; color: #64748b; }
    .score-val { font-weight: 700; }
    .pos .score-val { color: #047857; }
    .neg .score-val { color: #b91c1c; }
    .chip { display: inline-block; border: 1px solid #cbd5e1; border-radius: 999px; padding: 3px 8px; font-size: 12px; margin: 2px; background: #f8fafc; }
    .semantic { background: #f0f9ff; border-color: #7dd3fc; }
    .transient { background: #f5f3ff; border-color: #c4b5fd; }
    .warning, .risk { background: #fff7ed; border-color: #fdba74; }
    .chosen { background: #dbeafe; border-color: #60a5fa; }
    .affected { background: #fef2f2; border-color: #fca5a5; }
    .reason { background: #ecfdf5; border-color: #86efac; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border-bottom: 1px solid #e5e7eb; padding: 8px; text-align: left; vertical-align: top; }
    th { background: #f8fafc; color: #475569; font-size: 12px; }
    tr.chosen-row td { background: #eff6ff; }
    .risk-note { border: 1px solid #fed7aa; background: #fff7ed; border-radius: 8px; padding: 10px; margin: 8px 0; }
    .muted { color: #64748b; font-size: 12px; }
    .mono { font-family: Consolas, monospace; }
    """
    return f"""<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>Combat Turn Plan Probe</title>
  <style>{css}</style>
</head>
<body>
<main>
  <h1>Combat Turn Plan Probe</h1>
  <p class="intro">
    这不是“最优动作证明”，也不是训练标签。它只把当前回合压缩成少量 plan、动作序列类、顺序原因和副作用风险。
    读的时候先看 chosen 是否落在某些 plan 的 top first-action，再看风险说明和分项 score。
  </p>
  <p class="muted mono">report: {esc(report_path)}</p>
  <p class="muted mono">generated: {esc(datetime.now(timezone.utc).isoformat())}</p>
  {render_state(report)}
  {render_warnings(report)}
  {render_risk_notes(report)}
  {render_hand(report)}
  {render_plans(report)}
  {render_sequences(report, top_sequences)}
</main>
</body>
</html>
"""


def render_markdown(report: dict[str, Any], report_path: Path, top_sequences: int) -> str:
    source = report.get("source_trace") or {}
    state = report.get("state_summary") or {}
    chosen = chosen_context(report)
    lines = [
        "# Combat Turn Plan Probe",
        "",
        f"- Report: `{report_path}`",
        f"- Seed / step: `{source.get('seed')}` / `{source.get('step_index')}`",
        f"- Chosen: `{chosen['chosen_label']}`",
        f"- Chosen as best first action for plans: `{', '.join(chosen['best_plan_names']) or 'none'}`",
        f"- HP / block / incoming: `{state.get('player_hp')}` / `{state.get('player_block')}` / `{state.get('visible_incoming_damage')}`",
        f"- Energy / hand / monsters: `{state.get('energy')}` / `{state.get('hand_count')}` / `{state.get('alive_monster_count')}`",
        "",
        "## Plan Tops",
    ]
    for plan in report.get("plans") or []:
        keys = plan.get("best_action_keys") or []
        score = plan.get("best_score") or {}
        lines.extend(
            [
                "",
                f"### {plan.get('plan_name')}",
                f"- Sequence: `{action_sequence_label(keys) or 'none'}`",
                f"- Total/block/damage/setup/exhaust/risk: `{score.get('total_score')}` / `{score.get('block_score')}` / `{score.get('damage_score')}` / `{score.get('setup_score')}` / `{score.get('exhaust_value')}` / `{score.get('key_card_risk')},{score.get('random_risk')}`",
            ]
        )
    lines.extend(["", "## Risk Notes"])
    for note in (report.get("risk_notes") or [])[:20]:
        lines.append(
            f"- `{note.get('kind')}` on `{action_label(note.get('action_key'))}`: {note.get('message')}"
        )
    lines.extend(["", "## Top Sequence Classes"])
    compact: list[tuple[dict[str, Any], int]] = []
    seen: dict[str, int] = {}
    for sequence in report.get("sequence_classes") or []:
        label = action_sequence_label(sequence.get("action_keys") or [])
        if label in seen:
            existing_idx = seen[label]
            existing, count = compact[existing_idx]
            existing_score = score_value(existing.get("diagnostics") or {}, "total_score")
            current_score = score_value(sequence.get("diagnostics") or {}, "total_score")
            compact[existing_idx] = (sequence if current_score > existing_score else existing, count + 1)
        else:
            seen[label] = len(compact)
            compact.append((sequence, 1))
    for idx, (sequence, variant_count) in enumerate(compact[:top_sequences], start=1):
        score = sequence.get("diagnostics") or {}
        lines.append(
            f"{idx}. `{action_sequence_label(sequence.get('action_keys') or [])}` "
            f"total `{score.get('total_score')}`, block `{score.get('block_score')}`, damage `{score.get('damage_score')}`, "
            f"variants `{variant_count}`, order `{', '.join(str(x) for x in sequence.get('order_sensitive_reasons') or []) or 'none'}`"
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    report = read_json(args.report)
    out = args.out or args.report.with_suffix(".html")
    md_out = args.markdown_out or args.report.with_suffix(".md")
    out.parent.mkdir(parents=True, exist_ok=True)
    md_out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render_html(report, args.report, args.top_sequences), encoding="utf-8")
    md_out.write_text(render_markdown(report, args.report, args.top_sequences), encoding="utf-8")
    print(
        json.dumps(
            {
                "html_out": str(out),
                "markdown_out": str(md_out),
                "schema_version": report.get("schema_version"),
                "top_sequences": args.top_sequences,
            },
            indent=2,
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
