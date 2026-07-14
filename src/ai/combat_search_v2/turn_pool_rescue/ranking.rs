use std::cmp::Reverse;

use crate::content::monsters::EnemyId;
use crate::sim::combat::CombatPosition;

use super::super::{trajectory_report::summarize_state, SearchTerminalLabel};
use super::types::{
    CombatTurnPoolOpeningLineReport, CombatTurnPoolRescueLineSummary, TurnPoolLane, TurnPoolNode,
};

pub(super) fn keep_lane_nodes(nodes: &mut Vec<TurnPoolNode>, lane: TurnPoolLane, limit: usize) {
    nodes.sort_by_key(|node| Reverse(lane_rank(node, lane)));
    nodes.truncate(limit);
}

pub(super) fn lane_rank(node: &TurnPoolNode, lane: TurnPoolLane) -> (i32, i32, i32, i32, i32, i32) {
    let terminal = terminal_rank_for_line(node.terminal);
    let hp = node.position.combat.entities.player.current_hp;
    let enemy_hp = total_enemy_hp(&node.position);
    match lane {
        TurnPoolLane::Damage => (
            terminal,
            -enemy_hp,
            hp,
            -(node.actions.len() as i32),
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::Survival => (
            terminal,
            hp,
            -visible_pressure(&node.position),
            -enemy_hp,
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::Setup => (
            terminal,
            node.powers_played as i32,
            hp,
            -enemy_hp,
            -(node.actions.len() as i32),
            0,
        ),
        TurnPoolLane::PowerDelay => (
            terminal,
            -(node.powers_played as i32),
            -enemy_hp,
            hp,
            -(node.potions_used as i32),
            0,
        ),
        TurnPoolLane::PotionBurst => (
            terminal,
            node.potions_used as i32,
            -enemy_hp,
            hp,
            -(node.actions.len() as i32),
            0,
        ),
        TurnPoolLane::CultistCleanup => {
            let (cultists_alive, total_cultist_hp) = cultist_pressure(&node.position);
            (
                terminal,
                -(cultists_alive as i32),
                -total_cultist_hp,
                hp,
                -enemy_hp,
                -(node.actions.len() as i32),
            )
        }
    }
}

pub(super) fn turn_pool_opening_line_report(
    lane: TurnPoolLane,
    node: &TurnPoolNode,
) -> CombatTurnPoolOpeningLineReport {
    let (cultists_alive, total_cultist_hp) = cultist_pressure(&node.position);
    CombatTurnPoolOpeningLineReport {
        lane: lane.label(),
        terminal: node.terminal,
        final_hp: node.position.combat.entities.player.current_hp,
        turns: node.position.combat.turn.turn_count,
        actions: node.actions.clone(),
        potions_used: node.potions_used,
        powers_played: node.powers_played,
        cultists_alive,
        total_cultist_hp,
        end_state: summarize_state(&node.position.engine, &node.position.combat),
    }
}

pub(super) fn opening_cleanup_rank(
    line: &CombatTurnPoolOpeningLineReport,
) -> (i32, i32, i32, i32, i32) {
    (
        terminal_rank_for_line(line.terminal),
        -(line.cultists_alive as i32),
        -line.total_cultist_hp,
        line.final_hp,
        -(line.actions.len() as i32),
    )
}

pub(super) fn turn_pool_summary(
    lane: TurnPoolLane,
    node: &TurnPoolNode,
) -> CombatTurnPoolRescueLineSummary {
    CombatTurnPoolRescueLineSummary {
        lane: lane.label(),
        terminal: node.terminal,
        final_hp: node.position.combat.entities.player.current_hp,
        total_enemy_hp: total_enemy_hp(&node.position),
        living_enemy_count: living_enemy_count(&node.position),
        turns: node.position.combat.turn.turn_count,
        actions: node.actions.len(),
        potions_used: node.potions_used,
        powers_played: node.powers_played,
    }
}

pub(super) fn turn_pool_summary_rank(
    line: &CombatTurnPoolRescueLineSummary,
) -> (i32, i32, i32, i32) {
    let loss = line.terminal == SearchTerminalLabel::Loss;
    (
        turn_pool_summary_tier(line),
        if loss {
            -line.total_enemy_hp
        } else {
            line.turns as i32
        },
        if loss {
            line.turns as i32
        } else {
            -line.total_enemy_hp
        },
        line.final_hp,
    )
}

fn turn_pool_summary_tier(line: &CombatTurnPoolRescueLineSummary) -> i32 {
    match line.terminal {
        SearchTerminalLabel::Win => 4,
        SearchTerminalLabel::Loss if line.living_enemy_count == 1 && line.total_enemy_hp <= 50 => 3,
        SearchTerminalLabel::Unresolved => 2,
        SearchTerminalLabel::Loss => 1,
    }
}

fn terminal_rank_for_line(terminal: SearchTerminalLabel) -> i32 {
    match terminal {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

fn living_enemy_count(position: &CombatPosition) -> usize {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}

fn total_enemy_hp(position: &CombatPosition) -> i32 {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}

fn cultist_pressure(position: &CombatPosition) -> (usize, i32) {
    let cultists = position.combat.entities.monsters.iter().filter(|monster| {
        monster.is_alive_for_action()
            && EnemyId::from_id(monster.monster_type) == Some(EnemyId::Cultist)
    });
    cultists.fold((0usize, 0i32), |(count, hp), monster| {
        (
            count.saturating_add(1),
            hp.saturating_add(monster.current_hp.max(0)),
        )
    })
}

fn visible_pressure(position: &CombatPosition) -> i32 {
    position
        .combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| {
            crate::sim::combat_projection::monster_preview_total_damage_in_combat(
                &position.combat,
                monster,
            )
        })
        .sum::<i32>()
        .saturating_sub(position.combat.entities.player.block)
}
