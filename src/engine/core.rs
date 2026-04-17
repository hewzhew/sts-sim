use crate::content::powers::store;
use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatPhase, CombatState};
use crate::state::core::{ClientInput, EngineState, PendingChoice, RunResult};
use crate::state::selection::{EngineDiagnostic, EngineDiagnosticClass, EngineDiagnosticSeverity};
use smallvec::SmallVec;
use std::cell::Cell;

use super::pending_choices;
use super::targeting;

thread_local! {
    static SUPPRESS_ENGINE_WARNINGS_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn engine_warnings_enabled() -> bool {
    SUPPRESS_ENGINE_WARNINGS_DEPTH.with(|depth| depth.get() == 0)
}

fn record_engine_diagnostic(combat_state: &mut CombatState, diagnostic: EngineDiagnostic) {
    if engine_warnings_enabled() {
        combat_state.emit_diagnostic(diagnostic);
    }
}

pub(crate) fn with_suppressed_engine_warnings<T>(f: impl FnOnce() -> T) -> T {
    SUPPRESS_ENGINE_WARNINGS_DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        let result = f();
        depth.set(depth.get().saturating_sub(1));
        result
    })
}

fn compute_player_turn_start_draw_count(combat_state: &CombatState) -> i32 {
    let mut draw_count: i32 = 5 + combat_state.turn.turn_start_draw_modifier;
    if combat_state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::SneckoEye)
    {
        draw_count += 2;
    }
    draw_count
}

fn resolve_victory_hooks_immediately(combat_state: &mut CombatState) {
    let actions = crate::content::relics::hooks::on_victory(combat_state);
    if actions.is_empty() {
        return;
    }

    combat_state.queue_actions(actions);
    while let Some(action) = combat_state.pop_next_action() {
        crate::engine::action_handlers::execute_action(action, combat_state);
        combat_state.ensure_flush_next_queued_card();
    }
}

