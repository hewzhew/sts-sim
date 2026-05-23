use super::*;
use crate::sim::combat_identity::{audit_combat_card_identity, CombatCardIdentityGroup};

const SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug, Default)]
pub(super) struct CardIdentitySummary {
    active_cards: usize,
    action_payload_cards: usize,
    action_payload_placeholder_cards: usize,
    duplicate_groups: Vec<CardIdentityGroupSummary>,
    conflict_groups: Vec<CardIdentityGroupSummary>,
}

#[derive(Clone, Debug)]
struct CardIdentityGroupSummary {
    uuid: u32,
    occurrence_count: usize,
    distinct_card_labels: Vec<String>,
    locations: Vec<String>,
}

#[derive(Default)]
pub(super) struct CardIdentityDiagnosticsCollector {
    states_observed: u64,
    active_cards_observed: u64,
    action_payload_cards_observed: u64,
    action_payload_placeholder_cards: u64,
    states_with_duplicate_active_uuid: u64,
    duplicate_active_uuid_observations: u64,
    states_with_uuid_card_id_conflict: u64,
    uuid_card_id_conflict_observations: u64,
    max_duplicate_group_size: usize,
    largest_duplicate_groups: Vec<CardIdentityObservedGroup>,
    largest_conflict_groups: Vec<CardIdentityObservedGroup>,
}

#[derive(Clone, Debug)]
struct CardIdentityObservedGroup {
    observed_at_state_query: u64,
    group: CardIdentityGroupSummary,
}

pub(super) fn summarize_card_identity(combat: &CombatState) -> CardIdentitySummary {
    let audit = audit_combat_card_identity(combat);

    CardIdentitySummary {
        active_cards: audit.active_cards,
        action_payload_cards: audit.action_payload_cards,
        action_payload_placeholder_cards: audit.action_payload_placeholder_cards,
        duplicate_groups: audit
            .duplicate_active_uuid_groups
            .iter()
            .map(group_summary)
            .collect(),
        conflict_groups: audit
            .uuid_card_id_conflict_groups
            .iter()
            .map(group_summary)
            .collect(),
    }
}

fn group_summary(group: &CombatCardIdentityGroup) -> CardIdentityGroupSummary {
    CardIdentityGroupSummary {
        uuid: group.uuid,
        occurrence_count: group.occurrence_count,
        distinct_card_labels: group.distinct_card_labels.clone(),
        locations: group.locations.clone(),
    }
}

impl CardIdentityDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &CardIdentitySummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        self.active_cards_observed = self
            .active_cards_observed
            .saturating_add(summary.active_cards as u64);
        self.action_payload_cards_observed = self
            .action_payload_cards_observed
            .saturating_add(summary.action_payload_cards as u64);
        self.action_payload_placeholder_cards = self
            .action_payload_placeholder_cards
            .saturating_add(summary.action_payload_placeholder_cards as u64);

        if !summary.duplicate_groups.is_empty() {
            self.states_with_duplicate_active_uuid =
                self.states_with_duplicate_active_uuid.saturating_add(1);
            self.duplicate_active_uuid_observations = self
                .duplicate_active_uuid_observations
                .saturating_add(summary.duplicate_groups.len() as u64);
        }
        if !summary.conflict_groups.is_empty() {
            self.states_with_uuid_card_id_conflict =
                self.states_with_uuid_card_id_conflict.saturating_add(1);
            self.uuid_card_id_conflict_observations = self
                .uuid_card_id_conflict_observations
                .saturating_add(summary.conflict_groups.len() as u64);
        }
        for group in &summary.duplicate_groups {
            self.max_duplicate_group_size =
                self.max_duplicate_group_size.max(group.occurrence_count);
            remember_group(
                &mut self.largest_duplicate_groups,
                self.states_observed,
                group,
            );
        }
        for group in &summary.conflict_groups {
            remember_group(
                &mut self.largest_conflict_groups,
                self.states_observed,
                group,
            );
        }
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsCardIdentity {
        CombatSearchV2DiagnosticsCardIdentity {
            audit_policy: "active_combat_card_uuid_scan_plus_action_payload_placeholder_count",
            behavioral_effect: "diagnostic_only_no_search_ordering_no_prune_no_merge",
            states_observed: self.states_observed,
            active_cards_observed: self.active_cards_observed,
            action_payload_cards_observed: self.action_payload_cards_observed,
            action_payload_placeholder_cards: self.action_payload_placeholder_cards,
            states_with_duplicate_active_uuid: self.states_with_duplicate_active_uuid,
            duplicate_active_uuid_observations: self.duplicate_active_uuid_observations,
            states_with_uuid_card_id_conflict: self.states_with_uuid_card_id_conflict,
            uuid_card_id_conflict_observations: self.uuid_card_id_conflict_observations,
            max_duplicate_group_size: self.max_duplicate_group_size,
            largest_duplicate_groups: observed_group_reports(&self.largest_duplicate_groups),
            largest_conflict_groups: observed_group_reports(&self.largest_conflict_groups),
            notes: vec![
                "active cards include hand/draw/discard/exhaust/limbo and queued card plays",
                "action payload cards are counted separately because queued actions can hold constructors or direct-play payloads",
                "uuid 0 action payloads are placeholders before materialization, not active card instances",
                "duplicate active uuid observations are diagnostics and do not prove unsoundness without inspecting locations",
            ],
        }
    }
}

