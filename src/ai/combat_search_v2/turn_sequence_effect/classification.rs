use super::super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
use super::aggregate::TurnSequenceEffectAggregate;
use super::divergence::divergence;
use super::TurnSequenceDivergence;

impl TurnSequenceEffectAggregate {
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
