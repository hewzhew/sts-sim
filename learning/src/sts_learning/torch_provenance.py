"""Canonical provenance for the maintained categorical PyTorch baseline."""

from __future__ import annotations

import math
import operator
import struct
from collections.abc import Iterable, Mapping
from dataclasses import dataclass

import torch

from .combat_objective import CombatPolicyUpdateRule, CombatWinObjectiveConfig
from .manifests import (
    BehaviorManifestTemplate,
    ManifestArtifactId,
    ManifestArtifactKind,
)
from .torch_policy import (
    RaggedCategoricalPolicyConfig,
    RaggedScorerConfig,
    SemanticSchemaDimensions,
)
from .terminal_returns import (
    OnPolicyObjectiveConfig,
    RunPolicyUpdateRule,
)


class TorchProvenanceError(ValueError):
    """A runtime profile cannot be represented by exact canonical provenance."""


_MODEL_DEFINITION_VERSION = 1
_MODEL_CONFIG_VERSION = 1
_ACTOR_CRITIC_MODEL_DEFINITION_VERSION = 2
_ACTOR_CRITIC_MODEL_CONFIG_VERSION = 2
_SEMANTIC_SCHEMA_ENCODING_VERSION = 1
_OPTIMIZER_CONFIG_VERSION = 1
_TRAINER_IMPLEMENTATION_VERSION = 4
_TERMINAL_RETURN_CONFIG_VERSION = 1
_RUN_VALUE_PPO_TRAINER_IMPLEMENTATION_VERSION = 2
_RUN_VALUE_PPO_OBJECTIVE_VERSION = 2
_COMBAT_WIN_TRAINER_IMPLEMENTATION_VERSION = 3
_COMBAT_WIN_OBJECTIVE_VERSION = 3
_COMBAT_PPO_TRAINER_IMPLEMENTATION_VERSION = 4
_COMBAT_PPO_OBJECTIVE_VERSION = 4
_COMBAT_VALUE_PPO_TRAINER_IMPLEMENTATION_VERSION = 1
_COMBAT_VALUE_PPO_OBJECTIVE_VERSION = 1
_MAX_SCHEMA_BYTES = 1 << 20
_MAX_SCHEMA_DEPTH = 16
_MAX_SCHEMA_ITEMS = 100_000


@dataclass(frozen=True)
class AdamTrainingConfig:
    """Exact Adam configuration used by the online policy trainer."""

    learning_rate: float = 1e-3
    beta1: float = 0.9
    beta2: float = 0.999
    epsilon: float = 1e-8
    weight_decay: float = 0.0
    amsgrad: bool = False

    def __post_init__(self) -> None:
        learning_rate = _finite_float(self.learning_rate, "learning_rate")
        beta1 = _finite_float(self.beta1, "beta1")
        beta2 = _finite_float(self.beta2, "beta2")
        epsilon = _finite_float(self.epsilon, "epsilon")
        weight_decay = _finite_float(self.weight_decay, "weight_decay")
        if learning_rate <= 0.0:
            raise TorchProvenanceError("learning_rate must be positive")
        if not 0.0 <= beta1 < 1.0 or not 0.0 <= beta2 < 1.0:
            raise TorchProvenanceError("Adam beta values must be in [0, 1)")
        if epsilon <= 0.0:
            raise TorchProvenanceError("epsilon must be positive")
        if weight_decay < 0.0:
            raise TorchProvenanceError("weight_decay must be non-negative")
        if type(self.amsgrad) is not bool:
            raise TorchProvenanceError("amsgrad must be bool")
        object.__setattr__(self, "learning_rate", learning_rate)
        object.__setattr__(self, "beta1", beta1)
        object.__setattr__(self, "beta2", beta2)
        object.__setattr__(self, "epsilon", epsilon)
        object.__setattr__(self, "weight_decay", weight_decay)

    def create(self, parameters: Iterable[torch.Tensor]) -> torch.optim.Adam:
        """Create the optimizer whose implementation choices are all explicit."""

        return torch.optim.Adam(
            parameters,
            lr=self.learning_rate,
            betas=(self.beta1, self.beta2),
            eps=self.epsilon,
            weight_decay=self.weight_decay,
            amsgrad=self.amsgrad,
            foreach=False,
            maximize=False,
            capturable=False,
            differentiable=False,
            fused=False,
        )

    @property
    def artifact_id(self) -> ManifestArtifactId:
        flags = 1 if self.amsgrad else 0
        content = (
            b"STS-ADAM-CONFIG\x00"
            + struct.pack(
                ">I5dB",
                _OPTIMIZER_CONFIG_VERSION,
                self.learning_rate,
                self.beta1,
                self.beta2,
                self.epsilon,
                self.weight_decay,
                flags,
            )
            + _runtime_version_bytes()
        )
        return ManifestArtifactId.from_content(
            ManifestArtifactKind.OPTIMIZER_CONFIG,
            content,
        )


