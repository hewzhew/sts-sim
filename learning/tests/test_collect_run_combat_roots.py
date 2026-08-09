from __future__ import annotations

from dataclasses import replace
from pathlib import Path
from types import SimpleNamespace

import pytest

pytest.importorskip("torch")

from learning.tests.driver_fixtures import NumpyWinningBatchEnv  # noqa: E402
from learning.tests.run_training_fixtures import published_behavior  # noqa: E402
from sts_learning.combat_potion_lane import CombatPotionLane  # noqa: E402
from sts_learning.collect_run_combat_roots import (  # noqa: E402
    EncounterQuota,
    RequiredPotionSlot,
    RunCombatRootCollectionError,
    RunCombatRootCollectionConfig,
    run_run_combat_root_collection,
)
from sts_learning.evaluate_run import RunPotionLane  # noqa: E402
from sts_learning.torch_behavior import FrozenDecisionRule  # noqa: E402


class _RootCapturingWinningEnv(NumpyWinningBatchEnv):
    potion_ids = ("FearPotion", None, None)

    def __init__(self, seeds: list[int], ascension_level: int) -> None:
        assert ascension_level == 20
        super().__init__(seeds)

    @staticmethod
    def supported_potion_ids() -> list[str]:
        return ["FearPotion", "FirePotion"]

    @staticmethod
    def canonical_encounter_id(raw: str) -> str:
        if raw not in {"JawWorm", "Cultist"}:
            raise ValueError("unsupported encounter")
        return raw

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
                    potion_ids=list(self.potion_ids),
                    encounter_id=None if terminal else "JawWorm",
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
                    filled_potion_count=sum(
                        potion is not None for potion in self.potion_ids
                    ),
                    usable_potion_count=sum(
                        potion is not None for potion in self.potion_ids
                    ),
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


class _PotionlessRootCapturingWinningEnv(_RootCapturingWinningEnv):
    potion_ids = (None, None, None)


class _AlternatingEncounterRootEnv(_PotionlessRootCapturingWinningEnv):
    def public_run_contexts(self) -> list[tuple[int, SimpleNamespace]]:
        rows = super().public_run_contexts()
        for slot, context in rows:
            if not self.terminal[slot]:
                context.encounter_id = (
                    "JawWorm" if self.seeds[slot] % 2 == 0 else "Cultist"
                )
                context.monster_ids = [
                    "JawWorm" if self.seeds[slot] % 2 == 0 else "Cultist"
                ]
        return rows


def test_collection_captures_one_potion_root_per_seed_and_merges_once(
    tmp_path: Path,
) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(
        tmp_path,
        potion_lane=CombatPotionLane.NEVER,
    )
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
            ascension_level=20,
            min_floor=2,
            min_usable_potions=1,
            potion_lane=RunPotionLane.TRAINED,
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
    assert summary["requested_run_potion_lane"] == "trained"
    assert summary["run_potion_lane"] == "never"
    assert summary["schema"] == "sts-learning-run-combat-root-collection-v3"
    assert summary["ascension_level"] == 20
    assert summary["combat_decision_rule"] == "sampled"
    assert summary["collection_manifest_id"] == summary["behavior_manifest_id"]
    roots = summary["roots"]
    assert isinstance(roots, tuple)
    assert len({root["seed"] for root in roots}) == 2
    assert all(root["potion_ids"] == ("FearPotion", None, None) for root in roots)
    assert all(root["monster_ids"] == ("JawWorm",) for root in roots)
    assert all(root["prior_combats"] == () for root in roots)


def test_collection_can_make_only_combat_decisions_greedy(tmp_path: Path) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_PotionlessRootCapturingWinningEnv,
        environment_without_combat_potions=_PotionlessRootCapturingWinningEnv,
        environment_from_checkpoint=(
            _PotionlessRootCapturingWinningEnv.from_checkpoint_bytes
        ),
    )
    output = tmp_path / "combat-greedy-root.bin"

    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=1,
            max_batch_steps=1,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            ascension_level=20,
            combat_decision_rule=FrozenDecisionRule.GREEDY,
            min_floor=2,
            min_usable_potions=0,
            max_artifact_bytes=1024,
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=lambda payloads, *, max_bytes: payloads[0],
    )

    assert output.is_file()
    assert summary["combat_decision_rule"] == "greedy"
    assert summary["collection_manifest_id"] != summary["behavior_manifest_id"]


def test_collection_can_capture_a_potionless_combat_root(tmp_path: Path) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_PotionlessRootCapturingWinningEnv,
        environment_without_combat_potions=_PotionlessRootCapturingWinningEnv,
        environment_from_checkpoint=(
            _PotionlessRootCapturingWinningEnv.from_checkpoint_bytes
        ),
    )
    output = tmp_path / "potionless-root.bin"

    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=1,
            max_batch_steps=1,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            ascension_level=20,
            min_floor=2,
            min_usable_potions=0,
            max_artifact_bytes=1024,
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=lambda payloads, *, max_bytes: payloads[0],
    )

    assert output.is_file()
    assert summary["min_usable_potions"] == 0
    assert summary["roots"][0]["potion_ids"] == (None, None, None)
    assert summary["roots"][0]["usable_potion_count"] == 0


