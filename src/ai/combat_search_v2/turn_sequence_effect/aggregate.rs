use super::super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
use super::divergence::divergence;
use super::TurnSequenceDivergence;
use super::TurnSequenceEffectFingerprint;
use std::collections::BTreeSet;
#[derive(Clone, Debug, Default)]
pub(in crate::ai::combat_search_v2) struct TurnSequenceEffectAggregate {
    pub(super) terminal_keys: BTreeSet<String>,
    pub(super) legal_action_count_keys: BTreeSet<String>,
    pub(super) public_state_keys: BTreeSet<String>,
    pub(super) hand_public_order_keys: BTreeSet<String>,
    pub(super) hand_identity_order_keys: BTreeSet<String>,
    pub(super) draw_public_order_keys: BTreeSet<String>,
    pub(super) draw_identity_order_keys: BTreeSet<String>,
    pub(super) discard_public_order_keys: BTreeSet<String>,
    pub(super) discard_identity_order_keys: BTreeSet<String>,
    pub(super) exhaust_public_order_keys: BTreeSet<String>,
    pub(super) exhaust_identity_order_keys: BTreeSet<String>,
    pub(super) limbo_public_order_keys: BTreeSet<String>,
    pub(super) limbo_identity_order_keys: BTreeSet<String>,
    pub(super) pending_queue_keys: BTreeSet<String>,
    pub(super) rng_keys: BTreeSet<String>,
    pub(super) dominance_keys: BTreeSet<String>,
    pub(super) dominance_engine_keys: BTreeSet<String>,
    pub(super) dominance_turn_keys: BTreeSet<String>,
    pub(super) turn_draw_modifier_keys: BTreeSet<String>,
    pub(super) turn_action_counter_keys: BTreeSet<String>,
    pub(super) turn_played_card_history_keys: BTreeSet<String>,
    pub(super) turn_discard_counter_keys: BTreeSet<String>,
    pub(super) turn_orb_history_keys: BTreeSet<String>,
    pub(super) turn_combat_flag_keys: BTreeSet<String>,
    pub(super) dominance_meta_keys: BTreeSet<String>,
    pub(super) dominance_zones_keys: BTreeSet<String>,
    pub(super) dominance_monsters_keys: BTreeSet<String>,
    pub(super) dominance_powers_keys: BTreeSet<String>,
    pub(super) dominance_potions_keys: BTreeSet<String>,
    pub(super) dominance_queue_keys: BTreeSet<String>,
    pub(super) dominance_runtime_keys: BTreeSet<String>,
    pub(super) dominance_rng_keys: BTreeSet<String>,
    pub(super) dominance_player_keys: BTreeSet<String>,
    pub(super) resource_public_keys: BTreeSet<String>,
    pub(super) resource_cost_keys: BTreeSet<String>,
}

