#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from types import SimpleNamespace
from typing import Any

import numpy as np
import torch
from torch import nn

from build_structured_bc_teacher_dataset import legal_candidates
from combat_rl_common import REPO_ROOT, write_json, write_jsonl
from structured_candidate_ranker_common import (
    ACTION_CLASS_COUNT,
    CANDIDATE_FEATURE_DIM,
    MAX_RANKER_CANDIDATES,
    candidate_feature_vector,
)
from structured_combat_env import (
    CARD_ID_VOCAB,
    INTENT_KIND_IDS,
    MONSTER_ID_VOCAB,
    POTION_ID_VOCAB,
    POWER_ID_VOCAB,
    StructuredGymCombatEnv,
)
from structured_policy import StructuredPolicyNet, to_device_obs
from train_structured_combat_ppo import (
    current_energy_spent,
    first_step_signals,
    load_start_spec_name,
    parse_seed_list,
    start_spec_tag,
    summarize_benchmark_rows,
)


def _masked_logits(logits: torch.Tensor, mask: torch.Tensor) -> torch.Tensor:
    return logits.masked_fill(mask <= 0.5, -1e9)


class StructuredCandidateRanker(nn.Module):
    def __init__(
        self,
        *,
        card_vocab: int,
        potion_vocab: int,
        power_vocab: int,
        monster_vocab: int,
        intent_vocab: int,
        candidate_feature_dim: int = CANDIDATE_FEATURE_DIM,
        latent_dim: int = 32,
    ) -> None:
        super().__init__()
        self.state_encoder = StructuredPolicyNet(
            card_vocab=card_vocab,
            potion_vocab=potion_vocab,
            power_vocab=power_vocab,
            monster_vocab=monster_vocab,
            intent_vocab=intent_vocab,
            latent_dim=latent_dim,
        )
        self.score_head = nn.Sequential(
            nn.Linear(latent_dim + candidate_feature_dim, 96),
            nn.ReLU(),
            nn.Linear(96, 64),
            nn.ReLU(),
            nn.Linear(64, 1),
        )
        self.state_class_head = nn.Sequential(
            nn.Linear(latent_dim, 64),
            nn.ReLU(),
            nn.Linear(64, ACTION_CLASS_COUNT),
        )

    def forward(
        self,
        obs: dict[str, torch.Tensor],
        candidate_features: torch.Tensor,
    ) -> tuple[torch.Tensor, torch.Tensor]:
        state = self.state_encoder.encode(obs)
        batch, candidate_count, _ = candidate_features.shape
        tactical = state.tactical.unsqueeze(1).expand(batch, candidate_count, -1)
        scores = self.score_head(torch.cat([tactical, candidate_features.float()], dim=-1)).squeeze(-1)
        class_logits = self.state_class_head(state.tactical)
        return scores, class_logits


def load_ranker_dataset(path: Path) -> dict[str, Any]:
    with np.load(path, allow_pickle=False) as payload:
        obs = {
            key.removeprefix("obs__"): np.asarray(payload[key])
            for key in payload.files
            if key.startswith("obs__")
        }
        data = {key: np.asarray(payload[key]) for key in payload.files if not key.startswith("obs__")}
    required = {
        "candidate_features",
        "candidate_mask",
        "candidate_scores",
        "candidate_class",
        "candidate_actions",
        "best_mask",
        "best_index",
        "best_class",
    }
    missing = sorted(required - set(data))
    if missing:
        raise SystemExit(f"ranker dataset missing arrays {missing}: {path}")
    count = next(iter(obs.values())).shape[0]
    if any(value.shape[0] != count for value in obs.values()):
        raise SystemExit(f"ranker dataset obs arrays have inconsistent sample counts: {path}")
    if any(value.shape[0] != count for value in data.values()):
        raise SystemExit(f"ranker dataset arrays have inconsistent sample counts: {path}")
    if data["candidate_features"].shape[-1] != CANDIDATE_FEATURE_DIM:
        raise SystemExit(
            f"candidate feature dim mismatch {data['candidate_features'].shape[-1]} != {CANDIDATE_FEATURE_DIM}"
        )
    if "best_class_mask" not in data:
        class_mask = np.zeros((count, ACTION_CLASS_COUNT), dtype=np.float32)
        candidate_class = data["candidate_class"].astype(np.int64)
        best_mask = data["best_mask"] > 0.5
        for row_index in range(count):
            class_ids = candidate_class[row_index][best_mask[row_index]]
            if class_ids.size == 0:
                class_ids = np.asarray([int(data["best_class"][row_index])], dtype=np.int64)
            for class_id in class_ids:
                if 0 <= int(class_id) < ACTION_CLASS_COUNT:
                    class_mask[row_index, int(class_id)] = 1.0
        data["best_class_mask"] = class_mask
    return {"obs": obs, **data}


