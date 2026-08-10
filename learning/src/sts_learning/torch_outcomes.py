"""On-policy policy objectives over exact terminal experience."""

from __future__ import annotations

import math
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Protocol

import torch
from torch import Tensor

from .attempts import CompletedAttemptExperience
from .credit_assignment import (
    CreditAssignmentError,
    matched_episode_floor_context_leave_one_out_advantages,
    matched_floor_context_leave_one_out_advantages,
    matched_floor_leave_one_out_advantages,
)
from .combat_experience import (
    CombatDecisionExperienceBatch,
    CompletedCombatGroupExperience,
)
from .combat_objective import (
    CombatAllLossAxis,
    CombatAllWinAxis,
    CombatPolicyUpdateConfig,
    CombatPolicyUpdateRule,
    CombatWinObjectiveConfig,
)
from .combat_outcomes import CombatTerminalKind
from .combat_rollout import (
    COMBAT_ROLLOUT_VALUE_HEAD_WIDTH,
    CombatRolloutAxis,
    CombatRolloutError,
    build_complete_combat_rollout,
)
from .experience import DecisionExperienceBatch, ExperienceError
from .manifests import BehaviorManifestRegistry, BehaviorRuleBinding
from .policy import BehaviorManifestId, SelectionProbability
from .run_rollout import RunRolloutError, build_complete_run_rollout
from .semantic_concat import (
    SemanticBatchConcatLimits,
    concatenate_semantic_decision_batches,
)
from .terminal_returns import (
    FloorProgressReturnConfig,
    RunDecisionScope,
    RunPolicyUpdateConfig,
    RunPolicyUpdateRule,
    TerminalAdvantageMode,
    floor_progress_terminal_return,
    terminal_return_advantages,
)
from .torch_policy import (
    RaggedActorCriticOutput,
    RaggedCandidateLogits,
    RaggedMultiActorCriticOutput,
    RaggedCategoricalPolicyConfig,
)


class TorchOutcomeError(ValueError):
    """Complete outcome experience does not satisfy the policy objective."""


class CandidatePolicyScorer(Protocol):
    def __call__(self, decision_batch: Mapping[str, object]) -> RaggedCandidateLogits: ...


@dataclass(frozen=True)
class OnPolicyTerminalLoss:
    """One differentiable policy loss plus exact behavior provenance."""

    value: Tensor
    attempt_count: int
    decision_count: int
    actor_decision_count: int
    behavior_manifest_ids: tuple[tuple[BehaviorManifestId, ...], ...]
    selection_probabilities: tuple[tuple[SelectionProbability, ...], ...]
    approximate_kl: float
    clip_fraction: float
    entropy: float
    value_loss: float
    value_clip_fraction: float
    explained_variance: float | None
    actor_advantages: tuple[float, ...]
    critic_predictions: tuple[float, ...] | None
    value_diagnostics: RunValueDiagnostics | None


@dataclass(frozen=True)
class AttemptEqualSignalSummary:
    """Compact decision signal statistics under equal total attempt weight."""

    decision_count: int
    negative_decisions: int
    zero_decisions: int
    positive_decisions: int
    negative_weight: float
    zero_weight: float
    positive_weight: float
    weighted_mean: float
    weighted_standard_deviation: float
    minimum: float
    maximum: float


@dataclass(frozen=True)
class RunValueDiagnostics:
    """Frozen rollout diagnostics for one whole-run actor-critic objective."""

    actor_advantage: AttemptEqualSignalSummary | None
    critic_prediction: AttemptEqualSignalSummary
    return_to_go_target: AttemptEqualSignalSummary
    critic_residual: AttemptEqualSignalSummary
    actor_decision_count: int
    forced_decision_count: int
    explained_variance: float | None


@dataclass(frozen=True)
class OnPolicyCombatWinLoss:
    """One win-first combat loss without an HP/potion exchange rate.

    A root with mixed wins and losses uses only win advantage.  An all-win
    root may use the configured all-win terminal-HP advantage, so solved early
    combats can keep learning resource preservation. An exact all-loss root may
    use enemy-HP progress only when explicitly configured. Potion retention
    remains evidence only.
    """

    value: Tensor
    group_count: int
    signal_group_count: int
    win_signal_group_count: int
    terminal_hp_signal_group_count: int
    enemy_hp_progress_signal_group_count: int
    replicate_count: int
    decision_count: int
    behavior_manifest_ids: tuple[BehaviorManifestId, ...]
    selection_probabilities: tuple[tuple[SelectionProbability, ...], ...]
    approximate_kl: float
    clip_fraction: float
    entropy: float
    value_loss: float
    actor_advantages: tuple[float, ...]


@dataclass(frozen=True)
class _CombatPolicyLoss:
    value: Tensor
    approximate_kl: float
    clip_fraction: float
    entropy: float
    value_loss: float
    actor_advantages: tuple[float, ...]


@dataclass(frozen=True)
class _RunPolicyLoss:
    value: Tensor
    approximate_kl: float
    clip_fraction: float
    entropy: float
    value_loss: float
    value_clip_fraction: float
    explained_variance: float | None
    actor_decision_count: int
    actor_advantages: tuple[float, ...]
    critic_predictions: tuple[float, ...] | None
    value_diagnostics: RunValueDiagnostics | None


