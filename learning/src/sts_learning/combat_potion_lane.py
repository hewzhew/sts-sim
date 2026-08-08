"""Typed model-facing potion lanes shared by combat training and evaluation."""

from __future__ import annotations

from enum import Enum


class CombatPotionLaneError(RuntimeError):
    """A combat root source did not honor its declared potion lane."""


class CombatPotionLane(Enum):
    """Potion actions exposed to the model without changing engine legality."""

    ALL = "all"
    NEVER = "never"

    @property
    def allows_potions(self) -> bool:
        return self is CombatPotionLane.ALL


class CombatPotionLaneRootSource:
    """Bind one root source to a checked model-facing potion action surface."""

    def __init__(self, source: object, lane: CombatPotionLane) -> None:
        if not callable(getattr(source, "combat_group", None)):
            raise CombatPotionLaneError(
                "combat potion lane requires a combat-root source"
            )
        if not isinstance(lane, CombatPotionLane):
            raise CombatPotionLaneError("combat potion lane must be typed")
        self.source = source
        self.lane = lane

    def combat_group(self, slot_index: int, replicate_count: int):
        group = (
            self.source.combat_group(slot_index, replicate_count)
            if self.lane is CombatPotionLane.ALL
            else self.source.combat_group(slot_index, replicate_count, False)
        )
        if getattr(group, "allows_potions", None) != self.lane.allows_potions:
            raise CombatPotionLaneError(
                "combat root source ignored the declared potion lane"
            )
        return group
