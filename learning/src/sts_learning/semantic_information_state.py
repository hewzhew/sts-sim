"""Stable identities for the semantic graphs consumed by the policy.

The scorer has no positional embeddings.  It updates tokens by relation-aware
multiset aggregation, pools a row by summation, and scores candidate tokens.
This module mirrors that invariance: token numbering, table order, runtime
UUIDs, batch offsets, and candidate ordinals do not define an information
state.  Typed local features, relation direction/kind, graph multiplicity, and
the multiset of candidate semantics do.
"""

from __future__ import annotations

import hashlib
import operator
import struct
from collections.abc import Mapping
from dataclasses import dataclass

import numpy as np


class SemanticInformationStateError(ValueError):
    """A bridge semantic batch violated its row-alignment contract."""


_ROW_PERSON = b"sts-sem-info-v2"
_NODE_PERSON = b"sts-sem-node-v1"
_LAYER_PERSON = b"sts-sem-layer-v1"
_CANDIDATE_PERSON = b"sts-sem-cand-v1"


@dataclass(frozen=True)
class SemanticPolicyRowIdentity:
    """One scorer-visible row identity and its ordinal-aligned candidates."""

    information_state_id: str
    candidate_ids: tuple[str, ...]


def semantic_information_state_id(
    decision_batch: Mapping[str, object],
    row_index: int,
    *,
    relation_layers: int = 2,
) -> str:
    """Hash one row up to the permutations invisible to the current scorer."""

    return semantic_policy_row_identity(
        decision_batch,
        row_index,
        relation_layers=relation_layers,
    ).information_state_id


def semantic_candidate_ids(
    decision_batch: Mapping[str, object],
    row_index: int,
    *,
    relation_layers: int = 2,
) -> tuple[str, ...]:
    """Return scorer-visible candidate identities in the bridge's ordinal order."""

    return semantic_policy_row_identity(
        decision_batch,
        row_index,
        relation_layers=relation_layers,
    ).candidate_ids


def semantic_policy_row_identity(
    decision_batch: Mapping[str, object],
    row_index: int,
    *,
    relation_layers: int = 2,
) -> SemanticPolicyRowIdentity:
    """Build the information and candidate identities without ordinal identity."""

    row_digest, candidate_labels = _semantic_row_digest(
        decision_batch,
        row_index,
        relation_layers=relation_layers,
    )
    return SemanticPolicyRowIdentity(
        information_state_id=row_digest.hex(),
        candidate_ids=tuple(
            _digest_parts(_CANDIDATE_PERSON, [row_digest, label]).hex()
            for label in candidate_labels
        ),
    )


