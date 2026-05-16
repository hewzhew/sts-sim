use crate::map::node::RoomType;
use crate::rewards::state::RewardScreenContext;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{
    DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
    SelectionTargetRef,
};

fn remove_one_relic_from_rewards_after_chest_open(
    items: &mut Vec<crate::rewards::state::RewardItem>,
) {
    if let Some(pos) = items
        .iter()
        .position(|item| matches!(item, crate::rewards::state::RewardItem::Relic { .. }))
    {
        items.remove(pos);
        if matches!(
            items.get(pos),
            Some(crate::rewards::state::RewardItem::SapphireKey)
        ) {
            items.remove(pos);
        }
    }
}

use super::campfire_handler;
use super::shop_handler;

fn bottled_choice_target(
    reason: &crate::state::core::RunPendingChoiceReason,
) -> Option<(
    crate::content::relics::RelicId,
    crate::content::cards::CardType,
)> {
    match reason {
        crate::state::core::RunPendingChoiceReason::BottleFlame => Some((
            crate::content::relics::RelicId::BottledFlame,
            crate::content::cards::CardType::Attack,
        )),
        crate::state::core::RunPendingChoiceReason::BottleLightning => Some((
            crate::content::relics::RelicId::BottledLightning,
            crate::content::cards::CardType::Skill,
        )),
        crate::state::core::RunPendingChoiceReason::BottleTornado => Some((
            crate::content::relics::RelicId::BottledTornado,
            crate::content::cards::CardType::Power,
        )),
        _ => None,
    }
}

fn assign_bottled_card(
    run_state: &mut RunState,
    relic_id: crate::content::relics::RelicId,
    card_type: crate::content::cards::CardType,
    selected_indices: &[usize],
) {
    let Some(&idx) = selected_indices.first() else {
        return;
    };
    let Some(card) = run_state.master_deck.get(idx) else {
        return;
    };
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type != card_type {
        return;
    }

    let selected_uuid = card.uuid as i32;
    if let Some(relic) = run_state
        .relics
        .iter_mut()
        .rev()
        .find(|relic| relic.id == relic_id && relic.amount == 0)
    {
        relic.amount = selected_uuid;
    } else if let Some(relic) = run_state
        .relics
        .iter_mut()
        .rev()
        .find(|relic| relic.id == relic_id)
    {
        relic.amount = selected_uuid;
    }
}

fn run_selection_source(
    run_state: &RunState,
    reason: crate::state::core::RunPendingChoiceReason,
) -> DomainEventSource {
    if let Some(event) = run_state.event_state.as_ref() {
        return DomainEventSource::Event(event.id);
    }

    let has_relic = |id| run_state.relics.iter().any(|relic| relic.id == id);
    match reason {
        crate::state::core::RunPendingChoiceReason::TransformUpgraded
            if has_relic(crate::content::relics::RelicId::Astrolabe) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::Astrolabe)
        }
        crate::state::core::RunPendingChoiceReason::Purge
            if has_relic(crate::content::relics::RelicId::EmptyCage) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::EmptyCage)
        }
        crate::state::core::RunPendingChoiceReason::Duplicate
            if has_relic(crate::content::relics::RelicId::DollysMirror) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::DollysMirror)
        }
        crate::state::core::RunPendingChoiceReason::BottleFlame => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledFlame)
        }
        crate::state::core::RunPendingChoiceReason::BottleLightning => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledLightning)
        }
        crate::state::core::RunPendingChoiceReason::BottleTornado => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledTornado)
        }
        reason => DomainEventSource::Selection(reason.into()),
    }
}

fn resolve_run_pending_selection(input: ClientInput, run_state: &RunState) -> Option<Vec<usize>> {
    match input {
        ClientInput::SubmitSelection(SelectionResolution {
            scope: SelectionScope::Deck,
            selected,
        }) => Some(
            selected
                .into_iter()
                .filter_map(|target| match target {
                    SelectionTargetRef::CardUuid(uuid) => run_state
                        .master_deck
                        .iter()
                        .position(|card| card.uuid == uuid),
                })
                .collect(),
        ),
        ClientInput::SubmitDeckSelect(indices) => Some(indices),
        _ => None,
    }
}

