use super::turn_branching::TurnBranchTransition;
use super::*;

mod diagnostics;
pub(super) use diagnostics::TurnPrefixDiagnosticsCollector;

const PREFIX_PREVIEW_LIMIT: usize = 160;
const PREFIX_TOKEN_LIMIT: usize = 12;

#[derive(Clone, Debug, Default)]
pub(super) struct TurnPrefixState {
    prefix_length: u16,
    cards_played: u16,
    potions_used: u16,
    potions_discarded: u16,
    other_actions: u16,
    origin_key: Option<String>,
    tokens: Vec<String>,
    tokens_truncated: bool,
    signature_preview: String,
    signature_truncated: bool,
}

#[derive(Clone, Debug)]
pub(super) struct TurnPrefixSummary {
    prefix: TurnPrefixState,
    legal_actions: usize,
}

#[derive(Clone, Debug)]
struct TurnPrefixObservation {
    observed_at_state_query: u64,
    prefix: TurnPrefixState,
    legal_actions: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TurnPrefixKind {
    Empty,
    CardOnly,
    PotionOnly,
    CardAndPotion,
    WithDiscardPotion,
    WithOther,
}

pub(super) fn summarize_turn_prefix(
    prefix: &TurnPrefixState,
    legal_actions: usize,
) -> TurnPrefixSummary {
    TurnPrefixSummary {
        prefix: prefix.clone(),
        legal_actions,
    }
}

pub(super) fn advance_turn_prefix(
    current: &TurnPrefixState,
    parent_combat: &CombatState,
    input: &ClientInput,
    transition: TurnBranchTransition,
) -> TurnPrefixState {
    if transition.resets_turn_prefix() {
        return TurnPrefixState::default();
    }

    let Some(token) = prefix_token(parent_combat, input) else {
        return current.clone();
    };

    let mut next = current.clone();
    if next.prefix_length == 0 {
        next.origin_key = Some(turn_origin_key(parent_combat));
    }
    next.prefix_length = next.prefix_length.saturating_add(1);
    match input {
        ClientInput::PlayCard { .. } => next.cards_played = next.cards_played.saturating_add(1),
        ClientInput::UsePotion { .. } => next.potions_used = next.potions_used.saturating_add(1),
        ClientInput::DiscardPotion(_) => {
            next.potions_discarded = next.potions_discarded.saturating_add(1)
        }
        _ => next.other_actions = next.other_actions.saturating_add(1),
    }
    next.push_token(&token);
    next
}

fn prefix_token(parent_combat: &CombatState, input: &ClientInput) -> Option<String> {
    match input {
        ClientInput::PlayCard { card_index, .. } => {
            parent_combat.zones.hand.get(*card_index).map(|card| {
                format!(
                    "card:{}#{}",
                    crate::content::cards::java_id(card.id),
                    card.uuid
                )
            })
        }
        ClientInput::UsePotion { potion_index, .. } => Some(format!("potion:{potion_index}")),
        ClientInput::DiscardPotion(slot) => Some(format!("discard_potion:{slot}")),
        ClientInput::EndTurn => Some("end_turn".to_string()),
        _ => Some("other".to_string()),
    }
}

impl TurnPrefixState {
    fn kind(&self) -> TurnPrefixKind {
        if self.prefix_length == 0 {
            TurnPrefixKind::Empty
        } else if self.other_actions > 0 {
            TurnPrefixKind::WithOther
        } else if self.potions_discarded > 0 {
            TurnPrefixKind::WithDiscardPotion
        } else if self.cards_played > 0 && self.potions_used > 0 {
            TurnPrefixKind::CardAndPotion
        } else if self.cards_played > 0 {
            TurnPrefixKind::CardOnly
        } else if self.potions_used > 0 {
            TurnPrefixKind::PotionOnly
        } else {
            TurnPrefixKind::WithOther
        }
    }

    fn push_token(&mut self, token: &str) {
        if self.tokens.len() < PREFIX_TOKEN_LIMIT {
            self.tokens.push(token.to_string());
        } else {
            self.tokens_truncated = true;
        }

        if self.signature_truncated {
            return;
        }
        let separator_len = usize::from(!self.signature_preview.is_empty());
        let required_len = self.signature_preview.len() + separator_len + token.len();
        if required_len > PREFIX_PREVIEW_LIMIT {
            if self.signature_preview.len() + 4 <= PREFIX_PREVIEW_LIMIT {
                self.signature_preview.push_str(">...");
            }
            self.signature_truncated = true;
            return;
        }
        if !self.signature_preview.is_empty() {
            self.signature_preview.push('>');
        }
        self.signature_preview.push_str(token);
    }

    pub(super) fn prefix_length(&self) -> usize {
        self.prefix_length as usize
    }

    pub(super) fn origin_key(&self) -> Option<&str> {
        self.origin_key.as_deref()
    }

    pub(super) fn ordered_sequence_key(&self) -> Option<String> {
        if self.prefix_length == 0 {
            return None;
        }
        Some(token_sequence_key(&self.tokens, self.tokens_truncated))
    }

    pub(super) fn unordered_sequence_key(&self) -> Option<String> {
        if self.prefix_length == 0 {
            return None;
        }
        let mut tokens = self.tokens.clone();
        tokens.sort();
        Some(token_sequence_key(&tokens, self.tokens_truncated))
    }
}

fn token_sequence_key(tokens: &[String], truncated: bool) -> String {
    let mut key = tokens.join(">");
    if truncated {
        if !key.is_empty() {
            key.push('>');
        }
        key.push_str("...");
    }
    key
}

fn turn_origin_key(parent_combat: &CombatState) -> String {
    stable_debug_hash(&combat_dominance_key(
        &EngineState::CombatPlayerTurn,
        parent_combat,
    ))
}

fn stable_debug_hash<T: std::fmt::Debug>(value: &T) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{value:?}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

impl TurnPrefixKind {
    fn label(self) -> &'static str {
        match self {
            TurnPrefixKind::Empty => "empty",
            TurnPrefixKind::CardOnly => "card_only",
            TurnPrefixKind::PotionOnly => "potion_only",
            TurnPrefixKind::CardAndPotion => "card_and_potion",
            TurnPrefixKind::WithDiscardPotion => "with_discard_potion",
            TurnPrefixKind::WithOther => "with_other",
        }
    }
}

#[cfg(test)]
mod tests;
