from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from learning.tests.semantic_fixtures import (
    semantic_batch_fixture,
    semantic_schema_fixture,
)
from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_fixture,
    behavior_manifest_template_fixture,
)
from sts_learning import (
    BehaviorManifestCatalogLimits,
    BehaviorManifestRegistry,
    BehaviorRuleBinding,
    BoundedBehaviorManifestCatalog,
    ManifestArtifactId,
    ManifestArtifactKind,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_behavior import (
        CheckpointedCategoricalTorchPolicy,
        CheckpointedGreedyTorchPolicy,
        TorchBehaviorError,
        TorchBehaviorPublisher,
    )
    from sts_learning.torch_checkpoints import (
        BoundedTorchCheckpointStore,
        TorchCheckpointError,
        TorchCheckpointLimits,
    )
    from sts_learning.torch_policy import (
        RaggedCandidateScorer,
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )


def _scorer(*, schema=None):
    return RaggedCandidateScorer.from_bridge_schema(
        schema or semantic_schema_fixture(),
        RaggedScorerConfig(hidden_dim=8, relation_layers=1),
    )


def _store(root, *, checkpoints: int = 3):
    return BoundedTorchCheckpointStore(
        root,
        TorchCheckpointLimits(
            max_checkpoints=checkpoints,
            max_bytes_per_checkpoint=2 * 1024 * 1024,
            max_total_bytes=checkpoints * 2 * 1024 * 1024,
        ),
    )


def _catalog(root, *, manifests: int = 3):
    return BoundedBehaviorManifestCatalog(
        root,
        BehaviorManifestCatalogLimits(
            max_manifests=manifests,
            max_bytes_per_manifest=1024,
            max_total_bytes=manifests * 1024,
        ),
    )


def _alternative_behavior_rule() -> BehaviorRuleBinding:
    return BehaviorRuleBinding(
        implementation=ManifestArtifactId.from_content(
            ManifestArtifactKind.BEHAVIOR_RULE,
            b"test.alternative_behavior_rule\x00v1",
        ),
        configuration=ManifestArtifactId.from_content(
            ManifestArtifactKind.BEHAVIOR_RULE_CONFIG,
            b"test.alternative_behavior_rule.config\x00v1",
        ),
    )


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class TorchBehaviorPublicationTests(unittest.TestCase):
    def test_promotion_uses_fresh_frozen_checkpoint_not_live_shadow_model(self) -> None:
        torch.manual_seed(21)
        shadow = _scorer()
        batch = semantic_batch_fixture()
        expected = shadow(batch).values.detach().clone()
        registry = BehaviorManifestRegistry(capacity=3)

        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            ).publish(shadow, training_step=4)
            with torch.no_grad():
                shadow.scorer[-1].bias.add_(20.0)

            policy = CheckpointedGreedyTorchPolicy.promote(
                publication,
                store,
                catalog,
                registry,
                _scorer,
            )
            actual = policy.score(batch).values
            choice = policy.choose(batch)

            torch.testing.assert_close(actual, expected)
            self.assertFalse(actual.requires_grad)
            self.assertEqual(choice.behavior_manifest_id, publication.manifest_id)
            self.assertEqual(publication.manifest.training_step, 4)
            self.assertNotEqual(shadow(batch).values.detach().tolist(), expected.tolist())

    def test_registry_capacity_failure_publishes_no_checkpoint(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        registry.register(behavior_manifest_fixture())
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            )

            with self.assertRaisesRegex(ValueError, "capacity"):
                publisher.publish(_scorer(), training_step=1)
            self.assertEqual(store.snapshot.checkpoints, 0)
            self.assertEqual(catalog.snapshot.manifests, 0)

    def test_store_failure_registers_no_manifest(self) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        torch.manual_seed(22)
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"), checkpoints=1)
            catalog = _catalog(Path(root, "manifests"))
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            )
            publisher.publish(_scorer(), training_step=0)
            updated = _scorer()

            with self.assertRaisesRegex(TorchCheckpointError, "capacity"):
                publisher.publish(updated, training_step=1)
            self.assertEqual(registry.snapshot.registered_manifests, 1)

    def test_catalog_capacity_failure_publishes_no_checkpoint_or_registry_row(self) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"), manifests=1)
            catalog.commit(catalog.prepare(behavior_manifest_fixture()))
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            )

            with self.assertRaisesRegex(RuntimeError, "capacity"):
                publisher.publish(_scorer(), training_step=8)
            self.assertEqual(store.snapshot.checkpoints, 0)
            self.assertEqual(registry.snapshot.registered_manifests, 0)

    def test_training_step_changes_manifest_but_reuses_identical_checkpoint(self) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        scorer = _scorer()
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            )

            first = publisher.publish(scorer, training_step=2)
            second = publisher.publish(scorer, training_step=3)

            self.assertEqual(first.checkpoint_id, second.checkpoint_id)
            self.assertNotEqual(first.manifest_id, second.manifest_id)
            self.assertEqual(store.snapshot.checkpoints, 1)
            self.assertEqual(registry.snapshot.registered_manifests, 2)

    def test_behavior_rule_changes_manifest_and_greedy_recovery_fails_closed(
        self,
    ) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        scorer = _scorer()
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            greedy = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            ).publish(scorer, training_step=0)
            alternative = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    behavior_rule=_alternative_behavior_rule(),
                ),
            ).publish(scorer, training_step=0)

            self.assertEqual(greedy.checkpoint_id, alternative.checkpoint_id)
            self.assertNotEqual(greedy.manifest_id, alternative.manifest_id)
            with self.assertRaisesRegex(TorchBehaviorError, "greedy candidate rule"):
                CheckpointedGreedyTorchPolicy.promote(
                    alternative,
                    store,
                    catalog,
                    registry,
                    _scorer,
                )
            fresh_registry = BehaviorManifestRegistry(capacity=1)
            with self.assertRaisesRegex(TorchBehaviorError, "greedy candidate rule"):
                CheckpointedGreedyTorchPolicy.recover(
                    alternative.manifest_id,
                    store,
                    catalog,
                    fresh_registry,
                    _scorer,
                )
            self.assertEqual(fresh_registry.snapshot.registered_manifests, 0)

    def test_categorical_promotion_and_recovery_reproduce_injected_rng(self) -> None:
        config = RaggedCategoricalPolicyConfig(temperature=0.75)
        registry = BehaviorManifestRegistry(capacity=1)
        batch = semantic_batch_fixture()
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    behavior_rule=config.behavior_rule,
                ),
            ).publish(_scorer(), training_step=0)
            promoted = CheckpointedCategoricalTorchPolicy.promote(
                publication,
                store,
                catalog,
                registry,
                _scorer,
                config,
                torch.Generator().manual_seed(55),
            )
            recovered_registry = BehaviorManifestRegistry(capacity=1)
            recovered = CheckpointedCategoricalTorchPolicy.recover(
                publication.manifest_id,
                store,
                catalog,
                recovered_registry,
                _scorer,
                config,
                torch.Generator().manual_seed(55),
            )
            global_state = torch.random.get_rng_state().clone()

            promoted_choice = promoted.choose(batch)
            recovered_choice = recovered.choose(batch)

            self.assertEqual(promoted_choice, recovered_choice)
            self.assertTrue(torch.equal(torch.random.get_rng_state(), global_state))
            self.assertEqual(
                promoted_choice.behavior_manifest_id,
                publication.manifest_id,
            )
            self.assertTrue(
                all(
                    probability.value is not None
                    and 0.0 < probability.value <= 1.0
                    for probability in promoted_choice.selection_probabilities
                )
            )
            self.assertEqual(
                recovered_registry.resolve(publication.manifest_id),
                publication.manifest,
            )

    def test_categorical_rule_mismatch_fails_before_rng_or_registry_mutation(
        self,
    ) -> None:
        config = RaggedCategoricalPolicyConfig(temperature=0.5)
        wrong_config = RaggedCategoricalPolicyConfig(temperature=1.0)
        registry = BehaviorManifestRegistry(capacity=1)
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    behavior_rule=config.behavior_rule,
                ),
            ).publish(_scorer(), training_step=0)
            generator = torch.Generator().manual_seed(66)
            generator_state = generator.get_state().clone()
            fresh_registry = BehaviorManifestRegistry(capacity=1)

            with self.assertRaisesRegex(
                TorchBehaviorError,
                "categorical candidate rule",
            ):
                CheckpointedCategoricalTorchPolicy.recover(
                    publication.manifest_id,
                    store,
                    catalog,
                    fresh_registry,
                    _scorer,
                    wrong_config,
                    generator,
                )
            self.assertEqual(fresh_registry.snapshot.registered_manifests, 0)
            self.assertTrue(torch.equal(generator.get_state(), generator_state))
            with self.assertRaisesRegex(TorchBehaviorError, "global generator"):
                CheckpointedCategoricalTorchPolicy.promote(
                    publication,
                    store,
                    catalog,
                    registry,
                    _scorer,
                    config,
                    torch.default_generator,
                )

    def test_unregistered_or_schema_mismatched_publication_cannot_run(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(),
            ).publish(_scorer(), training_step=0)

            with self.assertRaisesRegex(TorchBehaviorError, "not registered"):
                CheckpointedGreedyTorchPolicy.promote(
                    publication,
                    store,
                    catalog,
                    BehaviorManifestRegistry(capacity=1),
                    _scorer,
                )

        mismatched_schema = dict(semantic_schema_fixture())
        mismatched_schema["version"] = 9
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                BehaviorManifestRegistry(capacity=1),
                behavior_manifest_template_fixture(),
            )
            with self.assertRaisesRegex(TorchBehaviorError, "schema version"):
                publisher.publish(_scorer(schema=mismatched_schema), training_step=0)
            self.assertEqual(store.snapshot.checkpoints, 0)

    def test_missing_checkpoint_does_not_partly_hydrate_recovery_registry(self) -> None:
        manifest = behavior_manifest_fixture()
        with tempfile.TemporaryDirectory() as root:
            store = _store(Path(root, "checkpoints"))
            catalog = _catalog(Path(root, "manifests"))
            catalog.commit(catalog.prepare(manifest))
            registry = BehaviorManifestRegistry(capacity=1)

            with self.assertRaisesRegex(TorchCheckpointError, "unknown model"):
                CheckpointedGreedyTorchPolicy.recover(
                    manifest.identity,
                    store,
                    catalog,
                    registry,
                    _scorer,
                )
            self.assertEqual(registry.snapshot.registered_manifests, 0)


if __name__ == "__main__":
    unittest.main()
