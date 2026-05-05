#!/usr/bin/env python3
from __future__ import annotations

import json
import random
import re
from pathlib import Path
from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium import spaces

from combat_rl_common import REPO_ROOT, curriculum_tag_from_spec_name
from gym_combat_env import CombatEnvDriver, discover_spec_paths

MAX_HAND = 10
MAX_MONSTERS = 5
MAX_POTIONS = 5
MAX_PLAYER_POWERS = 16
MAX_MONSTER_POWERS = 8
MAX_CHOICE_OPTIONS = 32
MAX_TARGET_OPTIONS = MAX_MONSTERS + 1  # 0 = no target, 1..N = monster slot + 1

ACTION_TYPE_END_TURN = 0
ACTION_TYPE_PLAY_CARD = 1
ACTION_TYPE_USE_POTION = 2
ACTION_TYPE_CHOICE = 3
ACTION_TYPE_PROCEED = 4
ACTION_TYPE_CANCEL = 5
ACTION_TYPE_COUNT = 6

CHOICE_FAMILIES = {
    "discovery_select",
    "card_reward_select",
    "stance_choice",
    "scry_select",
    "card_select",
    "hand_select",
    "grid_select",
}
PENDING_KIND_IDS = {
    None: 0,
    "discovery_select": 1,
    "card_reward_select": 2,
    "stance_choice": 3,
    "scry_select": 4,
    "hand_select": 5,
    "grid_select": 6,
}
PHASE_IDS = {
    "PlayerTurn": 1,
    "MonsterTurn": 2,
    "TurnTransition": 3,
}
TARGET_MODE_IDS = {
    "none": 0,
    "self": 1,
    "single_enemy": 2,
    "all_enemy": 3,
}
OPTION_SOURCE_IDS = {
    None: 0,
    "Hand": 1,
    "Draw": 2,
    "Discard": 3,
    "Exhaust": 4,
    "Limbo": 5,
    "MasterDeck": 6,
}
CHOICE_KIND_IDS = {
    None: 0,
    "discovery_select": 1,
    "card_reward_select": 2,
    "stance_choice": 3,
    "scry_select": 4,
    "hand_select": 5,
    "grid_select": 6,
    "card_select": 7,
}
INTENT_KIND_IDS = {
    "unknown": 0,
    "attack": 1,
    "attack_add_card": 2,
    "attack_upgrade_cards": 3,
    "attack_buff": 4,
    "attack_sustain": 5,
    "attack_debuff": 6,
    "attack_defend": 7,
    "add_card": 8,
    "buff": 9,
    "debuff": 10,
    "strong_debuff": 11,
    "defend": 12,
    "defend_debuff": 13,
    "defend_buff": 14,
    "heal": 15,
    "escape": 16,
    "magic": 17,
    "sleep": 18,
    "stun": 19,
    "debug": 20,
    "none": 21,
}
CARD_TYPE_IDS = {
    "unknown": 0,
    "attack": 1,
    "skill": 2,
    "power": 3,
    "status": 4,
    "curse": 5,
}


def _parse_rust_enum_variants(path: Path, enum_name: str) -> list[str]:
    payload = path.read_text(encoding="utf-8")
    match = re.search(rf"pub enum {re.escape(enum_name)} \{{(?P<body>.*?)\n\}}", payload, re.S)
    if not match:
        return []
    variants: list[str] = []
    for raw_line in match.group("body").splitlines():
        line = raw_line.split("//", 1)[0].strip().rstrip(",")
        if not line:
            continue
        if "{" in line or "(" in line or "=" in line:
            line = line.split("{", 1)[0].split("(", 1)[0].split("=", 1)[0].strip()
        if line:
            variants.append(line)
    return variants


def _build_vocab(enum_path: Path, enum_name: str) -> dict[str, int]:
    vocab = {"<unk>": 0}
    for index, variant in enumerate(_parse_rust_enum_variants(enum_path, enum_name), start=1):
        vocab[variant] = index
    return vocab


CARD_ID_VOCAB = _build_vocab(REPO_ROOT / "src" / "content" / "cards" / "mod.rs", "CardId")
POTION_ID_VOCAB = _build_vocab(REPO_ROOT / "src" / "content" / "potions" / "mod.rs", "PotionId")
POWER_ID_VOCAB = _build_vocab(REPO_ROOT / "src" / "content" / "powers" / "mod.rs", "PowerId")
MONSTER_ID_VOCAB = _build_vocab(REPO_ROOT / "src" / "content" / "monsters" / "mod.rs", "EnemyId")


def _id_lookup(vocab: dict[str, int], raw: str | None) -> int:
    return int(vocab.get(str(raw or ""), 0))


def _bool(value: Any) -> float:
    return 1.0 if value else 0.0


def _int_or(value: Any, default: int) -> int:
    if value is None:
        return int(default)
    return int(value)


def _power_numeric(power: dict[str, Any]) -> list[float]:
    return [
        float(power.get("amount") or 0.0),
        float(power.get("extra_data") or 0.0),
        _bool(power.get("just_applied")),
        _bool(power.get("is_debuff")),
    ]


