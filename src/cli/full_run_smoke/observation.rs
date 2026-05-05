use super::*;

pub fn build_observation(ctx: &EpisodeContext) -> RunObservationV0 {
    let combat = ctx.combat_state.as_ref();
    let active_hp = combat
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp);
    let active_max_hp = combat
        .map(|combat| combat.entities.player.max_hp)
        .unwrap_or(ctx.run_state.max_hp);

    RunObservationV0 {
        schema_version: FULL_RUN_OBSERVATION_SCHEMA_VERSION.to_string(),
        decision_type: decision_type(&ctx.engine_state).to_string(),
        engine_state: engine_state_label(&ctx.engine_state).to_string(),
        act: ctx.run_state.act_num,
        floor: ctx.run_state.floor_num,
        current_room: ctx
            .run_state
            .map
            .get_current_room_type()
            .map(|room_type| format!("{room_type:?}")),
        current_hp: active_hp,
        max_hp: active_max_hp,
        hp_ratio_milli: if active_max_hp > 0 {
            active_hp * 1000 / active_max_hp
        } else {
            0
        },
        gold: ctx.run_state.gold,
        deck_size: ctx.run_state.master_deck.len(),
        relic_count: ctx.run_state.relics.len(),
        potion_slots: ctx.run_state.potions.len(),
        filled_potion_slots: ctx
            .run_state
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count(),
        deck: build_deck_observation(&ctx.run_state),
        plan_profile: build_deck_plan_profile(&ctx.run_state),
        deck_cards: build_deck_card_observations(&ctx.run_state),
        relics: build_relic_observations(&ctx.run_state),
        potions: build_potion_observations(&ctx.run_state),
        map: build_map_observation_if_relevant(&ctx.engine_state, &ctx.run_state),
        next_nodes: build_next_node_observations(&ctx.run_state),
        act_boss: ctx
            .run_state
            .boss_list
            .first()
            .map(|boss| format!("{boss:?}")),
        reward_source: reward_source_label(&ctx.engine_state, &ctx.run_state),
        combat: combat.map(build_combat_observation),
        screen: build_screen_observation(&ctx.engine_state, &ctx.run_state),
    }
}

pub fn build_deck_card_observations(run_state: &RunState) -> Vec<RunDeckCardObservationV0> {
    run_state
        .master_deck
        .iter()
        .enumerate()
        .map(|(deck_index, card)| RunDeckCardObservationV0 {
            deck_index,
            uuid: card.uuid,
            card: build_card_feature(card.id, card.upgrades, run_state),
        })
        .collect()
}

pub fn build_deck_observation(run_state: &RunState) -> RunDeckObservationV0 {
    let mut out = RunDeckObservationV0::default();
    let mut cost_sum = 0i32;
    let mut cost_count = 0i32;
    for card in &run_state.master_deck {
        let def = crate::content::cards::get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => out.attack_count += 1,
            CardType::Skill => out.skill_count += 1,
            CardType::Power => out.power_count += 1,
            CardType::Status => out.status_count += 1,
            CardType::Curse => out.curse_count += 1,
        }
        if crate::content::cards::is_starter_basic(card.id) {
            out.starter_basic_count += 1;
        }
        if def.base_damage > 0 {
            out.damage_card_count += 1;
        }
        if def.base_block > 0 || card_is_block_core(card.id) {
            out.block_card_count += 1;
        }
        if card_draws_cards(card.id) {
            out.draw_card_count += 1;
        }
        if card_is_scaling_piece(card.id) {
            out.scaling_card_count += 1;
        }
        if def.exhaust || card_exhausts_other_cards(card.id) {
            out.exhaust_card_count += 1;
        }
        if def.cost >= 0 {
            cost_sum += def.cost as i32;
            cost_count += 1;
        }
    }
    out.average_cost_milli = if cost_count > 0 {
        cost_sum * 1000 / cost_count
    } else {
        0
    };
    out
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CardPlanAffordance {
    pub(crate) frontload: i32,
    pub(crate) block: i32,
    pub(crate) draw: i32,
    pub(crate) scaling: i32,
    pub(crate) aoe: i32,
    pub(crate) exhaust: i32,
    pub(crate) kill_window: i32,
    pub(crate) setup_cashout_risk: i32,
}

