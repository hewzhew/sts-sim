from __future__ import annotations

import unittest

import numpy as np

from sts_learning import (
    CombatGroupOutcomeAccumulator,
    CombatOutcomeError,
    CombatTerminalOutcome,
    CombatTerminalStepBatch,
    CompletedCombatGroup,
)
ROOT_ID = "12" * 32
COMBAT_HASH = "ab" * 32
WIN_KIND = 0
LOSS_KIND = 1


def _outcome(
    replicate: int,
    *,
    kind: int = WIN_KIND,
    final_hp: int = 80,
    potions_used: int = 0,
) -> CombatTerminalOutcome:
    return CombatTerminalOutcome(
        replicate_index=replicate,
        terminal_kind=kind,
        won=kind == WIN_KIND,
        start_hp=80,
        final_hp=final_hp,
        hp_loss=max(80 - final_hp, 0),
        turns=3,
        potions_used=potions_used,
        potions_discarded=0,
        cards_played=8,
    )


def _bridge_step(replicates: list[int]) -> dict[str, object]:
    rows = len(replicates)
    return {
        "root_id": ROOT_ID,
        "exact_combat_state_hash": COMBAT_HASH,
        "terminal_slot_indices": np.asarray(replicates, dtype=np.uint64),
        "terminal_kind": np.full(rows, WIN_KIND, dtype=np.uint8),
        "terminal_won": np.ones(rows, dtype=np.bool_),
        "terminal_start_hp": np.full(rows, 80, dtype=np.int32),
        "terminal_final_hp": np.asarray(
            [80 - 10 * replicate for replicate in replicates], dtype=np.int32
        ),
        "terminal_hp_loss": np.asarray(
            [10 * replicate for replicate in replicates], dtype=np.int32
        ),
        "terminal_turns": np.full(rows, 3, dtype=np.uint32),
        "terminal_potions_used": np.zeros(rows, dtype=np.uint32),
        "terminal_potions_discarded": np.zeros(rows, dtype=np.uint32),
        "terminal_cards_played": np.full(rows, 8, dtype=np.uint32),
    }


def test_bridge_terminal_columns_copy_into_typed_rows() -> None:
    batch = CombatTerminalStepBatch.from_bridge_step(
        _bridge_step([0, 2]),
        replicate_count=3,
    )

    assert tuple(row.replicate_index for row in batch.outcomes) == (0, 2)
    assert tuple(row.final_hp for row in batch.outcomes) == (80, 60)
    assert all(row.won for row in batch.outcomes)


def test_accumulator_rejects_duplicate_terminal_without_partial_commit() -> None:
    accumulator = CombatGroupOutcomeAccumulator(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        replicate_count=3,
    )
    accumulator.record(
        CombatTerminalStepBatch(ROOT_ID, COMBAT_HASH, (_outcome(0),))
    )

    try:
        accumulator.record(
            CombatTerminalStepBatch(
                ROOT_ID,
                COMBAT_HASH,
                (_outcome(1), _outcome(0)),
            )
        )
    except CombatOutcomeError:
        pass
    else:
        raise AssertionError("duplicate terminal replicate was accepted")

    assert accumulator.terminal_count == 1
    try:
        accumulator.finish()
    except CombatOutcomeError:
        pass
    else:
        raise AssertionError("incomplete combat group was accepted")


def test_accumulator_rejects_foreign_root_without_partial_commit() -> None:
    accumulator = CombatGroupOutcomeAccumulator(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        replicate_count=2,
    )

    try:
        accumulator.record(
            CombatTerminalStepBatch("34" * 32, COMBAT_HASH, (_outcome(0),))
        )
    except CombatOutcomeError:
        pass
    else:
        raise AssertionError("terminal batch from a different root was accepted")

    assert accumulator.terminal_count == 0


def test_grouped_axes_do_not_invent_one_hp_potion_exchange_rate() -> None:
    group = CompletedCombatGroup(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        outcomes=(
            _outcome(0, final_hp=80),
            _outcome(1, final_hp=60, potions_used=1),
            _outcome(2, final_hp=40),
        ),
    )

    advantages = group.grouped_advantages()

    assert advantages.win == (0.0, 0.0, 0.0)
    assert np.allclose(advantages.terminal_hp, (0.375, 0.0, -0.375))
    assert advantages.potion_retention == (0.5, -1.0, 0.5)
    assert not advantages.win_has_signal
    assert advantages.terminal_hp_has_signal
    assert advantages.potion_retention_has_signal


def test_grouped_win_axis_uses_only_same_root_siblings() -> None:
    group = CompletedCombatGroup(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        outcomes=(
            _outcome(0, kind=WIN_KIND),
            _outcome(1, kind=LOSS_KIND, final_hp=0),
        ),
    )

    advantages = group.grouped_advantages()

    assert advantages.win == (1.0, -1.0)
    assert advantages.win_has_signal


def test_same_root_group_rejects_mismatched_start_hp() -> None:
    second = CombatTerminalOutcome(
        replicate_index=1,
        terminal_kind=WIN_KIND,
        won=True,
        start_hp=79,
        final_hp=79,
        hp_loss=0,
        turns=3,
        potions_used=0,
        potions_discarded=0,
        cards_played=8,
    )

    try:
        CompletedCombatGroup(
            root_id=ROOT_ID,
            exact_combat_state_hash=COMBAT_HASH,
            outcomes=(_outcome(0), second),
        )
    except CombatOutcomeError:
        pass
    else:
        raise AssertionError("same-root start HP mismatch was accepted")


class CombatOutcomeTests(unittest.TestCase):
    def test_bridge_terminal_columns_copy_into_typed_rows(self) -> None:
        test_bridge_terminal_columns_copy_into_typed_rows()

    def test_accumulator_rejects_duplicate_terminal_without_partial_commit(self) -> None:
        test_accumulator_rejects_duplicate_terminal_without_partial_commit()

    def test_grouped_axes_do_not_invent_one_hp_potion_exchange_rate(self) -> None:
        test_grouped_axes_do_not_invent_one_hp_potion_exchange_rate()

    def test_accumulator_rejects_foreign_root_without_partial_commit(self) -> None:
        test_accumulator_rejects_foreign_root_without_partial_commit()

    def test_grouped_win_axis_uses_only_same_root_siblings(self) -> None:
        test_grouped_win_axis_uses_only_same_root_siblings()

    def test_same_root_group_rejects_mismatched_start_hp(self) -> None:
        test_same_root_group_rejects_mismatched_start_hp()


if __name__ == "__main__":
    unittest.main()
