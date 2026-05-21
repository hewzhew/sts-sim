use super::*;

pub(super) fn filtered_legal_moves(
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<ClientInput>,
    potion_policy: CombatSearchV2PotionPolicy,
) -> Vec<ClientInput> {
    match potion_policy {
        CombatSearchV2PotionPolicy::All => legal,
        CombatSearchV2PotionPolicy::Never => legal
            .into_iter()
            .filter(|input| !is_potion_input(input))
            .collect(),
        CombatSearchV2PotionPolicy::LethalOnly => {
            let allow_potions = matches!(engine, EngineState::CombatPlayerTurn)
                && visible_incoming_damage(combat)
                    >= combat.entities.player.current_hp + combat.entities.player.block;
            legal
                .into_iter()
                .filter(|input| allow_potions || !is_potion_input(input))
                .collect()
        }
    }
}

fn is_potion_input(input: &ClientInput) -> bool {
    matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    )
}

pub(super) fn terminal_label(engine: &EngineState, combat: &CombatState) -> SearchTerminalLabel {
    match combat_terminal(engine, combat) {
        CombatTerminal::Win => SearchTerminalLabel::Win,
        CombatTerminal::Loss => SearchTerminalLabel::Loss,
        CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
    }
}
