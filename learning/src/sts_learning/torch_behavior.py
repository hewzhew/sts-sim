"""Exact in-process PyTorch behavior binding and explicit persistence."""

from __future__ import annotations

import operator
from collections.abc import Callable, Mapping
from dataclasses import dataclass

import torch

from .manifests import (
    BehaviorManifest,
    BehaviorManifestRegistry,
    BehaviorManifestTemplate,
    BehaviorRuleBinding,
    GREEDY_BEHAVIOR_RULE_V1,
    ManifestArtifactId,
)
from .manifest_catalog import (
    BoundedBehaviorManifestCatalog,
    PreparedBehaviorManifest,
)
from .policy import BatchPolicyChoice, BehaviorManifestId
from .torch_checkpoints import (
    BoundedTorchCheckpointStore,
    PreparedTorchCheckpoint,
)
from .torch_policy import (
    RaggedCandidateLogits,
    RaggedCandidateScorer,
    RaggedCategoricalPolicyConfig,
    sample_ragged_categorical,
)


class TorchBehaviorError(RuntimeError):
    """A shadow model cannot be safely bound, promoted, or published."""


@dataclass(frozen=True)
class TorchBehaviorBinding:
    """Exact model/provenance identity, independent of durable publication."""

    manifest_id: BehaviorManifestId
    manifest: BehaviorManifest
    checkpoint_id: ManifestArtifactId

    def __post_init__(self) -> None:
        if not isinstance(self.manifest_id, BehaviorManifestId):
            raise TorchBehaviorError("behavior manifest id must be typed")
        if not isinstance(self.manifest, BehaviorManifest):
            raise TorchBehaviorError("behavior manifest must be typed")
        if not isinstance(self.checkpoint_id, ManifestArtifactId):
            raise TorchBehaviorError("behavior checkpoint id must be typed")
        if self.manifest_id != self.manifest.identity:
            raise TorchBehaviorError("behavior manifest id conflicts with content")
        if self.manifest.model_checkpoint != self.checkpoint_id:
            raise TorchBehaviorError("behavior checkpoint conflicts with manifest")


@dataclass(frozen=True)
class TorchBehaviorPublication(TorchBehaviorBinding):
    """A behavior binding committed to checkpoint and manifest stores."""


@dataclass(frozen=True)
class TorchBehaviorPublicationPreview:
    """Non-authoritative exact identity and capacity facts for one preview."""

    manifest_id: BehaviorManifestId
    checkpoint_id: ManifestArtifactId
    training_step: int
    checkpoint_payload_bytes: int
    manifest_payload_bytes: int
    requires_novel_capacity: bool


