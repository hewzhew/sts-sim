"""Typed terminal returns for sparse whole-run policy training."""

from __future__ import annotations

import operator
from dataclasses import dataclass

from .recovery import TerminalAttemptRecord


class TerminalReturnError(ValueError):
    """A terminal outcome or return profile is malformed."""


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
        object.__setattr__(self, "attempts_per_update", attempts)


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
