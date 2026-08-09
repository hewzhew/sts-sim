"""Bounded in-memory strategic demonstrations from the production owner."""

from __future__ import annotations

import operator
import time
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

import numpy as np

from .combat_potion_lane import CombatPotionLane
from .decision_progress import BridgeDecisionProgressProvider, DecisionRunProgress
from .evaluate_run import RunPotionLane, resolve_run_potion_lane
from .published_combat_behavior import (
    recover_compatible_combat_scorer,
    recover_published_combat_behavior,
)
from .semantic_batch import select_semantic_decision_rows
from .torch_combat_session_config import CombatSessionBridge, CombatWinSessionLimits
from .torch_policy import GreedyTorchPolicy
from .torch_session_config import CategoricalSessionBridge


class StrategicDemonstrationError(RuntimeError):
    """A production demonstration corpus could not be collected faithfully."""


class CombatAnchorMode(Enum):
    """How the collector obtains immutable combat scorer weights."""

    STRICT_PUBLICATION = "strict_publication"
    COMPATIBLE_WEIGHT_IMPORT = "compatible_weight_import"


@dataclass(frozen=True)
class StrategicDemonstrationConfig:
    behavior: Path
    ascension_level: int
    training_seed_start: int
    run_count: int
    slot_count: int = 16
    behavior_seed: int = 200_000
    max_batch_steps_per_cohort: int = 4_096
    max_decision_rounds_per_cohort: int = 32_768
    max_rows: int = 4_096
    max_array_bytes: int = 64 * 1024 * 1024
    wall_ms: int = 60_000
    potion_lane: RunPotionLane = RunPotionLane.TRAINED
    combat_anchor_mode: CombatAnchorMode = CombatAnchorMode.STRICT_PUBLICATION

    def __post_init__(self) -> None:
        behavior = Path(self.behavior).resolve()
        if not behavior.is_dir():
            raise StrategicDemonstrationError(
                "demonstration behavior must be a published directory"
            )
        object.__setattr__(self, "behavior", behavior)
        for name in (
            "run_count",
            "slot_count",
            "max_batch_steps_per_cohort",
            "max_decision_rounds_per_cohort",
            "max_rows",
            "max_array_bytes",
            "wall_ms",
        ):
            object.__setattr__(self, name, _positive(getattr(self, name), name))
        if self.slot_count > self.run_count:
            object.__setattr__(self, "slot_count", self.run_count)
        ascension = _nonnegative(self.ascension_level, "ascension_level")
        if ascension > 20:
            raise StrategicDemonstrationError(
                "ascension_level must be at most 20"
            )
        object.__setattr__(self, "ascension_level", ascension)
        for name in ("training_seed_start", "behavior_seed"):
            value = _nonnegative(getattr(self, name), name)
            if value >= 1 << 63:
                raise StrategicDemonstrationError(f"{name} must be below 2^63")
            object.__setattr__(self, name, value)
        if not isinstance(self.potion_lane, RunPotionLane):
            raise StrategicDemonstrationError("potion_lane must be typed")
        if not isinstance(self.combat_anchor_mode, CombatAnchorMode):
            raise StrategicDemonstrationError("combat_anchor_mode must be typed")


@dataclass(frozen=True)
class StrategicDemonstrationBatch:
    decision_batch: Mapping[str, object]
    target_ordinals: tuple[int, ...]
    episode_seeds: tuple[int, ...]
    acts: tuple[int, ...]
    floors: tuple[int, ...]
    context_kinds: tuple[int, ...]
    array_bytes: int

    @property
    def row_count(self) -> int:
        return len(self.target_ordinals)