pub fn tick_engine(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: Option<ClientInput>,
) -> bool {
    // Phase 1: pending choice overrides
    if let EngineState::PendingChoice(_) = engine_state {
        if let Some(cmd) = input {
            if resolve_pending_choice(engine_state, combat_state, cmd).is_ok() {
                if !matches!(engine_state, EngineState::PendingChoice(_)) {
                    *engine_state = EngineState::CombatProcessing;
                }
            }
        }
        return true;
    }

    // Phase 2: process input
    if *engine_state == EngineState::CombatPlayerTurn {
        if let Some(cmd) = input {
            if handle_player_turn_input(engine_state, combat_state, cmd).is_ok() {
                // After a card play, actions (damage, block, etc.) are queued.
                // Transition to CombatProcessing to drain the queue.
                if combat_state.has_pending_actions() || !combat_state.zones.queued_cards.is_empty()
                {
                    *engine_state = EngineState::CombatProcessing;
                }
            } else {
                return true;
            }
        } else {
            return true;
        }
    }

    // Phase 3: execute action queue
    if *engine_state == EngineState::CombatProcessing {
        combat_state.ensure_flush_next_queued_card();
        if combat_state.has_pending_actions() {
            let next_action = combat_state.pop_next_action().unwrap();

            // Intercept SuspendFor* actions and transition to PendingChoice.
            match next_action {
                Action::SuspendForHandSelect {
                    min,
                    max,
                    can_cancel,
                    filter,
                    reason,
                } => {
                    let candidate_uuids = hand_select_candidates(combat_state, filter);
                    let available = candidate_uuids.len() as u8;
                    if available == 0 {
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if min == 0 {
                                    EngineDiagnosticSeverity::Info
                                } else {
                                    EngineDiagnosticSeverity::Error
                                },
                                class: if min == 0 {
                                    EngineDiagnosticClass::Normalization
                                } else {
                                    EngineDiagnosticClass::Broken
                                },
                                message: format!(
                                    "auto-skipped empty hand select for {:?} with filter {:?} (requested min={}, max={})",
                                    reason, filter, min, max
                                ),
                            },
                        );
                        return true;
                    }

                    if available == 1 && min == 1 && max == 1 && !can_cancel {
                        let _ = pending_choices::handle_hand_select(
                            engine_state,
                            combat_state,
                            &candidate_uuids,
                            1,
                            true,
                            false,
                            reason,
                            ClientInput::SubmitHandSelect(vec![candidate_uuids[0]]),
                        );
                        return true;
                    }

                    let min_cards = min.min(available);
                    let max_cards = max.min(available);
                    if min_cards != min || max_cards != max {
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if min > available {
                                    EngineDiagnosticSeverity::Warning
                                } else {
                                    EngineDiagnosticSeverity::Info
                                },
                                class: if min > available {
                                    EngineDiagnosticClass::Suspicious
                                } else {
                                    EngineDiagnosticClass::Normalization
                                },
                                message: format!(
                                    "normalized hand select for {:?} with filter {:?} from min/max {}/{} to {}/{} because only {} candidates remain",
                                    reason, filter, min, max, min_cards, max_cards, available
                                ),
                            },
                        );
                    }

                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::HandSelect {
                        candidate_uuids,
                        min_cards,
                        max_cards,
                        can_cancel,
                        reason,
                    });
                    return true;
                }
                Action::SuspendForGridSelect {
                    source_pile,
                    min,
                    max,
                    can_cancel,
                    filter,
                    reason,
                } => {
                    let candidate_uuids = grid_select_candidates(combat_state, source_pile, filter);
                    let available = candidate_uuids.len() as u8;
                    if available == 0 {
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if min == 0 {
                                    EngineDiagnosticSeverity::Info
                                } else {
                                    EngineDiagnosticSeverity::Error
                                },
                                class: if min == 0 {
                                    EngineDiagnosticClass::Normalization
                                } else {
                                    EngineDiagnosticClass::Broken
                                },
                                message: format!(
                                    "auto-skipped empty grid select for {:?} on {:?} with filter {:?} (requested min={}, max={})",
                                    reason, source_pile, filter, min, max
                                ),
                            },
                        );
                        return true;
                    }

                    if available == 1 && min == 1 && max == 1 && !can_cancel {
                        let _ = pending_choices::handle_grid_select(
                            engine_state,
                            combat_state,
                            &candidate_uuids,
                            source_pile,
                            1,
                            1,
                            false,
                            reason,
                            ClientInput::SubmitGridSelect(vec![candidate_uuids[0]]),
                        );
                        return true;
                    }

                    let min_cards = min.min(available);
                    let max_cards = max.min(available);
                    if min_cards != min || max_cards != max {
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if min > available {
                                    EngineDiagnosticSeverity::Warning
                                } else {
                                    EngineDiagnosticSeverity::Info
                                },
                                class: if min > available {
                                    EngineDiagnosticClass::Suspicious
                                } else {
                                    EngineDiagnosticClass::Normalization
                                },
                                message: format!(
                                    "normalized grid select for {:?} on {:?} with filter {:?} from min/max {}/{} to {}/{} because only {} candidates remain",
                                    reason, source_pile, filter, min, max, min_cards, max_cards, available
                                ),
                            },
                        );
                    }

                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::GridSelect {
                        source_pile,
                        candidate_uuids,
                        min_cards,
                        max_cards,
                        can_cancel,
                        reason,
                    });
                    return true;
                }
                Action::SuspendForDiscovery {
                    colorless,
                    card_type,
                    cost_for_turn,
                } => {
                    // Generate 3 unique random cards from pool, filtered by card_type
                    // Java: DiscoveryAction.generateCardChoices(type) or
                    // generateColorlessCardChoices() -> 3 unique cards.
                    let mut pool: Vec<crate::content::cards::CardId> = if colorless {
                        combat_state.colorless_combat_pool()
                    } else {
                        let mut class_pool = Vec::new();
                        for &rarity in &[
                            crate::content::cards::CardRarity::Common,
                            crate::content::cards::CardRarity::Uncommon,
                            crate::content::cards::CardRarity::Rare,
                        ] {
                            class_pool
                                .extend(crate::content::cards::ironclad_pool_for_rarity(rarity));
                        }
                        class_pool
                    };

                    if let Some(ct) = card_type {
                        pool.retain(|&id| {
                            crate::content::cards::get_card_definition(id).card_type == ct
                        });
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !pool.is_empty() {
                        let idx = combat_state
                            .rng
                            .card_random_rng
                            .random(pool.len() as i32 - 1)
                            as usize;
                        let id = pool[idx];
                        if !cards.contains(&id) {
                            cards.push(id);
                        }
                    }
                    // Store cost_for_turn in the first element of limbo as a signal
                    // (it will be applied when the choice is resolved)
                    combat_state.turn.set_discovery_cost_for_turn(cost_for_turn);
                    update_monster_intents(combat_state);
                    *engine_state =
                        EngineState::PendingChoice(PendingChoice::DiscoverySelect(cards));
                    return true;
                }
                Action::SuspendForCardReward {
                    pool,
                    destination,
                    can_skip,
                } => {
                    // Generate 3 unique random cards from pool
                    use crate::runtime::action::CardRewardPool;
                    let mut card_pool: Vec<crate::content::cards::CardId> = Vec::new();
                    match pool {
                        CardRewardPool::ClassAll => {
                            // Java: returnTrulyRandomCardInCombat() -> all class cards.
                            for &rarity in &[
                                crate::content::cards::CardRarity::Common,
                                crate::content::cards::CardRarity::Uncommon,
                                crate::content::cards::CardRarity::Rare,
                            ] {
                                for &id in crate::content::cards::ironclad_pool_for_rarity(rarity) {
                                    card_pool.push(id);
                                }
                            }
                        }
                        CardRewardPool::Colorless => {
                            // Java: returnTrulyRandomColorlessCardInCombat()
                            for &id in crate::content::cards::COLORLESS_UNCOMMON_POOL {
                                let def = crate::content::cards::get_card_definition(id);
                                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                                    card_pool.push(id);
                                }
                            }
                            for &id in crate::content::cards::COLORLESS_RARE_POOL {
                                let def = crate::content::cards::get_card_definition(id);
                                if !def.tags.contains(&crate::content::cards::CardTag::Healing) {
                                    card_pool.push(id);
                                }
                            }
                        }
                    }
                    let mut cards = Vec::new();
                    while cards.len() < 3 && !card_pool.is_empty() {
                        let idx = combat_state
                            .rng
                            .card_random_rng
                            .random(card_pool.len() as i32 - 1)
                            as usize;
                        let id = card_pool[idx];
                        if !cards.contains(&id) {
                            cards.push(id);
                        }
                    }
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::CardRewardSelect {
                        cards,
                        destination,
                        can_skip,
                    });
                    return true;
                }
                Action::SuspendForStanceChoice => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::StanceChoice);
                    return true;
                }

                Action::FleeCombat => {
                    // Java SmokeBomb does not instantly jump to rewards. It flips the
                    // room/player into an escaping state, then the combat UI lingers
                    // through end-of-turn processing before rewards appear.
                    combat_state.turn.mark_player_escaping();
                    combat_state.turn.clear_escape_pending_reward();
                    return true;
                }
                _ => {
                    super::action_handlers::execute_action(next_action, combat_state);
                }
            }
            if combat_state.entities.player.current_hp <= 0 {
                combat_state.clear_pending_actions();
                *engine_state = EngineState::GameOver(RunResult::Defeat);
                return false;
            }
            if matches!(engine_state, EngineState::PendingChoice(_)) {
                return true;
            }
        } else {
            // Queue is empty; decide next state based on combat phase.
            match combat_state.turn.current_phase {
                CombatPhase::PlayerTurn => {
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::CombatPlayerTurn;
                }
                CombatPhase::TurnTransition => {
                    // === TURN TRANSITION: end of player turn -> enemy turn -> new player turn ===

                    // 1. Discard hand (respecting Retain and RunicPyramid)
                    let has_runic_pyramid = combat_state
                        .entities
                        .player
                        .has_relic(crate::content::relics::RelicId::RunicPyramid);
                    if has_runic_pyramid {
                        // RunicPyramid: retain all cards -> skip discard entirely.
                    } else {
                        let mut retained = Vec::new();
                        let mut discarded = Vec::new();
                        for card in combat_state.zones.hand.drain(..) {
                            // Check for actual retain: card.retain_override
                            if card.retain_override == Some(true) {
                                retained.push(card);
                            } else {
                                discarded.push(card);
                            }
                        }
                        // Java end-of-turn discard repeatedly removes hand.getTopCard(),
                        // so the surviving non-retained hand is discarded from top to bottom.
                        discarded.reverse();
                        combat_state.zones.discard_pile.extend(discarded);
                        combat_state.zones.hand = retained;
                    }

                    // Smoke Bomb escape path: Java leaves an intermediate combat
                    // snapshot after end-of-turn effects and discarding, but before
                    // any monster actions or player turn refresh. Emit that state
                    // first, then finish escaping on the following tick.
                    if combat_state.turn.counters.player_escaping {
                        if !combat_state.turn.counters.victory_triggered {
                            combat_state.turn.mark_victory_triggered();
                            resolve_victory_hooks_immediately(combat_state);
                            combat_state.turn.mark_escape_pending_reward();
                            *engine_state = EngineState::CombatProcessing;
                            return true;
                        }
                        if combat_state.turn.counters.escape_pending_reward {
                            *engine_state = EngineState::RewardScreen(
                                crate::rewards::state::RewardState::new(),
                            );
                            return false;
                        }
                        combat_state.turn.mark_escape_pending_reward();
                        *engine_state = EngineState::CombatProcessing;
                        return true;
                    }

                    // 1.5 === MONSTER PRE-TURN LOGIC ===
                    // Java: MonsterStartTurnAction calls MonsterGroup.applyPreTurnLogic() -> clears block, etc.
                    let alive_for_pre: Vec<_> = combat_state
                        .entities
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();

                    for mid in &alive_for_pre {
                        // 1. Clear block
                        let has_barricade = crate::content::powers::store::has_power(
                            combat_state,
                            *mid,
                            crate::content::powers::PowerId::Barricade,
                        );
                        if let Some(monster) = combat_state
                            .entities
                            .monsters
                            .iter_mut()
                            .find(|m| m.id == *mid)
                        {
                            if !has_barricade {
                                monster.block = 0;
                            }
                        }
                        // 2. Fire Start of Turn Powers (e.g. Poison tick, Flight regain)
                        for power in
                            &crate::content::powers::store::powers_snapshot_for(combat_state, *mid)
                        {
                            let hook_actions = crate::content::powers::resolve_power_at_turn_start(
                                power.power_type,
                                combat_state,
                                *mid,
                                power.amount,
                            );
                            for a in hook_actions {
                                combat_state.queue_action_back(a);
                            }
                        }
                    }
                    // 3. Drain pre-turn actions instantly
                    while let Some(action) = combat_state.pop_next_action() {
                        super::action_handlers::execute_action(action, combat_state);
                        if combat_state.entities.player.current_hp <= 0 {
                            combat_state.clear_pending_actions();
                            *engine_state = EngineState::GameOver(RunResult::Defeat);
                            return false;
                        }
                    }

                    // 2. Execute each alive monster's turn (player block absorbs damage)
                    combat_state.begin_monster_turn();
                    let mut monster_snapshots = Vec::new();
                    let mut dead_ids = Vec::new();
                    for m in &combat_state.entities.monsters {
                        if m.is_dying || m.is_escaped {
                            dead_ids.push(m.id);
                        } else {
                            monster_snapshots.push(m.clone());
                        }
                    }
                    for id in dead_ids {
                        store::remove_entity_powers(combat_state, id);
                    }
                    for monster in &monster_snapshots {
                        // Reset monster Invincible limit
                        let _ = store::with_power_mut(
                            combat_state,
                            monster.id,
                            crate::content::powers::PowerId::Invincible,
                            |inv| {
                                inv.amount = inv.extra_data;
                            },
                        );
                        let actions =
                            crate::content::monsters::resolve_monster_turn(combat_state, monster);
                        for action in actions {
                            combat_state.queue_action_back(action);
                        }
                        // Drain this monster's turn actions
                        while let Some(action) = combat_state.pop_next_action() {
                            super::action_handlers::execute_action(action, combat_state);
                            if combat_state.entities.player.current_hp <= 0 {
                                combat_state.clear_pending_actions();
                                *engine_state = EngineState::GameOver(RunResult::Defeat);
                                return false;
                            }
                        }
                    }
                    // (Monster actions now drained per-monster inside the for-loop above)

                    // 2.3 === COLLECTIVE END OF TURN ===
                    // Java: MonsterGroup.applyEndOfTurnPowers() calls p.atEndOfTurn(false) across all alive monsters.
                    let alive_monsters_for_end_turn: Vec<_> = combat_state
                        .entities
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in &alive_monsters_for_end_turn {
                        for power in
                            &crate::content::powers::store::powers_snapshot_for(combat_state, *mid)
                        {
                            let hook_actions = crate::content::powers::resolve_power_at_end_of_turn(
                                power,
                                combat_state,
                                *mid,
                            );
                            for a in hook_actions {
                                combat_state.queue_action_back(a);
                            }
                        }
                    }
                    // Drain atEndOfTurn collective actions
                    while let Some(action) = combat_state.pop_next_action() {
                        super::action_handlers::execute_action(action, combat_state);
                        if combat_state.entities.player.current_hp <= 0 {
                            combat_state.clear_pending_actions();
                            *engine_state = EngineState::GameOver(RunResult::Defeat);
                            return false;
                        }
                    }

                    // 2.5 === FULL ROUND END ===
                    // Java: applyEndOfTurnPowers() calls p.atEndOfRound() on player and all monsters
                    // Player powers:
                    for power in
                        &crate::content::powers::store::powers_snapshot_for(combat_state, 0)
                    {
                        let hook_actions = crate::content::powers::resolve_power_at_end_of_round(
                            power.power_type,
                            combat_state,
                            0,
                            power.amount,
                            power.just_applied,
                        );
                        for a in hook_actions {
                            combat_state.queue_action_back(a);
                        }
                    }
                    // Monster powers:
                    let alive_monsters: Vec<_> = combat_state
                        .entities
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped)
                        .map(|m| m.id)
                        .collect();
                    for mid in alive_monsters {
                        for power in
                            &crate::content::powers::store::powers_snapshot_for(combat_state, mid)
                        {
                            let hook_actions =
                                crate::content::powers::resolve_power_at_end_of_round(
                                    power.power_type,
                                    combat_state,
                                    mid,
                                    power.amount,
                                    power.just_applied,
                                );
                            for a in hook_actions {
                                combat_state.queue_action_back(a);
                            }
                        }
                    }
                    // Drain at_end_of_round actions
                    while let Some(action) = combat_state.pop_next_action() {
                        super::action_handlers::execute_action(action, combat_state);
                    }

                    // Clear all just_applied flags globally at the end of the round!
                    store::clear_just_applied_flags(combat_state);

                    // If player died during monster turn, immediate game over
                    if combat_state.entities.player.current_hp <= 0 {
                        combat_state.clear_pending_actions();
                        *engine_state = EngineState::GameOver(RunResult::Defeat);
                        return false;
                    }

                    // 3. (Intent rolling is handled by Action::RollMonsterMove in the queue)

                    // === NEW PLAYER TURN START ===
                    // 4. Clear player block (Barricade: keep all, Calipers: retain up to 15)
                    let has_barricade = crate::content::powers::store::has_power(
                        combat_state,
                        0,
                        crate::content::powers::PowerId::Barricade,
                    );
                    if !has_barricade {
                        let has_calipers = !combat_state
                            .entities
                            .player
                            .relic_buses
                            .on_calculate_block_retained
                            .is_empty();
                        if has_calipers {
                            let retained =
                                crate::content::relics::hooks::on_calculate_block_retained(
                                    combat_state,
                                    combat_state.entities.player.block,
                                );
                            combat_state.entities.player.block = retained;
                        } else {
                            combat_state.entities.player.block = 0;
                        }
                    }

                    // (Monster blocks are cleared per-monster at the start of each monster's turn above)

                    combat_state.begin_next_player_turn();
                    // Reset per-turn relic counters (OrangePellets)
                    for relic in combat_state.entities.player.relics.iter_mut() {
                        match relic.id {
                            crate::content::relics::RelicId::OrangePellets => relic.counter = 0,
                            _ => {}
                        }
                    }

                    // Reset player Invincible limit
                    let _ = store::with_power_mut(
                        combat_state,
                        0,
                        crate::content::powers::PowerId::Invincible,
                        |inv| {
                            inv.amount = inv.extra_data;
                        },
                    );

                    // 8. at_turn_start relic hooks (AncientTeaSet, HappyFlower, etc.)
                    // Java: relics fire atTurnStart BEFORE draw cards
                    let turn_start_actions =
                        crate::content::relics::hooks::at_turn_start(combat_state);
                    combat_state.queue_actions(turn_start_actions);

                    // 8.1. at_turn_start power hooks (Player)
                    // Java: player.applyStartOfTurnPowers()
                    for power in
                        &crate::content::powers::store::powers_snapshot_for(combat_state, 0)
                    {
                        let pa = crate::content::powers::resolve_power_at_turn_start(
                            power.power_type,
                            combat_state,
                            0,
                            power.amount,
                        );
                        for a in pa {
                            combat_state.queue_action_back(a);
                        }
                    }

                    // 8.2. applyStartOfTurnOrbs
                    let orb_actions = crate::content::orbs::hooks::at_turn_start(combat_state);
                    combat_state.queue_actions(orb_actions);

                    // 8.3. applyStartOfTurnCards (For Curses in hand)
                    let card_actions =
                        crate::content::cards::hooks::at_turn_start_in_hand(combat_state);
                    combat_state.queue_actions(card_actions);

                    // 9. Draw cards (default 5, reduced by DrawReduction power)
                    // Java consumes AbstractDungeon.player.gameHandSize here.
                    // Rust still derives the same result locally until a broader
                    // draw-target state is justified.
                    let draw_count = compute_player_turn_start_draw_count(combat_state);
                    if draw_count > 0 {
                        combat_state.queue_action_back(Action::DrawCards(draw_count as u32));
                    }
                    combat_state.queue_action_back(Action::PostDrawTrigger);

                    *engine_state = EngineState::CombatProcessing;
                }
                CombatPhase::MonsterTurn => {
                    // Monster actions drained, transition to player turn start
                    combat_state.turn.begin_player_phase();
                    *engine_state = EngineState::CombatProcessing;
                }
            }
            if combat_state.entities.player.current_hp <= 0 {
                *engine_state = EngineState::GameOver(RunResult::Defeat);
                return false;
            }
            return true;
        }
    }

    if combat_state.entities.monsters.iter().all(|m| {
        if m.is_escaped {
            return true;
        }
        if m.half_dead {
            return false;
        }
        if m.current_hp > 0 {
            return false;
        }
        let is_pending_rebirth = crate::content::powers::store::powers_for(combat_state, m.id)
            .is_some_and(|powers| {
                powers.iter().any(|p| {
                    matches!(
                        p.power_type,
                        crate::content::powers::PowerId::Regrow
                            | crate::content::powers::PowerId::Unawakened
                    )
                })
            });
        !is_pending_rebirth
    }) {
        if !combat_state.turn.counters.victory_triggered {
            combat_state.turn.mark_victory_triggered();
            resolve_victory_hooks_immediately(combat_state);
        }

        // Java does not cut off queued onUseCard / onDeath aftermath when the last monster dies.
        // Finish draining any already-queued actions (e.g. Rage block, relic hooks, death hooks)
        // before transitioning to rewards.
        if !combat_state.has_pending_actions()
            && combat_state.zones.limbo.is_empty()
            && combat_state.zones.queued_cards.is_empty()
        {
            *engine_state = EngineState::RewardScreen(crate::rewards::state::RewardState::new());
            return false;
        }
        *engine_state = EngineState::CombatProcessing;
    }

    true
}

