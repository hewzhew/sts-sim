use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;
use sts_simulator::eval::run_control::{
    build_decision_surface, render_run_control_details, render_run_control_state,
    RunControlCommand, RunControlSession,
};

use super::checkpoint_evidence::{
    render_checkpoint_campfire_evidence_v1, render_checkpoint_card_reward_evidence_v1,
    render_checkpoint_deck_mutation_v1, render_checkpoint_route_evidence_v1,
    render_checkpoint_shop_evidence_v1,
};
use super::command_inputs::{InspectCommandInput, InspectFiltersInput};
use super::final_boss_combat::{
    render_final_boss_combat_report_inspection_v1, render_last_auto_combat_checkpoint_inspection_v1,
};
use super::shop_challenge::render_checkpoint_shop_plan_challenge_v1;
use super::{combat_lab, inspect_summary, read_campaign_checkpoint_v1, read_campaign_report_v1};

pub(super) fn run_final_boss_combat_report_inspection(
    input: &InspectCommandInput,
) -> Result<(), String> {
    let path = input
        .report_path
        .as_ref()
        .ok_or_else(|| "--inspect-final-boss-combat requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(path)?;
    print!(
        "{}",
        render_final_boss_combat_report_inspection_v1(&report, input.filters.index.unwrap_or(0))?
    );
    Ok(())
}

pub(super) fn run_checkpoint_inspection(input: &InspectCommandInput) -> Result<(), String> {
    let path = input
        .checkpoint_path
        .as_ref()
        .ok_or_else(|| "--inspect-checkpoint requires a path".to_string())?;
    let checkpoint = read_campaign_checkpoint_v1(path)?;
    let report = input
        .report_path
        .as_ref()
        .map(read_campaign_report_v1)
        .transpose()?;
    let mut matches = Vec::new();
    for entry in checkpoint.sessions {
        let session = entry
            .session
            .clone()
            .into_session()
            .map_err(|err| format!("failed to restore checkpoint session: {err}"))?;
        if !checkpoint_session_matches_filters(&input.filters, &session) {
            continue;
        }
        matches.push((entry.commands, session));
    }
    if matches.is_empty() {
        return Err(format!(
            "no checkpoint sessions matched filters act={:?} floor={:?} boundary={:?} hp={:?}",
            input.filters.act, input.filters.floor, input.filters.boundary, input.filters.hp
        ));
    }
    if input.summary {
        if let Some(inspect_index) = input.filters.index {
            if inspect_index >= matches.len() {
                return Err(format!(
                    "--inspect-index {} is out of range for {} matching checkpoint session(s)",
                    inspect_index,
                    matches.len()
                ));
            }
            let selected = vec![matches.swap_remove(inspect_index)];
            println!(
                "{}",
                inspect_summary::render_checkpoint_inspect_summary_v1(
                    checkpoint.seed,
                    &selected,
                    report.as_ref(),
                    input.branch_examples,
                )
            );
            return Ok(());
        }
        println!(
            "{}",
            inspect_summary::render_checkpoint_inspect_summary_v1(
                checkpoint.seed,
                &matches,
                report.as_ref(),
                input.branch_examples,
            )
        );
        return Ok(());
    }
    let inspect_index = input.filters.index.unwrap_or(0);
    if inspect_index >= matches.len() {
        return Err(format!(
            "--inspect-index {} is out of range for {} matching checkpoint session(s)",
            inspect_index,
            matches.len()
        ));
    }

    let match_count = matches.len();
    let (commands, mut session) = matches.swap_remove(inspect_index);
    let (hp, max_hp) = inspect_visible_player_hp(&session);
    let surface = build_decision_surface(&session);
    println!(
        "Checkpoint inspection: seed={} match={}/{} act={} floor={} hp={}/{} boundary={}",
        checkpoint.seed,
        inspect_index + 1,
        match_count,
        session.run_state.act_num,
        session.run_state.floor_num,
        hp,
        max_hp,
        surface.view.header.title
    );
    println!("commands: {}", render_inspect_command_path(&commands));
    if input.modes.shop_evidence {
        println!("{}", render_checkpoint_shop_evidence_v1(&session)?);
    } else if input.modes.shop_challenge {
        println!(
            "{}",
            render_checkpoint_shop_plan_challenge_v1(
                checkpoint.seed,
                &session,
                &input.shop_challenge
            )?
        );
    } else if input.modes.card_reward_evidence {
        println!("{}", render_checkpoint_card_reward_evidence_v1(&session)?);
    } else if input.modes.campfire_evidence {
        println!("{}", render_checkpoint_campfire_evidence_v1(&session)?);
    } else if input.modes.deck_mutation {
        println!("{}", render_checkpoint_deck_mutation_v1(&session)?);
    } else if input.modes.route_evidence {
        println!("{}", render_checkpoint_route_evidence_v1(&session)?);
    } else if input.modes.last_auto_combat {
        print!(
            "{}",
            render_last_auto_combat_checkpoint_inspection_v1(
                checkpoint.seed,
                inspect_index,
                match_count,
                &session,
                &commands,
            )?
        );
    } else if input.modes.combat_lab {
        let branch = report_branch_for_commands_v1(report.as_ref(), &commands);
        println!(
            "{}",
            combat_lab::render_checkpoint_combat_lab_v1(
                checkpoint.seed,
                inspect_index,
                match_count,
                &session,
                &commands,
                branch,
                &input.search_options,
                input.modes.probe_boss,
            )
        );
    } else if input.modes.search {
        let outcome = session.apply_command(RunControlCommand::SearchCombat(
            input.search_options.clone(),
        ))?;
        println!("{}", outcome.message);
    } else {
        println!("{}", render_run_control_details(&session));
        println!();
        println!("{}", render_run_control_state(&session));
    }
    Ok(())
}

fn report_branch_for_commands_v1<'a>(
    report: Option<&'a BranchCampaignReportV1>,
    commands: &[String],
) -> Option<&'a sts_simulator::eval::branch_campaign::BranchCampaignBranchV1> {
    let report = report?;
    report
        .active
        .iter()
        .chain(report.frozen.iter())
        .chain(report.abandoned.iter())
        .chain(report.stuck.iter())
        .chain(report.victories.iter())
        .chain(report.dead.iter())
        .find(|branch| branch.commands == commands)
}

