use super::*;
use std::collections::BTreeMap;

const LARGEST_PENDING_CHOICE_SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct PendingChoiceProfile {
    kind: &'static str,
    reason: Option<String>,
    source_pile: Option<String>,
    candidate_count: usize,
    min_cards: usize,
    max_cards: usize,
    can_cancel: bool,
    fanout_class: &'static str,
    search_risk: &'static str,
}

#[derive(Default)]
pub(super) struct PendingChoiceDiagnosticsCollector {
    states_observed: u64,
    pending_choice_states: u64,
    high_fanout_states: u64,
    max_candidate_count: usize,
    kind_counts: BTreeMap<&'static str, MutablePendingChoiceKindCount>,
    largest_pending_choices: Vec<PendingChoiceObservation>,
}

#[derive(Clone, Debug, Default)]
struct MutablePendingChoiceKindCount {
    states: u64,
    max_candidate_count: usize,
}

#[derive(Clone, Debug)]
struct PendingChoiceObservation {
    observed_at_state_query: u64,
    profile: PendingChoiceProfile,
}

pub(super) fn summarize_pending_choice(engine: &EngineState) -> Option<PendingChoiceProfile> {
    let EngineState::PendingChoice(choice) = engine else {
        return None;
    };

    let profile = match choice {
        crate::state::core::PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => profile(
            "grid_select",
            Some(format!("{reason:?}")),
            Some(format!("{source_pile:?}")),
            candidate_uuids.len(),
            *min_cards as usize,
            *max_cards as usize,
            *can_cancel,
        ),
        crate::state::core::PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => profile(
            "hand_select",
            Some(format!("{reason:?}")),
            Some("Hand".to_string()),
            candidate_uuids.len(),
            *min_cards as usize,
            *max_cards as usize,
            *can_cancel,
        ),
        crate::state::core::PendingChoice::DiscoverySelect(choice) => profile(
            "discovery_select",
            Some("Discovery".to_string()),
            None,
            choice.cards.len(),
            usize::from(!choice.can_skip),
            1,
            choice.can_skip,
        ),
        crate::state::core::PendingChoice::ScrySelect { cards, .. } => profile(
            "scry_select",
            Some("Scry".to_string()),
            None,
            cards.len(),
            0,
            cards.len(),
            false,
        ),
        crate::state::core::PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => profile(
            "card_reward_select",
            Some(format!("{destination:?}")),
            None,
            cards.len(),
            usize::from(!can_skip),
            1,
            *can_skip,
        ),
        crate::state::core::PendingChoice::ForeignInfluenceSelect { cards, upgraded } => profile(
            "foreign_influence_select",
            Some(format!("upgraded:{upgraded}")),
            None,
            cards.len(),
            1,
            1,
            false,
        ),
        crate::state::core::PendingChoice::ChooseOneSelect { choices } => profile(
            "choose_one_select",
            Some("ChooseOne".to_string()),
            None,
            choices.len(),
            1,
            1,
            false,
        ),
        crate::state::core::PendingChoice::StanceChoice => profile(
            "stance_choice",
            Some("Stance".to_string()),
            None,
            2,
            1,
            1,
            false,
        ),
    };

    Some(profile)
}

fn profile(
    kind: &'static str,
    reason: Option<String>,
    source_pile: Option<String>,
    candidate_count: usize,
    min_cards: usize,
    max_cards: usize,
    can_cancel: bool,
) -> PendingChoiceProfile {
    let fanout_class = fanout_class(candidate_count);
    PendingChoiceProfile {
        kind,
        reason,
        source_pile,
        candidate_count,
        min_cards,
        max_cards,
        can_cancel,
        fanout_class,
        search_risk: search_risk(kind, fanout_class),
    }
}

