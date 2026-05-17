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

use crate::content::powers::store;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

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
        let heal_amount =
            crate::content::relics::hooks::on_calculate_heal(state, heal_amount.max(1));
        state.entities.player.current_hp =
            (state.entities.player.current_hp + heal_amount).min(state.entities.player.max_hp);
        return;
    }

    let lizard_unused = state
        .entities
        .player
        .relics
        .iter()
        .find(|r| r.id == crate::content::relics::RelicId::LizardTail)
        .map_or(false, |r| r.counter == -1 && !r.used_up);
    if lizard_unused {
        let heal_amount = crate::content::relics::hooks::on_calculate_heal(
            state,
            crate::content::relics::lizard_tail::revive_amount(state.entities.player.max_hp),
        );
        state.entities.player.current_hp =
            (state.entities.player.current_hp + heal_amount).min(state.entities.player.max_hp);
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
/// Fires power on_death hooks, monster on_death, relic hooks, and Darkling specials.
pub fn check_and_trigger_monster_death(state: &mut CombatState, target_id: usize) {
    let mut is_awakened_rebirth = false;
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
                state.queue_action_back(a);
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
                    state.queue_action_back(a);
                }
            }
        }

        let death_actions = crate::content::relics::hooks::on_monster_death(state, target_id);
        state.queue_actions(death_actions);
        if is_awakened_rebirth {
            let mut cleared_protocol_monster_id = None;
            if let Some(m) = state
                .entities
                .monsters
                .iter_mut()
                .find(|m| m.id == target_id)
            {
                m.current_hp = 0;
                m.is_dying = false;
                cleared_protocol_monster_id = Some(m.id);
                if dying_monster_type == Some(crate::content::monsters::EnemyId::AwakenedOne) {
                    m.half_dead = true;
                }
            }
            if let Some(monster_id) = cleared_protocol_monster_id {
                state.clear_monster_protocol_observation(monster_id);
            }
        }
    }
}