impl CardPlanAffordance {
    pub fn subtract(self, other: Self) -> Self {
        Self {
            frontload: self.frontload - other.frontload,
            block: self.block - other.block,
            draw: self.draw - other.draw,
            scaling: self.scaling - other.scaling,
            aoe: self.aoe - other.aoe,
            exhaust: self.exhaust - other.exhaust,
            kill_window: self.kill_window - other.kill_window,
            setup_cashout_risk: self.setup_cashout_risk - other.setup_cashout_risk,
        }
    }
}

pub fn build_deck_plan_profile(run_state: &RunState) -> DeckPlanProfileV0 {
    let mut profile = DeckPlanProfileV0 {
        score_kind: "heuristic".to_string(),
        ..DeckPlanProfileV0::default()
    };
    for card in &run_state.master_deck {
        let affordance = card_plan_affordance(card.id, card.upgrades);
        profile.frontload_supply += affordance.frontload;
        profile.block_supply += affordance.block;
        profile.draw_supply += affordance.draw;
        profile.scaling_supply += affordance.scaling;
        profile.aoe_supply += affordance.aoe;
        profile.exhaust_supply += affordance.exhaust;
        profile.kill_window_supply += affordance.kill_window;
        if crate::content::cards::is_starter_basic(card.id) {
            profile.starter_basic_burden += 10;
        }
    }
    profile.setup_cashout_risk = setup_cashout_risk_from_supplies(
        profile.frontload_supply,
        profile.block_supply,
        profile.draw_supply,
        profile.scaling_supply,
    );
    profile
}

pub fn card_plan_affordance(card_id: CardId, upgrades: u8) -> CardPlanAffordance {
    let def = crate::content::cards::get_card_definition(card_id);
    let damage = (def.base_damage + def.upgrade_damage * upgrades as i32).max(0);
    let block = (def.base_block + def.upgrade_block * upgrades as i32).max(0);
    let magic = (def.base_magic + def.upgrade_magic * upgrades as i32).max(0);
    let mut out = CardPlanAffordance::default();
    if damage > 0 {
        out.frontload += damage;
    }
    if block > 0 {
        out.block += block;
    } else if card_is_block_core(card_id) {
        out.block += 8;
    }
    if card_draws_cards(card_id) {
        out.draw += match card_id {
            CardId::Offering | CardId::BattleTrance | CardId::MasterOfStrategy => 18,
            CardId::ShrugItOff | CardId::PommelStrike | CardId::Backflip => 12,
            _ => 10,
        };
    }
    if card_is_scaling_piece(card_id) {
        out.scaling += match card_id {
            CardId::DemonForm | CardId::Corruption => 22,
            CardId::Inflame | CardId::FeelNoPain | CardId::DarkEmbrace => 16,
            _ => 12,
        };
        out.setup_cashout_risk += 4;
    }
    if matches!(def.target, crate::content::cards::CardTarget::AllEnemy) || def.is_multi_damage {
        out.aoe += 12 + damage / 2;
    }
    if card_is_multi_hit(card_id) {
        out.aoe += 4;
    }
    if card_exhausts_other_cards(card_id) {
        out.exhaust += match card_id {
            CardId::TrueGrit if upgrades == 0 => 5,
            CardId::TrueGrit => 14,
            CardId::SecondWind | CardId::FiendFire | CardId::BurningPact => 12,
            _ => 8,
        };
    }
    if matches!(
        card_id,
        CardId::Feed | CardId::HandOfGreed | CardId::RitualDagger
    ) {
        out.kill_window += 18;
    }
    if card_applies_vulnerable(card_id) {
        out.frontload += 8 + magic;
    }
    if card_applies_weak(card_id) {
        out.block += 6 + magic;
    }
    match card_id {
        CardId::Immolate => {
            out.frontload += 20;
            out.aoe += 20;
        }
        CardId::Disarm | CardId::Shockwave => {
            out.block += 18;
            out.scaling += 6;
        }
        CardId::Offering => {
            out.frontload += 8;
            out.draw += 6;
        }
        // Magic-based cards: primary value comes from base_magic,
        // not captured by base_damage/base_block.
        CardId::Flex => {
            // +magic strength for 1 turn → temporary frontload (~2 attacks)
            out.frontload += magic * 2;
        }
        CardId::Rage => {
            // +magic block per attack played → block over a turn (~3 attacks)
            out.block += magic * 3;
        }
        CardId::Combust => {
            // magic AOE damage per turn as power → ongoing aoe (~4 turns)
            out.aoe += magic * 4;
            out.scaling += 8;
        }
        _ => {}
    }
    out
}

