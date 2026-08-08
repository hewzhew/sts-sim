"""Compact opaque-artifact session for one bounded multi-root update."""

from __future__ import annotations

from collections.abc import Sequence
from pathlib import Path

import torch

from .combat_root_artifacts import (
    load_combat_root_source,
    normalize_combat_root_artifact,
    read_combat_root_artifact,
)
from .policy import BehaviorManifestId
from .combat_potion_lane import CombatPotionLaneRootSource
from .torch_behavior import TorchBehaviorPublication
from .torch_combat_batch_generation import (
    BoundedCombatWinBatchGenerationRunner,
    CombatWinBatchGenerationResult,
)
from .torch_combat_owners import create_combat_win_owner_graph
from .torch_combat_session import _positive_integer, _torch_seed
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinBatchSessionConfig,
    TorchCombatSessionError,
)


class CombatWinBatchSession:
    """One live multi-root trainer with explicit behavior publication."""

    def __init__(
        self,
        runner: BoundedCombatWinBatchGenerationRunner,
        *,
        artifact_byte_count: int,
    ) -> None:
        if not isinstance(runner, BoundedCombatWinBatchGenerationRunner):
            raise TorchCombatSessionError(
                "combat batch session requires a batch generation runner"
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
            raise TorchCombatSessionError(
                "combat batch session has no active behavior"
            )
        return active

    def advance(self) -> CombatWinBatchGenerationResult:
        """Collect every declared root, then train and promote at most once."""

        return self.runner.advance()

    def publish_active_behavior(self) -> TorchBehaviorPublication:
        """Durably publish the active frozen scorer without changing policy."""

        return self.runner.controller.publish_active()


class CombatWinBatchSessionFactory:
    """Own one decoded root batch and all shared multi-root training owners."""

    def __init__(
        self,
        root: str | Path,
        bridge: CombatSessionBridge,
        config: CombatWinBatchSessionConfig,
    ) -> None:
        if not isinstance(bridge, CombatSessionBridge):
            raise TorchCombatSessionError(
                "combat batch session bridge must be typed"
            )
        if not isinstance(config, CombatWinBatchSessionConfig):
            raise TorchCombatSessionError(
                "combat batch session config must be typed"
            )
        self.root = Path(root).resolve()
        if self.root.exists() and not self.root.is_dir():
            raise TorchCombatSessionError(
                "combat batch session root is not a directory"
            )
        self.root.mkdir(parents=True, exist_ok=True)
        self.bridge = bridge
        self.config = config

    def new_from_artifact_file(
        self,
        artifact: str | Path,
        *,
        model_seed: int,
        behavior_seeds: Sequence[int],
    ) -> CombatWinBatchSession:
        payload = read_combat_root_artifact(
            artifact,
            max_bytes=self.config.limits.max_artifact_bytes,
        )
        return self.new_from_artifact_bytes(
            payload,
            model_seed=model_seed,
            behavior_seeds=behavior_seeds,
        )

    def new_from_artifact_bytes(
        self,
        payload: bytes | bytearray | memoryview,
        *,
        model_seed: int,
        behavior_seeds: Sequence[int],
    ) -> CombatWinBatchSession:
        self._require_unused_root()
        artifact = normalize_combat_root_artifact(
            payload,
            max_bytes=self.config.limits.max_artifact_bytes,
        )
        normalized_model_seed = _torch_seed(model_seed, "model_seed")
        seeds = tuple(behavior_seeds)
        if len(seeds) != self.config.expected_roots:
            raise TorchCombatSessionError(
                "combat batch session requires one behavior seed per root"
            )
        normalized_seeds = tuple(
            _torch_seed(seed, f"behavior_seeds[{index}]")
            for index, seed in enumerate(seeds)
        )
        if len(set(normalized_seeds)) != len(normalized_seeds):
            raise TorchCombatSessionError(
                "combat batch session requires distinct behavior seeds"
            )
        source = load_combat_root_source(
            self.bridge,
            artifact,
            expected_roots=self.config.expected_roots,
            max_bytes=self.config.limits.max_artifact_bytes,
        )
        source = CombatPotionLaneRootSource(source, self.config.potion_lane)
        return self._new_from_combat_root_source(
            source,
            artifact_byte_count=len(artifact),
            model_seed=normalized_model_seed,
            behavior_seeds=normalized_seeds,
        )

    def _new_from_combat_root_source(
        self,
        source: object,
        *,
        artifact_byte_count: int,
        model_seed: int,
        behavior_seeds: tuple[int, ...],
    ) -> CombatWinBatchSession:
        if not callable(getattr(source, "combat_group", None)):
            raise TorchCombatSessionError(
                "combat-root artifact loader returned an invalid source"
            )
        owners = create_combat_win_owner_graph(
            self.root,
            self.bridge,
            self.config.profile,
            self.config.limits,
            model_seed=model_seed,
            controller_seed=behavior_seeds[0],
        )
        generators = tuple(
            torch.Generator(device="cpu").manual_seed(seed)
            for seed in behavior_seeds
        )
        runner = BoundedCombatWinBatchGenerationRunner(
            source,
            slot_indices=tuple(range(self.config.expected_roots)),
            replicate_count=self.config.replicate_count,
            behavior_generators=generators,
            max_roots=self.config.max_roots,
            limits=self.config.limits.experience,
            trainer=owners.trainer,
            controller=owners.controller,
            shadow_scorer=owners.shadow_scorer,
        )
        return CombatWinBatchSession(
            runner,
            artifact_byte_count=artifact_byte_count,
        )

    def _require_unused_root(self) -> None:
        if any(self.root.iterdir()):
            raise TorchCombatSessionError(
                "new combat batch session requires an unused experiment root"
            )