fn resolve_out_of_combat_defeat(engine_state: &mut EngineState, run_state: &RunState) -> bool {
    if run_state.current_hp <= 0 && !matches!(engine_state, EngineState::GameOver(_)) {
        *engine_state = EngineState::GameOver(crate::state::core::RunResult::Defeat);
        return true;
    }
    false
}

fn apply_combat_meta_change(run_state: &mut RunState, change: crate::runtime::combat::MetaChange) {
    match change {
        crate::runtime::combat::MetaChange::AddCardToMasterDeck(card_id) => {
            run_state.add_card_to_deck(card_id);
        }
        crate::runtime::combat::MetaChange::ModifyCardMisc { card_uuid, amount } => {
            run_state.modify_card_misc_value(card_uuid, amount);
        }
        crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid } => {
            run_state.upgrade_card_with_source(
                card_uuid,
                crate::state::selection::DomainEventSource::DeckMutation,
            );
        }
    }
}

pub fn tick_run(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    combat_state: &mut Option<CombatState>,
    input: Option<ClientInput>,
) -> bool {
    // Top level controller redirecting inputs
    match engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            if let Some(cs) = combat_state.as_mut() {
                let keep_running = super::core::tick_engine(engine_state, cs, input.clone());
                if !keep_running {
                    // Absorb combat player state back to RunState (HP, gold, relic counters)
                    run_state.absorb_combat_player(cs.entities.player.clone());
                    run_state.room_mugged |= cs.runtime.combat_mugged;
                    run_state.room_smoked |= cs.runtime.combat_smoked;

                    for change in cs.meta.meta_changes.drain(..) {
                        apply_combat_meta_change(run_state, change);
                    }

                    // Check for Act 3 boss victory → Act 4 transition
                    // Java: AbstractRoom:317 — if BossRoom + TheBeyond/TheEnding + 3 keys → skip rewards
                    if let EngineState::RewardScreen(rs) = engine_state {
                        let is_boss = cs.meta.is_boss_fight;
                        let is_elite = cs.meta.is_elite_fight;
                        let screen_context = if run_state.room_mugged {
                            RewardScreenContext::MuggedCombat
                        } else if run_state.room_smoked {
                            RewardScreenContext::SmokedCombat
                        } else {
                            RewardScreenContext::Standard
                        };
                        if !matches!(screen_context, RewardScreenContext::SmokedCombat) {
                            // Populate the actual dropped rewards for normal/mugged combat.
                            *rs = crate::rewards::generator::generate_combat_rewards(
                                run_state, is_elite, is_boss,
                            );
                            rs.items.append(&mut cs.runtime.pending_rewards);
                        }
                        rs.screen_context = screen_context;

                        if is_boss
                            && run_state.act_num == 3
                            && run_state.is_final_act_available
                            && run_state.keys[0]
                            && run_state.keys[1]
                            && run_state.keys[2]
                        {
                            // All 3 keys collected — transition to Act 4 (TheEnding)
                            let ending_map = crate::map::generator::generate_ending_map();
                            run_state.map = crate::map::state::MapState::new(ending_map);
                            run_state.act_num = 4;
                            *engine_state = EngineState::MapNavigation;
                        } else if is_boss && run_state.act_num <= 2 {
                            // Act 1 or Act 2 boss defeated — mark for act advance after rewards
                            run_state.pending_boss_reward = true;
                        } else if is_boss && run_state.act_num == 3 {
                            // Act 3 boss defeated without all keys → game victory (no Act 4)
                            *engine_state =
                                EngineState::GameOver(crate::state::core::RunResult::Victory);
                        } else {
                            // Normal (non-boss) elite reward generation adds emerald key if present
                            if is_elite && run_state.is_final_act_available && !run_state.keys[2] {
                                if let Some(node) = run_state.map.get_current_node() {
                                    if node.has_emerald_key {
                                        rs.items
                                            .push(crate::rewards::state::RewardItem::EmeraldKey);
                                    }
                                }
                            }
                        }
                    }
                    if let EngineState::GameOver(_) = engine_state {
                        return false;
                    }
                }
                true
            } else {
                eprintln!("Error: EngineState designates Combat but no CombatState was provided.");
                false
            }
        }
        EngineState::RunPendingChoice(rpc_state) => {
            if let Some(indices) = input
                .clone()
                .and_then(|value| resolve_run_pending_selection(value, run_state))
            {
                if indices.len() < rpc_state.min_choices || indices.len() > rpc_state.max_choices {
                    return true;
                }
                let mut seen_indices = Vec::new();
                for &idx in &indices {
                    let Some(card) = run_state.master_deck.get(idx) else {
                        return true;
                    };
                    if seen_indices.contains(&idx)
                        || !crate::state::core::run_pending_choice_allows_card_for_run(
                            &rpc_state.reason,
                            card,
                            run_state,
                        )
                    {
                        return true;
                    }
                    seen_indices.push(idx);
                }

                let mut sorted_indices = indices.clone();
                sorted_indices.sort_unstable();
                sorted_indices.reverse(); // Remove from highest index to lowest
                let source = run_selection_source(run_state, rpc_state.reason.clone());
                let selection_reason: SelectionReason = rpc_state.reason.clone().into();
                let selected_refs = sorted_indices
                    .iter()
                    .filter_map(|&idx| run_state.master_deck.get(idx))
                    .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                    .collect::<Vec<_>>();

                run_state.emit_event(DomainEvent::SelectionResolved {
                    scope: SelectionScope::Deck,
                    reason: selection_reason,
                    selected: selected_refs,
                    source,
                });

                match rpc_state.reason {
                    crate::state::core::RunPendingChoiceReason::Purge
                    | crate::state::core::RunPendingChoiceReason::PurgeNonBottled => {
                        for idx in sorted_indices {
                            if idx < run_state.master_deck.len() {
                                // Store removed card's rarity in event_state.internal_state
                                // so events (bonfire_elementals, bonfire_spirits) can apply
                                // rarity-based rewards after purge returns.
                                // Encoding: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare
                                if let Some(ref mut es) = run_state.event_state {
                                    let def = crate::content::cards::get_card_definition(
                                        run_state.master_deck[idx].id,
                                    );
                                    es.internal_state = match def.rarity {
                                        crate::content::cards::CardRarity::Curse => 0,
                                        crate::content::cards::CardRarity::Basic => 1,
                                        crate::content::cards::CardRarity::Common => 2,
                                        crate::content::cards::CardRarity::Special => 3,
                                        crate::content::cards::CardRarity::Uncommon => 4,
                                        crate::content::cards::CardRarity::Rare => 5,
                                    };
                                }
                                let uuid = run_state.master_deck[idx].uuid;
                                run_state.remove_card_from_deck_with_source(uuid, source);
                            }
                        }
                    }
                    crate::state::core::RunPendingChoiceReason::Upgrade => {
                        for idx in sorted_indices {
                            if idx < run_state.master_deck.len() {
                                let uuid = run_state.master_deck[idx].uuid;
                                run_state.upgrade_card_with_source(uuid, source);
                            }
                        }
                    }
                    crate::state::core::RunPendingChoiceReason::Transform
                    | crate::state::core::RunPendingChoiceReason::TransformNonBottled => {
                        for idx in sorted_indices {
                            if idx < run_state.master_deck.len() {
                                run_state.transform_card_with_source(idx, false, source);
                            }
                        }
                    }
                    crate::state::core::RunPendingChoiceReason::TransformUpgraded => {
                        for idx in sorted_indices {
                            if idx < run_state.master_deck.len() {
                                run_state.transform_card_with_source(idx, true, source);
                            }
                        }
                    }
                    crate::state::core::RunPendingChoiceReason::Duplicate => {
                        let cards_to_copy: Vec<_> = sorted_indices
                            .iter()
                            .filter_map(|&idx| run_state.master_deck.get(idx).cloned())
                            .collect();
                        for card in cards_to_copy {
                            run_state.add_card_instance_copy_to_deck_from(&card, source);
                        }
                    }
                    reason @ (crate::state::core::RunPendingChoiceReason::BottleFlame
                    | crate::state::core::RunPendingChoiceReason::BottleLightning
                    | crate::state::core::RunPendingChoiceReason::BottleTornado) => {
                        if let Some((relic_id, card_type)) = bottled_choice_target(&reason) {
                            assign_bottled_card(run_state, relic_id, card_type, &sorted_indices);
                        }
                    }
                }

                // Return to the previous stashed state (e.g. Map, Event, or Shop)
                *engine_state = *rpc_state.return_state.clone();
            } else if let Some(ClientInput::Cancel) = input {
                // Return to stashed state without mutating deck
                *engine_state = *rpc_state.return_state.clone();
            } else {
                // Input wasn't matched, preserve State
            }
            true
        }
        EngineState::MapNavigation => {
            // Extract travel target from input: normal adjacency or WingBoots flight
            let travel_target = match &input {
                Some(ClientInput::SelectMapNode(target_x)) => {
                    let target_y = if run_state.map.current_y == -1 {
                        0
                    } else {
                        run_state.map.current_y + 1
                    };
                    Some((*target_x as i32, target_y, false))
                }
                Some(ClientInput::FlyToNode(target_x, target_y)) => {
                    Some((*target_x as i32, *target_y as i32, true))
                }
                _ => None,
            };

            if let Some((target_x, target_y, is_flight)) = travel_target {
                // WingBoots: check if player has charges for flight
                let has_flight = if is_flight {
                    run_state.relics.iter().any(|r| {
                        r.id == crate::content::relics::RelicId::WingBoots && r.counter > 0
                    })
                } else {
                    false
                };

                if run_state
                    .map
                    .travel_to(target_x, target_y, has_flight)
                    .is_ok()
                {
                    run_state.room_mugged = false;
                    run_state.room_smoked = false;
                    // Increment floor number successfully entering a new room
                    run_state.floor_num += 1;

                    // WingBoots: decrement counter on successful flight (non-adjacent travel)
                    if is_flight {
                        if let Some(wb) = run_state
                            .relics
                            .iter_mut()
                            .find(|r| r.id == crate::content::relics::RelicId::WingBoots)
                        {
                            wb.counter -= 1;
                            if wb.counter == 0 {
                                wb.counter = -2;
                                wb.used_up = true;
                            }
                        }
                    }

                    // --- onEnterRoom() relic hooks (fire for ALL room types) ---
                    // MawBank: +12 gold each room entered (unless used up from spending gold)
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::MawBank && !r.used_up)
                    {
                        run_state.change_gold_with_source(
                            12,
                            DomainEventSource::Relic(crate::content::relics::RelicId::MawBank),
                        );
                    }

                    if let Some(room_type) = run_state.map.get_current_room_type() {
                        // --- Room-type-specific onEnterRoom hooks ---
                        // EternalFeather: heal (deck_size / 5 * 3) on entering RestRoom
                        if room_type == RoomType::RestRoom {
                            if run_state
                                .relics
                                .iter()
                                .any(|r| r.id == crate::content::relics::RelicId::EternalFeather)
                            {
                                let heal = (run_state.master_deck.len() / 5 * 3) as i32;
                                if heal > 0
                                    && !run_state.relics.iter().any(|r| {
                                        r.id == crate::content::relics::RelicId::MarkOfTheBloom
                                    })
                                {
                                    run_state.change_hp_with_source(
                                        heal,
                                        DomainEventSource::Relic(
                                            crate::content::relics::RelicId::EternalFeather,
                                        ),
                                    );
                                }
                            }
                        }
                        // SsserpentHead: +50 gold on entering EventRoom
                        if room_type == RoomType::EventRoom {
                            if run_state
                                .relics
                                .iter()
                                .any(|r| r.id == crate::content::relics::RelicId::SsserpentHead)
                            {
                                run_state.change_gold_with_source(
                                    50,
                                    DomainEventSource::Relic(
                                        crate::content::relics::RelicId::SsserpentHead,
                                    ),
                                );
                            }
                        }

                        match room_type {
                            RoomType::MonsterRoom
                            | RoomType::MonsterRoomElite
                            | RoomType::MonsterRoomBoss => {
                                // Instantiate combat
                                *engine_state = EngineState::CombatPlayerTurn;
                            }
                            RoomType::RestRoom => {
                                // Java: onEnterRestRoom() for all relics
                                run_state.on_enter_rest_room();
                                *engine_state = EngineState::Campfire;
                            }
                            RoomType::ShopRoom => {
                                // MealTicket: heal 15 HP on shop entry
                                if run_state
                                    .relics
                                    .iter()
                                    .any(|r| r.id == crate::content::relics::RelicId::MealTicket)
                                    && !run_state.relics.iter().any(|r| {
                                        r.id == crate::content::relics::RelicId::MarkOfTheBloom
                                    })
                                {
                                    run_state.change_hp_with_source(
                                        15,
                                        DomainEventSource::Relic(
                                            crate::content::relics::RelicId::MealTicket,
                                        ),
                                    );
                                }
                                *engine_state = EngineState::Shop(run_state.generate_shop());
                            }
                            RoomType::EventRoom => {
                                let event_id = run_state.generate_event();
                                let mut event_state =
                                    crate::state::events::EventState::new(event_id);
                                // Wire init functions for events with constructor-time RNG
                                use crate::state::events::EventId;
                                event_state.internal_state = match event_id {
                                    EventId::Nloth => crate::content::events::nloth::init_nloth_state(run_state),
                                    EventId::WeMeetAgain => crate::content::events::we_meet_again::init_we_meet_again_state(run_state),
                                    EventId::DeadAdventurer => crate::content::events::dead_adventurer::init_dead_adventurer_state(run_state),
                                    EventId::Designer => crate::content::events::designer::init_designer_state(run_state),
                                    EventId::WorldOfGoop => crate::content::events::goop_puddle::init_goop_puddle_state(run_state),
                                    EventId::Falling => crate::content::events::falling::init_falling_state(run_state),
                                    _ => 0,
                                };
                                // Events with extra_data init (complex state)
                                if event_id == EventId::MatchAndKeep {
                                    crate::content::events::match_and_keep::init_match_game_board(
                                        run_state,
                                        &mut event_state.extra_data,
                                    );
                                }
                                run_state.event_state = Some(event_state);
                                *engine_state = EngineState::EventRoom;
                            }
                            RoomType::TreasureRoom => {
                                let mut reward = crate::rewards::state::RewardState::new();

                                // --- onChestOpen() relic hooks (non-boss chest) ---
                                // CursedKey: add a random curse to deck
                                if run_state
                                    .relics
                                    .iter()
                                    .any(|r| r.id == crate::content::relics::RelicId::CursedKey)
                                {
                                    let curse_pool = crate::content::cards::get_curse_pool();
                                    if !curse_pool.is_empty() {
                                        let idx = run_state
                                            .rng_pool
                                            .card_rng
                                            .random_range(0, (curse_pool.len() - 1) as i32)
                                            as usize;
                                        run_state.add_card_to_deck_with_upgrades_from(
                                            curse_pool[idx],
                                            0,
                                            crate::state::selection::DomainEventSource::Relic(
                                                crate::content::relics::RelicId::CursedKey,
                                            ),
                                        );
                                    }
                                }

                                // Matryoshka: add an extra relic reward (75% Common, 25% Uncommon)
                                if let Some(mat) = run_state.relics.iter_mut().find(|r| {
                                    r.id == crate::content::relics::RelicId::Matryoshka
                                        && r.counter > 0
                                }) {
                                    mat.counter -= 1;
                                    if mat.counter == 0 {
                                        mat.counter = -2;
                                        mat.used_up = true;
                                    }
                                    let extra_tier =
                                        if run_state.rng_pool.relic_rng.random_boolean_chance(0.75)
                                        {
                                            crate::content::relics::RelicTier::Common
                                        } else {
                                            crate::content::relics::RelicTier::Uncommon
                                        };
                                    let extra_relic = run_state.random_relic_by_tier(extra_tier);
                                    reward.items.push(crate::rewards::state::RewardItem::Relic {
                                        relic_id: extra_relic,
                                    });
                                }

                                // Generate chest relic reward after onChestOpen hooks, matching
                                // Java AbstractChest.open(): Matryoshka inserts before the
                                // base chest relic, and SapphireKey links to the last relic.
                                let relic_id = run_state.random_relic();
                                reward
                                    .items
                                    .push(crate::rewards::state::RewardItem::Relic { relic_id });
                                if run_state.is_final_act_available && !run_state.keys[1] {
                                    reward
                                        .items
                                        .push(crate::rewards::state::RewardItem::SapphireKey);
                                }

                                // NlothsMask: remove one relic from rewards (onChestOpenAfter)
                                if let Some(mask) = run_state.relics.iter_mut().find(|r| {
                                    r.id == crate::content::relics::RelicId::NlothsMask
                                        && r.counter > 0
                                }) {
                                    mask.counter -= 1;
                                    if mask.counter == 0 {
                                        mask.counter = -2;
                                        mask.used_up = true;
                                    }
                                    remove_one_relic_from_rewards_after_chest_open(
                                        &mut reward.items,
                                    );
                                }

                                *engine_state = EngineState::RewardScreen(reward);
                            }
                            RoomType::TrueVictoryRoom => {
                                // Act 4 ending — true victory after defeating the Heart
                                *engine_state =
                                    EngineState::GameOver(crate::state::core::RunResult::Victory);
                            }
                        }
                    }
                }
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::Campfire => {
            let keep_running = campfire_handler::handle(engine_state, run_state, input);
            if !keep_running {
                return false;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::Shop(_) => {
            let mut transition = None;
            if let EngineState::Shop(shop) = engine_state {
                if let Some(new_state) = shop_handler::handle(run_state, shop, input.clone()) {
                    transition = Some(new_state);
                }
            }
            if let Some(new_state) = transition {
                *engine_state = new_state;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::EventRoom => {
            if let Some(ClientInput::EventChoice(choice_idx)) = input {
                if let Err(e) = crate::engine::event_handler::handle_event_choice(
                    engine_state,
                    run_state,
                    choice_idx,
                ) {
                    eprintln!("Event Error: {}", e);
                }
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::RewardScreen(_) => {
            let mut transition = None;
            if let EngineState::RewardScreen(rs) = engine_state {
                if let Some(new_state) =
                    crate::rewards::handler::handle(run_state, rs, input.clone())
                {
                    transition = Some(new_state);
                }
            }
            if let Some(new_state) = transition {
                *engine_state = new_state;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::BossRelicSelect(_) => {
            let mut transition = None;
            if let EngineState::BossRelicSelect(bs) = engine_state {
                if let Some(new_state) =
                    crate::rewards::boss_handler::handle(run_state, bs, input.clone())
                {
                    transition = Some(new_state);
                }
            }
            if let Some(new_state) = transition {
                *engine_state = new_state;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return false;
            }
            true
        }
        EngineState::EventCombat(ecs) => {
            // Event combat delegates to normal combat tick.
            // When combat ends (engine transitions away from Combat states),
            // we intercept and handle rewards/return based on EventCombatState.
            if let Some(cs) = combat_state.as_mut() {
                // Create a temporary combat engine state to tick
                let mut temp_state = EngineState::CombatPlayerTurn;
                let keep_running = super::core::tick_engine(&mut temp_state, cs, input.clone());

                if !keep_running {
                    // Absorb combat player state back to RunState (HP, gold, relic counters)
                    run_state.absorb_combat_player(cs.entities.player.clone());
                    run_state.room_mugged |= cs.runtime.combat_mugged;
                    run_state.room_smoked |= cs.runtime.combat_smoked;

                    for change in cs.meta.meta_changes.drain(..) {
                        apply_combat_meta_change(run_state, change);
                    }

                    // Combat ended. Check if player died.
                    if let EngineState::GameOver(_) = temp_state {
                        *engine_state = temp_state;
                        return false;
                    }

                    // Combat victory. Handle rewards.
                    if ecs.reward_allowed {
                        // Generate standard card rewards unless suppressed
                        let mut rewards = ecs.rewards.clone();
                        rewards.screen_context = if run_state.room_mugged {
                            RewardScreenContext::MuggedCombat
                        } else if run_state.room_smoked {
                            RewardScreenContext::SmokedCombat
                        } else {
                            RewardScreenContext::Standard
                        };
                        if !matches!(rewards.screen_context, RewardScreenContext::SmokedCombat) {
                            rewards.items.append(&mut cs.runtime.pending_rewards);
                        }
                        if !ecs.no_cards_in_rewards
                            && !matches!(rewards.screen_context, RewardScreenContext::SmokedCombat)
                        {
                            let card_reward = crate::rewards::generator::generate_combat_rewards(
                                run_state, false, false,
                            );
                            // Merge card reward items into pre-populated rewards
                            for item in card_reward.items {
                                if matches!(item, crate::rewards::state::RewardItem::Card { .. }) {
                                    rewards.items.push(item);
                                }
                            }
                        }
                        *engine_state = EngineState::RewardScreen(rewards);
                    } else {
                        // No rewards (e.g., Colosseum fight 1) — go directly to return
                        match ecs.post_combat_return {
                            crate::state::core::PostCombatReturn::EventRoom => {
                                *engine_state = EngineState::EventRoom;
                            }
                            crate::state::core::PostCombatReturn::MapNavigation => {
                                *engine_state = EngineState::MapNavigation;
                            }
                        }
                    }
                }
                // If combat is still running, stay in EventCombat
                true
            } else {
                eprintln!("Error: EventCombat but no CombatState provided.");
                false
            }
        }
        EngineState::GameOver(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_combat_meta_change, remove_one_relic_from_rewards_after_chest_open, tick_run,
    };
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;
    use crate::rewards::state::RewardItem;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::run::RunState;
    use crate::state::selection::{
        DomainEventSource, SelectionReason, SelectionResolution, SelectionScope, SelectionTargetRef,
    };

    fn run_state_with_first_room(room_type: RoomType) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(room_type);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        run_state.map = MapState::new(vec![vec![first], vec![second]]);
        run_state
    }

    #[test]
    fn meal_ticket_shop_entry_heal_uses_relic_source_and_mark_of_bloom_guard() {
        let mut run_state = run_state_with_first_room(RoomType::ShopRoom);
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::MealTicket));
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(run_state.current_hp, 35);
        assert!(matches!(engine_state, EngineState::Shop(_)));

        let mut blocked = run_state_with_first_room(RoomType::ShopRoom);
        blocked.current_hp = 20;
        blocked.max_hp = 80;
        blocked.relics.clear();
        blocked.relics.push(RelicState::new(RelicId::MealTicket));
        blocked
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut blocked_engine = EngineState::MapNavigation;
        let mut blocked_combat = None;

        assert!(tick_run(
            &mut blocked_engine,
            &mut blocked,
            &mut blocked_combat,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(blocked.current_hp, 20);
        assert!(matches!(blocked_engine, EngineState::Shop(_)));
    }

    #[test]
    fn eternal_feather_rest_room_heal_uses_relic_source_and_mark_of_bloom_guard() {
        let mut run_state = run_state_with_first_room(RoomType::RestRoom);
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::EternalFeather));
        run_state.master_deck = (0..10)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect();
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(run_state.current_hp, 26);
        assert!(matches!(engine_state, EngineState::Campfire));

        let mut blocked = run_state_with_first_room(RoomType::RestRoom);
        blocked.current_hp = 20;
        blocked.max_hp = 80;
        blocked.relics.clear();
        blocked
            .relics
            .push(RelicState::new(RelicId::EternalFeather));
        blocked
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        blocked.master_deck = (0..10)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect();
        let mut blocked_engine = EngineState::MapNavigation;
        let mut blocked_combat = None;

        assert!(tick_run(
            &mut blocked_engine,
            &mut blocked,
            &mut blocked_combat,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(blocked.current_hp, 20);
        assert!(matches!(blocked_engine, EngineState::Campfire));
    }

    #[test]
    fn bottled_relic_on_equip_filters_selection_by_card_type_and_marks_uuid() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.master_deck = vec![
            CombatCard::new(CardId::Bash, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::Inflame, 103),
        ];

        let next_state = run_state
            .obtain_relic_with_source(
                RelicId::BottledFlame,
                EngineState::MapNavigation,
                DomainEventSource::RewardScreen,
            )
            .expect("Bottled Flame should open a deck selection when an attack exists");

        let EngineState::RunPendingChoice(choice) = next_state else {
            panic!("Bottled Flame should return RunPendingChoice");
        };
        let request = choice.selection_request(&run_state);
        assert_eq!(request.reason, SelectionReason::BottleFlame);
        assert_eq!(request.targets, vec![SelectionTargetRef::CardUuid(101)]);

        let mut engine_state = EngineState::RunPendingChoice(choice);
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::BottledFlame)
                .map(|relic| relic.amount),
            Some(101)
        );
    }

    #[test]
    fn duplicate_selection_preserves_stat_equivalent_card_state_without_copying_bottle_attachment()
    {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        let mut bottled = RelicState::new(RelicId::BottledFlame);
        bottled.amount = 101;
        run_state.relics.push(bottled);

        let mut original = CombatCard::new(CardId::RitualDagger, 101);
        original.upgrades = 2;
        original.misc_value = 17;
        run_state.master_deck = vec![original];

        let next_state = run_state
            .obtain_relic_with_source(
                RelicId::DollysMirror,
                EngineState::MapNavigation,
                DomainEventSource::RewardScreen,
            )
            .expect("Dolly's Mirror should open a deck selection");

        let mut engine_state = next_state;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(run_state.master_deck.len(), 2);
        let copied = run_state
            .master_deck
            .iter()
            .find(|card| card.uuid != 101)
            .expect("Dolly's Mirror should add a copied card");
        assert_eq!(copied.id, CardId::RitualDagger);
        assert_eq!(copied.upgrades, 2);
        assert_eq!(copied.misc_value, 17);
        assert_ne!(copied.uuid, 101);
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::BottledFlame)
                .map(|relic| relic.amount),
            Some(101),
            "Java clears bottle flags on the copied card; Rust bottle attachment stays on original UUID"
        );
    }

    #[test]
    fn combat_misc_meta_change_updates_matching_master_deck_card() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut dagger = CombatCard::new(CardId::RitualDagger, 101);
        dagger.misc_value = 17;
        run_state.master_deck = vec![dagger];

        apply_combat_meta_change(
            &mut run_state,
            crate::runtime::combat::MetaChange::ModifyCardMisc {
                card_uuid: 101,
                amount: 3,
            },
        );

        assert_eq!(
            run_state.master_deck[0].misc_value, 20,
            "Java RitualDaggerAction updates player.masterDeck before GetAllInBattleInstances"
        );
    }

    #[test]
    fn combat_upgrade_meta_change_updates_matching_master_deck_card() {
        let mut run_state = RunState::new(1, 0, false, "Watcher");
        run_state.master_deck = vec![CombatCard::new(CardId::StrikeP, 201)];

        apply_combat_meta_change(
            &mut run_state,
            crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid: 201 },
        );

        assert_eq!(
            run_state.master_deck[0].upgrades, 1,
            "Java LessonLearnedAction upgrades a random canUpgrade() card from player.masterDeck"
        );
    }

    #[test]
    fn bottled_relic_uuid_counts_as_innate_during_combat_deck_initialization() {
        let mut state = crate::test_support::blank_test_combat();
        let mut bottle = RelicState::new(RelicId::BottledTornado);
        bottle.amount = 103;
        state.entities.player.add_relic(bottle);
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::Inflame, 103),
        ];

        state.apply_java_initialize_deck_order_after_shuffle();

        assert_eq!(
            state.zones.draw_pile.first().map(|card| card.uuid),
            Some(103),
            "the card selected by Bottled Tornado must be handled by the same start-hand path as innate cards"
        );
    }

    #[test]
    fn nloths_mask_chest_removal_also_removes_sapphire_key_linked_to_removed_relic() {
        let mut items = vec![
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        remove_one_relic_from_rewards_after_chest_open(&mut items);

        assert!(items.is_empty());
    }

    #[test]
    fn nloths_mask_removes_matryoshka_relic_before_base_chest_key_pair() {
        let mut items = vec![
            RewardItem::Relic {
                relic_id: RelicId::Omamori,
            },
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        remove_one_relic_from_rewards_after_chest_open(&mut items);

        assert_eq!(
            items,
            vec![
                RewardItem::Relic {
                    relic_id: RelicId::Mango,
                },
                RewardItem::SapphireKey,
            ],
            "Java Matryoshka adds its relic during onChestOpen before the base chest relic/key pair"
        );
    }
}