fn handle_player_turn_input(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    cmd: ClientInput,
) -> Result<(), &'static str> {
    match cmd {
        ClientInput::PlayCard { card_index, target } => {
            crate::engine::action_handlers::cards::handle_play_card_from_hand(
                card_index,
                target,
                combat_state,
            )
        }

        ClientInput::UsePotion {
            potion_index,
            mut target,
        } => {
            let potion = combat_state
                .entities
                .potions
                .get(potion_index)
                .and_then(|p| p.as_ref())
                .ok_or("Potion index out of range")?;
            let def = crate::content::potions::get_potion_definition(potion.id);
            target = targeting::resolve_target_request(
                combat_state,
                targeting::validation_for_potion_target(def.target_required),
                target,
            )?;
            // Queue UsePotion action; action_handlers.rs performs the actual work.
            combat_state.queue_action_back(Action::UsePotion {
                slot: potion_index,
                target: target.map(|t| t as usize),
            });
            Ok(())
        }

        ClientInput::DiscardPotion(slot) => {
            combat_state.queue_action_back(Action::DiscardPotion { slot });
            Ok(())
        }

        ClientInput::EndTurn => {
            // Queue end-of-turn processing
            // 1. EndTurnTrigger handles in-hand card effects (Burn, Decay, ethereal exhaust, etc.)
            combat_state.queue_action_back(Action::EndTurnTrigger);
            // 2. Relic at_end_of_turn hooks (Orichalcum, CloakClasp, ArtOfWar, etc.)
            let end_turn_relic_actions =
                crate::content::relics::hooks::at_end_of_turn(combat_state);
            combat_state.queue_actions(end_turn_relic_actions);
            // 3. Transition: the engine loop will detect CombatProcessing and handle
            //    discarding hand, applying power at_end_of_turn, enemy turns, draw, etc.
            *engine_state = EngineState::CombatProcessing;
            combat_state.begin_turn_transition();
            Ok(())
        }

        _ => Err("Invalid input for player turn"),
    }
}

