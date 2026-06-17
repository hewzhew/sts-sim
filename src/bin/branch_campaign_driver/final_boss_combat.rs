use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;

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
    lines.push("Timeline: step cards tw str hp boss_hp tags | action".to_string());

    let mut previous_time_warp: Option<i32> = None;
    let mut previous_strength: Option<i32> = None;
    let mut previous_early_end_pending = false;
    for action in &record.actions {
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
        let boss_hp = monster
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
            boss_hp,
            tag_text,
            action.action_key
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
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
        BranchCampaignReportV1, BRANCH_CAMPAIGN_SCHEMA_NAME, BRANCH_CAMPAIGN_SCHEMA_VERSION,
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
            rounds: Vec::new(),
        };

        let rendered = super::render_final_boss_combat_report_inspection_v1(&report, 0)
            .expect("final boss timeline renders");

        assert!(rendered.contains("Final boss combat record: seed=521"));
        assert!(rendered.contains("TIME_WARP_TRIGGER"));
        assert!(rendered.contains("monster_strength+2"));
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