def on_policy_terminal_loss(
    scorer: CandidatePolicyScorer,
    attempts: Sequence[CompletedAttemptExperience],
    registry: BehaviorManifestRegistry,
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    return_config: FloorProgressReturnConfig,
    advantage_mode: TerminalAdvantageMode,
    decision_scope: RunDecisionScope = RunDecisionScope.ALL,
    *,
    expected_behavior_rule: BehaviorRuleBinding | None = None,
    update_config: RunPolicyUpdateConfig = RunPolicyUpdateConfig(),
    require_matching_propensities: bool = True,
    fixed_actor_advantages: Sequence[float] | None = None,
    fixed_value_predictions: Sequence[float] | None = None,
) -> OnPolicyTerminalLoss:
    """Apply one exact REINFORCE or decision-local GAE-PPO objective.

    Every complete attempt contributes equal total weight regardless of its
    length. Return sign controls the relative probability direction. A
    single-candidate decision has no actor gradient; value PPO still trains
    its critic row.
    """

    if not callable(scorer):
        raise TorchOutcomeError("candidate policy scorer must be callable")
    if not isinstance(registry, BehaviorManifestRegistry):
        raise TorchOutcomeError("policy objective requires a behavior manifest registry")
    if not isinstance(concat_limits, SemanticBatchConcatLimits):
        raise TorchOutcomeError("policy objective requires semantic concat limits")
    if not isinstance(policy_config, RaggedCategoricalPolicyConfig):
        raise TorchOutcomeError("policy objective requires categorical policy config")
    if not isinstance(return_config, FloorProgressReturnConfig):
        raise TorchOutcomeError("policy objective requires terminal return config")
    if not isinstance(advantage_mode, TerminalAdvantageMode):
        raise TorchOutcomeError("policy objective requires typed advantage mode")
    if not isinstance(decision_scope, RunDecisionScope):
        raise TorchOutcomeError("policy objective requires typed decision scope")
    if expected_behavior_rule is None:
        expected_behavior_rule = policy_config.behavior_rule
    if not isinstance(expected_behavior_rule, BehaviorRuleBinding):
        raise TorchOutcomeError("policy objective behavior rule must be typed")
    if not isinstance(update_config, RunPolicyUpdateConfig):
        raise TorchOutcomeError("policy objective requires typed run update config")
    if type(require_matching_propensities) is not bool:
        raise TorchOutcomeError("run policy propensity check must be bool")
    if (
        update_config.uses_value_baseline
        and advantage_mode is not TerminalAdvantageMode.DECISION_LOCAL_GAE
    ):
        raise TorchOutcomeError("run value PPO requires decision-local GAE advantage")
    if fixed_actor_advantages is not None and not update_config.uses_value_baseline:
        raise TorchOutcomeError("fixed run advantages require a value baseline")
    if fixed_value_predictions is not None and not update_config.uses_value_baseline:
        raise TorchOutcomeError("fixed run values require a value baseline")
    normalized = tuple(attempts)
    if not normalized:
        raise TorchOutcomeError("policy objective requires at least one complete attempt")
    if not all(isinstance(attempt, CompletedAttemptExperience) for attempt in normalized):
        raise TorchOutcomeError("policy objective accepts only complete attempts")
    progress_presence = {
        batch.run_progress is not None
        for attempt in normalized
        for batch in attempt.batches
    }
    rollout = None
    if update_config.uses_value_baseline:
        if progress_presence != {True}:
            raise TorchOutcomeError(
                "run value PPO requires complete decision-time progress"
            )
        try:
            rollout = build_complete_run_rollout(normalized, return_config)
        except RunRolloutError as error:
            raise TorchOutcomeError(str(error)) from error
    matched_advantages = None
    advantages = None
    if update_config.uses_value_baseline:
        pass
    elif advantage_mode in (
        TerminalAdvantageMode.MATCHED_FLOOR_LEAVE_ONE_OUT,
        TerminalAdvantageMode.MATCHED_FLOOR_CONTEXT_LEAVE_ONE_OUT,
        TerminalAdvantageMode.MATCHED_EPISODE_FLOOR_CONTEXT_LEAVE_ONE_OUT,
    ):
        try:
            if advantage_mode is TerminalAdvantageMode.MATCHED_FLOOR_LEAVE_ONE_OUT:
                matched_advantages = matched_floor_leave_one_out_advantages(
                    normalized,
                    return_config,
                )
            elif (
                advantage_mode
                is TerminalAdvantageMode.MATCHED_FLOOR_CONTEXT_LEAVE_ONE_OUT
            ):
                matched_advantages = matched_floor_context_leave_one_out_advantages(
                    normalized,
                    return_config,
                )
            else:
                matched_advantages = (
                    matched_episode_floor_context_leave_one_out_advantages(
                        normalized,
                        return_config,
                    )
                )
        except CreditAssignmentError as error:
            raise TorchOutcomeError(str(error)) from error
    else:
        advantages = terminal_return_advantages(
            tuple(
                floor_progress_terminal_return(attempt.terminal, return_config)
                for attempt in normalized
            ),
            advantage_mode,
        )

    behavior_ids: list[tuple[BehaviorManifestId, ...]] = []
    probability_evidence: list[tuple[SelectionProbability, ...]] = []
    payloads: list[Mapping[str, object]] = []
    selected_ordinals: list[int] = []
    targets: list[float] = []
    value_weights: list[float] = []
    actor_weights: list[float] = []
    total_decisions = 0
    for attempt_index, attempt in enumerate(normalized):
        retained_decisions = sum(batch.decision_count for batch in attempt.batches)
        if attempt.decision_count != retained_decisions or retained_decisions <= 0:
            raise TorchOutcomeError(
                "complete attempt decision count disagrees with retained batches"
            )

        scoped_batches: list[tuple[int, DecisionExperienceBatch, tuple[int, ...]]] = []
        scoped_decisions = 0
        for batch_index, batch in enumerate(attempt.batches):
            _validate_batch(batch)
            row_indices = _decision_scope_rows(batch, decision_scope)
            if not row_indices:
                continue
            try:
                scoped = (
                    batch
                    if len(row_indices) == batch.decision_count
                    else batch.select_rows(row_indices)
                )
            except ExperienceError as error:
                raise TorchOutcomeError(
                    "cannot select configured whole-run decision scope"
                ) from error
            scoped_batches.append((batch_index, scoped, row_indices))
            scoped_decisions += scoped.decision_count
        if scoped_decisions == 0:
            raise TorchOutcomeError(
                "complete attempt has no decisions in the configured scope"
            )
        scoped_actor_decisions = scoped_decisions
        if rollout is not None:
            scoped_actor_decisions = 0
            for batch_index, _batch, row_indices in scoped_batches:
                if row_indices != (0,):
                    raise TorchOutcomeError(
                        "typed run rollout scope is not one-row batch aligned"
                    )
                scoped_actor_decisions += int(
                    rollout.attempts[attempt_index]
                    .rows[batch_index]
                    .actor_eligible
                )

        attempt_behavior_ids: list[BehaviorManifestId] = []
        attempt_probabilities: list[SelectionProbability] = []
        for batch_index, batch, row_indices in scoped_batches:
            try:
                manifest = registry.resolve(batch.behavior_manifest_id)
            except ValueError as error:
                raise TorchOutcomeError(
                    "complete attempt references an unknown behavior manifest"
                ) from error
            if manifest.behavior_rule != expected_behavior_rule:
                raise TorchOutcomeError(
                    "complete attempt behavior rule conflicts with policy config"
                )
            attempt_behavior_ids.append(batch.behavior_manifest_id)
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            attempt_probabilities.extend(batch.selection_probabilities)
            if rollout is not None:
                rollout_row = rollout.attempts[attempt_index].rows[batch_index]
                batch_targets = (rollout_row.return_to_go,)
            elif matched_advantages is None:
                assert advantages is not None
                batch_targets = (advantages[attempt_index],) * batch.decision_count
            else:
                original_targets = matched_advantages[attempt_index][batch_index]
                batch_targets = tuple(original_targets[row] for row in row_indices)
                if len(batch_targets) != batch.decision_count:
                    raise TorchOutcomeError(
                        "matched targets are misaligned with decision rows"
                    )
            targets.extend(batch_targets)
            value_weights.extend(
                [1.0 / (len(normalized) * scoped_decisions)]
                * batch.decision_count
            )
            if rollout is None:
                actor_weights.extend(
                    [1.0 / (len(normalized) * scoped_decisions)]
                    * batch.decision_count
                )
            else:
                actor_weights.append(
                    0.0
                    if not rollout_row.actor_eligible
                    or scoped_actor_decisions == 0
                    else 1.0
                    / (len(normalized) * scoped_actor_decisions)
                )
        behavior_ids.append(tuple(attempt_behavior_ids))
        probability_evidence.append(tuple(attempt_probabilities))
        total_decisions += scoped_decisions

    policy_loss = _run_policy_weighted_loss(
        scorer=scorer,
        payloads=payloads,
        selected_ordinals=selected_ordinals,
        selection_probabilities=tuple(
            probability
            for attempt_probabilities in probability_evidence
            for probability in attempt_probabilities
        ),
        targets=targets,
        value_weights=value_weights,
        actor_weights=actor_weights,
        concat_limits=concat_limits,
        policy_config=policy_config,
        update_config=update_config,
        require_matching_propensities=require_matching_propensities,
        fixed_actor_advantages=fixed_actor_advantages,
        fixed_value_predictions=fixed_value_predictions,
    )
    return OnPolicyTerminalLoss(
        value=policy_loss.value,
        attempt_count=len(normalized),
        decision_count=total_decisions,
        actor_decision_count=policy_loss.actor_decision_count,
        behavior_manifest_ids=tuple(behavior_ids),
        selection_probabilities=tuple(probability_evidence),
        approximate_kl=policy_loss.approximate_kl,
        clip_fraction=policy_loss.clip_fraction,
        entropy=policy_loss.entropy,
        value_loss=policy_loss.value_loss,
        value_clip_fraction=policy_loss.value_clip_fraction,
        explained_variance=policy_loss.explained_variance,
        actor_advantages=policy_loss.actor_advantages,
        critic_predictions=policy_loss.critic_predictions,
        value_diagnostics=policy_loss.value_diagnostics,
    )


