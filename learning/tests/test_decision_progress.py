from __future__ import annotations

from dataclasses import dataclass

import pytest

from sts_learning import (
    BridgeDecisionProgressProvider,
    DecisionProgressError,
    DecisionRunProgress,
)


@dataclass(frozen=True)
class _View:
    seed: int
    act: int
    floor: int
    is_combat: bool


class _Environment:
    def __init__(self, rows: object) -> None:
        self.rows = rows

    def public_run_contexts(self) -> object:
        return self.rows


def test_bridge_provider_returns_requested_slots_in_decision_order() -> None:
    provider = BridgeDecisionProgressProvider(
        _Environment(
            [
                (0, _View(seed=101, act=1, floor=3, is_combat=False)),
                (1, _View(seed=202, act=2, floor=21, is_combat=True)),
            ]
        )
    )

    assert provider.capture((1, 0)) == (
        DecisionRunProgress(episode_seed=202, act=2, floor=21, is_combat=True),
        DecisionRunProgress(episode_seed=101, act=1, floor=3, is_combat=False),
    )


@pytest.mark.parametrize(
    ("rows", "message"),
    [
        ([(0, _View(101, 1, 3, False)), (0, _View(101, 1, 3, False))], "repeat"),
        ([(0, _View(101, 1, 3, False))], "slot 1"),
        ([(0, object())], "missing seed"),
    ],
)
def test_bridge_provider_rejects_malformed_or_missing_contexts(
    rows: object,
    message: str,
) -> None:
    provider = BridgeDecisionProgressProvider(_Environment(rows))

    with pytest.raises(DecisionProgressError, match=message):
        provider.capture((1,) if message == "slot 1" else (0,))


def test_bridge_provider_requires_the_typed_bridge_surface() -> None:
    with pytest.raises(DecisionProgressError, match="public_run_contexts"):
        BridgeDecisionProgressProvider(object())
