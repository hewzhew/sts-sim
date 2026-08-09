"""Bounded command-line composition for real multi-root combat training."""

from __future__ import annotations

import argparse
import hashlib
import json
import time
from collections.abc import Mapping
from dataclasses import dataclass, replace
from pathlib import Path
from typing import TYPE_CHECKING, Protocol, TextIO

from .combat_objective import (
    CombatAllLossAxis,
    CombatAllWinAxis,
    CombatPolicyUpdateConfig,
    CombatPolicyUpdateRule,
    CombatWinObjectiveConfig,
)
from .combat_potion_lane import (
    CombatPotionLane,
    CombatPotionLaneError,
    normalize_combat_potion_slots,
)
from .torch_combat_batch_generation import (
    BoundedCombatWinBatchGenerationRunner,
    CombatWinBatchGenerationResult,
    CombatWinRootGenerationResult,
)
from .torch_combat_batch_session import CombatWinBatchSessionFactory
from .torch_behavior import TorchBehaviorPublication
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinBatchSessionConfig,
    CombatWinSessionLimits,
    CombatWinSessionProfile,
)
from .torch_policy import RaggedScorerConfig
from .torch_session_config import CategoricalSessionBridge

if TYPE_CHECKING:
    from .published_combat_behavior import PublishedCombatBehavior
    from .published_run_behavior import PublishedRunBehavior


LEGACY_COMBAT_TRAINING_SCHEMA = "sts-learning-combat-training-v3"
PREVIOUS_COMBAT_TRAINING_SCHEMA = "sts-learning-combat-training-v4"
COMBAT_TRAINING_SCHEMA = "sts-learning-combat-training-v5"


class CombatTrainingCommandError(RuntimeError):
    """A bounded combat training command is malformed."""


class _CombatTrainingSession(Protocol):
    artifact_byte_count: int
    runner: BoundedCombatWinBatchGenerationRunner

    def advance(self) -> CombatWinBatchGenerationResult: ...

    def publish_active_behavior(self) -> TorchBehaviorPublication: ...


@dataclass(frozen=True)
class CombatTrainingCommandConfig:
    artifact: Path
    output: Path
    root_count: int
    replicate_count: int
    updates: int
    model_seed: int
    behavior_seed_base: int
    potion_lane: CombatPotionLane = CombatPotionLane.ALL
    potion_slots: tuple[int, ...] = ()
    warm_start_behavior: Path | None = None
    policy_update: CombatPolicyUpdateConfig = CombatPolicyUpdateConfig()
    all_loss_axis: CombatAllLossAxis = CombatAllLossAxis.NONE

    def __post_init__(self) -> None:
        artifact = Path(self.artifact).resolve()
        output = Path(self.output).resolve()
        if not artifact.is_file():
            raise CombatTrainingCommandError("combat artifact is not a file")
        if output.exists() and (
            not output.is_dir() or any(output.iterdir())
        ):
            raise CombatTrainingCommandError(
                "combat training output must be absent or empty"
            )
        warm_start_behavior = self.warm_start_behavior
        if warm_start_behavior is not None:
            warm_start_behavior = Path(warm_start_behavior).resolve()
            if not warm_start_behavior.is_dir():
                raise CombatTrainingCommandError(
                    "combat warm-start behavior is not a directory"
                )
            if output == warm_start_behavior or warm_start_behavior in output.parents:
                raise CombatTrainingCommandError(
                    "combat training output must stay outside the warm-start behavior"
                )
        root_count = _positive(self.root_count, "root_count")
        replicate_count = _positive(self.replicate_count, "replicate_count")
        updates = _positive(self.updates, "updates")
        model_seed = _seed(self.model_seed, "model_seed")
        behavior_seed_base = _seed(
            self.behavior_seed_base,
            "behavior_seed_base",
        )
        if not isinstance(self.potion_lane, CombatPotionLane):
            raise CombatTrainingCommandError(
                "combat training potion_lane must be typed"
            )
        try:
            potion_slots = normalize_combat_potion_slots(
                self.potion_lane,
                self.potion_slots,
            )
        except CombatPotionLaneError as error:
            raise CombatTrainingCommandError(str(error)) from error
        if not isinstance(self.policy_update, CombatPolicyUpdateConfig):
            raise CombatTrainingCommandError(
                "combat training policy_update must be typed"
            )
        if not isinstance(self.all_loss_axis, CombatAllLossAxis):
            raise CombatTrainingCommandError(
                "combat training all_loss_axis must be typed"
            )
        if root_count < 2:
            raise CombatTrainingCommandError(
                "multi-root training requires at least two roots"
            )
        if replicate_count < 2:
            raise CombatTrainingCommandError(
                "combat training requires at least two replicates"
            )
        if behavior_seed_base + root_count > 1 << 63:
            raise CombatTrainingCommandError("behavior seeds must stay below 2^63")
        object.__setattr__(self, "artifact", artifact)
        object.__setattr__(self, "output", output)
        object.__setattr__(self, "root_count", root_count)
        object.__setattr__(self, "replicate_count", replicate_count)
        object.__setattr__(self, "updates", updates)
        object.__setattr__(self, "model_seed", model_seed)
        object.__setattr__(self, "behavior_seed_base", behavior_seed_base)
        object.__setattr__(self, "potion_slots", potion_slots)
        object.__setattr__(self, "warm_start_behavior", warm_start_behavior)

    @property
    def behavior_seeds(self) -> tuple[int, ...]:
        return tuple(
            self.behavior_seed_base + index
            for index in range(self.root_count)
        )


