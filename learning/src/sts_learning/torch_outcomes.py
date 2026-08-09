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
    CombatAllWinAxis,
    CombatPolicyUpdateConfig,
    CombatPolicyUpdateRule,
    CombatWinObjectiveConfig,
)
from .experience import DecisionExperienceBatch, ExperienceError
from .manifests import BehaviorManifestRegistry
from .policy import BehaviorManifestId, SelectionProbability
from .semantic_concat import (
    SemanticBatchConcatLimits,
    concatenate_semantic_decision_batches,
)
from .terminal_returns import (
    FloorProgressReturnConfig,
    RunDecisionScope,
    TerminalAdvantageMode,
    floor_progress_terminal_return,
    terminal_return_advantages,
)
from .torch_policy import (
    RaggedActorCriticOutput,
    RaggedCandidateLogits,
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
    behavior_manifest_ids: tuple[tuple[BehaviorManifestId, ...], ...]
    selection_probabilities: tuple[tuple[SelectionProbability, ...], ...]


@dataclass(frozen=True)
class OnPolicyCombatWinLoss:
    """One win-first combat loss without an HP/potion exchange rate.

    A root with mixed wins and losses uses only win advantage.  An all-win
    root may use the configured all-win terminal-HP advantage, so solved early
    combats can keep learning resource preservation. Potion retention remains
    evidence only.
    """

    value: Tensor
    group_count: int
    signal_group_count: int
    win_signal_group_count: int
    terminal_hp_signal_group_count: int
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


def on_policy_terminal_loss(
    scorer: CandidatePolicyScorer,
    attempts: Sequence[CompletedAttemptExperience],
    registry: BehaviorManifestRegistry,
    concat_limits: SemanticBatchConcatLimits,
    policy_config: RaggedCategoricalPolicyConfig,
    return_config: FloorProgressReturnConfig,
    advantage_mode: TerminalAdvantageMode,
    decision_scope: RunDecisionScope = RunDecisionScope.ALL,
) -> OnPolicyTerminalLoss:
    """Apply progress-return REINFORCE to exact sampled categorical behavior.

    Every complete attempt contributes equal total weight regardless of its
    length. Return sign controls the relative probability direction, and
    single-candidate decisions always have zero gradient.
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
    normalized = tuple(attempts)
    if not normalized:
        raise TorchOutcomeError("policy objective requires at least one complete attempt")
    if not all(isinstance(attempt, CompletedAttemptExperience) for attempt in normalized):
        raise TorchOutcomeError("policy objective accepts only complete attempts")
    matched_advantages = None
    advantages = None
    if advantage_mode in (
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
    weights: list[float] = []
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

        attempt_behavior_ids: list[BehaviorManifestId] = []
        attempt_probabilities: list[SelectionProbability] = []
        for batch_index, batch, row_indices in scoped_batches:
            try:
                manifest = registry.resolve(batch.behavior_manifest_id)
            except ValueError as error:
                raise TorchOutcomeError(
                    "complete attempt references an unknown behavior manifest"
                ) from error
            if manifest.behavior_rule != policy_config.behavior_rule:
                raise TorchOutcomeError(
                    "complete attempt behavior rule conflicts with policy config"
                )
            attempt_behavior_ids.append(batch.behavior_manifest_id)
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            attempt_probabilities.extend(batch.selection_probabilities)
            if matched_advantages is None:
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
            weights.extend(
                [1.0 / (len(normalized) * scoped_decisions)]
                * batch.decision_count
            )
        behavior_ids.append(tuple(attempt_behavior_ids))
        probability_evidence.append(tuple(attempt_probabilities))
        total_decisions += scoped_decisions

    value = _on_policy_weighted_loss(
        scorer=scorer,
        payloads=payloads,
        selected_ordinals=selected_ordinals,
        selection_probabilities=tuple(
            probability
            for attempt_probabilities in probability_evidence
            for probability in attempt_probabilities
        ),
        targets=targets,
        weights=weights,
        concat_limits=concat_limits,
        policy_config=policy_config,
        objective_name="terminal",
    )
    return OnPolicyTerminalLoss(
        value=value,
        attempt_count=len(normalized),
        decision_count=total_decisions,
        behavior_manifest_ids=tuple(behavior_ids),
        selection_probabilities=tuple(probability_evidence),
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
    only when configured and that axis varies. Potion retention is deliberately
    absent, so there is no HP/potion exchange rate.
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
    group_indices: list[int] = []
    leave_one_out_scales: list[float] = []
    total_replicates = 0
    total_decisions = 0
    win_signal_groups = 0
    terminal_hp_signal_groups = 0

    for group_index, group in enumerate(normalized):
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
            start_hp = group.outcomes.outcomes[0].start_hp
            selected_returns = tuple(
                outcome.final_hp / start_hp
                for outcome in group.outcomes.outcomes
            )
            terminal_hp_signal_groups += 1
        else:
            selected_advantages = advantages.win
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

        for batch in group.batches:
            payloads.append(batch.payload)
            selected_ordinals.extend(batch.selected_ordinals)
            group_probabilities.extend(batch.selection_probabilities)
            for replicate_index in batch.replicate_indices:
                targets.append(
                    selected_returns[replicate_index]
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
                group_indices.append(group_index)
                leave_one_out_scales.append(
                    replicate_count / (replicate_count - 1)
                )
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
        group_indices=group_indices,
        leave_one_out_scales=leave_one_out_scales,
        concat_limits=concat_limits,
        policy_config=policy_config,
        update_config=objective_config.policy_update,
        require_matching_propensities=require_matching_propensities,
        fixed_actor_advantages=fixed_actor_advantages,
    )
    return OnPolicyCombatWinLoss(
        value=policy_loss.value,
        group_count=len(normalized),
        signal_group_count=win_signal_groups + terminal_hp_signal_groups,
        win_signal_group_count=win_signal_groups,
        terminal_hp_signal_group_count=terminal_hp_signal_groups,
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


def _combat_policy_weighted_loss(
    *,
    scorer: CandidatePolicyScorer,
    payloads: Sequence[Mapping[str, object]],
    selected_ordinals: Sequence[int],
    selection_probabilities: Sequence[SelectionProbability],
    targets: Sequence[float],
    weights: Sequence[float],
    group_indices: Sequence[int],
    leave_one_out_scales: Sequence[float],
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
        == len(group_indices)
        == len(leave_one_out_scales)
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
        actor_critic = getattr(scorer, "actor_critic", None)
        if not callable(actor_critic):
            raise TorchOutcomeError(
                "combat value PPO requires an actor-critic scorer"
            )
        actor_critic_output = actor_critic(combined)
        if not isinstance(actor_critic_output, RaggedActorCriticOutput):
            raise TorchOutcomeError(
                "actor-critic scorer returned an invalid output"
            )
        logits = actor_critic_output.logits
        predicted_values = actor_critic_output.row_values
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
            group_tensor = torch.as_tensor(
                group_indices,
                dtype=torch.long,
                device=target_tensor.device,
            )
            scale_tensor = torch.as_tensor(
                leave_one_out_scales,
                dtype=target_tensor.dtype,
                device=target_tensor.device,
            )
            group_count = max(group_indices) + 1
            residual = target_tensor - predicted_values.detach()
            group_weight = torch.zeros(
                group_count,
                dtype=target_tensor.dtype,
                device=target_tensor.device,
            )
            group_residual = torch.zeros_like(group_weight)
            group_weight.index_add_(0, group_tensor, weight_tensor)
            group_residual.index_add_(0, group_tensor, residual * weight_tensor)
            if bool(torch.any(group_weight <= 0.0)):
                raise TorchOutcomeError(
                    "combat critic group weight must be positive"
                )
            actor_advantages = (
                residual - (group_residual / group_weight)[group_tensor]
            ) * scale_tensor
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
