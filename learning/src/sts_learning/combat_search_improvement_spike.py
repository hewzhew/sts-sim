"""Single-root public-chance combat policy-improvement feasibility spike."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
from dataclasses import asdict
from pathlib import Path

from .combat_outcomes import CombatTerminalStepBatch
from .combat_root_artifacts import load_combat_root_source, read_combat_root_artifact
from .published_combat_behavior import recover_published_combat_behavior
from .torch_behavior import FrozenGreedyTorchPolicy
from .torch_combat_session_config import CombatSessionBridge, CombatWinSessionLimits


def _mean_and_standard_error(values: list[float]) -> tuple[float | None, float | None]:
    if not values:
        return None, None
    mean = statistics.fmean(values)
    standard_error = (
        statistics.stdev(values) / math.sqrt(len(values)) if len(values) > 1 else None
    )
    return mean, standard_error


def _root_audit(source: object, root_slot: int) -> dict[str, object]:
    audit = source.combat_root_audit(root_slot)
    return {
        "seed": audit.seed,
        "act": audit.act,
        "floor": audit.floor,
        "ascension_level": audit.ascension_level,
        "hp": audit.hp,
        "max_hp": audit.max_hp,
        "encounter_id": audit.encounter_id,
        "monster_ids": audit.monster_ids,
        "master_deck_cards": audit.master_deck_cards,
        "relic_ids": audit.relic_ids,
        "potion_ids": audit.potion_ids,
    }


def _rollout_action(
    source: object,
    particle_slot: int,
    root_ordinal: int,
    policy: FrozenGreedyTorchPolicy,
    *,
    max_model_rounds: int,
    max_transitions: int,
) -> dict[str, object]:
    group = source.combat_group(particle_slot, 1, potion_slots=[])
    forced = False
    model_rounds = 0
    transitions = 0
    while group.terminal_count < 1:
        while not group.ready:
            if model_rounds >= max_model_rounds:
                return {"status": "unknown", "reason": "model_round_limit"}
            decision = group.decision_batch(semantic=True)
            if not forced:
                row_splits = tuple(int(value) for value in decision["candidate_row_splits"])
                if len(row_splits) != 2 or not 0 <= root_ordinal < row_splits[1]:
                    raise RuntimeError(
                        "root candidate ordinal is not present on a chance particle"
                    )
                ordinals = [root_ordinal]
                forced = True
            else:
                ordinals = list(policy.choose(decision).ordinals)
            group.choose(ordinals)
            model_rounds += 1
        if transitions >= max_transitions:
            return {"status": "unknown", "reason": "transition_limit"}
        step = group.step()
        terminal = CombatTerminalStepBatch.from_bridge_step(step, replicate_count=1)
        transitions += 1
        if terminal.outcomes:
            outcome = terminal.outcomes[0]
            row = asdict(outcome)
            row["terminal_kind"] = outcome.terminal_kind.name.lower()
            row.update(
                status="terminal",
                model_rounds=model_rounds,
                transitions=transitions,
            )
            return row
    raise RuntimeError("combat chance rollout terminated without an outcome")


def _evaluate_action(
    source: object,
    particle_seeds: tuple[int, ...],
    ordinal: int,
    policy: FrozenGreedyTorchPolicy,
    *,
    max_model_rounds: int,
    max_transitions: int,
) -> dict[str, object]:
    outcomes = [
        {
            "particle_seed": seed,
            **_rollout_action(
                source,
                particle_slot,
                ordinal,
                policy,
                max_model_rounds=max_model_rounds,
                max_transitions=max_transitions,
            ),
        }
        for particle_slot, seed in enumerate(particle_seeds)
    ]
    terminal = [row for row in outcomes if row["status"] == "terminal"]
    wins = [row for row in terminal if row["won"]]
    win_rate, win_rate_standard_error = _mean_and_standard_error(
        [1.0 if row["won"] else 0.0 for row in terminal]
    )
    mean_final_hp, mean_final_hp_standard_error = _mean_and_standard_error(
        [float(row["final_hp"]) for row in terminal]
    )
    mean_winning_final_hp, mean_winning_final_hp_standard_error = (
        _mean_and_standard_error([float(row["final_hp"]) for row in wins])
    )
    return {
        "ordinal": ordinal,
        "sample_count": len(outcomes),
        "completed_count": len(terminal),
        "unknown_count": len(outcomes) - len(terminal),
        "wins": len(wins),
        "win_rate": win_rate,
        "win_rate_standard_error": win_rate_standard_error,
        "mean_final_hp": mean_final_hp,
        "mean_final_hp_standard_error": mean_final_hp_standard_error,
        "mean_winning_final_hp": mean_winning_final_hp,
        "mean_winning_final_hp_standard_error": mean_winning_final_hp_standard_error,
        "outcomes": outcomes,
    }


def _proposal(actions: list[dict[str, object]]) -> tuple[int | None, str]:
    if any(int(action["unknown_count"]) for action in actions):
        return None, "incomplete_equal_allocation"
    if not actions or max(int(action["wins"]) for action in actions) == 0:
        return None, "no_action_demonstrated_a_win"
    selected = max(
        actions,
        key=lambda action: (
            int(action["wins"]),
            sum(
                int(row["final_hp"])
                for row in action["outcomes"]
                if row["status"] == "terminal"
            ),
            -int(action["ordinal"]),
        ),
    )
    return int(selected["ordinal"]), "win_count_then_total_final_hp"


def _paired_summary(
    seeds: tuple[int, ...],
    candidate: dict[str, object],
    baseline: dict[str, object],
) -> dict[str, object]:
    pairs = []
    for seed, candidate_row, baseline_row in zip(
        seeds,
        candidate["outcomes"],
        baseline["outcomes"],
        strict=True,
    ):
        if candidate_row["status"] != "terminal" or baseline_row["status"] != "terminal":
            pairs.append({"particle_seed": seed, "status": "unknown"})
            continue
        both_win = bool(candidate_row["won"] and baseline_row["won"])
        pairs.append(
            {
                "particle_seed": seed,
                "status": "paired",
                "candidate_won": bool(candidate_row["won"]),
                "baseline_won": bool(baseline_row["won"]),
                "win_delta": int(candidate_row["won"]) - int(baseline_row["won"]),
                "both_win_final_hp_delta": (
                    int(candidate_row["final_hp"]) - int(baseline_row["final_hp"])
                    if both_win
                    else None
                ),
            }
        )
    completed = [pair for pair in pairs if pair["status"] == "paired"]
    both_win_hp = [
        int(pair["both_win_final_hp_delta"])
        for pair in completed
        if pair["both_win_final_hp_delta"] is not None
    ]
    win_deltas = [float(pair["win_delta"]) for pair in completed]
    mean_win_delta, mean_win_delta_standard_error = _mean_and_standard_error(win_deltas)
    mean_hp_delta, mean_hp_delta_standard_error = _mean_and_standard_error(
        [float(value) for value in both_win_hp]
    )
    return {
        "paired_count": len(completed),
        "unknown_count": len(pairs) - len(completed),
        "candidate_wins": int(candidate["wins"]),
        "baseline_wins": int(baseline["wins"]),
        "mean_win_delta": mean_win_delta,
        "mean_win_delta_standard_error": mean_win_delta_standard_error,
        "mean_both_win_final_hp_delta": mean_hp_delta,
        "mean_both_win_final_hp_delta_standard_error": mean_hp_delta_standard_error,
        "pairs": pairs,
    }


def run(args: argparse.Namespace) -> dict[str, object]:
    limits = CombatWinSessionLimits()
    artifact = read_combat_root_artifact(args.artifact, max_bytes=limits.max_artifact_bytes)
    bridge = CombatSessionBridge.installed()
    try:
        from sts_learning_bridge import LearningBatchEnv
    except ImportError as error:
        raise RuntimeError("standalone learning bridge wheel is not installed") from error
    chance_loader = getattr(LearningBatchEnv, "from_combat_public_chance_particles", None)
    if not callable(chance_loader):
        raise RuntimeError("installed learning bridge does not provide public-chance particles")

    behavior = recover_published_combat_behavior(
        args.behavior,
        bridge,
        limits,
        behavior_seeds=(args.policy_seed,),
    )
    policy = FrozenGreedyTorchPolicy.from_behavior(behavior.policies[0])
    exact_source = load_combat_root_source(
        bridge,
        artifact,
        expected_roots=args.root_count,
        max_bytes=limits.max_artifact_bytes,
    )
    exact_group = exact_source.combat_group(args.root_slot, 1, potion_slots=[])
    audit_raw = exact_group.combat_decision_audit_json(0)
    if not isinstance(audit_raw, str):
        raise RuntimeError("source root has no atomic combat decision audit")
    audit = json.loads(audit_raw)
    candidates = audit.get("candidates")
    if not isinstance(candidates, list) or not candidates:
        raise RuntimeError("source root exposes no typed combat candidates")
    baseline_choice = policy.choose(exact_group.decision_batch(semantic=True))
    if len(baseline_choice.ordinals) != 1:
        raise RuntimeError("source root policy decision is not singular")
    baseline_ordinal = int(baseline_choice.ordinals[0])

    selection_seeds = tuple(args.seed_base + index for index in range(args.selection_particles))
    evaluation_seeds = tuple(
        args.seed_base + args.selection_particles + index
        for index in range(args.evaluation_particles)
    )
    selection_source = chance_loader(
        artifact,
        expected_roots=args.root_count,
        source_slot=args.root_slot,
        particle_seeds=list(selection_seeds),
        max_bytes=limits.max_artifact_bytes,
    )
    selection_actions = [
        {
            "candidate": candidates[ordinal],
            **_evaluate_action(
                selection_source,
                selection_seeds,
                ordinal,
                policy,
                max_model_rounds=args.max_model_rounds,
                max_transitions=args.max_transitions,
            ),
        }
        for ordinal in range(len(candidates))
    ]
    proposal_ordinal, proposal_rule = _proposal(selection_actions)

    evaluation: dict[str, object] | None = None
    if proposal_ordinal is not None:
        evaluation_source = chance_loader(
            artifact,
            expected_roots=args.root_count,
            source_slot=args.root_slot,
            particle_seeds=list(evaluation_seeds),
            max_bytes=limits.max_artifact_bytes,
        )
        candidate_result = _evaluate_action(
            evaluation_source,
            evaluation_seeds,
            proposal_ordinal,
            policy,
            max_model_rounds=args.max_model_rounds,
            max_transitions=args.max_transitions,
        )
        baseline_result = (
            candidate_result
            if baseline_ordinal == proposal_ordinal
            else _evaluate_action(
                evaluation_source,
                evaluation_seeds,
                baseline_ordinal,
                policy,
                max_model_rounds=args.max_model_rounds,
                max_transitions=args.max_transitions,
            )
        )
        evaluation = {
            "candidate": candidate_result,
            "baseline": baseline_result,
            "paired": _paired_summary(evaluation_seeds, candidate_result, baseline_result),
        }

    return {
        "schema_name": "CombatSearchImprovementFeasibilitySpike",
        "schema_version": 1,
        "teacher_valid": False,
        "claim": "single_root_wiring_feasibility_only",
        "artifact_sha256": hashlib.sha256(artifact).hexdigest(),
        "source_root_slot": args.root_slot,
        "source_root": _root_audit(exact_source, args.root_slot),
        "behavior_manifest_id": behavior.manifest_id.digest.hex(),
        "continuation_policy": "frozen_greedy_current_behavior",
        "chance_sampler": "independent_hidden_stream_reseed_v1",
        "potion_lane": "never",
        "selection_particle_seeds": selection_seeds,
        "evaluation_particle_seeds": evaluation_seeds,
        "selection_evaluation_disjoint": set(selection_seeds).isdisjoint(evaluation_seeds),
        "baseline_ordinal": baseline_ordinal,
        "baseline_candidate": candidates[baseline_ordinal],
        "proposal_ordinal": proposal_ordinal,
        "proposal_candidate": None if proposal_ordinal is None else candidates[proposal_ordinal],
        "proposal_rule": proposal_rule,
        "selection_actions": selection_actions,
        "evaluation": evaluation,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--behavior", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--root-count", type=int, default=1)
    parser.add_argument("--root-slot", type=int, default=0)
    parser.add_argument("--selection-particles", type=int, default=8)
    parser.add_argument("--evaluation-particles", type=int, default=8)
    parser.add_argument("--seed-base", type=int, default=2026081101000)
    parser.add_argument("--policy-seed", type=int, default=2026081101999)
    parser.add_argument("--max-model-rounds", type=int, default=512)
    parser.add_argument("--max-transitions", type=int, default=512)
    args = parser.parse_args()
    for name in (
        "root_count",
        "selection_particles",
        "evaluation_particles",
        "max_model_rounds",
        "max_transitions",
    ):
        if getattr(args, name) <= 0:
            parser.error(f"--{name.replace('_', '-')} must be positive")
    if args.root_slot < 0 or args.root_slot >= args.root_count:
        parser.error("--root-slot must be inside --root-count")
    if args.output.exists():
        parser.error("--output must be a fresh path")

    result = run(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    paired = None
    if result["evaluation"] is not None:
        paired = {
            key: value
            for key, value in result["evaluation"]["paired"].items()
            if key != "pairs"
        }
    print(
        json.dumps(
            {
                "output": str(args.output),
                "candidate_count": len(result["selection_actions"]),
                "baseline_ordinal": result["baseline_ordinal"],
                "proposal_ordinal": result["proposal_ordinal"],
                "proposal_rule": result["proposal_rule"],
                "paired": paired,
            },
            ensure_ascii=False,
            separators=(",", ":"),
        )
    )


if __name__ == "__main__":
    main()
