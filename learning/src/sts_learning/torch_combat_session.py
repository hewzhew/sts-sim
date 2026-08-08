"""Compact artifact-to-runner boundary for same-root combat win learning."""

from __future__ import annotations

import operator
from pathlib import Path

import torch

from .manifest_catalog import BoundedBehaviorManifestCatalog
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId
from .torch_behavior import (
    CategoricalTorchBehaviorController,
    TorchBehaviorPublication,
    TorchBehaviorPublisher,
)
from .torch_checkpoints import BoundedTorchCheckpointStore
from .torch_combat_generation import (
    BoundedCombatWinGenerationRunner,
    CombatWinGenerationResult,
)
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinSessionConfig,
    TorchCombatSessionError,
)
from .torch_combat_training import SynchronousCombatWinTrainer
from .torch_policy import RaggedCandidateScorer
from .torch_provenance import combat_win_training_manifest_template


class CombatWinSession:
    """One live fixed-root runner with explicit behavior publication."""

    def __init__(
        self,
        runner: BoundedCombatWinGenerationRunner,
        *,
        artifact_byte_count: int,
    ) -> None:
        if not isinstance(runner, BoundedCombatWinGenerationRunner):
            raise TorchCombatSessionError(
                "combat session requires a combat generation runner"
            )
        self.runner = runner
        self.artifact_byte_count = _positive_integer(
            artifact_byte_count,
            "artifact_byte_count",
        )

    @property
    def active_behavior_manifest_id(self) -> BehaviorManifestId:
        active = self.runner.controller.snapshot.active_manifest_id
        if active is None:
            raise TorchCombatSessionError("combat session has no active behavior")
        return active

    def advance(self) -> CombatWinGenerationResult:
        """Run at most one complete group and its immediate live promotion."""

        return self.runner.advance()

    def publish_active_behavior(self) -> TorchBehaviorPublication:
        """Durably publish the active frozen scorer without changing policy."""

        return self.runner.controller.publish_active()


