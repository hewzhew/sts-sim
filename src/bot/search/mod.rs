mod curiosity;
mod hand_select;
mod legal_moves;
mod mcts;
mod tactical_bonus;

pub use mcts::find_best_move;

pub(super) use curiosity::curiosity_archetype_move_bonus;
pub(super) use legal_moves::get_legal_moves;
pub(super) use tactical_bonus::tactical_move_bonus;

fn intent_hits(intent: &crate::combat::Intent) -> i32 {
    match intent {
        crate::combat::Intent::Attack { hits, .. }
        | crate::combat::Intent::AttackBuff { hits, .. }
        | crate::combat::Intent::AttackDebuff { hits, .. }
        | crate::combat::Intent::AttackDefend { hits, .. } => (*hits as i32).max(1),
        _ => 0,
    }
}
