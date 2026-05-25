use super::turn_branching::TurnBranchTransition;
use super::*;
use std::collections::BTreeMap;

const PREFIX_PREVIEW_LIMIT: usize = 160;
const PREFIX_TOKEN_LIMIT: usize = 12;
const LARGEST_PREFIX_FANOUT_SAMPLE_LIMIT: usize = 8;

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

#[derive(Default)]
pub(super) struct TurnPrefixDiagnosticsCollector {
    states_observed: u64,
    non_empty_prefix_states: u64,
    empty_prefix_states: u64,
    total_prefix_length: u64,
    max_prefix_length: usize,
    total_cards_played_in_prefix: u64,
    total_potions_used_in_prefix: u64,
    total_potions_discarded_in_prefix: u64,
    total_other_actions_in_prefix: u64,
    max_legal_actions_after_non_empty_prefix: usize,
    prefix_length_counts: BTreeMap<usize, u64>,
    prefix_kind_counts: BTreeMap<TurnPrefixKind, MutableTurnPrefixKindCount>,
    largest_prefix_fanouts: Vec<TurnPrefixObservation>,
}

#[derive(Clone, Debug)]
struct TurnPrefixObservation {
    observed_at_state_query: u64,
    prefix: TurnPrefixState,
    legal_actions: usize,
}

#[derive(Clone, Debug, Default)]
struct MutableTurnPrefixKindCount {
    states: u64,
    legal_actions_total: u64,
    max_prefix_length: usize,
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

impl TurnPrefixDiagnosticsCollector {
    pub(super) fn observe(&mut self, summary: &TurnPrefixSummary) {
        self.states_observed = self.states_observed.saturating_add(1);
        let prefix_length = summary.prefix.prefix_length as usize;
        self.total_prefix_length = self
            .total_prefix_length
            .saturating_add(prefix_length as u64);
        self.max_prefix_length = self.max_prefix_length.max(prefix_length);
        if prefix_length > 0 {
            self.non_empty_prefix_states = self.non_empty_prefix_states.saturating_add(1);
            self.max_legal_actions_after_non_empty_prefix = self
                .max_legal_actions_after_non_empty_prefix
                .max(summary.legal_actions);
        } else {
            self.empty_prefix_states = self.empty_prefix_states.saturating_add(1);
        }

        self.total_cards_played_in_prefix = self
            .total_cards_played_in_prefix
            .saturating_add(summary.prefix.cards_played as u64);
        self.total_potions_used_in_prefix = self
            .total_potions_used_in_prefix
            .saturating_add(summary.prefix.potions_used as u64);
        self.total_potions_discarded_in_prefix = self
            .total_potions_discarded_in_prefix
            .saturating_add(summary.prefix.potions_discarded as u64);
        self.total_other_actions_in_prefix = self
            .total_other_actions_in_prefix
            .saturating_add(summary.prefix.other_actions as u64);

        *self.prefix_length_counts.entry(prefix_length).or_insert(0) += 1;
        let kind = summary.prefix.kind();
        let kind_count = self.prefix_kind_counts.entry(kind).or_default();
        kind_count.states = kind_count.states.saturating_add(1);
        kind_count.legal_actions_total = kind_count
            .legal_actions_total
            .saturating_add(summary.legal_actions as u64);
        kind_count.max_prefix_length = kind_count.max_prefix_length.max(prefix_length);

        self.remember_largest_prefix_fanout(summary);
    }

