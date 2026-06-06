use std::fs;
use std::path::PathBuf;

use clap::Parser;
use sts_simulator::eval::branch_experiment::{
    run_branch_experiment_v1, BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1,
    BranchExperimentConfigV1, BranchExperimentReportV1,
};
use sts_simulator::eval::run_control::RunControlHpLossLimit;

#[derive(Debug, Parser)]
#[command(
    name = "branch_experiment_driver",
    about = "Run a small in-memory branch experiment over card reward choices"
)]
struct Args {
    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long)]
    final_act: bool,

    #[arg(long, default_value_t = 12)]
    max_branches: usize,

    #[arg(long)]
    max_per_frontier_group: Option<usize>,

    #[arg(long, default_value_t = 4)]
    max_depth: usize,

    #[arg(long, default_value_t = 128)]
    auto_max_ops: usize,

    #[arg(long)]
    experiment_wall_ms: Option<u64>,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long)]
    search_wall_ms: Option<u64>,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(long)]
    include_skip: bool,

    #[arg(long)]
    out: Option<PathBuf>,

    #[arg(long)]
    json: bool,
}

fn main() {
    let args = Args::parse();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
        seed: args.seed,
        ascension_level: args.ascension,
        player_class,
        final_act: args.final_act,
        max_branches: args.max_branches,
        max_branches_per_frontier_group: args.max_per_frontier_group,
        max_depth: args.max_depth,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: args.experiment_wall_ms,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms.or(Some(100)),
        search_max_hp_loss: parse_hp_loss_limit(args.max_hp_loss.as_deref())?,
        include_skip: args.include_skip,
        prefix_commands: args.prefix_commands,
    })?;
    if let Some(path) = args.out {
        let payload = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, payload)
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("{}", render_compact_report(&report));
        println!("full JSON written: {}", path.display());
    } else if args.json {
        let payload = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
        println!("{payload}");
    } else {
        println!("{}", render_compact_report(&report));
    }
    Ok(())
}

fn render_compact_report(report: &BranchExperimentReportV1) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "BranchExperimentV1 seed={} depth={} max_branches={} branch_points={} kept={} pruned={} groups={} elapsed={}ms",
        report.seed,
        report.max_depth,
        report.max_branches,
        report.explored_branch_points,
        report.branches.len(),
        report.pruned_branch_count,
        report.frontier_groups.len(),
        report.elapsed_wall_ms
    ));
    if report.branch_limit_hit {
        lines.push("branch limit hit: increase --max-branches or reduce --max-depth".to_string());
    }
    if report.frontier_group_limit_hit {
        lines.push(
            "frontier group cap hit: increase --max-per-frontier-group to keep more variants per same frontier"
                .to_string(),
        );
    }
    if report.wall_limit_hit {
        lines.push(
            "experiment wall-clock limit hit: increase --experiment-wall-ms for more exploration"
                .to_string(),
        );
    }
    lines.push("".to_string());
    lines.push("Frontier groups:".to_string());
    for group in report.frontier_groups.iter().take(8) {
        let offer = group
            .next_card_reward_offer
            .as_ref()
            .map(|cards| cards.join(", "))
            .unwrap_or_else(|| "-".to_string());
        let example = report
            .branches
            .iter()
            .find(|branch| branch.branch_id == group.representative_branch_id)
            .map(render_choice_path)
            .unwrap_or_else(|| "-".to_string());
        let lineage = render_lineage_flags(&group.lineage_flags);
        lines.push(format!(
            "  {:>2} branch(es) | {} | example: {} | next_reward=[{}]{}",
            group.branch_count, group.boundary_title, example, offer, lineage
        ));
    }
    if report.frontier_groups.len() > 8 {
        lines.push(format!(
            "  ... {} more group(s); use --json or --out for full detail",
            report.frontier_groups.len() - 8
        ));
    }
    lines.push("".to_string());
    lines.push("Kept branch examples:".to_string());
    for branch in report.branches.iter().take(10) {
        lines.push(render_branch_line(branch));
    }
    if report.branches.len() > 10 {
        lines.push(format!(
            "  ... {} more branch(es); use --json or --out for full detail",
            report.branches.len() - 10
        ));
    }
    lines.join("\n")
}

