"""Seed-isolated supervised initialization from production strategic labels."""

from __future__ import annotations

import copy
import math
import operator
import time
from collections import Counter
from collections.abc import Mapping
from dataclasses import dataclass

import numpy as np
import torch

from .seeds import SeedPartition, SeedPartitionSpec
from .semantic_batch import select_semantic_decision_rows
from .semantic_concat import (
    SemanticBatchConcatLimits,
    concatenate_semantic_decision_batches,
)
from .strategic_demonstrations import StrategicDemonstrationCorpus
from .torch_policy import (
    RaggedCandidateScorer,
    ragged_cross_entropy,
)


class StrategicBehaviorCloningError(RuntimeError):
    """A demonstration split or supervised update is not exact and bounded."""


@dataclass(frozen=True)
class StrategicDemonstrationPartition:
    decision_batch: Mapping[str, object]
    target_ordinals: tuple[int, ...]
    episode_seeds: tuple[int, ...]
    context_kinds: tuple[int, ...]

    def __post_init__(self) -> None:
        row_count = len(self.target_ordinals)
        if row_count == 0:
            raise StrategicBehaviorCloningError(
                "demonstration partition must contain at least one row"
            )
        if not (
            len(self.episode_seeds) == row_count
            and len(self.context_kinds) == row_count
        ):
            raise StrategicBehaviorCloningError(
                "demonstration partition metadata is misaligned"
            )
        candidate_counts = _integer_vector(
            self.decision_batch,
            "candidate_counts",
        )
        if candidate_counts.size != row_count:
            raise StrategicBehaviorCloningError(
                "demonstration partition decision rows are misaligned"
            )
        if any(
            target < 0 or target >= int(candidate_counts[index])
            for index, target in enumerate(self.target_ordinals)
        ):
            raise StrategicBehaviorCloningError(
                "demonstration target is outside its candidate row"
            )

    @property
    def row_count(self) -> int:
        return len(self.target_ordinals)

    @property
    def seed_count(self) -> int:
        return len(set(self.episode_seeds))

    @property
    def context_counts(self) -> dict[int, int]:
        return dict(sorted(Counter(self.context_kinds).items()))


@dataclass(frozen=True)
class StrategicDemonstrationSplit:
    training: StrategicDemonstrationPartition
    held_out: StrategicDemonstrationPartition
    partition_spec: SeedPartitionSpec

    def __post_init__(self) -> None:
        if not isinstance(self.partition_spec, SeedPartitionSpec):
            raise StrategicBehaviorCloningError("partition_spec must be typed")
        overlap = set(self.training.episode_seeds) & set(self.held_out.episode_seeds)
        if overlap:
            raise StrategicBehaviorCloningError(
                "training and held-out demonstrations share a run seed"
            )


@dataclass(frozen=True)
class StrategicBehaviorCloningConfig:
    epochs: int = 16
    learning_rate: float = 3e-4
    max_grad_norm: float = 1.0
    device_type: str = "cpu"

    def __post_init__(self) -> None:
        object.__setattr__(self, "epochs", _positive(self.epochs, "epochs"))
        for name in ("learning_rate", "max_grad_norm"):
            value = getattr(self, name)
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                raise StrategicBehaviorCloningError(f"{name} must be numeric")
            normalized = float(value)
            if not math.isfinite(normalized) or normalized <= 0.0:
                raise StrategicBehaviorCloningError(
                    f"{name} must be finite and positive"
                )
            object.__setattr__(self, name, normalized)
        if self.device_type not in {"cpu", "cuda"}:
            raise StrategicBehaviorCloningError(
                "device_type must be either cpu or cuda"
            )
        if self.device_type == "cuda" and not torch.cuda.is_available():
            raise StrategicBehaviorCloningError("CUDA was requested but is unavailable")


@dataclass(frozen=True)
class StrategicContextAgreement:
    context_kind: int
    rows: int
    correct: int

    @property
    def agreement(self) -> float:
        return self.correct / self.rows


