use super::*;

pub(super) fn filtered_legal_actions(
    legal: Vec<CombatActionChoice>,
    potion_policy: CombatSearchV2PotionPolicy,
) -> Vec<CombatActionChoice> {
    match potion_policy {
        CombatSearchV2PotionPolicy::All => legal,
        CombatSearchV2PotionPolicy::Never => legal
            .into_iter()
            .filter(|choice| !is_potion_input(&choice.input))
            .collect(),
    }
}

pub(super) fn is_potion_input(input: &ClientInput) -> bool {
    matches!(
        input,
        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
    )
}

pub(super) fn is_use_potion_input(input: &ClientInput) -> bool {
    matches!(input, ClientInput::UsePotion { .. })
}

pub(super) fn terminal_label(engine: &EngineState, combat: &CombatState) -> SearchTerminalLabel {
    match combat_terminal(engine, combat) {
        CombatTerminal::Win => SearchTerminalLabel::Win,
        CombatTerminal::Loss => SearchTerminalLabel::Loss,
        CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
    }
}
