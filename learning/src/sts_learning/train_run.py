"""Bounded whole-run on-policy training from a published combat warm start."""

from __future__ import annotations

import argparse
import json
import operator
import time
from dataclasses import dataclass, replace
from pathlib import Path
from typing import TextIO

from .credit_assignment import (
    CreditAssignmentComparison,
    DecisionCreditDistribution,
)
from .evaluation import (
    HeldOutEvaluationResult,
    HeldOutEvaluationSpec,
    evaluate_held_out_behavior,
)
from .combat_potion_lane import CombatPotionLane
from .evaluate_run import RunPotionLane, resolve_run_potion_lane
from .published_combat_behavior import (
    PublishedCombatBehavior,
    recover_published_combat_behavior,
)
from .seeds import SeedPartition, SeedSchedule
from .terminal_returns import (
    OnPolicyObjectiveConfig,
    RunDecisionScope,
    TerminalAdvantageMode,
)
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinSessionLimits,
)
from .torch_generation import CategoricalGenerationAdvanceResult
from .torch_session import (
    CategoricalOnlineSession,
    CategoricalOnlineSessionFactory,
    NoRecoveryCurriculum,
)
from .torch_session_config import (
    CategoricalOnlineProfile,
    CategoricalOnlineSessionConfig,
    CategoricalSessionBridge,
    CategoricalSessionLimits,
)


RUN_TRAINING_SCHEMA = "sts-learning-run-training-v2"


class RunTrainingCommandError(RuntimeError):
    """A bounded whole-run training command is malformed or incomplete."""


@dataclass(frozen=True)
class RunTrainingCommandConfig:
    warm_start_behavior: Path
    output: Path
    slot_count: int
    generations: int
    attempts_per_update: int
    max_batch_steps_per_generation: int
    model_seed: int
    behavior_seed: int
    training_seed_start: int
    evaluation_attempts: int
    evaluation_max_batch_steps: int
    evaluation_behavior_seed: int
    held_out_seed_start: int
    advantage_mode: TerminalAdvantageMode = TerminalAdvantageMode.RAW_RETURN
    decision_scope: RunDecisionScope = RunDecisionScope.ALL
    potion_lane: RunPotionLane = RunPotionLane.TRAINED

    def __post_init__(self) -> None:
        behavior = Path(self.warm_start_behavior).resolve()
        output = Path(self.output).resolve()
        if not behavior.is_dir():
            raise RunTrainingCommandError(
                "run training warm-start behavior is not a directory"
            )
        if output.exists() and (
            not output.is_dir() or any(output.iterdir())
        ):
            raise RunTrainingCommandError(
                "run training output must be absent or empty"
            )
        if output == behavior or behavior in output.parents:
            raise RunTrainingCommandError(
                "run training output must stay outside the warm-start behavior"
            )
        object.__setattr__(self, "warm_start_behavior", behavior)
        object.__setattr__(self, "output", output)
        if not isinstance(self.advantage_mode, TerminalAdvantageMode):
            raise RunTrainingCommandError(
                "run training advantage mode must be typed"
            )
        if not isinstance(self.decision_scope, RunDecisionScope):
            raise RunTrainingCommandError(
                "run training decision scope must be typed"
            )
        if not isinstance(self.potion_lane, RunPotionLane):
            raise RunTrainingCommandError(
                "run training potion lane must be typed"
            )
        for name in (
            "slot_count",
            "attempts_per_update",
            "max_batch_steps_per_generation",
            "evaluation_attempts",
            "evaluation_max_batch_steps",
        ):
            object.__setattr__(self, name, _positive(getattr(self, name), name))
        object.__setattr__(
            self,
            "generations",
            _nonnegative(self.generations, "generations"),
        )
        for name in (
            "model_seed",
            "behavior_seed",
            "training_seed_start",
            "evaluation_behavior_seed",
            "held_out_seed_start",
        ):
            object.__setattr__(self, name, _seed(getattr(self, name), name))


