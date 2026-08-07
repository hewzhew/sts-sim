from __future__ import annotations

from sts_learning import (
    AttemptKey,
    BehaviorManifest,
    BehaviorManifestId,
    CompletedAttemptExperience,
    DecisionExperienceBatch,
    DecisionLineage,
    ManifestArtifactId,
    ManifestArtifactKind,
    TerminalAttemptOutcome,
    TerminalAttemptRecord,
)


def behavior_manifest_fixture() -> BehaviorManifest:
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


def decision_batch_fixture(
    *,
    slot: int,
    value_indices: tuple[int, ...],
    selected_ordinals: tuple[int, ...],
    manifest_id: BehaviorManifestId,
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


def completed_attempt_fixture(
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
