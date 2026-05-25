use super::*;
use std::collections::BTreeMap;

const LARGEST_TURN_FANOUT_SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct TurnBranchingStateObservation {
    parent_turn_count: u32,
    parent_energy: u8,
    legal_actions: usize,
    generated_children: usize,
    same_turn_children: usize,
    next_turn_children: usize,
    pending_choice_children: usize,
    terminal_children: usize,
    other_children: usize,
    end_turn_children: usize,
    transition_counts: BTreeMap<TurnBranchTransitionCountKey, usize>,
}

#[derive(Default)]
pub(super) struct TurnBranchingDiagnosticsCollector {
    states_observed: u64,
    total_legal_actions: u64,
    total_generated_children: u64,
    same_turn_children: u64,
    next_turn_children: u64,
    pending_choice_children: u64,
    terminal_children: u64,
    other_children: u64,
    end_turn_children: u64,
    transition_counts: BTreeMap<TurnBranchTransitionCountKey, u64>,
    largest_turn_fanouts: Vec<TurnBranchingStateObservation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TurnBranchTransition {
    action_kind: TurnBranchActionKind,
    kind: TurnBranchTransitionKind,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TurnBranchTransitionKind {
    SameTurn,
    NextTurn,
    PendingChoice,
    Terminal,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TurnBranchActionKind {
    PlayCard,
    EndTurn,
    UsePotion,
    DiscardPotion,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TurnBranchTransitionCountKey {
    action_kind: TurnBranchActionKind,
    transition_kind: TurnBranchTransitionKind,
}

pub(super) fn classify_turn_branch_transition(
    parent_engine: &EngineState,
    parent_combat: &CombatState,
    input: &ClientInput,
    child_engine: &EngineState,
    child_combat: &CombatState,
) -> TurnBranchTransition {
    let action_kind = TurnBranchActionKind::from_input(input);
    let kind = if terminal_label(child_engine, child_combat) != SearchTerminalLabel::Unresolved {
        TurnBranchTransitionKind::Terminal
    } else if matches!(child_engine, EngineState::PendingChoice(_)) {
        TurnBranchTransitionKind::PendingChoice
    } else if is_same_turn_continuation(parent_engine, parent_combat, child_engine, child_combat) {
        TurnBranchTransitionKind::SameTurn
    } else if is_next_turn_transition(parent_combat, child_combat) {
        TurnBranchTransitionKind::NextTurn
    } else {
        TurnBranchTransitionKind::Other
    };

    TurnBranchTransition { action_kind, kind }
}

fn is_same_turn_continuation(
    parent_engine: &EngineState,
    parent_combat: &CombatState,
    child_engine: &EngineState,
    child_combat: &CombatState,
) -> bool {
    matches!(parent_engine, EngineState::CombatPlayerTurn)
        && matches!(child_engine, EngineState::CombatPlayerTurn)
        && child_combat.turn.turn_count == parent_combat.turn.turn_count
}

fn is_next_turn_transition(parent_combat: &CombatState, child_combat: &CombatState) -> bool {
    child_combat.turn.turn_count > parent_combat.turn.turn_count
}

impl TurnBranchingStateObservation {
    pub(super) fn new(parent_combat: &CombatState, legal_actions: usize) -> Self {
        Self {
            parent_turn_count: parent_combat.turn.turn_count,
            parent_energy: parent_combat.turn.energy,
            legal_actions,
            generated_children: 0,
            same_turn_children: 0,
            next_turn_children: 0,
            pending_choice_children: 0,
            terminal_children: 0,
            other_children: 0,
            end_turn_children: 0,
            transition_counts: BTreeMap::new(),
        }
    }

    pub(super) fn observe_child(&mut self, transition: TurnBranchTransition) {
        self.generated_children = self.generated_children.saturating_add(1);
        match transition.kind {
            TurnBranchTransitionKind::SameTurn => {
                self.same_turn_children = self.same_turn_children.saturating_add(1)
            }
            TurnBranchTransitionKind::NextTurn => {
                self.next_turn_children = self.next_turn_children.saturating_add(1)
            }
            TurnBranchTransitionKind::PendingChoice => {
                self.pending_choice_children = self.pending_choice_children.saturating_add(1)
            }
            TurnBranchTransitionKind::Terminal => {
                self.terminal_children = self.terminal_children.saturating_add(1)
            }
            TurnBranchTransitionKind::Other => {
                self.other_children = self.other_children.saturating_add(1)
            }
        }
        if transition.action_kind == TurnBranchActionKind::EndTurn {
            self.end_turn_children = self.end_turn_children.saturating_add(1);
        }

        let key = TurnBranchTransitionCountKey {
            action_kind: transition.action_kind,
            transition_kind: transition.kind,
        };
        *self.transition_counts.entry(key).or_insert(0) += 1;
    }
}

impl TurnBranchTransition {
    pub(super) fn frontier_priority_hint(self) -> i32 {
        match (self.action_kind, self.kind) {
            (_, TurnBranchTransitionKind::Terminal) => 40,
            (_, TurnBranchTransitionKind::PendingChoice) => 15,
            (TurnBranchActionKind::PlayCard, TurnBranchTransitionKind::SameTurn) => 12,
            (TurnBranchActionKind::UsePotion, TurnBranchTransitionKind::SameTurn) => 8,
            (TurnBranchActionKind::EndTurn, TurnBranchTransitionKind::NextTurn) => 0,
            (TurnBranchActionKind::DiscardPotion, _) => -20,
            (_, TurnBranchTransitionKind::SameTurn) => 4,
            (_, TurnBranchTransitionKind::NextTurn) => 0,
            (_, TurnBranchTransitionKind::Other) => 0,
        }
    }

    pub(super) fn resets_turn_prefix(self) -> bool {
        matches!(self.kind, TurnBranchTransitionKind::NextTurn)
    }

    #[cfg(test)]
    pub(super) fn test_same_turn_play_card() -> Self {
        Self {
            action_kind: TurnBranchActionKind::PlayCard,
            kind: TurnBranchTransitionKind::SameTurn,
        }
    }

    #[cfg(test)]
    pub(super) fn test_next_turn_end_turn() -> Self {
        Self {
            action_kind: TurnBranchActionKind::EndTurn,
            kind: TurnBranchTransitionKind::NextTurn,
        }
    }
}

impl TurnBranchingDiagnosticsCollector {
    pub(super) fn observe(&mut self, observation: &TurnBranchingStateObservation) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.total_legal_actions = self
            .total_legal_actions
            .saturating_add(observation.legal_actions as u64);
        self.total_generated_children = self
            .total_generated_children
            .saturating_add(observation.generated_children as u64);
        self.same_turn_children = self
            .same_turn_children
            .saturating_add(observation.same_turn_children as u64);
        self.next_turn_children = self
            .next_turn_children
            .saturating_add(observation.next_turn_children as u64);
        self.pending_choice_children = self
            .pending_choice_children
            .saturating_add(observation.pending_choice_children as u64);
        self.terminal_children = self
            .terminal_children
            .saturating_add(observation.terminal_children as u64);
        self.other_children = self
            .other_children
            .saturating_add(observation.other_children as u64);
        self.end_turn_children = self
            .end_turn_children
            .saturating_add(observation.end_turn_children as u64);

        for (key, count) in &observation.transition_counts {
            *self.transition_counts.entry(*key).or_insert(0) += *count as u64;
        }
        self.remember_largest_turn_fanout(observation);
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsTurnBranching {
        CombatSearchV2DiagnosticsTurnBranching {
            organization_policy: "turn_transition_classification_with_late_frontier_tie_break",
            behavioral_effect: "diagnostic_summary_plus_priority_hint_no_prune_no_merge",
            states_observed: self.states_observed,
            total_legal_actions: self.total_legal_actions,
            total_generated_children: self.total_generated_children,
            generated_children_per_state: rounded_ratio(
                self.total_generated_children,
                self.states_observed,
            ),
            same_turn_children: self.same_turn_children,
            next_turn_children: self.next_turn_children,
            pending_choice_children: self.pending_choice_children,
            terminal_children: self.terminal_children,
            other_children: self.other_children,
            end_turn_children: self.end_turn_children,
            same_turn_child_ratio: rounded_ratio(
                self.same_turn_children,
                self.total_generated_children,
            ),
            next_turn_child_ratio: rounded_ratio(
                self.next_turn_children,
                self.total_generated_children,
            ),
            transition_counts: self.transition_count_reports(),
            largest_turn_fanouts: self.largest_turn_fanout_reports(),
            notes: vec![
                "turn branching classifies generated children after simulator execution",
                "same_turn and next_turn are transition labels, not pruning rules",
                "priority hints are late frontier tie-breaks and do not remove legal actions",
                "future turn-prefix dominance must prove safety before pruning",
            ],
        }
    }

    fn remember_largest_turn_fanout(&mut self, observation: &TurnBranchingStateObservation) {
        if observation.generated_children <= 1 {
            return;
        }
        self.largest_turn_fanouts.push(observation.clone());
        self.largest_turn_fanouts.sort_by(|left, right| {
            right
                .generated_children
                .cmp(&left.generated_children)
                .then_with(|| right.same_turn_children.cmp(&left.same_turn_children))
                .then_with(|| left.parent_turn_count.cmp(&right.parent_turn_count))
        });
        self.largest_turn_fanouts
            .truncate(LARGEST_TURN_FANOUT_SAMPLE_LIMIT);
    }

    fn transition_count_reports(&self) -> Vec<CombatSearchV2DiagnosticsTurnTransitionCount> {
        self.transition_counts
            .iter()
            .map(
                |(key, children)| CombatSearchV2DiagnosticsTurnTransitionCount {
                    action_kind: key.action_kind.label().to_string(),
                    transition_kind: key.transition_kind.label().to_string(),
                    children: *children,
                },
            )
            .collect()
    }

    fn largest_turn_fanout_reports(&self) -> Vec<CombatSearchV2DiagnosticsTurnFanoutSample> {
        self.largest_turn_fanouts
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsTurnFanoutSample {
                parent_turn_count: sample.parent_turn_count,
                parent_energy: sample.parent_energy,
                legal_actions: sample.legal_actions,
                generated_children: sample.generated_children,
                same_turn_children: sample.same_turn_children,
                next_turn_children: sample.next_turn_children,
                pending_choice_children: sample.pending_choice_children,
                terminal_children: sample.terminal_children,
                end_turn_children: sample.end_turn_children,
            })
            .collect()
    }
}

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
}

impl TurnBranchActionKind {
    fn from_input(input: &ClientInput) -> Self {
        match input {
            ClientInput::PlayCard { .. } => Self::PlayCard,
            ClientInput::EndTurn => Self::EndTurn,
            ClientInput::UsePotion { .. } => Self::UsePotion,
            ClientInput::DiscardPotion(_) => Self::DiscardPotion,
            _ => Self::Other,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::PlayCard => "play_card",
            Self::EndTurn => "end_turn",
            Self::UsePotion => "use_potion",
            Self::DiscardPotion => "discard_potion",
            Self::Other => "other",
        }
    }
}

impl TurnBranchTransitionKind {
    fn label(self) -> &'static str {
        match self {
            Self::SameTurn => "same_turn",
            Self::NextTurn => "next_turn",
            Self::PendingChoice => "pending_choice",
            Self::Terminal => "terminal",
            Self::Other => "other",
        }
    }
}

#[cfg(test)]
mod tests;
