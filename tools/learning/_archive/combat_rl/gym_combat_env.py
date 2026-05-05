#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import random
import subprocess
from pathlib import Path
from typing import Any

import gymnasium as gym
import numpy as np
from gymnasium import spaces

from card_semantics import card_semantic_vector, card_semantics, normalize_card_name
from combat_rl_common import REPO_ROOT, curriculum_tag_from_spec_name, find_release_binary

MAX_ACTIONS = 64
MAX_HAND = 10
MAX_MONSTERS = 5
MAX_HEXAGHOST_WINDOWS = 4
CARD_HASH_BUCKETS = 2048
ACTION_HASH_BUCKETS = 2048
CARD_SEMANTIC_DIM = 27
PENDING_CHOICE_KINDS = {
    None: 0,
    "grid_select": 1,
    "hand_select": 2,
    "discovery_select": 3,
    "scry_select": 4,
    "card_reward_select": 5,
    "stance_choice": 6,
}
ACTION_FAMILY_IDS = {
    "end_turn": 1,
    "play_card": 2,
    "use_potion": 3,
    "discover_select": 4,
    "card_select": 5,
    "hand_select": 6,
    "grid_select": 7,
    "proceed": 8,
    "cancel": 9,
    "raw": 10,
}
HEXAGHOST_MOVE_IDS = {
    "Unknown": 0,
    "Divider": 1,
    "Tackle": 2,
    "Inflame": 3,
    "Sear": 4,
    "Activate": 5,
    "Inferno": 6,
}
DIAGNOSTIC_ONLY_BREAKDOWN_KEYS = {
    "stabilize_bonus",
    "idle_penalty",
    "total",
}


def _stable_token(text: str | None, buckets: int) -> float:
    if not text:
        return 0.0
    digest = hashlib.md5(text.encode("utf-8")).digest()
    value = int.from_bytes(digest[:4], "little") % buckets
    return float(value) / float(max(buckets - 1, 1))


def _bool(value: Any) -> float:
    return 1.0 if value else 0.0


class CombatEnvDriver:
    def __init__(self, binary: Path | None = None) -> None:
        self.binary = find_release_binary(binary, "combat_env_driver")
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
                stderr = self.proc.stderr.read()
            raise RuntimeError(f"combat_env_driver exited unexpectedly: {stderr}")
        response = json.loads(line)
        if not response.get("ok"):
            raise RuntimeError(str(response.get("error") or "unknown combat_env_driver error"))
        return response


