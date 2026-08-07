"""Bounded content-addressed provenance for behavior-policy checkpoints."""

from __future__ import annotations

import hashlib
import operator
import struct
from dataclasses import dataclass
from enum import IntEnum

from .policy import BehaviorManifestId


class BehaviorManifestError(ValueError):
    """A behavior manifest or registry operation is not exact and safe."""


class ManifestArtifactKind(IntEnum):
    """Typed external artifacts referenced by one behavior manifest."""

    MODEL_CHECKPOINT = 1
    MODEL_DEFINITION = 2
    MODEL_CONFIG = 3
    SEMANTIC_SCHEMA = 4
    OPTIMIZER_CONFIG = 5
    TRAINER_IMPLEMENTATION = 6


@dataclass(frozen=True, order=True)
class ManifestArtifactId:
    """SHA-256 content identity of an externally owned immutable artifact."""

    kind: ManifestArtifactKind
    digest: bytes

    def __post_init__(self) -> None:
        if not isinstance(self.kind, ManifestArtifactKind):
            raise BehaviorManifestError("manifest artifact kind must be typed")
        if not isinstance(self.digest, bytes):
            raise BehaviorManifestError("manifest artifact digest must be immutable bytes")
        if len(self.digest) != 32:
            raise BehaviorManifestError("manifest artifact digest must contain 32 bytes")

    @classmethod
    def from_content(
        cls,
        kind: ManifestArtifactKind,
        content: bytes,
    ) -> ManifestArtifactId:
        """Hash one immutable externally owned artifact without retaining it."""

        if not isinstance(content, bytes):
            raise BehaviorManifestError("manifest artifact content must be immutable bytes")
        return cls(kind, hashlib.sha256(content).digest())


@dataclass(frozen=True)
class BehaviorManifest:
    """Exact external identities needed to reproduce one behavior policy."""

    model_checkpoint: ManifestArtifactId
    model_definition: ManifestArtifactId
    model_config: ManifestArtifactId
    semantic_schema: ManifestArtifactId
    optimizer_config: ManifestArtifactId
    trainer_implementation: ManifestArtifactId
    semantic_schema_version: int
    training_step: int

    def __post_init__(self) -> None:
        for field_name, expected_kind in (
            ("model_checkpoint", ManifestArtifactKind.MODEL_CHECKPOINT),
            ("model_definition", ManifestArtifactKind.MODEL_DEFINITION),
            ("model_config", ManifestArtifactKind.MODEL_CONFIG),
            ("semantic_schema", ManifestArtifactKind.SEMANTIC_SCHEMA),
            ("optimizer_config", ManifestArtifactKind.OPTIMIZER_CONFIG),
            ("trainer_implementation", ManifestArtifactKind.TRAINER_IMPLEMENTATION),
        ):
            artifact = getattr(self, field_name)
            if not isinstance(artifact, ManifestArtifactId):
                raise BehaviorManifestError(f"{field_name} must be a ManifestArtifactId")
            if artifact.kind is not expected_kind:
                raise BehaviorManifestError(
                    f"{field_name} must have kind {expected_kind.name}"
                )
        object.__setattr__(
            self,
            "semantic_schema_version",
            _non_negative_integer(
                self.semantic_schema_version,
                "semantic_schema_version",
            ),
        )
        object.__setattr__(
            self,
            "training_step",
            _non_negative_integer(self.training_step, "training_step"),
        )

    @property
    def identity(self) -> BehaviorManifestId:
        """Return the canonical identity without retaining any artifact payload."""

        digest = hashlib.sha256(self._canonical_bytes()).digest()
        return BehaviorManifestId(digest)

    def _canonical_bytes(self) -> bytes:
        payload = bytearray(b"sts-behavior-manifest\x00")
        payload.extend(
            struct.pack(
                ">IQQ",
                1,
                self.semantic_schema_version,
                self.training_step,
            )
        )
        for artifact in (
            self.model_checkpoint,
            self.model_definition,
            self.model_config,
            self.semantic_schema,
            self.optimizer_config,
            self.trainer_implementation,
        ):
            payload.append(int(artifact.kind))
            payload.extend(artifact.digest)
        return bytes(payload)


