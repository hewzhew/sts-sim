use crate::runtime::combat::CombatState;
use crate::state::core::{
    ActiveCombat, ClientInput, CombatContext, CombatStartRequest, EngineState, EventCombatContext,
    PostCombatReturn, RoomCombatContext,
};
use crate::state::map::node::RoomType;
use crate::state::rewards::{RewardScreenContext, TreasureChestSize, TreasureChestState};
use crate::state::run::RunState;
use crate::state::selection::{
    DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
    SelectionTargetRef,
};

fn roll_treasure_chest_spec(run_state: &mut RunState) -> TreasureChestState {
    let roll = run_state.rng_pool.treasure_rng.random_range(0, 99);
    let size = if run_state.act_num >= 4 {
        TreasureChestSize::Medium
    } else if roll < 50 {
        TreasureChestSize::Small
    } else if roll < 83 {
        TreasureChestSize::Medium
    } else {
        TreasureChestSize::Large
    };

    let (common_chance, uncommon_chance, gold_chance, gold_amount) = match size {
        TreasureChestSize::Small => (75, 25, 50, 25),
        TreasureChestSize::Medium => (35, 50, 35, 50),
        TreasureChestSize::Large => (0, 75, 50, 75),
    };

    let reward_roll = run_state.rng_pool.treasure_rng.random_range(0, 99);
    let base_relic_tier = if reward_roll < common_chance {
        crate::content::relics::RelicTier::Common
    } else if reward_roll < common_chance + uncommon_chance {
        crate::content::relics::RelicTier::Uncommon
    } else {
        crate::content::relics::RelicTier::Rare
    };
    let gold_reward_base_amount = if reward_roll < gold_chance {
        Some(gold_amount)
    } else {
        None
    };

    TreasureChestState {
        size,
        base_relic_tier,
        gold_reward_base_amount,
    }
}

fn open_treasure_chest(
    run_state: &mut RunState,
    chest: TreasureChestState,
) -> crate::state::rewards::RewardState {
    let mut reward =
        crate::state::rewards::RewardState::with_context(RewardScreenContext::TreasureRoom);

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
                .card_rng
                .random_range(0, (curse_pool.len() - 1) as i32) as usize;
            run_state.add_card_to_deck_with_upgrades_from(
                curse_pool[idx],
                0,
                crate::state::selection::DomainEventSource::Relic(
                    crate::content::relics::RelicId::CursedKey,
                ),
            );
        }
    }

    // Matryoshka: add an extra relic reward (75% Common, 25% Uncommon)
    if let Some(mat) = run_state
        .relics
        .iter_mut()
        .find(|r| r.id == crate::content::relics::RelicId::Matryoshka && r.counter > 0)
    {
        mat.counter -= 1;
        if mat.counter == 0 {
            mat.counter = -2;
            mat.used_up = true;
        }
        let extra_tier = if run_state.rng_pool.relic_rng.random_boolean_chance(0.75) {
            crate::content::relics::RelicTier::Common
        } else {
            crate::content::relics::RelicTier::Uncommon
        };
        let extra_relic = run_state.random_relic_by_tier(extra_tier);
        reward.items.push(crate::state::rewards::RewardItem::Relic {
            relic_id: extra_relic,
        });
    }

    if let Some(base_amount) = chest.gold_reward_base_amount {
        let amount = if run_state.is_daily_run {
            base_amount
        } else {
            run_state
                .rng_pool
                .treasure_rng
                .random_f32_min_max(base_amount as f32 * 0.9, base_amount as f32 * 1.1)
                .round() as i32
        };
        crate::state::rewards::generator::add_gold_reward_like_java(&mut reward.items, amount);
    }

    // Generate chest relic reward after onChestOpen hooks, matching Java
    // AbstractChest.open(): Matryoshka inserts before the base chest relic,
    // and SapphireKey links to the last relic.
    let relic_id = run_state.random_relic_by_tier(chest.base_relic_tier);
    reward
        .items
        .push(crate::state::rewards::RewardItem::Relic { relic_id });
    if run_state.is_final_act_available && !run_state.keys[1] {
        reward
            .items
            .push(crate::state::rewards::RewardItem::SapphireKey);
    }

    // NlothsMask: remove one relic from rewards (onChestOpenAfter)
    if let Some(mask) = run_state
        .relics
        .iter_mut()
        .find(|r| r.id == crate::content::relics::RelicId::NlothsMask && r.counter > 0)
    {
        mask.counter -= 1;
        if mask.counter == 0 {
            mask.counter = -2;
            mask.used_up = true;
        }
        remove_one_relic_from_rewards_after_chest_open(&mut reward.items);
    }

    reward
}

fn remove_one_relic_from_rewards_after_chest_open(
    items: &mut Vec<crate::state::rewards::RewardItem>,
) {
    if let Some(pos) = items
        .iter()
        .position(|item| matches!(item, crate::state::rewards::RewardItem::Relic { .. }))
    {
        items.remove(pos);
        if matches!(
            items.get(pos),
            Some(crate::state::rewards::RewardItem::SapphireKey)
        ) {
            items.remove(pos);
        }
    }
}

use super::campfire_handler;
use super::shop_handler;

fn bottled_choice_target(
    reason: &crate::state::core::RunPendingChoiceReason,
) -> Option<(
    crate::content::relics::RelicId,
    crate::content::cards::CardType,
)> {
    match reason {
        crate::state::core::RunPendingChoiceReason::BottleFlame => Some((
            crate::content::relics::RelicId::BottledFlame,
            crate::content::cards::CardType::Attack,
        )),
        crate::state::core::RunPendingChoiceReason::BottleLightning => Some((
            crate::content::relics::RelicId::BottledLightning,
            crate::content::cards::CardType::Skill,
        )),
        crate::state::core::RunPendingChoiceReason::BottleTornado => Some((
            crate::content::relics::RelicId::BottledTornado,
            crate::content::cards::CardType::Power,
        )),
        _ => None,
    }
}

fn assign_bottled_card(
    run_state: &mut RunState,
    relic_id: crate::content::relics::RelicId,
    card_type: crate::content::cards::CardType,
    selected_indices: &[usize],
) {
    let Some(&idx) = selected_indices.first() else {
        return;
    };
    let Some(card) = run_state.master_deck.get(idx) else {
        return;
    };
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type != card_type {
        return;
    }

    let selected_uuid = card.uuid as i32;
    if let Some(relic) = run_state
        .relics
        .iter_mut()
        .rev()
        .find(|relic| relic.id == relic_id && relic.amount == 0)
    {
        relic.amount = selected_uuid;
    } else if let Some(relic) = run_state
        .relics
        .iter_mut()
        .rev()
        .find(|relic| relic.id == relic_id)
    {
        relic.amount = selected_uuid;
    }
}

fn run_selection_source(
    run_state: &RunState,
    reason: crate::state::core::RunPendingChoiceReason,
) -> DomainEventSource {
    if let Some(event) = run_state.event_state.as_ref() {
        return DomainEventSource::Event(event.id);
    }

    let has_relic = |id| run_state.relics.iter().any(|relic| relic.id == id);
    match reason {
        crate::state::core::RunPendingChoiceReason::TransformUpgraded
            if has_relic(crate::content::relics::RelicId::Astrolabe) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::Astrolabe)
        }
        crate::state::core::RunPendingChoiceReason::Purge
            if has_relic(crate::content::relics::RelicId::EmptyCage) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::EmptyCage)
        }
        crate::state::core::RunPendingChoiceReason::Duplicate
            if has_relic(crate::content::relics::RelicId::DollysMirror) =>
        {
            DomainEventSource::Relic(crate::content::relics::RelicId::DollysMirror)
        }
        crate::state::core::RunPendingChoiceReason::BottleFlame => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledFlame)
        }
        crate::state::core::RunPendingChoiceReason::BottleLightning => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledLightning)
        }
        crate::state::core::RunPendingChoiceReason::BottleTornado => {
            DomainEventSource::Relic(crate::content::relics::RelicId::BottledTornado)
        }
        reason => DomainEventSource::Selection(reason.into()),
    }
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

fn is_run_level_potion_context(engine_state: &EngineState) -> bool {
    matches!(
        engine_state,
        EngineState::MapNavigation
            | EngineState::EventRoom
            | EngineState::RewardScreen(_)
            | EngineState::TreasureRoom(_)
            | EngineState::Campfire
            | EngineState::Shop(_)
            | EngineState::RunPendingChoice(_)
            | EngineState::BossRelicSelect(_)
    )
}

fn run_event_is_we_meet_again(run_state: &RunState) -> bool {
    run_state
        .event_state
        .as_ref()
        .is_some_and(|event| event.id == crate::state::events::EventId::WeMeetAgain)
}

fn run_has_relic(run_state: &RunState, relic_id: crate::content::relics::RelicId) -> bool {
    run_state.relics.iter().any(|relic| relic.id == relic_id)
}

fn run_potion_potency(run_state: &RunState, potion_id: crate::content::potions::PotionId) -> i32 {
    let mut potency = crate::content::potions::get_potion_definition(potion_id).base_potency;
    if run_has_relic(run_state, crate::content::relics::RelicId::SacredBark) {
        potency *= 2;
    }
    potency
}

