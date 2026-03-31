use crate::combat::CombatState;
use crate::content::relics::RelicState;
use smallvec::SmallVec;
use crate::action::{Action, ActionInfo, AddTo};

/// Enchiridion
/// Event Relic
/// At the start of each combat, add a random Power card to your hand. It costs 0 this turn.
/// Java: atPreBattle() → returnTrulyRandomCardInCombat(CardType.POWER) → setCostForTurn(0) → addToBot(MakeTempCardInHandAction)
pub fn at_battle_start(_state: &CombatState, _relic: &mut RelicState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Java uses returnTrulyRandomCardInCombat(CardType.POWER) which picks from the full
    // srcCommon+srcUncommon+srcRare pool filtered to POWER type.
    // We use MakeRandomCardInHand with card_type=Power and cost_for_turn=0.
    actions.push(ActionInfo {
        action: Action::MakeRandomCardInHand {
            card_type: Some(crate::content::cards::CardType::Power),
            cost_for_turn: Some(0),
        },
        insertion_mode: AddTo::Bottom, // Java: addToBot
    });

    actions
}