fn fanout_class(candidate_count: usize) -> &'static str {
    match candidate_count {
        0 => "empty",
        1..=3 => "small",
        4..=8 => "medium",
        _ => "large",
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
        if profile.fanout_class == "large" {
            self.high_fanout_states = self.high_fanout_states.saturating_add(1);
        }

        let count = self.kind_counts.entry(profile.kind).or_default();
        count.states = count.states.saturating_add(1);
        count.max_candidate_count = count.max_candidate_count.max(profile.candidate_count);
        self.remember_largest_pending_choice(PendingChoiceObservation {
            observed_at_state_query: self.states_observed,
            profile: profile.clone(),
        });
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsPendingChoice {
        CombatSearchV2DiagnosticsPendingChoice {
            profiling_policy: "typed_pending_choice_profile_no_prune_no_auto_resolution",
            behavioral_effect: "diagnostic_only_search_expansion_unchanged",
            states_observed: self.states_observed,
            pending_choice_states: self.pending_choice_states,
            high_fanout_states: self.high_fanout_states,
            max_candidate_count: self.max_candidate_count,
            kind_counts: self.kind_count_reports(),
            largest_pending_choices: self.largest_pending_choice_reports(),
            notes: vec![
                "pending choice profile only classifies choice boundaries; it does not resolve or prune them",
                "large grid/hand/scry choices are search-risk signals, not evidence that any branch is safe to drop",
                "future compression must prove selection equivalence or order-insensitivity before pruning",
            ],
        }
    }

    fn remember_largest_pending_choice(&mut self, observation: PendingChoiceObservation) {
        if observation.profile.candidate_count <= 1 {
            return;
        }
        self.largest_pending_choices.push(observation);
        self.largest_pending_choices.sort_by(|left, right| {
            right
                .profile
                .candidate_count
                .cmp(&left.profile.candidate_count)
                .then_with(|| left.profile.kind.cmp(right.profile.kind))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_pending_choices
            .truncate(LARGEST_PENDING_CHOICE_SAMPLE_LIMIT);
    }

    fn kind_count_reports(&self) -> Vec<CombatSearchV2DiagnosticsPendingChoiceKindCount> {
        self.kind_counts
            .iter()
            .map(
                |(kind, count)| CombatSearchV2DiagnosticsPendingChoiceKindCount {
                    kind: (*kind).to_string(),
                    states: count.states,
                    max_candidate_count: count.max_candidate_count,
                },
            )
            .collect()
    }

    fn largest_pending_choice_reports(&self) -> Vec<CombatSearchV2DiagnosticsPendingChoiceSample> {
        self.largest_pending_choices
            .iter()
            .map(|observation| {
                let profile = &observation.profile;
                CombatSearchV2DiagnosticsPendingChoiceSample {
                    observed_at_state_query: observation.observed_at_state_query,
                    kind: profile.kind.to_string(),
                    reason: profile.reason.clone(),
                    source_pile: profile.source_pile.clone(),
                    candidate_count: profile.candidate_count,
                    min_cards: profile.min_cards,
                    max_cards: profile.max_cards,
                    can_cancel: profile.can_cancel,
                    fanout_class: profile.fanout_class,
                    search_risk: profile.search_risk,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_select_profile_marks_large_choices_as_high_fanout() {
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Draw,
            candidate_uuids: (0..12).collect(),
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::core::GridSelectReason::DrawPileToHand,
        });

        let profile = summarize_pending_choice(&engine).expect("pending choice should profile");

        assert_eq!(profile.kind, "grid_select");
        assert_eq!(profile.candidate_count, 12);
        assert_eq!(profile.fanout_class, "large");
        assert_eq!(profile.search_risk, "high_fanout_pending_choice");
    }

    #[test]
    fn collector_reports_pending_choice_without_behavioral_claim() {
        let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::StanceChoice);
        let profile = summarize_pending_choice(&engine);
        let mut collector = PendingChoiceDiagnosticsCollector::default();

        collector.observe(profile.as_ref());
        let report = collector.finish();

        assert_eq!(
            report.behavioral_effect,
            "diagnostic_only_search_expansion_unchanged"
        );
        assert_eq!(report.pending_choice_states, 1);
        assert_eq!(report.max_candidate_count, 2);
        assert_eq!(report.kind_counts[0].kind, "stance_choice");
    }
}