def _pool_power_summary(powers: list[dict[str, Any]]) -> list[float]:
    if not powers:
        return [0.0] * 6
    total_positive = 0.0
    total_negative = 0.0
    debuff_count = 0.0
    artifact_amount = 0.0
    vulnerable_amount = 0.0
    weak_amount = 0.0
    for power in powers:
        amount = float(power.get("amount") or 0.0)
        if amount > 0:
            total_positive += amount
        if amount < 0:
            total_negative += abs(amount)
        if power.get("is_debuff"):
            debuff_count += 1.0
        power_id = str(power.get("id") or "")
        if power_id == "Artifact":
            artifact_amount = amount
        if power_id == "Vulnerable":
            vulnerable_amount = amount
        if power_id == "Weak":
            weak_amount = amount
    return [
        total_positive,
        total_negative,
        debuff_count,
        artifact_amount,
        vulnerable_amount,
        weak_amount,
    ]


def _choice_candidates(payload: dict[str, Any]) -> list[dict[str, Any]]:
    return [
        candidate
        for candidate, legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or [])
        if legal and str(candidate.get("action_family") or "") in CHOICE_FAMILIES
    ]


def _target_index_from_candidate(candidate: dict[str, Any]) -> int:
    target_slot = candidate.get("target_slot")
    if target_slot is None:
        return 0
    return int(target_slot) + 1


def _normalized_selection(values: Any) -> tuple[int, ...] | None:
    if values is None:
        return None
    return tuple(sorted(int(value) for value in values))


def _normalized_uuid_selection(values: Any) -> tuple[str, ...] | None:
    if values is None:
        return None
    return tuple(sorted(str(value) for value in values))


def _derive_probe_targets(obs: dict[str, Any]) -> np.ndarray:
    pressure = obs.get("pressure") or {}
    player_hp = float(obs.get("player_hp") or 0.0)
    visible_unblocked = float(pressure.get("visible_unblocked") or 0.0)
    monsters = list(obs.get("monsters") or [])
    hand = list(obs.get("hand") or [])
    urgent_survival = 1.0 if visible_unblocked >= max(8.0, player_hp * 0.25) else 0.0
    lethal_exists = 1.0 if any(int(monster.get("current_hp") or 0) <= 8 for monster in monsters) else 0.0
    setup_window = 1.0 if any(str(card.get("card_type") or "") == "power" and card.get("playable") for card in hand) and urgent_survival == 0.0 else 0.0
    if len(monsters) > 1:
        hp_values = [int(monster.get("current_hp") or 0) for monster in monsters]
        aoe_balance_pressure = 1.0 if max(hp_values) - min(hp_values) <= 12 else 0.0
    else:
        aoe_balance_pressure = 0.0
    return np.asarray(
        [urgent_survival, lethal_exists, setup_window, aoe_balance_pressure],
        dtype=np.float32,
    )