def categorical_training_manifest_template(
    semantic_schema: Mapping[str, object],
    scorer_config: RaggedScorerConfig,
    behavior_config: RaggedCategoricalPolicyConfig,
    optimizer_config: AdamTrainingConfig,
    objective_config: OnPolicyObjectiveConfig,
    *,
    device_type: str,
) -> BehaviorManifestTemplate:
    """Bind the exact maintained model, schema, optimizer, and trainer profile."""

    if not isinstance(objective_config, OnPolicyObjectiveConfig):
        raise TorchProvenanceError("objective_config must be typed")
    return _categorical_manifest_template(
        semantic_schema,
        scorer_config,
        behavior_config,
        optimizer_config,
        categorical_trainer_implementation(objective_config),
        device_type=device_type,
    )


def combat_win_training_manifest_template(
    semantic_schema: Mapping[str, object],
    scorer_config: RaggedScorerConfig,
    behavior_config: RaggedCategoricalPolicyConfig,
    optimizer_config: AdamTrainingConfig,
    objective_config: CombatWinObjectiveConfig,
    *,
    device_type: str,
) -> BehaviorManifestTemplate:
    """Bind the same scorer stack to the distinct same-root win trainer."""

    if not isinstance(objective_config, CombatWinObjectiveConfig):
        raise TorchProvenanceError("combat objective_config must be typed")
    return _categorical_manifest_template(
        semantic_schema,
        scorer_config,
        behavior_config,
        optimizer_config,
        combat_win_trainer_implementation(objective_config),
        device_type=device_type,
    )


def _categorical_manifest_template(
    semantic_schema: Mapping[str, object],
    scorer_config: RaggedScorerConfig,
    behavior_config: RaggedCategoricalPolicyConfig,
    optimizer_config: AdamTrainingConfig,
    trainer_implementation: ManifestArtifactId,
    *,
    device_type: str,
) -> BehaviorManifestTemplate:
    if not isinstance(scorer_config, RaggedScorerConfig):
        raise TorchProvenanceError("scorer_config must be typed")
    if not isinstance(behavior_config, RaggedCategoricalPolicyConfig):
        raise TorchProvenanceError("behavior_config must be typed")
    if not isinstance(optimizer_config, AdamTrainingConfig):
        raise TorchProvenanceError("optimizer_config must be typed")
    if not isinstance(trainer_implementation, ManifestArtifactId):
        raise TorchProvenanceError("trainer_implementation must be typed")
    if type(device_type) is not str or not device_type:
        raise TorchProvenanceError("device_type must be a non-empty string")
    try:
        dimensions = SemanticSchemaDimensions.from_bridge_schema(semantic_schema)
    except (TypeError, ValueError) as error:
        raise TorchProvenanceError(str(error)) from error

    runtime = _runtime_version_bytes()
    model_definition_version = (
        _ACTOR_CRITIC_MODEL_DEFINITION_VERSION
        if scorer_config.value_head
        else _MODEL_DEFINITION_VERSION
    )
    model_definition = ManifestArtifactId.from_content(
        ManifestArtifactKind.MODEL_DEFINITION,
        b"STS-RAGGED-CANDIDATE-SCORER\x00"
        + struct.pack(">I", model_definition_version)
        + runtime,
    )
    encoded_device = device_type.encode("utf-8")
    if len(encoded_device) > 255:
        raise TorchProvenanceError("device_type is too large")
    if scorer_config.value_head:
        encoded_model_config = struct.pack(
            ">IQQBB",
            _ACTOR_CRITIC_MODEL_CONFIG_VERSION,
            scorer_config.hidden_dim,
            scorer_config.relation_layers,
            1,
            len(encoded_device),
        )
    else:
        encoded_model_config = struct.pack(
            ">IQQB",
            _MODEL_CONFIG_VERSION,
            scorer_config.hidden_dim,
            scorer_config.relation_layers,
            len(encoded_device),
        )
    model_config = ManifestArtifactId.from_content(
        ManifestArtifactKind.MODEL_CONFIG,
        b"STS-RAGGED-SCORER-CONFIG\x00"
        + encoded_model_config
        + encoded_device,
    )
    schema_content = (
        b"STS-SEMANTIC-SCHEMA\x00"
        + struct.pack(">I", _SEMANTIC_SCHEMA_ENCODING_VERSION)
        + _canonical_schema_bytes(semantic_schema)
    )
    semantic_schema_id = ManifestArtifactId.from_content(
        ManifestArtifactKind.SEMANTIC_SCHEMA,
        schema_content,
    )
    return BehaviorManifestTemplate(
        model_definition=model_definition,
        model_config=model_config,
        behavior_rule=behavior_config.behavior_rule,
        semantic_schema=semantic_schema_id,
        optimizer_config=optimizer_config.artifact_id,
        trainer_implementation=trainer_implementation,
        semantic_schema_version=dimensions.version,
    )


