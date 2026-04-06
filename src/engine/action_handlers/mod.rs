//! action_handlers — Unified action executor with domain-split sub-modules.
//!
//! Sub-modules:
//!   - damage:   Combat damage pipeline (Damage, FiendFire, Feed, Vampire, etc.)
//!   - cards:    Card pile management (Draw, Exhaust, MakeTemp, PlayCardDirect, etc.)
//!   - powers:   Power lifecycle (ApplyPower, RemovePower, Artifact, Stasis, etc.)
//!   - spawning: Monster lifecycle (Spawn, Escape, Suicide, RollMove, relics, etc.)

pub mod damage;
pub mod cards;
pub mod powers;
pub mod spawning;

use crate::action::Action;
use crate::combat::CombatState;

/// Synchronously checks for and applies Fairy In A Bottle or Lizard Tail when player HP hits 0.
pub fn try_revive(state: &mut CombatState) {
    if state.player.current_hp > 0 { return; }
    if state.player.has_relic(crate::content::relics::RelicId::MarkOfTheBloom) { return; }

    let fairy_slot = state.potions.iter().position(|p| {
        p.as_ref().map_or(false, |pot| pot.id == crate::content::potions::PotionId::FairyPotion)
    });
    if let Some(slot) = fairy_slot {
        state.potions[slot] = None;
        let mut potency = 0.3_f32;
        if state.player.has_relic(crate::content::relics::RelicId::SacredBark) {
            potency *= 2.0;
        }
        let heal_amount = (state.player.max_hp as f32 * potency) as i32;
        state.player.current_hp = heal_amount.max(1);
        return;
    }

    let lizard_unused = state.player.relics.iter()
        .find(|r| r.id == crate::content::relics::RelicId::LizardTail)
        .map_or(false, |r| !r.used_up);
    if lizard_unused {
        state.player.current_hp = std::cmp::max(1, state.player.max_hp / 2);
        if let Some(lt) = state.player.relics.iter_mut()
            .find(|r| r.id == crate::content::relics::RelicId::LizardTail) {
            lt.used_up = true;
            lt.counter = -2;
        }
    }
}

