use std::collections::{BTreeMap, BTreeSet};

use sts_simulator::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentReportV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionSlotV1;

pub(super) fn render_compact_report(report: &BranchExperimentReportV1) -> String {
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
    if let Some(path) = report.replay_trace_path.as_ref() {
        lines.push(format!(
            "replay trace: {} applied_steps={} stop={}",
            path,
            report.replay_trace_applied_steps,
            report
                .replay_trace_stop
                .as_deref()
                .unwrap_or("not_recorded")
        ));
    }
    let primary_retention_slots = report
        .branches
        .iter()
        .map(|branch| branch.retention.primary_slot)
        .collect::<Vec<_>>();
    if let Some(line) = render_primary_retention_slot_count_line(&primary_retention_slots) {
        lines.push(line);
    }
    let first_pick_outcomes = first_pick_outcome_summary_lines(report);
    if !first_pick_outcomes.is_empty() {
        lines.push("".to_string());
        lines.push("First-pick outcomes:".to_string());
        for line in first_pick_outcomes.iter().take(8) {
            lines.push(line.clone());
        }
        if first_pick_outcomes.len() > 8 {
            lines.push(format!(
                "  ... {} more first-pick outcome group(s); use --json or --out for full detail",
                first_pick_outcomes.len() - 8
            ));
        }
    }
    if report.explored_branch_points == 0 {
        let boundary = report
            .frontier_groups
            .first()
            .map(|group| group.boundary_title.as_str())
            .unwrap_or("unknown boundary");
        lines.push(format!(
            "no card reward branch point reached before {boundary}; provide --prefix commands or start from a trace/goto point that reaches a card reward"
        ));
    }
    if !report.reward_option_portfolios.is_empty() {
        lines.push("".to_string());
        lines.push("Reward option portfolios:".to_string());
        let summarized = summarize_reward_option_portfolios(&report.reward_option_portfolios);
        for summary in summarized.iter().take(4) {
            lines.push(format!(
                "  {} depth {}: kept {}/{} kept=[{}] pruned=[{}]{}",
                summary.boundary_title,
                summary.depth,
                summary.selected_count,
                summary.original_count,
                summary.kept,
                summary.pruned,
                if summary.count > 1 {
                    format!(" x{}", summary.count)
                } else {
                    String::new()
                },
            ));
        }
        if summarized.len() > 4 {
            lines.push(format!(
                "  ... {} more reward option portfolio event(s); use --json or --out for full detail",
                summarized.len() - 4
            ));
        }
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
        let first_picks = render_group_first_picks(report, &group.key);
        let lineage = render_lineage_flags(&group.lineage_flags);
        lines.push(format!(
            "  {:>2} branch(es) | {} | first_picks=[{}] | example: {} | next_reward=[{}]{}",
            group.branch_count, group.boundary_title, first_picks, example, offer, lineage
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
    for branch in ordered_branch_examples(report).into_iter().take(10) {
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

#[derive(Clone, Debug)]
struct RewardOptionPortfolioSummary {
    count: usize,
    depth: usize,
    boundary_title: String,
    original_count: usize,
    selected_count: usize,
    kept: String,
    pruned: String,
}

fn summarize_reward_option_portfolios(
    portfolios: &[BranchExperimentRewardOptionPortfolioV1],
) -> Vec<RewardOptionPortfolioSummary> {
    let mut groups = BTreeMap::<String, RewardOptionPortfolioSummary>::new();
    for portfolio in portfolios {
        let kept = render_reward_option_entries(&portfolio.selected_options);
        let pruned = render_reward_option_entries(&portfolio.pruned_options);
        let key = format!(
            "{}|{}|{}|{}|{}|{}",
            portfolio.boundary_title,
            portfolio.depth,
            portfolio.original_count,
            portfolio.selected_count,
            kept,
            pruned
        );
        groups
            .entry(key)
            .and_modify(|summary| summary.count += 1)
            .or_insert_with(|| RewardOptionPortfolioSummary {
                count: 1,
                depth: portfolio.depth,
                boundary_title: portfolio.boundary_title.clone(),
                original_count: portfolio.original_count,
                selected_count: portfolio.selected_count,
                kept,
                pruned,
            });
    }
    groups.into_values().collect()
}

fn render_reward_option_entries(
    entries: &[BranchExperimentRewardOptionPortfolioEntryV1],
) -> String {
    if entries.is_empty() {
        return "-".to_string();
    }
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.label, entry.semantic_class))
        .collect::<Vec<_>>()
        .join(", ")
}

#[derive(Clone, Debug)]
struct FirstPickOutcomeSummary {
    label: String,
    branch_count: usize,
    deepest_act: u8,
    deepest_floor: i32,
    min_hp: i32,
    max_hp: i32,
    primary_slots: BTreeMap<BranchRetentionSlotV1, usize>,
    package_states: BTreeMap<String, usize>,
    frontiers: BTreeMap<String, usize>,
}

fn first_pick_outcome_summary_lines(report: &BranchExperimentReportV1) -> Vec<String> {
    let mut summaries = BTreeMap::<String, FirstPickOutcomeSummary>::new();
    for branch in &report.branches {
        let Some(first_choice) = branch.choices.first() else {
            continue;
        };
        let entry = summaries
            .entry(first_choice.label.clone())
            .or_insert_with(|| FirstPickOutcomeSummary {
                label: first_choice.label.clone(),
                branch_count: 0,
                deepest_act: branch.summary.act,
                deepest_floor: branch.summary.floor,
                min_hp: branch.summary.hp,
                max_hp: branch.summary.hp,
                primary_slots: BTreeMap::new(),
                package_states: BTreeMap::new(),
                frontiers: BTreeMap::new(),
            });
        entry.branch_count += 1;
        if (branch.summary.act, branch.summary.floor) > (entry.deepest_act, entry.deepest_floor) {
            entry.deepest_act = branch.summary.act;
            entry.deepest_floor = branch.summary.floor;
        }
        entry.min_hp = entry.min_hp.min(branch.summary.hp);
        entry.max_hp = entry.max_hp.max(branch.summary.hp);
        *entry
            .primary_slots
            .entry(branch.retention.primary_slot)
            .or_default() += 1;
        for state in branch_package_state_tags(branch) {
            *entry.package_states.entry(state).or_default() += 1;
        }
        *entry
            .frontiers
            .entry(branch.summary.boundary_title.clone())
            .or_default() += 1;
    }

    let mut summaries = summaries.into_values().collect::<Vec<_>>();
    summaries.sort_by(|left, right| {
        right
            .branch_count
            .cmp(&left.branch_count)
            .then_with(|| {
                (right.deepest_act, right.deepest_floor)
                    .cmp(&(left.deepest_act, left.deepest_floor))
            })
            .then_with(|| right.max_hp.cmp(&left.max_hp))
            .then_with(|| left.label.cmp(&right.label))
    });
    summaries.iter().map(render_first_pick_outcome).collect()
}

fn render_first_pick_outcome(summary: &FirstPickOutcomeSummary) -> String {
    format!(
        "  {} | branches={} deepest=A{}F{} hp={} | primary=[{}] | packages=[{}] | frontiers=[{}]",
        summary.label,
        summary.branch_count,
        summary.deepest_act,
        summary.deepest_floor,
        render_hp_range(summary.min_hp, summary.max_hp),
        render_primary_slot_counts(&summary.primary_slots),
        render_package_state_counts(&summary.package_states),
        render_string_count_map(&summary.frontiers)
    )
}

fn branch_package_state_tags(branch: &BranchExperimentBranchReportV1) -> Vec<String> {
    let setup_keys = branch
        .summary
        .trajectory
        .setup_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let package_keys = branch
        .summary
        .trajectory
        .package_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut tags = Vec::new();
    for key in setup_keys.intersection(&package_keys) {
        tags.push(format!("closed:{key}"));
    }
    for key in setup_keys.difference(&package_keys) {
        tags.push(format!("open:{key}"));
    }
    for key in package_keys.difference(&setup_keys) {
        tags.push(format!("payoff_only:{key}"));
    }
    tags
}

fn render_hp_range(min_hp: i32, max_hp: i32) -> String {
    if min_hp == max_hp {
        min_hp.to_string()
    } else {
        format!("{min_hp}-{max_hp}")
    }
}

fn render_package_state_counts(counts: &BTreeMap<String, usize>) -> String {
    if counts.is_empty() {
        return "-".to_string();
    }
    render_string_count_map(counts)
}

fn render_primary_slot_counts(counts: &BTreeMap<BranchRetentionSlotV1, usize>) -> String {
    RETENTION_SLOT_DISPLAY_ORDER
        .iter()
        .filter_map(|slot| {
            counts
                .get(slot)
                .filter(|count| **count > 0)
                .map(|count| format!("{}={count}", retention_slot_name(*slot)))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_string_count_map(counts: &BTreeMap<String, usize>) -> String {
    counts
        .iter()
        .map(|(label, count)| format!("{label}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn ordered_branch_examples(
    report: &BranchExperimentReportV1,
) -> Vec<&BranchExperimentBranchReportV1> {
    let mut ordered = Vec::new();
    let mut used_indices = BTreeSet::new();
    let mut covered_frontier_and_first_pick = BTreeSet::new();

    for (index, branch) in report.branches.iter().enumerate() {
        let key = branch_example_diversity_key(branch);
        if covered_frontier_and_first_pick.insert(key) {
            ordered.push(branch);
            used_indices.insert(index);
        }
    }

    for (index, branch) in report.branches.iter().enumerate() {
        if used_indices.insert(index) {
            ordered.push(branch);
        }
    }
    ordered
}

fn branch_example_diversity_key(branch: &BranchExperimentBranchReportV1) -> String {
    let first_pick = branch
        .choices
        .first()
        .map(|choice| choice.label.as_str())
        .unwrap_or("-");
    format!("{}|{first_pick}", branch.frontier.key)
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
    let retention = render_retention_slots(&branch.retention.slots);
    let formation = render_formation_summary(branch);
    let trajectory = render_trajectory_summary(branch);
    format!(
        "  A{}F{} HP {}/{} gold {} | {}{} | {} | {} | keep=[{}] | choices: {} | next_reward=[{}]",
        branch.summary.act,
        branch.summary.floor,
        branch.summary.hp,
        branch.summary.max_hp,
        branch.summary.gold,
        branch.summary.boundary_title,
        status,
        formation,
        trajectory,
        retention,
        choices,
        next_reward
    )
}

fn render_formation_summary(branch: &BranchExperimentBranchReportV1) -> String {
    let strengths = branch
        .summary
        .formation_strengths
        .iter()
        .map(|strength| format!("{strength:?}"))
        .collect::<Vec<_>>();
    let needs = branch
        .summary
        .formation_needs
        .iter()
        .map(|need| format!("{need:?}"))
        .collect::<Vec<_>>();
    let strengths = if strengths.is_empty() {
        "-".to_string()
    } else {
        strengths.join("+")
    };
    let needs = if needs.is_empty() {
        "-".to_string()
    } else {
        needs.join("+")
    };
    format!(
        "formation={:?} strengths={} needs={}",
        branch.summary.formation_stage, strengths, needs
    )
}

fn render_trajectory_summary(branch: &BranchExperimentBranchReportV1) -> String {
    let trajectory = &branch.summary.trajectory;
    let setups = if trajectory.setup_keys.is_empty() {
        "-".to_string()
    } else {
        trajectory.setup_keys.join("+")
    };
    let packages = if trajectory.package_keys.is_empty() {
        "-".to_string()
    } else {
        trajectory.package_keys.join("+")
    };
    format!(
        "traj=setup:{} pkg:{} trans:{} eng:{}/{} def:{} de:{}",
        setups,
        packages,
        trajectory.transition_frontload_picks,
        trajectory.engine_generator_picks,
        trajectory.engine_payoff_picks,
        trajectory.defense_picks,
        trajectory.draw_energy_picks
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

fn render_group_first_picks(report: &BranchExperimentReportV1, group_key: &str) -> String {
    let mut picks = Vec::<String>::new();
    for branch in &report.branches {
        if branch.frontier.key != group_key {
            continue;
        }
        let Some(choice) = branch.choices.first() else {
            continue;
        };
        if !picks.contains(&choice.label) {
            picks.push(choice.label.clone());
        }
        if picks.len() >= 5 {
            break;
        }
    }
    if picks.is_empty() {
        "-".to_string()
    } else {
        picks.join(", ")
    }
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

fn render_retention_slots(slots: &[BranchRetentionSlotV1]) -> String {
    slots
        .iter()
        .map(|slot| retention_slot_name(*slot))
        .collect::<Vec<_>>()
        .join(",")
}

fn render_primary_retention_slot_count_line(slots: &[BranchRetentionSlotV1]) -> Option<String> {
    let mut counts = BTreeMap::<BranchRetentionSlotV1, usize>::new();
    for slot in slots {
        *counts.entry(*slot).or_default() += 1;
    }
    if counts.is_empty() {
        return None;
    }
    let rendered = RETENTION_SLOT_DISPLAY_ORDER
        .iter()
        .filter_map(|slot| {
            counts
                .get(slot)
                .filter(|count| **count > 0)
                .map(|count| format!("{}={count}", retention_slot_name(*slot)))
        })
        .collect::<Vec<_>>();
    Some(format!("Primary retention slots: {}", rendered.join(" ")))
}

const RETENTION_SLOT_DISPLAY_ORDER: [BranchRetentionSlotV1; 8] = [
    BranchRetentionSlotV1::Package,
    BranchRetentionSlotV1::EngineSetup,
    BranchRetentionSlotV1::Scaling,
    BranchRetentionSlotV1::DefenseEngine,
    BranchRetentionSlotV1::Survival,
    BranchRetentionSlotV1::Frontload,
    BranchRetentionSlotV1::CleanDeck,
    BranchRetentionSlotV1::Diversity,
];

fn retention_slot_name(slot: BranchRetentionSlotV1) -> &'static str {
    match slot {
        BranchRetentionSlotV1::Package => "package",
        BranchRetentionSlotV1::EngineSetup => "engine_setup",
        BranchRetentionSlotV1::Scaling => "scaling",
        BranchRetentionSlotV1::DefenseEngine => "defense",
        BranchRetentionSlotV1::Survival => "survival",
        BranchRetentionSlotV1::Frontload => "frontload",
        BranchRetentionSlotV1::CleanDeck => "clean",
        BranchRetentionSlotV1::Diversity => "diversity",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::noncombat_strategy_v1::StrategyDeckFormationStageV1;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::branch_experiment::{
        BranchExperimentChoiceV1, BranchExperimentFrontierV1, BranchExperimentLineageV1,
        BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRunSummaryV1,
    };
    use sts_simulator::eval::branch_experiment_retention::BranchRetentionDecisionV1;

    #[test]
    fn compact_report_is_human_sized() {
        let report = empty_report();

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("BranchExperimentV1 seed=1"));
        assert!(!rendered.trim_start().starts_with('{'));
    }

    #[test]
    fn compact_report_summarizes_reward_option_portfolio_pruning() {
        let mut report = empty_report();
        report.reward_option_portfolios = vec![portfolio_event(
            0,
            "Reward Screen",
            vec![
                portfolio_entry("rp 0", "Shrug It Off", "stabilizer:Block+CardDraw"),
                portfolio_entry("rp 2", "Body Slam", "payoff:block_engine"),
            ],
            vec![portfolio_entry(
                "rp 1",
                "Cleave",
                "pure_transition_frontload",
            )],
        )];

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("Reward option portfolios:"));
        assert!(rendered.contains("Reward Screen depth 0: kept 2/3"));
        assert!(rendered.contains("pruned=[Cleave:pure_transition_frontload]"));
    }

    #[test]
    fn compact_report_groups_duplicate_reward_option_portfolio_events() {
        let event = portfolio_event(
            1,
            "Card Reward",
            vec![portfolio_entry(
                "rp 0",
                "Clash",
                "pure_transition_frontload",
            )],
            vec![portfolio_entry(
                "rp 1",
                "Twin Strike",
                "pure_transition_frontload",
            )],
        );
        let mut duplicate = event.clone();
        duplicate.frontier_key = "frontier-b".to_string();
        let mut report = empty_report();
        report.reward_option_portfolios = vec![event, duplicate];

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("x2"));
        assert_eq!(
            rendered
                .matches("Twin Strike:pure_transition_frontload")
                .count(),
            1
        );
    }

    #[test]
    fn compact_report_explains_when_no_card_reward_branch_point_was_reached() {
        let mut report = empty_report();
        report.explored_branch_points = 0;
        report.frontier_groups = vec![
            sts_simulator::eval::branch_experiment::BranchExperimentFrontierGroupV1 {
                key: "neow".to_string(),
                branch_count: 1,
                representative_branch_id: "root".to_string(),
                boundary_title: "Neow Bonus".to_string(),
                next_card_reward_offer: None,
                lineage_flags: Vec::new(),
            },
        ];

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("no card reward branch point reached"));
        assert!(rendered.contains("--prefix"));
    }

    #[test]
    fn compact_report_summarizes_replayed_trace_prefix() {
        let mut report = empty_report();
        report.replay_trace_path = Some("tools/artifacts/traces/seed.trace.json".to_string());
        report.replay_trace_applied_steps = 7;
        report.replay_trace_stop = Some("TraceEnd".to_string());

        let rendered = render_compact_report(&report);

        assert!(rendered.contains("replay trace:"));
        assert!(rendered.contains("applied_steps=7"));
        assert!(rendered.contains("stop=TraceEnd"));
    }

    #[test]
    fn compact_report_renders_engine_setup_retention_slot() {
        let rendered = render_retention_slots(&[
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Diversity,
        ]);

        assert_eq!(rendered, "engine_setup,diversity");
    }

    #[test]
    fn primary_retention_slot_count_line_summarizes_kept_branch_shape() {
        let rendered = render_primary_retention_slot_count_line(&[
            BranchRetentionSlotV1::Package,
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Frontload,
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Frontload,
        ])
        .expect("slot count line");

        assert_eq!(
            rendered,
            "Primary retention slots: package=1 engine_setup=2 frontload=2"
        );
    }

    #[test]
    fn first_pick_outcome_summary_groups_kept_branch_results() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                branch_report(
                    "b0",
                    "Shockwave",
                    1,
                    4,
                    70,
                    BranchRetentionSlotV1::Package,
                    "Combat",
                ),
                branch_report(
                    "b1",
                    "Shockwave",
                    1,
                    6,
                    60,
                    BranchRetentionSlotV1::EngineSetup,
                    "Campfire",
                ),
                branch_report(
                    "b2",
                    "Armaments",
                    1,
                    5,
                    76,
                    BranchRetentionSlotV1::DefenseEngine,
                    "Combat",
                ),
            ],
            ..empty_report()
        };

        let lines = first_pick_outcome_summary_lines(&report);

        assert_eq!(
            lines[0],
            "  Shockwave | branches=2 deepest=A1F6 hp=60-70 | primary=[package=1 engine_setup=1] | packages=[-] | frontiers=[Campfire=1 Combat=1]"
        );
        assert_eq!(
            lines[1],
            "  Armaments | branches=1 deepest=A1F5 hp=76 | primary=[defense=1] | packages=[-] | frontiers=[Combat=1]"
        );
    }

    #[test]
    fn first_pick_outcome_summary_includes_generic_package_state() {
        let report = BranchExperimentReportV1 {
            branches: vec![
                branch_report_with_packages(
                    "b0",
                    "Body Slam",
                    &["block_engine", "exhaust_engine"],
                    &["block_engine", "upgrade_sink"],
                ),
                branch_report_with_packages("b1", "Body Slam", &["exhaust_engine"], &[]),
            ],
            ..empty_report()
        };

        let lines = first_pick_outcome_summary_lines(&report);

        assert_eq!(
            lines[0],
            "  Body Slam | branches=2 deepest=A1F5 hp=70 | primary=[package=2] | packages=[closed:block_engine=1 open:exhaust_engine=2 payoff_only:upgrade_sink=1] | frontiers=[Combat=2]"
        );
    }

    fn empty_report() -> BranchExperimentReportV1 {
        BranchExperimentReportV1 {
            schema_name: "BranchExperimentV1".to_string(),
            schema_version: 7,
            label_role: "diagnostic_not_teacher_label".to_string(),
            policy_quality_claim: false,
            seed: 1,
            replay_trace_path: None,
            replay_trace_applied_steps: 0,
            replay_trace_stop: None,
            max_branches: 4,
            max_depth: 1,
            explored_branch_points: 1,
            branch_limit_hit: false,
            frontier_group_limit_hit: false,
            wall_limit_hit: false,
            elapsed_wall_ms: 0,
            pruned_branch_count: 0,
            reward_option_portfolios: Vec::new(),
            frontier_groups: Vec::new(),
            branches: Vec::new(),
        }
    }

    fn portfolio_event(
        depth: usize,
        boundary_title: &str,
        selected_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
        pruned_options: Vec<BranchExperimentRewardOptionPortfolioEntryV1>,
    ) -> BranchExperimentRewardOptionPortfolioV1 {
        BranchExperimentRewardOptionPortfolioV1 {
            depth,
            frontier_key: "frontier".to_string(),
            boundary_title: boundary_title.to_string(),
            max_reward_options_per_branch: 2,
            original_count: selected_options.len() + pruned_options.len(),
            selected_count: selected_options.len(),
            selected_options,
            pruned_options,
        }
    }

    fn portfolio_entry(
        command: &str,
        label: &str,
        semantic_class: &str,
    ) -> BranchExperimentRewardOptionPortfolioEntryV1 {
        BranchExperimentRewardOptionPortfolioEntryV1 {
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: semantic_class.to_string(),
        }
    }

    fn branch_report(
        branch_id: &str,
        first_pick: &str,
        act: u8,
        floor: i32,
        hp: i32,
        primary_slot: BranchRetentionSlotV1,
        boundary_title: &str,
    ) -> BranchExperimentBranchReportV1 {
        BranchExperimentBranchReportV1 {
            branch_id: branch_id.to_string(),
            status: BranchExperimentBranchStatusV1::Active,
            rank_key: 0,
            retention: BranchRetentionDecisionV1 {
                primary_slot,
                slots: vec![primary_slot],
                reasons: Vec::new(),
            },
            choices: vec![BranchExperimentChoiceV1 {
                depth: 0,
                kind: "card_reward".to_string(),
                card: CardId::Strike,
                upgrades: 0,
                label: first_pick.to_string(),
                command: "rp 0".to_string(),
            }],
            stop_reason: boundary_title.to_string(),
            summary: BranchExperimentRunSummaryV1 {
                act,
                floor,
                hp,
                max_hp: 80,
                gold: 120,
                deck_count: 12,
                relic_count: 1,
                potion_count: 0,
                formation_stage: StrategyDeckFormationStageV1::PlanSeeded,
                formation_needs: Vec::new(),
                formation_strengths: Vec::new(),
                trajectory: Default::default(),
                boundary_title: boundary_title.to_string(),
            },
            frontier: BranchExperimentFrontierV1 {
                key: format!("A{act}F{floor}:{boundary_title}"),
                act,
                floor,
                boundary_title: boundary_title.to_string(),
                card_rng_counter: 0,
                card_blizz_randomizer: 0,
                next_card_reward_offer: None,
                lineage: BranchExperimentLineageV1 {
                    visibility: "test".to_string(),
                    public_policy_input: false,
                    direct_pick_consumes_card_rng: false,
                    same_reward_offer_lineage_key: "test".to_string(),
                    reward_screen_context: "test".to_string(),
                    reward_count_modifiers: Vec::new(),
                    card_pool_modifiers: Vec::new(),
                    rarity_modifiers: Vec::new(),
                    preview_modifiers: Vec::new(),
                    sequence_breakers_present: Vec::new(),
                },
            },
        }
    }

    fn branch_report_with_packages(
        branch_id: &str,
        first_pick: &str,
        setup_keys: &[&str],
        package_keys: &[&str],
    ) -> BranchExperimentBranchReportV1 {
        let mut branch = branch_report(
            branch_id,
            first_pick,
            1,
            5,
            70,
            BranchRetentionSlotV1::Package,
            "Combat",
        );
        branch.summary.trajectory.setup_keys =
            setup_keys.iter().map(|key| key.to_string()).collect();
        branch.summary.trajectory.package_keys =
            package_keys.iter().map(|key| key.to_string()).collect();
        branch
    }
}
