use crate::runtime::combat::{
    CombatState, DrawnCardRecord, Intent, MonsterProtocolState, QueuedCardHint,
};

use super::super::types::{
    CombatDrawnCardKey, CombatIntentKey, CombatMonsterProtocolIdentityKey,
    CombatMonsterProtocolKey, CombatMonsterProtocolObservationKey, CombatQueuedCardHintKey,
    CombatRuntimeHintsKey,
};

pub(super) fn runtime_key(combat: &CombatState) -> CombatRuntimeHintsKey {
    let runtime = &combat.runtime;
    let mut monster_protocol = runtime
        .monster_protocol
        .iter()
        .map(|(entity_id, state)| monster_protocol_key(*entity_id, state))
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

fn monster_protocol_key(
    entity_id: usize,
    state: &MonsterProtocolState,
) -> CombatMonsterProtocolKey {
    CombatMonsterProtocolKey {
        entity_id,
        observation: CombatMonsterProtocolObservationKey {
            visible_intent: intent_key(&state.observation.visible_intent),
            preview_damage_per_hit: state.observation.preview_damage_per_hit,
        },
        identity: CombatMonsterProtocolIdentityKey {
            instance_id: state.identity.instance_id,
            spawn_order: state.identity.spawn_order,
            draw_x: state.identity.draw_x,
            group_index: state.identity.group_index,
        },
    }
}

fn intent_key(intent: &Intent) -> CombatIntentKey {
    match intent {
        Intent::Attack { damage, hits } => CombatIntentKey::Attack {
            damage: *damage,
            hits: *hits,
        },
        Intent::AttackBuff { damage, hits } => CombatIntentKey::AttackBuff {
            damage: *damage,
            hits: *hits,
        },
        Intent::AttackDebuff { damage, hits } => CombatIntentKey::AttackDebuff {
            damage: *damage,
            hits: *hits,
        },
        Intent::AttackDefend { damage, hits } => CombatIntentKey::AttackDefend {
            damage: *damage,
            hits: *hits,
        },
        Intent::Buff => CombatIntentKey::Buff,
        Intent::Debuff => CombatIntentKey::Debuff,
        Intent::StrongDebuff => CombatIntentKey::StrongDebuff,
        Intent::Debug => CombatIntentKey::Debug,
        Intent::Defend => CombatIntentKey::Defend,
        Intent::DefendDebuff => CombatIntentKey::DefendDebuff,
        Intent::DefendBuff => CombatIntentKey::DefendBuff,
        Intent::Escape => CombatIntentKey::Escape,
        Intent::Magic => CombatIntentKey::Magic,
        Intent::None => CombatIntentKey::None,
        Intent::Sleep => CombatIntentKey::Sleep,
        Intent::Stun => CombatIntentKey::Stun,
        Intent::Unknown => CombatIntentKey::Unknown,
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
