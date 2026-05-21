use crate::content::cards::CardId;
use crate::content::powers::store;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;

pub fn handle_end_turn_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    // 1. Relics
    actions.extend(crate::content::relics::hooks::at_end_of_turn(state));

    // 2. Player powers
    for power in store::powers_snapshot_for(state, 0) {
        actions.extend(
            crate::content::powers::resolve_power_at_end_of_turn(&power, state, 0)
                .into_iter()
                .map(|a| ActionInfo {
                    action: a,
                    insertion_mode: AddTo::Bottom,
                }),
        );
    }

    // 3. Orbs
    actions.extend(crate::content::orbs::hooks::trigger_end_of_turn_orbs(state));

    // 4. Ethereal exhaust and status/curse in-hand triggers
    for card in &state.zones.hand {
        // Java DiscardAtEndOfTurnAction moves retain/selfRetain cards out of
        // hand before calling triggerOnEndOfPlayerTurn(), so explicit retain
        // wins over ethereal. Runic Pyramid does not set per-card retain and
        // therefore still allows ethereal cards to exhaust.
        if card.retain_override != Some(true)
            && !crate::content::cards::is_self_retain(card)
            && crate::content::cards::is_ethereal(card)
        {
            actions.push(ActionInfo {
                action: Action::ExhaustCard {
                    card_uuid: card.uuid,
                    source_pile: crate::state::PileType::Hand,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    for card in &state.zones.hand {
        if card.id == CardId::Burn {
            actions.extend(crate::content::cards::status::burn::on_end_turn_in_hand(
                state, card,
            ));
        }
        if card.id == CardId::Regret {
            actions.extend(crate::content::cards::curses::regret::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Decay {
            actions.extend(crate::content::cards::curses::decay::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Doubt {
            actions.extend(crate::content::cards::curses::doubt::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Pride {
            actions.extend(crate::content::cards::curses::pride::on_end_turn_in_hand(
                state,
            ));
        }
        if card.id == CardId::Shame {
            actions.extend(crate::content::cards::curses::shame::on_end_turn_in_hand(
                state,
            ));
        }
    }

    // 5. Stances
    actions.extend(crate::content::stances::hooks::on_end_of_turn(state));

    state.queue_actions(actions);
}

pub fn handle_post_draw_trigger(state: &mut CombatState) {
    let mut actions = smallvec::SmallVec::new();

    actions.extend(crate::content::relics::hooks::at_turn_start_post_draw(
        state,
    ));

    for power in &store::powers_snapshot_for(state, 0) {
        for action in crate::content::powers::resolve_power_on_post_draw(
            power.power_type,
            state,
            0,
            power.amount,
        ) {
            actions.push(ActionInfo {
                action,
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    state.queue_actions(actions);
}

pub fn handle_clear_card_queue(state: &mut CombatState) {
    state.zones.queued_cards.clear();
    state.engine.retain(|a| {
        !matches!(
            a,
            Action::EnqueueCardPlay { .. }
                | Action::PlayCardDirect { .. }
                | Action::FlushNextQueuedCard
        )
    });
}

pub fn handle_add_card_to_master_deck(card_id: CardId, state: &mut CombatState) {
    state
        .meta
        .meta_changes
        .push(crate::runtime::combat::MetaChange::AddCardToMasterDeck(
            card_id,
        ));
}

pub fn handle_pre_battle_trigger(state: &mut CombatState) {
    // 1. Monster pre-battle actions (CurlUp, ModeShift, etc.)
    // Java: AbstractRoom.initializeBattle() calls usePreBattleAction() for each monster
    let monsters_snapshot: Vec<_> = state
        .entities
        .monsters
        .iter()
        .filter_map(|m| {
            crate::content::monsters::EnemyId::from_id(m.monster_type).map(|eid| (eid, m.id))
        })
        .collect();
    for (enemy_id, monster_id) in &monsters_snapshot {
        if let Some(entity) = state.entities.monsters.iter().find(|m| m.id == *monster_id) {
            let entity_clone = entity.clone();
            let pre_actions = crate::content::monsters::resolve_pre_battle_actions(
                state,
                *enemy_id,
                &entity_clone,
                crate::content::monsters::PreBattleLegacyRng::MonsterHp,
            );
            for action in pre_actions {
                state.queue_action_back(action);
            }
        }
    }

    // 2. Relic pre-battle hooks (e.g. Snecko Eye applying Confusion)
    let pre_battle_actions = crate::content::relics::hooks::at_pre_battle(state);
    state.queue_actions(pre_battle_actions);

    // Auto-chain Phase 2
    state.queue_action_back(crate::runtime::action::Action::BattleStartPreDrawTrigger);
}

pub fn handle_battle_start_pre_draw_trigger(state: &mut CombatState) {
    let pre_draw_actions = crate::content::relics::hooks::at_battle_start_pre_draw(state);
    state.queue_actions(pre_draw_actions);

    // Java AbstractRoom.update() constructs the whole opening queue before
    // actionManager drains it:
    //   atBattleStartPreDraw hooks
    //   DrawCardAction
    //   atBattleStart hooks
    //   atTurnStart relics
    //   atTurnStartPostDraw relics
    //   card / power / orb atTurnStart hooks
    //
    // Therefore these hook methods must run synchronously here. Queuing a later
    // synthetic BattleStartTrigger would incorrectly let the initial draw execute
    // before atBattleStart / atTurnStart hooks have had a chance to enqueue.
    let draw_amount = crate::engine::core::compute_player_turn_start_draw_count(state);
    if draw_amount > 0 {
        state.queue_action_back(crate::runtime::action::Action::DrawCards(
            draw_amount as u32,
        ));
    }

    queue_initial_battle_start_hooks_after_draw_is_queued(state);
}

pub fn handle_battle_start_trigger(state: &mut CombatState) {
    // Relic battle-start hooks (e.g. Akabeko, Marbles)
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    state.queue_actions(battle_start_actions);
}

fn queue_initial_battle_start_hooks_after_draw_is_queued(state: &mut CombatState) {
    let battle_start_actions = crate::content::relics::hooks::at_battle_start(state);
    state.queue_actions(battle_start_actions);

    // Java AbstractPlayer.applyStartOfTurnRelics() calls stance.atStartOfTurn()
    // before relic atTurnStart hooks. Divinity queues a return to Neutral here.
    if state.entities.player.stance == crate::runtime::combat::StanceId::Divinity {
        state.queue_action_back(crate::runtime::action::Action::EnterStance(
            "Neutral".to_string(),
        ));
    }

    let turn_start_actions = crate::content::relics::hooks::at_turn_start(state);
    state.queue_actions(turn_start_actions);

    // Initial combat is special: AbstractRoom.update() calls only relic
    // atTurnStartPostDraw here, not power atStartOfTurnPostDraw.
    let post_draw_relic_actions = crate::content::relics::hooks::at_turn_start_post_draw(state);
    state.queue_actions(post_draw_relic_actions);

    let card_actions = crate::content::cards::hooks::at_turn_start_in_hand(state);
    state.queue_actions(card_actions);

    for power in &crate::content::powers::store::powers_snapshot_for(state, 0) {
        let power_actions =
            crate::content::powers::resolve_power_instance_at_turn_start(power, state, 0);
        for action in power_actions {
            state.queue_action_back(action);
        }
    }

    let orb_actions = crate::content::orbs::hooks::at_turn_start(state);
    state.queue_actions(orb_actions);
}