def run_combat_training(
    config: CombatTrainingCommandConfig,
    *,
    bridge: CombatSessionBridge | None = None,
    run_bridge: CategoricalSessionBridge | None = None,
) -> dict[str, object]:
    """Run bounded online updates and publish the final active behavior."""

    if not isinstance(config, CombatTrainingCommandConfig):
        raise CombatTrainingCommandError("combat training config must be typed")
    active_bridge = bridge if bridge is not None else CombatSessionBridge.installed()
    if not isinstance(active_bridge, CombatSessionBridge):
        raise CombatTrainingCommandError("combat training bridge must be typed")
    profile = replace(
        CombatWinSessionProfile(),
        scorer=RaggedScorerConfig(
            value_head=config.policy_update.uses_value_baseline,
        ),
        objective=CombatWinObjectiveConfig(
            groups_per_update=config.root_count,
            policy_update=config.policy_update,
            all_loss_axis=config.all_loss_axis,
        ),
    )
    limits = replace(
        CombatWinSessionLimits(),
        owner_capacity=max(16, config.updates + 1),
    )
    warm_start, warm_start_training_kind = _recover_combat_warm_start(
        config.warm_start_behavior,
        active_bridge,
        run_bridge,
        config.behavior_seed_base,
    )
    session = CombatWinBatchSessionFactory(
        config.output,
        active_bridge,
        CombatWinBatchSessionConfig(
            expected_roots=config.root_count,
            max_roots=config.root_count,
            replicate_count=config.replicate_count,
            profile=profile,
            limits=limits,
            potion_lane=config.potion_lane,
            potion_slots=config.potion_slots,
        ),
    ).new_from_artifact_file(
        config.artifact,
        model_seed=config.model_seed,
        behavior_seeds=config.behavior_seeds,
        initial_scorer=(
            None if warm_start is None else warm_start.policies[0].frozen_scorer
        ),
        initial_scorer_actor_only=warm_start_training_kind == "run",
    )

    return _run_combat_training_session(
        config,
        session=session,
        profile=profile,
        behavior_seeds=config.behavior_seeds,
        warm_start_manifest_id=(
            None if warm_start is None else warm_start.manifest_id.digest.hex()
        ),
        warm_start_checkpoint_id=(
            None if warm_start is None else warm_start.checkpoint_id.digest.hex()
        ),
        warm_start_training_step=(
            None if warm_start is None else warm_start.training_step
        ),
        warm_start_training_kind=warm_start_training_kind,
    )


