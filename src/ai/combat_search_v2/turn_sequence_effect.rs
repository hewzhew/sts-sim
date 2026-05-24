use super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
use super::*;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use std::collections::BTreeSet;

#[derive(Clone, Debug)]
pub(super) struct TurnSequenceEffectFingerprint {
    terminal_key: String,
    legal_action_count_key: String,
    public_state_key: String,
    hand_public_order_key: String,
    hand_identity_order_key: String,
    draw_public_order_key: String,
    draw_identity_order_key: String,
    discard_public_order_key: String,
    discard_identity_order_key: String,
    exhaust_public_order_key: String,
    exhaust_identity_order_key: String,
    limbo_public_order_key: String,
    limbo_identity_order_key: String,
    pending_queue_key: String,
    rng_key: String,
    dominance_key: String,
    dominance_engine_key: String,
    dominance_turn_key: String,
    dominance_meta_key: String,
    dominance_zones_key: String,
    dominance_monsters_key: String,
    dominance_powers_key: String,
    dominance_potions_key: String,
    dominance_queue_key: String,
    dominance_runtime_key: String,
    dominance_rng_key: String,
    dominance_player_key: String,
    resource_public_key: String,
    resource_cost_key: String,
}

#[derive(Clone, Debug, Default)]
pub(super) struct TurnSequenceEffectAggregate {
    terminal_keys: BTreeSet<String>,
    legal_action_count_keys: BTreeSet<String>,
    public_state_keys: BTreeSet<String>,
    hand_public_order_keys: BTreeSet<String>,
    hand_identity_order_keys: BTreeSet<String>,
    draw_public_order_keys: BTreeSet<String>,
    draw_identity_order_keys: BTreeSet<String>,
    discard_public_order_keys: BTreeSet<String>,
    discard_identity_order_keys: BTreeSet<String>,
    exhaust_public_order_keys: BTreeSet<String>,
    exhaust_identity_order_keys: BTreeSet<String>,
    limbo_public_order_keys: BTreeSet<String>,
    limbo_identity_order_keys: BTreeSet<String>,
    pending_queue_keys: BTreeSet<String>,
    rng_keys: BTreeSet<String>,
    dominance_keys: BTreeSet<String>,
    dominance_engine_keys: BTreeSet<String>,
    dominance_turn_keys: BTreeSet<String>,
    dominance_meta_keys: BTreeSet<String>,
    dominance_zones_keys: BTreeSet<String>,
    dominance_monsters_keys: BTreeSet<String>,
    dominance_powers_keys: BTreeSet<String>,
    dominance_potions_keys: BTreeSet<String>,
    dominance_queue_keys: BTreeSet<String>,
    dominance_runtime_keys: BTreeSet<String>,
    dominance_rng_keys: BTreeSet<String>,
    dominance_player_keys: BTreeSet<String>,
    resource_public_keys: BTreeSet<String>,
    resource_cost_keys: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct TurnSequenceDivergence {
    pub kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
}

pub(super) fn effect_fingerprint(
    node: &SearchNode,
    legal_actions: usize,
) -> TurnSequenceEffectFingerprint {
    let resource = node.resource_vector().diagnostic_parts();
    let dominance = combat_dominance_key(&node.engine, &node.combat);
    TurnSequenceEffectFingerprint {
        terminal_key: format!("{:?}", terminal_label(&node.engine, &node.combat)),
        legal_action_count_key: legal_actions.to_string(),
        public_state_key: stable_debug_hash(&public_state_projection(&node.engine, &node.combat)),
        hand_public_order_key: card_public_order_key(&node.combat.zones.hand),
        hand_identity_order_key: card_identity_order_key(&node.combat.zones.hand),
        draw_public_order_key: card_public_order_key(&node.combat.zones.draw_pile),
        draw_identity_order_key: card_identity_order_key(&node.combat.zones.draw_pile),
        discard_public_order_key: card_public_order_key(&node.combat.zones.discard_pile),
        discard_identity_order_key: card_identity_order_key(&node.combat.zones.discard_pile),
        exhaust_public_order_key: card_public_order_key(&node.combat.zones.exhaust_pile),
        exhaust_identity_order_key: card_identity_order_key(&node.combat.zones.exhaust_pile),
        limbo_public_order_key: card_public_order_key(&node.combat.zones.limbo),
        limbo_identity_order_key: card_identity_order_key(&node.combat.zones.limbo),
        pending_queue_key: stable_debug_hash(&(
            &node.combat.engine.action_queue,
            &node.combat.zones.queued_cards,
        )),
        rng_key: stable_debug_hash(&node.combat.rng),
        dominance_key: stable_debug_hash(&dominance),
        dominance_engine_key: stable_debug_hash(&dominance.common.engine),
        dominance_turn_key: stable_debug_hash(&dominance.common.turn),
        dominance_meta_key: stable_debug_hash(&dominance.common.meta),
        dominance_zones_key: stable_debug_hash(&dominance.common.zones),
        dominance_monsters_key: stable_debug_hash(&dominance.common.monsters),
        dominance_powers_key: stable_debug_hash(&dominance.common.powers),
        dominance_potions_key: stable_debug_hash(&dominance.common.potions),
        dominance_queue_key: stable_debug_hash(&dominance.common.queue),
        dominance_runtime_key: stable_debug_hash(&dominance.common.runtime),
        dominance_rng_key: stable_debug_hash(&dominance.common.rng),
        dominance_player_key: stable_debug_hash(&dominance.player),
        resource_public_key: stable_debug_hash(&(resource.hp, resource.block)),
        resource_cost_key: stable_debug_hash(&(
            resource.potions_used,
            resource.potions_discarded,
            resource.cards_played,
            resource.action_count,
        )),
    }
}

pub(super) fn effect_key(fingerprint: &TurnSequenceEffectFingerprint) -> String {
    stable_debug_hash(&[
        fingerprint.terminal_key.as_str(),
        fingerprint.legal_action_count_key.as_str(),
        fingerprint.public_state_key.as_str(),
        fingerprint.hand_public_order_key.as_str(),
        fingerprint.hand_identity_order_key.as_str(),
        fingerprint.draw_public_order_key.as_str(),
        fingerprint.draw_identity_order_key.as_str(),
        fingerprint.discard_public_order_key.as_str(),
        fingerprint.discard_identity_order_key.as_str(),
        fingerprint.exhaust_public_order_key.as_str(),
        fingerprint.exhaust_identity_order_key.as_str(),
        fingerprint.limbo_public_order_key.as_str(),
        fingerprint.limbo_identity_order_key.as_str(),
        fingerprint.pending_queue_key.as_str(),
        fingerprint.rng_key.as_str(),
        fingerprint.dominance_key.as_str(),
        fingerprint.dominance_engine_key.as_str(),
        fingerprint.dominance_turn_key.as_str(),
        fingerprint.dominance_meta_key.as_str(),
        fingerprint.dominance_zones_key.as_str(),
        fingerprint.dominance_monsters_key.as_str(),
        fingerprint.dominance_powers_key.as_str(),
        fingerprint.dominance_potions_key.as_str(),
        fingerprint.dominance_queue_key.as_str(),
        fingerprint.dominance_runtime_key.as_str(),
        fingerprint.dominance_rng_key.as_str(),
        fingerprint.dominance_player_key.as_str(),
        fingerprint.resource_public_key.as_str(),
        fingerprint.resource_cost_key.as_str(),
    ])
}

impl TurnSequenceEffectAggregate {
    pub(super) fn observe(&mut self, fingerprint: &TurnSequenceEffectFingerprint) {
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

    pub(super) fn classify(&self) -> TurnSequenceDivergence {
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

fn divergence(
    kind: StateDivergenceKind,
    first_divergence_path: Option<&'static str>,
    guessed_reveal_gate: StateAbstractionRevealGate,
) -> TurnSequenceDivergence {
    TurnSequenceDivergence {
        kind,
        first_divergence_path,
        guessed_reveal_gate,
    }
}

fn public_state_projection(engine: &EngineState, combat: &CombatState) -> impl std::fmt::Debug {
    let monsters = combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            (
                monster.slot,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_dying,
                monster.is_escaped,
                monster.half_dead,
                combat.monster_protocol_visible_intent(monster.id).clone(),
                combat.monster_protocol_preview_damage_per_hit(monster.id),
                power_public_key(combat.entities.power_db.get(&monster.id)),
            )
        })
        .collect::<Vec<_>>();
    let player_power_key =
        power_public_key(combat.entities.power_db.get(&combat.entities.player.id));
    (
        engine_label(engine),
        combat.turn.turn_count,
        combat.turn.current_phase.clone(),
        combat.turn.energy,
        combat.entities.player.current_hp,
        combat.entities.player.max_hp,
        combat.entities.player.block,
        combat.entities.player.stance,
        player_power_key,
        monsters,
    )
}

fn engine_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::CombatStart(_) => "combat_start",
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::PendingChoice(_) => "pending_choice",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::TreasureRoom(_) => "treasure_room",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::EventRoom => "event_room",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

fn card_public_order_key(cards: &[CombatCard]) -> String {
    stable_debug_hash(&cards.iter().map(card_public_signature).collect::<Vec<_>>())
}

fn card_identity_order_key(cards: &[CombatCard]) -> String {
    stable_debug_hash(
        &cards
            .iter()
            .map(|card| (card.uuid, card_public_signature(card)))
            .collect::<Vec<_>>(),
    )
}

fn card_public_signature(card: &CombatCard) -> impl std::fmt::Debug {
    (
        card.id,
        card.upgrades,
        card.misc_value,
        card.base_damage_override,
        card.base_block_override,
        card.cost_modifier,
        card.cost_for_turn,
        card.free_to_play_once,
    )
}

fn power_public_key(powers: Option<&Vec<Power>>) -> String {
    stable_debug_hash(
        &powers
            .into_iter()
            .flatten()
            .map(|power| {
                (
                    power.power_type,
                    power.amount,
                    power.extra_data,
                    matches!(power.payload, PowerPayload::Card(_)),
                    power.just_applied,
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn stable_debug_hash<T: std::fmt::Debug>(value: &T) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in format!("{value:?}").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;

    #[test]
    fn classifier_prefers_public_delta_over_hidden_identity_delta() {
        let mut aggregate = TurnSequenceEffectAggregate::default();
        aggregate.public_state_keys.insert("a".to_string());
        aggregate.public_state_keys.insert("b".to_string());
        aggregate.hand_identity_order_keys.insert("c".to_string());
        aggregate.hand_identity_order_keys.insert("d".to_string());

        let divergence = aggregate.classify();

        assert_eq!(divergence.kind, StateDivergenceKind::ImmediatePublicDelta);
        assert_eq!(
            divergence.guessed_reveal_gate,
            StateAbstractionRevealGate::NextLegalActionGeneration
        );
    }

    #[test]
    fn classifier_marks_identity_only_hand_delta_as_uuid_delta() {
        let mut aggregate = TurnSequenceEffectAggregate::default();
        aggregate.hand_public_order_keys.insert("same".to_string());
        aggregate
            .hand_identity_order_keys
            .insert("uuid-a".to_string());
        aggregate
            .hand_identity_order_keys
            .insert("uuid-b".to_string());

        let divergence = aggregate.classify();

        assert_eq!(divergence.kind, StateDivergenceKind::CardUuidDelta);
        assert_eq!(
            divergence.first_divergence_path,
            Some("combat.zones.hand.uuid_order")
        );
    }

    #[test]
    fn card_public_key_ignores_uuid_but_identity_key_keeps_it() {
        let left = vec![CombatCard::new(CardId::Strike, 1)];
        let right = vec![CombatCard::new(CardId::Strike, 2)];

        assert_eq!(card_public_order_key(&left), card_public_order_key(&right));
        assert_ne!(
            card_identity_order_key(&left),
            card_identity_order_key(&right)
        );
    }
}
