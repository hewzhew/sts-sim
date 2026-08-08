from __future__ import annotations

import time

import numpy as np

from sts_learning_bridge import (
    COMBAT_TERMINAL_LOSS,
    COMBAT_TERMINAL_UNRESOLVED,
    COMBAT_TERMINAL_WIN,
    CombatLearningBatchEnv,
    CombatLearningRecoveryRoot,
    CombatLearningRootContextV1,
    LearningBatchEnv,
    LearningCheckpointBatch,
    LearningPublicRunContextV1,
    PHASE_COMBAT_ROOT,
    PHASE_SELECTION,
    PHASE_STRATEGIC_ROOT,
    RUN_BOUNDARY_STRATEGIC,
    RUN_BOUNDARY_TERMINAL,
    SEMANTIC_COMPLETE,
    SEMANTIC_NO_CANDIDATE_TOKEN,
    SEMANTIC_RELATION_CANDIDATE_TARGETS,
    SEMANTIC_RELATION_OBSERVATION_HAS_CANDIDATE,
    SEMANTIC_SCHEMA_VERSION,
    SEMANTIC_TOKEN_CANDIDATE,
    semantic_schema,
)


_MULTIPLIER = 6_364_136_223_846_793_005
_INCREMENT = 1_442_695_040_888_963_407
_MASK_64 = (1 << 64) - 1
_SEED_XOR = 0x9E37_79B9_7F4A_7C15
_SCHEMA = semantic_schema()
_CATEGORY_VOCABULARY_SIZE = np.zeros(
    max(_SCHEMA["categorical_field"].values()) + 1, dtype=np.int64
)
for _field, _size in _SCHEMA["categorical_vocabulary_size"].items():
    _CATEGORY_VOCABULARY_SIZE[int(_field)] = int(_size)


def _pick(states: list[int], slot: int, length: int) -> int:
    assert length > 0
    states[slot] = (states[slot] * _MULTIPLIER + _INCREMENT) & _MASK_64
    return (states[slot] >> 32) % length


def _assert_parallel_table(table: dict, value_dtype: np.dtype) -> None:
    assert table["token_indices"].dtype == np.uint64
    assert table["field"].dtype == np.uint16
    assert table["value"].dtype == value_dtype
    assert table["token_indices"].shape == table["field"].shape
    assert table["field"].shape == table["value"].shape


def _assert_semantic(batch: dict) -> tuple[bool, bool, bool, bool]:
    semantic = batch["semantic"]
    assert semantic["schema_version"] == SEMANTIC_SCHEMA_VERSION
    completeness = semantic["completeness"]
    assert completeness.dtype == np.uint8
    assert completeness.shape == batch["slot_indices"].shape

    token = semantic["token"]
    token_splits = token["row_splits"]
    token_kinds = token["kind"]
    assert token_splits.dtype == np.uint64
    assert token_kinds.dtype == np.uint16
    assert token_splits.shape == (batch["slot_indices"].size + 1,)
    assert token_splits[0] == 0
    assert token_splits[-1] == token_kinds.size
    assert np.all(np.diff(token_splits) >= 0)

    categorical = semantic["categorical"]
    scalar = semantic["scalar"]
    relation = semantic["relation"]
    _assert_parallel_table(categorical, np.dtype(np.int64))
    _assert_parallel_table(scalar, np.dtype(np.float32))
    assert np.all(categorical["value"] >= 0)
    assert np.all(
        categorical["value"]
        < _CATEGORY_VOCABULARY_SIZE[categorical["field"]]
    )
    assert relation["source_token_indices"].dtype == np.uint64
    assert relation["relation"].dtype == np.uint16
    assert relation["target_token_indices"].dtype == np.uint64
    assert relation["source_token_indices"].shape == relation["relation"].shape
    assert relation["relation"].shape == relation["target_token_indices"].shape
    for indices in (
        categorical["token_indices"],
        scalar["token_indices"],
        relation["source_token_indices"],
        relation["target_token_indices"],
    ):
        assert np.all(indices < token_kinds.size)

    candidate_tokens = semantic["candidate_token_indices"]
    candidate_splits = batch["candidate_row_splits"]
    assert candidate_tokens.dtype == np.uint64
    assert candidate_tokens.size == candidate_splits[-1]

    saw_combat = False
    saw_selection = False
    saw_combat_candidate_target = False
    for row, phase in enumerate(batch["phase"]):
        token_start = int(token_splits[row])
        token_end = int(token_splits[row + 1])
        candidate_start = int(candidate_splits[row])
        candidate_end = int(candidate_splits[row + 1])
        row_candidates = candidate_tokens[candidate_start:candidate_end]
        assert completeness[row] == SEMANTIC_COMPLETE
        assert token_start < token_end
        assert np.all(row_candidates != SEMANTIC_NO_CANDIDATE_TOKEN)
        assert np.all(token_kinds[row_candidates] == SEMANTIC_TOKEN_CANDIDATE)
        for candidate in row_candidates:
            assert np.any(
                (relation["relation"] == SEMANTIC_RELATION_OBSERVATION_HAS_CANDIDATE)
                & (relation["target_token_indices"] == candidate)
            )
        if int(phase) in (PHASE_COMBAT_ROOT, PHASE_SELECTION):
            saw_combat = True
            row_kinds = token_kinds[token_start:token_end]
            assert np.any(row_kinds == _SCHEMA["token_kind"]["CombatPlayer"])
            assert np.any(row_kinds == _SCHEMA["token_kind"]["CombatCardZone"])
            saw_combat_candidate_target |= bool(
                np.any(
                    (relation["relation"] == SEMANTIC_RELATION_CANDIDATE_TARGETS)
                    & np.isin(relation["source_token_indices"], row_candidates)
                )
            )
        if int(phase) == PHASE_SELECTION:
            saw_selection = True
            assert np.any(
                token_kinds[token_start:token_end]
                == _SCHEMA["token_kind"]["SelectionState"]
            )

    saw_candidate_target = bool(
        np.any(relation["relation"] == SEMANTIC_RELATION_CANDIDATE_TARGETS)
    )
    return (
        saw_combat,
        saw_selection,
        saw_candidate_target,
        saw_combat_candidate_target,
    )


