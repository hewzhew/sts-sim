from __future__ import annotations

from collections.abc import Sequence
from pathlib import Path

from learning.tests.driver_fixtures import (
    FakeCheckpointBatch,
    NumpyWinningBatchEnv,
)
from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning.combat_potion_lane import CombatPotionLane
from sts_learning.torch_combat_session_config import CombatSessionBridge
from sts_learning.torch_session_config import CategoricalSessionBridge
from sts_learning.train_combat import (
    CombatTrainingCommandConfig,
    run_combat_training,
)


class CombatRootSource:
    def combat_group(
        self,
        slot_index: int,
        replicate_count: int,
        potion_slots: Sequence[int] | None = None,
    ) -> OneRoundCombatGroup:
        assert replicate_count == 2
        assert potion_slots in (None, ())
        return OneRoundCombatGroup(
            f"{slot_index + 1:02x}" * 32,
            f"{slot_index + 17:02x}" * 32,
            (True, False) if slot_index == 0 else (True, True),
            potion_slots=None if potion_slots is None else tuple(potion_slots),
        )


def published_behavior(
    root: Path,
    *,
    potion_lane: CombatPotionLane = CombatPotionLane.ALL,
) -> tuple[Path, CombatSessionBridge, CategoricalSessionBridge]:
    artifact = root / "combat-roots.bin"
    artifact.write_bytes(b"opaque-combat-roots")
    schema = semantic_schema_fixture()
    combat_bridge = CombatSessionBridge(
        combat_roots_from_artifact=lambda payload, **_: CombatRootSource(),
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
            potion_lane=potion_lane,
        ),
        bridge=combat_bridge,
    )
    run_bridge = CategoricalSessionBridge(
        environment=NumpyWinningBatchEnv,
        environment_without_combat_potions=NumpyWinningBatchEnv,
        environment_from_checkpoint=NumpyWinningBatchEnv.from_checkpoint_bytes,
        checkpoint_bank_from_checkpoint=(
            FakeCheckpointBatch.from_checkpoint_bytes
        ),
        semantic_schema=schema,
    )
    return behavior, combat_bridge, run_bridge