def run_run_training(
    config: RunTrainingCommandConfig,
    *,
    combat_bridge: CombatSessionBridge | None = None,
    run_bridge: CategoricalSessionBridge | None = None,
) -> dict[str, object]:
    """Train complete runs, publish once, then evaluate one held-out prefix."""

    if not isinstance(config, RunTrainingCommandConfig):
        raise RunTrainingCommandError("run training config must be typed")
    active_combat_bridge = (
        combat_bridge
        if combat_bridge is not None
        else CombatSessionBridge.installed()
    )
    active_run_bridge = (
        run_bridge
        if run_bridge is not None
        else CategoricalSessionBridge.installed()
    )
    if not isinstance(active_combat_bridge, CombatSessionBridge):
        raise RunTrainingCommandError("run training combat bridge must be typed")
    if not isinstance(active_run_bridge, CategoricalSessionBridge):
        raise RunTrainingCommandError(
            "run training environment bridge must be typed"
        )
    if active_combat_bridge.semantic_schema != active_run_bridge.semantic_schema:
        raise RunTrainingCommandError(
            "combat warm start and run environment semantic schemas differ"
        )
    warm_start = recover_published_combat_behavior(
        config.warm_start_behavior,
        active_combat_bridge,
        CombatWinSessionLimits(),
        (config.behavior_seed,),
    )
    potion_lane = resolve_run_potion_lane(config.potion_lane, warm_start)
    training_run_bridge = _bridge_for_potion_lane(
        active_run_bridge,
        potion_lane,
    )
    profile = replace(
        CategoricalOnlineProfile(),
        objective=OnPolicyObjectiveConfig(
            attempts_per_update=config.attempts_per_update,
            advantage_mode=config.advantage_mode,
            decision_scope=config.decision_scope,
        ),
    )
    limits = replace(
        CategoricalSessionLimits(),
        owner_capacity=max(16, config.generations + 2),
    )
    session_config = CategoricalOnlineSessionConfig(
        schedule=SeedSchedule(
            SeedPartition.TRAINING,
            next_candidate=config.training_seed_start,
        ),
        slot_count=config.slot_count,
        max_recoveries_per_episode=0,
        profile=profile,
        limits=limits,
    )
    factory = CategoricalOnlineSessionFactory(
        config.output,
        training_run_bridge,
        session_config,
        NoRecoveryCurriculum(),
    )
    session = factory.new(
        model_seed=config.model_seed,
        behavior_seed=config.behavior_seed,
        initial_scorer=warm_start.policies[0].frozen_scorer,
    )
    config.output.mkdir(parents=True, exist_ok=True)
    journal_path = config.output / "training.jsonl"
    with journal_path.open("x", encoding="utf-8", newline="\n") as journal:
        _write(journal, _configuration(config, warm_start, potion_lane))
        for generation in range(config.generations):
            started = time.perf_counter()
            result = session.advance_generation(
                max_batch_steps=config.max_batch_steps_per_generation
            )
            elapsed = time.perf_counter() - started
            _write(journal, _generation(generation, result, session, elapsed))
            progress = result.terminal_progress
            credit = session.runner.trainer.last_credit_assignment
            print(
                f"run_generation={generation} promoted={str(result.promoted).lower()} "
                f"attempts={result.terminal_attempts} "
                f"victories={result.terminal_victories} "
                f"defeats={result.terminal_defeats} "
                f"floor_sum={progress.floor_sum} "
                f"floor_counts={_counts(progress.floor_counts)} "
                f"batch_steps={result.batch_steps}/"
                f"{config.max_batch_steps_per_generation} "
                f"loss={_optional_float(session.runner.trainer.snapshot.last_loss)} "
                f"credit={_credit_line(credit)} "
                f"credit_floors={_credit_floor_line(credit)} "
                f"credit_scopes={_credit_scope_line(credit)} "
                f"seconds={elapsed:.3f}",
                flush=True,
            )
            if not result.promoted:
                raise RunTrainingCommandError(
                    "run training hit its batch-step bound before one update; "
                    "the partial live update was not published"
                )

        publication = session.runner.controller.publish_active()
        active_manifest_id = publication.manifest_id
        evaluation_policy = factory.recover_behavior(
            active_manifest_id,
            behavior_seed=config.evaluation_behavior_seed,
        )
        evaluation = evaluate_held_out_behavior(
            training_run_bridge.environment,
            evaluation_policy,
            schedule=SeedSchedule(
                SeedPartition.HELD_OUT,
                next_candidate=config.held_out_seed_start,
            ),
            spec=HeldOutEvaluationSpec(
                slot_count=1,
                terminal_attempt_target=config.evaluation_attempts,
                max_batch_steps=config.evaluation_max_batch_steps,
            ),
        )
        summary = _summary(
            config,
            warm_start,
            session,
            publication.checkpoint_id.digest.hex(),
            evaluation,
            potion_lane,
        )
        _write(journal, summary)

    with (config.output / "summary.json").open(
        "x",
        encoding="utf-8",
        newline="\n",
    ) as destination:
        json.dump(summary, destination, separators=(",", ":"), sort_keys=True)
        destination.write("\n")
    run = evaluation.run.summary
    print(
        "run_training_complete=true "
        f"potion_lane={potion_lane.value} "
        f"potion_lane_request={config.potion_lane.value} "
        f"generations={config.generations} "
        f"optimizer_steps={session.runner.trainer.snapshot.optimizer_steps} "
        f"held_out_attempts={run.terminal_attempts}/"
        f"{config.evaluation_attempts} "
        f"held_out_victories={run.victories} "
        f"held_out_floor_sum={run.terminal_progress.floor_sum} "
        f"held_out_floor_counts={_counts(run.terminal_progress.floor_counts)} "
        f"output={config.output}",
        flush=True,
    )
    return summary


