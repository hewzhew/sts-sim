use std::collections::BTreeMap;

use crate::ai::strategic::run_debt_ledger_v1;
use crate::eval::branch_experiment::BranchExperimentBranchReportV1;
use crate::eval::branch_experiment_trajectory::{
    branch_trajectory_key_v1, BranchTrajectorySignatureV1,
};
use crate::eval::run_control::RunControlSession;

use super::assessment::campaign_branch_assessment_from_session_v1;
use super::model::{BranchCampaignBranchSummaryV1, BranchCampaignBranchV1};
use super::state_graph::BranchStateStoreV1;
use super::BranchCampaignRunStateV1;

pub(super) fn campaign_refresh_all_branch_summaries_from_state_store_v1(
    state: &mut BranchCampaignRunStateV1,
) {
    campaign_refresh_branch_group_summaries_from_state_store_v1(
        &mut state.scheduled,
        &state.state_store,
    );
    campaign_refresh_branch_group_summaries_from_state_store_v1(
        &mut state.parked,
        &state.state_store,
    );
    campaign_refresh_branch_group_summaries_from_state_store_v1(
        &mut state.victories,
        &state.state_store,
    );
    campaign_refresh_branch_group_summaries_from_state_store_v1(
        &mut state.abandoned,
        &state.state_store,
    );
    campaign_refresh_branch_group_summaries_from_state_store_v1(
        &mut state.stuck,
        &state.state_store,
    );
}

fn campaign_refresh_branch_group_summaries_from_state_store_v1(
    branches: &mut [BranchCampaignBranchV1],
    state_store: &BranchStateStoreV1,
) {
    for branch in branches {
        if let Some(session) = state_store.get_session(&branch.commands) {
            campaign_refresh_branch_summary_from_session_v1(branch, session);
        }
    }
}

pub(super) fn campaign_refresh_branch_summaries_from_state_store_v1(
    branches: &mut [BranchCampaignBranchV1],
    state_store: &BranchStateStoreV1,
) {
    campaign_refresh_branch_group_summaries_from_state_store_v1(branches, state_store);
}

pub(super) fn campaign_refresh_branch_summary_from_session_v1(
    branch: &mut BranchCampaignBranchV1,
    session: &RunControlSession,
) {
    branch.assessment = Some(campaign_branch_assessment_from_session_v1(session));
    let event_boundary =
        crate::eval::event_boundary_packet_v1::event_boundary_packet_from_session_v1(session);
    let reward_boundary =
        crate::eval::reward_boundary_packet_v1::reward_boundary_packet_from_session_v1(session);
    let Some(summary) = branch.summary.as_mut() else {
        return;
    };
    summary.event_boundary = event_boundary;
    summary.reward_boundary = reward_boundary;
    summary.act = session.run_state.act_num;
    summary.floor = session.run_state.floor_num;
    let (hp, max_hp) = session.visible_player_hp();
    summary.hp = hp;
    summary.max_hp = max_hp;
    summary.gold = session.run_state.gold;
    summary.deck_count = session.run_state.master_deck.len();
    summary.deck_key = campaign_deck_key_v1(session);
    summary.boss = branch_campaign_boss_label_v1(session);
    summary.boss_pressure = branch_campaign_boss_pressure_labels_v1(session);
    summary.run_debt = branch_campaign_run_debt_labels_v1(session);
}

pub(super) fn campaign_summary_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchSummaryV1 {
    let trajectory_key = campaign_trajectory_key_from_report_branch_v1(parent, branch);
    BranchCampaignBranchSummaryV1 {
        act: branch.summary.act,
        floor: branch.summary.floor,
        hp: branch.summary.hp,
        max_hp: branch.summary.max_hp,
        gold: branch.summary.gold,
        deck_count: branch.summary.deck_count,
        deck_key: String::new(),
        formation_stage: format!("{:?}", branch.summary.formation_stage),
        formation_strengths: branch
            .summary
            .formation_strengths
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        formation_needs: branch
            .summary
            .formation_needs
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        trajectory_key,
        boss: String::new(),
        boss_pressure: Vec::new(),
        run_debt: Vec::new(),
        event_boundary: None,
        reward_boundary: None,
    }
}

