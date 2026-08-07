from __future__ import annotations

from learning.tests.semantic_fixtures import semantic_batch_fixture
from sts_learning import (
    AttemptKey,
    BehaviorManifest,
    BehaviorManifestId,
    BehaviorManifestTemplate,
    BehaviorRuleBinding,
    CompletedAttemptExperience,
    DETERMINISTIC_SELECTION,
    DecisionExperienceBatch,
    DecisionLineage,
    ManifestArtifactId,
    ManifestArtifactKind,
    GREEDY_BEHAVIOR_RULE_V1,
    SelectionProbability,
    TerminalAttemptOutcome,
    TerminalAttemptRecord,
    select_semantic_decision_rows,
)


def behavior_manifest_template_fixture(
    *,
    semantic_schema_version: int = 3,
    behavior_rule: BehaviorRuleBinding = GREEDY_BEHAVIOR_RULE_V1,
) -> BehaviorManifestTemplate:
    def artifact(kind: ManifestArtifactKind) -> ManifestArtifactId:
        return ManifestArtifactId(kind, bytes([int(kind)]) * 32)

    return BehaviorManifestTemplate(
        model_definition=artifact(ManifestArtifactKind.MODEL_DEFINITION),
        model_config=artifact(ManifestArtifactKind.MODEL_CONFIG),
        behavior_rule=behavior_rule,
        semantic_schema=artifact(ManifestArtifactKind.SEMANTIC_SCHEMA),
        optimizer_config=artifact(ManifestArtifactKind.OPTIMIZER_CONFIG),
        trainer_implementation=artifact(
            ManifestArtifactKind.TRAINER_IMPLEMENTATION
        ),
        semantic_schema_version=semantic_schema_version,
    )


def behavior_manifest_fixture() -> BehaviorManifest:
    return behavior_manifest_template_fixture().bind(
        ManifestArtifactId(
            ManifestArtifactKind.MODEL_CHECKPOINT,
            bytes([int(ManifestArtifactKind.MODEL_CHECKPOINT)]) * 32,
        ),
        training_step=0,
    )


def decision_batch_fixture(
    *,
    slot: int,
    semantic_row: int,
    selected_ordinal: int,
    manifest_id: BehaviorManifestId,
    selection_probability: SelectionProbability = DETERMINISTIC_SELECTION,
) -> DecisionExperienceBatch:
    lineage = DecisionLineage(
        key=AttemptKey(
            slot_index=slot,
            episode_seed=100 + slot,
            episode_generation=0,
            attempt_index=1,
        ),
        recoveries_used=0,
    )
    return DecisionExperienceBatch(
        payload=select_semantic_decision_rows(
            semantic_batch_fixture(),
            [semantic_row],
        ),
        lineages=(lineage,),
        selected_ordinals=(selected_ordinal,),
        selection_probabilities=(selection_probability,),
        behavior_manifest_id=manifest_id,
        decision_count=1,
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