def _recover_combat_warm_start(
    behavior: Path | None,
    combat_bridge: CombatSessionBridge,
    run_bridge: CategoricalSessionBridge | None,
    behavior_seed: int,
) -> tuple[PublishedCombatBehavior | PublishedRunBehavior | None, str | None]:
    if behavior is None:
        return None, None
    from .published_run_behavior import (
        is_run_training_publication,
        recover_published_run_behavior,
    )

    if is_run_training_publication(behavior):
        active_run_bridge = (
            run_bridge
            if run_bridge is not None
            else CategoricalSessionBridge.installed()
        )
        if not isinstance(active_run_bridge, CategoricalSessionBridge):
            raise CombatTrainingCommandError(
                "run warm-start recovery bridge must be typed"
            )
        if active_run_bridge.semantic_schema != combat_bridge.semantic_schema:
            raise CombatTrainingCommandError(
                "run warm-start and combat training semantic schemas differ"
            )
        return (
            recover_published_run_behavior(
                behavior,
                active_run_bridge,
                (behavior_seed,),
            ),
            "run",
        )
    from .published_combat_behavior import recover_published_combat_behavior

    return (
        recover_published_combat_behavior(
            behavior,
            combat_bridge,
            CombatWinSessionLimits(),
            (behavior_seed,),
        ),
        "combat",
    )


def _run_combat_training_session(
    config: CombatTrainingCommandConfig,
    *,
    session: _CombatTrainingSession,
    profile: CombatWinSessionProfile,
    behavior_seeds: tuple[int, ...],
    warm_start_manifest_id: str | None,
    warm_start_checkpoint_id: str | None,
    warm_start_training_step: int | None,
    warm_start_training_kind: str | None = None,
    configuration_extra: Mapping[str, object] | None = None,
    completion_extra: Mapping[str, object] | None = None,
) -> dict[str, object]:
    """Journal and publish one already-constructed combat training session."""

    configuration = {
        "schema": COMBAT_TRAINING_SCHEMA,
        "kind": "configuration",
        "artifact": str(config.artifact),
        "artifact_sha256": _sha256(config.artifact),
        "artifact_bytes": session.artifact_byte_count,
        "root_count": config.root_count,
        "replicate_count": config.replicate_count,
        "updates": config.updates,
        "model_seed": config.model_seed,
        "behavior_seeds": behavior_seeds,
        "all_win_axis": profile.objective.all_win_axis.name,
        "all_loss_axis": profile.objective.all_loss_axis.name,
        "policy_update_rule": profile.objective.policy_update.rule.name,
        "policy_update_epochs": profile.objective.policy_update.epochs,
        "policy_clip_coefficient": (
            profile.objective.policy_update.clip_coefficient
        ),
        "policy_entropy_coefficient": (
            profile.objective.policy_update.entropy_coefficient
        ),
        "policy_max_grad_norm": profile.objective.policy_update.max_grad_norm,
        "policy_target_kl": profile.objective.policy_update.target_kl,
        "policy_value_loss_coefficient": (
            profile.objective.policy_update.value_loss_coefficient
        ),
        "potion_lane": config.potion_lane.value,
        "potion_slots": config.potion_slots,
        "initialization": (
            "random"
            if config.warm_start_behavior is None
            else "published-behavior"
        ),
        "warm_start_behavior": (
            None
            if config.warm_start_behavior is None
            else str(config.warm_start_behavior)
        ),
        "warm_start_manifest_id": warm_start_manifest_id,
        "warm_start_checkpoint_id": warm_start_checkpoint_id,
        "warm_start_training_step": warm_start_training_step,
        "warm_start_training_kind": warm_start_training_kind,
    }
    _extend_record(configuration, configuration_extra)

    total_wins = 0
    total_losses = 0
    total_unresolved = 0
    started = time.perf_counter()
    with (config.output / "training.jsonl").open(
        "x",
        encoding="utf-8",
        newline="\n",
    ) as journal:
        _write(journal, configuration)
        for generation in range(config.updates):
            generation_started = time.perf_counter()
            result = session.advance()
            elapsed = time.perf_counter() - generation_started
            wins = sum(root.wins for root in result.roots)
            losses = sum(root.losses for root in result.roots)
            total_wins += wins
            total_losses += losses
            unresolved = sum(root.unresolved for root in result.roots)
            total_unresolved += unresolved
            _write(journal, _generation(generation, result, elapsed))
            root_wins = ",".join(str(root.wins) for root in result.roots)
            root_objectives = ",".join(
                _selected_objective(
                    root,
                    result.training.all_win_axis,
                    result.training.all_loss_axis,
                )
                for root in result.roots
            )
            print(
                f"generation={generation} "
                f"step_before={result.active_training_step_before} "
                f"wins={wins} losses={losses} unresolved={unresolved} "
                f"signal_groups={result.training.signal_group_count} "
                f"status={result.training.status.name} "
                f"promoted={str(result.promoted).lower()} "
                f"decisions={result.training.decision_count} "
                f"optimizer_steps={result.training.optimizer_steps_applied} "
                f"loss={result.training.loss:.8g} "
                f"approx_kl={result.training.approximate_kl:.6g} "
                f"clip_fraction={result.training.clip_fraction:.6g} "
                f"entropy={result.training.entropy:.6g} "
                f"value_loss={result.training.value_loss:.6g} "
                f"root_wins={root_wins} root_objectives={root_objectives} "
                f"seconds={elapsed:.3f}",
                flush=True,
            )

        publication = session.publish_active_behavior()
        snapshot = session.runner.trainer.snapshot
        summary: dict[str, object] = {
            "schema": COMBAT_TRAINING_SCHEMA,
            "kind": "completed",
            "updates": config.updates,
            "optimizer_steps": snapshot.optimizer_steps,
            "deliveries": snapshot.deliveries,
            "no_update_deliveries": snapshot.no_update_deliveries,
            "total_wins": total_wins,
            "total_losses": total_losses,
            "total_unresolved": total_unresolved,
            "elapsed_seconds": time.perf_counter() - started,
            "final_manifest_id": publication.manifest_id.digest.hex(),
            "final_checkpoint_id": publication.checkpoint_id.digest.hex(),
        }
        _extend_record(summary, completion_extra)
        _write(journal, summary)
    return summary


