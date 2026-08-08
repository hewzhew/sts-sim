"""Typed run progress captured at one policy decision boundary."""

from __future__ import annotations

import operator
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Protocol


class DecisionProgressError(ValueError):
    """A decision-time public run context is malformed or misaligned."""


@dataclass(frozen=True)
class DecisionRunProgress:
    """Minimal public run position aligned to one semantic decision row."""

    episode_seed: int
    act: int
    floor: int

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "episode_seed",
            _integer(self.episode_seed, "episode_seed", minimum=0),
        )
        object.__setattr__(self, "act", _integer(self.act, "act", minimum=0))
        object.__setattr__(self, "floor", _integer(self.floor, "floor", minimum=0))


class DecisionProgressProvider(Protocol):
    """Caller-owned source of public progress for selected environment slots."""

    def capture(
        self,
        slot_indices: Sequence[int],
    ) -> tuple[DecisionRunProgress, ...]: ...


class BridgeDecisionProgressProvider:
    """Adapt the bridge's compact public contexts without inspecting semantics."""

    def __init__(self, environment: object) -> None:
        source = getattr(environment, "public_run_contexts", None)
        if not callable(source):
            raise DecisionProgressError(
                "decision progress environment does not expose public_run_contexts()"
            )
        self._source = source

    def capture(
        self,
        slot_indices: Sequence[int],
    ) -> tuple[DecisionRunProgress, ...]:
        slots = tuple(
            _integer(slot, "decision progress slot", minimum=0)
            for slot in slot_indices
        )
        if len(set(slots)) != len(slots):
            raise DecisionProgressError("decision progress slots contain duplicates")
        rows = self._source()
        if not isinstance(rows, Sequence) or isinstance(rows, (str, bytes)):
            raise DecisionProgressError(
                "public_run_contexts() must return a sequence"
            )
        contexts: dict[int, DecisionRunProgress] = {}
        for row in rows:
            if not isinstance(row, Sequence) or isinstance(row, (str, bytes)):
                raise DecisionProgressError("public run context row must be a pair")
            if len(row) != 2:
                raise DecisionProgressError(
                    "public run context row must contain two values"
                )
            slot = _integer(row[0], "public run context slot", minimum=0)
            if slot in contexts:
                raise DecisionProgressError("public run contexts repeat a slot")
            view = row[1]
            contexts[slot] = DecisionRunProgress(
                episode_seed=_attribute(view, "seed"),
                act=_attribute(view, "act"),
                floor=_attribute(view, "floor"),
            )
        try:
            return tuple(contexts[slot] for slot in slots)
        except KeyError as error:
            raise DecisionProgressError(
                f"decision slot {error.args[0]} has no public run context"
            ) from error


def _attribute(source: object, name: str) -> object:
    try:
        return getattr(source, name)
    except AttributeError as error:
        raise DecisionProgressError(
            f"public run context is missing {name}"
        ) from error


def _integer(value: object, name: str, *, minimum: int) -> int:
    if isinstance(value, bool):
        raise DecisionProgressError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise DecisionProgressError(f"{name} must be an integer") from error
    if normalized < minimum:
        raise DecisionProgressError(f"{name} must be at least {minimum}")
    return normalized
