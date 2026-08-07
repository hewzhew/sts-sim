"""Typed behavior-policy identity and one batched policy choice."""

from __future__ import annotations

import operator
from collections.abc import Sequence
from dataclasses import dataclass


class PolicyChoiceError(ValueError):
    """A behavior manifest id or batched choice is malformed."""


@dataclass(frozen=True, order=True)
class BehaviorManifestId:
    """Caller-owned SHA-256 identity of one exact behavior-policy manifest."""

    digest: bytes

    def __post_init__(self) -> None:
        if not isinstance(self.digest, bytes):
            raise PolicyChoiceError("behavior manifest digest must be immutable bytes")
        if len(self.digest) != 32:
            raise PolicyChoiceError("behavior manifest digest must contain 32 bytes")


@dataclass(frozen=True)
class BatchPolicyChoice:
    """One model call's aligned ordinals and exact behavior provenance."""

    ordinals: tuple[int, ...]
    behavior_manifest_id: BehaviorManifestId

    def __post_init__(self) -> None:
        if not isinstance(self.behavior_manifest_id, BehaviorManifestId):
            raise PolicyChoiceError(
                "batch policy choice requires a BehaviorManifestId"
            )
        try:
            raw_ordinals = tuple(self.ordinals)
        except TypeError as error:
            raise PolicyChoiceError("policy ordinals must be a sequence") from error
        normalized: list[int] = []
        for value in raw_ordinals:
            try:
                normalized.append(operator.index(value))
            except TypeError as error:
                raise PolicyChoiceError("policy ordinal must be an integer") from error
        object.__setattr__(self, "ordinals", tuple(normalized))

    @classmethod
    def create(
        cls,
        ordinals: Sequence[int],
        behavior_manifest_id: BehaviorManifestId,
    ) -> BatchPolicyChoice:
        return cls(tuple(ordinals), behavior_manifest_id)