class CombatWinSessionFactory:
    """Own repetitive combat-root, scorer, optimizer, and controller wiring."""

    def __init__(
        self,
        root: str | Path,
        bridge: CombatSessionBridge,
        config: CombatWinSessionConfig,
    ) -> None:
        if not isinstance(bridge, CombatSessionBridge):
            raise TorchCombatSessionError("combat session bridge must be typed")
        if not isinstance(config, CombatWinSessionConfig):
            raise TorchCombatSessionError("combat session config must be typed")
        self.root = Path(root).resolve()
        if self.root.exists() and not self.root.is_dir():
            raise TorchCombatSessionError(
                "combat session root is not a directory"
            )
        self.root.mkdir(parents=True, exist_ok=True)
        self.bridge = bridge
        self.config = config
        profile = config.profile
        self.template = combat_win_training_manifest_template(
            bridge.semantic_schema,
            profile.scorer,
            profile.behavior,
            profile.optimizer,
            profile.objective,
            device_type=profile.device_type,
        )

    def new_from_artifact_file(
        self,
        artifact: str | Path,
        *,
        model_seed: int,
        behavior_seed: int,
    ) -> CombatWinSession:
        """Read one bounded opaque artifact and create generation zero."""

        path = Path(artifact).resolve()
        if not path.is_file():
            raise TorchCombatSessionError(
                "combat-root artifact is not a file"
            )
        size = path.stat().st_size
        if size <= 0:
            raise TorchCombatSessionError("combat-root artifact is empty")
        if size > self.config.limits.max_artifact_bytes:
            raise TorchCombatSessionError(
                "combat-root artifact exceeds its byte limit"
            )
        try:
            payload = path.read_bytes()
        except OSError as error:
            raise TorchCombatSessionError(
                "combat-root artifact could not be read"
            ) from error
        return self.new_from_artifact_bytes(
            payload,
            model_seed=model_seed,
            behavior_seed=behavior_seed,
        )

    def new_from_artifact_bytes(
        self,
        payload: bytes | bytearray | memoryview,
        *,
        model_seed: int,
        behavior_seed: int,
    ) -> CombatWinSession:
        """Import exact roots and create one fully wired in-process session."""

        self._require_unused_root()
        artifact = _artifact_bytes(
            payload,
            max_bytes=self.config.limits.max_artifact_bytes,
        )
        model_seed = _torch_seed(model_seed, "model_seed")
        behavior_seed = _torch_seed(behavior_seed, "behavior_seed")
        try:
            source = self.bridge.combat_roots_from_artifact(
                artifact,
                expected_roots=self.config.expected_roots,
                max_bytes=self.config.limits.max_artifact_bytes,
            )
        except Exception as error:
            raise TorchCombatSessionError(
                "combat-root artifact import failed"
            ) from error
        if not callable(getattr(source, "combat_group", None)):
            raise TorchCombatSessionError(
                "combat-root artifact loader returned an invalid source"
            )

        with torch.random.fork_rng(devices=[]):
            torch.manual_seed(model_seed)
            shadow = self._scorer()
        checkpoint_store, catalog = self._behavior_stores()
        registry = BehaviorManifestRegistry(
            capacity=self.config.limits.owner_capacity
        )
        controller = CategoricalTorchBehaviorController(
            TorchBehaviorPublisher(
                checkpoint_store,
                catalog,
                registry,
                self.template,
            ),
            self._scorer,
            self.config.profile.behavior,
            torch.Generator(device="cpu").manual_seed(behavior_seed),
        )
        optimizer = self.config.profile.optimizer.create(shadow.parameters())
        trainer = SynchronousCombatWinTrainer(
            shadow,
            optimizer,
            registry,
            self.config.limits.concat,
            self.config.profile.behavior,
            self.config.profile.objective,
        )
        controller.promote_live(shadow, training_step=0)
        runner = BoundedCombatWinGenerationRunner(
            source,
            slot_index=self.config.root_slot_index,
            replicate_count=self.config.replicate_count,
            limits=self.config.limits.experience,
            trainer=trainer,
            controller=controller,
            shadow_scorer=shadow,
        )
        return CombatWinSession(
            runner,
            artifact_byte_count=len(artifact),
        )

    def _require_unused_root(self) -> None:
        if any(self.root.iterdir()):
            raise TorchCombatSessionError(
                "new combat session requires an unused experiment root"
            )

    def _scorer(self) -> RaggedCandidateScorer:
        return RaggedCandidateScorer.from_bridge_schema(
            self.bridge.semantic_schema,
            self.config.profile.scorer,
        ).to(self.config.profile.device_type)

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


def _artifact_bytes(
    payload: bytes | bytearray | memoryview,
    *,
    max_bytes: int,
) -> bytes:
    if not isinstance(payload, (bytes, bytearray, memoryview)):
        raise TorchCombatSessionError("combat-root artifact must be bytes-like")
    normalized = bytes(payload)
    if not normalized:
        raise TorchCombatSessionError("combat-root artifact is empty")
    if len(normalized) > max_bytes:
        raise TorchCombatSessionError(
            "combat-root artifact exceeds its byte limit"
        )
    return normalized


def _torch_seed(value: object, name: str) -> int:
    normalized = _nonnegative_integer(value, name)
    if normalized >= 1 << 63:
        raise TorchCombatSessionError(f"{name} must be below 2^63")
    return normalized


def _positive_integer(value: object, name: str) -> int:
    normalized = _nonnegative_integer(value, name)
    if normalized == 0:
        raise TorchCombatSessionError(f"{name} must be positive")
    return normalized


def _nonnegative_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise TorchCombatSessionError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchCombatSessionError(f"{name} must be an integer") from error
    if normalized < 0:
        raise TorchCombatSessionError(f"{name} must be non-negative")
    return normalized