def index_obs(obs: dict[str, np.ndarray], indices: np.ndarray) -> dict[str, np.ndarray]:
    return {key: value[indices] for key, value in obs.items()}


def batch_from_dataset(dataset: dict[str, Any], indices: np.ndarray, device: torch.device) -> dict[str, Any]:
    return {
        "obs": to_device_obs(index_obs(dataset["obs"], indices), device),
        "candidate_features": torch.as_tensor(dataset["candidate_features"][indices], device=device).float(),
        "candidate_mask": torch.as_tensor(dataset["candidate_mask"][indices], device=device).float(),
        "candidate_scores": torch.as_tensor(dataset["candidate_scores"][indices], device=device).float(),
        "best_mask": torch.as_tensor(dataset["best_mask"][indices], device=device).float(),
        "best_index": torch.as_tensor(dataset["best_index"][indices], device=device).long(),
        "best_class": torch.as_tensor(dataset["best_class"][indices], device=device).long(),
        "best_class_mask": torch.as_tensor(dataset["best_class_mask"][indices], device=device).float(),
    }


def soft_top_loss(logits: torch.Tensor, mask: torch.Tensor, best_mask: torch.Tensor) -> torch.Tensor:
    masked = _masked_logits(logits, mask)
    log_probs = nn.functional.log_softmax(masked, dim=-1)
    target = best_mask / best_mask.sum(dim=-1, keepdim=True).clamp_min(1.0)
    return -(target * log_probs).sum(dim=-1).mean()


def soft_class_loss(logits: torch.Tensor, best_class_mask: torch.Tensor) -> torch.Tensor:
    log_probs = nn.functional.log_softmax(logits, dim=-1)
    target = best_class_mask / best_class_mask.sum(dim=-1, keepdim=True).clamp_min(1.0)
    return -(target * log_probs).sum(dim=-1).mean()


def evaluate_ranker_arrays(
    model: StructuredCandidateRanker,
    dataset: dict[str, Any],
    indices: np.ndarray,
    device: torch.device,
    batch_size: int,
) -> dict[str, Any]:
    if len(indices) == 0:
        return {
            "samples": 0,
            "top1_tie_match": 0.0,
            "top1_exact_match": 0.0,
            "class_accuracy": 0.0,
            "mean_teacher_regret": 0.0,
        }
    top1_tie = 0
    top1_exact = 0
    class_hits = 0
    class_exact_hits = 0
    regrets: list[float] = []
    model.eval()
    with torch.no_grad():
        for start in range(0, len(indices), batch_size):
            batch_indices = indices[start : start + batch_size]
            batch = batch_from_dataset(dataset, batch_indices, device)
            scores, class_logits = model(batch["obs"], batch["candidate_features"])
            pred = _masked_logits(scores, batch["candidate_mask"]).argmax(dim=-1)
            best_index = batch["best_index"]
            row = torch.arange(pred.shape[0], device=device)
            best_mask = batch["best_mask"]
            top1_tie += int((best_mask[row, pred] > 0.5).sum().item())
            top1_exact += int((pred == best_index).sum().item())
            class_pred = class_logits.argmax(dim=-1)
            class_hits += int((batch["best_class_mask"][row, class_pred] > 0.5).sum().item())
            class_exact_hits += int((class_pred == batch["best_class"]).sum().item())
            best_score = batch["candidate_scores"][row, best_index]
            pred_score = batch["candidate_scores"][row, pred]
            regrets.extend((best_score - pred_score).detach().cpu().numpy().astype(float).tolist())
    count = len(indices)
    return {
        "samples": int(count),
        "top1_tie_match": float(top1_tie / count),
        "top1_exact_match": float(top1_exact / count),
        "class_accuracy": float(class_hits / count),
        "class_exact_accuracy": float(class_exact_hits / count),
        "mean_teacher_regret": float(np.mean(regrets)) if regrets else 0.0,
    }


