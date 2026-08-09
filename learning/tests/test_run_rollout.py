from __future__ import annotations

import math

import numpy as np
import pytest

from learning.tests.policy_fixtures import BEHAVIOR_MANIFEST_ID
from sts_learning import (
    AttemptKey,
    CompletedAttemptExperience,
    DecisionExperienceBatch,
    DecisionLineage,
    DecisionRunProgress,
    FloorProgressReturnConfig,
    RunRolloutConfig,
    RunRolloutError,
    SelectionProbability,
    TerminalAttemptOutcome,
    TerminalAttemptRecord,
    build_complete_run_rollout,
    compute_complete_run_gae,
    floor_progress_terminal_return,
)


def _attempt(
    *,
    slot: int,
    decision_floors: tuple[int, ...],
    candidate_counts: tuple[int, ...] | None = None,
    terminal_floor: int,
    reward: int,
) -> CompletedAttemptExperience:
    if candidate_counts is None:
        candidate_counts = (2,) * len(decision_floors)
    assert len(candidate_counts) == len(decision_floors)
    lineage = DecisionLineage(
        key=AttemptKey(
            slot_index=slot,
            episode_seed=1000 + slot,
            episode_generation=0,
            attempt_index=1,
        ),
        recoveries_used=0,
    )
    batches = tuple(
        DecisionExperienceBatch(
            payload={
                "slot_indices": np.array([slot], dtype=np.uint64),
                "candidate_counts": np.array(
                    [candidate_count],
                    dtype=np.uint64,
                ),
            },
            lineages=(lineage,),
            selected_ordinals=(0,),
            selection_probabilities=(
                SelectionProbability.known(
                    1.0 if candidate_count == 1 else 1.0 / candidate_count
                ),
            ),
            behavior_manifest_id=BEHAVIOR_MANIFEST_ID,
            decision_count=1,
            payload_bytes=1,
            run_progress=(
                DecisionRunProgress(
                    episode_seed=lineage.key.episode_seed,
                    act=1,
                    floor=floor,
                    is_combat=True,
                    strategic_context_kind=None,
                ),
            ),
        )
        for floor, candidate_count in zip(
            decision_floors,
            candidate_counts,
            strict=True,
        )
    )
    terminal = TerminalAttemptRecord(
        episode_seed=lineage.key.episode_seed,
        episode_generation=lineage.key.episode_generation,
        attempt_index=lineage.key.attempt_index,
        recoveries_used=lineage.recoveries_used,
        terminal=TerminalAttemptOutcome(
            slot_index=slot,
            terminal_reward=reward,
            terminal_act=3,
            terminal_floor=terminal_floor,
            terminal_hp=20 if reward == 1 else 0,
            terminal_max_hp=80,
            terminal_gold=50,
        ),
    )
    return CompletedAttemptExperience(
        lineage=lineage,
        batches=batches,
        terminal=terminal,
        decision_count=len(batches),
        payload_bytes=len(batches),
    )


def test_defeat_rewards_credit_only_the_transition_that_reaches_a_floor() -> None:
    config = FloorProgressReturnConfig(target_floor=52)
    attempt = _attempt(
        slot=1,
        decision_floors=(0, 10, 10),
        terminal_floor=20,
        reward=-1,
    )

    rollout = build_complete_run_rollout((attempt,), config).attempts[0]

    assert tuple(row.reward for row in rollout.rows) == pytest.approx(
        (20 / 52, 0.0, -32 / 52)
    )
    assert tuple(row.return_to_go for row in rollout.rows) == pytest.approx(
        (-12 / 52, -32 / 52, -32 / 52)
    )
    assert rollout.prefix_reward == 0.0
    assert rollout.total_reward == pytest.approx(
        floor_progress_terminal_return(attempt.terminal, config)
    )


