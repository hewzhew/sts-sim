from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.run_training_fixtures import published_behavior
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning import CombatAllLossAxis
from sts_learning.combat_potion_lane import CombatPotionLane
from sts_learning import RunPolicyUpdateConfig
from sts_learning.published_combat_behavior import (
    PublishedCombatBehaviorError,
    recover_published_combat_behavior,
)
from sts_learning.torch_combat_session_config import CombatSessionBridge
from sts_learning.torch_combat_session_config import CombatWinSessionLimits
from sts_learning.train_combat import (
    CombatTrainingCommandConfig,
    run_combat_training,
)
from sts_learning.train_run import RunTrainingCommandConfig, run_run_training


_ROOTS = (
    ("12" * 32, "ab" * 32, (True, False), None),
    ("34" * 32, "cd" * 32, (True, True), (70, 50)),
)


class _RootSource:
    def __init__(self) -> None:
        self.calls: list[tuple[int, tuple[int, ...] | None]] = []

    def combat_group(
        self,
        slot_index: int,
        replicate_count: int,
        potion_slots: Sequence[int] | None = None,
    ):
        assert replicate_count == 2
        normalized_slots = (
            None if potion_slots is None else tuple(potion_slots)
        )
        self.calls.append((slot_index, normalized_slots))
        root_id, state_hash, wins, final_hps = _ROOTS[slot_index]
        return OneRoundCombatGroup(
            root_id,
            state_hash,
            wins,
            final_hps=final_hps,
            potion_slots=normalized_slots,
        )


class _EnemyProgressRootSource:
    def combat_group(
        self,
        slot_index: int,
        replicate_count: int,
        potion_slots: Sequence[int] | None = None,
    ) -> OneRoundCombatGroup:
        assert replicate_count == 2
        return OneRoundCombatGroup(
            f"{slot_index + 5:02x}" * 32,
            f"{slot_index + 21:02x}" * 32,
            (False, False),
            enemy_final_hps=(30, 10),
            potion_slots=None if potion_slots is None else tuple(potion_slots),
        )


