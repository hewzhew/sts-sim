use sts_combat_planner::SelectionTransactionCursor;

use crate::eval::fingerprint::combat_state_fingerprint_v2;
use crate::sim::combat::{CombatPosition, CombatStepper, EngineCombatStepper};
use crate::sim::combat_action::combat_action_key;
use crate::state::core::ClientInput;

use super::{
    OracleAnalysisCombatScratchActionSelectorV1, OracleAnalysisCombatScratchActionSurfaceV1,
    OracleAnalysisCombatScratchActionV1, OracleAnalysisCombatScratchCardV1,
    OracleAnalysisCombatScratchMonsterV1, OracleAnalysisCombatScratchPlayerV1,
    OracleAnalysisCombatScratchPositionV1, OracleAnalysisCombatScratchSelectionFamilyV1,
};

pub(super) fn action_surface_view(
    position: &CombatPosition,
    scratch_node_id: u64,
    selection_offset: usize,
    selection_limit: usize,
) -> Result<OracleAnalysisCombatScratchActionSurfaceV1, String> {
    let exact_state_hash = exact_hash(position);
    let surface = EngineCombatStepper.legal_action_surface(position);
    let atomic_actions = surface
        .atomic_actions
        .iter()
        .cloned()
        .enumerate()
        .map(|(index, input)| {
            action_view(
                position,
                OracleAnalysisCombatScratchActionSelectorV1::Atomic {
                    scratch_node_id,
                    action_index: index,
                },
                atomic_action_ref(&exact_state_hash, index),
                input,
            )
        })
        .collect();
    let mut selection_families = Vec::with_capacity(surface.selection_families.len());
    for (family_index, family) in surface.selection_families.into_iter().enumerate() {
        let mut cursor = SelectionTransactionCursor::new(&family).map_err(|gap| {
            format!("combat scratch cannot enumerate selection family {family_index}: {gap:?}")
        })?;
        let total_input_count = cursor.remaining_input_count();
        for _ in 0..selection_offset.min(total_input_count) {
            let _ = cursor.next_input();
        }
        let mut actions = Vec::new();
        for input_index in selection_offset..selection_offset.saturating_add(selection_limit) {
            let Some(input) = cursor.next_input() else {
                break;
            };
            actions.push(action_view(
                position,
                OracleAnalysisCombatScratchActionSelectorV1::Selection {
                    scratch_node_id,
                    family_index,
                    input_index,
                },
                selection_action_ref(&exact_state_hash, family_index, input_index),
                input,
            ));
        }
        let next_page_offset =
            (!cursor.is_exhausted()).then_some(selection_offset.saturating_add(actions.len()));
        selection_families.push(OracleAnalysisCombatScratchSelectionFamilyV1 {
            family_index,
            family,
            total_input_count,
            page_offset: selection_offset,
            page_limit: selection_limit,
            next_page_offset,
            actions,
        });
    }
    Ok(OracleAnalysisCombatScratchActionSurfaceV1 {
        exact_state_hash,
        atomic_actions,
        selection_families,
    })
}

fn action_view(
    position: &CombatPosition,
    selector: OracleAnalysisCombatScratchActionSelectorV1,
    action_ref: String,
    input: ClientInput,
) -> OracleAnalysisCombatScratchActionV1 {
    OracleAnalysisCombatScratchActionV1 {
        selector,
        action_ref,
        action_key: combat_action_key(&position.combat, &input),
        input,
    }
}

