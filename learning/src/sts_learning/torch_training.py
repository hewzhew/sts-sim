"""Synchronous on-policy optimizer sink for complete-attempt delivery."""

from __future__ import annotations

import math
import operator
import time
from dataclasses import dataclass

import torch

from .attempts import AttemptAssemblyDelivery, DroppedAttemptExperience
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId, SelectionProbability
from .semantic_concat import SemanticBatchConcatLimits
from .terminal_returns import FloorProgressReturnConfig
from .torch_outcomes import CandidatePolicyScorer, on_policy_terminal_loss
from .torch_policy import RaggedCategoricalPolicyConfig


class TorchTrainingError(RuntimeError):
    """A synchronous training delivery cannot safely commit."""


@dataclass(frozen=True)
class SynchronousPolicyTrainerSnapshot:
    deliveries: int
    optimizer_steps: int
    completed_attempts: int
    dropped_attempts: int
    trained_decisions: int
    last_loss: float | None
    last_behavior_manifest_ids: tuple[tuple[BehaviorManifestId, ...], ...] | None
    last_selection_probabilities: (
        tuple[tuple[SelectionProbability, ...], ...] | None
    )
    total_training_seconds: float
    last_training_seconds: float | None
    poisoned: bool


class SynchronousPolicyTrainer:
    """Train once per on-policy delivery and retain no experience payload."""

    def __init__(
        self,
        scorer: CandidatePolicyScorer,
        optimizer: torch.optim.Optimizer,
        registry: BehaviorManifestRegistry,
        concat_limits: SemanticBatchConcatLimits,
        policy_config: RaggedCategoricalPolicyConfig,
        return_config: FloorProgressReturnConfig,
        attempts_per_update: int,
        *,
        resume_snapshot: SynchronousPolicyTrainerSnapshot | None = None,
    ) -> None:
        if not callable(scorer):
            raise TorchTrainingError("candidate policy scorer must be callable")
        if not isinstance(optimizer, torch.optim.Optimizer):
            raise TorchTrainingError("optimizer must be a torch Optimizer")
        if not isinstance(registry, BehaviorManifestRegistry):
            raise TorchTrainingError("trainer requires a behavior manifest registry")
        if not isinstance(concat_limits, SemanticBatchConcatLimits):
            raise TorchTrainingError("trainer requires semantic concat limits")
        if not isinstance(policy_config, RaggedCategoricalPolicyConfig):
            raise TorchTrainingError("trainer requires categorical policy config")
        if not isinstance(return_config, FloorProgressReturnConfig):
            raise TorchTrainingError("trainer requires terminal return config")
        self.scorer = scorer
        self.optimizer = optimizer
        self.registry = registry
        self.concat_limits = concat_limits
        self.policy_config = policy_config
        self.return_config = return_config
        self.attempts_per_update = _positive_integer(
            attempts_per_update,
            "attempts_per_update",
        )
        restored = _validated_resume_snapshot(resume_snapshot)
        self._deliveries = restored.deliveries
        self._optimizer_steps = restored.optimizer_steps
        self._completed_attempts = restored.completed_attempts
        self._dropped_attempts = restored.dropped_attempts
        self._trained_decisions = restored.trained_decisions
        self._last_loss = restored.last_loss
        self._last_behavior_manifest_ids = restored.last_behavior_manifest_ids
        self._last_selection_probabilities = restored.last_selection_probabilities
        self._total_training_seconds = restored.total_training_seconds
        self._last_training_seconds = restored.last_training_seconds
        self._poisoned = False

    @property
    def snapshot(self) -> SynchronousPolicyTrainerSnapshot:
        return SynchronousPolicyTrainerSnapshot(
            deliveries=self._deliveries,
            optimizer_steps=self._optimizer_steps,
            completed_attempts=self._completed_attempts,
            dropped_attempts=self._dropped_attempts,
            trained_decisions=self._trained_decisions,
            last_loss=self._last_loss,
            last_behavior_manifest_ids=self._last_behavior_manifest_ids,
            last_selection_probabilities=self._last_selection_probabilities,
            total_training_seconds=self._total_training_seconds,
            last_training_seconds=self._last_training_seconds,
            poisoned=self._poisoned,
        )

    def __call__(self, delivery: AttemptAssemblyDelivery) -> None:
        if self._poisoned:
            raise TorchTrainingError("trainer is poisoned after an optimizer failure")
        if not isinstance(delivery, AttemptAssemblyDelivery):
            raise TorchTrainingError("trainer requires AttemptAssemblyDelivery input")
        if not all(
            isinstance(attempt, DroppedAttemptExperience)
            for attempt in delivery.dropped
        ):
            raise TorchTrainingError("dropped delivery rows are malformed")

        completed_count = len(delivery.completed)
        dropped_count = len(delivery.dropped)
        if completed_count == 0:
            self._deliveries += 1
            self._dropped_attempts += dropped_count
            return
        if completed_count != self.attempts_per_update:
            raise TorchTrainingError(
                "training delivery must contain exactly attempts_per_update "
                "completed attempts"
            )

        training_started = time.perf_counter()
        objective = on_policy_terminal_loss(
            self.scorer,
            delivery.completed,
            self.registry,
            self.concat_limits,
            self.policy_config,
            self.return_config,
        )
        if objective.value.ndim != 0 or not objective.value.requires_grad:
            raise TorchTrainingError(
                "realized outcome objective must be a differentiable scalar"
            )

        try:
            self.optimizer.zero_grad(set_to_none=True)
            objective.value.backward()
            gradients = tuple(
                parameter.grad
                for group in self.optimizer.param_groups
                for parameter in group["params"]
                if parameter.grad is not None
            )
            if not gradients:
                raise TorchTrainingError("optimizer received no gradients")
            if not all(bool(torch.all(torch.isfinite(gradient))) for gradient in gradients):
                raise TorchTrainingError("optimizer gradients must be finite")
            self.optimizer.step()
        except Exception:
            self._poisoned = True
            raise

        loss = float(objective.value.detach().item())
        if not math.isfinite(loss):
            self._poisoned = True
            raise TorchTrainingError("committed optimizer loss must be finite")
        self._deliveries += 1
        self._optimizer_steps += 1
        self._completed_attempts += completed_count
        self._dropped_attempts += dropped_count
        self._trained_decisions += objective.decision_count
        self._last_loss = loss
        self._last_behavior_manifest_ids = objective.behavior_manifest_ids
        self._last_selection_probabilities = objective.selection_probabilities
        elapsed = time.perf_counter() - training_started
        self._total_training_seconds += elapsed
        self._last_training_seconds = elapsed


