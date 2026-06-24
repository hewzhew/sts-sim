use std::collections::BTreeMap;

use sts_simulator::eval::branch_campaign::{BranchCampaignBranchV1, BranchCampaignReportV1};
use sts_simulator::eval::campaign_journal::{
    CampaignJournalCandidateV1, CampaignJournalEventPayloadV1,
};
use sts_simulator::eval::decision_path::DecisionPathEnvelopeV1;
use sts_simulator::eval::run_control::CombatAutomationTrajectoryRecordV1;

pub(super) fn render_final_boss_combat_report_inspection_v1(
    report: &BranchCampaignReportV1,
    inspect_index: usize,
) -> Result<String, String> {
    let candidates: Vec<(usize, &BranchCampaignBranchV1)> = report
        .victories
        .iter()
        .enumerate()
        .filter(|(_, branch)| branch.final_boss_combat_record.is_some())
        .collect();
    if candidates.is_empty() {
        let failures = final_boss_failure_branches_v1(report);
        if !failures.is_empty() {
            return Ok(render_final_boss_boundary_failure_inspection_v1(
                report,
                failures.as_slice(),
            ));
        }
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
    lines.extend(render_final_boss_comparison_lines_v1(report, branch));
    lines.extend(render_combat_automation_timeline_lines_v1(
        record.source.as_str(),
        record.action_count,
        &record.actions,
    ));
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_final_boss_boundary_failure_inspection_v1(
    report: &BranchCampaignReportV1,
    failures: &[&BranchCampaignBranchV1],
) -> String {
    let mut hp_min = i32::MAX;
    let mut hp_max = i32::MIN;
    let mut deck_min = usize::MAX;
    let mut deck_max = usize::MIN;
    let mut boss_counts = BTreeMap::<String, usize>::new();
    let mut pressure_counts = BTreeMap::<String, usize>::new();
    let mut debt_counts = BTreeMap::<String, usize>::new();
    for branch in failures {
        let Some(summary) = branch.summary.as_ref() else {
            continue;
        };
        hp_min = hp_min.min(summary.hp);
        hp_max = hp_max.max(summary.hp);
        deck_min = deck_min.min(summary.deck_count);
        deck_max = deck_max.max(summary.deck_count);
        let boss = if summary.boss.is_empty() {
            "unknown".to_string()
        } else {
            summary.boss.clone()
        };
        *boss_counts.entry(boss).or_default() += 1;
        for signal in &summary.boss_pressure {
            *pressure_counts.entry(signal.clone()).or_default() += 1;
        }
        for debt in &summary.run_debt {
            *debt_counts.entry(debt.clone()).or_default() += 1;
        }
    }

    let mut lines = Vec::new();
    lines.push(format!(
        "Final boss boundary failures: seed={} count={} combat_records=0 hp={}..{} deck={}..{} bosses=[{}]",
        report.seed,
        failures.len(),
        hp_min,
        hp_max,
        deck_min,
        deck_max,
        render_count_summary_v1(&boss_counts, 6)
    ));
    lines.push(
        "  note: no final boss combat trajectory record was captured; this report only summarizes campaign boundary facts."
            .to_string(),
    );
    if !pressure_counts.is_empty() {
        lines.push(format!(
            "  boss_pressure=[{}]",
            render_count_summary_v1(&pressure_counts, 8)
        ));
    }
    if !debt_counts.is_empty() {
        lines.push(format!(
            "  run_debt=[{}]",
            render_count_summary_v1(&debt_counts, 8)
        ));
    }
    lines.push("  examples:".to_string());
    for (index, branch) in failures.iter().take(5).enumerate() {
        lines.push(format!(
            "    {}. {}",
            index + 1,
            render_final_boss_branch_brief_v1(branch)
        ));
        if let Some(deck_key) = branch
            .summary
            .as_ref()
            .and_then(render_final_boss_deck_key_v1)
        {
            lines.push(format!("       deck: {deck_key}"));
        }
        if !branch.choice_labels.is_empty() {
            lines.push(format!(
                "       choices: {}",
                render_truncated_text(&branch.choice_labels.join(" -> "), 240)
            ));
        }
    }
    if failures.len() > 5 {
        lines.push(format!(
            "    ... {} more final boss boundary failure(s)",
            failures.len() - 5
        ));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_count_summary_v1(counts: &BTreeMap<String, usize>, limit: usize) -> String {
    let mut entries = counts.iter().collect::<Vec<_>>();
    entries.sort_by(|(left_label, left_count), (right_label, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_label.cmp(right_label))
    });
    entries
        .into_iter()
        .take(limit)
        .map(|(label, count)| format!("{}={count}", render_truncated_text(label, 72)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_final_boss_comparison_lines_v1(
    report: &BranchCampaignReportV1,
    selected_branch: &BranchCampaignBranchV1,
) -> Vec<String> {
    let failures = final_boss_failure_branches_v1(report);
    if failures.is_empty() {
        return Vec::new();
    }

    let victory_records = report
        .victories
        .iter()
        .filter(|branch| branch.final_boss_combat_record.is_some())
        .count();
    let mut hp_min = i32::MAX;
    let mut hp_max = i32::MIN;
    let mut deck_min = usize::MAX;
    let mut deck_max = usize::MIN;
    for branch in &failures {
        if let Some(summary) = branch.summary.as_ref() {
            hp_min = hp_min.min(summary.hp);
            hp_max = hp_max.max(summary.hp);
            deck_min = deck_min.min(summary.deck_count);
            deck_max = deck_max.max(summary.deck_count);
        }
    }

    let mut lines = Vec::new();
    lines.push("Comparison: final boss branches in this campaign report".to_string());
    lines.push(format!(
        "  victory_records={} victories={} final_boss_failures={} failure_hp={}..{} failure_deck={}..{}",
        victory_records,
        report.victories.len(),
        failures.len(),
        hp_min,
        hp_max,
        deck_min,
        deck_max
    ));
    lines.push(format!(
        "  selected victory: {}",
        render_final_boss_branch_brief_v1(selected_branch)
    ));
    if let Some(deck_key) = selected_branch
        .summary
        .as_ref()
        .and_then(render_final_boss_deck_key_v1)
    {
        lines.push(format!("    deck: {deck_key}"));
    }
    if final_boss_branch_details_are_compacted_v1(selected_branch)
        || failures
            .iter()
            .any(|branch| final_boss_branch_details_are_compacted_v1(branch))
    {
        lines.push(
            "  note: branch choice paths or deck_key are absent from the compact campaign state; use journal/lineage inspection for historical decision details."
                .to_string(),
        );
    }
    let divergence_groups =
        final_boss_divergence_groups_v1(report, selected_branch, failures.as_slice());
    lines.push("  divergence groups:".to_string());
    for (index, group) in divergence_groups.iter().take(5).enumerate() {
        lines.push(format!(
            "    {}. failures={} hp={}..{} deck={}..{} {}",
            index + 1,
            group.failure_count,
            group.hp_min,
            group.hp_max,
            group.deck_min,
            group.deck_max,
            group.divergence.render()
        ));
        for branch in group.examples.iter().take(2) {
            lines.push(format!(
                "       example: {}",
                render_final_boss_branch_brief_v1(branch)
            ));
        }
    }
    if divergence_groups.len() > 5 {
        lines.push(format!(
            "    ... {} more divergence group(s)",
            divergence_groups.len() - 5
        ));
    }
    lines
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FinalBossBranchDivergenceV1 {
    common_prefix: usize,
    victory: String,
    failure: String,
}

impl FinalBossBranchDivergenceV1 {
    fn render(&self) -> String {
        format!(
            "after {} decisions | victory -> {} | failure -> {}",
            self.common_prefix, self.victory, self.failure
        )
    }
}

#[derive(Clone, Debug)]
struct FinalBossDivergenceGroupV1<'a> {
    divergence: FinalBossBranchDivergenceV1,
    failure_count: usize,
    hp_min: i32,
    hp_max: i32,
    deck_min: usize,
    deck_max: usize,
    examples: Vec<&'a BranchCampaignBranchV1>,
}

fn final_boss_divergence_groups_v1<'a>(
    report: &BranchCampaignReportV1,
    victory: &BranchCampaignBranchV1,
    failures: &'a [&'a BranchCampaignBranchV1],
) -> Vec<FinalBossDivergenceGroupV1<'a>> {
    let mut groups = BTreeMap::<String, FinalBossDivergenceGroupV1<'a>>::new();
    for failure in failures {
        let divergence =
            final_boss_branch_divergence_v1(report, victory, failure).unwrap_or_else(|| {
                FinalBossBranchDivergenceV1 {
                    common_prefix: common_command_prefix_len_v1(
                        &victory.commands,
                        &failure.commands,
                    ),
                    victory: "no divergent command".to_string(),
                    failure: "no divergent command".to_string(),
                }
            });
        let key = format!(
            "{:04}|{}|{}",
            divergence.common_prefix, divergence.victory, divergence.failure
        );
        let Some(summary) = failure.summary.as_ref() else {
            continue;
        };
        let entry = groups
            .entry(key)
            .or_insert_with(|| FinalBossDivergenceGroupV1 {
                divergence: divergence.clone(),
                failure_count: 0,
                hp_min: summary.hp,
                hp_max: summary.hp,
                deck_min: summary.deck_count,
                deck_max: summary.deck_count,
                examples: Vec::new(),
            });
        entry.failure_count += 1;
        entry.hp_min = entry.hp_min.min(summary.hp);
        entry.hp_max = entry.hp_max.max(summary.hp);
        entry.deck_min = entry.deck_min.min(summary.deck_count);
        entry.deck_max = entry.deck_max.max(summary.deck_count);
        if entry.examples.len() < 2 {
            entry.examples.push(*failure);
        }
    }

    let mut groups = groups.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.divergence
            .common_prefix
            .cmp(&right.divergence.common_prefix)
            .then_with(|| right.failure_count.cmp(&left.failure_count))
            .then_with(|| left.divergence.failure.cmp(&right.divergence.failure))
    });
    groups
}

fn final_boss_branch_divergence_v1(
    report: &BranchCampaignReportV1,
    victory: &BranchCampaignBranchV1,
    failure: &BranchCampaignBranchV1,
) -> Option<FinalBossBranchDivergenceV1> {
    let common_prefix = common_command_prefix_len_v1(&victory.commands, &failure.commands);
    let victory_next = victory.commands.get(common_prefix)?;
    let failure_next = failure.commands.get(common_prefix)?;
    if victory_next == failure_next {
        return None;
    }
    Some(FinalBossBranchDivergenceV1 {
        common_prefix,
        victory: render_journal_command_candidate_label_v1(
            report,
            &victory.commands[..common_prefix],
            victory_next,
        ),
        failure: render_journal_command_candidate_label_v1(
            report,
            &failure.commands[..common_prefix],
            failure_next,
        ),
    })
}

fn common_command_prefix_len_v1(left: &[String], right: &[String]) -> usize {
    left.iter()
        .zip(right.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn render_journal_command_candidate_label_v1(
    report: &BranchCampaignReportV1,
    parent_commands: &[String],
    command: &str,
) -> String {
    report
        .journal
        .events
        .iter()
        .filter(|event| {
            let event_path = DecisionPathEnvelopeV1::from_commands(&event.branch_commands);
            let branch_path = DecisionPathEnvelopeV1::from_commands(parent_commands);
            event_path.journal_parent_depth_against(&branch_path) == Some(parent_commands.len())
        })
        .filter_map(|event| journal_event_candidate_for_command_v1(&event.payload, command))
        .next()
        .map(|candidate| format!("{} {{{}}}", candidate.label, candidate.command))
        .unwrap_or_else(|| format!("{{{command}}}"))
}

fn journal_event_candidate_for_command_v1<'a>(
    payload: &'a CampaignJournalEventPayloadV1,
    command: &str,
) -> Option<&'a CampaignJournalCandidateV1> {
    journal_event_candidates_from_payload_v1(payload)
        .iter()
        .find(|candidate| candidate.command == command)
}

fn journal_event_candidates_from_payload_v1(
    payload: &CampaignJournalEventPayloadV1,
) -> &[CampaignJournalCandidateV1] {
    match payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { candidates, .. } => candidates,
        CampaignJournalEventPayloadV1::RouteDecision { .. } => &[],
    }
}