def _assert_semantic_equal(left: dict, right: dict) -> None:
    assert left["schema_version"] == right["schema_version"]
    assert np.array_equal(left["completeness"], right["completeness"])
    for table_name in ("token", "categorical", "scalar", "relation"):
        assert left[table_name].keys() == right[table_name].keys()
        for column in left[table_name]:
            assert np.array_equal(left[table_name][column], right[table_name][column])
    assert np.array_equal(
        left["candidate_token_indices"], right["candidate_token_indices"]
    )


def _assert_decision_batch_equal(left: dict, right: dict) -> None:
    for field in (
        "slot_indices",
        "phase",
        "candidate_counts",
        "candidate_row_splits",
        "dense_action_mask",
    ):
        assert np.array_equal(left[field], right[field])
    _assert_semantic_equal(left["semantic"], right["semantic"])


def _choose_first_until_ready(env: LearningBatchEnv) -> None:
    rounds = 0
    while not env.ready:
        batch = env.decision_batch()
        env.choose([0] * int(batch["slot_indices"].size))
        rounds += 1
        assert rounds < 100


def _combat_root_context_values(context: CombatLearningRootContextV1) -> tuple[object, ...]:
    return (
        context.act,
        context.floor,
        context.ascension_level,
        context.turn,
        context.is_boss_fight,
        context.is_elite_fight,
        context.monster_count,
        context.living_monster_count,
        context.potion_slot_count,
        context.filled_potion_count,
        context.usable_potion_count,
        context.master_deck_card_count,
        context.relic_count,
        context.hand_card_count,
        context.hp,
        context.max_hp,
    )


