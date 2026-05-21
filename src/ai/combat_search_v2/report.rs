use super::*;

#[allow(clippy::too_many_arguments)]
pub fn trajectory_from_state(
    engine: EngineState,
    combat: CombatState,
    initial_hp: i32,
    actions: Vec<CombatSearchV2ActionTrace>,
    potions_used: u32,
    potions_discarded: u32,
    cards_played: u32,
    estimated: bool,
) -> CombatSearchV2TrajectoryReport {
    let node = SearchNode {
        engine,
        combat,
        actions,
        initial_hp,
        potions_used,
        potions_discarded,
        cards_played,
    };
    trajectory_report(&node, estimated)
}

pub(super) fn trajectory_report(
    node: &SearchNode,
    estimated: bool,
) -> CombatSearchV2TrajectoryReport {
    CombatSearchV2TrajectoryReport {
        terminal: terminal_label(&node.engine, &node.combat),
        estimated,
        actions: node.actions.clone(),
        final_hp: node.combat.entities.player.current_hp,
        final_block: node.combat.entities.player.block,
        hp_loss: (node.initial_hp - node.combat.entities.player.current_hp).max(0),
        turns: node.combat.turn.turn_count,
        potions_used: node.potions_used,
        potions_discarded: node.potions_discarded,
        cards_played: node.cards_played,
        enemy_final_state: node
            .combat
            .entities
            .monsters
            .iter()
            .enumerate()
            .map(|(slot, monster)| summarize_enemy(&node.combat, slot, monster))
            .collect(),
        final_state: summarize_state(&node.engine, &node.combat),
    }
}

pub(super) fn summarize_state(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatSearchV2StateSummary {
    CombatSearchV2StateSummary {
        engine_state: format!("{engine:?}"),
        terminal: terminal_label(engine, combat),
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        turn_count: combat.turn.turn_count,
        living_enemy_count: living_enemy_count(combat),
        total_enemy_hp: total_living_enemy_hp(combat),
        visible_incoming_damage: visible_incoming_damage(combat),
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        limbo_count: combat.zones.limbo.len(),
        queued_cards_count: combat.zones.queued_cards.len(),
    }
}

fn summarize_enemy(
    combat: &CombatState,
    slot: usize,
    monster: &MonsterEntity,
) -> CombatSearchV2EnemySummary {
    CombatSearchV2EnemySummary {
        slot,
        entity_id: monster.id,
        enemy_id: EnemyId::from_id(monster.monster_type)
            .map(|id| format!("{id:?}"))
            .unwrap_or_else(|| format!("MonsterType{}", monster.monster_type)),
        hp: monster.current_hp,
        max_hp: monster.max_hp,
        block: monster.block,
        alive: monster.is_alive_for_action(),
        escaped: monster.is_escaped,
        dying: monster.is_dying,
        half_dead: monster.half_dead,
        planned_move_id: monster.planned_move_id(),
        visible_intent: format!("{:?}", combat.monster_protocol_visible_intent(monster.id)),
        visible_incoming_damage: if monster.is_alive_for_action() {
            monster_preview_total_damage_in_combat(combat, monster)
        } else {
            0
        },
    }
}

pub(super) fn action_key(combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                format!(
                    "combat/play_card/hand:{card_index}/card:{}#{}/target:{}",
                    card_label(card),
                    card.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| format!("{input:?}")),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => combat
            .entities
            .potions
            .get(*potion_index)
            .and_then(|potion| potion.as_ref())
            .map(|potion| {
                format!(
                    "combat/use_potion/slot:{potion_index}/potion:{:?}#{}/target:{}",
                    potion.id,
                    potion.uuid,
                    target_label(combat, *target)
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "combat/use_potion/slot:{potion_index}/target:{}",
                    target_label(combat, *target)
                )
            }),
        ClientInput::DiscardPotion(slot) => format!("combat/discard_potion/slot:{slot}"),
        ClientInput::EndTurn => "combat/end_turn".to_string(),
        ClientInput::SubmitDiscoverChoice(index) => format!("combat/submit_choice/{index}"),
        ClientInput::SubmitHandSelect(uuids) => format!("combat/hand_select/{}", uuid_list(uuids)),
        ClientInput::SubmitGridSelect(uuids) => format!("combat/grid_select/{}", uuid_list(uuids)),
        ClientInput::SubmitScryDiscard(indices) => {
            format!(
                "combat/scry_discard/{}",
                indices
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        ClientInput::Cancel => "combat/cancel".to_string(),
        ClientInput::Proceed => "combat/proceed".to_string(),
        _ => format!("{input:?}"),
    }
}

fn card_label(card: &CombatCard) -> String {
    format!("{}+{}", cards::java_id(card.id), card.upgrades)
}

pub(super) fn target_label(combat: &CombatState, target: Option<usize>) -> String {
    match target {
        None => "none".to_string(),
        Some(entity_id) => combat
            .entities
            .monsters
            .iter()
            .position(|monster| monster.id == entity_id)
            .map(|slot| format!("monster_slot:{slot}"))
            .unwrap_or_else(|| format!("entity:{entity_id}")),
    }
}

fn uuid_list(uuids: &[u32]) -> String {
    uuids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