def on_policy_combat_win_loss(
    scorer: CandidatePolicyScorer,
    groups: Sequence[CompletedCombatGroupExperience],
    registry: BehaviorManifestRegistry,
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    objective_config: CombatWinObjectiveConfig,
    *,
    require_matching_propensities: bool = True,
    fixed_actor_advantages: Sequence[float] | None = None,
) -> OnPolicyCombatWinLoss:
    """Apply same-root win-first advantages with a typed all-win fallback.

    Every group has equal total weight. Inside a group, every replicate has
    equal total weight regardless of combat length, and its weight is split
    equally across only that replicate's retained decisions. A group with any
    win variation uses only win advantage. An all-win group uses terminal HP
    only when configured and that axis varies. An all-loss group uses enemy-HP
    progress only when explicitly configured, every terminal is an exact loss,
    and that axis varies. Potion retention is deliberately absent, so there is
    no exchange rate among the selected axes and potions.
    """

    if not callable(scorer):
        raise TorchOutcomeError("candidate policy scorer must be callable")
    if not isinstance(registry, BehaviorManifestRegistry):
        raise TorchOutcomeError(
            "combat win objective requires a behavior manifest registry"
        )
    if not isinstance(concat_limits, SemanticBatchConcatLimits):
        raise TorchOutcomeError(
            "combat win objective requires semantic concat limits"
        )
    if not isinstance(policy_config, RaggedCategoricalPolicyConfig):
        raise TorchOutcomeError(
            "combat win objective requires categorical policy config"
        )
    if not isinstance(objective_config, CombatWinObjectiveConfig):
        raise TorchOutcomeError("combat win objective requires typed objective config")
    if type(require_matching_propensities) is not bool:
        raise TorchOutcomeError(
            "combat win objective propensity check must be bool"
        )
    if (
        fixed_actor_advantages is not None
        and not objective_config.policy_update.uses_value_baseline
    ):
        raise TorchOutcomeError(
            "fixed combat advantages require a value baseline"
        )
    normalized = tuple(groups)
    if not normalized:
        raise TorchOutcomeError(
            "combat win objective requires at least one complete group"
        )

    seen_roots: set[tuple[str, str]] = set()
    behavior_ids: list[BehaviorManifestId] = []
    probability_evidence: list[tuple[SelectionProbability, ...]] = []
    payloads: list[Mapping[str, object]] = []
    selected_ordinals: list[int] = []
    targets: list[float] = []
    weights: list[float] = []
    value_head_indices: list[int] = []
    total_replicates = 0
    total_decisions = 0
    win_signal_groups = 0
    terminal_hp_signal_groups = 0
    enemy_hp_progress_signal_groups = 0

    for group in normalized:
        if not isinstance(group, CompletedCombatGroupExperience):
            raise TorchOutcomeError("combat win objective accepts only complete groups")
        root = (group.root_id, group.exact_combat_state_hash)
        if root in seen_roots:
            raise TorchOutcomeError("combat win objective repeats an exact root")
        seen_roots.add(root)
        try:
            manifest = registry.resolve(group.behavior_manifest_id)
        except ValueError as error:
            raise TorchOutcomeError(
                "combat group references an unknown behavior manifest"
            ) from error
        if manifest.behavior_rule != policy_config.behavior_rule:
            raise TorchOutcomeError(
                "combat group behavior rule conflicts with policy config"
            )

        advantages = group.grouped_advantages()
        if advantages.win_has_signal:
            selected_advantages = advantages.win
            selected_rollout_axis = CombatRolloutAxis.WIN
            selected_returns = tuple(
                1.0 if outcome.won else 0.0
                for outcome in group.outcomes.outcomes
            )
            win_signal_groups += 1
        elif (
            objective_config.all_win_axis is CombatAllWinAxis.TERMINAL_HP
            and all(outcome.won for outcome in group.outcomes.outcomes)
            and advantages.terminal_hp_has_signal
        ):
            selected_advantages = advantages.terminal_hp
            selected_rollout_axis = CombatRolloutAxis.PLAYER_HP_CHANGE
            start_hp = group.outcomes.outcomes[0].start_hp
            selected_returns = tuple(
                outcome.final_hp / start_hp
                for outcome in group.outcomes.outcomes
            )
            terminal_hp_signal_groups += 1
        elif (
            objective_config.all_loss_axis
            is CombatAllLossAxis.ENEMY_HP_PROGRESS
            and all(
                outcome.terminal_kind is CombatTerminalKind.LOSS
                for outcome in group.outcomes.outcomes
            )
            and advantages.enemy_hp_progress_has_signal
        ):
            selected_advantages = advantages.enemy_hp_progress
            selected_rollout_axis = CombatRolloutAxis.ENEMY_HP_CHANGE
            selected_returns = tuple(
                1.0 - outcome.enemy_final_hp / outcome.enemy_start_hp
                for outcome in group.outcomes.outcomes
            )
            enemy_hp_progress_signal_groups += 1
        else:
            selected_advantages = advantages.win
            selected_rollout_axis = CombatRolloutAxis.WIN
            selected_returns = tuple(
                1.0 if outcome.won else 0.0
                for outcome in group.outcomes.outcomes
            )
        replicate_count = len(group.outcomes.outcomes)
        decision_counts = [0] * replicate_count
        group_probabilities: list[SelectionProbability] = []
        for batch in group.batches:
            _validate_combat_batch(batch)
            for replicate_index in batch.replicate_indices:
                decision_counts[replicate_index] += 1
        if any(count == 0 for count in decision_counts):
            raise TorchOutcomeError(
                "combat win objective requires a retained decision for every replicate"
            )

        rollout_batches = None
        if objective_config.policy_update.uses_value_baseline:
            try:
                rollout_batches = build_complete_combat_rollout(group).batches
            except CombatRolloutError as error:
                raise TorchOutcomeError(str(error)) from error

        for batch_index, batch in enumerate(group.batches):
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            group_probabilities.extend(batch.selection_probabilities)
            rollout_batch = (
                None if rollout_batches is None else rollout_batches[batch_index]
            )
            for row_index, replicate_index in enumerate(batch.replicate_indices):
                targets.append(
                    rollout_batch.rows[row_index].return_to_go(
                        selected_rollout_axis
                    )
                    if objective_config.policy_update.uses_value_baseline
                    else selected_advantages[replicate_index]
                )
                weights.append(
                    1.0
                    / (
                        len(normalized)
                        * replicate_count
                        * decision_counts[replicate_index]
                    )
                )
                value_head_indices.append(int(selected_rollout_axis))
        behavior_ids.append(group.behavior_manifest_id)
        probability_evidence.append(tuple(group_probabilities))
        total_replicates += replicate_count
        total_decisions += group.decision_count

    policy_loss = _combat_policy_weighted_loss(
        scorer=scorer,
        payloads=payloads,
        selected_ordinals=selected_ordinals,
        selection_probabilities=tuple(
            probability
            for group_probabilities in probability_evidence
            for probability in group_probabilities
        ),
        targets=targets,
        weights=weights,
        value_head_indices=value_head_indices,
        concat_limits=concat_limits,
        policy_config=policy_config,
        update_config=objective_config.policy_update,
        require_matching_propensities=require_matching_propensities,
        fixed_actor_advantages=fixed_actor_advantages,
    )
    return OnPolicyCombatWinLoss(
        value=policy_loss.value,
        group_count=len(normalized),
        signal_group_count=(
            win_signal_groups
            + terminal_hp_signal_groups
            + enemy_hp_progress_signal_groups
        ),
        win_signal_group_count=win_signal_groups,
        terminal_hp_signal_group_count=terminal_hp_signal_groups,
        enemy_hp_progress_signal_group_count=(
            enemy_hp_progress_signal_groups
        ),
        replicate_count=total_replicates,
        decision_count=total_decisions,
        behavior_manifest_ids=tuple(behavior_ids),
        selection_probabilities=tuple(probability_evidence),
        approximate_kl=policy_loss.approximate_kl,
        clip_fraction=policy_loss.clip_fraction,
        entropy=policy_loss.entropy,
        value_loss=policy_loss.value_loss,
        actor_advantages=policy_loss.actor_advantages,
    )