@dataclass(frozen=True)
class StrategicAgreement:
    rows: int
    correct: int
    cross_entropy: float
    contexts: tuple[StrategicContextAgreement, ...]

    @property
    def agreement(self) -> float:
        return self.correct / self.rows


@dataclass(frozen=True)
class StrategicBehaviorCloneResult:
    scorer: RaggedCandidateScorer
    initial_training: StrategicAgreement
    initial_held_out: StrategicAgreement
    final_training: StrategicAgreement
    final_held_out: StrategicAgreement
    epochs: int
    elapsed_seconds: float


def split_strategic_demonstrations(
    corpus: StrategicDemonstrationCorpus,
    partition_spec: SeedPartitionSpec,
    concat_limits: SemanticBatchConcatLimits,
) -> StrategicDemonstrationSplit:
    """Split by original run seed before concatenating any semantic rows."""

    if not isinstance(corpus, StrategicDemonstrationCorpus):
        raise StrategicBehaviorCloningError("demonstration corpus must be typed")
    if not isinstance(partition_spec, SeedPartitionSpec):
        raise StrategicBehaviorCloningError("partition_spec must be typed")
    if not isinstance(concat_limits, SemanticBatchConcatLimits):
        raise StrategicBehaviorCloningError("concat_limits must be typed")

    batches: dict[SeedPartition, list[Mapping[str, object]]] = {
        SeedPartition.TRAINING: [],
        SeedPartition.HELD_OUT: [],
    }
    targets: dict[SeedPartition, list[int]] = {
        SeedPartition.TRAINING: [],
        SeedPartition.HELD_OUT: [],
    }
    seeds: dict[SeedPartition, list[int]] = {
        SeedPartition.TRAINING: [],
        SeedPartition.HELD_OUT: [],
    }
    contexts: dict[SeedPartition, list[int]] = {
        SeedPartition.TRAINING: [],
        SeedPartition.HELD_OUT: [],
    }
    for batch in corpus.batches:
        for partition in SeedPartition:
            rows = tuple(
                index
                for index, seed in enumerate(batch.episode_seeds)
                if partition_spec.classify(seed) is partition
            )
            if not rows:
                continue
            batches[partition].append(
                select_semantic_decision_rows(batch.decision_batch, rows)
            )
            targets[partition].extend(batch.target_ordinals[row] for row in rows)
            seeds[partition].extend(batch.episode_seeds[row] for row in rows)
            contexts[partition].extend(batch.context_kinds[row] for row in rows)

    partitioned_rows = sum(len(values) for values in targets.values())
    if partitioned_rows != corpus.teacher_rows:
        raise StrategicBehaviorCloningError(
            "demonstration corpus teacher-row accounting is inconsistent"
        )

    partitions: dict[SeedPartition, StrategicDemonstrationPartition] = {}
    for partition in SeedPartition:
        if not batches[partition]:
            raise StrategicBehaviorCloningError(
                f"demonstration corpus has no {partition.value} teacher rows"
            )
        partitions[partition] = StrategicDemonstrationPartition(
            decision_batch=concatenate_semantic_decision_batches(
                batches[partition],
                concat_limits,
            ),
            target_ordinals=tuple(targets[partition]),
            episode_seeds=tuple(seeds[partition]),
            context_kinds=tuple(contexts[partition]),
        )
    return StrategicDemonstrationSplit(
        training=partitions[SeedPartition.TRAINING],
        held_out=partitions[SeedPartition.HELD_OUT],
        partition_spec=partition_spec,
    )


