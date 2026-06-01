#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) enum PendingChoiceOrderingRole {
    ValueSelection,
    RemovalSelection,
    NeutralSelection,
    Cancel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct PendingChoiceOrderingHint {
    pub(in crate::ai::combat_search_v2) primary: i32,
    pub(in crate::ai::combat_search_v2) secondary: i32,
    pub(in crate::ai::combat_search_v2) selected_count_tiebreak: i32,
    pub(in crate::ai::combat_search_v2) role: PendingChoiceOrderingRole,
}
