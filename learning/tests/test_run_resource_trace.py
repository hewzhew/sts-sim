from __future__ import annotations

from sts_learning.run_resource_trace import (
    RunPublicContext,
    RunResourceTraceAccumulator,
)


def test_completed_combat_keeps_hp_and_potion_identities_separate() -> None:
    accumulator = RunResourceTraceAccumulator()
    start = _context(
        combat=True,
        hp=70,
        gold=50,
        potions=("FearPotion", "GamblersBrew"),
    )
    end = _context(
        combat=False,
        hp=61,
        gold=60,
        potions=("GamblersBrew", "StrengthPotion"),
    )

    accumulator.record(
        {0: start},
        {0: end},
        {
            "slot_indices": [0],
            "terminal_slot_indices": [],
            "terminal_reward": [],
        },
    )
    trace = accumulator.finish()

    assert len(trace.combat_transitions) == 1
    assert trace.combat_transitions[0].hp_loss == 9
    assert trace.hp_loss_sum == 9
    assert trace.potion_identity_losses == (("FearPotion", 1),)
    assert trace.potion_identity_gains == (("StrengthPotion", 1),)
    assert trace.open_combat_count == 0
    assert len(trace.seed_summaries) == 1
    assert trace.seed_summaries[0].combat_count == 1
    assert trace.seed_summaries[0].hp_loss_sum == 9
    assert trace.seed_summaries[0].open_combat is False
    assert trace.seed_summaries[0].last_potion_ids == (
        "GamblersBrew",
        "StrengthPotion",
    )


def test_unfinished_combat_stays_explicitly_censored() -> None:
    accumulator = RunResourceTraceAccumulator()

    accumulator.record(
        {0: _context(combat=False)},
        {0: _context(combat=True)},
        {
            "slot_indices": [0],
            "terminal_slot_indices": [],
            "terminal_reward": [],
        },
    )

    trace = accumulator.finish()
    assert trace.combat_transitions == ()
    assert trace.open_combat_count == 1
    assert len(trace.seed_summaries) == 1
    assert trace.seed_summaries[0].combat_count == 0
    assert trace.seed_summaries[0].last_floor == 4
    assert trace.seed_summaries[0].open_combat is True


def _context(
    *,
    combat: bool,
    hp: int = 70,
    gold: int = 50,
    potions: tuple[str | None, ...] = (),
) -> RunPublicContext:
    return RunPublicContext(
        slot_index=0,
        boundary_kind=1 if combat else 0,
        is_combat=combat,
        is_terminal=False,
        seed=123,
        act=1,
        floor=4,
        hp=hp,
        max_hp=80,
        gold=gold,
        potion_ids=potions,
    )