    pub(super) fn finish(&self) -> CombatSearchV2DiagnosticsTurnPrefix {
        CombatSearchV2DiagnosticsTurnPrefix {
            tracking_policy: "current_turn_prefix_summary_from_search_node",
            behavioral_effect: "diagnostic_only_no_turn_prefix_prune_no_merge",
            states_observed: self.states_observed,
            non_empty_prefix_states: self.non_empty_prefix_states,
            empty_prefix_states: self.empty_prefix_states,
            avg_prefix_length: rounded_ratio(self.total_prefix_length, self.states_observed),
            max_prefix_length: self.max_prefix_length,
            max_legal_actions_after_non_empty_prefix: self.max_legal_actions_after_non_empty_prefix,
            total_cards_played_in_prefix: self.total_cards_played_in_prefix,
            total_potions_used_in_prefix: self.total_potions_used_in_prefix,
            total_potions_discarded_in_prefix: self.total_potions_discarded_in_prefix,
            total_other_actions_in_prefix: self.total_other_actions_in_prefix,
            prefix_length_counts: self.prefix_length_count_reports(),
            prefix_kind_counts: self.prefix_kind_count_reports(),
            largest_prefix_fanouts: self.largest_prefix_fanout_reports(),
            notes: vec![
                "turn prefix state resets after a next-turn transition",
                "prefix signature previews are capped and diagnostic only",
                "turn prefix diagnostics do not merge different action orders",
                "future turn-local dominance must prove exact state and resource coverage",
            ],
        }
    }

    fn remember_largest_prefix_fanout(&mut self, summary: &TurnPrefixSummary) {
        if summary.prefix.prefix_length == 0 || summary.legal_actions <= 1 {
            return;
        }
        self.largest_prefix_fanouts.push(TurnPrefixObservation {
            observed_at_state_query: self.states_observed,
            prefix: summary.prefix.clone(),
            legal_actions: summary.legal_actions,
        });
        self.largest_prefix_fanouts.sort_by(|left, right| {
            right
                .legal_actions
                .cmp(&left.legal_actions)
                .then_with(|| right.prefix.prefix_length.cmp(&left.prefix.prefix_length))
                .then_with(|| {
                    left.observed_at_state_query
                        .cmp(&right.observed_at_state_query)
                })
        });
        self.largest_prefix_fanouts
            .truncate(LARGEST_PREFIX_FANOUT_SAMPLE_LIMIT);
    }

    fn prefix_length_count_reports(&self) -> Vec<CombatSearchV2DiagnosticsTurnPrefixLengthCount> {
        self.prefix_length_counts
            .iter()
            .map(
                |(prefix_length, states)| CombatSearchV2DiagnosticsTurnPrefixLengthCount {
                    prefix_length: *prefix_length,
                    states: *states,
                },
            )
            .collect()
    }

    fn prefix_kind_count_reports(&self) -> Vec<CombatSearchV2DiagnosticsTurnPrefixKindCount> {
        self.prefix_kind_counts
            .iter()
            .map(
                |(kind, count)| CombatSearchV2DiagnosticsTurnPrefixKindCount {
                    kind: kind.label().to_string(),
                    states: count.states,
                    legal_actions_total: count.legal_actions_total,
                    max_prefix_length: count.max_prefix_length,
                },
            )
            .collect()
    }

    fn largest_prefix_fanout_reports(
        &self,
    ) -> Vec<CombatSearchV2DiagnosticsTurnPrefixFanoutSample> {
        self.largest_prefix_fanouts
            .iter()
            .map(|sample| CombatSearchV2DiagnosticsTurnPrefixFanoutSample {
                observed_at_state_query: sample.observed_at_state_query,
                prefix_length: sample.prefix.prefix_length as usize,
                kind: sample.prefix.kind().label().to_string(),
                cards_played: sample.prefix.cards_played as usize,
                potions_used: sample.prefix.potions_used as usize,
                potions_discarded: sample.prefix.potions_discarded as usize,
                other_actions: sample.prefix.other_actions as usize,
                legal_actions: sample.legal_actions,
                signature_preview: sample.prefix.signature_preview.clone(),
                signature_truncated: sample.prefix.signature_truncated,
            })
            .collect()
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

fn rounded_ratio(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let value = numerator as f64 / denominator as f64;
    (value * 100.0).round() / 100.0
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