@dataclass(frozen=True)
class BehaviorManifestTemplate:
    """Fixed non-checkpoint provenance used to bind a published model state."""

    model_definition: ManifestArtifactId
    model_config: ManifestArtifactId
    semantic_schema: ManifestArtifactId
    optimizer_config: ManifestArtifactId
    trainer_implementation: ManifestArtifactId
    semantic_schema_version: int

    def __post_init__(self) -> None:
        for field_name, expected_kind in (
            ("model_definition", ManifestArtifactKind.MODEL_DEFINITION),
            ("model_config", ManifestArtifactKind.MODEL_CONFIG),
            ("semantic_schema", ManifestArtifactKind.SEMANTIC_SCHEMA),
            ("optimizer_config", ManifestArtifactKind.OPTIMIZER_CONFIG),
            ("trainer_implementation", ManifestArtifactKind.TRAINER_IMPLEMENTATION),
        ):
            artifact = getattr(self, field_name)
            if not isinstance(artifact, ManifestArtifactId):
                raise BehaviorManifestError(f"{field_name} must be a ManifestArtifactId")
            if artifact.kind is not expected_kind:
                raise BehaviorManifestError(
                    f"{field_name} must have kind {expected_kind.name}"
                )
        object.__setattr__(
            self,
            "semantic_schema_version",
            _non_negative_integer(
                self.semantic_schema_version,
                "semantic_schema_version",
            ),
        )

    def bind(
        self,
        model_checkpoint: ManifestArtifactId,
        *,
        training_step: int,
    ) -> BehaviorManifest:
        """Create one exact manifest; the returned identity includes the step."""

        return BehaviorManifest(
            model_checkpoint=model_checkpoint,
            model_definition=self.model_definition,
            model_config=self.model_config,
            semantic_schema=self.semantic_schema,
            optimizer_config=self.optimizer_config,
            trainer_implementation=self.trainer_implementation,
            semantic_schema_version=self.semantic_schema_version,
            training_step=training_step,
        )


@dataclass(frozen=True)
class BehaviorManifestRegistrySnapshot:
    capacity: int
    registered_manifests: int


class BehaviorManifestRegistry:
    """Fixed-capacity exact lookup; model objects and checkpoints stay external."""

    def __init__(self, capacity: int) -> None:
        self.capacity = _positive_integer(capacity, "capacity")
        self._entries: dict[BehaviorManifestId, BehaviorManifest] = {}

    @property
    def snapshot(self) -> BehaviorManifestRegistrySnapshot:
        return BehaviorManifestRegistrySnapshot(
            capacity=self.capacity,
            registered_manifests=len(self._entries),
        )

    def register(
        self,
        manifest: BehaviorManifest,
        *,
        claimed_id: BehaviorManifestId | None = None,
    ) -> BehaviorManifestId:
        """Register one manifest after checking any persisted claimed identity."""

        identity = self.preview_registration(manifest, claimed_id=claimed_id)
        if identity in self._entries:
            return identity
        self._entries[identity] = manifest
        return identity

    def preview_registration(
        self,
        manifest: BehaviorManifest,
        *,
        claimed_id: BehaviorManifestId | None = None,
    ) -> BehaviorManifestId:
        """Validate a registration without consuming registry capacity."""

        if not isinstance(manifest, BehaviorManifest):
            raise BehaviorManifestError("registry accepts only BehaviorManifest values")
        identity = manifest.identity
        if claimed_id is not None:
            if not isinstance(claimed_id, BehaviorManifestId):
                raise BehaviorManifestError("claimed manifest id must be typed")
            if claimed_id != identity:
                raise BehaviorManifestError(
                    "claimed behavior manifest id conflicts with manifest content"
                )
        existing = self._entries.get(identity)
        if existing is not None:
            if existing != manifest:
                raise BehaviorManifestError(
                    "behavior manifest identity conflicts with registered content"
                )
            return identity
        if len(self._entries) >= self.capacity:
            raise BehaviorManifestError("behavior manifest registry capacity exceeded")
        return identity

    def resolve(self, identity: BehaviorManifestId) -> BehaviorManifest:
        """Resolve only known typed identities."""

        if not isinstance(identity, BehaviorManifestId):
            raise BehaviorManifestError("behavior manifest lookup id must be typed")
        try:
            return self._entries[identity]
        except KeyError as error:
            raise BehaviorManifestError("unknown behavior manifest identity") from error

    def require_exact(
        self,
        identity: BehaviorManifestId,
        expected: BehaviorManifest,
    ) -> BehaviorManifest:
        """Resolve an id and reject any checkpoint, config, or schema mismatch."""

        if not isinstance(expected, BehaviorManifest):
            raise BehaviorManifestError("expected binding must be a BehaviorManifest")
        registered = self.resolve(identity)
        if registered != expected:
            raise BehaviorManifestError(
                "registered behavior manifest does not match the expected exact binding"
            )
        return registered


def _positive_integer(value: object, name: str) -> int:
    normalized = _integer(value, name)
    if normalized <= 0:
        raise BehaviorManifestError(f"{name} must be positive")
    return normalized


def _non_negative_integer(value: object, name: str) -> int:
    normalized = _integer(value, name)
    if normalized < 0:
        raise BehaviorManifestError(f"{name} must be non-negative")
    return normalized


def _integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise BehaviorManifestError(f"{name} must be an integer")
    try:
        return operator.index(value)
    except TypeError as error:
        raise BehaviorManifestError(f"{name} must be an integer") from error
