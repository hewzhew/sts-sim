//! action_handlers — Unified action executor with domain-split sub-modules.
//!
//! Sub-modules:
//!   - damage:   Combat damage pipeline (Damage, FiendFire, Feed, Vampire, etc.)
//!   - cards:    Card pile management (Draw, Exhaust, MakeTemp, PlayCardDirect, etc.)
//!   - powers:   Power lifecycle (ApplyPower, RemovePower, Artifact, Stasis, etc.)
//!   - spawning: Monster lifecycle (Spawn, Escape, Suicide, RollMove, relics, etc.)

pub mod cards;
pub mod damage;
pub mod powers;
pub mod spawning;

use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use crate::content::powers::store;

/// Synchronously checks for and applies Fairy In A Bottle or Lizard Tail when player HP hits 0.
pub fn try_revive(state: &mut CombatState) {
    if state.entities.player.current_hp > 0 {
        return;
    }
    if state
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::MarkOfTheBloom)
    {
        return;
    }

    let fairy_slot = state.entities.potions.iter().position(|p| {
        p.as_ref().map_or(false, |pot| {
            pot.id == crate::content::potions::PotionId::FairyPotion
        })
    });
    if let Some(slot) = fairy_slot {
        state.entities.potions[slot] = None;
        let mut potency = 0.3_f32;
        if state
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::SacredBark)
        {
            potency *= 2.0;
        }
        let heal_amount = (state.entities.player.max_hp as f32 * potency) as i32;
        state.entities.player.current_hp = heal_amount.max(1);
        return;
    }

    let lizard_unused = state
        .entities
        .player
        .relics
        .iter()
        .find(|r| r.id == crate::content::relics::RelicId::LizardTail)
        .map_or(false, |r| !r.used_up);
    if lizard_unused {
        state.entities.player.current_hp = std::cmp::max(1, state.entities.player.max_hp / 2);
        if let Some(lt) = state
            .entities
            .player
            .relics
            .iter_mut()
            .find(|r| r.id == crate::content::relics::RelicId::LizardTail)
        {
            lt.used_up = true;
            lt.counter = -2;
        }
    }
}

/// Centralized monster death handler.
/// Fires power on_death hooks, monster on_death, relic hooks, GremlinLeader/Darkling specials.
pub fn check_and_trigger_monster_death(state: &mut CombatState, target_id: usize) {
    let mut is_awakened_rebirth = false;
    let mut is_gremlin_leader = false;
    let mut triggered_death = false;
    let mut dying_monster_type: Option<crate::content::monsters::EnemyId> = None;

    if let Some(m) = state
        .entities
        .monsters
        .iter_mut()
        .find(|m| m.id == target_id)
    {
        if m.current_hp <= 0 && !m.is_dying {
            m.is_dying = true;
            let m_id = crate::content::monsters::EnemyId::from_id(m.monster_type);
            dying_monster_type = m_id;
            is_gremlin_leader = m_id == Some(crate::content::monsters::EnemyId::GremlinLeader);
            let has_rebirth_power = store::powers_for(state, target_id).is_some_and(|powers| {
                powers.iter().any(|p| {
                    matches!(
                        p.power_type,
                        crate::content::powers::PowerId::Regrow
                            | crate::content::powers::PowerId::Unawakened
                    )
                })
            });
            is_awakened_rebirth =
                has_rebirth_power && m_id == Some(crate::content::monsters::EnemyId::AwakenedOne);
            triggered_death = true;
        }
    }

    if triggered_death {
        // Fire power on_death hooks BEFORE clearing (SporeCloud, Stasis, Unawakened, etc.)
        for power in &store::powers_snapshot_for(state, target_id) {
            let death_actions = crate::content::powers::resolve_power_on_death(
                power.power_type,
                state,
                target_id,
                power.amount,
                power.extra_data,
            );
            for a in death_actions {
                state.engine.action_queue.push_back(a);
            }
        }

        if let Some(m_id) = dying_monster_type {
            if !is_awakened_rebirth {
                let m_clone = state
                    .entities
                    .monsters
                    .iter()
                    .find(|m| m.id == target_id)
                    .unwrap()
                    .clone();
                let death_actions_on_entity =
                    crate::content::monsters::resolve_on_death(m_id, state, &m_clone);
                for a in death_actions_on_entity {
                    state.engine.action_queue.push_back(a);
                }
            }
        }

        let death_actions = crate::content::relics::hooks::on_monster_death(state, target_id);
        crate::engine::core::queue_actions(&mut state.engine.action_queue, death_actions);

        if is_gremlin_leader {
            let minion_ids: Vec<_> = state
                .entities
                .monsters
                .iter()
                .filter(|min| min.id != target_id && !min.is_dying)
                .map(|min| min.id)
                .collect();
            for minion_id in minion_ids {
                state
                    .engine
                    .action_queue
                    .push_back(Action::Escape { target: minion_id });
            }
        }
        if is_awakened_rebirth {
            if let Some(m) = state
                .entities
                .monsters
                .iter_mut()
                .find(|m| m.id == target_id)
            {
                m.current_hp = 0;
                m.is_dying = false;
                m.current_intent = crate::runtime::combat::Intent::Unknown;
                if dying_monster_type == Some(crate::content::monsters::EnemyId::AwakenedOne) {
                    m.half_dead = true;
                }
            }
        }
    }
}