impl TurnSequenceEffectAggregate {
    pub(in crate::ai::combat_search_v2) fn observe(
        &mut self,
        fingerprint: &TurnSequenceEffectFingerprint,
    ) {
        self.terminal_keys.insert(fingerprint.terminal_key.clone());
        self.legal_action_count_keys
            .insert(fingerprint.legal_action_count_key.clone());
        self.public_state_keys
            .insert(fingerprint.public_state_key.clone());
        self.hand_public_order_keys
            .insert(fingerprint.hand_public_order_key.clone());
        self.hand_identity_order_keys
            .insert(fingerprint.hand_identity_order_key.clone());
        self.draw_public_order_keys
            .insert(fingerprint.draw_public_order_key.clone());
        self.draw_identity_order_keys
            .insert(fingerprint.draw_identity_order_key.clone());
        self.discard_public_order_keys
            .insert(fingerprint.discard_public_order_key.clone());
        self.discard_identity_order_keys
            .insert(fingerprint.discard_identity_order_key.clone());
        self.exhaust_public_order_keys
            .insert(fingerprint.exhaust_public_order_key.clone());
        self.exhaust_identity_order_keys
            .insert(fingerprint.exhaust_identity_order_key.clone());
        self.limbo_public_order_keys
            .insert(fingerprint.limbo_public_order_key.clone());
        self.limbo_identity_order_keys
            .insert(fingerprint.limbo_identity_order_key.clone());
        self.pending_queue_keys
            .insert(fingerprint.pending_queue_key.clone());
        self.rng_keys.insert(fingerprint.rng_key.clone());
        self.dominance_keys
            .insert(fingerprint.dominance_key.clone());
        self.dominance_engine_keys
            .insert(fingerprint.dominance_engine_key.clone());
        self.dominance_turn_keys
            .insert(fingerprint.dominance_turn_key.clone());
        self.turn_draw_modifier_keys
            .insert(fingerprint.turn_draw_modifier_key.clone());
        self.turn_action_counter_keys
            .insert(fingerprint.turn_action_counter_key.clone());
        self.turn_played_card_history_keys
            .insert(fingerprint.turn_played_card_history_key.clone());
        self.turn_discard_counter_keys
            .insert(fingerprint.turn_discard_counter_key.clone());
        self.turn_orb_history_keys
            .insert(fingerprint.turn_orb_history_key.clone());
        self.turn_combat_flag_keys
            .insert(fingerprint.turn_combat_flag_key.clone());
        self.dominance_meta_keys
            .insert(fingerprint.dominance_meta_key.clone());
        self.dominance_zones_keys
            .insert(fingerprint.dominance_zones_key.clone());
        self.dominance_monsters_keys
            .insert(fingerprint.dominance_monsters_key.clone());
        self.dominance_powers_keys
            .insert(fingerprint.dominance_powers_key.clone());
        self.dominance_potions_keys
            .insert(fingerprint.dominance_potions_key.clone());
        self.dominance_queue_keys
            .insert(fingerprint.dominance_queue_key.clone());
        self.dominance_runtime_keys
            .insert(fingerprint.dominance_runtime_key.clone());
        self.dominance_rng_keys
            .insert(fingerprint.dominance_rng_key.clone());
        self.dominance_player_keys
            .insert(fingerprint.dominance_player_key.clone());
        self.resource_public_keys
            .insert(fingerprint.resource_public_key.clone());
        self.resource_cost_keys
            .insert(fingerprint.resource_cost_key.clone());
    }

