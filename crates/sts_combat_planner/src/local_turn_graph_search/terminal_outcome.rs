use serde::Serialize;

/// Compact typed projection of one retained exact terminal witness.
///
/// Reports expose this fact vector instead of duplicating full action lines.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct LocalTurnGraphTerminalOutcomeSnapshotV1 {
    pub selected_by_local_hp_view: bool,
    pub final_hp: i32,
    pub final_max_hp: i32,
    pub recoverable_gold_delta: i32,
    pub recoverable_stolen_gold: i32,
    pub ritual_dagger_value: i32,
    pub genetic_algorithm_value: i32,
    pub external_burden_count: i32,
    pub potion_expenditures: u32,
    pub action_count: usize,
    pub negative_log_policy: f64,
}