def _configuration(
    config: RunTrainingCommandConfig,
    warm_start: PublishedCombatBehavior,
    potion_lane: CombatPotionLane,
) -> dict[str, object]:
    return {
        "schema": RUN_TRAINING_SCHEMA,
        "kind": "configuration",
        "warm_start_behavior": str(config.warm_start_behavior),
        "warm_start_manifest_id": warm_start.manifest_id.digest.hex(),
        "warm_start_checkpoint_id": warm_start.checkpoint_id.digest.hex(),
        "warm_start_training_step": warm_start.training_step,
        "warm_start_artifact_sha256": warm_start.training_artifact_sha256,
        "warm_start_potion_lane": warm_start.training_potion_lane.value,
        "requested_run_potion_lane": config.potion_lane.value,
        "run_potion_lane": potion_lane.value,
        "slot_count": config.slot_count,
        "generations": config.generations,
        "attempts_per_update": config.attempts_per_update,
        "advantage_mode": config.advantage_mode.name.lower(),
        "decision_scope": config.decision_scope.name.lower(),
        "max_batch_steps_per_generation": (
            config.max_batch_steps_per_generation
        ),
        "model_seed": config.model_seed,
        "behavior_seed": config.behavior_seed,
        "training_seed_start": config.training_seed_start,
        "evaluation_attempts": config.evaluation_attempts,
        "evaluation_max_batch_steps": config.evaluation_max_batch_steps,
        "evaluation_behavior_seed": config.evaluation_behavior_seed,
        "held_out_seed_start": config.held_out_seed_start,
    }


def _generation(
    generation: int,
    result: CategoricalGenerationAdvanceResult,
    session: CategoricalOnlineSession,
    elapsed: float,
) -> dict[str, object]:
    progress = result.terminal_progress
    trainer = session.runner.trainer.snapshot
    return {
        "schema": RUN_TRAINING_SCHEMA,
        "kind": "generation",
        "generation": generation,
        "promoted": result.promoted,
        "active_training_step_before": result.active_training_step_before,
        "optimizer_steps_after": result.optimizer_steps_after,
        "batch_steps": result.batch_steps,
        "terminal_attempts": result.terminal_attempts,
        "victories": result.terminal_victories,
        "defeats": result.terminal_defeats,
        "terminal_floor_sum": progress.floor_sum,
        "terminal_floor_counts": progress.floor_counts,
        "terminal_act_counts": progress.act_counts,
        "loss": trainer.last_loss,
        "trained_decisions": trainer.trained_decisions,
        "credit_assignment": _credit_assignment(
            session.runner.trainer.last_credit_assignment
        ),
        "elapsed_seconds": elapsed,
    }