    pub(in crate::ai::combat_search_v2) fn classify(&self) -> TurnSequenceDivergence {
        if self.terminal_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::TerminalDelta,
                Some("combat.terminal_label"),
                StateAbstractionRevealGate::CombatEnd,
            );
        }
        if self.legal_action_count_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::LegalActionDelta,
                Some("combat.legal_actions.count"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.public_state_keys.len() > 1 || self.resource_public_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::ImmediatePublicDelta,
                Some("combat.public_state"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.hand_public_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::HandOrderDelta,
                Some("combat.zones.hand"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.draw_public_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::DrawPileOrderDelta,
                Some("combat.zones.draw_pile"),
                StateAbstractionRevealGate::NextDraw,
            );
        }
        if self.discard_public_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::DiscardOrderDelta,
                Some("combat.zones.discard_pile"),
                StateAbstractionRevealGate::NextShuffle,
            );
        }
        if self.exhaust_public_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::ExhaustOrderDelta,
                Some("combat.zones.exhaust_pile"),
                StateAbstractionRevealGate::NextCardSelection,
            );
        }
        if self.limbo_public_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::PendingQueueDelta,
                Some("combat.zones.limbo"),
                StateAbstractionRevealGate::CurrentActionResolution,
            );
        }
        if self.pending_queue_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::PendingQueueDelta,
                Some("combat.engine.action_queue_or_queued_cards"),
                StateAbstractionRevealGate::CurrentActionResolution,
            );
        }
        if self.rng_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::RngStateDelta,
                Some("combat.rng"),
                StateAbstractionRevealGate::NextRandomCall,
            );
        }
        if self.dominance_rng_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::RngStateDelta,
                Some("combat.dominance.common.rng"),
                StateAbstractionRevealGate::NextRandomCall,
            );
        }
        if self.dominance_turn_keys.len() > 1 {
            if self.turn_draw_modifier_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnDrawModifierDelta,
                    Some("combat.turn.turn_start_draw_modifier"),
                    StateAbstractionRevealGate::NextDraw,
                );
            }
            if self.turn_action_counter_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnActionCounterDelta,
                    Some("combat.turn.counters.cards_or_attacks_played_this_turn"),
                    StateAbstractionRevealGate::NextLegalActionGeneration,
                );
            }
            if self.turn_played_card_history_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnPlayedCardHistoryDelta,
                    Some("combat.turn.counters.card_ids_played"),
                    StateAbstractionRevealGate::NextLegalActionGeneration,
                );
            }
            if self.turn_discard_counter_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnDiscardCounterDelta,
                    Some("combat.turn.counters.cards_discarded_this_turn"),
                    StateAbstractionRevealGate::NextLegalActionGeneration,
                );
            }
            if self.turn_orb_history_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnOrbHistoryDelta,
                    Some("combat.turn.counters.orb_or_mantra_history"),
                    StateAbstractionRevealGate::NextLegalActionGeneration,
                );
            }
            if self.turn_combat_flag_keys.len() > 1 {
                return divergence(
                    StateDivergenceKind::TurnCombatFlagDelta,
                    Some("combat.turn.counters.combat_flags"),
                    StateAbstractionRevealGate::Unknown,
                );
            }
            return divergence(
                StateDivergenceKind::TurnRuntimeDelta,
                Some("combat.turn"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.dominance_monsters_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::MonsterRuntimeDelta,
                Some("combat.monsters.runtime_or_turn_plan"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.dominance_powers_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::ImmediatePublicDelta,
                Some("combat.powers"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.dominance_runtime_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CombatRuntimeHintDelta,
                Some("combat.runtime_hints"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.dominance_potions_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::PotionStateDelta,
                Some("combat.potions"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.dominance_player_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::PlayerFutureDelta,
                Some("combat.player.future_relevant"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.dominance_queue_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::PendingQueueDelta,
                Some("combat.dominance.common.queue"),
                StateAbstractionRevealGate::CurrentActionResolution,
            );
        }
        if self.dominance_engine_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::EngineRuntimeDelta,
                Some("combat.engine_state"),
                StateAbstractionRevealGate::CurrentActionResolution,
            );
        }
        if self.dominance_meta_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CombatMetaDelta,
                Some("combat.meta"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.hand_identity_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CardUuidDelta,
                Some("combat.zones.hand.uuid_order"),
                StateAbstractionRevealGate::NextLegalActionGeneration,
            );
        }
        if self.draw_identity_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CardUuidDelta,
                Some("combat.zones.draw_pile.uuid_order"),
                StateAbstractionRevealGate::NextDraw,
            );
        }
        if self.discard_identity_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CardUuidDelta,
                Some("combat.zones.discard_pile.uuid_order"),
                StateAbstractionRevealGate::NextShuffle,
            );
        }
        if self.exhaust_identity_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CardUuidDelta,
                Some("combat.zones.exhaust_pile.uuid_order"),
                StateAbstractionRevealGate::NextCardSelection,
            );
        }
        if self.limbo_identity_order_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::CardUuidDelta,
                Some("combat.zones.limbo.uuid_order"),
                StateAbstractionRevealGate::CurrentActionResolution,
            );
        }
        if self.dominance_zones_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::ZoneRuntimeDelta,
                Some("combat.zones.runtime"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.resource_cost_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::Unknown,
                Some("search.resource_vector.costs"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        if self.dominance_keys.len() > 1 {
            return divergence(
                StateDivergenceKind::Unknown,
                Some("combat.dominance_key"),
                StateAbstractionRevealGate::Unknown,
            );
        }
        divergence(
            StateDivergenceKind::Unknown,
            None,
            StateAbstractionRevealGate::Unknown,
        )
    }
}
