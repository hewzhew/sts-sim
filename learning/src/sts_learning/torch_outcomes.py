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
    matched_floor_leave_one_out_advantages,
)
from .combat_experience import (
    CombatDecisionExperienceBatch,
    CompletedCombatGroupExperience,
)
from .combat_objective import CombatAllWinAxis, CombatWinObjectiveConfig
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
from .torch_policy import RaggedCandidateLogits, RaggedCategoricalPolicyConfig


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
    if advantage_mode is TerminalAdvantageMode.MATCHED_FLOOR_LEAVE_ONE_OUT:
        try:
            matched_advantages = matched_floor_leave_one_out_advantages(
                normalized,
                return_config,
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
                        "matched-floor targets are misaligned with decision rows"
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
    total_replicates = 0
    total_decisions = 0
    win_signal_groups = 0
    terminal_hp_signal_groups = 0

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
            win_signal_groups += 1
        elif (
            objective_config.all_win_axis is CombatAllWinAxis.TERMINAL_HP
            and all(outcome.won for outcome in group.outcomes.outcomes)
            and advantages.terminal_hp_has_signal
        ):
            selected_advantages = advantages.terminal_hp
            terminal_hp_signal_groups += 1
        else:
            selected_advantages = advantages.win
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
                targets.append(selected_advantages[replicate_index])
                weights.append(
                    1.0
                    / (
                        len(normalized)
                        * replicate_count
                        * decision_counts[replicate_index]
                    )
                )
        behavior_ids.append(group.behavior_manifest_id)
        probability_evidence.append(tuple(group_probabilities))
        total_replicates += replicate_count
        total_decisions += group.decision_count

    value = _on_policy_weighted_loss(
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
        concat_limits=concat_limits,
        policy_config=policy_config,
        objective_name="combat win-first",
    )
    return OnPolicyCombatWinLoss(
        value=value,
        group_count=len(normalized),
        signal_group_count=win_signal_groups + terminal_hp_signal_groups,
        win_signal_group_count=win_signal_groups,
        terminal_hp_signal_group_count=terminal_hp_signal_groups,
        replicate_count=total_replicates,
        decision_count=total_decisions,
        behavior_manifest_ids=tuple(behavior_ids),
        selection_probabilities=tuple(probability_evidence),
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
