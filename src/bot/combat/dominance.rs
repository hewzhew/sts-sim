use crate::state::core::ClientInput;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TurnResourceSummary {
    pub spent_potions: u8,
    pub hp_lost: i32,
    pub exhausted_cards: u16,
    pub final_hp: i32,
    pub final_block: i32,
}

impl TurnResourceSummary {
    pub fn at_frontier(final_hp: i32, final_block: i32) -> Self {
        Self {
            final_hp,
            final_block,
            ..Self::default()
        }
    }

    pub fn with_transition(
        mut self,
        input: &ClientInput,
        before_hp: i32,
        after_hp: i32,
        exhausted_delta: usize,
    ) -> Self {
        if matches!(
            input,
            ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
        ) {
            self.spent_potions = self.spent_potions.saturating_add(1);
        }
        self.hp_lost += (before_hp - after_hp).max(0);
        self.exhausted_cards = self
            .exhausted_cards
            .saturating_add(exhausted_delta.min(u16::MAX as usize) as u16);
        self
    }
}

pub fn strictly_dominates(left: &TurnResourceSummary, right: &TurnResourceSummary) -> bool {
    let no_worse = left.final_hp >= right.final_hp && left.final_block >= right.final_block;
    let strictly_better = left.final_hp > right.final_hp || left.final_block > right.final_block;

    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::{strictly_dominates, TurnResourceSummary};

    #[test]
    fn strict_dominance_prefers_higher_final_hp() {
        let baseline = TurnResourceSummary {
            final_hp: 34,
            final_block: 0,
            hp_lost: 0,
            ..TurnResourceSummary::default()
        };
        let worse_hp = TurnResourceSummary {
            final_hp: 31,
            final_block: 0,
            hp_lost: 3,
            ..TurnResourceSummary::default()
        };

        assert!(strictly_dominates(&baseline, &worse_hp));
        assert!(!strictly_dominates(&worse_hp, &baseline));
    }

    #[test]
    fn strict_dominance_does_not_treat_gross_hp_loss_as_hard_dominance() {
        let healed_line = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            hp_lost: 6,
            ..TurnResourceSummary::default()
        };
        let clean_line = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            hp_lost: 0,
            ..TurnResourceSummary::default()
        };

        assert!(!strictly_dominates(&clean_line, &healed_line));
        assert!(!strictly_dominates(&healed_line, &clean_line));
    }

    #[test]
    fn strict_dominance_does_not_treat_exhaust_count_as_hard_dominance() {
        let preserved = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            exhausted_cards: 0,
            ..TurnResourceSummary::default()
        };
        let spent = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            exhausted_cards: 1,
            ..TurnResourceSummary::default()
        };

        assert!(!strictly_dominates(&preserved, &spent));
        assert!(!strictly_dominates(&spent, &preserved));
    }

    #[test]
    fn strict_dominance_does_not_treat_spent_potions_as_hard_dominance() {
        let held = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            spent_potions: 0,
            ..TurnResourceSummary::default()
        };
        let spent = TurnResourceSummary {
            final_hp: 40,
            final_block: 0,
            spent_potions: 1,
            ..TurnResourceSummary::default()
        };

        assert!(!strictly_dominates(&held, &spent));
        assert!(!strictly_dominates(&spent, &held));
    }
}