@dataclass(frozen=True)
class _PreparedTorchBehaviorPublication:
    checkpoint: PreparedTorchCheckpoint
    durable_manifest: PreparedBehaviorManifest
    binding: TorchBehaviorBinding


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
        prepared = self._prepare(scorer, training_step=training_step)
        return self._commit(prepared)

    def publish_exact(
        self,
        scorer: RaggedCandidateScorer,
        binding: TorchBehaviorBinding,
    ) -> TorchBehaviorPublication:
        """Durably publish one already-active exact in-memory binding."""

        if not isinstance(binding, TorchBehaviorBinding):
            raise TorchBehaviorError("exact publication requires a behavior binding")
        prepared = self._prepare(
            scorer,
            training_step=binding.manifest.training_step,
        )
        if (
            prepared.binding.manifest_id != binding.manifest_id
            or prepared.binding.manifest != binding.manifest
            or prepared.binding.checkpoint_id != binding.checkpoint_id
        ):
            raise TorchBehaviorError(
                "active scorer no longer matches its behavior binding"
            )
        return self._commit(prepared)

    def bind_live(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> TorchBehaviorBinding:
        """Compute an exact behavior identity without writing durable state."""

        checkpoint, binding = self._bind(scorer, training_step=training_step)
        if checkpoint.artifact_id != binding.checkpoint_id:
            raise TorchBehaviorError("live behavior checkpoint identity changed")
        return binding

    def _commit(
        self,
        prepared: _PreparedTorchBehaviorPublication,
    ) -> TorchBehaviorPublication:
        self._preview(prepared, novel=False)
        binding = prepared.binding
        manifest = binding.manifest
        checkpoint_id = self.store.commit(prepared.checkpoint)
        if checkpoint_id != manifest.model_checkpoint:
            raise TorchBehaviorError("checkpoint store committed a different identity")
        durable_id = self.catalog.commit(prepared.durable_manifest)
        if durable_id != binding.manifest_id:
            raise TorchBehaviorError("manifest catalog committed a different identity")
        registered_id = self.registry.register(manifest, claimed_id=durable_id)
        if registered_id != binding.manifest_id:
            raise TorchBehaviorError("manifest registry committed a different identity")
        return TorchBehaviorPublication(
            binding.manifest_id,
            binding.manifest,
            binding.checkpoint_id,
        )

    def preview(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> TorchBehaviorPublicationPreview:
        """Verify an exact publication without mutating any owner."""

        prepared = self._prepare(scorer, training_step=training_step)
        self._preview(prepared, novel=False)
        return _publication_preview(prepared, novel=False)

    def preview_novel(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> TorchBehaviorPublicationPreview:
        """Reserve capacity for one new same-shape generation without mutation."""

        prepared = self._prepare(scorer, training_step=training_step)
        self._preview(prepared, novel=True)
        return _publication_preview(prepared, novel=True)

    def _prepare(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> _PreparedTorchBehaviorPublication:
        checkpoint, binding = self._bind(scorer, training_step=training_step)
        durable_manifest = self.catalog.prepare(binding.manifest)
        return _PreparedTorchBehaviorPublication(
            checkpoint=checkpoint,
            durable_manifest=durable_manifest,
            binding=binding,
        )

    def _bind(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> tuple[PreparedTorchCheckpoint, TorchBehaviorBinding]:
        if not isinstance(scorer, RaggedCandidateScorer):
            raise TorchBehaviorError("publisher requires a RaggedCandidateScorer")
        if scorer.schema.version != self.template.semantic_schema_version:
            raise TorchBehaviorError(
                "scorer schema version does not match behavior manifest template"
            )
        checkpoint = self.store.prepare(scorer)
        manifest = self.template.bind(
            checkpoint.artifact_id,
            training_step=training_step,
        )
        binding = TorchBehaviorBinding(
            manifest.identity,
            manifest,
            checkpoint.artifact_id,
        )
        return checkpoint, binding

    def _preview(
        self,
        prepared: _PreparedTorchBehaviorPublication,
        *,
        novel: bool,
    ) -> None:
        if novel:
            self.store.preview_novel_commit(prepared.checkpoint)
            self.catalog.preview_novel_commit(prepared.durable_manifest)
            self.registry.preview_novel_registration()
            return
        binding = prepared.binding
        self.store.preview_commit(prepared.checkpoint)
        self.catalog.preview_commit(prepared.durable_manifest)
        self.registry.preview_registration(
            binding.manifest,
            claimed_id=binding.manifest_id,
        )


def _publication_preview(
    prepared: _PreparedTorchBehaviorPublication,
    *,
    novel: bool,
) -> TorchBehaviorPublicationPreview:
    binding = prepared.binding
    return TorchBehaviorPublicationPreview(
        manifest_id=binding.manifest_id,
        checkpoint_id=binding.checkpoint_id,
        training_step=binding.manifest.training_step,
        checkpoint_payload_bytes=prepared.checkpoint.payload_bytes,
        manifest_payload_bytes=prepared.durable_manifest.payload_bytes,
        requires_novel_capacity=novel,
    )


_PROMOTION_TOKEN = object()


class CheckpointedGreedyTorchPolicy:
    """Frozen behavior scorer materialized only from a registered publication."""

    def __init__(
        self,
        scorer: RaggedCandidateScorer,
        binding: TorchBehaviorBinding,
        *,
        _token: object,
    ) -> None:
        if _token is not _PROMOTION_TOKEN:
            raise TorchBehaviorError("behavior policy must be created through promote")
        if not isinstance(binding, TorchBehaviorBinding):
            raise TorchBehaviorError("behavior policy requires an exact binding")
        self._scorer = scorer
        self.binding = binding

    @classmethod
    def promote(
        cls,
        publication: TorchBehaviorPublication,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        scorer_factory: Callable[[], RaggedCandidateScorer],
    ) -> CheckpointedGreedyTorchPolicy:
        model = _promote_frozen_scorer(
            publication,
            store,
            catalog,
            registry,
            scorer_factory,
            expected_rule=GREEDY_BEHAVIOR_RULE_V1,
            rule_name="greedy candidate rule",
        )
        return cls(model, publication, _token=_PROMOTION_TOKEN)

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

        model, publication = _recover_frozen_scorer(
            manifest_id,
            store,
            catalog,
            registry,
            scorer_factory,
            expected_rule=GREEDY_BEHAVIOR_RULE_V1,
            rule_name="greedy candidate rule",
        )
        return cls(model, publication, _token=_PROMOTION_TOKEN)

    @property
    def behavior_manifest_id(self) -> BehaviorManifestId:
        return self.binding.manifest_id

    def score(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits:
        with torch.inference_mode():
            return self._scorer(decision_batch)

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        ordinals = self.score(decision_batch).greedy_ordinals()
        return BatchPolicyChoice.deterministic(ordinals, self.behavior_manifest_id)


class FrozenCategoricalTorchPolicy:
    """Frozen exact behavior sampled through one injected random stream."""

    def __init__(
        self,
        scorer: RaggedCandidateScorer,
        binding: TorchBehaviorBinding,
        config: RaggedCategoricalPolicyConfig,
        generator: torch.Generator,
        *,
        _token: object,
    ) -> None:
        if _token is not _PROMOTION_TOKEN:
            raise TorchBehaviorError("behavior policy must be created through promote")
        if not isinstance(binding, TorchBehaviorBinding):
            raise TorchBehaviorError("behavior policy requires an exact binding")
        _validate_categorical_inputs(config, generator)
        _require_generator_device(scorer, generator)
        self._scorer = scorer
        self.binding = binding
        self.config = config
        self.generator = generator

    @classmethod
    def promote(
        cls,
        publication: TorchBehaviorPublication,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        scorer_factory: Callable[[], RaggedCandidateScorer],
        config: RaggedCategoricalPolicyConfig,
        generator: torch.Generator,
    ) -> FrozenCategoricalTorchPolicy:
        _validate_categorical_inputs(config, generator)
        model = _promote_frozen_scorer(
            publication,
            store,
            catalog,
            registry,
            scorer_factory,
            expected_rule=config.behavior_rule,
            rule_name="categorical candidate rule",
            model_validator=lambda scorer: _require_generator_device(
                scorer,
                generator,
            ),
        )
        return cls(
            model,
            publication,
            config,
            generator,
            _token=_PROMOTION_TOKEN,
        )

    @classmethod
    def recover(
        cls,
        manifest_id: BehaviorManifestId,
        store: BoundedTorchCheckpointStore,
        catalog: BoundedBehaviorManifestCatalog,
        registry: BehaviorManifestRegistry,
        scorer_factory: Callable[[], RaggedCandidateScorer],
        config: RaggedCategoricalPolicyConfig,
        generator: torch.Generator,
    ) -> FrozenCategoricalTorchPolicy:
        _validate_categorical_inputs(config, generator)
        model, publication = _recover_frozen_scorer(
            manifest_id,
            store,
            catalog,
            registry,
            scorer_factory,
            expected_rule=config.behavior_rule,
            rule_name="categorical candidate rule",
            model_validator=lambda scorer: _require_generator_device(
                scorer,
                generator,
            ),
        )
        return cls(
            model,
            publication,
            config,
            generator,
            _token=_PROMOTION_TOKEN,
        )

    @property
    def behavior_manifest_id(self) -> BehaviorManifestId:
        return self.binding.manifest_id

    @property
    def frozen_scorer(self) -> RaggedCandidateScorer:
        return self._scorer

    def fork(self, generator: torch.Generator) -> FrozenCategoricalTorchPolicy:
        """Share immutable behavior while giving one caller an independent RNG."""

        _validate_categorical_inputs(self.config, generator)
        _require_generator_device(self._scorer, generator)
        return FrozenCategoricalTorchPolicy(
            self._scorer,
            self.binding,
            self.config,
            generator,
            _token=_PROMOTION_TOKEN,
        )

    def score(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits:
        with torch.inference_mode():
            return self._scorer(decision_batch)

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        sample = sample_ragged_categorical(
            self.score(decision_batch),
            self.config,
            self.generator,
        )
        return BatchPolicyChoice.create(
            sample.ordinals,
            self.behavior_manifest_id,
            sample.selection_probabilities,
        )


class CheckpointedCategoricalTorchPolicy(FrozenCategoricalTorchPolicy):
    """Frozen behavior whose binding was verified through durable stores."""


@dataclass(frozen=True)
class CategoricalTorchBehaviorControllerSnapshot:
    active_manifest_id: BehaviorManifestId | None
    active_training_step: int | None
    successful_promotions: int


class CategoricalTorchBehaviorController:
    """Atomically switch one live categorical behavior after verified promotion."""

    def __init__(
        self,
        publisher: TorchBehaviorPublisher,
        scorer_factory: Callable[[], RaggedCandidateScorer],
        config: RaggedCategoricalPolicyConfig,
        generator: torch.Generator,
    ) -> None:
        if not isinstance(publisher, TorchBehaviorPublisher):
            raise TorchBehaviorError("controller requires a behavior publisher")
        if not callable(scorer_factory):
            raise TorchBehaviorError("controller requires a scorer factory")
        _validate_categorical_inputs(config, generator)
        self.publisher = publisher
        self.scorer_factory = scorer_factory
        self.config = config
        self.generator = generator
        self._policy: FrozenCategoricalTorchPolicy | None = None
        self._successful_promotions = 0

    @property
    def snapshot(self) -> CategoricalTorchBehaviorControllerSnapshot:
        binding = self._policy.binding if self._policy is not None else None
        return CategoricalTorchBehaviorControllerSnapshot(
            active_manifest_id=(
                binding.manifest_id if binding is not None else None
            ),
            active_training_step=(
                binding.manifest.training_step if binding is not None else None
            ),
            successful_promotions=self._successful_promotions,
        )

    def promote_live(
        self,
        scorer: RaggedCandidateScorer,
        *,
        training_step: int,
    ) -> TorchBehaviorBinding:
        """Freeze and activate a model without writing durable artifacts."""

        step = _training_step(training_step)
        current = self.snapshot.active_training_step
        if current is not None and step <= current:
            raise TorchBehaviorError(
                "controller training step must increase across promotions"
            )
        frozen = _clone_frozen_scorer(scorer, self.scorer_factory)
        binding = self.publisher.bind_live(frozen, training_step=step)
        _require_behavior_rule(
            binding.manifest,
            self.config.behavior_rule,
            "categorical candidate rule",
        )
        policy = FrozenCategoricalTorchPolicy(
            frozen,
            binding,
            self.config,
            self.generator,
            _token=_PROMOTION_TOKEN,
        )
        registered_id = self.publisher.registry.replace_active(
            self.snapshot.active_manifest_id,
            binding.manifest,
        )
        if registered_id != binding.manifest_id:
            raise TorchBehaviorError(
                "live registry committed a different behavior identity"
            )
        self._policy = policy
        self._successful_promotions += 1
        return binding

    def publish_active(self) -> TorchBehaviorPublication:
        """Durably publish the current frozen behavior without switching it."""

        if self._policy is None:
            raise TorchBehaviorError("categorical behavior controller is inactive")
        return self.publisher.publish_exact(
            self._policy.frozen_scorer,
            self._policy.binding,
        )

    def fork_active(
        self,
        generator: torch.Generator,
    ) -> FrozenCategoricalTorchPolicy:
        """Bind the active frozen scorer to one independent random stream."""

        if self._policy is None:
            raise TorchBehaviorError("categorical behavior controller is inactive")
        _validate_categorical_inputs(self.config, generator)
        _require_generator_device(self._policy.frozen_scorer, generator)
        return FrozenCategoricalTorchPolicy(
            self._policy.frozen_scorer,
            self._policy.binding,
            self.config,
            generator,
            _token=_PROMOTION_TOKEN,
        )

    def recover_and_promote(
        self,
        manifest_id: BehaviorManifestId,
        *,
        successful_promotions: int = 1,
    ) -> TorchBehaviorPublication:
        if self._policy is not None:
            raise TorchBehaviorError(
                "controller recovery requires an inactive behavior slot"
            )
        if isinstance(successful_promotions, bool):
            raise TorchBehaviorError("successful promotions must be an integer")
        try:
            promotion_count = operator.index(successful_promotions)
        except TypeError as error:
            raise TorchBehaviorError("successful promotions must be an integer") from error
        if promotion_count <= 0:
            raise TorchBehaviorError("successful promotions must be positive")
        policy = CheckpointedCategoricalTorchPolicy.recover(
            manifest_id,
            self.publisher.store,
            self.publisher.catalog,
            self.publisher.registry,
            self.scorer_factory,
            self.config,
            self.generator,
        )
        self._policy = policy
        self._successful_promotions = promotion_count
        if not isinstance(policy.binding, TorchBehaviorPublication):
            raise TorchBehaviorError("recovered behavior is not durably published")
        return policy.binding

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        if self._policy is None:
            raise TorchBehaviorError("categorical behavior controller is inactive")
        return self._policy.choose(decision_batch)


def _clone_frozen_scorer(
    scorer: RaggedCandidateScorer,
    scorer_factory: Callable[[], RaggedCandidateScorer],
) -> RaggedCandidateScorer:
    """Copy a shadow scorer into an independent in-process behavior model."""

    if not isinstance(scorer, RaggedCandidateScorer):
        raise TorchBehaviorError("live promotion requires a RaggedCandidateScorer")
    model = scorer_factory()
    if not isinstance(model, RaggedCandidateScorer):
        raise TorchBehaviorError(
            "behavior scorer factory did not create a RaggedCandidateScorer"
        )
    if model.schema.version != scorer.schema.version:
        raise TorchBehaviorError(
            "behavior scorer factory returned a different schema version"
        )
    try:
        model.load_state_dict(scorer.state_dict(), strict=True)
    except RuntimeError as error:
        raise TorchBehaviorError(
            "behavior scorer factory is incompatible with the shadow model"
        ) from error
    model.eval()
    model.requires_grad_(False)
    return model


def _promote_frozen_scorer(
    publication: TorchBehaviorPublication,
    store: BoundedTorchCheckpointStore,
    catalog: BoundedBehaviorManifestCatalog,
    registry: BehaviorManifestRegistry,
    scorer_factory: Callable[[], RaggedCandidateScorer],
    *,
    expected_rule: BehaviorRuleBinding,
    rule_name: str,
    model_validator: Callable[[RaggedCandidateScorer], None] | None = None,
) -> RaggedCandidateScorer:
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
    _require_behavior_rule(publication.manifest, expected_rule, rule_name)
    model = store.materialize(publication.checkpoint_id, scorer_factory)
    return _freeze_scorer(model, publication, model_validator)


def _recover_frozen_scorer(
    manifest_id: BehaviorManifestId,
    store: BoundedTorchCheckpointStore,
    catalog: BoundedBehaviorManifestCatalog,
    registry: BehaviorManifestRegistry,
    scorer_factory: Callable[[], RaggedCandidateScorer],
    *,
    expected_rule: BehaviorRuleBinding,
    rule_name: str,
    model_validator: Callable[[RaggedCandidateScorer], None] | None = None,
) -> tuple[RaggedCandidateScorer, TorchBehaviorPublication]:
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
    _require_behavior_rule(manifest, expected_rule, rule_name)
    publication = TorchBehaviorPublication(
        manifest_id=manifest_id,
        manifest=manifest,
        checkpoint_id=manifest.model_checkpoint,
    )
    model = store.materialize(publication.checkpoint_id, scorer_factory)
    frozen = _freeze_scorer(model, publication, model_validator)
    registry.preview_registration(manifest, claimed_id=manifest_id)
    registry.register(manifest, claimed_id=manifest_id)
    return frozen, publication


def _freeze_scorer(
    model: torch.nn.Module,
    publication: TorchBehaviorPublication,
    validator: Callable[[RaggedCandidateScorer], None] | None,
) -> RaggedCandidateScorer:
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
    if validator is not None:
        validator(model)
    return model


def _require_behavior_rule(
    manifest: BehaviorManifest,
    expected: BehaviorRuleBinding,
    name: str,
) -> None:
    if manifest.behavior_rule != expected:
        raise TorchBehaviorError(
            f"behavior manifest is not bound to the {name}"
        )


def _validate_categorical_inputs(
    config: RaggedCategoricalPolicyConfig,
    generator: torch.Generator,
) -> None:
    if not isinstance(config, RaggedCategoricalPolicyConfig):
        raise TorchBehaviorError("categorical policy requires typed config")
    if not isinstance(generator, torch.Generator):
        raise TorchBehaviorError("categorical policy requires an injected generator")
    if generator is torch.default_generator:
        raise TorchBehaviorError("categorical policy refuses the global generator")


def _require_generator_device(
    scorer: RaggedCandidateScorer,
    generator: torch.Generator,
) -> None:
    model_device = next(scorer.parameters()).device
    if torch.device(generator.device) != model_device:
        raise TorchBehaviorError(
            "categorical generator device must match the restored scorer"
        )


def _training_step(value: object) -> int:
    if isinstance(value, bool):
        raise TorchBehaviorError("training step must be an integer")
    try:
        step = operator.index(value)
    except TypeError as error:
        raise TorchBehaviorError("training step must be an integer") from error
    if step < 0:
        raise TorchBehaviorError("training step must be non-negative")
    return step
