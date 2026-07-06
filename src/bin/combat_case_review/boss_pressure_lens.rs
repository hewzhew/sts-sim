use sts_simulator::ai::combat_search_v2::CombatLineLabReport;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::{CombatState, MonsterEntity};
use sts_simulator::sim::combat_projection::monster_preview_total_damage_in_combat;

use super::search_types::SearchReview;

#[path = "boss_pressure_lens/line_reviews.rs"]
mod line_reviews;
#[path = "boss_pressure_lens/types.rs"]
mod types;

pub(super) use types::BossPressureLensReport;

use line_reviews::{aggregate_line_tags, collect_line_reviews};
use types::{BossPotionPermission, BossPressureObjective, CollectorStartSignals};

pub(super) fn boss_pressure_lens(
    case: &CombatCase,
    ladder: &[SearchReview],
    line_lab: Option<&CombatLineLabReport>,
) -> Option<BossPressureLensReport> {
    let combat = &case.position.combat;
    let collector = find_enemy(combat, EnemyId::TheCollector)?;
    let start = collector_start_signals(combat, collector);
    let phase = collector_phase(start.turn);
    let objectives = collector_objectives(&start, phase);
    let potion_permission = collector_potion_permission(&start, phase);
    let line_reviews = collect_line_reviews(ladder, line_lab);
    let mut tags = collector_tags(&start, phase);
    if potion_permission.level == "allow" {
        tags.push("collector_potion_window_open");
    }
    tags.extend(aggregate_line_tags(&line_reviews));
    tags.sort_unstable();
    tags.dedup();

    Some(BossPressureLensReport {
        schema: "boss_pressure_lens_v0",
        boss: "collector",
        phase,
        start,
        tags,
        objectives,
        potion_permission,
        line_reviews,
    })
}

fn collector_start_signals(
    combat: &CombatState,
    collector: &MonsterEntity,
) -> CollectorStartSignals {
    let player = &combat.entities.player;
    CollectorStartSignals {
        turn: combat.turn.turn_count,
        player_hp: player.current_hp,
        player_max_hp: player.max_hp,
        player_hp_percent: percent(player.current_hp, player.max_hp),
        collector_hp: collector.current_hp,
        collector_max_hp: collector.max_hp,
        collector_hp_percent: percent(collector.current_hp, collector.max_hp),
        torch_heads_alive: combat
            .entities
            .monsters
            .iter()
            .filter(|monster| enemy_id(monster) == Some(EnemyId::TorchHead))
            .filter(|monster| monster.is_alive_for_action())
            .count(),
        visible_incoming_damage: visible_incoming_damage(combat),
    }
}

fn collector_phase(turn: u32) -> &'static str {
    match turn {
        0 | 1 => "opening_spawn",
        2..=4 => "pre_mega_debuff",
        _ => "post_mega_debuff_or_late",
    }
}

fn collector_objectives(
    start: &CollectorStartSignals,
    phase: &'static str,
) -> Vec<BossPressureObjective> {
    vec![
        minion_pressure_objective(start),
        pre_debuff_stability_objective(start, phase),
        boss_hp_progress_objective(start, phase),
    ]
}

fn minion_pressure_objective(start: &CollectorStartSignals) -> BossPressureObjective {
    if start.torch_heads_alive == 0 {
        objective(
            "reduce_minion_pressure",
            "satisfied",
            "no_torch_heads_alive",
        )
    } else if start.torch_heads_alive >= 2 || start.visible_incoming_damage >= 18 {
        objective(
            "reduce_minion_pressure",
            "violated",
            "two_torch_heads_or_high_incoming",
        )
    } else {
        objective("reduce_minion_pressure", "watch", "one_torch_head_alive")
    }
}

fn pre_debuff_stability_objective(
    start: &CollectorStartSignals,
    phase: &'static str,
) -> BossPressureObjective {
    if phase != "opening_spawn" && phase != "pre_mega_debuff" {
        return objective(
            "pre_mega_debuff_stability",
            "unknown",
            "already_past_debuff_window",
        );
    }
    if start.player_hp_percent <= 40 || start.visible_incoming_damage >= start.player_hp / 2 {
        objective(
            "pre_mega_debuff_stability",
            "violated",
            "low_hp_or_large_visible_attack_before_debuff",
        )
    } else {
        objective(
            "pre_mega_debuff_stability",
            "watch",
            "debuff_window_not_resolved",
        )
    }
}

fn boss_hp_progress_objective(
    start: &CollectorStartSignals,
    phase: &'static str,
) -> BossPressureObjective {
    if phase == "opening_spawn" {
        return objective("boss_hp_progress", "unknown", "too_early_to_measure");
    }
    if start.collector_hp_percent >= 75 {
        objective(
            "boss_hp_progress",
            "violated",
            "slow_boss_damage_before_or_after_debuff",
        )
    } else {
        objective("boss_hp_progress", "watch", "boss_damage_progress_visible")
    }
}

fn collector_potion_permission(
    start: &CollectorStartSignals,
    phase: &'static str,
) -> BossPotionPermission {
    if (phase == "opening_spawn" || phase == "pre_mega_debuff")
        && (start.player_hp_percent <= 40
            || start.visible_incoming_damage >= 20
            || start.torch_heads_alive >= 2)
    {
        BossPotionPermission {
            level: "allow",
            reason: "prevent_collector_early_collapse",
        }
    } else {
        BossPotionPermission {
            level: "no_special_permission",
            reason: "collector_pressure_not_yet_emergency",
        }
    }
}

fn collector_tags(start: &CollectorStartSignals, phase: &'static str) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if start.torch_heads_alive >= 2 || start.visible_incoming_damage >= 18 {
        tags.push("collector_minion_pressure_high");
    }
    if (phase == "opening_spawn" || phase == "pre_mega_debuff") && start.player_hp_percent <= 40 {
        tags.push("collector_pre_debuff_collapse_risk");
    }
    if phase != "opening_spawn" && start.collector_hp_percent >= 75 {
        tags.push("collector_boss_progress_slow");
    }
    tags
}

fn objective(
    tag: &'static str,
    status: &'static str,
    reason: &'static str,
) -> BossPressureObjective {
    BossPressureObjective {
        tag,
        status,
        reason,
    }
}

fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

fn find_enemy(combat: &CombatState, id: EnemyId) -> Option<&MonsterEntity> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| enemy_id(monster) == Some(id) && monster.is_alive_for_action())
}

fn enemy_id(monster: &MonsterEntity) -> Option<EnemyId> {
    EnemyId::from_id(monster.monster_type)
}

fn percent(value: i32, max: i32) -> i32 {
    if max <= 0 {
        0
    } else {
        value.saturating_mul(100) / max
    }
}
