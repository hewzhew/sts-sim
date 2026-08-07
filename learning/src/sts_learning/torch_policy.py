"""Optional PyTorch policy components over the bridge-owned semantic graph.

This module is deliberately not imported by :mod:`sts_learning`.  Importing the
ordinary caller package therefore continues to require only NumPy; callers that
want this scorer opt into PyTorch and import this module explicitly.
"""

from __future__ import annotations

import operator
from collections.abc import Mapping, Sequence
from dataclasses import dataclass

import numpy as np
import torch
from torch import Tensor, nn

from .policy import BatchPolicyChoice, BehaviorManifestId


class TorchPolicyError(ValueError):
    """A schema, semantic batch, or ragged target violated the model contract."""


@dataclass(frozen=True)
class SemanticSchemaDimensions:
    """Numeric model dimensions derived from one bridge ``semantic_schema``."""

    version: int
    token_kind_size: int
    categorical_field_size: int
    scalar_field_size: int
    relation_kind_size: int
    valid_token_kinds: tuple[bool, ...]
    valid_categorical_fields: tuple[bool, ...]
    valid_scalar_fields: tuple[bool, ...]
    valid_relation_kinds: tuple[bool, ...]
    categorical_offsets: tuple[int, ...]
    categorical_vocabulary_sizes: tuple[int, ...]
    categorical_vocabulary_size: int

    @classmethod
    def from_bridge_schema(
        cls,
        schema: Mapping[str, object],
    ) -> SemanticSchemaDimensions:
        """Validate and compact dimensions without copying enum names."""

        version = _non_negative_integer(_required(schema, "version"), "schema version")
        token_kind_size, valid_token_kinds = _enum_dimension(schema, "token_kind")
        categorical_field_size, valid_categorical_fields = _enum_dimension(
            schema,
            "categorical_field",
        )
        scalar_field_size, valid_scalar_fields = _enum_dimension(schema, "scalar_field")
        relation_kind_size, valid_relation_kinds = _enum_dimension(
            schema,
            "relation_kind",
        )

        raw_sizes = _mapping(
            _required(schema, "categorical_vocabulary_size"),
            "categorical_vocabulary_size",
        )
        sizes_by_field: dict[int, int] = {}
        for raw_field, raw_size in raw_sizes.items():
            field = _non_negative_integer(raw_field, "categorical vocabulary field")
            size = _positive_integer(raw_size, f"categorical field {field} vocabulary")
            if field in sizes_by_field:
                raise TorchPolicyError(
                    f"categorical vocabulary repeats numeric field {field}"
                )
            sizes_by_field[field] = size
        expected_fields = {
            field for field, valid in enumerate(valid_categorical_fields) if valid
        }
        if set(sizes_by_field) != expected_fields:
            raise TorchPolicyError(
                "categorical vocabulary fields do not match categorical_field ids"
            )

        offsets: list[int] = []
        sizes: list[int] = []
        next_offset = 0
        for field in range(categorical_field_size):
            offsets.append(next_offset)
            size = sizes_by_field.get(field, 0)
            sizes.append(size)
            next_offset += size

        return cls(
            version=version,
            token_kind_size=token_kind_size,
            categorical_field_size=categorical_field_size,
            scalar_field_size=scalar_field_size,
            relation_kind_size=relation_kind_size,
            valid_token_kinds=valid_token_kinds,
            valid_categorical_fields=valid_categorical_fields,
            valid_scalar_fields=valid_scalar_fields,
            valid_relation_kinds=valid_relation_kinds,
            categorical_offsets=tuple(offsets),
            categorical_vocabulary_sizes=tuple(sizes),
            categorical_vocabulary_size=next_offset,
        )


@dataclass(frozen=True)
class RaggedScorerConfig:
    """Small backend configuration; it contains no game feature dictionary."""

    hidden_dim: int = 64
    relation_layers: int = 2

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "hidden_dim",
            _positive_integer(self.hidden_dim, "hidden_dim"),
        )
        object.__setattr__(
            self,
            "relation_layers",
            _non_negative_integer(self.relation_layers, "relation_layers"),
        )


