//! Parser for CommunicationMod text commands into replay actions.
//!
//! Converts commands like "play 3 0", "end", "potion use 0 1"
//! into structured ReplayAction variants.

/// A replay action parsed from a CommunicationMod command string.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayAction {
    /// Play a card from hand. `hand_index` is 0-based.
    PlayCard {
        hand_index: usize,
        target: Option<usize>,
    },
    /// End the current turn.
    EndTurn,
    /// Use a potion.
    UsePotion {
        slot: usize,
        target: Option<usize>,
    },
    /// Discard a potion.
    DiscardPotion {
        slot: usize,
    },
    /// Choose an option (card reward, event choice, etc.)
    Choose(usize),
    /// Start command (not a combat action)
    Start {
        class: String,
        seed: Option<String>,
    },
    /// Non-combat command (proceed, confirm, etc.)
    Other(String),
}

impl ReplayAction {
    /// Returns true if this action affects combat state (play, end, potion use).
    pub fn is_combat_action(&self) -> bool {
        matches!(self, 
            ReplayAction::PlayCard { .. } |
            ReplayAction::EndTurn |
            ReplayAction::UsePotion { .. }
        )
    }
}

/// Parse a CommunicationMod command string into a ReplayAction.
pub fn parse_command(cmd: &str) -> ReplayAction {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    
    match parts.first().copied() {
        Some("play") => {
            let hand_index = parts.get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            let target = parts.get(2)
                .and_then(|s| s.parse::<usize>().ok());
            ReplayAction::PlayCard { hand_index, target }
        }
        Some("end") => ReplayAction::EndTurn,
        Some("potion") => {
            match parts.get(1).copied() {
                Some("use") => {
                    let slot = parts.get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    let target = parts.get(3)
                        .and_then(|s| s.parse::<usize>().ok());
                    ReplayAction::UsePotion { slot, target }
                }
                Some("discard") => {
                    let slot = parts.get(2)
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(0);
                    ReplayAction::DiscardPotion { slot }
                }
                _ => ReplayAction::Other(cmd.to_string()),
            }
        }
        Some("choose") => {
            let index = parts.get(1)
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);
            ReplayAction::Choose(index)
        }
        Some("start") => {
            let class = parts.get(1).unwrap_or(&"ironclad").to_string();
            let seed = parts.get(3).map(|s| s.to_string());
            ReplayAction::Start { class, seed }
        }
        _ => ReplayAction::Other(cmd.to_string()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_play_card() {
        assert_eq!(
            parse_command("play 3 0"),
            ReplayAction::PlayCard { hand_index: 3, target: Some(0) }
        );
        assert_eq!(
            parse_command("play 2"),
            ReplayAction::PlayCard { hand_index: 2, target: None }
        );
    }

    #[test]
    fn test_parse_end() {
        assert_eq!(parse_command("end"), ReplayAction::EndTurn);
    }

    #[test]
    fn test_parse_potion() {
        assert_eq!(
            parse_command("potion use 0 1"),
            ReplayAction::UsePotion { slot: 0, target: Some(1) }
        );
        assert_eq!(
            parse_command("potion discard 2"),
            ReplayAction::DiscardPotion { slot: 2 }
        );
    }

    #[test]
    fn test_parse_choose() {
        assert_eq!(parse_command("choose 2"), ReplayAction::Choose(2));
    }

    #[test]
    fn test_parse_start() {
        let action = parse_command("start ironclad 0 ABC123");
        match action {
            ReplayAction::Start { class, seed } => {
                assert_eq!(class, "ironclad");
                assert_eq!(seed, Some("ABC123".to_string()));
            }
            _ => panic!("Expected Start"),
        }
    }

    #[test]
    fn test_is_combat_action() {
        assert!(parse_command("play 3 0").is_combat_action());
        assert!(parse_command("end").is_combat_action());
        assert!(parse_command("potion use 0").is_combat_action());
        assert!(!parse_command("choose 2").is_combat_action());
        assert!(!parse_command("start ironclad").is_combat_action());
    }
}
