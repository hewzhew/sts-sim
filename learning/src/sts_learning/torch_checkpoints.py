"""Deterministic tensor-only PyTorch checkpoints with a bounded file store."""

from __future__ import annotations

import hashlib
import operator
import os
import re
import tempfile
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path

from torch import nn

from ._torch_checkpoint_codec import (
    TorchCheckpointError,
    decode_state_dict,
    encode_state_dict,
    validate_compatible_state,
)
from .manifests import ManifestArtifactId, ManifestArtifactKind


@dataclass(frozen=True)
class TorchCheckpointLimits:
    max_checkpoints: int
    max_bytes_per_checkpoint: int
    max_total_bytes: int

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "max_checkpoints",
            _positive_integer(self.max_checkpoints, "max_checkpoints"),
        )
        object.__setattr__(
            self,
            "max_bytes_per_checkpoint",
            _positive_integer(
                self.max_bytes_per_checkpoint,
                "max_bytes_per_checkpoint",
            ),
        )
        object.__setattr__(
            self,
            "max_total_bytes",
            _positive_integer(self.max_total_bytes, "max_total_bytes"),
        )
        if self.max_bytes_per_checkpoint > self.max_total_bytes:
            raise TorchCheckpointError(
                "max_bytes_per_checkpoint cannot exceed max_total_bytes"
            )


@dataclass(frozen=True)
class PreparedTorchCheckpoint:
    artifact_id: ManifestArtifactId
    payload_bytes: int
    _payload: bytes = field(repr=False, compare=False)

    def __post_init__(self) -> None:
        if (
            not isinstance(self.artifact_id, ManifestArtifactId)
            or self.artifact_id.kind is not ManifestArtifactKind.MODEL_CHECKPOINT
        ):
            raise TorchCheckpointError(
                "prepared checkpoint requires a MODEL_CHECKPOINT artifact id"
            )
        if not isinstance(self._payload, bytes):
            raise TorchCheckpointError("prepared checkpoint payload must be immutable")
        if self.payload_bytes != len(self._payload):
            raise TorchCheckpointError("prepared checkpoint byte count is incorrect")
        if hashlib.sha256(self._payload).digest() != self.artifact_id.digest:
            raise TorchCheckpointError("prepared checkpoint digest is incorrect")


@dataclass(frozen=True)
class TorchCheckpointStoreSnapshot:
    checkpoints: int
    total_bytes: int
    max_checkpoints: int
    max_total_bytes: int