def _validated_resume_snapshot(
    snapshot: SynchronousPolicyTrainerSnapshot | None,
) -> SynchronousPolicyTrainerSnapshot:
    if snapshot is None:
        return SynchronousPolicyTrainerSnapshot(
            deliveries=0,
            optimizer_steps=0,
            completed_attempts=0,
            dropped_attempts=0,
            trained_decisions=0,
            last_loss=None,
            last_behavior_manifest_ids=None,
            last_selection_probabilities=None,
            total_training_seconds=0.0,
            last_training_seconds=None,
            poisoned=False,
        )
    if not isinstance(snapshot, SynchronousPolicyTrainerSnapshot):
        raise TorchTrainingError("trainer resume snapshot must be typed")
    if snapshot.poisoned:
        raise TorchTrainingError("cannot resume a poisoned trainer")
    for name in (
        "deliveries",
        "optimizer_steps",
        "completed_attempts",
        "dropped_attempts",
        "trained_decisions",
    ):
        value = getattr(snapshot, name)
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise TorchTrainingError(f"trainer resume {name} must be non-negative")
    if snapshot.last_loss is not None and not math.isfinite(snapshot.last_loss):
        raise TorchTrainingError("trainer resume loss must be finite")
    if (
        not math.isfinite(snapshot.total_training_seconds)
        or snapshot.total_training_seconds < 0.0
    ):
        raise TorchTrainingError("trainer resume total time must be finite")
    if snapshot.last_training_seconds is not None and (
        not math.isfinite(snapshot.last_training_seconds)
        or snapshot.last_training_seconds < 0.0
    ):
        raise TorchTrainingError("trainer resume last time must be finite")
    if snapshot.optimizer_steps == 0:
        if (
            snapshot.last_loss is not None
            or snapshot.last_behavior_manifest_ids is not None
            or snapshot.last_selection_probabilities is not None
            or snapshot.last_training_seconds is not None
        ):
            raise TorchTrainingError(
                "trainer without optimizer steps cannot have last-training evidence"
            )
    else:
        if (
            snapshot.last_loss is None
            or snapshot.last_behavior_manifest_ids is None
            or snapshot.last_selection_probabilities is None
            or snapshot.last_training_seconds is None
        ):
            raise TorchTrainingError(
                "trainer optimizer steps require last-training evidence"
            )
        if len(snapshot.last_behavior_manifest_ids) != len(
            snapshot.last_selection_probabilities
        ):
            raise TorchTrainingError("trainer resume evidence attempts are misaligned")
        for manifest_ids, probabilities in zip(
            snapshot.last_behavior_manifest_ids,
            snapshot.last_selection_probabilities,
            strict=True,
        ):
            if len(manifest_ids) != len(probabilities):
                raise TorchTrainingError(
                    "trainer resume evidence decisions are misaligned"
                )
            if not all(
                isinstance(manifest_id, BehaviorManifestId)
                for manifest_id in manifest_ids
            ):
                raise TorchTrainingError("trainer resume manifest ids are malformed")
            if not all(
                isinstance(probability, SelectionProbability)
                for probability in probabilities
            ):
                raise TorchTrainingError(
                    "trainer resume selection probabilities are malformed"
                )
    return snapshot


def _positive_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise TorchTrainingError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchTrainingError(f"{name} must be an integer") from error
    if normalized <= 0:
        raise TorchTrainingError(f"{name} must be positive")
    return normalized
