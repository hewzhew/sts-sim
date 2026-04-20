#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
import random
from collections import defaultdict
from pathlib import Path
from typing import Any

import numpy as np
import torch
from sklearn.feature_extraction import DictVectorizer
from sklearn.preprocessing import StandardScaler
from torch import nn

from combat_reranker_common import stable_split
from combat_rl_common import REPO_ROOT
from macro_counterfactual_common import (
    group_option_rows,
    iter_jsonl_rows,
    macro_candidate_card_semantics,
    macro_option_only_feature_dict,
    macro_state_feature_dict,
    REWARD_GAP_TARGET_KEYS,
    reward_package_gap_targets,
    top_scoring_macro_mistakes,
    write_json,
    write_jsonl,
)


def load_rows(path: Path) -> list[dict[str, Any]]:
    return iter_jsonl_rows(path) if path.exists() else []


def split_decision_ids(option_rows: list[dict[str, Any]]) -> dict[str, str]:
    decision_ids = sorted({str(row.get("decision_id") or "") for row in option_rows})
    return {decision_id: stable_split(decision_id) for decision_id in decision_ids}


def reward_hardcase_decision_ids(
    option_rows: list[dict[str, Any]],
    positive_sets: dict[str, set[str]],
) -> set[str]:
    grouped = group_option_rows(option_rows)
    hardcases: set[str] = set()
    for decision_id, rows in grouped.items():
        if str(rows[0].get("source_kind") or "") != "reward":
            continue
        positive_ids = positive_sets.get(decision_id, set())
        if not positive_ids:
            continue
        positive_skip = any(
            str(row.get("option_kind") or "") == "reward_skip"
            and str(row.get("option_id") or "") in positive_ids
            for row in rows
        )
        positive_take = any(
            str(row.get("option_kind") or "") == "reward_take_card"
            and str(row.get("option_id") or "") in positive_ids
            for row in rows
        )
        baseline_skip = any(
            bool(row.get("baseline_matches_option")) and str(row.get("option_kind") or "") == "reward_skip"
            for row in rows
        )
        baseline_take = any(
            bool(row.get("baseline_matches_option")) and str(row.get("option_kind") or "") == "reward_take_card"
            for row in rows
        )
        if (positive_take and baseline_skip) or (positive_skip and baseline_take):
            hardcases.add(decision_id)
    return hardcases


def split_decision_ids_with_reward_hardcase(
    option_rows: list[dict[str, Any]],
    positive_sets: dict[str, set[str]],
    *,
    reward_hardcase_holdout: bool,
) -> tuple[dict[str, str], set[str]]:
    split_map = split_decision_ids(option_rows)
    hardcases = reward_hardcase_decision_ids(option_rows, positive_sets)
    if not reward_hardcase_holdout:
        return split_map, hardcases
    for decision_id in hardcases:
        bucket = stable_split(f"reward-hardcase::{decision_id}")
        split_map[decision_id] = "test" if bucket == "test" else "val"
    return split_map, hardcases


def make_option_lookup(option_rows: list[dict[str, Any]]) -> dict[tuple[str, str], dict[str, Any]]:
    return {
        (str(row.get("decision_id") or ""), str(row.get("option_id") or "")): row
        for row in option_rows
    }


def build_pairwise_targets(
    option_rows: list[dict[str, Any]],
    edge_rows: list[dict[str, Any]],
) -> tuple[dict[str, dict[str, float]], dict[str, set[str]]]:
    grouped_options = group_option_rows(option_rows)
    grouped_edges: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in edge_rows:
        grouped_edges[str(row.get("decision_id") or "")].append(row)

    target_scores: dict[str, dict[str, float]] = {}
    positive_sets: dict[str, set[str]] = {}
    for decision_id, rows in grouped_options.items():
        score_map = {str(row.get("option_id") or ""): 0.0 for row in rows}
        for edge in grouped_edges.get(decision_id, []):
            preferred = str(edge.get("preferred_option_id") or "")
            rejected = str(edge.get("rejected_option_id") or "")
            margin = float(edge.get("vote_margin") or edge.get("strength") or 0.0)
            score_map[preferred] = score_map.get(preferred, 0.0) + margin
            score_map[rejected] = score_map.get(rejected, 0.0) - margin
        if not score_map:
            continue
        top_score = max(score_map.values(), default=0.0)
        positive_sets[decision_id] = {
            option_id
            for option_id, score in score_map.items()
            if abs(score - top_score) <= 1e-6
        }
        target_scores[decision_id] = score_map
    return target_scores, positive_sets


def build_source_vocab(option_rows: list[dict[str, Any]]) -> dict[str, int]:
    source_kinds = sorted({str(row.get("source_kind") or "unknown") for row in option_rows})
    return {source_kind: index for index, source_kind in enumerate(source_kinds)}


def build_option_type_vocab(option_rows: list[dict[str, Any]]) -> dict[str, int]:
    option_types = sorted(
        {f"{str(row.get('source_kind') or 'unknown')}::{str(row.get('option_kind') or 'unknown')}" for row in option_rows}
    )
    return {option_type: index for index, option_type in enumerate(option_types)}


def baseline_score(row: dict[str, Any]) -> float:
    return 1.0 if bool(row.get("baseline_matches_option")) else 0.0


def _dense_scaled(
    matrix,
    scaler: StandardScaler,
    *,
    fit: bool,
) -> np.ndarray:
    dense = matrix.toarray() if hasattr(matrix, "toarray") else np.asarray(matrix, dtype=np.float32)
    return (scaler.fit_transform(dense) if fit else scaler.transform(dense)).astype(np.float32)


def encode_option_rows_separate(
    option_rows: list[dict[str, Any]],
    *,
    state_vectorizer: DictVectorizer,
    state_scaler: StandardScaler,
    option_vectorizer: DictVectorizer,
    option_scaler: StandardScaler,
    fit: bool,
) -> tuple[np.ndarray, np.ndarray, list[dict[str, Any]]]:
    grouped = group_option_rows(option_rows)
    ordered_rows: list[dict[str, Any]] = []
    state_fit_rows: list[dict[str, Any]] = []
    for decision_id in sorted(grouped):
        group_rows = grouped[decision_id]
        ordered_rows.extend(group_rows)
        state_fit_rows.append(group_rows[0])

    state_feature_rows = [macro_state_feature_dict(row) for row in ordered_rows]
    option_feature_rows = [macro_option_only_feature_dict(row) for row in ordered_rows]

    if fit:
        fit_state_matrix = state_vectorizer.fit_transform(
            [macro_state_feature_dict(row) for row in state_fit_rows]
        ).astype(np.float32)
        state_scaler.fit(fit_state_matrix.toarray())
        option_matrix = option_vectorizer.fit_transform(option_feature_rows).astype(np.float32)
    else:
        option_matrix = option_vectorizer.transform(option_feature_rows).astype(np.float32)

    state_matrix = state_vectorizer.transform(state_feature_rows).astype(np.float32)
    state_scaled = _dense_scaled(state_matrix, state_scaler, fit=False)
    option_scaled = _dense_scaled(option_matrix, option_scaler, fit=fit)
    return state_scaled, option_scaled, ordered_rows


def build_target_distribution(
    option_ids: list[str],
    score_map: dict[str, float],
    positive_option_ids: set[str],
    *,
    temperature: float,
    margin_mix: float,
) -> np.ndarray:
    if not option_ids:
        return np.zeros(0, dtype=np.float32)
    raw_scores = np.asarray([float(score_map.get(option_id, 0.0)) for option_id in option_ids], dtype=np.float32)
    if np.allclose(raw_scores, raw_scores[0]):
        margin_target = np.full(len(option_ids), 1.0 / len(option_ids), dtype=np.float32)
    else:
        centered = raw_scores - float(raw_scores.max())
        scaled = np.clip(centered / max(float(temperature), 1e-3), -30.0, 30.0)
        exp_scores = np.exp(scaled).astype(np.float32)
        margin_target = exp_scores / np.clip(exp_scores.sum(), 1e-8, None)

    positive_target = np.zeros(len(option_ids), dtype=np.float32)
    for index, option_id in enumerate(option_ids):
        if option_id in positive_option_ids:
            positive_target[index] = 1.0
    if positive_target.sum() > 0:
        positive_target /= positive_target.sum()
    else:
        positive_target = margin_target.copy()

    mixed = ((1.0 - float(margin_mix)) * positive_target) + (float(margin_mix) * margin_target)
    mixed /= np.clip(mixed.sum(), 1e-8, None)
    return mixed.astype(np.float32)


