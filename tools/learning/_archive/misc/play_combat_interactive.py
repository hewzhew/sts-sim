#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_REWARD_CONFIG = {
    "victory_reward": 1.0,
    "defeat_reward": -1.0,
    "hp_loss_scale": 0.02,
    "catastrophe_unblocked_threshold": 18.0,
    "catastrophe_penalty": 0.25,
    "next_enemy_window_relief_scale": 0.0,
    "persistent_attack_script_relief_scale": 0.0,
}


def find_release_binary(explicit: Path | None, name: str) -> Path:
    if explicit and explicit.exists():
        return explicit
    exe = REPO_ROOT / "target" / "release" / f"{name}.exe"
    if exe.exists():
        return exe
    plain = REPO_ROOT / "target" / "release" / name
    if plain.exists():
        return plain
    raise SystemExit(f"missing release binary '{name}'; run cargo build --release --bin {name} first")


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


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def resolve_reward_setup(config_path: Path | None) -> tuple[str | None, dict[str, float]]:
    reward_config = dict(DEFAULT_REWARD_CONFIG)
    reward_mode = None
    if config_path is not None:
        config = load_config(config_path)
        reward_mode = str(config.get("reward_mode") or "minimal_rl")
        reward_config.update({key: float(value) for key, value in (config.get("reward") or {}).items()})
    return reward_mode, reward_config


def compute_effective_reward_terms(
    response: dict[str, Any],
    reward_mode: str | None,
    reward_config: dict[str, float],
) -> dict[str, Any] | None:
    if reward_mode not in {"minimal_rl", "hexaghost_disarm_credit_v1"}:
        return None
    payload = response.get("payload") or {}
    obs = payload.get("observation") or {}
    pressure = obs.get("pressure") or {}
    breakdown = response.get("reward_breakdown") or {}
    outcome = str(response.get("outcome") or "")
    terms: dict[str, Any] = {
        "reward_mode": reward_mode,
        "terminal_victory_term": 0.0,
        "terminal_defeat_term": 0.0,
        "hp_loss_term": float(breakdown.get("player_hp_delta") or 0.0) * float(reward_config["hp_loss_scale"]),
        "next_enemy_window_relief_term": 0.0,
        "persistent_attack_script_relief_term": 0.0,
        "catastrophe_term": 0.0,
        "used_breakdown_keys": ["player_hp_delta"] if "player_hp_delta" in breakdown else [],
    }
    if outcome == "victory":
        terms["terminal_victory_term"] = float(reward_config["victory_reward"])
    elif outcome == "defeat":
        terms["terminal_defeat_term"] = float(reward_config["defeat_reward"])
    if reward_mode == "minimal_rl":
        if "next_enemy_window_relief" in breakdown:
            terms["used_breakdown_keys"].append("next_enemy_window_relief")
        terms["next_enemy_window_relief_term"] = float(breakdown.get("next_enemy_window_relief") or 0.0) * float(
            reward_config["next_enemy_window_relief_scale"]
        )
    elif reward_mode == "hexaghost_disarm_credit_v1":
        if "persistent_attack_script_relief" in breakdown:
            terms["used_breakdown_keys"].append("persistent_attack_script_relief")
        terms["persistent_attack_script_relief_term"] = float(
            breakdown.get("persistent_attack_script_relief") or 0.0
        ) * float(reward_config["persistent_attack_script_relief_scale"])
    if float(pressure.get("visible_unblocked") or 0.0) >= float(reward_config["catastrophe_unblocked_threshold"]):
        terms["catastrophe_term"] = -float(reward_config["catastrophe_penalty"])
    terms["total_effective_reward"] = float(
        terms["terminal_victory_term"]
        + terms["terminal_defeat_term"]
        + terms["hp_loss_term"]
        + terms["next_enemy_window_relief_term"]
        + terms["persistent_attack_script_relief_term"]
        + terms["catastrophe_term"]
    )
    return terms


def hand_card_names(obs: dict[str, Any]) -> list[str]:
    return [str(card.get("name") or "") for card in (obs.get("hand") or [])]


