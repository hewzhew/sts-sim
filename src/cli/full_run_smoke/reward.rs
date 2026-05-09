use super::*;

pub fn normalize_player_class(player_class: &str) -> &'static str {
    match player_class.to_ascii_lowercase().as_str() {
        "ironclad" | "red" => "Ironclad",
        "silent" | "green" => "Silent",
        "defect" | "blue" => "Defect",
        "watcher" | "purple" => "Watcher",
        _ => "Ironclad",
    }
}

pub fn full_run_progress_score(ctx: &EpisodeContext) -> f32 {
    let active_hp = ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.current_hp)
        .unwrap_or(ctx.run_state.current_hp);
    let active_max_hp = ctx
        .combat_state
        .as_ref()
        .map(|combat| combat.entities.player.max_hp)
        .unwrap_or(ctx.run_state.max_hp);
    let hp_fraction = if active_max_hp > 0 {
        active_hp.max(0) as f32 / active_max_hp as f32
    } else {
        0.0
    };
    ctx.run_state.floor_num.max(0) as f32 + ctx.combat_win_count as f32 * 2.0 + hp_fraction
}

pub fn reward_item_type_label(item: &RewardItem) -> &'static str {
    match item {
        RewardItem::Gold { .. } => "gold",
        RewardItem::StolenGold { .. } => "stolen_gold",
        RewardItem::Card { .. } => "card_reward",
        RewardItem::Relic { .. } => "relic",
        RewardItem::Potion { .. } => "potion",
        RewardItem::EmeraldKey => "emerald_key",
        RewardItem::SapphireKey => "sapphire_key",
    }
}

pub fn reward_item_amount(item: &RewardItem) -> i32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => *amount,
        _ => 0,
    }
}

pub fn reward_item_claimable(run_state: &RunState, item: &RewardItem) -> bool {
    match item {
        RewardItem::Potion { .. } => {
            run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::Sozu)
                || run_state.potions.iter().any(Option::is_none)
        }
        _ => true,
    }
}

pub fn reward_item_capacity_blocked(run_state: &RunState, item: &RewardItem) -> bool {
    matches!(item, RewardItem::Potion { .. })
        && !run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Sozu)
        && !run_state.potions.iter().any(Option::is_none)
}

pub fn reward_item_likely_waste(run_state: &RunState, item: &RewardItem) -> bool {
    matches!(item, RewardItem::Potion { .. })
        && run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Sozu)
}

pub fn reward_item_claim_score(run_state: &RunState, item: &RewardItem) -> i32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => 60 + amount / 8,
        RewardItem::Relic { .. } => 120,
        RewardItem::Potion { .. } => {
            if reward_item_likely_waste(run_state, item) {
                -10
            } else if run_state.potions.iter().any(Option::is_none) {
                55
            } else {
                0
            }
        }
        RewardItem::Card { .. } => 70,
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 25,
    }
}

pub fn full_run_result_label(ctx: &EpisodeContext, done: bool, crash: Option<&String>) -> String {
    match &ctx.engine_state {
        EngineState::GameOver(RunResult::Victory) => "victory",
        EngineState::GameOver(RunResult::Defeat) => "defeat",
        _ if crash.is_some() => "crash",
        _ if done => "truncated",
        _ => "ongoing",
    }
    .to_string()
}

pub fn make_full_run_env_contract_failure(
    config: &FullRunEnvConfig,
    seed: u64,
    kind: &str,
    terminal_reason: &str,
    floor: i32,
    act: u8,
    step: Option<usize>,
    action_key: Option<String>,
    decision_type: Option<String>,
    engine_state: Option<String>,
    details: String,
) -> RunContractFailure {
    RunContractFailure {
        kind: kind.to_string(),
        episode_id: 0,
        seed,
        policy: "external_driver".to_string(),
        step,
        action_key,
        decision_type,
        engine_state,
        floor,
        act,
        terminal_reason: terminal_reason.to_string(),
        details,
        trace_path: None,
        reproduce_command: full_run_env_reproduce_command(config, seed),
    }
}

pub fn full_run_env_reproduce_command(config: &FullRunEnvConfig, seed: u64) -> String {
    let mut parts = vec![
        "python".to_string(),
        "tools\\learning\\collect_decision_records.py".to_string(),
        "--out".to_string(),
        format!("tmp\\decision_records\\contract_failure_seed_{seed}.jsonl"),
        "--seed-start".to_string(),
        seed.to_string(),
        "--episodes".to_string(),
        "1".to_string(),
        "--ascension".to_string(),
        config.ascension.to_string(),
        "--class".to_string(),
        cli_class_arg(config.player_class).to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
    ];
    if config.final_act {
        parts.push("--final-act".to_string());
    }
    parts.join(" ")
}