def categorical_trainer_implementation(
    objective_config: OnPolicyObjectiveConfig,
) -> ManifestArtifactId:
    """Bind the exact objective, return, and attempts-per-update contract."""

    if not isinstance(objective_config, OnPolicyObjectiveConfig):
        raise TorchProvenanceError("objective_config must be typed")
    update = objective_config.policy_update
    if update.rule is RunPolicyUpdateRule.REINFORCE:
        return ManifestArtifactId.from_content(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION,
            b"STS-SYNCHRONOUS-TERMINAL-POLICY-TRAINER\x00"
            + struct.pack(">I", _TRAINER_IMPLEMENTATION_VERSION)
            + b"STS-FLOOR-PROGRESS-RETURN\x00"
            + struct.pack(
                ">IQQBB",
                _TERMINAL_RETURN_CONFIG_VERSION,
                objective_config.terminal_return.target_floor,
                objective_config.attempts_per_update,
                int(objective_config.advantage_mode),
                int(objective_config.decision_scope),
            )
            + _runtime_version_bytes(),
        )
    max_grad_norm = update.max_grad_norm
    target_kl = update.target_kl
    if update.normalize_advantage:
        trainer_version = _RUN_VALUE_PPO_TRAINER_IMPLEMENTATION_VERSION
        objective_encoding = struct.pack(
            ">IQQBBBBQdddBdBd",
            _RUN_VALUE_PPO_OBJECTIVE_VERSION,
            objective_config.terminal_return.target_floor,
            objective_config.attempts_per_update,
            int(objective_config.advantage_mode),
            int(objective_config.decision_scope),
            int(update.rule),
            int(update.normalize_advantage),
            update.epochs,
            update.clip_coefficient,
            update.entropy_coefficient,
            update.value_loss_coefficient,
            int(max_grad_norm is not None),
            0.0 if max_grad_norm is None else max_grad_norm,
            int(target_kl is not None),
            0.0 if target_kl is None else target_kl,
        )
    else:
        trainer_version = 1
        objective_encoding = struct.pack(
            ">IQQBBBQdddBdBd",
            1,
            objective_config.terminal_return.target_floor,
            objective_config.attempts_per_update,
            int(objective_config.advantage_mode),
            int(objective_config.decision_scope),
            int(update.rule),
            update.epochs,
            update.clip_coefficient,
            update.entropy_coefficient,
            update.value_loss_coefficient,
            int(max_grad_norm is not None),
            0.0 if max_grad_norm is None else max_grad_norm,
            int(target_kl is not None),
            0.0 if target_kl is None else target_kl,
        )
    return ManifestArtifactId.from_content(
        ManifestArtifactKind.TRAINER_IMPLEMENTATION,
        b"STS-SYNCHRONOUS-RUN-PPO-CLIP-VALUE-TRAINER\x00"
        + struct.pack(">I", trainer_version)
        + b"STS-FLOOR-PROGRESS-RETURN-PPO-CLIP-VALUE\x00"
        + objective_encoding
        + _runtime_version_bytes(),
    )