fn render_branch_line(branch: &BranchExperimentBranchReportV1) -> String {
    let choices = render_choice_path(branch);
    let next_reward = branch
        .frontier
        .next_card_reward_offer
        .as_ref()
        .map(|cards| cards.join(", "))
        .unwrap_or_else(|| "-".to_string());
    let status = branch_status_suffix(branch.status);
    format!(
        "  A{}F{} HP {}/{} gold {} | {}{} | choices: {} | next_reward=[{}]",
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.gold,
        branch.summary.boundary_title,
        status,
        choices,
        next_reward
    )
}

fn render_choice_path(branch: &BranchExperimentBranchReportV1) -> String {
    if branch.choices.is_empty() {
        return "-".to_string();
    }
    branch
        .choices
        .iter()
        .map(|choice| choice.label.as_str())
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn branch_status_suffix(status: BranchExperimentBranchStatusV1) -> &'static str {
    match status {
        BranchExperimentBranchStatusV1::NeedsHumanBoundary => "",
        BranchExperimentBranchStatusV1::Active => " [active]",
        BranchExperimentBranchStatusV1::TerminalVictory => " [victory]",
        BranchExperimentBranchStatusV1::TerminalDefeat => " [defeat]",
        BranchExperimentBranchStatusV1::Failed => " [failed]",
        BranchExperimentBranchStatusV1::Pruned => " [pruned]",
    }
}

fn render_lineage_flags(flags: &[String]) -> String {
    if flags.is_empty() {
        return String::new();
    }
    format!(" | lineage_flags=[{}]", flags.join(", "))
}

fn parse_hp_loss_limit(value: Option<&str>) -> Result<Option<RunControlHpLossLimit>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.eq_ignore_ascii_case("off") || value.eq_ignore_ascii_case("unlimited") {
        return Ok(Some(RunControlHpLossLimit::Unlimited));
    }
    let limit = value
        .parse::<u32>()
        .map_err(|err| format!("invalid --max-hp-loss {value}: {err}"))?;
    Ok(Some(RunControlHpLossLimit::Limit(limit)))
}

fn canonical_player_class(value: &str) -> Result<&'static str, String> {
    match value.to_ascii_lowercase().as_str() {
        "ironclad" => Ok("Ironclad"),
        "silent" => Ok("Silent"),
        "defect" => Ok("Defect"),
        "watcher" => Ok("Watcher"),
        other => Err(format!(
            "unsupported class '{other}', expected ironclad|silent|defect|watcher"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unlimited_hp_loss_limit() {
        assert_eq!(
            parse_hp_loss_limit(Some("off")).expect("hp loss parses"),
            Some(RunControlHpLossLimit::Unlimited)
        );
    }

    #[test]
    fn canonicalizes_player_class() {
        assert_eq!(
            canonical_player_class("ironclad").expect("class parses"),
            "Ironclad"
        );
    }

    #[test]
    fn compact_report_is_human_sized() {
        let report = sts_simulator::eval::branch_experiment::BranchExperimentReportV1 {
            schema_name: "BranchExperimentV1".to_string(),
            schema_version: 1,
            label_role: "diagnostic_not_teacher_label".to_string(),
            policy_quality_claim: false,
            seed: 1,
            max_branches: 4,
            max_depth: 1,
            explored_branch_points: 1,
            branch_limit_hit: false,
            frontier_group_limit_hit: false,
            wall_limit_hit: false,
            elapsed_wall_ms: 0,
            pruned_branch_count: 0,
            frontier_groups: Vec::new(),
            branches: Vec::new(),
        };

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("BranchExperimentV1 seed=1"));
        assert!(!rendered.trim_start().starts_with('{'));
    }
}
