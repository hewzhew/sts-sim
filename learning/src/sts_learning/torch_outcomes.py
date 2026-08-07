"""Optional realized-behavior outcome objective over complete attempts."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Protocol

import torch
from torch import Tensor

from .attempts import CompletedAttemptExperience
from .experience import DecisionExperienceBatch
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId
from .semantic_concat import (
    SemanticBatchConcatLimits,
    concatenate_semantic_decision_batches,
)
from .torch_policy import RaggedCandidateLogits


class TorchOutcomeError(ValueError):
    """Complete outcome experience does not satisfy the value objective."""


class CandidateValueScorer(Protocol):
    def __call__(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits: ...


@dataclass(frozen=True)
class RealizedOutcomeValueLoss:
    """One differentiable loss plus the exact behavior provenance it consumed."""

    value: Tensor
    attempt_count: int
    decision_count: int
    behavior_manifest_ids: tuple[tuple[BehaviorManifestId, ...], ...]


def realized_outcome_value_loss(
    scorer: CandidateValueScorer,
    attempts: Sequence[CompletedAttemptExperience],
    registry: BehaviorManifestRegistry,
    concat_limits: SemanticBatchConcatLimits,
) -> RealizedOutcomeValueLoss:
    """Regress selected candidate values to sparse terminal outcomes.

    Every complete attempt contributes one mean squared-error term regardless
    of its length. Unselected candidates receive no direct outcome target.
    """

    if not callable(scorer):
        raise TorchOutcomeError("candidate value scorer must be callable")
    if not isinstance(registry, BehaviorManifestRegistry):
        raise TorchOutcomeError("value objective requires a behavior manifest registry")
    if not isinstance(concat_limits, SemanticBatchConcatLimits):
        raise TorchOutcomeError("value objective requires semantic concat limits")
    normalized = tuple(attempts)
    if not normalized:
        raise TorchOutcomeError("value objective requires at least one complete attempt")

    behavior_ids: list[tuple[BehaviorManifestId, ...]] = []
    payloads: list[Mapping[str, object]] = []
    selected_ordinals: list[int] = []
    targets: list[int] = []
    weights: list[float] = []
    total_decisions = 0
    for attempt in normalized:
        if not isinstance(attempt, CompletedAttemptExperience):
            raise TorchOutcomeError("value objective accepts only complete attempts")
        expected_decisions = sum(batch.decision_count for batch in attempt.batches)
        if attempt.decision_count != expected_decisions or expected_decisions <= 0:
            raise TorchOutcomeError(
                "complete attempt decision count disagrees with retained batches"
            )

        attempt_behavior_ids: list[BehaviorManifestId] = []
        for batch in attempt.batches:
            _validate_batch(batch)
            try:
                registry.resolve(batch.behavior_manifest_id)
            except ValueError as error:
                raise TorchOutcomeError(
                    "complete attempt references an unknown behavior manifest"
                ) from error
            attempt_behavior_ids.append(batch.behavior_manifest_id)
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            targets.extend(
                [attempt.terminal.terminal_reward] * batch.decision_count
            )
            weights.extend(
                [1.0 / (len(normalized) * expected_decisions)]
                * batch.decision_count
            )
        behavior_ids.append(tuple(attempt_behavior_ids))
        total_decisions += expected_decisions

    combined = concatenate_semantic_decision_batches(payloads, concat_limits)
    logits = scorer(combined)
    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchOutcomeError(
            "candidate value scorer must return RaggedCandidateLogits"
        )
    selected = _selected_values(logits, selected_ordinals)
    target_tensor = torch.as_tensor(
        targets,
        dtype=selected.dtype,
        device=selected.device,
    )
    weight_tensor = torch.as_tensor(
        weights,
        dtype=selected.dtype,
        device=selected.device,
    )
    errors = (selected - target_tensor).square()
    if not bool(torch.all(torch.isfinite(errors))):
        raise TorchOutcomeError("realized outcome value errors must be finite")
    value = torch.sum(errors * weight_tensor)
    return RealizedOutcomeValueLoss(
        value=value,
        attempt_count=len(normalized),
        decision_count=total_decisions,
        behavior_manifest_ids=tuple(behavior_ids),
    )


def _validate_batch(batch: object) -> None:
    if not isinstance(batch, DecisionExperienceBatch):
        raise TorchOutcomeError(
            "complete attempt batches must be DecisionExperienceBatch values"
        )
    if batch.decision_count != len(batch.selected_ordinals):
        raise TorchOutcomeError("decision batch ordinals are misaligned")


def _selected_values(
    logits: RaggedCandidateLogits,
    selected_ordinals: Sequence[int],
) -> Tensor:
    ordinals = torch.as_tensor(
        tuple(selected_ordinals),
        dtype=torch.long,
        device=logits.values.device,
    )
    if ordinals.ndim != 1 or ordinals.numel() != logits.row_count:
        raise TorchOutcomeError("selected ordinals must contain one value per row")
    lengths = logits.row_splits[1:] - logits.row_splits[:-1]
    if bool(torch.any(ordinals < 0)) or bool(torch.any(ordinals >= lengths)):
        raise TorchOutcomeError("selected ordinal is outside its candidate row")
    return logits.values[logits.row_splits[:-1] + ordinals]