/// Executes one queued action by delegating to the relevant domain handler.
pub fn execute_action(action: Action, state: &mut CombatState) {
    match action {
        // === Damage domain ===
        Action::Damage(info) => damage::handle_damage(info, state),
        Action::Barrage { damage } => cards::handle_barrage(damage, state),
        Action::PummelDamage(info) => damage::handle_pummel_damage(info, state),
        Action::BaneDamage(info) => damage::handle_bane_damage(info, state),
        Action::WallopDamage(info) => damage::handle_wallop_damage(info, state),
        Action::Judgement { target, cutoff } => damage::handle_judgement(target, cutoff, state),
        Action::InstantKill { target } => damage::handle_instant_kill(target, state),
        Action::TriggerMarks { card_id } => damage::handle_trigger_marks(card_id, state),
        Action::DamagePerAttackPlayed(info) => damage::handle_damage_per_attack_played(info, state),
        Action::HeelHook(info) => damage::handle_heel_hook(info, state),
        Action::Flechettes(info) => damage::handle_flechettes(info, state),
        #[rustfmt::skip] Action::MonsterAttack { source, target, base_damage, damage_kind } => damage::handle_monster_attack(source, target, base_damage, damage_kind, state),
        Action::DamageAllEnemies {
            source,
            damages,
            damage_type,
            is_modified,
        } => damage::handle_damage_all_enemies(source, damages, damage_type, is_modified, state),
        Action::OrbDamage {
            source,
            target,
            base_damage,
        } => damage::handle_orb_damage(source, target, base_damage, state),
        Action::OrbDamageRandomEnemy {
            source,
            base_damage,
        } => damage::handle_orb_damage_random_enemy(source, base_damage, state),
        Action::OrbDamageAllEnemies {
            source,
            base_damage,
        } => damage::handle_orb_damage_all_enemies(source, base_damage, state),
        Action::Whirlwind {
            damages,
            damage_type,
            free_to_play_once,
            energy_on_use,
        } => damage::handle_whirlwind(
            damages,
            damage_type,
            free_to_play_once,
            energy_on_use,
            state,
        ),
        Action::Skewer {
            target,
            damage_info,
            free_to_play_once,
            energy_on_use,
        } => damage::handle_skewer(target, damage_info, free_to_play_once, energy_on_use, state),
        Action::Sunder {
            target,
            damage_info,
            energy_gain,
        } => damage::handle_sunder(target, damage_info, energy_gain, state),
        Action::DamageRandomEnemy {
            source,
            base_damage,
            damage_type,
        } => damage::handle_damage_random_enemy(source, base_damage, damage_type, state),
        Action::AttackDamageRandomEnemyCard { card } => {
            damage::handle_attack_damage_random_enemy_card(*card, state)
        }
        Action::DropkickDamageAndEffect {
            target,
            damage_info,
        } => damage::handle_dropkick(target, damage_info, state),
        Action::SpotWeakness { target, amount } => {
            powers::handle_spot_weakness(target, amount, state)
        }
        Action::ApplyWeakIfTargetAttacking { target, amount } => {
            powers::handle_apply_weak_if_target_attacking(target, amount, state)
        }
        Action::FearNoEvil {
            target,
            damage_info,
        } => handle_fear_no_evil(target, damage_info, state),
        Action::Ftl {
            target,
            damage_info,
            card_play_count,
        } => damage::handle_ftl(target, damage_info, card_play_count, state),
        Action::Doppelganger {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => powers::handle_doppelganger(upgraded, free_to_play_once, energy_on_use, state),
        Action::Malaise {
            target,
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => powers::handle_malaise(target, upgraded, free_to_play_once, energy_on_use, state),
        Action::Collect {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => powers::handle_collect(upgraded, free_to_play_once, energy_on_use, state),
        Action::ConjureBlade {
            free_to_play_once,
            energy_on_use,
        } => cards::handle_conjure_blade(free_to_play_once, energy_on_use, state),
        Action::Meditate { amount } => cards::handle_meditate(amount, state),
        Action::FiendFire {
            target,
            damage_info,
        } => damage::handle_fiend_fire(target, damage_info, state),
        Action::Feed {
            target,
            damage_info,
            max_hp_amount,
        } => damage::handle_feed(target, damage_info, max_hp_amount, state),
        Action::LessonLearned {
            target,
            damage_info,
        } => damage::handle_lesson_learned(target, damage_info, state),
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
        Action::PoisonLoseHp { target, amount } => {
            damage::handle_poison_lose_hp(target, amount, state)
        }
        Action::SetCurrentHp { target, hp } => damage::handle_set_current_hp(target, hp, state),
        Action::GainBlock { target, amount } => damage::handle_gain_block(target, amount, state),
        Action::DoubleBlock { target } => damage::handle_double_block(target, state),
        Action::GainBlockRandomMonster { source, amount } => {
            damage::handle_gain_block_random_monster(source, amount, state)
        }
        Action::LoseBlock { target, amount } => damage::handle_lose_block(target, amount, state),
        Action::RemoveAllBlock { target } => damage::handle_remove_all_block(target, state),
        Action::Heal { target, amount } => damage::handle_heal(target, amount, state),
        Action::GainGold { amount } => damage::handle_gain_gold(amount, state),
        Action::StealPlayerGold { thief_id, amount } => {
            damage::handle_steal_player_gold(thief_id, amount, state)
        }
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
        Action::ApplyPowerWithPayload {
            source,
            target,
            power_id,
            amount,
            instance_id,
            extra_data,
            payload,
        } => powers::handle_apply_power_with_payload(
            source,
            target,
            power_id,
            amount,
            instance_id,
            extra_data,
            payload,
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
        Action::DoubleEnergy => powers::handle_double_energy(state),
        Action::GainEnergyIfDiscardedThisTurn { amount } => {
            if state.turn.counters.cards_discarded_this_turn > 0 {
                powers::handle_gain_energy(amount, state);
            }
        }
        Action::FollowUp => handle_follow_up(state),
        Action::Sanctity { draw_amount } => handle_sanctity(draw_amount, state),
        Action::CrushJoints { target, amount } => handle_crush_joints(target, amount, state),
        Action::SashWhip { target, amount } => handle_sash_whip(target, amount, state),
        Action::GainMaxHp { amount } => powers::handle_gain_max_hp(amount, state),
        Action::LoseMaxHp { target, amount } => powers::handle_lose_max_hp(target, amount, state),

        // === Card domain ===
        Action::DrawCards(amount) => cards::handle_draw_cards(amount, state),
        Action::InnerPeace { draw_amount } => handle_inner_peace(draw_amount, state),
        Action::Indignation { amount } => handle_indignation(amount, state),
        Action::DrawForUniqueOrbTypes {
            amount_per_orb_type,
        } => cards::handle_draw_for_unique_orb_types(amount_per_orb_type, state),
        Action::DrawCardsWithHistory {
            amount,
            clear_history,
        } => cards::handle_draw_cards_with_history(amount, clear_history, state),
        Action::EscapePlanBlockIfSkill { block } => {
            cards::handle_escape_plan_block_if_skill(block, state)
        }
        Action::ScrapeFollowUp => cards::handle_scrape_follow_up(state),
        Action::ExpertiseDraw { target_hand_size } => {
            cards::handle_expertise_draw(target_hand_size, state)
        }
        Action::CalculatedGamble { draw_extra } => {
            cards::handle_calculated_gamble(draw_extra, state)
        }
        Action::BladeFury { upgraded } => cards::handle_blade_fury(upgraded, state),
        Action::ApplyBulletTime => cards::handle_apply_bullet_time(state),
        Action::UnloadNonAttack => cards::handle_unload_non_attack(state),
        Action::RetainNonEtherealHandCards => cards::handle_retain_non_ethereal_hand_cards(state),
        Action::EmptyDeckShuffle => cards::handle_empty_deck_shuffle(state),
        Action::ShuffleDiscardIntoDraw => cards::handle_shuffle_discard_into_draw(state),
        Action::ShuffleAllIntoDraw => cards::handle_shuffle_all_into_draw(state),
        Action::ShuffleDrawPile { trigger_relics } => {
            cards::handle_shuffle_draw_pile(trigger_relics, state)
        }
        Action::DiscardCard { card_uuid } => cards::handle_discard_card(card_uuid, state),
        Action::DiscardFromHand {
            amount,
            random,
            end_turn,
        } => cards::handle_discard_from_hand(amount, random, end_turn, state),
        Action::DiscardToHand {
            card_uuid,
            cost_for_turn,
        } => cards::handle_discard_to_hand(card_uuid, cost_for_turn, state),
        Action::AllCostToHand { cost_target } => cards::handle_all_cost_to_hand(cost_target, state),
        Action::ExhaustCard {
            card_uuid,
            source_pile,
        } => cards::handle_exhaust_card(card_uuid, source_pile, state),
        Action::ExhaustFromHand {
            amount,
            random,
            any_number,
            can_pick_zero,
        } => cards::handle_exhaust_from_hand(amount, random, any_number, can_pick_zero, state),
        Action::Recycle => cards::handle_recycle(state),
        Action::MoveCard {
            card_uuid,
            from,
            to,
        } => cards::handle_move_card(card_uuid, from, to, state),
        Action::PutOnDeck { amount, random } => cards::handle_put_on_deck(amount, random, state),
        Action::Forethought { upgraded } => cards::handle_forethought(upgraded, state),
        Action::DiscardPileToTopOfDeck => cards::handle_discard_pile_to_top_of_deck(state),
        Action::ExhumeCard { card_uuid, upgrade } => {
            cards::handle_exhume_card(card_uuid, upgrade, state)
        }
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
            to_bottom,
            upgraded,
        } => cards::handle_make_temp_card_in_draw_pile(
            card_id,
            amount,
            random_spot,
            to_bottom,
            upgraded,
            state,
        ),
        Action::MakeCopyInHand { original, amount } => {
            cards::handle_make_copy_in_hand(original, amount, state)
        }
        Action::MakeCopyInDrawPile {
            original,
            amount,
            random_spot,
            to_bottom,
        } => cards::handle_make_copy_in_draw_pile(original, amount, random_spot, to_bottom, state),
        Action::MakeCopyInDiscard { original, amount } => {
            cards::handle_make_copy_in_discard(original, amount, state)
        }
        Action::ReturnStasisCard { card_uuid, to_hand } => {
            cards::handle_return_stasis_card(card_uuid, to_hand, state)
        }
        Action::MakeTempCardInDiscardAndDeck { card_id, amount } => {
            cards::handle_make_temp_card_in_discard_and_deck(card_id, amount, state)
        }
        Action::ReduceAllHandCosts { amount } => cards::handle_reduce_all_hand_costs(amount, state),
        Action::ReduceRetainedHandCosts { amount } => {
            cards::handle_reduce_retained_hand_costs(amount, state)
        }
        Action::Enlightenment { permanent } => cards::handle_enlightenment(permanent, state),
        Action::Halt { block, additional } => cards::handle_halt(block, additional, state),
        Action::Madness => cards::handle_madness(state),
        Action::UpgradeAllInHand => cards::handle_upgrade_all_in_hand(state),
        Action::UpgradeAllCardsInCombat => cards::handle_upgrade_all_cards_in_combat(state),
        Action::UpgradeAllBurns => cards::handle_upgrade_all_burns(state),
        Action::UpgradeCard { card_uuid } => cards::handle_upgrade_card(card_uuid, state),
        Action::UpgradeRandomCard => cards::handle_upgrade_random_card(state),
        Action::ModifyCardMisc { card_uuid, amount } => {
            cards::handle_modify_card_misc(card_uuid, amount, state)
        }
        Action::ModifyCardDamage { card_uuid, amount } => {
            cards::handle_modify_card_damage(card_uuid, amount, state)
        }
        Action::Gash { card_uuid, amount } => cards::handle_gash(card_uuid, amount, state),
        Action::ModifyCardBlock { card_uuid, amount } => {
            cards::handle_modify_card_block(card_uuid, amount, state)
        }
        Action::ReduceCardCostForCombat { card_uuid, amount } => {
            cards::handle_reduce_card_cost_for_combat(card_uuid, amount, state)
        }
        Action::RandomizeHandCosts => cards::handle_randomize_hand_costs(state),
        Action::MakeRandomCardInHand {
            card_type,
            cost_for_turn,
        } => cards::handle_make_random_card_in_hand(card_type, cost_for_turn, state),
        Action::Nightmare { amount } => cards::handle_nightmare(amount, state),
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
        Action::Transmutation {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => cards::handle_transmutation(upgraded, free_to_play_once, energy_on_use, state),
        Action::AggregateEnergy { divide_amount } => {
            cards::handle_aggregate_energy(divide_amount, state)
        }
        Action::Tempest {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => cards::handle_tempest(upgraded, free_to_play_once, energy_on_use, state),
        Action::MultiCast {
            upgraded,
            free_to_play_once,
            energy_on_use,
        } => cards::handle_multicast(upgraded, free_to_play_once, energy_on_use, state),
        Action::ReinforcedBody {
            block_amount,
            free_to_play_once,
            energy_on_use,
        } => cards::handle_reinforced_body(block_amount, free_to_play_once, energy_on_use, state),
        Action::UseCardDone {
            should_exhaust,
            trigger_after_use_hooks,
        } => cards::handle_use_card_done(should_exhaust, trigger_after_use_hooks, state),
        Action::UseCardAfterUseHooks { card } => {
            cards::handle_use_card_after_use_hooks(*card, state)
        }
        Action::QueueEarlyEndTurn => cards::handle_queue_early_end_turn(state),
        Action::SkipEnemiesTurn => cards::handle_skip_enemies_turn(state),
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
        Action::QueuePlayTopCardToBottom { target, exhaust } => {
            cards::handle_queue_play_top_card_to_bottom(target, exhaust, state)
        }
        Action::UsePotion { slot, target } => cards::handle_use_potion(slot, target, state),
        Action::DiscardPotion { slot } => {
            if slot < state.entities.potions.len() {
                state.entities.potions[slot] = None;
            }
        }
        Action::ObtainPotion => cards::handle_obtain_potion(state),
        Action::ObtainSpecificPotion(potion_id) => {
            cards::obtain_specific_potion_if_allowed(state, potion_id);
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
        } => {
            let _ = spawning::handle_spawn_monster(
                monster_id,
                slot,
                current_hp,
                max_hp,
                logical_position,
                protocol_draw_x,
                is_minion,
                state,
            );
        }
        Action::SpawnMonsterSmart {
            monster_id,
            logical_position,
            hp,
            protocol_draw_x,
            is_minion,
        } => spawning::handle_spawn_monster_smart(
            monster_id,
            logical_position,
            hp,
            protocol_draw_x,
            is_minion,
            state,
        ),
        Action::SpawnCollectorTorch {
            collector_id,
            slot,
            logical_position,
            hp,
            protocol_draw_x,
        } => spawning::handle_spawn_collector_torch(
            collector_id,
            slot,
            logical_position,
            hp,
            protocol_draw_x,
            state,
        ),
        Action::Suicide { target } => spawning::handle_suicide(target, state),
        Action::Escape { target } => spawning::handle_escape(target, state),
        Action::AddCombatReward { item } => spawning::handle_add_combat_reward(item, state),
        Action::RollMonsterMove { monster_id } => {
            spawning::handle_roll_monster_move(monster_id, state)
        }
        Action::SetMonsterMove {
            monster_id,
            next_move_byte,
            planned_steps,
            planned_visible_spec,
        } => spawning::handle_set_monster_move(
            monster_id,
            next_move_byte,
            planned_steps,
            planned_visible_spec,
            state,
        ),
        Action::UpdateMonsterRuntime { monster_id, patch } => {
            spawning::handle_update_monster_runtime(monster_id, patch, state)
        }
        Action::GuardianModeShiftThresholdTriggered {
            monster_id,
            hp_lost,
        } => {
            crate::content::monsters::exordium::the_guardian::handle_mode_shift_threshold_triggered(
                monster_id, hp_lost, state,
            )
        }
        Action::GuardianEnterDefensiveMode {
            monster_id,
            next_threshold,
        } => crate::content::monsters::exordium::the_guardian::handle_enter_defensive_mode(
            monster_id,
            next_threshold,
            state,
        ),
        Action::ReviveMonster { target } => spawning::handle_revive_monster(target, state),
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
        Action::DecreaseMaxOrb(amount) => handle_decrease_max_orb(amount, state),
        Action::ChannelOrb(orb_id) => handle_channel_orb(orb_id, state),
        Action::ChannelRandomOrbs { amount } => handle_channel_random_orbs(amount, state),
        Action::ChannelOrbEntity { orb } => handle_channel_orb_entity(orb, state),
        Action::EvokeOrb => crate::content::orbs::hooks::evoke_next_orb_now(state),
        Action::EvokeOrbWithoutRemoving => {
            crate::content::orbs::hooks::evoke_next_orb_without_removing_now(state)
        }
        Action::Fission { upgraded } => handle_fission(upgraded, state),
        Action::RemoveAllOrbs => crate::content::orbs::hooks::remove_all_orbs_now(state),
        Action::EvokeAllOrbs => crate::content::orbs::hooks::queue_evoke_all_orbs_now(state),
        Action::RedoOrb => handle_redo_orb(state),
        Action::TriggerStartOfTurnOrbs => {
            crate::content::orbs::hooks::trigger_start_of_turn_orbs_now(state)
        }
        Action::TriggerEndOfTurnOrbs => {
            crate::content::orbs::hooks::trigger_end_of_turn_orbs_now(state)
        }
        Action::TriggerImpulseOrbs => crate::content::orbs::hooks::trigger_impulse_orbs_now(state),
        Action::TriggerFirstOrbStartAndEnd { times } => {
            crate::content::orbs::hooks::trigger_first_orb_start_and_end_now(state, times)
        }
        Action::TriggerDarkImpulseOrbs => {
            crate::content::orbs::hooks::trigger_dark_impulse_orbs_now(state)
        }
        Action::EnterStance(stance) => handle_enter_stance(&stance, state),

        // === Pass-through / unhandled ===
        // These variants exist but have no handler yet or are handled inline elsewhere
        Action::PlayCard { .. }
        | Action::UseCard { .. }
        | Action::StartTurnTrigger
        | Action::FleeCombat
        | Action::AbortDeath { .. }
        | Action::ExecuteMonsterTurn(_)
        | Action::SpawnEncounter { .. }
        | Action::Scry(_) => {
            #[cfg(debug_assertions)]
            eprintln!("[action_handlers] Unhandled action: {:?}", action);
        }
        Action::SuspendForHandSelect { .. }
        | Action::SuspendForGridSelect { .. }
        | Action::SuspendForDiscovery { .. }
        | Action::SuspendForForeignInfluence { .. }
        | Action::SuspendForStanceChoice
        | Action::SuspendForChooseOne { .. }
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

fn handle_decrease_max_orb(amount: u8, state: &mut CombatState) {
    for _ in 0..amount {
        if state.entities.player.max_orbs == 0 {
            return;
        }
        state.entities.player.max_orbs = state.entities.player.max_orbs.saturating_sub(1);
        if !state.entities.player.orbs.is_empty() {
            state.entities.player.orbs.pop();
        }
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
    let mut new_orb = crate::runtime::combat::OrbEntity::new(orb_id);
    if !matches!(
        new_orb.id,
        crate::runtime::combat::OrbId::Empty | crate::runtime::combat::OrbId::Plasma
    ) {
        let focus = crate::content::powers::store::power_amount(
            state,
            0,
            crate::content::powers::PowerId::Focus,
        );
        new_orb.passive_amount = (new_orb.base_passive_amount + focus).max(0);
        if new_orb.id != crate::runtime::combat::OrbId::Dark {
            new_orb.evoke_amount = (new_orb.base_evoke_amount + focus).max(0);
        }
    }
    if let Some(empty_slot) = state
        .entities
        .player
        .orbs
        .iter()
        .position(|orb| orb.id == crate::runtime::combat::OrbId::Empty)
    {
        state.entities.player.orbs[empty_slot] = new_orb;
        state.turn.record_orb_channeled(orb_id);
    } else {
        state.queue_action_front(Action::ChannelOrb(orb_id));
        state.queue_action_front(Action::EvokeOrb);
    }
}

fn handle_channel_random_orbs(amount: u8, state: &mut CombatState) {
    use crate::runtime::combat::OrbId;

    let mut orbs = Vec::with_capacity(amount as usize);
    for _ in 0..amount {
        let roll = state.rng.card_random_rng.random(3);
        let orb = match roll {
            0 => OrbId::Dark,
            1 => OrbId::Frost,
            2 => OrbId::Lightning,
            _ => OrbId::Plasma,
        };
        orbs.push(orb);
    }

    for orb in orbs.into_iter().rev() {
        state.queue_action_front(Action::ChannelOrb(orb));
    }
}

fn handle_channel_orb_entity(orb: crate::runtime::combat::OrbEntity, state: &mut CombatState) {
    let orb_id = orb.id;
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
    if let Some(empty_slot) = state
        .entities
        .player
        .orbs
        .iter()
        .position(|existing| existing.id == crate::runtime::combat::OrbId::Empty)
    {
        state.entities.player.orbs[empty_slot] = orb;
        state.turn.record_orb_channeled(orb_id);
    }
}

fn handle_fission(upgraded: bool, state: &mut CombatState) {
    let orb_count = crate::content::orbs::hooks::filled_orb_count(state) as i32;
    state.queue_action_front(Action::DrawCards(orb_count.max(0) as u32));
    state.queue_action_front(Action::GainEnergy { amount: orb_count });
    if upgraded {
        state.queue_action_front(Action::EvokeAllOrbs);
    } else {
        state.queue_action_front(Action::RemoveAllOrbs);
    }
}

fn handle_redo_orb(state: &mut CombatState) {
    let Some(orb) = state.entities.player.orbs.first().cloned() else {
        return;
    };
    if orb.id == crate::runtime::combat::OrbId::Empty {
        return;
    }
    state.queue_action_front(Action::ChannelOrbEntity { orb });
    state.queue_action_front(Action::EvokeOrb);
}

fn handle_inner_peace(draw_amount: u32, state: &mut CombatState) {
    if state.entities.player.stance == crate::runtime::combat::StanceId::Calm {
        state.queue_action_front(Action::DrawCards(draw_amount));
    } else {
        state.queue_action_front(Action::EnterStance("Calm".to_string()));
    }
}

fn handle_indignation(amount: i32, state: &mut CombatState) {
    if state.entities.player.stance == crate::runtime::combat::StanceId::Wrath {
        let targets: Vec<_> = state
            .entities
            .monsters
            .iter()
            .map(|monster| monster.id)
            .collect();
        for target in targets {
            state.queue_action_back(Action::ApplyPower {
                source: 0,
                target,
                power_id: crate::content::powers::PowerId::Vulnerable,
                amount,
            });
        }
    } else {
        state.queue_action_back(Action::EnterStance("Wrath".to_string()));
    }
}

fn handle_follow_up(state: &mut CombatState) {
    if previous_played_card_type(state) == Some(crate::content::cards::CardType::Attack) {
        state.queue_action_front(Action::GainEnergy { amount: 1 });
    }
}

fn handle_sanctity(draw_amount: u32, state: &mut CombatState) {
    if previous_played_card_type(state) == Some(crate::content::cards::CardType::Skill) {
        state.queue_action_front(Action::DrawCards(draw_amount));
    }
}

fn handle_crush_joints(target: usize, amount: i32, state: &mut CombatState) {
    if previous_played_card_type(state) == Some(crate::content::cards::CardType::Skill) {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target,
            power_id: crate::content::powers::PowerId::Vulnerable,
            amount,
        });
    }
}

fn handle_sash_whip(target: usize, amount: i32, state: &mut CombatState) {
    if previous_played_card_type(state) == Some(crate::content::cards::CardType::Attack) {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target,
            power_id: crate::content::powers::PowerId::Weak,
            amount,
        });
    }
}

fn previous_played_card_type(state: &CombatState) -> Option<crate::content::cards::CardType> {
    let played = &state.turn.counters.card_ids_played_this_combat;
    if played.len() < 2 {
        return None;
    }
    let previous_card_id = played[played.len() - 2];
    Some(crate::content::cards::get_card_definition(previous_card_id).card_type)
}

fn handle_fear_no_evil(
    target: usize,
    damage_info: crate::runtime::action::DamageInfo,
    state: &mut CombatState,
) {
    if monster_has_java_attack_intent_for_fear_no_evil(state, target) {
        state.queue_action_front(Action::EnterStance("Calm".to_string()));
    }
    state.queue_action_front(Action::Damage(damage_info));
}

fn monster_has_java_attack_intent_for_fear_no_evil(state: &CombatState, target: usize) -> bool {
    if state
        .monster_protocol_visible_intent(target)
        .is_java_attack_intent()
    {
        return true;
    }

    state
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == target)
        .is_some_and(|monster| {
            monster
                .move_state
                .planned_visible_spec
                .as_ref()
                .is_some_and(|spec| spec.attack().is_some())
                || monster
                    .move_state
                    .planned_steps
                    .as_ref()
                    .is_some_and(|steps| {
                        steps.iter().any(|step| {
                            matches!(step, crate::semantics::combat::MoveStep::Attack(_))
                        })
                    })
        })
}

fn handle_enter_stance(stance: &str, state: &mut CombatState) {
    if crate::content::powers::store::has_power(
        state,
        0,
        crate::content::powers::PowerId::CannotChangeStance,
    ) {
        return;
    }
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
    for power in &crate::content::powers::store::powers_snapshot_for(state, 0) {
        for action in crate::content::powers::resolve_power_on_change_stance(
            power.power_type,
            0,
            power.amount,
            old_stance,
            new_stance,
        ) {
            state.queue_action_back(action);
        }
    }
    crate::content::relics::hooks::on_change_stance(state, old_stance, new_stance);
    if old_stance == crate::runtime::combat::StanceId::Calm {
        state.queue_action_back(Action::GainEnergy { amount: 2 });
    }
    state.entities.player.stance = new_stance;
    if new_stance == crate::runtime::combat::StanceId::Divinity {
        state.queue_action_back(Action::GainEnergy { amount: 3 });
    }
    let card_actions = crate::content::cards::hooks::on_change_stance_from_discard(state);
    state.queue_actions(card_actions);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::powers::PowerId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::{Power, PowerPayload, StanceId};
    use crate::test_support::blank_test_combat;

    #[test]
    fn cannot_change_stance_power_blocks_change_stance_action() {
        let mut state = blank_test_combat();
        state.entities.player.stance = StanceId::Calm;
        state.turn.energy = 0;
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::CannotChangeStance,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_enter_stance("Wrath", &mut state);

        assert_eq!(state.entities.player.stance, StanceId::Calm);
        assert_eq!(
            state.turn.energy, 0,
            "Java ChangeStanceAction returns before oldStance.onExitStance when CannotChangeStancePower is present"
        );
    }

    #[test]
    fn stance_energy_is_queued_in_java_change_stance_order() {
        let mut state = blank_test_combat();
        state.entities.player.stance = StanceId::Calm;
        state.turn.energy = 0;
        state
            .entities
            .player
            .add_relic(RelicState::new(RelicId::VioletLotus));
        state.zones.discard_pile = vec![crate::runtime::combat::CombatCard::new(
            CardId::FlurryOfBlows,
            91001,
        )];
        state.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::RushdownPower,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        handle_enter_stance("Wrath", &mut state);

        assert_eq!(state.entities.player.stance, StanceId::Wrath);
        assert_eq!(
            state.turn.energy, 0,
            "Java CalmStance.onExitStance queues GainEnergyAction instead of mutating energy immediately"
        );
        assert_eq!(state.pop_next_action(), Some(Action::DrawCards(2)));
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainEnergy { amount: 1 })
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainEnergy { amount: 2 })
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::DiscardToHand {
                card_uuid: 91001,
                cost_for_turn: None,
            })
        );
        assert!(state.pop_next_action().is_none());
    }

    #[test]
    fn divinity_enter_energy_is_queued_after_stance_changes() {
        let mut state = blank_test_combat();
        state.turn.energy = 0;

        handle_enter_stance("Divinity", &mut state);

        assert_eq!(state.entities.player.stance, StanceId::Divinity);
        assert_eq!(
            state.turn.energy, 0,
            "Java DivinityStance.onEnterStance queues GainEnergyAction instead of mutating energy immediately"
        );
        assert_eq!(
            state.pop_next_action(),
            Some(Action::GainEnergy { amount: 3 })
        );
        assert!(state.pop_next_action().is_none());
    }
}