@dataclass(frozen=True)
class StrategicDemonstrationCorpus:
    batches: tuple[StrategicDemonstrationBatch, ...]
    requested_runs: int
    completed_runs: int
    victories: int
    defeats: int
    batch_steps: int
    decision_rounds: int
    teacher_rows: int
    combat_rows: int
    strategic_selection_rows: int
    unavailable_strategic_root_rows: int
    array_bytes: int
    elapsed_seconds: float
    stop_reason: str
    combat_anchor_mode: CombatAnchorMode
    combat_anchor_provenance_mismatches: tuple[str, ...]
    terminal_episode_seeds: tuple[int, ...]
    terminal_rewards: tuple[int, ...]
    terminal_acts: tuple[int, ...]
    terminal_floors: tuple[int, ...]
    terminal_hps: tuple[int, ...]
    terminal_max_hps: tuple[int, ...]

    def __post_init__(self) -> None:
        terminal_columns = (
            self.terminal_episode_seeds,
            self.terminal_rewards,
            self.terminal_acts,
            self.terminal_floors,
            self.terminal_hps,
            self.terminal_max_hps,
        )
        if any(len(column) != self.completed_runs for column in terminal_columns):
            raise StrategicDemonstrationError(
                "terminal outcome columns are misaligned with completed runs"
            )
        if len(set(self.terminal_episode_seeds)) != self.completed_runs:
            raise StrategicDemonstrationError("terminal run seeds are not unique")
        if any(reward not in {-1, 1} for reward in self.terminal_rewards):
            raise StrategicDemonstrationError("terminal rewards must be exact outcomes")
        if self.victories != sum(reward > 0 for reward in self.terminal_rewards):
            raise StrategicDemonstrationError("terminal victory accounting disagrees")
        if self.defeats != sum(reward < 0 for reward in self.terminal_rewards):
            raise StrategicDemonstrationError("terminal defeat accounting disagrees")

    @property
    def context_counts(self) -> dict[int, int]:
        counts = Counter(
            context
            for batch in self.batches
            for context in batch.context_kinds
        )
        return dict(sorted(counts.items()))

    @property
    def decisions_per_second(self) -> float:
        if self.elapsed_seconds <= 0.0:
            return 0.0
        return self.decision_rounds / self.elapsed_seconds

    @property
    def terminal_floor_counts(self) -> dict[tuple[int, int], int]:
        return dict(sorted(Counter(zip(self.terminal_acts, self.terminal_floors)).items()))


