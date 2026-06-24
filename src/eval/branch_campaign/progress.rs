#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchCampaignProgressEventV1 {
    CampaignStarted {
        seed: u64,
        max_rounds: usize,
        round_depth: usize,
        max_scheduled: usize,
        max_parked: usize,
    },
    RoundStarted {
        round: usize,
        max_rounds: usize,
        scheduled_branches: usize,
        parked_branches: usize,
    },
    BranchStarted {
        round: usize,
        branch_index: usize,
        branch_count: usize,
        choices: String,
    },
    BranchFinished {
        round: usize,
        branch_index: usize,
        branch_count: usize,
        produced_branches: usize,
        explored_branch_points: usize,
        elapsed_wall_ms: u64,
        start_elapsed_wall_ms: u64,
        replay_start_source: Option<BranchCampaignReplayStartSourceV1>,
        replay_suffix_commands: usize,
        combat_budget_retry_used: bool,
        wall_limit_hit: bool,
        branch_limit_hit: bool,
    },
    RoundFinished {
        round: usize,
        started_scheduled: usize,
        produced_branches: usize,
        scheduled_after: usize,
        parked_added: usize,
        strategy_requests: usize,
    },
    CampaignFinished {
        stop_reason: String,
        scheduled: usize,
        parked: usize,
        victories: usize,
        stuck: usize,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchCampaignReplayStartSourceV1 {
    Exact,
    Ancestor,
}

impl BranchCampaignReplayStartSourceV1 {
    fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Ancestor => "ancestor",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchCampaignProgressDetailV1 {
    Summary,
    Verbose,
}

pub fn render_branch_campaign_progress_event_with_detail_v1(
    event: &BranchCampaignProgressEventV1,
    detail: BranchCampaignProgressDetailV1,
) -> Option<String> {
    match detail {
        BranchCampaignProgressDetailV1::Verbose => {
            Some(render_branch_campaign_progress_event_v1(event))
        }
        BranchCampaignProgressDetailV1::Summary => {
            render_branch_campaign_progress_summary_v1(event)
        }
    }
}

fn render_branch_campaign_progress_summary_v1(
    event: &BranchCampaignProgressEventV1,
) -> Option<String> {
    match event {
        BranchCampaignProgressEventV1::CampaignStarted {
            seed,
            max_rounds,
            round_depth,
            max_scheduled,
            max_parked,
        } => Some(format!(
            "campaign start: seed={seed} rounds={max_rounds} depth={round_depth} scheduled_cap={max_scheduled} parked_cap={max_parked}"
        )),
        BranchCampaignProgressEventV1::RoundStarted {
            round,
            max_rounds,
            scheduled_branches,
            parked_branches,
        } => Some(format!(
            "round {round}/{max_rounds}: scheduled={scheduled_branches} parked={parked_branches}"
        )),
        BranchCampaignProgressEventV1::BranchStarted { .. } => None,
        BranchCampaignProgressEventV1::BranchFinished {
            round,
            branch_index,
            branch_count,
            produced_branches,
            explored_branch_points,
            elapsed_wall_ms,
            combat_budget_retry_used,
            wall_limit_hit,
            branch_limit_hit,
            ..
        } => {
            if *elapsed_wall_ms < 5_000
                && !*combat_budget_retry_used
                && !*wall_limit_hit
                && !*branch_limit_hit
            {
                return None;
            }

            let mut extras = Vec::new();
            if *elapsed_wall_ms >= 5_000 {
                extras.push(format!("elapsed={}", format_progress_seconds_v1(*elapsed_wall_ms)));
            }
            if *combat_budget_retry_used {
                extras.push("retry=combat_budget".to_string());
            }
            let limits = render_progress_limits_v1(*branch_limit_hit, *wall_limit_hit);
            if !limits.is_empty() {
                extras.push(format!("limits=[{limits}]"));
            }
            let suffix = if extras.is_empty() {
                String::new()
            } else {
                format!(" | {}", extras.join(" "))
            };
            Some(format!(
                "round {round}: branch {branch_index}/{branch_count} done produced={produced_branches} branch_points={explored_branch_points}{suffix}"
            ))
        }
        BranchCampaignProgressEventV1::RoundFinished {
            round,
            produced_branches,
            scheduled_after,
            parked_added,
            strategy_requests,
            ..
        } => {
            let mut extras = Vec::new();
            if *parked_added > 0 {
                extras.push(format!("parked+={parked_added}"));
            }
            if *strategy_requests > 0 {
                extras.push(format!("strategy_requests={strategy_requests}"));
            }
            let suffix = if extras.is_empty() {
                String::new()
            } else {
                format!(" {}", extras.join(" "))
            };
            Some(format!(
                "round {round} done: candidates={produced_branches} scheduled={scheduled_after}{suffix}"
            ))
        }
        BranchCampaignProgressEventV1::CampaignFinished {
            stop_reason,
            scheduled,
            parked,
            victories,
            stuck,
        } => Some(format!(
            "campaign finished: stop={stop_reason} scheduled={scheduled} parked={parked} victories={victories} stuck={stuck}"
        )),
    }
}

pub fn render_branch_campaign_progress_event_v1(event: &BranchCampaignProgressEventV1) -> String {
    match event {
        BranchCampaignProgressEventV1::CampaignStarted {
            seed,
            max_rounds,
            round_depth,
            max_scheduled,
            max_parked,
        } => format!(
            "campaign start: seed={seed} rounds={max_rounds} round_depth={round_depth} scheduled_cap={max_scheduled} parked_cap={max_parked}"
        ),
        BranchCampaignProgressEventV1::RoundStarted {
            round,
            max_rounds,
            scheduled_branches,
            parked_branches,
        } => format!(
            "round {round}/{max_rounds}: advancing {scheduled_branches} scheduled branch(es), parked={parked_branches}"
        ),
        BranchCampaignProgressEventV1::BranchStarted {
            round,
            branch_index,
            branch_count,
            choices,
        } => format!(
            "round {round}: branch {branch_index}/{branch_count} running | choices: {choices}"
        ),
        BranchCampaignProgressEventV1::BranchFinished {
            round,
            branch_index,
            branch_count,
            produced_branches,
            explored_branch_points,
            elapsed_wall_ms,
            start_elapsed_wall_ms,
            replay_start_source,
            replay_suffix_commands,
            combat_budget_retry_used,
            wall_limit_hit,
            branch_limit_hit,
        } => {
            let limits = render_progress_limits_v1(*branch_limit_hit, *wall_limit_hit);
            let limits = if limits.is_empty() { "-" } else { &limits };
            let retry = if *combat_budget_retry_used {
                " retry=combat_budget"
            } else {
                ""
            };
            let start = if *start_elapsed_wall_ms > 0 {
                format!(" start_ms={start_elapsed_wall_ms}")
            } else {
                String::new()
            };
            let replay = replay_start_source
                .map(|source| format!(" replay={} suffix={replay_suffix_commands}", source.as_str()))
                .unwrap_or_default();
            format!(
                "round {round}: branch {branch_index}/{branch_count} done | produced={produced_branches} branch_points={explored_branch_points} elapsed_ms={elapsed_wall_ms}{start}{replay}{retry} limits=[{limits}]"
            )
        }
        BranchCampaignProgressEventV1::RoundFinished {
            round,
            started_scheduled,
            produced_branches,
            scheduled_after,
            parked_added,
            strategy_requests,
        } => format!(
            "round {round} done: started={started_scheduled} produced={produced_branches} scheduled_after={scheduled_after} parked_added={parked_added} strategy_requests={strategy_requests}"
        ),
        BranchCampaignProgressEventV1::CampaignFinished {
            stop_reason,
            scheduled,
            parked,
            victories,
            stuck,
        } => format!(
            "campaign finished: stop={stop_reason} scheduled={scheduled} parked={parked} victories={victories} stuck={stuck}"
        ),
    }
}

fn render_progress_limits_v1(branch_limit_hit: bool, wall_limit_hit: bool) -> String {
    let mut limits = Vec::new();
    if branch_limit_hit {
        limits.push("branch");
    }
    if wall_limit_hit {
        limits.push("wall");
    }
    limits.join(",")
}

fn format_progress_seconds_v1(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}
