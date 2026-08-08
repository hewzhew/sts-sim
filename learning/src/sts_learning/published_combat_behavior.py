"""Exact recovery of a completed combat-training publication."""

from __future__ import annotations

import json
from collections.abc import Sequence
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Mapping

import torch

from .combat_objective import CombatWinObjectiveConfig
from .manifest_catalog import BoundedBehaviorManifestCatalog
from .manifests import (
    BehaviorManifestRegistry,
    ManifestArtifactId,
    ManifestArtifactKind,
)
from .policy import BehaviorManifestId
from .torch_behavior import (
    CheckpointedCategoricalTorchPolicy,
    FrozenCategoricalTorchPolicy,
)
from .torch_checkpoints import BoundedTorchCheckpointStore
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinSessionLimits,
    CombatWinSessionProfile,
)
from .torch_policy import RaggedCandidateScorer
from .torch_provenance import combat_win_training_manifest_template
from .train_combat import COMBAT_TRAINING_SCHEMA


_MAX_TRAINING_JOURNAL_BYTES = 16 * 1024 * 1024


class PublishedCombatBehaviorError(RuntimeError):
    """A durable combat behavior is incomplete or has conflicting provenance."""


@dataclass(frozen=True)
class PublishedCombatBehavior:
    """One verified frozen policy fork per caller-provided RNG stream."""

    manifest_id: BehaviorManifestId
    checkpoint_id: ManifestArtifactId
    training_step: int
    training_root_count: int
    policies: tuple[FrozenCategoricalTorchPolicy, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.manifest_id, BehaviorManifestId):
            raise PublishedCombatBehaviorError(
                "published combat behavior requires a typed manifest id"
            )
        if not isinstance(self.checkpoint_id, ManifestArtifactId):
            raise PublishedCombatBehaviorError(
                "published combat behavior requires a typed checkpoint id"
            )
        if self.checkpoint_id.kind is not ManifestArtifactKind.MODEL_CHECKPOINT:
            raise PublishedCombatBehaviorError(
                "published combat behavior checkpoint has the wrong kind"
            )
        object.__setattr__(
            self,
            "training_step",
            _nonnegative(self.training_step, "training_step"),
        )
        object.__setattr__(
            self,
            "training_root_count",
            _positive(self.training_root_count, "training_root_count"),
        )
        policies = tuple(self.policies)
        if not policies or not all(
            isinstance(policy, FrozenCategoricalTorchPolicy)
            for policy in policies
        ):
            raise PublishedCombatBehaviorError(
                "published combat behavior requires frozen policy forks"
            )
        if any(
            policy.behavior_manifest_id != self.manifest_id
            for policy in policies
        ):
            raise PublishedCombatBehaviorError(
                "published combat policy fork changed manifest identity"
            )
        if len({id(policy.generator) for policy in policies}) != len(policies):
            raise PublishedCombatBehaviorError(
                "published combat policy forks must use independent RNG streams"
            )
        object.__setattr__(self, "policies", policies)