def collect_strategic_demonstrations(
    config: StrategicDemonstrationConfig,
    *,
    combat_bridge: CombatSessionBridge | None = None,
    run_bridge: CategoricalSessionBridge | None = None,
) -> StrategicDemonstrationCorpus:
    """Follow production strategic roots and retain only exact teacher rows."""

    if not isinstance(config, StrategicDemonstrationConfig):
        raise StrategicDemonstrationError("demonstration config must be typed")
    active_combat_bridge = combat_bridge or CombatSessionBridge.installed()
    active_run_bridge = run_bridge or CategoricalSessionBridge.installed()
    if active_combat_bridge.semantic_schema != active_run_bridge.semantic_schema:
        raise StrategicDemonstrationError(
            "combat behavior and run environment semantic schemas differ"
        )
    if config.combat_anchor_mode is CombatAnchorMode.STRICT_PUBLICATION:
        recovered = recover_published_combat_behavior(
            config.behavior,
            active_combat_bridge,
            CombatWinSessionLimits(),
            (config.behavior_seed,),
        )
        combat_policy = GreedyTorchPolicy(
            recovered.policies[0].frozen_scorer,
            recovered.manifest_id,
        )
        anchor_mismatches: tuple[str, ...] = ()
    else:
        recovered = recover_compatible_combat_scorer(
            config.behavior,
            active_combat_bridge,
            CombatWinSessionLimits(),
        )
        combat_policy = GreedyTorchPolicy(
            recovered.scorer,
            recovered.source_manifest_id,
        )
        anchor_mismatches = recovered.provenance_mismatches
    potion_lane = resolve_run_potion_lane(config.potion_lane, recovered)
    environment_factory = (
        active_run_bridge.environment
        if potion_lane is CombatPotionLane.ALL
        else active_run_bridge.environment_without_combat_potions
    )
    try:
        from sts_learning_bridge import PHASE_SELECTION, PHASE_STRATEGIC_ROOT
    except ImportError as error:
        raise StrategicDemonstrationError(
            "installed bridge does not expose production behavior phases"
        ) from error

    batches: list[StrategicDemonstrationBatch] = []
    completed_runs = 0
    victories = 0
    defeats = 0
    batch_steps = 0
    decision_rounds = 0
    teacher_rows = 0
    combat_rows = 0
    strategic_selection_rows = 0
    unavailable_strategic_root_rows = 0
    array_bytes = 0
    terminal_episode_seeds: list[int] = []
    terminal_rewards: list[int] = []
    terminal_acts: list[int] = []
    terminal_floors: list[int] = []
    terminal_hps: list[int] = []
    terminal_max_hps: list[int] = []
    stop_reason = "completed_runs"
    started = time.perf_counter()
    next_seed = config.training_seed_start

    while completed_runs < config.run_count:
        if _deadline_reached(started, config.wall_ms):
            stop_reason = "wall_ms"
            break
        cohort_size = min(config.slot_count, config.run_count - completed_runs)
        seeds = list(range(next_seed, next_seed + cohort_size))
        next_seed += cohort_size
        env = environment_factory(seeds, config.ascension_level)
        provider = BridgeDecisionProgressProvider(env)
        cohort_steps = 0
        cohort_rounds = 0
        while env.terminal_count < env.slot_count:
            if _deadline_reached(started, config.wall_ms):
                stop_reason = "wall_ms"
                break
            if cohort_steps >= config.max_batch_steps_per_cohort:
                stop_reason = "batch_steps"
                break
            if cohort_rounds >= config.max_decision_rounds_per_cohort:
                stop_reason = "decision_rounds"
                break
            batch = env.decision_batch(
                semantic=True,
                production_behavior=True,
            )
            slots = _integer_vector(batch, "slot_indices")
            phases = _integer_vector(batch, "phase")
            available = _bool_vector(batch, "production_behavior_available")
            targets = _integer_vector(batch, "production_behavior_ordinals")
            if not (
                slots.shape == phases.shape == available.shape == targets.shape
            ):
                raise StrategicDemonstrationError(
                    "production behavior columns are misaligned"
                )
            progress = provider.capture(tuple(map(int, slots)))
            if len(progress) != slots.size:
                raise StrategicDemonstrationError(
                    "production behavior progress is misaligned"
                )
            if np.any(available & (phases != PHASE_STRATEGIC_ROOT)):
                raise StrategicDemonstrationError(
                    "production behavior labeled a non-strategic-root row"
                )

            available_rows = tuple(int(row) for row in np.flatnonzero(available))
            remaining_rows = config.max_rows - teacher_rows
            if available_rows and remaining_rows <= 0:
                stop_reason = "max_rows"
                break
            if len(available_rows) > remaining_rows:
                available_rows = available_rows[:remaining_rows]
                stop_reason = "max_rows"
            if available_rows:
                demonstration = _demonstration_batch(
                    batch,
                    progress,
                    targets,
                    available_rows,
                )
                if array_bytes + demonstration.array_bytes > config.max_array_bytes:
                    stop_reason = "max_array_bytes"
                    break
                batches.append(demonstration)
                teacher_rows += demonstration.row_count
                array_bytes += demonstration.array_bytes

            combat_rows += sum(row.is_combat for row in progress)
            strategic_selection_rows += sum(
                not row.is_combat and int(phases[index]) == PHASE_SELECTION
                for index, row in enumerate(progress)
            )
            unavailable_strategic_root_rows += sum(
                not row.is_combat
                and int(phases[index]) == PHASE_STRATEGIC_ROOT
                and not bool(available[index])
                for index, row in enumerate(progress)
            )

            fallback = combat_policy.choose(batch)
            ordinals = list(fallback.ordinals)
            for index, row in enumerate(progress):
                if available[index]:
                    ordinals[index] = int(targets[index])
                elif not row.is_combat:
                    ordinals[index] = 0
            env.choose(ordinals)
            decision_rounds += slots.size
            cohort_rounds += slots.size
            if env.ready:
                step = env.step()
                terminal_slots = _integer_vector(step, "terminal_slot_indices")
                rewards = _integer_vector(step, "terminal_reward")
                acts = _integer_vector(step, "terminal_act")
                floors = _integer_vector(step, "terminal_floor")
                hps = _integer_vector(step, "terminal_hp")
                max_hps = _integer_vector(step, "terminal_max_hp")
                if not (
                    terminal_slots.shape
                    == rewards.shape
                    == acts.shape
                    == floors.shape
                    == hps.shape
                    == max_hps.shape
                ):
                    raise StrategicDemonstrationError(
                        "terminal outcome columns are misaligned"
                    )
                if any(int(slot) >= len(seeds) for slot in terminal_slots):
                    raise StrategicDemonstrationError(
                        "terminal outcome references an unknown cohort slot"
                    )
                terminal_episode_seeds.extend(seeds[int(slot)] for slot in terminal_slots)
                terminal_rewards.extend(map(int, rewards))
                terminal_acts.extend(map(int, acts))
                terminal_floors.extend(map(int, floors))
                terminal_hps.extend(map(int, hps))
                terminal_max_hps.extend(map(int, max_hps))
                completed_runs += rewards.size
                victories += int(np.count_nonzero(rewards > 0))
                defeats += int(np.count_nonzero(rewards < 0))
                batch_steps += 1
                cohort_steps += 1
            if stop_reason in {"max_rows", "max_array_bytes"}:
                break
        if stop_reason != "completed_runs":
            break

    elapsed = time.perf_counter() - started
    return StrategicDemonstrationCorpus(
        batches=tuple(batches),
        requested_runs=config.run_count,
        completed_runs=completed_runs,
        victories=victories,
        defeats=defeats,
        batch_steps=batch_steps,
        decision_rounds=decision_rounds,
        teacher_rows=teacher_rows,
        combat_rows=combat_rows,
        strategic_selection_rows=strategic_selection_rows,
        unavailable_strategic_root_rows=unavailable_strategic_root_rows,
        array_bytes=array_bytes,
        elapsed_seconds=elapsed,
        stop_reason=stop_reason,
        combat_anchor_mode=config.combat_anchor_mode,
        combat_anchor_provenance_mismatches=anchor_mismatches,
        terminal_episode_seeds=tuple(terminal_episode_seeds),
        terminal_rewards=tuple(terminal_rewards),
        terminal_acts=tuple(terminal_acts),
        terminal_floors=tuple(terminal_floors),
        terminal_hps=tuple(terminal_hps),
        terminal_max_hps=tuple(terminal_max_hps),
    )