fn resolve_pending_choice(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: ClientInput,
) -> Result<(), &'static str> {
    let choice = if let EngineState::PendingChoice(c) = engine_state {
        c.clone()
    } else {
        return Err("Not in a pending choice state");
    };

    match choice {
        PendingChoice::ScrySelect {
            cards,
            card_uuids: _,
        } => pending_choices::handle_scry(engine_state, combat_state, cards.len(), input),
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards: count,
            max_cards: _,
            can_cancel: cancellable,
            reason,
        } => pending_choices::handle_hand_select(
            engine_state,
            combat_state,
            &candidate_uuids,
            count as usize,
            false,
            cancellable,
            reason,
            input,
        ),
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => pending_choices::handle_grid_select(
            engine_state,
            combat_state,
            &candidate_uuids,
            source_pile,
            min_cards,
            max_cards,
            can_cancel,
            reason,
            input,
        ),
        PendingChoice::DiscoverySelect(ref cards) => {
            // Player picks one card from the discovery options
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                if idx < cards.len() {
                    let card_id = cards[idx];
                    let uuid = 50000
                        + combat_state.zones.hand.len() as u32
                        + combat_state.zones.discard_pile.len() as u32;
                    let mut card = crate::runtime::combat::CombatCard::new(card_id, uuid);
                    // Apply cost override from the SuspendForDiscovery action
                    if let Some(cost) = combat_state.turn.take_discovery_cost_for_turn() {
                        card.cost_for_turn = Some(cost);
                    }
                    if combat_state.zones.hand.len() < 10 {
                        combat_state.zones.hand.push(card);
                    } else {
                        combat_state.zones.discard_pile.push(card);
                    }
                    *engine_state = EngineState::CombatProcessing;
                    return Ok(());
                }
            }
            Err("Invalid discovery choice")
        }
        PendingChoice::CardRewardSelect {
            ref cards,
            destination,
            can_skip,
        } => {
            // Player picks one card from the reward options, or Cancel if can_skip
            match input {
                ClientInput::SubmitDiscoverChoice(idx) => {
                    if idx < cards.len() {
                        let card_id = cards[idx];
                        let uuid = 50000
                            + combat_state.zones.hand.len() as u32
                            + combat_state.zones.discard_pile.len() as u32
                            + combat_state.zones.draw_pile.len() as u32;
                        let card = crate::runtime::combat::CombatCard::new(card_id, uuid);
                        match destination {
                            crate::runtime::action::CardDestination::Hand => {
                                // Java ChooseOneColorless: hand (or discard if full)
                                if combat_state.zones.hand.len() < 10 {
                                    combat_state.zones.hand.push(card);
                                } else {
                                    combat_state.zones.discard_pile.push(card);
                                }
                            }
                            crate::runtime::action::CardDestination::DrawPileRandom => {
                                // Java CodexAction: add to draw pile at random position
                                if combat_state.zones.draw_pile.is_empty() {
                                    combat_state.zones.draw_pile.push(card);
                                } else {
                                    let pos = combat_state
                                        .rng
                                        .card_random_rng
                                        .random(combat_state.zones.draw_pile.len() as i32)
                                        as usize;
                                    combat_state
                                        .zones
                                        .draw_pile
                                        .insert(pos.min(combat_state.zones.draw_pile.len()), card);
                                }
                            }
                        }
                        *engine_state = EngineState::CombatProcessing;
                        Ok(())
                    } else {
                        Err("Invalid card reward choice index")
                    }
                }
                ClientInput::Cancel if can_skip => {
                    // Java CodexAction: canSkip=true -> player can skip picking.
                    *engine_state = EngineState::CombatProcessing;
                    Ok(())
                }
                _ => Err("Invalid input for card reward selection"),
            }
        }
        PendingChoice::StanceChoice => {
            // Player picks 0=Wrath, 1=Calm
            if let ClientInput::SubmitDiscoverChoice(idx) = input {
                let stance = match idx {
                    0 => "Wrath",
                    1 => "Calm",
                    _ => return Err("Invalid stance choice (expected 0=Wrath or 1=Calm)"),
                };
                combat_state.queue_action_back(Action::EnterStance(stance.to_string()));
                *engine_state = EngineState::CombatProcessing;
                Ok(())
            } else {
                Err("Expected SubmitDiscoverChoice for stance selection")
            }
        }
    }
}

