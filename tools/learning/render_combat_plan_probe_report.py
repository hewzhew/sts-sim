#!/usr/bin/env python3
from __future__ import annotations

import argparse
import html
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from combat_rl_common import REPO_ROOT


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Render a human review report for combat abstraction plan probes. "
            "The report is for sample quality review, not a training artifact."
        )
    )
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--case-source", default="card", choices=["card", "full", "both"])
    parser.add_argument("--max-cases", type=int, default=50)
    parser.add_argument("--top-candidates", type=int, default=8)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--markdown-out", type=Path)
    return parser.parse_args()


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def resolve_path(path_text: str | None, base: Path) -> Path | None:
    if not path_text:
        return None
    path = Path(path_text)
    if path.exists():
        return path
    if not path.is_absolute():
        candidate = base / path
        if candidate.exists():
            return candidate
        candidate = REPO_ROOT / path
        if candidate.exists():
            return candidate
    return path


def default_outputs(summary_path: Path) -> tuple[Path, Path]:
    html_out = summary_path.with_name("combat_plan_probe_report.html")
    md_out = summary_path.with_name("combat_plan_probe_report.md")
    return html_out, md_out


def esc(value: Any) -> str:
    return html.escape(str(value if value is not None else ""))


def fmt_float(value: Any, digits: int = 2) -> str:
    try:
        return f"{float(value):.{digits}f}"
    except (TypeError, ValueError):
        return ""


def action_label(action_key: Any) -> str:
    key = str(action_key or "")
    if key == "combat/end_turn":
        return "EndTurn"
    if key.startswith("combat/play_card/card:"):
        card = key.split("card:", 1)[1].split("/", 1)[0]
        target = key.split("target:", 1)[1] if "target:" in key else ""
        return f"{card} -> {target}" if target else card
    if key.startswith("combat/use_potion/"):
        return "Potion " + key.rsplit("/", 1)[-1]
    return key


def case_rows(case: dict[str, Any], summary_path: Path) -> list[dict[str, Any]]:
    rows_path = resolve_path(str(case.get("rows_path") or ""), summary_path.parent)
    if rows_path is None:
        return []
    return read_jsonl(rows_path)


def row_for_key(rows: list[dict[str, Any]], key: Any) -> dict[str, Any] | None:
    text = str(key or "")
    for row in rows:
        if str(row.get("candidate_key") or "") == text:
            return row
    return None


def selected_cases(summary: dict[str, Any], source: str) -> list[dict[str, Any]]:
    if source == "card":
        return list(summary.get("card_plan_probe_cases") or [])
    if source == "full":
        return list(summary.get("plan_probe_cases") or [])
    cases = []
    seen = set()
    for key in ("card_plan_probe_cases", "plan_probe_cases"):
        for case in summary.get(key) or []:
            case_id = str(case.get("case_id") or "")
            if case_id not in seen:
                seen.add(case_id)
                cases.append(case)
    return cases


def badge(text: Any, klass: str = "") -> str:
    cls = f"badge {klass}".strip()
    return f'<span class="{cls}">{esc(text)}</span>'


def metric_card(label: str, value: Any, detail: str = "") -> str:
    return (
        '<div class="metric">'
        f'<div class="metric-label">{esc(label)}</div>'
        f'<div class="metric-value">{esc(value)}</div>'
        f'<div class="metric-detail">{esc(detail)}</div>'
        "</div>"
    )


def render_counts(title: str, counts: dict[str, Any]) -> str:
    rows = []
    for key, value in sorted((counts or {}).items(), key=lambda item: str(item[0])):
        rows.append(f"<tr><td>{esc(key)}</td><td>{esc(value)}</td></tr>")
    body = "\n".join(rows) if rows else '<tr><td colspan="2">empty</td></tr>'
    return f"<section><h2>{esc(title)}</h2><table><tbody>{body}</tbody></table></section>"


