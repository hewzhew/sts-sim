from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.run_training_fixtures import published_behavior
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning.combat_potion_lane import CombatPotionLane
from sts_learning.torch_combat_session_config import CombatSessionBridge
from sts_learning.train_combat import (
    CombatTrainingCommandConfig,
    run_combat_training,
)


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
    assert records[0]["schema"] == "sts-learning-combat-training-v3"
    assert records[0]["potion_lane"] == "never"
    assert records[0]["potion_slots"] == []
    assert records[0]["initialization"] == "random"
    assert records[0]["warm_start_manifest_id"] is None
    assert records[1]["roots"][1]["slot_index"] == 1
    assert records[1]["roots"][1]["selected_objective"] == "hp"
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
    assert records[-1]["final_manifest_id"] != configuration[
        "warm_start_manifest_id"
    ]