fn checkpoint_session_matches_filters(
    filters: &InspectFiltersInput,
    session: &RunControlSession,
) -> bool {
    if filters
        .act
        .is_some_and(|act| session.run_state.act_num != act)
    {
        return false;
    }
    if filters
        .floor
        .is_some_and(|floor| session.run_state.floor_num != floor)
    {
        return false;
    }
    if filters
        .hp
        .is_some_and(|hp| inspect_visible_player_hp(session).0 != hp)
    {
        return false;
    }
    if let Some(boundary) = filters.boundary.as_ref() {
        let expected = normalized_inspect_boundary_title_v1(boundary);
        let actual = normalized_inspect_boundary_title_v1(
            &build_decision_surface(session).view.header.title,
        );
        if actual != expected {
            return false;
        }
    }
    true
}

fn normalized_inspect_boundary_title_v1(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn inspect_visible_player_hp(session: &RunControlSession) -> (i32, i32) {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp))
}

fn render_inspect_command_path(commands: &[String]) -> String {
    const HEAD: usize = 4;
    const TAIL: usize = 6;
    if commands.is_empty() {
        return "-".to_string();
    }
    if commands.len() <= HEAD + TAIL + 1 {
        return commands.join(" -> ");
    }
    let mut parts = Vec::new();
    parts.extend(commands.iter().take(HEAD).cloned());
    parts.push(format!("... {} more ...", commands.len() - HEAD - TAIL));
    parts.extend(commands.iter().skip(commands.len() - TAIL).cloned());
    parts.join(" -> ")
}
