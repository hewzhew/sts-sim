use crate::content::powers::store;
use crate::runtime::action::{Action, ActionInfo};
use crate::runtime::combat::{CombatPhase, CombatState};
use crate::state::core::{
    ClientInput, DiscoveryChoiceState, EngineState, PendingChoice, RunResult,
};
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

pub(crate) fn compute_player_turn_start_draw_count(combat_state: &CombatState) -> i32 {
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

fn class_combat_card_pool(player_class: &str) -> Vec<crate::content::cards::CardId> {
    let mut class_pool = Vec::new();
    for &rarity in &[
        crate::content::cards::CardRarity::Common,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Rare,
    ] {
        class_pool.extend(crate::engine::campfire_handler::card_pool_for_class(
            player_class,
            rarity,
        ));
    }
    class_pool
}

fn discovery_card_pool(
    combat_state: &CombatState,
    colorless: bool,
    card_type: Option<crate::content::cards::CardType>,
) -> Vec<crate::content::cards::CardId> {
    let mut pool = if colorless {
        combat_state.colorless_combat_pool()
    } else {
        class_combat_card_pool(combat_state.meta.player_class)
    };
    if let Some(ct) = card_type {
        pool.retain(|&id| crate::content::cards::get_card_definition(id).card_type == ct);
    }
    pool
}

fn generate_discovery_choices(
    combat_state: &mut CombatState,
    colorless: bool,
    card_type: Option<crate::content::cards::CardType>,
) -> Vec<crate::content::cards::CardId> {
    let pool = discovery_card_pool(combat_state, colorless, card_type);
    let mut cards = Vec::new();
    while cards.len() < 3 && !pool.is_empty() {
        let idx = combat_state
            .rng
            .card_random_rng
            .random(pool.len() as i32 - 1) as usize;
        let id = pool[idx];
        if !cards.contains(&id) {
            cards.push(id);
        }
    }
    cards
}

fn resolve_victory_hooks_immediately(combat_state: &mut CombatState) {
    let mut actions = crate::content::relics::hooks::on_victory(combat_state);
    for power in crate::content::powers::store::powers_snapshot_for(combat_state, 0) {
        let power_actions = crate::content::powers::resolve_power_on_victory(
            power.power_type,
            combat_state,
            0,
            power.amount,
        );
        for action in power_actions {
            actions.push(ActionInfo {
                action,
                insertion_mode: crate::runtime::action::AddTo::Bottom,
            });
        }
    }
    if actions.is_empty() {
        return;
    }

    combat_state.queue_actions(actions);
    while let Some(action) = combat_state.pop_next_action() {
        crate::engine::action_handlers::execute_action(action, combat_state);
        combat_state.ensure_flush_next_queued_card();
    }
}

fn settle_victory_if_ready(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
) -> Option<bool> {
    if !combat_state.entities.monsters.iter().all(|m| {
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
        return None;
    }

    if !combat_state.turn.counters.victory_triggered {
        combat_state.turn.mark_victory_triggered();
        resolve_victory_hooks_immediately(combat_state);
    }

    if !combat_state.has_pending_actions()
        && combat_state.zones.queued_cards.is_empty()
        && !combat_state.zones.limbo.is_empty()
    {
        let limbo_cards = std::mem::take(&mut combat_state.zones.limbo);
        for card in limbo_cards {
            combat_state.add_card_to_discard_pile_top(card);
        }
    }

    // Java does not cut off queued onUseCard / onDeath aftermath when the last monster dies.
    // Finish draining any already-queued actions (e.g. Rage block, relic hooks, death hooks)
    // before transitioning to rewards.
    if !combat_state.has_pending_actions()
        && combat_state.zones.limbo.is_empty()
        && combat_state.zones.queued_cards.is_empty()
    {
        *engine_state = EngineState::RewardScreen(crate::rewards::state::RewardState::new());
        return Some(false);
    }
    *engine_state = EngineState::CombatProcessing;
    Some(true)
}

pub fn is_smoke_escape_stable_boundary(
    engine_state: &EngineState,
    combat_state: &CombatState,
) -> bool {
    matches!(engine_state, EngineState::CombatProcessing)
        && matches!(combat_state.turn.current_phase, CombatPhase::TurnTransition)
        && combat_state.runtime.combat_smoked
        && combat_state.turn.counters.player_escaping
        && combat_state.turn.counters.victory_triggered
        && combat_state.turn.counters.escape_pending_reward
        && !combat_state.has_pending_actions()
        && combat_state.zones.queued_cards.is_empty()
        && combat_state.zones.limbo.is_empty()
}

fn discard_hand_for_turn_transition(combat_state: &mut CombatState) {
    let has_runic_pyramid = combat_state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::RunicPyramid);
    if has_runic_pyramid {
        // RunicPyramid keeps the hand, but Java RestoreRetainedCardsAction
        // still clears one-turn retain flags created by RetainCardsAction.
        for card in &mut combat_state.zones.hand {
            if card.retain_override == Some(true) {
                card.retain_override = None;
            }
        }
        return;
    }

    let mut retained = Vec::new();
    let mut discarded = Vec::new();
    for mut card in combat_state.zones.hand.drain(..) {
        if card.retain_override == Some(true) {
            card.retain_override = None;
            retained.push(card);
        } else {
            discarded.push(card);
        }
    }

    // Java end-of-turn discard repeatedly removes hand.getTopCard(), so the
    // surviving non-retained hand is discarded from top to bottom.
    discarded.reverse();
    for card in discarded {
        combat_state.add_card_to_discard_pile_top(card);
    }
    combat_state.zones.hand = retained;
}

pub fn tick_engine(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: Option<ClientInput>,
) -> bool {
    // Phase 1: pending choice overrides
    if let EngineState::PendingChoice(_) = engine_state {
        if let Some(cmd) = input {
            match resolve_pending_choice(engine_state, combat_state, cmd) {
                Ok(()) => {
                    if !matches!(engine_state, EngineState::PendingChoice(_)) {
                        *engine_state = EngineState::CombatProcessing;
                    }
                }
                Err(err) => record_engine_diagnostic(
                    combat_state,
                    EngineDiagnostic {
                        severity: EngineDiagnosticSeverity::Error,
                        class: EngineDiagnosticClass::Broken,
                        message: format!("Rejected pending-choice input: {err}"),
                    },
                ),
            }
        }
        return true;
    }

    // Phase 2: process input
    if *engine_state == EngineState::CombatPlayerTurn {
        if let Some(cmd) = input {
            match handle_player_turn_input(engine_state, combat_state, cmd) {
                Ok(()) => {
                    // After a card play, actions (damage, block, etc.) are queued.
                    // Transition to CombatProcessing to drain the queue.
                    if combat_state.has_pending_actions()
                        || !combat_state.zones.queued_cards.is_empty()
                    {
                        *engine_state = EngineState::CombatProcessing;
                    }
                }
                Err(err) => {
                    record_engine_diagnostic(
                        combat_state,
                        EngineDiagnostic {
                            severity: EngineDiagnosticSeverity::Error,
                            class: EngineDiagnosticClass::Broken,
                            message: format!("Rejected player-turn input: {err}"),
                        },
                    );
                    return true;
                }
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
                        let legal_empty_fizzle = min == 0
                            || hand_select_can_fizzle_when_empty(reason)
                            || (filter == crate::state::HandSelectFilter::Upgradeable
                                && reason == crate::state::HandSelectReason::Upgrade);
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if legal_empty_fizzle {
                                    EngineDiagnosticSeverity::Info
                                } else {
                                    EngineDiagnosticSeverity::Error
                                },
                                class: if legal_empty_fizzle {
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
                    let candidate_uuids =
                        grid_select_candidates(combat_state, source_pile, filter, reason);
                    let available = candidate_uuids.len() as u8;
                    if available == 0 {
                        let legal_empty_fizzle = min == 0
                            || (source_pile == crate::state::PileType::Discard
                                && matches!(
                                    reason,
                                    crate::state::GridSelectReason::DiscardToHand
                                        | crate::state::GridSelectReason::DiscardToHandNoCostChange
                                ));
                        record_engine_diagnostic(
                            combat_state,
                            EngineDiagnostic {
                                severity: if legal_empty_fizzle {
                                    EngineDiagnosticSeverity::Info
                                } else {
                                    EngineDiagnosticSeverity::Error
                                },
                                class: if legal_empty_fizzle {
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
                    can_skip,
                } => {
                    // Java DiscoveryAction.generateCardChoices(type) /
                    // generateColorlessCardChoices() samples three unique card
                    // IDs when the screen opens.
                    let cards = generate_discovery_choices(combat_state, colorless, card_type);
                    combat_state.turn.set_discovery_cost_for_turn(cost_for_turn);
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::PendingChoice(PendingChoice::DiscoverySelect(
                        DiscoveryChoiceState {
                            cards,
                            colorless,
                            card_type,
                            can_skip,
                        },
                    ));
                    return true;
                }
                Action::SuspendForCardReward {
                    pool,
                    destination,
                    can_skip,
                    skip_if_monsters_basically_dead,
                } => {
                    if skip_if_monsters_basically_dead
                        && combat_state.are_monsters_basically_dead_java()
                    {
                        return true;
                    }
                    // Generate 3 unique random cards from pool
                    use crate::runtime::action::CardRewardPool;
                    let mut card_pool: Vec<crate::content::cards::CardId> = Vec::new();
                    match pool {
                        CardRewardPool::ClassAll => {
                            // Java: returnTrulyRandomCardInCombat() -> all cards for
                            // the current player's class, not always Ironclad.
                            card_pool
                                .extend(class_combat_card_pool(combat_state.meta.player_class));
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

                Action::Scry(amount) => {
                    let amount = crate::content::relics::hooks::on_scry(combat_state, amount);
                    if amount == 0 || combat_state.zones.draw_pile.is_empty() {
                        return true;
                    }
                    let limit = amount.min(combat_state.zones.draw_pile.len());
                    let cards = combat_state
                        .zones
                        .draw_pile
                        .iter()
                        .take(limit)
                        .map(|card| card.id)
                        .collect();
                    let card_uuids = combat_state
                        .zones
                        .draw_pile
                        .iter()
                        .take(limit)
                        .map(|card| card.uuid)
                        .collect();

                    update_monster_intents(combat_state);
                    *engine_state =
                        EngineState::PendingChoice(PendingChoice::ScrySelect { cards, card_uuids });
                    return true;
                }

                Action::FleeCombat => {
                    // Java SmokeBomb does not instantly jump to rewards. It flips the
                    // room/player into an escaping state, then the combat UI lingers
                    // through end-of-turn processing before rewards appear.
                    combat_state.runtime.combat_smoked = true;
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
                    if crate::content::relics::unceasing_top::maybe_on_refresh_hand(combat_state) {
                        *engine_state = EngineState::CombatProcessing;
                        return true;
                    }
                    update_monster_intents(combat_state);
                    *engine_state = EngineState::CombatPlayerTurn;
                }
                CombatPhase::TurnTransition => {
                    // === TURN TRANSITION: end of player turn -> enemy turn -> new player turn ===

                    // 1. Discard hand (respecting Retain and RunicPyramid)
                    discard_hand_for_turn_transition(combat_state);

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
                                crate::rewards::state::RewardState::with_context(
                                    crate::rewards::state::RewardScreenContext::SmokedCombat,
                                ),
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
                            let hook_actions =
                                crate::content::powers::resolve_power_instance_at_turn_start(
                                    power,
                                    combat_state,
                                    *mid,
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
                    let player_had_blur_for_block_retention =
                        crate::content::powers::store::has_power(
                            combat_state,
                            0,
                            crate::content::powers::PowerId::Blur,
                        );
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

                    // 3. Intent rolling is usually handled by Action::RollMonsterMove in the queue,
                    //    but freshly spawned monsters may roll immediately during spawn to match
                    //    Java SpawnMonsterAction.init() timing.

                    // === NEW PLAYER TURN START ===
                    // 4. Clear player block (Barricade: keep all, Calipers: retain up to 15)
                    let has_barricade = crate::content::powers::store::has_power(
                        combat_state,
                        0,
                        crate::content::powers::PowerId::Barricade,
                    );
                    if !has_barricade && !player_had_blur_for_block_retention {
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
                    crate::engine::action_handlers::powers::apply_player_turn_energy_recharge_hooks(
                        combat_state,
                    );
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

                    // 7.9. current stance atStartOfTurn.
                    // Java: AbstractPlayer.applyStartOfTurnRelics() calls stance.atStartOfTurn()
                    // before relic atTurnStart hooks. Divinity queues a return to Neutral here.
                    if combat_state.entities.player.stance
                        == crate::runtime::combat::StanceId::Divinity
                    {
                        combat_state.queue_action_back(Action::EnterStance("Neutral".to_string()));
                    }

                    // 8. at_turn_start relic hooks (AncientTeaSet, HappyFlower, etc.)
                    // Java: stance and relics fire atTurnStart BEFORE draw cards
                    let turn_start_actions =
                        crate::content::relics::hooks::at_turn_start(combat_state);
                    combat_state.queue_actions(turn_start_actions);

                    // 8.1. at_turn_start power hooks (Player)
                    // Java: player.applyStartOfTurnPowers()
                    for power in
                        &crate::content::powers::store::powers_snapshot_for(combat_state, 0)
                    {
                        let pa = crate::content::powers::resolve_power_instance_at_turn_start(
                            power,
                            combat_state,
                            0,
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
            if let Some(keep_running) = settle_victory_if_ready(engine_state, combat_state) {
                return keep_running;
            }
            return true;
        }
    }

    if let Some(keep_running) = settle_victory_if_ready(engine_state, combat_state) {
        return keep_running;
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
            // Queue Java callEndOfTurnActions equivalent. The marker expands to
            // relics, powers, orb passives, in-hand card triggers, and stance cleanup.
            combat_state.queue_action_back(Action::EndTurnTrigger);
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
        PendingChoice::ScrySelect { cards, card_uuids } => pending_choices::handle_scry(
            engine_state,
            combat_state,
            cards.len(),
            &card_uuids,
            input,
        ),
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel: cancellable,
            reason,
        } => pending_choices::handle_hand_select(
            engine_state,
            combat_state,
            &candidate_uuids,
            max_cards as usize,
            min_cards == max_cards,
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
        PendingChoice::DiscoverySelect(ref choice) => {
            // Player picks one card from the discovery options
            let choice = choice.clone();
            match input {
                ClientInput::SubmitDiscoverChoice(idx) if idx < choice.cards.len() => {
                    // Java DiscoveryAction.update() calls generateCardChoices()
                    // before checking whether the screen returned a selected
                    // discoveryCard, so resuming the action burns one more
                    // unused set of random choices.
                    let _ = generate_discovery_choices(
                        combat_state,
                        choice.colorless,
                        choice.card_type,
                    );
                    let card_id = choice.cards[idx];
                    let uuid = combat_state.next_card_uuid();
                    let mut card = crate::content::cards::make_fresh_card_copy_for_combat(
                        card_id,
                        uuid,
                        combat_state,
                    );
                    // Apply cost override from the SuspendForDiscovery action
                    if let Some(cost) = combat_state.turn.take_discovery_cost_for_turn() {
                        card.set_cost_for_turn_java(cost as i32);
                    }
                    crate::content::cards::apply_master_reality_to_generated_card(
                        &mut card,
                        combat_state,
                        2,
                    );
                    if combat_state.zones.hand.len() < 10 {
                        if crate::content::powers::store::has_power(
                            combat_state,
                            0,
                            crate::content::powers::PowerId::Corruption,
                        ) {
                            crate::content::cards::ironclad::corruption::corruption_on_card_draw(
                                combat_state,
                                &mut card,
                            );
                        }
                        crate::content::cards::evaluate_card(&mut card, combat_state, None);
                        combat_state.zones.hand.push(card);
                    } else {
                        combat_state.add_card_to_discard_pile_top(card);
                    }
                    *engine_state = EngineState::CombatProcessing;
                    Ok(())
                }
                ClientInput::Cancel if choice.can_skip => {
                    let _ = generate_discovery_choices(
                        combat_state,
                        choice.colorless,
                        choice.card_type,
                    );
                    let _ = combat_state.turn.take_discovery_cost_for_turn();
                    *engine_state = EngineState::CombatProcessing;
                    Ok(())
                }
                _ => Err("Invalid discovery choice"),
            }
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
                        let uuid = combat_state.next_card_uuid();
                        let mut card = crate::content::cards::make_fresh_card_copy_for_combat(
                            card_id,
                            uuid,
                            combat_state,
                        );
                        match destination {
                            crate::runtime::action::CardDestination::Hand => {
                                // Java ChooseOneColorless: hand (or discard if full)
                                crate::content::cards::apply_master_reality_to_generated_card(
                                    &mut card,
                                    combat_state,
                                    2,
                                );
                                if combat_state.zones.hand.len() < 10 {
                                    if crate::content::powers::store::has_power(
                                        combat_state,
                                        0,
                                        crate::content::powers::PowerId::Corruption,
                                    ) {
                                        crate::content::cards::ironclad::corruption::corruption_on_card_draw(
                                            combat_state,
                                            &mut card,
                                        );
                                    }
                                    crate::content::cards::evaluate_card(
                                        &mut card,
                                        combat_state,
                                        None,
                                    );
                                    combat_state.zones.hand.push(card);
                                } else {
                                    combat_state.add_card_to_discard_pile_top(card);
                                }
                            }
                            crate::runtime::action::CardDestination::DrawPileRandom => {
                                // Java CodexAction: add to draw pile at random position
                                crate::content::cards::apply_master_reality_to_generated_card(
                                    &mut card,
                                    combat_state,
                                    1,
                                );
                                combat_state.add_card_to_draw_pile_random_spot(card);
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

fn hand_select_can_fizzle_when_empty(reason: crate::state::HandSelectReason) -> bool {
    matches!(
        reason,
        crate::state::HandSelectReason::Discard
            | crate::state::HandSelectReason::Exhaust
            | crate::state::HandSelectReason::PutOnDrawPile
            | crate::state::HandSelectReason::Setup
            | crate::state::HandSelectReason::PutToBottomOfDraw
            | crate::state::HandSelectReason::Nightmare { .. }
            | crate::state::HandSelectReason::Recycle
    )
}

fn grid_select_candidates(
    combat_state: &mut CombatState,
    source_pile: crate::state::PileType,
    filter: crate::state::GridSelectFilter,
    reason: crate::state::GridSelectReason,
) -> Vec<u32> {
    match reason {
        crate::state::GridSelectReason::DrawPileToHand
            if source_pile == crate::state::PileType::Draw
                && filter == crate::state::GridSelectFilter::Any =>
        {
            return java_better_draw_pile_to_hand_candidates(combat_state);
        }
        crate::state::GridSelectReason::SkillFromDeckToHand
            if source_pile == crate::state::PileType::Draw
                && filter == crate::state::GridSelectFilter::Skill =>
        {
            return java_deck_to_hand_type_candidates(
                combat_state,
                crate::content::cards::CardType::Skill,
            );
        }
        crate::state::GridSelectReason::AttackFromDeckToHand
            if source_pile == crate::state::PileType::Draw
                && filter == crate::state::GridSelectFilter::Attack =>
        {
            return java_deck_to_hand_type_candidates(
                combat_state,
                crate::content::cards::CardType::Attack,
            );
        }
        _ => {}
    }

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

fn java_better_draw_pile_to_hand_candidates(combat_state: &CombatState) -> Vec<u32> {
    let mut cards: Vec<&crate::runtime::combat::CombatCard> =
        combat_state.zones.draw_pile.iter().rev().collect();

    cards.sort_by(|a, b| {
        let a_name = crate::content::cards::get_card_definition(a.id).name;
        let b_name = crate::content::cards::get_card_definition(b.id).name;
        a_name.cmp(b_name)
    });
    cards.sort_by(|a, b| {
        let a_rarity =
            java_card_rarity_ordinal(crate::content::cards::get_card_definition(a.id).rarity);
        let b_rarity =
            java_card_rarity_ordinal(crate::content::cards::get_card_definition(b.id).rarity);
        b_rarity.cmp(&a_rarity)
    });
    cards.sort_by(|a, b| {
        let a_status = crate::content::cards::get_card_definition(a.id).card_type
            == crate::content::cards::CardType::Status;
        let b_status = crate::content::cards::get_card_definition(b.id).card_type
            == crate::content::cards::CardType::Status;
        a_status.cmp(&b_status)
    });

    cards.into_iter().map(|card| card.uuid).collect()
}

fn java_deck_to_hand_type_candidates(
    combat_state: &mut CombatState,
    card_type: crate::content::cards::CardType,
) -> Vec<u32> {
    let matching_uuids: Vec<u32> = combat_state
        .zones
        .draw_pile
        .iter()
        .rev()
        .filter(|card| crate::content::cards::get_card_definition(card.id).card_type == card_type)
        .map(|card| card.uuid)
        .collect();

    let mut candidates = Vec::new();
    for uuid in matching_uuids {
        if candidates.is_empty() {
            candidates.push(uuid);
        } else {
            let index = combat_state
                .rng
                .card_random_rng
                .random(candidates.len() as i32 - 1) as usize;
            candidates.insert(index, uuid);
        }
    }
    candidates
}

fn java_card_rarity_ordinal(rarity: crate::content::cards::CardRarity) -> u8 {
    match rarity {
        crate::content::cards::CardRarity::Basic => 0,
        crate::content::cards::CardRarity::Special => 1,
        crate::content::cards::CardRarity::Common => 2,
        crate::content::cards::CardRarity::Uncommon => 3,
        crate::content::cards::CardRarity::Rare => 4,
        crate::content::cards::CardRarity::Curse => 5,
    }
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
    // `ActionInfo` order is the Java call order.  Java `addToTop`
    // inserts at index 0 immediately, so later top insertions run before
    // earlier top insertions.
    for a in actions {
        match a.insertion_mode {
            crate::runtime::action::AddTo::Top => queue.push_front(a.action),
            crate::runtime::action::AddTo::Bottom => queue.push_back(a.action),
        }
    }
}

/// Legacy Java UI preview refresh. Rust engine no longer mutates protocol observation caches here.
pub fn update_monster_intents(_combat_state: &mut CombatState) {}

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
            EngineState::CombatProcessing if is_smoke_escape_stable_boundary(es, cs) => break,
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

#[cfg(test)]
mod tests {
    use super::{class_combat_card_pool, discard_hand_for_turn_transition, resolve_pending_choice};
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::action::{Action, CardDestination};
    use crate::runtime::combat::{CombatCard, Power, StanceId};
    use crate::state::core::{
        ClientInput, DiscoveryChoiceState, EngineState, GridSelectFilter, GridSelectReason,
        PendingChoice, PileType,
    };
    use crate::test_support::{blank_test_combat, planned_monster};

    #[test]
    fn class_combat_card_pool_uses_current_player_class_not_ironclad_fallback() {
        let silent_pool = class_combat_card_pool("Silent");
        assert!(silent_pool.contains(&CardId::Acrobatics));
        assert!(silent_pool.contains(&CardId::Adrenaline));
        assert!(
            !silent_pool.contains(&CardId::PommelStrike),
            "Discovery/Codex-style class pools must not hard-code Ironclad cards for Silent"
        );

        let ironclad_pool = class_combat_card_pool("Ironclad");
        assert!(ironclad_pool.contains(&CardId::PommelStrike));
        assert!(!ironclad_pool.contains(&CardId::Acrobatics));
    }

    #[test]
    fn divinity_returns_to_neutral_at_start_of_next_player_turn() {
        let mut combat_state = blank_test_combat();
        let mut engine_state = EngineState::CombatProcessing;
        combat_state.entities.monsters = vec![planned_monster(EnemyId::Cultist, 3)];
        combat_state.entities.player.stance = StanceId::Divinity;
        combat_state.turn.begin_turn_transition();

        for _ in 0..64 {
            if engine_state == EngineState::CombatPlayerTurn {
                break;
            }
            assert!(super::tick_engine(
                &mut engine_state,
                &mut combat_state,
                None
            ));
        }

        assert_eq!(
            combat_state.entities.player.stance,
            StanceId::Neutral,
            "Java DivinityStance.atStartOfTurn queues ChangeStanceAction(\"Neutral\") at the next player turn start"
        );
    }

    #[test]
    fn seek_grid_candidates_match_java_better_draw_pile_sort_order() {
        let mut combat_state = blank_test_combat();
        let mut engine_state = EngineState::CombatProcessing;
        combat_state.zones.draw_pile = vec![
            CombatCard::new(CardId::StrikeB, 10),
            CombatCard::new(CardId::DefendB, 20),
            CombatCard::new(CardId::Bash, 30),
        ];
        combat_state.queue_action_back(Action::SuspendForGridSelect {
            source_pile: PileType::Draw,
            min: 1,
            max: 1,
            can_cancel: false,
            filter: GridSelectFilter::Any,
            reason: GridSelectReason::DrawPileToHand,
        });

        assert!(super::tick_engine(
            &mut engine_state,
            &mut combat_state,
            None
        ));

        match engine_state {
            EngineState::PendingChoice(PendingChoice::GridSelect {
                candidate_uuids, ..
            }) => assert_eq!(
                candidate_uuids,
                vec![30, 20, 10],
                "Java BetterDrawPileToHandAction sorts the temporary draw-pile group before opening grid select"
            ),
            other => panic!("Seek-style grid select should remain pending, got {other:?}"),
        }
    }

    #[test]
    fn secret_technique_grid_candidates_consume_java_add_to_random_spot_rng() {
        let mut combat_state = blank_test_combat();
        let mut engine_state = EngineState::CombatProcessing;
        combat_state.zones.draw_pile = vec![
            CombatCard::new(CardId::StrikeB, 10),
            CombatCard::new(CardId::DefendB, 20),
            CombatCard::new(CardId::Seek, 30),
            CombatCard::new(CardId::DefendG, 40),
        ];

        let mut expected_rng = combat_state.rng.clone();
        let mut expected_candidates = Vec::new();
        for uuid in [40_u32, 30, 20] {
            if expected_candidates.is_empty() {
                expected_candidates.push(uuid);
            } else {
                let index = expected_rng
                    .card_random_rng
                    .random(expected_candidates.len() as i32 - 1)
                    as usize;
                expected_candidates.insert(index, uuid);
            }
        }

        combat_state.queue_action_back(Action::SuspendForGridSelect {
            source_pile: PileType::Draw,
            min: 1,
            max: 1,
            can_cancel: false,
            filter: GridSelectFilter::Skill,
            reason: GridSelectReason::SkillFromDeckToHand,
        });

        assert!(super::tick_engine(
            &mut engine_state,
            &mut combat_state,
            None
        ));

        match engine_state {
            EngineState::PendingChoice(PendingChoice::GridSelect {
                candidate_uuids, ..
            }) => assert_eq!(
                candidate_uuids, expected_candidates,
                "Java SkillFromDeckToHandAction builds its temporary group with addToRandomSpot"
            ),
            other => {
                panic!("Secret Technique-style grid select should remain pending, got {other:?}")
            }
        }
        assert_eq!(
            combat_state.rng.card_random_rng.counter, expected_rng.card_random_rng.counter,
            "opening Secret Technique's multi-candidate grid select consumes cardRandomRng"
        );
    }

    #[test]
    fn discovery_resume_burns_java_unused_choice_rng() {
        let mut combat_state = blank_test_combat();
        let mut engine_state = EngineState::CombatProcessing;
        combat_state.queue_action_back(crate::runtime::action::Action::SuspendForDiscovery {
            colorless: false,
            card_type: None,
            cost_for_turn: Some(0),
            can_skip: false,
        });

        assert!(super::tick_engine(
            &mut engine_state,
            &mut combat_state,
            None
        ));
        let counter_after_open = combat_state.rng.card_random_rng.counter;
        let selected_id = match &engine_state {
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(choice)) => {
                assert!(!choice.can_skip);
                assert_eq!(choice.cards.len(), 3);
                choice.cards[0]
            }
            other => panic!("DiscoveryAction should open a discovery choice, got {other:?}"),
        };

        resolve_pending_choice(
            &mut engine_state,
            &mut combat_state,
            ClientInput::SubmitDiscoverChoice(0),
        )
        .expect("valid discovery choice should resolve");

        assert!(
            combat_state.rng.card_random_rng.counter >= counter_after_open + 3,
            "Java DiscoveryAction.update regenerates an unused choice set when the action resumes"
        );
        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(combat_state.zones.hand.len(), 1);
        assert_eq!(combat_state.zones.hand[0].id, selected_id);
    }

    #[test]
    fn typed_discovery_choice_can_skip_and_still_burns_resume_rng() {
        let mut combat_state = blank_test_combat();
        let mut engine_state = EngineState::CombatProcessing;
        combat_state.queue_action_back(crate::runtime::action::Action::SuspendForDiscovery {
            colorless: false,
            card_type: Some(crate::content::cards::CardType::Attack),
            cost_for_turn: Some(0),
            can_skip: true,
        });

        assert!(super::tick_engine(
            &mut engine_state,
            &mut combat_state,
            None
        ));
        let counter_after_open = combat_state.rng.card_random_rng.counter;
        match &engine_state {
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(choice)) => {
                assert!(choice.can_skip);
                assert_eq!(
                    choice.card_type,
                    Some(crate::content::cards::CardType::Attack)
                );
                assert_eq!(choice.cards.len(), 3);
            }
            other => panic!("typed DiscoveryAction should open a skippable choice, got {other:?}"),
        }

        resolve_pending_choice(&mut engine_state, &mut combat_state, ClientInput::Cancel)
            .expect("skippable typed discovery choice should accept cancel");

        assert!(
            combat_state.rng.card_random_rng.counter >= counter_after_open + 3,
            "Java typed DiscoveryAction burns the resume-time generated choice set even when skipped"
        );
        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert!(combat_state.zones.hand.is_empty());
        assert_eq!(combat_state.turn.take_discovery_cost_for_turn(), None);
    }

    #[test]
    fn discovery_selection_uses_java_make_copy_and_master_reality_path() {
        let mut combat_state = blank_test_combat();
        combat_state.turn.counters.times_damaged_this_combat = 2;
        combat_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut engine_state =
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::BloodForBlood],
                colorless: false,
                card_type: None,
                can_skip: false,
            }));

        resolve_pending_choice(
            &mut engine_state,
            &mut combat_state,
            ClientInput::SubmitDiscoverChoice(0),
        )
        .expect("valid discovery choice should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(combat_state.zones.hand.len(), 1);
        let card = &combat_state.zones.hand[0];
        assert_eq!(card.id, CardId::BloodForBlood);
        assert_eq!(card.cost_modifier, -2);
        assert_eq!(
            card.get_cost(),
            1,
            "Java Discovery keeps Blood for Blood.makeCopy damagedThisCombat discount before Master Reality upgrades it"
        );
        assert_eq!(
            card.upgrades, 1,
            "Blood for Blood ignores the second Master Reality upgrade call because it is already upgraded"
        );
    }

    #[test]
    fn pending_choice_generated_cards_use_combat_uuid_counter() {
        let mut combat_state = blank_test_combat();
        combat_state.zones.card_uuid_counter = 100;

        let mut first_choice =
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::Strike],
                colorless: false,
                card_type: None,
                can_skip: false,
            }));
        resolve_pending_choice(
            &mut first_choice,
            &mut combat_state,
            ClientInput::SubmitDiscoverChoice(0),
        )
        .expect("first discovery choice should resolve");

        let mut second_choice =
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::Defend],
                colorless: false,
                card_type: None,
                can_skip: false,
            }));
        resolve_pending_choice(
            &mut second_choice,
            &mut combat_state,
            ClientInput::SubmitDiscoverChoice(0),
        )
        .expect("second discovery choice should resolve");

        assert_eq!(combat_state.zones.hand[0].uuid, 101);
        assert_eq!(combat_state.zones.hand[1].uuid, 102);
        assert_eq!(combat_state.zones.card_uuid_counter, 102);
    }

    #[test]
    fn card_reward_selection_preserves_codex_master_reality_single_draw_path() {
        let mut combat_state = blank_test_combat();
        combat_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::MasterRealityPower,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        let mut engine_state = EngineState::PendingChoice(PendingChoice::CardRewardSelect {
            cards: vec![CardId::SearingBlow],
            destination: CardDestination::DrawPileRandom,
            can_skip: true,
        });

        resolve_pending_choice(
            &mut engine_state,
            &mut combat_state,
            ClientInput::SubmitDiscoverChoice(0),
        )
        .expect("valid Codex-style choice should resolve");

        assert_eq!(engine_state, EngineState::CombatProcessing);
        assert_eq!(combat_state.zones.draw_pile.len(), 1);
        assert_eq!(combat_state.zones.draw_pile[0].id, CardId::SearingBlow);
        assert_eq!(
            combat_state.zones.draw_pile[0].upgrades, 1,
            "Java CodexAction relies on ShowCardAndAddToDrawPileEffect for one Master Reality upgrade"
        );
    }

    #[test]
    fn turn_transition_retains_selected_cards_once_and_clears_flag() {
        let mut combat_state = blank_test_combat();
        let mut retained = CombatCard::new(CardId::Defend, 20);
        retained.retain_override = Some(true);
        combat_state.zones.hand = vec![CombatCard::new(CardId::Strike, 10), retained];

        discard_hand_for_turn_transition(&mut combat_state);

        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .map(|card| (card.id, card.uuid, card.retain_override))
                .collect::<Vec<_>>(),
            vec![(CardId::Defend, 20, None)],
            "Java RestoreRetainedCardsAction clears AbstractCard.retain after preserving the card"
        );
        assert_eq!(
            combat_state
                .zones
                .discard_pile
                .iter()
                .map(|card| (card.id, card.uuid))
                .collect::<Vec<_>>(),
            vec![(CardId::Strike, 10)]
        );
    }

    #[test]
    fn runic_pyramid_keeps_hand_but_clears_one_turn_retain_flags() {
        let mut combat_state = blank_test_combat();
        combat_state
            .entities
            .player
            .add_relic(RelicState::new(RelicId::RunicPyramid));
        let mut explicitly_retained = CombatCard::new(CardId::Defend, 20);
        explicitly_retained.retain_override = Some(true);
        combat_state.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            explicitly_retained,
            CombatCard::new(CardId::Bash, 30),
        ];

        discard_hand_for_turn_transition(&mut combat_state);

        assert_eq!(
            combat_state
                .zones
                .hand
                .iter()
                .map(|card| (card.id, card.uuid, card.retain_override))
                .collect::<Vec<_>>(),
            vec![
                (CardId::Strike, 10, None),
                (CardId::Defend, 20, None),
                (CardId::Bash, 30, None),
            ],
            "Runic Pyramid's global retention does not keep RetainCardsAction's one-turn retain flag alive"
        );
        assert!(combat_state.zones.discard_pile.is_empty());
    }

    #[test]
    fn blur_retains_player_block_through_next_turn_while_power_ticks_down() {
        let mut combat_state = blank_test_combat();
        combat_state.entities.player.block = 12;
        combat_state.entities.monsters = vec![crate::test_support::planned_monster(
            crate::content::monsters::EnemyId::Cultist,
            3,
        )];
        combat_state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Blur,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
        combat_state.turn.begin_turn_transition();
        let mut engine_state = EngineState::CombatProcessing;

        for _ in 0..64 {
            if engine_state == EngineState::CombatPlayerTurn {
                break;
            }
            assert!(super::tick_engine(
                &mut engine_state,
                &mut combat_state,
                None
            ));
        }

        assert_eq!(engine_state, EngineState::CombatPlayerTurn);
        assert_eq!(
            combat_state.entities.player.block, 12,
            "Java GameActionManager skips new-turn block loss while Blur exists"
        );
        assert!(
            !crate::content::powers::store::has_power(&combat_state, 0, PowerId::Blur),
            "Java BlurPower ticks down while still preserving that turn's block"
        );
    }
}