def recover_published_combat_behavior(
    behavior_root: str | Path,
    bridge: CombatSessionBridge,
    limits: CombatWinSessionLimits,
    behavior_seeds: Sequence[int],
) -> PublishedCombatBehavior:
    """Verify complete provenance and materialize one immutable scorer."""

    root = Path(behavior_root).resolve()
    if not root.is_dir():
        raise PublishedCombatBehaviorError(
            "published combat behavior is not a directory"
        )
    if not isinstance(bridge, CombatSessionBridge):
        raise PublishedCombatBehaviorError(
            "published combat behavior recovery requires a typed bridge"
        )
    if not isinstance(limits, CombatWinSessionLimits):
        raise PublishedCombatBehaviorError(
            "published combat behavior recovery requires typed limits"
        )
    seeds = tuple(
        _seed(seed, f"behavior_seeds[{index}]")
        for index, seed in enumerate(behavior_seeds)
    )
    if not seeds or len(set(seeds)) != len(seeds):
        raise PublishedCombatBehaviorError(
            "published combat behavior requires distinct RNG seeds"
        )

    configuration, completed = _training_boundary_records(
        root / "training.jsonl"
    )
    training_root_count = _positive(
        configuration.get("root_count"),
        "training root_count",
    )
    profile = replace(
        CombatWinSessionProfile(),
        objective=CombatWinObjectiveConfig(
            groups_per_update=training_root_count
        ),
    )
    if configuration.get("all_win_axis") != profile.objective.all_win_axis.name:
        raise PublishedCombatBehaviorError(
            "training journal has an unsupported all-win objective"
        )

    store = BoundedTorchCheckpointStore(
        root / "behavior-checkpoints",
        limits.checkpoint_store,
    )
    catalog = BoundedBehaviorManifestCatalog(
        root / "behavior-manifests",
        limits.manifest_catalog,
    )
    if store.snapshot.checkpoints != 1 or catalog.snapshot.manifests != 1:
        raise PublishedCombatBehaviorError(
            "published behavior must contain exactly one checkpoint and manifest"
        )
    manifest_id = catalog.manifest_ids[0]
    manifest = catalog.resolve(manifest_id)
    final_manifest = _required_string(completed, "final_manifest_id")
    final_checkpoint = _required_string(completed, "final_checkpoint_id")
    training_step = _nonnegative(
        completed.get("optimizer_steps"),
        "optimizer_steps",
    )
    if manifest_id.digest.hex() != final_manifest:
        raise PublishedCombatBehaviorError(
            "training journal and durable manifest identity disagree"
        )
    if manifest.model_checkpoint.digest.hex() != final_checkpoint:
        raise PublishedCombatBehaviorError(
            "training journal and durable checkpoint identity disagree"
        )
    if manifest.training_step != training_step:
        raise PublishedCombatBehaviorError(
            "training journal and manifest training step disagree"
        )
    expected = combat_win_training_manifest_template(
        bridge.semantic_schema,
        profile.scorer,
        profile.behavior,
        profile.optimizer,
        profile.objective,
        device_type=profile.device_type,
    ).bind(
        manifest.model_checkpoint,
        training_step=manifest.training_step,
    )
    if manifest != expected:
        raise PublishedCombatBehaviorError(
            "published behavior does not match the maintained combat profile"
        )

    def scorer_factory() -> RaggedCandidateScorer:
        return RaggedCandidateScorer.from_bridge_schema(
            bridge.semantic_schema,
            profile.scorer,
        ).to(profile.device_type)

    generators = tuple(
        torch.Generator(device="cpu").manual_seed(seed) for seed in seeds
    )
    registry = BehaviorManifestRegistry(capacity=1)
    first = CheckpointedCategoricalTorchPolicy.recover(
        manifest_id,
        store,
        catalog,
        registry,
        scorer_factory,
        profile.behavior,
        generators[0],
    )
    return PublishedCombatBehavior(
        manifest_id=manifest_id,
        checkpoint_id=manifest.model_checkpoint,
        training_step=training_step,
        training_root_count=training_root_count,
        policies=(first,) + tuple(
            first.fork(generator) for generator in generators[1:]
        ),
    )


def _training_boundary_records(
    journal: Path,
) -> tuple[dict[str, object], dict[str, object]]:
    if not journal.is_file():
        raise PublishedCombatBehaviorError(
            "published behavior is missing training.jsonl"
        )
    try:
        journal_bytes = journal.stat().st_size
    except OSError as error:
        raise PublishedCombatBehaviorError(
            "training journal could not be inspected"
        ) from error
    if journal_bytes > _MAX_TRAINING_JOURNAL_BYTES:
        raise PublishedCombatBehaviorError(
            "training journal exceeds its recovery byte limit"
        )
    first_line: tuple[int, str] | None = None
    last_line: tuple[int, str] | None = None
    try:
        with journal.open("r", encoding="utf-8") as source:
            for line_number, line in enumerate(source, start=1):
                if not line.strip():
                    continue
                if first_line is None:
                    first_line = (line_number, line)
                last_line = (line_number, line)
    except OSError as error:
        raise PublishedCombatBehaviorError(
            "training journal could not be read"
        ) from error
    if first_line is None or last_line is None:
        raise PublishedCombatBehaviorError("training journal is empty")
    first = _journal_record(*first_line)
    last = _journal_record(*last_line)
    if (
        first.get("schema") != COMBAT_TRAINING_SCHEMA
        or first.get("kind") != "configuration"
        or last.get("schema") != COMBAT_TRAINING_SCHEMA
        or last.get("kind") != "completed"
    ):
        raise PublishedCombatBehaviorError(
            "training journal lacks exact configuration/completion boundaries"
        )
    return first, last


def _journal_record(line_number: int, line: str) -> dict[str, object]:
    try:
        value = json.loads(line)
    except json.JSONDecodeError as error:
        raise PublishedCombatBehaviorError(
            f"training journal line {line_number} is invalid JSON"
        ) from error
    if not isinstance(value, dict):
        raise PublishedCombatBehaviorError(
            f"training journal line {line_number} is not an object"
        )
    return value


def _required_string(value: Mapping[str, object], name: str) -> str:
    field = value.get(name)
    if not isinstance(field, str) or not field:
        raise PublishedCombatBehaviorError(f"{name} must be a non-empty string")
    return field


def _positive(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized == 0:
        raise PublishedCombatBehaviorError(
            f"{name} must be a positive integer"
        )
    return normalized


def _nonnegative(value: object, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise PublishedCombatBehaviorError(
            f"{name} must be a non-negative integer"
        )
    return value


def _seed(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized >= 1 << 63:
        raise PublishedCombatBehaviorError(
            f"{name} must be below 2^63"
        )
    return normalized