def test_training_command_runs_updates_journals_and_publishes(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    artifact = tmp_path / "roots.bin"
    artifact.write_bytes(b"opaque-combat-roots")
    output = tmp_path / "run"
    source = _RootSource()
    bridge = CombatSessionBridge(
        combat_roots_from_artifact=lambda payload, **_: source,
        semantic_schema=semantic_schema_fixture(),
    )

    summary = run_combat_training(
        CombatTrainingCommandConfig(
            artifact=artifact,
            output=output,
            root_count=2,
            replicate_count=2,
            updates=2,
            model_seed=41,
            behavior_seed_base=92,
            potion_lane=CombatPotionLane.NEVER,
        ),
        bridge=bridge,
    )

    assert source.calls == [
        (0, ()),
        (1, ()),
        (0, ()),
        (1, ()),
    ]
    assert summary["optimizer_steps"] == 2
    records = tuple(
        json.loads(line)
        for line in (output / "training.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    )
    assert tuple(record["kind"] for record in records) == (
        "configuration",
        "generation",
        "generation",
        "completed",
    )
    assert records[0]["schema"] == "sts-learning-combat-training-v5"
    assert records[0]["policy_update_rule"] == "REINFORCE"
    assert records[0]["potion_lane"] == "never"
    assert records[0]["potion_slots"] == []
    assert records[0]["initialization"] == "random"
    assert records[0]["warm_start_manifest_id"] is None
    assert records[0]["warm_start_training_kind"] is None
    assert records[1]["roots"][1]["slot_index"] == 1
    assert records[1]["roots"][1]["selected_objective"] == "hp"
    assert records[1]["roots"][1]["enemy_hp_progress_signal_replicates"] == 0
    assert records[1]["active_manifest_id_before"] != records[1][
        "active_manifest_id_after"
    ]
    assert len(tuple((output / "behavior-checkpoints").iterdir())) == 1
    assert len(tuple((output / "behavior-manifests").iterdir())) == 1
    stdout = capsys.readouterr().out
    assert "root_wins=1,2 root_objectives=win,hp" in stdout


def test_training_command_warm_starts_from_a_verified_published_behavior(
    tmp_path: Path,
) -> None:
    behavior, bridge, _ = published_behavior(tmp_path)
    output = tmp_path / "warm-started"

    run_combat_training(
        CombatTrainingCommandConfig(
            artifact=tmp_path / "combat-roots.bin",
            output=output,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=42,
            behavior_seed_base=100,
            potion_lane=CombatPotionLane.NEVER,
            warm_start_behavior=behavior,
        ),
        bridge=bridge,
    )

    records = tuple(
        json.loads(line)
        for line in (output / "training.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    )
    configuration = records[0]
    assert configuration["initialization"] == "published-behavior"
    assert configuration["warm_start_behavior"] == str(behavior.resolve())
    assert configuration["warm_start_manifest_id"] is not None
    assert configuration["warm_start_checkpoint_id"] is not None
    assert configuration["warm_start_training_step"] == 1
    assert configuration["warm_start_training_kind"] == "combat"
    assert records[-1]["final_manifest_id"] != configuration[
        "warm_start_manifest_id"
    ]


def test_training_command_publishes_explicit_all_loss_objective(
    tmp_path: Path,
) -> None:
    artifact = tmp_path / "all-loss-roots.bin"
    artifact.write_bytes(b"opaque-all-loss-roots")
    output = tmp_path / "all-loss-training"
    bridge = CombatSessionBridge(
        combat_roots_from_artifact=lambda payload, **_: _EnemyProgressRootSource(),
        semantic_schema=semantic_schema_fixture(),
    )

    run_combat_training(
        CombatTrainingCommandConfig(
            artifact=artifact,
            output=output,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=45,
            behavior_seed_base=110,
            potion_lane=CombatPotionLane.NEVER,
            all_loss_axis=CombatAllLossAxis.ENEMY_HP_PROGRESS,
        ),
        bridge=bridge,
    )

    records = tuple(
        json.loads(line)
        for line in (output / "training.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    )
    assert records[0]["all_loss_axis"] == "ENEMY_HP_PROGRESS"
    assert records[1]["enemy_hp_progress_signal_group_count"] == 2
    assert tuple(
        root["selected_objective"] for root in records[1]["roots"]
    ) == ("enemy-hp-progress", "enemy-hp-progress")
    assert records[-1]["total_unresolved"] == 0

    recovered = recover_published_combat_behavior(
        output,
        bridge,
        CombatWinSessionLimits(),
        (701,),
    )
    assert (
        recovered.training_all_loss_axis
        is CombatAllLossAxis.ENEMY_HP_PROGRESS
    )


def test_recovery_reports_semantic_schema_version_mismatch(tmp_path: Path) -> None:
    behavior, bridge, _ = published_behavior(tmp_path)
    mismatched_schema = dict(bridge.semantic_schema)
    mismatched_schema["version"] = int(mismatched_schema["version"]) + 1
    mismatched_bridge = CombatSessionBridge(
        combat_roots_from_artifact=bridge.combat_roots_from_artifact,
        semantic_schema=mismatched_schema,
    )

    with pytest.raises(
        PublishedCombatBehaviorError,
        match="semantic schema version 2 does not match installed version 3",
    ):
        recover_published_combat_behavior(
            behavior,
            mismatched_bridge,
            CombatWinSessionLimits(),
            (701,),
        )


def test_training_command_warm_starts_actor_only_from_run_value_behavior(
    tmp_path: Path,
) -> None:
    combat_behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_behavior = tmp_path / "run-value-behavior"
    run_run_training(
        RunTrainingCommandConfig(
            warm_start_behavior=combat_behavior,
            output=run_behavior,
            slot_count=2,
            generations=1,
            attempts_per_update=2,
            max_batch_steps_per_generation=1,
            model_seed=43,
            behavior_seed=94,
            training_seed_start=0,
            evaluation_attempts=2,
            evaluation_max_batch_steps=2,
            evaluation_behavior_seed=501,
            held_out_seed_start=1000,
            policy_update=RunPolicyUpdateConfig.ppo_clip_value(),
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
    )
    output = tmp_path / "run-warm-started-combat"

    run_combat_training(
        CombatTrainingCommandConfig(
            artifact=tmp_path / "combat-roots.bin",
            output=output,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=44,
            behavior_seed_base=102,
            potion_lane=CombatPotionLane.NEVER,
            warm_start_behavior=run_behavior,
        ),
        bridge=combat_bridge,
        run_bridge=run_bridge,
    )

    configuration = json.loads(
        (output / "training.jsonl").read_text(encoding="utf-8").splitlines()[0]
    )
    assert configuration["warm_start_training_kind"] == "run"
    assert configuration["warm_start_manifest_id"] is not None
    assert configuration["policy_update_rule"] == "REINFORCE"
