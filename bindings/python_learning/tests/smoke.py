from __future__ import annotations

import time

import numpy as np

from sts_learning_bridge import LearningBatchEnv


_MULTIPLIER = 6_364_136_223_846_793_005
_INCREMENT = 1_442_695_040_888_963_407
_MASK_64 = (1 << 64) - 1
_SEED_XOR = 0x9E37_79B9_7F4A_7C15


def _pick(states: list[int], slot: int, length: int) -> int:
    assert length > 0
    states[slot] = (states[slot] * _MULTIPLIER + _INCREMENT) & _MASK_64
    return (states[slot] >> 32) % length


def main() -> None:
    seeds = list(range(1, 6))
    random_states = [seed ^ _SEED_XOR for seed in seeds]
    env = LearningBatchEnv(seeds)
    total_steps = 0
    started = time.perf_counter()

    initial = env.decision_batch(dense_mask=True)
    invalid = [0] * int(initial["slot_indices"].size)
    invalid[0] = int(initial["candidate_counts"][0])
    try:
        env.choose(invalid)
    except ValueError:
        pass
    else:
        raise AssertionError("out-of-range candidate ordinal was accepted")
    after_rejection = env.decision_batch(dense_mask=True)
    for field in (
        "slot_indices",
        "phase",
        "candidate_counts",
        "candidate_row_splits",
        "dense_action_mask",
    ):
        assert np.array_equal(initial[field], after_rejection[field])

    while env.terminal_count < env.slot_count:
        while not env.ready:
            batch = env.decision_batch(dense_mask=True)
            slots = batch["slot_indices"]
            counts = batch["candidate_counts"]
            row_splits = batch["candidate_row_splits"]
            mask = batch["dense_action_mask"]

            assert slots.dtype == np.uint64
            assert counts.dtype == np.uint64
            assert row_splits.dtype == np.uint64
            assert mask.dtype == np.bool_
            assert row_splits.shape == (slots.size + 1,)
            assert np.array_equal(np.diff(row_splits), counts)
            assert np.array_equal(mask.sum(axis=1), counts)

            ordinals = [
                _pick(random_states, int(slot), int(count))
                for slot, count in zip(slots, counts, strict=True)
            ]
            env.choose(ordinals)

        step = env.step()
        total_steps += int(step["slot_indices"].size)
        assert total_steps < 100_000

    elapsed = time.perf_counter() - started
    assert total_steps == 330
    print(
        "python_learning_bridge_smoke "
        f"episodes={env.slot_count} steps={total_steps} "
        f"elapsed_ms={elapsed * 1000:.1f} "
        f"steps_per_second={total_steps / elapsed:.0f}"
    )
    env.reset_slot(0, 99)
    assert env.terminal_count == env.slot_count - 1
    assert np.array_equal(env.decision_batch()["slot_indices"], np.array([0], dtype=np.uint64))
    try:
        env.reset_slot(0, 100)
    except ValueError:
        pass
    else:
        raise AssertionError("active slot reset was accepted")


if __name__ == "__main__":
    main()
