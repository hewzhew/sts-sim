"""Held-out combat execution with no experience or training owners."""

from __future__ import annotations

import operator
from collections.abc import Sequence
from dataclasses import dataclass

from .combat_outcomes import (
    CombatGroupOutcomeAccumulator,
    CombatOutcomeError,
    CombatTerminalStepBatch,
    CompletedCombatGroup,
)
from .policy import BehaviorManifestId
from .torch_behavior import FrozenCategoricalTorchPolicy


class CombatEvaluationError(RuntimeError):
    """A held-out combat evaluation is malformed or incomplete."""


@dataclass(frozen=True)
class CombatEvaluationLimits:
    """Hard execution bounds for one held-out root."""

    max_model_rounds: int = 4_096
    max_transitions: int = 4_096

    def __post_init__(self) -> None:
        for name in ("max_model_rounds", "max_transitions"):
            object.__setattr__(
                self,
                name,
                _positive_integer(getattr(self, name), name),
            )


@dataclass(frozen=True)
class CombatEvaluationRootResult:
    """Terminal facts for every replicate of one exact held-out root."""

    group: CompletedCombatGroup
    model_rounds: int
    transitions: int

    def __post_init__(self) -> None:
        if not isinstance(self.group, CompletedCombatGroup):
            raise CombatEvaluationError(
                "combat evaluation root requires a completed combat group"
            )
        object.__setattr__(
            self,
            "model_rounds",
            _nonnegative_integer(self.model_rounds, "model_rounds"),
        )
        object.__setattr__(
            self,
            "transitions",
            _positive_integer(self.transitions, "transitions"),
        )

    @property
    def wins(self) -> int:
        return sum(outcome.won for outcome in self.group.outcomes)

    @property
    def losses(self) -> int:
        return len(self.group.outcomes) - self.wins


@dataclass(frozen=True)
class CombatHeldOutEvaluationResult:
    """One frozen manifest evaluated once across distinct exact roots."""

    behavior_manifest_id: BehaviorManifestId
    roots: tuple[CombatEvaluationRootResult, ...]

    def __post_init__(self) -> None:
        if not isinstance(self.behavior_manifest_id, BehaviorManifestId):
            raise CombatEvaluationError(
                "combat evaluation requires a typed behavior manifest id"
            )
        roots = tuple(self.roots)
        if not roots or not all(
            isinstance(root, CombatEvaluationRootResult) for root in roots
        ):
            raise CombatEvaluationError(
                "combat evaluation requires typed root results"
            )
        identities = tuple(
            (root.group.root_id, root.group.exact_combat_state_hash)
            for root in roots
        )
        if len(set(identities)) != len(identities):
            raise CombatEvaluationError(
                "combat evaluation repeated an exact root"
            )
        object.__setattr__(self, "roots", roots)

    @property
    def wins(self) -> int:
        return sum(root.wins for root in self.roots)

    @property
    def losses(self) -> int:
        return sum(root.losses for root in self.roots)


