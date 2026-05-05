#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

import numpy as np
from sklearn.feature_extraction import DictVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import (
    accuracy_score,
    balanced_accuracy_score,
    brier_score_loss,
    classification_report,
    f1_score,
    precision_score,
    recall_score,
)

from combat_rl_common import REPO_ROOT, iter_jsonl, write_json, write_jsonl

TRIGGER_TARGET = "needs_exact_trigger_target"
REGIME_TARGET = "regime"
VALID_TARGETS = {TRIGGER_TARGET, REGIME_TARGET}
TARGET_ALIASES = {
    "trigger": TRIGGER_TARGET,
    TRIGGER_TARGET: TRIGGER_TARGET,
    REGIME_TARGET: REGIME_TARGET,
}


def load_rows(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [row for _, row in iter_jsonl(path)]


def add_indicator(features: dict[str, float], prefix: str, name: str, value: float = 1.0) -> None:
    if not name:
        return
    features[f"{prefix}={name}"] = float(value)


def add_numeric(features: dict[str, float], name: str, value: Any) -> None:
    if value is None:
        return
    try:
        features[name] = float(value)
    except (TypeError, ValueError):
        return


def safe_balanced_accuracy(y_true: np.ndarray, y_pred: np.ndarray) -> float:
    if y_true.size == 0:
        return 0.0
    if len(np.unique(y_true)) < 2:
        return float(accuracy_score(y_true, y_pred))
    return float(balanced_accuracy_score(y_true, y_pred))


def state_corpus_feature_dict(row: dict[str, Any]) -> dict[str, float]:
    features: dict[str, float] = {}
    snapshot = row.get("combat_snapshot") or {}
    player = snapshot.get("player") or {}
    monsters = snapshot.get("monsters") or []
    turn = snapshot.get("turn") or {}
    zones = snapshot.get("zones") or {}
    runtime = snapshot.get("runtime") or {}

    add_indicator(features, "player_class", str(row.get("player_class") or ""))
    add_indicator(features, "engine_state", str(row.get("engine_state") or ""))
    add_indicator(features, "screen_type", str(row.get("screen_type") or "unknown"))
    add_numeric(features, "ascension_level", row.get("ascension_level") or 0)
    add_numeric(features, "living_monsters", row.get("living_monsters") or 0)
    add_numeric(features, "legal_moves", row.get("legal_moves") or 0)
    add_numeric(features, "reduced_legal_moves", row.get("reduced_legal_moves") or 0)

    encounter_signature = row.get("encounter_signature") or []
    for encounter in encounter_signature:
        add_indicator(features, "encounter", str(encounter))

    current_hp = float(player.get("current_hp") or 0.0)
    max_hp = float(player.get("max_hp") or 0.0)
    add_numeric(features, "player_current_hp", current_hp)
    add_numeric(features, "player_max_hp", max_hp)
    add_numeric(features, "player_hp_ratio", current_hp / max(max_hp, 1.0))
    add_numeric(features, "player_block", player.get("block") or 0)
    add_numeric(features, "player_energy_master", player.get("energy_master") or 0)
    add_numeric(features, "player_gold", player.get("gold") or 0)
    add_indicator(features, "player_stance", str(player.get("stance") or "unknown"))

    potions = player.get("potions") or []
    add_numeric(features, "filled_potion_slots", sum(1 for potion in potions if potion))
    relics = player.get("relics") or []
    add_numeric(features, "player_relic_count", len(relics))
    for relic in relics:
        add_indicator(features, "relic", str(relic))

    player_powers = player.get("powers") or []
    add_numeric(features, "player_power_count", len(player_powers))
    for power in player_powers:
        power_id = str(power.get("id") or "")
        add_indicator(features, "player_power_present", power_id)
        add_numeric(features, f"player_power_amount::{power_id}", power.get("amount") or 0)

    add_numeric(features, "turn_count", turn.get("turn_count") or 0)
    add_numeric(features, "turn_energy", turn.get("energy") or 0)
    add_numeric(features, "cards_played_this_turn", turn.get("cards_played_this_turn") or 0)
    add_numeric(features, "attacks_played_this_turn", turn.get("attacks_played_this_turn") or 0)
    add_indicator(features, "turn_phase", str(turn.get("phase") or "unknown"))

    add_numeric(features, "action_queue_len", runtime.get("action_queue_len") or 0)
    add_numeric(features, "combat_smoked", 1 if runtime.get("combat_smoked") else 0)
    add_numeric(features, "combat_mugged", 1 if runtime.get("combat_mugged") else 0)

    hand = zones.get("hand") or []
    add_numeric(features, "hand_count", len(hand))
    add_numeric(features, "draw_count", zones.get("draw_count") or 0)
    add_numeric(features, "discard_count", zones.get("discard_count") or 0)
    add_numeric(features, "exhaust_count", zones.get("exhaust_count") or 0)
    add_numeric(features, "limbo_count", zones.get("limbo_count") or 0)
    add_numeric(features, "queued_count", zones.get("queued_count") or 0)
    add_numeric(features, "hand_unique_cards", len({str(card.get("id") or "") for card in hand if card.get("id")}))

    zero_cost = one_cost = two_plus_cost = 0
    for card in hand:
        card_id = str(card.get("id") or "")
        if card_id:
            add_indicator(features, "hand_card", card_id)
            features[f"hand_card_count::{card_id}"] = features.get(f"hand_card_count::{card_id}", 0.0) + 1.0
        upgrades = int(card.get("upgrades") or 0)
        add_numeric(features, f"hand_upgrades::{card_id}", upgrades)
        raw_cost = card.get("cost_for_turn")
        if raw_cost is None:
            raw_cost = card.get("cost")
        try:
            cost = int(raw_cost)
        except (TypeError, ValueError):
            cost = 0
        if cost <= 0:
            zero_cost += 1
        elif cost == 1:
            one_cost += 1
        else:
            two_plus_cost += 1
        if card.get("free_to_play_once"):
            add_numeric(features, "free_to_play_once_count", features.get("free_to_play_once_count", 0.0) + 1.0)
    add_numeric(features, "zero_cost_cards_in_hand", zero_cost)
    add_numeric(features, "one_cost_cards_in_hand", one_cost)
    add_numeric(features, "two_plus_cost_cards_in_hand", two_plus_cost)

    living_monsters = [monster for monster in monsters if int(monster.get("current_hp") or 0) > 0 and not monster.get("is_escaped")]
    add_numeric(features, "monster_count", len(monsters))
    add_numeric(features, "living_monster_count_snapshot", len(living_monsters))
    add_numeric(features, "monster_total_hp", sum(float(monster.get("current_hp") or 0) for monster in living_monsters))
    add_numeric(features, "monster_total_block", sum(float(monster.get("block") or 0) for monster in living_monsters))
    add_numeric(
        features,
        "monster_lowest_hp",
        min((float(monster.get("current_hp") or 0) for monster in living_monsters), default=0.0),
    )
    add_numeric(
        features,
        "monster_highest_hp",
        max((float(monster.get("current_hp") or 0) for monster in living_monsters), default=0.0),
    )

    for monster in living_monsters:
        monster_id = str(monster.get("id") or "")
        add_indicator(features, "monster", monster_id)
        features[f"monster_count::{monster_id}"] = features.get(f"monster_count::{monster_id}", 0.0) + 1.0
        add_numeric(features, f"monster_hp::{monster_id}", monster.get("current_hp") or 0)
        add_numeric(features, f"monster_block::{monster_id}", monster.get("block") or 0)
        add_numeric(features, f"monster_planned_move::{monster_id}", monster.get("planned_move_id") or 0)
        for power in monster.get("powers") or []:
            power_id = str(power.get("id") or "")
            add_indicator(features, "monster_power_present", power_id)
            features[f"monster_power_count::{power_id}"] = features.get(f"monster_power_count::{power_id}", 0.0) + 1.0
            add_numeric(
                features,
                f"monster_power_amount::{power_id}",
                features.get(f"monster_power_amount::{power_id}", 0.0) + float(power.get("amount") or 0),
            )

    return features


def top_coefficients(
    coef: np.ndarray,
    feature_names: list[str],
    *,
    positive: bool,
    limit: int = 12,
) -> list[dict[str, float]]:
    order = np.argsort(coef)
    if positive:
        order = order[::-1]
    selected = order[:limit]
    return [
        {
            "feature": feature_names[int(index)],
            "weight": float(coef[int(index)]),
        }
        for index in selected
    ]


def binary_metrics(y_true: np.ndarray, y_pred: np.ndarray, y_prob: np.ndarray) -> dict[str, Any]:
    if y_true.size == 0:
        return {"rows": 0}
    metrics: dict[str, Any] = {
        "rows": int(y_true.size),
        "positive_rows": int(y_true.sum()),
        "negative_rows": int((y_true == 0).sum()),
        "accuracy": float(accuracy_score(y_true, y_pred)),
        "balanced_accuracy": safe_balanced_accuracy(y_true, y_pred),
        "precision": float(precision_score(y_true, y_pred, zero_division=0)),
        "recall": float(recall_score(y_true, y_pred, zero_division=0)),
        "f1": float(f1_score(y_true, y_pred, zero_division=0)),
    }
    if len(np.unique(y_true)) >= 2:
        metrics["brier"] = float(brier_score_loss(y_true.astype(np.float32), y_prob.astype(np.float32)))
    return metrics


def multiclass_metrics(
    y_true: np.ndarray,
    y_pred: np.ndarray,
    label_names: list[str],
) -> dict[str, Any]:
    if y_true.size == 0:
        return {"rows": 0}
    report = classification_report(
        y_true,
        y_pred,
        labels=list(range(len(label_names))),
        target_names=label_names,
        output_dict=True,
        zero_division=0,
    )
    return {
        "rows": int(y_true.size),
        "accuracy": float(accuracy_score(y_true, y_pred)),
        "balanced_accuracy": safe_balanced_accuracy(y_true, y_pred),
        "class_report": report,
    }


def binary_label_summary(rows: list[dict[str, Any]], field: str) -> dict[str, int]:
    positives = sum(1 for row in rows if row.get(field))
    negatives = len(rows) - positives
    return {
        "rows": len(rows),
        "positive_rows": positives,
        "negative_rows": negatives,
    }


def multiclass_label_summary(rows: list[dict[str, Any]], field: str) -> dict[str, Any]:
    counts: dict[str, int] = {}
    for row in rows:
        label = str(row.get(field) or "unknown")
        counts[label] = counts.get(label, 0) + 1
    return {
        "rows": len(rows),
        "distinct_label_count": len(counts),
        "label_counts": counts,
    }


def parse_requested_targets(raw_targets: str) -> list[str]:
    requested: list[str] = []
    for raw_piece in raw_targets.replace(";", ",").split(","):
        piece = raw_piece.strip()
        if not piece:
            continue
        canonical = TARGET_ALIASES.get(piece)
        if canonical is None:
            valid = ", ".join(sorted(VALID_TARGETS | {"trigger"}))
            raise SystemExit(f"unknown auxiliary target '{piece}'; valid targets: {valid}")
        if canonical not in requested:
            requested.append(canonical)
    if not requested:
        raise SystemExit("no auxiliary targets requested")
    return requested


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Train a lightweight state-corpus auxiliary baseline for exact-trigger and regime targets."
    )
    parser.add_argument(
        "--split-dir",
        default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset" / "state_corpus_split",
        type=Path,
        help="Directory containing train.jsonl / val.jsonl / test.jsonl from split-state-corpus.",
    )
    parser.add_argument(
        "--output-prefix",
        default="state_corpus_aux_baseline",
        help="Prefix for metrics and predictions artifacts.",
    )
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    parser.add_argument(
        "--targets",
        default=f"{TRIGGER_TARGET},{REGIME_TARGET}",
        help=(
            "Comma-separated auxiliary targets to train. "
            "Supported: needs_exact_trigger_target (alias: trigger), regime"
        ),
    )
    args = parser.parse_args()

    metrics_out = args.metrics_out or (args.split_dir / f"{args.output_prefix}_metrics.json")
    predictions_out = args.predictions_out or (args.split_dir / f"{args.output_prefix}_predictions.jsonl")
    requested_targets = parse_requested_targets(args.targets)

    train_rows = load_rows(args.split_dir / "train.jsonl")
    val_rows = load_rows(args.split_dir / "val.jsonl")
    test_rows = load_rows(args.split_dir / "test.jsonl")
    if not train_rows:
        raise SystemExit("no state-corpus train rows found")

    vectorizer = DictVectorizer(sparse=True)
    x_train = vectorizer.fit_transform(state_corpus_feature_dict(row) for row in train_rows)
    if hasattr(vectorizer, "get_feature_names_out"):
        feature_names = list(vectorizer.get_feature_names_out())
    else:
        feature_names = list(vectorizer.feature_names_)

    trigger_train_summary = binary_label_summary(train_rows, TRIGGER_TARGET)
    trigger_y_train = np.asarray(
        [1 if row.get(TRIGGER_TARGET) else 0 for row in train_rows],
        dtype=np.int32,
    )
    regime_train_summary = multiclass_label_summary(train_rows, REGIME_TARGET)

    supported_targets: list[str] = []
    skipped_targets: dict[str, str] = {}

    trigger_model: LogisticRegression | None = None
    if TRIGGER_TARGET in requested_targets and len(np.unique(trigger_y_train)) >= 2:
        trigger_model = LogisticRegression(
            max_iter=1000,
            solver="liblinear",
            class_weight="balanced",
            random_state=0,
        )
        trigger_model.fit(x_train, trigger_y_train)
        supported_targets.append(TRIGGER_TARGET)
    elif TRIGGER_TARGET in requested_targets:
        skipped_targets[TRIGGER_TARGET] = "single_class_train"

    regime_labels = sorted({str(row.get(REGIME_TARGET) or "unknown") for row in train_rows})
    regime_to_index: dict[str, int] = {}
    regime_model: LogisticRegression | None = None
    if REGIME_TARGET in requested_targets and len(regime_labels) >= 2:
        regime_to_index = {label: index for index, label in enumerate(regime_labels)}
        regime_y_train = np.asarray([regime_to_index[str(row.get(REGIME_TARGET) or "unknown")] for row in train_rows], dtype=np.int32)
        regime_model = LogisticRegression(
            max_iter=1000,
            solver="lbfgs",
            class_weight="balanced",
            random_state=0,
        )
        regime_model.fit(x_train, regime_y_train)
        supported_targets.append(REGIME_TARGET)
    elif REGIME_TARGET in requested_targets:
        skipped_targets[REGIME_TARGET] = "single_class_train"

    if not supported_targets:
        raise SystemExit(
            "no supported auxiliary targets: "
            + ", ".join(f"{target}={reason}" for target, reason in skipped_targets.items())
        )

    def evaluate_split(rows: list[dict[str, Any]], split_name: str) -> tuple[dict[str, Any], list[dict[str, Any]]]:
        if not rows:
            return {"rows": 0}, []
        x = vectorizer.transform(state_corpus_feature_dict(row) for row in rows)
        raw_regime_labels = [str(row.get(REGIME_TARGET) or "unknown") for row in rows]
        split_metrics: dict[str, Any] = {"rows": len(rows)}

        trigger_y = np.asarray([1 if row.get(TRIGGER_TARGET) else 0 for row in rows], dtype=np.int32)
        trigger_prob: np.ndarray | None = None
        trigger_pred: np.ndarray | None = None
        if trigger_model is not None:
            trigger_prob = trigger_model.predict_proba(x)[:, 1]
            trigger_pred = (trigger_prob >= 0.5).astype(np.int32)
            split_metrics["trigger"] = binary_metrics(trigger_y, trigger_pred, trigger_prob)
        elif TRIGGER_TARGET in requested_targets:
            split_metrics["trigger"] = {
                "supported": False,
                "reason": skipped_targets[TRIGGER_TARGET],
                **binary_label_summary(rows, TRIGGER_TARGET),
            }
        else:
            split_metrics["trigger"] = {
                "requested": False,
                **binary_label_summary(rows, TRIGGER_TARGET),
            }

        regime_pred: np.ndarray | None = None
        regime_prob: np.ndarray | None = None
        if regime_model is not None:
            unseen_regimes = sorted({label for label in raw_regime_labels if label not in regime_to_index})
            mapped_regimes = np.asarray([regime_to_index.get(label, -1) for label in raw_regime_labels], dtype=np.int32)
            regime_pred = regime_model.predict(x)
            regime_prob = regime_model.predict_proba(x)

            valid_regime_mask = mapped_regimes >= 0
            regime_metrics_payload = multiclass_metrics(
                mapped_regimes[valid_regime_mask],
                regime_pred[valid_regime_mask],
                regime_labels,
            )
            regime_metrics_payload["unseen_regime_labels"] = unseen_regimes
            regime_metrics_payload["unseen_regime_row_count"] = int((~valid_regime_mask).sum())
            split_metrics["regime"] = regime_metrics_payload
        elif REGIME_TARGET in requested_targets:
            split_metrics["regime"] = {
                "supported": False,
                "reason": skipped_targets[REGIME_TARGET],
                **multiclass_label_summary(rows, REGIME_TARGET),
            }
        else:
            split_metrics["regime"] = {
                "requested": False,
                **multiclass_label_summary(rows, REGIME_TARGET),
            }

        predictions: list[dict[str, Any]] = []
        for index, row in enumerate(rows):
            regime_probs = {}
            if regime_prob is not None:
                regime_probs = {
                    label: float(regime_prob[index][class_index])
                    for class_index, label in enumerate(regime_labels)
                }
            predictions.append(
                {
                    "split": split_name,
                    "sample_id": row.get("sample_id"),
                    "source_kind": row.get("source_kind"),
                    "run_id": row.get("run_id"),
                    "response_id": row.get("response_id"),
                    "frame_id": row.get("frame_id"),
                    "requested_targets": requested_targets,
                    "actual_needs_exact_trigger_target": bool(row.get(TRIGGER_TARGET)),
                    "pred_needs_exact_trigger_target": None if trigger_pred is None else bool(trigger_pred[index]),
                    "trigger_probability": None if trigger_prob is None else float(trigger_prob[index]),
                    "actual_regime": raw_regime_labels[index],
                    "pred_regime": None if regime_pred is None else regime_labels[int(regime_pred[index])],
                    "regime_probabilities": regime_probs,
                    "curriculum_buckets": row.get("curriculum_buckets") or [],
                }
            )

        return split_metrics, predictions

    train_metrics, train_predictions = evaluate_split(train_rows, "train")
    val_metrics, val_predictions = evaluate_split(val_rows, "val")
    test_metrics, test_predictions = evaluate_split(test_rows, "test")

    metrics = {
        "model": "logistic_state_corpus_aux_baseline",
        "split_dir": str(args.split_dir),
        "output_prefix": args.output_prefix,
        "feature_count": len(feature_names),
        "requested_targets": requested_targets,
        "supported_targets": supported_targets,
        "skipped_targets": skipped_targets,
        "train_rows": len(train_rows),
        "val_rows": len(val_rows),
        "test_rows": len(test_rows),
        "train_label_summary": {
            TRIGGER_TARGET: trigger_train_summary,
            REGIME_TARGET: regime_train_summary,
        },
        "regime_labels": regime_labels,
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "trigger_feature_weights": (
            {
                "top_positive": top_coefficients(trigger_model.coef_[0], feature_names, positive=True),
                "top_negative": top_coefficients(trigger_model.coef_[0], feature_names, positive=False),
            }
            if trigger_model is not None
            else {}
        ),
        "regime_feature_weights": (
            {
                label: {
                    "top_positive": top_coefficients(regime_model.coef_[index], feature_names, positive=True),
                    "top_negative": top_coefficients(regime_model.coef_[index], feature_names, positive=False),
                }
                for index, label in enumerate(regime_labels)
            }
            if regime_model is not None
            else {}
        ),
        "notes": [
            "baseline is state-level and auxiliary only; it does not predict final actions",
            "targets are exact-trigger need and regime classification from state-corpus records",
            "features intentionally exclude decision_audit-derived targets such as regime, exact verdicts, and screened-out counts",
            "single-class train targets are skipped instead of aborting the whole auxiliary run",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, train_predictions + val_predictions + test_predictions)

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote state-corpus auxiliary metrics to {metrics_out}")
    print(f"wrote state-corpus auxiliary predictions to {predictions_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