def _decision_scope_rows(
    batch: DecisionExperienceBatch,
    scope: RunDecisionScope,
) -> tuple[int, ...]:
    if scope is RunDecisionScope.ALL:
        return tuple(range(batch.decision_count))
    if batch.run_progress is None:
        raise TorchOutcomeError(
            "strategic decision scope requires decision-time run progress"
        )
    if len(batch.run_progress) != batch.decision_count:
        raise TorchOutcomeError("decision scope progress rows are misaligned")
    return tuple(
        row
        for row, progress in enumerate(batch.run_progress)
        if not progress.is_combat
    )


def _validate_batch(batch: object) -> None:
    if not isinstance(batch, DecisionExperienceBatch):
        raise TorchOutcomeError(
            "complete attempt batches must be DecisionExperienceBatch values"
        )
    if batch.decision_count != len(batch.selected_ordinals):
        raise TorchOutcomeError("decision batch ordinals are misaligned")
    if batch.decision_count != len(batch.selection_probabilities):
        raise TorchOutcomeError(
            "decision batch selection probabilities are misaligned"
        )
    if not all(
        isinstance(probability, SelectionProbability)
        for probability in batch.selection_probabilities
    ):
        raise TorchOutcomeError(
            "decision batch selection probabilities must be typed"
        )


def _validate_combat_batch(batch: object) -> None:
    if not isinstance(batch, CombatDecisionExperienceBatch):
        raise TorchOutcomeError(
            "complete combat groups must contain CombatDecisionExperienceBatch values"
        )
    if batch.decision_count != len(batch.replicate_indices):
        raise TorchOutcomeError("combat decision batch replicates are misaligned")
    if batch.decision_count != len(batch.selected_ordinals):
        raise TorchOutcomeError("combat decision batch ordinals are misaligned")
    if batch.decision_count != len(batch.selection_probabilities):
        raise TorchOutcomeError(
            "combat decision batch selection probabilities are misaligned"
        )
    if not all(
        isinstance(probability, SelectionProbability)
        for probability in batch.selection_probabilities
    ):
        raise TorchOutcomeError(
            "combat decision batch selection probabilities must be typed"
        )