def render_plan_box(title: str, case: dict[str, Any], row: dict[str, Any] | None, prefix: str) -> str:
    key = case.get(f"{prefix}_action_key")
    if prefix == "best_card":
        key = case.get("best_card_candidate_key")
    if prefix == "best":
        key = case.get("best_candidate_key")
    if prefix == "chosen":
        key = case.get("chosen_action_key")
    plan_role = case.get(f"{prefix}_root_plan_role") or (row or {}).get("root_plan_role")
    plan_fit = case.get(f"{prefix}_root_plan_fit") or (row or {}).get("root_plan_fit")
    block_need = case.get(f"{prefix}_root_block_need") or (row or {}).get("root_block_need")
    block_reduce = case.get(f"{prefix}_root_unblocked_reduction") or (row or {}).get("root_unblocked_reduction")
    damage = case.get(f"{prefix}_root_damage_delta") or (row or {}).get("root_damage_delta")
    return f"""
    <div class="plan-box">
      <h3>{esc(title)}</h3>
      <div class="action">{esc(action_label(key))}</div>
      <div class="mono small">{esc(key)}</div>
      <div class="chips">
        {badge(plan_role or "unknown", "role")}
        {badge(plan_fit or "unknown", "fit")}
      </div>
      <dl>
        <dt>Block need</dt><dd>{esc(block_need)}</dd>
        <dt>Unblocked reduction</dt><dd>{esc(block_reduce)}</dd>
        <dt>Damage delta</dt><dd>{esc(damage)}</dd>
        <dt>Expected end HP</dt><dd>{esc(fmt_float((row or {}).get("expected_end_hp")))}</dd>
        <dt>Combat win prob</dt><dd>{esc(fmt_float((row or {}).get("combat_win_prob")))}</dd>
      </dl>
    </div>
    """


def render_candidate_table(rows: list[dict[str, Any]], case: dict[str, Any], top_n: int) -> str:
    chosen_key = str(case.get("chosen_action_key") or "")
    best_card_key = str(case.get("best_card_candidate_key") or "")
    card_rows = [row for row in rows if str(row.get("action_family") or "") in {"play_card", "end_turn"}]
    card_rows = sorted(card_rows, key=lambda row: int(row.get("rank") or 999))[:top_n]
    out = []
    for row in card_rows:
        key = str(row.get("candidate_key") or "")
        classes = []
        if key == chosen_key:
            classes.append("chosen")
        if key == best_card_key:
            classes.append("best")
        out.append(
            "<tr class=\"{}\">".format(" ".join(classes))
            + f"<td>{esc(row.get('rank'))}</td>"
            + f"<td><strong>{esc(action_label(key))}</strong><div class=\"mono small\">{esc(key)}</div></td>"
            + f"<td>{esc(row.get('root_plan_role'))}<br><span class=\"muted\">{esc(row.get('root_plan_fit'))}</span></td>"
            + f"<td>{esc(row.get('pressure_class'))}</td>"
            + f"<td>{esc(row.get('root_block_need'))} / {esc(row.get('root_unblocked_reduction'))}</td>"
            + f"<td>{esc(row.get('root_damage_delta'))}</td>"
            + f"<td>{esc(fmt_float(row.get('expected_end_hp')))}</td>"
            + f"<td>{esc(fmt_float(row.get('combat_win_prob')))}</td>"
            + "</tr>"
        )
    if not out:
        out.append('<tr><td colspan="8">No card/end-turn rows found.</td></tr>')
    return """
    <table class="candidate-table">
      <thead>
        <tr>
          <th>Rank</th><th>Candidate</th><th>Plan</th><th>Pressure</th>
          <th>Need / reduce</th><th>Damage</th><th>End HP</th><th>Win</th>
        </tr>
      </thead>
      <tbody>
    """ + "\n".join(out) + "</tbody></table>"


def render_case(case: dict[str, Any], summary_path: Path, top_n: int, index: int) -> str:
    rows = case_rows(case, summary_path)
    chosen_row = row_for_key(rows, case.get("chosen_action_key"))
    best_card_row = row_for_key(rows, case.get("best_card_candidate_key"))
    flags = " ".join(badge(flag, "flag") for flag in case.get("card_diagnostic_flags") or [])
    reasons = " ".join(badge(reason, "reason") for reason in case.get("card_readiness_reasons") or [])
    report_path = case.get("report_path")
    rows_path = case.get("rows_path")
    return f"""
    <article class="case-card" id="case-{index}">
      <header>
        <div>
          <h2>#{index + 1} {esc(case.get('case_id'))}</h2>
          <div class="meta">
            {badge('floor ' + str(case.get('floor')), 'meta-badge')}
            {badge('hp ' + str(case.get('hp')), 'meta-badge')}
            {badge(case.get('chosen_pressure_class'), 'pressure')}
            {badge(case.get('card_plan_readiness'), 'ready')}
          </div>
        </div>
        <div class="regret">
          <div>HP gap <strong>{esc(fmt_float(case.get('card_hp_regret')))}</strong></div>
          <div>Win gap <strong>{esc(fmt_float(case.get('card_combat_win_regret')))}</strong></div>
        </div>
      </header>
      <div class="reason-row"><strong>Reasons:</strong> {reasons or 'none'} <strong>Flags:</strong> {flags or 'none'}</div>
      <div class="plan-grid">
        {render_plan_box('Chosen', case, chosen_row, 'chosen')}
        {render_plan_box('Best card-only', case, best_card_row, 'best_card')}
      </div>
      <details open>
        <summary>Top card/end-turn candidates</summary>
        {render_candidate_table(rows, case, top_n)}
      </details>
      <details>
        <summary>Source files</summary>
        <div class="mono small">case report: {esc(report_path)}</div>
        <div class="mono small">rows: {esc(rows_path)}</div>
      </details>
    </article>
    """


