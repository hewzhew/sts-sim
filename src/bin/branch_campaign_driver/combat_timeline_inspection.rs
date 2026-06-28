use sts_simulator::eval::run_control::{
    CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1, RunControlSession,
};

pub(super) fn render_last_auto_combat_checkpoint_inspection_v1(
    seed: u64,
    match_index: usize,
    match_count: usize,
    session: &RunControlSession,
    commands: &[String],
) -> Result<String, String> {
    let record = session.last_combat_automation_trajectory().ok_or_else(|| {
        "selected checkpoint session has no last automation trajectory; choose a branch whose last combat was resolved by search-combat".to_string()
    })?;
    let mut lines = Vec::new();
    lines.push(format!(
        "Last auto combat record: seed={} match={}/{} source={} actions={} snapshots={}",
        seed,
        match_index + 1,
        match_count,
        record.source,
        record.action_count,
        record
            .actions
            .iter()
            .filter(|action| action.combat_after.is_some())
            .count()
    ));
    lines.push(format!(
        "Branch: A{}F{} HP {}/{} gold {} deck {}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.master_deck.len()
    ));
    if !commands.is_empty() {
        lines.push(format!(
            "Commands: {}",
            render_truncated_text(&commands.join(" -> "), 360)
        ));
    }
    lines.extend(render_combat_automation_record_timeline_lines_v1(record));
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_combat_automation_record_timeline_lines_v1(
    record: &CombatAutomationTrajectoryRecordV1,
) -> Vec<String> {
    render_combat_automation_timeline_lines_v1(
        record.source.label(),
        record.action_count,
        &record.actions,
    )
}

fn render_combat_automation_timeline_lines_v1(
    source: &str,
    action_count: usize,
    actions: &[CombatAutomationActionV1],
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "Timeline: source={source} actions={action_count} | step cards tw str hp enemy_hp tags | action"
    ));

    let mut previous_time_warp: Option<i32> = None;
    let mut previous_strength: Option<i32> = None;
    let mut previous_early_end_pending = false;
    for action in actions {
        let Some(after) = action.combat_after.as_ref() else {
            lines.push(format!(
                "  {:>3} legacy-no-snapshot | {}",
                action.step_index, action.action_key
            ));
            continue;
        };
        let monster = after.monsters.first();
        let time_warp = monster.map(|monster| monster.time_warp).unwrap_or_default();
        let strength = monster.map(|monster| monster.strength).unwrap_or_default();
        let enemy_hp = monster
            .map(|monster| format!("{}/{}", monster.hp, monster.max_hp))
            .unwrap_or_else(|| "-".to_string());
        let mut tags = Vec::new();
        if previous_early_end_pending && !after.early_end_turn_pending {
            tags.push("forced_end_resolved_before_action");
        }
        if after.early_end_turn_pending {
            tags.push("early_end_pending");
        }
        if previous_time_warp.is_some_and(|previous| previous >= 11) && time_warp == 0 {
            tags.push("TIME_WARP_TRIGGER");
        }
        if previous_strength.is_some_and(|previous| strength == previous + 2) {
            tags.push("monster_strength+2");
        }
        previous_time_warp = Some(time_warp);
        previous_strength = Some(strength);
        previous_early_end_pending = after.early_end_turn_pending;
        let tag_text = if tags.is_empty() {
            "-".to_string()
        } else {
            tags.join(",")
        };
        lines.push(format!(
            "  {:>3} {:>2} {:>2} {:>3} {}/{} {:>9} {:<38} | {}",
            action.step_index,
            after.cards_played_this_turn,
            time_warp,
            strength,
            after.player_hp,
            after.player_max_hp,
            enemy_hp,
            tag_text,
            action.action_key
        ));
    }
    lines
}

fn render_truncated_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut rendered = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    rendered.push_str("...");
    rendered
}
