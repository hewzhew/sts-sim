from __future__ import annotations

import unittest

from sts_learning import (
    BehaviorManifest,
    BehaviorManifestError,
    BehaviorManifestId,
    BehaviorManifestRegistry,
    ManifestArtifactId,
    ManifestArtifactKind,
)


def _artifact(kind: ManifestArtifactKind, marker: int) -> ManifestArtifactId:
    return ManifestArtifactId(kind, bytes([marker]) * 32)


def _manifest(*, checkpoint_marker: int = 1, config_marker: int = 3) -> BehaviorManifest:
    return BehaviorManifest(
        model_checkpoint=_artifact(
            ManifestArtifactKind.MODEL_CHECKPOINT,
            checkpoint_marker,
        ),
        model_definition=_artifact(ManifestArtifactKind.MODEL_DEFINITION, 2),
        model_config=_artifact(ManifestArtifactKind.MODEL_CONFIG, config_marker),
        semantic_schema=_artifact(ManifestArtifactKind.SEMANTIC_SCHEMA, 4),
        optimizer_config=_artifact(ManifestArtifactKind.OPTIMIZER_CONFIG, 5),
        trainer_implementation=_artifact(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION,
            6,
        ),
        semantic_schema_version=1,
        training_step=10,
    )


class BehaviorManifestTests(unittest.TestCase):
    def test_identity_is_canonical_and_changes_with_exact_checkpoint(self) -> None:
        first = _manifest()
        same = _manifest()
        updated = _manifest(checkpoint_marker=7)

        self.assertEqual(first.identity, same.identity)
        self.assertNotEqual(first.identity, updated.identity)

    def test_artifact_kind_cannot_be_swapped_between_manifest_fields(self) -> None:
        with self.assertRaisesRegex(BehaviorManifestError, "MODEL_CHECKPOINT"):
            BehaviorManifest(
                model_checkpoint=_artifact(ManifestArtifactKind.MODEL_CONFIG, 1),
                model_definition=_artifact(ManifestArtifactKind.MODEL_DEFINITION, 2),
                model_config=_artifact(ManifestArtifactKind.MODEL_CONFIG, 3),
                semantic_schema=_artifact(ManifestArtifactKind.SEMANTIC_SCHEMA, 4),
                optimizer_config=_artifact(ManifestArtifactKind.OPTIMIZER_CONFIG, 5),
                trainer_implementation=_artifact(
                    ManifestArtifactKind.TRAINER_IMPLEMENTATION,
                    6,
                ),
                semantic_schema_version=1,
                training_step=10,
            )

    def test_registry_rejects_unknown_and_conflicting_claimed_id(self) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        manifest = _manifest()

        with self.assertRaisesRegex(BehaviorManifestError, "unknown"):
            registry.resolve(BehaviorManifestId(b"\xff" * 32))
        with self.assertRaisesRegex(BehaviorManifestError, "conflicts"):
            registry.register(
                manifest,
                claimed_id=BehaviorManifestId(b"\x00" * 32),
            )

        self.assertEqual(registry.snapshot.registered_manifests, 0)

    def test_exact_binding_rejects_checkpoint_and_config_mismatch(self) -> None:
        registry = BehaviorManifestRegistry(capacity=2)
        manifest = _manifest()
        identity = registry.register(manifest)

        self.assertIs(registry.require_exact(identity, manifest), manifest)
        for mismatched in (
            _manifest(checkpoint_marker=7),
            _manifest(config_marker=8),
        ):
            with self.assertRaisesRegex(BehaviorManifestError, "exact binding"):
                registry.require_exact(identity, mismatched)

    def test_registry_is_idempotent_but_never_evicts_past_capacity(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        manifest = _manifest()
        identity = registry.register(manifest)

        self.assertEqual(registry.register(manifest), identity)
        with self.assertRaisesRegex(BehaviorManifestError, "capacity"):
            registry.register(_manifest(checkpoint_marker=9))
        self.assertEqual(registry.snapshot.registered_manifests, 1)


if __name__ == "__main__":
    unittest.main()
