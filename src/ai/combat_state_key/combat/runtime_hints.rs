use crate::runtime::combat::{CombatState, DrawnCardRecord, QueuedCardHint};

use super::super::types::{
    CombatDrawnCardKey, CombatMonsterProtocolKey, CombatQueuedCardHintKey, CombatRuntimeHintsKey,
};

pub(super) fn runtime_key(combat: &CombatState) -> CombatRuntimeHintsKey {
    let runtime = &combat.runtime;
    let mut monster_protocol = runtime
        .monster_protocol
        .iter()
        .map(|(entity_id, state)| CombatMonsterProtocolKey {
            entity_id: *entity_id,
            payload: format!("{state:?}"),
        })
        .collect::<Vec<_>>();
    monster_protocol.sort_by_key(|entry| entry.entity_id);

    CombatRuntimeHintsKey {
        using_card: runtime.using_card,
        card_queue: runtime
            .card_queue
            .iter()
            .map(queued_card_hint_key)
            .collect(),
        colorless_combat_pool: runtime.colorless_combat_pool.clone(),
        emitted_events: runtime
            .emitted_events
            .iter()
            .map(|event| format!("{event:?}"))
            .collect(),
        engine_diagnostics: runtime
            .engine_diagnostics
            .iter()
            .map(|diagnostic| format!("{diagnostic:?}"))
            .collect(),
        pending_rewards: runtime
            .pending_rewards
            .iter()
            .map(|reward| format!("{reward:?}"))
            .collect(),
        power_instance_counter: runtime.power_instance_counter,
        last_drawn_cards: runtime
            .last_drawn_cards
            .iter()
            .map(drawn_card_key)
            .collect(),
        monster_protocol,
        combat_mugged: runtime.combat_mugged,
        combat_smoked: runtime.combat_smoked,
    }
}

fn queued_card_hint_key(hint: &QueuedCardHint) -> CombatQueuedCardHintKey {
    CombatQueuedCardHintKey {
        card_uuid: hint.card_uuid,
        card_id: hint.card_id,
        target_monster_index: hint.target_monster_index,
        energy_on_use: hint.energy_on_use,
        ignore_energy_total: hint.ignore_energy_total,
        autoplay: hint.autoplay,
        random_target: hint.random_target,
        is_end_turn_autoplay: hint.is_end_turn_autoplay,
        purge_on_use: hint.purge_on_use,
    }
}

fn drawn_card_key(card: &DrawnCardRecord) -> CombatDrawnCardKey {
    CombatDrawnCardKey {
        card_uuid: card.card_uuid,
        card_id: card.card_id,
    }
}