pub fn setup_cashout_risk_from_supplies(
    frontload_supply: i32,
    block_supply: i32,
    draw_supply: i32,
    scaling_supply: i32,
) -> i32 {
    if scaling_supply <= 0 {
        return 0;
    }
    (scaling_supply * 2 - block_supply - draw_supply - frontload_supply / 3).max(0)
}

pub fn build_relic_observations(run_state: &RunState) -> Vec<RunRelicObservationV0> {
    run_state
        .relics
        .iter()
        .map(|relic| RunRelicObservationV0 {
            relic_id: format!("{:?}", relic.id),
            counter: relic.counter,
            used_up: relic.used_up,
            amount: relic.amount,
        })
        .collect()
}

pub fn build_potion_observations(run_state: &RunState) -> Vec<RunPotionSlotObservationV0> {
    run_state
        .potions
        .iter()
        .enumerate()
        .map(|(slot_index, slot)| match slot {
            Some(potion) => RunPotionSlotObservationV0 {
                slot_index,
                potion_id: Some(format!("{:?}", potion.id)),
                uuid: Some(potion.uuid),
                can_use: potion.can_use,
                can_discard: potion.can_discard,
                requires_target: potion.requires_target,
            },
            None => RunPotionSlotObservationV0 {
                slot_index,
                potion_id: None,
                uuid: None,
                can_use: false,
                can_discard: false,
                requires_target: false,
            },
        })
        .collect()
}

pub fn build_map_observation(run_state: &RunState) -> RunMapObservationV0 {
    let nodes = run_state
        .map
        .graph
        .iter()
        .flat_map(|row| row.iter())
        .filter(|node| {
            node.class.is_some()
                || !node.edges.is_empty()
                || !node.parents.is_empty()
                || node.has_emerald_key
        })
        .map(|node| map_node_observation(run_state, node.x, node.y))
        .collect();
    RunMapObservationV0 {
        current_x: run_state.map.current_x,
        current_y: run_state.map.current_y,
        boss_node_available: run_state.map.boss_node_available,
        has_emerald_key: run_state.map.has_emerald_key,
        nodes,
    }
}

pub fn build_map_observation_if_relevant(
    engine_state: &EngineState,
    run_state: &RunState,
) -> Option<RunMapObservationV0> {
    match engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::EventCombat(_)
        | EngineState::PendingChoice(PendingChoice::GridSelect { .. })
        | EngineState::PendingChoice(PendingChoice::HandSelect { .. })
        | EngineState::PendingChoice(PendingChoice::DiscoverySelect(_))
        | EngineState::PendingChoice(PendingChoice::ScrySelect { .. })
        | EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. })
        | EngineState::PendingChoice(PendingChoice::StanceChoice)
        | EngineState::GameOver(_) => None,
        EngineState::RewardScreen(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::BossRelicSelect(_) => Some(build_map_observation(run_state)),
    }
}

pub fn build_next_node_observations(run_state: &RunState) -> Vec<RunMapNodeObservationV0> {
    legal_map_actions(run_state)
        .into_iter()
        .filter_map(|action| match action {
            ClientInput::SelectMapNode(x) => {
                let y = if run_state.map.current_y == -1 {
                    0
                } else if run_state.map.current_y == 14 {
                    15
                } else {
                    run_state.map.current_y + 1
                };
                Some(map_node_observation(run_state, x as i32, y))
            }
            ClientInput::FlyToNode(x, y) => {
                Some(map_node_observation(run_state, x as i32, y as i32))
            }
            _ => None,
        })
        .collect()
}

