"""Distill searched combat decision trajectories and run full-combat checks."""

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

from .combat_driver import CombatGroupDriver
from .combat_experience import CombatExperienceLimits
from .combat_root_artifacts import load_combat_root_source, read_combat_root_artifact
from .policy import BehaviorManifestId
from .published_combat_behavior import recover_compatible_combat_scorer
from .semantic_concat import concatenate_semantic_decision_batches
from .torch_combat_session_config import CombatSessionBridge, CombatWinSessionLimits
from .torch_policy import (
    GreedyTorchPolicy,
    RaggedCandidateScorer,
    ragged_cross_entropy,
)


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
    """Fit searched trajectory rows and evaluate disjoint complete combats."""

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
        require_unique_seeds=False,
    )
    held_out = _load_partition(
        held_out_pairs,
        bridge=bridge,
        limits=limits,
        max_artifact_bytes=max_artifact_bytes,
        require_unique_seeds=True,
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
    rollout_limits = CombatExperienceLimits(
        max_decisions=2_048,
        max_payload_bytes=64 * 1024 * 1024,
        max_model_rounds=1_024,
        max_transitions=4_096,
    )
    initial_full_combat = _full_combat_metrics(
        anchor,
        held_out,
        warm_start.source_manifest_id,
        rollout_limits,
    )
    scorer = copy.deepcopy(anchor)
    scorer.requires_grad_(True)
    scorer.train()
    optimizer = torch.optim.Adam(scorer.parameters(), lr=learning_rate)
    training_batch, training_targets = _improved_policy_batch(training, limits)
    update_losses: list[float] = []
    gradient_norms: list[float] = []
    for _ in range(epochs):
        optimizer.zero_grad(set_to_none=True)
        loss = ragged_cross_entropy(scorer(training_batch), training_targets)
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
    candidate_manifest_id = BehaviorManifestId(
        hashlib.sha256(
            warm_start.source_manifest_id.digest
            + b"combat-search-trajectory-distillation-spike-v2"
            + str(output).encode("utf-8")
        ).digest()
    )
    final_full_combat = _full_combat_metrics(
        scorer,
        held_out,
        candidate_manifest_id,
        rollout_limits,
    )
    full_combat_delta = _compare_full_combat_metrics(
        initial_full_combat,
        final_full_combat,
    )
    result = {
        "schema": "sts-learning-combat-search-distillation-spike-v2",
        "teacher_valid": False,
        "model_published": False,
        "claim": "bounded_search_trajectory_full_combat_feasibility_only",
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
            "held_out_full_combat_replicates_per_root": 2,
            "held_out_full_combat_rule": "frozen_greedy_model_only_no_search_suffix",
        },
        "updates": {
            "losses": tuple(update_losses),
            "gradient_norms": tuple(gradient_norms),
        },
        "initial": {
            "training": initial_training,
            "held_out": initial_held_out,
            "held_out_full_combat": initial_full_combat,
        },
        "final": {
            "training": final_training,
            "held_out": final_held_out,
            "held_out_full_combat": final_full_combat,
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
        "held_out_full_combat_delta": full_combat_delta,
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
        "initial_held_out": _without_rows(initial_held_out),
        "final_held_out": _without_rows(final_held_out),
        "held_out_delta": result["held_out_delta"],
        "initial_held_out_full_combat": _without_rows(initial_full_combat),
        "final_held_out_full_combat": _without_rows(final_full_combat),
        "held_out_full_combat_delta": _without_rows(full_combat_delta),
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
    require_unique_seeds: bool,
) -> dict[str, object]:
    records: list[dict[str, object]] = []
    sources: list[dict[str, object]] = []
    seeds: list[int] = []
    root_ids: list[str] = []
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
            root_id = str(group.root_id)
            root_ids.append(root_id)
            records.append(
                {
                    "batch": batch,
                    "source_owner": source,
                    "source_slot": slot,
                    "root_id": root_id,
                    "exact_combat_state_hash": str(group.exact_combat_state_hash),
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
    if len(set(root_ids)) != len(root_ids):
        raise CombatSearchDistillationError("one partition repeats an exact root")
    if require_unique_seeds and len(set(seeds)) != len(seeds):
        raise CombatSearchDistillationError("one partition repeats a run seed")
    proposal_records = tuple(
        record for record in records if record["proposal_ordinal"] is not None
    )
    return {
        "records": tuple(records),
        "proposal_records": proposal_records,
        "sources": tuple(sources),
        "seeds": tuple(seeds),
        "root_ids": tuple(root_ids),
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


def _full_combat_metrics(
    scorer: RaggedCandidateScorer,
    partition: Mapping[str, object],
    behavior_manifest_id: BehaviorManifestId,
    limits: CombatExperienceLimits,
) -> dict[str, object]:
    scorer.eval()
    policy = GreedyTorchPolicy(scorer, behavior_manifest_id)
    rows: list[dict[str, object]] = []
    replicate_count = 2
    for record in partition["records"]:
        source = record["source_owner"]
        env = source.combat_group(
            operator.index(record["source_slot"]),
            replicate_count,
            potion_slots=[],
        )
        run = CombatGroupDriver(env, policy, limits).run()
        outcomes = tuple(run.experience.outcomes.outcomes)
        wins = sum(outcome.won for outcome in outcomes)
        final_hps = tuple(outcome.final_hp for outcome in outcomes)
        winning_hps = tuple(
            outcome.final_hp for outcome in outcomes if outcome.won
        )
        rows.append(
            {
                "root_id": record["root_id"],
                "seed": record["seed"],
                "ascension_level": record["ascension_level"],
                "encounter_id": record["encounter_id"],
                "win_count": wins,
                "loss_count": replicate_count - wins,
                "final_hp_sum": sum(final_hps),
                "mean_final_hp": statistics.fmean(final_hps),
                "mean_winning_final_hp": (
                    statistics.fmean(winning_hps) if winning_hps else None
                ),
                "decision_count": run.experience.decision_count,
                "model_rounds": run.model_rounds,
                "transitions": run.transitions,
            }
        )
    result = _summarize_full_combat_rows(rows, replicate_count)
    result["by_ascension"] = {
        str(ascension): _summarize_full_combat_rows(
            [row for row in rows if row["ascension_level"] == ascension],
            replicate_count,
        )
        for ascension in sorted({row["ascension_level"] for row in rows})
    }
    result["rows"] = tuple(rows)
    return result


def _summarize_full_combat_rows(
    rows: Sequence[Mapping[str, object]],
    replicate_count: int,
) -> dict[str, object]:
    wins = sum(operator.index(row["win_count"]) for row in rows)
    final_hps = [float(row["mean_final_hp"]) for row in rows]
    winning_hps = [
        float(row["mean_winning_final_hp"])
        for row in rows
        if row["mean_winning_final_hp"] is not None
    ]
    return {
        "root_count": len(rows),
        "replicates_per_root": replicate_count,
        "win_count": wins,
        "loss_count": len(rows) * replicate_count - wins,
        "win_rate": wins / (len(rows) * replicate_count),
        "all_win_root_count": sum(
            operator.index(row["win_count"]) == replicate_count for row in rows
        ),
        "mean_final_hp": statistics.fmean(final_hps),
        "mean_winning_final_hp": (
            statistics.fmean(winning_hps) if winning_hps else None
        ),
    }


def _compare_full_combat_metrics(
    baseline: Mapping[str, object],
    candidate: Mapping[str, object],
) -> dict[str, object]:
    baseline_rows = {
        str(row["root_id"]): row for row in baseline["rows"]
    }
    candidate_rows = {
        str(row["root_id"]): row for row in candidate["rows"]
    }
    if baseline_rows.keys() != candidate_rows.keys():
        raise CombatSearchDistillationError(
            "full-combat evaluations do not cover the same exact roots"
        )
    improved = 0
    equal = 0
    regressed = 0
    both_win_hp_deltas: list[float] = []
    rows: list[dict[str, object]] = []
    replicate_count = operator.index(baseline["replicates_per_root"])
    for root_id, baseline_row in baseline_rows.items():
        candidate_row = candidate_rows[root_id]
        baseline_quality = (
            operator.index(baseline_row["win_count"]),
            operator.index(baseline_row["final_hp_sum"]),
        )
        candidate_quality = (
            operator.index(candidate_row["win_count"]),
            operator.index(candidate_row["final_hp_sum"]),
        )
        if candidate_quality > baseline_quality:
            status = "improved"
            improved += 1
        elif candidate_quality == baseline_quality:
            status = "equal"
            equal += 1
        else:
            status = "regressed"
            regressed += 1
        hp_delta = None
        if baseline_quality[0] == candidate_quality[0] == replicate_count:
            hp_delta = (
                candidate_quality[1] - baseline_quality[1]
            ) / replicate_count
            both_win_hp_deltas.append(float(hp_delta))
        rows.append(
            {
                "root_id": root_id,
                "seed": baseline_row["seed"],
                "ascension_level": baseline_row["ascension_level"],
                "encounter_id": baseline_row["encounter_id"],
                "baseline_win_count": baseline_quality[0],
                "candidate_win_count": candidate_quality[0],
                "both_win_final_hp_delta": hp_delta,
                "status": status,
            }
        )
    return {
        "root_count": len(rows),
        "improved_root_count": improved,
        "equal_root_count": equal,
        "regressed_root_count": regressed,
        "win_count_delta": (
            operator.index(candidate["win_count"])
            - operator.index(baseline["win_count"])
        ),
        "mean_both_win_final_hp_delta": (
            statistics.fmean(both_win_hp_deltas)
            if both_win_hp_deltas
            else None
        ),
        "rows": tuple(rows),
    }


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
        "unique_run_seed_count": len(set(partition["seeds"])),
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


def _without_rows(value: Mapping[str, object]) -> dict[str, object]:
    return {key: item for key, item in value.items() if key != "rows"}


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
        description=(
            "Fit strict search-trajectory proposals and evaluate complete held-out "
            "combats without publishing a model"
        )
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
