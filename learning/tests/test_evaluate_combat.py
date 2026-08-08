from __future__ import annotations

import json
from collections.abc import Sequence
from pathlib import Path

import pytest

pytest.importorskip("torch")

from learning.tests.semantic_fixtures import semantic_schema_fixture
from learning.tests.torch_combat_fixtures import OneRoundCombatGroup
from sts_learning.combat_evaluation import combat_observed_resource_frontier
from sts_learning.combat_outcomes import CombatTerminalOutcome
from sts_learning.combat_potion_lane import CombatPotionLane
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
        root_id, state_hash, wins, final_hps = self.roots[slot_index]
        return OneRoundCombatGroup(
            root_id,
            state_hash,
            wins,
            final_hps=final_hps,
            potion_slots=normalized_slots,
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

    assert summary["schema"] == "sts-learning-combat-held-out-evaluation-v5"
    assert summary["potion_lane"] == "all"
    assert summary["potion_slots"] == ()
    assert training_source.calls == [(0, None), (1, None)]
    assert evaluation_source.calls == [(0, None), (1, None)]
    assert summary["wins"] == 3
    assert summary["losses"] == 1
    assert summary["behavior_training_step"] == 1
    assert summary["behavior_training_root_count"] == 2
    assert summary["final_hp_sum"] == 201
    assert summary["gold_delta_sum"] == 0
    assert summary["lost_potion_ids"] == {"EntropicBrew": 2}
    assert summary["gained_potion_ids"] == {"BlockPotion": 2}
    assert summary["observed_resource_frontier_replicates"] == 3
    assert summary["observed_resource_dominated_replicates"] == 0
    assert summary["observed_resource_incomparable_winning_pairs"] == 1
    assert tuple(root["wins"] for root in summary["roots"]) == (1, 2)
    assert summary["roots"][0]["context"]["floor"] == 4
    assert summary["roots"][0]["context"]["gold"] == 99
    assert summary["roots"][0]["context"]["potion_ids"] == (
        "EntropicBrew",
        "GamblersBrew",
    )
    assert tuple(
        outcome["final_hp"] for outcome in summary["roots"][0]["outcomes"]
    ) == (0, 61)
    assert summary["roots"][0]["outcomes"][1]["lost_potion_ids"] == (
        "EntropicBrew",
    )
    assert summary["roots"][0]["outcomes"][1]["gained_potion_ids"] == (
        "BlockPotion",
    )
    assert summary["roots"][1]["observed_resource_order"] == {
        "winning_replicate_indices": (0, 1),
        "frontier_replicate_indices": (0, 1),
        "dominated_replicate_indices": (),
        "dominators_by_replicate": ((), ()),
        "strict_order_pair_count": 0,
        "equivalent_pair_count": 0,
        "incomparable_pair_count": 1,
    }
    assert (behavior / "training.jsonl").read_bytes() == training_journal_before
    assert tuple(path.name for path in output.iterdir()) == ("evaluation.json",)
    assert json.loads((output / "evaluation.json").read_text(encoding="utf-8"))[
        "behavior_manifest_id"
    ] == summary["behavior_manifest_id"]
    all_stdout = capsys.readouterr().out
    assert (
        "evaluation_complete=true wins=3 losses=1 potion_lane=all "
        "potion_slots=all "
        "root_wins=1,2"
    ) in all_stdout
    assert "root_sites=A1F4,A1F4" in all_stdout
    assert (
        "root_resource_frontiers=1,2 root_resource_dominated=0,0" in all_stdout
    )
    assert "root_start_potions=EntropicBrew+GamblersBrew" in all_stdout
    assert (
        "lost_potions=EntropicBrew:2 gained_potions=BlockPotion:2" in all_stdout
    )

    no_potion_output = tmp_path / "held-out-no-potions"
    no_potion_summary = run_combat_evaluation(
        CombatEvaluationCommandConfig(
            artifact=evaluation_artifact,
            behavior=behavior,
            output=no_potion_output,
            root_count=2,
            replicate_count=2,
            behavior_seed_base=1_000,
            potion_lane=CombatPotionLane.NEVER,
        ),
        bridge=bridge,
    )

    assert no_potion_summary["potion_lane"] == "never"
    assert no_potion_summary["potions_used"] == 0
    assert no_potion_summary["potions_discarded"] == 0
    assert no_potion_summary["lost_potion_ids"] == {}
    assert no_potion_summary["gained_potion_ids"] == {}
    assert evaluation_source.calls == [
        (0, None),
        (1, None),
        (0, ()),
        (1, ()),
    ]
    assert all(
        outcome["final_potion_ids"]
        == ("EntropicBrew", "GamblersBrew")
        for root in no_potion_summary["roots"]
        for outcome in root["outcomes"]
    )
    assert (behavior / "training.jsonl").read_bytes() == training_journal_before
    no_potion_stdout = capsys.readouterr().out
    assert (
        "evaluation_complete=true wins=3 losses=1 potion_lane=never "
        "potion_slots=none "
        "root_wins=1,2"
    ) in no_potion_stdout

    selected_output = tmp_path / "held-out-root-slot-one"
    selected_summary = run_combat_evaluation(
        CombatEvaluationCommandConfig(
            artifact=evaluation_artifact,
            behavior=behavior,
            output=selected_output,
            root_count=2,
            replicate_count=2,
            behavior_seed_base=1_000,
            potion_lane=CombatPotionLane.ROOT_SLOTS,
            potion_slots=(1,),
        ),
        bridge=bridge,
    )

    assert selected_summary["potion_lane"] == "root-slots"
    assert selected_summary["potion_slots"] == (1,)
    assert selected_summary["potions_used"] == 2
    assert selected_summary["lost_potion_ids"] == {"GamblersBrew": 2}
    assert selected_summary["gained_potion_ids"] == {"BlockPotion": 2}
    assert evaluation_source.calls[-2:] == [(0, (1,)), (1, (1,))]
    selected_stdout = capsys.readouterr().out
    assert "potion_lane=root-slots potion_slots=1" in selected_stdout


def test_observed_resource_frontier_keeps_hp_potion_tradeoffs_incomparable() -> None:
    def outcome(
        replicate_index: int,
        *,
        won: bool,
        hp: int,
        potions: tuple[str | None, ...],
    ) -> CombatTerminalOutcome:
        return CombatTerminalOutcome(
            replicate_index=replicate_index,
            terminal_kind=1 if won else 2,
            won=won,
            start_hp=80,
            final_hp=hp,
            final_max_hp=80,
            final_gold=99,
            hp_loss=80 - hp,
            turns=3,
            potions_used=0,
            potions_discarded=0,
            cards_played=8,
            final_potion_ids=potions,
        )

    frontier = combat_observed_resource_frontier(
        (
            outcome(0, won=True, hp=60, potions=("BlockPotion", "SkillPotion")),
            outcome(1, won=True, hp=70, potions=("BlockPotion", None)),
            outcome(2, won=True, hp=50, potions=("BlockPotion", None)),
            outcome(3, won=False, hp=0, potions=("BlockPotion", "SkillPotion")),
        )
    )

    assert frontier.winning_replicate_indices == (0, 1, 2)
    assert frontier.frontier_replicate_indices == (0, 1)
    assert frontier.dominated_replicate_indices == (2,)
    assert frontier.dominators_by_replicate == ((), (), (0, 1), ())
    assert frontier.strict_order_pair_count == 2
    assert frontier.equivalent_pair_count == 0
    assert frontier.incomparable_pair_count == 1