def train_strategic_behavior_clone(
    combat_anchor: RaggedCandidateScorer,
    demonstrations: StrategicDemonstrationSplit,
    config: StrategicBehaviorCloningConfig,
) -> StrategicBehaviorCloneResult:
    """Train a copied scorer; the caller's combat anchor remains immutable."""

    if not isinstance(combat_anchor, RaggedCandidateScorer):
        raise StrategicBehaviorCloningError("combat_anchor must be a scorer")
    if combat_anchor.training or any(
        parameter.requires_grad for parameter in combat_anchor.parameters()
    ):
        raise StrategicBehaviorCloningError("combat_anchor must be frozen")
    if not isinstance(demonstrations, StrategicDemonstrationSplit):
        raise StrategicBehaviorCloningError("demonstrations must be a typed split")
    if not isinstance(config, StrategicBehaviorCloningConfig):
        raise StrategicBehaviorCloningError("config must be typed")

    started = time.perf_counter()
    scorer = copy.deepcopy(combat_anchor).to(config.device_type)
    scorer.requires_grad_(True)
    initial_training = _agreement(scorer, demonstrations.training)
    initial_held_out = _agreement(scorer, demonstrations.held_out)
    optimizer = torch.optim.Adam(scorer.parameters(), lr=config.learning_rate)
    scorer.train()
    for _ in range(config.epochs):
        optimizer.zero_grad(set_to_none=True)
        logits = scorer(demonstrations.training.decision_batch)
        loss = ragged_cross_entropy(logits, demonstrations.training.target_ordinals)
        if not bool(torch.isfinite(loss)):
            raise StrategicBehaviorCloningError("behavior cloning loss is not finite")
        loss.backward()
        gradients = tuple(
            parameter.grad
            for parameter in scorer.parameters()
            if parameter.grad is not None
        )
        if not gradients or any(
            not bool(torch.all(torch.isfinite(gradient))) for gradient in gradients
        ):
            raise StrategicBehaviorCloningError(
                "behavior cloning gradients are missing or non-finite"
            )
        norm = torch.nn.utils.clip_grad_norm_(
            tuple(scorer.parameters()),
            config.max_grad_norm,
        )
        if not bool(torch.isfinite(norm)):
            raise StrategicBehaviorCloningError(
                "behavior cloning gradient norm is not finite"
            )
        optimizer.step()

    scorer.eval()
    scorer.requires_grad_(False)
    return StrategicBehaviorCloneResult(
        scorer=scorer,
        initial_training=initial_training,
        initial_held_out=initial_held_out,
        final_training=_agreement(scorer, demonstrations.training),
        final_held_out=_agreement(scorer, demonstrations.held_out),
        epochs=config.epochs,
        elapsed_seconds=time.perf_counter() - started,
    )


def _agreement(
    scorer: RaggedCandidateScorer,
    partition: StrategicDemonstrationPartition,
) -> StrategicAgreement:
    scorer.eval()
    with torch.inference_mode():
        logits = scorer(partition.decision_batch)
        loss = ragged_cross_entropy(logits, partition.target_ordinals)
        predicted = tuple(logits.greedy_ordinals())
    correct = tuple(
        predicted[index] == target
        for index, target in enumerate(partition.target_ordinals)
    )
    context_counts: Counter[int] = Counter()
    context_correct: Counter[int] = Counter()
    for context, matched in zip(partition.context_kinds, correct, strict=True):
        context_counts[context] += 1
        context_correct[context] += int(matched)
    return StrategicAgreement(
        rows=partition.row_count,
        correct=sum(correct),
        cross_entropy=float(loss.detach().cpu().item()),
        contexts=tuple(
            StrategicContextAgreement(
                context_kind=context,
                rows=context_counts[context],
                correct=context_correct[context],
            )
            for context in sorted(context_counts)
        ),
    )


def _integer_vector(source: Mapping[str, object], key: str) -> np.ndarray:
    value = source.get(key)
    if not isinstance(value, np.ndarray) or value.ndim != 1:
        raise StrategicBehaviorCloningError(f"{key} must be a NumPy vector")
    if value.dtype.kind not in "iu":
        raise StrategicBehaviorCloningError(f"{key} must contain integers")
    return value


def _positive(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise StrategicBehaviorCloningError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise StrategicBehaviorCloningError(f"{name} must be an integer") from error
    if normalized <= 0:
        raise StrategicBehaviorCloningError(f"{name} must be positive")
    return normalized
