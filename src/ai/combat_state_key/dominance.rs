use crate::content::cards::CardType;
use crate::runtime::action::Action;
use crate::runtime::action::CardDestination;
use crate::runtime::combat::{
    CombatCard, CombatPhase, CombatState, DrawnCardRecord, EphemeralCounters, MetaChange,
    MonsterEntity, OrbEntity, Power, PowerPayload, QueuedCardHint,
};
use crate::runtime::rng::{RngPool, StsRng};
use crate::state::core::{
    EngineState, GridSelectReason, HandSelectReason, PendingChoice, PileType,
};

use super::types::{
    CombatCardDestinationKey, CombatCardKey, CombatCardTypeKey, CombatChooseOneCardKey,
    CombatDominanceKey, CombatDominancePlayerKey, CombatDrawnCardKey, CombatEngineKey,
    CombatEntityPowersKey, CombatExactPlayerKey, CombatExactStateKey, CombatGridSelectReasonKey,
    CombatHandSelectReasonKey, CombatMetaChangeKey, CombatMetaKey, CombatMonsterKey,
    CombatMonsterProtocolKey, CombatOrbKey, CombatPendingChoiceKey, CombatPhaseKey,
    CombatPileTypeKey, CombatPlayerFutureKey, CombatPotionKey, CombatPotionSlotKey, CombatPowerKey,
    CombatPowerPayloadKey, CombatQueuedActionKey, CombatQueuedCardHintKey, CombatQueuedCardKey,
    CombatRelicKey, CombatRngPoolKey, CombatRuntimeHintsKey, CombatRuntimeKey, CombatStsRngKey,
    CombatTargetKey, CombatTurnCountersKey, CombatTurnKey, CombatZonesKey,
};

/// Exact in-combat runtime key used by Combat Search V2 transposition pruning.
/// This is stricter than `stable_outcome_key`: player hp/block, card
/// instances, queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_exact_runtime_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatExactStateKey {
    CombatExactStateKey {
        common: combat_runtime_key(engine, combat),
        player: player_exact_key(combat),
    }
}

