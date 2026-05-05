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
        hand = ""
        if "hand:" in key:
            hand_idx = key.split("hand:", 1)[1].split("/", 1)[0]
            if hand_idx:
                hand = f"[h{hand_idx}]"
        target = key.split("target:", 1)[1] if "target:" in key else "none"
        if target in {"none", ""}:
            return f"{card}{hand}"
        return f"{card}{hand} -> {target}"
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


QUERY_LABELS = {
    "CanLethal": "本回合能斩杀吗",
    "CanFullBlock": "能全防吗",
    "CanFullBlockThenMaxDamage": "全防后最多能打多少",
    "CanPlaySetupAndStillBlock": "能发育且不漏伤吗",
    "CanPreserveKillWindow": "能保留斩杀窗口吗",
}

STATUS_LABELS = {
    "feasible": "可兑现",
    "partial": "部分可兑现",
    "not_feasible": "不可兑现",
    "not_applicable": "不适用",
}


def prefixed_value(values: list[Any], prefix: str) -> str:
    for value in values:
        text = str(value)
        if text.startswith(prefix):
            return text[len(prefix) :]
    return ""


def query_primary_line(query: dict[str, Any]) -> str:
    name = str(query.get("query_name") or "")
    status = str(query.get("status") or "")
    outcome = query.get("outcome") or {}
    failed = query.get("failed_constraints") or []
    notes = query.get("notes") or []
    if name == "CanLethal":
        if status == "feasible":
            return f"能杀完，伤害 {outcome.get('damage_done')}"
        missing = prefixed_value(failed, "missing_damage:")
        return f"最多伤害 {outcome.get('damage_done', 0)}，还差 {missing or outcome.get('total_monster_hp', 0)}"
    if name == "CanFullBlock":
        if status == "feasible":
            return f"可全防，剩余能量 {outcome.get('remaining_energy')}"
        unblocked = prefixed_value(failed, "unblocked_damage:")
        return f"不能全防，最少漏伤 {unblocked or outcome.get('projected_unblocked_damage', 0)}"
    if name == "CanFullBlockThenMaxDamage":
        if status == "feasible":
            return f"可全防并打 {outcome.get('damage_done', 0)} 伤害"
        return f"只能部分防，当前最好打 {outcome.get('damage_done', 0)} 伤害"
    if name == "CanPlaySetupAndStillBlock":
        if status == "not_applicable":
            return "手里没有发育/成长牌"
        if status == "feasible":
            return f"可打发育牌并全防，剩余能量 {outcome.get('remaining_energy')}"
        return f"能发育但会漏伤 {outcome.get('projected_unblocked_damage', 0)}"
    if name == "CanPreserveKillWindow":
        if status == "not_applicable":
            return "手里没有 Feed / Hand of Greed / Ritual Dagger"
        if status == "feasible":
            return f"可保留 {outcome.get('kill_window_target_count', 0)} 个斩杀窗口"
        return "没有找到可保留的斩杀窗口"
    return "; ".join(str(note) for note in notes[:2]) or status


def query_detail_line(query: dict[str, Any]) -> str:
    outcome = query.get("outcome") or {}
    if not outcome:
        return "没有可展示的当前回合序列"
    return (
        f"防 {outcome.get('block_after')} / 漏 {outcome.get('projected_unblocked_damage')} / "
        f"伤 {outcome.get('damage_done')} / 剩能 {outcome.get('remaining_energy')} / "
        f"剩手 {outcome.get('remaining_hand_count')} / 怪血 {outcome.get('total_monster_hp')}"
    )


