use super::*;
use crate::rewards::state::RewardCard;
use std::collections::HashSet;

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
        keys: RunKeyObservationV0 {
            ruby: ctx.run_state.keys[0],
            sapphire: ctx.run_state.keys[1],
            emerald: ctx.run_state.keys[2],
        },
        deck: build_deck_observation(&ctx.run_state),
        plan_profile: build_deck_plan_profile(&ctx.run_state),
        deck_cards: build_deck_card_observations(&ctx.run_state),
        relics: build_relic_observations(&ctx.run_state),
        potions: build_potion_observations(
            &ctx.run_state,
            ctx.combat_state.as_ref(),
            &ctx.engine_state,
        ),
        map: build_map_observation_if_relevant(&ctx.engine_state, &ctx.run_state),
        next_nodes: build_next_node_observations(&ctx.run_state),
        map_route_context: build_map_route_context_if_relevant(&ctx.run_state),
        act_boss: ctx.run_state.boss_key.map(|boss| format!("{boss:?}")),
        reward_source: reward_source_label(&ctx.engine_state, &ctx.run_state),
        combat: combat.map(|combat| build_combat_observation(&ctx.engine_state, combat)),
        screen: build_screen_observation(&ctx.engine_state, &ctx.run_state),
        recording_view: build_recording_view(ctx, active_hp, active_max_hp),
        decision_frame: build_decision_frame(ctx),
    }
}

