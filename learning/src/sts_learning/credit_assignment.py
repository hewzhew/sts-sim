"""Non-authoritative diagnostics for whole-run decision credit targets."""

from __future__ import annotations

import math
from collections.abc import Sequence
from dataclasses import dataclass

from .attempts import CompletedAttemptExperience
from .decision_progress import DecisionRunProgress
from .terminal_returns import (
    FloorProgressReturnConfig,
    floor_progress_terminal_return,
)


class CreditAssignmentError(ValueError):
    """Complete attempts cannot produce aligned decision credit evidence."""


@dataclass(frozen=True)
class DecisionCreditDistribution:
    """Compact sign and range summary for one decision-target definition."""

    decision_count: int
    negative: int
    zero: int
    positive: int
    minimum: float
    maximum: float
    mean: float


@dataclass(frozen=True)
class DecisionFloorCreditComparison:
    """Target comparison for decisions observed at one public run floor."""

    floor: int
    terminal_broadcast: DecisionCreditDistribution
    remaining_progress: DecisionCreditDistribution
    matched_floor_advantage: DecisionCreditDistribution


@dataclass(frozen=True)
class CreditAssignmentComparison:
    """Current terminal broadcast beside a decision-local progress target."""

    attempt_count: int
    terminal_broadcast: DecisionCreditDistribution
    remaining_progress: DecisionCreditDistribution
    matched_floor_advantage: DecisionCreditDistribution
    by_decision_floor: tuple[DecisionFloorCreditComparison, ...]


def compare_credit_assignment(
    attempts: Sequence[CompletedAttemptExperience],
    config: FloorProgressReturnConfig,
) -> CreditAssignmentComparison:
    """Compare target distributions without changing the optimizer objective."""

    normalized = tuple(attempts)
    if not normalized:
        raise CreditAssignmentError("credit comparison requires complete attempts")
    if not all(isinstance(attempt, CompletedAttemptExperience) for attempt in normalized):
        raise CreditAssignmentError("credit comparison accepts only complete attempts")
    if not isinstance(config, FloorProgressReturnConfig):
        raise CreditAssignmentError("credit comparison requires a floor return config")

    broadcast: list[float] = []
    remaining: list[float] = []
    decision_rows: list[tuple[int, int, float, float]] = []
    attempt_floor_returns: dict[int, dict[int, float]] = {}
    for attempt_index, attempt in enumerate(normalized):
        attempt_broadcast = floor_progress_terminal_return(attempt.terminal, config)
        observed_decisions = 0
        for batch in attempt.batches:
            if batch.run_progress is None:
                raise CreditAssignmentError(
                    "credit comparison requires decision-time run progress"
                )
            if len(batch.run_progress) != batch.decision_count:
                raise CreditAssignmentError(
                    "decision-time run progress is misaligned with its batch"
                )
            for progress in batch.run_progress:
                local = remaining_floor_progress_return(
                    attempt,
                    progress,
                    config,
                )
                broadcast.append(attempt_broadcast)
                remaining.append(local)
                decision_rows.append(
                    (attempt_index, progress.floor, attempt_broadcast, local)
                )
                previous = attempt_floor_returns.setdefault(
                    progress.floor,
                    {},
                ).setdefault(attempt_index, local)
                if previous != local:
                    raise CreditAssignmentError(
                        "one attempt has conflicting targets at the same floor"
                    )
                observed_decisions += 1
        if observed_decisions != attempt.decision_count:
            raise CreditAssignmentError(
                "complete attempt progress rows disagree with its decision count"
            )

    floor_advantages: dict[tuple[int, int], float] = {}
    for floor, attempt_values in attempt_floor_returns.items():
        if len(attempt_values) == 1:
            attempt_index = next(iter(attempt_values))
            floor_advantages[(attempt_index, floor)] = 0.0
            continue
        values = tuple(attempt_values.values())
        mean = math.fsum(values) / len(values)
        scale = len(values) / (len(values) - 1)
        for attempt_index, value in attempt_values.items():
            floor_advantages[(attempt_index, floor)] = scale * (value - mean)

    matched = [
        floor_advantages[(attempt_index, floor)]
        for attempt_index, floor, _, _ in decision_rows
    ]
    by_floor: dict[int, tuple[list[float], list[float], list[float]]] = {}
    for row, advantage in zip(decision_rows, matched, strict=True):
        _, floor, attempt_broadcast, local = row
        floor_broadcast, floor_remaining, floor_matched = by_floor.setdefault(
            floor,
            ([], [], []),
        )
        floor_broadcast.append(attempt_broadcast)
        floor_remaining.append(local)
        floor_matched.append(advantage)

    return CreditAssignmentComparison(
        attempt_count=len(normalized),
        terminal_broadcast=_distribution(broadcast),
        remaining_progress=_distribution(remaining),
        matched_floor_advantage=_distribution(matched),
        by_decision_floor=tuple(
            DecisionFloorCreditComparison(
                floor=floor,
                terminal_broadcast=_distribution(values[0]),
                remaining_progress=_distribution(values[1]),
                matched_floor_advantage=_distribution(values[2]),
            )
            for floor, values in sorted(by_floor.items())
        ),
    )


def remaining_floor_progress_return(
    attempt: CompletedAttemptExperience,
    progress: DecisionRunProgress,
    config: FloorProgressReturnConfig,
) -> float:
    """Measure terminal progress relative to the decision's remaining horizon.

    Victory retains the reserved ``+1`` target. A defeat at the same effective
    floor as the decision is ``-1``; advancing through all remaining configured
    floors approaches, but never reaches, ``+1``.
    """

    if not isinstance(attempt, CompletedAttemptExperience):
        raise CreditAssignmentError("remaining progress requires a complete attempt")
    if not isinstance(progress, DecisionRunProgress):
        raise CreditAssignmentError("remaining progress requires typed run progress")
    if not isinstance(config, FloorProgressReturnConfig):
        raise CreditAssignmentError("remaining progress requires a floor return config")
    if progress.episode_seed != attempt.lineage.key.episode_seed:
        raise CreditAssignmentError("decision progress seed changed within one attempt")
    if attempt.terminal.terminal_reward == 1:
        return 1.0

    ceiling = config.target_floor - 1
    start = min(progress.floor, ceiling)
    end = min(attempt.terminal.terminal.terminal_floor, ceiling)
    if end < start:
        raise CreditAssignmentError(
            "terminal floor precedes a retained decision floor"
        )
    remaining_horizon = config.target_floor - start
    return -1.0 + (2.0 * (end - start) / remaining_horizon)


def _distribution(values: Sequence[float]) -> DecisionCreditDistribution:
    normalized = tuple(float(value) for value in values)
    if not normalized or not all(math.isfinite(value) for value in normalized):
        raise CreditAssignmentError("credit distribution requires finite targets")
    negative = sum(value < 0.0 for value in normalized)
    zero = sum(value == 0.0 for value in normalized)
    positive = len(normalized) - negative - zero
    return DecisionCreditDistribution(
        decision_count=len(normalized),
        negative=negative,
        zero=zero,
        positive=positive,
        minimum=min(normalized),
        maximum=max(normalized),
        mean=math.fsum(normalized) / len(normalized),
    )