def trace_row(
    *,
    start_spec: Path,
    seed_hint: int,
    reward_mode: str | None,
    step_index: int,
    action_index: int,
    response_before: dict[str, Any],
    response_after: dict[str, Any],
    effective_terms: dict[str, Any] | None,
) -> dict[str, Any]:
    payload_before = response_before.get("payload") or {}
    obs_before = payload_before.get("observation") or {}
    pressure_before = obs_before.get("pressure") or {}
    script_before = obs_before.get("hexaghost_future_script") or {}
    windows_before = list(script_before.get("windows") or [])
    next_window_before = windows_before[0] if windows_before else None
    candidates_before = payload_before.get("action_candidates") or []
    mask_before = payload_before.get("action_mask") or []
    return {
        "recorded_at_utc": datetime.now(timezone.utc).isoformat(),
        "start_spec": str(start_spec),
        "seed_hint": int(seed_hint),
        "reward_mode": reward_mode,
        "step_index": int(step_index),
        "turn_count_before": int(obs_before.get("turn_count") or 0),
        "phase_before": obs_before.get("phase"),
        "engine_state_before": obs_before.get("engine_state"),
        "player_hp_before": obs_before.get("player_hp"),
        "player_block_before": obs_before.get("player_block"),
        "energy_before": obs_before.get("energy"),
        "hand_cards_before": hand_card_names(obs_before),
        "raw_visible_incoming_before": pressure_before.get("visible_incoming"),
        "raw_visible_unblocked_before": pressure_before.get("visible_unblocked"),
        "effective_next_script_before": next_window_before,
        "legal_actions_before": [
            {
                "action_index": idx,
                "legal": bool(mask_before[idx]) if idx < len(mask_before) else False,
                "label": candidate.get("label"),
                "action_family": candidate.get("action_family"),
                "hand_slot": candidate.get("slot_index"),
                "target": candidate.get("target"),
            }
            for idx, candidate in enumerate(candidates_before)
        ],
        "chosen_action_index": int(action_index),
        "chosen_action_label": response_after.get("chosen_action_label"),
        "outcome_after": response_after.get("outcome"),
        "raw_driver_reward_after": response_after.get("reward"),
        "reward_breakdown_after": response_after.get("reward_breakdown"),
        "effective_reward_terms_after": effective_terms,
    }


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def format_monsters(
    monsters: list[dict[str, Any]],
    hexaghost_next_window: dict[str, Any] | None = None,
) -> list[str]:
    lines: list[str] = []
    for idx, monster in enumerate(monsters):
        hp = monster.get("current_hp")
        max_hp = monster.get("max_hp")
        block = monster.get("block") or 0
        name = monster.get("name") or f"monster_{idx}"
        intent = monster.get("visible_intent") or "Unknown"
        line = f"{idx}. {name} HP {hp}/{max_hp} Block {block}"
        if idx == 0 and hexaghost_next_window:
            line += (
                " EffectiveNextIntent "
                + f"{hexaghost_next_window.get('move_kind')} "
                + "{ "
                + f"damage: {hexaghost_next_window.get('damage_per_hit')}, "
                + f"hits: {hexaghost_next_window.get('hits')}, "
                + f"total: {hexaghost_next_window.get('total_raw_damage')}"
                + " }"
            )
            if intent and intent != "Unknown":
                line += f" [raw intent chain: {intent}]"
        else:
            line += f" Intent {intent}"
        lines.append(line)
    return lines


def format_hand(hand: list[dict[str, Any]]) -> list[str]:
    lines: list[str] = []
    for card in hand:
        idx = card.get("index")
        name = card.get("name")
        cost = card.get("cost_for_turn")
        playable = "Y" if card.get("playable") else "N"
        lines.append(f"slot {idx}. {name} (cost {cost}, playable {playable})")
    return lines


