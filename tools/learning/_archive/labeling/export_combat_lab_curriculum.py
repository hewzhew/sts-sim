#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

from combat_reranker_common import curriculum_tag_from_spec_name, parse_move_label, write_json, write_jsonl
from curriculum_dynamic_teacher import dynamic_teacher_for_row

REPO_ROOT = Path(__file__).resolve().parents[2]


def find_combat_lab_binary(explicit: Path | None) -> Path:
    if explicit and explicit.exists():
        return explicit
    exe = REPO_ROOT / "target" / "release" / "combat_lab.exe"
    if exe.exists():
        return exe
    bin_path = REPO_ROOT / "target" / "release" / "combat_lab"
    if bin_path.exists():
        return bin_path
    raise SystemExit("missing combat_lab release binary; run cargo build --release --bin combat_lab first")


def run_spec(binary: Path, spec_path: Path, episodes: int, depth: int, base_seed: int, out_dir: Path) -> None:
    cmd = [
        str(binary),
        "--author-spec",
        str(spec_path),
        "--episodes",
        str(episodes),
        "--policy",
        "bot",
        "--depth",
        str(depth),
        "--variant-mode",
        "reshuffle_draw",
        "--base-seed",
        str(base_seed),
        "--out-dir",
        str(out_dir),
    ]
    subprocess.run(cmd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def build_state_snapshot(step: dict[str, Any], trace: dict[str, Any]) -> dict[str, Any]:
    monsters = []
    for monster in step.get("monsters_before") or []:
        monsters.append(
            {
                "id": monster.get("id") or f"monster_{monster.get('slot', 0)}",
                "name": monster.get("id") or f"monster_{monster.get('slot', 0)}",
                "current_hp": int(monster.get("hp") or 0),
                "max_hp": int(monster.get("max_hp") or monster.get("hp") or 0),
                "block": int(monster.get("block") or 0),
                "intent": monster.get("intent"),
                "powers": [],
            }
        )
    hand = []
    for index, card in enumerate(step.get("hand_before") or []):
        hand.append(
            {
                "id": f"{card}:{index}",
                "name": str(card).split("(", 1)[0],
                "cost": 0,
                "upgrades": 0,
            }
        )
    return {
        "act": 0,
        "floor": 0,
        "gold": 0,
        "player": {
            "current_hp": int(step.get("player_hp_before") or 0),
            "max_hp": int(trace.get("final_player_hp") or step.get("player_hp_before") or 0),
            "block": int(step.get("player_block_before") or 0),
            "energy": int(step.get("energy_before") or 0),
            "powers": [],
        },
        "monsters": monsters,
        "zones": {
            "hand_count": int(step.get("hand_size_before") or len(hand)),
            "draw_count": int(step.get("draw_size_before") or 0),
            "discard_count": int(step.get("discard_size_before") or 0),
            "exhaust_count": 0,
            "hand": hand,
            "draw": [],
            "discard": [],
            "exhaust": [],
        },
    }


def hand_card_name(hand_before: list[str], zero_index: int | None) -> str | None:
    if zero_index is None:
        return None
    if zero_index < 0 or zero_index >= len(hand_before):
        return None
    return str(hand_before[zero_index]).split("(", 1)[0]


def resolve_action_move_label(action_debug: str | None, hand_before: list[str]) -> str:
    text = str(action_debug or "")
    match = re.match(r"^PlayCard \{ card_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", text)
    if match:
        zero_slot = int(match.group("slot"))
        slot = zero_slot + 1
        card_name = hand_card_name(hand_before, zero_slot)
        target_idx = match.group("target_idx")
        if target_idx is None:
            return f"Play #{slot} {card_name}" if card_name else f"Play #{slot}"
        return f"Play #{slot} {card_name} @{int(target_idx)}" if card_name else f"Play #{slot} @{int(target_idx)}"
    match = re.match(r"^UsePotion \{ potion_index: (?P<slot>\d+), target: (?P<target>Some\((?P<target_idx>\d+)\)|None) \}$", text)
    if match:
        slot = int(match.group("slot"))
        target_idx = match.group("target_idx")
        if target_idx is None:
            return f"UsePotion#{slot}"
        return f"UsePotion#{slot} @{int(target_idx)}"
    if text == "EndTurn":
        return "EndTurn"
    return text


def normalize_candidates(step: dict[str, Any]) -> list[dict[str, Any]]:
    hand_before = list(step.get("hand_before") or [])
    raw_candidates = (step.get("policy_decision") or {}).get("candidate_scores") or []
    normalized = []
    for search_rank, candidate in enumerate(raw_candidates):
        action = candidate.get("action") or {}
        move_label = resolve_action_move_label(action.get("debug"), hand_before)
        parsed = parse_move_label(move_label)
        normalized.append(
            {
                "move_label": move_label,
                "move_family": parsed["move_family"],
                "card_name": parsed["card_name"],
                "slot_index": parsed["slot_index"],
                "has_target": parsed["has_target"],
                "target_index": parsed["target_index"],
                "score": float(candidate.get("score") or 0.0),
                "source": candidate.get("source"),
                "visits": candidate.get("visits"),
                "avg_score": float(candidate.get("avg_score") or candidate.get("score") or 0.0),
                "search_rank": search_rank,
            }
        )
    return normalized


def row_tags(step: dict[str, Any], chosen_move: str, normalized_candidates: list[dict[str, Any]]) -> list[str]:
    tags = [str(tag) for tag in (step.get("bad_action_tags") or [])]
    monsters = step.get("monsters_before") or []
    incoming = int(step.get("state_features_preview", {}).get("incoming_damage") or 0.0)
    living_hps = [int(monster.get("hp") or 0) for monster in monsters if int(monster.get("hp") or 0) > 0]
    lowest_hp = min(living_hps, default=999)
    candidate_moves = [candidate.get("move_label") or "" for candidate in normalized_candidates]
    has_non_end_turn_candidate = any(move and move != "EndTurn" for move in candidate_moves)
    if lowest_hp <= 12 and chosen_move == "EndTurn" and has_non_end_turn_candidate:
        tags.append("kill_now_missed")
    if lowest_hp <= 12 and "Defend" in chosen_move and any("Defend" not in move and move != "EndTurn" for move in candidate_moves):
        tags.append("kill_now_missed")
    if incoming <= 5 and "Defend" in chosen_move and has_non_end_turn_candidate:
        tags.append("overdefend_light_pressure")
    if any("Flex" in str(card) for card in (step.get("hand_before") or [])) and "Flex" not in chosen_move and any("Flex" in move for move in candidate_moves):
        tags.append("setup_flex_missed")
    if "Power Through" in chosen_move and incoming <= 3 and any("Power Through" not in move and move != "EndTurn" for move in candidate_moves):
        tags.append("power_through_played_without_incoming")
    if any(token in chosen_move for token in ("Slimed", "Wound", "Dazed", "Burn")) and any(
        all(token not in move for token in ("Slimed", "Wound", "Dazed", "Burn")) and move != "EndTurn"
        for move in candidate_moves
    ):
        tags.append("survival_override_played_status_or_curse")
    return sorted(set(tags))


def row_from_step(spec_name: str, trace: dict[str, Any], step: dict[str, Any]) -> dict[str, Any]:
    hand_before = list(step.get("hand_before") or [])
    chosen_move = resolve_action_move_label(step.get("chosen_action"), hand_before)
    normalized_candidates = normalize_candidates(step)
    tags = row_tags(step, chosen_move, normalized_candidates)
    final_action = ((step.get("policy_decision") or {}).get("final_action") or {})
    row = {
        "sample_origin": "combat_lab_spec",
        "teacher_source": "combat_lab_policy_trace",
        "curriculum_tag": curriculum_tag_from_spec_name(spec_name),
        "state_source": "combat_lab_trace",
        "label_source": "combat_lab_policy_trace",
        "label_strength": "baseline_weak",
        "sample_id": f"{spec_name}::ep{trace.get('episode_id')}::step{step.get('step_index')}",
        "spec_name": spec_name,
        "run_id": None,
        "seed": trace.get("seed"),
        "episode_id": trace.get("episode_id"),
        "turn_index": step.get("turn_index"),
        "step_index": step.get("step_index"),
        "chosen_move": chosen_move,
        "action_kind": step.get("action_kind") or final_action.get("kind"),
        "outcome": trace.get("outcome"),
        "final_player_hp": trace.get("final_player_hp"),
        "path_score": trace.get("path_score"),
        "sample_tags": tags,
        "snapshot_normalized_state": build_state_snapshot(step, trace),
        "state_features_preview": step.get("state_features_preview") or {},
        "state_features_full": step.get("state_features_full") or {},
        "bad_action_tags": step.get("bad_action_tags") or [],
        "candidate_scores": (step.get("policy_decision") or {}).get("candidate_scores") or [],
        "normalized_candidates": normalized_candidates,
        "top_candidate_move": normalized_candidates[0]["move_label"] if normalized_candidates else None,
        "chosen_matches_top_candidate": bool(normalized_candidates and chosen_move == normalized_candidates[0]["move_label"]),
    }
    dynamic = dynamic_teacher_for_row(row)
    row["dynamic_teacher_preferred_moves"] = dynamic.get("preferred_moves") or []
    row["dynamic_teacher_best_move"] = dynamic.get("oracle_best_move")
    row["dynamic_teacher_margin"] = float(dynamic.get("oracle_margin") or 0.0)
    row["dynamic_teacher_score"] = dynamic.get("best_teacher_score")
    row["dynamic_teacher_chosen_score"] = dynamic.get("chosen_teacher_score")
    row["dynamic_teacher_tie_tolerance"] = dynamic.get("tie_tolerance")
    row["dynamic_teacher_label_strength"] = dynamic.get("label_strength")
    row["dynamic_teacher_source"] = dynamic.get("teacher_source")
    row["dynamic_teacher_active"] = bool(dynamic.get("active"))
    if row["dynamic_teacher_active"]:
        row["sample_tags"] = sorted(set([*row["sample_tags"], "dynamic_semantic_disagreement"]))
    return row


def main() -> int:
    parser = argparse.ArgumentParser(description="Run combat_lab specs locally and export curriculum rows.")
    parser.add_argument(
        "--spec-dir",
        default=REPO_ROOT / "data" / "combat_lab" / "specs",
        type=Path,
        help="Directory containing combat_lab author specs.",
    )
    parser.add_argument(
        "--out-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset",
        type=Path,
        help="Directory where curriculum rows and summaries will be written.",
    )
    parser.add_argument(
        "--combat-lab-binary",
        default=None,
        type=Path,
        help="Optional explicit path to combat_lab release binary.",
    )
    parser.add_argument("--episodes", default=8, type=int, help="Episodes to run per spec.")
    parser.add_argument("--depth", default=6, type=int, help="Search depth for combat_lab bot policy.")
    parser.add_argument("--base-seed", default=1, type=int, help="Base seed for combat_lab runs.")
    parser.add_argument(
        "--sample-limit-per-spec",
        default=12,
        type=int,
        help="Maximum exported curriculum rows per spec after high-value filtering.",
    )
    args = parser.parse_args()

    binary = find_combat_lab_binary(args.combat_lab_binary)
    spec_paths = sorted(args.spec_dir.glob("*.json"))
    exported_rows: list[dict[str, Any]] = []
    spec_counts: Counter[str] = Counter()
    tag_counts: Counter[str] = Counter()
    summary_specs: list[dict[str, Any]] = []

    with tempfile.TemporaryDirectory(prefix="combat_lab_curriculum_", dir=str(REPO_ROOT / "tmp")) as tmp_dir_name:
        tmp_dir = Path(tmp_dir_name)
        for spec_path in spec_paths:
            spec_name = spec_path.stem
            spec_out = tmp_dir / spec_name
            run_spec(binary, spec_path, args.episodes, args.depth, args.base_seed, spec_out)
            trace_paths = sorted(spec_out.glob("trace_*.json"))
            spec_rows: list[dict[str, Any]] = []
            for trace_path in trace_paths:
                with trace_path.open("r", encoding="utf-8") as handle:
                    trace = json.load(handle)
                for step in trace.get("steps") or []:
                    row = row_from_step(spec_name, trace, step)
                    if row["sample_tags"] or (step.get("bad_action_tags") or []) or row.get("dynamic_teacher_active"):
                        spec_rows.append(row)
            spec_rows.sort(
                key=lambda row: (
                    float(row.get("dynamic_teacher_margin") or 0.0),
                    len(row.get("sample_tags") or []),
                    float(row.get("path_score") or 0.0),
                ),
                reverse=True,
            )
            selected_rows = spec_rows[: args.sample_limit_per_spec]
            exported_rows.extend(selected_rows)
            spec_counts[spec_name] = len(selected_rows)
            for row in selected_rows:
                for tag in row.get("sample_tags") or []:
                    tag_counts[tag] += 1
            summary_specs.append(
                {
                    "spec_name": spec_name,
                    "curriculum_tag": curriculum_tag_from_spec_name(spec_name),
                    "exported_rows": len(selected_rows),
                    "episodes": args.episodes,
                    "trace_count": len(trace_paths),
                }
            )

    summary = {
        "spec_dir": str(args.spec_dir),
        "combat_lab_binary": str(binary),
        "episodes_per_spec": args.episodes,
        "depth": args.depth,
        "base_seed": args.base_seed,
        "spec_count": len(spec_paths),
        "exported_rows": len(exported_rows),
        "rows_per_spec": dict(spec_counts),
        "sample_tag_counts": dict(tag_counts),
        "dynamic_teacher_active_rows": int(sum(1 for row in exported_rows if row.get("dynamic_teacher_active"))),
        "candidate_count_histogram": dict(Counter(len(row.get("normalized_candidates") or []) for row in exported_rows)),
        "specs": summary_specs,
        "notes": [
            "combat_lab curriculum rows are local offline tactical curriculum samples",
            "this exporter keeps tagged rows and dynamic semantic disagreements against the chosen move",
            "rows keep normalized candidate move labels so they can feed offline curriculum packing",
            "rows are intended for review and weak curriculum teacher construction, not direct runtime use",
        ],
    }
    write_jsonl(args.out_dir / "combat_lab_curriculum_rows.jsonl", exported_rows)
    write_json(args.out_dir / "combat_lab_curriculum_summary.json", summary)

    print(json.dumps(summary, indent=2, ensure_ascii=False))
    print(f"wrote combat_lab curriculum rows to {args.out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
