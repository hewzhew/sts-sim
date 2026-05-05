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

pub fn full_run_action_shaping_reward(
    ctx: &EpisodeContext,
    action: &ClientInput,
    profile: RewardShapingProfile,
) -> f32 {
    let EngineState::RewardScreen(reward_state) = &ctx.engine_state else {
        return 0.0;
    };
    if let Some(cards) = &reward_state.pending_card_choice {
        return match action {
            ClientInput::SelectCard(index) => cards.get(*index).map_or(0.0, |card| match profile {
                RewardShapingProfile::Baseline => {
                    let score = rule_card_offer_score(card.id, &ctx.run_state);
                    (score as f32 / 300.0).clamp(-0.20, 0.35)
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    let delta = add_card_plan_delta(card.id, card.upgrades, &ctx.run_state);
                    (delta.plan_adjusted_score as f32 / 220.0).clamp(-0.25, 0.65)
                }
            }),
            ClientInput::Proceed => match profile {
                RewardShapingProfile::Baseline => {
                    let best_score = cards
                        .iter()
                        .map(|card| rule_card_offer_score(card.id, &ctx.run_state))
                        .max()
                        .unwrap_or(0);
                    if best_score >= 70 {
                        -0.18
                    } else if best_score <= 20 {
                        0.04
                    } else {
                        -0.05
                    }
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    let best_score = cards
                        .iter()
                        .map(|card| {
                            add_card_plan_delta(card.id, card.upgrades, &ctx.run_state)
                                .plan_adjusted_score
                        })
                        .max()
                        .unwrap_or(0);
                    if best_score >= 110 {
                        -0.65
                    } else if best_score >= 70 {
                        -0.40
                    } else if best_score <= 20 {
                        0.05
                    } else {
                        -0.12
                    }
                }
            },
            _ => 0.0,
        };
    }

    match action {
        ClientInput::ClaimReward(index) => reward_state
            .items
            .get(*index)
            .map(|item| match profile {
                RewardShapingProfile::Baseline => {
                    reward_item_shaping_value(&ctx.run_state, item).min(0.35)
                }
                RewardShapingProfile::PlanDeficitV0 => {
                    plan_deficit_reward_item_shaping_value(&ctx.run_state, item).min(0.55)
                }
            })
            .unwrap_or(0.0),
        ClientInput::Proceed => {
            let skipped_value = reward_state
                .items
                .iter()
                .map(|item| match profile {
                    RewardShapingProfile::Baseline => {
                        reward_item_shaping_value(&ctx.run_state, item).max(0.0)
                    }
                    RewardShapingProfile::PlanDeficitV0 => {
                        plan_deficit_reward_item_shaping_value(&ctx.run_state, item).max(0.0)
                    }
                })
                .sum::<f32>();
            match profile {
                RewardShapingProfile::Baseline => -skipped_value.min(0.60),
                RewardShapingProfile::PlanDeficitV0 => -skipped_value.min(1.00),
            }
        }
        _ => 0.0,
    }
}

pub fn reward_item_shaping_value(run_state: &RunState, item: &RewardItem) -> f32 {
    match item {
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => {
            (*amount as f32 / 240.0).clamp(0.05, 0.25)
        }
        RewardItem::Relic { .. } => 0.30,
        RewardItem::Potion { .. } => {
            if run_state
                .relics
                .iter()
                .any(|relic| relic.id == RelicId::Sozu)
            {
                -0.05
            } else if run_state.potions.iter().any(Option::is_none) {
                0.08
            } else {
                0.0
            }
        }
        RewardItem::Card { .. } => 0.12,
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 0.04,
    }
}

pub fn plan_deficit_reward_item_shaping_value(run_state: &RunState, item: &RewardItem) -> f32 {
    match item {
        RewardItem::Card { .. } => 0.42,
        RewardItem::Relic { .. } => 0.48,
        RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => {
            (*amount as f32 / 180.0).clamp(0.08, 0.35)
        }
        RewardItem::Potion { .. } => {
            if reward_item_likely_waste(run_state, item) {
                -0.04
            } else if run_state.potions.iter().any(Option::is_none) {
                0.10
            } else {
                0.0
            }
        }
        RewardItem::EmeraldKey | RewardItem::SapphireKey => 0.06,
    }
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
        ".venv-rl\\Scripts\\python.exe".to_string(),
        "tools\\learning\\smoke_full_run_env.py".to_string(),
        "--episodes".to_string(),
        "1".to_string(),
        "--seed".to_string(),
        seed.to_string(),
        "--ascension".to_string(),
        config.ascension.to_string(),
        "--class".to_string(),
        cli_class_arg(config.player_class).to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
        "--reward-shaping-profile".to_string(),
        config.reward_shaping_profile.as_str().to_string(),
    ];
    if config.final_act {
        parts.push("--final-act".to_string());
    }
    parts.join(" ")
}

