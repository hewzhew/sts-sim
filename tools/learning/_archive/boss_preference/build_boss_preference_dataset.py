#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


LABEL_TO_ID = {
    "prefer_a": 0,
    "prefer_b": 1,
    "close_enough": 2,
}

RATIONALE_TAGS = [
    "current_window_relief",
    "next_window_risk",
    "irreversible_resource_spend",
    "setup_timing",
]

def read_jsonl(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                yield json.loads(line)


def candidate_by_name(record, name: str):
    for candidate in record["candidates"]:
        if candidate["name"].lower() == name.lower():
            return candidate
    raise ValueError(f"missing candidate {name} in {record['case_id']}")


def build_features(record):
    a = candidate_by_name(record, "A")
    b = candidate_by_name(record, "B")
    state = record["state_summary"]

    features = {
        "state_player_hp": float(state["player_hp"]),
        "state_player_block": float(state["player_block"]),
        "state_energy": float(state["energy"]),
        "state_enemy_total_hp": float(state["enemy_total_hp"]),
        "state_visible_incoming": float(state["visible_incoming"]),
        "state_hand_size": float(state["hand_size"]),
        "state_playable_cards": float(state["playable_cards"]),
        "state_attack_cards_in_hand": float(state["attack_cards_in_hand"]),
        "state_skill_cards_in_hand": float(state["skill_cards_in_hand"]),
        "state_power_cards_in_hand": float(state["power_cards_in_hand"]),
    }

    for prefix, candidate in (("a", a), ("b", b)):
        line = candidate["line_summary"]
        features[f"{prefix}_play_count"] = float(line["play_count"])
        features[f"{prefix}_total_energy_spend"] = float(line["total_energy_spend"])
        features[f"{prefix}_attack_plays"] = float(line["attack_plays"])
        features[f"{prefix}_skill_plays"] = float(line["skill_plays"])
        features[f"{prefix}_power_plays"] = float(line["power_plays"])
        features[f"{prefix}_enemy_targeted_plays"] = float(line["enemy_targeted_plays"])
        features[f"{prefix}_untargeted_plays"] = float(line["untargeted_plays"])
        features[f"{prefix}_end_turn_steps"] = float(line["end_turn_steps"])
        breakdown = candidate["score_breakdown"]
        features[f"{prefix}_steps"] = float(breakdown["steps"])
        features[f"{prefix}_hp_loss"] = float(breakdown["hp_loss"])
        features[f"{prefix}_final_total_monster_hp"] = float(
            breakdown["final_total_monster_hp"]
        )
        features[f"{prefix}_threat_relief_before_first_enemy_window"] = float(
            breakdown["threat_relief_before_first_enemy_window"]
        )
        features[f"{prefix}_defense_gap_at_first_enemy_window"] = float(
            breakdown["defense_gap_at_first_enemy_window"]
        )
        features[f"{prefix}_dead_draw_burden_at_next_player_window"] = float(
            breakdown["dead_draw_burden_at_next_player_window"]
        )
        features[f"{prefix}_collateral_exhaust_cost_of_immediate_conversion"] = float(
            breakdown["collateral_exhaust_cost_of_immediate_conversion"]
        )

    features["delta_hp_loss"] = features["a_hp_loss"] - features["b_hp_loss"]
    features["delta_final_monster_hp"] = (
        features["a_final_total_monster_hp"] - features["b_final_total_monster_hp"]
    )
    features["delta_threat_relief"] = (
        features["a_threat_relief_before_first_enemy_window"]
        - features["b_threat_relief_before_first_enemy_window"]
    )
    features["delta_defense_gap"] = (
        features["a_defense_gap_at_first_enemy_window"]
        - features["b_defense_gap_at_first_enemy_window"]
    )
    features["delta_dead_draw_burden"] = (
        features["a_dead_draw_burden_at_next_player_window"]
        - features["b_dead_draw_burden_at_next_player_window"]
    )
    features["delta_collateral_exhaust_cost"] = (
        features["a_collateral_exhaust_cost_of_immediate_conversion"]
        - features["b_collateral_exhaust_cost_of_immediate_conversion"]
    )
    features["delta_steps"] = features["a_steps"] - features["b_steps"]
    features["delta_play_count"] = features["a_play_count"] - features["b_play_count"]
    features["delta_total_energy_spend"] = (
        features["a_total_energy_spend"] - features["b_total_energy_spend"]
    )
    features["delta_attack_plays"] = features["a_attack_plays"] - features["b_attack_plays"]
    features["delta_skill_plays"] = features["a_skill_plays"] - features["b_skill_plays"]
    features["delta_power_plays"] = features["a_power_plays"] - features["b_power_plays"]
    features["delta_enemy_targeted_plays"] = (
        features["a_enemy_targeted_plays"] - features["b_enemy_targeted_plays"]
    )
    features["delta_untargeted_plays"] = (
        features["a_untargeted_plays"] - features["b_untargeted_plays"]
    )
    return features


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--ledger", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    args = parser.parse_args()

    rows = []
    for record in read_jsonl(args.ledger):
        label = record["expected_label"]
        if label not in LABEL_TO_ID:
            raise ValueError(f"unsupported label {label}")
        rows.append(
            {
                "case_id": record["case_id"],
                "validation_pack": record["validation_pack"],
                "boss": record["boss"],
                "label": label,
                "label_id": LABEL_TO_ID[label],
                "expected_label": record["expected_label"],
                "observed_label": record["observed_label"],
                "features": build_features(record),
            }
        )

    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=True) + "\n")
    print(f"wrote {len(rows)} dataset rows to {args.out}")


if __name__ == "__main__":
    main()
