#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import random
import subprocess
from pathlib import Path
from typing import Any

try:
    import gymnasium as gym
    import numpy as np
    from gymnasium import spaces
except ModuleNotFoundError as err:
    missing = err.name or "required package"
    raise SystemExit(
        f"Missing Python dependency '{missing}'. Use the project RL venv instead of system Python:\n"
        r"  .\.venv-rl\Scripts\python.exe tools\learning\smoke_full_run_env.py"
        "\nIf the venv is not initialized, run:\n"
        r"  python -m venv .venv-rl"
        "\n"
        r"  .\.venv-rl\Scripts\python.exe -m pip install -r tools\learning\requirements-hybrid-rl.txt"
    ) from err

from combat_rl_common import REPO_ROOT, find_release_binary

MAX_ACTIONS = 256
CARD_FEATURES = 25
ACTION_FEATURES = 5 + CARD_FEATURES
BASE_OBS_DIM = 41
OBS_DIM = BASE_OBS_DIM + (MAX_ACTIONS * ACTION_FEATURES)
PLAN_PROFILE_FEATURES = 10
PLAN_DELTA_FEATURES = 13
REWARD_ACTION_FEATURES = 10
PLAN_BASE_OBS_DIM = BASE_OBS_DIM + PLAN_PROFILE_FEATURES
PLAN_ACTION_FEATURES = ACTION_FEATURES + PLAN_DELTA_FEATURES
PLAN_REWARD_ACTION_FEATURES = PLAN_ACTION_FEATURES + REWARD_ACTION_FEATURES
PLAN_OBS_DIM = PLAN_BASE_OBS_DIM + (MAX_ACTIONS * PLAN_ACTION_FEATURES)
PLAN_REWARD_OBS_DIM = PLAN_BASE_OBS_DIM + (MAX_ACTIONS * PLAN_REWARD_ACTION_FEATURES)

DECISION_TYPE_IDS = {
    "none": 0,
    "combat": 1,
    "combat_hand_select": 2,
    "combat_grid_select": 3,
    "combat_discovery": 4,
    "combat_scry": 5,
    "combat_card_reward": 6,
    "combat_stance": 7,
    "reward": 8,
    "reward_card_choice": 15,
    "campfire": 9,
    "shop": 10,
    "map": 11,
    "event": 12,
    "run_deck_selection": 13,
    "boss_relic": 14,
}

ACTION_TYPE_IDS = {
    "play_card": 1,
    "use_potion": 2,
    "discard_potion": 3,
    "end_turn": 4,
    "submit_card_choice": 5,
    "submit_discover_choice": 6,
    "select_map_node": 7,
    "fly_to_node": 8,
    "select_event_option": 9,
    "campfire_option": 10,
    "event_choice": 11,
    "submit_scry_discard": 12,
    "submit_selection": 13,
    "submit_hand_select": 14,
    "submit_grid_select": 15,
    "submit_deck_select": 16,
    "claim_reward": 17,
    "select_card": 18,
    "buy_card": 19,
    "buy_relic": 20,
    "buy_potion": 21,
    "purge_card": 22,
    "submit_relic_choice": 23,
    "proceed": 24,
    "cancel": 25,
}


def _stable_token(text: str | None, buckets: int = 4096) -> float:
    if not text:
        return 0.0
    digest = hashlib.md5(text.encode("utf-8")).digest()
    value = int.from_bytes(digest[:4], "little") % buckets
    return float(value) / float(max(buckets - 1, 1))


