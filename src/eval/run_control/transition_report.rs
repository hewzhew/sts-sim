use std::collections::HashMap;

use crate::content::cards::{get_card_definition, CardId};
use crate::content::potions::{get_potion_definition, PotionId};
use crate::content::relics::RelicId;
use crate::runtime::combat::CombatCard;
use crate::state::core::ClientInput;

use super::session::RunControlSession;
use super::view_model::{build_run_control_view_model, client_input_hint};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RunApplyStatus {
    Running,
    Victory,
    Defeat,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct TransitionAction {
    label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RunVisibleSnapshot {
    title: String,
    current_hp: i32,
    max_hp: i32,
    gold: i32,
    act: u8,
    floor: i32,
    keys: [bool; 3],
    relics: Vec<RelicSnapshot>,
    potions: Vec<Option<PotionSnapshot>>,
    deck: Vec<CardSnapshot>,
    combat: Option<CombatSnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RelicSnapshot {
    id: RelicId,
    counter: i32,
    used_up: bool,
    amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PotionSnapshot {
    id: PotionId,
    uuid: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CardSnapshot {
    id: CardId,
    uuid: u32,
    upgrades: u8,
}

#[derive(Clone, Debug, PartialEq)]
struct CombatSnapshot {
    player_hp: i32,
    player_max_hp: i32,
    player_block: i32,
    energy: i32,
    monsters: Vec<MonsterSnapshot>,
    hand_count: usize,
    draw_count: usize,
    discard_count: usize,
    exhaust_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct MonsterSnapshot {
    id: usize,
    label: String,
    hp: i32,
    max_hp: i32,
    block: i32,
    alive: bool,
}

pub(super) fn transition_action_for_input(
    session: &RunControlSession,
    input: &ClientInput,
) -> TransitionAction {
    let view = build_run_control_view_model(session);
    let label = view
        .candidates
        .iter()
        .find(|candidate| candidate.action.executable_input().as_ref() == Some(input))
        .map(|candidate| candidate.label.clone())
        .unwrap_or_else(|| client_input_hint(input));
    TransitionAction { label }
}

pub(super) fn render_transition_report(
    action: TransitionAction,
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    status: RunApplyStatus,
) -> String {
    let mut lines = Vec::new();
    lines.push("Result:".to_string());
    lines.push(format!("  Chose: {}", action.label));
    push_resource_delta(before, after, &mut lines);
    push_relic_delta(before, after, &mut lines);
    push_potion_delta(before, after, &mut lines);
    push_deck_delta(before, after, &mut lines);
    push_key_delta(before, after, &mut lines);
    push_combat_delta(before, after, &mut lines);
    if before.act != after.act || before.floor != after.floor {
        lines.push(format!(
            "  Location: Act {} Floor {} -> Act {} Floor {}",
            before.act, before.floor, after.act, after.floor
        ));
    }
    if before.title != after.title {
        lines.push(format!("  Advanced to: {}", after.title));
    }
    match status {
        RunApplyStatus::Running => {}
        RunApplyStatus::Victory => lines.push("  Run ended: victory".to_string()),
        RunApplyStatus::Defeat => lines.push("  Run ended: defeat".to_string()),
        RunApplyStatus::Stopped => lines.push("  Engine stopped".to_string()),
    }
    if lines.len() == 2 {
        lines.push("  No visible state changes.".to_string());
    }
    lines.join("\n")
}

impl RunVisibleSnapshot {
    pub(super) fn capture(session: &RunControlSession) -> Self {
        let view = build_run_control_view_model(session);
        let (current_hp, max_hp) = session
            .active_combat
            .as_ref()
            .map(|active| {
                (
                    active.combat_state.entities.player.current_hp,
                    active.combat_state.entities.player.max_hp,
                )
            })
            .unwrap_or((session.run_state.current_hp, session.run_state.max_hp));
        Self {
            title: view.header.title,
            current_hp,
            max_hp,
            gold: session.run_state.gold,
            act: session.run_state.act_num,
            floor: session.run_state.floor_num,
            keys: session.run_state.keys,
            relics: session
                .run_state
                .relics
                .iter()
                .map(|relic| RelicSnapshot {
                    id: relic.id,
                    counter: relic.counter,
                    used_up: relic.used_up,
                    amount: relic.amount,
                })
                .collect(),
            potions: session
                .run_state
                .potions
                .iter()
                .map(|slot| {
                    slot.as_ref().map(|potion| PotionSnapshot {
                        id: potion.id,
                        uuid: potion.uuid,
                    })
                })
                .collect(),
            deck: session
                .run_state
                .master_deck
                .iter()
                .map(card_snapshot)
                .collect(),
            combat: session.active_combat.as_ref().map(|active| {
                let combat = &active.combat_state;
                CombatSnapshot {
                    player_hp: combat.entities.player.current_hp,
                    player_max_hp: combat.entities.player.max_hp,
                    player_block: combat.entities.player.block,
                    energy: i32::from(combat.turn.energy),
                    monsters: combat
                        .entities
                        .monsters
                        .iter()
                        .map(|monster| MonsterSnapshot {
                            id: monster.id,
                            label: super::view_model::monster_name(monster.monster_type),
                            hp: monster.current_hp,
                            max_hp: monster.max_hp,
                            block: monster.block,
                            alive: monster.is_alive_for_action(),
                        })
                        .collect(),
                    hand_count: combat.zones.hand.len(),
                    draw_count: combat.zones.draw_pile.len(),
                    discard_count: combat.zones.discard_pile.len(),
                    exhaust_count: combat.zones.exhaust_pile.len(),
                }
            }),
        }
    }
}

fn card_snapshot(card: &CombatCard) -> CardSnapshot {
    CardSnapshot {
        id: card.id,
        uuid: card.uuid,
        upgrades: card.upgrades,
    }
}

fn push_resource_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    if before.current_hp != after.current_hp || before.max_hp != after.max_hp {
        lines.push(format!(
            "  HP: {}/{} -> {}/{}",
            before.current_hp, before.max_hp, after.current_hp, after.max_hp
        ));
    }
    if before.gold != after.gold {
        lines.push(format!("  Gold: {} -> {}", before.gold, after.gold));
    }
}

fn push_relic_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    let before_counts = relic_counts(&before.relics);
    let after_counts = relic_counts(&after.relics);
    let mut ids = before_counts
        .keys()
        .chain(after_counts.keys())
        .copied()
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| relic_label(*id));
    ids.dedup();
    for id in ids {
        let old = before_counts.get(&id).copied().unwrap_or(0);
        let new = after_counts.get(&id).copied().unwrap_or(0);
        if new > old {
            for _ in 0..(new - old) {
                lines.push(format!("  Gained relic: {}", relic_label(id)));
            }
        } else if old > new {
            for _ in 0..(old - new) {
                lines.push(format!("  Lost relic: {}", relic_label(id)));
            }
        }
    }

    for after_relic in &after.relics {
        if let Some(before_relic) = before
            .relics
            .iter()
            .find(|before_relic| before_relic.id == after_relic.id)
        {
            let mut changes = Vec::new();
            if before_relic.counter != after_relic.counter {
                changes.push(format!(
                    "counter {} -> {}",
                    before_relic.counter, after_relic.counter
                ));
            }
            if before_relic.amount != after_relic.amount {
                changes.push(format!(
                    "amount {} -> {}",
                    before_relic.amount, after_relic.amount
                ));
            }
            if before_relic.used_up != after_relic.used_up {
                changes.push(format!(
                    "used {} -> {}",
                    before_relic.used_up, after_relic.used_up
                ));
            }
            if !changes.is_empty() {
                lines.push(format!(
                    "  Relic changed: {} ({})",
                    relic_label(after_relic.id),
                    changes.join(", ")
                ));
            }
        }
    }
}

fn push_potion_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    let max_len = before.potions.len().max(after.potions.len());
    for slot in 0..max_len {
        let old = before.potions.get(slot).and_then(|value| value.as_ref());
        let new = after.potions.get(slot).and_then(|value| value.as_ref());
        match (old, new) {
            (None, Some(potion)) => lines.push(format!(
                "  Gained potion: {} in slot {}",
                potion_label(potion.id),
                slot
            )),
            (Some(potion), None) => lines.push(format!(
                "  Lost potion: {} from slot {}",
                potion_label(potion.id),
                slot
            )),
            (Some(old), Some(new)) if old.id != new.id || old.uuid != new.uuid => {
                lines.push(format!(
                    "  Potion slot {}: {} -> {}",
                    slot,
                    potion_label(old.id),
                    potion_label(new.id)
                ));
            }
            _ => {}
        }
    }
}

fn push_deck_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    let before_by_uuid = before
        .deck
        .iter()
        .map(|card| (card.uuid, card))
        .collect::<HashMap<_, _>>();
    let after_by_uuid = after
        .deck
        .iter()
        .map(|card| (card.uuid, card))
        .collect::<HashMap<_, _>>();

    for card in &before.deck {
        if !after_by_uuid.contains_key(&card.uuid) {
            lines.push(format!("  Removed card: {}", card_label(card)));
        }
    }
    for card in &after.deck {
        match before_by_uuid.get(&card.uuid) {
            None => lines.push(format!("  Added card: {}", card_label(card))),
            Some(before_card) if before_card.id != card.id => {
                lines.push(format!(
                    "  Transformed card: {} -> {}",
                    card_label(before_card),
                    card_label(card)
                ));
            }
            Some(before_card) if before_card.upgrades != card.upgrades => {
                lines.push(format!(
                    "  Upgraded card: {} -> {}",
                    card_label(before_card),
                    card_label(card)
                ));
            }
            _ => {}
        }
    }
}

fn push_key_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    for (idx, (old, new)) in before.keys.iter().zip(after.keys.iter()).enumerate() {
        if old != new {
            lines.push(format!(
                "  {} key: {}",
                key_label(idx),
                if *new { "obtained" } else { "lost" }
            ));
        }
    }
}

