"""Bounded optimizer-step generations over the online PyTorch caller."""

from __future__ import annotations

import operator
from dataclasses import dataclass

import torch

from .attempts import AttemptAssemblerSnapshot, BoundedAttemptAssembler
from .attempt_batching import AttemptUpdateBatchError, BoundedAttemptUpdateBatcher
from .driver import BatchDriverError, BatchDriverResumeBoundary, OnlineBatchDriver
from .policy import BehaviorManifestId
from .torch_behavior import (
    CategoricalTorchBehaviorController,
    CategoricalTorchBehaviorControllerSnapshot,
    TorchBehaviorPublication,
)
from .torch_policy import RaggedCandidateScorer
from .torch_provenance import categorical_trainer_implementation
from .torch_training import SynchronousPolicyTrainer, SynchronousPolicyTrainerSnapshot


class TorchGenerationError(RuntimeError):
    """A generation runner is miswired or cannot make an exact promotion."""


@dataclass(frozen=True)
class CategoricalGenerationAdvanceResult:
    """Compact facts for one bounded continuation toward the next generation."""

    active_manifest_id_before: BehaviorManifestId
    active_training_step_before: int
    batch_step_limit: int
    batch_steps: int
    terminal_attempts: int
    terminal_flushes: int
    optimizer_steps_before: int
    optimizer_steps_after: int
    promotion_target_training_step: int
    publication: TorchBehaviorPublication | None

    @property
    def promoted(self) -> bool:
        return self.publication is not None

    @property
    def step_limit_reached(self) -> bool:
        return not self.promoted and self.batch_steps == self.batch_step_limit


@dataclass(frozen=True)
class CategoricalGenerationResumeBoundary:
    """Typed quiescent state admitted for a future exact resume publication."""

    driver: BatchDriverResumeBoundary
    assembler: AttemptAssemblerSnapshot
    trainer: SynchronousPolicyTrainerSnapshot
    controller: CategoricalTorchBehaviorControllerSnapshot