def candidate_features_for_env(info: dict[str, Any]) -> tuple[list[dict[str, Any]], np.ndarray, np.ndarray]:
    candidates = legal_candidates(info)[:MAX_RANKER_CANDIDATES]
    features = np.zeros((1, MAX_RANKER_CANDIDATES, CANDIDATE_FEATURE_DIM), dtype=np.float32)
    mask = np.zeros((1, MAX_RANKER_CANDIDATES), dtype=np.float32)
    raw = info.get("raw_observation") or {}
    for index, candidate in enumerate(candidates):
        features[0, index] = np.asarray(candidate_feature_vector(raw, candidate), dtype=np.float32)
        mask[0, index] = 1.0
    return candidates, features, mask


def select_ranker_action(
    model: StructuredCandidateRanker,
    env: StructuredGymCombatEnv,
    obs: dict[str, np.ndarray],
    info: dict[str, Any],
    device: torch.device,
) -> dict[str, int]:
    candidates, candidate_features, candidate_mask = candidate_features_for_env(info)
    if not candidates:
        return {"action_type": 0, "card_slot": 0, "target_slot": 0, "potion_slot": 0, "choice_index": 0}
    model.eval()
    with torch.no_grad():
        obs_tensor = to_device_obs({key: value[None, ...] for key, value in obs.items()}, device)
        features_t = torch.as_tensor(candidate_features, device=device).float()
        mask_t = torch.as_tensor(candidate_mask, device=device).float()
        scores, _ = model(obs_tensor, features_t)
        index = int(_masked_logits(scores, mask_t).argmax(dim=-1)[0].item())
    return env.candidate_to_canonical(candidates[index])