def _assert_same_root_combat_group(env: LearningBatchEnv, slot: int) -> None:
    before = env.decision_batch(dense_mask=True, semantic=True)
    available = dict(env.combat_root_contexts())
    assert slot in available
    preview_context = available[slot]
    assert isinstance(preview_context, CombatLearningRootContextV1)
    payload = env.combat_root_artifact_bytes([slot], max_bytes=16 * 1024 * 1024)
    merged_payload = LearningBatchEnv.merge_combat_root_artifact_bytes(
        [payload],
        max_bytes=16 * 1024 * 1024,
    )
    restored = LearningBatchEnv.from_combat_root_artifact_bytes(
        merged_payload,
        expected_roots=1,
        max_bytes=16 * 1024 * 1024,
    )
    restored_context = dict(restored.combat_root_contexts())[0]
    assert _combat_root_context_values(preview_context) == _combat_root_context_values(
        restored_context
    )
    group = env.combat_group(slot, 2)
    assert isinstance(group, CombatLearningBatchEnv)
    assert group.potion_slots is None
    no_potion_group = env.combat_group(slot, 2, ())
    assert no_potion_group.potion_slots == []
    assert no_potion_group.root_id == group.root_id
    root_slot_group = env.combat_group(slot, 2, (0,))
    assert root_slot_group.potion_slots == [0]
    assert group.replicate_count == 2
    assert group.terminal_count == 0
    assert len(group.root_id) == 64
    assert len(group.exact_combat_state_hash) == 64
    context = group.root_context
    assert isinstance(context, CombatLearningRootContextV1)
    assert _combat_root_context_values(preview_context) == _combat_root_context_values(context)
    assert context.act >= 1
    assert group.root_gold >= 0
    assert len(group.root_potion_ids) == context.potion_slot_count
    assert all(potion is None or isinstance(potion, str) for potion in group.root_potion_ids)
    assert context.floor >= 0
    assert context.ascension_level >= 0
    assert context.turn >= 0
    assert isinstance(context.is_boss_fight, bool)
    assert isinstance(context.is_elite_fight, bool)
    assert 0 <= context.living_monster_count <= context.monster_count
    assert 0 <= context.usable_potion_count <= context.filled_potion_count
    assert context.filled_potion_count <= context.potion_slot_count
    assert context.master_deck_card_count > 0
    assert context.relic_count >= 0
    assert context.hand_card_count >= 0
    assert 0 < context.hp <= context.max_hp

    random_states = [slot ^ _SEED_XOR, slot ^ _SEED_XOR ^ 0xA5A5_A5A5]
    terminal_seen: set[int] = set()
    recovery_checked = False
    rounds = 0
    while group.terminal_count < group.replicate_count:
        while not group.ready:
            batch = group.decision_batch(dense_mask=True, semantic=True)
            assert np.all(batch["phase"] != PHASE_STRATEGIC_ROOT)
            _assert_semantic(batch)
            ordinals = [
                _pick(random_states, int(replicate), int(count))
                for replicate, count in zip(
                    batch["slot_indices"], batch["candidate_counts"], strict=True
                )
            ]
            group.choose(ordinals)
        step = group.step()
        assert step["root_id"] == group.root_id
        assert step["exact_combat_state_hash"] == group.exact_combat_state_hash
        terminal_count = int(step["terminal_slot_indices"].size)
        assert step["slot_indices"].dtype == np.uint64
        assert step["terminated"].dtype == np.bool_
        assert step["terminal_slot_indices"].dtype == np.uint64
        assert step["terminal_kind"].dtype == np.uint8
        assert step["terminal_won"].dtype == np.bool_
        assert np.array_equal(
            step["terminal_slot_indices"],
            step["slot_indices"][step["terminated"]],
        )
        assert np.all(
            np.isin(
                step["terminal_kind"],
                [
                    COMBAT_TERMINAL_WIN,
                    COMBAT_TERMINAL_LOSS,
                    COMBAT_TERMINAL_UNRESOLVED,
                ],
            )
        )
        assert np.array_equal(
            step["terminal_won"], step["terminal_kind"] == COMBAT_TERMINAL_WIN
        )
        for key in (
            "terminal_start_hp",
            "terminal_final_hp",
            "terminal_hp_loss",
            "terminal_final_max_hp",
            "terminal_final_gold",
        ):
            assert step[key].dtype == np.int32
            assert step[key].shape == (terminal_count,)
        assert len(step["terminal_potion_ids"]) == terminal_count
        assert all(
            len(potions) == context.potion_slot_count
            for potions in step["terminal_potion_ids"]
        )
        for key in (
            "terminal_turns",
            "terminal_potions_used",
            "terminal_potions_discarded",
            "terminal_cards_played",
        ):
            assert step[key].dtype == np.uint32
            assert step[key].shape == (terminal_count,)
        terminal_seen.update(int(value) for value in step["terminal_slot_indices"])
        if not recovery_checked:
            active = [
                int(replicate)
                for replicate, terminated in zip(
                    step["slot_indices"], step["terminated"], strict=True
                )
                if not bool(terminated)
            ]
            if active:
                recovery = group.capture_recovery_root(active[0])
                assert isinstance(recovery, CombatLearningRecoveryRoot)
                assert recovery.source_root_id == group.root_id
                assert (
                    recovery.source_exact_combat_state_hash
                    == group.exact_combat_state_hash
                )
                assert recovery.source_replicate_index == active[0]
                recovered_group = recovery.spawn_group(2)
                assert recovered_group.root_id == recovery.root_id
                assert (
                    recovered_group.exact_combat_state_hash
                    == recovery.exact_combat_state_hash
                )
                assert recovered_group.replicate_count == 2
                recovery_checked = True
        rounds += 1
        assert rounds < 1_000

    assert terminal_seen == {0, 1}
    assert recovery_checked
    _assert_decision_batch_equal(
        before, env.decision_batch(dense_mask=True, semantic=True)
    )