fn final_boss_branch_details_are_compacted_v1(branch: &BranchCampaignBranchV1) -> bool {
    branch.choice_labels.is_empty()
        || branch
            .summary
            .as_ref()
            .is_some_and(|summary| summary.deck_count > 0 && summary.deck_key.is_empty())
}

fn final_boss_failure_branches_v1(report: &BranchCampaignReportV1) -> Vec<&BranchCampaignBranchV1> {
    report
        .abandoned
        .iter()
        .filter(|branch| {
            branch.frontier_title == "Combat"
                && branch
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.act == 3 && summary.floor >= 48)
        })
        .collect()
}

fn render_final_boss_branch_brief_v1(branch: &BranchCampaignBranchV1) -> String {
    let state = branch
        .summary
        .as_ref()
        .map(|summary| {
            format!(
                "A{}F{} HP {}/{} gold {} deck {} boss={}",
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
            )
        })
        .unwrap_or_else(|| "no-summary".to_string());
    let stop_reason = if branch.stop_reason.is_empty() {
        String::new()
    } else {
        format!(
            " stop={}",
            render_truncated_text(branch.stop_reason.as_str(), 96)
        )
    };
    format!("{state}{stop_reason}")
}

fn render_final_boss_deck_key_v1(
    summary: &sts_simulator::eval::branch_campaign::BranchCampaignBranchSummaryV1,
) -> Option<String> {
    if summary.deck_key.is_empty() {
        None
    } else {
        Some(render_truncated_text(summary.deck_key.as_str(), 260))
    }
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
            run_prelude: Default::default(),
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
                continuation_origin: None,
                decision_candidate_axis: None,
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
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
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

    #[test]
    fn final_boss_inspect_reports_boundary_failures_without_combat_record() {
        let report = BranchCampaignReportV1 {
            schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
            seed: 1401646639,
            run_domain: BranchCampaignRunDomainV1::default(),
            run_prelude: Default::default(),
            rounds_completed: 64,
            stop_reason: "max_rounds".to_string(),
            active: Vec::new(),
            frozen: Vec::new(),
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: vec![BranchCampaignBranchV1 {
                branch_id: "final-boss-failure".to_string(),
                commands: vec!["rp 2".to_string(), "smith 16".to_string()],
                choice_labels: vec!["Shockwave".to_string(), "Smith Power Through".to_string()],
                summary: Some(BranchCampaignBranchSummaryV1 {
                    act: 3,
                    floor: 48,
                    hp: 80,
                    max_hp: 80,
                    gold: 421,
                    deck_count: 25,
                    deck_key: "Armaments+, Bloodletting, Clothesline, Ghostly Armor+".to_string(),
                    formation_stage: "PlanCommitted".to_string(),
                    formation_strengths: Vec::new(),
                    formation_needs: Vec::new(),
                    trajectory_key: String::new(),
                    boss: "AwakenedOne".to_string(),
                    boss_pressure: vec!["missing:phase_power_plan".to_string()],
                    run_debt: vec!["SneckoEye=random_cost_deck_shape_debt".to_string()],
                    event_boundary: None,
                    reward_boundary: None,
                }),
                strategic_summary: BranchSignatureCompact::default(),
                frontier_title: "Combat".to_string(),
                status: BranchCampaignBranchStatusV1::Abandoned,
                stop_reason: "combat search did not find an executable complete win".to_string(),
                continuation_origin: None,
                decision_candidate_axis: None,
                lineage_decision_signal_rank_adjustment: 0,
                rank_key: -799000,
                final_boss_combat_record: None,
                combat_lab_probes: Vec::new(),
            }],
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: Default::default(),
            rounds: Vec::new(),
        };

        let rendered = super::render_final_boss_combat_report_inspection_v1(&report, 0)
            .expect("boundary failures should be inspectable without a combat trajectory");

        assert!(rendered.contains("Final boss boundary failures"));
        assert!(rendered.contains("bosses=[AwakenedOne=1]"));
        assert!(rendered.contains("stop=combat search did not find an executable complete win"));
        assert!(!rendered.contains("likely issue"));
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
