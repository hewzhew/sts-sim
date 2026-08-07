"""Explicit publication and promotion of checkpointed PyTorch behavior."""

from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass

import torch

from .manifests import (
    BehaviorManifest,
    BehaviorManifestRegistry,
    BehaviorManifestTemplate,
    ManifestArtifactId,
)
from .manifest_catalog import BoundedBehaviorManifestCatalog
from .policy import BatchPolicyChoice, BehaviorManifestId
from .torch_checkpoints import BoundedTorchCheckpointStore
from .torch_policy import RaggedCandidateLogits, RaggedCandidateScorer


class TorchBehaviorError(RuntimeError):
    """A shadow model cannot be safely published or promoted."""


@dataclass(frozen=True)
class TorchBehaviorPublication:
    manifest_id: BehaviorManifestId
    manifest: BehaviorManifest
    checkpoint_id: ManifestArtifactId

    def __post_init__(self) -> None:
        if not isinstance(self.manifest_id, BehaviorManifestId):
            raise TorchBehaviorError("publication manifest id must be typed")
        if not isinstance(self.manifest, BehaviorManifest):
            raise TorchBehaviorError("publication manifest must be typed")
        if not isinstance(self.checkpoint_id, ManifestArtifactId):
            raise TorchBehaviorError("publication checkpoint id must be typed")
        if self.manifest_id != self.manifest.identity:
            raise TorchBehaviorError("publication manifest id conflicts with content")
        if self.manifest.model_checkpoint != self.checkpoint_id:
            raise TorchBehaviorError("publication checkpoint conflicts with manifest")


