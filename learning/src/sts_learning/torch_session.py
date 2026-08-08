"""Compact new/restore boundary for one categorical online learning session."""

from __future__ import annotations

import operator
from dataclasses import dataclass
from pathlib import Path

import torch

from .attempts import BoundedAttemptAssembler
from .attempt_batching import BoundedAttemptUpdateBatcher
from .driver import (
    BatchCurriculum,
    OnlineBatchDriver,
    RecoveryPlan,
    initialize_population,
)
from .experience import ExperienceSegmentBuffer
from .manifest_catalog import (
    BoundedBehaviorManifestCatalog,
)
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId
from .resume_store import BoundedResumeStore, ResumeManifestId
from .torch_behavior import (
    CategoricalTorchBehaviorController,
    CheckpointedCategoricalTorchPolicy,
    TorchBehaviorPublisher,
)
from .torch_checkpoints import BoundedTorchCheckpointStore
from .torch_generation import (
    BoundedCategoricalGenerationRunner,
    CategoricalGenerationAdvanceResult,
)
from .torch_policy import RaggedCandidateScorer
from .torch_provenance import categorical_training_manifest_template
from .torch_resume_publication import (
    CategoricalGenerationResumePublisher,
    CategoricalResumePublication,
)
from .torch_resume_restore import (
    CategoricalGenerationResumeRestorer,
    CategoricalResumeRestoreConfig,
    CategoricalResumeRestoreFactories,
)
from .torch_session_config import (
    CategoricalOnlineSessionConfig,
    CategoricalSessionBridge,
    TorchSessionError,
)
from .torch_training import SynchronousPolicyTrainer


class NoRecoveryCurriculum:
    """Explicit zero-recovery baseline curriculum."""

    def plan_recovery(self, accounting, snapshots) -> RecoveryPlan:
        return RecoveryPlan()


@dataclass(frozen=True)
class CategoricalSessionAdvance:
    """One bounded generation attempt and its optional durable resume point."""

    generation: CategoricalGenerationAdvanceResult
    resume: CategoricalResumePublication | None


class CategoricalOnlineSession:
    """One live runner with a compact generation-and-publish operation."""

    def __init__(
        self,
        runner: BoundedCategoricalGenerationRunner,
        resume_publisher: CategoricalGenerationResumePublisher,
    ) -> None:
        if not isinstance(runner, BoundedCategoricalGenerationRunner):
            raise TorchSessionError("session requires a generation runner")
        if not isinstance(resume_publisher, CategoricalGenerationResumePublisher):
            raise TorchSessionError("session requires a resume publisher")
        self.runner = runner
        self.resume_publisher = resume_publisher

    @property
    def active_behavior_manifest_id(self) -> BehaviorManifestId:
        active = self.runner.controller.snapshot.active_manifest_id
        if active is None:
            raise TorchSessionError("session has no active behavior")
        return active

    def publish(self) -> CategoricalResumePublication:
        return self.resume_publisher.publish(self.runner)

    def advance_generation(self, *, max_batch_steps: int) -> CategoricalSessionAdvance:
        generation = self.runner.advance(max_batch_steps=max_batch_steps)
        resume = self.publish() if generation.promoted else None
        return CategoricalSessionAdvance(generation=generation, resume=resume)


