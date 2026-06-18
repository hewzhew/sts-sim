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
            combat_budget_retry_used,
            wall_limit_hit,
            branch_limit_hit,
        } => {
            let mut limits = Vec::new();
            if *branch_limit_hit {
                limits.push("branch");
            }
            if *wall_limit_hit {
                limits.push("wall");
            }
            let limits = if limits.is_empty() {
                "-".to_string()
            } else {
                limits.join(",")
            };
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
            format!(
                "round {round}: branch {branch_index}/{branch_count} done | produced={produced_branches} branch_points={explored_branch_points} elapsed_ms={elapsed_wall_ms}{start}{retry} limits=[{limits}]"
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
            rehydrated_recovered,
            checkpoint_recovered,
        } => {
            let mut sources = Vec::new();
            if *filled_active > 0 {
                sources.push(format!("fill={filled_active}"));
            }
            if *stronger_rebalanced > 0 {
                sources.push(format!("stronger={stronger_rebalanced}"));
            }
            if *diversity_rebalanced > 0 {
                sources.push(format!("diversity={diversity_rebalanced}"));
            }
            if *rehydrated_recovered > 0 {
                sources.push(format!("rehydrated={rehydrated_recovered}"));
            }
            if *checkpoint_recovered > 0 {
                sources.push(format!("checkpoint={checkpoint_recovered}"));
            }
            let source_suffix = if sources.is_empty() {
                String::new()
            } else {
                format!(" sources=[{}]", sources.join(" "))
            };
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