pub fn map_node_observation(run_state: &RunState, x: i32, y: i32) -> RunMapNodeObservationV0 {
    if y == 15 {
        return RunMapNodeObservationV0 {
            x,
            y,
            room_type: Some("MonsterRoomBoss".to_string()),
            has_emerald_key: false,
            reachable_now: run_state.map.can_travel_to(x, y, false),
            edges: Vec::new(),
        };
    }
    let node = run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize));
    let edges = node
        .map(|node| {
            node.edges
                .iter()
                .map(|edge| RunMapEdgeObservationV0 {
                    dst_x: edge.dst_x,
                    dst_y: edge.dst_y,
                })
                .collect()
        })
        .unwrap_or_default();
    RunMapNodeObservationV0 {
        x,
        y,
        room_type: node.and_then(|node| node.class).map(room_type_name),
        has_emerald_key: node.is_some_and(|node| node.has_emerald_key),
        reachable_now: run_state.map.can_travel_to(x, y, false),
        edges,
    }
}

pub fn room_type_name(room_type: RoomType) -> String {
    format!("{room_type:?}")
}

pub fn reward_source_label(engine_state: &EngineState, run_state: &RunState) -> Option<String> {
    match engine_state {
        EngineState::RewardScreen(reward_state) => {
            if run_state.pending_boss_reward {
                Some("boss_combat_reward".to_string())
            } else {
                Some(format!(
                    "{:?}:{:?}",
                    reward_state.screen_context,
                    run_state.map.get_current_room_type()
                ))
            }
        }
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => {
            Some("combat_card_reward_select".to_string())
        }
        _ => None,
    }
}

pub fn build_combat_observation(combat: &CombatState) -> RunCombatObservationV0 {
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && !monster.half_dead)
        .collect::<Vec<_>>();
    let dying_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_dying)
        .count();
    let half_dead_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.half_dead)
        .count();
    let zero_hp_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.current_hp <= 0)
        .count();
    let pending_rebirth_monster_count = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            crate::content::powers::store::powers_for(combat, monster.id).is_some_and(|powers| {
                powers.iter().any(|power| {
                    matches!(
                        power.power_type,
                        crate::content::powers::PowerId::Regrow
                            | crate::content::powers::PowerId::Unawakened
                    )
                })
            })
        })
        .count();
    let visible_incoming_damage = alive_monsters
        .iter()
        .map(|monster| {
            crate::projection::combat::monster_preview_total_damage_in_combat(combat, monster)
        })
        .sum();

    RunCombatObservationV0 {
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy as i32,
        combat_phase: combat_phase_label(combat).to_string(),
        turn_count: combat.turn.turn_count,
        hand_count: combat.zones.hand.len(),
        hand_cards: build_combat_hand_card_observations(combat),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        alive_monster_count: alive_monsters.len(),
        dying_monster_count,
        half_dead_monster_count,
        zero_hp_monster_count,
        pending_rebirth_monster_count,
        total_monster_hp: alive_monsters
            .iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        visible_incoming_damage,
        pending_action_count: combat.action_queue_len(),
        queued_card_count: combat.zones.queued_cards.len(),
        limbo_count: combat.zones.limbo.len(),
    }
}

