from __future__ import annotations

from dataclasses import replace

import pytest

from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_fixture,
    completed_attempt_fixture,
    decision_batch_fixture,
)
from sts_learning import (
    CreditAssignmentError,
    DecisionRunProgress,
    FloorProgressReturnConfig,
    compare_credit_assignment,
    matched_floor_leave_one_out_advantages,
    remaining_floor_progress_return,
)


def _attempt(*, reward: int, terminal_floor: int, decision_floors: tuple[int, ...]):
    manifest_id = behavior_manifest_fixture().identity
    batches = tuple(
        replace(
            decision_batch_fixture(
                slot=0,
                semantic_row=index % 2,
                selected_ordinal=0,
                manifest_id=manifest_id,
            ),
            run_progress=(
                DecisionRunProgress(
                    episode_seed=100,
                    act=1,
                    floor=floor,
                    is_combat=(index % 2 == 0),
                    strategic_context_kind=None if index % 2 == 0 else 3,
                ),
            ),
        )
        for index, floor in enumerate(decision_floors)
    )
    attempt = completed_attempt_fixture(slot=0, batches=batches, reward=reward)
    return replace(
        attempt,
        terminal=replace(
            attempt.terminal,
            terminal=replace(
                attempt.terminal.terminal,
                terminal_floor=terminal_floor,
            ),
        ),
    )


def test_remaining_progress_assigns_stronger_blame_near_a_defeat() -> None:
    attempt = _attempt(reward=-1, terminal_floor=20, decision_floors=(0, 10, 20))
    config = FloorProgressReturnConfig(target_floor=52)

    targets = tuple(
        remaining_floor_progress_return(attempt, batch.run_progress[0], config)
        for batch in attempt.batches
    )

    assert targets == pytest.approx((-12 / 52, -22 / 42, -1.0))
    assert targets[0] > targets[1] > targets[2]


def test_credit_comparison_keeps_victory_reserved_and_groups_decision_floors() -> None:
    defeat = _attempt(reward=-1, terminal_floor=20, decision_floors=(0, 10, 20))
    victory = _attempt(reward=1, terminal_floor=55, decision_floors=(10, 40))

    comparison = compare_credit_assignment(
        (defeat, victory),
        FloorProgressReturnConfig(target_floor=52),
    )

    assert comparison.attempt_count == 2
    assert comparison.terminal_broadcast.decision_count == 5
    assert comparison.terminal_broadcast.positive == 2
    assert comparison.remaining_progress.positive == 2
    assert comparison.matched_floor_advantage.negative == 1
    assert comparison.matched_floor_advantage.zero == 3
    assert comparison.matched_floor_advantage.positive == 1
    assert comparison.matched_floor_context_advantage.zero == 5
    assert tuple(
        (row.is_combat, row.remaining_progress.decision_count)
        for row in comparison.by_combat_scope
    ) == ((False, 2), (True, 3))
    assert tuple(
        (row.context_kind, row.remaining_progress.decision_count)
        for row in comparison.by_strategic_context
    ) == ((3, 2),)
    assert comparison.by_strategic_context[0].strategic_scope_weight == 1.0
    assert (
        comparison.by_strategic_context[0].matched_floor_strategic_weighted_target
        < 0.0
    )
    assert (
        comparison.by_strategic_context[
            0
        ].matched_floor_context_strategic_weighted_target
        == 0.0
    )
    assert tuple(item.floor for item in comparison.by_decision_floor) == (0, 10, 20, 40)
    assert comparison.by_decision_floor[-1].remaining_progress.minimum == 1.0

    aligned = matched_floor_leave_one_out_advantages(
        (defeat, victory),
        FloorProgressReturnConfig(target_floor=52),
    )
    assert tuple(len(attempt) for attempt in aligned) == (3, 2)
    assert aligned[0][0] == (0.0,)
    assert aligned[0][1][0] < 0.0
    assert aligned[1][0][0] > 0.0


def test_credit_comparison_rejects_missing_or_impossible_progress() -> None:
    missing = _attempt(reward=-1, terminal_floor=20, decision_floors=(0,))
    missing = replace(
        missing,
        batches=(replace(missing.batches[0], run_progress=None),),
    )
    with pytest.raises(CreditAssignmentError, match="decision-time"):
        compare_credit_assignment((missing,), FloorProgressReturnConfig())

    impossible = _attempt(reward=-1, terminal_floor=4, decision_floors=(5,))
    with pytest.raises(CreditAssignmentError, match="precedes"):
        compare_credit_assignment((impossible,), FloorProgressReturnConfig())