class BoundedTorchCheckpointStore:
    """No-eviction content store for exact, tensor-only model checkpoints."""

    def __init__(self, root: str | os.PathLike[str], limits: TorchCheckpointLimits):
        if not isinstance(limits, TorchCheckpointLimits):
            raise TorchCheckpointError("checkpoint store limits must be typed")
        self.limits = limits
        self.root = Path(root).resolve()
        if self.root.exists() and not self.root.is_dir():
            raise TorchCheckpointError("checkpoint store root is not a directory")
        self.root.mkdir(exist_ok=True)
        self._entries: dict[ManifestArtifactId, tuple[Path, int]] = {}
        self._load_existing()

    @property
    def snapshot(self) -> TorchCheckpointStoreSnapshot:
        return TorchCheckpointStoreSnapshot(
            checkpoints=len(self._entries),
            total_bytes=sum(size for _, size in self._entries.values()),
            max_checkpoints=self.limits.max_checkpoints,
            max_total_bytes=self.limits.max_total_bytes,
        )

    def prepare(self, model: nn.Module) -> PreparedTorchCheckpoint:
        if not isinstance(model, nn.Module):
            raise TorchCheckpointError("checkpoint source must be a torch Module")
        payload = encode_state_dict(
            model.state_dict(),
            max_bytes=self.limits.max_bytes_per_checkpoint,
        )
        artifact_id = ManifestArtifactId(
            ManifestArtifactKind.MODEL_CHECKPOINT,
            hashlib.sha256(payload).digest(),
        )
        return PreparedTorchCheckpoint(artifact_id, len(payload), payload)

    def commit(self, prepared: PreparedTorchCheckpoint) -> ManifestArtifactId:
        if not isinstance(prepared, PreparedTorchCheckpoint):
            raise TorchCheckpointError("checkpoint commit must be prepared")
        if prepared.payload_bytes > self.limits.max_bytes_per_checkpoint:
            raise TorchCheckpointError("checkpoint exceeds its per-checkpoint byte limit")
        existing = self._entries.get(prepared.artifact_id)
        if existing is not None:
            if self._read_verified(prepared.artifact_id) != prepared._payload:
                raise TorchCheckpointError(
                    "checkpoint digest conflicts with stored checkpoint content"
                )
            return prepared.artifact_id
        if len(self._entries) >= self.limits.max_checkpoints:
            raise TorchCheckpointError("checkpoint store capacity exceeded")
        if self.snapshot.total_bytes + prepared.payload_bytes > self.limits.max_total_bytes:
            raise TorchCheckpointError("checkpoint store total byte limit exceeded")

        target = self._path(prepared.artifact_id)
        temporary: Path | None = None
        published = False
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb",
                prefix=".pending-",
                suffix=".tmp",
                dir=self.root,
                delete=False,
            ) as output:
                temporary = Path(output.name)
                output.write(prepared._payload)
                output.flush()
                os.fsync(output.fileno())
            try:
                os.link(temporary, target)
                published = True
            except FileExistsError:
                if (
                    target.stat().st_size != prepared.payload_bytes
                    or target.read_bytes() != prepared._payload
                ):
                    raise TorchCheckpointError(
                        "checkpoint target conflicts with prepared content"
                    )
        finally:
            if temporary is not None and temporary.exists():
                temporary.unlink()

        if published:
            self._entries[prepared.artifact_id] = (target, prepared.payload_bytes)
        elif prepared.artifact_id not in self._entries:
            self._entries[prepared.artifact_id] = (target, prepared.payload_bytes)
        return prepared.artifact_id

    def materialize(
        self,
        artifact_id: ManifestArtifactId,
        factory: Callable[[], nn.Module],
    ) -> nn.Module:
        """Restore into a fresh model so an existing scorer cannot be half-mutated."""

        if not callable(factory):
            raise TorchCheckpointError("checkpoint model factory must be callable")
        payload = self._read_verified(artifact_id)
        state = decode_state_dict(payload)
        model = factory()
        if not isinstance(model, nn.Module):
            raise TorchCheckpointError("checkpoint factory did not return a torch Module")
        validate_compatible_state(model.state_dict(), state)
        model.load_state_dict(state, strict=True)
        restored = self.prepare(model)
        if restored.artifact_id != artifact_id:
            raise TorchCheckpointError("restored model does not reproduce checkpoint digest")
        return model

    def _load_existing(self) -> None:
        total_bytes = 0
        entries: dict[ManifestArtifactId, tuple[Path, int]] = {}
        with os.scandir(self.root) as directory:
            for entry in directory:
                if not entry.is_file(follow_symlinks=False):
                    raise TorchCheckpointError(
                        f"checkpoint store contains unexpected entry {entry.name!r}"
                    )
                match = _CHECKPOINT_NAME.fullmatch(entry.name)
                if match is None:
                    raise TorchCheckpointError(
                        f"checkpoint store contains unexpected file {entry.name!r}"
                    )
                size = entry.stat(follow_symlinks=False).st_size
                if size > self.limits.max_bytes_per_checkpoint:
                    raise TorchCheckpointError(
                        "existing checkpoint exceeds its byte limit"
                    )
                total_bytes += size
                if total_bytes > self.limits.max_total_bytes:
                    raise TorchCheckpointError(
                        "existing checkpoints exceed total byte limit"
                    )
                if len(entries) >= self.limits.max_checkpoints:
                    raise TorchCheckpointError(
                        "existing checkpoints exceed store capacity"
                    )
                artifact_id = ManifestArtifactId(
                    ManifestArtifactKind.MODEL_CHECKPOINT,
                    bytes.fromhex(match.group(1)),
                )
                path = Path(entry.path)
                payload = path.read_bytes()
                if (
                    len(payload) != size
                    or hashlib.sha256(payload).digest() != artifact_id.digest
                ):
                    raise TorchCheckpointError("existing checkpoint digest is corrupt")
                decode_state_dict(payload)
                entries[artifact_id] = (path, size)
        self._entries = entries

    def _read_verified(self, artifact_id: ManifestArtifactId) -> bytes:
        if (
            not isinstance(artifact_id, ManifestArtifactId)
            or artifact_id.kind is not ManifestArtifactKind.MODEL_CHECKPOINT
        ):
            raise TorchCheckpointError("checkpoint lookup id must be typed")
        try:
            path, expected_size = self._entries[artifact_id]
        except KeyError as error:
            raise TorchCheckpointError("unknown model checkpoint identity") from error
        payload = path.read_bytes()
        if len(payload) != expected_size:
            raise TorchCheckpointError("stored checkpoint size changed")
        if hashlib.sha256(payload).digest() != artifact_id.digest:
            raise TorchCheckpointError("stored checkpoint digest changed")
        return payload

    def _path(self, artifact_id: ManifestArtifactId) -> Path:
        return self.root / f"{artifact_id.digest.hex()}.ststorch"


_CHECKPOINT_NAME = re.compile(r"([0-9a-f]{64})\.ststorch")


def _positive_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise TorchCheckpointError(f"{name} must be an integer")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise TorchCheckpointError(f"{name} must be an integer") from error
    if normalized <= 0:
        raise TorchCheckpointError(f"{name} must be positive")
    return normalized