def evaluate_env_policy(
    model: StructuredCandidateRanker,
    *,
    eval_cases: list[tuple[Any, Path]],
    spec_source: str,
    draw_order_variant: str,
    reward_mode: str,
    reward_config: dict[str, float],
    driver_binary: Path | None,
    max_episode_steps: int,
    device: torch.device,
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    rows: list[dict[str, Any]] = []
    for case, spec_path in eval_cases:
        env = StructuredGymCombatEnv(
            [spec_path],
            spec_source=spec_source,
            driver_binary=driver_binary,
            max_episode_steps=max_episode_steps,
            seed=case.seed,
            draw_order_variant=draw_order_variant,
            reward_mode=reward_mode,
            reward_config=reward_config,
        )
        try:
            obs, info = env.reset(options={"spec_path": spec_path, "seed_hint": case.seed})
            done = False
            truncated = False
            reward_total = 0.0
            steps = 0
            invalid_actions = 0
            decoder_failures = 0
            energy_spent = 0.0
            first_action_label = None
            first_step_metrics: dict[str, Any] | None = None
            while not done and not truncated:
                before = info.get("raw_observation") or {}
                action = select_ranker_action(model, env, obs, info, device)
                energy_spent += current_energy_spent(before, action)
                obs, reward, done, truncated, info = env.step(action)
                reward_total += float(reward)
                invalid_actions += 1 if info.get("invalid_action") else 0
                decoder_failures += 1 if info.get("decoder_failure") else 0
                if first_action_label is None:
                    first_action_label = info.get("chosen_action_label")
                    first_step_metrics = first_step_signals(before, info.get("raw_observation") or {}, first_action_label)
                steps += 1
            row = {
                "spec_name": case.spec_name,
                "tag": case.tag,
                "seed": case.seed,
                "reward_total": reward_total,
                "steps": steps,
                "invalid_actions": invalid_actions,
                "decoder_failures": decoder_failures,
                "outcome": info.get("outcome"),
                "first_action_label": first_action_label,
                "energy_spent": energy_spent,
            }
            if first_step_metrics is not None:
                row.update(first_step_metrics)
            rows.append(row)
        finally:
            env.close()
    return summarize_benchmark_rows(rows), rows


def main() -> None:
    parser = argparse.ArgumentParser(description="Train a structured state+candidate combat ranker.")
    parser.add_argument("--dataset", required=True, type=Path)
    parser.add_argument("--start-spec", action="append", required=True, type=Path)
    parser.add_argument("--eval-seeds", default="2009,2010,2011,2012")
    parser.add_argument("--draw-order-variant", choices=["exact", "reshuffle_draw"], default="reshuffle_draw")
    parser.add_argument("--reward-mode", choices=["legacy", "minimal_rl"], default="minimal_rl")
    parser.add_argument("--victory-reward", default=1.0, type=float)
    parser.add_argument("--defeat-reward", default=-1.0, type=float)
    parser.add_argument("--hp-loss-scale", default=0.02, type=float)
    parser.add_argument("--enemy-hp-delta-scale", default=0.01, type=float)
    parser.add_argument("--kill-bonus-scale", default=0.0, type=float)
    parser.add_argument("--catastrophe-unblocked-threshold", default=18.0, type=float)
    parser.add_argument("--catastrophe-penalty", default=0.25, type=float)
    parser.add_argument("--next-enemy-window-relief-scale", default=0.0, type=float)
    parser.add_argument("--persistent-attack-script-relief-scale", default=0.0, type=float)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--max-episode-steps", default=128, type=int)
    parser.add_argument("--device", choices=["auto", "cpu", "cuda"], default="auto")
    parser.add_argument("--epochs", default=40, type=int)
    parser.add_argument("--batch-size", default=32, type=int)
    parser.add_argument("--learning-rate", default=3e-4, type=float)
    parser.add_argument("--class-loss-coef", default=0.25, type=float)
    parser.add_argument("--train-percent", default=80, type=int)
    parser.add_argument("--seed", default=7, type=int)
    parser.add_argument("--output-prefix", default="structured_candidate_ranker")
    parser.add_argument("--model-out", default=None, type=Path)
    parser.add_argument("--metrics-out", default=None, type=Path)
    parser.add_argument("--episodes-out", default=None, type=Path)
    args = parser.parse_args()

    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    if args.device == "auto":
        device = torch.device("cuda" if torch.cuda.is_available() else "cpu")
    elif args.device == "cuda":
        if not torch.cuda.is_available():
            raise SystemExit("requested --device cuda but torch.cuda.is_available() is false")
        device = torch.device("cuda")
    else:
        device = torch.device("cpu")

    dataset = load_ranker_dataset(args.dataset)
    count = next(iter(dataset["obs"].values())).shape[0]
    indices = np.arange(count)
    rng = np.random.default_rng(args.seed)
    rng.shuffle(indices)
    train_count = max(1, int(count * max(min(args.train_percent, 99), 1) / 100))
    train_indices = indices[:train_count]
    val_indices = indices[train_count:] if train_count < count else indices[:0]

    model = StructuredCandidateRanker(
        card_vocab=max(len(CARD_ID_VOCAB), 1),
        potion_vocab=max(len(POTION_ID_VOCAB), 1),
        power_vocab=max(len(POWER_ID_VOCAB), 1),
        monster_vocab=max(len(MONSTER_ID_VOCAB), 1),
        intent_vocab=max(len(INTENT_KIND_IDS), 1),
    ).to(device)
    optimizer = torch.optim.AdamW(model.parameters(), lr=args.learning_rate)
    timer = time.perf_counter()
    losses: list[float] = []
    rank_losses: list[float] = []
    class_losses: list[float] = []
    for _ in range(args.epochs):
        rng.shuffle(train_indices)
        model.train()
        for start in range(0, len(train_indices), args.batch_size):
            batch_indices = train_indices[start : start + args.batch_size]
            batch = batch_from_dataset(dataset, batch_indices, device)
            scores, class_logits = model(batch["obs"], batch["candidate_features"])
            rank_loss = soft_top_loss(scores, batch["candidate_mask"], batch["best_mask"])
            class_loss = soft_class_loss(class_logits, batch["best_class_mask"])
            loss = rank_loss + float(args.class_loss_coef) * class_loss
            optimizer.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            losses.append(float(loss.detach().cpu()))
            rank_losses.append(float(rank_loss.detach().cpu()))
            class_losses.append(float(class_loss.detach().cpu()))

    train_metrics = evaluate_ranker_arrays(model, dataset, train_indices, device, args.batch_size)
    val_metrics = evaluate_ranker_arrays(model, dataset, val_indices, device, args.batch_size)
    reward_config = {
        "victory_reward": float(args.victory_reward),
        "defeat_reward": float(args.defeat_reward),
        "hp_loss_scale": float(args.hp_loss_scale),
        "enemy_hp_delta_scale": float(args.enemy_hp_delta_scale),
        "kill_bonus_scale": float(args.kill_bonus_scale),
        "catastrophe_unblocked_threshold": float(args.catastrophe_unblocked_threshold),
        "catastrophe_penalty": float(args.catastrophe_penalty),
        "next_enemy_window_relief_scale": float(args.next_enemy_window_relief_scale),
        "persistent_attack_script_relief_scale": float(args.persistent_attack_script_relief_scale),
    }
    eval_seeds = parse_seed_list(args.eval_seeds)
    eval_cases = []
    for spec_path in args.start_spec:
        spec_name = load_start_spec_name(spec_path)
        tag = start_spec_tag(spec_path)
        for seed in eval_seeds:
            eval_cases.append((SimpleNamespace(spec_name=spec_name, seed=int(seed), tag=tag), spec_path))
    eval_metrics, eval_rows = evaluate_env_policy(
        model,
        eval_cases=eval_cases,
        spec_source="start_spec",
        draw_order_variant=args.draw_order_variant,
        reward_mode=args.reward_mode,
        reward_config=reward_config,
        driver_binary=args.driver_binary,
        max_episode_steps=args.max_episode_steps,
        device=device,
    )
    elapsed = time.perf_counter() - timer

    dataset_dir = REPO_ROOT / "tools" / "artifacts" / "learning_dataset"
    prefix = str(args.output_prefix or "").strip()
    model_out = args.model_out or dataset_dir / f"{prefix}_model.pt"
    metrics_out = args.metrics_out or dataset_dir / f"{prefix}_metrics.json"
    episodes_out = args.episodes_out or dataset_dir / f"{prefix}_eval_rows.jsonl"
    model_out.parent.mkdir(parents=True, exist_ok=True)
    metrics_out.parent.mkdir(parents=True, exist_ok=True)
    episodes_out.parent.mkdir(parents=True, exist_ok=True)
    torch.save(
        {
            "model_state": model.state_dict(),
            "config": {
                "card_vocab": max(len(CARD_ID_VOCAB), 1),
                "potion_vocab": max(len(POTION_ID_VOCAB), 1),
                "power_vocab": max(len(POWER_ID_VOCAB), 1),
                "monster_vocab": max(len(MONSTER_ID_VOCAB), 1),
                "intent_vocab": max(len(INTENT_KIND_IDS), 1),
                "candidate_feature_dim": CANDIDATE_FEATURE_DIM,
                "max_candidates": MAX_RANKER_CANDIDATES,
            },
        },
        model_out,
    )
    metrics = {
        "model": "structured_state_candidate_ranker",
        "dataset": str(args.dataset),
        "dataset_samples": int(count),
        "train_samples": int(len(train_indices)),
        "val_samples": int(len(val_indices)),
        "epochs": int(args.epochs),
        "batch_size": int(args.batch_size),
        "device": str(device),
        "torch": {
            "version": torch.__version__,
            "cuda_available": bool(torch.cuda.is_available()),
            "cuda_version": torch.version.cuda,
            "cuda_device": torch.cuda.get_device_name(0) if torch.cuda.is_available() else None,
        },
        "loss": {
            "mean_total": float(np.mean(losses)) if losses else 0.0,
            "mean_rank": float(np.mean(rank_losses)) if rank_losses else 0.0,
            "mean_class": float(np.mean(class_losses)) if class_losses else 0.0,
        },
        "offline_train": train_metrics,
        "offline_val": val_metrics,
        "eval": eval_metrics,
        "reward": reward_config,
        "timing": {
            "total_seconds": float(elapsed),
        },
        "notes": [
            "state encoder scores legal candidates instead of predicting concrete card slots directly",
            "candidate features include action abstraction and static/target context, not branch outcomes",
            "offline rank target uses the teacher top/tie set from one-step branch scores",
        ],
    }
    write_json(metrics_out, metrics)
    write_jsonl(episodes_out, eval_rows)
    print(json.dumps(metrics, indent=2, ensure_ascii=False), flush=True)
    print(f"wrote candidate ranker metrics to {metrics_out}", flush=True)


if __name__ == "__main__":
    main()