class BoundedCategoricalGenerationRunner:
    """Train for one on-policy optimizer step, then promote exactly once."""

    def __init__(
        self,
        driver: OnlineBatchDriver,
        assembler: BoundedAttemptAssembler,
        update_batcher: BoundedAttemptUpdateBatcher,
        trainer: SynchronousPolicyTrainer,
        controller: CategoricalTorchBehaviorController,
        shadow_scorer: RaggedCandidateScorer,
        *,
        optimizer_steps_per_generation: int,
    ) -> None:
        if not isinstance(driver, OnlineBatchDriver):
            raise TorchGenerationError("generation runner requires an online driver")
        if not isinstance(assembler, BoundedAttemptAssembler):
            raise TorchGenerationError("generation runner requires an attempt assembler")
        if not isinstance(update_batcher, BoundedAttemptUpdateBatcher):
            raise TorchGenerationError(
                "generation runner requires an attempt update batcher"
            )
        if not isinstance(trainer, SynchronousPolicyTrainer):
            raise TorchGenerationError("generation runner requires a policy trainer")
        if not isinstance(controller, CategoricalTorchBehaviorController):
            raise TorchGenerationError("generation runner requires a behavior controller")
        if not isinstance(shadow_scorer, RaggedCandidateScorer):
            raise TorchGenerationError("generation runner requires a shadow scorer")
        steps = _positive_count(
            optimizer_steps_per_generation,
            "optimizer_steps_per_generation",
        )
        if steps != 1:
            raise TorchGenerationError(
                "on-policy generation requires exactly one optimizer step"
            )
        self.driver = driver
        self.assembler = assembler
        self.update_batcher = update_batcher
        self.trainer = trainer
        self.controller = controller
        self.shadow_scorer = shadow_scorer
        self._optimizer_steps_per_generation = steps
        self._validate_wiring()

    @property
    def optimizer_steps_per_generation(self) -> int:
        return self._optimizer_steps_per_generation

    def require_resume_boundary(self) -> CategoricalGenerationResumeBoundary:
        """Fail closed unless every synchronous owner is safely checkpointable."""

        self._validate_wiring()
        try:
            driver = self.driver.require_resume_boundary()
        except BatchDriverError as error:
            raise TorchGenerationError(str(error)) from error
        assembler = self.assembler.snapshot
        if (
            assembler.open_attempts != 0
            or assembler.dropped_open_attempts != 0
            or assembler.retained_decisions != 0
            or assembler.retained_payload_bytes != 0
        ):
            raise TorchGenerationError(
                "resume boundary requires no open attempt-assembly state"
            )
        if driver.experience_next_sequence_index is None:
            raise TorchGenerationError(
                "generation resume requires an experience segment buffer"
            )
        if assembler.next_sequence_index != driver.experience_next_sequence_index:
            raise TorchGenerationError(
                "experience buffer and attempt assembler sequence indices differ"
            )

        try:
            self.update_batcher.require_quiescent()
        except AttemptUpdateBatchError as error:
            raise TorchGenerationError(str(error)) from error

        trainer = self.trainer.snapshot
        if trainer.poisoned:
            raise TorchGenerationError("generation trainer is poisoned")
        controller = self.controller.snapshot
        if (
            controller.active_manifest_id is None
            or controller.active_training_step is None
        ):
            raise TorchGenerationError("generation controller has no active behavior")
        if trainer.optimizer_steps < controller.active_training_step:
            raise TorchGenerationError(
                "trainer optimizer step is behind the active behavior generation"
            )
        return CategoricalGenerationResumeBoundary(
            driver=driver,
            assembler=assembler,
            trainer=trainer,
            controller=controller,
        )

    def advance(self, *, max_batch_steps: int) -> CategoricalGenerationAdvanceResult:
        """Continue the current target, promoting only after enough real updates."""

        step_limit = _non_negative_count(max_batch_steps, "max_batch_steps")
        self._validate_wiring()
        controller_before = self.controller.snapshot
        active_step = controller_before.active_training_step
        active_manifest_id = controller_before.active_manifest_id
        if active_step is None or active_manifest_id is None:
            raise TorchGenerationError("generation controller has no active behavior")
        trainer_before = self.trainer.snapshot
        if trainer_before.poisoned:
            raise TorchGenerationError("generation trainer is poisoned")
        if trainer_before.optimizer_steps < active_step:
            raise TorchGenerationError(
                "trainer optimizer step is behind the active behavior generation"
            )

        target_step = active_step + self.optimizer_steps_per_generation
        if trainer_before.optimizer_steps < target_step:
            self.controller.publisher.preview_novel(
                self.shadow_scorer,
                training_step=target_step,
            )
        else:
            self.controller.publisher.preview(
                self.shadow_scorer,
                training_step=trainer_before.optimizer_steps,
            )
        batch_steps = 0
        terminal_attempts = 0
        terminal_flushes = 0
        while (
            self.trainer.snapshot.optimizer_steps < target_step
            and batch_steps < step_limit
        ):
            result = self.driver.advance()
            batch_steps += 1
            terminal_attempts += len(result.attempts)
            if result.attempts:
                self.driver.flush_experience()
                terminal_flushes += 1

        trainer_after = self.trainer.snapshot
        if trainer_after.poisoned:
            raise TorchGenerationError("generation trainer became poisoned")
        publication = None
        if trainer_after.optimizer_steps >= target_step:
            publication = self.controller.publish_and_promote(
                self.shadow_scorer,
                training_step=trainer_after.optimizer_steps,
            )

        return CategoricalGenerationAdvanceResult(
            active_manifest_id_before=active_manifest_id,
            active_training_step_before=active_step,
            batch_step_limit=step_limit,
            batch_steps=batch_steps,
            terminal_attempts=terminal_attempts,
            terminal_flushes=terminal_flushes,
            optimizer_steps_before=trainer_before.optimizer_steps,
            optimizer_steps_after=trainer_after.optimizer_steps,
            promotion_target_training_step=target_step,
            publication=publication,
        )

    def _validate_wiring(self) -> None:
        if self.driver.policy is not self.controller:
            raise TorchGenerationError("driver policy is not the generation controller")
        if self.driver.experience_sink is not self.assembler:
            raise TorchGenerationError("driver is not wired to the generation assembler")
        if self.assembler.completed_attempt_sink is not self.update_batcher:
            raise TorchGenerationError(
                "assembler is not wired to the generation update batcher"
            )
        if self.update_batcher.update_sink is not self.trainer:
            raise TorchGenerationError(
                "attempt update batcher is not wired to the generation trainer"
            )
        if (
            self.update_batcher.attempts_per_update
            != self.trainer.objective_config.attempts_per_update
        ):
            raise TorchGenerationError(
                "attempt update batcher and trainer disagree on attempts_per_update"
            )
        if self.trainer.scorer is not self.shadow_scorer:
            raise TorchGenerationError(
                "trainer does not score the generation shadow model"
            )
        if self.trainer.registry is not self.controller.publisher.registry:
            raise TorchGenerationError(
                "trainer and controller do not share one manifest registry"
            )
        active_manifest_id = self.controller.snapshot.active_manifest_id
        if active_manifest_id is None:
            raise TorchGenerationError("generation controller has no active behavior")
        if (
            self.update_batcher.pending_behavior_manifest_id is not None
            and self.update_batcher.pending_behavior_manifest_id != active_manifest_id
        ):
            raise TorchGenerationError(
                "pending attempt update batch does not match active behavior"
            )
        active_manifest = self.trainer.registry.resolve(active_manifest_id)
        if active_manifest.trainer_implementation != categorical_trainer_implementation(
            self.trainer.objective_config,
        ):
            raise TorchGenerationError(
                "active behavior conflicts with the trainer implementation"
            )
        _require_exact_optimizer_parameters(
            self.trainer.optimizer,
            self.shadow_scorer,
        )


def _require_exact_optimizer_parameters(
    optimizer: torch.optim.Optimizer,
    scorer: RaggedCandidateScorer,
) -> None:
    optimizer_parameters = tuple(
        parameter
        for group in optimizer.param_groups
        for parameter in group["params"]
    )
    scorer_parameters = tuple(scorer.parameters())
    optimizer_ids = tuple(id(parameter) for parameter in optimizer_parameters)
    scorer_ids = tuple(id(parameter) for parameter in scorer_parameters)
    if len(set(optimizer_ids)) != len(optimizer_ids):
        raise TorchGenerationError("generation optimizer repeats a model parameter")
    if set(optimizer_ids) != set(scorer_ids):
        raise TorchGenerationError(
            "generation optimizer does not own exactly the shadow model parameters"
        )


def _positive_count(value: int, name: str) -> int:
    normalized = _non_negative_count(value, name)
    if normalized == 0:
        raise TorchGenerationError(f"{name} must be positive")
    return normalized


def _non_negative_count(value: int, name: str) -> int:
    if isinstance(value, bool):
        raise TorchGenerationError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchGenerationError(f"{name} must be an integer") from error
    if normalized < 0:
        raise TorchGenerationError(f"{name} must be non-negative")
    return normalized