def render_plan_queries(report: dict[str, Any]) -> str:
    queries = report.get("plan_queries") or []
    if not queries:
        return (
            "<section class='panel query-panel'><h2>Plan Queries</h2>"
            "<p class='muted'>这个 report 还没有 plan_queries；它可能是旧 schema。</p></section>"
        )
    cards = []
    for query in queries:
        status = str(query.get("status") or "")
        name = str(query.get("query_name") or "")
        keys = query.get("best_action_keys") or []
        failed = query.get("failed_constraints") or []
        notes = query.get("notes") or []
        flags = []
        if query.get("needs_deeper_search"):
            flags.append(chip("需要更深搜索", "warning"))
        for item in failed[:3]:
            flags.append(chip(item, "risk"))
        sequence = action_sequence_label(keys) or "无序列"
        cards.append(
            f"<article class='query-card query-{esc(status)}'>"
            f"<header><h3>{esc(QUERY_LABELS.get(name, name))}</h3>{chip(STATUS_LABELS.get(status, status), status)}</header>"
            f"<div class='query-main'>{esc(query_primary_line(query))}</div>"
            f"<div class='muted'>{esc(query_detail_line(query))}</div>"
            f"<div class='sequence'>{esc(sequence)}</div>"
            f"<div>{''.join(flags)}</div>"
            f"<p class='muted'>{esc('; '.join(str(note) for note in notes[:2]))}</p>"
            "</article>"
        )
    return (
        "<section class='panel query-panel'><h2>Plan Queries</h2>"
        "<p class='muted'>先看这里：这些不是第一手推荐，而是在问“这个战术意图在当前回合能不能兑现”。</p>"
        f"<div class='query-grid'>{''.join(cards)}</div>"
        "</section>"
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


def pressure_label(state: dict[str, Any]) -> str:
    incoming = score_value(state, "visible_incoming_damage")
    unblocked = score_value(state, "unblocked_incoming_damage")
    hp = score_value(state, "player_hp")
    if incoming <= 0:
        return "无来伤窗口"
    if unblocked >= hp and hp > 0:
        return "致死压力"
    if unblocked >= max(hp // 2, 1):
        return "高压"
    if unblocked >= 6:
        return "中压"
    return "低压"


def best_affordance_for_role(report: dict[str, Any], wanted: str) -> dict[str, Any] | None:
    for affordance in report.get("first_action_affordances") or []:
        supports = filtered_supports(report, affordance)
        if classify_action_role(report, affordance, supports) == wanted:
            return affordance
    return None


def chosen_affordance(report: dict[str, Any]) -> dict[str, Any] | None:
    chosen = chosen_context(report)["chosen"]
    for affordance in report.get("first_action_affordances") or []:
        if affordance.get("action_key") == chosen:
            return affordance
    return None


def one_line_action_summary(report: dict[str, Any], affordance: dict[str, Any] | None) -> str:
    if not affordance:
        return "无"
    supports = filtered_supports(report, affordance)
    role = classify_action_role(report, affordance, supports)
    component = affordance.get("component_max") or {}
    support_text = ", ".join(
        PLAN_LABELS.get(str(support.get("plan_name") or ""), str(support.get("plan_name") or ""))
        for support in supports[:2]
    )
    if not support_text:
        support_text = role
    return (
        f"{action_label(affordance.get('action_key'))}: {support_text}, "
        f"防 {component.get('block_score')}, 伤 {component.get('damage_score')}"
    )


def attention_items(report: dict[str, Any], affordance: dict[str, Any] | None) -> list[str]:
    items: list[str] = []
    state = report.get("state_summary") or {}
    if affordance:
        component = affordance.get("component_max") or {}
        incoming = score_value(state, "visible_incoming_damage")
        block = score_value(component, "block_score")
        damage = score_value(component, "damage_score")
        if incoming > 0 and block < 80:
            items.append("当前选择不是全防线，可能接受部分来伤")
        if damage >= 72:
            items.append("当前选择是高伤害推进线")
        if affordance.get("order_sensitive_reasons"):
            items.append("顺序敏感，尤其是易伤/虚弱/抽牌/击杀窗口")
        if affordance.get("risk_note_kinds"):
            items.append("存在显式副作用风险，需要看 Risk Notes")
    if not items and pressure_label(state) in {"高压", "致死压力"}:
        items.append("高压局面，优先检查防御/生存计划")
    return items[:3]


def render_turn_summary(report: dict[str, Any]) -> str:
    state = report.get("state_summary") or {}
    chosen = chosen_affordance(report)
    attack = best_affordance_for_role(report, "进攻/击杀")
    defense = best_affordance_for_role(report, "防御")
    hybrid = best_affordance_for_role(report, "攻防混合")
    attention = attention_items(report, chosen)
    attention_html = "".join(f"<li>{esc(item)}</li>" for item in attention) or "<li>没有显式高风险提示。</li>"
    summary_cards = [
        stat("局面压力", pressure_label(state), f"incoming {state.get('visible_incoming_damage')}, HP {state.get('player_hp')}"),
        stat("当前选择", one_line_action_summary(report, chosen), "只是解释路线，不是最优证明"),
        stat("进攻代表", one_line_action_summary(report, attack), ""),
        stat("防御代表", one_line_action_summary(report, defense), ""),
        stat("攻防混合代表", one_line_action_summary(report, hybrid), ""),
    ]
    return (
        "<section class='panel summary-panel'><h2>Turn Summary</h2>"
        "<p class='muted'>先看这里：它把下面的完整表压缩成当前回合的几条可读路线。</p>"
        f"<div class='stats'>{''.join(summary_cards)}</div>"
        f"<h3>Attention</h3><ul class='attention-list'>{attention_html}</ul>"
        "</section>"
    )


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


PLAN_LABELS = {
    "Lethal": "斩杀",
    "KillThreateningEnemy": "击杀威胁",
    "FullBlock": "全防",
    "BlockEnoughThenDamage": "先防后打",
    "MaxDamage": "最大输出",
    "SetupPowerOrScaling": "发育/能力",
    "ExhaustBadCards": "烧牌清理",
    "PreserveKeyCards": "保留关键牌",
}

TRADEOFF_LABELS = {
    "can_end_combat": "可结束战斗",
    "can_kill_enemy": "可击杀敌人",
    "strong_defense_line": "强防御线",
    "partial_defense_line": "部分防御",
    "strong_damage_progress": "高伤害推进",
    "damage_progress": "伤害推进",
    "setup_or_scaling": "发育/成长",
    "exhaust_cleanup_or_synergy": "烧牌收益",
    "accepts_hp_loss": "接受掉血",
    "spends_or_destroys_hand": "消耗/破坏手牌",
    "explicit_risk_note": "有副作用风险",
    "order_sensitive": "顺序敏感",
    "debuff_before_damage_can_change_value": "易伤/虚弱顺序重要",
    "draw_changes_future_action_space": "抽牌改变后续空间",
    "exhaust_changes_hand_and_deck_state": "烧牌改变手牌/牌堆",
    "random_effect_requires_risk_model": "随机效果需风险评估",
    "possible_kill_changes_incoming_damage": "击杀会改变来伤",
}


def parse_action_key(action_key: Any) -> dict[str, str]:
    key = str(action_key or "")
    parsed = {"card": "", "hand": "", "target": ""}
    if not key.startswith("combat/play_card/card:"):
        return parsed
    rest = key.split("card:", 1)[1]
    parsed["card"] = rest.split("/", 1)[0]
    if "hand:" in key:
        parsed["hand"] = key.split("hand:", 1)[1].split("/", 1)[0]
    if "target:" in key:
        parsed["target"] = key.split("target:", 1)[1]
    return parsed


def hand_semantics(report: dict[str, Any], affordance: dict[str, Any]) -> set[str]:
    parsed = parse_action_key(affordance.get("action_key"))
    if not parsed["card"] or not parsed["hand"]:
        return set()
    try:
        hand_index = int(parsed["hand"])
    except ValueError:
        return set()
    for card in report.get("hand_cards") or []:
        if card.get("card_id") == parsed["card"] and card.get("hand_index") == hand_index:
            return set(card.get("base_semantics") or []) | set(card.get("transient_tags") or [])
    return set()


def action_has_exhaust_context(report: dict[str, Any], affordance: dict[str, Any]) -> bool:
    component = affordance.get("component_max") or {}
    semantics = hand_semantics(report, affordance)
    risk_kinds = set(affordance.get("risk_note_kinds") or [])
    return (
        score_value(component, "exhaust_value") > 0
        or bool(risk_kinds)
        or bool(
            semantics
            & {
                "exhaust_outlet",
                "random_exhaust",
                "chosen_exhaust",
                "exhaust_non_attacks",
                "exhaust_hand_for_damage",
                "risk_overlay_required",
            }
        )
    )


def action_has_setup_context(report: dict[str, Any], affordance: dict[str, Any]) -> bool:
    component = affordance.get("component_max") or {}
    semantics = hand_semantics(report, affordance)
    return score_value(component, "setup_score") > 0 or bool(semantics & {"power", "setup_or_scaling"})


def filtered_supports(report: dict[str, Any], affordance: dict[str, Any]) -> list[dict[str, Any]]:
    has_exhaust = action_has_exhaust_context(report, affordance)
    has_setup = action_has_setup_context(report, affordance)
    has_risk = bool(affordance.get("risk_note_kinds")) or score_value(affordance.get("component_max") or {}, "key_card_risk") < 0
    kept: list[dict[str, Any]] = []
    for support in affordance.get("supported_plans") or []:
        plan = str(support.get("plan_name") or "")
        rank = int(support.get("rank") or 99)
        gap = int(support.get("score_gap_to_best") or 0)
        if rank > 3 and gap > 25:
            continue
        if plan == "ExhaustBadCards" and not has_exhaust:
            continue
        if plan == "SetupPowerOrScaling" and not has_setup:
            continue
        if plan == "PreserveKeyCards" and not (has_risk or has_exhaust):
            continue
        kept.append(support)
    return kept


def classify_action_role(report: dict[str, Any], affordance: dict[str, Any], supports: list[dict[str, Any]]) -> str:
    plans = {str(support.get("plan_name") or "") for support in supports}
    component = affordance.get("component_max") or {}
    block = score_value(component, "block_score")
    damage = score_value(component, "damage_score")
    has_attack = bool(plans & {"Lethal", "KillThreateningEnemy", "MaxDamage"}) or damage >= 72
    has_defense = "FullBlock" in plans or block >= 80 or any(
        support.get("plan_name") == "BlockEnoughThenDamage" and int(support.get("rank") or 99) <= 2
        for support in supports
    )
    if has_attack and has_defense:
        return "攻防混合"
    if has_attack:
        return "进攻/击杀"
    if has_defense:
        return "防御"
    if block > 0 and damage == 0:
        return "防御"
    if action_has_setup_context(report, affordance):
        return "发育/能力"
    if action_has_exhaust_context(report, affordance):
        return "烧牌/清理"
    if affordance.get("action_key") == "combat/end_turn":
        return "结束回合"
    return "低信息"


def support_chips(supports: list[dict[str, Any]], *, raw: bool = False) -> str:
    if not supports:
        return "<span class='muted'>没有明确计划支持</span>"
    chips = []
    visible = supports if raw else supports[:4]
    for support in visible:
        plan = str(support.get("plan_name") or "")
        label = PLAN_LABELS.get(plan, plan)
        rank = support.get("rank")
        gap = support.get("score_gap_to_best")
        level = support.get("support_level")
        text = f"{label} #{rank}"
        if raw:
            text += f" ({level}, gap {gap})"
        elif int(gap or 0) > 0:
            text += f" / 差 {gap}"
        chips.append(chip(text, "chosen" if rank == 1 else "support"))
    if not raw and len(supports) > len(visible):
        chips.append(chip(f"+{len(supports) - len(visible)} more", "support"))
    return " ".join(chips)


def translated_tags(tags: list[Any]) -> str:
    rendered = []
    for tag in tags:
        text = str(tag)
        rendered.append(chip(TRADEOFF_LABELS.get(text, text), "tradeoff"))
    return " ".join(rendered)


def outcome_summary(affordance: dict[str, Any]) -> str:
    component = affordance.get("component_max") or {}
    return (
        f"<strong>防 {esc(component.get('block_score'))}</strong> / "
        f"<strong>伤 {esc(component.get('damage_score'))}</strong>"
        f"<div class='muted'>setup {esc(component.get('setup_score'))}, exhaust {esc(component.get('exhaust_value'))}, "
        f"seqs {esc(affordance.get('sequence_count'))}</div>"
    )


def risk_summary(affordance: dict[str, Any]) -> str:
    component = affordance.get("component_max") or {}
    tags = list(affordance.get("major_tradeoffs") or [])
    for reason in affordance.get("order_sensitive_reasons") or []:
        if reason not in tags:
            tags.append(reason)
    risks = list(affordance.get("risk_note_kinds") or [])
    risk_text = (
        f"<div class='muted'>key/random {esc(component.get('key_card_risk'))}/{esc(component.get('random_risk'))}</div>"
    )
    translated = translated_tags(tags)
    if risks:
        translated += " " + " ".join(chip(risk, "risk") for risk in risks)
    return (translated or "<span class='muted'>无显式副作用</span>") + risk_text


def render_affordances(report: dict[str, Any]) -> str:
    affordances = report.get("first_action_affordances") or []
    chosen = chosen_context(report)["chosen"]
    rows = []
    for affordance in affordances:
        clean_supports = filtered_supports(report, affordance)
        role = classify_action_role(report, affordance, clean_supports)
        supports = support_chips(clean_supports)
        raw_supports = support_chips(affordance.get("supported_plans") or [], raw=True)
        classes = []
        if affordance.get("action_key") == chosen:
            classes.append("chosen-row")
        rows.append(
            f"<tr class='{' '.join(classes)}'>"
            f"<td><strong>{esc(affordance.get('action_label'))}</strong><div class='muted mono'>{esc(affordance.get('action_key'))}</div></td>"
            f"<td><span class='role-pill'>{esc(role)}</span></td>"
            f"<td>{supports}<details><summary>raw evidence</summary>{raw_supports}</details></td>"
            f"<td>{outcome_summary(affordance)}</td>"
            f"<td>{risk_summary(affordance)}</td>"
            "</tr>"
        )
    body = "\n".join(rows) or "<tr><td colspan='5'>No first-action affordances in this report.</td></tr>"
    return f"""
    <section class="panel">
      <h2>First Action Roles</h2>
      <p class="muted">
        这张表只解释“第一手动作能兑现什么当前回合计划”。它不是推荐排序；同一动作可能属于进攻、防御、发育、烧牌等不同角色。
        raw evidence 保留原始 plan rank，默认先看角色、最佳用途和代价风险。
      </p>
      <table>
        <thead>
          <tr>
            <th>第一手动作</th><th>角色</th><th>最佳用途</th><th>结果范围</th><th>代价 / 风险</th>
          </tr>
        </thead>
        <tbody>{body}</tbody>
      </table>
    </section>
    """


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
    .summary-panel { border-color: #93c5fd; background: #f8fbff; }
    .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(170px, 1fr)); gap: 12px; }
    .stat { background: #f8fafc; border: 1px solid #e5e7eb; border-radius: 6px; padding: 10px; }
    .stat-label { color: #64748b; font-size: 12px; }
    .stat-value { font-size: 20px; font-weight: 700; margin: 3px 0; overflow-wrap: anywhere; }
    .stat-detail { color: #64748b; font-size: 12px; overflow-wrap: anywhere; }
    .plan-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: 12px; }
    .plan-card { border: 1px solid #e5e7eb; background: #fbfdff; border-radius: 8px; padding: 12px; }
    .chosen-plan { border-color: #2563eb; background: #eff6ff; }
    .plan-card header { display: flex; justify-content: space-between; gap: 8px; align-items: center; }
    .query-panel { border-color: #bfdbfe; background: #f8fbff; }
    .query-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(260px, 1fr)); gap: 12px; }
    .query-card { border: 1px solid #dbe3ef; background: #fff; border-radius: 8px; padding: 12px; }
    .query-card header { display: flex; justify-content: space-between; gap: 8px; align-items: center; margin-bottom: 8px; }
    .query-main { font-weight: 700; margin: 6px 0; line-height: 1.35; }
    .query-feasible { border-color: #86efac; }
    .query-partial { border-color: #fdba74; }
    .query-not_feasible { border-color: #fca5a5; }
    .query-not_applicable { opacity: 0.82; }
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
    .support { background: #eef2ff; border-color: #a5b4fc; }
    .feasible { background: #dcfce7; border-color: #86efac; }
    .partial { background: #ffedd5; border-color: #fdba74; }
    .not_feasible { background: #fee2e2; border-color: #fca5a5; }
    .not_applicable { background: #f1f5f9; border-color: #cbd5e1; }
    .tradeoff { background: #f0fdf4; border-color: #86efac; }
    .chosen { background: #dbeafe; border-color: #60a5fa; }
    .role-pill { display: inline-block; font-weight: 700; border: 1px solid #94a3b8; border-radius: 6px; padding: 5px 8px; background: #f8fafc; white-space: nowrap; }
    .affected { background: #fef2f2; border-color: #fca5a5; }
    .reason { background: #ecfdf5; border-color: #86efac; }
    .attention-list { margin: 8px 0 0; padding-left: 22px; line-height: 1.5; }
    details { margin-top: 6px; }
    summary { color: #64748b; cursor: pointer; font-size: 12px; }
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
    读的时候先看 Plan Queries 回答战术意图能否兑现，再看 First Action Roles 的证据和风险说明。
  </p>
  <p class="muted mono">report: {esc(report_path)}</p>
  <p class="muted mono">generated: {esc(datetime.now(timezone.utc).isoformat())}</p>
  {render_plan_queries(report)}
  {render_turn_summary(report)}
  {render_state(report)}
  {render_warnings(report)}
  {render_affordances(report)}
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
        "## Plan Queries",
    ]
    for query in report.get("plan_queries") or []:
        outcome = query.get("outcome") or {}
        lines.append(
            f"- `{query.get('query_name')}`: `{query.get('status')}`; "
            f"{query_primary_line(query)}; "
            f"sequence `{action_sequence_label(query.get('best_action_keys') or []) or 'none'}`; "
            f"outcome 防/漏/伤/剩能 `{outcome.get('block_after')}`/`{outcome.get('projected_unblocked_damage')}`/`{outcome.get('damage_done')}`/`{outcome.get('remaining_energy')}`; "
            f"deeper `{query.get('needs_deeper_search')}`"
        )
    lines.extend([
        "",
        "## Turn Summary",
        f"- 局面压力: `{pressure_label(state)}`",
        f"- 当前选择: `{one_line_action_summary(report, chosen_affordance(report))}`",
        f"- 进攻代表: `{one_line_action_summary(report, best_affordance_for_role(report, '进攻/击杀'))}`",
        f"- 防御代表: `{one_line_action_summary(report, best_affordance_for_role(report, '防御'))}`",
        f"- 攻防混合代表: `{one_line_action_summary(report, best_affordance_for_role(report, '攻防混合'))}`",
        f"- 注意: `{'; '.join(attention_items(report, chosen_affordance(report))) or '没有显式高风险提示'}`",
        "",
        "## First Action Roles",
    ])
    for affordance in report.get("first_action_affordances") or []:
        component = affordance.get("component_max") or {}
        clean_supports = filtered_supports(report, affordance)
        role = classify_action_role(report, affordance, clean_supports)
        visible_supports = clean_supports[:4]
        supports = ", ".join(
            f"{PLAN_LABELS.get(str(support.get('plan_name') or ''), support.get('plan_name'))}#{support.get('rank')}"
            + (f"/差{support.get('score_gap_to_best')}" if int(support.get("score_gap_to_best") or 0) > 0 else "")
            for support in visible_supports
        )
        if len(clean_supports) > len(visible_supports):
            supports += f", +{len(clean_supports) - len(visible_supports)} more"
        tags = list(affordance.get("major_tradeoffs") or [])
        for reason in affordance.get("order_sensitive_reasons") or []:
            if reason not in tags:
                tags.append(reason)
        tradeoffs = ", ".join(TRADEOFF_LABELS.get(str(item), str(item)) for item in tags)
        lines.append(
            f"- `{affordance.get('action_label')}`: role `{role}`; "
            f"use: {supports or '没有明确计划支持'}; "
            f"range 防/伤/setup/exhaust `{component.get('block_score')}`/`{component.get('damage_score')}`/`{component.get('setup_score')}`/`{component.get('exhaust_value')}`; "
            f"risk `{component.get('key_card_risk')},{component.get('random_risk')}`; "
            f"cost: {tradeoffs or '无显式副作用'}"
        )
    lines.extend(
        [
            "",
            "## Plan Tops",
        ]
    )
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
            compact[existing_idx] = (
                sequence if current_score > existing_score else existing,
                count + 1,
            )
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
