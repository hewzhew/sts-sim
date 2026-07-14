use crate::content::cards::{get_card_definition, is_starter_basic, CardType};
use crate::content::relics::{energy_master_delta, RelicId};
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunStrategicFacts {
    pub entering_act: u8,
    pub starter_basic_count: usize,
    pub curse_count: usize,
    pub has_energy_relic: bool,
    pub has_runic_pyramid: bool,
}

impl RunStrategicFacts {
    pub fn from_run_state(run_state: &RunState) -> Self {
        Self {
            entering_act: run_state.act_num.saturating_add(1),
            starter_basic_count: run_state
                .master_deck
                .iter()
                .filter(|card| is_starter_basic(card.id))
                .count(),
            curse_count: run_state
                .master_deck
                .iter()
                .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
                .count(),
            has_energy_relic: run_state
                .relics
                .iter()
                .any(|relic| energy_master_delta(relic.id) > 0),
            has_runic_pyramid: run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::RunicPyramid),
        }
    }

    pub fn has_act2_energy_gap(&self) -> bool {
        self.entering_act == 2 && !self.has_energy_relic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::{RelicId, RelicState};

    #[test]
    fn strategic_facts_report_runic_pyramid_without_policy_judgment() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        run.relics.push(RelicState::new(RelicId::RunicPyramid));

        assert!(RunStrategicFacts::from_run_state(&run).has_runic_pyramid);
    }
}