def _summary(
    config: RunTrainingCommandConfig,
    warm_start: PublishedCombatBehavior,
    session: CategoricalOnlineSession,
    checkpoint_id: str,
    evaluation: HeldOutEvaluationResult,
    potion_lane: CombatPotionLane,
) -> dict[str, object]:
    run = evaluation.run.summary
    progress = run.terminal_progress
    return {
        "schema": RUN_TRAINING_SCHEMA,
        "kind": "completed",
        "warm_start_manifest_id": warm_start.manifest_id.digest.hex(),
        "warm_start_checkpoint_id": warm_start.checkpoint_id.digest.hex(),
        "active_behavior_manifest_id": (
            session.active_behavior_manifest_id.digest.hex()
        ),
        "active_behavior_checkpoint_id": checkpoint_id,
        "requested_run_potion_lane": config.potion_lane.value,
        "run_potion_lane": potion_lane.value,
        "generations": config.generations,
        "optimizer_steps": session.runner.trainer.snapshot.optimizer_steps,
        "held_out_target_reached": evaluation.complete,
        "held_out_step_limit_reached": evaluation.step_limit_reached,
        "held_out_attempts": run.terminal_attempts,
        "held_out_victories": run.victories,
        "held_out_defeats": run.defeats,
        "held_out_floor_sum": progress.floor_sum,
        "held_out_floor_counts": progress.floor_counts,
        "held_out_act_counts": progress.act_counts,
        "held_out_batch_steps": run.batch_steps,
        "held_out_slot_count": 1,
        "held_out_seed_end": evaluation.schedule_end.next_candidate,
    }


def _write(journal: TextIO, value: dict[str, object]) -> None:
    journal.write(json.dumps(value, separators=(",", ":"), sort_keys=True))
    journal.write("\n")
    journal.flush()


def _bridge_for_potion_lane(
    bridge: CategoricalSessionBridge,
    lane: CombatPotionLane,
) -> CategoricalSessionBridge:
    if lane is CombatPotionLane.ALL:
        return bridge
    if lane is not CombatPotionLane.NEVER:
        raise RunTrainingCommandError(
            "whole-run training supports only all or never potion lanes"
        )
    return replace(
        bridge,
        environment=bridge.environment_without_combat_potions,
        environment_from_checkpoint=_reject_no_potion_resume,
    )


def _reject_no_potion_resume(*_args: object, **_kwargs: object) -> object:
    raise RunTrainingCommandError(
        "no-potion whole-run training does not support resume checkpoints"
    )


