use crate::action::{Action, ActionInfo, AddTo};
use crate::content::relics::RelicState;
use smallvec::SmallVec;

fn first_turn_flag(relic_state: &RelicState) -> bool {
    relic_state.amount != 0
}

fn set_first_turn_flag(relic_state: &mut RelicState, first_turn: bool) {
    relic_state.amount = i32::from(first_turn);
}

/// Java Pocketwatch.atBattleStart():
///   counter = 0
///   firstTurn = true
pub fn at_battle_start(relic_state: &mut RelicState) {
    relic_state.counter = 0;
    set_first_turn_flag(relic_state, true);
}

/// Java Pocketwatch.onPlayCard():
///   ++counter
pub fn on_use_card(relic_state: &mut RelicState) {
    relic_state.counter += 1;
}

/// Java Pocketwatch.atTurnStartPostDraw():
///   if (counter <= 3 && !firstTurn) draw 3 else firstTurn = false
///   counter = 0
pub fn at_turn_start_post_draw(relic_state: &mut RelicState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    if relic_state.counter <= 3 && !first_turn_flag(relic_state) {
        actions.push(ActionInfo {
            action: Action::DrawCards(3),
            insertion_mode: AddTo::Bottom,
        });
    } else {
        set_first_turn_flag(relic_state, false);
    }

    relic_state.counter = 0;
    actions
}

pub fn on_victory(relic_state: &mut RelicState) {
    relic_state.counter = -1;
    set_first_turn_flag(relic_state, false);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::{RelicId, RelicState};

    #[test]
    fn battle_start_primes_first_turn_and_zero_counter() {
        let mut relic = RelicState::new(RelicId::Pocketwatch);
        relic.counter = 7;
        relic.amount = 0;

        at_battle_start(&mut relic);

        assert_eq!(relic.counter, 0);
        assert_eq!(relic.amount, 1);
    }

    #[test]
    fn first_turn_post_draw_does_not_draw_and_clears_first_turn_flag() {
        let mut relic = RelicState::new(RelicId::Pocketwatch);
        at_battle_start(&mut relic);
        relic.counter = 2;

        let actions = at_turn_start_post_draw(&mut relic);

        assert!(actions.is_empty());
        assert_eq!(relic.counter, 0);
        assert_eq!(relic.amount, 0);
    }

    #[test]
    fn later_turn_with_three_or_fewer_cards_draws_three() {
        let mut relic = RelicState::new(RelicId::Pocketwatch);
        relic.counter = 3;
        relic.amount = 0;

        let actions = at_turn_start_post_draw(&mut relic);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, Action::DrawCards(3));
        assert_eq!(relic.counter, 0);
    }
}