@dataclass(frozen=True)
class RaggedCandidateLogits:
    """Flat candidate logits with their unchanged decision-row boundaries."""

    values: Tensor
    row_splits: Tensor

    def __post_init__(self) -> None:
        if self.values.ndim != 1:
            raise TorchPolicyError("candidate logits must be one-dimensional")
        if self.row_splits.ndim != 1 or self.row_splits.numel() == 0:
            raise TorchPolicyError("candidate row_splits must be a non-empty vector")
        if self.row_splits.dtype is not torch.long:
            raise TorchPolicyError("candidate row_splits must use torch.long")
        if int(self.row_splits[0].item()) != 0:
            raise TorchPolicyError("candidate row_splits must start at zero")
        if int(self.row_splits[-1].item()) != self.values.numel():
            raise TorchPolicyError("candidate row_splits do not end at logits length")

    @property
    def row_count(self) -> int:
        return self.row_splits.numel() - 1

    def greedy_ordinals(self) -> list[int]:
        """Return observation-local argmax ordinals for a driver policy call."""

        ordinals: list[int] = []
        splits = self.row_splits.detach().cpu().tolist()
        for start, end in zip(splits[:-1], splits[1:], strict=True):
            if start == end:
                raise TorchPolicyError("cannot choose from an empty candidate row")
            ordinal = int(torch.argmax(self.values[start:end]).item())
            ordinals.append(ordinal)
        return ordinals


class _RelationLayer(nn.Module):
    def __init__(self, hidden_dim: int, relation_kind_size: int) -> None:
        super().__init__()
        self.relation = nn.Embedding(relation_kind_size, hidden_dim)
        self.outgoing = nn.Linear(hidden_dim, hidden_dim, bias=False)
        self.incoming = nn.Linear(hidden_dim, hidden_dim, bias=False)
        self.project = nn.Linear(hidden_dim, hidden_dim)
        self.norm = nn.LayerNorm(hidden_dim)

    def forward(
        self,
        state: Tensor,
        source: Tensor,
        relation: Tensor,
        target: Tensor,
    ) -> Tensor:
        if source.numel() == 0:
            return state
        relation_state = self.relation(relation)
        messages = torch.zeros_like(state)
        messages.index_add_(
            0,
            source,
            self.outgoing(state[target]) + relation_state,
        )
        messages.index_add_(
            0,
            target,
            self.incoming(state[source]) + relation_state,
        )
        degree = torch.zeros(
            state.shape[0],
            dtype=state.dtype,
            device=state.device,
        )
        ones = torch.ones(source.shape[0], dtype=state.dtype, device=state.device)
        degree.index_add_(0, source, ones)
        degree.index_add_(0, target, ones)
        messages = messages / degree.clamp_min(1.0).sqrt().unsqueeze(1)
        return self.norm(state + torch.nn.functional.silu(self.project(messages)))


