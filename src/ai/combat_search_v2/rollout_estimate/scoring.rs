use super::super::terminal_rank;
use super::types::RolloutNodeEstimate;

impl RolloutNodeEstimate {
    pub(in crate::ai::combat_search_v2) fn priority_terminal_rank(self) -> i32 {
        if self.evaluated {
            terminal_rank(self.terminal)
        } else {
            0
        }
    }

    pub(in crate::ai::combat_search_v2) fn enemy_progress(self) -> i32 {
        -(self.phase_adjusted_enemy_effort)
    }

    pub(in crate::ai::combat_search_v2) fn potion_conservation(self) -> i32 {
        -((self.potions_used + self.potions_discarded) as i32)
    }

    pub(in crate::ai::combat_search_v2) fn faster_turns(self) -> i32 {
        -(self.turns as i32)
    }

    pub(in crate::ai::combat_search_v2) fn fewer_cards_played(self) -> i32 {
        -(self.cards_played as i32)
    }
}