def render_html(summary: dict[str, Any], summary_path: Path, cases: list[dict[str, Any]], top_n: int) -> str:
    counts = summary.get("counts") or {}
    css = """
    body { font-family: Segoe UI, Arial, sans-serif; margin: 0; background: #f6f7f9; color: #111827; }
    main { max-width: 1240px; margin: 0 auto; padding: 24px; }
    h1 { margin: 0 0 8px; font-size: 28px; }
    h2 { margin: 0 0 12px; font-size: 18px; }
    h3 { margin: 0 0 8px; font-size: 15px; }
    section, .case-card { background: #fff; border: 1px solid #d8dee8; border-radius: 8px; padding: 16px; margin: 16px 0; }
    .intro { color: #4b5563; line-height: 1.45; }
    .metrics { display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 12px; margin-top: 16px; }
    .metric { background: #f9fafb; border: 1px solid #e5e7eb; border-radius: 6px; padding: 12px; }
    .metric-label { color: #6b7280; font-size: 12px; }
    .metric-value { font-size: 24px; font-weight: 700; margin: 4px 0; }
    .metric-detail { color: #6b7280; font-size: 12px; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border-bottom: 1px solid #e5e7eb; padding: 8px; text-align: left; vertical-align: top; }
    th { background: #f9fafb; font-size: 12px; color: #4b5563; }
    .case-card header { display: flex; justify-content: space-between; gap: 16px; align-items: flex-start; border-bottom: 1px solid #e5e7eb; padding-bottom: 12px; margin-bottom: 12px; }
    .meta, .chips, .reason-row { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
    .reason-row { margin: 8px 0 14px; color: #374151; }
    .badge { display: inline-block; border: 1px solid #cbd5e1; border-radius: 999px; padding: 3px 8px; font-size: 12px; background: #f8fafc; }
    .pressure { background: #fff7ed; border-color: #fdba74; }
    .ready { background: #ecfdf5; border-color: #6ee7b7; }
    .flag { background: #fef2f2; border-color: #fca5a5; }
    .reason { background: #eff6ff; border-color: #93c5fd; }
    .role { background: #f5f3ff; border-color: #c4b5fd; }
    .fit { background: #f0fdf4; border-color: #86efac; }
    .plan-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 12px; }
    .plan-box { border: 1px solid #e5e7eb; border-radius: 8px; padding: 12px; background: #fbfdff; }
    .action { font-weight: 700; font-size: 16px; margin-bottom: 4px; }
    dl { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px; margin: 12px 0 0; }
    dt { color: #6b7280; font-size: 12px; }
    dd { margin: 0; font-weight: 600; }
    .regret { text-align: right; color: #374151; }
    .mono { font-family: Consolas, monospace; }
    .small { font-size: 12px; color: #64748b; }
    .muted { color: #6b7280; }
    tr.chosen td { background: #fff7ed; }
    tr.best td { background: #ecfdf5; }
    tr.chosen.best td { background: #eef2ff; }
    details { margin-top: 12px; }
    summary { cursor: pointer; font-weight: 600; }
    """
    metric_html = "".join(
        [
            metric_card("Selected cases", counts.get("selected_cases"), "batch roots"),
            metric_card("Card plan probes", summary.get("card_training_ready_case_count"), "review target"),
            metric_card("Full plan probes", summary.get("training_ready_case_count"), "not potion-filtered"),
            metric_card("Potion opportunities", (summary.get("diagnostic_flag_counts") or {}).get("potion_opportunity", 0), "diagnostic"),
            metric_card("Pressure classes", len(summary.get("pressure_class_counts") or {}), "coverage"),
        ]
    )
    case_html = "\n".join(render_case(case, summary_path, top_n, index) for index, case in enumerate(cases))
    return f"""<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <title>Combat Plan Probe Report</title>
  <style>{css}</style>
</head>
<body>
<main>
  <h1>Combat Plan Probe Report</h1>
  <p class="intro">
    This report is for human quality review of card-only combat plan probes. It does not define
    training labels by itself. Full-policy potion dominated cases are tracked in the overview but
    filtered out of the primary case list.
  </p>
  <p class="mono small">summary: {esc(summary_path)}</p>
  <p class="mono small">generated: {esc(datetime.now(timezone.utc).isoformat())}</p>
  <div class="metrics">{metric_html}</div>
  {render_counts('Pressure Classes', summary.get('pressure_class_counts') or {})}
  {render_counts('Card Plan Readiness', summary.get('card_plan_readiness_counts') or {})}
  {render_counts('Diagnostic Flags', summary.get('diagnostic_flag_counts') or {})}
  <section>
    <h2>Review Guidance</h2>
    <p class="intro">
      Keep a case only if the chosen-vs-best-card comparison has a believable combat-plan meaning:
      defense versus damage, damage versus setup, immediate block versus future mitigation, or
      no-attack window usage. Downgrade cases where EndTurn is the unexplained best card action,
      continuation behavior looks arbitrary, or the state is already a macro-caused collapse.
    </p>
  </section>
  {case_html or '<section>No plan probe cases selected.</section>'}
</main>
</body>
</html>
"""


