#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import time
from collections import Counter, defaultdict
from datetime import datetime, timezone
from pathlib import Path
from statistics import mean
from typing import Any

from sb3_contrib import MaskablePPO

from combat_rl_common import REPO_ROOT, find_release_binary, write_json
from full_run_candidate_policy import FullRunCandidateScorerPolicy  # noqa: F401
from full_run_env import FullRunGymEnv


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Evaluate full-run policies with capability-oriented metrics, not just floor reached."
    )
    parser.add_argument("--episodes", type=int, default=20)
    parser.add_argument("--seed", type=int, default=50000)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--class", dest="player_class", default="ironclad")
    parser.add_argument("--final-act", action="store_true")
    parser.add_argument("--max-steps", type=int, default=5000)
    parser.add_argument("--policies", default="random_masked,rule_baseline_v0")
    parser.add_argument("--model", type=Path)
    parser.add_argument("--model-name", default="ppo_model")
    parser.add_argument("--driver-binary", type=Path)
    parser.add_argument("--sts-dev-tool-binary", type=Path)
    parser.add_argument("--artifact-dir", type=Path)
    parser.add_argument("--out", type=Path)
    parser.add_argument("--keep-traces", action="store_true")
    parser.add_argument(
        "--reward-shaping-profile",
        choices=["baseline", "plan_deficit_v0"],
        default="baseline",
    )
    parser.add_argument("--feature-profile", choices=["baseline", "plan_v0"], default="baseline")
    parser.add_argument(
        "--plan-query-report",
        action="append",
        default=[],
        metavar="POLICY=PATH",
        help="Optional combat_plan_query_batch_report.json for a policy, used as eval signal only.",
    )
    return parser.parse_args()


def parse_policy_list(text: str) -> list[str]:
    policies = [part.strip() for part in str(text or "").split(",") if part.strip()]
    if not policies:
        raise SystemExit("expected at least one policy")
    allowed = {"random_masked", "rule_baseline_v0", "model"}
    unknown = [policy for policy in policies if policy not in allowed]
    if unknown:
        raise SystemExit(f"unknown policy names: {unknown}; allowed={sorted(allowed)}")
    return policies