/// In-combat bucket used by Combat Search V2 resource dominance pruning. This
/// is not an exact transposition key: current HP/block are intentionally left
/// out because they are compared through `ResourceVector`, but card instances,
/// queue, monster runtime, powers, potions, and RNG remain in.
pub(crate) fn combat_dominance_bucket_key(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatDominanceKey {
    CombatDominanceKey {
        common: combat_runtime_key(engine, combat),
        player: CombatDominancePlayerKey {
            future_relevant: player_future_key(combat),
        },
    }
}

fn combat_runtime_key(engine: &EngineState, combat: &CombatState) -> CombatRuntimeKey {
    CombatRuntimeKey {
        engine: engine_key(engine),
        turn: turn_key(combat),
        meta: meta_key(combat),
        zones: zones_key(combat),
        monsters: monsters_key(combat),
        powers: powers_key(combat),
        potions: potions_key(combat),
        queue: queue_key(combat),
        runtime: runtime_key(combat),
        rng: rng_pool_key(&combat.rng.pool),
    }
}

fn player_exact_key(combat: &CombatState) -> CombatExactPlayerKey {
    let player = &combat.entities.player;
    CombatExactPlayerKey {
        current_hp: player.current_hp,
        block: player.block,
        future_relevant: player_future_key(combat),
    }
}

fn engine_key(engine: &EngineState) -> CombatEngineKey {
    match engine {
        EngineState::CombatPlayerTurn => CombatEngineKey::CombatPlayerTurn,
        EngineState::CombatProcessing => CombatEngineKey::CombatProcessing,
        EngineState::PendingChoice(choice) => {
            CombatEngineKey::PendingChoice(pending_choice_key(choice))
        }
        EngineState::RewardScreen(value) => CombatEngineKey::RewardScreen(format!("{value:?}")),
        EngineState::TreasureRoom(value) => CombatEngineKey::TreasureRoom(format!("{value:?}")),
        EngineState::Campfire => CombatEngineKey::Campfire,
        EngineState::Shop(value) => CombatEngineKey::Shop(format!("{value:?}")),
        EngineState::MapNavigation => CombatEngineKey::MapNavigation,
        EngineState::EventRoom => CombatEngineKey::EventRoom,
        EngineState::RunPendingChoice(value) => {
            CombatEngineKey::RunPendingChoice(format!("{value:?}"))
        }
        EngineState::EventCombat(value) => CombatEngineKey::EventCombat(format!("{value:?}")),
        EngineState::BossRelicSelect(value) => {
            CombatEngineKey::BossRelicSelect(format!("{value:?}"))
        }
        EngineState::GameOver(value) => CombatEngineKey::GameOver(format!("{value:?}")),
    }
}

fn pending_choice_key(choice: &PendingChoice) -> CombatPendingChoiceKey {
    match choice {
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => CombatPendingChoiceKey::GridSelect {
            source_pile: pile_type_key(*source_pile),
            candidate_uuids: candidate_uuids.clone(),
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            reason: grid_select_reason_key(*reason),
        },
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => CombatPendingChoiceKey::HandSelect {
            candidate_uuids: candidate_uuids.clone(),
            min_cards: *min_cards,
            max_cards: *max_cards,
            can_cancel: *can_cancel,
            reason: hand_select_reason_key(*reason),
        },
        PendingChoice::DiscoverySelect(state) => CombatPendingChoiceKey::DiscoverySelect {
            cards: state.cards.clone(),
            colorless: state.colorless,
            card_type: state.card_type.map(card_type_key),
            amount: state.amount,
            can_skip: state.can_skip,
        },
        PendingChoice::ScrySelect { cards, card_uuids } => CombatPendingChoiceKey::ScrySelect {
            cards: cards.clone(),
            card_uuids: card_uuids.clone(),
        },
        PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => CombatPendingChoiceKey::CardRewardSelect {
            cards: cards.clone(),
            destination: card_destination_key(*destination),
            can_skip: *can_skip,
        },
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => {
            CombatPendingChoiceKey::ForeignInfluenceSelect {
                cards: cards.clone(),
                upgraded: *upgraded,
            }
        }
        PendingChoice::ChooseOneSelect { choices } => CombatPendingChoiceKey::ChooseOneSelect {
            choices: choices
                .iter()
                .map(|choice| CombatChooseOneCardKey {
                    card_id: choice.card_id,
                    upgrades: choice.upgrades,
                })
                .collect(),
        },
        PendingChoice::StanceChoice => CombatPendingChoiceKey::StanceChoice,
    }
}

fn pile_type_key(value: PileType) -> CombatPileTypeKey {
    match value {
        PileType::Draw => CombatPileTypeKey::Draw,
        PileType::Discard => CombatPileTypeKey::Discard,
        PileType::Exhaust => CombatPileTypeKey::Exhaust,
        PileType::Hand => CombatPileTypeKey::Hand,
        PileType::Limbo => CombatPileTypeKey::Limbo,
        PileType::MasterDeck => CombatPileTypeKey::MasterDeck,
    }
}

fn hand_select_reason_key(value: HandSelectReason) -> CombatHandSelectReasonKey {
    match value {
        HandSelectReason::Exhaust => CombatHandSelectReasonKey::Exhaust,
        HandSelectReason::Discard => CombatHandSelectReasonKey::Discard,
        HandSelectReason::Retain => CombatHandSelectReasonKey::Retain,
        HandSelectReason::PutOnDrawPile => CombatHandSelectReasonKey::PutOnDrawPile,
        HandSelectReason::PutToBottomOfDraw => CombatHandSelectReasonKey::PutToBottomOfDraw,
        HandSelectReason::Setup => CombatHandSelectReasonKey::Setup,
        HandSelectReason::Copy { amount } => CombatHandSelectReasonKey::Copy { amount },
        HandSelectReason::Nightmare { amount } => CombatHandSelectReasonKey::Nightmare { amount },
        HandSelectReason::Upgrade => CombatHandSelectReasonKey::Upgrade,
        HandSelectReason::GamblingChip => CombatHandSelectReasonKey::GamblingChip,
        HandSelectReason::Recycle => CombatHandSelectReasonKey::Recycle,
    }
}

fn grid_select_reason_key(value: GridSelectReason) -> CombatGridSelectReasonKey {
    match value {
        GridSelectReason::MoveToDrawPile => CombatGridSelectReasonKey::MoveToDrawPile,
        GridSelectReason::Exhume { upgrade } => CombatGridSelectReasonKey::Exhume { upgrade },
        GridSelectReason::DrawPileToHand => CombatGridSelectReasonKey::DrawPileToHand,
        GridSelectReason::SkillFromDeckToHand => CombatGridSelectReasonKey::SkillFromDeckToHand,
        GridSelectReason::AttackFromDeckToHand => CombatGridSelectReasonKey::AttackFromDeckToHand,
        GridSelectReason::DiscardToHand => CombatGridSelectReasonKey::DiscardToHand,
        GridSelectReason::DiscardToHandNoCostChange => {
            CombatGridSelectReasonKey::DiscardToHandNoCostChange
        }
        GridSelectReason::DiscardToHandRetain => CombatGridSelectReasonKey::DiscardToHandRetain,
        GridSelectReason::Omniscience { play_amount } => {
            CombatGridSelectReasonKey::Omniscience { play_amount }
        }
    }
}

fn card_type_key(value: CardType) -> CombatCardTypeKey {
    match value {
        CardType::Attack => CombatCardTypeKey::Attack,
        CardType::Skill => CombatCardTypeKey::Skill,
        CardType::Power => CombatCardTypeKey::Power,
        CardType::Status => CombatCardTypeKey::Status,
        CardType::Curse => CombatCardTypeKey::Curse,
    }
}

fn card_destination_key(value: CardDestination) -> CombatCardDestinationKey {
    match value {
        CardDestination::Hand => CombatCardDestinationKey::Hand,
        CardDestination::DrawPileRandom => CombatCardDestinationKey::DrawPileRandom,
    }
}

fn player_future_key(combat: &CombatState) -> CombatPlayerFutureKey {
    let player = &combat.entities.player;
    CombatPlayerFutureKey {
        entity_id: player.id,
        max_hp: player.max_hp,
        facing_left: player.facing_left,
        gold_delta_this_combat: player.gold_delta_this_combat,
        gold: player.gold,
        max_orbs: player.max_orbs,
        orbs: player.orbs.iter().map(orb_key).collect(),
        stance: player.stance,
        relics: player
            .relics
            .iter()
            .map(|relic| CombatRelicKey {
                id: relic.id,
                counter: relic.counter,
                used_up: relic.used_up,
                amount: relic.amount,
            })
            .collect(),
        relic_buses: format!("{:?}", player.relic_buses),
        energy_master: player.energy_master,
    }
}

fn orb_key(orb: &OrbEntity) -> CombatOrbKey {
    CombatOrbKey {
        id: orb.id,
        base_passive_amount: orb.base_passive_amount,
        base_evoke_amount: orb.base_evoke_amount,
        passive_amount: orb.passive_amount,
        evoke_amount: orb.evoke_amount,
    }
}

fn turn_key(combat: &CombatState) -> CombatTurnKey {
    let turn = &combat.turn;
    CombatTurnKey {
        turn_count: turn.turn_count,
        phase: phase_key(turn.current_phase),
        energy: turn.energy,
        turn_start_draw_modifier: turn.turn_start_draw_modifier,
        counters: turn_counters_key(&turn.counters),
    }
}

fn phase_key(phase: CombatPhase) -> CombatPhaseKey {
    match phase {
        CombatPhase::PlayerTurn => CombatPhaseKey::PlayerTurn,
        CombatPhase::MonsterTurn => CombatPhaseKey::MonsterTurn,
        CombatPhase::TurnTransition => CombatPhaseKey::TurnTransition,
    }
}

fn turn_counters_key(counters: &EphemeralCounters) -> CombatTurnCountersKey {
    CombatTurnCountersKey {
        cards_played_this_turn: counters.cards_played_this_turn,
        attacks_played_this_turn: counters.attacks_played_this_turn,
        cards_discarded_this_turn: counters.cards_discarded_this_turn,
        card_ids_played_this_turn: counters.card_ids_played_this_turn.clone(),
        card_ids_played_this_combat: counters.card_ids_played_this_combat.clone(),
        orbs_channeled_this_turn: counters.orbs_channeled_this_turn.clone(),
        orbs_channeled_this_combat: counters.orbs_channeled_this_combat.clone(),
        mantra_gained_this_combat: counters.mantra_gained_this_combat,
        times_damaged_this_combat: counters.times_damaged_this_combat,
        victory_triggered: counters.victory_triggered,
        discovery_cost_for_turn: counters.discovery_cost_for_turn,
        early_end_turn_pending: counters.early_end_turn_pending,
        skip_monster_turn_pending: counters.skip_monster_turn_pending,
        player_escaping: counters.player_escaping,
        escape_pending_reward: counters.escape_pending_reward,
    }
}

fn meta_key(combat: &CombatState) -> CombatMetaKey {
    let meta = &combat.meta;
    CombatMetaKey {
        ascension_level: meta.ascension_level,
        player_class: meta.player_class,
        is_boss_fight: meta.is_boss_fight,
        is_elite_fight: meta.is_elite_fight,
        master_deck_snapshot: meta.master_deck_snapshot.iter().map(card_key).collect(),
        meta_changes: meta.meta_changes.iter().map(meta_change_key).collect(),
    }
}

fn meta_change_key(change: &MetaChange) -> CombatMetaChangeKey {
    match change {
        MetaChange::AddCardToMasterDeck(card_id) => {
            CombatMetaChangeKey::AddCardToMasterDeck(*card_id)
        }
        MetaChange::ModifyCardMisc { card_uuid, amount } => CombatMetaChangeKey::ModifyCardMisc {
            card_uuid: *card_uuid,
            amount: *amount,
        },
        MetaChange::UpgradeMasterDeckCard { card_uuid } => {
            CombatMetaChangeKey::UpgradeMasterDeckCard {
                card_uuid: *card_uuid,
            }
        }
    }
}

fn zones_key(combat: &CombatState) -> CombatZonesKey {
    CombatZonesKey {
        card_uuid_counter: combat.zones.card_uuid_counter,
        hand: zone_key(&combat.zones.hand),
        draw: zone_key(&combat.zones.draw_pile),
        discard: zone_key(&combat.zones.discard_pile),
        exhaust: zone_key(&combat.zones.exhaust_pile),
        limbo: zone_key(&combat.zones.limbo),
        queued: combat
            .zones
            .queued_cards
            .iter()
            .map(|queued| CombatQueuedCardKey {
                card: card_key(&queued.card),
                target: target_key(combat, queued.target),
                energy_on_use: queued.energy_on_use,
                ignore_energy_total: queued.ignore_energy_total,
                autoplay: queued.autoplay,
                random_target: queued.random_target,
                is_end_turn_autoplay: queued.is_end_turn_autoplay,
                purge_on_use: queued.purge_on_use,
                source: queued.source,
            })
            .collect(),
    }
}

fn zone_key(cards: &[CombatCard]) -> Vec<CombatCardKey> {
    cards.iter().map(card_key).collect()
}

fn card_key(card: &CombatCard) -> CombatCardKey {
    CombatCardKey {
        id: card.id,
        uuid: card.uuid,
        upgrades: card.upgrades,
        misc_value: card.misc_value,
        base_damage_override: card.base_damage_override,
        base_block_override: card.base_block_override,
        cost_modifier: card.cost_modifier,
        cost_for_turn: card.cost_for_turn,
        base_damage_mut: card.base_damage_mut,
        base_block_mut: card.base_block_mut,
        base_magic_num_mut: card.base_magic_num_mut,
        multi_damage: card.multi_damage.iter().copied().collect(),
        exhaust_override: card.exhaust_override,
        retain_override: card.retain_override,
        free_to_play_once: card.free_to_play_once,
        energy_on_use: card.energy_on_use,
    }
}

fn target_key(combat: &CombatState, target: Option<usize>) -> CombatTargetKey {
    match target {
        None => CombatTargetKey::None,
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(CombatTargetKey::MonsterSlot)
            .unwrap_or(CombatTargetKey::Entity(entity_id)),
    }
}

fn monsters_key(combat: &CombatState) -> Vec<CombatMonsterKey> {
    combat.entities.monsters.iter().map(monster_key).collect()
}

fn monster_key(monster: &MonsterEntity) -> CombatMonsterKey {
    CombatMonsterKey {
        entity_id: monster.id,
        monster_type: monster.monster_type,
        current_hp: monster.current_hp,
        max_hp: monster.max_hp,
        block: monster.block,
        slot: monster.slot,
        logical_position: monster.logical_position,
        is_dying: monster.is_dying,
        is_escaped: monster.is_escaped,
        half_dead: monster.half_dead,
        planned_move_id: monster.planned_move_id(),
        move_history: format!("{:?}", monster.move_history()),
        turn_plan: format!("{:?}", monster.turn_plan()),
        runtime: monster_runtime_key(monster),
    }
}

fn monster_runtime_key(monster: &MonsterEntity) -> String {
    format!(
        "{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}|{:?}",
        monster.hexaghost,
        monster.louse,
        monster.jaw_worm,
        monster.thief,
        monster.byrd,
        monster.chosen,
        monster.snecko,
        monster.shelled_parasite,
        monster.bronze_automaton,
        monster.bronze_orb,
        monster.book_of_stabbing,
        monster.collector,
        monster.champ,
        monster.awakened_one,
        monster.corrupt_heart,
        monster.writhing_mass,
        monster.spiker,
        monster.spire_shield,
        monster.spire_spear,
        monster.slaver_red,
        monster.gremlin_leader,
        monster.gremlin_nob,
        monster.gremlin_wizard,
        monster.cultist,
        monster.sentry,
        monster.slime_boss,
        monster.large_slime,
        monster.spheric_guardian,
        monster.reptomancer,
        monster.darkling,
        monster.nemesis,
        monster.giant_head,
        monster.time_eater,
        monster.donu,
        monster.deca,
        monster.transient,
        monster.exploder,
        monster.maw,
    )
}

fn powers_key(combat: &CombatState) -> Vec<CombatEntityPowersKey> {
    let mut entries = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| {
            let powers = powers.iter().map(power_key).collect::<Vec<_>>();
            CombatEntityPowersKey {
                entity_id: *entity,
                powers,
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.entity_id);
    entries
}

fn power_key(power: &Power) -> CombatPowerKey {
    CombatPowerKey {
        power_type: power.power_type,
        instance_id: power.instance_id,
        amount: power.amount,
        extra_data: power.extra_data,
        payload: match &power.payload {
            PowerPayload::None => CombatPowerPayloadKey::None,
            PowerPayload::Card(card) => CombatPowerPayloadKey::Card(card_key(card)),
        },
        just_applied: power.just_applied,
    }
}

fn potions_key(combat: &CombatState) -> Vec<CombatPotionSlotKey> {
    combat
        .entities
        .potions
        .iter()
        .enumerate()
        .map(|(slot, potion)| CombatPotionSlotKey {
            slot,
            potion: potion.as_ref().map(|potion| CombatPotionKey {
                id: potion.id,
                uuid: potion.uuid,
                can_use: potion.can_use,
                can_discard: potion.can_discard,
                requires_target: potion.requires_target,
            }),
        })
        .collect()
}

fn queue_key(combat: &CombatState) -> Vec<CombatQueuedActionKey> {
    combat.engine.action_queue.iter().map(action_key).collect()
}

fn action_key(action: &Action) -> CombatQueuedActionKey {
    CombatQueuedActionKey {
        discriminant: std::mem::discriminant(action),
        payload: format!("{action:?}"),
    }
}

fn runtime_key(combat: &CombatState) -> CombatRuntimeHintsKey {
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

fn rng_pool_key(pool: &RngPool) -> CombatRngPoolKey {
    CombatRngPoolKey {
        monster_rng: sts_rng_key(&pool.monster_rng),
        event_rng: sts_rng_key(&pool.event_rng),
        merchant_rng: sts_rng_key(&pool.merchant_rng),
        card_rng: sts_rng_key(&pool.card_rng),
        treasure_rng: sts_rng_key(&pool.treasure_rng),
        relic_rng: sts_rng_key(&pool.relic_rng),
        potion_rng: sts_rng_key(&pool.potion_rng),
        monster_hp_rng: sts_rng_key(&pool.monster_hp_rng),
        ai_rng: sts_rng_key(&pool.ai_rng),
        shuffle_rng: sts_rng_key(&pool.shuffle_rng),
        card_random_rng: sts_rng_key(&pool.card_random_rng),
        misc_rng: sts_rng_key(&pool.misc_rng),
        math_rng: sts_rng_key(&pool.math_rng),
    }
}

fn sts_rng_key(rng: &StsRng) -> CombatStsRngKey {
    CombatStsRngKey {
        seed0: rng.seed0,
        seed1: rng.seed1,
        counter: rng.counter,
    }
}
