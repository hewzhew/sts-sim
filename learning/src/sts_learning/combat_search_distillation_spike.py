"""Bounded, non-publishing distillation check for natural-root search proposals."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import operator
import statistics
import time
from collections.abc import Mapping, Sequence
from pathlib import Path

import torch

from .combat_root_artifacts import load_combat_root_source, read_combat_root_artifact
from .published_combat_behavior import recover_compatible_combat_scorer
from .semantic_concat import concatenate_semantic_decision_batches
from .torch_combat_session_config import CombatSessionBridge, CombatWinSessionLimits
from .torch_policy import RaggedCandidateScorer, ragged_cross_entropy


class CombatSearchDistillationError(RuntimeError):
    """Search evidence, semantic rows, or the bounded update was inconsistent."""


def run_combat_search_distillation_spike(
    *,
    training_pairs: Sequence[tuple[Path, Path]],
    held_out_pairs: Sequence[tuple[Path, Path]],
    behavior: Path,
    output: Path,
    epochs: int,
    learning_rate: float,
    max_grad_norm: float,
    max_artifact_bytes: int,
) -> dict[str, object]:
    """Fit only training proposals and evaluate every disjoint held-out root."""

    epochs = _positive(epochs, "epochs")
    learning_rate = _positive_float(learning_rate, "learning_rate")
    max_grad_norm = _positive_float(max_grad_norm, "max_grad_norm")
    max_artifact_bytes = _positive(max_artifact_bytes, "max_artifact_bytes")
    output = Path(output).resolve()
    if output.exists() or not output.parent.is_dir():
        raise CombatSearchDistillationError(
            "output must be a fresh file below an existing directory"
        )
    if not training_pairs or not held_out_pairs:
        raise CombatSearchDistillationError(
            "distillation requires training and held-out corpus pairs"
        )

    bridge = CombatSessionBridge.installed()
    limits = CombatWinSessionLimits(max_artifact_bytes=max_artifact_bytes)
    training = _load_partition(
        training_pairs,
        bridge=bridge,
        limits=limits,
        max_artifact_bytes=max_artifact_bytes,
    )
    held_out = _load_partition(
        held_out_pairs,
        bridge=bridge,
        limits=limits,
        max_artifact_bytes=max_artifact_bytes,
    )
    overlap = set(training["seeds"]) & set(held_out["seeds"])
    if overlap:
        raise CombatSearchDistillationError(
            "training and held-out natural roots share run seeds"
        )
    if not training["proposal_records"] or not held_out["proposal_records"]:
        raise CombatSearchDistillationError(
            "both partitions require at least one strict search proposal"
        )

    warm_start = recover_compatible_combat_scorer(
        behavior,
        bridge,
        limits,
    )
    anchor = warm_start.scorer
    if anchor.training or any(parameter.requires_grad for parameter in anchor.parameters()):
        raise CombatSearchDistillationError("recovered anchor must be frozen")

    started = time.perf_counter()
    initial_training = _partition_metrics(anchor, training, limits)
    initial_held_out = _partition_metrics(anchor, held_out, limits)
    scorer = copy.deepcopy(anchor)
    scorer.requires_grad_(True)
    scorer.train()
    optimizer = torch.optim.Adam(scorer.parameters(), lr=learning_rate)
    training_batch, training_targets = _improved_policy_batch(training, limits)
    update_losses: list[float] = []
    gradient_norms: list[float] = []
    for _ in range(epochs):
        optimizer.zero_grad(set_to_none=True)
        logits = scorer(training_batch)
        loss = ragged_cross_entropy(logits, training_targets)
        if not bool(torch.isfinite(loss)):
            raise CombatSearchDistillationError("distillation loss is not finite")
        loss.backward()
        gradients = tuple(
            parameter.grad
            for parameter in scorer.parameters()
            if parameter.grad is not None
        )
        if not gradients or any(
            not bool(torch.all(torch.isfinite(gradient))) for gradient in gradients
        ):
            raise CombatSearchDistillationError(
                "distillation gradients are missing or non-finite"
            )
        norm = torch.nn.utils.clip_grad_norm_(
            tuple(scorer.parameters()),
            max_grad_norm,
        )
        if not bool(torch.isfinite(norm)):
            raise CombatSearchDistillationError(
                "distillation gradient norm is not finite"
            )
        optimizer.step()
        update_losses.append(float(loss.detach().cpu().item()))
        gradient_norms.append(float(norm.detach().cpu().item()))
    scorer.eval()
    scorer.requires_grad_(False)

    final_training = _partition_metrics(scorer, training, limits)
    final_held_out = _partition_metrics(scorer, held_out, limits)
    result = {
        "schema": "sts-learning-combat-search-distillation-spike-v1",
        "teacher_valid": False,
        "model_published": False,
        "claim": "bounded_search_signal_generalization_feasibility_only",
        "training_source_manifest_id": warm_start.source_manifest_id.digest.hex(),
        "training": _partition_receipt(training),
        "held_out": _partition_receipt(held_out),
        "seed_overlap": tuple(sorted(overlap)),
        "config": {
            "epochs": epochs,
            "learning_rate": learning_rate,
            "max_grad_norm": max_grad_norm,
            "optimizer": "fresh_adam",
            "loss": "ragged_cross_entropy_on_strict_proposal_else_frozen_baseline",
            "potion_lane": "never",
        },
        "updates": {
            "losses": tuple(update_losses),
            "gradient_norms": tuple(gradient_norms),
        },
        "initial": {
            "training": initial_training,
            "held_out": initial_held_out,
        },
        "final": {
            "training": final_training,
            "held_out": final_held_out,
        },
        "held_out_delta": {
            "proposal_correct": (
                final_held_out["proposal_correct"]
                - initial_held_out["proposal_correct"]
            ),
            "proposal_cross_entropy": (
                final_held_out["proposal_cross_entropy"]
                - initial_held_out["proposal_cross_entropy"]
            ),
            "mean_both_win_hp_vs_baseline": _optional_delta(
                final_held_out["mean_both_win_hp_vs_baseline"],
                initial_held_out["mean_both_win_hp_vs_baseline"],
            ),
            "regressed_root_count": (
                final_held_out["regressed_root_count"]
                - initial_held_out["regressed_root_count"]
            ),
        },
        "elapsed_seconds": time.perf_counter() - started,
    }
    with output.open("x", encoding="utf-8", newline="\n") as destination:
        json.dump(result, destination, separators=(",", ":"), sort_keys=True)
        destination.write("\n")
    receipt = {
        "schema": result["schema"],
        "teacher_valid": False,
        "model_published": False,
        "training_proposals": result["training"]["proposal_count"],
        "held_out_proposals": result["held_out"]["proposal_count"],
        "initial_held_out": initial_held_out,
        "final_held_out": final_held_out,
        "held_out_delta": result["held_out_delta"],
        "artifact": str(output),
    }
    print(json.dumps(receipt, separators=(",", ":"), sort_keys=True), flush=True)
    return result


def _load_partition(
    pairs: Sequence[tuple[Path, Path]],
    *,
    bridge: CombatSessionBridge,
    limits: CombatWinSessionLimits,
    max_artifact_bytes: int,
) -> dict[str, object]:
    records: list[dict[str, object]] = []
    sources: list[dict[str, object]] = []
    seeds: list[int] = []
    for artifact_raw, manifest_raw in pairs:
        artifact = Path(artifact_raw).resolve()
        manifest_path = Path(manifest_raw).resolve()
        payload = read_combat_root_artifact(
            artifact,
            max_bytes=max_artifact_bytes,
        )
        manifest = _read_manifest(manifest_path)
        source_receipt = _mapping(manifest.get("source"), "source")
        root_count = _positive(source_receipt.get("root_count"), "root_count")
        digest = hashlib.sha256(payload).hexdigest()
        if source_receipt.get("artifact_sha256") != digest:
            raise CombatSearchDistillationError(
                "search manifest and opaque root artifact digest disagree"
            )
        search_config = _mapping(manifest.get("search"), "search")
        if search_config.get("potion_lane") != "never":
            raise CombatSearchDistillationError(
                "distillation only accepts no-potion search evidence"
            )
        source = load_combat_root_source(
            bridge,
            payload,
            expected_roots=root_count,
            max_bytes=max_artifact_bytes,
        )
        roots = _mapping_sequence(manifest, "roots")
        if len(roots) != root_count:
            raise CombatSearchDistillationError(
                "search manifest does not cover every declared natural root"
            )
        for expected_slot, root in enumerate(roots):
            slot = operator.index(root.get("root_slot"))
            if slot != expected_slot:
                raise CombatSearchDistillationError(
                    "search roots are not in exact artifact order"
                )
            group = source.combat_group(slot, 1, potion_slots=[])
            batch = group.decision_batch(semantic=True)
            counts = tuple(operator.index(value) for value in batch["candidate_counts"])
            if counts != (operator.index(root.get("candidate_count")),):
                raise CombatSearchDistillationError(
                    "search manifest and semantic candidate count disagree"
                )
            audit = _mapping(root.get("root"), "root")
            seed = _seed(audit.get("seed"), "root seed")
            seeds.append(seed)
            records.append(
                {
                    "batch": batch,
                    "seed": seed,
                    "ascension_level": operator.index(audit.get("ascension_level")),
                    "encounter_id": audit.get("encounter_id"),
                    "baseline_ordinal": operator.index(root.get("baseline_ordinal")),
                    "proposal_ordinal": (
                        None
                        if root.get("proposal_ordinal") is None
                        else operator.index(root.get("proposal_ordinal"))
                    ),
                    "actions": _mapping_sequence(
                        _mapping(root.get("search"), "root search"),
                        "actions",
                    ),
                }
            )
        sources.append(
            {
                "artifact": str(artifact),
                "artifact_sha256": digest,
                "search_manifest": str(manifest_path),
                "root_count": root_count,
            }
        )
    if len(set(seeds)) != len(seeds):
        raise CombatSearchDistillationError("one partition repeats a run seed")
    proposal_records = tuple(
        record for record in records if record["proposal_ordinal"] is not None
    )
    return {
        "records": tuple(records),
        "proposal_records": proposal_records,
        "sources": tuple(sources),
        "seeds": tuple(seeds),
    }


def _partition_metrics(
    scorer: RaggedCandidateScorer,
    partition: Mapping[str, object],
    limits: CombatWinSessionLimits,
) -> dict[str, object]:
    records = tuple(partition["records"])
    all_batch = concatenate_semantic_decision_batches(
        [record["batch"] for record in records],
        limits.concat,
    )
    scorer.eval()
    with torch.inference_mode():
        predictions = tuple(scorer(all_batch).greedy_ordinals())
    proposal_batch, proposal_targets = _proposal_batch(partition, limits)
    with torch.inference_mode():
        proposal_logits = scorer(proposal_batch)
        proposal_loss = ragged_cross_entropy(proposal_logits, proposal_targets)

    improved = 0
    equal = 0
    regressed = 0
    unknown = 0
    hp_deltas: list[float] = []
    best_hp_regrets: list[float] = []
    rows: list[dict[str, object]] = []
    for record, predicted in zip(records, predictions, strict=True):
        actions = tuple(record["actions"])
        baseline = operator.index(record["baseline_ordinal"])
        chosen = operator.index(predicted)
        if not 0 <= chosen < len(actions):
            raise CombatSearchDistillationError(
                "scorer selected outside the searched candidate surface"
            )
        chosen_quality = _action_quality(actions[chosen])
        baseline_quality = _action_quality(actions[baseline])
        best_ordinal = max(
            range(len(actions)),
            key=lambda ordinal: (*_action_quality(actions[ordinal]), -ordinal),
        )
        best_quality = _action_quality(actions[best_ordinal])
        if chosen_quality is None or baseline_quality is None:
            status = "unknown"
            unknown += 1
        elif chosen_quality > baseline_quality:
            status = "improved"
            improved += 1
        elif chosen_quality == baseline_quality:
            status = "equal"
            equal += 1
        else:
            status = "regressed"
            regressed += 1
        hp_delta = _both_win_hp_delta(actions[chosen], actions[baseline])
        if hp_delta is not None:
            hp_deltas.append(float(hp_delta))
        if chosen_quality is not None and best_quality is not None:
            if chosen_quality[0] == best_quality[0] == 1:
                best_hp_regrets.append(float(best_quality[1] - chosen_quality[1]))
        rows.append(
            {
                "seed": record["seed"],
                "ascension_level": record["ascension_level"],
                "encounter_id": record["encounter_id"],
                "baseline_ordinal": baseline,
                "proposal_ordinal": record["proposal_ordinal"],
                "predicted_ordinal": chosen,
                "best_search_ordinal": best_ordinal,
                "status_vs_baseline": status,
                "both_win_hp_delta_vs_baseline": hp_delta,
            }
        )
    proposal_predictions = tuple(proposal_logits.greedy_ordinals())
    proposal_correct = sum(
        prediction == target
        for prediction, target in zip(
            proposal_predictions,
            proposal_targets,
            strict=True,
        )
    )
    return {
        "root_count": len(records),
        "proposal_count": len(proposal_targets),
        "proposal_correct": proposal_correct,
        "proposal_agreement": proposal_correct / len(proposal_targets),
        "proposal_cross_entropy": float(proposal_loss.detach().cpu().item()),
        "improved_root_count": improved,
        "equal_root_count": equal,
        "regressed_root_count": regressed,
        "unknown_root_count": unknown,
        "mean_both_win_hp_vs_baseline": (
            statistics.fmean(hp_deltas) if hp_deltas else None
        ),
        "mean_best_search_hp_regret": (
            statistics.fmean(best_hp_regrets) if best_hp_regrets else None
        ),
        "rows": tuple(rows),
    }


def _proposal_batch(
    partition: Mapping[str, object],
    limits: CombatWinSessionLimits,
) -> tuple[Mapping[str, object], tuple[int, ...]]:
    records = tuple(partition["proposal_records"])
    if not records:
        raise CombatSearchDistillationError("partition has no proposal rows")
    return (
        concatenate_semantic_decision_batches(
            [record["batch"] for record in records],
            limits.concat,
        ),
        tuple(operator.index(record["proposal_ordinal"]) for record in records),
    )


def _improved_policy_batch(
    partition: Mapping[str, object],
    limits: CombatWinSessionLimits,
) -> tuple[Mapping[str, object], tuple[int, ...]]:
    records = tuple(partition["records"])
    if not records:
        raise CombatSearchDistillationError("partition has no natural roots")
    return (
        concatenate_semantic_decision_batches(
            [record["batch"] for record in records],
            limits.concat,
        ),
        tuple(
            operator.index(
                record["baseline_ordinal"]
                if record["proposal_ordinal"] is None
                else record["proposal_ordinal"]
            )
            for record in records
        ),
    )


def _action_quality(action: Mapping[str, object]) -> tuple[int, int] | None:
    if operator.index(action.get("budget_unknown_count")):
        return None
    return (
        operator.index(action.get("exact_win_count")),
        operator.index(action.get("winning_final_hp_sum")),
    )


def _both_win_hp_delta(
    chosen: Mapping[str, object],
    baseline: Mapping[str, object],
) -> int | None:
    if (
        operator.index(chosen.get("exact_win_count")) != 1
        or operator.index(baseline.get("exact_win_count")) != 1
    ):
        return None
    return operator.index(chosen.get("winning_final_hp_sum")) - operator.index(
        baseline.get("winning_final_hp_sum")
    )


def _partition_receipt(partition: Mapping[str, object]) -> dict[str, object]:
    records = tuple(partition["records"])
    return {
        "root_count": len(records),
        "proposal_count": len(partition["proposal_records"]),
        "seeds": partition["seeds"],
        "ascension_counts": {
            str(ascension): sum(
                record["ascension_level"] == ascension for record in records
            )
            for ascension in sorted({record["ascension_level"] for record in records})
        },
        "sources": partition["sources"],
    }


def _read_manifest(path: Path) -> Mapping[str, object]:
    if not path.is_file():
        raise CombatSearchDistillationError("search manifest does not exist")
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CombatSearchDistillationError("cannot read search manifest") from error
    if not isinstance(payload, Mapping):
        raise CombatSearchDistillationError("search manifest must be an object")
    if (
        payload.get("schema") != "sts-learning-natural-combat-search-census-v1"
        or payload.get("teacher_valid") is not False
    ):
        raise CombatSearchDistillationError("unsupported natural search manifest")
    return payload


def _mapping(value: object, name: str) -> Mapping[str, object]:
    if not isinstance(value, Mapping):
        raise CombatSearchDistillationError(f"{name} must be an object")
    return value


def _mapping_sequence(
    source: Mapping[str, object],
    key: str,
) -> tuple[Mapping[str, object], ...]:
    value = source.get(key)
    if not isinstance(value, Sequence) or isinstance(value, (str, bytes)):
        raise CombatSearchDistillationError(f"{key} must be a sequence")
    return tuple(_mapping(item, key) for item in value)


def _optional_delta(final: object, initial: object) -> float | None:
    if final is None or initial is None:
        return None
    return float(final) - float(initial)


def _positive(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise CombatSearchDistillationError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise CombatSearchDistillationError(f"{name} must be an integer") from error
    if normalized <= 0:
        raise CombatSearchDistillationError(f"{name} must be positive")
    return normalized


def _seed(value: object, name: str) -> int:
    normalized = _positive(value, name)
    if normalized >= 1 << 64:
        raise CombatSearchDistillationError(f"{name} must be below 2^64")
    return normalized


def _positive_float(value: object, name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise CombatSearchDistillationError(f"{name} must be numeric")
    normalized = float(value)
    if not math.isfinite(normalized) or normalized <= 0.0:
        raise CombatSearchDistillationError(f"{name} must be finite and positive")
    return normalized


def _pair_arguments(
    artifacts: Sequence[Path],
    manifests: Sequence[Path],
    name: str,
) -> tuple[tuple[Path, Path], ...]:
    if len(artifacts) != len(manifests) or not artifacts:
        raise CombatSearchDistillationError(
            f"{name} artifacts and search manifests must be non-empty and aligned"
        )
    return tuple(zip(artifacts, manifests, strict=True))


def _parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fit strict natural-root search proposals without publishing a model"
    )
    parser.add_argument("--training-artifact", type=Path, action="append", required=True)
    parser.add_argument("--training-search", type=Path, action="append", required=True)
    parser.add_argument("--held-out-artifact", type=Path, action="append", required=True)
    parser.add_argument("--held-out-search", type=Path, action="append", required=True)
    parser.add_argument("--behavior", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--epochs", type=int, default=16)
    parser.add_argument("--learning-rate", type=float, default=3e-4)
    parser.add_argument("--max-grad-norm", type=float, default=1.0)
    parser.add_argument("--max-artifact-bytes", type=int, default=16 * 1024 * 1024)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = _parse_args(argv)
    run_combat_search_distillation_spike(
        training_pairs=_pair_arguments(
            args.training_artifact,
            args.training_search,
            "training",
        ),
        held_out_pairs=_pair_arguments(
            args.held_out_artifact,
            args.held_out_search,
            "held-out",
        ),
        behavior=args.behavior,
        output=args.output,
        epochs=args.epochs,
        learning_rate=args.learning_rate,
        max_grad_norm=args.max_grad_norm,
        max_artifact_bytes=args.max_artifact_bytes,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
