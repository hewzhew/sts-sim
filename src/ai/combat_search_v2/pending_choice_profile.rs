use super::pending_choice_fanout::{fanout_class, pending_choice_fanout};
use super::*;

mod reporting;
mod types;

use types::PendingChoiceObservation;
pub(super) use types::{PendingChoiceDiagnosticsCollector, PendingChoiceProfile};

pub(super) fn summarize_pending_choice(engine: &EngineState) -> Option<PendingChoiceProfile> {
    let EngineState::PendingChoice(pending) = engine else {
        return None;
    };

    let profile = match pending {
        crate::state::core::PendingChoice::GridSelect {
            source_pile,
            reason,
            ..
        } => profile(
            "grid_select",
            Some(format!("{reason:?}")),
            Some(format!("{source_pile:?}")),
            pending,
        ),
        crate::state::core::PendingChoice::HandSelect { reason, .. } => profile(
            "hand_select",
            Some(format!("{reason:?}")),
            Some("Hand".to_string()),
            pending,
        ),
        crate::state::core::PendingChoice::DiscoverySelect(_) => profile(
            "discovery_select",
            Some("Discovery".to_string()),
            None,
            pending,
        ),
        crate::state::core::PendingChoice::ScrySelect { .. } => {
            profile("scry_select", Some("Scry".to_string()), None, pending)
        }
        crate::state::core::PendingChoice::CardRewardSelect { destination, .. } => profile(
            "card_reward_select",
            Some(format!("{destination:?}")),
            None,
            pending,
        ),
        crate::state::core::PendingChoice::ForeignInfluenceSelect { upgraded, .. } => profile(
            "foreign_influence_select",
            Some(format!("upgraded:{upgraded}")),
            None,
            pending,
        ),
        crate::state::core::PendingChoice::ChooseOneSelect { .. } => profile(
            "choose_one_select",
            Some("ChooseOne".to_string()),
            None,
            pending,
        ),
        crate::state::core::PendingChoice::StanceChoice => {
            profile("stance_choice", Some("Stance".to_string()), None, pending)
        }
    };

    Some(profile)
}

fn profile(
    kind: &'static str,
    reason: Option<String>,
    source_pile: Option<String>,
    choice: &crate::state::core::PendingChoice,
) -> PendingChoiceProfile {
    let fanout = pending_choice_fanout(choice);
    let (min_cards, max_cards, can_cancel) = choice_bounds(choice);
    let fanout_class = fanout_class(fanout.estimated_action_fanout);
    PendingChoiceProfile {
        kind,
        reason,
        source_pile,
        candidate_count: fanout.candidate_count,
        estimated_action_fanout: fanout.estimated_action_fanout,
        min_cards,
        max_cards,
        can_cancel,
        fanout_class,
        search_risk: search_risk(kind, fanout_class),
    }
}

fn choice_bounds(choice: &crate::state::core::PendingChoice) -> (usize, usize, bool) {
    match choice {
        crate::state::core::PendingChoice::GridSelect {
            min_cards,
            max_cards,
            can_cancel,
            ..
        }
        | crate::state::core::PendingChoice::HandSelect {
            min_cards,
            max_cards,
            can_cancel,
            ..
        } => (*min_cards as usize, *max_cards as usize, *can_cancel),
        crate::state::core::PendingChoice::DiscoverySelect(choice) => {
            (usize::from(!choice.can_skip), 1, choice.can_skip)
        }
        crate::state::core::PendingChoice::ScrySelect { cards, .. } => (0, cards.len(), false),
        crate::state::core::PendingChoice::CardRewardSelect { can_skip, .. } => {
            (usize::from(!can_skip), 1, *can_skip)
        }
        crate::state::core::PendingChoice::ForeignInfluenceSelect { .. }
        | crate::state::core::PendingChoice::ChooseOneSelect { .. }
        | crate::state::core::PendingChoice::StanceChoice => (1, 1, false),
    }
}

fn search_risk(kind: &str, fanout_class: &str) -> &'static str {
    match (kind, fanout_class) {
        ("grid_select" | "hand_select" | "scry_select", "large") => "high_fanout_pending_choice",
        ("grid_select" | "hand_select" | "scry_select", "medium") => {
            "exact_branching_pending_choice"
        }
        ("discovery_select" | "card_reward_select" | "foreign_influence_select", _) => {
            "generated_choice_branching"
        }
        ("choose_one_select" | "stance_choice", _) => "typed_small_choice",
        _ => "low_fanout_pending_choice",
    }
}

impl PendingChoiceDiagnosticsCollector {
    pub(super) fn observe(&mut self, profile: Option<&PendingChoiceProfile>) {
        self.states_observed = self.states_observed.saturating_add(1);
        let Some(profile) = profile else {
            return;
        };

        self.pending_choice_states = self.pending_choice_states.saturating_add(1);
        self.max_candidate_count = self.max_candidate_count.max(profile.candidate_count);
        if profile.search_risk == "high_fanout_pending_choice" {
            self.high_fanout_states = self.high_fanout_states.saturating_add(1);
        }

        let count = self.kind_counts.entry(profile.kind).or_default();
        count.states = count.states.saturating_add(1);
        count.max_candidate_count = count.max_candidate_count.max(profile.candidate_count);
        count.max_estimated_action_fanout = count
            .max_estimated_action_fanout
            .max(profile.estimated_action_fanout);
        self.remember_largest_pending_choice(PendingChoiceObservation {
            observed_at_state_query: self.states_observed,
            profile: profile.clone(),
        });
    }

    pub(super) fn observe_ordering(
        &mut self,
        profile: Option<&PendingChoiceProfile>,
        ordering: &ActionOrderingSummary,
    ) {
        if profile.is_none() {
            return;
        }

        self.expanded_pending_choice_states = self.expanded_pending_choice_states.saturating_add(1);
        self.legal_actions_from_pending_choice = self
            .legal_actions_from_pending_choice
            .saturating_add(ordering.action_count() as u64);
        self.max_legal_actions_from_pending_choice = self
            .max_legal_actions_from_pending_choice
            .max(ordering.action_count());

        for (role, count) in ordering.role_counts() {
            let mutable = self.ordering_role_counts.entry(role).or_default();
            mutable.actions = mutable.actions.saturating_add(count as u64);
        }
        if let Some(first_role) = ordering.first_role() {
            let mutable = self.ordering_role_counts.entry(first_role).or_default();
            mutable.first_actions = mutable.first_actions.saturating_add(1);
        }
    }

    pub(super) fn observe_child_transition(
        &mut self,
        profile: Option<&PendingChoiceProfile>,
        truncated: bool,
        child_engine: &EngineState,
    ) {
        if profile.is_none() {
            return;
        }

        if truncated {
            self.truncated_children = self.truncated_children.saturating_add(1);
        } else if matches!(child_engine, EngineState::PendingChoice(_)) {
            self.still_pending_children = self.still_pending_children.saturating_add(1);
        } else {
            self.resolved_children = self.resolved_children.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests;
