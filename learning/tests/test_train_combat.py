from __future__ import annotations

import json
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
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
        self.calls: list[int] = []

    def combat_group(self, slot_index: int, replicate_count: int):
        assert replicate_count == 2
        self.calls.append(slot_index)
        root_id, state_hash, wins, final_hps = _ROOTS[slot_index]
        return OneRoundCombatGroup(
            root_id,
            state_hash,
            wins,
            final_hps=final_hps,
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
        ),
        bridge=bridge,
    )

    assert source.calls == [0, 1, 0, 1]
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
    assert records[1]["roots"][1]["slot_index"] == 1
    assert records[1]["roots"][1]["selected_objective"] == "hp"
    assert records[1]["active_manifest_id_before"] != records[1][
        "active_manifest_id_after"
    ]
    assert len(tuple((output / "behavior-checkpoints").iterdir())) == 1
    assert len(tuple((output / "behavior-manifests").iterdir())) == 1
    stdout = capsys.readouterr().out
    assert "root_wins=1,2 root_objectives=win,hp" in stdout