def combat_win_trainer_implementation(
    objective_config: CombatWinObjectiveConfig,
) -> ManifestArtifactId:
    """Bind same-root win-first/HP-fallback learning and update width."""

    if not isinstance(objective_config, CombatWinObjectiveConfig):
        raise TorchProvenanceError("combat objective_config must be typed")
    update = objective_config.policy_update
    if update.rule is CombatPolicyUpdateRule.REINFORCE:
        return ManifestArtifactId.from_content(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION,
            b"STS-SYNCHRONOUS-COMBAT-WIN-FIRST-POLICY-TRAINER\x00"
            + struct.pack(">I", _COMBAT_WIN_TRAINER_IMPLEMENTATION_VERSION)
            + b"STS-SAME-ROOT-WIN-FIRST-OPTIONAL-ALL-WIN-AXIS\x00"
            + struct.pack(
                ">IQB",
                _COMBAT_WIN_OBJECTIVE_VERSION,
                objective_config.groups_per_update,
                int(objective_config.all_win_axis),
            )
            + _runtime_version_bytes(),
        )
    if update.rule is CombatPolicyUpdateRule.PPO_CLIP_VALUE:
        max_grad_norm = update.max_grad_norm
        target_kl = update.target_kl
        return ManifestArtifactId.from_content(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION,
            b"STS-SYNCHRONOUS-COMBAT-PPO-CLIP-VALUE-TRAINER\x00"
            + struct.pack(
                ">I",
                _COMBAT_VALUE_PPO_TRAINER_IMPLEMENTATION_VERSION,
            )
            + b"STS-SAME-ROOT-WIN-FIRST-PPO-CLIP-VALUE\x00"
            + struct.pack(
                ">IQBBQdddBdBd",
                _COMBAT_VALUE_PPO_OBJECTIVE_VERSION,
                objective_config.groups_per_update,
                int(objective_config.all_win_axis),
                int(update.rule),
                update.epochs,
                update.clip_coefficient,
                update.entropy_coefficient,
                update.value_loss_coefficient,
                int(max_grad_norm is not None),
                0.0 if max_grad_norm is None else max_grad_norm,
                int(target_kl is not None),
                0.0 if target_kl is None else target_kl,
            )
            + _runtime_version_bytes(),
        )
    max_grad_norm = update.max_grad_norm
    target_kl = update.target_kl
    return ManifestArtifactId.from_content(
        ManifestArtifactKind.TRAINER_IMPLEMENTATION,
        b"STS-SYNCHRONOUS-COMBAT-PPO-CLIP-POLICY-TRAINER\x00"
        + struct.pack(">I", _COMBAT_PPO_TRAINER_IMPLEMENTATION_VERSION)
        + b"STS-SAME-ROOT-WIN-FIRST-PPO-CLIP\x00"
        + struct.pack(
            ">IQBBQddBdBd",
            _COMBAT_PPO_OBJECTIVE_VERSION,
            objective_config.groups_per_update,
            int(objective_config.all_win_axis),
            int(update.rule),
            update.epochs,
            update.clip_coefficient,
            update.entropy_coefficient,
            int(max_grad_norm is not None),
            0.0 if max_grad_norm is None else max_grad_norm,
            int(target_kl is not None),
            0.0 if target_kl is None else target_kl,
        )
        + _runtime_version_bytes(),
    )


def _canonical_schema_bytes(schema: Mapping[str, object]) -> bytes:
    if not isinstance(schema, Mapping):
        raise TorchProvenanceError("semantic schema must be a mapping")
    payload = _encode_schema_value(schema, depth=0)
    if len(payload) > _MAX_SCHEMA_BYTES:
        raise TorchProvenanceError("semantic schema exceeds its byte limit")
    return payload


def _encode_schema_value(value: object, *, depth: int) -> bytes:
    if depth > _MAX_SCHEMA_DEPTH:
        raise TorchProvenanceError("semantic schema exceeds its nesting limit")
    if type(value) is int:
        try:
            normalized = operator.index(value)
            return b"I" + struct.pack(">q", normalized)
        except (TypeError, struct.error) as error:
            raise TorchProvenanceError(
                "semantic schema integer is outside signed 64-bit range"
            ) from error
    if type(value) is str:
        encoded = value.encode("utf-8")
        if len(encoded) > _MAX_SCHEMA_BYTES:
            raise TorchProvenanceError("semantic schema string is too large")
        return b"S" + struct.pack(">I", len(encoded)) + encoded
    if isinstance(value, Mapping):
        if len(value) > _MAX_SCHEMA_ITEMS:
            raise TorchProvenanceError("semantic schema has too many entries")
        rows: list[tuple[bytes, bytes]] = []
        encoded_bytes = 5
        for key, item in value.items():
            if type(key) not in (int, str):
                raise TorchProvenanceError(
                    "semantic schema keys must be integers or strings"
                )
            encoded_key = _encode_schema_value(key, depth=depth + 1)
            encoded_value = _encode_schema_value(item, depth=depth + 1)
            encoded_bytes += len(encoded_key) + len(encoded_value)
            if encoded_bytes > _MAX_SCHEMA_BYTES:
                raise TorchProvenanceError(
                    "semantic schema exceeds its byte limit"
                )
            rows.append((encoded_key, encoded_value))
        rows.sort(key=lambda row: row[0])
        return (
            b"M"
            + struct.pack(">I", len(rows))
            + b"".join(key + item for key, item in rows)
        )
    raise TorchProvenanceError(
        f"semantic schema contains unsupported {type(value).__name__} value"
    )


def _runtime_version_bytes() -> bytes:
    encoded = str(torch.__version__).encode("utf-8")
    if len(encoded) > 255:
        raise TorchProvenanceError("PyTorch version string is too large")
    return struct.pack(">B", len(encoded)) + encoded


def _finite_float(value: object, name: str) -> float:
    if isinstance(value, bool):
        raise TorchProvenanceError(f"{name} must be a real number")
    try:
        normalized = float(value)
    except (TypeError, ValueError) as error:
        raise TorchProvenanceError(f"{name} must be a real number") from error
    if not math.isfinite(normalized):
        raise TorchProvenanceError(f"{name} must be finite")
    return normalized
