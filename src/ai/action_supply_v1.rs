use crate::ai::card_semantics_v1::{
    card_mechanics_profile_v1, relic_mechanics_profile_v1, ActionSupplyTraitsV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionSupplySourceV1 {
    Card(CardId),
    Relic(RelicId),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ActionSupplySourceFactV1 {
    pub source: ActionSupplySourceV1,
    pub traits: ActionSupplyTraitsV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct ActionSupplyProfileV1 {
    pub opening_once_options: u8,
    pub delayed_per_turn_sources: u8,
    pub same_turn_burst_sources: u8,
    pub triggered_repeatable_sources: u8,
    pub additional_play_sources: u8,
    pub cost_or_resource_compression_sources: u8,
    pub potentially_recursive_sources: u8,
    pub sources: Vec<ActionSupplySourceFactV1>,
}

impl ActionSupplyProfileV1 {
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    fn record(&mut self, source: ActionSupplySourceV1, traits: ActionSupplyTraitsV1) {
        if traits.is_empty() {
            return;
        }
        self.opening_once_options = self
            .opening_once_options
            .saturating_add(traits.opening_once_options);
        self.delayed_per_turn_sources = self
            .delayed_per_turn_sources
            .saturating_add(u8::from(traits.delayed_per_turn));
        self.same_turn_burst_sources = self
            .same_turn_burst_sources
            .saturating_add(u8::from(traits.same_turn_burst()));
        self.triggered_repeatable_sources = self
            .triggered_repeatable_sources
            .saturating_add(u8::from(traits.triggered_repeatable));
        self.additional_play_sources = self
            .additional_play_sources
            .saturating_add(u8::from(traits.additional_play));
        self.cost_or_resource_compression_sources = self
            .cost_or_resource_compression_sources
            .saturating_add(u8::from(traits.cost_or_resource_compression));
        self.potentially_recursive_sources = self
            .potentially_recursive_sources
            .saturating_add(u8::from(traits.potentially_recursive));
        self.sources
            .push(ActionSupplySourceFactV1 { source, traits });
    }
}

pub fn action_supply_profile_v1(run_state: &RunState) -> ActionSupplyProfileV1 {
    let mut profile = ActionSupplyProfileV1::default();
    for relic in &run_state.relics {
        let traits = relic_mechanics_profile_v1(relic.id).action_supply;
        profile.record(ActionSupplySourceV1::Relic(relic.id), traits);
    }
    for card in &run_state.master_deck {
        let traits = card_mechanics_profile_v1(card.id).action_supply;
        profile.record(ActionSupplySourceV1::Card(card.id), traits);
    }
    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;

    #[test]
    fn run_profile_keeps_opening_supply_neutral_and_repeatable_supply_separate() {
        let mut run = RunState::new(20260711002, 0, false, "Ironclad");
        run.relics = vec![
            RelicState::new(RelicId::Enchiridion),
            RelicState::new(RelicId::DeadBranch),
        ];
        run.add_card_to_deck(CardId::BladeDance);
        run.add_card_to_deck(CardId::DoubleTap);
        run.add_card_to_deck(CardId::Corruption);

        let profile = action_supply_profile_v1(&run);

        assert_eq!(profile.opening_once_options, 1);
        assert_eq!(profile.delayed_per_turn_sources, 0);
        assert_eq!(profile.same_turn_burst_sources, 1);
        assert_eq!(profile.triggered_repeatable_sources, 1);
        assert_eq!(profile.additional_play_sources, 1);
        assert_eq!(profile.cost_or_resource_compression_sources, 1);
        assert_eq!(profile.potentially_recursive_sources, 1);
        assert_eq!(profile.sources.len(), 5);
    }

    #[test]
    fn unknown_or_ordinary_mechanics_do_not_create_action_supply() {
        let mut run = RunState::new(20260711003, 0, false, "Ironclad");
        run.add_card_to_deck(CardId::Strike);
        run.relics.push(RelicState::new(RelicId::Vajra));

        assert!(action_supply_profile_v1(&run).is_empty());
    }
}
