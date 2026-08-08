from __future__ import annotations

import numpy as np

from learning.tests.semantic_fixtures import semantic_batch_fixture
from sts_learning import (
    BehaviorManifestId,
    CombatDecisionExperienceBatch,
    CombatTerminalOutcome,
    CompletedCombatGroup,
    CompletedCombatGroupExperience,
    SelectionProbability,
    select_semantic_decision_rows,
)


ROOT_ID = "12" * 32
COMBAT_HASH = "ab" * 32


class OneRoundCombatGroup:
    """A real semantic policy round followed by fixed typed outcomes."""

    replicate_count = 2

    def __init__(
        self,
        root_id: str,
        exact_combat_state_hash: str,
        wins: tuple[bool, bool],
    ) -> None:
        self.root_id = root_id
        self.exact_combat_state_hash = exact_combat_state_hash
        self.wins = wins
        self.terminal_count = 0
        self.ready = False
        self.choose_calls = 0

    def decision_batch(self, *, semantic: bool) -> dict[str, object]:
        if not semantic:
            raise AssertionError("combat generation requires semantic rows")
        batch = semantic_batch_fixture()
        batch["slot_indices"] = np.asarray([0, 1], dtype=np.uint64)
        return batch

    def choose(self, ordinals: list[int]) -> None:
        if self.ready or len(ordinals) != self.replicate_count:
            raise AssertionError("combat group received an invalid choice")
        self.choose_calls += 1
        self.ready = True

    def step(self) -> dict[str, object]:
        if not self.ready or self.terminal_count != 0:
            raise AssertionError("combat group cannot step now")
        self.terminal_count = self.replicate_count
        final_hp = np.asarray(
            [70 if won else 0 for won in self.wins],
            dtype=np.int32,
        )
        return {
            "root_id": self.root_id,
            "exact_combat_state_hash": self.exact_combat_state_hash,
            "terminal_slot_indices": np.asarray([0, 1], dtype=np.uint64),
            "terminal_kind": np.asarray(
                [0 if won else 1 for won in self.wins],
                dtype=np.uint8,
            ),
            "terminal_won": np.asarray(self.wins, dtype=np.bool_),
            "terminal_start_hp": np.asarray([80, 80], dtype=np.int32),
            "terminal_final_hp": final_hp,
            "terminal_hp_loss": 80 - final_hp,
            "terminal_turns": np.asarray([3, 5], dtype=np.uint32),
            "terminal_potions_used": np.asarray([0, 1], dtype=np.uint32),
            "terminal_potions_discarded": np.asarray([0, 0], dtype=np.uint32),
            "terminal_cards_played": np.asarray([8, 12], dtype=np.uint32),
        }


class ExactCombatRootSource:
    """Return fresh groups from a caller-declared sequence of exact roots."""

    def __init__(
        self,
        groups: tuple[tuple[str, str, tuple[bool, bool]], ...],
    ) -> None:
        self.specifications = groups
        self.groups: list[OneRoundCombatGroup] = []
        self.call_count = 0

    def combat_group(
        self,
        slot_index: int,
        replicate_count: int,
    ) -> OneRoundCombatGroup:
        if slot_index != 0 or replicate_count != 2:
            raise AssertionError("combat source received different generation bounds")
        try:
            specification = self.specifications[self.call_count]
        except IndexError as error:
            raise AssertionError("combat source was run too many times") from error
        self.call_count += 1
        group = OneRoundCombatGroup(*specification)
        self.groups.append(group)
        return group


def combat_group_experience_fixture(
    manifest_id: BehaviorManifestId,
    *,
    wins: tuple[bool, bool],
    first_probability: SelectionProbability | None = None,
    final_hps: tuple[int, int] | None = None,
    potions_used: tuple[int, int] = (0, 1),
) -> CompletedCombatGroupExperience:
    first = CombatDecisionExperienceBatch(
        sequence_index=0,
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        replicate_indices=(0, 1),
        payload=semantic_batch_fixture(),
        selected_ordinals=(0, 0),
        selection_probabilities=(
            first_probability
            if first_probability is not None
            else SelectionProbability.known(0.5),
            SelectionProbability.known(1.0 / 3.0),
        ),
        behavior_manifest_id=manifest_id,
        decision_count=2,
        payload_bytes=1,
    )
    second = CombatDecisionExperienceBatch(
        sequence_index=1,
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        replicate_indices=(1,),
        payload=select_semantic_decision_rows(semantic_batch_fixture(), [0]),
        selected_ordinals=(0,),
        selection_probabilities=(SelectionProbability.known(0.5),),
        behavior_manifest_id=manifest_id,
        decision_count=1,
        payload_bytes=1,
    )
    if final_hps is None:
        final_hps = (70 if wins[0] else 0, 10 if wins[1] else 0)
    outcomes = CompletedCombatGroup(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        outcomes=(
            _outcome(
                0,
                wins[0],
                final_hp=final_hps[0],
                potions_used=potions_used[0],
            ),
            _outcome(
                1,
                wins[1],
                final_hp=final_hps[1],
                potions_used=potions_used[1],
            ),
        ),
    )
    return CompletedCombatGroupExperience(
        root_id=ROOT_ID,
        exact_combat_state_hash=COMBAT_HASH,
        behavior_manifest_id=manifest_id,
        batches=(first, second),
        outcomes=outcomes,
        decision_count=3,
        payload_bytes=2,
    )


def _outcome(
    replicate_index: int,
    won: bool,
    *,
    final_hp: int,
    potions_used: int,
) -> CombatTerminalOutcome:
    return CombatTerminalOutcome(
        replicate_index=replicate_index,
        terminal_kind=0 if won else 1,
        won=won,
        start_hp=80,
        final_hp=final_hp,
        hp_loss=80 - final_hp,
        turns=3,
        potions_used=potions_used,
        potions_discarded=0,
        cards_played=8,
    )