class TorchBehaviorPublisher:
    """Publish checkpoint then manifest; returning is the visibility boundary."""

    def __init__(
        self,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        template: BehaviorManifestTemplate,
    ) -> None:
        if not isinstance(store, BoundedTorchCheckpointStore):
            raise TorchBehaviorError("publisher requires a checkpoint store")
        if not isinstance(registry, BehaviorManifestRegistry):
            raise TorchBehaviorError("publisher requires a behavior manifest registry")
        if not isinstance(catalog, BoundedBehaviorManifestCatalog):
            raise TorchBehaviorError("publisher requires a durable manifest catalog")
        if not isinstance(template, BehaviorManifestTemplate):
            raise TorchBehaviorError("publisher requires a behavior manifest template")
        self.store = store
        self.catalog = catalog
        self.registry = registry
        self.template = template

    def publish(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> TorchBehaviorPublication:
        if not isinstance(scorer, RaggedCandidateScorer):
            raise TorchBehaviorError("publisher requires a RaggedCandidateScorer")
        if scorer.schema.version != self.template.semantic_schema_version:
            raise TorchBehaviorError(
                "scorer schema version does not match behavior manifest template"
            )
        prepared = self.store.prepare(scorer)
        manifest = self.template.bind(
            prepared.artifact_id,
            training_step=training_step,
        )
        prepared_manifest = self.catalog.prepare(manifest)
        manifest_id = self.registry.preview_registration(manifest)
        self.store.preview_commit(prepared)
        self.catalog.preview_commit(prepared_manifest)
        checkpoint_id = self.store.commit(prepared)
        if checkpoint_id != manifest.model_checkpoint:
            raise TorchBehaviorError("checkpoint store committed a different identity")
        durable_id = self.catalog.commit(prepared_manifest)
        if durable_id != manifest_id:
            raise TorchBehaviorError("manifest catalog committed a different identity")
        registered_id = self.registry.register(manifest, claimed_id=durable_id)
        if registered_id != manifest_id:
            raise TorchBehaviorError("manifest registry committed a different identity")
        return TorchBehaviorPublication(manifest_id, manifest, checkpoint_id)


_PROMOTION_TOKEN = object()


class CheckpointedGreedyTorchPolicy:
    """Frozen behavior scorer materialized only from a registered publication."""

    def __init__(
        self,
        scorer: RaggedCandidateScorer,
        publication: TorchBehaviorPublication,
        *,
        _token: object,
    ) -> None:
        if _token is not _PROMOTION_TOKEN:
            raise TorchBehaviorError("behavior policy must be created through promote")
        self._scorer = scorer
        self.publication = publication

    @classmethod
    def promote(
        cls,
        publication: TorchBehaviorPublication,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        scorer_factory: Callable[[], RaggedCandidateScorer],
    ) -> CheckpointedGreedyTorchPolicy:
        if not isinstance(publication, TorchBehaviorPublication):
            raise TorchBehaviorError("promotion requires a typed publication")
        if not isinstance(store, BoundedTorchCheckpointStore):
            raise TorchBehaviorError("promotion requires a checkpoint store")
        if not isinstance(catalog, BoundedBehaviorManifestCatalog):
            raise TorchBehaviorError("promotion requires a durable manifest catalog")
        if not isinstance(registry, BehaviorManifestRegistry):
            raise TorchBehaviorError("promotion requires a manifest registry")
        try:
            durable = catalog.resolve(publication.manifest_id)
            if durable != publication.manifest:
                raise TorchBehaviorError(
                    "durable manifest does not match behavior publication"
                )
            registry.require_exact(publication.manifest_id, publication.manifest)
        except ValueError as error:
            raise TorchBehaviorError(
                "publication is not registered for behavior promotion"
            ) from error
        model = store.materialize(publication.checkpoint_id, scorer_factory)
        return cls._from_restored(model, publication)

    @classmethod
    def recover(
        cls,
        manifest_id: BehaviorManifestId,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        scorer_factory: Callable[[], RaggedCandidateScorer],
    ) -> CheckpointedGreedyTorchPolicy:
        """Recover from durable owners without a prior in-memory publication."""

        if not isinstance(manifest_id, BehaviorManifestId):
            raise TorchBehaviorError("recovery manifest id must be typed")
        if not isinstance(store, BoundedTorchCheckpointStore):
            raise TorchBehaviorError("recovery requires a checkpoint store")
        if not isinstance(catalog, BoundedBehaviorManifestCatalog):
            raise TorchBehaviorError("recovery requires a durable manifest catalog")
        if not isinstance(registry, BehaviorManifestRegistry):
            raise TorchBehaviorError("recovery requires a manifest registry")
        try:
            manifest = catalog.resolve(manifest_id)
        except RuntimeError as error:
            raise TorchBehaviorError("durable behavior manifest is unavailable") from error
        publication = TorchBehaviorPublication(
            manifest_id=manifest_id,
            manifest=manifest,
            checkpoint_id=manifest.model_checkpoint,
        )
        model = store.materialize(publication.checkpoint_id, scorer_factory)
        registry.preview_registration(manifest, claimed_id=manifest_id)
        registry.register(manifest, claimed_id=manifest_id)
        return cls._from_restored(model, publication)

    @classmethod
    def _from_restored(
        cls,
        model: torch.nn.Module,
        publication: TorchBehaviorPublication,
    ) -> CheckpointedGreedyTorchPolicy:
        if not isinstance(model, RaggedCandidateScorer):
            raise TorchBehaviorError(
                "checkpoint factory did not create a RaggedCandidateScorer"
            )
        if model.schema.version != publication.manifest.semantic_schema_version:
            raise TorchBehaviorError(
                "restored scorer schema version does not match publication"
            )
        model.eval()
        model.requires_grad_(False)
        return cls(model, publication, _token=_PROMOTION_TOKEN)

    @property
    def behavior_manifest_id(self) -> BehaviorManifestId:
        return self.publication.manifest_id

    def score(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits:
        with torch.inference_mode():
            return self._scorer(decision_batch)

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        ordinals = self.score(decision_batch).greedy_ordinals()
        return BatchPolicyChoice.create(ordinals, self.behavior_manifest_id)
