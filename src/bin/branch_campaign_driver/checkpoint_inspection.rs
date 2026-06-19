use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::run_control::{
    build_decision_surface, render_run_control_details, render_run_control_state,
    RunControlCommand, RunControlSearchCombatOptions, RunControlSession,
};

use super::checkpoint_evidence::{
    render_checkpoint_campfire_evidence_v1, render_checkpoint_card_reward_evidence_v1,
    render_checkpoint_deck_mutation_v1, render_checkpoint_route_evidence_v1,
    render_checkpoint_shop_evidence_v1,
};
use super::final_boss_combat::{
    render_final_boss_combat_report_inspection_v1, render_last_auto_combat_checkpoint_inspection_v1,
};
use super::shop_challenge::render_checkpoint_shop_plan_challenge_v1;
use super::{
    combat_lab, inspect_summary, parse_hp_loss_limit, read_campaign_checkpoint_v1,
    read_campaign_report_v1, Args,
};

pub(super) fn run_final_boss_combat_report_inspection(args: &Args) -> Result<(), String> {
    let path = args
        .inspect_report
        .as_ref()
        .ok_or_else(|| "--inspect-final-boss-combat requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(path)?;
    print!(
        "{}",
        render_final_boss_combat_report_inspection_v1(&report, args.inspect_index.unwrap_or(0))?
    );
    Ok(())
}

pub(super) fn run_checkpoint_inspection(args: &Args) -> Result<(), String> {
    let path = args
        .inspect_checkpoint
        .as_ref()
        .ok_or_else(|| "--inspect-checkpoint requires a path".to_string())?;
    let checkpoint = read_campaign_checkpoint_v1(path)?;
    let report = args
        .inspect_report
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
        if !checkpoint_session_matches_filters(args, &session) {
            continue;
        }
        matches.push((entry.commands, session));
    }
    if matches.is_empty() {
        return Err(format!(
            "no checkpoint sessions matched filters act={:?} floor={:?} boundary={:?} hp={:?}",
            args.inspect_act, args.inspect_floor, args.inspect_boundary, args.inspect_hp
        ));
    }
    if args.inspect_summary {
        if let Some(inspect_index) = args.inspect_index {
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
                    args.branch_examples,
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
                args.branch_examples,
            )
        );
        return Ok(());
    }
    let inspect_index = args.inspect_index.unwrap_or(0);
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
    if args.inspect_shop_evidence {
        println!("{}", render_checkpoint_shop_evidence_v1(&session)?);
    } else if args.challenge_shop_plans {
        println!(
            "{}",
            render_checkpoint_shop_plan_challenge_v1(checkpoint.seed, &session, args)?
        );
    } else if args.inspect_card_reward_evidence {
        println!("{}", render_checkpoint_card_reward_evidence_v1(&session)?);
    } else if args.inspect_campfire_evidence {
        println!("{}", render_checkpoint_campfire_evidence_v1(&session)?);
    } else if args.inspect_deck_mutation {
        println!("{}", render_checkpoint_deck_mutation_v1(&session)?);
    } else if args.inspect_route_evidence {
        println!("{}", render_checkpoint_route_evidence_v1(&session)?);
    } else if args.inspect_last_auto_combat {
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
    } else if args.inspect_combat_lab {
        let options = inspect_search_options_from_args(args)?;
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
                &options,
                args.probe_boss,
            )
        );
    } else if args.inspect_search {
        let options = inspect_search_options_from_args(args)?;
        let outcome = session.apply_command(RunControlCommand::SearchCombat(options))?;
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

fn checkpoint_session_matches_filters(args: &Args, session: &RunControlSession) -> bool {
    if args
        .inspect_act
        .is_some_and(|act| session.run_state.act_num != act)
    {
        return false;
    }
    if args
        .inspect_floor
        .is_some_and(|floor| session.run_state.floor_num != floor)
    {
        return false;
    }
    if args
        .inspect_hp
        .is_some_and(|hp| inspect_visible_player_hp(session).0 != hp)
    {
        return false;
    }
    if let Some(boundary) = args.inspect_boundary.as_ref() {
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

pub(super) fn inspect_search_options_from_args(
    args: &Args,
) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    options.max_nodes = args.search_max_nodes.or(options.max_nodes);
    options.wall_ms = options.wall_ms.or(Some(args.search_wall_ms));
    options.max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?.or(options.max_hp_loss);
    Ok(options)
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
