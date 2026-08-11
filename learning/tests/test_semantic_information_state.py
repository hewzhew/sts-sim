from __future__ import annotations

from copy import deepcopy

import numpy as np
import pytest

from sts_learning.semantic_information_state import (
    SemanticInformationStateError,
    semantic_candidate_ids,
    semantic_information_state_id,
)


def _one_row() -> dict[str, object]:
    return {
        "phase": np.asarray([1], dtype=np.uint8),
        "candidate_counts": np.asarray([1], dtype=np.uint64),
        "candidate_row_splits": np.asarray([0, 1], dtype=np.uint64),
        "semantic": {
            "schema_version": 8,
            "completeness": np.asarray([1], dtype=np.uint8),
            "token": {
                "row_splits": np.asarray([0, 2], dtype=np.uint64),
                "kind": np.asarray([1, 10], dtype=np.uint16),
            },
            "categorical": {
                "token_indices": np.asarray([0], dtype=np.uint64),
                "field": np.asarray([31], dtype=np.uint16),
                "value": np.asarray([7], dtype=np.int64),
            },
            "scalar": {
                "token_indices": np.asarray([1], dtype=np.uint64),
                "field": np.asarray([42], dtype=np.uint16),
                "value": np.asarray([3.5], dtype=np.float32),
            },
            "relation": {
                "source_token_indices": np.asarray([0], dtype=np.uint64),
                "relation": np.asarray([2], dtype=np.uint16),
                "target_token_indices": np.asarray([1], dtype=np.uint64),
            },
            "candidate_token_indices": np.asarray([1], dtype=np.uint64),
        },
    }


def _same_row_after_prefix() -> dict[str, object]:
    return {
        "phase": np.asarray([0, 1], dtype=np.uint8),
        "candidate_counts": np.asarray([0, 1], dtype=np.uint64),
        "candidate_row_splits": np.asarray([0, 0, 1], dtype=np.uint64),
        "semantic": {
            "schema_version": 8,
            "completeness": np.asarray([1, 1], dtype=np.uint8),
            "token": {
                "row_splits": np.asarray([0, 1, 3], dtype=np.uint64),
                "kind": np.asarray([99, 1, 10], dtype=np.uint16),
            },
            "categorical": {
                "token_indices": np.asarray([0, 1], dtype=np.uint64),
                "field": np.asarray([1, 31], dtype=np.uint16),
                "value": np.asarray([2, 7], dtype=np.int64),
            },
            "scalar": {
                "token_indices": np.asarray([2], dtype=np.uint64),
                "field": np.asarray([42], dtype=np.uint16),
                "value": np.asarray([3.5], dtype=np.float32),
            },
            "relation": {
                "source_token_indices": np.asarray([1], dtype=np.uint64),
                "relation": np.asarray([2], dtype=np.uint16),
                "target_token_indices": np.asarray([2], dtype=np.uint64),
            },
            "candidate_token_indices": np.asarray([2], dtype=np.uint64),
        },
    }


def test_information_state_id_ignores_batch_token_offsets() -> None:
    assert semantic_information_state_id(
        _one_row(), 0
    ) == semantic_information_state_id(_same_row_after_prefix(), 1)
    assert semantic_candidate_ids(_one_row(), 0) == semantic_candidate_ids(
        _same_row_after_prefix(), 1
    )


def test_information_state_id_ignores_candidate_order_but_keeps_candidate_multiset() -> None:
    source = _one_row()
    two_candidates = deepcopy(source)
    two_candidates["candidate_counts"] = np.asarray([2], dtype=np.uint64)
    two_candidates["candidate_row_splits"] = np.asarray([0, 2], dtype=np.uint64)
    two_candidates["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [0, 1], dtype=np.uint64
    )
    reordered = deepcopy(two_candidates)
    reordered["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [1, 0], dtype=np.uint64
    )
    repeated = deepcopy(two_candidates)
    repeated["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [1, 1], dtype=np.uint64
    )

    assert semantic_information_state_id(
        two_candidates, 0
    ) == semantic_information_state_id(reordered, 0)
    assert semantic_information_state_id(
        repeated, 0
    ) != semantic_information_state_id(two_candidates, 0)


def test_candidate_ids_follow_semantics_when_ordinals_are_reordered() -> None:
    ordered = _one_row()
    ordered["candidate_counts"] = np.asarray([2], dtype=np.uint64)
    ordered["candidate_row_splits"] = np.asarray([0, 2], dtype=np.uint64)
    ordered["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [0, 1], dtype=np.uint64
    )
    reordered = deepcopy(ordered)
    reordered["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [1, 0], dtype=np.uint64
    )

    ordered_ids = semantic_candidate_ids(ordered, 0)
    reordered_ids = semantic_candidate_ids(reordered, 0)

    assert ordered_ids[0] != ordered_ids[1]
    assert reordered_ids == tuple(reversed(ordered_ids))


def test_candidate_ids_expose_scorer_indistinguishable_duplicates() -> None:
    repeated = _one_row()
    repeated["candidate_counts"] = np.asarray([2], dtype=np.uint64)
    repeated["candidate_row_splits"] = np.asarray([0, 2], dtype=np.uint64)
    repeated["semantic"]["candidate_token_indices"] = np.asarray(  # type: ignore[index]
        [1, 1], dtype=np.uint64
    )

    first, second = semantic_candidate_ids(repeated, 0)

    assert first == second


def test_information_state_id_changes_with_scalar_value() -> None:
    source = _one_row()
    changed_scalar = deepcopy(source)
    changed_scalar["semantic"]["scalar"]["value"] = np.asarray(  # type: ignore[index]
        [3.75], dtype=np.float32
    )

    source_id = semantic_information_state_id(source, 0)
    assert semantic_information_state_id(changed_scalar, 0) != source_id


def test_information_state_id_rejects_cross_row_relations() -> None:
    malformed = _same_row_after_prefix()
    malformed["semantic"]["relation"]["target_token_indices"] = np.asarray(  # type: ignore[index]
        [0], dtype=np.uint64
    )

    with pytest.raises(SemanticInformationStateError, match="escapes"):
        semantic_information_state_id(malformed, 1)
