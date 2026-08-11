"""Optional PyTorch policy components over the bridge-owned semantic graph.

This module is deliberately not imported by :mod:`sts_learning`.  Importing the
ordinary caller package therefore continues to require only NumPy; callers that
want this scorer opt into PyTorch and import this module explicitly.
"""

from __future__ import annotations

import math
import operator
import struct
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from numbers import Real

import numpy as np
import torch
from torch import Tensor, nn

from .manifests import (
    BehaviorRuleBinding,
    ManifestArtifactId,
    ManifestArtifactKind,
)
from .policy import (
    BatchPolicyChoice,
    BehaviorManifestId,
    SelectionProbability,
)


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
    identity_residual_categorical_fields: tuple[int, ...]

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

        raw_identity_fields = _mapping(
            _required(schema, "identity_residual_categorical_fields"),
            "identity_residual_categorical_fields",
        )
        if any(
            type(enabled) is not int or enabled != 1
            for enabled in raw_identity_fields.values()
        ):
            raise TorchPolicyError(
                "identity_residual_categorical_fields values must be integer one"
            )
        identity_fields = tuple(
            _non_negative_integer(field, "identity residual categorical field")
            for field in raw_identity_fields.keys()
        )
        if len(set(identity_fields)) != len(identity_fields):
            raise TorchPolicyError(
                "identity_residual_categorical_fields repeats a field"
            )
        if not set(identity_fields).issubset(expected_fields):
            raise TorchPolicyError(
                "identity_residual_categorical_fields contains an unknown field"
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
            identity_residual_categorical_fields=identity_fields,
        )


