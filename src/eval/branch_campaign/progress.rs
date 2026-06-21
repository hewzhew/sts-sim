#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchCampaignProgressEventV1 {
    CampaignStarted {
        seed: u64,
        max_rounds: usize,
        round_depth: usize,
        max_active: usize,
        max_frozen: usize,
    },
    RoundStarted {
        round: usize,
        max_rounds: usize,
        active_branches: usize,
        frozen_branches: usize,
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
        started_active: usize,
        produced_branches: usize,
        active_after: usize,
        frozen_added: usize,
        strategy_requests: usize,
    },
    FrozenPromoted {
        promoted: usize,
        active_after: usize,
        frozen_remaining: usize,
        filled_active: usize,
        stronger_rebalanced: usize,
        diversity_rebalanced: usize,
        coverage_rebalanced: usize,
        rehydrated_recovered: usize,
        checkpoint_recovered: usize,
    },
    CampaignFinished {
        stop_reason: String,
        active: usize,
        frozen: usize,
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
            max_active,
            max_frozen,
        } => Some(format!(
            "campaign start: seed={seed} rounds={max_rounds} depth={round_depth} active_cap={max_active} frozen_cap={max_frozen}"
        )),
        BranchCampaignProgressEventV1::RoundStarted {
            round,
            max_rounds,
            active_branches,
            frozen_branches,
        } => Some(format!(
            "round {round}/{max_rounds}: active={active_branches} frozen={frozen_branches}"
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
            active_after,
            frozen_added,
            strategy_requests,
            ..
        } => {
            let mut extras = Vec::new();
            if *frozen_added > 0 {
                extras.push(format!("frozen+={frozen_added}"));
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
                "round {round} done: candidates={produced_branches} active={active_after}{suffix}"
            ))
        }
        BranchCampaignProgressEventV1::FrozenPromoted {
            promoted,
            active_after,
            frozen_remaining,
            filled_active,
            stronger_rebalanced,
            diversity_rebalanced,
            coverage_rebalanced,
            rehydrated_recovered,
            checkpoint_recovered,
        } => {
            if *promoted == 0 {
                return None;
            }
            let source_suffix = render_progress_promotion_sources_v1(
                *filled_active,
                *stronger_rebalanced,
                *diversity_rebalanced,
                *coverage_rebalanced,
                *rehydrated_recovered,
                *checkpoint_recovered,
            );
            Some(format!(
                "promoted {promoted}: active={active_after} frozen={frozen_remaining}{source_suffix}"
            ))
        }
        BranchCampaignProgressEventV1::CampaignFinished {
            stop_reason,
            active,
            frozen,
            victories,
            stuck,
        } => Some(format!(
            "campaign finished: stop={stop_reason} active={active} frozen={frozen} victories={victories} stuck={stuck}"
        )),
    }
}

pub fn render_branch_campaign_progress_event_v1(event: &BranchCampaignProgressEventV1) -> String {
    match event {
        BranchCampaignProgressEventV1::CampaignStarted {
            seed,
            max_rounds,
            round_depth,
            max_active,
            max_frozen,
        } => format!(
            "campaign start: seed={seed} rounds={max_rounds} round_depth={round_depth} active_cap={max_active} frozen_cap={max_frozen}"
        ),
        BranchCampaignProgressEventV1::RoundStarted {
            round,
            max_rounds,
            active_branches,
            frozen_branches,
        } => format!(
            "round {round}/{max_rounds}: advancing {active_branches} active branch(es), frozen={frozen_branches}"
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
            started_active,
            produced_branches,
            active_after,
            frozen_added,
            strategy_requests,
        } => format!(
            "round {round} done: started={started_active} produced={produced_branches} active_after={active_after} frozen_added={frozen_added} strategy_requests={strategy_requests}"
        ),
        BranchCampaignProgressEventV1::FrozenPromoted {
            promoted,
            active_after,
            frozen_remaining,
            filled_active,
            stronger_rebalanced,
            diversity_rebalanced,
            coverage_rebalanced,
            rehydrated_recovered,
            checkpoint_recovered,
        } => {
            let source_suffix = render_progress_promotion_sources_v1(
                *filled_active,
                *stronger_rebalanced,
                *diversity_rebalanced,
                *coverage_rebalanced,
                *rehydrated_recovered,
                *checkpoint_recovered,
            );
            format!(
                "promoted/rebalanced {promoted} frozen branch(es); active_after={active_after} frozen={frozen_remaining}{source_suffix}"
            )
        }
        BranchCampaignProgressEventV1::CampaignFinished {
            stop_reason,
            active,
            frozen,
            victories,
            stuck,
        } => format!(
            "campaign finished: stop={stop_reason} active={active} frozen={frozen} victories={victories} stuck={stuck}"
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

fn render_progress_promotion_sources_v1(
    filled_active: usize,
    stronger_rebalanced: usize,
    diversity_rebalanced: usize,
    coverage_rebalanced: usize,
    rehydrated_recovered: usize,
    checkpoint_recovered: usize,
) -> String {
    let mut sources = Vec::new();
    if filled_active > 0 {
        sources.push(format!("fill={filled_active}"));
    }
    if stronger_rebalanced > 0 {
        sources.push(format!("stronger={stronger_rebalanced}"));
    }
    if diversity_rebalanced > 0 {
        sources.push(format!("diversity={diversity_rebalanced}"));
    }
    if coverage_rebalanced > 0 {
        sources.push(format!("coverage={coverage_rebalanced}"));
    }
    if rehydrated_recovered > 0 {
        sources.push(format!("rehydrated={rehydrated_recovered}"));
    }
    if checkpoint_recovered > 0 {
        sources.push(format!("checkpoint={checkpoint_recovered}"));
    }
    if sources.is_empty() {
        String::new()
    } else {
        format!(" sources=[{}]", sources.join(" "))
    }
}

fn format_progress_seconds_v1(ms: u64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}