fn push_combat_delta(
    before: &RunVisibleSnapshot,
    after: &RunVisibleSnapshot,
    lines: &mut Vec<String>,
) {
    match (&before.combat, &after.combat) {
        (None, Some(after_combat)) => {
            lines.push("  Combat started.".to_string());
            lines.push(format!(
                "  Player: HP {}/{} Block {} Energy {}",
                after_combat.player_hp,
                after_combat.player_max_hp,
                after_combat.player_block,
                after_combat.energy
            ));
            for monster in &after_combat.monsters {
                lines.push(format!(
                    "  Enemy: {} HP {}/{} Block {}",
                    monster.label, monster.hp, monster.max_hp, monster.block
                ));
            }
        }
        (Some(_), None) => lines.push("  Combat ended.".to_string()),
        (Some(before_combat), Some(after_combat)) => {
            if before_combat.player_hp != after_combat.player_hp
                || before_combat.player_block != after_combat.player_block
                || before_combat.energy != after_combat.energy
            {
                lines.push(format!(
                    "  Player: HP {}/{} -> {}/{} | Block {} -> {} | Energy {} -> {}",
                    before_combat.player_hp,
                    before_combat.player_max_hp,
                    after_combat.player_hp,
                    after_combat.player_max_hp,
                    before_combat.player_block,
                    after_combat.player_block,
                    before_combat.energy,
                    after_combat.energy
                ));
            }
            for after_monster in &after_combat.monsters {
                if let Some(before_monster) = before_combat
                    .monsters
                    .iter()
                    .find(|monster| monster.id == after_monster.id)
                {
                    if before_monster.hp != after_monster.hp
                        || before_monster.block != after_monster.block
                        || before_monster.alive != after_monster.alive
                    {
                        lines.push(format!(
                            "  Enemy {}: HP {}/{} -> {}/{} | Block {} -> {} | Alive {} -> {}",
                            after_monster.label,
                            before_monster.hp,
                            before_monster.max_hp,
                            after_monster.hp,
                            after_monster.max_hp,
                            before_monster.block,
                            after_monster.block,
                            before_monster.alive,
                            after_monster.alive
                        ));
                    }
                }
            }
            if before_combat.hand_count != after_combat.hand_count
                || before_combat.draw_count != after_combat.draw_count
                || before_combat.discard_count != after_combat.discard_count
                || before_combat.exhaust_count != after_combat.exhaust_count
            {
                lines.push(format!(
                    "  Piles: hand {} -> {}, draw {} -> {}, discard {} -> {}, exhaust {} -> {}",
                    before_combat.hand_count,
                    after_combat.hand_count,
                    before_combat.draw_count,
                    after_combat.draw_count,
                    before_combat.discard_count,
                    after_combat.discard_count,
                    before_combat.exhaust_count,
                    after_combat.exhaust_count
                ));
            }
        }
        (None, None) => {}
    }
}