def source_target_params(
    source_kind: str,
    *,
    default_temperature: float,
    default_margin_mix: float,
    reward_temperature: float | None,
    reward_margin_mix: float | None,
    shop_temperature: float | None,
    shop_margin_mix: float | None,
    campfire_temperature: float | None,
    campfire_margin_mix: float | None,
) -> tuple[float, float]:
    if source_kind == "reward":
        return (
            float(reward_temperature if reward_temperature is not None else default_temperature),
            float(reward_margin_mix if reward_margin_mix is not None else default_margin_mix),
        )
    if source_kind == "shop":
        return (
            float(shop_temperature if shop_temperature is not None else default_temperature),
            float(shop_margin_mix if shop_margin_mix is not None else default_margin_mix),
        )
    if source_kind == "campfire":
        return (
            float(campfire_temperature if campfire_temperature is not None else default_temperature),
            float(campfire_margin_mix if campfire_margin_mix is not None else default_margin_mix),
        )
    return float(default_temperature), float(default_margin_mix)


def build_group_examples(
    *,
    option_rows: list[dict[str, Any]],
    state_scaled: np.ndarray,
    option_scaled: np.ndarray,
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
    source_to_index: dict[str, int],
    option_type_to_index: dict[str, int],
    target_temperature: float,
    margin_target_mix: float,
    reward_target_temperature: float | None,
    reward_margin_target_mix: float | None,
    shop_target_temperature: float | None,
    shop_margin_target_mix: float | None,
    campfire_target_temperature: float | None,
    campfire_margin_target_mix: float | None,
) -> tuple[list[dict[str, Any]], dict[str, dict[str, float]]]:
    grouped = group_option_rows(option_rows)
    row_by_key = {
        (str(row.get("decision_id") or ""), str(row.get("option_id") or "")): (index, row)
        for index, row in enumerate(option_rows)
    }
    examples: list[dict[str, Any]] = []
    target_distributions: dict[str, dict[str, float]] = {}
    for decision_id in sorted(grouped):
        rows = grouped[decision_id]
        option_rows_x = []
        option_ids: list[str] = []
        option_type_indices: list[int] = []
        reward_gap_rows: list[list[float]] = []
        reward_gap_mask: list[bool] = []
        reward_skip_mask: list[bool] = []
        reward_take_mask: list[bool] = []
        state_index = row_by_key[(decision_id, str(rows[0].get("option_id") or ""))][0]
        for idx, row in enumerate(rows):
            key = (decision_id, str(row.get("option_id") or ""))
            source_index, _ = row_by_key[key]
            option_rows_x.append(option_scaled[source_index])
            option_id = str(row.get("option_id") or "")
            option_ids.append(option_id)
            option_type_key = f"{str(row.get('source_kind') or 'unknown')}::{str(row.get('option_kind') or 'unknown')}"
            option_type_indices.append(int(option_type_to_index[option_type_key]))
            reward_gap = reward_package_gap_targets(row)
            reward_gap_rows.append([float(reward_gap[key]) for key in REWARD_GAP_TARGET_KEYS])
            reward_gap_mask.append(
                str(row.get("source_kind") or "") == "reward"
                and str(row.get("option_kind") or "") == "reward_take_card"
            )
            reward_skip_mask.append(
                str(row.get("source_kind") or "") == "reward"
                and str(row.get("option_kind") or "") == "reward_skip"
            )
            reward_take_mask.append(
                str(row.get("source_kind") or "") == "reward"
                and str(row.get("option_kind") or "") == "reward_take_card"
            )
        score_values = sorted(
            (float((target_scores.get(decision_id) or {}).get(option_id, 0.0)) for option_id in option_ids),
            reverse=True,
        )
        top_score = score_values[0] if score_values else 0.0
        second_score = next((value for value in score_values if value < top_score - 1e-6), top_score)
        source_kind = str(rows[0].get("source_kind") or "unknown")
        source_temperature, source_mix = source_target_params(
            source_kind,
            default_temperature=float(target_temperature),
            default_margin_mix=float(margin_target_mix),
            reward_temperature=reward_target_temperature,
            reward_margin_mix=reward_margin_target_mix,
            shop_temperature=shop_target_temperature,
            shop_margin_mix=shop_margin_target_mix,
            campfire_temperature=campfire_target_temperature,
            campfire_margin_mix=campfire_margin_target_mix,
        )
        target = build_target_distribution(
            option_ids,
            target_scores.get(decision_id) or {},
            positive_sets.get(decision_id, set()),
            temperature=source_temperature,
            margin_mix=source_mix,
        )
        if target.sum() <= 0:
            continue
        target_distributions[decision_id] = {
            option_id: float(weight) for option_id, weight in zip(option_ids, target, strict=False)
        }
        examples.append(
            {
                "decision_id": decision_id,
                "rows": rows,
                "state_x": state_scaled[state_index].astype(np.float32),
                "option_x": np.stack(option_rows_x, axis=0).astype(np.float32),
                "target": target.astype(np.float32),
                "positive_option_ids": positive_sets.get(decision_id, set()),
                "positive_mask": np.asarray([option_id in positive_sets.get(decision_id, set()) for option_id in option_ids], dtype=bool),
                "reward_gap_target": np.asarray(reward_gap_rows, dtype=np.float32),
                "reward_gap_mask": np.asarray(reward_gap_mask, dtype=bool),
                "reward_skip_mask": np.asarray(reward_skip_mask, dtype=bool),
                "reward_take_mask": np.asarray(reward_take_mask, dtype=bool),
                "decision_weight": 1.0 + max(top_score - second_score, 0.0) * 0.15,
                "source_index": int(source_to_index[source_kind]),
                "option_type_index": np.asarray(option_type_indices, dtype=np.int64),
                "source_kind": source_kind,
                "is_reward_hardcase": False,
            }
        )
    return examples, target_distributions