class RaggedCandidateScorer(nn.Module):
    """Score every candidate in one sparse, ragged semantic graph batch."""

    def __init__(
        self,
        schema: SemanticSchemaDimensions,
        config: RaggedScorerConfig | None = None,
    ) -> None:
        super().__init__()
        self.schema = schema
        self.config = config or RaggedScorerConfig()
        hidden_dim = self.config.hidden_dim

        self.token_kind = nn.Embedding(schema.token_kind_size, hidden_dim)
        self.categorical_value = nn.Embedding(
            schema.categorical_vocabulary_size,
            hidden_dim,
        )
        self.scalar_bias = nn.Embedding(schema.scalar_field_size, hidden_dim)
        self.scalar_weight = nn.Embedding(schema.scalar_field_size, hidden_dim)
        self.relation_layers = nn.ModuleList(
            _RelationLayer(hidden_dim, schema.relation_kind_size)
            for _ in range(self.config.relation_layers)
        )
        self.scorer = nn.Sequential(
            nn.Linear(hidden_dim * 2, hidden_dim),
            nn.SiLU(),
            nn.Linear(hidden_dim, 1),
        )
        self.register_buffer(
            "_categorical_offsets",
            torch.tensor(schema.categorical_offsets, dtype=torch.long),
            persistent=False,
        )
        self.register_buffer(
            "_categorical_sizes",
            torch.tensor(schema.categorical_vocabulary_sizes, dtype=torch.long),
            persistent=False,
        )
        for name, values in (
            ("_valid_token_kinds", schema.valid_token_kinds),
            ("_valid_categorical_fields", schema.valid_categorical_fields),
            ("_valid_scalar_fields", schema.valid_scalar_fields),
            ("_valid_relation_kinds", schema.valid_relation_kinds),
        ):
            self.register_buffer(
                name,
                torch.tensor(values, dtype=torch.bool),
                persistent=False,
            )

    @classmethod
    def from_bridge_schema(
        cls,
        schema: Mapping[str, object],
        config: RaggedScorerConfig | None = None,
    ) -> RaggedCandidateScorer:
        return cls(SemanticSchemaDimensions.from_bridge_schema(schema), config)

    def forward(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits:
        device = self.token_kind.weight.device
        semantic = _mapping(_required(decision_batch, "semantic"), "semantic")
        batch_version = _non_negative_integer(
            _required(semantic, "schema_version"),
            "semantic schema_version",
        )
        if batch_version != self.schema.version:
            raise TorchPolicyError(
                f"semantic schema version {batch_version} does not match model "
                f"version {self.schema.version}"
            )

        slot_indices = _index_tensor(
            _required(decision_batch, "slot_indices"),
            device,
            "slot_indices",
        )
        row_count = slot_indices.numel()
        token = _mapping(_required(semantic, "token"), "semantic.token")
        token_kind = _index_tensor(
            _required(token, "kind"),
            device,
            "semantic.token.kind",
        )
        token_splits = _index_tensor(
            _required(token, "row_splits"),
            device,
            "semantic.token.row_splits",
        )
        candidate_splits = _index_tensor(
            _required(decision_batch, "candidate_row_splits"),
            device,
            "candidate_row_splits",
        )
        candidate_tokens = _index_tensor(
            _required(semantic, "candidate_token_indices"),
            device,
            "semantic.candidate_token_indices",
        )
        _validate_splits(token_splits, row_count, token_kind.numel(), "token")
        _validate_splits(
            candidate_splits,
            row_count,
            candidate_tokens.numel(),
            "candidate",
            require_non_empty=True,
        )
        _validate_schema_ids(token_kind, self._valid_token_kinds, "token kind")
        _validate_range(candidate_tokens, token_kind.numel(), "candidate token")

        candidate_counts = _index_tensor(
            _required(decision_batch, "candidate_counts"),
            device,
            "candidate_counts",
        )
        if candidate_counts.shape != (row_count,) or not torch.equal(
            candidate_counts,
            candidate_splits[1:] - candidate_splits[:-1],
        ):
            raise TorchPolicyError("candidate_counts disagree with candidate_row_splits")

        token_row_ids = _row_ids(token_splits)
        candidate_row_ids = _row_ids(candidate_splits)
        if not torch.equal(token_row_ids[candidate_tokens], candidate_row_ids):
            raise TorchPolicyError("a candidate token escapes its decision row")

        state = self.token_kind(token_kind)
        state = state + self._categorical_state(semantic, token_kind.numel(), device)
        state = state + self._scalar_state(semantic, token_kind.numel(), device)

        relation_table = _mapping(
            _required(semantic, "relation"),
            "semantic.relation",
        )
        source = _index_tensor(
            _required(relation_table, "source_token_indices"),
            device,
            "semantic.relation.source_token_indices",
        )
        relation = _index_tensor(
            _required(relation_table, "relation"),
            device,
            "semantic.relation.relation",
        )
        target = _index_tensor(
            _required(relation_table, "target_token_indices"),
            device,
            "semantic.relation.target_token_indices",
        )
        _validate_aligned((source, relation, target), "relation")
        _validate_range(source, token_kind.numel(), "relation source token")
        _validate_range(target, token_kind.numel(), "relation target token")
        _validate_schema_ids(relation, self._valid_relation_kinds, "relation kind")
        if source.numel() and not torch.equal(token_row_ids[source], token_row_ids[target]):
            raise TorchPolicyError("a semantic relation escapes its decision row")
        for layer in self.relation_layers:
            state = layer(state, source, relation, target)

        row_sum = torch.zeros(
            (row_count, self.config.hidden_dim),
            dtype=state.dtype,
            device=device,
        )
        row_sum.index_add_(0, token_row_ids, state)
        row_lengths = (token_splits[1:] - token_splits[:-1]).clamp_min(1)
        row_state = row_sum / row_lengths.to(state.dtype).unsqueeze(1)
        score_inputs = torch.cat(
            (state[candidate_tokens], row_state[candidate_row_ids]),
            dim=1,
        )
        values = self.scorer(score_inputs).squeeze(1)
        return RaggedCandidateLogits(values=values, row_splits=candidate_splits)

    def _categorical_state(
        self,
        semantic: Mapping[str, object],
        token_count: int,
        device: torch.device,
    ) -> Tensor:
        table = _mapping(_required(semantic, "categorical"), "semantic.categorical")
        token_indices = _index_tensor(
            _required(table, "token_indices"),
            device,
            "semantic.categorical.token_indices",
        )
        field = _index_tensor(
            _required(table, "field"),
            device,
            "semantic.categorical.field",
        )
        value = _index_tensor(
            _required(table, "value"),
            device,
            "semantic.categorical.value",
        )
        _validate_aligned((token_indices, field, value), "categorical")
        _validate_range(token_indices, token_count, "categorical token")
        _validate_schema_ids(
            field,
            self._valid_categorical_fields,
            "categorical field",
        )
        if value.numel() and bool(torch.any(value >= self._categorical_sizes[field])):
            raise TorchPolicyError("categorical value exceeds its schema vocabulary")
        encoded = self.categorical_value(self._categorical_offsets[field] + value)
        return _mean_by_token(encoded, token_indices, token_count)

    def _scalar_state(
        self,
        semantic: Mapping[str, object],
        token_count: int,
        device: torch.device,
    ) -> Tensor:
        table = _mapping(_required(semantic, "scalar"), "semantic.scalar")
        token_indices = _index_tensor(
            _required(table, "token_indices"),
            device,
            "semantic.scalar.token_indices",
        )
        field = _index_tensor(
            _required(table, "field"),
            device,
            "semantic.scalar.field",
        )
        value = _float_tensor(
            _required(table, "value"),
            device,
            "semantic.scalar.value",
        )
        _validate_aligned((token_indices, field, value), "scalar")
        _validate_range(token_indices, token_count, "scalar token")
        _validate_schema_ids(field, self._valid_scalar_fields, "scalar field")
        if not bool(torch.all(torch.isfinite(value))):
            raise TorchPolicyError("scalar values must be finite")
        encoded = self.scalar_bias(field) + self.scalar_weight(field) * value.unsqueeze(1)
        return _mean_by_token(encoded, token_indices, token_count)


class GreedyTorchPolicy:
    """Driver adapter that performs one batched scorer call per decision round."""

    def __init__(
        self,
        scorer: RaggedCandidateScorer,
        behavior_manifest_id: BehaviorManifestId,
    ) -> None:
        if not isinstance(behavior_manifest_id, BehaviorManifestId):
            raise TorchPolicyError("greedy policy requires a BehaviorManifestId")
        self.scorer = scorer
        self.behavior_manifest_id = behavior_manifest_id

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        with torch.inference_mode():
            ordinals = self.scorer(decision_batch).greedy_ordinals()
        return BatchPolicyChoice.create(ordinals, self.behavior_manifest_id)


def ragged_cross_entropy(
    logits: RaggedCandidateLogits,
    target_ordinals: Sequence[int] | Tensor,
) -> Tensor:
    """Mean row-wise cross entropy without padding the candidate surface."""

    targets = torch.as_tensor(
        target_ordinals,
        dtype=torch.long,
        device=logits.values.device,
    )
    if targets.ndim != 1 or targets.numel() != logits.row_count:
        raise TorchPolicyError("target ordinals must contain one value per row")
    lengths = logits.row_splits[1:] - logits.row_splits[:-1]
    if bool(torch.any(targets < 0)) or bool(torch.any(targets >= lengths)):
        raise TorchPolicyError("target ordinal is outside its candidate row")

    row_ids = _row_ids(logits.row_splits)
    row_max = torch.full(
        (logits.row_count,),
        -torch.inf,
        dtype=logits.values.dtype,
        device=logits.values.device,
    )
    row_max.scatter_reduce_(0, row_ids, logits.values.detach(), reduce="amax")
    exponentials = torch.exp(logits.values - row_max[row_ids])
    row_sum = torch.zeros_like(row_max)
    row_sum.index_add_(0, row_ids, exponentials)
    log_normalizer = row_max + torch.log(row_sum)
    target_indices = logits.row_splits[:-1] + targets
    return (log_normalizer - logits.values[target_indices]).mean()


def _mean_by_token(values: Tensor, token_indices: Tensor, token_count: int) -> Tensor:
    result = torch.zeros(
        (token_count, values.shape[1]),
        dtype=values.dtype,
        device=values.device,
    )
    if token_indices.numel() == 0:
        return result
    result.index_add_(0, token_indices, values)
    counts = torch.zeros(token_count, dtype=values.dtype, device=values.device)
    counts.index_add_(
        0,
        token_indices,
        torch.ones(token_indices.shape[0], dtype=values.dtype, device=values.device),
    )
    return result / counts.clamp_min(1.0).unsqueeze(1)


def _row_ids(row_splits: Tensor) -> Tensor:
    lengths = row_splits[1:] - row_splits[:-1]
    return torch.repeat_interleave(
        torch.arange(lengths.numel(), device=row_splits.device),
        lengths,
    )


def _validate_splits(
    splits: Tensor,
    row_count: int,
    flat_count: int,
    name: str,
    *,
    require_non_empty: bool = False,
) -> None:
    if splits.shape != (row_count + 1,):
        raise TorchPolicyError(f"{name} row_splits have the wrong length")
    if int(splits[0].item()) != 0 or int(splits[-1].item()) != flat_count:
        raise TorchPolicyError(f"{name} row_splits do not bound the flat table")
    lengths = splits[1:] - splits[:-1]
    if bool(torch.any(lengths < 0)):
        raise TorchPolicyError(f"{name} row_splits are not monotonic")
    if require_non_empty and bool(torch.any(lengths == 0)):
        raise TorchPolicyError(f"{name} rows must not be empty")


def _validate_range(values: Tensor, upper_bound: int, name: str) -> None:
    if values.numel() and (
        bool(torch.any(values < 0)) or bool(torch.any(values >= upper_bound))
    ):
        raise TorchPolicyError(f"{name} is outside 0..{upper_bound}")


def _validate_schema_ids(values: Tensor, valid: Tensor, name: str) -> None:
    _validate_range(values, valid.numel(), name)
    if values.numel() and not bool(torch.all(valid[values])):
        raise TorchPolicyError(f"{name} uses a reserved schema id")


def _validate_aligned(values: tuple[Tensor, ...], name: str) -> None:
    if any(value.ndim != 1 for value in values):
        raise TorchPolicyError(f"{name} columns must be one-dimensional")
    if len({value.numel() for value in values}) != 1:
        raise TorchPolicyError(f"{name} columns are misaligned")


def _index_tensor(value: object, device: torch.device, name: str) -> Tensor:
    try:
        result = torch.as_tensor(
            _writable_numpy_copy(value),
            dtype=torch.long,
            device=device,
        )
    except (TypeError, ValueError, RuntimeError, OverflowError) as error:
        raise TorchPolicyError(f"{name} is not an integer tensor") from error
    if result.ndim != 1:
        raise TorchPolicyError(f"{name} must be one-dimensional")
    return result


def _float_tensor(value: object, device: torch.device, name: str) -> Tensor:
    try:
        result = torch.as_tensor(
            _writable_numpy_copy(value),
            dtype=torch.float32,
            device=device,
        )
    except (TypeError, ValueError, RuntimeError, OverflowError) as error:
        raise TorchPolicyError(f"{name} is not a float tensor") from error
    if result.ndim != 1:
        raise TorchPolicyError(f"{name} must be one-dimensional")
    return result


def _writable_numpy_copy(value: object) -> object:
    if isinstance(value, np.ndarray) and not value.flags.writeable:
        return value.copy()
    return value


def _enum_dimension(
    schema: Mapping[str, object],
    key: str,
) -> tuple[int, tuple[bool, ...]]:
    values = _mapping(_required(schema, key), key).values()
    numeric = {_non_negative_integer(value, f"{key} id") for value in values}
    if not numeric:
        raise TorchPolicyError(f"{key} must not be empty")
    dimension = max(numeric) + 1
    valid = tuple(index in numeric for index in range(dimension))
    return dimension, valid


def _mapping(value: object, name: str) -> Mapping[object, object]:
    if not isinstance(value, Mapping):
        raise TorchPolicyError(f"{name} must be a mapping")
    return value


def _required(mapping: Mapping[object, object], key: str) -> object:
    try:
        return mapping[key]
    except KeyError as error:
        raise TorchPolicyError(f"missing required field {key}") from error


def _non_negative_integer(value: object, name: str) -> int:
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchPolicyError(f"{name} must be an integer") from error
    if normalized < 0:
        raise TorchPolicyError(f"{name} must be non-negative")
    return normalized


def _positive_integer(value: object, name: str) -> int:
    normalized = _non_negative_integer(value, name)
    if normalized == 0:
        raise TorchPolicyError(f"{name} must be positive")
    return normalized