pub(super) fn resolve_action_selector(
    position: &CombatPosition,
    selector: OracleAnalysisCombatScratchActionSelectorV1,
) -> Result<ClientInput, String> {
    let surface = EngineCombatStepper.legal_action_surface(position);
    let input = match selector {
        OracleAnalysisCombatScratchActionSelectorV1::Atomic { action_index, .. } => surface
            .atomic_actions
            .get(action_index)
            .cloned()
            .ok_or_else(|| format!("combat scratch has no atomic action {action_index}"))?,
        OracleAnalysisCombatScratchActionSelectorV1::Selection {
            family_index,
            input_index,
            ..
        } => {
            let family = surface
                .selection_families
                .get(family_index)
                .ok_or_else(|| format!("combat scratch has no selection family {family_index}"))?;
            let mut cursor = SelectionTransactionCursor::new(family).map_err(|gap| {
                format!("combat scratch cannot enumerate selection family {family_index}: {gap:?}")
            })?;
            let mut selected = None;
            for current in 0..=input_index {
                let Some(input) = cursor.next_input() else {
                    break;
                };
                if current == input_index {
                    selected = Some(input);
                    break;
                }
            }
            selected.ok_or_else(|| {
                format!("combat scratch selection family {family_index} has no input {input_index}")
            })?
        }
        OracleAnalysisCombatScratchActionSelectorV1::Card {
            card_uuid, target, ..
        } => {
            let card_index = position
                .combat
                .zones
                .hand
                .iter()
                .position(|card| card.uuid == card_uuid)
                .ok_or_else(|| format!("combat scratch hand has no card uuid {card_uuid}"))?;
            ClientInput::PlayCard { card_index, target }
        }
        OracleAnalysisCombatScratchActionSelectorV1::HandCard {
            hand_index,
            target_index,
            ..
        } => {
            if position.combat.zones.hand.get(hand_index).is_none() {
                return Err(format!(
                    "combat scratch hand has no local card index {hand_index}"
                ));
            }
            let target = local_monster_entity_id(position, target_index)?;
            ClientInput::PlayCard {
                card_index: hand_index,
                target,
            }
        }
        OracleAnalysisCombatScratchActionSelectorV1::Potion {
            potion_uuid,
            target,
            ..
        } => {
            let potion_index = position
                .combat
                .entities
                .potions
                .iter()
                .position(|potion| {
                    potion
                        .as_ref()
                        .is_some_and(|potion| potion.uuid == potion_uuid)
                })
                .ok_or_else(|| {
                    format!("combat scratch inventory has no potion uuid {potion_uuid}")
                })?;
            ClientInput::UsePotion {
                potion_index,
                target,
            }
        }
        OracleAnalysisCombatScratchActionSelectorV1::PotionSlot {
            potion_slot,
            target_index,
            ..
        } => {
            if !position
                .combat
                .entities
                .potions
                .get(potion_slot)
                .is_some_and(Option::is_some)
            {
                return Err(format!(
                    "combat scratch inventory has no potion in local slot {potion_slot}"
                ));
            }
            let target = local_monster_entity_id(position, target_index)?;
            ClientInput::UsePotion {
                potion_index: potion_slot,
                target,
            }
        }
        OracleAnalysisCombatScratchActionSelectorV1::EndTurn { .. } => ClientInput::EndTurn,
    };
    if !EngineCombatStepper.is_legal_action(position, &input) {
        return Err("combat scratch selector no longer resolves to a legal input".to_string());
    }
    Ok(input)
}

fn local_monster_entity_id(
    position: &CombatPosition,
    target_index: Option<usize>,
) -> Result<Option<usize>, String> {
    target_index
        .map(|monster_index| {
            position
                .combat
                .entities
                .monsters
                .get(monster_index)
                .map(|monster| monster.id)
                .ok_or_else(|| format!("combat scratch has no local monster index {monster_index}"))
        })
        .transpose()
}

