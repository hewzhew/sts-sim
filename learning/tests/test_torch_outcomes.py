from __future__ import annotations

import importlib.util
import unittest

from sts_learning import (
    BehaviorManifest,
    BehaviorManifestRegistry,
    CompletedAttemptExperience,
    DecisionExperienceBatch,
    DecisionLineage,
    AttemptKey,
    ManifestArtifactId,
    ManifestArtifactKind,
    TerminalAttemptOutcome,
    TerminalAttemptRecord,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_outcomes import (
        TorchOutcomeError,
        realized_outcome_value_loss,
    )
    from sts_learning.torch_policy import RaggedCandidateLogits


def _manifest() -> BehaviorManifest:
    def artifact(kind: ManifestArtifactKind) -> ManifestArtifactId:
        return ManifestArtifactId(kind, bytes([int(kind)]) * 32)

    return BehaviorManifest(
        model_checkpoint=artifact(ManifestArtifactKind.MODEL_CHECKPOINT),
        model_definition=artifact(ManifestArtifactKind.MODEL_DEFINITION),
        model_config=artifact(ManifestArtifactKind.MODEL_CONFIG),
        semantic_schema=artifact(ManifestArtifactKind.SEMANTIC_SCHEMA),
        optimizer_config=artifact(ManifestArtifactKind.OPTIMIZER_CONFIG),
        trainer_implementation=artifact(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION
        ),
        semantic_schema_version=2,
        training_step=0,
    )


def _batch(
    *,
    slot: int,
    value_indices: tuple[int, ...],
    selected_ordinals: tuple[int, ...],
    manifest_id,
) -> DecisionExperienceBatch:
    lineages = tuple(
        DecisionLineage(
            key=AttemptKey(
                slot_index=slot,
                episode_seed=100 + slot,
                episode_generation=0,
                attempt_index=1,
            ),
            recoveries_used=0,
        )
        for _ in selected_ordinals
    )
    return DecisionExperienceBatch(
        payload={"value_indices": value_indices},
        lineages=lineages,
        selected_ordinals=selected_ordinals,
        behavior_manifest_id=manifest_id,
        decision_count=len(selected_ordinals),
        payload_bytes=1,
    )


def _attempt(
    *,
    slot: int,
    batches: tuple[DecisionExperienceBatch, ...],
    reward: int,
) -> CompletedAttemptExperience:
    lineage = batches[0].lineages[0]
    terminal = TerminalAttemptRecord(
        episode_seed=lineage.key.episode_seed,
        episode_generation=lineage.key.episode_generation,
        attempt_index=lineage.key.attempt_index,
        recoveries_used=lineage.recoveries_used,
        terminal=TerminalAttemptOutcome(
            slot_index=slot,
            terminal_reward=reward,
            terminal_act=3,
            terminal_floor=40,
            terminal_hp=20 if reward == 1 else 0,
            terminal_max_hp=80,
            terminal_gold=50,
        ),
    )
    return CompletedAttemptExperience(
        lineage=lineage,
        batches=batches,
        terminal=terminal,
        decision_count=sum(batch.decision_count for batch in batches),
        payload_bytes=sum(batch.payload_bytes for batch in batches),
    )


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class RealizedOutcomeValueLossTests(unittest.TestCase):
    def test_only_selected_candidates_are_targeted_and_attempts_are_equal_weight(self) -> None:
        manifest = _manifest()
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(manifest)
        values = torch.nn.Parameter(
            torch.tensor([-1.0, 90.0, 0.0, 80.0, 0.0, 70.0, 0.0, 60.0])
        )

        def scorer(payload):
            indices = torch.as_tensor(payload["value_indices"], dtype=torch.long)
            return RaggedCandidateLogits(
                values=values[indices],
                row_splits=torch.arange(0, len(indices) + 1, 2),
            )

        short = _attempt(
            slot=1,
            batches=(
                _batch(
                    slot=1,
                    value_indices=(0, 1),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
            ),
            reward=1,
        )
        long = _attempt(
            slot=2,
            batches=(
                _batch(
                    slot=2,
                    value_indices=(2, 3),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
                _batch(
                    slot=2,
                    value_indices=(4, 5),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
                _batch(
                    slot=2,
                    value_indices=(6, 7),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
            ),
            reward=-1,
        )

        result = realized_outcome_value_loss(scorer, (short, long), registry)
        result.value.backward()

        self.assertEqual(float(result.value.detach()), 2.5)
        self.assertEqual(result.attempt_count, 2)
        self.assertEqual(result.decision_count, 4)
        self.assertEqual(
            result.behavior_manifest_ids,
            ((manifest_id,), (manifest_id, manifest_id, manifest_id)),
        )
        self.assertEqual(values.grad[0].item(), -2.0)
        for index in (1, 3, 5, 7):
            self.assertEqual(values.grad[index].item(), 0.0)
        for index in (2, 4, 6):
            self.assertAlmostEqual(values.grad[index].item(), 1.0 / 3.0, places=6)

    def test_unknown_behavior_manifest_fails_before_training(self) -> None:
        manifest = _manifest()
        unregistered_id = manifest.identity
        registry = BehaviorManifestRegistry(capacity=1)
        batch = _batch(
            slot=1,
            value_indices=(0, 1),
            selected_ordinals=(0,),
            manifest_id=unregistered_id,
        )

        with self.assertRaisesRegex(TorchOutcomeError, "unknown behavior"):
            realized_outcome_value_loss(
                lambda payload: RaggedCandidateLogits(
                    values=torch.zeros(2),
                    row_splits=torch.tensor([0, 2]),
                ),
                (_attempt(slot=1, batches=(batch,), reward=1),),
                registry,
            )

    def test_empty_or_non_complete_input_cannot_create_a_loss(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)

        with self.assertRaisesRegex(TorchOutcomeError, "at least one"):
            realized_outcome_value_loss(lambda payload: None, (), registry)
        with self.assertRaisesRegex(TorchOutcomeError, "only complete"):
            realized_outcome_value_loss(
                lambda payload: None,
                (object(),),  # type: ignore[arg-type]
                registry,
            )


if __name__ == "__main__":
    unittest.main()