def test_unobserved_prefix_is_conserved_but_never_credited_to_an_action() -> None:
    config = FloorProgressReturnConfig(target_floor=52)
    attempt = _attempt(
        slot=1,
        decision_floors=(5, 51),
        terminal_floor=55,
        reward=1,
    )

    rollout = build_complete_run_rollout((attempt,), config).attempts[0]

    assert rollout.prefix_reward == pytest.approx(10 / 52)
    assert tuple(row.reward for row in rollout.rows) == pytest.approx(
        (92 / 52, -50 / 52)
    )
    assert tuple(row.return_to_go for row in rollout.rows) == pytest.approx(
        (42 / 52, -50 / 52)
    )
    assert rollout.total_reward == pytest.approx(1.0)


def test_long_and_short_attempts_have_equal_weight_and_forced_rows_are_not_actor_samples() -> None:
    short = _attempt(
        slot=1,
        decision_floors=(0,),
        candidate_counts=(2,),
        terminal_floor=0,
        reward=-1,
    )
    long = _attempt(
        slot=2,
        decision_floors=(0, 0, 0),
        candidate_counts=(1, 3, 2),
        terminal_floor=0,
        reward=-1,
    )

    rollout = build_complete_run_rollout(
        (short, long),
        FloorProgressReturnConfig(),
    )

    assert rollout.decision_count == 4
    assert rollout.actor_decision_count == 3
    assert math.fsum(row.value_weight for row in rollout.attempts[0].rows) == 0.5
    assert math.fsum(row.value_weight for row in rollout.attempts[1].rows) == 0.5
    assert math.fsum(row.actor_weight for row in rollout.attempts[0].rows) == 0.5
    assert math.fsum(row.actor_weight for row in rollout.attempts[1].rows) == 0.5
    assert rollout.attempts[1].rows[0].actor_eligible is False
    assert rollout.attempts[1].rows[0].actor_weight == 0.0


def test_monte_carlo_gae_equals_decision_local_return_to_go() -> None:
    attempt = _attempt(
        slot=1,
        decision_floors=(0, 10, 10),
        terminal_floor=20,
        reward=-1,
    )
    rollout = build_complete_run_rollout(
        (attempt,),
        FloorProgressReturnConfig(target_floor=52),
    )

    evaluated = compute_complete_run_gae(
        rollout,
        ((0.1, 0.2, 0.3),),
    )

    expected_returns = tuple(row.return_to_go for row in rollout.attempts[0].rows)
    assert evaluated.returns[0] == pytest.approx(expected_returns)
    assert evaluated.advantages[0] == pytest.approx(
        tuple(
            target - value
            for target, value in zip(
                expected_returns,
                evaluated.value_predictions[0],
                strict=True,
            )
        )
    )


def test_floor_cap_preserves_the_historical_reserved_victory_return() -> None:
    config = FloorProgressReturnConfig(target_floor=52)
    defeat = _attempt(
        slot=1,
        decision_floors=(0, 51),
        terminal_floor=999,
        reward=-1,
    )
    victory = _attempt(
        slot=2,
        decision_floors=(0, 51),
        terminal_floor=999,
        reward=1,
    )

    rollouts = build_complete_run_rollout((defeat, victory), config).attempts

    assert rollouts[0].total_reward == pytest.approx(1.0 - 2.0 / 52)
    assert rollouts[1].total_reward == pytest.approx(1.0)


def test_malformed_order_and_invented_action_discount_fail_closed() -> None:
    decreasing = _attempt(
        slot=1,
        decision_floors=(0, 10, 9),
        terminal_floor=20,
        reward=-1,
    )
    terminal_before_decision = _attempt(
        slot=2,
        decision_floors=(0, 20),
        terminal_floor=10,
        reward=-1,
    )

    with pytest.raises(RunRolloutError, match="chronological"):
        build_complete_run_rollout((decreasing,), FloorProgressReturnConfig())
    with pytest.raises(RunRolloutError, match="precedes"):
        build_complete_run_rollout(
            (terminal_before_decision,),
            FloorProgressReturnConfig(),
        )
    with pytest.raises(RunRolloutError, match="gamma=1"):
        RunRolloutConfig(gamma=0.99)