def _assert_explicit_checkpoint_replays_exactly() -> None:
    env = LearningBatchEnv([37])
    root = env.decision_batch(dense_mask=True, semantic=True)
    root_checkpoint = env.checkpoint_slot(0)
    try:
        env.restore_slot(1, root_checkpoint)
    except ValueError:
        pass
    else:
        raise AssertionError("out-of-range checkpoint restore was accepted")
    _choose_first_until_ready(env)
    ready_checkpoint = env.checkpoint_slot(0)

    first_step = env.step()
    first_after = env.decision_batch(dense_mask=True, semantic=True)
    env.restore_slot(0, ready_checkpoint)
    assert env.ready
    replay_step = env.step()
    replay_after = env.decision_batch(dense_mask=True, semantic=True)
    for field in ("slot_indices", "reward", "terminated"):
        assert np.array_equal(first_step[field], replay_step[field])
    _assert_decision_batch_equal(first_after, replay_after)

    env.restore_slot(0, root_checkpoint)
    restored_root = env.decision_batch(dense_mask=True, semantic=True)
    _assert_decision_batch_equal(root, restored_root)

    batch_env = LearningBatchEnv([41, 42])
    batch_root = batch_env.decision_batch(dense_mask=True, semantic=True)
    checkpoint_batch = batch_env.checkpoint_slots([0, 1])
    assert len(checkpoint_batch) == 2
    assert len(checkpoint_batch.select([1])) == 1
    try:
        checkpoint_batch.select([1, 1])
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate checkpoint selection was accepted")
    try:
        checkpoint_batch.select([2])
    except ValueError:
        pass
    else:
        raise AssertionError("missing checkpoint selection was accepted")
    _choose_first_until_ready(batch_env)
    first_batch_step = batch_env.step()
    first_batch_after = batch_env.decision_batch(dense_mask=True, semantic=True)

    batch_env.restore_slots([0, 1], checkpoint_batch)
    restored_batch_root = batch_env.decision_batch(dense_mask=True, semantic=True)
    _assert_decision_batch_equal(batch_root, restored_batch_root)
    try:
        batch_env.restore_slots([1, 0], checkpoint_batch)
    except ValueError:
        pass
    else:
        raise AssertionError("cross-slot checkpoint restore was accepted")
    _assert_decision_batch_equal(
        restored_batch_root,
        batch_env.decision_batch(dense_mask=True, semantic=True),
    )
    try:
        batch_env.restore_slots([0, 0], checkpoint_batch)
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate-target checkpoint batch was accepted")
    after_rejected_batch = batch_env.decision_batch(dense_mask=True, semantic=True)
    _assert_decision_batch_equal(restored_batch_root, after_rejected_batch)

    _choose_first_until_ready(batch_env)
    replay_batch_step = batch_env.step()
    replay_batch_after = batch_env.decision_batch(dense_mask=True, semantic=True)
    for field in ("slot_indices", "reward", "terminated"):
        assert np.array_equal(first_batch_step[field], replay_batch_step[field])
    _assert_decision_batch_equal(first_batch_after, replay_batch_after)

    replacement = batch_env.checkpoint_slots([0])
    checkpoint_batch = checkpoint_batch.updated(replacement)
    assert len(checkpoint_batch) == 2
    batch_env.restore_slots([0, 1], checkpoint_batch.select([0, 1]))
    restored_after_update = batch_env.decision_batch(dense_mask=True, semantic=True)
    assert np.array_equal(
        restored_after_update["slot_indices"],
        np.array([0, 1], dtype=np.uint64),
    )
    foreign = LearningBatchEnv([44, 45, 46]).checkpoint_slots([2])
    try:
        checkpoint_batch.updated(foreign)
    except ValueError:
        pass
    else:
        raise AssertionError("checkpoint update added a missing slot")