class StructuredGymCombatEnv(gym.Env[dict[str, np.ndarray], dict[str, int]]):
    metadata = {"render_modes": []}

    def __init__(
        self,
        spec_paths: list[Path],
        spec_source: str = "author_spec",
        driver_binary: Path | None = None,
        invalid_action_penalty: float = -2.0,
        max_episode_steps: int = 64,
        seed: int = 0,
        seed_pool: list[int] | None = None,
        draw_order_variant: str = "exact",
        reward_mode: str = "legacy",
        reward_config: dict[str, float] | None = None,
    ) -> None:
        super().__init__()
        if not spec_paths:
            raise ValueError("StructuredGymCombatEnv requires at least one spec path")
        if spec_source not in {"author_spec", "start_spec"}:
            raise ValueError(f"unsupported spec_source '{spec_source}'")
        if draw_order_variant not in {"exact", "reshuffle_draw"}:
            raise ValueError(f"unsupported draw_order_variant '{draw_order_variant}'")
        self.spec_paths = [Path(path) for path in spec_paths]
        self.spec_source = spec_source
        self.driver = CombatEnvDriver(driver_binary)
        self.invalid_action_penalty = float(invalid_action_penalty)
        self.max_episode_steps = int(max_episode_steps)
        self._rng = random.Random(seed)
        self.seed_pool = [int(value) for value in (seed_pool or [])]
        self.draw_order_variant = draw_order_variant
        self.reward_mode = str(reward_mode or "legacy")
        self.reward_config = {
            "victory_reward": 1.0,
            "defeat_reward": -1.0,
            "hp_loss_scale": 0.02,
            "enemy_hp_delta_scale": 0.0,
            "kill_bonus_scale": 0.0,
            "catastrophe_unblocked_threshold": 18.0,
            "catastrophe_penalty": 0.25,
            "next_enemy_window_relief_scale": 0.0,
            "persistent_attack_script_relief_scale": 0.0,
        }
        if reward_config:
            self.reward_config.update({key: float(value) for key, value in reward_config.items()})
        self._last_response: dict[str, Any] | None = None
        self._current_spec: Path | None = None
        self._step_count = 0

        self.observation_space = spaces.Dict(
            {
                "global_numeric": spaces.Box(low=-1e6, high=1e6, shape=(20,), dtype=np.float32),
                "player_power_ids": spaces.Box(low=0, high=max(len(POWER_ID_VOCAB), 1), shape=(MAX_PLAYER_POWERS,), dtype=np.int64),
                "player_power_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_PLAYER_POWERS, 4), dtype=np.float32),
                "player_power_mask": spaces.Box(low=0, high=1, shape=(MAX_PLAYER_POWERS,), dtype=np.float32),
                "hand_ids": spaces.Box(low=0, high=max(len(CARD_ID_VOCAB), 1), shape=(MAX_HAND,), dtype=np.int64),
                "hand_type_ids": spaces.Box(low=0, high=8, shape=(MAX_HAND,), dtype=np.int64),
                "hand_target_ids": spaces.Box(low=0, high=8, shape=(MAX_HAND,), dtype=np.int64),
                "hand_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_HAND, 7), dtype=np.float32),
                "hand_mask": spaces.Box(low=0, high=1, shape=(MAX_HAND,), dtype=np.float32),
                "potion_ids": spaces.Box(low=0, high=max(len(POTION_ID_VOCAB), 1), shape=(MAX_POTIONS,), dtype=np.int64),
                "potion_target_ids": spaces.Box(low=0, high=8, shape=(MAX_POTIONS,), dtype=np.int64),
                "potion_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_POTIONS, 2), dtype=np.float32),
                "potion_mask": spaces.Box(low=0, high=1, shape=(MAX_POTIONS,), dtype=np.float32),
                "monster_ids": spaces.Box(low=0, high=max(len(MONSTER_ID_VOCAB), 1), shape=(MAX_MONSTERS,), dtype=np.int64),
                "monster_intent_ids": spaces.Box(low=0, high=64, shape=(MAX_MONSTERS,), dtype=np.int64),
                "monster_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_MONSTERS, 20), dtype=np.float32),
                "monster_mask": spaces.Box(low=0, high=1, shape=(MAX_MONSTERS,), dtype=np.float32),
                "monster_power_ids": spaces.Box(low=0, high=max(len(POWER_ID_VOCAB), 1), shape=(MAX_MONSTERS, MAX_MONSTER_POWERS), dtype=np.int64),
                "monster_power_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_MONSTERS, MAX_MONSTER_POWERS, 4), dtype=np.float32),
                "monster_power_mask": spaces.Box(low=0, high=1, shape=(MAX_MONSTERS, MAX_MONSTER_POWERS), dtype=np.float32),
                "choice_kind_id": spaces.Box(low=0, high=16, shape=(1,), dtype=np.int64),
                "choice_card_ids": spaces.Box(low=0, high=max(len(CARD_ID_VOCAB), 1), shape=(MAX_CHOICE_OPTIONS,), dtype=np.int64),
                "choice_source_ids": spaces.Box(low=0, high=16, shape=(MAX_CHOICE_OPTIONS,), dtype=np.int64),
                "choice_numeric": spaces.Box(low=-1e6, high=1e6, shape=(MAX_CHOICE_OPTIONS, 4), dtype=np.float32),
                "choice_mask": spaces.Box(low=0, high=1, shape=(MAX_CHOICE_OPTIONS,), dtype=np.float32),
                "action_type_mask": spaces.Box(low=0, high=1, shape=(ACTION_TYPE_COUNT,), dtype=np.float32),
                "play_card_mask": spaces.Box(low=0, high=1, shape=(MAX_HAND,), dtype=np.float32),
                "play_card_target_mask": spaces.Box(low=0, high=1, shape=(MAX_HAND, MAX_TARGET_OPTIONS), dtype=np.float32),
                "use_potion_mask": spaces.Box(low=0, high=1, shape=(MAX_POTIONS,), dtype=np.float32),
                "use_potion_target_mask": spaces.Box(low=0, high=1, shape=(MAX_POTIONS, MAX_TARGET_OPTIONS), dtype=np.float32),
                "choice_option_mask": spaces.Box(low=0, high=1, shape=(MAX_CHOICE_OPTIONS,), dtype=np.float32),
            }
        )
        self.action_space = spaces.Dict(
            {
                "action_type": spaces.Discrete(ACTION_TYPE_COUNT),
                "card_slot": spaces.Discrete(MAX_HAND),
                "target_slot": spaces.Discrete(MAX_TARGET_OPTIONS),
                "potion_slot": spaces.Discrete(MAX_POTIONS),
                "choice_index": spaces.Discrete(MAX_CHOICE_OPTIONS),
            }
        )

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[dict[str, np.ndarray], dict[str, Any]]:
        super().reset(seed=seed)
        if seed is not None:
            self._rng.seed(seed)
        options = options or {}
        chosen_spec = Path(options.get("spec_path")) if options.get("spec_path") else self._rng.choice(self.spec_paths)
        self._current_spec = chosen_spec
        if options.get("seed_hint") is not None:
            seed_hint = int(options.get("seed_hint"))
        elif self.seed_pool:
            seed_hint = int(self._rng.choice(self.seed_pool))
        else:
            seed_hint = int(self._rng.randrange(1, 2**31))
        response = self.driver.request(
            {
                "cmd": "reset",
                self.spec_source: str(chosen_spec),
                "seed_hint": seed_hint,
                "draw_order_variant": self.draw_order_variant,
            }
        )
        self._last_response = response
        self._step_count = 0
        info = self._info_from_response(response, invalid_action=False, decoder_failure=False)
        return self._encode_observation(response), info

    def step(
        self,
        action: dict[str, int],
    ) -> tuple[dict[str, np.ndarray], float, bool, bool, dict[str, Any]]:
        if self._last_response is None:
            raise RuntimeError("reset must be called before step")
        action_index, invalid_action, decoder_failure = self._decode_action_index(action)
        invalid_penalty = self.invalid_action_penalty if invalid_action else 0.0
        if invalid_action:
            response = self._last_response
        else:
            response = self.driver.request({"cmd": "step", "action_index": action_index})
            self._last_response = response
        self._step_count += 1
        terminated = bool(response.get("done"))
        truncated = bool(not terminated and self._step_count >= self.max_episode_steps)
        info = self._info_from_response(response, invalid_action=invalid_action, decoder_failure=decoder_failure)
        if self.reward_mode == "legacy":
            reward = float(response.get("reward") or 0.0) + float(invalid_penalty)
            info["effective_reward_terms"] = {
                "reward_mode": "legacy",
                "legacy_reward_term": float(response.get("reward") or 0.0),
                "invalid_action_penalty": float(invalid_penalty),
                "total_effective_reward": float(reward),
            }
        else:
            reward_terms = self._effective_reward_terms(info, invalid_penalty)
            reward = float(reward_terms["total_effective_reward"])
            info["effective_reward_terms"] = reward_terms
        return self._encode_observation(response), reward, terminated, truncated, info

    def close(self) -> None:
        self.driver.close()

    def sample_random_legal_action(self) -> dict[str, int]:
        if self._last_response is None:
            raise RuntimeError("reset must be called before sampling")
        payload = self._last_response.get("payload") or {}
        candidates = [candidate for candidate, legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or []) if legal]
        if not candidates:
            return {"action_type": ACTION_TYPE_END_TURN, "card_slot": 0, "target_slot": 0, "potion_slot": 0, "choice_index": 0}
        choice_candidates = _choice_candidates(payload)
        candidate = self._rng.choice(candidates)
        family = str(candidate.get("action_family") or "")
        action = {
            "action_type": ACTION_TYPE_END_TURN,
            "card_slot": 0,
            "target_slot": 0,
            "potion_slot": 0,
            "choice_index": 0,
        }
        if family == "play_card":
            action["action_type"] = ACTION_TYPE_PLAY_CARD
            action["card_slot"] = _int_or(candidate.get("slot_index"), 0)
            action["target_slot"] = _target_index_from_candidate(candidate)
        elif family == "use_potion":
            action["action_type"] = ACTION_TYPE_USE_POTION
            action["potion_slot"] = _int_or(candidate.get("slot_index"), 0)
            action["target_slot"] = _target_index_from_candidate(candidate)
        elif family in CHOICE_FAMILIES:
            action["action_type"] = ACTION_TYPE_CHOICE
            action["choice_index"] = max(0, choice_candidates.index(candidate)) if candidate in choice_candidates else 0
        elif family == "proceed":
            action["action_type"] = ACTION_TYPE_PROCEED
        elif family == "cancel":
            action["action_type"] = ACTION_TYPE_CANCEL
        return action

    def candidate_to_canonical(self, candidate: dict[str, Any]) -> dict[str, int]:
        family = str(candidate.get("action_family") or "")
        action = {
            "action_type": ACTION_TYPE_END_TURN,
            "card_slot": 0,
            "target_slot": 0,
            "potion_slot": 0,
            "choice_index": 0,
        }
        if family == "play_card":
            action["action_type"] = ACTION_TYPE_PLAY_CARD
            action["card_slot"] = _int_or(candidate.get("slot_index"), 0)
            action["target_slot"] = _target_index_from_candidate(candidate)
        elif family == "use_potion":
            action["action_type"] = ACTION_TYPE_USE_POTION
            action["potion_slot"] = _int_or(candidate.get("slot_index"), 0)
            action["target_slot"] = _target_index_from_candidate(candidate)
        elif family in CHOICE_FAMILIES:
            action["action_type"] = ACTION_TYPE_CHOICE
            action["choice_kind"] = candidate.get("choice_kind")
            action["selection_indices"] = [int(value) for value in list(candidate.get("selection_indices") or [])]
            action["selection_uuids"] = [str(value) for value in list(candidate.get("selection_uuids") or [])]
            payload = (self._last_response or {}).get("payload") or {}
            choice_candidates = _choice_candidates(payload)
            if candidate in choice_candidates:
                action["choice_index"] = choice_candidates.index(candidate)
        elif family == "proceed":
            action["action_type"] = ACTION_TYPE_PROCEED
        elif family == "cancel":
            action["action_type"] = ACTION_TYPE_CANCEL
        return action

    def decode_canonical_action(self, action: dict[str, int]) -> tuple[int, bool, bool]:
        return self._decode_action_index(action)

    def _decode_action_index(self, action: dict[str, int]) -> tuple[int, bool, bool]:
        if self._last_response is None:
            return -1, True, False
        payload = self._last_response.get("payload") or {}
        candidates = list(payload.get("action_candidates") or [])
        legal = list(payload.get("action_mask") or [])
        action_type = _int_or(action.get("action_type"), 0)
        if action_type == ACTION_TYPE_END_TURN:
            matches = [candidate for candidate, is_legal in zip(candidates, legal) if is_legal and str(candidate.get("action_family") or "") == "end_turn"]
        elif action_type == ACTION_TYPE_PLAY_CARD:
            slot = _int_or(action.get("card_slot"), 0)
            target_slot = _int_or(action.get("target_slot"), 0)
            matches = [
                candidate
                for candidate, is_legal in zip(candidates, legal)
                if is_legal
                and str(candidate.get("action_family") or "") == "play_card"
                and _int_or(candidate.get("slot_index"), -1) == slot
                and _target_index_from_candidate(candidate) == target_slot
            ]
        elif action_type == ACTION_TYPE_USE_POTION:
            slot = _int_or(action.get("potion_slot"), 0)
            target_slot = _int_or(action.get("target_slot"), 0)
            matches = [
                candidate
                for candidate, is_legal in zip(candidates, legal)
                if is_legal
                and str(candidate.get("action_family") or "") == "use_potion"
                and _int_or(candidate.get("slot_index"), -1) == slot
                and _target_index_from_candidate(candidate) == target_slot
            ]
        elif action_type == ACTION_TYPE_CHOICE:
            choice_candidates = _choice_candidates(payload)
            selection_indices = _normalized_selection(action.get("selection_indices"))
            selection_uuids = _normalized_uuid_selection(action.get("selection_uuids"))
            choice_kind = action.get("choice_kind")
            if selection_indices is not None or selection_uuids is not None:
                matches = [
                    candidate
                    for candidate in choice_candidates
                    if (choice_kind is None or str(candidate.get("choice_kind") or "") == str(choice_kind))
                    and (
                        selection_indices is None
                        or _normalized_selection(candidate.get("selection_indices")) == selection_indices
                    )
                    and (
                        selection_uuids is None
                        or _normalized_uuid_selection(candidate.get("selection_uuids")) == selection_uuids
                    )
                ]
            else:
                choice_index = _int_or(action.get("choice_index"), 0)
                if choice_index < 0 or choice_index >= len(choice_candidates):
                    return -1, True, False
                matches = [choice_candidates[choice_index]]
        elif action_type == ACTION_TYPE_PROCEED:
            matches = [candidate for candidate, is_legal in zip(candidates, legal) if is_legal and str(candidate.get("action_family") or "") == "proceed"]
        elif action_type == ACTION_TYPE_CANCEL:
            matches = [candidate for candidate, is_legal in zip(candidates, legal) if is_legal and str(candidate.get("action_family") or "") == "cancel"]
        else:
            return -1, True, False
        if not matches:
            return -1, True, False
        if len(matches) > 1:
            return -1, False, True
        return int(matches[0].get("index")), False, False

    def _encode_observation(self, response: dict[str, Any]) -> dict[str, np.ndarray]:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        pressure = obs.get("pressure") or {}
        pending_choice = obs.get("pending_choice") or {}
        global_numeric = np.asarray(
            [
                float(obs.get("player_hp") or 0.0),
                float(obs.get("player_hp") or 0.0) / max(float(obs.get("player_max_hp") or 1.0), 1.0),
                float(obs.get("player_block") or 0.0),
                float(obs.get("energy") or 0.0),
                float(obs.get("turn_count") or 0.0),
                float(obs.get("draw_count") or 0.0),
                float(obs.get("discard_count") or 0.0),
                float(obs.get("exhaust_count") or 0.0),
                float(len(obs.get("hand") or [])),
                float(len(obs.get("monsters") or [])),
                float((obs.get("turn_prefix") or {}).get("cards_played_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("attacks_played_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("skills_played_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("powers_played_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("energy_spent_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("damage_dealt_this_turn") or 0.0),
                float((obs.get("turn_prefix") or {}).get("damage_taken_this_turn") or 0.0),
                float(PENDING_KIND_IDS.get(obs.get("pending_choice_kind"), 0)),
                float(PHASE_IDS.get(str(obs.get("phase") or ""), 0)),
                _bool("PendingChoice(" in str(obs.get("engine_state") or "")),
            ],
            dtype=np.float32,
        )

        player_power_ids = np.zeros((MAX_PLAYER_POWERS,), dtype=np.int64)
        player_power_numeric = np.zeros((MAX_PLAYER_POWERS, 4), dtype=np.float32)
        player_power_mask = np.zeros((MAX_PLAYER_POWERS,), dtype=np.float32)
        for index, power in enumerate(list(obs.get("player_powers") or [])[:MAX_PLAYER_POWERS]):
            player_power_ids[index] = _id_lookup(POWER_ID_VOCAB, power.get("id"))
            player_power_numeric[index] = np.asarray(_power_numeric(power), dtype=np.float32)
            player_power_mask[index] = 1.0

        hand_ids = np.zeros((MAX_HAND,), dtype=np.int64)
        hand_type_ids = np.zeros((MAX_HAND,), dtype=np.int64)
        hand_target_ids = np.zeros((MAX_HAND,), dtype=np.int64)
        hand_numeric = np.zeros((MAX_HAND, 7), dtype=np.float32)
        hand_mask = np.zeros((MAX_HAND,), dtype=np.float32)
        for index, card in enumerate(list(obs.get("hand") or [])[:MAX_HAND]):
            hand_ids[index] = _id_lookup(CARD_ID_VOCAB, card.get("card_id"))
            hand_type_ids[index] = int(CARD_TYPE_IDS.get(str(card.get("card_type") or ""), 0))
            hand_target_ids[index] = int(TARGET_MODE_IDS.get(str(card.get("target_mode") or ""), 0))
            hand_numeric[index] = np.asarray(
                [
                    float(card.get("cost_for_turn") or 0.0),
                    _bool(card.get("upgraded")),
                    _bool(card.get("playable")),
                    _bool(card.get("exhausts_when_played")),
                    _bool(card.get("ethereal")),
                    _bool(card.get("retain")),
                    float(card.get("index") or 0.0),
                ],
                dtype=np.float32,
            )
            hand_mask[index] = 1.0

        potion_ids = np.zeros((MAX_POTIONS,), dtype=np.int64)
        potion_target_ids = np.zeros((MAX_POTIONS,), dtype=np.int64)
        potion_numeric = np.zeros((MAX_POTIONS, 2), dtype=np.float32)
        potion_mask = np.zeros((MAX_POTIONS,), dtype=np.float32)
        for index, potion in enumerate(list(obs.get("potions") or [])[:MAX_POTIONS]):
            potion_ids[index] = _id_lookup(POTION_ID_VOCAB, potion.get("potion_id"))
            potion_target_ids[index] = int(TARGET_MODE_IDS.get(str(potion.get("target_mode") or ""), 0))
            potion_numeric[index] = np.asarray(
                [float(potion.get("slot") or 0.0), _bool(potion.get("usable"))],
                dtype=np.float32,
            )
            potion_mask[index] = 1.0

        monster_ids = np.zeros((MAX_MONSTERS,), dtype=np.int64)
        monster_intent_ids = np.zeros((MAX_MONSTERS,), dtype=np.int64)
        monster_numeric = np.zeros((MAX_MONSTERS, 20), dtype=np.float32)
        monster_mask = np.zeros((MAX_MONSTERS,), dtype=np.float32)
        monster_power_ids = np.zeros((MAX_MONSTERS, MAX_MONSTER_POWERS), dtype=np.int64)
        monster_power_numeric = np.zeros((MAX_MONSTERS, MAX_MONSTER_POWERS, 4), dtype=np.float32)
        monster_power_mask = np.zeros((MAX_MONSTERS, MAX_MONSTER_POWERS), dtype=np.float32)
        for index, monster in enumerate(list(obs.get("monsters") or [])[:MAX_MONSTERS]):
            mechanic_state = monster.get("mechanic_state") or {}
            powers = list(monster.get("powers") or [])
            power_summary = _pool_power_summary(powers)
            monster_ids[index] = _id_lookup(MONSTER_ID_VOCAB, monster.get("monster_id"))
            monster_intent_ids[index] = int(
                INTENT_KIND_IDS.get(str((monster.get("intent_payload") or {}).get("kind") or ""), 0)
            )
            monster_numeric[index] = np.asarray(
                [
                    float(monster.get("current_hp") or 0.0),
                    float(monster.get("max_hp") or 0.0),
                    float(monster.get("block") or 0.0),
                    float(monster.get("current_hp") or 0.0) / max(float(monster.get("max_hp") or 1.0), 1.0),
                    _bool(monster.get("alive")),
                    _bool(monster.get("targetable")),
                    float((monster.get("intent_payload") or {}).get("damage_per_hit") or 0.0),
                    float((monster.get("intent_payload") or {}).get("hits") or 0.0),
                    float((monster.get("intent_payload") or {}).get("total_damage") or 0.0),
                    float(monster.get("belief_expected_incoming") or 0.0),
                    float(monster.get("belief_max_incoming") or 0.0),
                    float(mechanic_state.get("planned_move_id") or 0.0),
                    float(len(mechanic_state.get("move_history") or [])),
                    float(mechanic_state.get("split_threshold") or 0.0),
                    _bool(mechanic_state.get("split_ready")),
                    float(mechanic_state.get("regrow_counter") or 0.0),
                    float(mechanic_state.get("guardian_threshold") or 0.0),
                    float(mechanic_state.get("guardian_damage_taken") or 0.0),
                    _bool(mechanic_state.get("sleeping")),
                    float(sum(power_summary)),
                ],
                dtype=np.float32,
            )
            monster_mask[index] = 1.0
            for power_index, power in enumerate(powers[:MAX_MONSTER_POWERS]):
                monster_power_ids[index, power_index] = _id_lookup(POWER_ID_VOCAB, power.get("id"))
                monster_power_numeric[index, power_index] = np.asarray(_power_numeric(power), dtype=np.float32)
                monster_power_mask[index, power_index] = 1.0

        choice_kind_id = np.asarray([CHOICE_KIND_IDS.get(obs.get("pending_choice_kind"), 0)], dtype=np.int64)
        choice_card_ids = np.zeros((MAX_CHOICE_OPTIONS,), dtype=np.int64)
        choice_source_ids = np.zeros((MAX_CHOICE_OPTIONS,), dtype=np.int64)
        choice_numeric = np.zeros((MAX_CHOICE_OPTIONS, 4), dtype=np.float32)
        choice_mask = np.zeros((MAX_CHOICE_OPTIONS,), dtype=np.float32)
        for index, option in enumerate(list(pending_choice.get("options") or [])[:MAX_CHOICE_OPTIONS]):
            choice_card_ids[index] = _id_lookup(CARD_ID_VOCAB, option.get("card_id"))
            choice_source_ids[index] = int(OPTION_SOURCE_IDS.get(option.get("source_pile"), 0))
            choice_numeric[index] = np.asarray(
                [
                    float(option.get("option_index") or 0.0),
                    _bool(option.get("card_uuid") is not None),
                    float(len(option.get("selection_uuids") or [])),
                    float(len(str(option.get("label") or ""))),
                ],
                dtype=np.float32,
            )
            choice_mask[index] = 1.0

        action_type_mask = np.zeros((ACTION_TYPE_COUNT,), dtype=np.float32)
        play_card_mask = np.zeros((MAX_HAND,), dtype=np.float32)
        play_card_target_mask = np.zeros((MAX_HAND, MAX_TARGET_OPTIONS), dtype=np.float32)
        use_potion_mask = np.zeros((MAX_POTIONS,), dtype=np.float32)
        use_potion_target_mask = np.zeros((MAX_POTIONS, MAX_TARGET_OPTIONS), dtype=np.float32)
        choice_option_mask = np.zeros((MAX_CHOICE_OPTIONS,), dtype=np.float32)

        choice_candidates = _choice_candidates(payload)
        for local_index, candidate in enumerate(choice_candidates[:MAX_CHOICE_OPTIONS]):
            choice_option_mask[local_index] = 1.0
        for candidate, is_legal in zip(payload.get("action_candidates") or [], payload.get("action_mask") or []):
            if not is_legal:
                continue
            family = str(candidate.get("action_family") or "")
            if family == "end_turn":
                action_type_mask[ACTION_TYPE_END_TURN] = 1.0
            elif family == "play_card":
                action_type_mask[ACTION_TYPE_PLAY_CARD] = 1.0
                slot = _int_or(candidate.get("slot_index"), -1)
                if 0 <= slot < MAX_HAND:
                    play_card_mask[slot] = 1.0
                    play_card_target_mask[slot, _target_index_from_candidate(candidate)] = 1.0
            elif family == "use_potion":
                action_type_mask[ACTION_TYPE_USE_POTION] = 1.0
                slot = _int_or(candidate.get("slot_index"), -1)
                if 0 <= slot < MAX_POTIONS:
                    use_potion_mask[slot] = 1.0
                    use_potion_target_mask[slot, _target_index_from_candidate(candidate)] = 1.0
            elif family in CHOICE_FAMILIES:
                action_type_mask[ACTION_TYPE_CHOICE] = 1.0
            elif family == "proceed":
                action_type_mask[ACTION_TYPE_PROCEED] = 1.0
            elif family == "cancel":
                action_type_mask[ACTION_TYPE_CANCEL] = 1.0

        return {
            "global_numeric": global_numeric,
            "player_power_ids": player_power_ids,
            "player_power_numeric": player_power_numeric,
            "player_power_mask": player_power_mask,
            "hand_ids": hand_ids,
            "hand_type_ids": hand_type_ids,
            "hand_target_ids": hand_target_ids,
            "hand_numeric": hand_numeric,
            "hand_mask": hand_mask,
            "potion_ids": potion_ids,
            "potion_target_ids": potion_target_ids,
            "potion_numeric": potion_numeric,
            "potion_mask": potion_mask,
            "monster_ids": monster_ids,
            "monster_intent_ids": monster_intent_ids,
            "monster_numeric": monster_numeric,
            "monster_mask": monster_mask,
            "monster_power_ids": monster_power_ids,
            "monster_power_numeric": monster_power_numeric,
            "monster_power_mask": monster_power_mask,
            "choice_kind_id": choice_kind_id,
            "choice_card_ids": choice_card_ids,
            "choice_source_ids": choice_source_ids,
            "choice_numeric": choice_numeric,
            "choice_mask": choice_mask,
            "action_type_mask": action_type_mask,
            "play_card_mask": play_card_mask,
            "play_card_target_mask": play_card_target_mask,
            "use_potion_mask": use_potion_mask,
            "use_potion_target_mask": use_potion_target_mask,
            "choice_option_mask": choice_option_mask,
        }

    def _info_from_response(
        self,
        response: dict[str, Any],
        *,
        invalid_action: bool,
        decoder_failure: bool,
    ) -> dict[str, Any]:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        pressure = obs.get("pressure") or {}
        reward_breakdown = response.get("reward_breakdown") or {}
        spec_name = response.get("spec_name") or obs.get("env_name") or (self._current_spec.stem if self._current_spec else None)
        probe_targets = _derive_probe_targets(obs)
        return {
            "spec_name": spec_name,
            "curriculum_tag": curriculum_tag_from_spec_name(Path(spec_name).stem if spec_name else "unknown"),
            "invalid_action": invalid_action,
            "decoder_failure": decoder_failure,
            "legal_action_count": payload.get("legal_action_count"),
            "chosen_action_label": response.get("chosen_action_label"),
            "outcome": response.get("outcome"),
            "player_hp": obs.get("player_hp"),
            "energy": obs.get("energy"),
            "turn_count": obs.get("turn_count"),
            "visible_incoming": pressure.get("visible_incoming"),
            "visible_unblocked": pressure.get("visible_unblocked"),
            "reward_breakdown": reward_breakdown,
            "logged_reward_breakdown": reward_breakdown,
            "action_candidates": payload.get("action_candidates") or [],
            "pending_choice_kind": obs.get("pending_choice_kind"),
            "probe_targets": probe_targets,
            "raw_observation": obs,
        }

    def _effective_reward_terms(self, info: dict[str, Any], invalid_penalty: float) -> dict[str, Any]:
        if self.reward_mode != "minimal_rl":
            raise RuntimeError(f"unsupported reward_mode '{self.reward_mode}'")
        breakdown = info.get("logged_reward_breakdown") or info.get("reward_breakdown") or {}
        terms = {
            "reward_mode": self.reward_mode,
            "invalid_action_penalty": float(invalid_penalty),
            "terminal_victory_term": 0.0,
            "terminal_defeat_term": 0.0,
            "hp_loss_term": 0.0,
            "enemy_hp_delta_term": 0.0,
            "kill_bonus_term": 0.0,
            "next_enemy_window_relief_term": 0.0,
            "catastrophe_term": 0.0,
            "used_breakdown_keys": [],
        }
        outcome = str(info.get("outcome") or "")
        if outcome == "victory":
            terms["terminal_victory_term"] = float(self.reward_config["victory_reward"])
        elif outcome == "defeat":
            terms["terminal_defeat_term"] = float(self.reward_config["defeat_reward"])
        if "player_hp_delta" in breakdown:
            terms["used_breakdown_keys"].append("player_hp_delta")
        terms["hp_loss_term"] = float(breakdown.get("player_hp_delta") or 0.0) * float(
            self.reward_config["hp_loss_scale"]
        )
        if "enemy_hp_delta" in breakdown:
            terms["used_breakdown_keys"].append("enemy_hp_delta")
        terms["enemy_hp_delta_term"] = float(breakdown.get("enemy_hp_delta") or 0.0) * float(
            self.reward_config["enemy_hp_delta_scale"]
        )
        if "kill_bonus" in breakdown:
            terms["used_breakdown_keys"].append("kill_bonus")
        terms["kill_bonus_term"] = float(breakdown.get("kill_bonus") or 0.0) * float(
            self.reward_config["kill_bonus_scale"]
        )
        if "next_enemy_window_relief" in breakdown:
            terms["used_breakdown_keys"].append("next_enemy_window_relief")
        terms["next_enemy_window_relief_term"] = float(
            breakdown.get("next_enemy_window_relief") or 0.0
        ) * float(self.reward_config["next_enemy_window_relief_scale"])
        if float(info.get("visible_unblocked") or 0.0) >= float(
            self.reward_config["catastrophe_unblocked_threshold"]
        ):
            terms["catastrophe_term"] = -float(self.reward_config["catastrophe_penalty"])
        terms["total_effective_reward"] = float(
            terms["invalid_action_penalty"]
            + terms["terminal_victory_term"]
            + terms["terminal_defeat_term"]
            + terms["hp_loss_term"]
            + terms["enemy_hp_delta_term"]
            + terms["kill_bonus_term"]
            + terms["next_enemy_window_relief_term"]
            + terms["catastrophe_term"]
        )
        return terms


__all__ = [
    "ACTION_TYPE_CANCEL",
    "ACTION_TYPE_CHOICE",
    "ACTION_TYPE_COUNT",
    "ACTION_TYPE_END_TURN",
    "ACTION_TYPE_PLAY_CARD",
    "ACTION_TYPE_PROCEED",
    "ACTION_TYPE_USE_POTION",
    "MAX_CHOICE_OPTIONS",
    "MAX_HAND",
    "MAX_MONSTERS",
    "MAX_POTIONS",
    "MAX_TARGET_OPTIONS",
    "StructuredGymCombatEnv",
    "discover_spec_paths",
]
