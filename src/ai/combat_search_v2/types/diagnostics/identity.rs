use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsCardIdentity {
    pub audit_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub states_observed: u64,
    pub active_cards_observed: u64,
    pub action_payload_cards_observed: u64,
    pub action_payload_placeholder_cards: u64,
    pub states_with_duplicate_active_uuid: u64,
    pub duplicate_active_uuid_observations: u64,
    pub states_with_uuid_card_id_conflict: u64,
    pub uuid_card_id_conflict_observations: u64,
    pub max_duplicate_group_size: usize,
    pub largest_duplicate_groups: Vec<CombatSearchV2DiagnosticsCardIdentitySample>,
    pub largest_conflict_groups: Vec<CombatSearchV2DiagnosticsCardIdentitySample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2DiagnosticsCardIdentitySample {
    pub observed_at_state_query: u64,
    pub uuid: u32,
    pub occurrence_count: usize,
    pub distinct_card_labels: Vec<String>,
    pub locations: Vec<String>,
}
