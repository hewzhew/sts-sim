"""Typed configuration for same-root combat learning objectives."""

from __future__ import annotations

import operator
from dataclasses import dataclass


class CombatObjectiveError(ValueError):
    """A combat objective configuration is malformed."""


@dataclass(frozen=True)
class CombatWinObjectiveConfig:
    """Exact number of distinct-root groups consumed by one update attempt."""

    groups_per_update: int = 1

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
        object.__setattr__(self, "groups_per_update", normalized)