/// Centralized monster death handler.
/// Fires power on_death hooks, monster on_death, relic hooks, GremlinLeader/Darkling specials.
pub fn check_and_trigger_monster_death(state: &mut CombatState, target_id: usize) {
    let mut is_darkling_or_awakened = false;
    let mut is_gremlin_leader = false;
    let mut triggered_death = false;
    let mut dying_monster_type: Option<crate::content::monsters::EnemyId> = None;

    if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
        if m.current_hp <= 0 && !m.is_dying {
            m.is_dying = true;
            let m_id = crate::content::monsters::EnemyId::from_id(m.monster_type);
            dying_monster_type = m_id;
            is_gremlin_leader = m_id == Some(crate::content::monsters::EnemyId::GremlinLeader);
            is_darkling_or_awakened = m_id == Some(crate::content::monsters::EnemyId::Darkling) || m_id == Some(crate::content::monsters::EnemyId::AwakenedOne);
            triggered_death = true;
        }
    }

    if triggered_death {
        // Fire power on_death hooks BEFORE clearing (SporeCloud, Stasis, Unawakened, etc.)
        if let Some(powers) = state.power_db.get(&target_id).cloned() {
            for power in &powers {
                let death_actions = crate::content::powers::resolve_power_on_death(
                    power.power_type, state, target_id, power.amount
                );
                for a in death_actions {
                    state.action_queue.push_back(a);
                }
            }
        }

        if let Some(m_id) = dying_monster_type {
            if !is_darkling_or_awakened {
                let m_clone = state.monsters.iter().find(|m| m.id == target_id).unwrap().clone();
                let death_actions_on_entity = crate::content::monsters::resolve_on_death(m_id, state, &m_clone);
                for a in death_actions_on_entity {
                    state.action_queue.push_back(a);
                }
            }
        }

        let death_actions = crate::content::relics::hooks::on_monster_death(state, target_id);
        crate::engine::core::queue_actions(&mut state.action_queue, death_actions);

        if is_gremlin_leader {
            let minion_ids: Vec<_> = state.monsters.iter()
                .filter(|min| min.id != target_id && !min.is_dying)
                .map(|min| min.id)
                .collect();
            for minion_id in minion_ids {
                state.action_queue.push_back(Action::Escape { target: minion_id });
            }
        }
        if is_darkling_or_awakened {
            if let Some(m) = state.monsters.iter_mut().find(|m| m.id == target_id) {
                m.current_hp = 0;
                m.is_dying = false;
                m.current_intent = crate::combat::Intent::Unknown;
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
        Action::DamageAllEnemies { source, damages, damage_type, is_modified } =>
            damage::handle_damage_all_enemies(source, damages, damage_type, is_modified, state),
        Action::AttackDamageRandomEnemy { base_damage, damage_type: _ } =>
            damage::handle_attack_damage_random_enemy(base_damage, state),
        Action::DropkickDamageAndEffect { target, damage_info } =>
            damage::handle_dropkick(target, damage_info, state),
        Action::FiendFire { target, damage_info } =>
            damage::handle_fiend_fire(target, damage_info, state),
        Action::Feed { target, damage_info, max_hp_amount } =>
            damage::handle_feed(target, damage_info, max_hp_amount, state),
        Action::VampireDamage(info) => damage::handle_vampire_damage(info, state),
        Action::VampireDamageAllEnemies { source: _, damages, damage_type: _ } =>
            damage::handle_vampire_damage_all_enemies(damages, state),
        Action::LoseHp { target, amount } => damage::handle_lose_hp(target, amount, state),
        Action::GainBlock { target, amount } => damage::handle_gain_block(target, amount, state),
        Action::GainBlockRandomMonster { source, amount } =>
            damage::handle_gain_block_random_monster(source, amount, state),
        Action::LoseBlock { target, amount } => damage::handle_lose_block(target, amount, state),
        Action::Heal { target, amount } => damage::handle_heal(target, amount, state),
        Action::LimitBreak => damage::handle_limit_break(state),
        Action::BlockPerNonAttack { block_per_card } =>
            damage::handle_block_per_non_attack(block_per_card, state),
        Action::ExhaustAllNonAttack => damage::handle_exhaust_all_non_attack(state),
        Action::ExhaustRandomCard { amount } => damage::handle_exhaust_random_card(amount, state),

        // === Power domain ===
        Action::ApplyPower { source, target, power_id, amount } =>
            powers::handle_apply_power(source, target, power_id, amount, state),
        Action::RemovePower { target, power_id } =>
            powers::handle_remove_power(target, power_id, state),
        Action::RemoveAllDebuffs { target } =>
            powers::handle_remove_all_debuffs(target, state),
        Action::ApplyStasis { target_id } =>
            powers::handle_apply_stasis(target_id, state),
        Action::UpdatePowerExtraData { target, power_id, value } =>
            powers::handle_update_power_extra_data(target, power_id, value, state),
        Action::AwakenedRebirthClear { target } =>
            powers::handle_awakened_rebirth_clear(target, state),
        Action::GainEnergy { amount } => powers::handle_gain_energy(amount, state),
        Action::GainMaxHp { amount } => powers::handle_gain_max_hp(amount, state),
        Action::LoseMaxHp { target, amount } => powers::handle_lose_max_hp(target, amount, state),

        // === Card domain ===
        Action::DrawCards(amount) => cards::handle_draw_cards(amount, state),
        Action::EmptyDeckShuffle => cards::handle_empty_deck_shuffle(state),
        Action::DiscardCard { card_uuid } => cards::handle_discard_card(card_uuid, state),
        Action::ExhaustCard { card_uuid, source_pile } =>
            cards::handle_exhaust_card(card_uuid, source_pile, state),
        Action::MoveCard { card_uuid, from, to } =>
            cards::handle_move_card(card_uuid, from, to, state),
        Action::MakeTempCardInHand { card_id, amount, upgraded } =>
            cards::handle_make_temp_card_in_hand(card_id, amount, upgraded, state),
        Action::MakeTempCardInDiscard { card_id, amount, upgraded } =>
            cards::handle_make_temp_card_in_discard(card_id, amount, upgraded, state),
        Action::MakeTempCardInDrawPile { card_id, amount, random_spot, upgraded } =>
            cards::handle_make_temp_card_in_draw_pile(card_id, amount, random_spot, upgraded, state),
        Action::MakeCopyInHand { original, amount } =>
            cards::handle_make_copy_in_hand(original, amount, state),
        Action::MakeCopyInDiscard { original, amount } =>
            cards::handle_make_copy_in_discard(original, amount, state),
        Action::MakeTempCardInDiscardAndDeck { card_id, amount } =>
            cards::handle_make_temp_card_in_discard_and_deck(card_id, amount, state),
        Action::ReduceAllHandCosts { amount } =>
            cards::handle_reduce_all_hand_costs(amount, state),
        Action::UpgradeAllInHand => cards::handle_upgrade_all_in_hand(state),
        Action::UpgradeAllBurns => cards::handle_upgrade_all_burns(state),
        Action::UpgradeCard { card_uuid } => cards::handle_upgrade_card(card_uuid, state),
        Action::UpgradeRandomCard => cards::handle_upgrade_random_card(state),
        Action::ModifyCardMisc { card_uuid, amount } =>
            cards::handle_modify_card_misc(card_uuid, amount, state),
        Action::RandomizeHandCosts => cards::handle_randomize_hand_costs(state),
        Action::MummifiedHandEffect => cards::handle_mummified_hand_effect(state),
        Action::MakeRandomCardInHand { card_type, cost_for_turn } =>
            cards::handle_make_random_card_in_hand(card_type, cost_for_turn, state),
        Action::MakeRandomColorlessCardInHand { rarity: _, cost_for_turn } =>
            cards::handle_make_random_colorless_card_in_hand(cost_for_turn, state),
        Action::UseCardDone { should_exhaust } =>
            cards::handle_use_card_done(should_exhaust, state),
        Action::PlayCardDirect { card, target, purge } =>
            cards::handle_play_card_direct(card, target, purge, state),
        Action::UsePotion { slot, target } =>
            cards::handle_use_potion(slot, target, state),
        Action::DiscardPotion { slot } => {
            if slot < state.potions.len() {
                state.potions[slot] = None;
            }
        },
        Action::ObtainPotion => cards::handle_obtain_potion(state),
        Action::ObtainSpecificPotion(potion_id) => {
            if !state.player.has_relic(crate::content::relics::RelicId::Sozu) {
                if let Some(slot) = state.potions.iter().position(|p| p.is_none()) {
                    state.potions[slot] = Some(crate::content::potions::Potion::new(potion_id, 40000 + slot as u32));
                }
            }
        },
        Action::EndTurnTrigger => cards::handle_end_turn_trigger(state),
        Action::ClearCardQueue => cards::handle_clear_card_queue(state),
        Action::AddCardToMasterDeck { card_id } =>
            cards::handle_add_card_to_master_deck(card_id, state),
        Action::BattleStartTrigger => cards::handle_battle_start_trigger(state),

        // === Spawning / Monster lifecycle domain ===
        Action::SpawnMonster { monster_id, slot, current_hp, max_hp, logical_position } =>
            spawning::handle_spawn_monster(monster_id, slot, current_hp, max_hp, logical_position, state),
        Action::SpawnMonsterSmart { monster_id, logical_position, current_hp, max_hp } =>
            spawning::handle_spawn_monster_smart(monster_id, logical_position, current_hp, max_hp, state),
        Action::Suicide { target } => spawning::handle_suicide(target, state),
        Action::Escape { target } => spawning::handle_escape(target, state),
        Action::RollMonsterMove { monster_id } =>
            spawning::handle_roll_monster_move(monster_id, state),
        Action::SetMonsterMove { monster_id, next_move_byte, intent } =>
            spawning::handle_set_monster_move(monster_id, next_move_byte, intent, state),
        Action::UpdateRelicCounter { relic_id, counter } =>
            spawning::handle_update_relic_counter(relic_id, counter, state),
        Action::UpdateRelicAmount { relic_id, amount } =>
            spawning::handle_update_relic_amount(relic_id, amount, state),
        Action::UpdateRelicUsedUp { relic_id, used_up } =>
            spawning::handle_update_relic_used_up(relic_id, used_up, state),

        // === Pass-through / unhandled ===
        // These variants exist but have no handler yet or are handled inline elsewhere
        Action::PlayCard { .. } | Action::UseCard { .. } | Action::StartTurnTrigger | Action::PostDrawTrigger
        | Action::AbortDeath { .. } | Action::FleeCombat | Action::ExecuteMonsterTurn(_)
        | Action::SpawnEncounter { .. } | Action::Scry(_) | Action::ChannelOrb(_)
        | Action::EvokeOrb | Action::TriggerPassiveOrbs | Action::IncreaseMaxOrb(_)
        | Action::EnterStance(_) | Action::SuspendForHandSelect { .. }
        | Action::SuspendForGridSelect { .. } | Action::SuspendForDiscovery { .. }
        | Action::SuspendForStanceChoice | Action::SuspendForCardReward { .. }
        | Action::PlayTopCard { .. } => {
            #[cfg(debug_assertions)]
            eprintln!("[action_handlers] Unhandled action: {:?}", action);
        }
    }
}