def format_actions(candidates: list[dict[str, Any]], mask: list[bool]) -> list[str]:
    lines: list[str] = []
    for idx, candidate in enumerate(candidates):
        legal = "Y" if idx < len(mask) and mask[idx] else "N"
        family = candidate.get("action_family") or "unknown"
        label = candidate.get("label") or ""
        slot_index = candidate.get("slot_index")
        slot_text = f" hand_slot={slot_index}" if slot_index is not None else ""
        lines.append(f"action {idx:>2} [{legal}] {family:<12} {label}{slot_text}")
    return lines


def describe_hexaghost_script(script: dict[str, Any]) -> tuple[str | None, list[str]]:
    windows = list(script.get("windows") or [])[:4]
    if not windows:
        return None, []
    next_window = windows[0]
    summary = (
        f"next scripted attack t+{next_window.get('enemy_turn_index')} "
        f"{next_window.get('move_kind')} total={next_window.get('total_raw_damage')} "
        f"(hits={next_window.get('hits')} dmg/hit={next_window.get('damage_per_hit')})"
    )
    lines: list[str] = []
    for window in windows:
        lines.append(
            f"t+{window.get('enemy_turn_index')} {window.get('move_kind')} "
            f"hits={window.get('hits')} dmg/hit={window.get('damage_per_hit')} "
            f"total={window.get('total_raw_damage')}"
        )
    return summary, lines


