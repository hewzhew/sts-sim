use super::{
    CombatPolicyInformationSetKeyV1, CombatPolicyObservationEnvelopeV1, CombatPublicActionV1,
    CombatTurnOptionExpansionBudgetSnapshotV1, CombatTurnOptionObservableEffectEvidenceV1,
    CombatTurnOptionPrefixCandidateV1,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatTurnOptionCandidateExpansionStateV1 {
    Unopened,
    Expanded,
    TransitionConsumed,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatTurnOptionWideningCandidateViewV1<'a> {
    pub action: &'a CombatPublicActionV1,
    pub state: CombatTurnOptionCandidateExpansionStateV1,
    pub result: Option<&'a CombatTurnOptionPrefixCandidateV1>,
    pub observable_effect: Option<&'a CombatTurnOptionObservableEffectEvidenceV1>,
}

#[derive(Clone, Debug)]
pub struct CombatTurnOptionWideningContextV1<'a> {
    pub information_set: &'a CombatPolicyInformationSetKeyV1,
    pub observation: &'a CombatPolicyObservationEnvelopeV1,
    pub scenario_count: usize,
    pub candidates: Vec<CombatTurnOptionWideningCandidateViewV1<'a>>,
    pub expansion_order: &'a [CombatPublicActionV1],
    pub budget: CombatTurnOptionExpansionBudgetSnapshotV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatTurnOptionWideningChoiceV1 {
    Expand { action: CombatPublicActionV1 },
    Exhausted,
}

pub trait CombatTurnOptionWideningScheduleV1 {
    fn select_next(
        &self,
        context: &CombatTurnOptionWideningContextV1<'_>,
    ) -> CombatTurnOptionWideningChoiceV1;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StableEnumerationCombatTurnOptionWideningScheduleV1;

impl CombatTurnOptionWideningScheduleV1 for StableEnumerationCombatTurnOptionWideningScheduleV1 {
    fn select_next(
        &self,
        context: &CombatTurnOptionWideningContextV1<'_>,
    ) -> CombatTurnOptionWideningChoiceV1 {
        context
            .candidates
            .iter()
            .find(|candidate| {
                candidate.state == CombatTurnOptionCandidateExpansionStateV1::Unopened
            })
            .map(|candidate| CombatTurnOptionWideningChoiceV1::Expand {
                action: candidate.action.clone(),
            })
            .unwrap_or(CombatTurnOptionWideningChoiceV1::Exhausted)
    }
}
