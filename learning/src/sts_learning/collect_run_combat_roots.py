"""Capture typed later-combat roots while a frozen policy advances real runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import operator
import time
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol, cast

from .combat_potion_lane import CombatPotionLane
from .driver import (
    BatchEnvironment,
    OnlineBatchDriver,
    RecoveryPlan,
    initialize_population,
)
from .evaluate_run import RunPotionLane, resolve_run_potion_lane
from .published_combat_behavior import (
    PublishedCombatBehavior,
    recover_published_combat_behavior,
)
from .published_run_behavior import (
    PublishedRunBehavior,
    is_run_training_publication,
    recover_published_run_behavior,
)
from .recovery import RecoverySlotSnapshot, TerminalAccountingBatch
from .run_resource_trace import RunPublicContext
from .seeds import SeedPartition, SeedSchedule
from .torch_combat_session_config import CombatSessionBridge, CombatWinSessionLimits
from .torch_session_config import CategoricalSessionBridge


class RunCombatRootCollectionError(RuntimeError):
    """A bounded root collection lost identity, provenance, or a resource bound."""


class _RootExportEnvironment(BatchEnvironment, Protocol):
    def public_run_contexts(self) -> Sequence[object]: ...

    def combat_root_contexts(self) -> Sequence[object]: ...

    def combat_root_artifact_bytes(
        self,
        slot_indices: list[int],
        *,
        max_bytes: int,
    ) -> bytes: ...


@dataclass(frozen=True)
class RunCombatRootCollectionConfig:
    behavior: Path
    output: Path
    root_count: int
    max_batch_steps: int
    wall_ms: int
    behavior_seed: int
    training_seed_start: int
    min_floor: int = 2
    min_usable_potions: int = 1
    potion_lane: RunPotionLane = RunPotionLane.TRAINED
    max_artifact_bytes: int = 16 * 1024 * 1024

    def __post_init__(self) -> None:
        behavior = Path(self.behavior).resolve()
        output = Path(self.output).resolve()
        if not behavior.is_dir():
            raise RunCombatRootCollectionError(
                "root collection behavior is not a directory"
            )
        if output.exists():
            raise RunCombatRootCollectionError(
                "root collection output must be a fresh file"
            )
        if output == behavior or behavior in output.parents:
            raise RunCombatRootCollectionError(
                "root collection output must stay outside the behavior directory"
            )
        if not output.parent.is_dir():
            raise RunCombatRootCollectionError(
                "root collection output parent does not exist"
            )
        if not isinstance(self.potion_lane, RunPotionLane):
            raise RunCombatRootCollectionError(
                "root collection potion lane must be typed"
            )
        object.__setattr__(self, "behavior", behavior)
        object.__setattr__(self, "output", output)
        root_count = _positive(self.root_count, "root_count")
        if root_count > 64:
            raise RunCombatRootCollectionError("root_count must be at most 64")
        object.__setattr__(self, "root_count", root_count)
        for name in ("max_batch_steps", "wall_ms", "max_artifact_bytes"):
            object.__setattr__(self, name, _positive(getattr(self, name), name))
        object.__setattr__(self, "min_floor", _nonnegative(self.min_floor, "min_floor"))
        object.__setattr__(
            self,
            "min_usable_potions",
            _positive(self.min_usable_potions, "min_usable_potions"),
        )
        object.__setattr__(
            self,
            "behavior_seed",
            _seed(self.behavior_seed, "behavior_seed"),
        )
        object.__setattr__(
            self,
            "training_seed_start",
            _seed(self.training_seed_start, "training_seed_start"),
        )


@dataclass(frozen=True)
class CapturedRunCombatRoot:
    seed: int
    act: int
    floor: int
    hp: int
    max_hp: int
    potion_ids: tuple[str | None, ...]
    filled_potion_count: int
    usable_potion_count: int


class _RootCaptureSink:
    def __init__(self, config: RunCombatRootCollectionConfig) -> None:
        self.config = config
        self.payloads: list[bytes] = []
        self.roots: list[CapturedRunCombatRoot] = []
        self._captured_seeds: set[int] = set()

    @property
    def complete(self) -> bool:
        return len(self.roots) >= self.config.root_count

    def observe(self, env: _RootExportEnvironment) -> None:
        if self.complete:
            return
        public = {
            context.slot_index: context
            for context in (
                RunPublicContext.from_bridge_row(row)
                for row in env.public_run_contexts()
            )
        }
        for row in env.combat_root_contexts():
            if self.complete:
                break
            slot, root = _root_context_row(row)
            context = public.get(slot)
            if context is None:
                raise RunCombatRootCollectionError(
                    "combat root has no aligned public run context"
                )
            if not context.is_combat or context.is_terminal:
                raise RunCombatRootCollectionError(
                    "combat root public context is not an active combat"
                )
            if context.seed in self._captured_seeds:
                continue
            floor = _root_integer(root, "floor", minimum=0)
            act = _root_integer(root, "act", minimum=0)
            hp = _root_integer(root, "hp", minimum=0)
            max_hp = _root_integer(root, "max_hp", minimum=1)
            filled = _root_integer(root, "filled_potion_count", minimum=0)
            usable = _root_integer(root, "usable_potion_count", minimum=0)
            if (act, floor, hp, max_hp) != (
                context.act,
                context.floor,
                context.hp,
                context.max_hp,
            ):
                raise RunCombatRootCollectionError(
                    "combat root and public run context disagree: "
                    f"slot={slot} seed={context.seed} "
                    f"root=(act={act},floor={floor},hp={hp},max_hp={max_hp}) "
                    f"public=(act={context.act},floor={context.floor},"
                    f"hp={context.hp},max_hp={context.max_hp})"
                )
            if filled != sum(potion is not None for potion in context.potion_ids):
                raise RunCombatRootCollectionError(
                    "combat root and public potion inventory disagree"
                )
            if usable > filled:
                raise RunCombatRootCollectionError(
                    "usable potion count exceeds filled inventory"
                )
            if floor < self.config.min_floor:
                continue
            if usable < self.config.min_usable_potions:
                continue
            payload = bytes(
                env.combat_root_artifact_bytes(
                    [slot],
                    max_bytes=self.config.max_artifact_bytes,
                )
            )
            if not payload or len(payload) > self.config.max_artifact_bytes:
                raise RunCombatRootCollectionError(
                    "captured combat root payload violates its byte bound"
                )
            self.payloads.append(payload)
            self.roots.append(
                CapturedRunCombatRoot(
                    seed=context.seed,
                    act=act,
                    floor=floor,
                    hp=hp,
                    max_hp=max_hp,
                    potion_ids=context.potion_ids,
                    filled_potion_count=filled,
                    usable_potion_count=usable,
                )
            )
            self._captured_seeds.add(context.seed)


class _CapturingEnvironment:
    def __init__(
        self,
        env: _RootExportEnvironment,
        sink: _RootCaptureSink,
    ) -> None:
        self.env = env
        self.sink = sink

    @property
    def slot_count(self) -> int:
        return self.env.slot_count

    @property
    def terminal_count(self) -> int:
        return self.env.terminal_count

    @property
    def ready(self) -> bool:
        return self.env.ready

    def decision_batch(self, *, semantic: bool = False) -> Mapping[str, object]:
        self.sink.observe(self.env)
        return self.env.decision_batch(semantic=semantic)

    def choose(self, ordinals: list[int]) -> None:
        self.env.choose(ordinals)

    def step(self) -> Mapping[str, object]:
        return self.env.step()

    def checkpoint_slots(self, slot_indices: list[int]):
        return self.env.checkpoint_slots(slot_indices)

    def restore_slots(self, slot_indices: list[int], checkpoints: object) -> None:
        self.env.restore_slots(slot_indices, checkpoints)

    def reset_slots(self, slot_indices: list[int], seeds: list[int]) -> None:
        self.env.reset_slots(slot_indices, seeds)

    def reset_slots_checkpointed(
        self,
        slot_indices: list[int],
        seeds: list[int],
    ):
        return self.env.reset_slots_checkpointed(slot_indices, seeds)

    def checkpoint_bytes(self, *, max_bytes: int) -> bytes:
        return self.env.checkpoint_bytes(max_bytes=max_bytes)


class _NoRecovery:
    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        return RecoveryPlan()


def run_run_combat_root_collection(
    config: RunCombatRootCollectionConfig,
    *,
    combat_bridge: CombatSessionBridge | None = None,
    run_bridge: CategoricalSessionBridge | None = None,
    artifact_merger: Callable[..., bytes] | None = None,
) -> dict[str, object]:
    """Capture one qualifying exact combat root from each distinct run seed."""

    if not isinstance(config, RunCombatRootCollectionConfig):
        raise RunCombatRootCollectionError("root collection config must be typed")
    active_combat_bridge = (
        combat_bridge if combat_bridge is not None else CombatSessionBridge.installed()
    )
    active_run_bridge = (
        run_bridge if run_bridge is not None else CategoricalSessionBridge.installed()
    )
    if not isinstance(active_combat_bridge, CombatSessionBridge):
        raise RunCombatRootCollectionError("combat bridge must be typed")
    if not isinstance(active_run_bridge, CategoricalSessionBridge):
        raise RunCombatRootCollectionError("run bridge must be typed")
    if active_combat_bridge.semantic_schema != active_run_bridge.semantic_schema:
        raise RunCombatRootCollectionError(
            "combat behavior and run environment semantic schemas differ"
        )

    recovered: PublishedCombatBehavior | PublishedRunBehavior
    if is_run_training_publication(config.behavior):
        recovered = recover_published_run_behavior(
            config.behavior,
            active_run_bridge,
            (config.behavior_seed,),
        )
        behavior_kind = "run"
    else:
        recovered = recover_published_combat_behavior(
            config.behavior,
            active_combat_bridge,
            CombatWinSessionLimits(),
            (config.behavior_seed,),
        )
        behavior_kind = "combat"
    potion_lane = resolve_run_potion_lane(config.potion_lane, recovered)
    environment_factory = (
        active_run_bridge.environment
        if potion_lane is CombatPotionLane.ALL
        else active_run_bridge.environment_without_combat_potions
    )
    merger = artifact_merger or getattr(
        active_run_bridge.environment,
        "merge_combat_root_artifact_bytes",
        None,
    )
    if not callable(merger):
        raise RunCombatRootCollectionError(
            "run bridge does not provide opaque combat-root merging"
        )

    sink = _RootCaptureSink(config)

    def capturing_factory(seeds: list[int]) -> _CapturingEnvironment:
        env = environment_factory(seeds)
        for name in (
            "public_run_contexts",
            "combat_root_contexts",
            "combat_root_artifact_bytes",
        ):
            if not callable(getattr(env, name, None)):
                raise RunCombatRootCollectionError(
                    f"run environment lacks {name}()"
                )
        return _CapturingEnvironment(cast(_RootExportEnvironment, env), sink)

    schedule_start = SeedSchedule(
        SeedPartition.TRAINING,
        next_candidate=config.training_seed_start,
    )
    population = initialize_population(
        capturing_factory,
        slot_count=1,
        schedule=schedule_start,
        max_recoveries_per_episode=0,
    )
    driver = OnlineBatchDriver(
        population,
        policy=recovered.policies[0],
        curriculum=_NoRecovery(),
    )
    started = time.perf_counter()
    batch_steps = 0
    terminal_attempts = 0
    while not sink.complete and batch_steps < config.max_batch_steps:
        if (time.perf_counter() - started) * 1000 >= config.wall_ms:
            break
        result = driver.advance()
        batch_steps += 1
        terminal_attempts += len(result.attempts)
    elapsed = time.perf_counter() - started
    if not sink.complete:
        raise RunCombatRootCollectionError(
            "root collection did not reach its target within the step/deadline bounds: "
            f"roots={len(sink.roots)}/{config.root_count} "
            f"batch_steps={batch_steps}/{config.max_batch_steps}"
        )

    payload = bytes(merger(sink.payloads, max_bytes=config.max_artifact_bytes))
    if not payload or len(payload) > config.max_artifact_bytes:
        raise RunCombatRootCollectionError(
            "merged combat-root artifact violates its byte bound"
        )
    with config.output.open("xb") as destination:
        destination.write(payload)

    summary: dict[str, object] = {
        "schema": "sts-learning-run-combat-root-collection-v1",
        "behavior": str(config.behavior),
        "behavior_training_kind": behavior_kind,
        "behavior_manifest_id": recovered.manifest_id.digest.hex(),
        "behavior_checkpoint_id": recovered.checkpoint_id.digest.hex(),
        "behavior_seed": config.behavior_seed,
        "run_potion_lane": potion_lane.value,
        "training_seed_start": config.training_seed_start,
        "training_seed_end": driver.schedule.next_candidate,
        "min_floor": config.min_floor,
        "min_usable_potions": config.min_usable_potions,
        "root_count": len(sink.roots),
        "terminal_attempts": terminal_attempts,
        "batch_steps": batch_steps,
        "elapsed_seconds": elapsed,
        "artifact": str(config.output),
        "artifact_bytes": len(payload),
        "artifact_sha256": hashlib.sha256(payload).hexdigest(),
        "roots": tuple(
            {
                "seed": root.seed,
                "act": root.act,
                "floor": root.floor,
                "hp": root.hp,
                "max_hp": root.max_hp,
                "potion_ids": root.potion_ids,
                "filled_potion_count": root.filled_potion_count,
                "usable_potion_count": root.usable_potion_count,
            }
            for root in sink.roots
        ),
    }
    print(json.dumps(summary, separators=(",", ":"), sort_keys=True), flush=True)
    return summary


def _root_context_row(row: object) -> tuple[int, object]:
    if not isinstance(row, Sequence) or isinstance(row, (str, bytes)):
        raise RunCombatRootCollectionError("combat root context row must be a pair")
    if len(row) != 2:
        raise RunCombatRootCollectionError(
            "combat root context row must contain two values"
        )
    return _nonnegative(row[0], "combat root slot"), row[1]


def _root_integer(root: object, name: str, *, minimum: int) -> int:
    try:
        value = getattr(root, name)
    except AttributeError as error:
        raise RunCombatRootCollectionError(
            f"combat root context lacks {name}"
        ) from error
    normalized = _nonnegative(value, name)
    if normalized < minimum:
        raise RunCombatRootCollectionError(f"{name} must be at least {minimum}")
    return normalized


def _positive(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized == 0:
        raise RunCombatRootCollectionError(f"{name} must be positive")
    return normalized


def _nonnegative(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise RunCombatRootCollectionError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise RunCombatRootCollectionError(f"{name} must be an integer") from error
    if normalized < 0:
        raise RunCombatRootCollectionError(f"{name} must be non-negative")
    return normalized


def _seed(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized >= 1 << 63:
        raise RunCombatRootCollectionError(f"{name} must be below 2^63")
    return normalized


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Capture exact later-combat roots with typed potion inventory while "
            "a frozen behavior advances training-partition runs."
        )
    )
    parser.add_argument("--behavior", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--roots", type=int, required=True)
    parser.add_argument("--max-batch-steps", type=int, default=4096)
    parser.add_argument("--wall-ms", type=int, default=60_000)
    parser.add_argument("--behavior-seed", type=int, default=10_000)
    parser.add_argument("--training-seed-start", type=int, default=10_000_000)
    parser.add_argument("--min-floor", type=int, default=2)
    parser.add_argument("--min-usable-potions", type=int, default=1)
    parser.add_argument(
        "--potion-lane",
        choices=tuple(lane.value for lane in RunPotionLane),
        default=RunPotionLane.TRAINED.value,
    )
    parser.add_argument("--max-artifact-bytes", type=int, default=16 * 1024 * 1024)
    return parser


def main() -> int:
    arguments = _parser().parse_args()
    run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=arguments.behavior,
            output=arguments.output,
            root_count=arguments.roots,
            max_batch_steps=arguments.max_batch_steps,
            wall_ms=arguments.wall_ms,
            behavior_seed=arguments.behavior_seed,
            training_seed_start=arguments.training_seed_start,
            min_floor=arguments.min_floor,
            min_usable_potions=arguments.min_usable_potions,
            potion_lane=RunPotionLane(arguments.potion_lane),
            max_artifact_bytes=arguments.max_artifact_bytes,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
