use crate::ai::strategy::deck_admission::{
    assess_deck_admission_from_inventory, DeckAdmission, DeckAdmissionContext,
};
use crate::ai::strategy::deck_construction_pressure::{
    assess_deck_construction_pressure, reward_construction_lane_adjustment,
    ConstructionLaneAdjustment, DeckConstructionContext, DeckConstructionPressure,
};
use crate::ai::strategy::deck_role_inventory::{card_is_stable_strength_source, DeckRoleInventory};
use crate::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit_summary, DeckStrategicDeficitSummary, StrategicDeficitLevel,
};
use crate::ai::strategy::reward_admission::RewardAdmission;
use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckPlanSnapshot {
    pub context: DeckAdmissionContext,
    pub boss_key: Option<EncounterId>,
    pub deck_size: usize,
    pub roles: DeckRoleInventory,
    pub construction: DeckConstructionPressure,
    pub strategic_deficit: DeckStrategicDeficitSummary,
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
        .with_boss_key(run_state.boss_key)
    }

    pub fn from_deck(
        deck: &[CombatCard],
        context: DeckAdmissionContext,
        run_facts: RunStrategicFacts,
    ) -> Self {
        Self {
            context,
            boss_key: None,
            deck_size: deck.len(),
            roles: DeckRoleInventory::from_deck(deck),
            construction: assess_deck_construction_pressure(
                deck,
                DeckConstructionContext { act: context.act },
            ),
            strategic_deficit: assess_deck_strategic_deficit_summary(deck, run_facts),
            run_facts,
        }
    }

    pub fn with_boss_key(mut self, boss_key: Option<EncounterId>) -> Self {
        self.boss_key = boss_key;
        self
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

    pub fn repairs_strength_package_reliability(self, candidate: Option<(CardId, u8)>) -> bool {
        self.roles.strength_multiplier_units > 0
            && self.roles.strength_source_units == 1
            && matches!(
                self.strategic_deficit.boss_scaling_plan,
                StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
            )
            && candidate
                .is_some_and(|(card, upgrades)| card_is_stable_strength_source(card, upgrades))
    }
}