def _extend_record(
    record: dict[str, object],
    extra: Mapping[str, object] | None,
) -> None:
    if extra is None:
        return
    overlap = record.keys() & extra.keys()
    if overlap:
        fields = ", ".join(sorted(overlap))
        raise CombatTrainingCommandError(
            f"combat training journal extension repeats fields: {fields}"
        )
    record.update(extra)


def _generation(
    index: int,
    result: CombatWinBatchGenerationResult,
    elapsed: float,
) -> dict[str, object]:
    return {
        "schema": COMBAT_TRAINING_SCHEMA,
        "kind": "generation",
        "generation": index,
        "active_training_step_before": result.active_training_step_before,
        "active_manifest_id_before": result.active_manifest_id_before.digest.hex(),
        "active_manifest_id_after": (
            result.promotion.manifest_id.digest.hex()
            if result.promotion is not None
            else result.active_manifest_id_before.digest.hex()
        ),
        "promoted": result.promoted,
        "status": result.training.status.name,
        "loss": result.training.loss,
        "signal_group_count": result.training.signal_group_count,
        "win_signal_group_count": result.training.win_signal_group_count,
        "terminal_hp_signal_group_count": (
            result.training.terminal_hp_signal_group_count
        ),
        "enemy_hp_progress_signal_group_count": (
            result.training.enemy_hp_progress_signal_group_count
        ),
        "decision_count": result.training.decision_count,
        "optimizer_steps_after": result.training.optimizer_steps_after,
        "optimizer_steps_applied": result.training.optimizer_steps_applied,
        "approximate_kl": result.training.approximate_kl,
        "clip_fraction": result.training.clip_fraction,
        "entropy": result.training.entropy,
        "value_loss": result.training.value_loss,
        "roots": tuple(
            _root(
                slot_index,
                root,
                result.training.all_win_axis,
                result.training.all_loss_axis,
            )
            for slot_index, root in enumerate(result.roots)
        ),
        "elapsed_seconds": elapsed,
    }


def _root(
    slot_index: int,
    root: CombatWinRootGenerationResult,
    all_win_axis: CombatAllWinAxis,
    all_loss_axis: CombatAllLossAxis,
) -> dict[str, object]:
    signals = root.signals
    return {
        "slot_index": slot_index,
        "root_id": root.root_id,
        "exact_combat_state_hash": root.exact_combat_state_hash,
        "wins": root.wins,
        "losses": root.losses,
        "unresolved": root.unresolved,
        "model_rounds": root.model_rounds,
        "transitions": root.transitions,
        "decision_count": signals.decision_count,
        "win_signal_replicates": signals.win.replicate_count,
        "terminal_hp_signal_replicates": signals.terminal_hp.replicate_count,
        "enemy_hp_progress_signal_replicates": (
            signals.enemy_hp_progress.replicate_count
        ),
        "potion_signal_replicates": signals.potion_retention.replicate_count,
        "selected_objective": _selected_objective(
            root,
            all_win_axis,
            all_loss_axis,
        ),
    }


