use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{CombatLineLabReport, SearchTerminalLabel};
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::runtime::combat::{CombatState, MonsterEntity};
use sts_simulator::sim::combat_projection::monster_preview_total_damage_in_combat;

use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct BossPressureLensReport {
    schema: &'static str,
    boss: &'static str,
    phase: &'static str,
    start: CollectorStartSignals,
    tags: Vec<&'static str>,
    objectives: Vec<BossPressureObjective>,
    potion_permission: BossPotionPermission,
    line_reviews: Vec<BossLineReview>,
}

#[derive(Serialize)]
struct CollectorStartSignals {
    turn: u32,
    player_hp: i32,
    player_max_hp: i32,
    player_hp_percent: i32,
    collector_hp: i32,
    collector_max_hp: i32,
    collector_hp_percent: i32,
    torch_heads_alive: usize,
    visible_incoming_damage: i32,
}

#[derive(Serialize)]
struct BossPressureObjective {
    tag: &'static str,
    status: &'static str,
    reason: &'static str,
}

#[derive(Serialize)]
struct BossPotionPermission {
    level: &'static str,
    reason: &'static str,
}

#[derive(Serialize)]
struct BossLineReview {
    source: String,
    terminal: SearchTerminalLabel,
    final_hp: Option<i32>,
    hp_loss: Option<i32>,
    turns: Option<u32>,
    potions_used: Option<u32>,
    tags: Vec<&'static str>,
}

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

fn collect_line_reviews(
    ladder: &[SearchReview],
    line_lab: Option<&CombatLineLabReport>,
) -> Vec<BossLineReview> {
    let mut reviews: Vec<_> = ladder.iter().map(line_review_from_search).collect();
    if let Some(turn_pool) = line_lab.and_then(|report| report.turn_pool.as_ref()) {
        reviews.extend(turn_pool.lanes.iter().map(|line| BossLineReview {
            source: format!("turn_pool:{}", line.lane),
            terminal: line.terminal,
            final_hp: Some(line.final_hp),
            hp_loss: None,
            turns: Some(line.turns),
            potions_used: Some(line.potions_used),
            tags: line_quality_tags(
                line.terminal,
                Some(line.final_hp),
                Some(line.turns),
                Some(line.potions_used),
                line.living_enemy_count,
                line.total_enemy_hp,
            ),
        }));
    }
    reviews
}

fn aggregate_line_tags(line_reviews: &[BossLineReview]) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "no_win_left_multi_target_pressure")
    }) {
        tags.push("collector_lines_leave_multi_target_pressure");
    }
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "no_win_boss_hp_still_high")
    }) {
        tags.push("collector_lines_leave_boss_hp_high");
    }
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "failed_after_debuff_window")
    }) {
        tags.push("collector_lines_fail_after_debuff_window");
    }
    if line_reviews
        .iter()
        .any(|line| line.tags.iter().any(|tag| *tag == "dirty_win_low_hp"))
    {
        tags.push("collector_dirty_win_only");
    }
    tags
}

fn line_review_from_search(review: &SearchReview) -> BossLineReview {
    let progress = review.facts.diagnostic_progress.as_ref();
    BossLineReview {
        source: review.label.to_string(),
        terminal: progress
            .map(|facts| facts.terminal)
            .unwrap_or(SearchTerminalLabel::Unresolved),
        final_hp: review
            .final_hp
            .or_else(|| progress.map(|facts| facts.final_hp)),
        hp_loss: review
            .hp_loss
            .or_else(|| progress.map(|facts| facts.hp_loss)),
        turns: review.turns.or_else(|| progress.map(|facts| facts.turns)),
        potions_used: review
            .potions_used
            .or_else(|| progress.map(|facts| facts.potions_used)),
        tags: line_quality_tags(
            progress
                .map(|facts| facts.terminal)
                .unwrap_or(SearchTerminalLabel::Unresolved),
            review
                .final_hp
                .or_else(|| progress.map(|facts| facts.final_hp)),
            review.turns.or_else(|| progress.map(|facts| facts.turns)),
            review
                .potions_used
                .or_else(|| progress.map(|facts| facts.potions_used)),
            progress
                .map(|facts| facts.living_enemy_count)
                .unwrap_or_default(),
            progress
                .map(|facts| facts.total_enemy_hp)
                .unwrap_or_default(),
        ),
    }
}

fn line_quality_tags(
    terminal: SearchTerminalLabel,
    final_hp: Option<i32>,
    turns: Option<u32>,
    potions_used: Option<u32>,
    living_enemy_count: usize,
    total_enemy_hp: i32,
) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if terminal == SearchTerminalLabel::Win && final_hp.is_some_and(|hp| hp <= 10) {
        tags.push("dirty_win_low_hp");
    }
    if terminal == SearchTerminalLabel::Win
        && final_hp.is_some_and(|hp| hp <= 20)
        && potions_used.unwrap_or(0) > 0
    {
        tags.push("potion_rescue_desperate");
    }
    if terminal != SearchTerminalLabel::Win && living_enemy_count >= 2 {
        tags.push("no_win_left_multi_target_pressure");
    }
    if terminal != SearchTerminalLabel::Win && total_enemy_hp >= 80 {
        tags.push("no_win_boss_hp_still_high");
    }
    if terminal != SearchTerminalLabel::Win && turns.unwrap_or(0) >= 4 {
        tags.push("failed_after_debuff_window");
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