fn relic_counts(relics: &[RelicSnapshot]) -> HashMap<RelicId, usize> {
    let mut counts = HashMap::new();
    for relic in relics {
        *counts.entry(relic.id).or_insert(0) += 1;
    }
    counts
}

fn card_label(card: &CardSnapshot) -> String {
    let name = get_card_definition(card.id).name;
    if card.upgrades == 0 {
        name.to_string()
    } else {
        format!("{name}+{}", card.upgrades)
    }
}

fn potion_label(id: PotionId) -> &'static str {
    get_potion_definition(id).name
}

fn relic_label(id: RelicId) -> String {
    debug_words(&format!("{id:?}"))
}

fn key_label(idx: usize) -> &'static str {
    match idx {
        0 => "Ruby",
        1 => "Sapphire",
        2 => "Emerald",
        _ => "Unknown",
    }
}

fn debug_words(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::RelicState;
    use crate::eval::run_control::session::{RunControlConfig, RunControlSession};
    use crate::eval::run_control::RunControlCommand;

    #[test]
    fn transition_report_renders_relic_swap_from_state_diff() {
        let before = RunVisibleSnapshot {
            title: "Neow Bonus".to_string(),
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            act: 1,
            floor: 0,
            keys: [false; 3],
            relics: vec![RelicSnapshot {
                id: RelicId::BurningBlood,
                counter: -1,
                used_up: false,
                amount: 0,
            }],
            potions: vec![None, None, None],
            deck: Vec::new(),
            combat: None,
        };
        let after = RunVisibleSnapshot {
            relics: vec![RelicSnapshot {
                id: RelicId::Astrolabe,
                counter: -1,
                used_up: false,
                amount: 0,
            }],
            ..before.clone()
        };
        let rendered = render_transition_report(
            TransitionAction {
                label: "Boss relic swap".to_string(),
            },
            &before,
            &after,
            RunApplyStatus::Running,
        );

        assert!(rendered.contains("Lost relic: Burning Blood"));
        assert!(rendered.contains("Gained relic: Astrolabe"));
        assert!(!rendered.contains("ok"));
    }

    #[test]
    fn transition_report_captures_neow_gold_delta() {
        let mut session = RunControlSession::new(RunControlConfig {
            seed: 521,
            ..RunControlConfig::default()
        });
        session
            .apply_command(RunControlCommand::DefaultCandidate)
            .expect("Neow intro should advance");
        let before = RunVisibleSnapshot::capture(&session);
        let action = transition_action_for_input(&session, &ClientInput::EventChoice(1));
        session
            .apply_command(RunControlCommand::Candidate("1".to_string()))
            .expect("gold option should execute");
        let after = RunVisibleSnapshot::capture(&session);
        let rendered = render_transition_report(action, &before, &after, RunApplyStatus::Running);

        assert!(rendered.contains("Chose: Obtain 100 Gold."));
        assert!(rendered.contains("Gold: 99 -> 199"));
    }

    #[test]
    fn snapshot_records_relics_without_consuming_run_events() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::Anchor));
        session.run_state.emitted_events.push(
            crate::state::selection::DomainEvent::RelicObtained {
                relic_id: RelicId::Anchor,
                source: crate::state::selection::DomainEventSource::RewardScreen,
            },
        );

        let _snapshot = RunVisibleSnapshot::capture(&session);

        assert_eq!(session.run_state.emitted_events.len(), 1);
    }
}
