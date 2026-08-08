from __future__ import annotations

from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

import pytest

pytest.importorskip("torch")

from learning.tests.driver_fixtures import NumpyWinningBatchEnv  # noqa: E402
from learning.tests.run_training_fixtures import published_behavior  # noqa: E402
from sts_learning.collect_run_combat_roots import (  # noqa: E402
    RequiredPotionSlot,
    RunCombatRootCollectionError,
    RunCombatRootCollectionConfig,
    run_run_combat_root_collection,
)
from sts_learning.evaluate_run import RunPotionLane  # noqa: E402


class _RootCapturingWinningEnv(NumpyWinningBatchEnv):
    @staticmethod
    def supported_potion_ids() -> list[str]:
        return ["FearPotion", "FirePotion"]

    def public_run_contexts(self) -> list[tuple[int, SimpleNamespace]]:
        return [
            (
                slot,
                SimpleNamespace(
                    boundary_kind=2 if terminal else 1,
                    is_combat=not terminal,
                    is_terminal=terminal,
                    strategic_context_kind=None,
                    seed=self.seeds[slot],
                    act=1,
                    floor=3,
                    hp=20 if terminal else 70,
                    max_hp=80,
                    gold=50,
                    potion_ids=["FearPotion", None, None],
                    monster_ids=[] if terminal else ["JawWorm"],
                ),
            )
            for slot, terminal in enumerate(self.terminal)
        ]

    def combat_root_contexts(self) -> list[tuple[int, SimpleNamespace]]:
        return [
            (
                slot,
                SimpleNamespace(
                    act=1,
                    floor=3,
                    hp=70,
                    max_hp=80,
                    filled_potion_count=1,
                    usable_potion_count=1,
                ),
            )
            for slot, terminal in enumerate(self.terminal)
            if not terminal
        ]

    def combat_root_artifact_bytes(
        self,
        slot_indices: list[int],
        *,
        max_bytes: int,
    ) -> bytes:
        assert slot_indices == [0]
        payload = b"opaque-root:" + self.seeds[0].to_bytes(8, "big")
        assert len(payload) <= max_bytes
        return payload


def test_collection_captures_one_potion_root_per_seed_and_merges_once(
    tmp_path: Path,
) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_RootCapturingWinningEnv,
        environment_without_combat_potions=_RootCapturingWinningEnv,
        environment_from_checkpoint=(
            _RootCapturingWinningEnv.from_checkpoint_bytes
        ),
    )
    merge_calls: list[tuple[bytes, ...]] = []

    def merge(payloads: list[bytes], *, max_bytes: int) -> bytes:
        merge_calls.append(tuple(payloads))
        assert max_bytes == 1024
        return b"merged-root-artifact"

    output = tmp_path / "later-potion-roots.bin"
    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=2,
            max_batch_steps=2,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            min_floor=2,
            min_usable_potions=1,
            potion_lane=RunPotionLane.NEVER,
            max_artifact_bytes=1024,
            required_potion=RequiredPotionSlot(0, "FearPotion"),
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=merge,
    )

    assert output.read_bytes() == b"merged-root-artifact"
    assert len(merge_calls) == 1
    assert len(merge_calls[0]) == 2
    assert merge_calls[0][0] != merge_calls[0][1]
    assert summary["root_count"] == 2
    assert summary["terminal_attempts"] == 2
    assert summary["required_potion_id"] == "FearPotion"
    assert summary["required_potion_slot"] == 0
    roots = summary["roots"]
    assert isinstance(roots, tuple)
    assert len({root["seed"] for root in roots}) == 2
    assert all(root["potion_ids"] == ("FearPotion", None, None) for root in roots)
    assert all(root["monster_ids"] == ("JawWorm",) for root in roots)
    assert all(root["prior_combats"] == () for root in roots)


def test_bounded_unmatched_potion_collection_publishes_no_artifact(
    tmp_path: Path,
) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_RootCapturingWinningEnv,
        environment_without_combat_potions=_RootCapturingWinningEnv,
        environment_from_checkpoint=(
            _RootCapturingWinningEnv.from_checkpoint_bytes
        ),
    )
    output = tmp_path / "incomplete-roots.bin"

    with pytest.raises(RunCombatRootCollectionError, match="did not reach"):
        run_run_combat_root_collection(
            RunCombatRootCollectionConfig(
                behavior=behavior,
                output=output,
                root_count=1,
                max_batch_steps=1,
                wall_ms=10_000,
                behavior_seed=95,
                training_seed_start=200,
                min_floor=2,
                min_usable_potions=1,
                potion_lane=RunPotionLane.NEVER,
                max_artifact_bytes=1024,
                required_potion=RequiredPotionSlot(0, "FirePotion"),
            ),
            combat_bridge=combat_bridge,
            run_bridge=run_bridge,
            artifact_merger=lambda *_args, **_kwargs: pytest.fail(
                "incomplete collection must not merge"
            ),
        )

    assert not output.exists()