def _on_policy_weighted_loss(
    *,
    scorer: CandidatePolicyScorer,
    payloads: Sequence[Mapping[str, object]],
    selected_ordinals: Sequence[int],
    selection_probabilities: Sequence[SelectionProbability],
    targets: Sequence[float],
    weights: Sequence[float],
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    objective_name: str,
) -> Tensor:
    decision_count = len(selected_ordinals)
    if not (
        len(selection_probabilities)
        == len(targets)
        == len(weights)
        == decision_count
        > 0
    ):
        raise TorchOutcomeError(f"{objective_name} objective rows are misaligned")
    if not all(math.isfinite(target) for target in targets):
        raise TorchOutcomeError(f"{objective_name} objective targets must be finite")
    if not all(math.isfinite(weight) and weight > 0.0 for weight in weights):
        raise TorchOutcomeError(f"{objective_name} objective weights must be positive")

    combined = concatenate_semantic_decision_batches(payloads, concat_limits)
    logits = scorer(combined)
    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchOutcomeError(
            "candidate policy scorer must return RaggedCandidateLogits"
        )
    selected_log_probabilities = _selected_log_probabilities(
        logits,
        selected_ordinals,
        policy_config,
    )
    _require_matching_propensities(
        logits,
        selected_ordinals,
        selection_probabilities,
        policy_config,
    )
    target_tensor = torch.as_tensor(
        targets,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    weight_tensor = torch.as_tensor(
        weights,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    terms = -target_tensor * selected_log_probabilities
    if not bool(torch.all(torch.isfinite(terms))):
        raise TorchOutcomeError(f"{objective_name} policy loss terms must be finite")
    return torch.sum(terms * weight_tensor)


def _run_policy_weighted_loss(
    *,
    scorer: CandidatePolicyScorer,
    payloads: Sequence[Mapping[str, object]],
    selected_ordinals: Sequence[int],
    selection_probabilities: Sequence[SelectionProbability],
    targets: Sequence[float],
    value_weights: Sequence[float],
    actor_weights: Sequence[float],
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    update_config: RunPolicyUpdateConfig,
    require_matching_propensities: bool,
    fixed_actor_advantages: Sequence[float] | None,
    fixed_value_predictions: Sequence[float] | None,
) -> _RunPolicyLoss:
    if update_config.rule is RunPolicyUpdateRule.REINFORCE:
        if not require_matching_propensities:
            raise TorchOutcomeError("run REINFORCE cannot reuse an updated batch")
        return _RunPolicyLoss(
            value=_on_policy_weighted_loss(
                scorer=scorer,
                payloads=payloads,
                selected_ordinals=selected_ordinals,
                selection_probabilities=selection_probabilities,
                targets=targets,
                weights=value_weights,
                concat_limits=concat_limits,
                policy_config=policy_config,
                objective_name="terminal",
            ),
            approximate_kl=0.0,
            clip_fraction=0.0,
            entropy=0.0,
            value_loss=0.0,
            value_clip_fraction=0.0,
            explained_variance=None,
            actor_decision_count=len(selected_ordinals),
            actor_advantages=tuple(float(target) for target in targets),
            critic_predictions=None,
            value_diagnostics=None,
        )

    decision_count = len(selected_ordinals)
    if not (
        len(selection_probabilities)
        == len(targets)
        == len(value_weights)
        == len(actor_weights)
        == decision_count
        > 0
    ):
        raise TorchOutcomeError("run PPO objective rows are misaligned")
    if not all(math.isfinite(target) for target in targets):
        raise TorchOutcomeError("run PPO objective targets must be finite")
    if not all(
        math.isfinite(weight) and weight > 0.0 for weight in value_weights
    ):
        raise TorchOutcomeError("run PPO value weights must be positive")
    if not all(
        math.isfinite(weight) and weight >= 0.0 for weight in actor_weights
    ):
        raise TorchOutcomeError("run PPO actor weights must be non-negative")
    recorded_log_probabilities: list[float] = []
    for evidence in selection_probabilities:
        if not isinstance(evidence, SelectionProbability):
            raise TorchOutcomeError("run PPO probabilities must be typed")
        value = evidence.value
        if value is None or not math.isfinite(value) or not 0.0 < value <= 1.0:
            raise TorchOutcomeError(
                "run PPO requires positive recorded probabilities"
            )
        recorded_log_probabilities.append(math.log(value))

    combined = concatenate_semantic_decision_batches(payloads, concat_limits)
    actor_critic = getattr(scorer, "actor_critic", None)
    if not callable(actor_critic):
        raise TorchOutcomeError("run value PPO requires an actor-critic scorer")
    actor_critic_output = actor_critic(combined)
    if not isinstance(actor_critic_output, RaggedActorCriticOutput):
        raise TorchOutcomeError("actor-critic scorer returned an invalid output")
    logits = actor_critic_output.logits
    predicted_values = actor_critic_output.row_values
    selected_log_probabilities = _selected_log_probabilities(
        logits,
        selected_ordinals,
        policy_config,
    )
    if require_matching_propensities:
        _require_matching_propensities(
            logits,
            selected_ordinals,
            selection_probabilities,
            policy_config,
        )
    old_log_probability_tensor = torch.as_tensor(
        recorded_log_probabilities,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    target_tensor = torch.as_tensor(
        targets,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    value_weight_tensor = torch.as_tensor(
        value_weights,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    actor_weight_tensor = torch.as_tensor(
        actor_weights,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    if predicted_values.shape != target_tensor.shape:
        raise TorchOutcomeError("run critic values are misaligned")
    if fixed_actor_advantages is None:
        actor_advantages = target_tensor - predicted_values.detach()
        actor_weight = torch.sum(actor_weight_tensor)
        if update_config.normalize_advantage:
            if bool(actor_weight > 0.0):
                advantage_mean = (
                    torch.sum(actor_advantages * actor_weight_tensor) / actor_weight
                )
                advantage_variance = torch.sum(
                    (actor_advantages - advantage_mean).square()
                    * actor_weight_tensor
                ) / actor_weight
                actor_advantages = (
                    actor_advantages - advantage_mean
                ) / torch.clamp_min(torch.sqrt(advantage_variance), 1e-8)
        actor_advantages = torch.where(
            actor_weight_tensor > 0.0,
            actor_advantages,
            torch.zeros_like(actor_advantages),
        )
    else:
        if (
            len(fixed_actor_advantages) != decision_count
            or not all(math.isfinite(value) for value in fixed_actor_advantages)
        ):
            raise TorchOutcomeError(
                "fixed run advantages are misaligned or non-finite"
            )
        actor_advantages = torch.as_tensor(
            fixed_actor_advantages,
            dtype=target_tensor.dtype,
            device=target_tensor.device,
        )
        if bool(
            torch.any(
                (actor_weight_tensor == 0.0) & (actor_advantages != 0.0)
            )
        ):
            raise TorchOutcomeError("forced run rows must have zero fixed advantage")
    if fixed_value_predictions is None:
        old_values = predicted_values.detach()
    else:
        if (
            len(fixed_value_predictions) != decision_count
            or not all(math.isfinite(value) for value in fixed_value_predictions)
        ):
            raise TorchOutcomeError(
                "fixed run value predictions are misaligned or non-finite"
            )
        old_values = torch.as_tensor(
            fixed_value_predictions,
            dtype=target_tensor.dtype,
            device=target_tensor.device,
        )
    value_error = predicted_values - target_tensor
    value_losses = value_error.square()
    value_clip_fraction = torch.zeros(
        (),
        dtype=target_tensor.dtype,
        device=target_tensor.device,
    )
    value_clip = update_config.value_clip_coefficient
    if value_clip is not None:
        clipped_values = old_values + torch.clamp(
            predicted_values - old_values,
            -value_clip,
            value_clip,
        )
        value_losses = torch.maximum(
            value_losses,
            (clipped_values - target_tensor).square(),
        )
        value_clip_fraction = torch.sum(
            ((predicted_values - old_values).abs() > value_clip).to(
                value_weight_tensor.dtype
            )
            * value_weight_tensor
        ) / torch.sum(value_weight_tensor)
    value_loss = 0.5 * torch.sum(value_losses * value_weight_tensor)
    log_ratio = selected_log_probabilities - old_log_probability_tensor
    ratio = torch.exp(log_ratio)
    clipped_ratio = torch.clamp(
        ratio,
        1.0 - update_config.clip_coefficient,
        1.0 + update_config.clip_coefficient,
    )
    policy_terms = torch.maximum(
        -actor_advantages * ratio,
        -actor_advantages * clipped_ratio,
    )
    entropies = _ragged_entropies(logits, policy_config)
    loss = torch.sum(policy_terms * actor_weight_tensor) - (
        update_config.entropy_coefficient
        * torch.sum(entropies * actor_weight_tensor)
    ) + update_config.value_loss_coefficient * value_loss
    actor_weight = torch.sum(actor_weight_tensor)
    if bool(actor_weight > 0.0):
        approximate_kl = torch.sum(
            ((ratio - 1.0) - log_ratio) * actor_weight_tensor
        ) / actor_weight
        clip_fraction = torch.sum(
            ((ratio - 1.0).abs() > update_config.clip_coefficient).to(
                actor_weight_tensor.dtype
            )
            * actor_weight_tensor
        ) / actor_weight
        mean_entropy = torch.sum(entropies * actor_weight_tensor) / actor_weight
    else:
        approximate_kl = torch.zeros_like(loss)
        clip_fraction = torch.zeros_like(loss)
        mean_entropy = torch.zeros_like(loss)
    explained_variance = _weighted_explained_variance(
        target_tensor.detach(),
        predicted_values.detach(),
        value_weight_tensor,
    )
    diagnostics = (
        loss,
        approximate_kl,
        clip_fraction,
        mean_entropy,
        value_loss,
        value_clip_fraction,
    )
    if not all(bool(torch.all(torch.isfinite(value))) for value in diagnostics):
        raise TorchOutcomeError("run PPO objective must be finite")
    detached_advantages = tuple(
        float(value) for value in actor_advantages.detach().cpu().tolist()
    )
    detached_predictions = tuple(
        float(value) for value in predicted_values.detach().cpu().tolist()
    )
    value_diagnostics = None
    if fixed_actor_advantages is None:
        detached_targets = tuple(
            float(value) for value in target_tensor.detach().cpu().tolist()
        )
        detached_residuals = tuple(
            target - prediction
            for target, prediction in zip(
                detached_targets,
                detached_predictions,
                strict=True,
            )
        )
        value_diagnostics = RunValueDiagnostics(
            actor_advantage=_optional_actor_signal_summary(
                detached_advantages,
                actor_weights,
            ),
            critic_prediction=_attempt_equal_signal_summary(
                detached_predictions,
                value_weights,
            ),
            return_to_go_target=_attempt_equal_signal_summary(
                detached_targets,
                value_weights,
            ),
            critic_residual=_attempt_equal_signal_summary(
                detached_residuals,
                value_weights,
            ),
            actor_decision_count=sum(weight > 0.0 for weight in actor_weights),
            forced_decision_count=sum(weight == 0.0 for weight in actor_weights),
            explained_variance=explained_variance,
        )
    return _RunPolicyLoss(
        value=loss,
        approximate_kl=float(approximate_kl.detach().item()),
        clip_fraction=float(clip_fraction.detach().item()),
        entropy=float(mean_entropy.detach().item()),
        value_loss=float(value_loss.detach().item()),
        value_clip_fraction=float(value_clip_fraction.detach().item()),
        explained_variance=explained_variance,
        actor_decision_count=sum(weight > 0.0 for weight in actor_weights),
        actor_advantages=detached_advantages,
        critic_predictions=detached_predictions,
        value_diagnostics=value_diagnostics,
    )


def _attempt_equal_signal_summary(
    values: Sequence[float],
    weights: Sequence[float],
) -> AttemptEqualSignalSummary:
    """Summarize rows without allowing longer attempts to dominate moments."""

    normalized_values = tuple(float(value) for value in values)
    normalized_weights = tuple(float(weight) for weight in weights)
    if (
        len(normalized_values) != len(normalized_weights)
        or not normalized_values
        or not all(math.isfinite(value) for value in normalized_values)
        or not all(
            math.isfinite(weight) and weight > 0.0
            for weight in normalized_weights
        )
    ):
        raise TorchOutcomeError("run value diagnostics are misaligned or non-finite")
    total_weight = math.fsum(normalized_weights)
    if not math.isfinite(total_weight) or total_weight <= 0.0:
        raise TorchOutcomeError("run value diagnostic weight must be positive")
    scaled_weights = tuple(weight / total_weight for weight in normalized_weights)
    weighted_mean = math.fsum(
        value * weight
        for value, weight in zip(
            normalized_values,
            scaled_weights,
            strict=True,
        )
    )
    weighted_variance = math.fsum(
        weight * (value - weighted_mean) ** 2
        for value, weight in zip(
            normalized_values,
            scaled_weights,
            strict=True,
        )
    )
    negative = tuple(value < 0.0 for value in normalized_values)
    zero = tuple(value == 0.0 for value in normalized_values)
    positive = tuple(value > 0.0 for value in normalized_values)
    return AttemptEqualSignalSummary(
        decision_count=len(normalized_values),
        negative_decisions=sum(negative),
        zero_decisions=sum(zero),
        positive_decisions=sum(positive),
        negative_weight=math.fsum(
            weight
            for weight, selected in zip(scaled_weights, negative, strict=True)
            if selected
        ),
        zero_weight=math.fsum(
            weight
            for weight, selected in zip(scaled_weights, zero, strict=True)
            if selected
        ),
        positive_weight=math.fsum(
            weight
            for weight, selected in zip(scaled_weights, positive, strict=True)
            if selected
        ),
        weighted_mean=weighted_mean,
        weighted_standard_deviation=math.sqrt(max(0.0, weighted_variance)),
        minimum=min(normalized_values),
        maximum=max(normalized_values),
    )


def _optional_actor_signal_summary(
    values: Sequence[float],
    weights: Sequence[float],
) -> AttemptEqualSignalSummary | None:
    eligible = tuple(
        (value, weight)
        for value, weight in zip(values, weights, strict=True)
        if weight > 0.0
    )
    if not eligible:
        return None
    return _attempt_equal_signal_summary(
        tuple(value for value, _weight in eligible),
        tuple(weight for _value, weight in eligible),
    )


def _weighted_explained_variance(
    targets: Tensor,
    predictions: Tensor,
    weights: Tensor,
) -> float | None:
    """Return SB3-style explained variance under attempt-equal row weights."""

    total_weight = torch.sum(weights)
    target_mean = torch.sum(targets * weights) / total_weight
    target_variance = (
        torch.sum((targets - target_mean).square() * weights) / total_weight
    )
    if bool(target_variance <= 1e-12):
        return None
    residual = targets - predictions
    residual_mean = torch.sum(residual * weights) / total_weight
    residual_variance = (
        torch.sum((residual - residual_mean).square() * weights) / total_weight
    )
    explained = 1.0 - residual_variance / target_variance
    if not bool(torch.isfinite(explained)):
        raise TorchOutcomeError("run explained variance must be finite")
    return float(explained.detach().item())


def _combat_policy_weighted_loss(
    *,
    scorer: CandidatePolicyScorer,
    payloads: Sequence[Mapping[str, object]],
    selected_ordinals: Sequence[int],
    selection_probabilities: Sequence[SelectionProbability],
    targets: Sequence[float],
    weights: Sequence[float],
    value_head_indices: Sequence[int],
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    update_config: CombatPolicyUpdateConfig,
    require_matching_propensities: bool,
    fixed_actor_advantages: Sequence[float] | None,
) -> _CombatPolicyLoss:
    if not isinstance(update_config, CombatPolicyUpdateConfig):
        raise TorchOutcomeError("combat policy update config must be typed")
    if update_config.rule is CombatPolicyUpdateRule.REINFORCE:
        if not require_matching_propensities:
            raise TorchOutcomeError("REINFORCE cannot reuse an updated policy batch")
        return _CombatPolicyLoss(
            value=_on_policy_weighted_loss(
                scorer=scorer,
                payloads=payloads,
                selected_ordinals=selected_ordinals,
                selection_probabilities=selection_probabilities,
                targets=targets,
                weights=weights,
                concat_limits=concat_limits,
                policy_config=policy_config,
                objective_name="combat win-first",
            ),
            approximate_kl=0.0,
            clip_fraction=0.0,
            entropy=0.0,
            value_loss=0.0,
            actor_advantages=tuple(float(target) for target in targets),
        )

    decision_count = len(selected_ordinals)
    if not (
        len(selection_probabilities)
        == len(targets)
        == len(weights)
        == len(value_head_indices)
        == decision_count
        > 0
    ):
        raise TorchOutcomeError("combat PPO objective rows are misaligned")
    if not all(math.isfinite(target) for target in targets):
        raise TorchOutcomeError("combat PPO objective targets must be finite")
    if not all(math.isfinite(weight) and weight > 0.0 for weight in weights):
        raise TorchOutcomeError("combat PPO objective weights must be positive")
    recorded_log_probabilities: list[float] = []
    for evidence in selection_probabilities:
        if not isinstance(evidence, SelectionProbability):
            raise TorchOutcomeError(
                "combat PPO objective probabilities must be typed"
            )
        value = evidence.value
        if value is None or not math.isfinite(value) or not 0.0 < value <= 1.0:
            raise TorchOutcomeError(
                "combat PPO objective requires positive recorded probabilities"
            )
        recorded_log_probabilities.append(math.log(value))

    combined = concatenate_semantic_decision_batches(payloads, concat_limits)
    predicted_values: Tensor | None = None
    if update_config.uses_value_baseline:
        actor_critic_multi = getattr(scorer, "actor_critic_multi", None)
        if not callable(actor_critic_multi):
            raise TorchOutcomeError(
                "combat value PPO requires a multi actor-critic scorer"
            )
        actor_critic_output = actor_critic_multi(combined)
        if not isinstance(actor_critic_output, RaggedMultiActorCriticOutput):
            raise TorchOutcomeError(
                "multi actor-critic scorer returned an invalid output"
            )
        logits = actor_critic_output.logits
        all_predicted_values = actor_critic_output.row_values
        if all_predicted_values.shape[1] != COMBAT_ROLLOUT_VALUE_HEAD_WIDTH:
            raise TorchOutcomeError(
                "combat value PPO requires exactly three fixed value columns"
            )
        value_head_tensor = torch.as_tensor(
            value_head_indices,
            dtype=torch.long,
            device=all_predicted_values.device,
        )
        if bool(
            torch.any(value_head_tensor < 0)
            or torch.any(value_head_tensor >= all_predicted_values.shape[1])
        ):
            raise TorchOutcomeError("combat value head index is outside the scorer")
        predicted_values = all_predicted_values[
            torch.arange(
                decision_count,
                dtype=torch.long,
                device=all_predicted_values.device,
            ),
            value_head_tensor,
        ]
    else:
        logits = scorer(combined)
    if not isinstance(logits, RaggedCandidateLogits):
        raise TorchOutcomeError(
            "candidate policy scorer must return RaggedCandidateLogits"
        )
    selected_log_probabilities = _selected_log_probabilities(
        logits,
        selected_ordinals,
        policy_config,
    )
    if require_matching_propensities:
        _require_matching_propensities(
            logits,
            selected_ordinals,
            selection_probabilities,
            policy_config,
        )
    old_log_probability_tensor = torch.as_tensor(
        recorded_log_probabilities,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    target_tensor = torch.as_tensor(
        targets,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    weight_tensor = torch.as_tensor(
        weights,
        dtype=selected_log_probabilities.dtype,
        device=selected_log_probabilities.device,
    )
    actor_advantages = target_tensor
    value_loss = torch.zeros(
        (),
        dtype=target_tensor.dtype,
        device=target_tensor.device,
    )
    if predicted_values is not None:
        if predicted_values.shape != target_tensor.shape:
            raise TorchOutcomeError(
                "combat critic values are misaligned"
            )
        if fixed_actor_advantages is None:
            actor_advantages = target_tensor - predicted_values.detach()
        else:
            if (
                len(fixed_actor_advantages) != decision_count
                or not all(
                    math.isfinite(value) for value in fixed_actor_advantages
                )
            ):
                raise TorchOutcomeError(
                    "fixed combat advantages are misaligned or non-finite"
                )
            actor_advantages = torch.as_tensor(
                fixed_actor_advantages,
                dtype=target_tensor.dtype,
                device=target_tensor.device,
            )
        value_error = predicted_values - target_tensor
        value_loss = 0.5 * torch.sum(value_error.square() * weight_tensor)
    log_ratio = selected_log_probabilities - old_log_probability_tensor
    ratio = torch.exp(log_ratio)
    clipped_ratio = torch.clamp(
        ratio,
        1.0 - update_config.clip_coefficient,
        1.0 + update_config.clip_coefficient,
    )
    policy_terms = torch.maximum(
        -actor_advantages * ratio,
        -actor_advantages * clipped_ratio,
    )
    entropies = _ragged_entropies(logits, policy_config)
    loss = torch.sum(policy_terms * weight_tensor) - (
        update_config.entropy_coefficient
        * torch.sum(entropies * weight_tensor)
    ) + update_config.value_loss_coefficient * value_loss
    diagnostic_weight = torch.sum(weight_tensor)
    approximate_kl = torch.sum(
        ((ratio - 1.0) - log_ratio) * weight_tensor
    ) / diagnostic_weight
    clip_fraction = torch.sum(
        ((ratio - 1.0).abs() > update_config.clip_coefficient).to(
            weight_tensor.dtype
        )
        * weight_tensor
    ) / diagnostic_weight
    mean_entropy = torch.sum(entropies * weight_tensor) / diagnostic_weight
    diagnostics = (
        loss,
        approximate_kl,
        clip_fraction,
        mean_entropy,
        value_loss,
    )
    if not all(bool(torch.all(torch.isfinite(value))) for value in diagnostics):
        raise TorchOutcomeError("combat PPO objective must be finite")
    return _CombatPolicyLoss(
        value=loss,
        approximate_kl=float(approximate_kl.detach().item()),
        clip_fraction=float(clip_fraction.detach().item()),
        entropy=float(mean_entropy.detach().item()),
        value_loss=float(value_loss.detach().item()),
        actor_advantages=tuple(
            float(value)
            for value in actor_advantages.detach().cpu().tolist()
        ),
    )


def _selected_log_probabilities(
    logits: RaggedCandidateLogits,
    selected_ordinals: Sequence[int],
    config: RaggedCategoricalPolicyConfig,
) -> Tensor:
    ordinals = torch.as_tensor(
        tuple(selected_ordinals),
        dtype=torch.long,
        device=logits.values.device,
    )
    if ordinals.ndim != 1 or ordinals.numel() != logits.row_count:
        raise TorchOutcomeError("selected ordinals must contain one value per row")
    lengths = logits.row_splits[1:] - logits.row_splits[:-1]
    if bool(torch.any(ordinals < 0)) or bool(torch.any(ordinals >= lengths)):
        raise TorchOutcomeError("selected ordinal is outside its candidate row")
    row_ids = torch.repeat_interleave(
        torch.arange(
            logits.row_count,
            dtype=torch.long,
            device=logits.values.device,
        ),
        lengths,
    )
    scaled = logits.values / config.temperature
    row_max = torch.full(
        (logits.row_count,),
        -torch.inf,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_max.scatter_reduce_(0, row_ids, scaled.detach(), reduce="amax")
    shifted = scaled - row_max[row_ids]
    row_sum = torch.zeros(
        logits.row_count,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_sum.index_add_(0, row_ids, torch.exp(shifted))
    log_probabilities = shifted - torch.log(row_sum[row_ids])
    return log_probabilities[logits.row_splits[:-1] + ordinals]


def _ragged_entropies(
    logits: RaggedCandidateLogits,
    config: RaggedCategoricalPolicyConfig,
) -> Tensor:
    lengths = logits.row_splits[1:] - logits.row_splits[:-1]
    row_ids = torch.repeat_interleave(
        torch.arange(
            logits.row_count,
            dtype=torch.long,
            device=logits.values.device,
        ),
        lengths,
    )
    scaled = logits.values / config.temperature
    row_max = torch.full(
        (logits.row_count,),
        -torch.inf,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_max.scatter_reduce_(0, row_ids, scaled.detach(), reduce="amax")
    shifted = scaled - row_max[row_ids]
    row_sum = torch.zeros(
        logits.row_count,
        dtype=scaled.dtype,
        device=scaled.device,
    )
    row_sum.index_add_(0, row_ids, torch.exp(shifted))
    log_probabilities = shifted - torch.log(row_sum[row_ids])
    probabilities = torch.exp(log_probabilities)
    entropies = torch.zeros_like(row_sum)
    entropies.index_add_(
        0,
        row_ids,
        -(probabilities * log_probabilities),
    )
    return entropies


def _require_matching_propensities(
    logits: RaggedCandidateLogits,
    selected_ordinals: Sequence[int],
    recorded: Sequence[SelectionProbability],
    config: RaggedCategoricalPolicyConfig,
) -> None:
    if len(recorded) != logits.row_count:
        raise TorchOutcomeError("selection probabilities must contain one value per row")
    splits = logits.row_splits.detach().cpu().tolist()
    for row, (start, end, ordinal, evidence) in enumerate(
        zip(
            splits[:-1],
            splits[1:],
            selected_ordinals,
            recorded,
            strict=True,
        )
    ):
        if evidence.value is None:
            raise TorchOutcomeError(
                "on-policy objective requires known selection probabilities"
            )
        probabilities = torch.softmax(
            logits.values[start:end].detach().to(dtype=torch.float64)
            / config.temperature,
            dim=0,
        )
        expected = float(probabilities[ordinal].item())
        if not math.isclose(
            evidence.value,
            expected,
            rel_tol=1e-6,
            abs_tol=1e-8,
        ):
            raise TorchOutcomeError(
                f"selection probability at row {row} is off-policy"
            )
