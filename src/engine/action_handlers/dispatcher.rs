use super::{cards, damage, orbs, powers, spawning, stances};
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;

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
        } => stances::handle_fear_no_evil(target, damage_info, state),
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
        Action::FollowUp => stances::handle_follow_up(state),
        Action::Sanctity { draw_amount } => stances::handle_sanctity(draw_amount, state),
        Action::CrushJoints { target, amount } => {
            stances::handle_crush_joints(target, amount, state)
        }
        Action::SashWhip { target, amount } => stances::handle_sash_whip(target, amount, state),
        Action::GainMaxHp { amount } => powers::handle_gain_max_hp(amount, state),
        Action::LoseMaxHp { target, amount } => powers::handle_lose_max_hp(target, amount, state),

        // === Card domain ===
        Action::DrawCards(amount) => cards::handle_draw_cards(amount, state),
        Action::InnerPeace { draw_amount } => stances::handle_inner_peace(draw_amount, state),
        Action::Indignation { amount } => stances::handle_indignation(amount, state),
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
        Action::MakeConstructedCopyInHand { original, amount } => {
            cards::handle_make_constructed_copy_in_hand(original, amount, state)
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
            if let Some(potion_slot) = state.entities.potions.get_mut(slot) {
                if potion_slot
                    .as_ref()
                    .is_some_and(|potion| potion.can_discard)
                {
                    *potion_slot = None;
                }
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
        Action::RedSkullBattleStartCheck => {
            crate::content::relics::red_skull::battle_start_check(state);
        }
        Action::DodecahedronTurnStartCheck => {
            crate::content::relics::dodecahedron::Dodecahedron::turn_start_check(state);
        }

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
        Action::SpawnGremlinLeaderMinion {
            leader_id,
            slot,
            monster_id,
            logical_position,
            hp,
            protocol_draw_x,
        } => spawning::handle_spawn_gremlin_leader_minion(
            leader_id,
            slot,
            monster_id,
            logical_position,
            hp,
            protocol_draw_x,
            state,
        ),
        Action::SpawnReptomancerDagger {
            reptomancer_id,
            slot,
            logical_position,
            hp,
            protocol_draw_x,
        } => spawning::handle_spawn_reptomancer_dagger(
            reptomancer_id,
            slot,
            logical_position,
            hp,
            protocol_draw_x,
            state,
        ),
        Action::Suicide {
            target,
            trigger_relics,
        } => spawning::handle_suicide(target, trigger_relics, state),
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

        Action::IncreaseMaxOrb(amount) => orbs::handle_increase_max_orb(amount, state),
        Action::DecreaseMaxOrb(amount) => orbs::handle_decrease_max_orb(amount, state),
        Action::ChannelOrb(orb_id) => orbs::handle_channel_orb(orb_id, state),
        Action::ChannelRandomOrbs { amount } => orbs::handle_channel_random_orbs(amount, state),
        Action::ChannelOrbEntity { orb } => orbs::handle_channel_orb_entity(orb, state),
        Action::EvokeOrb => crate::content::orbs::hooks::evoke_next_orb_now(state),
        Action::EvokeOrbWithoutRemoving => {
            crate::content::orbs::hooks::evoke_next_orb_without_removing_now(state)
        }
        Action::Fission { upgraded } => orbs::handle_fission(upgraded, state),
        Action::RemoveAllOrbs => crate::content::orbs::hooks::remove_all_orbs_now(state),
        Action::EvokeAllOrbs => crate::content::orbs::hooks::queue_evoke_all_orbs_now(state),
        Action::RedoOrb => orbs::handle_redo_orb(state),
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
        Action::EnterStance(stance) => stances::handle_enter_stance(&stance, state),

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