class CategoricalOnlineSessionFactory:
    """Own all repetitive new/restore wiring below one experiment root."""

    def __init__(
        self,
        root: str | Path,
        bridge: CategoricalSessionBridge,
        config: CategoricalOnlineSessionConfig,
        curriculum: BatchCurriculum,
    ) -> None:
        if not isinstance(bridge, CategoricalSessionBridge):
            raise TorchSessionError("session bridge must be typed")
        if not isinstance(config, CategoricalOnlineSessionConfig):
            raise TorchSessionError("session config must be typed")
        if not callable(getattr(curriculum, "plan_recovery", None)):
            raise TorchSessionError("session curriculum must plan recovery")
        if (
            isinstance(curriculum, NoRecoveryCurriculum)
            and config.max_recoveries_per_episode != 0
        ):
            raise TorchSessionError(
                "zero-recovery curriculum requires a zero recovery budget"
            )
        self.root = Path(root).resolve()
        if self.root.exists() and not self.root.is_dir():
            raise TorchSessionError("session root is not a directory")
        self.root.mkdir(parents=True, exist_ok=True)
        self.bridge = bridge
        self.config = config
        self.curriculum = curriculum
        profile = config.profile
        self.template = categorical_training_manifest_template(
            bridge.semantic_schema,
            profile.scorer,
            profile.behavior,
            profile.optimizer,
            profile.objective,
            device_type=profile.device_type,
        )

    def new(
        self,
        *,
        model_seed: int,
        behavior_seed: int,
    ) -> CategoricalOnlineSession:
        """Create generation zero only in an unused experiment root."""

        model_seed = _torch_seed(model_seed, "model_seed")
        behavior_seed = _torch_seed(behavior_seed, "behavior_seed")
        if any(self.root.iterdir()):
            raise TorchSessionError("new session requires an unused experiment root")
        checkpoint_store, catalog = self._behavior_stores()
        resume_store = self._resume_store()
        if (
            checkpoint_store.snapshot.checkpoints != 0
            or catalog.snapshot.manifests != 0
            or resume_store.snapshot.components != 0
            or resume_store.snapshot.manifests != 0
        ):
            raise TorchSessionError("new session requires an unused experiment root")

        population = initialize_population(
            self.bridge.environment,
            slot_count=self.config.slot_count,
            schedule=self.config.schedule,
            max_recoveries_per_episode=(
                self.config.max_recoveries_per_episode
            ),
        )
        with torch.random.fork_rng(devices=[]):
            torch.manual_seed(model_seed)
            shadow = self._scorer()
        registry = BehaviorManifestRegistry(
            capacity=self.config.limits.owner_capacity
        )
        controller = self._controller(
            torch.Generator(device="cpu").manual_seed(behavior_seed),
            checkpoint_store,
            catalog,
            registry,
        )
        optimizer = self.config.profile.optimizer.create(shadow.parameters())
        trainer = SynchronousPolicyTrainer(
            shadow,
            optimizer,
            registry,
            self.config.limits.concat,
            self.config.profile.behavior,
            self.config.profile.objective,
        )
        update_batcher = BoundedAttemptUpdateBatcher(
            self.config.profile.objective.attempts_per_update,
            self.config.limits.attempt_updates,
            trainer,
        )
        assembler = BoundedAttemptAssembler(
            self.config.limits.attempts,
            update_batcher,
        )
        controller.publish_and_promote(shadow, training_step=0)
        driver = OnlineBatchDriver(
            population,
            policy=controller,
            curriculum=self.curriculum,
            max_decision_rounds_per_step=(
                self.config.max_decision_rounds_per_step
            ),
            experience_buffer=ExperienceSegmentBuffer(
                self.config.limits.experience
            ),
            experience_sink=assembler,
        )
        runner = BoundedCategoricalGenerationRunner(
            driver,
            assembler,
            update_batcher,
            trainer,
            controller,
            shadow,
            optimizer_steps_per_generation=(
                self.config.profile.optimizer_steps_per_generation
            ),
        )
        return self._session(runner, resume_store)

    def restore(self, manifest_id: ResumeManifestId) -> CategoricalOnlineSession:
        """Reopen stores and rebuild one complete fresh runner."""

        resume_store = self._resume_store()
        restored = CategoricalGenerationResumeRestorer(
            resume_store,
            CategoricalResumeRestoreConfig(
                factories=CategoricalResumeRestoreFactories(
                    environment_from_checkpoint=(
                        self.bridge.environment_from_checkpoint
                    ),
                    checkpoint_bank_from_checkpoint=(
                        self.bridge.checkpoint_bank_from_checkpoint
                    ),
                    shadow_scorer=self._scorer,
                    optimizer=lambda scorer: self.config.profile.optimizer.create(
                        scorer.parameters()
                    ),
                    controller=self._fresh_controller,
                ),
                curriculum=self.curriculum,
                experience_limits=self.config.limits.experience,
                attempt_limits=self.config.limits.attempts,
                attempt_update_limits=self.config.limits.attempt_updates,
                concat_limits=self.config.limits.concat,
                objective=self.config.profile.objective,
                payload_limits=self.config.limits.resume_payloads,
                expected_generator_device_type=(
                    self.config.profile.device_type
                ),
                max_decision_rounds_per_step=(
                    self.config.max_decision_rounds_per_step
                ),
            ),
        ).restore(manifest_id)
        driver_state = restored.boundary.driver
        if driver_state.slot_count != self.config.slot_count:
            raise TorchSessionError(
                "restored slot_count conflicts with the session config"
            )
        if (
            driver_state.max_recoveries_per_episode
            != self.config.max_recoveries_per_episode
        ):
            raise TorchSessionError(
                "restored recovery budget conflicts with the session config"
            )
        if (
            driver_state.schedule.partition is not self.config.schedule.partition
            or driver_state.schedule.spec != self.config.schedule.spec
        ):
            raise TorchSessionError(
                "restored seed partition conflicts with the session config"
            )
        return self._session(restored.runner, resume_store)

    def recover_behavior(
        self,
        manifest_id: BehaviorManifestId,
        *,
        behavior_seed: int,
    ) -> CheckpointedCategoricalTorchPolicy:
        """Materialize one frozen behavior for held-out evaluation."""

        seed = _torch_seed(behavior_seed, "behavior_seed")
        checkpoint_store, catalog = self._behavior_stores()
        return CheckpointedCategoricalTorchPolicy.recover(
            manifest_id,
            checkpoint_store,
            catalog,
            BehaviorManifestRegistry(capacity=1),
            self._scorer,
            self.config.profile.behavior,
            torch.Generator(device="cpu").manual_seed(seed),
        )

    def _scorer(self) -> RaggedCandidateScorer:
        return RaggedCandidateScorer.from_bridge_schema(
            self.bridge.semantic_schema,
            self.config.profile.scorer,
        ).to(self.config.profile.device_type)

    def _fresh_controller(
        self,
        generator: torch.Generator,
    ) -> CategoricalTorchBehaviorController:
        checkpoint_store, catalog = self._behavior_stores()
        registry = BehaviorManifestRegistry(
            capacity=self.config.limits.owner_capacity
        )
        return self._controller(
            generator,
            checkpoint_store,
            catalog,
            registry,
        )

    def _controller(
        self,
        generator: torch.Generator,
        checkpoint_store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
    ) -> CategoricalTorchBehaviorController:
        return CategoricalTorchBehaviorController(
            TorchBehaviorPublisher(
                checkpoint_store,
                catalog,
                registry,
                self.template,
            ),
            self._scorer,
            self.config.profile.behavior,
            generator,
        )

    def _behavior_stores(
        self,
    ) -> tuple[BoundedTorchCheckpointStore, BoundedBehaviorManifestCatalog]:
        limits = self.config.limits
        return (
            BoundedTorchCheckpointStore(
                self.root / "behavior-checkpoints",
                limits.checkpoint_store,
            ),
            BoundedBehaviorManifestCatalog(
                self.root / "behavior-manifests",
                limits.manifest_catalog,
            ),
        )

    def _resume_store(self) -> BoundedResumeStore:
        return BoundedResumeStore(
            self.root / "resume",
            self.config.limits.resume_store,
        )

    def _session(
        self,
        runner: BoundedCategoricalGenerationRunner,
        resume_store: BoundedResumeStore,
    ) -> CategoricalOnlineSession:
        return CategoricalOnlineSession(
            runner,
            CategoricalGenerationResumePublisher(
                resume_store,
                self.config.limits.resume_payloads,
            ),
        )


def _torch_seed(value: object, name: str) -> int:
    normalized = _non_negative_integer(value, name)
    if normalized >= 1 << 63:
        raise TorchSessionError(f"{name} must be below 2^63")
    return normalized


def _non_negative_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise TorchSessionError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchSessionError(f"{name} must be an integer") from error
    if normalized < 0:
        raise TorchSessionError(f"{name} must be non-negative")
    return normalized