fn build_decision_frame(ctx: &EpisodeContext) -> RunDecisionFrameV1 {
    let decision_kind = decision_type(&ctx.engine_state).to_string();
    let (prompt, source, warnings) = match &ctx.engine_state {
        EngineState::CombatPlayerTurn | EngineState::EventCombat(_) => (
            "Choose a combat action.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "combat_turn".to_string(),
                label: "Player combat turn".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        EngineState::PendingChoice(choice) => pending_choice_decision_prompt(choice),
        EngineState::MapNavigation => (
            "Choose a map route.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "map_navigation".to_string(),
                label: "Map route choice".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        EngineState::RewardScreen(reward_state) => {
            if reward_state.pending_card_choice.is_some() {
                (
                    "Choose a reward card or skip.".to_string(),
                    Some(RunDecisionSourceV1 {
                        kind: "reward_card_choice".to_string(),
                        label: "Reward card choice".to_string(),
                        action_key: None,
                        card_instance_id: None,
                        card_name: None,
                    }),
                    Vec::new(),
                )
            } else {
                (
                    "Claim rewards or proceed.".to_string(),
                    Some(RunDecisionSourceV1 {
                        kind: "reward_screen".to_string(),
                        label: "Reward screen".to_string(),
                        action_key: None,
                        card_instance_id: None,
                        card_name: None,
                    }),
                    Vec::new(),
                )
            }
        }
        EngineState::Shop(_) => (
            "Choose a shop action.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "shop".to_string(),
                label: "Merchant shop".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        EngineState::Campfire => (
            "Choose a campfire action.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "campfire".to_string(),
                label: "Campfire".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        EngineState::BossRelicSelect(_) => (
            "Choose a boss relic.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "boss_relic_select".to_string(),
                label: "Boss relic choice".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        EngineState::EventRoom => (
            "Choose an event option.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "event".to_string(),
                label: ctx
                    .run_state
                    .event_state
                    .as_ref()
                    .map(|event| format!("{:?}", event.id))
                    .unwrap_or_else(|| "Event".to_string()),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        _ => (
            "Choose a legal action.".to_string(),
            None,
            vec!["decision_frame_generic_fallback".to_string()],
        ),
    };
    RunDecisionFrameV1 {
        schema_name: "DecisionFrameV1".to_string(),
        schema_version: 1,
        decision_kind,
        prompt,
        source,
        warnings,
    }
}

fn pending_choice_decision_prompt(
    choice: &PendingChoice,
) -> (String, Option<RunDecisionSourceV1>, Vec<String>) {
    match choice {
        PendingChoice::HandSelect {
            min_cards,
            max_cards,
            reason,
            ..
        } => (
            format!(
                "{}: choose {} card(s) in hand.",
                hand_select_reason_label(reason),
                if min_cards == max_cards {
                    min_cards.to_string()
                } else {
                    format!("{min_cards}-{max_cards}")
                }
            ),
            Some(RunDecisionSourceV1 {
                kind: "combat_pending_hand_select".to_string(),
                label: hand_select_reason_label(reason),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        PendingChoice::GridSelect {
            min_cards,
            max_cards,
            reason,
            source_pile,
            ..
        } => (
            format!(
                "{reason:?}: choose {} card(s) from {:?}.",
                if min_cards == max_cards {
                    min_cards.to_string()
                } else {
                    format!("{min_cards}-{max_cards}")
                },
                source_pile
            ),
            Some(RunDecisionSourceV1 {
                kind: "combat_pending_grid_select".to_string(),
                label: format!("{reason:?}"),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        PendingChoice::DiscoverySelect(_) => (
            "Choose a discovered card.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "combat_discovery_select".to_string(),
                label: "Discovery".to_string(),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            Vec::new(),
        ),
        _ => (
            "Resolve combat pending choice.".to_string(),
            Some(RunDecisionSourceV1 {
                kind: "combat_pending_choice".to_string(),
                label: format!("{choice:?}"),
                action_key: None,
                card_instance_id: None,
                card_name: None,
            }),
            vec!["pending_choice_prompt_is_generic".to_string()],
        ),
    }
}

fn hand_select_reason_label(reason: &crate::state::HandSelectReason) -> String {
    match reason {
        crate::state::HandSelectReason::Upgrade => "Upgrade".to_string(),
        crate::state::HandSelectReason::Exhaust => "Exhaust".to_string(),
        crate::state::HandSelectReason::Discard => "Discard".to_string(),
        crate::state::HandSelectReason::PutOnDrawPile => "Put on draw pile".to_string(),
        crate::state::HandSelectReason::Setup => "Put on draw pile".to_string(),
        crate::state::HandSelectReason::PutToBottomOfDraw => {
            "Put on bottom of draw pile".to_string()
        }
        crate::state::HandSelectReason::Retain => "Retain".to_string(),
        crate::state::HandSelectReason::GamblingChip => "Discard for Gambling Chip".to_string(),
        crate::state::HandSelectReason::Recycle => "Recycle".to_string(),
        crate::state::HandSelectReason::Copy { amount } => format!("Copy x{amount}"),
        crate::state::HandSelectReason::Nightmare { amount } => format!("Nightmare x{amount}"),
    }
}

fn build_recording_view(
    ctx: &EpisodeContext,
    active_hp: i32,
    active_max_hp: i32,
) -> RunRecordingViewV1 {
    let mut state_lines = vec![format!(
        "Act {} Floor {} | HP {}/{} | Gold {} | Boss {}",
        ctx.run_state.act_num,
        ctx.run_state.floor_num,
        active_hp,
        active_max_hp,
        ctx.run_state.gold,
        ctx.run_state
            .boss_key
            .map(|boss| format!("{boss:?}"))
            .unwrap_or_else(|| "Unknown".to_string())
    )];
    if !ctx.run_state.relics.is_empty() {
        state_lines.push(format!(
            "Relics: {}",
            ctx.run_state
                .relics
                .iter()
                .map(|relic| recording_relic_label(relic))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    let potion_lines = ctx
        .run_state
        .potions
        .iter()
        .enumerate()
        .filter_map(|(slot, potion)| {
            potion.as_ref().map(|potion| {
                let def = crate::content::potions::get_potion_definition(potion.id);
                format!("{slot}:{}", def.name)
            })
        })
        .collect::<Vec<_>>();
    if !potion_lines.is_empty() {
        state_lines.push(format!("Potions: {}", potion_lines.join(" | ")));
    }

    let mut context_lines = Vec::new();
    let mut warning_lines = Vec::new();
    match &ctx.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::EventCombat(_)
        | EngineState::PendingChoice(_) => {
            if let Some(combat) = ctx.combat_state.as_ref() {
                context_lines.push(format!(
                    "Combat: energy={} block={} incoming={} turn={} monster_hp={}",
                    combat.turn.energy,
                    combat.entities.player.block,
                    visible_incoming_damage_for_recording(combat),
                    combat.turn.turn_count,
                    combat
                        .entities
                        .monsters
                        .iter()
                        .filter(|monster| monster.is_alive_for_action() && !monster.half_dead)
                        .map(|monster| monster.current_hp.max(0))
                        .sum::<i32>()
                ));
                for (slot, monster) in combat.entities.monsters.iter().enumerate() {
                    if !monster.is_alive_for_action() || monster.half_dead {
                        continue;
                    }
                    let name = crate::content::monsters::EnemyId::from_id(monster.monster_type)
                        .map(|enemy| enemy.get_name())
                        .unwrap_or("Unknown Monster");
                    let move_preview =
                        crate::projection::combat::project_monster_move_preview_in_combat(
                            combat, monster,
                        );
                    let visible_intent_kind = format!("{:?}", move_preview.visible_intent);
                    context_lines.push(format!(
                        "Enemy: slot={} {} hp={}/{} block={} intent={}",
                        slot,
                        name,
                        monster.current_hp,
                        monster.max_hp,
                        monster.block,
                        monster_visible_intent_label(
                            &visible_intent_kind,
                            move_preview.damage_per_hit,
                            move_preview.hits,
                            move_preview.total_damage,
                        ),
                    ));
                }
            }
        }
        EngineState::MapNavigation => {
            for line in recording_route_lines(&ctx.run_state) {
                context_lines.push(line);
            }
        }
        EngineState::RewardScreen(reward_state) => {
            if let Some(cards) = reward_state.pending_card_choice.as_ref() {
                context_lines.push(format!(
                    "Cards: {}",
                    cards
                        .iter()
                        .map(recording_reward_card_label)
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            } else if !reward_state.items.is_empty() {
                context_lines.push(format!(
                    "Rewards: {}",
                    reward_state
                        .items
                        .iter()
                        .map(recording_reward_item_label)
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
        }
        EngineState::Shop(shop) => {
            let mut shop_lines = Vec::new();
            if !shop.cards.is_empty() {
                shop_lines.push(format!(
                    "Cards: {}",
                    shop.cards
                        .iter()
                        .map(|item| {
                            let def = crate::content::cards::get_card_definition(item.card_id);
                            format!("{} {}g", def.name, item.price)
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
            if !shop.relics.is_empty() {
                shop_lines.push(format!(
                    "Relics: {}",
                    shop.relics
                        .iter()
                        .map(|item| format!("{:?} {}g", item.relic_id, item.price))
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
            if !shop.potions.is_empty() {
                shop_lines.push(format!(
                    "Potions: {}",
                    shop.potions
                        .iter()
                        .map(|item| {
                            let def =
                                crate::content::potions::get_potion_definition(item.potion_id);
                            format!("{} {}g", def.name, item.price)
                        })
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
            if shop.purge_available {
                shop_lines.push(format!("Remove card: {}g", shop.purge_cost));
            }
            context_lines.extend(shop_lines);
        }
        EngineState::Campfire => {
            context_lines.push(format!(
                "Campfire: rest heals {} HP",
                ctx.run_state.max_hp * 30 / 100
            ));
        }
        EngineState::BossRelicSelect(state) => {
            context_lines.push(format!(
                "Boss relics: {}",
                state
                    .relics
                    .iter()
                    .map(|relic| format!("{relic:?}"))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(&ctx.run_state);
            context_lines.push(format!(
                "Selection: {:?} {}",
                request.reason,
                request.constraint.describe(request.targets.len())
            ));
        }
        EngineState::TreasureRoom(_) => {
            context_lines.push("Treasure room".to_string());
        }
        EngineState::EventRoom => {
            if let Some(event) = ctx.run_state.event_state.as_ref() {
                context_lines.push(format!("Event: {:?}", event.id));
            }
        }
        EngineState::CombatProcessing | EngineState::GameOver(_) => {}
    }
    if context_lines.is_empty() {
        warning_lines.push("recording_context_empty".to_string());
    }
    RunRecordingViewV1 {
        schema_name: "RecordingViewV1".to_string(),
        schema_version: 1,
        recording_source: "rust_runtime".to_string(),
        state_lines,
        context_lines,
        warning_lines,
    }
}

fn recording_relic_label(relic: &crate::content::relics::RelicState) -> String {
    let mut label = format!("{:?}", relic.id);
    if relic.counter >= 0 {
        label.push_str(&format!(" counter={}", relic.counter));
    }
    if relic.amount != 0 {
        label.push_str(&format!(" amount={}", relic.amount));
    }
    if relic.used_up {
        label.push_str(" used");
    }
    label
}

fn recording_reward_card_label(card: &RewardCard) -> String {
    let def = crate::content::cards::get_card_definition(card.id);
    format!(
        "{}{} ({} {:?}) cost={}",
        def.name,
        upgrade_suffix(card.upgrades),
        format!("{:?}", def.card_type),
        def.rarity,
        def.cost
    )
}

fn recording_reward_item_label(item: &crate::rewards::state::RewardItem) -> String {
    match item {
        crate::rewards::state::RewardItem::Gold { amount } => format!("{amount} Gold"),
        crate::rewards::state::RewardItem::StolenGold { amount } => format!("{amount} Stolen Gold"),
        crate::rewards::state::RewardItem::Card { cards } => {
            format!("Card reward: choose 1 of {}", cards.len())
        }
        crate::rewards::state::RewardItem::Relic { relic_id } => format!("{relic_id:?}"),
        crate::rewards::state::RewardItem::Potion { potion_id } => {
            let def = crate::content::potions::get_potion_definition(*potion_id);
            def.name.to_string()
        }
        crate::rewards::state::RewardItem::EmeraldKey => "Emerald Key".to_string(),
        crate::rewards::state::RewardItem::SapphireKey => "Sapphire Key".to_string(),
    }
}

fn recording_route_lines(run_state: &RunState) -> Vec<String> {
    let Some(route_context) = build_map_route_context_if_relevant(run_state) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for choice in route_context.route_choices.iter().take(8) {
        lines.push(format!(
            "{}: {} shop={} fire={} fights3={}",
            choice.action_key,
            choice.room_label,
            recording_optional_floor(choice.earliest_shop_floor),
            recording_optional_floor(choice.earliest_fire_floor),
            choice.forced_fights_next_3
        ));
    }
    lines
}

fn visible_incoming_damage_for_recording(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action() && !monster.half_dead)
        .filter_map(|monster| {
            crate::content::monsters::resolve_monster_turn_plan(combat, monster)
                .summary_spec()
                .attack()
                .map(|attack| attack.total_base_damage())
        })
        .sum()
}

fn recording_optional_floor(floor: Option<i32>) -> String {
    floor
        .map(|floor| floor.to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[allow(dead_code)]
fn recording_move_spec_label(spec: &crate::semantics::combat::MonsterMoveSpec) -> String {
    use crate::semantics::combat::MonsterMoveSpec;
    let attack_label = |attack: &crate::semantics::combat::AttackSpec| {
        if attack.hits > 1 {
            format!("Attack {}x{}", attack.base_damage, attack.hits)
        } else {
            format!("Attack {}", attack.base_damage)
        }
    };
    match spec {
        MonsterMoveSpec::Attack(attack) => attack_label(attack),
        MonsterMoveSpec::AttackAddCard(attack, _) => format!("{} + Add Card", attack_label(attack)),
        MonsterMoveSpec::AttackUpgradeCards(attack, _) => {
            format!("{} + Upgrade Cards", attack_label(attack))
        }
        MonsterMoveSpec::AttackBuff(attack, _) => format!("{} + Buff", attack_label(attack)),
        MonsterMoveSpec::AttackSustain(attack) => format!("{} + Sustain", attack_label(attack)),
        MonsterMoveSpec::AttackDebuff(attack, _) => format!("{} + Debuff", attack_label(attack)),
        MonsterMoveSpec::AttackDefend(attack, defend) => {
            format!("{} + Block {}", attack_label(attack), defend.block)
        }
        MonsterMoveSpec::AddCard(_) => "Add Card".to_string(),
        MonsterMoveSpec::Buff(_) => "Buff".to_string(),
        MonsterMoveSpec::Debuff(_) => "Debuff".to_string(),
        MonsterMoveSpec::StrongDebuff(_) => "Strong Debuff".to_string(),
        MonsterMoveSpec::Defend(defend) => format!("Block {}", defend.block),
        MonsterMoveSpec::DefendDebuff(defend, _) => format!("Block {} + Debuff", defend.block),
        MonsterMoveSpec::DefendBuff(defend, _) => format!("Block {} + Buff", defend.block),
        MonsterMoveSpec::Heal(heal) => format!("Heal {}", heal.amount),
        MonsterMoveSpec::Escape => "Escape".to_string(),
        MonsterMoveSpec::Magic => "Magic".to_string(),
        MonsterMoveSpec::Sleep => "Sleep".to_string(),
        MonsterMoveSpec::Stun => "Stun".to_string(),
        MonsterMoveSpec::Debug => "Debug".to_string(),
        MonsterMoveSpec::None => "None".to_string(),
        MonsterMoveSpec::Unknown => "Unknown".to_string(),
    }
}

fn upgrade_suffix(upgrades: u8) -> String {
    if upgrades > 0 {
        format!("+{upgrades}")
    } else {
        String::new()
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
    let mut profile = DeckPlanProfileV0::default();
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

pub fn build_potion_observations(
    run_state: &RunState,
    combat: Option<&CombatState>,
    engine_state: &EngineState,
) -> Vec<RunPotionSlotObservationV0> {
    let source_potions = combat
        .map(|combat| &combat.entities.potions)
        .unwrap_or(&run_state.potions);
    let is_we_meet_again_event = run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.id == crate::state::events::EventId::WeMeetAgain);
    source_potions
        .iter()
        .enumerate()
        .map(|(slot_index, slot)| match slot {
            Some(potion) => RunPotionSlotObservationV0 {
                slot_index,
                potion_id: Some(format!("{:?}", potion.id)),
                uuid: Some(potion.uuid),
                can_use: potion_can_use_in_observation(
                    potion,
                    combat,
                    engine_state,
                    is_we_meet_again_event,
                ),
                can_discard: potion.can_discard
                    && crate::content::potions::potion_can_discard_in_event(is_we_meet_again_event),
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
        boss_node_available: run_state.map.boss_node_available
            || run_state.map.boss_node_available_now(),
        has_emerald_key: run_state.keys[2],
        nodes,
    }
}

pub fn build_map_observation_if_relevant(
    engine_state: &EngineState,
    run_state: &RunState,
) -> Option<RunMapObservationV0> {
    match engine_state {
        EngineState::GameOver(_) => None,
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::EventCombat(_)
        | EngineState::PendingChoice(_)
        | EngineState::RewardScreen(_)
        | EngineState::TreasureRoom(_)
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

pub fn build_map_route_context_if_relevant(run_state: &RunState) -> Option<RunMapRouteContextV1> {
    let route_choices = legal_map_actions(run_state)
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
                Some((
                    action_key_for_input(&ClientInput::SelectMapNode(x), None),
                    x as i32,
                    y,
                ))
            }
            ClientInput::FlyToNode(x, y) => Some((
                action_key_for_input(&ClientInput::FlyToNode(x, y), None),
                x as i32,
                y as i32,
            )),
            _ => None,
        })
        .map(|(action_key, x, y)| map_route_choice(run_state, action_key, x, y))
        .collect::<Vec<_>>();
    if route_choices.is_empty() {
        return None;
    }
    Some(RunMapRouteContextV1 {
        schema_name: "MapRouteContextV1".to_string(),
        schema_version: 1,
        decision_authority: "evidence_only".to_string(),
        not_final_action: true,
        map_scope: "current_to_act_boss".to_string(),
        context_level: "route_envelope_to_act_boss".to_string(),
        current_x: run_state.map.current_x,
        current_y: run_state.map.current_y,
        act_boss: run_state.boss_key.map(|boss| format!("{boss:?}")),
        route_choices,
        truth_warnings: vec![
            "route_envelope_counts_graph_paths_equally_not_player_policy_probability".to_string(),
            "route_summary_excludes_hidden_future_rewards_and_events".to_string(),
            "paths_are_unweighted_graph_paths_not_outcome_probabilities".to_string(),
        ],
    })
}

#[derive(Clone, Debug, Default)]
struct RoutePathStats {
    elites: i32,
    fires: i32,
    shops: i32,
    chests: i32,
    events: i32,
    monsters: i32,
    monsters_next_3: i32,
    first_elite_floor: Option<i32>,
    first_fire_floor: Option<i32>,
    first_shop_floor: Option<i32>,
    rest_before_first_elite: bool,
    burning_elite: bool,
}

const MAX_ROUTE_PATHS: usize = 4096;

fn map_route_choice(
    run_state: &RunState,
    action_key: String,
    x: i32,
    y: i32,
) -> RunMapRouteChoiceV1 {
    let node_obs = map_node_observation(run_state, x, y);
    let mut paths = Vec::new();
    let mut budget_exhausted = false;
    collect_route_paths(
        run_state,
        x,
        y,
        0,
        RoutePathStats::default(),
        &mut paths,
        &mut budget_exhausted,
    );
    let reachable_nodes = reachable_nodes_from(run_state, x, y);
    let branch_count = reachable_nodes.len();
    let shops_reachable = count_reachable_room(run_state, &reachable_nodes, RoomType::ShopRoom);
    let chests_reachable =
        count_reachable_room(run_state, &reachable_nodes, RoomType::TreasureRoom);
    let events_reachable = count_reachable_room(run_state, &reachable_nodes, RoomType::EventRoom);
    let burning_elite_reachable = reachable_nodes.iter().any(|(node_x, node_y)| {
        map_node_at(run_state, *node_x, *node_y).is_some_and(|node| {
            node.has_emerald_key && node.class == Some(RoomType::MonsterRoomElite)
        })
    });
    if paths.is_empty() {
        return RunMapRouteChoiceV1 {
            action_key,
            next_x: x,
            next_y: y,
            room_type: node_obs.room_type.clone(),
            room_label: route_room_label(node_obs.room_type.as_deref(), node_obs.has_emerald_key),
            burning_elite: node_obs.has_emerald_key,
            reachable_paths_to_boss: 0,
            min_elites: 0,
            max_elites: 0,
            expected_elites_milli: 0,
            min_fires: 0,
            max_fires: 0,
            expected_fires_milli: 0,
            min_shops: 0,
            max_shops: 0,
            expected_shops_milli: 0,
            shops_reachable,
            chests_reachable,
            events_reachable,
            forced_fights_next_3: 0,
            earliest_shop_floor: None,
            earliest_fire_floor: None,
            rest_before_first_elite: false,
            local_flex: "none".to_string(),
            global_path_flex: "none".to_string(),
            path_flexibility: "none".to_string(),
            branch_count,
            burning_elite_reachable,
            burning_elite_on_path: node_obs.has_emerald_key,
            risk_label: "unknown_unreachable_boss".to_string(),
            risk_vector: route_risk_vector(0, 0, 0, 0, 0, 0, 0, 0, "none"),
            notes: vec!["no route to boss found from this next node".to_string()],
        };
    }
    let path_count = paths.len();
    let min_elites = paths.iter().map(|path| path.elites).min().unwrap_or(0);
    let max_elites = paths.iter().map(|path| path.elites).max().unwrap_or(0);
    let min_fires = paths.iter().map(|path| path.fires).min().unwrap_or(0);
    let max_fires = paths.iter().map(|path| path.fires).max().unwrap_or(0);
    let min_shops = paths.iter().map(|path| path.shops).min().unwrap_or(0);
    let max_shops = paths.iter().map(|path| path.shops).max().unwrap_or(0);
    let forced_fights_next_3 = paths
        .iter()
        .map(|path| path.monsters_next_3)
        .min()
        .unwrap_or(0);
    let earliest_shop_floor = paths.iter().filter_map(|path| path.first_shop_floor).min();
    let earliest_fire_floor = paths.iter().filter_map(|path| path.first_fire_floor).min();
    let rest_before_first_elite = paths.iter().any(|path| path.rest_before_first_elite);
    let local_flex = route_local_flex_label(branch_count);
    let global_path_flex = route_global_path_flex_label(path_count);
    let path_flexibility = route_combined_flex_label(&local_flex, &global_path_flex);
    let risk_vector = route_risk_vector(
        min_elites,
        max_elites,
        forced_fights_next_3,
        min_fires,
        max_fires,
        min_shops,
        max_shops,
        path_count,
        &global_path_flex,
    );
    let risk_label = route_risk_label(&risk_vector);
    let mut notes = route_notes(
        min_elites,
        max_elites,
        forced_fights_next_3,
        max_shops,
        max_fires,
        rest_before_first_elite,
        burning_elite_reachable,
    );
    if budget_exhausted {
        notes.push("route_path_budget_exhausted_summary_is_truncated".to_string());
    }
    RunMapRouteChoiceV1 {
        action_key,
        next_x: x,
        next_y: y,
        room_type: node_obs.room_type.clone(),
        room_label: route_room_label(node_obs.room_type.as_deref(), node_obs.has_emerald_key),
        burning_elite: node_obs.has_emerald_key,
        reachable_paths_to_boss: path_count,
        min_elites,
        max_elites,
        expected_elites_milli: average_milli(paths.iter().map(|path| path.elites)),
        min_fires,
        max_fires,
        expected_fires_milli: average_milli(paths.iter().map(|path| path.fires)),
        min_shops,
        max_shops,
        expected_shops_milli: average_milli(paths.iter().map(|path| path.shops)),
        shops_reachable,
        chests_reachable,
        events_reachable,
        forced_fights_next_3,
        earliest_shop_floor,
        earliest_fire_floor,
        rest_before_first_elite,
        local_flex,
        global_path_flex,
        path_flexibility,
        branch_count,
        burning_elite_reachable,
        burning_elite_on_path: node_obs.has_emerald_key,
        risk_label,
        risk_vector,
        notes,
    }
}

fn collect_route_paths(
    run_state: &RunState,
    x: i32,
    y: i32,
    depth: usize,
    mut stats: RoutePathStats,
    out: &mut Vec<RoutePathStats>,
    budget_exhausted: &mut bool,
) {
    if out.len() >= MAX_ROUTE_PATHS {
        *budget_exhausted = true;
        return;
    }
    if y == 15 {
        out.push(stats);
        return;
    }
    let Some(node) = map_node_at(run_state, x, y) else {
        return;
    };
    let floor = y + 1;
    if let Some(room_type) = node.class {
        apply_room_to_route_stats(&mut stats, room_type, floor, depth, node.has_emerald_key);
    }
    if y >= 14 {
        out.push(stats);
        return;
    }
    if node.edges.is_empty() {
        return;
    }
    for edge in &node.edges {
        collect_route_paths(
            run_state,
            edge.dst_x,
            edge.dst_y,
            depth + 1,
            stats.clone(),
            out,
            budget_exhausted,
        );
        if *budget_exhausted {
            return;
        }
    }
}

fn apply_room_to_route_stats(
    stats: &mut RoutePathStats,
    room_type: RoomType,
    floor: i32,
    depth: usize,
    burning_elite: bool,
) {
    match room_type {
        RoomType::MonsterRoomElite => {
            stats.elites += 1;
            stats.first_elite_floor.get_or_insert(floor);
            if burning_elite {
                stats.burning_elite = true;
            }
        }
        RoomType::RestRoom => {
            stats.fires += 1;
            stats.first_fire_floor.get_or_insert(floor);
            if stats.first_elite_floor.is_none() {
                stats.rest_before_first_elite = true;
            }
        }
        RoomType::ShopRoom => {
            stats.shops += 1;
            stats.first_shop_floor.get_or_insert(floor);
        }
        RoomType::TreasureRoom => stats.chests += 1,
        RoomType::EventRoom => stats.events += 1,
        RoomType::MonsterRoom => {
            stats.monsters += 1;
            if depth < 3 {
                stats.monsters_next_3 += 1;
            }
        }
        RoomType::MonsterRoomBoss | RoomType::TrueVictoryRoom => {}
    }
}

fn reachable_nodes_from(run_state: &RunState, x: i32, y: i32) -> HashSet<(i32, i32)> {
    let mut out = HashSet::new();
    collect_reachable_nodes(run_state, x, y, &mut out);
    out
}

fn collect_reachable_nodes(run_state: &RunState, x: i32, y: i32, out: &mut HashSet<(i32, i32)>) {
    if y == 15 || !out.insert((x, y)) {
        return;
    }
    let Some(node) = map_node_at(run_state, x, y) else {
        return;
    };
    for edge in &node.edges {
        collect_reachable_nodes(run_state, edge.dst_x, edge.dst_y, out);
    }
}

fn count_reachable_room(
    run_state: &RunState,
    reachable_nodes: &HashSet<(i32, i32)>,
    room_type: RoomType,
) -> i32 {
    reachable_nodes
        .iter()
        .filter(|(x, y)| {
            map_node_at(run_state, *x, *y).is_some_and(|node| node.class == Some(room_type))
        })
        .count() as i32
}

fn map_node_at(run_state: &RunState, x: i32, y: i32) -> Option<&crate::map::node::MapRoomNode> {
    run_state
        .map
        .graph
        .get(y.max(0) as usize)
        .and_then(|row| row.get(x.max(0) as usize))
}

fn average_milli(values: impl Iterator<Item = i32>) -> i32 {
    let mut sum = 0i64;
    let mut count = 0i64;
    for value in values {
        sum += value as i64;
        count += 1;
    }
    if count == 0 {
        0
    } else {
        ((sum * 1000) / count) as i32
    }
}

fn route_room_label(room_type: Option<&str>, burning_elite: bool) -> String {
    let label = match room_type.unwrap_or("Unknown") {
        "MonsterRoom" => "Monster",
        "MonsterRoomElite" => "Elite",
        "MonsterRoomBoss" => "Boss",
        "RestRoom" => "Rest",
        "ShopRoom" => "Shop",
        "TreasureRoom" => "Chest",
        "EventRoom" => "Event",
        other => other,
    };
    if burning_elite {
        format!("{label} [Burning Elite]")
    } else {
        label.to_string()
    }
}

fn route_local_flex_label(branch_count: usize) -> String {
    if branch_count >= 20 {
        "high".to_string()
    } else if branch_count >= 10 {
        "medium".to_string()
    } else if branch_count >= 4 {
        "low".to_string()
    } else {
        "locked".to_string()
    }
}

fn route_global_path_flex_label(path_count: usize) -> String {
    if path_count >= 48 {
        "high".to_string()
    } else if path_count >= 16 {
        "medium".to_string()
    } else if path_count >= 4 {
        "low".to_string()
    } else if path_count >= 1 {
        "locked".to_string()
    } else {
        "none".to_string()
    }
}

fn route_combined_flex_label(local_flex: &str, global_path_flex: &str) -> String {
    format!("local:{local_flex}/global:{global_path_flex}")
}

fn route_risk_vector(
    min_elites: i32,
    max_elites: i32,
    forced_fights_next_3: i32,
    min_fires: i32,
    max_fires: i32,
    min_shops: i32,
    max_shops: i32,
    path_count: usize,
    global_path_flex: &str,
) -> RunRouteRiskVectorV1 {
    let early_pressure = if forced_fights_next_3 >= 3 {
        "high"
    } else if forced_fights_next_3 >= 2 {
        "medium"
    } else if forced_fights_next_3 >= 1 {
        "low"
    } else {
        "none"
    };
    let elite_ceiling = if min_elites >= 1 {
        "forced"
    } else if max_elites >= 3 {
        "high_optional"
    } else if max_elites >= 1 {
        "medium_optional"
    } else if forced_fights_next_3 >= 3 {
        "none"
    } else {
        "none"
    };
    let shop_access = if max_shops <= 0 {
        "none"
    } else if min_shops > 0 {
        "guaranteed"
    } else {
        "optional"
    };
    let recovery_access = if min_fires > 0 {
        "guaranteed"
    } else if max_fires > 0 {
        "optional"
    } else {
        "none"
    };
    let boss_prep_support = if max_shops > 0 && max_fires > 0 && path_count >= 16 {
        "strong"
    } else if max_shops > 0 || max_fires > 0 {
        "moderate"
    } else {
        "weak"
    };
    RunRouteRiskVectorV1 {
        early_pressure: early_pressure.to_string(),
        elite_ceiling: elite_ceiling.to_string(),
        shop_access: shop_access.to_string(),
        recovery_access: recovery_access.to_string(),
        path_flexibility: global_path_flex.to_string(),
        boss_prep_support: boss_prep_support.to_string(),
    }
}

fn route_risk_label(risk: &RunRouteRiskVectorV1) -> String {
    format!(
        "early:{}/elite:{}/shop:{}/recovery:{}/flex:{}/boss_prep:{}",
        risk.early_pressure,
        risk.elite_ceiling,
        risk.shop_access,
        risk.recovery_access,
        risk.path_flexibility,
        risk.boss_prep_support
    )
}

fn route_notes(
    min_elites: i32,
    max_elites: i32,
    forced_fights_next_3: i32,
    max_shops: i32,
    max_fires: i32,
    rest_before_first_elite: bool,
    burning_elite_reachable: bool,
) -> Vec<String> {
    let mut notes = Vec::new();
    if min_elites > 0 {
        notes.push("elite_unavoidable_on_all_paths".to_string());
    } else if max_elites > 0 {
        notes.push("elite_optional_on_some_paths".to_string());
    }
    if forced_fights_next_3 > 0 {
        notes.push(format!(
            "at_least_{forced_fights_next_3}_monster_room(s)_in_next_3"
        ));
    }
    if max_shops > 0 {
        notes.push("shop_reachable_before_boss".to_string());
    }
    if max_fires > 0 {
        notes.push("rest_site_reachable_before_boss".to_string());
    }
    if rest_before_first_elite {
        notes.push("can_rest_before_first_elite_on_some_path".to_string());
    }
    if burning_elite_reachable {
        notes.push("burning_elite_reachable".to_string());
    }
    notes
}

pub fn map_node_observation(run_state: &RunState, x: i32, y: i32) -> RunMapNodeObservationV0 {
    let has_wing_boots = run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::WingBoots && relic.counter > 0);
    let reachable_now = run_state.map.can_travel_to(x, y, false)
        || (has_wing_boots && run_state.map.can_travel_to(x, y, true));
    if y == 15 {
        return RunMapNodeObservationV0 {
            x,
            y,
            room_type: Some("MonsterRoomBoss".to_string()),
            has_emerald_key: false,
            reachable_now,
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
        reachable_now,
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

pub fn build_combat_observation(
    engine_state: &EngineState,
    combat: &CombatState,
) -> RunCombatObservationV0 {
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
        player_powers: build_power_observations(combat, combat.entities.player.id),
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
        pending_choice_kind: run_pending_choice_kind(engine_state),
        pending_choice: build_run_pending_choice_observation(engine_state, combat),
        monsters: build_monster_observations(combat),
        encounter_hints: build_combat_encounter_hints(combat),
    }
}

fn build_monster_observations(combat: &CombatState) -> Vec<RunMonsterObservationV0> {
    combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            let enemy = crate::content::monsters::EnemyId::from_id(monster.monster_type);
            let move_preview =
                crate::projection::combat::project_monster_move_preview_in_combat(combat, monster);
            let visible_intent_kind = format!("{:?}", move_preview.visible_intent);
            let visible_intent = Some(monster_visible_intent_label(
                &visible_intent_kind,
                move_preview.damage_per_hit,
                move_preview.hits,
                move_preview.total_damage,
            ));
            let monster_id = enemy
                .map(|enemy| format!("{enemy:?}"))
                .unwrap_or_else(|| format!("MonsterType{}", monster.monster_type));
            let name = enemy
                .map(|enemy| enemy.get_name().to_string())
                .unwrap_or_else(|| monster_id.clone());
            RunMonsterObservationV0 {
                entity_id: monster.id,
                slot: monster.slot,
                monster_id: monster_id.clone(),
                name,
                current_hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                alive: monster.is_alive_for_action(),
                planned_move_id: monster.planned_move_id(),
                visible_intent,
                visible_intent_kind,
                visible_intent_damage_per_hit: move_preview.damage_per_hit,
                visible_intent_hits: move_preview.hits,
                visible_intent_total_damage: move_preview.total_damage,
                powers: build_power_observations(combat, monster.id),
                mechanic_hints: monster_mechanic_hints(&monster_id),
            }
        })
        .collect()
}

fn monster_visible_intent_label(
    kind: &str,
    damage_per_hit: Option<i32>,
    hits: u8,
    total_damage: Option<i32>,
) -> String {
    let damage_text = match (damage_per_hit, hits) {
        (Some(damage), hits) if hits > 1 => Some(format!("{damage}x{hits}")),
        (Some(damage), _) => Some(damage.to_string()),
        _ => total_damage.map(|damage| damage.to_string()),
    };
    match damage_text {
        Some(damage) if kind.starts_with("Attack") => format!("{kind} {damage}"),
        _ => kind.to_string(),
    }
}

fn build_power_observations(combat: &CombatState, entity_id: usize) -> Vec<RunPowerObservationV0> {
    crate::content::powers::store::powers_for(combat, entity_id)
        .map(|powers| {
            powers
                .iter()
                .map(|power| RunPowerObservationV0 {
                    power_id: format!("{:?}", power.power_type),
                    amount: power.amount,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_combat_encounter_hints(combat: &CombatState) -> Vec<String> {
    let mut hints = Vec::new();
    for monster in &combat.entities.monsters {
        if let Some(enemy) = crate::content::monsters::EnemyId::from_id(monster.monster_type) {
            hints.extend(monster_mechanic_hints(&format!("{enemy:?}")));
        }
    }
    hints.sort();
    hints.dedup();
    hints
}

fn monster_mechanic_hints(monster_id: &str) -> Vec<String> {
    match monster_id {
        "GremlinNob" => vec![
            "mechanic:gremlin_nob_enrage: playing Skill cards increases Gremlin Nob strength".to_string(),
            "strategy:gremlin_nob_plan: prioritize fast damage and only block when it prevents large immediate HP loss".to_string(),
        ],
        "Lagavulin" => vec![
            "mechanic:lagavulin_sleep: starts asleep until damaged or enough turns pass".to_string(),
            "strategy:lagavulin_plan: use asleep turns to set up or deal burst damage before it wakes".to_string(),
            "mechanic:lagavulin_debuff: long fights are dangerous because Lagavulin can reduce strength and dexterity".to_string(),
        ],
        "SlimeBoss" => vec![
            "mechanic:slime_boss_split: splits when pushed below threshold".to_string(),
            "strategy:slime_boss_plan: avoid weak split turns; set up a strong split".to_string(),
        ],
        "Hexaghost" => vec![
            "mechanic:hexaghost_first_attack: first big attack scales with current HP".to_string(),
            "strategy:hexaghost_plan: deck needs frontloaded damage and status handling".to_string(),
        ],
        _ => Vec::new(),
    }
}

fn run_pending_choice_kind(engine_state: &EngineState) -> Option<String> {
    match engine_state {
        EngineState::PendingChoice(choice) => Some(
            match choice {
                PendingChoice::GridSelect { .. } => "grid_select",
                PendingChoice::HandSelect { .. } => "hand_select",
                PendingChoice::DiscoverySelect(_) => "discovery_select",
                PendingChoice::ScrySelect { .. } => "scry_select",
                PendingChoice::CardRewardSelect { .. } => "card_reward_select",
                PendingChoice::ForeignInfluenceSelect { .. } => "foreign_influence_select",
                PendingChoice::ChooseOneSelect { .. } => "choose_one_select",
                PendingChoice::StanceChoice => "stance_choice",
            }
            .to_string(),
        ),
        _ => None,
    }
}

fn build_run_pending_choice_observation(
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<RunPendingChoiceObservationV0> {
    let EngineState::PendingChoice(choice) = engine_state else {
        return None;
    };
    match choice {
        PendingChoice::DiscoverySelect(choice) => Some(RunPendingChoiceObservationV0 {
            kind: "discovery_select".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: choice.can_skip,
            reason: None,
            source_pile: None,
            options: choice
                .cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: crate::content::cards::get_card_definition(*card_id)
                            .name
                            .to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: None,
                        selection_uuids: Vec::new(),
                        source_pile: None,
                        subject_ref: None,
                        before_summary: None,
                        after_summary: None,
                        delta_summary: None,
                        preview_status: None,
                    },
                )
                .collect(),
        }),
        PendingChoice::CardRewardSelect {
            cards,
            destination,
            can_skip,
        } => Some(RunPendingChoiceObservationV0 {
            kind: "card_reward_select".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: *can_skip,
            reason: Some(format!("{destination:?}")),
            source_pile: None,
            options: cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: crate::content::cards::get_card_definition(*card_id)
                            .name
                            .to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: None,
                        selection_uuids: Vec::new(),
                        source_pile: None,
                        subject_ref: None,
                        before_summary: None,
                        after_summary: None,
                        delta_summary: None,
                        preview_status: None,
                    },
                )
                .collect(),
        }),
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => {
            Some(RunPendingChoiceObservationV0 {
                kind: "foreign_influence_select".to_string(),
                min_select: 1,
                max_select: 1,
                can_cancel: false,
                reason: Some(format!("upgraded={upgraded}")),
                source_pile: None,
                options: cards
                    .iter()
                    .enumerate()
                    .map(
                        |(option_index, card_id)| RunPendingChoiceOptionObservationV0 {
                            option_index,
                            label: crate::content::cards::get_card_definition(*card_id)
                                .name
                                .to_string(),
                            card_id: Some(format!("{card_id:?}")),
                            card_uuid: None,
                            selection_uuids: Vec::new(),
                            source_pile: None,
                            subject_ref: None,
                            before_summary: None,
                            after_summary: None,
                            delta_summary: None,
                            preview_status: None,
                        },
                    )
                    .collect(),
            })
        }
        PendingChoice::ChooseOneSelect { choices } => Some(RunPendingChoiceObservationV0 {
            kind: "choose_one_select".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: false,
            reason: None,
            source_pile: None,
            options: choices
                .iter()
                .enumerate()
                .map(|(option_index, choice)| {
                    let def = crate::content::cards::get_card_definition(choice.card_id);
                    RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: if choice.upgrades > 0 {
                            format!("{}+{}", def.name, choice.upgrades)
                        } else {
                            def.name.to_string()
                        },
                        card_id: Some(format!("{:?}", choice.card_id)),
                        card_uuid: None,
                        selection_uuids: Vec::new(),
                        source_pile: None,
                        subject_ref: None,
                        before_summary: None,
                        after_summary: None,
                        delta_summary: None,
                        preview_status: None,
                    }
                })
                .collect(),
        }),
        PendingChoice::StanceChoice => Some(RunPendingChoiceObservationV0 {
            kind: "stance_choice".to_string(),
            min_select: 1,
            max_select: 1,
            can_cancel: false,
            reason: None,
            source_pile: None,
            options: vec![
                RunPendingChoiceOptionObservationV0 {
                    option_index: 0,
                    label: "Wrath".to_string(),
                    card_id: None,
                    card_uuid: None,
                    selection_uuids: Vec::new(),
                    source_pile: None,
                    subject_ref: None,
                    before_summary: None,
                    after_summary: None,
                    delta_summary: None,
                    preview_status: None,
                },
                RunPendingChoiceOptionObservationV0 {
                    option_index: 1,
                    label: "Calm".to_string(),
                    card_id: None,
                    card_uuid: None,
                    selection_uuids: Vec::new(),
                    source_pile: None,
                    subject_ref: None,
                    before_summary: None,
                    after_summary: None,
                    delta_summary: None,
                    preview_status: None,
                },
            ],
        }),
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Some(RunPendingChoiceObservationV0 {
            kind: "hand_select".to_string(),
            min_select: *min_cards,
            max_select: *max_cards,
            can_cancel: *can_cancel,
            reason: Some(format!("{reason:?}")),
            source_pile: Some("Hand".to_string()),
            options: candidate_uuids
                .iter()
                .enumerate()
                .map(|(option_index, uuid)| {
                    let card = find_combat_card_by_uuid(combat, *uuid);
                    let preview = card
                        .map(|card| pending_hand_select_option_preview(reason, card))
                        .unwrap_or_else(|| {
                            PendingChoiceOptionPreview::unavailable("card not found")
                        });
                    RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: card
                            .map(format_combat_card_label)
                            .unwrap_or_else(|| format!("card#{uuid}")),
                        card_id: card.map(|card| format!("{:?}", card.id)),
                        card_uuid: Some(*uuid),
                        selection_uuids: vec![*uuid],
                        source_pile: Some("Hand".to_string()),
                        subject_ref: Some(format!("hand_card_uuid:{uuid}")),
                        before_summary: preview.before_summary,
                        after_summary: preview.after_summary,
                        delta_summary: preview.delta_summary,
                        preview_status: Some(preview.status),
                    }
                })
                .collect(),
        }),
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => Some(RunPendingChoiceObservationV0 {
            kind: "grid_select".to_string(),
            min_select: *min_cards,
            max_select: *max_cards,
            can_cancel: *can_cancel,
            reason: Some(format!("{reason:?}")),
            source_pile: Some(run_pile_type_name(*source_pile)),
            options: candidate_uuids
                .iter()
                .enumerate()
                .map(|(option_index, uuid)| {
                    let card = find_combat_card_by_uuid(combat, *uuid);
                    RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: card
                            .map(format_combat_card_label)
                            .unwrap_or_else(|| format!("card#{uuid}")),
                        card_id: card.map(|card| format!("{:?}", card.id)),
                        card_uuid: Some(*uuid),
                        selection_uuids: vec![*uuid],
                        source_pile: Some(run_pile_type_name(*source_pile)),
                        subject_ref: Some(format!("card_uuid:{uuid}")),
                        before_summary: None,
                        after_summary: None,
                        delta_summary: None,
                        preview_status: None,
                    }
                })
                .collect(),
        }),
        PendingChoice::ScrySelect { cards, card_uuids } => Some(RunPendingChoiceObservationV0 {
            kind: "scry_select".to_string(),
            min_select: 0,
            max_select: cards.len() as u8,
            can_cancel: true,
            reason: None,
            source_pile: Some("Draw".to_string()),
            options: cards
                .iter()
                .enumerate()
                .map(
                    |(option_index, card_id)| RunPendingChoiceOptionObservationV0 {
                        option_index,
                        label: crate::content::cards::get_card_definition(*card_id)
                            .name
                            .to_string(),
                        card_id: Some(format!("{card_id:?}")),
                        card_uuid: card_uuids.get(option_index).copied(),
                        selection_uuids: card_uuids
                            .get(option_index)
                            .copied()
                            .into_iter()
                            .collect(),
                        source_pile: Some("Draw".to_string()),
                        subject_ref: card_uuids
                            .get(option_index)
                            .map(|uuid| format!("draw_card_uuid:{uuid}")),
                        before_summary: None,
                        after_summary: None,
                        delta_summary: None,
                        preview_status: None,
                    },
                )
                .collect(),
        }),
    }
}

fn find_combat_card_by_uuid(
    combat: &CombatState,
    uuid: u32,
) -> Option<&crate::runtime::combat::CombatCard> {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .find(|card| card.uuid == uuid)
        .or_else(|| {
            combat
                .zones
                .queued_cards
                .iter()
                .map(|queued| &queued.card)
                .find(|card| card.uuid == uuid)
        })
}

fn format_combat_card_label(card: &crate::runtime::combat::CombatCard) -> String {
    let name = crate::content::cards::get_card_definition(card.id).name;
    if card.upgrades > 0 {
        format!("{name}+{}", card.upgrades)
    } else {
        name.to_string()
    }
}

struct PendingChoiceOptionPreview {
    before_summary: Option<String>,
    after_summary: Option<String>,
    delta_summary: Option<String>,
    status: String,
}

impl PendingChoiceOptionPreview {
    fn unavailable(reason: &str) -> Self {
        Self {
            before_summary: None,
            after_summary: None,
            delta_summary: None,
            status: format!("unavailable:{reason}"),
        }
    }
}

fn pending_hand_select_option_preview(
    reason: &crate::state::HandSelectReason,
    card: &crate::runtime::combat::CombatCard,
) -> PendingChoiceOptionPreview {
    match reason {
        crate::state::HandSelectReason::Upgrade => {
            let before_upgrades = card.upgrades;
            let after_upgrades = before_upgrades.saturating_add(1);
            PendingChoiceOptionPreview {
                before_summary: Some(card_summary_for_upgrades(card.id, before_upgrades)),
                after_summary: Some(card_summary_for_upgrades(card.id, after_upgrades)),
                delta_summary: Some(format!(
                    "upgrade: {}",
                    card_upgrade_delta_summary(card.id, before_upgrades)
                )),
                status: "available".to_string(),
            }
        }
        crate::state::HandSelectReason::Exhaust => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some("exhaust selected card".to_string()),
            status: "available".to_string(),
        },
        crate::state::HandSelectReason::Discard | crate::state::HandSelectReason::GamblingChip => {
            PendingChoiceOptionPreview {
                before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
                after_summary: None,
                delta_summary: Some("discard selected card".to_string()),
                status: "available".to_string(),
            }
        }
        crate::state::HandSelectReason::PutOnDrawPile | crate::state::HandSelectReason::Setup => {
            PendingChoiceOptionPreview {
                before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
                after_summary: None,
                delta_summary: Some("put selected card on top of draw pile".to_string()),
                status: "available".to_string(),
            }
        }
        crate::state::HandSelectReason::PutToBottomOfDraw => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some("put selected card on bottom of draw pile".to_string()),
            status: "available".to_string(),
        },
        crate::state::HandSelectReason::Retain => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some("retain selected card".to_string()),
            status: "available".to_string(),
        },
        crate::state::HandSelectReason::Recycle => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some("exhaust selected card for energy".to_string()),
            status: "available".to_string(),
        },
        crate::state::HandSelectReason::Copy { amount } => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some(format!("create {amount} copy/copies of selected card")),
            status: "available".to_string(),
        },
        crate::state::HandSelectReason::Nightmare { amount } => PendingChoiceOptionPreview {
            before_summary: Some(card_summary_for_upgrades(card.id, card.upgrades)),
            after_summary: None,
            delta_summary: Some(format!("create {amount} copy/copies next turn")),
            status: "available".to_string(),
        },
    }
}

fn card_summary_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    format!(
        "{} [{}]",
        card_name_for_upgrades(card_id, upgrades),
        card_effect_summary_for_upgrades(card_id, upgrades)
    )
}

fn card_name_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    let def = crate::content::cards::get_card_definition(card_id);
    if upgrades > 0 {
        format!("{}+{}", def.name, upgrades)
    } else {
        def.name.to_string()
    }
}

fn card_effect_summary_for_upgrades(card_id: CardId, upgrades: u8) -> String {
    let parts = card_numeric_effect_parts(card_id, upgrades)
        .into_iter()
        .map(|(label, value)| format!("{label} {value}"))
        .collect::<Vec<_>>();
    if parts.is_empty() {
        "no direct numeric effect".to_string()
    } else {
        parts.join(", ")
    }
}

fn card_upgrade_delta_summary(card_id: CardId, upgrades: u8) -> String {
    let before = card_numeric_effect_parts(card_id, upgrades);
    let after = card_numeric_effect_parts(card_id, upgrades.saturating_add(1));
    let mut labels = before.iter().map(|(label, _)| *label).collect::<Vec<_>>();
    for (label, _) in &after {
        if !labels.contains(label) {
            labels.push(*label);
        }
    }
    let mut deltas = Vec::new();
    for label in labels {
        let before_value = before
            .iter()
            .find(|(candidate, _)| *candidate == label)
            .map(|(_, value)| *value);
        let after_value = after
            .iter()
            .find(|(candidate, _)| *candidate == label)
            .map(|(_, value)| *value);
        if before_value == after_value {
            continue;
        }
        match (before_value, after_value) {
            (Some(before), Some(after)) => deltas.push(format!("{label} {before} -> {after}")),
            (None, Some(after)) => deltas.push(format!("{label} {after}")),
            (Some(before), None) => deltas.push(format!("{label} {before} -> none")),
            (None, None) => {}
        }
    }
    if deltas.is_empty() {
        "no numeric card text delta".to_string()
    } else {
        deltas.join(", ")
    }
}

fn card_numeric_effect_parts(card_id: CardId, upgrades: u8) -> Vec<(&'static str, i32)> {
    let def = crate::content::cards::get_card_definition(card_id);
    let damage = def.base_damage + def.upgrade_damage * upgrades as i32;
    let block = def.base_block + def.upgrade_block * upgrades as i32;
    let magic = def.base_magic + def.upgrade_magic * upgrades as i32;
    let mut parts = Vec::new();
    if damage > 0 {
        parts.push(("dmg", damage));
    }
    if block > 0 || card_is_block_core(card_id) {
        parts.push(("block", block.max(0)));
    }
    if magic > 0 {
        if card_applies_vulnerable(card_id) {
            parts.push(("vuln", magic));
        } else if card_applies_weak(card_id) {
            parts.push(("weak", magic));
        } else if card_draws_cards(card_id) {
            parts.push(("draw", magic));
        } else if card_gains_energy(card_id) {
            parts.push(("energy", magic));
        } else {
            parts.push(("magic", magic));
        }
    }
    parts
}

fn run_pile_type_name(pile: crate::state::core::PileType) -> String {
    match pile {
        crate::state::core::PileType::Draw => "Draw",
        crate::state::core::PileType::Discard => "Discard",
        crate::state::core::PileType::Exhaust => "Exhaust",
        crate::state::core::PileType::Hand => "Hand",
        crate::state::core::PileType::Limbo => "Limbo",
        crate::state::core::PileType::MasterDeck => "MasterDeck",
    }
    .to_string()
}

pub fn build_combat_hand_card_observations(
    combat: &CombatState,
) -> Vec<RunCombatHandCardObservationV0> {
    combat
        .zones
        .hand
        .iter()
        .enumerate()
        .map(|(hand_index, card)| {
            let playable = crate::content::cards::can_play_card(card, combat).is_ok();
            let mut transient_tags = Vec::new();
            transient_tags.push(if playable { "playable" } else { "unplayable" }.to_string());
            if card.cost_for_turn.is_some() {
                transient_tags.push("cost_for_turn_override".to_string());
            }
            if card.free_to_play_once {
                transient_tags.push("free_to_play_once".to_string());
            }

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
            }
        })
        .collect()
}

fn potion_can_use_in_observation(
    potion: &crate::content::potions::Potion,
    combat: Option<&CombatState>,
    engine_state: &EngineState,
    is_we_meet_again_event: bool,
) -> bool {
    if let Some(combat) = combat {
        return matches!(engine_state, EngineState::CombatPlayerTurn)
            && !is_we_meet_again_event
            && crate::content::potions::potion_can_use_in_combat_like_java(potion, combat);
    }

    potion.can_use
        && crate::content::potions::potion_can_use_out_of_combat(potion.id, is_we_meet_again_event)
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
        EngineState::EventRoom => {
            let event_options = event_option_observations(run_state);
            RunScreenObservationV0 {
                event_option_count: event_options
                    .iter()
                    .filter(|option| !option.disabled)
                    .count(),
                event_options,
                ..empty_screen_observation()
            }
        }
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
        event_options: Vec::new(),
        reward_item_count: 0,
        reward_card_choice_count: 0,
        reward_phase: "none".to_string(),
        reward_items: Vec::new(),
        reward_card_choices: Vec::new(),
        reward_claimable_item_count: 0,
        reward_unclaimed_card_item_count: 0,
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
    let reward_phase = if reward_state.pending_card_choice.is_some() {
        "card_choice"
    } else if reward_claimable_item_count > 0 {
        "claim_items"
    } else {
        "cleanup"
    };
    let reward_card_choices = reward_state
        .pending_card_choice
        .as_ref()
        .map(|cards| {
            cards
                .iter()
                .enumerate()
                .map(|(option_index, card)| {
                    reward_card_choice_observation(run_state, option_index, card)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    RunScreenObservationV0 {
        reward_item_count: reward_state.items.len(),
        reward_card_choice_count: reward_state
            .pending_card_choice
            .as_ref()
            .map(Vec::len)
            .unwrap_or(0),
        reward_phase: reward_phase.to_string(),
        reward_items,
        reward_card_choices,
        reward_claimable_item_count,
        reward_unclaimed_card_item_count,
        ..empty_screen_observation()
    }
}

pub fn reward_card_choice_observation(
    run_state: &RunState,
    option_index: usize,
    card: &RewardCard,
) -> RunRewardCardChoiceObservationV0 {
    let def = crate::content::cards::get_card_definition(card.id);
    RunRewardCardChoiceObservationV0 {
        option_index,
        card_id: format!("{:?}", card.id),
        card_name: def.name.to_string(),
        upgrades: card.upgrades,
        card_type: format!("{:?}", def.card_type),
        rarity: format!("{:?}", def.rarity),
        cost: def.cost,
        base_semantics: base_semantics_for_card(card.id, card.upgrades),
        deck_copies: run_state
            .master_deck
            .iter()
            .filter(|deck_card| deck_card.id == card.id)
            .count(),
        card: build_card_feature(card.id, card.upgrades, run_state),
        plan_delta: add_card_plan_delta(card.id, card.upgrades, run_state),
        semantic_descriptor: action_semantic_descriptor_for_reward_card(
            option_index,
            card,
            run_state,
        ),
    }
}

pub fn reward_item_observation(
    run_state: &RunState,
    item_index: usize,
    item: &RewardItem,
) -> RunRewardItemObservationV0 {
    let claimable = reward_item_claimable(run_state, item);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::potions::{Potion, PotionId};
    use crate::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;
    use crate::state::events::{EventId, EventState};

    fn potion_obs_by_id(
        observations: &[RunPotionSlotObservationV0],
        id: PotionId,
    ) -> &RunPotionSlotObservationV0 {
        let needle = format!("{id:?}");
        observations
            .iter()
            .find(|slot| slot.potion_id.as_deref() == Some(needle.as_str()))
            .expect("potion observation should exist")
    }

    #[test]
    fn non_combat_potion_observation_uses_java_can_use_overrides() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions = vec![
            Some(Potion::new(PotionId::BloodPotion, 1)),
            Some(Potion::new(PotionId::FruitJuice, 2)),
            Some(Potion::new(PotionId::EntropicBrew, 3)),
            Some(Potion::new(PotionId::BlockPotion, 4)),
            Some(Potion::new(PotionId::FairyPotion, 5)),
        ];

        let observations = build_potion_observations(&run_state, None, &EngineState::MapNavigation);

        assert!(potion_obs_by_id(&observations, PotionId::BloodPotion).can_use);
        assert!(potion_obs_by_id(&observations, PotionId::FruitJuice).can_use);
        assert!(potion_obs_by_id(&observations, PotionId::EntropicBrew).can_use);
        assert!(!potion_obs_by_id(&observations, PotionId::BlockPotion).can_use);
        assert!(!potion_obs_by_id(&observations, PotionId::FairyPotion).can_use);
        assert!(observations.iter().all(|slot| slot.can_discard));
    }

    #[test]
    fn we_meet_again_blocks_potion_use_and_discard_observation() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::WeMeetAgain));
        run_state.potions = vec![
            Some(Potion::new(PotionId::BloodPotion, 1)),
            Some(Potion::new(PotionId::EntropicBrew, 2)),
            Some(Potion::new(PotionId::FirePotion, 3)),
        ];

        let observations = build_potion_observations(&run_state, None, &EngineState::EventRoom);

        assert!(observations.iter().all(|slot| !slot.can_use));
        assert!(observations.iter().all(|slot| !slot.can_discard));
    }

    #[test]
    fn combat_potion_observation_uses_combat_slots_not_stale_run_slots() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions = vec![Some(Potion::new(PotionId::BloodPotion, 10))];
        let mut combat =
            crate::test_support::combat_with_monsters(vec![crate::test_support::test_monster(
                crate::content::monsters::EnemyId::JawWorm,
            )]);
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 20))];

        let observations =
            build_potion_observations(&run_state, Some(&combat), &EngineState::CombatPlayerTurn);

        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].potion_id.as_deref(), Some("FirePotion"));
        assert!(observations[0].can_use);
        assert!(observations[0].can_discard);
        assert!(observations[0].requires_target);
    }

    #[test]
    fn map_observation_separates_owned_emerald_key_from_emerald_elite_marker() {
        let mut emerald_elite = MapRoomNode::new(0, 0);
        emerald_elite.class = Some(RoomType::MonsterRoomElite);
        emerald_elite.has_emerald_key = true;

        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.map = MapState::new(vec![vec![emerald_elite]]);
        run_state.map.has_emerald_key = false;
        run_state.keys[2] = true;

        let observation = build_map_observation(&run_state);

        assert!(
            observation.has_emerald_key,
            "top-level map observation reports the player's obtained Emerald key"
        );
        assert!(
            observation.nodes[0].has_emerald_key,
            "node-level field still reports the Emerald elite marker"
        );
    }

    #[test]
    fn map_observation_derives_boss_node_availability_from_position() {
        let mut top_rest = MapRoomNode::new(0, 14);
        top_rest.class = Some(RoomType::RestRoom);
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.map = MapState::new(vec![vec![top_rest]]);
        run_state.map.current_x = 0;
        run_state.map.current_y = 14;
        run_state.map.boss_node_available = false;

        assert!(build_map_observation(&run_state).boss_node_available);

        let mut elite = MapRoomNode::new(0, 2);
        elite.class = Some(RoomType::MonsterRoomElite);
        elite.edges.insert(MapEdge::new(0, 2, 0, 3));
        let mut boss = MapRoomNode::new(0, 3);
        boss.class = Some(RoomType::MonsterRoomBoss);
        run_state.map = MapState::new(vec![
            vec![MapRoomNode::new(0, 0)],
            vec![MapRoomNode::new(0, 1)],
            vec![elite],
            vec![boss],
        ]);
        run_state.map.current_x = 0;
        run_state.map.current_y = 2;

        assert!(
            build_map_observation(&run_state).boss_node_available,
            "TheEnding exposes the boss hitbox from the Shield/Spear node even though it is not row 14"
        );
    }

    #[test]
    fn combat_observation_keeps_public_map_context_like_java_top_panel() {
        let mut start = MapRoomNode::new(0, 0);
        start.class = Some(RoomType::MonsterRoom);
        start.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut next = MapRoomNode::new(0, 1);
        next.class = Some(RoomType::ShopRoom);

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.map = MapState::new(vec![vec![start], vec![next]]);
        run_state.map.current_x = 0;
        run_state.map.current_y = 0;

        let ctx = EpisodeContext {
            engine_state: EngineState::CombatPlayerTurn,
            run_state,
            combat_state: Some(crate::test_support::blank_test_combat()),
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let observation = build_observation(&ctx);

        assert!(
            observation.map.is_some(),
            "Java TopPanel allows opening the map during normal combat screen NONE"
        );
        assert_eq!(observation.next_nodes.len(), 1);
        assert_eq!(
            observation.next_nodes[0].room_type.as_deref(),
            Some("ShopRoom")
        );
    }

    #[test]
    fn run_observation_exposes_all_top_panel_keys() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.keys = [true, false, true];
        let ctx = EpisodeContext {
            engine_state: EngineState::MapNavigation,
            run_state,
            combat_state: None,
            stashed_event_combat: None,
            forced_engine_ticks: 0,
            combat_win_count: 0,
        };

        let observation = build_observation(&ctx);

        assert!(observation.keys.ruby);
        assert!(!observation.keys.sapphire);
        assert!(observation.keys.emerald);
    }
}
