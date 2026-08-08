"""Strict recovery of one completed whole-run training publication."""

from __future__ import annotations

import json
import operator
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, replace
from pathlib import Path

from .combat_potion_lane import CombatPotionLane
from .manifests import BehaviorManifestId, ManifestArtifactId, ManifestArtifactKind
from .run_sampling import RunSamplingMode
from .seeds import SeedPartition, SeedSchedule
from .terminal_returns import (
    OnPolicyObjectiveConfig,
    RunDecisionScope,
    TerminalAdvantageMode,
)
from .torch_behavior import CheckpointedCategoricalTorchPolicy
from .torch_session import CategoricalOnlineSessionFactory, NoRecoveryCurriculum
from .torch_session_config import (
    CategoricalOnlineProfile,
    CategoricalOnlineSessionConfig,
    CategoricalSessionBridge,
    CategoricalSessionLimits,
)


RUN_TRAINING_SCHEMA = "sts-learning-run-training-v2"
_MAX_TRAINING_JOURNAL_BYTES = 16 * 1024 * 1024


class PublishedRunBehaviorError(RuntimeError):
    """A durable whole-run behavior is incomplete or has conflicting provenance."""


@dataclass(frozen=True)
class PublishedRunBehavior:
    """One verified run-trained policy fork per requested RNG stream."""

    manifest_id: BehaviorManifestId
    checkpoint_id: ManifestArtifactId
    training_step: int
    training_potion_lane: CombatPotionLane
    training_sampling_mode: RunSamplingMode
    objective: OnPolicyObjectiveConfig
    policies: tuple[CheckpointedCategoricalTorchPolicy, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.manifest_id, BehaviorManifestId):
            raise PublishedRunBehaviorError("run behavior manifest id must be typed")
        if not isinstance(self.checkpoint_id, ManifestArtifactId):
            raise PublishedRunBehaviorError("run behavior checkpoint id must be typed")
        if self.checkpoint_id.kind is not ManifestArtifactKind.MODEL_CHECKPOINT:
            raise PublishedRunBehaviorError("run behavior checkpoint has the wrong kind")
        _nonnegative(self.training_step, "training_step")
        if not isinstance(self.training_potion_lane, CombatPotionLane):
            raise PublishedRunBehaviorError("run behavior potion lane must be typed")
        if self.training_potion_lane is CombatPotionLane.ROOT_SLOTS:
            raise PublishedRunBehaviorError("whole-run training cannot publish root slots")
        if not isinstance(self.training_sampling_mode, RunSamplingMode):
            raise PublishedRunBehaviorError("run behavior sampling mode must be typed")
        if not isinstance(self.objective, OnPolicyObjectiveConfig):
            raise PublishedRunBehaviorError("run behavior objective must be typed")
        if not self.policies or not all(
            isinstance(policy, CheckpointedCategoricalTorchPolicy)
            for policy in self.policies
        ):
            raise PublishedRunBehaviorError("run behavior policies must be checkpointed")
        if any(
            policy.behavior_manifest_id != self.manifest_id
            for policy in self.policies
        ):
            raise PublishedRunBehaviorError("run behavior policy identity changed")


def recover_published_run_behavior(
    behavior_root: str | Path,
    bridge: CategoricalSessionBridge,
    behavior_seeds: Sequence[int],
) -> PublishedRunBehavior:
    """Verify run-training boundaries and materialize immutable policies."""

    root = Path(behavior_root).resolve()
    if not root.is_dir():
        raise PublishedRunBehaviorError("published run behavior is not a directory")
    if not isinstance(bridge, CategoricalSessionBridge):
        raise PublishedRunBehaviorError("run behavior recovery requires a typed bridge")
    seeds = tuple(
        _seed(seed, f"behavior_seeds[{index}]")
        for index, seed in enumerate(behavior_seeds)
    )
    if not seeds or len(set(seeds)) != len(seeds):
        raise PublishedRunBehaviorError("run behavior requires distinct RNG seeds")

    configuration, completed = _training_boundary_records(root / "training.jsonl")
    objective = OnPolicyObjectiveConfig(
        attempts_per_update=_positive(
            configuration.get("attempts_per_update"),
            "attempts_per_update",
        ),
        advantage_mode=_enum_name(
            TerminalAdvantageMode,
            configuration.get("advantage_mode"),
            "advantage_mode",
        ),
        decision_scope=_enum_name(
            RunDecisionScope,
            configuration.get("decision_scope", "all"),
            "decision_scope",
        ),
    )
    training_step = _nonnegative(
        completed.get("optimizer_steps"),
        "optimizer_steps",
    )
    manifest_id = BehaviorManifestId(
        _digest(
            completed.get("active_behavior_manifest_id"),
            "active_behavior_manifest_id",
        )
    )
    checkpoint_id = ManifestArtifactId(
        ManifestArtifactKind.MODEL_CHECKPOINT,
        _digest(
            completed.get("active_behavior_checkpoint_id"),
            "active_behavior_checkpoint_id",
        ),
    )
    potion_lane = _potion_lane(completed.get("run_potion_lane"))
    if _potion_lane(configuration.get("run_potion_lane")) is not potion_lane:
        raise PublishedRunBehaviorError("run potion lane changed across publication")
    configuration_sampling_mode = _sampling_mode(
        configuration.get(
            "sampling_mode",
            RunSamplingMode.INDEPENDENT_COHORTS.value,
        )
    )
    completed_sampling_mode = _sampling_mode(
        completed.get(
            "sampling_mode",
            RunSamplingMode.INDEPENDENT_COHORTS.value,
        )
    )
    if completed_sampling_mode is not configuration_sampling_mode:
        raise PublishedRunBehaviorError(
            "run sampling mode changed across publication"
        )

    profile = replace(CategoricalOnlineProfile(), objective=objective)
    limits = replace(
        CategoricalSessionLimits(),
        owner_capacity=max(16, _nonnegative(configuration.get("generations"), "generations") + 2),
    )
    session_config = CategoricalOnlineSessionConfig(
        schedule=SeedSchedule(
            SeedPartition.TRAINING,
            next_candidate=_seed(
                configuration.get("training_seed_start"),
                "training_seed_start",
            ),
        ),
        slot_count=_positive(configuration.get("slot_count"), "slot_count"),
        max_recoveries_per_episode=0,
        profile=profile,
        limits=limits,
    )
    factory = CategoricalOnlineSessionFactory(
        root,
        bridge,
        session_config,
        NoRecoveryCurriculum(),
    )
    policies = tuple(
        factory.recover_behavior(manifest_id, behavior_seed=seed)
        for seed in seeds
    )
    manifest = policies[0].binding.manifest
    if manifest.model_checkpoint != checkpoint_id:
        raise PublishedRunBehaviorError(
            "run journal and durable checkpoint identity disagree"
        )
    if manifest.training_step != training_step:
        raise PublishedRunBehaviorError(
            "run journal and durable manifest training step disagree"
        )
    return PublishedRunBehavior(
        manifest_id=manifest_id,
        checkpoint_id=checkpoint_id,
        training_step=training_step,
        training_potion_lane=potion_lane,
        training_sampling_mode=configuration_sampling_mode,
        objective=objective,
        policies=policies,
    )


