from __future__ import annotations

import time

import numpy as np

from sts_learning_bridge import (
    LearningBatchEnv,
    PHASE_STRATEGIC_ROOT,
    SEMANTIC_COMPLETE,
    SEMANTIC_NO_CANDIDATE_TOKEN,
    SEMANTIC_NOT_ENCODED,
    SEMANTIC_RELATION_CANDIDATE_TARGETS,
    SEMANTIC_RELATION_OBSERVATION_HAS_CANDIDATE,
    SEMANTIC_SCHEMA_VERSION,
    SEMANTIC_TOKEN_CANDIDATE,
    semantic_schema,
)


_MULTIPLIER = 6_364_136_223_846_793_005
_INCREMENT = 1_442_695_040_888_963_407
_MASK_64 = (1 << 64) - 1
_SEED_XOR = 0x9E37_79B9_7F4A_7C15


def _pick(states: list[int], slot: int, length: int) -> int:
    assert length > 0
    states[slot] = (states[slot] * _MULTIPLIER + _INCREMENT) & _MASK_64
    return (states[slot] >> 32) % length


def _assert_parallel_table(table: dict, value_dtype: np.dtype) -> None:
    assert table["token_indices"].dtype == np.uint64
    assert table["field"].dtype == np.uint16
    assert table["value"].dtype == value_dtype
    assert table["token_indices"].shape == table["field"].shape
    assert table["field"].shape == table["value"].shape


def _assert_semantic(batch: dict) -> tuple[bool, bool]:
    semantic = batch["semantic"]
    assert semantic["schema_version"] == SEMANTIC_SCHEMA_VERSION
    completeness = semantic["completeness"]
    assert completeness.dtype == np.uint8
    assert completeness.shape == batch["slot_indices"].shape

    token = semantic["token"]
    token_splits = token["row_splits"]
    token_kinds = token["kind"]
    assert token_splits.dtype == np.uint64
    assert token_kinds.dtype == np.uint16
    assert token_splits.shape == (batch["slot_indices"].size + 1,)
    assert token_splits[0] == 0
    assert token_splits[-1] == token_kinds.size
    assert np.all(np.diff(token_splits) >= 0)

    categorical = semantic["categorical"]
    scalar = semantic["scalar"]
    relation = semantic["relation"]
    _assert_parallel_table(categorical, np.dtype(np.int64))
    _assert_parallel_table(scalar, np.dtype(np.float32))
    assert relation["source_token_indices"].dtype == np.uint64
    assert relation["relation"].dtype == np.uint16
    assert relation["target_token_indices"].dtype == np.uint64
    assert relation["source_token_indices"].shape == relation["relation"].shape
    assert relation["relation"].shape == relation["target_token_indices"].shape
    for indices in (
        categorical["token_indices"],
        scalar["token_indices"],
        relation["source_token_indices"],
        relation["target_token_indices"],
    ):
        assert np.all(indices < token_kinds.size)

    candidate_tokens = semantic["candidate_token_indices"]
    candidate_splits = batch["candidate_row_splits"]
    assert candidate_tokens.dtype == np.uint64
    assert candidate_tokens.size == candidate_splits[-1]

    saw_not_encoded = False
    for row, phase in enumerate(batch["phase"]):
        token_start = int(token_splits[row])
        token_end = int(token_splits[row + 1])
        candidate_start = int(candidate_splits[row])
        candidate_end = int(candidate_splits[row + 1])
        row_candidates = candidate_tokens[candidate_start:candidate_end]
        if int(phase) == PHASE_STRATEGIC_ROOT:
            assert completeness[row] == SEMANTIC_COMPLETE
            assert token_start < token_end
            assert np.all(row_candidates != SEMANTIC_NO_CANDIDATE_TOKEN)
            assert np.all(token_kinds[row_candidates] == SEMANTIC_TOKEN_CANDIDATE)
            for candidate in row_candidates:
                assert np.any(
                    (relation["relation"] == SEMANTIC_RELATION_OBSERVATION_HAS_CANDIDATE)
                    & (relation["target_token_indices"] == candidate)
                )
        else:
            saw_not_encoded = True
            assert completeness[row] == SEMANTIC_NOT_ENCODED
            assert token_start == token_end
            assert np.all(row_candidates == SEMANTIC_NO_CANDIDATE_TOKEN)

    saw_candidate_target = bool(
        np.any(relation["relation"] == SEMANTIC_RELATION_CANDIDATE_TARGETS)
    )
    return saw_not_encoded, saw_candidate_target


def _assert_semantic_equal(left: dict, right: dict) -> None:
    assert left["schema_version"] == right["schema_version"]
    assert np.array_equal(left["completeness"], right["completeness"])
    for table_name in ("token", "categorical", "scalar", "relation"):
        assert left[table_name].keys() == right[table_name].keys()
        for column in left[table_name]:
            assert np.array_equal(left[table_name][column], right[table_name][column])
    assert np.array_equal(
        left["candidate_token_indices"], right["candidate_token_indices"]
    )


def main() -> None:
    schema = semantic_schema()
    assert schema["version"] == SEMANTIC_SCHEMA_VERSION
    assert schema["completeness"]["Complete"] == SEMANTIC_COMPLETE
    assert schema["token_kind"]["Candidate"] == SEMANTIC_TOKEN_CANDIDATE
    assert len(schema["action_kind"]) == 25
    assert (
        schema["categorical_vocabulary_size"][
            schema["categorical_field"]["ActionCardId"]
        ]
        == schema["domain_vocabulary_size"]["card_id"]
        == 371
    )

    seeds = list(range(1, 6))
    random_states = [seed ^ _SEED_XOR for seed in seeds]
    env = LearningBatchEnv(seeds)
    total_steps = 0
    started = time.perf_counter()

    initial = env.decision_batch(dense_mask=True, semantic=True)
    initial_not_encoded, _ = _assert_semantic(initial)
    assert not initial_not_encoded
    invalid = [0] * int(initial["slot_indices"].size)
    invalid[0] = int(initial["candidate_counts"][0])
    try:
        env.choose(invalid)
    except ValueError:
        pass
    else:
        raise AssertionError("out-of-range candidate ordinal was accepted")
    after_rejection = env.decision_batch(dense_mask=True, semantic=True)
    for field in (
        "slot_indices",
        "phase",
        "candidate_counts",
        "candidate_row_splits",
        "dense_action_mask",
    ):
        assert np.array_equal(initial[field], after_rejection[field])
    _assert_semantic_equal(initial["semantic"], after_rejection["semantic"])

    saw_not_encoded = False
    saw_candidate_target = False

    while env.terminal_count < env.slot_count:
        while not env.ready:
            batch = env.decision_batch(dense_mask=True, semantic=True)
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
            batch_not_encoded, batch_candidate_target = _assert_semantic(batch)
            saw_not_encoded |= batch_not_encoded
            saw_candidate_target |= batch_candidate_target

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
    assert saw_not_encoded
    assert saw_candidate_target
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