fn remember_group(
    groups: &mut Vec<CardIdentityObservedGroup>,
    observed_at_state_query: u64,
    group: &CardIdentityGroupSummary,
) {
    groups.push(CardIdentityObservedGroup {
        observed_at_state_query,
        group: group.clone(),
    });
    groups.sort_by(|left, right| {
        right
            .group
            .occurrence_count
            .cmp(&left.group.occurrence_count)
            .then_with(|| {
                right
                    .group
                    .distinct_card_labels
                    .len()
                    .cmp(&left.group.distinct_card_labels.len())
            })
            .then_with(|| {
                left.observed_at_state_query
                    .cmp(&right.observed_at_state_query)
            })
    });
    groups.truncate(SAMPLE_LIMIT);
}

fn observed_group_reports(
    groups: &[CardIdentityObservedGroup],
) -> Vec<CombatSearchV2DiagnosticsCardIdentitySample> {
    groups
        .iter()
        .map(|group| CombatSearchV2DiagnosticsCardIdentitySample {
            observed_at_state_query: group.observed_at_state_query,
            uuid: group.group.uuid,
            occurrence_count: group.group.occurrence_count,
            distinct_card_labels: group.group.distinct_card_labels.clone(),
            locations: group.group.locations.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::action::Action;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::blank_test_combat;

    #[test]
    fn duplicate_active_uuid_is_reported() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 10),
        ];

        let summary = summarize_card_identity(&combat);
        let mut collector = CardIdentityDiagnosticsCollector::default();
        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(report.states_with_duplicate_active_uuid, 1);
        assert_eq!(report.states_with_uuid_card_id_conflict, 1);
        assert_eq!(report.largest_conflict_groups[0].uuid, 10);
        assert_eq!(report.largest_conflict_groups[0].occurrence_count, 2);
    }

    #[test]
    fn action_payload_placeholder_is_not_active_duplicate() {
        let mut combat = blank_test_combat();
        combat
            .engine
            .action_queue
            .push_back(Action::MakeCopyInHand {
                original: Box::new(CombatCard::new(CardId::Strike, 0)),
                amount: 1,
            });

        let summary = summarize_card_identity(&combat);
        let mut collector = CardIdentityDiagnosticsCollector::default();
        collector.observe(&summary);
        let report = collector.finish();

        assert_eq!(report.states_with_duplicate_active_uuid, 0);
        assert_eq!(report.action_payload_cards_observed, 1);
        assert_eq!(report.action_payload_placeholder_cards, 1);
    }
}
