use crate::ai::strategy::deck_admission::{
    assess_deck_admission_from_inventory, DeckAdmission, DeckAdmissionContext,
};
use crate::ai::strategy::deck_construction_pressure::{
    assess_deck_construction_pressure, reward_construction_lane_adjustment,
    ConstructionLaneAdjustment, DeckConstructionContext, DeckConstructionPressure,
};
use crate::ai::strategy::deck_role_inventory::DeckRoleInventory;
use crate::ai::strategy::reward_admission::RewardAdmission;
use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckPlanSnapshot {
    pub context: DeckAdmissionContext,
    pub deck_size: usize,
    pub roles: DeckRoleInventory,
    pub construction: DeckConstructionPressure,
    pub run_facts: RunStrategicFacts,
}

impl DeckPlanSnapshot {
    pub fn from_run_state(run_state: &RunState) -> Self {
        Self::from_deck(
            &run_state.master_deck,
            DeckAdmissionContext {
                act: run_state.act_num,
                current_hp: run_state.current_hp,
                max_hp: run_state.max_hp,
            },
            RunStrategicFacts::from_run_state(run_state),
        )
    }

    pub fn from_deck(
        deck: &[CombatCard],
        context: DeckAdmissionContext,
        run_facts: RunStrategicFacts,
    ) -> Self {
        Self {
            context,
            deck_size: deck.len(),
            roles: DeckRoleInventory::from_deck(deck),
            construction: assess_deck_construction_pressure(
                deck,
                DeckConstructionContext { act: context.act },
            ),
            run_facts,
        }
    }

    pub fn survival_pressure(self) -> bool {
        self.context.survival_pressure()
    }

    pub fn deck_admission(self, admission: &RewardAdmission) -> DeckAdmission {
        assess_deck_admission_from_inventory(self.deck_size, self.context, &self.roles, admission)
    }

    pub fn reward_lane_adjustment(self, admission: &RewardAdmission) -> ConstructionLaneAdjustment {
        reward_construction_lane_adjustment(self.construction, admission)
    }
}
