"""Bounded command-line composition for real multi-root combat training."""

from __future__ import annotations

import argparse
import hashlib
import json
import time
from dataclasses import dataclass, replace
from pathlib import Path
from typing import TextIO

from .combat_objective import CombatWinObjectiveConfig
from .torch_combat_batch_generation import (
    CombatWinBatchGenerationResult,
    CombatWinRootGenerationResult,
)
from .torch_combat_batch_session import CombatWinBatchSessionFactory
from .torch_combat_session_config import (
    CombatSessionBridge,
    CombatWinBatchSessionConfig,
    CombatWinSessionLimits,
    CombatWinSessionProfile,
)


_SCHEMA = "sts-learning-combat-training-v1"


class CombatTrainingCommandError(RuntimeError):
    """A bounded combat training command is malformed."""


@dataclass(frozen=True)
class CombatTrainingCommandConfig:
    artifact: Path
    output: Path
    root_count: int
    replicate_count: int
    updates: int
    model_seed: int
    behavior_seed_base: int

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
        root_count = _positive(self.root_count, "root_count")
        replicate_count = _positive(self.replicate_count, "replicate_count")
        updates = _positive(self.updates, "updates")
        model_seed = _seed(self.model_seed, "model_seed")
        behavior_seed_base = _seed(
            self.behavior_seed_base,
            "behavior_seed_base",
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
) -> dict[str, object]:
    """Run bounded online updates and publish the final active behavior."""

    if not isinstance(config, CombatTrainingCommandConfig):
        raise CombatTrainingCommandError("combat training config must be typed")
    active_bridge = bridge if bridge is not None else CombatSessionBridge.installed()
    if not isinstance(active_bridge, CombatSessionBridge):
        raise CombatTrainingCommandError("combat training bridge must be typed")
    profile = replace(
        CombatWinSessionProfile(),
        objective=CombatWinObjectiveConfig(groups_per_update=config.root_count),
    )
    limits = replace(
        CombatWinSessionLimits(),
        owner_capacity=max(16, config.updates + 1),
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
        ),
    ).new_from_artifact_file(
        config.artifact,
        model_seed=config.model_seed,
        behavior_seeds=config.behavior_seeds,
    )

    total_wins = 0
    total_losses = 0
    started = time.perf_counter()
    with (config.output / "training.jsonl").open(
        "x",
        encoding="utf-8",
        newline="\n",
    ) as journal:
        _write(
            journal,
            {
                "schema": _SCHEMA,
                "kind": "configuration",
                "artifact": str(config.artifact),
                "artifact_sha256": _sha256(config.artifact),
                "artifact_bytes": session.artifact_byte_count,
                "root_count": config.root_count,
                "replicate_count": config.replicate_count,
                "updates": config.updates,
                "model_seed": config.model_seed,
                "behavior_seeds": config.behavior_seeds,
                "all_win_axis": profile.objective.all_win_axis.name,
            },
        )
        for generation in range(config.updates):
            generation_started = time.perf_counter()
            result = session.advance()
            elapsed = time.perf_counter() - generation_started
            wins = sum(root.wins for root in result.roots)
            losses = sum(root.losses for root in result.roots)
            total_wins += wins
            total_losses += losses
            _write(journal, _generation(generation, result, elapsed))
            root_wins = ",".join(str(root.wins) for root in result.roots)
            root_objectives = ",".join(
                _selected_objective(root) for root in result.roots
            )
            print(
                f"generation={generation} "
                f"step_before={result.active_training_step_before} "
                f"wins={wins} losses={losses} "
                f"signal_groups={result.training.signal_group_count} "
                f"status={result.training.status.name} "
                f"promoted={str(result.promoted).lower()} "
                f"decisions={result.training.decision_count} "
                f"loss={result.training.loss:.8g} "
                f"root_wins={root_wins} root_objectives={root_objectives} "
                f"seconds={elapsed:.3f}",
                flush=True,
            )

        publication = session.publish_active_behavior()
        snapshot = session.runner.trainer.snapshot
        summary: dict[str, object] = {
            "schema": _SCHEMA,
            "kind": "completed",
            "updates": config.updates,
            "optimizer_steps": snapshot.optimizer_steps,
            "deliveries": snapshot.deliveries,
            "no_update_deliveries": snapshot.no_update_deliveries,
            "total_wins": total_wins,
            "total_losses": total_losses,
            "elapsed_seconds": time.perf_counter() - started,
            "final_manifest_id": publication.manifest_id.digest.hex(),
            "final_checkpoint_id": publication.checkpoint_id.digest.hex(),
        }
        _write(journal, summary)
    return summary


def _generation(
    index: int,
    result: CombatWinBatchGenerationResult,
    elapsed: float,
) -> dict[str, object]:
    return {
        "schema": _SCHEMA,
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
        "decision_count": result.training.decision_count,
        "optimizer_steps_after": result.training.optimizer_steps_after,
        "roots": tuple(
            _root(slot_index, root)
            for slot_index, root in enumerate(result.roots)
        ),
        "elapsed_seconds": elapsed,
    }


def _root(
    slot_index: int,
    root: CombatWinRootGenerationResult,
) -> dict[str, object]:
    signals = root.signals
    return {
        "slot_index": slot_index,
        "root_id": root.root_id,
        "exact_combat_state_hash": root.exact_combat_state_hash,
        "wins": root.wins,
        "losses": root.losses,
        "model_rounds": root.model_rounds,
        "transitions": root.transitions,
        "decision_count": signals.decision_count,
        "win_signal_replicates": signals.win.replicate_count,
        "terminal_hp_signal_replicates": signals.terminal_hp.replicate_count,
        "potion_signal_replicates": signals.potion_retention.replicate_count,
        "selected_objective": _selected_objective(root),
    }


def _selected_objective(root: CombatWinRootGenerationResult) -> str:
    if root.signals.win.has_signal:
        return "win"
    if root.wins == root.replicate_count and root.signals.terminal_hp.has_signal:
        return "hp"
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
        )
    )
    print(
        "training_complete=true "
        f"optimizer_steps={summary['optimizer_steps']} "
        f"wins={summary['total_wins']} losses={summary['total_losses']} "
        f"seconds={summary['elapsed_seconds']:.3f} "
        f"output={arguments.output.resolve()}",
        flush=True,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