def is_run_training_publication(behavior_root: str | Path) -> bool:
    """Classify only the first bounded journal record, without fallback guessing."""

    root = Path(behavior_root).resolve()
    journal = root / "training.jsonl"
    if not journal.is_file() or journal.stat().st_size > _MAX_TRAINING_JOURNAL_BYTES:
        return False
    try:
        with journal.open("r", encoding="utf-8") as source:
            for line in source:
                if line.strip():
                    value = json.loads(line)
                    return (
                        isinstance(value, dict)
                        and value.get("schema") == RUN_TRAINING_SCHEMA
                        and value.get("kind") == "configuration"
                    )
    except (OSError, json.JSONDecodeError):
        return False
    return False


def _training_boundary_records(
    journal: Path,
) -> tuple[dict[str, object], dict[str, object]]:
    try:
        size = journal.stat().st_size
    except OSError as error:
        raise PublishedRunBehaviorError("run training journal is unavailable") from error
    if size <= 0 or size > _MAX_TRAINING_JOURNAL_BYTES:
        raise PublishedRunBehaviorError("run training journal size is invalid")
    first: tuple[int, str] | None = None
    last: tuple[int, str] | None = None
    try:
        with journal.open("r", encoding="utf-8") as source:
            for line_number, line in enumerate(source, start=1):
                if not line.strip():
                    continue
                if first is None:
                    first = (line_number, line)
                last = (line_number, line)
    except OSError as error:
        raise PublishedRunBehaviorError("run training journal could not be read") from error
    if first is None or last is None:
        raise PublishedRunBehaviorError("run training journal is empty")
    configuration = _journal_record(*first)
    completed = _journal_record(*last)
    if (
        configuration.get("schema") != RUN_TRAINING_SCHEMA
        or configuration.get("kind") != "configuration"
        or completed.get("schema") != RUN_TRAINING_SCHEMA
        or completed.get("kind") != "completed"
    ):
        raise PublishedRunBehaviorError(
            "run training journal lacks exact configuration/completion boundaries"
        )
    return configuration, completed


def _journal_record(line_number: int, line: str) -> dict[str, object]:
    try:
        value = json.loads(line)
    except json.JSONDecodeError as error:
        raise PublishedRunBehaviorError(
            f"run training journal line {line_number} is invalid JSON"
        ) from error
    if not isinstance(value, dict):
        raise PublishedRunBehaviorError(
            f"run training journal line {line_number} is not an object"
        )
    return value


def _enum_name(enum_type, value: object, name: str):
    if not isinstance(value, str):
        raise PublishedRunBehaviorError(f"{name} must be text")
    try:
        return enum_type[value.upper()]
    except KeyError as error:
        raise PublishedRunBehaviorError(f"{name} is unsupported") from error


def _potion_lane(value: object) -> CombatPotionLane:
    try:
        lane = CombatPotionLane(value)
    except (TypeError, ValueError) as error:
        raise PublishedRunBehaviorError("run_potion_lane is unsupported") from error
    if lane is CombatPotionLane.ROOT_SLOTS:
        raise PublishedRunBehaviorError("whole-run publication cannot use root slots")
    return lane


def _sampling_mode(value: object) -> RunSamplingMode:
    try:
        return RunSamplingMode(value)
    except (TypeError, ValueError) as error:
        raise PublishedRunBehaviorError(
            "run sampling mode is unsupported"
        ) from error


def _digest(value: object, name: str) -> bytes:
    if not isinstance(value, str) or len(value) != 64 or value != value.lower():
        raise PublishedRunBehaviorError(f"{name} must be a lowercase SHA-256 digest")
    try:
        return bytes.fromhex(value)
    except ValueError as error:
        raise PublishedRunBehaviorError(f"{name} must be a lowercase SHA-256 digest") from error


def _positive(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized == 0:
        raise PublishedRunBehaviorError(f"{name} must be positive")
    return normalized


def _nonnegative(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise PublishedRunBehaviorError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise PublishedRunBehaviorError(f"{name} must be an integer") from error
    if normalized < 0:
        raise PublishedRunBehaviorError(f"{name} must be non-negative")
    return normalized


def _seed(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized >= 1 << 63:
        raise PublishedRunBehaviorError(f"{name} must be below 2^63")
    return normalized