/// Executes a single atomic Action off the queue.
/// This is the thin dispatcher — each arm delegates to the appropriate sub-module.
pub fn execute_action(action: Action, state: &mut CombatState) {
    match action {
        // === Damage domain ===
        Action::Damage(info) => damage::handle_damage(info, state),
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => damage::handle_damage_all_enemies(source, damages, damage_type, is_modified, state),
        Action::AttackDamageRandomEnemy {
            base_damage,
            damage_type,
            applies_target_modifiers,
        } => damage::handle_attack_damage_random_enemy(
            base_damage,
            damage_type,
            applies_target_modifiers,
            state,
        ),
        Action::DropkickDamageAndEffect {
            target,
            damage_info,
        } => damage::handle_dropkick(target, damage_info, state),
        Action::FiendFire {
            target,
            damage_info,
        } => damage::handle_fiend_fire(target, damage_info, state),
        Action::Feed {
            target,
            damage_info,
            max_hp_amount,
        } => damage::handle_feed(target, damage_info, max_hp_amount, state),
        Action::HandOfGreed {
            target,
            damage_info,
            gold_amount,
        } => damage::handle_hand_of_greed(target, damage_info, gold_amount, state),
        Action::RitualDagger {
            target,
            damage_info,
            misc_amount,
            card_uuid,
        } => damage::handle_ritual_dagger(target, damage_info, misc_amount, card_uuid, state),
        Action::VampireDamage(info) => damage::handle_vampire_damage(info, state),
        Action::VampireDamageAllEnemies {
            source,
            damages,
            damage_type,
        } => damage::handle_vampire_damage_all_enemies(source, damages, damage_type, state),
        Action::LoseHp {
            target,
            amount,
            triggers_rupture,
        } => damage::handle_lose_hp(target, amount, triggers_rupture, state),
        Action::SetCurrentHp { target, hp } => damage::handle_set_current_hp(target, hp, state),
        Action::GainBlock { target, amount } => damage::handle_gain_block(target, amount, state),
        Action::GainBlockRandomMonster { source, amount } => {
            damage::handle_gain_block_random_monster(source, amount, state)
        }
        Action::LoseBlock { target, amount } => damage::handle_lose_block(target, amount, state),
        Action::Heal { target, amount } => damage::handle_heal(target, amount, state),
        Action::GainGold { amount } => damage::handle_gain_gold(amount, state),
        Action::LimitBreak => damage::handle_limit_break(state),
        Action::BlockPerNonAttack { block_per_card } => {
            damage::handle_block_per_non_attack(block_per_card, state)
        }
        Action::ExhaustAllNonAttack => damage::handle_exhaust_all_non_attack(state),
        Action::ExhaustRandomCard { amount } => damage::handle_exhaust_random_card(amount, state),

        // === Power domain ===
        Action::ApplyPower {
            source,
            target,
            power_id,
            amount,
        } => powers::handle_apply_power(source, target, power_id, amount, state),
        Action::ApplyPowerDetailed {
            source,
            target,
            power_id,
            amount,
            instance_id,
            extra_data,
        } => powers::handle_apply_power_detailed(
            source,
            target,
            power_id,
            amount,
            instance_id,
            extra_data,
            state,
        ),
        Action::ReducePower {
            target,
            power_id,
            amount,
        } => powers::handle_reduce_power(target, power_id, amount, state),
        Action::ReducePowerInstance {
            target,
            power_id,
            instance_id,
            amount,
        } => powers::handle_reduce_power_instance(target, power_id, instance_id, amount, state),
        Action::BouncingFlask {
            target,
            amount,
            num_times,
        } => powers::handle_bouncing_flask(target, amount, num_times, state),
        Action::RemovePower { target, power_id } => {
            powers::handle_remove_power(target, power_id, state)
        }
        Action::RemovePowerInstance {
            target,
            power_id,
            instance_id,
        } => powers::handle_remove_power_instance(target, power_id, instance_id, state),
        Action::RemoveAllDebuffs { target } => powers::handle_remove_all_debuffs(target, state),
        Action::ApplyStasis { target_id } => powers::handle_apply_stasis(target_id, state),
        Action::UpdatePowerExtraData {
            target,
            power_id,
            value,
        } => powers::handle_update_power_extra_data(target, power_id, value, state),
        Action::UpdatePowerExtraDataInstance {
            target,
            power_id,
            instance_id,
            value,
        } => powers::handle_update_power_extra_data_instance(
            target,
            power_id,
            instance_id,
            value,
            state,
        ),
        Action::AwakenedRebirthClear { target } => {
            powers::handle_awakened_rebirth_clear(target, state)
        }
        Action::TriggerTimeWarpEndTurn { owner } => {
            powers::handle_trigger_time_warp_end_turn(owner, state)
        }
        Action::GainEnergy { amount } => powers::handle_gain_energy(amount, state),
        Action::GainMaxHp { amount } => powers::handle_gain_max_hp(amount, state),
        Action::LoseMaxHp { target, amount } => powers::handle_lose_max_hp(target, amount, state),

        // === Card domain ===
        Action::DrawCards(amount) => cards::handle_draw_cards(amount, state),
        Action::EmptyDeckShuffle => cards::handle_empty_deck_shuffle(state),
        Action::ShuffleDiscardIntoDraw => cards::handle_shuffle_discard_into_draw(state),
        Action::DiscardCard { card_uuid } => cards::handle_discard_card(card_uuid, state),
        Action::ExhaustCard {
            card_uuid,
            source_pile,
        } => cards::handle_exhaust_card(card_uuid, source_pile, state),
        Action::MoveCard {
            card_uuid,
            from,
            to,
        } => cards::handle_move_card(card_uuid, from, to, state),
        Action::RemoveCardFromPile { card_uuid, from } => {
            cards::handle_remove_card_from_pile(card_uuid, from, state)
        }
        Action::MakeTempCardInHand {
            card_id,
            amount,
            upgraded,
        } => cards::handle_make_temp_card_in_hand(card_id, amount, upgraded, state),
        Action::MakeTempCardInDiscard {
            card_id,
            amount,
            upgraded,
        } => cards::handle_make_temp_card_in_discard(card_id, amount, upgraded, state),
        Action::MakeTempCardInDrawPile {
            card_id,
            amount,
            random_spot,
            upgraded,
        } => {
            cards::handle_make_temp_card_in_draw_pile(card_id, amount, random_spot, upgraded, state)
        }
        Action::MakeCopyInHand { original, amount } => {
            cards::handle_make_copy_in_hand(original, amount, state)
        }
        Action::MakeCopyInDiscard { original, amount } => {
            cards::handle_make_copy_in_discard(original, amount, state)
        }
        Action::MakeTempCardInDiscardAndDeck { card_id, amount } => {
            cards::handle_make_temp_card_in_discard_and_deck(card_id, amount, state)
        }
        Action::ReduceAllHandCosts { amount } => cards::handle_reduce_all_hand_costs(amount, state),
        Action::Enlightenment { permanent } => cards::handle_enlightenment(permanent, state),
        Action::Madness => cards::handle_madness(state),
        Action::UpgradeAllInHand => cards::handle_upgrade_all_in_hand(state),
        Action::UpgradeAllBurns => cards::handle_upgrade_all_burns(state),
        Action::UpgradeCard { card_uuid } => cards::handle_upgrade_card(card_uuid, state),
        Action::UpgradeRandomCard => cards::handle_upgrade_random_card(state),
        Action::ModifyCardMisc { card_uuid, amount } => {
            cards::handle_modify_card_misc(card_uuid, amount, state)
        }
        Action::ModifyCardDamage { card_uuid, amount } => {
            cards::handle_modify_card_damage(card_uuid, amount, state)
        }
        Action::RandomizeHandCosts => cards::handle_randomize_hand_costs(state),
        Action::MummifiedHandEffect => cards::handle_mummified_hand_effect(state),
        Action::MakeRandomCardInHand {
            card_type,
            cost_for_turn,
        } => cards::handle_make_random_card_in_hand(card_type, cost_for_turn, state),
        Action::MakeRandomCardInDrawPile {
            card_type,
            cost_for_turn,
            random_spot,
        } => cards::handle_make_random_card_in_draw_pile(
            card_type,
            cost_for_turn,
            random_spot,
            state,
        ),
        Action::DrawPileToHandByType { amount, card_type } => {
            cards::handle_draw_pile_to_hand_by_type(amount, card_type, state)
        }
        Action::MakeRandomColorlessCardInHand {
            cost_for_turn,
            upgraded,
        } => cards::handle_make_random_colorless_card_in_hand(cost_for_turn, upgraded, state),
        Action::UseCardDone { should_exhaust } => {
            cards::handle_use_card_done(should_exhaust, state)
        }
        Action::QueueEarlyEndTurn => cards::handle_queue_early_end_turn(state),
        Action::EnqueueCardPlay { item, in_front } => {
            cards::handle_enqueue_card_play(*item, in_front, state)
        }
        Action::PostDrawTrigger => cards::handle_post_draw_trigger(state),
        Action::FlushNextQueuedCard => cards::handle_flush_next_queued_card(state),
        Action::PlayCardDirect {
            card,
            target,
            purge,
        } => cards::handle_play_card_direct(card, target, purge, state),
        Action::PlayTopCard { target, exhaust } => {
            cards::handle_play_top_card(target, exhaust, state)
        }
        Action::PlayTopCardsBuffered {
            count,
            target,
            exhaust,
        } => cards::handle_play_top_cards_buffered(count, target, exhaust, state),
        Action::UsePotion { slot, target } => cards::handle_use_potion(slot, target, state),
        Action::DiscardPotion { slot } => {
            if slot < state.entities.potions.len() {
                state.entities.potions[slot] = None;
            }
        }
        Action::ObtainPotion => cards::handle_obtain_potion(state),
        Action::ObtainSpecificPotion(potion_id) => {
            if !state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::Sozu)
            {
                if let Some(slot) = state.entities.potions.iter().position(|p| p.is_none()) {
                    state.entities.potions[slot] = Some(crate::content::potions::Potion::new(
                        potion_id,
                        40000 + slot as u32,
                    ));
                }
            }
        }
        Action::EndTurnTrigger => cards::handle_end_turn_trigger(state),
        Action::ClearCardQueue => cards::handle_clear_card_queue(state),
        Action::AddCardToMasterDeck { card_id } => {
            cards::handle_add_card_to_master_deck(card_id, state)
        }
        Action::BattleStartTrigger => cards::handle_battle_start_trigger(state),
        Action::PreBattleTrigger => cards::handle_pre_battle_trigger(state),
        Action::BattleStartPreDrawTrigger => cards::handle_battle_start_pre_draw_trigger(state),

        // === Spawning / Monster lifecycle domain ===
        Action::SpawnMonster {
            monster_id,
            slot,
            current_hp,
            max_hp,
            logical_position,
            protocol_draw_x,
            is_minion,
        } => spawning::handle_spawn_monster(
            monster_id,
            slot,
            current_hp,
            max_hp,
            logical_position,
            protocol_draw_x,
            is_minion,
            state,
        ),
        Action::SpawnMonsterSmart {
            monster_id,
            logical_position,
            current_hp,
            max_hp,
            protocol_draw_x,
            is_minion,
        } => spawning::handle_spawn_monster_smart(
            monster_id,
            logical_position,
            current_hp,
            max_hp,
            protocol_draw_x,
            is_minion,
            state,
        ),
        Action::Suicide { target } => spawning::handle_suicide(target, state),
        Action::Escape { target } => spawning::handle_escape(target, state),
        Action::RollMonsterMove { monster_id } => {
            spawning::handle_roll_monster_move(monster_id, state)
        }
        Action::SetMonsterMove {
            monster_id,
            next_move_byte,
            intent,
        } => spawning::handle_set_monster_move(monster_id, next_move_byte, intent, state),
        Action::UpdateHexaghostState {
            monster_id,
            activated,
            orb_active_count,
            burn_upgraded,
        } => spawning::handle_update_hexaghost_state(
            monster_id,
            activated,
            orb_active_count,
            burn_upgraded,
            state,
        ),
        Action::UpdateRelicCounter { relic_id, counter } => {
            spawning::handle_update_relic_counter(relic_id, counter, state)
        }
        Action::UpdateRelicAmount { relic_id, amount } => {
            spawning::handle_update_relic_amount(relic_id, amount, state)
        }
        Action::UpdateRelicUsedUp { relic_id, used_up } => {
            spawning::handle_update_relic_used_up(relic_id, used_up, state)
        }

        Action::IncreaseMaxOrb(amount) => handle_increase_max_orb(amount, state),
        Action::ChannelOrb(orb_id) => handle_channel_orb(orb_id, state),
        Action::EnterStance(stance) => handle_enter_stance(&stance, state),

        // === Pass-through / unhandled ===
        // These variants exist but have no handler yet or are handled inline elsewhere
        Action::PlayCard { .. }
        | Action::UseCard { .. }
        | Action::StartTurnTrigger
        | Action::AbortDeath { .. }
        | Action::FleeCombat
        | Action::ExecuteMonsterTurn(_)
        | Action::SpawnEncounter { .. }
        | Action::Scry(_)
        | Action::EvokeOrb
        | Action::TriggerPassiveOrbs => {
            #[cfg(debug_assertions)]
            eprintln!("[action_handlers] Unhandled action: {:?}", action);
        }
        Action::SuspendForHandSelect { .. }
        | Action::SuspendForGridSelect { .. }
        | Action::SuspendForDiscovery { .. }
        | Action::SuspendForStanceChoice
        | Action::SuspendForCardReward { .. } => {
            // These suspend actions are intercepted in engine::core and converted into
            // PendingChoice states. Reaching the thin dispatcher is not actionable noise.
        }
    }
}