pub fn build_combat_hand_card_observations(
    combat: &CombatState,
) -> Vec<RunCombatHandCardObservationV0> {
    let context = build_card_role_context(combat);
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(hand_index, card)| {
            let playable = crate::content::cards::can_play_card(card, combat).is_ok();
            let role = classify_hand_card_with_context(combat, hand_index, &context);
            let mut transient_tags = Vec::new();
            transient_tags.push(if playable { "playable" } else { "unplayable" }.to_string());
            if card.cost_for_turn.is_some() {
                transient_tags.push("cost_for_turn_override".to_string());
            }
            if card.free_to_play_once {
                transient_tags.push("free_to_play_once".to_string());
            }
            transient_tags.push(format!("role:{}", hand_card_role_label(role)));

            RunCombatHandCardObservationV0 {
                hand_index,
                card_instance_id: card.uuid,
                card_id: format!("{:?}", card.id),
                upgraded: card.upgrades > 0,
                upgrades: card.upgrades,
                cost_for_turn: card.get_cost(),
                playable,
                base_semantics: base_semantics_for_card(card.id, card.upgrades),
                transient_tags,
                estimated_role_scores: RunHandCardRoleScoresV0 {
                    score_kind: "heuristic_not_truth".to_string(),
                    role: hand_card_role_label(role).to_string(),
                    keeper: combat_retention_score_for_uuid(combat, card.uuid),
                    fuel: combat_fuel_score_for_uuid(combat, card.uuid),
                    exhaust: combat_exhaust_score_for_uuid(combat, card.uuid),
                    retention: combat_retention_score_for_uuid(combat, card.uuid),
                    copy: combat_copy_score_for_uuid(combat, card.uuid),
                },
            }
        })
        .collect()
}

pub fn hand_card_role_label(role: HandCardRole) -> &'static str {
    match role {
        HandCardRole::CoreKeeper => "core_keeper",
        HandCardRole::SequencedPiece => "sequenced_piece",
        HandCardRole::SituationalResource => "situational_resource",
        HandCardRole::LowValueFuel => "low_value_fuel",
    }
}

pub fn base_semantics_for_card(card_id: CardId, upgrades: u8) -> Vec<String> {
    let def = crate::content::cards::get_card_definition(card_id);
    let mut tags = Vec::new();
    match def.card_type {
        CardType::Attack => tags.push("attack".to_string()),
        CardType::Skill => tags.push("skill".to_string()),
        CardType::Power => tags.push("power".to_string()),
        CardType::Status => tags.push("status".to_string()),
        CardType::Curse => tags.push("curse".to_string()),
    }
    if def.base_damage + def.upgrade_damage * upgrades as i32 > 0 {
        tags.push("damage".to_string());
    }
    if def.base_block + def.upgrade_block * upgrades as i32 > 0 || card_is_block_core(card_id) {
        tags.push("block".to_string());
    }
    if def.exhaust {
        tags.push("self_exhaust".to_string());
    }
    if card_draws_cards(card_id) {
        tags.push("draw".to_string());
    }
    if card_gains_energy(card_id) {
        tags.push("energy".to_string());
    }
    if card_applies_weak(card_id) {
        tags.push("apply_weak".to_string());
    }
    if card_applies_vulnerable(card_id) {
        tags.push("apply_vulnerable".to_string());
    }
    if card_is_scaling_piece(card_id) {
        tags.push("setup_or_scaling".to_string());
    }
    if card_exhausts_other_cards(card_id) {
        tags.push("exhaust_outlet".to_string());
    }
    match card_id {
        CardId::TrueGrit if upgrades == 0 => {
            tags.push("random_exhaust".to_string());
            tags.push("risk_overlay_required".to_string());
        }
        CardId::TrueGrit => tags.push("chosen_exhaust".to_string()),
        CardId::SecondWind => {
            tags.push("exhaust_non_attacks".to_string());
            tags.push("block_from_hand_destruction".to_string());
        }
        CardId::FiendFire => {
            tags.push("exhaust_hand_for_damage".to_string());
            tags.push("hand_destruction_risk".to_string());
        }
        _ => {}
    }
    if def.target == crate::content::cards::CardTarget::AllEnemy || def.is_multi_damage {
        tags.push("multi_target_or_multi_damage".to_string());
    }
    tags
}

pub fn combat_phase_label(combat: &CombatState) -> &'static str {
    match combat.turn.current_phase {
        crate::runtime::combat::CombatPhase::PlayerTurn => "player_turn",
        crate::runtime::combat::CombatPhase::TurnTransition => "turn_transition",
        crate::runtime::combat::CombatPhase::MonsterTurn => "monster_turn",
    }
}

