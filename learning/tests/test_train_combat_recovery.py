from __future__ import annotations

import json
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.test_torch_combat_recovery_session import _ReplayableSource
from sts_learning.combat_potion_lane import CombatPotionLane
from sts_learning.published_combat_behavior import recover_published_combat_behavior
from sts_learning.torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinSessionLimits,
)
from sts_learning.train_combat_recovery import (
    run_combat_recovery_training,
)
from sts_learning.train_combat import CombatTrainingCommandConfig


def test_recovery_training_journal_is_a_recoverable_combat_publication(
    tmp_path: Path,
) -> None:
    artifact = tmp_path / "source.bin"
    artifact.write_bytes(b"opaque-root")
    output = tmp_path / "training"
    source = _ReplayableSource()
    bridge = CombatSessionBridge(
        combat_roots_from_artifact=lambda payload, **_: source,
        semantic_schema=semantic_schema_fixture(),
    )

    summary = run_combat_recovery_training(
        CombatTrainingCommandConfig(
            artifact=artifact,
            output=output,
            root_count=2,
            replicate_count=2,
            updates=1,
            model_seed=7,
            behavior_seed_base=11,
            potion_lane=CombatPotionLane.ROOT_SLOTS,
            potion_slots=(0,),
        ),
        bridge=bridge,
    )

    records = tuple(
        json.loads(line)
        for line in (output / "training.jsonl")
        .read_text(encoding="utf-8")
        .splitlines()
    )
    assert records[0]["curriculum"] == "verified-win-terminal-nearest"
    assert records[0]["teacher_replicate_index"] == 1
    assert records[0]["root_count"] == 2
    assert records[-1]["final_manifest_id"] == summary["final_manifest_id"]
    assert summary["source_wins"] == 1
    assert summary["teacher_final_hp"] == 30

    recovered = recover_published_combat_behavior(
        output,
        bridge,
        CombatWinSessionLimits(),
        (99,),
    )
    assert recovered.training_root_count == 2
    assert recovered.training_potion_lane is CombatPotionLane.ROOT_SLOTS
    assert recovered.training_potion_slots == (0,)