pub(super) fn resolve_action_ref(
    position: &CombatPosition,
    action_ref: &str,
) -> Result<ClientInput, String> {
    let parts = action_ref.split('/').collect::<Vec<_>>();
    let exact_state_hash = exact_hash(position);
    if parts.first().copied() != Some("combat_scratch_v1")
        || parts.get(1).copied() != Some(exact_state_hash.as_str())
    {
        return Err(
            "combat scratch action ref is stale or belongs to another exact state".to_string(),
        );
    }
    let surface = EngineCombatStepper.legal_action_surface(position);
    let input = match parts.as_slice() {
        [_, _, "atomic", index] => {
            let index = parse_ref_index(index, "atomic action")?;
            surface
                .atomic_actions
                .get(index)
                .cloned()
                .ok_or_else(|| format!("combat scratch has no atomic action {index}"))?
        }
        [_, _, "selection", family_index, input_index] => {
            let family_index = parse_ref_index(family_index, "selection family")?;
            let input_index = parse_ref_index(input_index, "selection input")?;
            let family = surface
                .selection_families
                .get(family_index)
                .ok_or_else(|| format!("combat scratch has no selection family {family_index}"))?;
            let mut cursor = SelectionTransactionCursor::new(family).map_err(|gap| {
                format!("combat scratch cannot enumerate selection family {family_index}: {gap:?}")
            })?;
            let mut selected = None;
            for current in 0..=input_index {
                let Some(input) = cursor.next_input() else {
                    break;
                };
                if current == input_index {
                    selected = Some(input);
                    break;
                }
            }
            selected.ok_or_else(|| {
                format!("combat scratch selection family {family_index} has no input {input_index}")
            })?
        }
        _ => return Err("invalid combat scratch action ref".to_string()),
    };
    if !EngineCombatStepper.is_legal_action(position, &input) {
        return Err("combat scratch action ref no longer resolves to a legal input".to_string());
    }
    Ok(input)
}

fn parse_ref_index(value: &str, kind: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid combat scratch {kind} index '{value}'"))
}

fn atomic_action_ref(exact_state_hash: &str, index: usize) -> String {
    format!("combat_scratch_v1/{exact_state_hash}/atomic/{index}")
}

fn selection_action_ref(exact_state_hash: &str, family_index: usize, input_index: usize) -> String {
    format!("combat_scratch_v1/{exact_state_hash}/selection/{family_index}/{input_index}")
}

pub(super) fn position_view(position: &CombatPosition) -> OracleAnalysisCombatScratchPositionV1 {
    let combat = &position.combat;
    let player = &combat.entities.player;
    OracleAnalysisCombatScratchPositionV1 {
        fingerprint: combat_state_fingerprint_v2(position),
        terminal: EngineCombatStepper.terminal(position),
        turn: combat.turn.turn_count,
        phase: combat.turn.current_phase,
        counters: combat.turn.counters.clone(),
        player: OracleAnalysisCombatScratchPlayerV1 {
            current_hp: player.current_hp,
            max_hp: player.max_hp,
            block: player.block,
            energy: combat.turn.energy,
            stance: player.stance,
            orbs: player.orbs.clone(),
            relics: player.relics.clone(),
            powers: crate::content::powers::store::powers_snapshot_for(combat, player.id),
        },
        hand: combat.zones.hand.iter().cloned().map(card_view).collect(),
        draw_pile_top_first: combat
            .zones
            .draw_pile
            .iter()
            .cloned()
            .map(card_view)
            .collect(),
        discard_pile: combat
            .zones
            .discard_pile
            .iter()
            .cloned()
            .map(card_view)
            .collect(),
        exhaust_pile: combat
            .zones
            .exhaust_pile
            .iter()
            .cloned()
            .map(card_view)
            .collect(),
        limbo: combat.zones.limbo.iter().cloned().map(card_view).collect(),
        potions: combat.entities.potions.clone(),
        monsters: combat
            .entities
            .monsters
            .iter()
            .map(|monster| OracleAnalysisCombatScratchMonsterV1 {
                entity_id: monster.id,
                slot: monster.slot,
                label: crate::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                current_hp: monster.current_hp,
                max_hp: monster.max_hp,
                block: monster.block,
                is_dying: monster.is_dying,
                is_escaped: monster.is_escaped,
                half_dead: monster.half_dead,
                planned_move_id: monster.planned_move_id(),
                planned_steps: monster.move_state.planned_steps.clone().unwrap_or_default(),
                intent: super::super::exact_monster_intent(combat, monster),
                thief: monster.thief.clone(),
                powers: crate::content::powers::store::powers_snapshot_for(combat, monster.id),
            })
            .collect(),
    }
}

fn card_view(card: crate::runtime::combat::CombatCard) -> OracleAnalysisCombatScratchCardV1 {
    OracleAnalysisCombatScratchCardV1 {
        effective_cost: card.cost_for_turn_java(),
        card,
    }
}

pub(super) fn exact_hash(position: &CombatPosition) -> String {
    crate::ai::combat_state_key::combat_exact_state_hash_v2(&position.engine, &position.combat)
}