def _demonstration_batch(
    batch: Mapping[str, object],
    progress: Sequence[DecisionRunProgress],
    targets: np.ndarray,
    rows: tuple[int, ...],
) -> StrategicDemonstrationBatch:
    model_batch = {
        key: batch[key]
        for key in (
            "slot_indices",
            "phase",
            "candidate_counts",
            "candidate_row_splits",
            "semantic",
        )
    }
    if "dense_action_mask" in batch:
        model_batch["dense_action_mask"] = batch["dense_action_mask"]
    selected = select_semantic_decision_rows(model_batch, rows)
    target_ordinals = tuple(int(targets[row]) for row in rows)
    candidate_counts = _integer_vector(selected, "candidate_counts")
    if any(
        target < 0 or target >= int(candidate_counts[index])
        for index, target in enumerate(target_ordinals)
    ):
        raise StrategicDemonstrationError(
            "production behavior ordinal is outside its candidate row"
        )
    selected_progress = tuple(progress[row] for row in rows)
    contexts = tuple(row.strategic_context_kind for row in selected_progress)
    if any(context is None for context in contexts):
        raise StrategicDemonstrationError(
            "teacher row has no strategic context kind"
        )
    return StrategicDemonstrationBatch(
        decision_batch=selected,
        target_ordinals=target_ordinals,
        episode_seeds=tuple(row.episode_seed for row in selected_progress),
        acts=tuple(row.act for row in selected_progress),
        floors=tuple(row.floor for row in selected_progress),
        context_kinds=tuple(int(context) for context in contexts),
        array_bytes=_array_bytes(selected),
    )


def _array_bytes(value: object) -> int:
    if isinstance(value, np.ndarray):
        return value.nbytes
    if isinstance(value, Mapping):
        return sum(_array_bytes(item) for item in value.values())
    return 0


def _integer_vector(source: Mapping[str, object], key: str) -> np.ndarray:
    value = source.get(key)
    if not isinstance(value, np.ndarray) or value.ndim != 1:
        raise StrategicDemonstrationError(f"{key} must be a NumPy vector")
    if value.dtype.kind not in "iu":
        raise StrategicDemonstrationError(f"{key} must contain integers")
    return value


def _bool_vector(source: Mapping[str, object], key: str) -> np.ndarray:
    value = source.get(key)
    if not isinstance(value, np.ndarray) or value.ndim != 1:
        raise StrategicDemonstrationError(f"{key} must be a NumPy vector")
    if value.dtype != np.bool_:
        raise StrategicDemonstrationError(f"{key} must contain booleans")
    return value


def _deadline_reached(started: float, wall_ms: int) -> bool:
    return (time.perf_counter() - started) * 1_000 >= wall_ms


def _positive(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized == 0:
        raise StrategicDemonstrationError(f"{name} must be positive")
    return normalized


def _nonnegative(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise StrategicDemonstrationError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise StrategicDemonstrationError(f"{name} must be an integer") from error
    if normalized < 0:
        raise StrategicDemonstrationError(f"{name} must be non-negative")
    return normalized