class GymCombatEnv(gym.Env[np.ndarray, int]):
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
        reward_mode: str = "legacy",
        reward_config: dict[str, float] | None = None,
        draw_order_variant: str = "exact",
    ) -> None:
        super().__init__()
        if not spec_paths:
            raise ValueError("GymCombatEnv requires at least one spec path")
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
            "invalid_action_penalty": self.invalid_action_penalty,
        }
        if reward_config:
            self.reward_config.update({key: float(value) for key, value in reward_config.items()})
        self._last_response: dict[str, Any] | None = None
        self._current_spec: Path | None = None
        self._step_count = 0
        self._setup_activated = False
        obs_dim = self._observation_length()
        self.observation_space = spaces.Box(low=-1e6, high=1e6, shape=(obs_dim,), dtype=np.float32)
        self.action_space = spaces.Discrete(MAX_ACTIONS)

    def reset(self, *, seed: int | None = None, options: dict[str, Any] | None = None) -> tuple[np.ndarray, dict[str, Any]]:
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
        self._setup_activated = False
        info = self._info_from_response(response, invalid_action=False)
        return self._encode_observation(response), info

    def step(self, action: int) -> tuple[np.ndarray, float, bool, bool, dict[str, Any]]:
        if self._last_response is None:
            raise RuntimeError("GymCombatEnv.step called before reset")
        payload = self._last_response.get("payload") or {}
        action_mask = list(payload.get("action_mask") or [])
        invalid_action = bool(action >= len(action_mask) or action >= MAX_ACTIONS or not action_mask[action])
        penalty = self.invalid_action_penalty if invalid_action else 0.0
        if invalid_action:
            legal_indices = [idx for idx, legal in enumerate(action_mask) if legal]
            if not legal_indices:
                raise RuntimeError("no legal actions available for combat env step")
            action = legal_indices[0]
        response = self.driver.request({"cmd": "step", "action_index": int(action)})
        self._last_response = response
        self._step_count += 1
        terminated = bool(response.get("done"))
        truncated = self._step_count >= self.max_episode_steps and not terminated
        info = self._info_from_response(response, invalid_action=invalid_action)
        if self.reward_mode == "legacy":
            reward = self._compute_reward(response, info, penalty)
            effective_reward_terms = {
                "reward_mode": "legacy",
                "legacy_reward_term": float(reward),
                "invalid_action_penalty": float(penalty),
                "used_breakdown_keys": [],
                "total_effective_reward": float(reward),
            }
        else:
            effective_reward_terms = self._effective_reward_terms(info, penalty)
            reward = float(effective_reward_terms["total_effective_reward"])
        info["effective_reward_terms"] = effective_reward_terms
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

    def close(self) -> None:
        self.driver.close()

    @staticmethod
    def _observation_length() -> int:
        base = 19 + 4 + (MAX_HEXAGHOST_WINDOWS * 5)
        monster_features = MAX_MONSTERS * 8
        hand_features = MAX_HAND * (5 + CARD_SEMANTIC_DIM)
        action_features = MAX_ACTIONS * (6 + CARD_SEMANTIC_DIM)
        return base + monster_features + hand_features + action_features

    @staticmethod
    def _action_card_name(label: str | None) -> str:
        text = str(label or "")
        if text.startswith("Play #"):
            head = text.split(" @", 1)[0]
            parts = head.split(" ", 2)
            if len(parts) >= 3:
                return normalize_card_name(parts[2])
        return ""

    def _bucket_shaping_bonus(self, info: dict[str, Any]) -> float:
        tag = str(info.get("curriculum_tag") or "")
        if tag != "setup_before_payoff":
            return 0.0
        label = str(info.get("chosen_action_label") or "")
        card_name = self._action_card_name(label)
        semantics = card_semantics(card_name)
        bonus = 0.0
        first_action = self._step_count == 1
        if first_action and float(semantics["setup_tag"]) > 0.0:
            bonus += 1.5
            self._setup_activated = True
        elif first_action and float(semantics["payoff_tag"]) > 0.0 and not self._setup_activated:
            bonus -= 0.15
        elif first_action and (
            "Defend" in label
            or "Impervious" in label
            or "Power Through" in label
            or label == "EndTurn"
        ):
            if int(info.get("visible_incoming") or 0) <= 5 and int(info.get("value_incoming") or 0) <= 5:
                bonus -= 0.75
        if float(semantics["setup_tag"]) > 0.0:
            self._setup_activated = True
        if self._setup_activated and float(semantics["payoff_tag"]) > 0.0 and not first_action:
            bonus += 0.6
        return bonus

    def _effective_reward_terms(self, info: dict[str, Any], invalid_penalty: float) -> dict[str, Any]:
        breakdown = info.get("logged_reward_breakdown") or info.get("reward_breakdown") or {}
        used_breakdown_keys: list[str] = []
        terms = {
            "reward_mode": self.reward_mode,
            "invalid_action_penalty": float(invalid_penalty),
            "terminal_victory_term": 0.0,
            "terminal_defeat_term": 0.0,
            "hp_loss_term": 0.0,
            "enemy_hp_delta_term": 0.0,
            "kill_bonus_term": 0.0,
            "next_enemy_window_relief_term": 0.0,
            "persistent_attack_script_relief_term": 0.0,
            "catastrophe_term": 0.0,
            "used_breakdown_keys": used_breakdown_keys,
        }

        outcome = str(info.get("outcome") or "")
        if outcome == "victory":
            terms["terminal_victory_term"] = float(self.reward_config["victory_reward"])
        elif outcome == "defeat":
            terms["terminal_defeat_term"] = float(self.reward_config["defeat_reward"])

        if "player_hp_delta" in breakdown:
            used_breakdown_keys.append("player_hp_delta")
        terms["hp_loss_term"] = float(breakdown.get("player_hp_delta") or 0.0) * float(
            self.reward_config["hp_loss_scale"]
        )
        if "enemy_hp_delta" in breakdown:
            used_breakdown_keys.append("enemy_hp_delta")
        terms["enemy_hp_delta_term"] = float(breakdown.get("enemy_hp_delta") or 0.0) * float(
            self.reward_config["enemy_hp_delta_scale"]
        )
        if "kill_bonus" in breakdown:
            used_breakdown_keys.append("kill_bonus")
        terms["kill_bonus_term"] = float(breakdown.get("kill_bonus") or 0.0) * float(
            self.reward_config["kill_bonus_scale"]
        )

        if self.reward_mode == "minimal_rl":
            if "next_enemy_window_relief" in breakdown:
                used_breakdown_keys.append("next_enemy_window_relief")
            terms["next_enemy_window_relief_term"] = float(
                breakdown.get("next_enemy_window_relief") or 0.0
            ) * float(self.reward_config["next_enemy_window_relief_scale"])
        elif self.reward_mode == "hexaghost_disarm_credit_v1":
            if "persistent_attack_script_relief" in breakdown:
                used_breakdown_keys.append("persistent_attack_script_relief")
            terms["persistent_attack_script_relief_term"] = float(
                breakdown.get("persistent_attack_script_relief") or 0.0
            ) * float(self.reward_config["persistent_attack_script_relief_scale"])

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
            + terms["persistent_attack_script_relief_term"]
            + terms["catastrophe_term"]
        )
        return terms

    def _compute_reward(self, response: dict[str, Any], info: dict[str, Any], invalid_penalty: float) -> float:
        if self.reward_mode == "legacy":
            return float(response.get("reward") or 0.0) + invalid_penalty + self._bucket_shaping_bonus(info)

        if self.reward_mode not in {"minimal_rl", "hexaghost_disarm_credit_v1"}:
            raise RuntimeError(f"unsupported reward_mode '{self.reward_mode}'")
        return float(self._effective_reward_terms(info, invalid_penalty)["total_effective_reward"])

    def _encode_observation(self, response: dict[str, Any]) -> np.ndarray:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        pressure = obs.get("pressure") or {}
        belief = obs.get("belief") or {}
        values: list[float] = [
            float(obs.get("player_hp") or 0),
            float(obs.get("player_max_hp") or 0),
            float(obs.get("player_block") or 0),
            float(obs.get("energy") or 0),
            float(obs.get("turn_count") or 0),
            float(obs.get("draw_count") or 0),
            float(obs.get("discard_count") or 0),
            float(obs.get("exhaust_count") or 0),
            float(len(obs.get("hand") or [])),
            float(len(obs.get("monsters") or [])),
            float(pressure.get("visible_incoming") or 0),
            float(pressure.get("visible_unblocked") or 0),
            float(pressure.get("value_incoming") or 0),
            float(pressure.get("value_unblocked") or 0),
            float(pressure.get("survival_guard_incoming") or 0),
            float(pressure.get("survival_guard_unblocked") or 0),
            float(belief.get("expected_incoming_damage") or 0.0),
            float(belief.get("max_incoming_damage") or 0),
            float(PENDING_CHOICE_KINDS.get(obs.get("pending_choice_kind"), 0)),
        ]
        hexaghost_future_script = obs.get("hexaghost_future_script") or {}
        windows = list(hexaghost_future_script.get("windows") or [])
        values.extend(
            [
                float(hexaghost_future_script.get("future_raw_damage_total") or 0),
                float(hexaghost_future_script.get("future_multihit_raw_damage_total") or 0),
                float(hexaghost_future_script.get("future_attack_windows") or 0),
                float(hexaghost_future_script.get("future_inferno_windows") or 0),
            ]
        )
        for window in windows[:MAX_HEXAGHOST_WINDOWS]:
            values.extend(
                [
                    float(window.get("total_raw_damage") or 0),
                    float(window.get("damage_per_hit") or 0),
                    float(window.get("hits") or 0),
                    _bool(window.get("is_multihit")),
                    float(HEXAGHOST_MOVE_IDS.get(str(window.get("move_kind") or "Unknown"), 0)),
                ]
            )
        values.extend([0.0] * ((MAX_HEXAGHOST_WINDOWS * 5) - (len(values) - 23)))

        monster_start = len(values)
        for monster in list(obs.get("monsters") or [])[:MAX_MONSTERS]:
            max_hp = max(float(monster.get("max_hp") or 0), 1.0)
            values.extend(
                [
                    float(monster.get("current_hp") or 0),
                    float(monster.get("block") or 0),
                    float(monster.get("current_hp") or 0) / max_hp,
                    float(monster.get("belief_expected_incoming") or 0.0),
                    float(monster.get("belief_max_incoming") or 0),
                    _stable_token(str(monster.get("name") or ""), CARD_HASH_BUCKETS),
                    _stable_token(str(monster.get("visible_intent") or ""), ACTION_HASH_BUCKETS),
                    float({"visible": 1, "inferred": 2, "unknown": 0}.get(str(monster.get("belief_certainty") or "").lower(), 0)),
                ]
            )
        values.extend([0.0] * ((MAX_MONSTERS * 8) - (len(values) - monster_start)))

        hand_start = len(values)
        for card in list(obs.get("hand") or [])[:MAX_HAND]:
            semantics_vector = card_semantic_vector(
                card.get("card_id") or card.get("name"),
                cost_for_turn=card.get("cost_for_turn"),
                playable=card.get("playable"),
                upgraded=card.get("upgraded"),
            )
            values.extend(
                [
                    _stable_token(str(card.get("card_id") or card.get("name") or ""), CARD_HASH_BUCKETS),
                    float(card.get("cost_for_turn") or 0),
                    _bool(card.get("playable")),
                    _bool(card.get("upgraded")),
                    float(card.get("index") or 0),
                    *semantics_vector,
                ]
            )
        values.extend([0.0] * ((MAX_HAND * (5 + CARD_SEMANTIC_DIM)) - (len(values) - hand_start)))

        action_start = len(values)
        candidates = list(payload.get("action_candidates") or [])
        mask = list(payload.get("action_mask") or [])
        for index in range(min(len(candidates), MAX_ACTIONS)):
            candidate = candidates[index]
            semantics_vector = card_semantic_vector(candidate.get("card_name") or candidate.get("label"))
            values.extend(
                [
                    _bool(mask[index] if index < len(mask) else False),
                    float(ACTION_FAMILY_IDS.get(str(candidate.get("action_family") or ""), 0)),
                    _stable_token(str(candidate.get("card_name") or candidate.get("label") or ""), ACTION_HASH_BUCKETS),
                    float(candidate.get("slot_index") if candidate.get("slot_index") is not None else -1),
                    float(candidate.get("target") if candidate.get("target") is not None else -1),
                    float(index),
                    *semantics_vector,
                ]
            )
        values.extend([0.0] * ((MAX_ACTIONS * (6 + CARD_SEMANTIC_DIM)) - (len(values) - action_start)))
        return np.asarray(values, dtype=np.float32)

    def _info_from_response(self, response: dict[str, Any], invalid_action: bool) -> dict[str, Any]:
        payload = response.get("payload") or {}
        obs = payload.get("observation") or {}
        pressure = obs.get("pressure") or {}
        spec_name = response.get("spec_name") or obs.get("env_name") or (self._current_spec.stem if self._current_spec else None)
        logged_reward_breakdown = response.get("reward_breakdown") or {}
        return {
            "spec_name": spec_name,
            "curriculum_tag": curriculum_tag_from_spec_name(Path(spec_name).stem if spec_name else "unknown"),
            "reward_breakdown": logged_reward_breakdown,
            "logged_reward_breakdown": logged_reward_breakdown,
            "diagnostic_only_breakdown_keys": sorted(
                key for key in logged_reward_breakdown.keys() if key in DIAGNOSTIC_ONLY_BREAKDOWN_KEYS
            ),
            "chosen_action_label": response.get("chosen_action_label"),
            "outcome": response.get("outcome"),
            "invalid_action": invalid_action,
            "legal_action_count": payload.get("legal_action_count"),
            "player_hp": obs.get("player_hp"),
            "player_max_hp": obs.get("player_max_hp"),
            "player_block": obs.get("player_block"),
            "energy": obs.get("energy"),
            "turn_count": obs.get("turn_count"),
            "visible_incoming": (pressure.get("visible_incoming") or 0),
            "visible_unblocked": (pressure.get("visible_unblocked") or 0),
            "value_incoming": (pressure.get("value_incoming") or 0),
            "value_unblocked": (pressure.get("value_unblocked") or 0),
            "action_candidates": payload.get("action_candidates") or [],
            "hand_cards": [str(card.get("name") or card.get("card_id") or "") for card in (obs.get("hand") or [])],
            "monster_states": obs.get("monsters") or [],
            "hexaghost_future_script": obs.get("hexaghost_future_script"),
        }


def discover_spec_paths(spec_dir: Path | None = None) -> list[Path]:
    root = spec_dir or (REPO_ROOT / "data" / "combat_lab" / "specs")
    return sorted(root.glob("*.json"))