def print_state(response: dict[str, Any], step_index: int, reward_mode: str | None) -> None:
    payload = response.get("payload") or {}
    obs = payload.get("observation") or {}
    pressure = obs.get("pressure") or {}
    script = obs.get("hexaghost_future_script") or {}
    script_summary, script_lines = describe_hexaghost_script(script)
    script_windows = list(script.get("windows") or [])
    next_script_window = script_windows[0] if script_windows else None
    effective_scripted_incoming = int(next_script_window.get("total_raw_damage") or 0) if next_script_window else None
    player_block = int(obs.get("player_block") or 0)
    effective_scripted_unblocked = (
        max(effective_scripted_incoming - player_block, 0) if effective_scripted_incoming is not None else None
    )
    print()
    print("=" * 72)
    print(
        f"ActionStep {step_index} | Turn {obs.get('turn_count')} | {obs.get('env_name')} "
        f"| phase={obs.get('phase')} state={obs.get('engine_state')}"
    )
    print(
        f"Player HP {obs.get('player_hp')}/{obs.get('player_max_hp')} "
        f"Block {obs.get('player_block')} Energy {obs.get('energy')} "
        f"Draw {obs.get('draw_count')} Discard {obs.get('discard_count')} Exhaust {obs.get('exhaust_count')}"
    )
    if effective_scripted_incoming is not None:
        print(
            f"Effective incoming={effective_scripted_incoming} "
            f"Effective unblocked={effective_scripted_unblocked}"
        )
        print(
            f"Raw intent-chain incoming={pressure.get('visible_incoming', 0)} "
            f"Raw intent-chain unblocked={pressure.get('visible_unblocked', 0)} "
            f"Raw value_incoming={pressure.get('value_incoming', 0)}"
        )
        if script_summary:
            print(f"Effective scripted view: {script_summary}")
    else:
        print(
            f"Incoming={pressure.get('visible_incoming', 0)} "
            f"Unblocked={pressure.get('visible_unblocked', 0)} "
            f"ValueIncoming={pressure.get('value_incoming', 0)}"
        )
    print("Monsters:")
    for line in format_monsters(obs.get("monsters") or [], next_script_window):
        print(f"  {line}")
    print("Hand:")
    for line in format_hand(obs.get("hand") or []):
        print(f"  {line}")
    if script_lines:
        print("Hexaghost future script:")
        for line in script_lines:
            print(f"  {line}")
    print("Legal actions:")
    for line in format_actions(payload.get("action_candidates") or [], payload.get("action_mask") or []):
        print(f"  {line}")
    print("Commands: enter action index, `r` to refresh, `q` to quit")
    if reward_mode:
        print(f"Reward mode context: {reward_mode}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Minimal interactive terminal player for combat_env_driver.")
    parser.add_argument("--start-spec", required=True, type=Path)
    parser.add_argument("--seed", default=0, type=int)
    parser.add_argument("--driver-binary", default=None, type=Path)
    parser.add_argument("--config", default=None, type=Path)
    parser.add_argument("--show-effective-reward", action="store_true")
    parser.add_argument("--show-raw-reward", action="store_true")
    parser.add_argument("--save-trace", default=None, type=Path)
    args = parser.parse_args()

    start_spec = args.start_spec
    if not start_spec.is_absolute():
        start_spec = (REPO_ROOT / start_spec).resolve()
    config_path = args.config
    if config_path is not None and not config_path.is_absolute():
        config_path = (REPO_ROOT / config_path).resolve()
    reward_mode, reward_config = resolve_reward_setup(config_path)
    save_trace_path = args.save_trace
    if save_trace_path is not None and not save_trace_path.is_absolute():
        save_trace_path = (REPO_ROOT / save_trace_path).resolve()
    trace_rows: list[dict[str, Any]] = []

    driver = CombatEnvDriver(args.driver_binary)
    try:
        response = driver.request(
            {
                "cmd": "reset",
                "start_spec": str(start_spec),
                "seed_hint": int(args.seed),
            }
        )
        step_index = 0
        while True:
            print_state(response, step_index, reward_mode if args.show_effective_reward else None)
            user = input("> ").strip()
            if user.lower() in {"q", "quit", "exit"}:
                print("quit")
                return 0
            if user.lower() in {"r", "refresh"}:
                response = driver.request({"cmd": "observation"})
                continue
            if not user.isdigit():
                print("enter a legal action index, `r`, or `q`")
                continue
            action_index = int(user)
            payload = response.get("payload") or {}
            action_mask = list(payload.get("action_mask") or [])
            if action_index >= len(action_mask):
                print(f"action {action_index} is out of range for {len(action_mask)} candidates")
                continue
            if not action_mask[action_index]:
                print(f"action {action_index} is currently illegal; choose a [Y] action")
                continue
            response_before = response
            response = driver.request({"cmd": "step", "action_index": action_index})
            step_index += 1
            reward_breakdown = response.get("reward_breakdown") or {}
            effective_terms = (
                compute_effective_reward_terms(response, reward_mode, reward_config)
                if args.show_effective_reward
                else None
            )
            if save_trace_path is not None:
                trace_rows.append(
                    trace_row(
                        start_spec=start_spec,
                        seed_hint=int(args.seed),
                        reward_mode=reward_mode,
                        step_index=step_index,
                        action_index=action_index,
                        response_before=response_before,
                        response_after=response,
                        effective_terms=effective_terms
                        if effective_terms is not None
                        else compute_effective_reward_terms(response, reward_mode, reward_config),
                    )
                )
            summary_parts = [f"chosen={response.get('chosen_action_label')}", f"outcome={response.get('outcome')}"]
            if args.show_effective_reward and effective_terms is not None:
                summary_parts.append(f"effective_reward={effective_terms.get('total_effective_reward')}")
            if args.show_raw_reward:
                summary_parts.append(f"raw_driver_reward={response.get('reward')}")
            print(" ".join(summary_parts))
            if args.show_effective_reward and effective_terms is not None:
                print(
                    "effective_terms: "
                    + ", ".join(
                        [
                            f"hp_loss={effective_terms['hp_loss_term']}",
                            f"next_window={effective_terms['next_enemy_window_relief_term']}",
                            f"persistent_script={effective_terms['persistent_attack_script_relief_term']}",
                            f"catastrophe={effective_terms['catastrophe_term']}",
                        ]
                    )
                )
            if args.show_raw_reward:
                idle_penalty = reward_breakdown.get("idle_penalty")
                if idle_penalty not in (None, 0, 0.0):
                    print(
                        "logged_breakdown contains diagnostic fields such as "
                        f"idle_penalty={idle_penalty}; raw_driver_reward may include them."
                    )
            if response.get("done"):
                print("episode finished")
                print_state(response, step_index, reward_mode if args.show_effective_reward else None)
                return 0
    finally:
        driver.close()
        if save_trace_path is not None:
            write_jsonl(save_trace_path, trace_rows)


if __name__ == "__main__":
    raise SystemExit(main())