pub fn build_screen_observation(
    engine_state: &EngineState,
    run_state: &RunState,
) -> RunScreenObservationV0 {
    match engine_state {
        EngineState::EventRoom => RunScreenObservationV0 {
            event_option_count: crate::engine::event_handler::get_event_options(run_state)
                .iter()
                .filter(|option| !option.ui.disabled)
                .count(),
            ..empty_screen_observation()
        },
        EngineState::RewardScreen(reward_state) => {
            build_reward_screen_observation(run_state, reward_state)
        }
        EngineState::Shop(shop) => RunScreenObservationV0 {
            shop_card_count: shop.cards.len(),
            shop_relic_count: shop.relics.len(),
            shop_potion_count: shop.potions.len(),
            ..empty_screen_observation()
        },
        EngineState::BossRelicSelect(state) => RunScreenObservationV0 {
            boss_relic_choice_count: state.relics.len(),
            ..empty_screen_observation()
        },
        EngineState::RunPendingChoice(choice) => RunScreenObservationV0 {
            selection_target_count: choice.selection_request(run_state).targets.len(),
            ..empty_screen_observation()
        },
        EngineState::PendingChoice(choice) => RunScreenObservationV0 {
            selection_target_count: choice
                .selection_request()
                .map(|request| request.targets.len())
                .unwrap_or(0),
            ..empty_screen_observation()
        },
        _ => empty_screen_observation(),
    }
}

pub fn empty_screen_observation() -> RunScreenObservationV0 {
    RunScreenObservationV0 {
        event_option_count: 0,
        reward_item_count: 0,
        reward_card_choice_count: 0,
        reward_phase: "none".to_string(),
        reward_items: Vec::new(),
        reward_claimable_item_count: 0,
        reward_unclaimed_card_item_count: 0,
        reward_free_value_score: 0,
        shop_card_count: 0,
        shop_relic_count: 0,
        shop_potion_count: 0,
        boss_relic_choice_count: 0,
        selection_target_count: 0,
    }
}

pub fn build_reward_screen_observation(
    run_state: &RunState,
    reward_state: &RewardState,
) -> RunScreenObservationV0 {
    let reward_items = reward_state
        .items
        .iter()
        .enumerate()
        .map(|(item_index, item)| reward_item_observation(run_state, item_index, item))
        .collect::<Vec<_>>();
    let reward_claimable_item_count = reward_items.iter().filter(|item| item.claimable).count();
    let reward_unclaimed_card_item_count = reward_items
        .iter()
        .filter(|item| item.opens_card_choice)
        .count();
    let reward_free_value_score = reward_items
        .iter()
        .filter(|item| item.claimable)
        .map(|item| item.free_value_score.max(0))
        .sum::<i32>();
    let reward_phase = if reward_state.pending_card_choice.is_some() {
        "card_choice"
    } else if reward_claimable_item_count > 0 {
        "claim_items"
    } else {
        "cleanup"
    };

    RunScreenObservationV0 {
        reward_item_count: reward_state.items.len(),
        reward_card_choice_count: reward_state
            .pending_card_choice
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0),
        reward_phase: reward_phase.to_string(),
        reward_items,
        reward_claimable_item_count,
        reward_unclaimed_card_item_count,
        reward_free_value_score,
        ..empty_screen_observation()
    }
}

pub fn reward_item_observation(
    run_state: &RunState,
    item_index: usize,
    item: &RewardItem,
) -> RunRewardItemObservationV0 {
    let claimable = reward_item_claimable(run_state, item);
    let likely_waste = reward_item_likely_waste(run_state, item);
    let capacity_blocked = reward_item_capacity_blocked(run_state, item);
    RunRewardItemObservationV0 {
        item_index,
        item_type: reward_item_type_label(item).to_string(),
        amount: reward_item_amount(item),
        card_choice_count: match item {
            RewardItem::Card { cards } => cards.len(),
            _ => 0,
        },
        relic_id: match item {
            RewardItem::Relic { relic_id } => Some(format!("{relic_id:?}")),
            _ => None,
        },
        potion_id: match item {
            RewardItem::Potion { potion_id } => Some(format!("{potion_id:?}")),
            _ => None,
        },
        claimable,
        opens_card_choice: matches!(item, RewardItem::Card { .. }),
        free_value_score: reward_item_claim_score(run_state, item),
        likely_waste,
        capacity_blocked,
    }
}

