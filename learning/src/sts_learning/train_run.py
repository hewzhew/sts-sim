"""Bounded whole-run on-policy training from a published combat warm start."""

from __future__ import annotations

import argparse
import json
import operator
import time
from dataclasses import dataclass, replace
from pathlib import Path
from typing import TextIO

from .evaluation import (
    HeldOutEvaluationResult,
    HeldOutEvaluationSpec,
    evaluate_held_out_behavior,
)
from .published_combat_behavior import (
    PublishedCombatBehavior,
    recover_published_combat_behavior,
)
from .seeds import SeedPartition, SeedSchedule
from .terminal_returns import OnPolicyObjectiveConfig, TerminalAdvantageMode
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


RUN_TRAINING_SCHEMA = "sts-learning-run-training-v1"


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
    profile = replace(
        CategoricalOnlineProfile(),
        objective=OnPolicyObjectiveConfig(
            attempts_per_update=config.attempts_per_update,
            advantage_mode=config.advantage_mode,
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
        active_run_bridge,
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
        _write(journal, _configuration(config, warm_start))
        for generation in range(config.generations):
            started = time.perf_counter()
            result = session.advance_generation(
                max_batch_steps=config.max_batch_steps_per_generation
            )
            elapsed = time.perf_counter() - started
            _write(journal, _generation(generation, result, session, elapsed))
            progress = result.terminal_progress
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
            active_run_bridge.environment,
            evaluation_policy,
            schedule=SeedSchedule(
                SeedPartition.HELD_OUT,
                next_candidate=config.held_out_seed_start,
            ),
            spec=HeldOutEvaluationSpec(
                slot_count=config.slot_count,
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
        "slot_count": config.slot_count,
        "generations": config.generations,
        "attempts_per_update": config.attempts_per_update,
        "advantage_mode": config.advantage_mode.name.lower(),
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
        "elapsed_seconds": elapsed,
    }


def _summary(
    config: RunTrainingCommandConfig,
    warm_start: PublishedCombatBehavior,
    session: CategoricalOnlineSession,
    checkpoint_id: str,
    evaluation: HeldOutEvaluationResult,
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
        "held_out_seed_end": evaluation.schedule_end.next_candidate,
    }


def _write(journal: TextIO, value: dict[str, object]) -> None:
    journal.write(json.dumps(value, separators=(",", ":"), sort_keys=True))
    journal.write("\n")
    journal.flush()


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
        choices=("raw-return", "leave-one-out"),
        default="raw-return",
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
                else TerminalAdvantageMode.LEAVE_ONE_OUT
            ),
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