def _semantic_row_digest(
    decision_batch: Mapping[str, object],
    row_index: int,
    *,
    relation_layers: int,
) -> tuple[bytes, tuple[bytes, ...]]:

    row = operator.index(row_index)
    layers = operator.index(relation_layers)
    if layers < 0:
        raise SemanticInformationStateError("relation_layers must be non-negative")
    phases = _array(decision_batch, "phase", np.uint8)
    candidate_counts = _array(decision_batch, "candidate_counts", np.uint64)
    candidate_splits = _array(decision_batch, "candidate_row_splits", np.uint64)
    semantic = _mapping(decision_batch, "semantic")
    completeness = _array(semantic, "completeness", np.uint8)
    token = _mapping(semantic, "token")
    token_splits = _array(token, "row_splits", np.uint64)
    token_kinds = _array(token, "kind", np.uint16)

    row_count = phases.size
    if candidate_counts.size != row_count:
        raise SemanticInformationStateError(
            "decision candidate counts and phases are misaligned"
        )
    if completeness.size != row_count:
        raise SemanticInformationStateError(
            "semantic completeness and decision phases are misaligned"
        )
    if token_splits.size != row_count + 1:
        raise SemanticInformationStateError(
            "semantic token row_splits has the wrong length"
        )
    if candidate_splits.size != row_count + 1:
        raise SemanticInformationStateError(
            "decision candidate row_splits has the wrong length"
        )
    if not 0 <= row < row_count:
        raise SemanticInformationStateError(
            f"semantic row {row} is outside {row_count} rows"
        )

    token_start = int(token_splits[row])
    token_end = int(token_splits[row + 1])
    if not 0 <= token_start <= token_end <= token_kinds.size:
        raise SemanticInformationStateError("semantic token row range is invalid")
    token_count = token_end - token_start
    candidate_start = int(candidate_splits[row])
    candidate_end = int(candidate_splits[row + 1])
    if candidate_end - candidate_start != int(candidate_counts[row]):
        raise SemanticInformationStateError(
            "decision candidate count disagrees with its row split"
        )
    candidate_tokens = _array(semantic, "candidate_token_indices", np.uint64)
    if not 0 <= candidate_start <= candidate_end <= candidate_tokens.size:
        raise SemanticInformationStateError(
            "semantic candidate row range is invalid"
        )

    local_features: list[list[bytes]] = [[] for _ in range(token_count)]
    for local, kind in enumerate(token_kinds[token_start:token_end]):
        local_features[local].append(b"k" + struct.pack("<H", int(kind)))
    categorical = _mapping(semantic, "categorical")
    _append_local_features(
        local_features,
        categorical,
        token_start,
        token_end,
        scalar=False,
    )
    scalar = _mapping(semantic, "scalar")
    _append_local_features(
        local_features,
        scalar,
        token_start,
        token_end,
        scalar=True,
    )
    labels = [
        _digest_parts(_NODE_PERSON, sorted(features)) for features in local_features
    ]

    relation = _mapping(semantic, "relation")
    sources = _array(relation, "source_token_indices", np.uint64)
    relation_kinds = _array(relation, "relation", np.uint16)
    targets = _array(relation, "target_token_indices", np.uint64)
    if sources.size != relation_kinds.size or sources.size != targets.size:
        raise SemanticInformationStateError("semantic relation columns misalign")
    mask = (sources >= token_start) & (sources < token_end)
    selected_sources = sources[mask]
    selected_relations = relation_kinds[mask]
    selected_targets = targets[mask]
    if np.any((selected_targets < token_start) | (selected_targets >= token_end)):
        raise SemanticInformationStateError(
            "semantic relation target escapes its decision row"
        )
    edges = tuple(
        (
            int(source) - token_start,
            int(relation_kind),
            int(target) - token_start,
        )
        for source, relation_kind, target in zip(
            selected_sources,
            selected_relations,
            selected_targets,
            strict=True,
        )
    )
    for _ in range(layers):
        messages: list[list[bytes]] = [[] for _ in range(token_count)]
        for source, relation_kind, target in edges:
            relation_bytes = struct.pack("<H", relation_kind)
            messages[source].append(b"o" + relation_bytes + labels[target])
            messages[target].append(b"i" + relation_bytes + labels[source])
        labels = [
            _digest_parts(_LAYER_PERSON, [label, *sorted(token_messages)])
            for label, token_messages in zip(labels, messages, strict=True)
        ]

    normalized_candidates = candidate_tokens[candidate_start:candidate_end]
    if np.any(
        (normalized_candidates < token_start) | (normalized_candidates >= token_end)
    ):
        raise SemanticInformationStateError(
            "semantic candidate token escapes its decision row"
        )
    candidate_labels = tuple(
        labels[int(token_index) - token_start]
        for token_index in normalized_candidates
    )
    header = struct.pack(
        "<IBBQQ",
        operator.index(semantic["schema_version"]),
        int(phases[row]),
        int(completeness[row]),
        layers,
        token_count,
    )
    row_digest = _digest_parts(
        _ROW_PERSON,
        [header, *sorted(labels), b"candidates", *sorted(candidate_labels)],
    )
    return row_digest, candidate_labels


def _append_local_features(
    destination: list[list[bytes]],
    table: Mapping[str, object],
    token_start: int,
    token_end: int,
    *,
    scalar: bool,
) -> None:
    token_indices = _array(table, "token_indices", np.uint64)
    fields = _array(table, "field", np.uint16)
    values = np.asarray(table.get("value"))
    if token_indices.size != fields.size or token_indices.size != values.size:
        raise SemanticInformationStateError("semantic sparse table columns misalign")
    mask = (token_indices >= token_start) & (token_indices < token_end)
    for token_index, field, value in zip(
        token_indices[mask], fields[mask], values[mask], strict=True
    ):
        local = int(token_index) - token_start
        if scalar:
            encoded = b"s" + struct.pack("<Hf", int(field), float(np.float32(value)))
        else:
            encoded = b"c" + struct.pack("<Hq", int(field), int(value))
        destination[local].append(encoded)


def _digest_parts(person: bytes, parts: list[bytes]) -> bytes:
    hasher = hashlib.blake2b(digest_size=32, person=person)
    for part in parts:
        hasher.update(len(part).to_bytes(8, "little"))
        hasher.update(part)
    return hasher.digest()


def _mapping(source: Mapping[str, object], key: str) -> Mapping[str, object]:
    value = source.get(key)
    if not isinstance(value, Mapping):
        raise SemanticInformationStateError(f"semantic {key} must be a mapping")
    return value


def _array(
    source: Mapping[str, object],
    key: str,
    dtype: np.dtype[object] | type[np.generic],
) -> np.ndarray:
    if key not in source:
        raise SemanticInformationStateError(f"semantic batch is missing {key}")
    value = np.asarray(source[key], dtype=dtype)
    if value.ndim != 1:
        raise SemanticInformationStateError(f"semantic {key} must be one-dimensional")
    return value