class MLPEncoder(nn.Module):
    def __init__(self, in_features: int, hidden: int, dropout: float) -> None:
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, hidden),
            nn.LayerNorm(hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(hidden, hidden),
            nn.LayerNorm(hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
        )

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.net(x)


class MacroChoiceTowerNet(nn.Module):
    def __init__(
        self,
        *,
        state_in_features: int,
        option_in_features: int,
        source_count: int,
        option_type_count: int,
        reward_source_index: int | None,
        state_hidden: int = 160,
        option_hidden: int = 160,
        fusion_hidden: int = 192,
        source_embed_dim: int = 16,
        option_type_embed_dim: int = 12,
        dropout: float = 0.12,
    ) -> None:
        super().__init__()
        self.reward_source_index = reward_source_index
        shared_hidden = max(state_hidden, option_hidden)
        self.shared_hidden = shared_hidden
        self.fusion_hidden = fusion_hidden
        self.state_encoder = MLPEncoder(state_in_features, state_hidden, dropout)
        self.option_encoder = MLPEncoder(option_in_features, option_hidden, dropout)
        self.state_project = nn.Sequential(
            nn.Linear(state_hidden, shared_hidden),
            nn.LayerNorm(shared_hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
        )
        self.option_project = nn.Sequential(
            nn.Linear(option_hidden, shared_hidden),
            nn.LayerNorm(shared_hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
        )
        self.source_embedding = nn.Embedding(source_count, source_embed_dim)
        self.option_type_embedding = nn.Embedding(option_type_count, option_type_embed_dim)
        self.source_option_adapters = nn.ModuleList(
            [
                nn.Sequential(
                    nn.Linear(shared_hidden, shared_hidden),
                    nn.LayerNorm(shared_hidden),
                    nn.ReLU(),
                    nn.Dropout(dropout),
                )
                for _ in range(source_count)
            ]
        )
        self.fusion = nn.Sequential(
            nn.Linear((shared_hidden * 3) + source_embed_dim + option_type_embed_dim, fusion_hidden),
            nn.LayerNorm(fusion_hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(fusion_hidden, fusion_hidden),
            nn.LayerNorm(fusion_hidden),
            nn.ReLU(),
            nn.Dropout(dropout),
        )
        self.source_fusions = nn.ModuleList(
            [
                nn.Sequential(
                    nn.Linear(fusion_hidden, fusion_hidden),
                    nn.LayerNorm(fusion_hidden),
                    nn.ReLU(),
                    nn.Dropout(dropout),
                )
                for _ in range(source_count)
            ]
        )
        self.shared_head = nn.Linear(fusion_hidden, 1)
        self.source_heads = nn.ModuleList(
            [
                nn.Sequential(
                    nn.Linear(fusion_hidden, max(fusion_hidden // 2, 32)),
                    nn.ReLU(),
                    nn.Dropout(dropout),
                    nn.Linear(max(fusion_hidden // 2, 32), 1),
                )
                for _ in range(source_count)
            ]
        )
        self.reward_gap_head = nn.Sequential(
            nn.Linear(fusion_hidden, max(fusion_hidden // 2, 32)),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(max(fusion_hidden // 2, 32), len(REWARD_GAP_TARGET_KEYS)),
        )
        self.reward_gate_head = nn.Sequential(
            nn.Linear(shared_hidden + (fusion_hidden * 2), max(fusion_hidden // 2, 32)),
            nn.ReLU(),
            nn.Dropout(dropout),
            nn.Linear(max(fusion_hidden // 2, 32), 1),
        )

    def forward_group(
        self,
        state_x: torch.Tensor,
        option_x: torch.Tensor,
        source_index: int,
        option_type_index: torch.Tensor,
        reward_skip_mask: torch.Tensor | None = None,
        reward_take_mask: torch.Tensor | None = None,
        *,
        return_aux: bool = False,
    ) -> tuple[torch.Tensor, torch.Tensor | None, torch.Tensor | None] | torch.Tensor:
        if state_x.dim() == 1:
            state_x = state_x.unsqueeze(0)
        state_hidden = self.state_project(self.state_encoder(state_x))
        option_hidden = self.option_project(self.option_encoder(option_x))
        option_hidden = option_hidden + self.source_option_adapters[int(source_index)](option_hidden)
        if state_hidden.shape[0] == 1:
            state_hidden = state_hidden.expand(option_hidden.shape[0], -1)
        source_idx = torch.full(
            (option_hidden.shape[0],),
            int(source_index),
            dtype=torch.long,
            device=option_hidden.device,
        )
        source_hidden = self.source_embedding(source_idx)
        option_type_hidden = self.option_type_embedding(option_type_index)
        fused_input = torch.cat(
            [state_hidden, option_hidden, state_hidden * option_hidden, source_hidden, option_type_hidden],
            dim=-1,
        )
        fused = self.fusion(fused_input)
        source_fused = self.source_fusions[int(source_index)](fused)
        shared_logits = self.shared_head(fused).squeeze(-1)
        source_logits = self.source_heads[int(source_index)](source_fused).squeeze(-1)
        logits = shared_logits + source_logits
        if not return_aux:
            return logits
        reward_gap_logits = None
        reward_gate_logit = None
        if self.reward_source_index is not None and int(source_index) == int(self.reward_source_index):
            reward_gap_logits = self.reward_gap_head(source_fused)
            has_skip = reward_skip_mask is not None and bool(reward_skip_mask.any().item())
            has_take = reward_take_mask is not None and bool(reward_take_mask.any().item())
            if has_skip and has_take:
                gate_input = torch.cat(
                    [
                        state_hidden[0],
                        source_fused[reward_take_mask].mean(dim=0),
                        source_fused[reward_skip_mask].mean(dim=0),
                    ],
                    dim=-1,
                )
                reward_gate_logit = self.reward_gate_head(gate_input).squeeze(-1)
        return logits, reward_gap_logits, reward_gate_logit


def choice_loss_for_group(
    logits: torch.Tensor,
    target: torch.Tensor,
    positive_mask: torch.Tensor,
    *,
    pairwise_aux_weight: float,
    reward_gate_logit: torch.Tensor | None = None,
    reward_skip_mask: torch.Tensor | None = None,
    reward_take_mask: torch.Tensor | None = None,
    reward_gate_weight: float = 0.0,
    reward_take_choice_weight: float = 0.0,
) -> tuple[torch.Tensor, dict[str, float]]:
    use_reward_hierarchy = (
        reward_gate_logit is not None
        and reward_skip_mask is not None
        and reward_take_mask is not None
        and bool(reward_skip_mask.any().item())
        and bool(reward_take_mask.any().item())
    )
    if use_reward_hierarchy:
        log_probs, _probs, _gate_prob = reward_hierarchical_distribution(
            logits,
            reward_gate_logit,
            reward_skip_mask,
            reward_take_mask,
        )
        score_tensor = log_probs
    else:
        log_probs = torch.log_softmax(logits, dim=0)
        score_tensor = logits
    choice_loss = -(target * log_probs).sum()
    pairwise_loss = torch.tensor(0.0, device=logits.device)
    reward_gate_loss = torch.tensor(0.0, device=logits.device)
    reward_take_choice_loss = torch.tensor(0.0, device=logits.device)
    negative_mask = ~positive_mask
    if pairwise_aux_weight > 0 and positive_mask.any() and negative_mask.any():
        pos_logits = score_tensor[positive_mask]
        neg_logits = score_tensor[negative_mask]
        pairwise_loss = torch.nn.functional.softplus(
            -(pos_logits.unsqueeze(1) - neg_logits.unsqueeze(0))
        ).mean()
    if use_reward_hierarchy:
        take_target_mass = torch.clamp(target[reward_take_mask].sum(), min=0.0, max=1.0)
        reward_gate_loss = torch.nn.functional.binary_cross_entropy_with_logits(
            reward_gate_logit.reshape(1),
            take_target_mass.reshape(1),
        )
        take_target = target[reward_take_mask]
        take_target_sum = take_target.sum()
        if float(take_target_sum.item()) > 1e-8:
            take_target = take_target / take_target_sum
            take_log_probs = torch.log_softmax(logits[reward_take_mask], dim=0)
            reward_take_choice_loss = -(take_target * take_log_probs).sum()
    total = (
        choice_loss
        + pairwise_aux_weight * pairwise_loss
        + reward_gate_weight * reward_gate_loss
        + reward_take_choice_weight * reward_take_choice_loss
    )
    return total, {
        "choice_loss": float(choice_loss.item()),
        "pairwise_loss": float(pairwise_loss.item()) if pairwise_aux_weight > 0 else 0.0,
        "reward_gate_loss": float(reward_gate_loss.item()) if use_reward_hierarchy else 0.0,
        "reward_take_choice_loss": float(reward_take_choice_loss.item()) if use_reward_hierarchy else 0.0,
    }


def reward_aux_loss_for_group(
    reward_gap_logits: torch.Tensor | None,
    reward_gap_target: torch.Tensor,
    reward_gap_mask: torch.Tensor,
) -> tuple[torch.Tensor, float]:
    if reward_gap_logits is None or not reward_gap_mask.any():
        zero = reward_gap_target.sum() * 0.0
        return zero, 0.0
    masked_logits = reward_gap_logits[reward_gap_mask]
    masked_target = reward_gap_target[reward_gap_mask]
    loss = torch.nn.functional.binary_cross_entropy_with_logits(masked_logits, masked_target)
    return loss, float(loss.item())


def reward_skip_margin_loss_for_group(
    logits: torch.Tensor,
    positive_mask: torch.Tensor,
    reward_skip_mask: torch.Tensor,
    reward_take_mask: torch.Tensor,
    *,
    margin: float,
) -> tuple[torch.Tensor, float]:
    if not reward_skip_mask.any() or not reward_take_mask.any():
        zero = logits.sum() * 0.0
        return zero, 0.0

    positive_take_mask = reward_take_mask & positive_mask
    positive_skip_mask = reward_skip_mask & positive_mask
    if positive_take_mask.any() and not positive_skip_mask.any():
        lhs = logits[positive_take_mask]
        rhs = logits[reward_skip_mask]
    elif positive_skip_mask.any() and not positive_take_mask.any():
        lhs = logits[positive_skip_mask]
        rhs = logits[reward_take_mask]
    else:
        zero = logits.sum() * 0.0
        return zero, 0.0

    loss = torch.nn.functional.softplus(float(margin) - (lhs.unsqueeze(1) - rhs.unsqueeze(0))).mean()
    return loss, float(loss.item())


def reward_hierarchical_distribution(
    logits: torch.Tensor,
    reward_gate_logit: torch.Tensor | None,
    reward_skip_mask: torch.Tensor,
    reward_take_mask: torch.Tensor,
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor | None]:
    if reward_gate_logit is None or not bool(reward_skip_mask.any().item()) or not bool(reward_take_mask.any().item()):
        log_probs = torch.log_softmax(logits, dim=0)
        probs = torch.softmax(logits, dim=0)
        return log_probs, probs, None

    log_probs = torch.full_like(logits, -1e9)
    take_log_prob = -torch.nn.functional.softplus(-reward_gate_logit)
    skip_log_prob = -torch.nn.functional.softplus(reward_gate_logit)
    log_probs[reward_take_mask] = take_log_prob + torch.log_softmax(logits[reward_take_mask], dim=0)
    log_probs[reward_skip_mask] = skip_log_prob + torch.log_softmax(logits[reward_skip_mask], dim=0)
    probs = torch.exp(log_probs)
    probs = probs / torch.clamp(probs.sum(), min=1e-8)
    return log_probs, probs, torch.sigmoid(reward_gate_logit)


def validate_groups(
    model: MacroChoiceTowerNet,
    examples: list[dict[str, Any]],
    device: torch.device,
    pairwise_aux_weight: float,
    reward_aux_weight: float,
    reward_skip_margin_weight: float,
    reward_skip_margin: float,
    reward_gate_weight: float,
    reward_take_choice_weight: float,
) -> dict[str, float]:
    if not examples:
        return {
            "mean_total_loss": 0.0,
            "mean_choice_loss": 0.0,
            "mean_pairwise_loss": 0.0,
            "mean_reward_aux_loss": 0.0,
            "mean_reward_skip_margin_loss": 0.0,
            "mean_reward_gate_loss": 0.0,
            "mean_reward_take_choice_loss": 0.0,
        }
    model.eval()
    total_loss = 0.0
    choice_loss = 0.0
    pair_loss = 0.0
    reward_aux = 0.0
    reward_skip_margin_loss_total = 0.0
    reward_gate_loss_total = 0.0
    reward_take_choice_loss_total = 0.0
    with torch.no_grad():
        for example in examples:
            state_x = torch.as_tensor(example["state_x"], dtype=torch.float32, device=device)
            option_x = torch.as_tensor(example["option_x"], dtype=torch.float32, device=device)
            option_type_index = torch.as_tensor(example["option_type_index"], dtype=torch.long, device=device)
            target = torch.as_tensor(example["target"], dtype=torch.float32, device=device)
            positive_mask = torch.as_tensor(example["positive_mask"], dtype=torch.bool, device=device)
            reward_gap_target = torch.as_tensor(example["reward_gap_target"], dtype=torch.float32, device=device)
            reward_gap_mask = torch.as_tensor(example["reward_gap_mask"], dtype=torch.bool, device=device)
            reward_skip_mask = torch.as_tensor(example["reward_skip_mask"], dtype=torch.bool, device=device)
            reward_take_mask = torch.as_tensor(example["reward_take_mask"], dtype=torch.bool, device=device)
            logits, reward_gap_logits, reward_gate_logit = model.forward_group(
                state_x,
                option_x,
                int(example["source_index"]),
                option_type_index,
                reward_skip_mask=reward_skip_mask,
                reward_take_mask=reward_take_mask,
                return_aux=True,
            )
            choice_total, parts = choice_loss_for_group(
                logits,
                target,
                positive_mask,
                pairwise_aux_weight=pairwise_aux_weight,
                reward_gate_logit=reward_gate_logit,
                reward_skip_mask=reward_skip_mask,
                reward_take_mask=reward_take_mask,
                reward_gate_weight=reward_gate_weight,
                reward_take_choice_weight=reward_take_choice_weight,
            )
            reward_aux_loss, reward_aux_part = reward_aux_loss_for_group(
                reward_gap_logits,
                reward_gap_target,
                reward_gap_mask,
            )
            reward_skip_margin_loss, reward_skip_margin_part = reward_skip_margin_loss_for_group(
                reward_hierarchical_distribution(logits, reward_gate_logit, reward_skip_mask, reward_take_mask)[0],
                positive_mask,
                reward_skip_mask,
                reward_take_mask,
                margin=reward_skip_margin,
            )
            loss = (
                choice_total
                + (float(reward_aux_weight) * reward_aux_loss)
                + (float(reward_skip_margin_weight) * reward_skip_margin_loss)
            )
            total_loss += float(loss.item())
            choice_loss += parts["choice_loss"]
            pair_loss += parts["pairwise_loss"]
            reward_aux += reward_aux_part
            reward_skip_margin_loss_total += reward_skip_margin_part
            reward_gate_loss_total += parts.get("reward_gate_loss", 0.0)
            reward_take_choice_loss_total += parts.get("reward_take_choice_loss", 0.0)
    count = float(len(examples))
    return {
        "mean_total_loss": round(total_loss / count, 6),
        "mean_choice_loss": round(choice_loss / count, 6),
        "mean_pairwise_loss": round(pair_loss / count, 6),
        "mean_reward_aux_loss": round(reward_aux / count, 6),
        "mean_reward_skip_margin_loss": round(reward_skip_margin_loss_total / count, 6),
        "mean_reward_gate_loss": round(reward_gate_loss_total / count, 6),
        "mean_reward_take_choice_loss": round(reward_take_choice_loss_total / count, 6),
    }


def build_inference_functions(
    *,
    model: MacroChoiceTowerNet,
    grouped_rows: dict[str, list[dict[str, Any]]],
    state_vectorizer: DictVectorizer,
    state_scaler: StandardScaler,
    option_vectorizer: DictVectorizer,
    option_scaler: StandardScaler,
    source_to_index: dict[str, int],
    option_type_to_index: dict[str, int],
    device: torch.device,
):
    cache: dict[str, dict[str, dict[str, float]]] = {}

    def infer_decision(decision_id: str) -> dict[str, dict[str, float]]:
        cached = cache.get(decision_id)
        if cached is not None:
            return cached
        rows = grouped_rows[decision_id]
        state_dense = _dense_scaled(
            state_vectorizer.transform([macro_state_feature_dict(rows[0])]).astype(np.float32),
            state_scaler,
            fit=False,
        )
        option_dense = _dense_scaled(
            option_vectorizer.transform([macro_option_only_feature_dict(row) for row in rows]).astype(np.float32),
            option_scaler,
            fit=False,
        )
        state_x = torch.as_tensor(state_dense[0], dtype=torch.float32, device=device)
        option_x = torch.as_tensor(option_dense, dtype=torch.float32, device=device)
        source_kind = str(rows[0].get("source_kind") or "unknown")
        option_type_index = torch.as_tensor(
            [
                option_type_to_index[f"{str(row.get('source_kind') or 'unknown')}::{str(row.get('option_kind') or 'unknown')}"]
                for row in rows
            ],
            dtype=torch.long,
            device=device,
        )
        reward_skip_mask = torch.as_tensor(
            [str(row.get("option_kind") or "") == "reward_skip" for row in rows],
            dtype=torch.bool,
            device=device,
        )
        reward_take_mask = torch.as_tensor(
            [str(row.get("option_kind") or "") == "reward_take_card" for row in rows],
            dtype=torch.bool,
            device=device,
        )
        with torch.no_grad():
            logits, _reward_gap_logits, reward_gate_logit = model.forward_group(
                state_x,
                option_x,
                int(source_to_index[source_kind]),
                option_type_index,
                reward_skip_mask=reward_skip_mask,
                reward_take_mask=reward_take_mask,
                return_aux=True,
            )
            log_probs, probabilities_t, _reward_gate_prob = reward_hierarchical_distribution(
                logits,
                reward_gate_logit,
                reward_skip_mask,
                reward_take_mask,
            )
            probabilities = probabilities_t.cpu().numpy()
            logit_values = log_probs.cpu().numpy()
        result = {
            str(row.get("option_id") or ""): {
                "logit": float(logit),
                "probability": float(prob),
            }
            for row, logit, prob in zip(rows, logit_values, probabilities, strict=False)
        }
        cache[decision_id] = result
        return result

    def score_fn(row: dict[str, Any]) -> float:
        decision_id = str(row.get("decision_id") or "")
        option_id = str(row.get("option_id") or "")
        return float(infer_decision(decision_id)[option_id]["logit"])

    def probability_fn(rows: list[dict[str, Any]]) -> dict[str, float]:
        decision_id = str(rows[0].get("decision_id") or "")
        return {
            option_id: float(meta["probability"])
            for option_id, meta in infer_decision(decision_id).items()
        }

    return score_fn, probability_fn


def evaluate_pairwise(
    edge_rows: list[dict[str, Any]],
    option_lookup: dict[tuple[str, str], dict[str, Any]],
    score_fn,
) -> dict[str, Any]:
    total = 0
    correct = 0
    weighted_total = 0.0
    weighted_correct = 0.0
    for edge in edge_rows:
        decision_id = str(edge.get("decision_id") or "")
        preferred_id = str(edge.get("preferred_option_id") or "")
        rejected_id = str(edge.get("rejected_option_id") or "")
        preferred = option_lookup.get((decision_id, preferred_id))
        rejected = option_lookup.get((decision_id, rejected_id))
        if preferred is None or rejected is None:
            continue
        pref_score = float(score_fn(preferred))
        rej_score = float(score_fn(rejected))
        weight = float(edge.get("strength") or 1.0)
        total += 1
        weighted_total += weight
        if pref_score > rej_score:
            correct += 1
            weighted_correct += weight
    return {
        "pairwise_total": total,
        "pairwise_correct": correct,
        "pairwise_agreement": round(correct / float(max(total, 1)), 6),
        "weighted_pairwise_agreement": round(weighted_correct / float(max(weighted_total, 1.0)), 6),
    }


def evaluate_decisions(
    *,
    option_rows: list[dict[str, Any]],
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
    target_distributions: dict[str, dict[str, float]],
    score_fn,
    probability_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    grouped = group_option_rows(option_rows)
    predictions: list[dict[str, Any]] = []
    decision_count = 0
    top1_match = 0
    baseline_top1_match = 0
    choice_nll_total = 0.0
    positive_mass_total = 0.0
    source_correct: dict[str, int] = defaultdict(int)
    source_total: dict[str, int] = defaultdict(int)
    source_baseline_correct: dict[str, int] = defaultdict(int)

    for decision_id, rows in grouped.items():
        decision_count += 1
        source_kind = str(rows[0].get("source_kind") or "unknown")
        source_total[source_kind] += 1
        probabilities = probability_fn(rows)
        scored_rows = sorted(
            (
                {
                    "option_id": str(row.get("option_id") or ""),
                    "label": str(row.get("label") or ""),
                    "option_kind": str(row.get("option_kind") or ""),
                    "score": round(float(score_fn(row)), 6),
                    "probability": round(float(probabilities[str(row.get("option_id") or "")]), 6),
                    "target_score": round(
                        float((target_scores.get(decision_id) or {}).get(str(row.get("option_id") or ""), 0.0)),
                        6,
                    ),
                    "is_positive": str(row.get("option_id") or "") in positive_sets.get(decision_id, set()),
                    "is_baseline": bool(row.get("baseline_matches_option")),
                }
                for row in rows
            ),
            key=lambda item: item["score"],
            reverse=True,
        )
        predicted = scored_rows[0]
        top1_hit = bool(predicted["is_positive"])
        top1_match += int(top1_hit)
        source_correct[source_kind] += int(top1_hit)

        baseline = next((item for item in scored_rows if item["is_baseline"]), None)
        baseline_hit = bool(baseline and baseline["is_positive"])
        baseline_top1_match += int(baseline_hit)
        source_baseline_correct[source_kind] += int(baseline_hit)

        positive_ids = positive_sets.get(decision_id, set())
        target_distribution = target_distributions.get(decision_id) or {}
        positive_mass = sum(item["probability"] for item in scored_rows if item["option_id"] in positive_ids)
        positive_mass_total += positive_mass
        choice_nll = 0.0
        for item in scored_rows:
            target_prob = float(target_distribution.get(item["option_id"], 0.0))
            if target_prob > 0:
                choice_nll += -target_prob * np.log(max(item["probability"], 1e-8))
        choice_nll_total += choice_nll

        predictions.append(
            {
                "decision_id": decision_id,
                "run_id": rows[0].get("run_id"),
                "source_kind": source_kind,
                "screen_type": rows[0].get("screen_type"),
                "baseline_choice_kind": rows[0].get("baseline_choice_kind"),
                "predicted_option_id": predicted["option_id"],
                "predicted_label": predicted["label"],
                "predicted_kind": predicted["option_kind"],
                "baseline_option_id": baseline["option_id"] if baseline else None,
                "baseline_label": baseline["label"] if baseline else None,
                "positive_option_ids": sorted(positive_ids),
                "top1_match": top1_hit,
                "baseline_top1_match": baseline_hit,
                "positive_mass": round(float(positive_mass), 6),
                "choice_nll": round(float(choice_nll), 6),
                "scores": scored_rows,
            }
        )

    source_breakdown = {}
    for source_kind in sorted(source_total):
        total = source_total[source_kind]
        model_correct = source_correct[source_kind]
        baseline_correct = source_baseline_correct[source_kind]
        source_breakdown[source_kind] = {
            "decision_count": total,
            "top1_match_rate": round(model_correct / float(max(total, 1)), 6),
            "baseline_top1_match_rate": round(baseline_correct / float(max(total, 1)), 6),
            "top1_improvement": round((model_correct - baseline_correct) / float(max(total, 1)), 6),
        }

    metrics = {
        "decision_count": decision_count,
        "top1_match": top1_match,
        "top1_match_rate": round(top1_match / float(max(decision_count, 1)), 6),
        "baseline_top1_match": baseline_top1_match,
        "baseline_top1_match_rate": round(baseline_top1_match / float(max(decision_count, 1)), 6),
        "top1_improvement": round((top1_match - baseline_top1_match) / float(max(decision_count, 1)), 6),
        "mean_choice_nll": round(choice_nll_total / float(max(decision_count, 1)), 6),
        "mean_positive_mass": round(positive_mass_total / float(max(decision_count, 1)), 6),
        "source_breakdown": source_breakdown,
    }
    return metrics, predictions


def evaluate_split(
    *,
    name: str,
    option_rows: list[dict[str, Any]],
    edge_rows: list[dict[str, Any]],
    option_lookup: dict[tuple[str, str], dict[str, Any]],
    target_scores: dict[str, dict[str, float]],
    positive_sets: dict[str, set[str]],
    target_distributions: dict[str, dict[str, float]],
    score_fn,
    probability_fn,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    pairwise = evaluate_pairwise(edge_rows, option_lookup, score_fn)
    decision_metrics, predictions = evaluate_decisions(
        option_rows=option_rows,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=target_distributions,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    return {
        "split": name,
        "option_rows": len(option_rows),
        "edge_rows": len(edge_rows),
        **pairwise,
        **decision_metrics,
    }, predictions


def filter_rows_by_decision_ids(rows: list[dict[str, Any]], decision_ids: set[str]) -> list[dict[str, Any]]:
    return [row for row in rows if str(row.get("decision_id") or "") in decision_ids]


def main() -> int:
    parser = argparse.ArgumentParser(description="Train a two-tower grouped macro counterfactual choice model.")
    parser.add_argument("--dataset-dir", default=REPO_ROOT / "tools" / "artifacts" / "learning_dataset", type=Path)
    parser.add_argument("--dataset-prefix", default="macro_counterfactual_pilot")
    parser.add_argument("--output-prefix", default="macro_counterfactual_choice_tower_torch")
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--predictions-out", default=None, type=Path)
    parser.add_argument("--review-out", default=None, type=Path)
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--epochs", default=220, type=int)
    parser.add_argument("--patience", default=30, type=int)
    parser.add_argument("--state-hidden", default=160, type=int)
    parser.add_argument("--option-hidden", default=160, type=int)
    parser.add_argument("--fusion-hidden", default=192, type=int)
    parser.add_argument("--source-embed-dim", default=16, type=int)
    parser.add_argument("--option-type-embed-dim", default=12, type=int)
    parser.add_argument("--dropout", default=0.12, type=float)
    parser.add_argument("--lr", default=6e-4, type=float)
    parser.add_argument("--weight-decay", default=2e-4, type=float)
    parser.add_argument("--pairwise-aux-weight", default=0.2, type=float)
    parser.add_argument("--reward-aux-weight", default=0.2, type=float)
    parser.add_argument("--reward-skip-margin-weight", default=0.0, type=float)
    parser.add_argument("--reward-skip-margin", default=0.2, type=float)
    parser.add_argument("--reward-gate-weight", default=0.35, type=float)
    parser.add_argument("--reward-take-choice-weight", default=0.2, type=float)
    parser.add_argument("--reward-hardcase-holdout", action="store_true")
    parser.add_argument("--reward-hardcase-train-multiplier", default=1.0, type=float)
    parser.add_argument("--target-temperature", default=0.75, type=float)
    parser.add_argument("--margin-target-mix", default=0.65, type=float)
    parser.add_argument("--reward-target-temperature", default=1.0, type=float)
    parser.add_argument("--reward-margin-target-mix", default=0.25, type=float)
    parser.add_argument("--shop-target-temperature", default=0.85, type=float)
    parser.add_argument("--shop-margin-target-mix", default=0.45, type=float)
    parser.add_argument("--campfire-target-temperature", default=0.65, type=float)
    parser.add_argument("--campfire-margin-target-mix", default=0.2, type=float)
    parser.add_argument("--device", default="cpu")
    args = parser.parse_args()

    options_path = args.dataset_dir / f"{args.dataset_prefix}_options.jsonl"
    pairwise_path = args.dataset_dir / f"{args.dataset_prefix}_pairwise.jsonl"
    option_rows = load_rows(options_path)
    edge_rows = load_rows(pairwise_path)
    if not option_rows or not edge_rows:
        raise SystemExit(f"missing macro counterfactual dataset rows for prefix '{args.dataset_prefix}'")

    option_lookup = make_option_lookup(option_rows)
    target_scores, positive_sets = build_pairwise_targets(option_rows, edge_rows)
    split_map, reward_hardcases = split_decision_ids_with_reward_hardcase(
        option_rows,
        positive_sets,
        reward_hardcase_holdout=bool(args.reward_hardcase_holdout),
    )
    source_to_index = build_source_vocab(option_rows)
    option_type_to_index = build_option_type_vocab(option_rows)

    train_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_option_rows = [row for row in option_rows if split_map[str(row.get("decision_id") or "")] == "test"]
    train_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "train"]
    val_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "val"]
    test_edge_rows = [row for row in edge_rows if split_map[str(row.get("decision_id") or "")] == "test"]

    state_vectorizer = DictVectorizer(sparse=True)
    state_scaler = StandardScaler()
    option_vectorizer = DictVectorizer(sparse=True)
    option_scaler = StandardScaler()
    train_state_scaled, train_option_scaled, train_rows_ordered = encode_option_rows_separate(
        train_option_rows,
        state_vectorizer=state_vectorizer,
        state_scaler=state_scaler,
        option_vectorizer=option_vectorizer,
        option_scaler=option_scaler,
        fit=True,
    )
    val_state_scaled, val_option_scaled, val_rows_ordered = encode_option_rows_separate(
        val_option_rows,
        state_vectorizer=state_vectorizer,
        state_scaler=state_scaler,
        option_vectorizer=option_vectorizer,
        option_scaler=option_scaler,
        fit=False,
    )
    test_state_scaled, test_option_scaled, test_rows_ordered = encode_option_rows_separate(
        test_option_rows,
        state_vectorizer=state_vectorizer,
        state_scaler=state_scaler,
        option_vectorizer=option_vectorizer,
        option_scaler=option_scaler,
        fit=False,
    )

    train_examples, train_target_distributions = build_group_examples(
        option_rows=train_rows_ordered,
        state_scaled=train_state_scaled,
        option_scaled=train_option_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
        source_to_index=source_to_index,
        option_type_to_index=option_type_to_index,
        target_temperature=float(args.target_temperature),
        margin_target_mix=float(args.margin_target_mix),
        reward_target_temperature=float(args.reward_target_temperature),
        reward_margin_target_mix=float(args.reward_margin_target_mix),
        shop_target_temperature=float(args.shop_target_temperature),
        shop_margin_target_mix=float(args.shop_margin_target_mix),
        campfire_target_temperature=float(args.campfire_target_temperature),
        campfire_margin_target_mix=float(args.campfire_margin_target_mix),
    )
    val_examples, val_target_distributions = build_group_examples(
        option_rows=val_rows_ordered,
        state_scaled=val_state_scaled,
        option_scaled=val_option_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
        source_to_index=source_to_index,
        option_type_to_index=option_type_to_index,
        target_temperature=float(args.target_temperature),
        margin_target_mix=float(args.margin_target_mix),
        reward_target_temperature=float(args.reward_target_temperature),
        reward_margin_target_mix=float(args.reward_margin_target_mix),
        shop_target_temperature=float(args.shop_target_temperature),
        shop_margin_target_mix=float(args.shop_margin_target_mix),
        campfire_target_temperature=float(args.campfire_target_temperature),
        campfire_margin_target_mix=float(args.campfire_margin_target_mix),
    )
    test_examples, test_target_distributions = build_group_examples(
        option_rows=test_rows_ordered,
        state_scaled=test_state_scaled,
        option_scaled=test_option_scaled,
        target_scores=target_scores,
        positive_sets=positive_sets,
        source_to_index=source_to_index,
        option_type_to_index=option_type_to_index,
        target_temperature=float(args.target_temperature),
        margin_target_mix=float(args.margin_target_mix),
        reward_target_temperature=float(args.reward_target_temperature),
        reward_margin_target_mix=float(args.reward_margin_target_mix),
        shop_target_temperature=float(args.shop_target_temperature),
        shop_margin_target_mix=float(args.shop_margin_target_mix),
        campfire_target_temperature=float(args.campfire_target_temperature),
        campfire_margin_target_mix=float(args.campfire_margin_target_mix),
    )

    all_target_distributions = {}
    all_target_distributions.update(train_target_distributions)
    all_target_distributions.update(val_target_distributions)
    all_target_distributions.update(test_target_distributions)

    reward_hardcase_train_ids = {
        decision_id
        for decision_id in reward_hardcases
        if split_map.get(decision_id) == "train"
    }
    reward_hardcase_val_ids = {
        decision_id
        for decision_id in reward_hardcases
        if split_map.get(decision_id) == "val"
    }
    reward_hardcase_test_ids = {
        decision_id
        for decision_id in reward_hardcases
        if split_map.get(decision_id) == "test"
    }
    reward_hardcase_multiplier = float(args.reward_hardcase_train_multiplier)
    for example in train_examples:
        if example["decision_id"] in reward_hardcase_train_ids:
            example["is_reward_hardcase"] = True
            example["decision_weight"] *= reward_hardcase_multiplier
    for examples in (val_examples, test_examples):
        for example in examples:
            if example["decision_id"] in reward_hardcases:
                example["is_reward_hardcase"] = True

    device = torch.device(args.device)
    model = MacroChoiceTowerNet(
        state_in_features=int(train_state_scaled.shape[1]),
        option_in_features=int(train_option_scaled.shape[1]),
        source_count=len(source_to_index),
        option_type_count=len(option_type_to_index),
        reward_source_index=source_to_index.get("reward"),
        state_hidden=int(args.state_hidden),
        option_hidden=int(args.option_hidden),
        fusion_hidden=int(args.fusion_hidden),
        source_embed_dim=int(args.source_embed_dim),
        option_type_embed_dim=int(args.option_type_embed_dim),
        dropout=float(args.dropout),
    ).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=float(args.lr), weight_decay=float(args.weight_decay))

    best_state = copy.deepcopy(model.state_dict())
    best_val = float("inf")
    best_epoch = 0
    epochs_without_improvement = 0
    train_rng = random.Random(0)

    for epoch in range(int(args.epochs)):
        model.train()
        shuffled = train_examples.copy()
        train_rng.shuffle(shuffled)
        for example in shuffled:
            state_x = torch.as_tensor(example["state_x"], dtype=torch.float32, device=device)
            option_x = torch.as_tensor(example["option_x"], dtype=torch.float32, device=device)
            option_type_index = torch.as_tensor(example["option_type_index"], dtype=torch.long, device=device)
            target = torch.as_tensor(example["target"], dtype=torch.float32, device=device)
            positive_mask = torch.as_tensor(example["positive_mask"], dtype=torch.bool, device=device)
            reward_gap_target = torch.as_tensor(example["reward_gap_target"], dtype=torch.float32, device=device)
            reward_gap_mask = torch.as_tensor(example["reward_gap_mask"], dtype=torch.bool, device=device)
            reward_skip_mask = torch.as_tensor(example["reward_skip_mask"], dtype=torch.bool, device=device)
            reward_take_mask = torch.as_tensor(example["reward_take_mask"], dtype=torch.bool, device=device)
            logits, reward_gap_logits, reward_gate_logit = model.forward_group(
                state_x,
                option_x,
                int(example["source_index"]),
                option_type_index,
                reward_skip_mask=reward_skip_mask,
                reward_take_mask=reward_take_mask,
                return_aux=True,
            )
            choice_loss, _parts = choice_loss_for_group(
                logits,
                target,
                positive_mask,
                pairwise_aux_weight=float(args.pairwise_aux_weight),
                reward_gate_logit=reward_gate_logit,
                reward_skip_mask=reward_skip_mask,
                reward_take_mask=reward_take_mask,
                reward_gate_weight=float(args.reward_gate_weight),
                reward_take_choice_weight=float(args.reward_take_choice_weight),
            )
            reward_aux_loss, _reward_aux_part = reward_aux_loss_for_group(
                reward_gap_logits,
                reward_gap_target,
                reward_gap_mask,
            )
            reward_skip_margin_loss, _reward_skip_margin_part = reward_skip_margin_loss_for_group(
                reward_hierarchical_distribution(logits, reward_gate_logit, reward_skip_mask, reward_take_mask)[0],
                positive_mask,
                reward_skip_mask,
                reward_take_mask,
                margin=float(args.reward_skip_margin),
            )
            loss = (
                choice_loss
                + (float(args.reward_aux_weight) * reward_aux_loss)
                + (float(args.reward_skip_margin_weight) * reward_skip_margin_loss)
            )
            weighted_loss = loss * float(example.get("decision_weight") or 1.0)
            optimizer.zero_grad()
            weighted_loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()

        val_loss = validate_groups(
            model,
            val_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
            reward_aux_weight=float(args.reward_aux_weight),
            reward_skip_margin_weight=float(args.reward_skip_margin_weight),
            reward_skip_margin=float(args.reward_skip_margin),
            reward_gate_weight=float(args.reward_gate_weight),
            reward_take_choice_weight=float(args.reward_take_choice_weight),
        )["mean_total_loss"]
        if val_loss + 1e-6 < best_val:
            best_val = val_loss
            best_state = copy.deepcopy(model.state_dict())
            best_epoch = epoch + 1
            epochs_without_improvement = 0
        else:
            epochs_without_improvement += 1
            if epochs_without_improvement >= int(args.patience):
                break

    model.load_state_dict(best_state)

    full_grouped_rows = group_option_rows(option_rows)
    score_fn, probability_fn = build_inference_functions(
        model=model,
        grouped_rows=full_grouped_rows,
        state_vectorizer=state_vectorizer,
        state_scaler=state_scaler,
        option_vectorizer=option_vectorizer,
        option_scaler=option_scaler,
        source_to_index=source_to_index,
        option_type_to_index=option_type_to_index,
        device=device,
    )
    baseline_fn = baseline_score

    train_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    val_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    test_metrics, test_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=score_fn,
        probability_fn=probability_fn,
    )
    train_baseline_metrics, _ = evaluate_split(
        name="train",
        option_rows=train_option_rows,
        edge_rows=train_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )
    val_baseline_metrics, _ = evaluate_split(
        name="val",
        option_rows=val_option_rows,
        edge_rows=val_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )
    test_baseline_metrics, baseline_predictions = evaluate_split(
        name="test",
        option_rows=test_option_rows,
        edge_rows=test_edge_rows,
        option_lookup=option_lookup,
        target_scores=target_scores,
        positive_sets=positive_sets,
        target_distributions=all_target_distributions,
        score_fn=baseline_fn,
        probability_fn=lambda rows: {
            str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
            for row in rows
        },
    )

    reward_hardcase_test_metrics = None
    reward_hardcase_test_predictions = None
    reward_hardcase_baseline_metrics = None
    if reward_hardcase_test_ids:
        reward_hardcase_test_metrics, reward_hardcase_test_predictions = evaluate_split(
            name="test_reward_hardcase",
            option_rows=filter_rows_by_decision_ids(test_option_rows, reward_hardcase_test_ids),
            edge_rows=filter_rows_by_decision_ids(test_edge_rows, reward_hardcase_test_ids),
            option_lookup=option_lookup,
            target_scores=target_scores,
            positive_sets=positive_sets,
            target_distributions=all_target_distributions,
            score_fn=score_fn,
            probability_fn=probability_fn,
        )
        reward_hardcase_baseline_metrics, _ = evaluate_split(
            name="test_reward_hardcase",
            option_rows=filter_rows_by_decision_ids(test_option_rows, reward_hardcase_test_ids),
            edge_rows=filter_rows_by_decision_ids(test_edge_rows, reward_hardcase_test_ids),
            option_lookup=option_lookup,
            target_scores=target_scores,
            positive_sets=positive_sets,
            target_distributions=all_target_distributions,
            score_fn=baseline_fn,
            probability_fn=lambda rows: {
                str(row.get("option_id") or ""): (1.0 if bool(row.get("baseline_matches_option")) else 0.0)
                for row in rows
            },
        )

    metrics_out = args.metrics_out or (args.dataset_dir / f"{args.output_prefix}_metrics.json")
    predictions_out = args.predictions_out or (args.dataset_dir / f"{args.output_prefix}_predictions.jsonl")
    review_out = args.review_out or (args.dataset_dir / f"{args.output_prefix}_review.json")
    model_out = args.model_out or (args.dataset_dir / f"{args.output_prefix}_model.pt")

    torch.save(
        {
            "state_dict": model.state_dict(),
            "config": {
                "state_in_features": int(train_state_scaled.shape[1]),
                "option_in_features": int(train_option_scaled.shape[1]),
                "source_count": len(source_to_index),
                "option_type_count": len(option_type_to_index),
                "reward_source_index": source_to_index.get("reward"),
                "state_hidden": int(args.state_hidden),
                "option_hidden": int(args.option_hidden),
                "fusion_hidden": int(args.fusion_hidden),
                "source_embed_dim": int(args.source_embed_dim),
                "option_type_embed_dim": int(args.option_type_embed_dim),
                "dropout": float(args.dropout),
            },
            "source_to_index": source_to_index,
            "option_type_to_index": option_type_to_index,
            "state_vectorizer": state_vectorizer,
            "state_scaler": state_scaler,
            "option_vectorizer": option_vectorizer,
            "option_scaler": option_scaler,
        },
        model_out,
    )

    metrics = {
        "model": "macro_grouped_choice_tower_torch",
        "dataset_prefix": args.dataset_prefix,
        "feature_count": {
            "state": int(len(state_vectorizer.feature_names_)),
            "option": int(len(option_vectorizer.feature_names_)),
            "total": int(len(state_vectorizer.feature_names_) + len(option_vectorizer.feature_names_)),
        },
        "reward_hardcase": {
            "enabled_holdout": bool(args.reward_hardcase_holdout),
            "train_count": len(reward_hardcase_train_ids),
            "val_count": len(reward_hardcase_val_ids),
            "test_count": len(reward_hardcase_test_ids),
            "train_multiplier": float(args.reward_hardcase_train_multiplier),
        },
        "source_vocab": source_to_index,
        "option_type_vocab_size": len(option_type_to_index),
        "decision_splits": {
            "train": len({row["decision_id"] for row in train_option_rows}),
            "val": len({row["decision_id"] for row in val_option_rows}),
            "test": len({row["decision_id"] for row in test_option_rows}),
        },
        "row_splits": {
            "train_option_rows": len(train_option_rows),
            "val_option_rows": len(val_option_rows),
            "test_option_rows": len(test_option_rows),
            "train_edge_rows": len(train_edge_rows),
            "val_edge_rows": len(val_edge_rows),
            "test_edge_rows": len(test_edge_rows),
        },
        "train_decision_examples": len(train_examples),
        "val_decision_examples": len(val_examples),
        "test_decision_examples": len(test_examples),
        "best_epoch": best_epoch,
        "best_val_loss": best_val,
        "train_loss_snapshot": validate_groups(
            model,
            train_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
            reward_aux_weight=float(args.reward_aux_weight),
            reward_skip_margin_weight=float(args.reward_skip_margin_weight),
            reward_skip_margin=float(args.reward_skip_margin),
            reward_gate_weight=float(args.reward_gate_weight),
            reward_take_choice_weight=float(args.reward_take_choice_weight),
        ),
        "val_loss_snapshot": validate_groups(
            model,
            val_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
            reward_aux_weight=float(args.reward_aux_weight),
            reward_skip_margin_weight=float(args.reward_skip_margin_weight),
            reward_skip_margin=float(args.reward_skip_margin),
            reward_gate_weight=float(args.reward_gate_weight),
            reward_take_choice_weight=float(args.reward_take_choice_weight),
        ),
        "test_loss_snapshot": validate_groups(
            model,
            test_examples,
            device,
            pairwise_aux_weight=float(args.pairwise_aux_weight),
            reward_aux_weight=float(args.reward_aux_weight),
            reward_skip_margin_weight=float(args.reward_skip_margin_weight),
            reward_skip_margin=float(args.reward_skip_margin),
            reward_gate_weight=float(args.reward_gate_weight),
            reward_take_choice_weight=float(args.reward_take_choice_weight),
        ),
        "train": train_metrics,
        "val": val_metrics,
        "test": test_metrics,
        "baseline_controls": {
            "train": train_baseline_metrics,
            "val": val_baseline_metrics,
            "test": test_baseline_metrics,
        },
        "reward_hardcase_test": reward_hardcase_test_metrics,
        "reward_hardcase_test_baseline": reward_hardcase_baseline_metrics,
        "config": {
            "epochs": int(args.epochs),
            "patience": int(args.patience),
            "state_hidden": int(args.state_hidden),
            "option_hidden": int(args.option_hidden),
            "fusion_hidden": int(args.fusion_hidden),
            "source_embed_dim": int(args.source_embed_dim),
            "option_type_embed_dim": int(args.option_type_embed_dim),
            "dropout": float(args.dropout),
            "lr": float(args.lr),
            "weight_decay": float(args.weight_decay),
            "pairwise_aux_weight": float(args.pairwise_aux_weight),
            "reward_aux_weight": float(args.reward_aux_weight),
            "reward_skip_margin_weight": float(args.reward_skip_margin_weight),
            "reward_skip_margin": float(args.reward_skip_margin),
            "reward_gate_weight": float(args.reward_gate_weight),
            "reward_take_choice_weight": float(args.reward_take_choice_weight),
            "reward_hardcase_holdout": bool(args.reward_hardcase_holdout),
            "reward_hardcase_train_multiplier": float(args.reward_hardcase_train_multiplier),
            "target_temperature": float(args.target_temperature),
            "margin_target_mix": float(args.margin_target_mix),
            "reward_target_temperature": float(args.reward_target_temperature),
            "reward_margin_target_mix": float(args.reward_margin_target_mix),
            "shop_target_temperature": float(args.shop_target_temperature),
            "shop_margin_target_mix": float(args.shop_margin_target_mix),
            "campfire_target_temperature": float(args.campfire_target_temperature),
            "campfire_margin_target_mix": float(args.campfire_margin_target_mix),
        },
        "notes": [
            "state and option features are encoded in separate towers before grouped softmax choice",
            "option typing is source-specific via source adapters and source-kind::option-kind embeddings",
            "source kind is modeled both via embedding and a source-specific residual head",
            "targets are shaped from the counterfactual score margins instead of using top-bucket uniform weights",
            "reward_take_card options also train a reward-specific auxiliary head for package-gap filling",
            "reward groups use a hierarchical gate: skip vs take_card, then take_card intra-choice",
            "reward groups also train a skip-vs-take_card margin loss aligned to the teacher-positive side",
            "a light pairwise auxiliary term is kept to preserve ordering pressure between positive and negative options",
        ],
    }

    write_json(metrics_out, metrics)
    write_jsonl(predictions_out, test_predictions)
    write_json(
        review_out,
        {
            "metrics_path": str(metrics_out),
            "predictions_path": str(predictions_out),
            "model_path": str(model_out),
            "hard_mistakes": top_scoring_macro_mistakes(test_predictions),
            "reward_hardcase_mistakes": top_scoring_macro_mistakes(reward_hardcase_test_predictions or []),
            "baseline_control_mistakes": top_scoring_macro_mistakes(baseline_predictions),
        },
    )

    print(json.dumps(metrics, indent=2, ensure_ascii=False))
    print(f"wrote macro grouped choice tower metrics to {metrics_out}")
    print(f"wrote macro grouped choice tower predictions to {predictions_out}")
    print(f"wrote macro grouped choice tower model to {model_out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