fn hand_select_candidates(
    combat_state: &CombatState,
    filter: crate::state::HandSelectFilter,
) -> Vec<u32> {
    combat_state
        .zones
        .hand
        .iter()
        .filter(|card| hand_candidate_matches(card, filter))
        .map(|card| card.uuid)
        .collect()
}

fn hand_candidate_matches(
    card: &crate::runtime::combat::CombatCard,
    filter: crate::state::HandSelectFilter,
) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    match filter {
        crate::state::HandSelectFilter::Any => true,
        crate::state::HandSelectFilter::Upgradeable => {
            (card.id == crate::content::cards::CardId::SearingBlow || card.upgrades == 0)
                && def.card_type != crate::content::cards::CardType::Status
                && def.card_type != crate::content::cards::CardType::Curse
        }
        crate::state::HandSelectFilter::AttackOrPower => {
            matches!(
                def.card_type,
                crate::content::cards::CardType::Attack | crate::content::cards::CardType::Power
            )
        }
    }
}

fn grid_select_candidates(
    combat_state: &CombatState,
    source_pile: crate::state::PileType,
    filter: crate::state::GridSelectFilter,
) -> Vec<u32> {
    let pile: &[crate::runtime::combat::CombatCard] = match source_pile {
        crate::state::PileType::Draw => &combat_state.zones.draw_pile,
        crate::state::PileType::Discard => &combat_state.zones.discard_pile,
        crate::state::PileType::Exhaust => &combat_state.zones.exhaust_pile,
        crate::state::PileType::Hand => &combat_state.zones.hand,
        crate::state::PileType::Limbo => &combat_state.zones.limbo,
        crate::state::PileType::MasterDeck => &[],
    };

    pile.iter()
        .filter(|card| grid_candidate_matches(card, filter))
        .map(|card| card.uuid)
        .collect()
}