fn campaign_deck_key_v1(session: &RunControlSession) -> String {
    let mut counts = BTreeMap::<String, usize>::new();
    for card in &session.run_state.master_deck {
        *counts
            .entry(format!("{:?}+{}", card.id, card.upgrades))
            .or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(card, count)| format!("{card}x{count}"))
        .collect::<Vec<_>>()
        .join(";")
}

fn branch_campaign_boss_label_v1(session: &RunControlSession) -> String {
    branch_campaign_boss_v1(session)
        .map(|boss| format!("{boss:?}"))
        .unwrap_or_default()
}

fn branch_campaign_boss_pressure_labels_v1(session: &RunControlSession) -> Vec<String> {
    let _ = session;
    // Disabled: this report-only pressure model cannot yet measure whether a
    // route actually satisfied the pressure it labels, so rendering it as a
    // warning is misleading noise.
    Vec::new()
}

fn branch_campaign_run_debt_labels_v1(session: &RunControlSession) -> Vec<String> {
    run_debt_ledger_v1(&session.run_state).compact_labels()
}

fn branch_campaign_boss_v1(
    session: &RunControlSession,
) -> Option<crate::content::monsters::factory::EncounterId> {
    session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied())
}

fn campaign_trajectory_key_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> String {
    let mut trajectory = parent
        .summary
        .as_ref()
        .and_then(|summary| parse_branch_trajectory_key_for_campaign_v1(&summary.trajectory_key))
        .unwrap_or_default();
    merge_campaign_branch_trajectory_v1(&mut trajectory, &branch.summary.trajectory);
    branch_trajectory_key_v1(&trajectory)
}

fn merge_campaign_branch_trajectory_v1(
    target: &mut BranchTrajectorySignatureV1,
    source: &BranchTrajectorySignatureV1,
) {
    target.frontload_picks = target
        .frontload_picks
        .saturating_add(source.frontload_picks);
    target.transition_frontload_picks = target
        .transition_frontload_picks
        .saturating_add(source.transition_frontload_picks);
    target.scaling_picks = target.scaling_picks.saturating_add(source.scaling_picks);
    target.defense_picks = target.defense_picks.saturating_add(source.defense_picks);
    target.engine_generator_picks = target
        .engine_generator_picks
        .saturating_add(source.engine_generator_picks);
    target.engine_payoff_picks = target
        .engine_payoff_picks
        .saturating_add(source.engine_payoff_picks);
    target.draw_energy_picks = target
        .draw_energy_picks
        .saturating_add(source.draw_energy_picks);
    merge_campaign_trajectory_keys_v1(&mut target.setup_keys, &source.setup_keys);
    merge_campaign_trajectory_keys_v1(&mut target.package_keys, &source.package_keys);
}

fn merge_campaign_trajectory_keys_v1(target: &mut Vec<String>, source: &[String]) {
    for key in source {
        if !target.iter().any(|existing| existing == key) {
            target.push(key.clone());
        }
    }
    target.sort();
}

fn parse_branch_trajectory_key_for_campaign_v1(key: &str) -> Option<BranchTrajectorySignatureV1> {
    if key.trim().is_empty() {
        return None;
    }
    let mut signature = BranchTrajectorySignatureV1::default();
    for part in key.split('|') {
        if let Some(value) = part.strip_prefix("setup=") {
            signature.setup_keys = parse_campaign_trajectory_key_list_v1(value);
        } else if let Some(value) = part.strip_prefix("pkg=") {
            signature.package_keys = parse_campaign_trajectory_key_list_v1(value);
        } else if let Some(value) = part.strip_prefix("frontload=") {
            signature.frontload_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("transition=") {
            signature.transition_frontload_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("scaling=") {
            signature.scaling_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("defense=") {
            signature.defense_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("engine_gen=") {
            signature.engine_generator_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("engine_payoff=") {
            signature.engine_payoff_picks = value.parse().ok()?;
        } else if let Some(value) = part.strip_prefix("draw_energy=") {
            signature.draw_energy_picks = value.parse().ok()?;
        }
    }
    signature.setup_keys.sort();
    signature.package_keys.sort();
    Some(signature)
}

fn parse_campaign_trajectory_key_list_v1(value: &str) -> Vec<String> {
    if value == "-" || value.is_empty() {
        return Vec::new();
    }
    value
        .split('+')
        .filter(|key| !key.is_empty())
        .map(str::to_string)
        .collect()
}
