use super::action_effects::CardPlayEffectDiagnostics;
use super::action_priority::{priority_for_input, ActionOrderingPriority, ActionOrderingRole};
use super::*;
use std::collections::BTreeMap;

mod diagnostics;
pub(super) use diagnostics::ActionOrderingDiagnosticsCollector;
#[cfg(test)]
use diagnostics::ACTION_EFFECT_SAMPLE_LIMIT;

#[derive(Clone, Debug)]
pub(super) struct IndexedActionChoice {
    pub(super) original_action_id: usize,
    pub(super) choice: CombatActionChoice,
}

pub(super) type OrderedActionChoice = IndexedActionChoice;

#[derive(Clone, Debug)]
pub(super) struct ActionOrderingResult {
    pub(super) choices: Vec<OrderedActionChoice>,
    pub(super) summary: ActionOrderingSummary,
}

#[derive(Clone, Debug)]
pub(super) struct ActionOrderingSummary {
    action_count: usize,
    max_position_shift: usize,
    role_counts: BTreeMap<ActionOrderingRole, usize>,
    first_role: Option<ActionOrderingRole>,
    first_original_action_id: Option<usize>,
    first_action_key: Option<String>,
    phase_signal_actions: usize,
    root_action_prior_scored_actions: usize,
    action_effect_samples: Vec<ActionOrderingActionEffectSummary>,
}

#[derive(Clone, Debug)]
struct ActionOrderingEntry {
    original_action_id: usize,
    choice: CombatActionChoice,
    priority: ActionOrderingPriority,
    root_action_prior_score: Option<f64>,
}

#[derive(Clone, Debug)]
struct ActionOrderingActionEffectSummary {
    original_action_id: usize,
    ordered_index: usize,
    role: ActionOrderingRole,
    action_key: String,
    effects: CardPlayEffectDiagnostics,
}

impl ActionOrderingSummary {
    pub(super) fn action_count(&self) -> usize {
        self.action_count
    }

    pub(super) fn first_role(&self) -> Option<ActionOrderingRole> {
        self.first_role
    }

    pub(super) fn role_counts(&self) -> impl Iterator<Item = (ActionOrderingRole, usize)> + '_ {
        self.role_counts.iter().map(|(role, count)| (*role, *count))
    }
}

#[cfg(test)]
fn order_action_choices(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<CombatActionChoice>,
) -> ActionOrderingResult {
    order_indexed_action_choices(
        engine,
        combat,
        choices
            .into_iter()
            .enumerate()
            .map(|(original_action_id, choice)| IndexedActionChoice {
                original_action_id,
                choice,
            })
            .collect(),
    )
}

pub(super) fn order_indexed_action_choices(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<IndexedActionChoice>,
) -> ActionOrderingResult {
    order_indexed_action_choices_with_prior(
        engine,
        combat,
        choices,
        None,
        CombatSearchV2PhaseGuardPolicy::Default,
    )
}

pub(super) fn order_indexed_action_choices_with_prior(
    engine: &EngineState,
    combat: &CombatState,
    choices: Vec<IndexedActionChoice>,
    root_action_prior: Option<&CombatSearchV2RootActionPrior>,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
) -> ActionOrderingResult {
    let exact_state_hash = root_action_prior
        .filter(|prior| !prior.is_empty())
        .map(|_| combat_exact_state_hash_v1(engine, combat));
    let mut entries = choices
        .into_iter()
        .map(|indexed| ActionOrderingEntry {
            original_action_id: indexed.original_action_id,
            root_action_prior_score: exact_state_hash.as_ref().and_then(|state_hash| {
                root_action_prior
                    .and_then(|prior| prior.score(state_hash, &indexed.choice.action_key))
            }),
            priority: priority_for_input(engine, combat, &indexed.choice.input, phase_guard_policy),
            choice: indexed.choice,
        })
        .collect::<Vec<_>>();

    if action_ordering_enabled(engine) {
        entries.sort_by(|left, right| {
            compare_action_ordering_entries(left, right)
                .then_with(|| left.original_action_id.cmp(&right.original_action_id))
        });
    }

    let summary = summarize_ordering(&entries);
    let choices = entries
        .into_iter()
        .map(|entry| IndexedActionChoice {
            original_action_id: entry.original_action_id,
            choice: entry.choice,
        })
        .collect();

    ActionOrderingResult { choices, summary }
}

fn compare_action_ordering_entries(
    left: &ActionOrderingEntry,
    right: &ActionOrderingEntry,
) -> std::cmp::Ordering {
    right
        .priority
        .role_rank
        .cmp(&left.priority.role_rank)
        .then_with(|| {
            compare_prior_scores(right.root_action_prior_score, left.root_action_prior_score)
        })
        .then_with(|| right.priority.cmp(&left.priority))
}

fn compare_prior_scores(left: Option<f64>, right: Option<f64>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn action_ordering_enabled(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    )
}

fn summarize_ordering(entries: &[ActionOrderingEntry]) -> ActionOrderingSummary {
    let mut role_counts = BTreeMap::new();
    let mut max_position_shift = 0usize;
    let mut phase_signal_actions = 0usize;
    let mut root_action_prior_scored_actions = 0usize;
    let mut action_effect_samples = Vec::new();
    for (ordered_index, entry) in entries.iter().enumerate() {
        *role_counts.entry(entry.priority.role).or_insert(0) += 1;
        max_position_shift =
            max_position_shift.max(entry.original_action_id.abs_diff(ordered_index));
        if entry.root_action_prior_score.is_some() {
            root_action_prior_scored_actions += 1;
        }
        if entry.priority.phase_hint.has_signal() {
            phase_signal_actions += 1;
        }
        if entry.priority.effects.has_reactive_signal() {
            action_effect_samples.push(ActionOrderingActionEffectSummary {
                original_action_id: entry.original_action_id,
                ordered_index,
                role: entry.priority.role,
                action_key: entry.choice.action_key.clone(),
                effects: entry.priority.effects,
            });
        }
    }

    ActionOrderingSummary {
        action_count: entries.len(),
        max_position_shift,
        role_counts,
        first_role: entries.first().map(|entry| entry.priority.role),
        first_original_action_id: entries.first().map(|entry| entry.original_action_id),
        first_action_key: entries.first().map(|entry| entry.choice.action_key.clone()),
        phase_signal_actions,
        root_action_prior_scored_actions,
        action_effect_samples,
    }
}

#[cfg(test)]
mod tests;