def _selected_objective(
    root: CombatWinRootGenerationResult,
    all_win_axis: CombatAllWinAxis,
    all_loss_axis: CombatAllLossAxis,
) -> str:
    if root.signals.win.has_signal:
        return "win"
    if (
        all_win_axis is CombatAllWinAxis.TERMINAL_HP
        and root.wins == root.replicate_count
        and root.signals.terminal_hp.has_signal
    ):
        return "hp"
    if (
        all_loss_axis is CombatAllLossAxis.ENEMY_HP_PROGRESS
        and root.losses == root.replicate_count
        and root.signals.enemy_hp_progress.has_signal
    ):
        return "enemy-hp-progress"
    return "none"


def _write(journal: TextIO, value: dict[str, object]) -> None:
    journal.write(json.dumps(value, separators=(",", ":"), sort_keys=True))
    journal.write("\n")
    journal.flush()


def _sha256(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def _positive(value: int, name: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise CombatTrainingCommandError(f"{name} must be a positive integer")
    return value


def _seed(value: int, name: str) -> int:
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or not 0 <= value < 1 << 63
    ):
        raise CombatTrainingCommandError(f"{name} must be an integer in [0, 2^63)")
    return value


def _potion_slots_text(arguments: argparse.Namespace) -> str:
    lane = CombatPotionLane(arguments.potion_lane)
    if lane is CombatPotionLane.ALL:
        return "all"
    if lane is CombatPotionLane.NEVER:
        return "none"
    return "+".join(str(slot) for slot in arguments.potion_slot)


def _policy_update(name: str) -> CombatPolicyUpdateConfig:
    if name == "reinforce":
        return CombatPolicyUpdateConfig(
            rule=CombatPolicyUpdateRule.REINFORCE,
        )
    if name == "ppo-clip":
        return CombatPolicyUpdateConfig.ppo_clip()
    if name == "ppo-clip-value":
        return CombatPolicyUpdateConfig.ppo_clip_value()
    raise CombatTrainingCommandError("unsupported combat policy update")


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Train one shared behavior from an opaque combat-root batch.",
    )
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--roots", type=int, required=True)
    parser.add_argument("--replicates", type=int, default=8)
    parser.add_argument("--updates", type=int, required=True)
    parser.add_argument("--model-seed", type=int, default=0)
    parser.add_argument("--behavior-seed-base", type=int, default=1_000)
    parser.add_argument("--warm-start-behavior", type=Path)
    parser.add_argument(
        "--policy-update",
        choices=("reinforce", "ppo-clip", "ppo-clip-value"),
        default="reinforce",
    )
    parser.add_argument(
        "--all-loss-axis",
        choices=("none", "enemy-hp-progress"),
        default="none",
    )
    parser.add_argument(
        "--potion-lane",
        choices=tuple(lane.value for lane in CombatPotionLane),
        default=CombatPotionLane.ALL.value,
    )
    parser.add_argument("--potion-slot", action="append", type=int, default=[])
    return parser


def main() -> int:
    arguments = _parser().parse_args()
    summary = run_combat_training(
        CombatTrainingCommandConfig(
            artifact=arguments.artifact,
            output=arguments.output,
            root_count=arguments.roots,
            replicate_count=arguments.replicates,
            updates=arguments.updates,
            model_seed=arguments.model_seed,
            behavior_seed_base=arguments.behavior_seed_base,
            potion_lane=CombatPotionLane(arguments.potion_lane),
            potion_slots=tuple(arguments.potion_slot),
            warm_start_behavior=arguments.warm_start_behavior,
            policy_update=_policy_update(arguments.policy_update),
            all_loss_axis=(
                CombatAllLossAxis.ENEMY_HP_PROGRESS
                if arguments.all_loss_axis == "enemy-hp-progress"
                else CombatAllLossAxis.NONE
            ),
        )
    )
    print(
        "training_complete=true "
        f"optimizer_steps={summary['optimizer_steps']} "
        f"policy_update={arguments.policy_update} "
        f"all_loss_axis={arguments.all_loss_axis} "
        f"potion_lane={arguments.potion_lane} "
        f"potion_slots={_potion_slots_text(arguments)} "
        f"wins={summary['total_wins']} losses={summary['total_losses']} "
        f"seconds={summary['elapsed_seconds']:.3f} "
        f"output={arguments.output.resolve()}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
