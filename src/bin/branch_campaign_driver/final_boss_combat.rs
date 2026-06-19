use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;
use sts_simulator::eval::run_control::CombatAutomationTrajectoryRecordV1;

pub(super) fn render_final_boss_combat_report_inspection_v1(
    report: &BranchCampaignReportV1,
    inspect_index: usize,
) -> Result<String, String> {
    let candidates: Vec<(
        usize,
        &sts_simulator::eval::branch_campaign::BranchCampaignBranchV1,
    )> = report
        .victories
        .iter()
        .enumerate()
        .filter(|(_, branch)| branch.final_boss_combat_record.is_some())
        .collect();
    if candidates.is_empty() {
        return Err("campaign report contains no final boss combat records".to_string());
    }
    if inspect_index >= candidates.len() {
        return Err(format!(
            "--inspect-index {inspect_index} is out of range for {} final boss combat record(s)",
            candidates.len()
        ));
    }
    let (victory_index, branch) = candidates[inspect_index];
    let record = branch
        .final_boss_combat_record
        .as_ref()
        .expect("candidate filter requires a final boss combat record");
    let mut lines = Vec::new();
    lines.push(format!(
        "Final boss combat record: seed={} victory={}/{} source={} actions={} snapshots={}",
        report.seed,
        victory_index + 1,
        report.victories.len(),
        record.source,
        record.action_count,
        record
            .actions
            .iter()
            .filter(|action| action.combat_after.is_some())
            .count()
    ));
    if let Some(summary) = branch.summary.as_ref() {
        lines.push(format!(
            "Branch: A{}F{} HP {}/{} gold {} deck {} boss={}",
            summary.act,
            summary.floor,
            summary.hp,
            summary.max_hp,
            summary.gold,
            summary.deck_count,
            if summary.boss.is_empty() {
                "unknown"
            } else {
                summary.boss.as_str()
            }
        ));
    }
    if !branch.choice_labels.is_empty() {
        lines.push(format!(
            "Choices: {}",
            render_truncated_text(&branch.choice_labels.join(" -> "), 360)
        ));
    }
    lines.extend(render_combat_automation_timeline_lines_v1(
        record.source.as_str(),
        record.action_count,
        &record.actions,
    ));
    Ok(format!("{}\n", lines.join("\n")))
}

pub(super) fn render_last_auto_combat_checkpoint_inspection_v1(
    seed: u64,
    match_index: usize,
    match_count: usize,
    session: &sts_simulator::eval::run_control::RunControlSession,
    commands: &[String],
) -> Result<String, String> {
    let record = session.last_combat_automation_trajectory().ok_or_else(|| {
        "selected checkpoint session has no last automation trajectory; rerun campaign with a checkpoint created after this feature, or choose a branch whose last combat was resolved by search-combat".to_string()
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
        record.source.as_str(),
        record.action_count,
        &record.actions,
    )
}

fn render_combat_automation_timeline_lines_v1(
    source: &str,
    action_count: usize,
    actions: &[sts_simulator::eval::run_control::CombatAutomationActionV1],
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

#[cfg(test)]
mod tests {
    use sts_simulator::ai::strategic::BranchSignatureCompact;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
        BranchCampaignReportV1, BranchCampaignRunDomainV1, BRANCH_CAMPAIGN_SCHEMA_NAME,
        BRANCH_CAMPAIGN_SCHEMA_VERSION,
    };
    use sts_simulator::eval::branch_experiment::BranchExperimentBossCombatRecordV1;
    use sts_simulator::eval::run_control::{
        CombatAutomationActionV1, CombatAutomationMonsterStateV1, CombatAutomationStepStateV1,
    };
    use sts_simulator::state::core::ClientInput;

    #[test]
    fn final_boss_combat_timeline_marks_time_warp_trigger() {
        let report = BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed: 521,
            run_domain: BranchCampaignRunDomainV1::default(),
            rounds_completed: 1,
            stop_reason: "victory".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: vec![BranchCampaignBranchV1 {
                branch_id: "winner".to_string(),
                commands: Vec::new(),
                choice_labels: vec!["Limit Break".to_string()],
                summary: Some(BranchCampaignBranchSummaryV1 {
                    act: 3,
                    floor: 48,
                    hp: 50,
                    max_hp: 80,
                    gold: 123,
                    deck_count: 20,
                    deck_key: String::new(),
                    formation_stage: "PlanSeeded".to_string(),
                    formation_strengths: Vec::new(),
                    formation_needs: Vec::new(),
                    trajectory_key: String::new(),
                    boss: "Time Eater".to_string(),
                    boss_pressure: Vec::new(),
                    run_debt: Vec::new(),
                    event_boundary: None,
                    reward_boundary: None,
                }),
                strategic_summary: BranchSignatureCompact::default(),
                frontier_title: "Game Over Victory".to_string(),
                status: BranchCampaignBranchStatusV1::TerminalVictory,
                stop_reason: "victory".to_string(),
                lineage_decision_signal_rank_adjustment: 0,
                rank_key: 0,
                final_boss_combat_record: Some(BranchExperimentBossCombatRecordV1 {
                    source: "final_boss_combat".to_string(),
                    action_count: 2,
                    actions: vec![
                        combat_action_with_time_warp(0, 11, 0, false),
                        combat_action_with_time_warp(1, 0, 2, true),
                    ],
                    label_role: "behavior_policy_not_teacher".to_string(),
                }),
                combat_lab_probes: Vec::new(),
            }],
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            rounds: Vec::new(),
        };

        let rendered = super::render_final_boss_combat_report_inspection_v1(&report, 0)
            .expect("final boss timeline renders");

        assert!(rendered.contains("Final boss combat record: seed=521"));
        assert!(rendered.contains("TIME_WARP_TRIGGER"));
        assert!(rendered.contains("monster_strength+2"));
        assert!(rendered.contains("early_end_pending"));
    }

    #[test]
    fn shared_auto_combat_timeline_renders_checkpoint_records() {
        let lines = super::render_combat_automation_timeline_lines_v1(
            "search_combat",
            1,
            &[combat_action_with_time_warp(0, 11, 0, true)],
        );
        let rendered = lines.join("\n");

        assert!(rendered.contains("source=search_combat"));
        assert!(rendered.contains("early_end_pending"));
    }

    fn combat_action_with_time_warp(
        step_index: usize,
        time_warp: i32,
        strength: i32,
        early_end_turn_pending: bool,
    ) -> CombatAutomationActionV1 {
        CombatAutomationActionV1 {
            step_index,
            action_key: format!("combat/play_card/test/{step_index}"),
            input: ClientInput::EndTurn,
            drawn_cards: Vec::new(),
            combat_after: Some(CombatAutomationStepStateV1 {
                player_hp: 50,
                player_max_hp: 80,
                player_block: 0,
                energy: 3,
                cards_played_this_turn: 11,
                early_end_turn_pending,
                monsters: vec![CombatAutomationMonsterStateV1 {
                    id: 0,
                    label: "Time Eater".to_string(),
                    hp: 300,
                    max_hp: 456,
                    block: 0,
                    alive: true,
                    time_warp,
                    strength,
                }],
            }),
        }
    }
}
