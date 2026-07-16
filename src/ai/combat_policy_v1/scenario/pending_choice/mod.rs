mod enumerate;
mod project;
mod types;

pub(super) use enumerate::exact_pending_choice_inputs;
pub(super) use project::{public_pending_choice_action, public_pending_choice_observation};
pub use types::*;

pub(super) fn pending_choice_kind(
    choice: &crate::state::core::PendingChoice,
) -> CombatPublicPendingChoiceKindV1 {
    use crate::state::core::PendingChoice;

    match choice {
        PendingChoice::HandSelect { .. } => CombatPublicPendingChoiceKindV1::HandSelect,
        PendingChoice::GridSelect { .. } => CombatPublicPendingChoiceKindV1::GridSelect,
        PendingChoice::DiscoverySelect(_) => CombatPublicPendingChoiceKindV1::DiscoverySelect,
        PendingChoice::ScrySelect { .. } => CombatPublicPendingChoiceKindV1::ScrySelect,
        PendingChoice::CardRewardSelect { .. } => CombatPublicPendingChoiceKindV1::CardRewardSelect,
        PendingChoice::ForeignInfluenceSelect { .. } => {
            CombatPublicPendingChoiceKindV1::ForeignInfluenceSelect
        }
        PendingChoice::ChooseOneSelect { .. } => CombatPublicPendingChoiceKindV1::ChooseOneSelect,
        PendingChoice::StanceChoice => CombatPublicPendingChoiceKindV1::StanceChoice,
    }
}