def render_markdown(summary: dict[str, Any], summary_path: Path, cases: list[dict[str, Any]]) -> str:
    lines = [
        "# Combat Plan Probe Report",
        "",
        f"- Summary: `{summary_path}`",
        f"- Selected cases: `{(summary.get('counts') or {}).get('selected_cases')}`",
        f"- Card plan probes: `{summary.get('card_training_ready_case_count')}`",
        f"- Full plan probes: `{summary.get('training_ready_case_count')}`",
        f"- Potion opportunities: `{(summary.get('diagnostic_flag_counts') or {}).get('potion_opportunity', 0)}`",
        "",
        "## Cases",
    ]
    for index, case in enumerate(cases, start=1):
        lines.extend(
            [
                "",
                f"### {index}. `{case.get('case_id')}`",
                f"- Floor/HP/pressure: `{case.get('floor')}` / `{case.get('hp')}` / `{case.get('chosen_pressure_class')}`",
                f"- Chosen: `{case.get('chosen_action_key')}` -> `{case.get('chosen_root_plan_role')}` / `{case.get('chosen_root_plan_fit')}`",
                f"- Best card: `{case.get('best_card_candidate_key')}` -> `{case.get('best_card_root_plan_role')}` / `{case.get('best_card_root_plan_fit')}`",
                f"- Card HP gap / win gap: `{fmt_float(case.get('card_hp_regret'))}` / `{fmt_float(case.get('card_combat_win_regret'))}`",
                f"- Reasons: `{', '.join(str(x) for x in case.get('card_readiness_reasons') or [])}`",
                f"- Flags: `{', '.join(str(x) for x in case.get('card_diagnostic_flags') or [])}`",
                f"- Rows: `{case.get('rows_path')}`",
            ]
        )
    return "\n".join(lines) + "\n"


def main() -> None:
    args = parse_args()
    summary_path = args.summary
    summary = read_json(summary_path)
    html_default, md_default = default_outputs(summary_path)
    out = args.out or html_default
    markdown_out = args.markdown_out or md_default
    cases = selected_cases(summary, args.case_source)[: max(int(args.max_cases), 0)]
    out.parent.mkdir(parents=True, exist_ok=True)
    markdown_out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(render_html(summary, summary_path, cases, args.top_candidates), encoding="utf-8")
    markdown_out.write_text(render_markdown(summary, summary_path, cases), encoding="utf-8")
    print(
        json.dumps(
            {
                "html_out": str(out),
                "markdown_out": str(markdown_out),
                "case_count": len(cases),
                "case_source": args.case_source,
            },
            indent=2,
            ensure_ascii=False,
        )
    )


if __name__ == "__main__":
    main()
