use super::super::{
    CombatSearchV2DiagnosticsCardIdentity, CombatSearchV2DiagnosticsCardIdentitySample,
};
use super::collector::{CardIdentityDiagnosticsCollector, CardIdentityObservedGroup};

impl CardIdentityDiagnosticsCollector {
    pub(in crate::ai::combat_search_v2) fn finish(&self) -> CombatSearchV2DiagnosticsCardIdentity {
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