def run_rust_policy(args: argparse.Namespace, policy: str, artifact_dir: Path) -> dict[str, Any]:
    binary = find_release_binary(args.sts_dev_tool_binary, "sts_dev_tool")
    policy_dir = artifact_dir / policy
    trace_dir = policy_dir / "traces"
    summary_path = policy_dir / "summary.json"
    policy_dir.mkdir(parents=True, exist_ok=True)
    cmd = [
        str(binary),
        "run-batch",
        "--episodes",
        str(args.episodes),
        "--seed",
        str(args.seed),
        "--policy",
        policy,
        "--ascension",
        str(args.ascension),
        "--class",
        args.player_class,
        "--max-steps",
        str(args.max_steps),
        "--determinism-check",
        "--reward-shaping-profile",
        args.reward_shaping_profile,
        "--summary-out",
        str(summary_path),
        "--trace-dir",
        str(trace_dir),
    ]
    if args.final_act:
        cmd.append("--final-act")

    start = time.perf_counter()
    proc = subprocess.run(
        cmd,
        cwd=str(REPO_ROOT),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    elapsed = time.perf_counter() - start
    if proc.returncode != 0:
        raise RuntimeError(f"{policy} run-batch failed with code {proc.returncode}: {proc.stderr}")

    trace_files = sorted(trace_dir.glob("episode_*.json"))
    episodes = [episode_from_trace_file(path) for path in trace_files]
    summary = summarize_policy(policy, episodes, elapsed)
    summary["source"] = {
        "kind": "rust_run_batch_trace",
        "summary_path": str(summary_path),
        "trace_dir": str(trace_dir),
        "traces_kept": bool(args.keep_traces),
    }
    if not args.keep_traces:
        # Keep the JSON summary and final capability report, but avoid large trace buildup.
        for path in trace_files:
            path.unlink(missing_ok=True)
        try:
            trace_dir.rmdir()
        except OSError:
            pass
    return summary


def episode_from_trace_file(path: Path) -> dict[str, Any]:
    data = json.loads(path.read_text(encoding="utf-8"))
    summary = data.get("summary") or {}
    steps = []
    for raw in data.get("steps") or []:
        observation = raw.get("observation") or {}
        chosen_candidate = {}
        candidates = raw.get("action_mask") or []
        index = int(raw.get("chosen_action_index") or 0)
        if 0 <= index < len(candidates) and isinstance(candidates[index], dict):
            chosen_candidate = candidates[index]
        steps.append(
            {
                "step": int(raw.get("step_index") or 0),
                "floor": int(raw.get("floor") or observation.get("floor") or 0),
                "act": int(raw.get("act") or observation.get("act") or 0),
                "observation": observation,
                "decision_type": str(raw.get("decision_type") or observation.get("decision_type") or "unknown"),
                "engine_state": str(raw.get("engine_state") or observation.get("engine_state") or "unknown"),
                "chosen_action_key": str(raw.get("chosen_action_key") or chosen_candidate.get("action_key") or ""),
                "chosen_action": raw.get("chosen_action") or chosen_candidate.get("action") or {},
                "chosen_candidate": chosen_candidate,
                "action_mask": candidates,
            }
        )
    return {"summary": summary, "steps": steps}


def run_model_policy(args: argparse.Namespace, artifact_dir: Path) -> dict[str, Any]:
    if args.model is None:
        raise SystemExit("--model is required when policies includes 'model'")
    model = MaskablePPO.load(str(args.model))
    policy_dir = artifact_dir / args.model_name
    trace_dir = policy_dir / "traces"
    if args.keep_traces:
        trace_dir.mkdir(parents=True, exist_ok=True)
    env = FullRunGymEnv(
        driver_binary=args.driver_binary,
        seed=args.seed,
        ascension=args.ascension,
        final_act=args.final_act,
        player_class=args.player_class,
        max_episode_steps=args.max_steps,
        reward_shaping_profile=args.reward_shaping_profile,
        feature_profile=args.feature_profile,
    )
    episodes: list[dict[str, Any]] = []
    start = time.perf_counter()
    try:
        for episode_index in range(args.episodes):
            run_seed = args.seed + episode_index
            obs, info = env.reset(options={"run_seed": run_seed, "max_steps": args.max_steps})
            done = False
            truncated = False
            steps: list[dict[str, Any]] = []
            reward_total = 0.0
            while not done and not truncated:
                action_masks = env.action_masks()
                action, _ = model.predict(obs, deterministic=True, action_masks=action_masks)
                action = int(action)
                candidate = candidate_at(info, action)
                raw_payload = info.get("raw_payload") or {}
                observation = raw_payload.get("observation") or {}
                steps.append(
                    {
                        "step": len(steps),
                        "step_index": len(steps),
                        "floor": int(info.get("floor") or observation.get("floor") or 0),
                        "act": int(info.get("act") or observation.get("act") or 0),
                        "hp": int(info.get("current_hp") or observation.get("current_hp") or 0),
                        "max_hp": int(info.get("max_hp") or observation.get("max_hp") or 0),
                        "gold": int(observation.get("gold") or 0),
                        "deck_size": int(info.get("deck_size") or observation.get("deck_size") or 0),
                        "relic_count": int(info.get("relic_count") or observation.get("relic_count") or 0),
                        "legal_action_count": int(info.get("legal_action_count") or len(info.get("action_candidates") or [])),
                        "observation": observation,
                        "decision_type": str(info.get("decision_type") or observation.get("decision_type") or "unknown"),
                        "engine_state": str(info.get("engine_state") or observation.get("engine_state") or "unknown"),
                        "chosen_action_index": action,
                        "chosen_action_key": str(candidate.get("action_key") or ""),
                        "chosen_action": candidate.get("action") or {},
                        "chosen_candidate": candidate,
                        "action_mask": info.get("action_candidates") or [],
                    }
                )
                obs, reward, done, truncated, info = env.step(action)
                reward_total += float(reward)
            summary = {
                "episode_id": episode_index,
                "seed": run_seed,
                "result": "truncated" if truncated and not done else info.get("result"),
                "terminal_reason": "python_truncated"
                if truncated and not done
                else info.get("terminal_reason"),
                "floor": int(info.get("floor") or 0),
                "act": int(info.get("act") or 0),
                "steps": len(steps),
                "total_reward": reward_total,
                "combat_win_count": int(info.get("combat_win_count") or 0),
                "crash": info.get("crash"),
                "illegal_actions": 0,
                "hp": int(info.get("current_hp") or 0),
                "max_hp": int(info.get("max_hp") or 0),
                "deck_size": int(info.get("deck_size") or 0),
                "relic_count": int(info.get("relic_count") or 0),
            }
            episode = {"summary": summary, "steps": steps}
            if args.keep_traces:
                trace_path = trace_dir / f"episode_{episode_index:04}_seed_{run_seed}.json"
                summary["trace_path"] = str(trace_path)
                write_full_run_trace(
                    trace_path=trace_path,
                    args=args,
                    policy=args.model_name,
                    seed=run_seed,
                    summary=summary,
                    steps=steps,
                )
            episodes.append(episode)
    finally:
        env.close()
    elapsed = time.perf_counter() - start
    summary = summarize_policy(args.model_name, episodes, elapsed)
    summary["source"] = {
        "kind": "python_maskable_ppo",
        "model": str(args.model),
        "trace_dir": str(trace_dir) if args.keep_traces else None,
        "traces_kept": bool(args.keep_traces),
    }
    return summary


def write_full_run_trace(
    *,
    trace_path: Path,
    args: argparse.Namespace,
    policy: str,
    seed: int,
    summary: dict[str, Any],
    steps: list[dict[str, Any]],
) -> None:
    trace_steps = []
    for step in steps:
        trace_steps.append(
            {
                "step_index": int(step.get("step_index") or step.get("step") or 0),
                "floor": int(step.get("floor") or 0),
                "act": int(step.get("act") or 0),
                "engine_state": str(step.get("engine_state") or "unknown"),
                "decision_type": str(step.get("decision_type") or "unknown"),
                "hp": int(step.get("hp") or 0),
                "max_hp": int(step.get("max_hp") or 0),
                "gold": int(step.get("gold") or 0),
                "deck_size": int(step.get("deck_size") or 0),
                "relic_count": int(step.get("relic_count") or 0),
                "legal_action_count": int(step.get("legal_action_count") or 0),
                "observation": step.get("observation") or {},
                "action_mask": step.get("action_mask") or [],
                "chosen_action_index": int(step.get("chosen_action_index") or 0),
                "chosen_action_key": str(step.get("chosen_action_key") or ""),
                "chosen_action": step.get("chosen_action") or {},
            }
        )
    write_json(
        trace_path,
        {
            "observation_schema_version": "full_run_observation_v3",
            "action_schema_version": "full_run_action_candidate_set_v1",
            "config": {
                "seed": seed,
                "ascension": args.ascension,
                "final_act": bool(args.final_act),
                "player_class": canonical_player_class(args.player_class),
                "max_steps": args.max_steps,
                "policy": policy,
                "source": "python_maskable_ppo",
            },
            "summary": summary,
            "steps": trace_steps,
        },
    )


def canonical_player_class(player_class: str) -> str:
    normalized = str(player_class or "").strip().lower()
    return {
        "ironclad": "Ironclad",
        "red": "Ironclad",
        "silent": "Silent",
        "green": "Silent",
        "defect": "Defect",
        "blue": "Defect",
        "watcher": "Watcher",
        "purple": "Watcher",
    }.get(normalized, player_class)


def candidate_at(info: dict[str, Any], index: int) -> dict[str, Any]:
    candidates = list(info.get("action_candidates") or [])
    if 0 <= index < len(candidates) and isinstance(candidates[index], dict):
        return candidates[index]
    return {}


def summarize_policy(policy: str, episodes: list[dict[str, Any]], elapsed: float) -> dict[str, Any]:
    elapsed = max(elapsed, 1e-6)
    summaries = [episode.get("summary") or {} for episode in episodes]
    steps = [step for episode in episodes for step in episode.get("steps") or []]
    action_type_counts = Counter(action_type(step.get("chosen_action") or {}) for step in steps)
    action_prefix_counts = Counter(action_prefix(step.get("chosen_action_key") or "") for step in steps)
    decision_type_counts = Counter(str(step.get("decision_type") or "unknown") for step in steps)
    floors = [int(summary.get("floor") or 0) for summary in summaries]
    rewards = [float(summary.get("total_reward") or summary.get("reward_total") or 0.0) for summary in summaries]
    combat_wins = [int(summary.get("combat_win_count") or 0) for summary in summaries]
    crashes = sum(1 for summary in summaries if summary.get("crash"))
    illegal = sum(int(summary.get("illegal_actions") or 0) for summary in summaries)
    truncated = sum(1 for summary in summaries if summary.get("terminal_reason") == "python_truncated")

    combat = combat_capabilities(episodes)
    rewards_summary = reward_capabilities(steps)
    macro = macro_capabilities(steps)
    deck = deck_capabilities(episodes)
    boss = boss_capabilities(episodes)
    diagnostics = diagnostic_flags(
        crashes=crashes,
        illegal=illegal,
        truncated=truncated,
        steps=steps,
        progression_floor=safe_mean(floors),
        combat=combat,
        boss=boss,
        deck=deck,
        action_type_counts=action_type_counts,
        decision_type_counts=decision_type_counts,
        rewards_summary=rewards_summary,
        macro=macro,
    )

    return {
        "policy": policy,
        "episodes": len(episodes),
        "generated_step_count": len(steps),
        "wall_seconds": elapsed,
        "steps_per_second": len(steps) / elapsed,
        "contract": {
            "crash_count": crashes,
            "illegal_action_count": illegal,
            "python_truncated_count": truncated,
            "result_counts": dict(Counter(str(summary.get("result") or "unknown") for summary in summaries)),
            "terminal_reason_counts": dict(
                Counter(str(summary.get("terminal_reason") or "unknown") for summary in summaries)
            ),
        },
        "progression": {
            "average_floor": safe_mean(floors),
            "median_floor": median(floors),
            "average_reward": safe_mean(rewards),
            "average_combat_wins": safe_mean(combat_wins),
            "combat_wins_per_floor": safe_ratio(sum(combat_wins), sum(max(floor, 1) for floor in floors)),
            "act_counts": dict(Counter(str(summary.get("act") or 0) for summary in summaries)),
        },
        "combat": combat,
        "boss": boss,
        "rewards": rewards_summary,
        "macro": macro,
        "deck": deck,
        "counts": {
            "decision_type_counts": dict(decision_type_counts),
            "action_type_counts": dict(action_type_counts),
            "action_key_prefix_counts": dict(action_prefix_counts),
        },
        "diagnostic_flags": diagnostics,
        "failure_examples": [
            summary
            for summary in summaries
            if summary.get("crash") or summary.get("terminal_reason") == "python_truncated"
        ][:5],
    }


def combat_capabilities(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    combat_steps = []
    combat_groups: dict[tuple[int, int], list[dict[str, Any]]] = defaultdict(list)
    potion_uses = 0
    end_turns = 0
    play_cards = 0
    attack_cards = 0
    skill_cards = 0
    power_cards = 0
    high_unblocked = 0
    lethal_pressure = 0
    low_hp_decisions = 0
    total_visible_unblocked = 0.0

    for episode_index, episode in enumerate(episodes):
        for step in episode.get("steps") or []:
            obs = step.get("observation") or {}
            combat = obs.get("combat") or {}
            if not isinstance(combat, dict) or not combat:
                continue
            combat_steps.append(step)
            combat_groups[(episode_index, int(obs.get("floor") or step.get("floor") or 0))].append(step)
            act = action_type(step.get("chosen_action") or {})
            if act == "use_potion":
                potion_uses += 1
            elif act == "end_turn":
                end_turns += 1
            elif act == "play_card":
                play_cards += 1
                card = (step.get("chosen_candidate") or {}).get("card") or {}
                card_type = int(card.get("card_type_id") or 0)
                if card_type == 1:
                    attack_cards += 1
                elif card_type == 2:
                    skill_cards += 1
                elif card_type == 3:
                    power_cards += 1
            hp = float(combat.get("player_hp") or obs.get("current_hp") or 0)
            max_hp = float(obs.get("max_hp") or 0)
            block = float(combat.get("player_block") or 0)
            incoming = float(combat.get("visible_incoming_damage") or 0)
            unblocked = max(0.0, incoming - block)
            total_visible_unblocked += unblocked
            if unblocked > 0:
                high_unblocked += 1
            if hp > 0 and unblocked >= hp:
                lethal_pressure += 1
            if max_hp > 0 and hp / max_hp <= 0.30:
                low_hp_decisions += 1

    hp_losses = []
    decision_counts = []
    for group_steps in combat_groups.values():
        if not group_steps:
            continue
        hps = [
            int((step.get("observation") or {}).get("current_hp") or 0)
            for step in group_steps
        ]
        decision_counts.append(len(group_steps))
        if hps:
            hp_losses.append(max(0, hps[0] - min(hps)))

    total = len(combat_steps)
    return {
        "combat_decision_count": total,
        "combat_encounter_floor_count": len(combat_groups),
        "average_decisions_per_combat_floor": safe_mean(decision_counts),
        "average_visible_hp_loss_per_combat_floor": safe_mean(hp_losses),
        "average_visible_unblocked_per_combat_decision": safe_ratio(total_visible_unblocked, total),
        "high_unblocked_decision_share": safe_ratio(high_unblocked, total),
        "lethal_pressure_decision_share": safe_ratio(lethal_pressure, total),
        "low_hp_combat_decision_share": safe_ratio(low_hp_decisions, total),
        "potion_uses_per_combat_floor": safe_ratio(potion_uses, len(combat_groups)),
        "play_card_share": safe_ratio(play_cards, total),
        "end_turn_share": safe_ratio(end_turns, total),
        "attack_card_share_of_played_cards": safe_ratio(attack_cards, play_cards),
        "skill_card_share_of_played_cards": safe_ratio(skill_cards, play_cards),
        "power_card_share_of_played_cards": safe_ratio(power_cards, play_cards),
    }


def boss_capabilities(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    entries = []
    boss_floors: set[tuple[int, int]] = set()
    for episode_index, episode in enumerate(episodes):
        seen: set[int] = set()
        for step in episode.get("steps") or []:
            obs = step.get("observation") or {}
            if obs.get("current_room") != "MonsterRoomBoss":
                continue
            combat = obs.get("combat") or {}
            if not combat:
                continue
            floor = int(obs.get("floor") or step.get("floor") or 0)
            boss_floors.add((episode_index, floor))
            if floor in seen:
                continue
            seen.add(floor)
            hp = float(obs.get("current_hp") or combat.get("player_hp") or 0)
            max_hp = float(obs.get("max_hp") or 0)
            entries.append({"floor": floor, "act": int(obs.get("act") or 0), "hp_ratio": safe_ratio(hp, max_hp)})
    by_act = defaultdict(list)
    for entry in entries:
        by_act[str(entry["act"])].append(entry["hp_ratio"])
    return {
        "boss_combat_entry_count": len(entries),
        "boss_combat_floor_count": len(boss_floors),
        "average_boss_entry_hp_ratio": safe_mean([entry["hp_ratio"] for entry in entries]),
        "average_boss_entry_hp_ratio_by_act": {
            act: safe_mean(values) for act, values in sorted(by_act.items())
        },
    }


def card_with_plan_score(candidate: dict[str, Any]) -> dict[str, Any]:
    card = dict(candidate.get("card") or {})
    plan_delta = candidate.get("plan_delta") or {}
    card["plan_adjusted_score"] = float(
        plan_delta.get("plan_adjusted_score", card.get("rule_score") or 0) or 0
    )
    return card


def reward_capabilities(steps: list[dict[str, Any]]) -> dict[str, Any]:
    card_choice_decisions = 0
    card_selects = 0
    card_skips = 0
    reward_claims = 0
    reward_proceeds = 0
    selected_rule_scores = []
    selected_card_types = Counter()
    skipped_good_offers = 0
    best_offer_rule_scores = []
    missed_best_gaps = []
    missed_best_gap_ge_30 = 0
    plan_adjusted_best_offer_scores = []
    plan_adjusted_missed_gaps = []
    plan_adjusted_gap_ge_30 = 0
    draw_card_offer_count = 0
    draw_card_select_count = 0
    draw_card_skip_count = 0
    scaling_card_offer_count = 0
    scaling_card_select_count = 0
    scaling_card_skip_count = 0

    for step in steps:
        decision = str(step.get("decision_type") or "")
        act = action_type(step.get("chosen_action") or {})
        candidate = step.get("chosen_candidate") or {}
        obs = step.get("observation") or {}
        screen = obs.get("screen") or {}
        if decision == "reward_card_choice":
            card_choice_decisions += 1
            cards = [
                card_with_plan_score(item)
                for item in step.get("action_mask", []) or []
                if isinstance(item, dict) and (item.get("card") or {})
            ]
            best_card = max(cards, key=lambda card: float(card.get("rule_score") or 0), default={})
            best_plan_card = max(
                cards,
                key=lambda card: float(card.get("plan_adjusted_score") or card.get("rule_score") or 0),
                default={},
            )
            best_score = float(best_card.get("rule_score") or 0)
            best_plan_score = float(
                best_plan_card.get("plan_adjusted_score") or best_plan_card.get("rule_score") or 0
            )
            best_offer_rule_scores.append(best_score)
            plan_adjusted_best_offer_scores.append(best_plan_score)
            offer_has_draw = any(bool(card.get("draws_cards")) for card in cards)
            offer_has_scaling = any(bool(card.get("scaling_piece")) for card in cards)
            if offer_has_draw:
                draw_card_offer_count += 1
            if offer_has_scaling:
                scaling_card_offer_count += 1

            if act == "select_card":
                card_selects += 1
                card = candidate.get("card") or {}
                plan_delta = candidate.get("plan_delta") or {}
                selected_score = float(card.get("rule_score") or 0)
                selected_plan_score = float(
                    plan_delta.get("plan_adjusted_score", selected_score) or selected_score
                )
                selected_rule_scores.append(selected_score)
                selected_card_types[str(card.get("card_type_id") or 0)] += 1
                if bool(card.get("draws_cards")):
                    draw_card_select_count += 1
                if bool(card.get("scaling_piece")):
                    scaling_card_select_count += 1
                gap = max(best_score - selected_score, 0.0)
                missed_best_gaps.append(gap)
                if gap >= 30:
                    missed_best_gap_ge_30 += 1
                plan_gap = max(best_plan_score - selected_plan_score, 0.0)
                plan_adjusted_missed_gaps.append(plan_gap)
                if plan_gap >= 30:
                    plan_adjusted_gap_ge_30 += 1
            elif act == "proceed":
                card_skips += 1
                if offer_has_draw:
                    draw_card_skip_count += 1
                if offer_has_scaling:
                    scaling_card_skip_count += 1
                if best_score >= 70:
                    skipped_good_offers += 1
                missed_best_gaps.append(max(best_score - 5.0, 0.0))
                if best_score - 5.0 >= 30:
                    missed_best_gap_ge_30 += 1
                plan_adjusted_missed_gaps.append(max(best_plan_score - 5.0, 0.0))
                if best_plan_score - 5.0 >= 30:
                    plan_adjusted_gap_ge_30 += 1
        if decision == "reward":
            if act == "claim_reward":
                reward_claims += 1
            elif act == "proceed":
                reward_proceeds += 1 if int(screen.get("reward_item_count") or 0) > 0 else 0

    return {
        "reward_card_choice_decisions": card_choice_decisions,
        "card_select_count": card_selects,
        "card_skip_count": card_skips,
        "card_skip_share": safe_ratio(card_skips, card_choice_decisions),
        "selected_card_rule_score_average": safe_mean(selected_rule_scores),
        "best_offer_rule_score_average": safe_mean(best_offer_rule_scores),
        "missed_best_rule_score_gap_average": safe_mean(missed_best_gaps),
        "missed_best_rule_score_gap_ge_30_count": missed_best_gap_ge_30,
        "plan_adjusted_best_offer_score_average": safe_mean(plan_adjusted_best_offer_scores),
        "plan_adjusted_missed_best_gap_average": safe_mean(plan_adjusted_missed_gaps),
        "plan_adjusted_missed_best_gap_ge_30_count": plan_adjusted_gap_ge_30,
        "selected_card_type_counts": dict(selected_card_types),
        "skipped_good_offer_count": skipped_good_offers,
        "draw_card_offer_count": draw_card_offer_count,
        "draw_card_select_count": draw_card_select_count,
        "draw_card_skip_count": draw_card_skip_count,
        "draw_card_select_share_when_offered": safe_ratio(draw_card_select_count, draw_card_offer_count),
        "scaling_card_offer_count": scaling_card_offer_count,
        "scaling_card_select_count": scaling_card_select_count,
        "scaling_card_skip_count": scaling_card_skip_count,
        "scaling_card_select_share_when_offered": safe_ratio(
            scaling_card_select_count, scaling_card_offer_count
        ),
        "reward_claim_count": reward_claims,
        "reward_proceed_with_items_count": reward_proceeds,
        "reward_unclaimed_item_proceed_share": safe_ratio(
            reward_proceeds, reward_claims + reward_proceeds
        ),
        "reward_claims_per_reward_proceed": safe_ratio(reward_claims, reward_proceeds),
    }


def macro_capabilities(steps: list[dict[str, Any]]) -> dict[str, Any]:
    counts = Counter(action_type(step.get("chosen_action") or {}) for step in steps)
    campfire_choices = Counter()
    for step in steps:
        key = str(step.get("chosen_action_key") or "")
        if key.startswith("campfire/"):
            campfire_choices[key.split("/", 1)[1].split("/", 1)[0]] += 1
    shop_actions = {
        "buy_card": counts.get("buy_card", 0),
        "buy_relic": counts.get("buy_relic", 0),
        "buy_potion": counts.get("buy_potion", 0),
        "purge_card": counts.get("purge_card", 0),
    }
    total_shop_spend_actions = sum(shop_actions.values())
    smith_count = campfire_choices.get("smith", 0)
    rest_count = campfire_choices.get("rest", 0)
    return {
        "map_choice_count": counts.get("select_map_node", 0),
        "event_choice_count": counts.get("event_choice", 0) + counts.get("select_event_option", 0),
        "shop_action_counts": shop_actions,
        "shop_resource_action_count": total_shop_spend_actions,
        "campfire_choice_counts": dict(campfire_choices),
        "smith_to_rest_ratio": safe_ratio(smith_count, rest_count) if rest_count else float(smith_count),
        "purge_to_buy_ratio": safe_ratio(shop_actions["purge_card"], shop_actions["buy_card"] + shop_actions["buy_relic"] + shop_actions["buy_potion"]),
    }


def deck_capabilities(episodes: list[dict[str, Any]]) -> dict[str, Any]:
    final_decks = []
    for episode in episodes:
        last_obs = last_observation(episode)
        deck = last_obs.get("deck") or {}
        if isinstance(deck, dict) and deck:
            final_decks.append(deck)
    return {
        "final_deck_count": len(final_decks),
        "average_attack_density": average_deck_field(final_decks, "attack_count"),
        "average_skill_density": average_deck_field(final_decks, "skill_count"),
        "average_power_density": average_deck_field(final_decks, "power_count"),
        "average_damage_density": average_deck_field(final_decks, "damage_card_count"),
        "average_block_density": average_deck_field(final_decks, "block_card_count"),
        "average_draw_density": average_deck_field(final_decks, "draw_card_count"),
        "average_scaling_density": average_deck_field(final_decks, "scaling_card_count"),
        "average_starter_basic_density": average_deck_field(final_decks, "starter_basic_count"),
        "average_status_count": safe_mean([float(deck.get("status_count") or 0) for deck in final_decks]),
        "average_curse_count": safe_mean([float(deck.get("curse_count") or 0) for deck in final_decks]),
        "average_cost": safe_mean([float(deck.get("average_cost_milli") or 0) / 1000.0 for deck in final_decks]),
    }


def average_deck_field(decks: list[dict[str, Any]], field: str) -> float:
    values = []
    for deck in decks:
        total = (
            float(deck.get("attack_count") or 0)
            + float(deck.get("skill_count") or 0)
            + float(deck.get("power_count") or 0)
        )
        values.append(safe_ratio(float(deck.get(field) or 0), max(total, 1.0)))
    return safe_mean(values)


def diagnostic_flags(
    *,
    crashes: int,
    illegal: int,
    truncated: int,
    steps: list[dict[str, Any]],
    progression_floor: float,
    combat: dict[str, Any],
    boss: dict[str, Any],
    deck: dict[str, Any],
    action_type_counts: Counter[str],
    decision_type_counts: Counter[str],
    rewards_summary: dict[str, Any],
    macro: dict[str, Any],
) -> list[str]:
    flags = []
    if crashes:
        flags.append("crash")
    if illegal:
        flags.append("illegal_action")
    if truncated:
        flags.append("python_truncated")
    total_actions = sum(action_type_counts.values())
    if total_actions:
        action, count = action_type_counts.most_common(1)[0]
        if count / total_actions >= 0.85:
            flags.append(f"action_collapse:{action}")
    total_decisions = sum(decision_type_counts.values())
    if total_decisions:
        decision, count = decision_type_counts.most_common(1)[0]
        if count / total_decisions >= 0.95:
            flags.append(f"decision_collapse:{decision}")
    card_skip_share = float(rewards_summary.get("card_skip_share") or 0)
    if card_skip_share >= 0.70:
        flags.append("reward_card_skip_high")
    elif card_skip_share >= 0.20:
        flags.append("reward_card_skip_nontrivial")
    if int(rewards_summary.get("skipped_good_offer_count") or 0) > 0:
        flags.append("skipped_good_card_offer")
    if int(rewards_summary.get("reward_proceed_with_items_count") or 0) >= 10:
        if float(rewards_summary.get("reward_unclaimed_item_proceed_share") or 0) >= 0.20:
            flags.append("reward_item_claim_avoidance")
    if int(rewards_summary.get("missed_best_rule_score_gap_ge_30_count") or 0) > 0:
        flags.append("reward_card_large_rule_score_regret")
    if int(rewards_summary.get("plan_adjusted_missed_best_gap_ge_30_count") or 0) > 0:
        flags.append("reward_card_plan_adjusted_regret")
    if int(rewards_summary.get("draw_card_offer_count") or 0) >= 10:
        if float(rewards_summary.get("draw_card_select_share_when_offered") or 0) < 0.20:
            flags.append("reward_card_draw_avoidance")
    if int(rewards_summary.get("scaling_card_offer_count") or 0) >= 10:
        if float(rewards_summary.get("scaling_card_select_share_when_offered") or 0) < 0.20:
            flags.append("reward_card_scaling_avoidance")
    if int(boss.get("boss_combat_entry_count") or 0) > 0:
        if float(boss.get("average_boss_entry_hp_ratio") or 0) < 0.55:
            flags.append("boss_entry_hp_weak")
    elif progression_floor >= 12.0:
        flags.append("boss_entry_missing_despite_progress")
    if progression_floor >= 10.0 and int(deck.get("final_deck_count") or 0) > 0:
        if float(deck.get("average_draw_density") or 0) < 0.04:
            flags.append("deck_draw_density_low")
        if float(deck.get("average_scaling_density") or 0) < 0.03:
            flags.append("deck_scaling_density_low")
        if float(deck.get("average_starter_basic_density") or 0) > 0.55:
            flags.append("starter_basic_density_high")
    if float(macro.get("smith_to_rest_ratio") or 0) >= 4.0:
        flags.append("over_smith_low_rest")
    if float(combat.get("potion_uses_per_combat_floor") or 0) > 1.0:
        flags.append("potion_use_rate_high")
    if steps and int(macro.get("shop_resource_action_count") or 0) == 0:
        flags.append("no_shop_resource_actions")
    return flags


def last_observation(episode: dict[str, Any]) -> dict[str, Any]:
    steps = episode.get("steps") or []
    for step in reversed(steps):
        obs = step.get("observation") or {}
        if obs:
            return obs
    return {}


def action_type(action: dict[str, Any]) -> str:
    if isinstance(action, dict):
        return str(action.get("type") or "unknown")
    return "unknown"


def action_prefix(key: str) -> str:
    parts = str(key or "unknown").split("/")
    if len(parts) >= 2:
        return "/".join(parts[:2])
    return str(key or "unknown")


def safe_mean(values: list[float] | list[int]) -> float:
    return float(mean(values)) if values else 0.0


def safe_ratio(num: float | int, den: float | int) -> float:
    den_f = float(den)
    if den_f == 0:
        return 0.0
    return float(num) / den_f


def median(values: list[int]) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    mid = len(ordered) // 2
    if len(ordered) % 2:
        return float(ordered[mid])
    return (ordered[mid - 1] + ordered[mid]) / 2.0


def parse_named_paths(values: list[str]) -> dict[str, Path]:
    out: dict[str, Path] = {}
    for raw in values:
        if "=" not in raw:
            raise SystemExit(f"--plan-query-report must use POLICY=PATH, got {raw!r}")
        name, path = raw.split("=", 1)
        name = name.strip()
        if not name:
            raise SystemExit(f"--plan-query-report has empty policy name: {raw!r}")
        out[name] = Path(path.strip())
    return out


def plan_query_eval_signals(paths: dict[str, Path]) -> list[dict[str, Any]]:
    signals = []
    for policy, path in sorted(paths.items()):
        report = json.loads(path.read_text(encoding="utf-8"))
        summary = report.get("summary") or {}
        flags = summary.get("flag_counts") or {}
        signals.append(
            {
                "policy": policy,
                "report_path": str(path),
                "case_count": int(summary.get("case_count") or 0),
                "query_status_counts": summary.get("query_status_counts") or {},
                "flag_counts": flags,
                "core_metrics": {
                    "missed_full_block_line": int(flags.get("missed_full_block_line") or 0),
                    "full_block_damage_gap": int(flags.get("full_block_damage_gap") or 0),
                    "setup_and_block_available_clean": int(
                        flags.get("setup_and_block_available_clean") or 0
                    ),
                    "setup_available_but_leaks": int(flags.get("setup_available_but_leaks") or 0),
                    "near_lethal_small_gap": int(flags.get("near_lethal_small_gap") or 0),
                    "needs_deeper_search": int(summary.get("needs_deeper_search_cases") or 0),
                },
            }
        )
    return signals


def main() -> int:
    args = parse_args()
    policies = parse_policy_list(args.policies)
    artifact_dir = args.artifact_dir or (REPO_ROOT / "tools" / "artifacts" / "full_run_capabilities")
    artifact_dir.mkdir(parents=True, exist_ok=True)
    out_path = args.out or artifact_dir / "full_run_capability_report.json"

    policy_reports = []
    for policy in policies:
        if policy == "model":
            policy_reports.append(run_model_policy(args, artifact_dir))
        else:
            policy_reports.append(run_rust_policy(args, policy, artifact_dir))

    report = {
        "report_version": "full_run_capability_report_v1",
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        "purpose": "full-run capability probes; metrics are diagnostic, not policy-strength proof",
        "config": {
            "episodes": args.episodes,
            "seed": args.seed,
            "ascension": args.ascension,
            "player_class": args.player_class,
            "final_act": args.final_act,
            "max_steps": args.max_steps,
            "policies": policies,
            "model": str(args.model) if args.model else None,
            "reward_shaping_profile": args.reward_shaping_profile,
            "feature_profile": args.feature_profile,
        },
        "policies": policy_reports,
        "plan_query_eval_signals": plan_query_eval_signals(parse_named_paths(args.plan_query_report)),
        "comparison": compare_policy_reports(policy_reports),
    }
    write_json(out_path, report)
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if not any(report.get("diagnostic_flags") for report in policy_reports) else 0


def compare_policy_reports(reports: list[dict[str, Any]]) -> dict[str, Any]:
    if not reports:
        return {}
    by_name = {str(report.get("policy")): report for report in reports}
    baseline = by_name.get("rule_baseline_v0") or by_name.get("random_masked") or reports[0]
    baseline_name = str(baseline.get("policy"))
    out = {"baseline_policy": baseline_name, "deltas": {}}
    metrics = {
        "average_floor": ("progression", "average_floor"),
        "average_combat_wins": ("progression", "average_combat_wins"),
        "average_visible_hp_loss_per_combat_floor": ("combat", "average_visible_hp_loss_per_combat_floor"),
        "potion_uses_per_combat_floor": ("combat", "potion_uses_per_combat_floor"),
        "boss_combat_entry_count": ("boss", "boss_combat_entry_count"),
        "average_boss_entry_hp_ratio": ("boss", "average_boss_entry_hp_ratio"),
        "card_skip_share": ("rewards", "card_skip_share"),
        "selected_card_rule_score_average": ("rewards", "selected_card_rule_score_average"),
        "best_offer_rule_score_average": ("rewards", "best_offer_rule_score_average"),
        "missed_best_rule_score_gap_average": ("rewards", "missed_best_rule_score_gap_average"),
        "missed_best_rule_score_gap_ge_30_count": ("rewards", "missed_best_rule_score_gap_ge_30_count"),
        "plan_adjusted_best_offer_score_average": ("rewards", "plan_adjusted_best_offer_score_average"),
        "plan_adjusted_missed_best_gap_average": ("rewards", "plan_adjusted_missed_best_gap_average"),
        "plan_adjusted_missed_best_gap_ge_30_count": ("rewards", "plan_adjusted_missed_best_gap_ge_30_count"),
        "skipped_good_offer_count": ("rewards", "skipped_good_offer_count"),
        "draw_card_select_share_when_offered": ("rewards", "draw_card_select_share_when_offered"),
        "scaling_card_select_share_when_offered": ("rewards", "scaling_card_select_share_when_offered"),
        "shop_resource_action_count": ("macro", "shop_resource_action_count"),
        "smith_to_rest_ratio": ("macro", "smith_to_rest_ratio"),
        "average_draw_density": ("deck", "average_draw_density"),
        "average_scaling_density": ("deck", "average_scaling_density"),
        "average_starter_basic_density": ("deck", "average_starter_basic_density"),
    }
    for report in reports:
        name = str(report.get("policy"))
        if name == baseline_name:
            continue
        out["deltas"][name] = {
            metric: nested_metric(report, section, key) - nested_metric(baseline, section, key)
            for metric, (section, key) in metrics.items()
        }
    return out


def nested_metric(report: dict[str, Any], section: str, key: str) -> float:
    return float((report.get(section) or {}).get(key) or 0)


if __name__ == "__main__":
    raise SystemExit(main())
