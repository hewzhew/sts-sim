use crate::map::node::RoomType;
use crate::rewards::state::RewardScreenContext;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::{
    DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
    SelectionTargetRef,
};

use super::campfire_handler;
use super::shop_handler;

fn run_selection_source(
    run_state: &RunState,
    reason: crate::state::core::RunPendingChoiceReason,
) -> DomainEventSource {
    run_state
        .event_state
        .as_ref()
        .map(|event| DomainEventSource::Event(event.id))
        .unwrap_or_else(|| DomainEventSource::Selection(reason.into()))
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
                        match change {
                            crate::runtime::combat::MetaChange::AddCardToMasterDeck(card_id) => {
                                run_state.add_card_to_deck(card_id);
                            }
                        }
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
                // Validation against min/max would securely happen here or in the UI client.
                // Assuming it's valid:
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
                    crate::state::core::RunPendingChoiceReason::Purge => {
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
                    crate::state::core::RunPendingChoiceReason::Transform => {
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
                        // Duplicate: copy the selected card(s) and add to deck
                        let cards_to_copy: Vec<_> = sorted_indices
                            .iter()
                            .filter_map(|&idx| {
                                run_state.master_deck.get(idx).map(|c| (c.id, c.upgrades))
                            })
                            .collect();
                        for (card_id, upgrades) in cards_to_copy {
                            run_state
                                .add_card_to_deck_with_upgrades_from(card_id, upgrades, source);
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
                    if let Some(maw) = run_state
                        .relics
                        .iter()
                        .find(|r| r.id == crate::content::relics::RelicId::MawBank && !r.used_up)
                    {
                        let _ = maw; // borrow satisfied
                        run_state.gold += 12;
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
                                    run_state.current_hp =
                                        (run_state.current_hp + heal).min(run_state.max_hp);
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
                                run_state.gold += 50;
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
                                // AncientTeaSet: set counter = -2 → at_turn_start grants +2 energy
                                for relic in run_state.relics.iter_mut() {
                                    if relic.id == crate::content::relics::RelicId::AncientTeaSet {
                                        relic.counter = -2;
                                    }
                                }
                                *engine_state = EngineState::Campfire;
                            }
                            RoomType::ShopRoom => {
                                // MealTicket: heal 15 HP on shop entry
                                if run_state
                                    .relics
                                    .iter()
                                    .any(|r| r.id == crate::content::relics::RelicId::MealTicket)
                                {
                                    run_state.current_hp =
                                        (run_state.current_hp + 15).min(run_state.max_hp);
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
                                // Generate chest relic reward (simplified chest logic)
                                let relic_id = run_state.random_relic();
                                reward
                                    .items
                                    .push(crate::rewards::state::RewardItem::Relic { relic_id });
                                // Sapphire key: paired with relic (Java: AbstractChest:87)
                                if run_state.is_final_act_available && !run_state.keys[1] {
                                    reward
                                        .items
                                        .push(crate::rewards::state::RewardItem::SapphireKey);
                                }

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
                                            .misc_rng
                                            .random_range(0, (curse_pool.len() - 1) as i32)
                                            as usize;
                                        run_state.add_card_to_deck(curse_pool[idx]);
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
                                    let extra_relic = run_state.random_relic();
                                    reward.items.push(crate::rewards::state::RewardItem::Relic {
                                        relic_id: extra_relic,
                                    });
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
                                    // Remove the first relic reward
                                    if let Some(pos) = reward.items.iter().position(|item| {
                                        matches!(
                                            item,
                                            crate::rewards::state::RewardItem::Relic { .. }
                                        )
                                    }) {
                                        reward.items.remove(pos);
                                    }
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
                        match change {
                            crate::runtime::combat::MetaChange::AddCardToMasterDeck(card_id) => {
                                run_state.add_card_to_deck(card_id);
                            }
                        }
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
