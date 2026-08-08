"""Typed terminal returns for sparse whole-run policy training."""

from __future__ import annotations

import math
import operator
from collections.abc import Sequence
from dataclasses import dataclass
from enum import IntEnum
from numbers import Real

from .recovery import TerminalAttemptRecord


class TerminalReturnError(ValueError):
    """A terminal outcome or return profile is malformed."""


class TerminalAdvantageMode(IntEnum):
    """How one complete-attempt batch turns terminal returns into advantages."""

    RAW_RETURN = 0
    LEAVE_ONE_OUT = 1


@dataclass(frozen=True)
class FloorProgressReturnConfig:
    """Map terminal floor progress below one reserved victory return."""

    target_floor: int = 52

    def __post_init__(self) -> None:
        if isinstance(self.target_floor, bool):
            raise TerminalReturnError("target_floor must be an integer, not bool")
        try:
            target = operator.index(self.target_floor)
        except TypeError as error:
            raise TerminalReturnError("target_floor must be an integer") from error
        if target < 2:
            raise TerminalReturnError("target_floor must be at least two")
        object.__setattr__(self, "target_floor", target)


@dataclass(frozen=True)
class OnPolicyObjectiveConfig:
    """Exact terminal objective and complete-attempt update size."""

    terminal_return: FloorProgressReturnConfig = FloorProgressReturnConfig()
    attempts_per_update: int = 8
    advantage_mode: TerminalAdvantageMode = TerminalAdvantageMode.RAW_RETURN

    def __post_init__(self) -> None:
        if not isinstance(self.terminal_return, FloorProgressReturnConfig):
            raise TerminalReturnError("terminal_return must be typed")
        if isinstance(self.attempts_per_update, bool):
            raise TerminalReturnError(
                "attempts_per_update must be an integer, not bool"
            )
        try:
            attempts = operator.index(self.attempts_per_update)
        except TypeError as error:
            raise TerminalReturnError(
                "attempts_per_update must be an integer"
            ) from error
        if attempts <= 0:
            raise TerminalReturnError("attempts_per_update must be positive")
        if not isinstance(self.advantage_mode, TerminalAdvantageMode):
            raise TerminalReturnError(
                "advantage_mode must be TerminalAdvantageMode"
            )
        if (
            self.advantage_mode is TerminalAdvantageMode.LEAVE_ONE_OUT
            and attempts < 2
        ):
            raise TerminalReturnError(
                "leave-one-out advantage requires at least two attempts per update"
            )
        object.__setattr__(self, "attempts_per_update", attempts)


def terminal_return_advantages(
    returns: Sequence[float],
    mode: TerminalAdvantageMode,
) -> tuple[float, ...]:
    """Convert one independent attempt batch into typed policy advantages."""

    if not isinstance(mode, TerminalAdvantageMode):
        raise TerminalReturnError("advantage mode must be TerminalAdvantageMode")
    normalized: list[float] = []
    for value in returns:
        if isinstance(value, bool) or not isinstance(value, Real):
            raise TerminalReturnError("terminal returns must be real numbers")
        number = float(value)
        if not math.isfinite(number):
            raise TerminalReturnError("terminal returns must be finite")
        normalized.append(number)
    if not normalized:
        raise TerminalReturnError("advantage calculation requires terminal returns")
    if mode is TerminalAdvantageMode.RAW_RETURN:
        return tuple(normalized)
    if len(normalized) < 2:
        raise TerminalReturnError(
            "leave-one-out advantage requires at least two terminal returns"
        )
    batch_mean = math.fsum(normalized) / len(normalized)
    scale = len(normalized) / (len(normalized) - 1)
    return tuple(scale * (value - batch_mean) for value in normalized)


def floor_progress_terminal_return(
    attempt: TerminalAttemptRecord,
    config: FloorProgressReturnConfig,
) -> float:
    """Return ``1`` for victory and a bounded floor signal for defeat."""

    if not isinstance(attempt, TerminalAttemptRecord):
        raise TerminalReturnError("terminal return requires a terminal attempt record")
    if not isinstance(config, FloorProgressReturnConfig):
        raise TerminalReturnError("terminal return requires a floor-progress config")
    if attempt.terminal_reward == 1:
        return 1.0
    capped_floor = min(attempt.terminal.terminal_floor, config.target_floor - 1)
    return -1.0 + (2.0 * capped_floor / config.target_floor)