@dataclass(frozen=True)
class RaggedScorerConfig:
    """Small backend configuration; it contains no game feature dictionary."""

    hidden_dim: int = 64
    relation_layers: int = 2
    value_head: bool = False
    value_head_width: int = 1

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
        if type(self.value_head) is not bool:
            raise TorchPolicyError("value_head must be bool")
        object.__setattr__(
            self,
            "value_head_width",
            _positive_integer(self.value_head_width, "value_head_width"),
        )
        if self.value_head_width > 64:
            raise TorchPolicyError("value_head_width must not exceed 64")
        if not self.value_head and self.value_head_width != 1:
            raise TorchPolicyError(
                "value_head_width must be one when the value head is disabled"
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


@dataclass(frozen=True)
class RaggedActorCriticOutput:
    """Candidate policy logits and one scalar value per decision row."""

    logits: RaggedCandidateLogits
    row_values: Tensor

    def __post_init__(self) -> None:
        if not isinstance(self.logits, RaggedCandidateLogits):
            raise TorchPolicyError("actor-critic logits must be ragged")
        if self.row_values.ndim != 1:
            raise TorchPolicyError("actor-critic values must be one-dimensional")
        if self.row_values.numel() != self.logits.row_count:
            raise TorchPolicyError("actor-critic values must align to decision rows")


@dataclass(frozen=True)
class RaggedMultiActorCriticOutput:
    """Candidate logits and fixed-semantic value columns per decision row."""

    logits: RaggedCandidateLogits
    row_values: Tensor

    def __post_init__(self) -> None:
        if not isinstance(self.logits, RaggedCandidateLogits):
            raise TorchPolicyError("multi actor-critic logits must be ragged")
        if self.row_values.ndim != 2 or self.row_values.shape[1] <= 0:
            raise TorchPolicyError(
                "multi actor-critic values must be a non-empty matrix"
            )
        if self.row_values.shape[0] != self.logits.row_count:
            raise TorchPolicyError(
                "multi actor-critic values must align to decision rows"
            )


_RAGGED_CATEGORICAL_RULE_V1 = ManifestArtifactId.from_content(
    ManifestArtifactKind.BEHAVIOR_RULE,
    b"sts_learning.ragged_categorical_inverse_cdf\x00v1",
)


@dataclass(frozen=True)
class RaggedCategoricalPolicyConfig:
    """Temperature-scaled categorical behavior with canonical provenance."""

    temperature: float = 1.0

    def __post_init__(self) -> None:
        if isinstance(self.temperature, bool) or not isinstance(self.temperature, Real):
            raise TorchPolicyError("categorical temperature must be a real number")
        normalized = float(self.temperature)
        if not math.isfinite(normalized) or normalized <= 0.0:
            raise TorchPolicyError("categorical temperature must be finite and positive")
        object.__setattr__(self, "temperature", normalized)

    @property
    def behavior_rule(self) -> BehaviorRuleBinding:
        payload = b"sts_learning.ragged_categorical_temperature\x00v1" + struct.pack(
            ">d",
            self.temperature,
        )
        return BehaviorRuleBinding(
            implementation=_RAGGED_CATEGORICAL_RULE_V1,
            configuration=ManifestArtifactId.from_content(
                ManifestArtifactKind.BEHAVIOR_RULE_CONFIG,
                payload,
            ),
        )


@dataclass(frozen=True)
class RaggedCategoricalSample:
    """Aligned sampled ordinals and their probabilities at selection time."""

    ordinals: tuple[int, ...]
    selection_probabilities: tuple[SelectionProbability, ...]


def sample_ragged_categorical(
    logits: RaggedCandidateLogits,
    config: RaggedCategoricalPolicyConfig,
    generator: torch.Generator,
) -> RaggedCategoricalSample:
    """Sample every legal ragged row from one caller-owned random stream."""

    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchPolicyError("categorical sampling requires RaggedCandidateLogits")
    if not isinstance(config, RaggedCategoricalPolicyConfig):
        raise TorchPolicyError("categorical sampling requires typed config")
    if not isinstance(generator, torch.Generator):
        raise TorchPolicyError("categorical sampling requires a caller-owned generator")
    if generator is torch.default_generator:
        raise TorchPolicyError("categorical sampling refuses the global generator")
    generator_device = torch.device(generator.device)
    if generator_device.type != logits.values.device.type:
        raise TorchPolicyError(
            "categorical generator device type must match candidate logits"
        )
    probability_rows = _categorical_probability_rows(logits, config)

    uniforms = torch.rand(
        (len(probability_rows),),
        dtype=torch.float64,
        device=logits.values.device,
        generator=generator,
    )
    return _sample_probability_rows(probability_rows, uniforms)


def sample_ragged_categorical_rows(
    logits: RaggedCandidateLogits,
    config: RaggedCategoricalPolicyConfig,
    generators: Sequence[torch.Generator],
) -> RaggedCategoricalSample:
    """Sample each ragged row from its own caller-owned random stream."""

    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchPolicyError("categorical sampling requires RaggedCandidateLogits")
    if not isinstance(config, RaggedCategoricalPolicyConfig):
        raise TorchPolicyError("categorical sampling requires typed config")
    row_generators = tuple(generators)
    if len(row_generators) != logits.row_count:
        raise TorchPolicyError(
            "categorical row generators must align to decision rows"
        )
    if not all(isinstance(generator, torch.Generator) for generator in row_generators):
        raise TorchPolicyError(
            "categorical row sampling requires caller-owned generators"
        )
    if any(generator is torch.default_generator for generator in row_generators):
        raise TorchPolicyError("categorical sampling refuses the global generator")
    if len({id(generator) for generator in row_generators}) != len(row_generators):
        raise TorchPolicyError(
            "categorical row sampling requires independent generators"
        )
    logits_device = logits.values.device.type
    if any(
        torch.device(generator.device).type != logits_device
        for generator in row_generators
    ):
        raise TorchPolicyError(
            "categorical generator device type must match candidate logits"
        )

    probability_rows = _categorical_probability_rows(logits, config)
    uniforms = tuple(
        torch.rand(
            (),
            dtype=torch.float64,
            device=logits.values.device,
            generator=generator,
        )
        for generator in row_generators
    )
    return _sample_probability_rows(probability_rows, uniforms)


def _categorical_probability_rows(
    logits: RaggedCandidateLogits,
    config: RaggedCategoricalPolicyConfig,
) -> tuple[Tensor, ...]:
    if not bool(torch.all(torch.isfinite(logits.values))):
        raise TorchPolicyError("categorical candidate logits must be finite")

    splits = logits.row_splits.detach().cpu().tolist()
    probability_rows: list[Tensor] = []
    for start, end in zip(splits[:-1], splits[1:], strict=True):
        if start >= end:
            raise TorchPolicyError(
                "categorical sampling requires non-empty increasing rows"
            )
        row = logits.values[start:end].detach().to(dtype=torch.float64)
        probabilities = torch.softmax(row / config.temperature, dim=0)
        if not bool(torch.all(torch.isfinite(probabilities))):
            raise TorchPolicyError("categorical probabilities must be finite")
        if not bool(torch.any(probabilities > 0.0)):
            raise TorchPolicyError("categorical row has no positive probability")
        probability_rows.append(probabilities)
    return tuple(probability_rows)


def _sample_probability_rows(
    probability_rows: Sequence[Tensor],
    uniforms: Sequence[Tensor] | Tensor,
) -> RaggedCategoricalSample:
    ordinals: list[int] = []
    selected_probabilities: list[SelectionProbability] = []
    for probabilities, uniform in zip(
        probability_rows,
        uniforms,
        strict=True,
    ):
        cumulative = torch.cumsum(probabilities, dim=0)
        cumulative[-1] = 1.0
        ordinal = int(torch.searchsorted(cumulative, uniform, right=True).item())
        probability = float(probabilities[ordinal].item())
        ordinals.append(ordinal)
        selected_probabilities.append(SelectionProbability.known(probability))
    return RaggedCategoricalSample(
        ordinals=tuple(ordinals),
        selection_probabilities=tuple(selected_probabilities),
    )


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
        with torch.no_grad():
            for field in schema.identity_residual_categorical_fields:
                start = schema.categorical_offsets[field]
                end = start + schema.categorical_vocabulary_sizes[field]
                self.categorical_value.weight[start:end].zero_()
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
        self.value_head = None
        if self.config.value_head:
            self.value_head = nn.Sequential(
                nn.Linear(hidden_dim, hidden_dim),
                nn.SiLU(),
                nn.Linear(hidden_dim, self.config.value_head_width),
            )
            final = self.value_head[-1]
            assert isinstance(final, nn.Linear)
            nn.init.zeros_(final.weight)
            nn.init.zeros_(final.bias)
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
        logits, _ = self._forward_encoded(decision_batch)
        return logits

    def actor_critic(
        self,
        decision_batch: Mapping[str, object],
    ) -> RaggedActorCriticOutput:
        if self.value_head is None:
            raise TorchPolicyError("actor-critic output requires a value head")
        if self.config.value_head_width != 1:
            raise TorchPolicyError(
                "scalar actor-critic output requires value_head_width one"
            )
        logits, row_state = self._forward_encoded(decision_batch)
        return RaggedActorCriticOutput(
            logits=logits,
            row_values=self.value_head(row_state).squeeze(1),
        )

    def actor_critic_multi(
        self,
        decision_batch: Mapping[str, object],
    ) -> RaggedMultiActorCriticOutput:
        if self.value_head is None:
            raise TorchPolicyError("multi actor-critic output requires a value head")
        logits, row_state = self._forward_encoded(decision_batch)
        return RaggedMultiActorCriticOutput(
            logits=logits,
            row_values=self.value_head(row_state),
        )

    def _forward_encoded(
        self,
        decision_batch: Mapping[str, object],
    ) -> tuple[RaggedCandidateLogits, Tensor]:
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
        return (
            RaggedCandidateLogits(values=values, row_splits=candidate_splits),
            row_state,
        )

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


def load_scorer_warm_start(
    target: RaggedCandidateScorer,
    source: RaggedCandidateScorer,
    *,
    actor_only: bool = False,
) -> None:
    """Copy a compatible scorer, optionally resetting objective-local value state."""

    if not isinstance(target, RaggedCandidateScorer) or not isinstance(
        source,
        RaggedCandidateScorer,
    ):
        raise TorchPolicyError("scorer warm start requires typed scorers")
    if type(actor_only) is not bool:
        raise TorchPolicyError("scorer warm-start actor_only must be bool")
    try:
        same_value_profile = (
            target.config.value_head == source.config.value_head
            and target.config.value_head_width == source.config.value_head_width
        )
        if not actor_only and same_value_profile:
            target.load_state_dict(source.state_dict(), strict=True)
            return
        actor_state = {
            key: value
            for key, value in source.state_dict().items()
            if not key.startswith("value_head.")
        }
        incompatible = target.load_state_dict(actor_state, strict=False)
        expected_missing = {
            key
            for key in target.state_dict()
            if key.startswith("value_head.")
        }
        if (
            incompatible.unexpected_keys
            or set(incompatible.missing_keys) != expected_missing
        ):
            raise RuntimeError("actor-only warm start changed shared policy keys")
    except RuntimeError as error:
        raise TorchPolicyError(
            "warm-start scorer is incompatible with the maintained profile"
        ) from error


def configure_critic_only_training(scorer: RaggedCandidateScorer) -> None:
    """Freeze the complete actor path and leave only the scalar value head live."""

    if not isinstance(scorer, RaggedCandidateScorer):
        raise TorchPolicyError("critic-only training requires a typed scorer")
    if scorer.value_head is None or scorer.config.value_head_width != 1:
        raise TorchPolicyError("critic-only training requires one scalar value head")
    value_parameters = 0
    for name, parameter in scorer.named_parameters():
        is_value_parameter = name.startswith("value_head.")
        parameter.requires_grad_(is_value_parameter)
        value_parameters += int(is_value_parameter)
    if value_parameters == 0:
        raise TorchPolicyError("critic-only training found no value parameters")


def require_matching_actor_state(
    left: RaggedCandidateScorer,
    right: RaggedCandidateScorer,
) -> None:
    """Reject a critic initialization that changed any actor-owned tensor."""

    if not isinstance(left, RaggedCandidateScorer) or not isinstance(
        right,
        RaggedCandidateScorer,
    ):
        raise TorchPolicyError("actor-state comparison requires typed scorers")
    if (
        left.schema != right.schema
        or left.config.hidden_dim != right.config.hidden_dim
        or left.config.relation_layers != right.config.relation_layers
    ):
        raise TorchPolicyError("critic initialization changed the actor definition")
    left_state = {
        name: tensor
        for name, tensor in left.state_dict().items()
        if not name.startswith("value_head.")
    }
    right_state = {
        name: tensor
        for name, tensor in right.state_dict().items()
        if not name.startswith("value_head.")
    }
    if left_state.keys() != right_state.keys():
        raise TorchPolicyError("critic initialization changed actor state keys")
    for name in left_state:
        left_tensor = left_state[name].detach().cpu()
        right_tensor = right_state[name].detach().cpu()
        if not torch.equal(left_tensor, right_tensor):
            raise TorchPolicyError(
                f"critic initialization changed actor tensor {name!r}"
            )


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
        return BatchPolicyChoice.deterministic(ordinals, self.behavior_manifest_id)


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