class FullRunEnvDriver:
    def __init__(self, binary: Path | None = None) -> None:
        self.binary = find_release_binary(binary, "full_run_env_driver")
        self.proc: subprocess.Popen[str] | None = None

    def start(self) -> None:
        if self.proc is not None:
            return
        self.proc = subprocess.Popen(
            [str(self.binary)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            cwd=str(REPO_ROOT),
            bufsize=1,
        )
        self.request({"cmd": "ping"})

    def close(self) -> None:
        if self.proc is None:
            return
        try:
            self.request({"cmd": "close"})
        except Exception:
            pass
        proc = self.proc
        self.proc = None
        try:
            proc.terminate()
        except Exception:
            pass
        try:
            proc.wait(timeout=1.0)
        except Exception:
            pass
        for handle in (proc.stdin, proc.stdout, proc.stderr):
            if handle is None:
                continue
            try:
                handle.close()
            except Exception:
                pass

    def request(self, payload: dict[str, Any]) -> dict[str, Any]:
        self.start()
        assert self.proc is not None and self.proc.stdin is not None and self.proc.stdout is not None
        self.proc.stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            stderr = ""
            if self.proc.stderr is not None:
                try:
                    stderr = self.proc.stderr.read()
                except Exception:
                    stderr = ""
            raise RuntimeError(f"full_run_env_driver exited unexpectedly: {stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(str(response.get("error") or "unknown full_run_env_driver error"))
        return response


class FullRunGymEnv(gym.Env[np.ndarray, int]):
    metadata = {"render_modes": []}

    def __init__(
        self,
        driver_binary: Path | None = None,
        seed: int = 1,
        ascension: int = 0,
        final_act: bool = False,
        player_class: str = "ironclad",
        max_episode_steps: int = 5000,
        invalid_action_penalty: float = -2.0,
        reward_shaping_profile: str = "baseline",
        feature_profile: str = "baseline",
    ) -> None:
        super().__init__()
        if feature_profile not in {"baseline", "plan_v0", "plan_reward_v0"}:
            raise ValueError("feature_profile must be 'baseline', 'plan_v0', or 'plan_reward_v0'")
        self.driver = FullRunEnvDriver(driver_binary)
        self._rng = random.Random(seed)
        self.seed = int(seed)
        self.ascension = int(ascension)
        self.final_act = bool(final_act)
        self.player_class = str(player_class)
        self.max_episode_steps = int(max_episode_steps)
        self.invalid_action_penalty = float(invalid_action_penalty)
        self.reward_shaping_profile = str(reward_shaping_profile or "baseline")
        self.feature_profile = str(feature_profile)
        self.base_obs_dim = PLAN_BASE_OBS_DIM if self.feature_profile != "baseline" else BASE_OBS_DIM
        self.action_features = {
            "baseline": ACTION_FEATURES,
            "plan_v0": PLAN_ACTION_FEATURES,
            "plan_reward_v0": PLAN_REWARD_ACTION_FEATURES,
        }[self.feature_profile]
        self.obs_dim = self.base_obs_dim + (MAX_ACTIONS * self.action_features)
        self._last_response: dict[str, Any] | None = None
        self._step_count = 0
        self.observation_space = spaces.Box(low=-1e6, high=1e6, shape=(self.obs_dim,), dtype=np.float32)
        self.action_space = spaces.Discrete(MAX_ACTIONS)

    def reset(
        self,
        *,
        seed: int | None = None,
        options: dict[str, Any] | None = None,
    ) -> tuple[np.ndarray, dict[str, Any]]:
        super().reset(seed=seed)
        if seed is not None:
            self._rng.seed(seed)
        options = options or {}
        run_seed = int(options.get("run_seed") or options.get("seed") or self._rng.randrange(1, 2**31))
        response = self.driver.request(
            {
                "cmd": "reset",
                "seed": run_seed,
                "ascension": int(options.get("ascension", self.ascension)),
                "final_act": bool(options.get("final_act", self.final_act)),
                "class": str(options.get("class", self.player_class)),
                "max_steps": int(options.get("max_steps", self.max_episode_steps)),
                "reward_shaping_profile": str(options.get("reward_shaping_profile", self.reward_shaping_profile)),
            }
        )
        self._last_response = response
        self._step_count = 0
        return self._encode_observation(response), self._info_from_response(response, invalid_action=False)

    def step(self, action: int) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        if self._last_response is None:
            raise RuntimeError("FullRunGymEnv.step called before reset")
        action_mask = self.action_masks()
        invalid_action = bool(action < 0 or action >= MAX_ACTIONS or not action_mask[action])
        penalty = self.invalid_action_penalty if invalid_action else 0.0
        if invalid_action:
            legal_indices = np.flatnonzero(action_mask)
            if len(legal_indices) == 0:
                raise RuntimeError("no legal actions available for full-run env step")
            action = int(legal_indices[0])

        response = self.driver.request({"cmd": "step", "action_index": int(action)})
        self._last_response = response
        self._step_count += 1
        terminated = bool(response.get("done"))
        truncated = self._step_count >= self.max_episode_steps and not terminated
        reward = float(response.get("reward") or 0.0) + penalty
        info = self._info_from_response(response, invalid_action=invalid_action)
        info["invalid_action_penalty"] = penalty
        return self._encode_observation(response), reward, terminated, truncated, info

    def action_masks(self) -> np.ndarray:
        mask = np.zeros(MAX_ACTIONS, dtype=bool)
        if self._last_response is None:
            return mask
        payload = self._last_response.get("payload") or {}
        current = list(payload.get("action_mask") or [])
        limit = min(len(current), MAX_ACTIONS)
        mask[:limit] = np.asarray(current[:limit], dtype=bool)
        return mask

    def sample_random_legal_action(self) -> int:
        legal_indices = np.flatnonzero(self.action_masks())
        if len(legal_indices) == 0:
            return 0
        return int(self._rng.choice([int(index) for index in legal_indices]))

    def close(self) -> None:
        self.driver.close()

    def _encode_observation(self, response: dict[str, Any]) -> np.ndarray:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        combat = obs.get("combat") or {}
        screen = obs.get("screen") or {}
        info = response.get("info") or {}
        mask = list(payload.get("action_mask") or [])
        candidates = list(payload.get("action_candidates") or [])
        base = [
            float(obs.get("act") or 0),
            float(obs.get("floor") or 0),
            float(obs.get("current_hp") or 0),
            float(obs.get("max_hp") or 0),
            float(obs.get("hp_ratio_milli") or 0) / 1000.0,
            float(obs.get("gold") or 0),
            float(obs.get("deck_size") or 0),
            float(obs.get("relic_count") or 0),
            float(obs.get("potion_slots") or 0),
            float(obs.get("filled_potion_slots") or 0),
            float(payload.get("legal_action_count") or 0),
            float(DECISION_TYPE_IDS.get(str(obs.get("decision_type") or "none"), 0)),
            float(combat.get("player_block") or 0),
            float(combat.get("energy") or 0),
            float(combat.get("turn_count") or 0),
            float(combat.get("hand_count") or 0),
            float(combat.get("draw_count") or 0),
            float(combat.get("discard_count") or 0),
            float(combat.get("exhaust_count") or 0),
            float(combat.get("alive_monster_count") or 0),
            float(combat.get("total_monster_hp") or 0),
            float(combat.get("visible_incoming_damage") or 0),
            float(info.get("combat_win_count") or 0),
            float(screen.get("reward_item_count") or 0),
            float(screen.get("reward_card_choice_count") or 0),
            float(screen.get("shop_card_count") or 0),
            float(screen.get("shop_relic_count") or 0),
            float(screen.get("shop_potion_count") or 0),
            *self._deck_features(obs.get("deck") or {}),
        ]
        if self.feature_profile != "baseline":
            base.extend(self._plan_profile_features(obs.get("plan_profile") or {}))
        values = list(base)
        for index in range(min(len(candidates), MAX_ACTIONS)):
            candidate = candidates[index] or {}
            action = candidate.get("action") or {}
            action_type = str(action.get("type") or "")
            values.extend(
                [
                    1.0 if index < len(mask) and bool(mask[index]) else 0.0,
                    float(ACTION_TYPE_IDS.get(action_type, 0)),
                    _stable_token(str(candidate.get("action_key") or "")),
                    float(candidate.get("action_id") or 0) / float(2**32 - 1),
                    float(index),
                    *self._card_features(candidate.get("card")),
                ]
            )
            if self.feature_profile != "baseline":
                values.extend(self._plan_delta_features(candidate.get("plan_delta") or {}))
            if self.feature_profile == "plan_reward_v0":
                values.extend(self._reward_action_features(candidate.get("reward_structure") or {}))
        missing_action_features = (MAX_ACTIONS * self.action_features) - (len(values) - self.base_obs_dim)
        values.extend([0.0] * max(missing_action_features, 0))
        return np.asarray(values[: self.obs_dim], dtype=np.float32)

    def _deck_features(self, deck: dict[str, Any]) -> list[float]:
        deck_size = max(float(deck.get("attack_count") or 0) + float(deck.get("skill_count") or 0) + float(deck.get("power_count") or 0), 1.0)
        return [
            float(deck.get("attack_count") or 0) / deck_size,
            float(deck.get("skill_count") or 0) / deck_size,
            float(deck.get("power_count") or 0) / deck_size,
            float(deck.get("status_count") or 0) / 10.0,
            float(deck.get("curse_count") or 0) / 10.0,
            float(deck.get("starter_basic_count") or 0) / deck_size,
            float(deck.get("damage_card_count") or 0) / deck_size,
            float(deck.get("block_card_count") or 0) / deck_size,
            float(deck.get("draw_card_count") or 0) / deck_size,
            float(deck.get("scaling_card_count") or 0) / deck_size,
            float(deck.get("exhaust_card_count") or 0) / deck_size,
            float(deck.get("average_cost_milli") or 0) / 1000.0,
            1.0,
        ]

    def _plan_profile_features(self, profile: dict[str, Any]) -> list[float]:
        return [
            float(profile.get("frontload_supply") or 0) / 120.0,
            float(profile.get("block_supply") or 0) / 120.0,
            float(profile.get("draw_supply") or 0) / 80.0,
            float(profile.get("scaling_supply") or 0) / 80.0,
            float(profile.get("aoe_supply") or 0) / 80.0,
            float(profile.get("exhaust_supply") or 0) / 80.0,
            float(profile.get("kill_window_supply") or 0) / 80.0,
            float(profile.get("starter_basic_burden") or 0) / 100.0,
            float(profile.get("setup_cashout_risk") or 0) / 100.0,
            1.0,
        ]

    def _plan_delta_features(self, delta: dict[str, Any]) -> list[float]:
        plan_adjusted = max(min(float(delta.get("plan_adjusted_score") or 0.0), 240.0), -240.0) / 240.0
        return [
            float(delta.get("frontload_delta") or 0) / 80.0,
            float(delta.get("block_delta") or 0) / 80.0,
            float(delta.get("draw_delta") or 0) / 40.0,
            float(delta.get("scaling_delta") or 0) / 40.0,
            float(delta.get("aoe_delta") or 0) / 60.0,
            float(delta.get("exhaust_delta") or 0) / 40.0,
            float(delta.get("kill_window_delta") or 0) / 40.0,
            float(delta.get("starter_basic_burden_delta") or 0) / 40.0,
            float(delta.get("setup_cashout_risk_delta") or 0) / 40.0,
            float(delta.get("deck_deficit_bonus") or 0) / 120.0,
            float(delta.get("bloat_penalty") or 0) / 80.0,
            float(delta.get("duplicate_penalty") or 0) / 80.0,
            plan_adjusted,
        ]

    def _reward_action_features(self, reward: dict[str, Any]) -> list[float]:
        item_type = str(reward.get("claim_reward_item_type") or "")
        return [
            1.0 if reward.get("is_reward_action") else 0.0,
            1.0 if reward.get("claim_opens_card_choice") else 0.0,
            max(min(float(reward.get("claim_free_value_score") or 0.0), 180.0), -80.0) / 180.0,
            1.0 if reward.get("claim_likely_waste") else 0.0,
            1.0 if reward.get("claim_capacity_blocked") else 0.0,
            1.0 if reward.get("is_proceed_with_unclaimed_rewards") else 0.0,
            float(reward.get("unclaimed_reward_count") or 0) / 8.0,
            float(reward.get("unclaimed_card_reward_count") or 0) / 5.0,
            1.0 if reward.get("proceed_is_cleanup") else 0.0,
            _stable_token(item_type),
        ]

    def _card_features(self, card: Any) -> list[float]:
        if not isinstance(card, dict):
            return [0.0] * CARD_FEATURES
        rule_score = float(card.get("rule_score") or 0.0)
        rule_score = max(min(rule_score, 200.0), -200.0) / 200.0
        return [
            1.0,
            float(card.get("card_id_hash") or 0) / float(2**32 - 1),
            float(card.get("card_type_id") or 0) / 5.0,
            float(card.get("rarity_id") or 0) / 6.0,
            float(card.get("cost") or 0) / 4.0,
            float(card.get("upgrades") or 0) / 5.0,
            float(card.get("base_damage") or 0) / 50.0,
            float(card.get("base_block") or 0) / 50.0,
            float(card.get("base_magic") or 0) / 20.0,
            float(card.get("upgraded_damage") or 0) / 60.0,
            float(card.get("upgraded_block") or 0) / 60.0,
            float(card.get("upgraded_magic") or 0) / 25.0,
            1.0 if card.get("exhaust") else 0.0,
            1.0 if card.get("ethereal") else 0.0,
            1.0 if card.get("innate") else 0.0,
            1.0 if card.get("aoe") else 0.0,
            1.0 if card.get("multi_damage") else 0.0,
            1.0 if card.get("starter_basic") else 0.0,
            1.0 if card.get("draws_cards") else 0.0,
            1.0 if card.get("gains_energy") else 0.0,
            1.0 if card.get("applies_weak") else 0.0,
            1.0 if card.get("applies_vulnerable") else 0.0,
            1.0 if card.get("scaling_piece") else 0.0,
            float(card.get("deck_copies") or 0) / 5.0,
            rule_score,
        ]

    def _info_from_response(self, response: dict[str, Any], invalid_action: bool) -> dict[str, Any]:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        info = dict(response.get("info") or {})
        info.update(
            {
                "invalid_action": invalid_action,
                "reward": response.get("reward"),
                "done": response.get("done"),
                "chosen_action_key": response.get("chosen_action_key"),
                "legal_action_count": payload.get("legal_action_count"),
                "decision_type": obs.get("decision_type"),
                "engine_state": obs.get("engine_state"),
                "act": obs.get("act"),
                "floor": obs.get("floor"),
                "current_hp": obs.get("current_hp"),
                "max_hp": obs.get("max_hp"),
                "gold": obs.get("gold"),
                "deck_size": obs.get("deck_size"),
                "relic_count": obs.get("relic_count"),
                "action_candidates": payload.get("action_candidates") or [],
                "raw_payload": payload,
            }
        )
        return info


__all__ = [
    "FullRunEnvDriver",
    "FullRunGymEnv",
    "MAX_ACTIONS",
    "ACTION_FEATURES",
    "BASE_OBS_DIM",
    "OBS_DIM",
    "PLAN_ACTION_FEATURES",
    "PLAN_BASE_OBS_DIM",
    "PLAN_OBS_DIM",
    "PLAN_REWARD_ACTION_FEATURES",
    "PLAN_REWARD_OBS_DIM",
]
