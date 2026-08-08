"""On-policy terminal policy objective over complete attempts."""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Protocol

import torch
from torch import Tensor

from .attempts import CompletedAttemptExperience
from .experience import DecisionExperienceBatch
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId, SelectionProbability
from .semantic_concat import (
    SemanticBatchConcatLimits,
    concatenate_semantic_decision_batches,
)
from .torch_policy import RaggedCandidateLogits, RaggedCategoricalPolicyConfig


class TorchOutcomeError(ValueError):
    """Complete outcome experience does not satisfy the policy objective."""


class CandidatePolicyScorer(Protocol):
    def __call__(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits: ...


@dataclass(frozen=True)
class OnPolicyTerminalLoss:
    """One differentiable policy loss plus exact behavior provenance."""

    value: Tensor
    attempt_count: int
    decision_count: int
    behavior_manifest_ids: tuple[tuple[BehaviorManifestId, ...], ...]
    selection_probabilities: tuple[tuple[SelectionProbability, ...], ...]


def on_policy_terminal_loss(
    scorer: CandidatePolicyScorer,
    attempts: Sequence[CompletedAttemptExperience],
    registry: BehaviorManifestRegistry,
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
) -> OnPolicyTerminalLoss:
    """Apply terminal REINFORCE to the exact sampled categorical behavior.

    Every complete attempt contributes equal total weight regardless of its
    length. A victory raises the relative probability of sampled actions; a
    defeat lowers it. Single-candidate decisions therefore have zero gradient.
    """

    if not callable(scorer):
        raise TorchOutcomeError("candidate policy scorer must be callable")
    if not isinstance(registry, BehaviorManifestRegistry):
        raise TorchOutcomeError("policy objective requires a behavior manifest registry")
    if not isinstance(concat_limits, SemanticBatchConcatLimits):
        raise TorchOutcomeError("policy objective requires semantic concat limits")
    if not isinstance(policy_config, RaggedCategoricalPolicyConfig):
        raise TorchOutcomeError("policy objective requires categorical policy config")
    normalized = tuple(attempts)
    if not normalized:
        raise TorchOutcomeError("policy objective requires at least one complete attempt")

    behavior_ids: list[tuple[BehaviorManifestId, ...]] = []
    probability_evidence: list[tuple[SelectionProbability, ...]] = []
    payloads: list[Mapping[str, object]] = []
    selected_ordinals: list[int] = []
    targets: list[int] = []
    weights: list[float] = []
    total_decisions = 0
    for attempt in normalized:
        if not isinstance(attempt, CompletedAttemptExperience):
            raise TorchOutcomeError("policy objective accepts only complete attempts")
        expected_decisions = sum(batch.decision_count for batch in attempt.batches)
        if attempt.decision_count != expected_decisions or expected_decisions <= 0:
            raise TorchOutcomeError(
                "complete attempt decision count disagrees with retained batches"
            )

        attempt_behavior_ids: list[BehaviorManifestId] = []
        attempt_probabilities: list[SelectionProbability] = []
        for batch in attempt.batches:
            _validate_batch(batch)
            try:
                manifest = registry.resolve(batch.behavior_manifest_id)
            except ValueError as error:
                raise TorchOutcomeError(
                    "complete attempt references an unknown behavior manifest"
                ) from error
            if manifest.behavior_rule != policy_config.behavior_rule:
                raise TorchOutcomeError(
                    "complete attempt behavior rule conflicts with policy config"
                )
            attempt_behavior_ids.append(batch.behavior_manifest_id)
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            attempt_probabilities.extend(batch.selection_probabilities)
            targets.extend(
                [attempt.terminal.terminal_reward] * batch.decision_count
            )
            weights.extend(
                [1.0 / (len(normalized) * expected_decisions)]
                * batch.decision_count
            )
        behavior_ids.append(tuple(attempt_behavior_ids))
        probability_evidence.append(tuple(attempt_probabilities))
        total_decisions += expected_decisions

    combined = concatenate_semantic_decision_batches(payloads, concat_limits)
    logits = scorer(combined)
    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchOutcomeError(
            "candidate policy scorer must return RaggedCandidateLogits"
        )
    selected_log_probabilities = _selected_log_probabilities(
        logits,
        selected_ordinals,
        policy_config,
    )
    _require_matching_propensities(
        logits,
        selected_ordinals,
        tuple(
            probability
            for attempt_probabilities in probability_evidence
            for probability in attempt_probabilities
        ),
        policy_config,
    )
    reward_tensor = torch.as_tensor(
        targets,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    weight_tensor = torch.as_tensor(
        weights,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    terms = -reward_tensor * selected_log_probabilities
    if not bool(torch.all(torch.isfinite(terms))):
        raise TorchOutcomeError("terminal policy loss terms must be finite")
    value = torch.sum(terms * weight_tensor)
    return OnPolicyTerminalLoss(
        value=value,
        attempt_count=len(normalized),
        decision_count=total_decisions,
        behavior_manifest_ids=tuple(behavior_ids),
        selection_probabilities=tuple(probability_evidence),
    )


def _validate_batch(batch: object) -> None:
    if not isinstance(batch, DecisionExperienceBatch):
        raise TorchOutcomeError(
            "complete attempt batches must be DecisionExperienceBatch values"
        )
    if batch.decision_count != len(batch.selected_ordinals):
        raise TorchOutcomeError("decision batch ordinals are misaligned")
    if batch.decision_count != len(batch.selection_probabilities):
        raise TorchOutcomeError(
            "decision batch selection probabilities are misaligned"
        )
    if not all(
        isinstance(probability, SelectionProbability)
        for probability in batch.selection_probabilities
    ):
        raise TorchOutcomeError(
            "decision batch selection probabilities must be typed"
        )


def _selected_log_probabilities(
    logits: RaggedCandidateLogits,
    selected_ordinals: Sequence[int],
    config: RaggedCategoricalPolicyConfig,
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
    row_ids = torch.repeat_interleave(
        torch.arange(
            logits.row_count,
            dtype=torch.long,
            device=logits.values.device,
        ),
        lengths,
    )
    scaled = logits.values / config.temperature
    row_max = torch.full(
        (logits.row_count,),
        -torch.inf,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_max.scatter_reduce_(0, row_ids, scaled.detach(), reduce="amax")
    shifted = scaled - row_max[row_ids]
    row_sum = torch.zeros(
        logits.row_count,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_sum.index_add_(0, row_ids, torch.exp(shifted))
    log_probabilities = shifted - torch.log(row_sum[row_ids])
    return log_probabilities[logits.row_splits[:-1] + ordinals]


def _require_matching_propensities(
    logits: RaggedCandidateLogits,
    selected_ordinals: Sequence[int],
    recorded: Sequence[SelectionProbability],
    config: RaggedCategoricalPolicyConfig,
) -> None:
    if len(recorded) != logits.row_count:
        raise TorchOutcomeError("selection probabilities must contain one value per row")
    splits = logits.row_splits.detach().cpu().tolist()
    for row, (start, end, ordinal, evidence) in enumerate(
        zip(
            splits[:-1],
            splits[1:],
            selected_ordinals,
            recorded,
            strict=True,
        )
    ):
        if evidence.value is None:
            raise TorchOutcomeError(
                "on-policy objective requires known selection probabilities"
            )
        probabilities = torch.softmax(
            logits.values[start:end].detach().to(dtype=torch.float64)
            / config.temperature,
            dim=0,
        )
        expected = float(probabilities[ordinal].item())
        if not math.isclose(
            evidence.value,
            expected,
            rel_tol=1e-6,
            abs_tol=1e-8,
        ):
            raise TorchOutcomeError(
                f"selection probability at row {row} is off-policy"
            )