fn apply_run_level_on_use_potion_relics(run_state: &mut RunState) {
    if run_has_relic(run_state, crate::content::relics::RelicId::ToyOrnithopter) {
        run_state.heal_with_source(
            5,
            DomainEventSource::Relic(crate::content::relics::RelicId::ToyOrnithopter),
        );
    }
}

fn handle_run_level_potion_input(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    input: &Option<ClientInput>,
) -> bool {
    if !is_run_level_potion_context(engine_state) {
        return false;
    }

    let is_we_meet_again_event = run_event_is_we_meet_again(run_state);
    match input {
        Some(ClientInput::DiscardPotion(slot)) => {
            if !crate::content::potions::potion_can_discard_in_event(is_we_meet_again_event) {
                return true;
            }
            let Some((potion_id, can_discard)) = run_state
                .potions
                .get(*slot)
                .and_then(|slot| slot.as_ref())
                .map(|potion| (potion.id, potion.can_discard))
            else {
                return true;
            };
            if !can_discard {
                return true;
            }
            run_state.remove_potion_at_with_source(*slot, DomainEventSource::Potion(potion_id));
            true
        }
        Some(ClientInput::UsePotion {
            potion_index,
            target,
        }) => {
            if target.is_some() {
                return true;
            }
            let Some((potion_id, can_use)) = run_state
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .map(|potion| (potion.id, potion.can_use))
            else {
                return true;
            };
            if !can_use {
                return true;
            }
            if !crate::content::potions::potion_can_use_out_of_combat(
                potion_id,
                is_we_meet_again_event,
            ) {
                return true;
            }

            let source = DomainEventSource::Potion(potion_id);
            match potion_id {
                crate::content::potions::PotionId::BloodPotion => {
                    let potency = run_potion_potency(run_state, potion_id);
                    let heal_amount = (run_state.max_hp as f32 * (potency as f32 / 100.0)) as i32;
                    run_state.heal_with_source(heal_amount, source);
                    apply_run_level_on_use_potion_relics(run_state);
                    run_state.remove_potion_at_with_source(*potion_index, source);
                }
                crate::content::potions::PotionId::FruitJuice => {
                    let potency = run_potion_potency(run_state, potion_id);
                    run_state.gain_max_hp_with_source(potency, potency, source);
                    apply_run_level_on_use_potion_relics(run_state);
                    run_state.remove_potion_at_with_source(*potion_index, source);
                }
                crate::content::potions::PotionId::EntropicBrew => {
                    let generated =
                        if run_has_relic(run_state, crate::content::relics::RelicId::Sozu) {
                            Vec::new()
                        } else {
                            let potion_slots = run_state.potions.len();
                            let potion_class = run_state.potion_class();
                            (0..potion_slots)
                                .map(|_| {
                                    crate::content::potions::random_potion(
                                        &mut run_state.rng_pool.potion_rng,
                                        potion_class,
                                        false,
                                    )
                                })
                                .collect::<Vec<_>>()
                        };
                    apply_run_level_on_use_potion_relics(run_state);
                    run_state.remove_potion_at_with_source(*potion_index, source);
                    for potion_id in generated {
                        run_state.obtain_potion_with_source(
                            crate::content::potions::Potion::new(potion_id, 0),
                            source,
                        );
                    }
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}

fn apply_combat_meta_change(run_state: &mut RunState, change: crate::runtime::combat::MetaChange) {
    match change {
        crate::runtime::combat::MetaChange::AddCardToMasterDeck(card_id) => {
            run_state.add_card_to_deck(card_id);
        }
        crate::runtime::combat::MetaChange::ModifyCardMisc { card_uuid, amount } => {
            run_state.modify_card_misc_value(card_uuid, amount);
        }
        crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid } => {
            run_state.upgrade_card_with_source(
                card_uuid,
                crate::state::selection::DomainEventSource::DeckMutation,
            );
        }
    }
}

fn combat_start_request_for_room(
    run_state: &mut RunState,
    room_type: RoomType,
) -> Result<CombatStartRequest, String> {
    let encounter = match room_type {
        RoomType::MonsterRoom => run_state
            .peek_next_encounter()
            .ok_or_else(|| "normal encounter queue is empty".to_string())?,
        RoomType::MonsterRoomElite => run_state
            .peek_next_elite()
            .ok_or_else(|| "elite encounter queue is empty".to_string())?,
        RoomType::MonsterRoomBoss => run_state
            .next_boss()
            .ok_or_else(|| "boss encounter queue is empty".to_string())?,
        other => return Err(format!("room type {other:?} is not combat")),
    };
    Ok(CombatStartRequest::room(encounter, room_type))
}

fn start_pending_combat_if_needed(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    active_combat: &mut Option<ActiveCombat>,
) -> bool {
    let request = match engine_state {
        EngineState::CombatStart(request) if active_combat.is_none() => request.clone(),
        EngineState::CombatStart(_) => {
            eprintln!("Error: CombatStart requested while ActiveCombat already exists.");
            return false;
        }
        _ => return true,
    };

    match start_active_combat(run_state, request) {
        Ok(active) => {
            *engine_state = active.engine_state.clone();
            *active_combat = Some(active);
            true
        }
        Err(err) => {
            eprintln!("Combat start error: {err}");
            false
        }
    }
}

fn start_active_combat(
    run_state: &mut RunState,
    request: CombatStartRequest,
) -> Result<ActiveCombat, String> {
    let (engine_state, combat_state) = crate::sim::combat_start::build_natural_combat_start(
        run_state,
        request.encounter_id,
        request.room_type,
    )?;
    Ok(ActiveCombat::new(
        engine_state,
        combat_state,
        request.context,
    ))
}

fn finish_active_combat(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    active_combat: &mut Option<ActiveCombat>,
) -> bool {
    let Some(mut active) = active_combat.take() else {
        eprintln!("Error: tried to finish combat without ActiveCombat.");
        return false;
    };

    run_state.absorb_combat_player(active.combat_state.entities.player.clone());
    run_state.potions = active.combat_state.entities.potions.clone();
    run_state.room_mugged |= active.combat_state.runtime.combat_mugged;
    run_state.room_smoked |= active.combat_state.runtime.combat_smoked;

    for change in active.combat_state.meta.meta_changes.drain(..) {
        apply_combat_meta_change(run_state, change);
    }

    if let EngineState::GameOver(_) = engine_state {
        return false;
    }

    match active.context {
        CombatContext::Room(_) => {
            finish_room_combat(engine_state, run_state, &mut active.combat_state);
        }
        CombatContext::Event(event_context) => {
            finish_event_combat(
                engine_state,
                run_state,
                &mut active.combat_state,
                event_context,
            );
        }
    }

    if matches!(engine_state, EngineState::GameOver(_)) {
        return false;
    }
    start_pending_combat_if_needed(engine_state, run_state, active_combat)
}

fn finish_room_combat(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    combat_state: &mut CombatState,
) {
    // Java: AbstractRoom.update() skips normal reward-screen opening for
    // TheBeyond/TheEnding boss rooms. On A20, ProceedButton sends the player
    // directly to the second Act 3 boss while `bossList.size() == 2`.
    if let EngineState::RewardScreen(rs) = engine_state {
        let is_boss = combat_state.meta.is_boss_fight;
        let is_elite = combat_state.meta.is_elite_fight;

        if is_boss && run_state.act_num == 3 {
            if run_state.should_start_act3_double_boss() {
                run_state.reveal_next_boss_from_list();
                match combat_start_request_for_room(run_state, RoomType::MonsterRoomBoss) {
                    Ok(request) => *engine_state = EngineState::CombatStart(request),
                    Err(err) => {
                        eprintln!("Act 3 double boss start error: {err}");
                        *engine_state =
                            EngineState::GameOver(crate::state::core::RunResult::Defeat);
                    }
                }
            } else if run_state.is_final_act_available
                && run_state.keys[0]
                && run_state.keys[1]
                && run_state.keys[2]
            {
                run_state.enter_final_act();
                *engine_state = EngineState::MapNavigation;
            } else {
                *engine_state = EngineState::GameOver(crate::state::core::RunResult::Victory);
            }
        } else {
            let screen_context = if run_state.room_mugged {
                RewardScreenContext::MuggedCombat
            } else if run_state.room_smoked {
                RewardScreenContext::SmokedCombat
            } else {
                RewardScreenContext::Standard
            };
            let mut existing_items = Vec::new();
            existing_items.append(&mut combat_state.runtime.pending_rewards);
            let normal_monster_rewards_allowed = !combat_state.have_monsters_escaped_java();
            if matches!(screen_context, RewardScreenContext::SmokedCombat) {
                let _hidden_room_rewards =
                    crate::state::rewards::generator::generate_combat_rewards_from_existing_with_escape_gate(
                        run_state,
                        is_elite,
                        is_boss,
                        existing_items,
                        false,
                        normal_monster_rewards_allowed,
                    );
                *rs = crate::state::rewards::RewardState::with_context(
                    RewardScreenContext::SmokedCombat,
                );
            } else {
                *rs = if existing_items.is_empty() && normal_monster_rewards_allowed {
                    crate::state::rewards::generator::generate_combat_rewards(
                        run_state, is_elite, is_boss,
                    )
                } else {
                    crate::state::rewards::generator::generate_combat_rewards_from_existing_with_escape_gate(
                        run_state,
                        is_elite,
                        is_boss,
                        existing_items,
                        true,
                        normal_monster_rewards_allowed,
                    )
                };
                rs.screen_context = screen_context;
            }

            if is_boss && run_state.act_num <= 2 {
                run_state.pending_boss_reward = true;
            }
        }
    }
}

fn finish_event_combat(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    combat_state: &mut CombatState,
    event_context: EventCombatContext,
) {
    if event_context.reward_allowed {
        let mut rewards = event_context.rewards;
        rewards.screen_context = if run_state.room_mugged {
            RewardScreenContext::MuggedCombat
        } else if run_state.room_smoked {
            RewardScreenContext::SmokedCombat
        } else {
            RewardScreenContext::Standard
        };
        if !matches!(rewards.screen_context, RewardScreenContext::SmokedCombat) {
            rewards
                .items
                .append(&mut combat_state.runtime.pending_rewards);
            crate::state::rewards::generator::add_potion_reward_like_java(
                run_state,
                &mut rewards.items,
            );
        }
        if !event_context.no_cards_in_rewards
            && !matches!(rewards.screen_context, RewardScreenContext::SmokedCombat)
        {
            rewards.items.extend(
                crate::state::rewards::generator::generate_card_reward_items(
                    run_state, false, false, false,
                ),
            );
        } else {
            let mut hidden_items = std::mem::take(&mut rewards.items);
            hidden_items.append(&mut combat_state.runtime.pending_rewards);
            crate::state::rewards::generator::add_potion_reward_like_java(
                run_state,
                &mut hidden_items,
            );
        }
        *engine_state = EngineState::RewardScreen(rewards);
    } else {
        match event_context.post_combat_return {
            PostCombatReturn::EventRoom => {
                *engine_state = EngineState::EventRoom;
            }
            PostCombatReturn::MapNavigation => {
                *engine_state = EngineState::MapNavigation;
            }
        }
    }
}

pub fn tick_run(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    combat_state: &mut Option<CombatState>,
    input: Option<ClientInput>,
) -> bool {
    let context = CombatContext::Room(RoomCombatContext {
        room_type: run_state
            .map
            .get_current_room_type()
            .unwrap_or(RoomType::MonsterRoom),
    });
    let mut active_combat = combat_state
        .take()
        .map(|combat| ActiveCombat::new(engine_state.clone(), combat, context));
    let keep_running =
        tick_run_active_with_observer(engine_state, run_state, &mut active_combat, input)
            .keep_running;
    *combat_state = active_combat.map(|active| active.combat_state);
    keep_running
}

#[derive(Clone, Debug)]
pub struct FinishedActiveCombat {
    pub engine_state: EngineState,
    pub combat_state: CombatState,
}

#[derive(Clone, Debug)]
pub struct RunTickOutcome {
    pub keep_running: bool,
    pub finished_combat: Option<FinishedActiveCombat>,
}

impl RunTickOutcome {
    fn without_finished(keep_running: bool) -> Self {
        Self {
            keep_running,
            finished_combat: None,
        }
    }
}

pub fn tick_run_active(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    active_combat: &mut Option<ActiveCombat>,
    input: Option<ClientInput>,
) -> bool {
    tick_run_active_with_observer(engine_state, run_state, active_combat, input).keep_running
}

pub fn tick_run_active_with_observer(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    active_combat: &mut Option<ActiveCombat>,
    input: Option<ClientInput>,
) -> RunTickOutcome {
    if handle_run_level_potion_input(engine_state, run_state, &input) {
        if resolve_out_of_combat_defeat(engine_state, run_state) {
            return RunTickOutcome {
                keep_running: false,
                finished_combat: None,
            };
        }
        return RunTickOutcome {
            keep_running: true,
            finished_combat: None,
        };
    }

    if !start_pending_combat_if_needed(engine_state, run_state, active_combat) {
        return RunTickOutcome {
            keep_running: false,
            finished_combat: None,
        };
    }

    // Top level controller redirecting inputs
    let keep_running = match engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            if let Some(active) = active_combat.as_mut() {
                active.engine_state = engine_state.clone();
                let keep_running = super::core::tick_engine(
                    &mut active.engine_state,
                    &mut active.combat_state,
                    input.clone(),
                );
                *engine_state = active.engine_state.clone();
                if !keep_running {
                    let finished_combat =
                        active_combat.as_ref().map(|active| FinishedActiveCombat {
                            engine_state: active.engine_state.clone(),
                            combat_state: active.combat_state.clone(),
                        });
                    let keep_running = finish_active_combat(engine_state, run_state, active_combat);
                    return RunTickOutcome {
                        keep_running,
                        finished_combat,
                    };
                }
                true
            } else {
                eprintln!("Error: EngineState designates Combat but no ActiveCombat was provided.");
                false
            }
        }
        EngineState::RunPendingChoice(rpc_state) => {
            if let Some(indices) = input
                .clone()
                .and_then(|value| resolve_run_pending_selection(value, run_state))
            {
                if indices.len() < rpc_state.min_choices || indices.len() > rpc_state.max_choices {
                    return RunTickOutcome::without_finished(true);
                }
                let mut seen_indices = Vec::new();
                for &idx in &indices {
                    let Some(card) = run_state.master_deck.get(idx) else {
                        return RunTickOutcome::without_finished(true);
                    };
                    if seen_indices.contains(&idx)
                        || !crate::state::core::run_pending_choice_allows_card_for_run(
                            &rpc_state.reason,
                            card,
                            run_state,
                        )
                    {
                        return RunTickOutcome::without_finished(true);
                    }
                    seen_indices.push(idx);
                }

                let source = run_selection_source(run_state, rpc_state.reason.clone());
                let selection_reason: SelectionReason = rpc_state.reason.clone().into();
                let selected_refs = indices
                    .iter()
                    .filter_map(|&idx| run_state.master_deck.get(idx))
                    .map(|card| SelectionTargetRef::CardUuid(card.uuid))
                    .collect::<Vec<_>>();
                let selected_uuids_in_order = selected_refs
                    .iter()
                    .map(|target| match target {
                        SelectionTargetRef::CardUuid(uuid) => *uuid,
                    })
                    .collect::<Vec<_>>();

                let mut sorted_indices = indices.clone();
                sorted_indices.sort_unstable();
                sorted_indices.reverse(); // Remove from highest index to lowest

                run_state.emit_event(DomainEvent::SelectionResolved {
                    scope: SelectionScope::Deck,
                    reason: selection_reason,
                    selected: selected_refs,
                    source,
                });

                match rpc_state.reason {
                    crate::state::core::RunPendingChoiceReason::Purge
                    | crate::state::core::RunPendingChoiceReason::PurgeNonBottled => {
                        for uuid in selected_uuids_in_order {
                            if let Some(idx) = run_state
                                .master_deck
                                .iter()
                                .position(|card| card.uuid == uuid)
                            {
                                let event_id_for_selection =
                                    run_state.event_state.as_ref().map(|es| es.id);
                                // Store removed card's rarity in event_state.internal_state
                                // so events (bonfire_elementals, bonfire_spirits) can apply
                                // rarity-based rewards after purge returns.
                                // Encoding: 0=Curse, 1=Basic, 2=Common, 3=Special, 4=Uncommon, 5=Rare
                                let def = crate::content::cards::get_card_definition(
                                    run_state.master_deck[idx].id,
                                );
                                let rarity_state = match def.rarity {
                                    crate::content::cards::CardRarity::Curse => 0,
                                    crate::content::cards::CardRarity::Basic => 1,
                                    crate::content::cards::CardRarity::Common => 2,
                                    crate::content::cards::CardRarity::Special => 3,
                                    crate::content::cards::CardRarity::Uncommon => 4,
                                    crate::content::cards::CardRarity::Rare => 5,
                                };
                                if let Some(ref mut es) = run_state.event_state {
                                    es.internal_state = rarity_state;
                                }
                                match event_id_for_selection {
                                    Some(crate::state::events::EventId::BonfireElementals) => {
                                        let mut reward_engine_state = EngineState::EventRoom;
                                        crate::content::events::bonfire_elementals::apply_offer_reward(
                                            &mut reward_engine_state,
                                            run_state,
                                            rarity_state,
                                        );
                                        if let Some(ref mut es) = run_state.event_state {
                                            es.current_screen = 3;
                                        }
                                    }
                                    Some(crate::state::events::EventId::BonfireSpirits) => {
                                        let mut reward_engine_state = EngineState::EventRoom;
                                        crate::content::events::bonfire_spirits::apply_offer_reward(
                                            &mut reward_engine_state,
                                            run_state,
                                            rarity_state,
                                        );
                                        if let Some(ref mut es) = run_state.event_state {
                                            es.current_screen = 3;
                                        }
                                    }
                                    _ => {}
                                }
                                if run_state.event_state.as_ref().is_some_and(|es| {
                                    es.id == crate::state::events::EventId::NoteForYourself
                                }) {
                                    let saved_card = &run_state.master_deck[idx];
                                    run_state.note_for_yourself_card = saved_card.id;
                                    run_state.note_for_yourself_upgrades = saved_card.upgrades;
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
                    crate::state::core::RunPendingChoiceReason::Transform
                    | crate::state::core::RunPendingChoiceReason::TransformNonBottled => {
                        if source == DomainEventSource::Event(crate::state::events::EventId::Neow)
                            && selected_uuids_in_order.len() > 1
                        {
                            run_state.transform_card_uuids_after_removing_all_with_source(
                                &selected_uuids_in_order,
                                false,
                                source,
                            );
                        } else if selected_uuids_in_order.len() > 1 {
                            run_state.transform_card_uuids_deferred_obtain_with_source(
                                &selected_uuids_in_order,
                                false,
                                source,
                            );
                        } else {
                            run_state.transform_card_uuids_with_source(
                                &selected_uuids_in_order,
                                false,
                                source,
                            );
                        }
                    }
                    crate::state::core::RunPendingChoiceReason::TransformUpgraded => {
                        run_state.transform_card_uuids_deferred_obtain_with_source(
                            &selected_uuids_in_order,
                            true,
                            source,
                        );
                    }
                    crate::state::core::RunPendingChoiceReason::Duplicate => {
                        let cards_to_copy: Vec<_> = sorted_indices
                            .iter()
                            .filter_map(|&idx| run_state.master_deck.get(idx).cloned())
                            .collect();
                        for card in cards_to_copy {
                            run_state.add_card_instance_copy_to_deck_from(&card, source);
                        }
                    }
                    reason @ (crate::state::core::RunPendingChoiceReason::BottleFlame
                    | crate::state::core::RunPendingChoiceReason::BottleLightning
                    | crate::state::core::RunPendingChoiceReason::BottleTornado) => {
                        if let Some((relic_id, card_type)) = bottled_choice_target(&reason) {
                            assign_bottled_card(run_state, relic_id, card_type, &sorted_indices);
                        }
                    }
                }

                // Return to the previous stashed state (e.g. Map, Event, or Shop)
                *engine_state = *rpc_state.return_state.clone();
                if run_state.complete_pending_boss_act_transition() {
                    *engine_state = EngineState::MapNavigation;
                }
                if matches!(engine_state, EngineState::EventRoom) {
                    if let Err(e) =
                        crate::engine::event_handler::handle_event_post_run_pending_choice(
                            engine_state,
                            run_state,
                        )
                    {
                        eprintln!("Event post-selection error: {}", e);
                        return RunTickOutcome::without_finished(false);
                    }
                }
            } else if let Some(ClientInput::Cancel) = input {
                // Return to stashed state without mutating deck
                *engine_state = *rpc_state.return_state.clone();
                if run_state.complete_pending_boss_act_transition() {
                    *engine_state = EngineState::MapNavigation;
                }
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
                let previous_room_type = run_state.map.get_current_room_type();
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
                    run_state.complete_current_room_encounter(previous_room_type);
                    run_state.room_mugged = false;
                    run_state.room_smoked = false;
                    run_state.event_state = None;
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
                    if run_state
                        .relics
                        .iter()
                        .any(|r| r.id == crate::content::relics::RelicId::MawBank && !r.used_up)
                    {
                        run_state.change_gold_with_source(
                            12,
                            DomainEventSource::Relic(crate::content::relics::RelicId::MawBank),
                        );
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
                                    run_state.change_hp_with_source(
                                        heal,
                                        DomainEventSource::Relic(
                                            crate::content::relics::RelicId::EternalFeather,
                                        ),
                                    );
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
                                run_state.change_gold_with_source(
                                    50,
                                    DomainEventSource::Relic(
                                        crate::content::relics::RelicId::SsserpentHead,
                                    ),
                                );
                            }
                        }

                        let actual_room_type = if room_type == RoomType::EventRoom {
                            match run_state.roll_question_mark_room_type(previous_room_type) {
                                crate::state::events::generator::RoomRoll::Monster => {
                                    RoomType::MonsterRoom
                                }
                                crate::state::events::generator::RoomRoll::Shop => {
                                    RoomType::ShopRoom
                                }
                                crate::state::events::generator::RoomRoll::Treasure => {
                                    RoomType::TreasureRoom
                                }
                                crate::state::events::generator::RoomRoll::Event => {
                                    RoomType::EventRoom
                                }
                                crate::state::events::generator::RoomRoll::Elite => {
                                    RoomType::MonsterRoomElite
                                }
                            }
                        } else {
                            room_type
                        };

                        if actual_room_type != room_type {
                            let _ = run_state.map.set_current_room_type(actual_room_type);
                        }

                        match actual_room_type {
                            RoomType::MonsterRoom
                            | RoomType::MonsterRoomElite
                            | RoomType::MonsterRoomBoss => {
                                match combat_start_request_for_room(run_state, actual_room_type) {
                                    Ok(request) => {
                                        *engine_state = EngineState::CombatStart(request);
                                    }
                                    Err(err) => {
                                        eprintln!("Combat start request error: {err}");
                                        return RunTickOutcome::without_finished(false);
                                    }
                                }
                            }
                            RoomType::RestRoom => {
                                // Java: onEnterRestRoom() for all relics
                                run_state.on_enter_rest_room();
                                *engine_state = EngineState::Campfire;
                            }
                            RoomType::ShopRoom => {
                                // MealTicket: heal 15 HP on shop entry
                                if run_state
                                    .relics
                                    .iter()
                                    .any(|r| r.id == crate::content::relics::RelicId::MealTicket)
                                    && !run_state.relics.iter().any(|r| {
                                        r.id == crate::content::relics::RelicId::MarkOfTheBloom
                                    })
                                {
                                    run_state.change_hp_with_source(
                                        15,
                                        DomainEventSource::Relic(
                                            crate::content::relics::RelicId::MealTicket,
                                        ),
                                    );
                                }
                                *engine_state = EngineState::Shop(run_state.generate_shop());
                            }
                            RoomType::EventRoom => {
                                let event_id = run_state.generate_event_from_event_room_duplicate();
                                let mut event_state =
                                    crate::state::events::EventState::new(event_id);
                                // Wire init functions for events with constructor-time RNG
                                use crate::state::events::EventId;
                                event_state.internal_state = match event_id {
                                    EventId::Nloth => crate::content::events::nloth::init_nloth_state(run_state),
                                    EventId::DeadAdventurer => crate::content::events::dead_adventurer::init_dead_adventurer_state(run_state),
                                    EventId::Designer => crate::content::events::designer::init_designer_state(run_state),
                                    EventId::WorldOfGoop => crate::content::events::goop_puddle::init_goop_puddle_state(run_state),
                                    EventId::Falling => crate::content::events::falling::init_falling_state(run_state),
                                    _ => 0,
                                };
                                if event_id == EventId::WeMeetAgain {
                                    crate::content::events::we_meet_again::init_we_meet_again_event_state(run_state, &mut event_state);
                                }
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
                                *engine_state =
                                    EngineState::TreasureRoom(roll_treasure_chest_spec(run_state));
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
                return RunTickOutcome::without_finished(false);
            }
            if !start_pending_combat_if_needed(engine_state, run_state, active_combat) {
                return RunTickOutcome::without_finished(false);
            }
            true
        }
        EngineState::TreasureRoom(chest) => {
            match input {
                Some(ClientInput::OpenChest) => {
                    let reward = open_treasure_chest(run_state, *chest);
                    *engine_state = EngineState::RewardScreen(reward);
                }
                Some(ClientInput::Proceed) | Some(ClientInput::Cancel) => {
                    *engine_state = EngineState::MapNavigation;
                }
                _ => {}
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return RunTickOutcome::without_finished(false);
            }
            if !start_pending_combat_if_needed(engine_state, run_state, active_combat) {
                return RunTickOutcome::without_finished(false);
            }
            true
        }
        EngineState::Campfire => {
            let keep_running = campfire_handler::handle(engine_state, run_state, input);
            if !keep_running {
                return RunTickOutcome::without_finished(false);
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return RunTickOutcome::without_finished(false);
            }
            if !start_pending_combat_if_needed(engine_state, run_state, active_combat) {
                return RunTickOutcome::without_finished(false);
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
                return RunTickOutcome::without_finished(false);
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
                return RunTickOutcome::without_finished(false);
            }
            if !start_pending_combat_if_needed(engine_state, run_state, active_combat) {
                return RunTickOutcome::without_finished(false);
            }
            true
        }
        EngineState::RewardScreen(_) => {
            let mut transition = None;
            if let EngineState::RewardScreen(rs) = engine_state {
                if let Some(new_state) =
                    crate::engine::reward_handler::handle(run_state, rs, input.clone())
                {
                    transition = Some(new_state);
                }
            }
            if let Some(new_state) = transition {
                *engine_state = new_state;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return RunTickOutcome::without_finished(false);
            }
            true
        }
        EngineState::BossRelicSelect(_) => {
            let mut transition = None;
            if let EngineState::BossRelicSelect(bs) = engine_state {
                if let Some(new_state) =
                    crate::engine::boss_reward_handler::handle(run_state, bs, input.clone())
                {
                    transition = Some(new_state);
                }
            }
            if let Some(new_state) = transition {
                *engine_state = new_state;
            }
            if resolve_out_of_combat_defeat(engine_state, run_state) {
                return RunTickOutcome::without_finished(false);
            }
            true
        }
        EngineState::CombatStart(_) => {
            start_pending_combat_if_needed(engine_state, run_state, active_combat)
        }
        EngineState::GameOver(_) => false,
    };
    RunTickOutcome {
        keep_running,
        finished_combat: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_combat_meta_change, open_treasure_chest,
        remove_one_relic_from_rewards_after_chest_open, tick_run, tick_run_active,
    };
    use crate::content::cards::CardId;
    use crate::content::relics::{RelicId, RelicState, RelicTier};
    use crate::runtime::combat::CombatCard;
    use crate::runtime::rng::StsRng;
    use crate::state::core::{
        ActiveCombat, ClientInput, CombatContext, EngineState, EventCombatContext, PostCombatReturn,
    };
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;
    use crate::state::rewards::{
        RewardItem, RewardScreenContext, RewardState, TreasureChestSize, TreasureChestState,
    };
    use crate::state::run::RunState;
    use crate::state::selection::{
        DomainEvent, DomainEventSource, SelectionReason, SelectionResolution, SelectionScope,
        SelectionTargetRef,
    };

    fn run_state_with_first_room(room_type: RoomType) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(room_type);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        run_state.map = MapState::new(vec![vec![first], vec![second]]);
        run_state
    }

    #[test]
    fn act3_a20_first_boss_starts_second_boss_without_reward_or_victory() {
        use crate::content::monsters::factory::EncounterId;
        use crate::content::monsters::EnemyId;

        let mut run_state = RunState::new(1, 20, true, "Ironclad");
        run_state.act_num = 3;
        run_state.boss_list = vec![
            EncounterId::AwakenedOne,
            EncounterId::TimeEater,
            EncounterId::DonuAndDeca,
        ];
        run_state.boss_key = Some(EncounterId::AwakenedOne);
        assert_eq!(run_state.next_boss(), Some(EncounterId::AwakenedOne));

        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        let mut boss = crate::test_support::test_monster(EnemyId::AwakenedOne);
        boss.current_hp = 0;
        boss.is_dying = true;
        combat.entities.monsters.push(boss);

        let mut engine_state = EngineState::CombatProcessing;
        let mut combat_state = Some(combat);

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            None,
        ));

        assert!(matches!(engine_state, EngineState::CombatPlayerTurn));
        assert!(combat_state.is_some());
        assert_eq!(run_state.boss_key, Some(EncounterId::TimeEater));
        assert_eq!(run_state.boss_list, vec![EncounterId::DonuAndDeca]);
    }

    #[test]
    fn act3_boss_with_all_keys_enters_initialized_final_act() {
        use crate::content::monsters::factory::EncounterId;
        use crate::content::monsters::EnemyId;

        let mut run_state = RunState::new(1, 19, true, "Ironclad");
        run_state.act_num = 3;
        run_state.keys = [true, true, true];
        run_state.boss_list = vec![
            EncounterId::AwakenedOne,
            EncounterId::TimeEater,
            EncounterId::DonuAndDeca,
        ];
        run_state.boss_key = Some(EncounterId::AwakenedOne);
        assert_eq!(run_state.next_boss(), Some(EncounterId::AwakenedOne));

        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        let mut boss = crate::test_support::test_monster(EnemyId::AwakenedOne);
        boss.current_hp = 0;
        boss.is_dying = true;
        combat.entities.monsters.push(boss);

        let mut engine_state = EngineState::CombatProcessing;
        let mut combat_state = Some(combat);

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            None,
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(run_state.act_num, 4);
        assert_eq!(
            run_state.elite_monster_list,
            vec![EncounterId::ShieldAndSpear; 3]
        );
        assert_eq!(run_state.boss_key, Some(EncounterId::TheHeart));
        assert!(combat_state.is_none());
    }

    #[test]
    fn event_combat_rewards_do_not_call_standard_combat_loot_generator() {
        use crate::content::monsters::EnemyId;

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::WhiteBeastStatue));
        let treasure_before = run_state.rng_pool.treasure_rng.counter;
        let relic_before = run_state.rng_pool.relic_rng.counter;
        let potion_before = run_state.rng_pool.potion_rng.counter;

        let mut event_rewards = RewardState::new();
        event_rewards.items.push(RewardItem::Gold { amount: 100 });
        let mut engine_state = EngineState::CombatProcessing;
        let event_context = EventCombatContext {
            rewards: event_rewards,
            reward_allowed: true,
            no_cards_in_rewards: false,
            elite_trigger: false,
            post_combat_return: PostCombatReturn::MapNavigation,
        };

        let mut combat = crate::test_support::blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::WhiteBeastStatue));
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.current_hp = 0;
        monster.is_dying = true;
        combat.entities.monsters.push(monster);
        let mut active_combat = Some(ActiveCombat::new(
            EngineState::CombatProcessing,
            combat,
            CombatContext::Event(event_context),
        ));

        assert!(tick_run_active(
            &mut engine_state,
            &mut run_state,
            &mut active_combat,
            Some(ClientInput::EndTurn),
        ));

        let EngineState::RewardScreen(rewards) = engine_state else {
            panic!("event combat should open a reward screen");
        };
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, treasure_before,
            "EventRoom combat does not add standard monster gold rewards"
        );
        assert_eq!(
            run_state.rng_pool.relic_rng.counter, relic_before,
            "EventRoom combat does not call MonsterRoomElite.dropReward or random relic reward generation"
        );
        assert!(
            run_state.rng_pool.potion_rng.counter > potion_before,
            "EventRoom addPotionToRewards still uses potionRng"
        );
        assert_eq!(run_state.potion_drop_chance_mod, -10);
        assert!(matches!(rewards.items[0], RewardItem::Gold { amount: 100 }));
        assert!(matches!(rewards.items[1], RewardItem::Potion { .. }));
        assert!(matches!(rewards.items[2], RewardItem::Card { .. }));
        assert_eq!(
            rewards
                .items
                .iter()
                .filter(|item| matches!(item, RewardItem::Gold { .. }))
                .count(),
            1,
            "event combat keeps pre-populated event gold without adding standard monster gold"
        );
    }

    #[test]
    fn finished_combat_syncs_potion_slots_back_to_run_state() {
        use crate::content::monsters::EnemyId;
        use crate::content::potions::{Potion, PotionId};

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions[0] = Some(Potion::new(PotionId::FruitJuice, 42));

        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = run_state.potions.clone();
        combat.entities.potions[0] = None;
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.current_hp = 0;
        monster.is_dying = true;
        combat.entities.monsters.push(monster);

        let mut engine_state = EngineState::CombatProcessing;
        let mut active_combat = Some(ActiveCombat::new(
            EngineState::CombatProcessing,
            combat,
            CombatContext::Room(crate::state::core::RoomCombatContext {
                room_type: crate::state::map::node::RoomType::MonsterRoom,
            }),
        ));

        assert!(tick_run_active(
            &mut engine_state,
            &mut run_state,
            &mut active_combat,
            Some(ClientInput::EndTurn),
        ));
        assert!(
            run_state.potions[0].is_none(),
            "combat potion inventory must persist after combat ends"
        );
    }

    #[test]
    fn smoked_combat_consumes_hidden_room_reward_rng_without_visible_rewards() {
        use crate::content::monsters::EnemyId;

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::WhiteBeastStatue));
        let treasure_before = run_state.rng_pool.treasure_rng.counter;
        let potion_before = run_state.rng_pool.potion_rng.counter;
        let card_before = run_state.rng_pool.card_rng.counter;

        let mut combat = crate::test_support::blank_test_combat();
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::WhiteBeastStatue));
        combat.runtime.combat_smoked = true;
        combat
            .runtime
            .pending_rewards
            .push(RewardItem::StolenGold { amount: 40 });
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.current_hp = 0;
        monster.is_dying = true;
        combat.entities.monsters.push(monster);

        let mut engine_state = EngineState::CombatProcessing;
        let mut combat_state = Some(combat);

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            None,
        ));

        let EngineState::RewardScreen(rewards) = engine_state else {
            panic!("smoked combat should still reach a reward/proceed screen");
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::SmokedCombat);
        assert!(
            rewards.items.is_empty(),
            "Java openCombat(smoked=true) does not call setupItemReward, so generated room rewards are not visible"
        );
        assert!(
            run_state.rng_pool.treasure_rng.counter > treasure_before,
            "Java still adds normal room gold before opening the smoked reward screen"
        );
        assert!(
            run_state.rng_pool.potion_rng.counter > potion_before,
            "Java still calls addPotionToRewards before opening the smoked reward screen"
        );
        assert_eq!(
            run_state.rng_pool.card_rng.counter, card_before,
            "Java smoked reward screen skips CombatRewardScreen.setupItemReward card generation"
        );
        assert_eq!(run_state.potion_drop_chance_mod, -10);
    }

    #[test]
    fn mugged_all_escaped_normal_combat_skips_standard_gold_and_base_potion_chance() {
        use crate::content::monsters::EnemyId;

        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        let treasure_before = run_state.rng_pool.treasure_rng.counter;
        let potion_before = run_state.rng_pool.potion_rng.counter;

        let mut combat = crate::test_support::blank_test_combat();
        combat.runtime.combat_mugged = true;
        let mut monster = crate::test_support::test_monster(EnemyId::Looter);
        monster.is_escaped = true;
        combat.entities.monsters.push(monster);

        let mut engine_state = EngineState::CombatProcessing;
        let mut combat_state = Some(combat);

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            None,
        ));

        let EngineState::RewardScreen(rewards) = engine_state else {
            panic!("mugged escaped combat should still open a reward screen");
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::MuggedCombat);
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, treasure_before,
            "Java skips ordinary MonsterRoom gold when every monster escaped"
        );
        assert_eq!(
            run_state.rng_pool.potion_rng.counter,
            potion_before + 1,
            "Java addPotionToRewards still rolls potionRng even when escaped monsters force chance to 0"
        );
        assert_eq!(
            run_state.potion_drop_chance_mod, 10,
            "the chance-0 potion roll follows the Java miss path"
        );
        assert!(
            !rewards
                .items
                .iter()
                .any(|item| matches!(item, RewardItem::Gold { .. } | RewardItem::Potion { .. })),
            "all-escaped ordinary MonsterRoom should not create standard gold or a base potion reward"
        );
        assert!(
            rewards
                .items
                .iter()
                .any(|item| matches!(item, RewardItem::Card { .. })),
            "CombatRewardScreen.setupItemReward still appends card rewards for mugged combat"
        );
    }

    #[test]
    fn meal_ticket_shop_entry_heal_uses_relic_source_and_mark_of_bloom_guard() {
        let mut run_state = run_state_with_first_room(RoomType::ShopRoom);
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::MealTicket));
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(run_state.current_hp, 35);
        assert!(matches!(engine_state, EngineState::Shop(_)));

        let mut blocked = run_state_with_first_room(RoomType::ShopRoom);
        blocked.current_hp = 20;
        blocked.max_hp = 80;
        blocked.relics.clear();
        blocked.relics.push(RelicState::new(RelicId::MealTicket));
        blocked
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        let mut blocked_engine = EngineState::MapNavigation;
        let mut blocked_combat = None;

        assert!(tick_run(
            &mut blocked_engine,
            &mut blocked,
            &mut blocked_combat,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(blocked.current_hp, 20);
        assert!(matches!(blocked_engine, EngineState::Shop(_)));
    }

    #[test]
    fn treasure_room_uses_java_chest_reward_rolls_before_relic_pool_draw() {
        fn small_gold_common_chest_seed() -> u64 {
            (1..10_000)
                .find(|seed| {
                    let mut rng = StsRng::new(*seed);
                    rng.random_range(0, 99) < 50 && rng.random_range(0, 99) < 50
                })
                .expect("seed for small chest with gold and common relic")
        }

        let mut run_state = run_state_with_first_room(RoomType::TreasureRoom);
        run_state.relics.clear();
        run_state.rng_pool.treasure_rng = StsRng::new(small_gold_common_chest_seed());
        run_state.common_relic_pool = vec![RelicId::Anchor];
        run_state.uncommon_relic_pool = vec![RelicId::Sundial];
        run_state.rare_relic_pool = vec![RelicId::Mango];
        let relic_rng_before = run_state.rng_pool.relic_rng.counter;

        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert!(matches!(engine_state, EngineState::TreasureRoom(_)));
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::OpenChest),
        ));

        let EngineState::RewardScreen(rewards) = engine_state else {
            panic!("treasure room should open a reward screen");
        };
        assert!(
            matches!(rewards.items[0], RewardItem::Gold { .. }),
            "Java AbstractChest.open adds chest gold before the base chest relic"
        );
        assert_eq!(
            rewards
                .items
                .iter()
                .filter_map(|item| match item {
                    RewardItem::Relic { relic_id } => Some(*relic_id),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            vec![RelicId::Anchor],
            "Java chest reward tier is decided by treasureRng, then removes from that tier pool"
        );
        assert_eq!(
            run_state.rng_pool.relic_rng.counter, relic_rng_before,
            "Java chest tier selection does not consume relicRng"
        );
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, 3,
            "Java consumes treasureRng for chest size, chest reward roll, and non-daily gold jitter"
        );
    }

    #[test]
    fn treasure_room_gold_reward_does_not_receive_golden_idol_bonus() {
        fn small_gold_common_chest_seed() -> u64 {
            (1..10_000)
                .find(|seed| {
                    let mut rng = StsRng::new(*seed);
                    rng.random_range(0, 99) < 50 && rng.random_range(0, 99) < 50
                })
                .expect("seed for small chest with gold and common relic")
        }

        let mut run_state = run_state_with_first_room(RoomType::TreasureRoom);
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::GoldenIdol));
        run_state.rng_pool.treasure_rng = StsRng::new(small_gold_common_chest_seed());
        run_state.common_relic_pool = vec![RelicId::Anchor];

        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert!(matches!(engine_state, EngineState::TreasureRoom(_)));
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::OpenChest),
        ));

        let EngineState::RewardScreen(mut rewards) = engine_state else {
            panic!("treasure room should open a reward screen");
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::TreasureRoom);
        let RewardItem::Gold { amount } = rewards.items[0] else {
            panic!("small chest seed should create chest gold");
        };
        let gold_before = run_state.gold;

        crate::engine::reward_handler::handle(
            &mut run_state,
            &mut rewards,
            Some(ClientInput::ClaimReward(0)),
        );

        assert_eq!(
            run_state.gold,
            gold_before + amount,
            "Java RewardItem.applyGoldBonus skips Golden Idol inside TreasureRoom"
        );
    }

    #[test]
    fn treasure_room_chest_can_be_skipped_after_entry_like_java_complete_room() {
        fn small_gold_common_chest_seed() -> u64 {
            (1..10_000)
                .find(|seed| {
                    let mut rng = StsRng::new(*seed);
                    rng.random_range(0, 99) < 50 && rng.random_range(0, 99) < 50
                })
                .expect("seed for small chest with gold and common relic")
        }

        let mut run_state = run_state_with_first_room(RoomType::TreasureRoom);
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::CursedKey));
        run_state.rng_pool.treasure_rng = StsRng::new(small_gold_common_chest_seed());
        run_state.common_relic_pool = vec![RelicId::Anchor];
        let deck_before = run_state.master_deck.len();
        let relic_pool_before = run_state.common_relic_pool.clone();

        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert!(matches!(engine_state, EngineState::TreasureRoom(_)));
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, 2,
            "Java TreasureRoom.onPlayerEntry constructs/randomizes the chest before opening"
        );

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::Proceed),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(
            run_state.master_deck.len(),
            deck_before,
            "Cursed Key only fires from AbstractChest.open(false), not from entering or skipping the room"
        );
        assert_eq!(
            run_state.common_relic_pool, relic_pool_before,
            "Skipping the chest must not consume the chest relic reward"
        );
        assert_eq!(
            run_state.rng_pool.treasure_rng.counter, 2,
            "Skipping avoids the non-daily chest gold jitter consumed inside AbstractChest.open"
        );
    }

    #[test]
    fn cursed_key_chest_obtain_hooks_run_before_curse_obtained_event() {
        let mut run_state = run_state_with_first_room(RoomType::TreasureRoom);
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::CursedKey));
        run_state.relics.push(RelicState::new(RelicId::CeramicFish));
        run_state.common_relic_pool = vec![RelicId::Anchor];

        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert!(matches!(engine_state, EngineState::TreasureRoom(_)));
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::OpenChest),
        ));

        let events = run_state.take_emitted_events();
        let fish_gold_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::GoldChanged {
                        delta: 9,
                        source: DomainEventSource::Relic(RelicId::CursedKey),
                        ..
                    }
                )
            })
            .expect("Cursed Key chest curse should run Ceramic Fish obtain hook");
        let obtained_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::CardObtained {
                        card,
                        source: DomainEventSource::Relic(RelicId::CursedKey),
                    } if crate::content::cards::get_curse_pool().contains(&card.id)
                )
            })
            .expect("Cursed Key chest opening should obtain a random curse");

        assert!(
            fish_gold_pos < obtained_pos,
            "Java CursedKey.onChestOpen queues ShowCardAndObtainEffect; that effect runs onObtainCard before Soul.obtain"
        );
    }

    #[test]
    fn question_mark_tiny_chest_forces_actual_treasure_after_event_room_enter_hooks() {
        fn small_gold_common_chest_seed() -> u64 {
            (1..10_000)
                .find(|seed| {
                    let mut rng = StsRng::new(*seed);
                    rng.random_range(0, 99) < 50 && rng.random_range(0, 99) < 50
                })
                .expect("seed for small chest with gold and common relic")
        }

        let mut run_state = run_state_with_first_room(RoomType::EventRoom);
        run_state.relics.clear();
        let mut tiny_chest = RelicState::new(RelicId::TinyChest);
        tiny_chest.counter = 3;
        run_state.relics.push(tiny_chest);
        run_state
            .relics
            .push(RelicState::new(RelicId::SsserpentHead));
        run_state.rng_pool.treasure_rng = StsRng::new(small_gold_common_chest_seed());
        run_state.common_relic_pool = vec![RelicId::Anchor];
        let gold_before = run_state.gold;

        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));

        assert_eq!(
            run_state.gold,
            gold_before + 50,
            "Java SsserpentHead sees the original ? EventRoom during onEnterRoom, before EventHelper.roll replaces it"
        );
        assert_eq!(
            run_state.map.get_current_room_type(),
            Some(RoomType::TreasureRoom),
            "Java EventHelper.roll replaces the ? room with the actual rolled room"
        );
        let tiny_chest = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::TinyChest)
            .expect("Tiny Chest should be present");
        assert_eq!(tiny_chest.counter, 0);
        assert_eq!(
            run_state.rng_pool.event_rng.counter, 1,
            "Java still consumes eventRng for EventHelper.roll before Tiny Chest forces the result"
        );
        assert!(
            run_state.event_state.is_none(),
            "forced treasure must not continue into specific event generation"
        );
        assert!(matches!(engine_state, EngineState::TreasureRoom(_)));
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::OpenChest),
        ));
        assert!(matches!(engine_state, EngineState::RewardScreen(_)));
    }

    #[test]
    fn event_room_specific_event_selection_uses_duplicate_event_rng_like_java() {
        use crate::state::events::EventId;

        let mut run_state = run_state_with_first_room(RoomType::EventRoom);
        run_state.event_generator.monster_chance = 0.0;
        run_state.event_generator.shop_chance = 0.0;
        run_state.event_generator.treasure_chance = 0.0;
        run_state.event_generator.shrine_chance = 0.0;
        run_state.event_generator.event_pool = vec![EventId::BigFish];
        run_state.event_generator.shrine_pool.clear();
        run_state.event_generator.one_time_event_pool.clear();
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));

        assert!(matches!(engine_state, EngineState::EventRoom));
        assert_eq!(
            run_state
                .event_state
                .as_ref()
                .expect("event state should be initialized")
                .id,
            EventId::BigFish
        );
        assert!(
            run_state.event_generator.event_pool.is_empty(),
            "Java generateEvent mutates the event pool even though it uses a duplicate RNG"
        );
        assert_eq!(
            run_state.rng_pool.event_rng.counter, 1,
            "Java commits only EventHelper.roll's eventRng consumption; EventRoom.onPlayerEntry selects the concrete event with a duplicate RNG"
        );
    }

    #[test]
    fn eternal_feather_rest_room_heal_uses_relic_source_and_mark_of_bloom_guard() {
        let mut run_state = run_state_with_first_room(RoomType::RestRoom);
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state
            .relics
            .push(RelicState::new(RelicId::EternalFeather));
        run_state.master_deck = (0..10)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect();
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(run_state.current_hp, 26);
        assert!(matches!(engine_state, EngineState::Campfire));

        let mut blocked = run_state_with_first_room(RoomType::RestRoom);
        blocked.current_hp = 20;
        blocked.max_hp = 80;
        blocked.relics.clear();
        blocked
            .relics
            .push(RelicState::new(RelicId::EternalFeather));
        blocked
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        blocked.master_deck = (0..10)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect();
        let mut blocked_engine = EngineState::MapNavigation;
        let mut blocked_combat = None;

        assert!(tick_run(
            &mut blocked_engine,
            &mut blocked,
            &mut blocked_combat,
            Some(ClientInput::SelectMapNode(0)),
        ));
        assert_eq!(blocked.current_hp, 20);
        assert!(matches!(blocked_engine, EngineState::Campfire));
    }

    #[test]
    fn run_level_blood_potion_uses_sacred_bark_toy_ornithopter_and_consumes_slot() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 10;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::SacredBark));
        run_state
            .relics
            .push(RelicState::new(RelicId::ToyOrnithopter));
        run_state.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::BloodPotion,
            101,
        ))];
        run_state.emitted_events.clear();
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            }),
        ));

        assert_eq!(run_state.current_hp, 47);
        assert!(run_state.potions[0].is_none());
        assert!(run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::HpChanged {
                delta: 32,
                source: DomainEventSource::Potion(crate::content::potions::PotionId::BloodPotion),
                ..
            }
        )));
        assert!(run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::HpChanged {
                delta: 5,
                source: DomainEventSource::Relic(RelicId::ToyOrnithopter),
                ..
            }
        )));
        assert!(run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::PotionLost {
                potion_id: crate::content::potions::PotionId::BloodPotion,
                slot: 0,
                source: DomainEventSource::Potion(crate::content::potions::PotionId::BloodPotion),
            }
        )));
    }

    #[test]
    fn run_level_potion_discard_is_blocked_by_we_meet_again() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(crate::state::events::EventState::new(
            crate::state::events::EventId::WeMeetAgain,
        ));
        run_state.potions = vec![Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::FirePotion,
            101,
        ))];
        let mut engine_state = EngineState::EventRoom;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::DiscardPotion(0)),
        ));

        assert_eq!(
            run_state.potions[0].as_ref().map(|potion| potion.id),
            Some(crate::content::potions::PotionId::FirePotion)
        );
    }

    #[test]
    fn run_level_potion_execution_respects_imported_affordance_flags() {
        let mut disabled_use = RunState::new(1, 0, false, "Ironclad");
        disabled_use.current_hp = 10;
        disabled_use.max_hp = 80;
        disabled_use.potions = vec![Some(
            crate::content::potions::Potion::with_affordance_truth(
                crate::content::potions::PotionId::BloodPotion,
                101,
                false,
                true,
                false,
            ),
        )];
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut disabled_use,
            &mut combat_state,
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            }),
        ));

        assert_eq!(disabled_use.current_hp, 10);
        assert!(
            disabled_use.potions[0].is_some(),
            "Java PotionPopUp checks potion.canUse before calling use()"
        );

        let mut disabled_discard = RunState::new(1, 0, false, "Ironclad");
        disabled_discard.potions = vec![Some(
            crate::content::potions::Potion::with_affordance_truth(
                crate::content::potions::PotionId::FirePotion,
                102,
                false,
                false,
                true,
            ),
        )];
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut disabled_discard,
            &mut combat_state,
            Some(ClientInput::DiscardPotion(0)),
        ));

        assert!(
            disabled_discard.potions[0].is_some(),
            "Java PotionPopUp checks potion.canDiscard before destroying the slot"
        );
    }

    #[test]
    fn run_level_entropic_brew_consumes_slot_and_refills_without_limited_filter() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.potions = vec![
            Some(crate::content::potions::Potion::new(
                crate::content::potions::PotionId::EntropicBrew,
                101,
            )),
            None,
            None,
        ];
        let mut engine_state = EngineState::RewardScreen(crate::state::rewards::RewardState::new());
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            }),
        ));

        assert_eq!(
            run_state
                .potions
                .iter()
                .filter(|slot| slot.is_some())
                .count(),
            3
        );
        assert!(run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::PotionLost {
                potion_id: crate::content::potions::PotionId::EntropicBrew,
                slot: 0,
                source: DomainEventSource::Potion(crate::content::potions::PotionId::EntropicBrew),
            }
        )));
        assert_eq!(
            run_state
                .emitted_events
                .iter()
                .filter(|event| matches!(
                    event,
                    crate::state::selection::DomainEvent::PotionObtained {
                        source: DomainEventSource::Potion(
                            crate::content::potions::PotionId::EntropicBrew
                        ),
                        ..
                    }
                ))
                .count(),
            3
        );
    }

    #[test]
    fn run_level_entropic_brew_with_sozu_consumes_without_generating_potions() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 10;
        run_state.max_hp = 80;
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Sozu));
        run_state
            .relics
            .push(RelicState::new(RelicId::ToyOrnithopter));
        run_state.potions = vec![
            Some(crate::content::potions::Potion::new(
                crate::content::potions::PotionId::EntropicBrew,
                101,
            )),
            None,
            None,
        ];
        let potion_rng_before = run_state.rng_pool.potion_rng.counter;
        let mut engine_state = EngineState::MapNavigation;
        let mut combat_state = None;

        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            }),
        ));

        assert!(run_state.potions.iter().all(|slot| slot.is_none()));
        assert_eq!(
            run_state.rng_pool.potion_rng.counter, potion_rng_before,
            "Java EntropicBrew non-combat Sozu branch flashes Sozu and does not call returnRandomPotion"
        );
        assert_eq!(
            run_state.current_hp, 15,
            "Java PotionPopUp still calls relic onUsePotion after EntropicBrew.use(), even when Sozu blocks potion generation"
        );
        assert!(!run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::PotionObtained { .. }
        )));
        assert!(run_state.emitted_events.iter().any(|event| matches!(
            event,
            crate::state::selection::DomainEvent::HpChanged {
                delta: 5,
                source: DomainEventSource::Relic(RelicId::ToyOrnithopter),
                ..
            }
        )));
    }

    #[test]
    fn bottled_relic_on_equip_filters_selection_by_card_type_and_marks_uuid() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.master_deck = vec![
            CombatCard::new(CardId::Bash, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::Inflame, 103),
        ];

        let next_state = run_state
            .obtain_relic_with_source(
                RelicId::BottledFlame,
                EngineState::MapNavigation,
                DomainEventSource::RewardScreen,
            )
            .expect("Bottled Flame should open a deck selection when an attack exists");

        let EngineState::RunPendingChoice(choice) = next_state else {
            panic!("Bottled Flame should return RunPendingChoice");
        };
        let request = choice.selection_request(&run_state);
        assert_eq!(request.reason, SelectionReason::BottleFlame);
        assert_eq!(request.targets, vec![SelectionTargetRef::CardUuid(101)]);

        let mut engine_state = EngineState::RunPendingChoice(choice);
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::BottledFlame)
                .map(|relic| relic.amount),
            Some(101)
        );
    }

    #[test]
    fn duplicate_selection_preserves_stat_equivalent_card_state_without_copying_bottle_attachment()
    {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        let mut bottled = RelicState::new(RelicId::BottledFlame);
        bottled.amount = 101;
        run_state.relics.push(bottled);

        let mut original = CombatCard::new(CardId::RitualDagger, 101);
        original.upgrades = 2;
        original.misc_value = 17;
        original.base_damage_override = Some(23);
        original.base_block_override = Some(14);
        original.cost_modifier = -1;
        original.cost_for_turn = Some(0);
        original.free_to_play_once = true;
        original.base_damage_mut = 99;
        original.base_block_mut = 88;
        original.base_magic_num_mut = 77;
        original.multi_damage = smallvec::smallvec![1, 2, 3];
        original.exhaust_override = Some(true);
        original.retain_override = Some(true);
        original.energy_on_use = 5;
        run_state.master_deck = vec![original];

        let next_state = run_state
            .obtain_relic_with_source(
                RelicId::DollysMirror,
                EngineState::MapNavigation,
                DomainEventSource::RewardScreen,
            )
            .expect("Dolly's Mirror should open a deck selection");

        let mut engine_state = next_state;
        let mut combat_state = None;
        assert!(tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(ClientInput::SubmitSelection(SelectionResolution {
                scope: SelectionScope::Deck,
                selected: vec![SelectionTargetRef::CardUuid(101)],
            })),
        ));

        assert!(matches!(engine_state, EngineState::MapNavigation));
        assert_eq!(run_state.master_deck.len(), 2);
        let copied = run_state
            .master_deck
            .iter()
            .find(|card| card.uuid != 101)
            .expect("Dolly's Mirror should add a copied card");
        assert_eq!(copied.id, CardId::RitualDagger);
        assert_eq!(copied.upgrades, 2);
        assert_eq!(copied.misc_value, 17);
        assert_eq!(copied.base_damage_override, Some(23));
        assert_eq!(copied.base_block_override, Some(14));
        assert_eq!(copied.cost_modifier, -1);
        assert_eq!(copied.cost_for_turn, Some(0));
        assert!(copied.free_to_play_once);
        assert_eq!(copied.base_damage_mut, 0);
        assert_eq!(copied.base_block_mut, 0);
        assert_eq!(copied.base_magic_num_mut, 0);
        assert!(copied.multi_damage.is_empty());
        assert_eq!(copied.exhaust_override, None);
        assert_eq!(copied.retain_override, None);
        assert_eq!(copied.energy_on_use, 0);
        assert_ne!(copied.uuid, 101);
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::BottledFlame)
                .map(|relic| relic.amount),
            Some(101),
            "Java clears bottle flags on the copied card; Rust bottle attachment stays on original UUID"
        );
    }

    #[test]
    fn combat_misc_meta_change_updates_matching_master_deck_card() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut dagger = CombatCard::new(CardId::RitualDagger, 101);
        dagger.misc_value = 17;
        run_state.master_deck = vec![dagger];

        apply_combat_meta_change(
            &mut run_state,
            crate::runtime::combat::MetaChange::ModifyCardMisc {
                card_uuid: 101,
                amount: 3,
            },
        );

        assert_eq!(
            run_state.master_deck[0].misc_value, 20,
            "Java RitualDaggerAction updates player.masterDeck before GetAllInBattleInstances"
        );
    }

    #[test]
    fn combat_upgrade_meta_change_updates_matching_master_deck_card() {
        let mut run_state = RunState::new(1, 0, false, "Watcher");
        run_state.master_deck = vec![CombatCard::new(CardId::StrikeP, 201)];

        apply_combat_meta_change(
            &mut run_state,
            crate::runtime::combat::MetaChange::UpgradeMasterDeckCard { card_uuid: 201 },
        );

        assert_eq!(
            run_state.master_deck[0].upgrades, 1,
            "Java LessonLearnedAction upgrades a random canUpgrade() card from player.masterDeck"
        );
    }

    #[test]
    fn bottled_relic_uuid_counts_as_innate_during_combat_deck_initialization() {
        let mut state = crate::test_support::blank_test_combat();
        let mut bottle = RelicState::new(RelicId::BottledTornado);
        bottle.amount = 103;
        state.entities.player.add_relic(bottle);
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 102),
            CombatCard::new(CardId::Inflame, 103),
        ];

        state.apply_java_initialize_deck_order_after_shuffle();

        assert_eq!(
            state.zones.draw_pile.first().map(|card| card.uuid),
            Some(103),
            "the card selected by Bottled Tornado must be handled by the same start-hand path as innate cards"
        );
    }

    #[test]
    fn matryoshka_on_chest_open_adds_extra_relic_before_base_chest_relic() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Matryoshka));
        run_state.common_relic_pool = vec![RelicId::Anchor];
        run_state.uncommon_relic_pool = vec![RelicId::Anchor];
        run_state.rare_relic_pool = vec![RelicId::Mango];
        let relic_rng_before = run_state.rng_pool.relic_rng.counter;

        let rewards = open_treasure_chest(
            &mut run_state,
            TreasureChestState {
                size: TreasureChestSize::Small,
                base_relic_tier: RelicTier::Rare,
                gold_reward_base_amount: None,
            },
        );

        let relic_rewards = rewards
            .items
            .iter()
            .filter_map(|item| match item {
                RewardItem::Relic { relic_id } => Some(*relic_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            relic_rewards,
            vec![RelicId::Anchor, RelicId::Mango],
            "Java Matryoshka.onChestOpen inserts its extra relic before AbstractChest adds the base chest relic"
        );
        let matryoshka = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Matryoshka)
            .expect("Matryoshka should remain owned");
        assert_eq!(matryoshka.counter, 1);
        assert!(!matryoshka.used_up);
        assert_eq!(
            run_state.rng_pool.relic_rng.counter,
            relic_rng_before + 1,
            "Java Matryoshka consumes relicRng only for randomBoolean(0.75)"
        );
    }

    #[test]
    fn nloths_mask_on_chest_open_after_removes_first_relic_after_matryoshka_and_base_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Matryoshka));
        run_state.relics.push(RelicState::new(RelicId::NlothsMask));
        run_state.common_relic_pool = vec![RelicId::Anchor];
        run_state.uncommon_relic_pool = vec![RelicId::Anchor];
        run_state.rare_relic_pool = vec![RelicId::Mango];

        let rewards = open_treasure_chest(
            &mut run_state,
            TreasureChestState {
                size: TreasureChestSize::Small,
                base_relic_tier: RelicTier::Rare,
                gold_reward_base_amount: None,
            },
        );

        let relic_rewards = rewards
            .items
            .iter()
            .filter_map(|item| match item {
                RewardItem::Relic { relic_id } => Some(*relic_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            relic_rewards,
            vec![RelicId::Mango],
            "Java N'loth's Mask runs after AbstractChest adds the base relic, so it removes Matryoshka's earlier extra relic first"
        );
        let mask = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::NlothsMask)
            .expect("N'loth's Mask should remain owned");
        assert_eq!(mask.counter, -2);
        assert!(mask.used_up);
    }

    #[test]
    fn nloths_mask_chest_removal_also_removes_sapphire_key_linked_to_removed_relic() {
        let mut items = vec![
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        remove_one_relic_from_rewards_after_chest_open(&mut items);

        assert!(items.is_empty());
    }

    #[test]
    fn nloths_mask_removes_matryoshka_relic_before_base_chest_key_pair() {
        let mut items = vec![
            RewardItem::Relic {
                relic_id: RelicId::Omamori,
            },
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        remove_one_relic_from_rewards_after_chest_open(&mut items);

        assert_eq!(
            items,
            vec![
                RewardItem::Relic {
                    relic_id: RelicId::Mango,
                },
                RewardItem::SapphireKey,
            ],
            "Java Matryoshka adds its relic during onChestOpen before the base chest relic/key pair"
        );
    }
}
