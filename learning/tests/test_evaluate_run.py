from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.driver_fixtures import (
    FakeCheckpointBatch,
    NumpyWinningBatchEnv,
)
from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning.evaluate_run import (
    RunEvaluationCommandConfig,
    run_run_evaluation,
)
from sts_learning.torch_combat_session_config import CombatSessionBridge
from sts_learning.torch_session_config import (
    CategoricalSessionBridge,
)
from sts_learning.train_run import (
    RunTrainingCommandConfig,
    run_run_training,
)
from sts_learning.train_combat import (
    CombatTrainingCommandConfig,
    run_combat_training,
)


class _CombatRootSource:
    def combat_group(
        self,
        slot_index: int,
        replicate_count: int,
        potion_slots: Sequence[int] | None = None,
    ) -> OneRoundCombatGroup:
        assert replicate_count == 2
        assert potion_slots is None
        return OneRoundCombatGroup(
            f"{slot_index + 1:02x}" * 32,
            f"{slot_index + 17:02x}" * 32,
            (True, False) if slot_index == 0 else (True, True),
            potion_slots=None,
        )


def test_run_evaluation_uses_frozen_combat_behavior_without_recovery(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    behavior, combat_bridge, run_bridge = _published_behavior(tmp_path)
    capsys.readouterr()

    output = tmp_path / "run-evaluation"
    summary = run_run_evaluation(
        RunEvaluationCommandConfig(
            behavior=behavior,
            output=output,
            slot_count=2,
            terminal_attempts=2,
            max_batch_steps=1,
            behavior_seed=501,
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
    )

    assert summary["schema"] == "sts-learning-run-held-out-evaluation-v1"
    assert summary["kind"] == "completed"
    assert summary["target_reached"] is True
    assert summary["step_limit_reached"] is False
    assert summary["terminal_attempts"] == 2
    assert summary["victories"] == 2
    assert summary["defeats"] == 0
    assert summary["terminal_floor_sum"] == 80
    assert summary["min_terminal_floor"] == 40
    assert summary["max_terminal_floor"] == 40
    assert summary["terminal_floor_counts"] == ((40, 2),)
    assert summary["terminal_act_counts"] == ((3, 2),)
    assert summary["recoveries"] == 0
    assert (output / "evaluation.json").is_file()
    stdout = capsys.readouterr().out
    assert (
        "run_evaluation_complete=true target_reached=true attempts=2/2 "
        "victories=2 defeats=0 floor_sum=80 floor_min=40 floor_max=40 "
        "floor_counts=40:2 act_counts=3:2"
    ) in stdout


def test_run_training_warm_starts_publishes_and_evaluates(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    behavior, combat_bridge, run_bridge = _published_behavior(tmp_path)
    capsys.readouterr()

    output = tmp_path / "run-training"
    summary = run_run_training(
        RunTrainingCommandConfig(
            warm_start_behavior=behavior,
            output=output,
            slot_count=2,
            generations=1,
            attempts_per_update=2,
            max_batch_steps_per_generation=1,
            model_seed=43,
            behavior_seed=94,
            training_seed_start=0,
            evaluation_attempts=2,
            evaluation_max_batch_steps=1,
            evaluation_behavior_seed=501,
            held_out_seed_start=1000,
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
    )

    assert summary["kind"] == "completed"
    assert summary["generations"] == 1
    assert summary["optimizer_steps"] == 1
    assert summary["held_out_target_reached"] is True
    assert summary["held_out_attempts"] == 2
    assert summary["held_out_victories"] == 2
    assert summary["held_out_floor_sum"] == 80
    assert summary["held_out_floor_counts"] == ((40, 2),)
    assert (output / "summary.json").is_file()
    records = tuple(
        json.loads(line)
        for line in (output / "training.jsonl").read_text(
            encoding="utf-8"
        ).splitlines()
    )
    assert tuple(record["kind"] for record in records) == (
        "configuration",
        "generation",
        "completed",
    )
    stdout = capsys.readouterr().out
    assert (
        "run_generation=0 promoted=true attempts=2 victories=2 defeats=0 "
        "floor_sum=80 floor_counts=40:2"
    ) in stdout
    assert (
        "run_training_complete=true generations=1 optimizer_steps=1 "
        "held_out_attempts=2/2 held_out_victories=2 "
        "held_out_floor_sum=80 held_out_floor_counts=40:2"
    ) in stdout


def _published_behavior(
    root: Path,
) -> tuple[Path, CombatSessionBridge, CategoricalSessionBridge]:
    artifact = root / "combat-roots.bin"
    artifact.write_bytes(b"opaque-combat-roots")
    schema = semantic_schema_fixture()
    combat_bridge = CombatSessionBridge(
        combat_roots_from_artifact=lambda payload, **_: _CombatRootSource(),
        semantic_schema=schema,
    )
    behavior = root / "behavior"
    run_combat_training(
        CombatTrainingCommandConfig(
            artifact=artifact,
            output=behavior,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=41,
            behavior_seed_base=92,
        ),
        bridge=combat_bridge,
    )
    run_bridge = CategoricalSessionBridge(
        environment=NumpyWinningBatchEnv,
        environment_from_checkpoint=NumpyWinningBatchEnv.from_checkpoint_bytes,
        checkpoint_bank_from_checkpoint=(
            FakeCheckpointBatch.from_checkpoint_bytes
        ),
        semantic_schema=schema,
    )
    return behavior, combat_bridge, run_bridge
