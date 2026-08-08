"""Typed configuration for same-root combat learning objectives."""

from __future__ import annotations

import operator
from dataclasses import dataclass
from enum import IntEnum


class CombatObjectiveError(ValueError):
    """A combat objective configuration is malformed."""


class CombatAllWinAxis(IntEnum):
    """Optional learning axis after every same-root replicate wins."""

    NONE = 0
    TERMINAL_HP = 1


@dataclass(frozen=True)
class CombatWinObjectiveConfig:
    """Exact width and all-win fallback axis of the combat objective."""

    groups_per_update: int = 1
    all_win_axis: CombatAllWinAxis = CombatAllWinAxis.TERMINAL_HP

    def __post_init__(self) -> None:
        value = self.groups_per_update
        if isinstance(value, bool):
            raise CombatObjectiveError("groups_per_update must be an integer, not bool")
        try:
            normalized = operator.index(value)
        except TypeError as error:
            raise CombatObjectiveError(
                "groups_per_update must be an integer"
            ) from error
        if normalized <= 0:
            raise CombatObjectiveError("groups_per_update must be positive")
        if not isinstance(self.all_win_axis, CombatAllWinAxis):
            raise CombatObjectiveError("all_win_axis must be CombatAllWinAxis")
        object.__setattr__(self, "groups_per_update", normalized)
