use super::phase_profile::{PendingChoicePhaseKind, PendingChoicePhaseProfileV1};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct RolloutPendingChoiceProgress {
    pub(super) pending_choices_seen: usize,
    pub(super) pending_choice_actions_simulated: usize,
    pub(super) max_pending_choice_candidate_count: usize,
    pub(super) max_pending_choice_estimated_action_fanout: usize,
    pub(super) last_pending_choice_kind: Option<PendingChoicePhaseKind>,
    pub(super) stopped_on_high_fanout_pending_choice: bool,
}

impl RolloutPendingChoiceProgress {
    pub(super) fn observe_boundary(&mut self, pending_choice: PendingChoicePhaseProfileV1) {
        if !pending_choice.present {
            return;
        }

        self.pending_choices_seen = self.pending_choices_seen.saturating_add(1);
        self.max_pending_choice_candidate_count = self
            .max_pending_choice_candidate_count
            .max(pending_choice.candidate_count);
        self.max_pending_choice_estimated_action_fanout = self
            .max_pending_choice_estimated_action_fanout
            .max(pending_choice.estimated_action_fanout);
        self.last_pending_choice_kind = pending_choice.kind;
        if pending_choice.high_fanout {
            self.stopped_on_high_fanout_pending_choice = true;
        }
    }

    pub(super) fn note_simulated_action(&mut self, pending_choice: PendingChoicePhaseProfileV1) {
        if pending_choice.present {
            self.pending_choice_actions_simulated =
                self.pending_choice_actions_simulated.saturating_add(1);
        }
    }

    pub(super) fn last_pending_choice_kind_label(self) -> Option<&'static str> {
        self.last_pending_choice_kind
            .map(PendingChoicePhaseKind::label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_tracks_high_fanout_boundary_without_action() {
        let mut progress = RolloutPendingChoiceProgress::default();

        progress.observe_boundary(PendingChoicePhaseProfileV1 {
            present: true,
            kind: Some(PendingChoicePhaseKind::ScrySelect),
            candidate_count: 7,
            estimated_action_fanout: 128,
            high_fanout: true,
        });

        assert_eq!(progress.pending_choices_seen, 1);
        assert_eq!(progress.pending_choice_actions_simulated, 0);
        assert_eq!(progress.max_pending_choice_candidate_count, 7);
        assert_eq!(progress.max_pending_choice_estimated_action_fanout, 128);
        assert_eq!(
            progress.last_pending_choice_kind_label(),
            Some("scry_select")
        );
        assert!(progress.stopped_on_high_fanout_pending_choice);
    }
}