def _assert_cross_process_checkpoint_replays_exactly() -> None:
    max_bytes = 16 * 1024 * 1024
    env = LearningBatchEnv([37])
    root = env.decision_batch(dense_mask=True, semantic=True)
    root_payload = bytes(env.checkpoint_bytes(max_bytes=max_bytes))
    assert root_payload == bytes(env.checkpoint_bytes(max_bytes=max_bytes))
    restored_root_env = LearningBatchEnv.from_checkpoint_bytes(
        root_payload,
        expected_slots=1,
        max_bytes=max_bytes,
    )
    _assert_decision_batch_equal(
        root,
        restored_root_env.decision_batch(dense_mask=True, semantic=True),
    )

    for payload, expected_slots, byte_limit in (
        (root_payload, 2, max_bytes),
        (root_payload, 1, len(root_payload) - 1),
        (bytes([root_payload[0] ^ 0xFF]) + root_payload[1:], 1, max_bytes),
    ):
        try:
            LearningBatchEnv.from_checkpoint_bytes(
                payload,
                expected_slots=expected_slots,
                max_bytes=byte_limit,
            )
        except ValueError:
            pass
        else:
            raise AssertionError("invalid cross-process checkpoint was accepted")

    _choose_first_until_ready(env)
    ready_payload = bytes(env.checkpoint_bytes(max_bytes=max_bytes))
    restored_ready_env = LearningBatchEnv.from_checkpoint_bytes(
        ready_payload,
        expected_slots=1,
        max_bytes=max_bytes,
    )
    assert restored_ready_env.ready
    first_step = env.step()
    replay_step = restored_ready_env.step()
    for field in (
        "slot_indices",
        "reward",
        "terminated",
        "terminal_slot_indices",
        "terminal_reward",
    ):
        assert np.array_equal(first_step[field], replay_step[field])
    _assert_decision_batch_equal(
        env.decision_batch(dense_mask=True, semantic=True),
        restored_ready_env.decision_batch(dense_mask=True, semantic=True),
    )


def _assert_cross_process_checkpoint_bank_replays_episode_roots() -> None:
    max_bytes = 32 * 1024 * 1024
    source_env = LearningBatchEnv([37, 38])
    source_root = source_env.decision_batch(dense_mask=True, semantic=True)
    bank = source_env.checkpoint_slots([0, 1])
    payload = bytes(bank.checkpoint_bytes(max_bytes=max_bytes))
    assert payload == bytes(bank.checkpoint_bytes(max_bytes=max_bytes))

    restored_bank = LearningCheckpointBatch.from_checkpoint_bytes(
        payload,
        expected_slot_indices=[0, 1],
        max_bytes=max_bytes,
    )
    assert len(restored_bank) == 2
    target_env = LearningBatchEnv([99, 100])
    target_env.restore_slots([0, 1], restored_bank)
    _assert_decision_batch_equal(
        source_root,
        target_env.decision_batch(dense_mask=True, semantic=True),
    )

    for expected_slot_indices in ([1, 0], [0], [0, 0]):
        try:
            LearningCheckpointBatch.from_checkpoint_bytes(
                payload,
                expected_slot_indices=expected_slot_indices,
                max_bytes=max_bytes,
            )
        except ValueError:
            pass
        else:
            raise AssertionError("checkpoint bank accepted mismatched slot identity")