fn grid_candidate_matches(
    card: &crate::runtime::combat::CombatCard,
    filter: crate::state::GridSelectFilter,
) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    match filter {
        crate::state::GridSelectFilter::Any => true,
        crate::state::GridSelectFilter::NonExhume => {
            card.id != crate::content::cards::CardId::Exhume
        }
        crate::state::GridSelectFilter::Skill => {
            def.card_type == crate::content::cards::CardType::Skill
        }
        crate::state::GridSelectFilter::Attack => {
            def.card_type == crate::content::cards::CardType::Attack
        }
    }
}

pub fn queue_actions(
    queue: &mut std::collections::VecDeque<Action>,
    actions: SmallVec<[ActionInfo; 4]>,
) {
    let mut to_bottom = vec![];
    let mut to_front = vec![];

    for a in actions {
        match a.insertion_mode {
            crate::runtime::action::AddTo::Top => to_front.push(a.action),
            crate::runtime::action::AddTo::Bottom => to_bottom.push(a.action),
        }
    }

    // Top actions: push in reverse so first item ends up at front
    for action in to_front.into_iter().rev() {
        queue.push_front(action);
    }
    for action in to_bottom {
        queue.push_back(action);
    }
}

/// Java: AbstractMonster.applyPowers()
/// Interrogates each living monster, extracts base Damage from its `current_intent`,
/// runs it through `calculate_monster_damage()`, and stores the mutated result in
/// `intent_preview_damage`.
/// This is purely for updating the UI visually before user interaction.
pub fn update_monster_intents(combat_state: &mut CombatState) {
    let alive_monsters: Vec<_> = combat_state
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped)
        .map(|m| m.id)
        .collect();

    for mid in alive_monsters {
        let mut new_intent_preview_damage = 0;

        // Temporarily extract current intent (cannot borrow mutably directly since we need state)
        if let Some(monster) = combat_state.entities.monsters.iter().find(|m| m.id == mid) {
            if let crate::runtime::combat::Intent::Attack { damage, .. }
            | crate::runtime::combat::Intent::AttackBuff { damage, .. }
            | crate::runtime::combat::Intent::AttackDebuff { damage, .. }
            | crate::runtime::combat::Intent::AttackDefend { damage, .. } =
                monster.current_intent
            {
                // `damage` in the enum represents the pure base damage
                new_intent_preview_damage =
                    crate::content::powers::calculate_monster_damage(damage, mid, 0, combat_state);
            } else {
                new_intent_preview_damage = -1; // Not an attack intent
            }
        }

        // Apply it back
        if let Some(monster) = combat_state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == mid)
        {
            if new_intent_preview_damage != -1 {
                monster.intent_preview_damage = new_intent_preview_damage;
            } else {
                monster.intent_preview_damage = 0;
            }
        }
    }
}

