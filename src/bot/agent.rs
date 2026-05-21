use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;

/// Combat-only diagnostic agent.
///
/// Noncombat and run-level decisions are intentionally not implemented here:
/// those choices need an external controller or explicit action selection, not
/// a hidden hand-written macro policy.
pub struct Agent {
    bot_depth: u32,
}

impl Agent {
    pub fn new() -> Self {
        Self { bot_depth: 6 }
    }

    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    pub(crate) const fn bot_depth(&self) -> u32 {
        self.bot_depth
    }

    pub fn decide(
        &mut self,
        es: &EngineState,
        _rs: &RunState,
        cs: &Option<CombatState>,
        verbose: bool,
    ) -> ClientInput {
        if matches!(
            es,
            EngineState::CombatPlayerTurn
                | EngineState::PendingChoice(_)
                | EngineState::EventCombat(_)
        ) {
            if let Some(combat) = cs.as_ref() {
                let diagnostics = crate::bot::combat::diagnose_root_search_with_depth(
                    es,
                    combat,
                    self.bot_depth(),
                    4000,
                );
                if verbose {
                    eprintln!(
                        "[agent] combat_move={:?} legal_moves={}",
                        diagnostics.chosen_move, diagnostics.legal_moves
                    );
                }
                return diagnostics.chosen_move;
            }
        }

        ClientInput::Proceed
    }
}