fn handle_increase_max_orb(amount: u8, state: &mut CombatState) {
    if amount == 0 {
        return;
    }
    state.entities.player.max_orbs = state.entities.player.max_orbs.saturating_add(amount);
    while state.entities.player.orbs.len() < state.entities.player.max_orbs as usize {
        state
            .entities
            .player
            .orbs
            .push(crate::runtime::combat::OrbEntity::new(
                crate::runtime::combat::OrbId::Empty,
            ));
    }
}

fn handle_channel_orb(orb_id: crate::runtime::combat::OrbId, state: &mut CombatState) {
    if state.entities.player.max_orbs == 0 {
        return;
    }
    while state.entities.player.orbs.len() < state.entities.player.max_orbs as usize {
        state
            .entities
            .player
            .orbs
            .push(crate::runtime::combat::OrbEntity::new(
                crate::runtime::combat::OrbId::Empty,
            ));
    }
    let new_orb = crate::runtime::combat::OrbEntity::new(orb_id);
    if let Some(empty_slot) = state
        .entities
        .player
        .orbs
        .iter()
        .position(|orb| orb.id == crate::runtime::combat::OrbId::Empty)
    {
        state.entities.player.orbs[empty_slot] = new_orb;
    } else {
        state.entities.player.orbs.remove(0);
        state.entities.player.orbs.push(new_orb);
    }
}

fn handle_enter_stance(stance: &str, state: &mut CombatState) {
    let new_stance = match stance {
        "Wrath" => crate::runtime::combat::StanceId::Wrath,
        "Calm" => crate::runtime::combat::StanceId::Calm,
        "Divinity" => crate::runtime::combat::StanceId::Divinity,
        _ => crate::runtime::combat::StanceId::Neutral,
    };
    let old_stance = state.entities.player.stance;
    if old_stance == new_stance {
        return;
    }
    if old_stance == crate::runtime::combat::StanceId::Calm {
        state.turn.energy += 2;
    }
    if new_stance == crate::runtime::combat::StanceId::Divinity {
        state.turn.energy += 3;
    }
    state.entities.player.stance = new_stance;
    crate::content::relics::hooks::on_change_stance(state, old_stance, new_stance);
}