/// Run tick_engine until it returns to CombatPlayerTurn or game over.
pub fn tick_until_stable_turn(
    es: &mut EngineState,
    cs: &mut CombatState,
    input: ClientInput,
) -> bool {
    // First tick with input
    let alive = tick_engine(es, cs, Some(input));
    if !alive {
        return false;
    }

    // After any input: engine stays at CombatPlayerTurn but actions may be queued.
    // We need to force CombatProcessing to drain the action queue.
    if *es == EngineState::CombatPlayerTurn
        && (cs.has_pending_actions() || !cs.zones.queued_cards.is_empty())
    {
        *es = EngineState::CombatProcessing;
    }

    // Keep ticking until we're back at PlayerTurn (waiting for input), or we're in a PendingChoice
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing => {}
            EngineState::PendingChoice(_) => break, // Would need user input
            EngineState::GameOver(_) => return false,
            _ => break, // RewardScreen, etc.
        }
        let alive = tick_engine(es, cs, None);
        if !alive {
            return false;
        }
        iterations += 1;
        if iterations > 1000 {
            record_engine_diagnostic(
                cs,
                EngineDiagnostic {
                    severity: EngineDiagnosticSeverity::Warning,
                    class: EngineDiagnosticClass::Suspicious,
                    message: "tick loop exceeded 1000 iterations".to_string(),
                },
            );
            break;
        }
    }
    true
}
