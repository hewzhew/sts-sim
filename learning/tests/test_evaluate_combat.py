from __future__ import annotations

import json
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning.evaluate_combat import (
    CombatEvaluationCommandConfig,
    run_combat_evaluation,
)
from sts_learning.torch_combat_session_config import CombatSessionBridge
from sts_learning.train_combat import (
    CombatTrainingCommandConfig,
    run_combat_training,
)


class _RootSource:
    def __init__(
        self,
        roots: tuple[
            tuple[str, str, tuple[bool, bool], tuple[int, int] | None],
            ...,
        ],
    ) -> None:
        self.roots = roots
        self.calls: list[int] = []

    def combat_group(self, slot_index: int, replicate_count: int):
        assert replicate_count == 2
        self.calls.append(slot_index)
        root_id, state_hash, wins, final_hps = self.roots[slot_index]
        return OneRoundCombatGroup(
            root_id,
            state_hash,
            wins,
            final_hps=final_hps,
        )


def test_evaluation_recovers_published_behavior_without_training_or_experience(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    # This contract must cross publication, durable recovery, and a distinct
    # artifact; a smaller fixture would no longer protect the command boundary.
    training_artifact = tmp_path / "training.bin"
    evaluation_artifact = tmp_path / "evaluation.bin"
    training_artifact.write_bytes(b"opaque-training-roots")
    evaluation_artifact.write_bytes(b"opaque-evaluation-roots")
    training_source = _RootSource(
        (
            ("12" * 32, "ab" * 32, (True, False), None),
            ("34" * 32, "cd" * 32, (True, True), (70, 50)),
        )
    )
    evaluation_source = _RootSource(
        (
            ("56" * 32, "ef" * 32, (False, True), (0, 61)),
            ("78" * 32, "01" * 32, (True, True), (72, 68)),
        )
    )

    def load(payload: bytes, **_: object) -> _RootSource:
        if payload == training_artifact.read_bytes():
            return training_source
        if payload == evaluation_artifact.read_bytes():
            return evaluation_source
        raise AssertionError("unexpected artifact")

    bridge = CombatSessionBridge(
        combat_roots_from_artifact=load,
        semantic_schema=semantic_schema_fixture(),
    )
    behavior = tmp_path / "behavior"
    run_combat_training(
        CombatTrainingCommandConfig(
            artifact=training_artifact,
            output=behavior,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=41,
            behavior_seed_base=92,
        ),
        bridge=bridge,
    )
    training_journal_before = (behavior / "training.jsonl").read_bytes()
    output = tmp_path / "held-out"

    summary = run_combat_evaluation(
        CombatEvaluationCommandConfig(
            artifact=evaluation_artifact,
            behavior=behavior,
            output=output,
            root_count=2,
            replicate_count=2,
            behavior_seed_base=1_000,
        ),
        bridge=bridge,
    )

    assert training_source.calls == [0, 1]
    assert evaluation_source.calls == [0, 1]
    assert summary["wins"] == 3
    assert summary["losses"] == 1
    assert summary["behavior_training_step"] == 1
    assert summary["behavior_training_root_count"] == 2
    assert summary["final_hp_sum"] == 201
    assert tuple(root["wins"] for root in summary["roots"]) == (1, 2)
    assert tuple(
        outcome["final_hp"] for outcome in summary["roots"][0]["outcomes"]
    ) == (0, 61)
    assert (behavior / "training.jsonl").read_bytes() == training_journal_before
    assert tuple(path.name for path in output.iterdir()) == ("evaluation.json",)
    assert json.loads((output / "evaluation.json").read_text(encoding="utf-8"))[
        "behavior_manifest_id"
    ] == summary["behavior_manifest_id"]
    stdout = capsys.readouterr().out
    assert "evaluation_complete=true wins=3 losses=1 root_wins=1,2" in stdout
