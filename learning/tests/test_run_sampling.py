from __future__ import annotations

from dataclasses import replace

import pytest

from sts_learning import (
    EpisodeRootRetryCurriculum,
    RecoverySlotSnapshot,
    RecoverySlotStatus,
    RunSamplingError,
    TerminalAccountingBatch,
    TerminalAttemptOutcome,
    TerminalAttemptRecord,
)


def _terminal(
    *,
    seed: int,
    attempt_index: int,
    reward: int,
) -> tuple[TerminalAccountingBatch, tuple[RecoverySlotSnapshot, ...]]:
    outcome = TerminalAttemptOutcome(
        slot_index=0,
        terminal_reward=reward,
        terminal_act=1,
        terminal_floor=attempt_index,
        terminal_hp=1 if reward == 1 else 0,
        terminal_max_hp=80,
        terminal_gold=99,
    )
    record = TerminalAttemptRecord(
        episode_seed=seed,
        episode_generation=0,
        attempt_index=attempt_index,
        recoveries_used=attempt_index - 1,
        terminal=outcome,
    )
    status = (
        RecoverySlotStatus.VICTORY_COMPLETE
        if reward == 1
        else RecoverySlotStatus.DEFEAT_PENDING
    )
    return (
        TerminalAccountingBatch(
            attempts=(record,),
            completed_episodes=(),
        ),
        (
            RecoverySlotSnapshot(
                slot_index=0,
                episode_seed=seed,
                episode_generation=0,
                attempt_index=attempt_index,
                recoveries_used=attempt_index - 1,
                status=status,
                pending_terminal=outcome if reward == -1 else None,
            ),
        ),
    )


def test_retry_curriculum_closes_the_update_even_after_an_early_victory() -> None:
    curriculum = EpisodeRootRetryCurriculum(attempts_per_update=3)

    first = curriculum.plan_recovery(
        *_terminal(seed=10, attempt_index=1, reward=-1)
    )
    victory = curriculum.plan_recovery(
        *_terminal(seed=10, attempt_index=2, reward=1)
    )
    boundary = curriculum.plan_recovery(
        *_terminal(seed=20, attempt_index=1, reward=-1)
    )

    assert first.slot_indices == (0,)
    assert victory.slot_indices == ()
    assert boundary.slot_indices == ()
    assert curriculum.attempts_in_update == 0


def test_retry_curriculum_rejects_unaligned_or_degenerate_batches() -> None:
    with pytest.raises(RunSamplingError, match="at least two"):
        EpisodeRootRetryCurriculum(attempts_per_update=1)

    curriculum = EpisodeRootRetryCurriculum(attempts_per_update=2)
    accounting, snapshots = _terminal(seed=10, attempt_index=1, reward=-1)
    with pytest.raises(RunSamplingError, match="seed disagree"):
        curriculum.plan_recovery(
            accounting,
            (replace(snapshots[0], episode_seed=11),),
        )
    with pytest.raises(RunSamplingError, match="status disagree"):
        curriculum.plan_recovery(
            accounting,
            (
                replace(
                    snapshots[0],
                    status=RecoverySlotStatus.VICTORY_COMPLETE,
                    pending_terminal=None,
                ),
            ),
        )
    assert curriculum.attempts_in_update == 0