class CombatHeldOutEvaluator:
    """Evaluate fresh groups without retaining decisions or mutating behavior."""

    def __init__(
        self,
        source: object,
        *,
        slot_indices: Sequence[int],
        replicate_count: int,
        policies: Sequence[FrozenCategoricalTorchPolicy],
        max_roots: int,
        limits: CombatEvaluationLimits | None = None,
    ) -> None:
        if not callable(getattr(source, "combat_group", None)):
            raise CombatEvaluationError(
                "combat evaluation requires a combat-root source"
            )
        slots = tuple(
            _nonnegative_integer(slot, f"slot_indices[{index}]")
            for index, slot in enumerate(slot_indices)
        )
        if not slots or len(set(slots)) != len(slots):
            raise CombatEvaluationError(
                "combat evaluation requires distinct root slots"
            )
        root_bound = _positive_integer(max_roots, "max_roots")
        if len(slots) > root_bound:
            raise CombatEvaluationError(
                "combat evaluation roots exceed max_roots"
            )
        replicates = _positive_integer(replicate_count, "replicate_count")
        if replicates < 2:
            raise CombatEvaluationError(
                "combat evaluation requires at least two replicates"
            )
        frozen_policies = tuple(policies)
        if len(frozen_policies) != len(slots) or not all(
            isinstance(policy, FrozenCategoricalTorchPolicy)
            for policy in frozen_policies
        ):
            raise CombatEvaluationError(
                "combat evaluation requires one frozen policy per root"
            )
        manifest_ids = {
            policy.behavior_manifest_id for policy in frozen_policies
        }
        if len(manifest_ids) != 1:
            raise CombatEvaluationError(
                "combat evaluation policies must share one frozen manifest"
            )
        if len({id(policy.generator) for policy in frozen_policies}) != len(
            frozen_policies
        ):
            raise CombatEvaluationError(
                "combat evaluation requires independent policy RNG streams"
            )
        active_limits = CombatEvaluationLimits() if limits is None else limits
        if not isinstance(active_limits, CombatEvaluationLimits):
            raise CombatEvaluationError(
                "combat evaluation limits must be typed"
            )
        self.source = source
        self.slot_indices = slots
        self.replicate_count = replicates
        self.policies = frozen_policies
        self.max_roots = root_bound
        self.limits = active_limits
        self.behavior_manifest_id = next(iter(manifest_ids))
        self._started = False

    def evaluate(self) -> CombatHeldOutEvaluationResult:
        """Consume every declared root exactly once under the frozen manifest."""

        if self._started:
            raise CombatEvaluationError(
                "combat held-out evaluation is single-use"
            )
        self._started = True
        generator_states = tuple(
            policy.generator.get_state().clone() for policy in self.policies
        )
        roots: list[CombatEvaluationRootResult] = []
        observed_identities: set[tuple[object, object]] = set()
        try:
            for slot_index, policy in zip(
                self.slot_indices,
                self.policies,
                strict=True,
            ):
                group = self.source.combat_group(
                    slot_index,
                    self.replicate_count,
                )
                if getattr(group, "replicate_count", None) != self.replicate_count:
                    raise CombatEvaluationError(
                        "combat evaluation source changed the replicate count"
                    )
                identity = (
                    getattr(group, "root_id", None),
                    getattr(group, "exact_combat_state_hash", None),
                )
                if identity in observed_identities:
                    raise CombatEvaluationError(
                        "combat evaluation source repeated an exact root"
                    )
                observed_identities.add(identity)
                roots.append(
                    _evaluate_group(
                        group,
                        policy,
                        self.limits,
                    )
                )
        except Exception:
            for policy, state in zip(
                self.policies,
                generator_states,
                strict=True,
            ):
                policy.generator.set_state(state)
            raise
        return CombatHeldOutEvaluationResult(
            self.behavior_manifest_id,
            tuple(roots),
        )


def _evaluate_group(
    env: object,
    policy: FrozenCategoricalTorchPolicy,
    limits: CombatEvaluationLimits,
) -> CombatEvaluationRootResult:
    if getattr(env, "terminal_count", None) != 0:
        raise CombatEvaluationError(
            "combat evaluation requires a fresh group"
        )
    try:
        outcomes = CombatGroupOutcomeAccumulator(
            root_id=env.root_id,
            exact_combat_state_hash=env.exact_combat_state_hash,
            replicate_count=env.replicate_count,
        )
    except (AttributeError, CombatOutcomeError) as error:
        raise CombatEvaluationError(
            "combat evaluation source exposed an invalid root identity"
        ) from error
    model_rounds = 0
    transitions = 0
    while outcomes.terminal_count < env.replicate_count:
        while not env.ready:
            if model_rounds >= limits.max_model_rounds:
                raise CombatEvaluationError(
                    "combat evaluation exceeded its model-round limit"
                )
            decision_batch = env.decision_batch(semantic=True)
            choice = policy.choose(decision_batch)
            if choice.behavior_manifest_id != policy.behavior_manifest_id:
                raise CombatEvaluationError(
                    "combat evaluation policy changed behavior manifest"
                )
            env.choose(list(choice.ordinals))
            model_rounds += 1
        if transitions >= limits.max_transitions:
            raise CombatEvaluationError(
                "combat evaluation exceeded its transition limit"
            )
        step = env.step()
        try:
            outcomes.record(
                CombatTerminalStepBatch.from_bridge_step(
                    step,
                    replicate_count=env.replicate_count,
                )
            )
        except CombatOutcomeError as error:
            raise CombatEvaluationError(str(error)) from error
        transitions += 1
        if outcomes.terminal_count != env.terminal_count:
            raise CombatEvaluationError(
                "bridge and caller combat terminal counts diverged"
            )
    return CombatEvaluationRootResult(
        group=outcomes.finish(),
        model_rounds=model_rounds,
        transitions=transitions,
    )


def _positive_integer(value: object, name: str) -> int:
    normalized = _nonnegative_integer(value, name)
    if normalized == 0:
        raise CombatEvaluationError(f"{name} must be positive")
    return normalized


def _nonnegative_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise CombatEvaluationError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise CombatEvaluationError(f"{name} must be an integer") from error
    if normalized < 0:
        raise CombatEvaluationError(f"{name} must be non-negative")
    return normalized