def _positive(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized == 0:
        raise RunTrainingCommandError(f"{name} must be positive")
    return normalized


def _nonnegative(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise RunTrainingCommandError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise RunTrainingCommandError(f"{name} must be an integer") from error
    if normalized < 0:
        raise RunTrainingCommandError(f"{name} must be non-negative")
    return normalized


def _seed(value: object, name: str) -> int:
    normalized = _nonnegative(value, name)
    if normalized >= 1 << 63:
        raise RunTrainingCommandError(f"{name} must be below 2^63")
    return normalized


def _counts(values: tuple[tuple[int, int], ...]) -> str:
    return ",".join(f"{key}:{count}" for key, count in values) or "none"


def _optional_float(value: float | None) -> str:
    return "none" if value is None else f"{value:.9g}"


def _credit_line(comparison: CreditAssignmentComparison | None) -> str:
    if comparison is None:
        return "unavailable"
    terminal = comparison.terminal_broadcast
    local = comparison.remaining_progress
    matched = comparison.matched_floor_advantage
    return (
        f"broadcast:{terminal.negative}/{terminal.zero}/{terminal.positive}"
        f"@{terminal.mean:.4f};"
        f"local:{local.negative}/{local.zero}/{local.positive}@{local.mean:.4f};"
        f"matched:{matched.negative}/{matched.zero}/{matched.positive}"
        f"@{matched.mean:.4f}"
    )


def _credit_floor_line(comparison: CreditAssignmentComparison | None) -> str:
    if comparison is None:
        return "unavailable"
    return ",".join(
        f"{row.floor}:{row.remaining_progress.decision_count}"
        f"@{row.terminal_broadcast.mean:.4f}>{row.remaining_progress.mean:.4f}"
        f"#{row.matched_floor_advantage.negative}/"
        f"{row.matched_floor_advantage.zero}/"
        f"{row.matched_floor_advantage.positive}"
        for row in comparison.by_decision_floor
    )


def _credit_scope_line(comparison: CreditAssignmentComparison | None) -> str:
    if comparison is None:
        return "unavailable"
    return ",".join(
        f"{'combat' if row.is_combat else 'strategic'}:"
        f"{row.remaining_progress.decision_count}"
        f"#{row.matched_floor_advantage.negative}/"
        f"{row.matched_floor_advantage.zero}/"
        f"{row.matched_floor_advantage.positive}"
        for row in comparison.by_combat_scope
    )


def _credit_assignment(
    comparison: CreditAssignmentComparison | None,
) -> dict[str, object] | None:
    if comparison is None:
        return None
    return {
        "attempt_count": comparison.attempt_count,
        "terminal_broadcast": _credit_distribution(
            comparison.terminal_broadcast
        ),
        "remaining_progress": _credit_distribution(
            comparison.remaining_progress
        ),
        "matched_floor_advantage": _credit_distribution(
            comparison.matched_floor_advantage
        ),
        "by_decision_floor": [
            {
                "floor": row.floor,
                "terminal_broadcast": _credit_distribution(
                    row.terminal_broadcast
                ),
                "remaining_progress": _credit_distribution(
                    row.remaining_progress
                ),
                "matched_floor_advantage": _credit_distribution(
                    row.matched_floor_advantage
                ),
            }
            for row in comparison.by_decision_floor
        ],
        "by_combat_scope": [
            {
                "is_combat": row.is_combat,
                "terminal_broadcast": _credit_distribution(
                    row.terminal_broadcast
                ),
                "remaining_progress": _credit_distribution(
                    row.remaining_progress
                ),
                "matched_floor_advantage": _credit_distribution(
                    row.matched_floor_advantage
                ),
            }
            for row in comparison.by_combat_scope
        ],
    }


def _credit_distribution(
    distribution: DecisionCreditDistribution,
) -> dict[str, int | float]:
    return {
        "decision_count": distribution.decision_count,
        "negative": distribution.negative,
        "zero": distribution.zero,
        "positive": distribution.positive,
        "minimum": distribution.minimum,
        "maximum": distribution.maximum,
        "mean": distribution.mean,
    }


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Warm-start bounded whole-run on-policy training from one "
            "published combat behavior."
        ),
    )
    parser.add_argument("--warm-start-behavior", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--slots", type=int, default=4)
    parser.add_argument("--generations", type=int, default=1)
    parser.add_argument("--attempts-per-update", type=int, default=8)
    parser.add_argument("--max-batch-steps", type=int, default=4096)
    parser.add_argument("--model-seed", type=int, default=0)
    parser.add_argument("--behavior-seed", type=int, default=90_000)
    parser.add_argument("--training-seed-start", type=int, default=0)
    parser.add_argument("--evaluation-attempts", type=int, default=16)
    parser.add_argument("--evaluation-max-batch-steps", type=int, default=4096)
    parser.add_argument("--evaluation-behavior-seed", type=int, default=100_000)
    parser.add_argument("--held-out-seed-start", type=int, default=1_000_000)
    parser.add_argument(
        "--advantage-mode",
        choices=("raw-return", "leave-one-out", "matched-floor"),
        default="raw-return",
    )
    parser.add_argument(
        "--decision-scope",
        choices=("all", "strategic"),
        default="all",
    )
    parser.add_argument(
        "--potion-lane",
        choices=tuple(lane.value for lane in RunPotionLane),
        default=RunPotionLane.TRAINED.value,
    )
    return parser


def main() -> int:
    arguments = _parser().parse_args()
    run_run_training(
        RunTrainingCommandConfig(
            warm_start_behavior=arguments.warm_start_behavior,
            output=arguments.output,
            slot_count=arguments.slots,
            generations=arguments.generations,
            attempts_per_update=arguments.attempts_per_update,
            max_batch_steps_per_generation=arguments.max_batch_steps,
            model_seed=arguments.model_seed,
            behavior_seed=arguments.behavior_seed,
            training_seed_start=arguments.training_seed_start,
            evaluation_attempts=arguments.evaluation_attempts,
            evaluation_max_batch_steps=arguments.evaluation_max_batch_steps,
            evaluation_behavior_seed=arguments.evaluation_behavior_seed,
            held_out_seed_start=arguments.held_out_seed_start,
            advantage_mode=(
                TerminalAdvantageMode.RAW_RETURN
                if arguments.advantage_mode == "raw-return"
                else (
                    TerminalAdvantageMode.LEAVE_ONE_OUT
                    if arguments.advantage_mode == "leave-one-out"
                    else TerminalAdvantageMode.MATCHED_FLOOR_LEAVE_ONE_OUT
                )
            ),
            decision_scope=(
                RunDecisionScope.ALL
                if arguments.decision_scope == "all"
                else RunDecisionScope.STRATEGIC
            ),
            potion_lane=RunPotionLane(arguments.potion_lane),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