def main() -> None:
    assert callable(LearningBatchEnv.from_combat_root_artifact_bytes)
    assert callable(LearningBatchEnv.merge_combat_root_artifact_bytes)
    assert callable(LearningBatchEnv.supported_potion_ids)
    assert callable(LearningBatchEnv.combat_root_artifact_bytes)
    potion_ids = LearningBatchEnv.supported_potion_ids()
    assert len(potion_ids) == len(set(potion_ids))
    assert "FirePotion" in potion_ids
    schema = _SCHEMA
    assert schema["version"] == SEMANTIC_SCHEMA_VERSION
    assert schema["completeness"]["Complete"] == SEMANTIC_COMPLETE
    assert schema["token_kind"]["Candidate"] == SEMANTIC_TOKEN_CANDIDATE
    assert len(schema["action_kind"]) == 28
    assert len(schema["combat_action_kind"]) == 8
    assert set(schema["categorical_field"].values()) == set(
        schema["categorical_vocabulary_size"]
    )
    for field_name, enum_name in (
        ("ContextKind", "context_kind"),
        ("ActionKind", "action_kind"),
        ("RewardKind", "reward_kind"),
        ("CombatActionKind", "combat_action_kind"),
        ("IntentKind", "intent_kind"),
        ("EnemyIdentityKind", "enemy_identity_kind"),
        ("PublicCounterKind", "public_counter_kind"),
        ("CardZoneKind", "card_zone_kind"),
        ("IndexedChoiceReasonKind", "indexed_choice_reason_kind"),
        ("IndexedChoiceCandidateKind", "indexed_choice_candidate_kind"),
        ("SelectionReasonKind", "selection_reason_kind"),
        ("SelectionCandidateKind", "selection_candidate_kind"),
        ("SelectionDomainKind", "selection_domain_kind"),
        ("CounterItemKind", "counter_item_kind"),
    ):
        field = schema["categorical_field"][field_name]
        assert max(schema[enum_name].values()) < schema[
            "categorical_vocabulary_size"
        ][field]
    assert (
        schema["categorical_vocabulary_size"][
            schema["categorical_field"]["ActionCardId"]
        ]
        == schema["domain_vocabulary_size"]["card_id"]
        == 371
    )
    assert schema["domain_vocabulary_size"]["enemy_id"] == 65
    assert schema["domain_vocabulary_size"]["power_id"] == 135
    _assert_explicit_checkpoint_replays_exactly()
    _assert_cross_process_checkpoint_replays_exactly()
    _assert_cross_process_checkpoint_bank_replays_episode_roots()

    seeds = list(range(1, 6))
    random_states = [seed ^ _SEED_XOR for seed in seeds]
    env = LearningBatchEnv(seeds)
    total_steps = 0
    terminal_slots_seen: set[int] = set()
    started = time.perf_counter()

    initial_contexts = env.public_run_contexts()
    assert [slot for slot, _ in initial_contexts] == list(range(env.slot_count))
    assert all(
        isinstance(context, LearningPublicRunContextV1)
        for _, context in initial_contexts
    )
    assert all(
        context.boundary_kind == RUN_BOUNDARY_STRATEGIC
        for _, context in initial_contexts
    )
    assert all(
        context.strategic_context_kind == semantic_schema()["context_kind"]["Event"]
        for _, context in initial_contexts
    )
    assert not any(
        context.is_combat or context.is_terminal
        for _, context in initial_contexts
    )
    assert all(context.monster_ids == [] for _, context in initial_contexts)
    assert all(
        context.act == 1 and context.floor == 0
        for _, context in initial_contexts
    )
    assert [context.seed for _, context in initial_contexts] == seeds
    assert all(context.hp == context.max_hp for _, context in initial_contexts)
    assert all(context.gold >= 0 for _, context in initial_contexts)
    assert all(isinstance(context.potion_ids, list) for _, context in initial_contexts)

    initial = env.decision_batch(dense_mask=True, semantic=True)
    initial_combat, initial_selection, _, initial_combat_target = _assert_semantic(
        initial
    )
    assert not initial_combat
    assert not initial_selection
    assert not initial_combat_target
    invalid = [0] * int(initial["slot_indices"].size)
    invalid[0] = int(initial["candidate_counts"][0])
    try:
        env.choose(invalid)
    except ValueError:
        pass
    else:
        raise AssertionError("out-of-range candidate ordinal was accepted")
    after_rejection = env.decision_batch(dense_mask=True, semantic=True)
    for field in (
        "slot_indices",
        "phase",
        "candidate_counts",
        "candidate_row_splits",
        "dense_action_mask",
    ):
        assert np.array_equal(initial[field], after_rejection[field])
    _assert_semantic_equal(initial["semantic"], after_rejection["semantic"])

    saw_combat = False
    saw_selection = False
    saw_candidate_target = False
    saw_combat_candidate_target = False
    checked_combat_group = False

    while env.terminal_count < env.slot_count:
        while not env.ready:
            batch = env.decision_batch(dense_mask=True, semantic=True)
            slots = batch["slot_indices"]
            counts = batch["candidate_counts"]
            row_splits = batch["candidate_row_splits"]
            mask = batch["dense_action_mask"]

            assert slots.dtype == np.uint64
            assert counts.dtype == np.uint64
            assert row_splits.dtype == np.uint64
            assert mask.dtype == np.bool_
            assert row_splits.shape == (slots.size + 1,)
            assert np.array_equal(np.diff(row_splits), counts)
            assert np.array_equal(mask.sum(axis=1), counts)
            (
                batch_combat,
                batch_selection,
                batch_candidate_target,
                batch_combat_candidate_target,
            ) = _assert_semantic(batch)
            saw_combat |= batch_combat
            saw_selection |= batch_selection
            saw_candidate_target |= batch_candidate_target
            saw_combat_candidate_target |= batch_combat_candidate_target

            if not checked_combat_group:
                combat_rows = np.flatnonzero(batch["phase"] == PHASE_COMBAT_ROOT)
                if combat_rows.size:
                    _assert_same_root_combat_group(env, int(slots[int(combat_rows[0])]))
                    checked_combat_group = True

            ordinals = [
                _pick(random_states, int(slot), int(count))
                for slot, count in zip(slots, counts, strict=True)
            ]
            env.choose(ordinals)

        step = env.step()
        total_steps += int(step["slot_indices"].size)
        terminal_count = int(step["terminal_slot_indices"].size)
        assert step["terminal_slot_indices"].dtype == np.uint64
        assert np.array_equal(
            step["terminal_slot_indices"],
            step["slot_indices"][step["terminated"]],
        )
        assert step["terminal_reward"].dtype == np.int8
        assert np.array_equal(
            step["terminal_reward"],
            step["reward"][step["terminated"]],
        )
        assert np.all(step["terminal_reward"] != 0)
        assert step["terminal_act"].dtype == np.uint8
        for key in (
            "terminal_floor",
            "terminal_hp",
            "terminal_max_hp",
            "terminal_gold",
        ):
            assert step[key].dtype == np.int32
            assert step[key].shape == (terminal_count,)
        assert step["terminal_act"].shape == (terminal_count,)
        if terminal_count:
            assert np.all(step["terminal_act"] >= 1)
            assert np.all(step["terminal_floor"] >= 0)
            assert np.all(step["terminal_max_hp"] > 0)
            assert np.all(step["terminal_hp"] <= step["terminal_max_hp"])
            assert np.all(step["terminal_gold"] >= 0)
        terminal_slots_seen.update(int(slot) for slot in step["terminal_slot_indices"])
        assert total_steps < 100_000

    elapsed = time.perf_counter() - started
    # The exact seeded trajectory length is policy-dependent. Completion,
    # boundedness, terminal alignment, and semantic coverage above are the
    # bridge contracts; a new candidate prior must not require a new magic
    # step-count golden.
    assert terminal_slots_seen == set(range(env.slot_count))
    terminal_contexts = env.public_run_contexts()
    assert all(
        context.boundary_kind == RUN_BOUNDARY_TERMINAL
        for _, context in terminal_contexts
    )
    assert all(
        context.is_terminal and not context.is_combat
        for _, context in terminal_contexts
    )
    assert saw_combat
    assert saw_candidate_target
    assert saw_combat_candidate_target
    assert checked_combat_group
    print(
        "python_learning_bridge_smoke "
        f"episodes={env.slot_count} steps={total_steps} "
        f"elapsed_ms={elapsed * 1000:.1f} "
        f"steps_per_second={total_steps / elapsed:.0f}"
    )
    reset_checkpoints = env.reset_slots_checkpointed([0, 1], [99, 100])
    assert len(reset_checkpoints) == 2
    assert env.terminal_count == env.slot_count - 2
    reset_batch = env.decision_batch(dense_mask=True, semantic=True)
    assert np.array_equal(reset_batch["slot_indices"], np.array([0, 1], dtype=np.uint64))
    try:
        env.reset_slots([2, 2], [101, 102])
    except ValueError:
        pass
    else:
        raise AssertionError("duplicate-target reset batch was accepted")
    _assert_decision_batch_equal(
        reset_batch, env.decision_batch(dense_mask=True, semantic=True)
    )
    try:
        env.reset_slot(0, 103)
    except ValueError:
        pass
    else:
        raise AssertionError("active slot reset was accepted")
    _choose_first_until_ready(env)
    env.step()
    env.restore_slots([0, 1], reset_checkpoints)
    _assert_decision_batch_equal(
        reset_batch,
        env.decision_batch(dense_mask=True, semantic=True),
    )


if __name__ == "__main__":
    main()
