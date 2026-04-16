mod catalog;
mod policy;
mod signals;
mod targets;
mod valuation;

use crate::runtime::combat::CombatState;
use crate::content::potions::{get_potion_definition, PotionId};
use crate::state::core::ClientInput;

pub(crate) use catalog::category_label;
pub(crate) use policy::{choose_immediate_potion_candidate, immediate_potion_snapshot};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotionCategory {
    Survival,
    Lethal,
    Setup,
    Recovery,
    Escape,
    RandomGeneration,
}

#[derive(Clone, Debug)]
pub struct PotionCandidate {
    pub potion_id: PotionId,
    pub input: ClientInput,
    pub priority: i32,
    pub base_priority: i32,
    pub special_priority: i32,
    pub target_priority: i32,
    pub reason: String,
    pub category: PotionCategory,
}

#[derive(Clone, Debug)]
pub struct PotionDecisionSnapshot {
    pub minimum_priority: i32,
    pub context_summary: String,
    pub chosen: Option<PotionCandidate>,
    pub candidates: Vec<PotionCandidate>,
}

impl PotionCandidate {
    pub fn debug_summary(&self, minimum_priority: i32) -> String {
        let potion_name = get_potion_definition(self.potion_id).name;
        let (slot, target) = match self.input {
            ClientInput::UsePotion {
                potion_index,
                target,
            } => (potion_index, target),
            _ => (usize::MAX, None),
        };
        let delta = self.priority - minimum_priority;
        format!(
            "slot={} potion={} category={} total={} delta_vs_min={:+} parts(base={}, special={}, target={}) target={:?} reason={}",
            slot,
            potion_name,
            category_label(self.category),
            self.priority,
            delta,
            self.base_priority,
            self.special_priority,
            self.target_priority,
            target,
            self.reason
        )
    }
}

pub fn choose_immediate_potion(combat: &CombatState) -> Option<ClientInput> {
    choose_immediate_potion_candidate(combat).map(|candidate| candidate.input)
}

pub fn candidate_potion_moves(combat: &CombatState) -> Vec<ClientInput> {
    policy::candidate_potion_moves(combat)
}
