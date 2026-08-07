"""Optional synchronous optimizer sink for bounded complete-attempt delivery."""

from __future__ import annotations

import math
import time
from dataclasses import dataclass

import torch

from .attempts import AttemptAssemblyDelivery, DroppedAttemptExperience
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId, SelectionProbability
from .semantic_concat import SemanticBatchConcatLimits
from .torch_outcomes import CandidateValueScorer, realized_outcome_value_loss


class TorchTrainingError(RuntimeError):
    """A synchronous training delivery cannot safely commit."""


@dataclass(frozen=True)
class SynchronousValueTrainerSnapshot:
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


class SynchronousValueTrainer:
    """Train once per delivery and retain no experience queue or tensor payload."""

    def __init__(
        self,
        scorer: CandidateValueScorer,
        optimizer: torch.optim.Optimizer,
        registry: BehaviorManifestRegistry,
        concat_limits: SemanticBatchConcatLimits,
    ) -> None:
        if not callable(scorer):
            raise TorchTrainingError("candidate value scorer must be callable")
        if not isinstance(optimizer, torch.optim.Optimizer):
            raise TorchTrainingError("optimizer must be a torch Optimizer")
        if not isinstance(registry, BehaviorManifestRegistry):
            raise TorchTrainingError("trainer requires a behavior manifest registry")
        if not isinstance(concat_limits, SemanticBatchConcatLimits):
            raise TorchTrainingError("trainer requires semantic concat limits")
        self.scorer = scorer
        self.optimizer = optimizer
        self.registry = registry
        self.concat_limits = concat_limits
        self._deliveries = 0
        self._optimizer_steps = 0
        self._completed_attempts = 0
        self._dropped_attempts = 0
        self._trained_decisions = 0
        self._last_loss: float | None = None
        self._last_behavior_manifest_ids: (
            tuple[tuple[BehaviorManifestId, ...], ...] | None
        ) = None
        self._last_selection_probabilities: (
            tuple[tuple[SelectionProbability, ...], ...] | None
        ) = None
        self._total_training_seconds = 0.0
        self._last_training_seconds: float | None = None
        self._poisoned = False

    @property
    def snapshot(self) -> SynchronousValueTrainerSnapshot:
        return SynchronousValueTrainerSnapshot(
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

        training_started = time.perf_counter()
        objective = realized_outcome_value_loss(
            self.scorer,
            delivery.completed,
            self.registry,
            self.concat_limits,
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