def test_collection_can_require_distinct_encounters(tmp_path: Path) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_AlternatingEncounterRootEnv,
        environment_without_combat_potions=_AlternatingEncounterRootEnv,
        environment_from_checkpoint=_AlternatingEncounterRootEnv.from_checkpoint_bytes,
    )
    output = tmp_path / "distinct-encounters.bin"

    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=2,
            max_batch_steps=2,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            ascension_level=20,
            min_floor=2,
            min_usable_potions=0,
            max_artifact_bytes=1024,
            distinct_encounters=True,
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=lambda payloads, *, max_bytes: b"merged",
    )

    assert summary["distinct_encounters"] is True
    assert {root["encounter_id"] for root in summary["roots"]} == {
        "Cultist",
        "JawWorm",
    }


def test_collection_can_require_one_exact_encounter(tmp_path: Path) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_RootCapturingWinningEnv,
        environment_without_combat_potions=_RootCapturingWinningEnv,
        environment_from_checkpoint=_RootCapturingWinningEnv.from_checkpoint_bytes,
    )
    output = tmp_path / "jaw-worm-root.bin"

    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=1,
            max_batch_steps=1,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            ascension_level=20,
            min_floor=2,
            min_usable_potions=0,
            max_artifact_bytes=1024,
            required_encounter_id="JawWorm",
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=lambda payloads, *, max_bytes: payloads[0],
    )

    assert summary["required_encounter_id"] == "JawWorm"
    assert summary["roots"][0]["encounter_id"] == "JawWorm"


def test_collection_fulfills_each_encounter_quota_from_distinct_seeds(
    tmp_path: Path,
) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_AlternatingEncounterRootEnv,
        environment_without_combat_potions=_AlternatingEncounterRootEnv,
        environment_from_checkpoint=_AlternatingEncounterRootEnv.from_checkpoint_bytes,
    )
    output = tmp_path / "encounter-quotas.bin"

    summary = run_run_combat_root_collection(
        RunCombatRootCollectionConfig(
            behavior=behavior,
            output=output,
            root_count=3,
            max_batch_steps=4,
            wall_ms=10_000,
            behavior_seed=94,
            training_seed_start=100,
            ascension_level=20,
            min_floor=2,
            min_usable_potions=0,
            max_artifact_bytes=1024,
            encounter_quotas=(
                EncounterQuota("JawWorm", 1),
                EncounterQuota("Cultist", 2),
            ),
        ),
        combat_bridge=combat_bridge,
        run_bridge=run_bridge,
        artifact_merger=lambda payloads, *, max_bytes: b"merged",
    )

    assert output.is_file()
    assert [root["seed"] for root in summary["roots"]] == [100, 101, 103]
    assert summary["encounter_quotas"] == (
        {
            "encounter_id": "JawWorm",
            "requested_roots": 1,
            "captured_roots": 1,
        },
        {
            "encounter_id": "Cultist",
            "requested_roots": 2,
            "captured_roots": 2,
        },
    )


def test_incomplete_encounter_quota_publishes_no_artifact(tmp_path: Path) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_AlternatingEncounterRootEnv,
        environment_without_combat_potions=_AlternatingEncounterRootEnv,
        environment_from_checkpoint=_AlternatingEncounterRootEnv.from_checkpoint_bytes,
    )
    output = tmp_path / "incomplete-encounter-quotas.bin"

    with pytest.raises(
        RunCombatRootCollectionError,
        match=r"Cultist=1/2",
    ):
        run_run_combat_root_collection(
            RunCombatRootCollectionConfig(
                behavior=behavior,
                output=output,
                root_count=3,
                max_batch_steps=3,
                wall_ms=10_000,
                behavior_seed=94,
                training_seed_start=100,
                ascension_level=20,
                min_floor=2,
                min_usable_potions=0,
                max_artifact_bytes=1024,
                encounter_quotas=(
                    EncounterQuota("JawWorm", 1),
                    EncounterQuota("Cultist", 2),
                ),
            ),
            combat_bridge=combat_bridge,
            run_bridge=run_bridge,
            artifact_merger=lambda *_args, **_kwargs: pytest.fail(
                "incomplete encounter quotas must not merge"
            ),
        )

    assert not output.exists()


def test_required_encounter_rejects_unsupported_identity_before_collection(
    tmp_path: Path,
) -> None:
    behavior, combat_bridge, run_bridge = published_behavior(tmp_path)
    run_bridge = replace(
        run_bridge,
        environment=_RootCapturingWinningEnv,
        environment_without_combat_potions=_RootCapturingWinningEnv,
        environment_from_checkpoint=_RootCapturingWinningEnv.from_checkpoint_bytes,
    )
    output = tmp_path / "unsupported-monster.bin"

    with pytest.raises(
        RunCombatRootCollectionError,
        match="unsupported by the installed bridge",
    ):
        run_run_combat_root_collection(
            RunCombatRootCollectionConfig(
                behavior=behavior,
                output=output,
                root_count=1,
                max_batch_steps=1,
                wall_ms=10_000,
                behavior_seed=94,
                training_seed_start=100,
                ascension_level=20,
                min_floor=2,
                min_usable_potions=0,
                max_artifact_bytes=1024,
                required_encounter_id="NotAMonster",
            ),
            combat_bridge=combat_bridge,
            run_bridge=run_bridge,
        )

    assert not output.exists()


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
                ascension_level=20,
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
