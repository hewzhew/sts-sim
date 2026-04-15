use std::io::{self, BufRead, Write};
use sts_simulator::combat::CombatState;
use sts_simulator::engine::run_loop::tick_run;
use sts_simulator::state::core::*;
use sts_simulator::state::run::RunState;
use sts_simulator::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};
use sts_simulator::state::RewardItem;

// ─── Smart Silence System ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisplayMode {
    TotalSilence = 0,
    Summary = 1,
    DeepDebug = 2,
}

pub enum LogRule {
    ActReached(u8),
    HpBelowPercent(f32),
    BossCombat,
}

pub struct DashboardConfig {
    pub base_mode: DisplayMode,
    pub intercept_rules: Vec<LogRule>,
}

impl DashboardConfig {
    pub fn new(base_mode: DisplayMode) -> Self {
        Self {
            base_mode,
            intercept_rules: Vec::new(),
        }
    }
}

// ─── Input source / mode ────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum InputMode {
    Manual,
    Auto,
    /// Bot handles current combat only, then returns to Manual.
    SkipCombat,
}

struct AppState {
    mode: InputMode,
    dashboard: DashboardConfig,
    current_display: DisplayMode,
    agent: sts_simulator::bot::agent::Agent,
    combat_cards_played: usize,
    combat_start_hp: i32,
    /// Stashed EventCombatState so event-triggered combat can route through
    /// normal CombatProcessing flow while preserving post-combat logic.
    stashed_event_combat: Option<EventCombatState>,
}

fn print_end_turn_diagnostic(combat: &CombatState) {
    let lines = sts_simulator::bot::combat_heuristic::describe_end_turn_options(combat);
    if let Some(summary) = lines.first() {
        println!("  [BOT] End Turn | {}", summary);
        if let Some(best_alt) = lines.get(1) {
            println!("  [BOT]   alt: {}", best_alt);
        }
    } else {
        println!("  [BOT] End Turn");
    }
}

fn describe_selection_resolution(
    resolution: &SelectionResolution,
    run_state: &RunState,
    combat_state: &Option<CombatState>,
) -> String {
    let labels = resolution
        .selected
        .iter()
        .filter_map(|target| match target {
            SelectionTargetRef::CardUuid(uuid) => match resolution.scope {
                SelectionScope::Deck => run_state
                    .master_deck
                    .iter()
                    .find(|card| card.uuid == *uuid)
                    .map(|card| {
                        sts_simulator::cli::display::describe_card_snapshot(
                            &sts_simulator::state::selection::DomainCardSnapshot {
                                id: card.id,
                                upgrades: card.upgrades,
                                uuid: card.uuid,
                            },
                        )
                    }),
                SelectionScope::Hand | SelectionScope::Grid => {
                    combat_state.as_ref().and_then(|cs| {
                        cs.zones
                            .hand
                            .iter()
                            .chain(cs.zones.draw_pile.iter())
                            .chain(cs.zones.discard_pile.iter())
                            .chain(cs.zones.exhaust_pile.iter())
                            .chain(cs.zones.limbo.iter())
                            .find(|card| card.uuid == *uuid)
                            .map(|card| {
                                sts_simulator::cli::display::describe_card_snapshot(
                                    &sts_simulator::state::selection::DomainCardSnapshot {
                                        id: card.id,
                                        upgrades: card.upgrades,
                                        uuid: card.uuid,
                                    },
                                )
                            })
                    })
                }
            },
        })
        .collect::<Vec<_>>();
    if labels.is_empty() {
        format!("{:?} selection", resolution.scope)
    } else {
        labels.join(", ")
    }
}

fn describe_bot_decision(
    decision: &ClientInput,
    run_state: &RunState,
    combat_state: &Option<CombatState>,
) -> Option<String> {
    match decision {
        ClientInput::PlayCard { card_index, target } => combat_state.as_ref().and_then(|combat| {
            sts_simulator::cli::display::describe_play_card_choice(combat, *card_index, *target)
        }),
        ClientInput::UsePotion {
            potion_index,
            target,
        } => combat_state.as_ref().and_then(|combat| {
            sts_simulator::cli::display::describe_potion_use_choice(combat, *potion_index, *target)
        }),
        ClientInput::SelectMapNode(x) => Some(
            sts_simulator::cli::display::describe_bot_map_choice(run_state, *x),
        ),
        _ => None,
    }
}

fn format_context_suffix(
    rule_context_summary: Option<&str>,
    rationale_key: Option<&str>,
    context_delta: i32,
) -> String {
    let mut parts = Vec::new();
    if let Some(summary) = rule_context_summary {
        parts.push(format!("ctx={summary}"));
    }
    if let Some(rationale) = rationale_key {
        parts.push(format!("rationale={rationale}"));
    }
    if context_delta != 0 {
        parts.push(format!("delta_ctx={context_delta}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" | {}", parts.join(" "))
    }
}

fn describe_reward_screen_decision(
    engine_state: &EngineState,
    run_state: &RunState,
    decision: &ClientInput,
) -> Option<String> {
    let EngineState::RewardScreen(reward) = engine_state else {
        return None;
    };

    match decision {
        ClientInput::SelectCard(idx) | ClientInput::SubmitDiscoverChoice(idx) => {
            let cards = reward.pending_card_choice.as_ref()?;
            let offered_ids = cards.iter().map(|card| card.id).collect::<Vec<_>>();
            let evaluation =
                sts_simulator::bot::reward_heuristics::evaluate_reward_screen_for_run_detailed(
                    &offered_ids,
                    run_state,
                );
            let card_eval = evaluation.offered_cards.get(*idx)?;
            let reward_card = cards.get(*idx)?;
            let def = sts_simulator::content::cards::get_card_definition(reward_card.id);
            Some(format!(
                "Pick {}{} [reward {}]{}",
                def.name,
                if reward_card.upgrades > 0 {
                    format!(" +{}", reward_card.upgrades)
                } else {
                    String::new()
                },
                idx,
                format_context_suffix(
                    card_eval.delta_rule_context_summary,
                    card_eval.delta_context_rationale_key,
                    card_eval.delta_context
                )
            ))
        }
        ClientInput::ClaimReward(idx) => reward.items.get(*idx).map(|item| match item {
            RewardItem::Gold { amount } | RewardItem::StolenGold { amount } => {
                format!("Reward #{idx} -> Gold {amount}")
            }
            RewardItem::Card { .. } => format!("Reward #{idx} -> Card Reward"),
            RewardItem::Relic { relic_id } => format!("Reward #{idx} -> Relic {:?}", relic_id),
            RewardItem::Potion { potion_id } => format!("Reward #{idx} -> Potion {:?}", potion_id),
            RewardItem::EmeraldKey => format!("Reward #{idx} -> Emerald Key"),
            RewardItem::SapphireKey => format!("Reward #{idx} -> Sapphire Key"),
        }),
        _ => None,
    }
}

fn describe_shop_screen_decision(
    engine_state: &EngineState,
    run_state: &RunState,
    decision: &ClientInput,
) -> Option<String> {
    let EngineState::Shop(shop) = engine_state else {
        return None;
    };

    match decision {
        ClientInput::BuyCard(idx) => {
            let card = shop.cards.get(*idx)?;
            let def = sts_simulator::content::cards::get_card_definition(card.card_id);
            let delta =
                sts_simulator::bot::deck_delta_eval::compare_pick_vs_skip(run_state, card.card_id);
            Some(format!(
                "Shop Card: {} [price {}]{}",
                def.name,
                card.price,
                format_context_suffix(
                    delta.rule_context_summary,
                    delta.context_rationale_key,
                    delta.context_delta
                )
            ))
        }
        ClientInput::BuyRelic(idx) => shop
            .relics
            .get(*idx)
            .map(|relic| format!("Shop Relic: {:?} [price {}]", relic.relic_id, relic.price)),
        ClientInput::BuyPotion(idx) => shop.potions.get(*idx).map(|potion| {
            format!(
                "Shop Potion: {:?} [price {}]",
                potion.potion_id, potion.price
            )
        }),
        ClientInput::PurgeCard(idx) => run_state.master_deck.get(*idx).map(|card| {
            let snapshot = sts_simulator::state::selection::DomainCardSnapshot {
                id: card.id,
                upgrades: card.upgrades,
                uuid: card.uuid,
            };
            format!(
                "Shop Purge: {} [cost {}]",
                sts_simulator::cli::display::describe_card_snapshot(&snapshot),
                shop.purge_cost
            )
        }),
        ClientInput::Proceed => Some("Shop: Save gold".to_string()),
        _ => None,
    }
}

fn print_emitted_outputs(
    run_state: &mut RunState,
    combat_state: &mut Option<CombatState>,
    debug: bool,
) {
    for event in run_state.take_emitted_events() {
        if debug
            || !matches!(
                event,
                sts_simulator::state::selection::DomainEvent::SelectionResolved { .. }
            )
        {
            println!(
                "{}",
                sts_simulator::cli::display::render_user_feed_event(&event)
            );
        }
    }
    if let Some(combat) = combat_state.as_mut() {
        for event in combat.take_emitted_events() {
            if debug
                || !matches!(
                    event,
                    sts_simulator::state::selection::DomainEvent::SelectionResolved { .. }
                )
            {
                println!(
                    "{}",
                    sts_simulator::cli::display::render_user_feed_event(&event)
                );
            }
        }
        if debug {
            for diagnostic in combat.take_engine_diagnostics() {
                println!(
                    "{}",
                    sts_simulator::cli::display::render_engine_diagnostic(&diagnostic)
                );
            }
        } else {
            let _ = combat.take_engine_diagnostics();
        }
    }
}

impl AppState {
    pub fn evaluate_display_mode(&mut self, run: &RunState, _combat: Option<&CombatState>) {
        let mut target_mode = self.dashboard.base_mode;
        for rule in &self.dashboard.intercept_rules {
            let triggered = match rule {
                LogRule::ActReached(act) => run.act_num >= *act,
                LogRule::HpBelowPercent(pct) => {
                    (run.current_hp as f32 / run.max_hp.max(1) as f32) < *pct
                }
                LogRule::BossCombat => _combat.map_or(false, |c| c.meta.is_boss_fight),
            };
            if triggered && target_mode < DisplayMode::DeepDebug {
                target_mode = DisplayMode::DeepDebug;
            }
        }
        self.current_display = target_mode;
    }

    fn is_bot_active(&self, es: &EngineState) -> bool {
        match self.mode {
            InputMode::Auto => true,
            InputMode::SkipCombat => matches!(
                es,
                EngineState::CombatPlayerTurn
                    | EngineState::CombatProcessing
                    | EngineState::PendingChoice(_)
                    | EngineState::EventCombat(_)
            ),
            InputMode::Manual => false,
        }
    }

    pub fn is_debug(&self) -> bool {
        self.current_display == DisplayMode::DeepDebug
    }
    pub fn is_silence(&self) -> bool {
        self.current_display == DisplayMode::TotalSilence
    }
}

// ─── Main ───────────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    });

    let ascension: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

    let has_flag = |f: &str| args.iter().any(|a| a.trim().contains(f));
    let flag_value = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|idx| args.get(idx + 1))
            .cloned()
    };
    let flag_values = |flag: &str| {
        let mut values = Vec::new();
        let mut idx = 0usize;
        while idx < args.len() {
            if args[idx] == flag {
                if let Some(value) = args.get(idx + 1) {
                    values.push(value.clone());
                }
                idx += 1;
            }
            idx += 1;
        }
        values
    };
    let player_class = match flag_value("--class")
        .unwrap_or_else(|| "ironclad".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "silent" => "Silent",
        "defect" => "Defect",
        "watcher" => "Watcher",
        _ => "Ironclad",
    };
    let start_auto = has_flag("--auto");
    let coverage_mode = if has_flag("--coverage-off") {
        sts_simulator::bot::coverage::CoverageMode::Off
    } else if has_flag("--coverage-aggressive") {
        sts_simulator::bot::coverage::CoverageMode::AggressiveNovel
    } else {
        sts_simulator::bot::coverage::CoverageMode::PreferNovel
    };
    let curiosity_target = flag_value("--curiosity-card")
        .map(sts_simulator::bot::coverage::CuriosityTarget::card)
        .or_else(|| {
            flag_value("--curiosity-relic")
                .map(sts_simulator::bot::coverage::CuriosityTarget::relic)
        })
        .or_else(|| {
            flag_value("--curiosity-potion")
                .map(sts_simulator::bot::coverage::CuriosityTarget::potion)
        })
        .or_else(|| {
            flag_value("--curiosity-archetype")
                .map(sts_simulator::bot::coverage::CuriosityTarget::archetype)
        })
        .or_else(|| {
            flag_value("--curiosity-power")
                .map(sts_simulator::bot::coverage::CuriosityTarget::power_tag)
        })
        .or_else(|| {
            flag_value("--curiosity-pile")
                .map(sts_simulator::bot::coverage::CuriosityTarget::pile_tag)
        })
        .or_else(|| {
            flag_value("--curiosity-pending")
                .map(sts_simulator::bot::coverage::CuriosityTarget::pending_choice)
        })
        .or_else(|| {
            flag_value("--curiosity-source")
                .map(sts_simulator::bot::coverage::CuriosityTarget::source)
        });

    // Parse Dashboard Configuration
    let mut db_config = DashboardConfig::new(DisplayMode::DeepDebug);
    if has_flag("--silent") {
        db_config.base_mode = DisplayMode::TotalSilence;
    } else if has_flag("--summary") {
        db_config.base_mode = DisplayMode::Summary;
    } else if has_flag("--fast-act1") {
        db_config.base_mode = DisplayMode::Summary;
        db_config.intercept_rules.push(LogRule::ActReached(2));
    } else if has_flag("--panic-watch") {
        db_config.base_mode = DisplayMode::Summary;
        db_config.intercept_rules.push(LogRule::HpBelowPercent(0.5));
    } else if has_flag("--boss-only") {
        db_config.base_mode = DisplayMode::TotalSilence;
        db_config.intercept_rules.push(LogRule::BossCombat);
    } else if has_flag("--debug") {
        db_config.base_mode = DisplayMode::DeepDebug;
    }

    let initial_display = db_config.base_mode;

    let mut app = AppState {
        mode: if start_auto {
            InputMode::Auto
        } else {
            InputMode::Manual
        },
        dashboard: db_config,
        current_display: initial_display,
        agent: sts_simulator::bot::agent::Agent::new(),
        combat_cards_played: 0,
        combat_start_hp: 0,
        stashed_event_combat: None,
    };
    app.agent.set_coverage_mode(coverage_mode);
    app.agent.set_curiosity_target(curiosity_target.clone());

    if has_flag("--live-comm") {
        let parity_mode = match flag_value("--live-comm-parity-mode")
            .unwrap_or_else(|| {
                if has_flag("--live-comm-strict") {
                    "strict".to_string()
                } else {
                    "survey".to_string()
                }
            })
            .to_ascii_lowercase()
            .as_str()
        {
            "strict" => sts_simulator::cli::live_comm::LiveParityMode::Strict,
            _ => sts_simulator::cli::live_comm::LiveParityMode::Survey,
        };
        let config = sts_simulator::cli::live_comm::LiveCommConfig {
            human_card_reward_audit: has_flag("--live-comm-human-card-reward"),
            human_boss_combat_handoff: has_flag("--live-comm-human-boss-combat"),
            sidecar_shadow: has_flag("--sidecar-shadow"),
            parity_mode,
            combat_search_budget: flag_value("--live-comm-search-budget")
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| match parity_mode {
                    sts_simulator::cli::live_comm::LiveParityMode::Strict => 1200,
                    sts_simulator::cli::live_comm::LiveParityMode::Survey => 900,
                }),
            watch_capture: sts_simulator::cli::live_comm::LiveWatchCaptureConfig {
                cards: flag_values("--live-watch-card"),
                relics: flag_values("--live-watch-relic"),
                powers: flag_values("--live-watch-power"),
                monsters: flag_values("--live-watch-monster"),
                screens: flag_values("--live-watch-screen"),
                room_phases: flag_values("--live-watch-room-phase"),
                command_kinds: flag_values("--live-watch-command-kind"),
                match_mode: match flag_value("--live-watch-match")
                    .unwrap_or_else(|| "any".to_string())
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "all" => sts_simulator::cli::live_comm::LiveWatchMatchMode::All,
                    _ => sts_simulator::cli::live_comm::LiveWatchMatchMode::Any,
                },
                window_responses: flag_value("--live-watch-window")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(6),
                dedupe_window_responses: flag_value("--live-watch-dedupe-window")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(3),
                max_captures: flag_value("--live-watch-max")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(20),
                out_dir: flag_value("--live-watch-dir")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| {
                        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                            .join("tests")
                            .join("live_captures")
                    }),
            },
        };
        sts_simulator::cli::live_comm::run_live_comm_loop(app.agent, config);
        return;
    }

    if !app.is_silence() {
        println!("=== Slay the Spire Simulator ===");
        println!(
            "Seed: {}  Ascension: {}  Class: {}",
            seed, ascension, player_class
        );
        println!(
            "Mode: {}  Display: {:?}  Coverage: {:?}{}",
            if app.mode == InputMode::Auto {
                "auto"
            } else {
                "manual"
            },
            app.current_display,
            coverage_mode,
            curiosity_target
                .as_ref()
                .map(|target| format!("  Curiosity: {}", target.label()))
                .unwrap_or_default()
        );
        println!("Type 'help' for commands.\n");
    }

    let mut run_state = RunState::new(seed, ascension, false, player_class);

    let mut engine_state = EngineState::EventRoom;
    let mut combat_state: Option<CombatState> = None;

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut last_floor_num = 0;
    let mut watchdog = sts_simulator::SimulationWatchdog::new(100, 5);

    'outer: loop {
        app.evaluate_display_mode(&run_state, combat_state.as_ref());

        // ── Init combat if needed (normal combat) ──
        if matches!(engine_state, EngineState::CombatPlayerTurn) && combat_state.is_none() {
            combat_state = Some(init_combat(&mut run_state, app.current_display));
            app.combat_start_hp = run_state.current_hp;
            app.combat_cards_played = 0;
            engine_state = EngineState::CombatProcessing;
            // Drain initial start-of-combat triggers (relics/innate cards) using the proper run_loop
            loop {
                let keep_running =
                    tick_run(&mut engine_state, &mut run_state, &mut combat_state, None);
                if !keep_running || !matches!(engine_state, EngineState::CombatProcessing) {
                    break;
                }
            }
        }

        // ── Init combat if needed (EventCombat → stash + normal flow) ──
        if let EngineState::EventCombat(ecs) = engine_state.clone() {
            if combat_state.is_none() {
                if let Some(encounter_id) = encounter_key_to_id(ecs.encounter_key) {
                    if app.current_display >= DisplayMode::Summary {
                        println!("  [Event Combat: {:?}]", encounter_id);
                    }
                    // Stash the event combat state for post-combat handling
                    app.stashed_event_combat = Some(ecs);
                    combat_state = Some(init_event_combat(
                        &mut run_state,
                        encounter_id,
                        app.current_display,
                    ));
                    app.combat_start_hp = run_state.current_hp;
                    app.combat_cards_played = 0;
                    // Switch to normal combat flow so action queue gets processed
                    engine_state = EngineState::CombatProcessing;
                    loop {
                        let kr =
                            tick_run(&mut engine_state, &mut run_state, &mut combat_state, None);
                        if !kr || !matches!(engine_state, EngineState::CombatProcessing) {
                            break;
                        }
                    }
                } else {
                    eprintln!("  ERROR: Unknown encounter_key '{}'", ecs.encounter_key);
                }
            }
        }

        if run_state.floor_num != last_floor_num {
            last_floor_num = run_state.floor_num;
            if app.current_display == DisplayMode::Summary {
                let room_str = run_state
                    .map
                    .get_current_room_type()
                    .map(|r| format!("{:?}", r))
                    .unwrap_or_else(|| "Unknown".to_string());
                println!(
                    "\n[Floor {} | Act {}] HP: {}/{} | Gold: {} | Room: {}",
                    run_state.floor_num,
                    run_state.act_num,
                    run_state.current_hp,
                    run_state.max_hp,
                    run_state.gold,
                    room_str
                );
            }
        }

        if matches!(engine_state, EngineState::GameOver(_)) {
            println!("\n=== GAME OVER ===");
            match &engine_state {
                EngineState::GameOver(RunResult::Victory) => println!("  VICTORY!"),
                EngineState::GameOver(RunResult::Defeat) => println!("  DEFEAT."),
                _ => println!("  (unexpected end)"),
            }
            print_run_summary(&run_state);
            break;
        }

        // ── Print state (always for manual, gated for bot) ──
        if !app.is_bot_active(&engine_state) || app.is_debug() {
            sts_simulator::cli::display::print_state(&engine_state, &run_state, &combat_state);
        }

        // ── Get input: bot or human ──
        let input = if app.is_bot_active(&engine_state) {
            // In SkipCombat mode, suppress per-card spam — only show turn boundaries
            let verbose = app.mode != InputMode::SkipCombat;
            if verbose && app.current_display >= DisplayMode::Summary {
                if let Some(cs) = &combat_state {
                    println!(
                        "  [BOT] Hand: {} Energy: {}",
                        cs.zones.hand.len(),
                        cs.turn.energy
                    );
                }
            }
            let decision =
                app.agent
                    .decide(&engine_state, &run_state, &combat_state, app.is_debug());
            if app.is_debug() {
                if let Some(description) =
                    describe_bot_decision(&decision, &run_state, &combat_state)
                {
                    println!("  [BOT] {description}");
                    println!("  [BOT][raw] {:?}", decision);
                } else if let Some(description) =
                    describe_reward_screen_decision(&engine_state, &run_state, &decision)
                {
                    println!("  [BOT] {description}");
                    println!("  [BOT][raw] {:?}", decision);
                } else if let Some(description) =
                    describe_shop_screen_decision(&engine_state, &run_state, &decision)
                {
                    println!("  [BOT] {description}");
                    println!("  [BOT][raw] {:?}", decision);
                } else {
                    println!("  [BOT] {:?}", decision);
                }
            } else if verbose && app.current_display >= DisplayMode::Summary {
                match &decision {
                    ClientInput::PlayCard { .. } | ClientInput::UsePotion { .. } => {
                        if let Some(description) =
                            describe_bot_decision(&decision, &run_state, &combat_state)
                        {
                            println!("  [BOT] {description}");
                        }
                    }
                    ClientInput::EndTurn => {
                        if let Some(cs) = &combat_state {
                            print_end_turn_diagnostic(cs);
                        } else {
                            println!("  [BOT] End Turn");
                        }
                    }
                    ClientInput::Proceed => {
                        if let Some(description) =
                            describe_shop_screen_decision(&engine_state, &run_state, &decision)
                        {
                            println!("  [BOT] {description}");
                        }
                    }
                    ClientInput::EventChoice(choice_idx) => {
                        if let Some(event) = run_state.event_state.as_ref() {
                            let choices =
                                sts_simulator::engine::event_handler::get_event_choices(&run_state);
                            if let Some(decision) =
                                sts_simulator::bot::event_policy::choose_local_event_choice(
                                    &run_state, event, &choices,
                                )
                            {
                                if decision.option_index == *choice_idx {
                                    let context =
                                        sts_simulator::bot::event_policy::local_event_context(
                                            &run_state, event, &choices,
                                        );
                                    println!(
                                        "  [BOT] Event: {}",
                                        sts_simulator::bot::event_policy::describe_choice(
                                            &context, &decision
                                        )
                                    );
                                } else if let Some(choice) = choices.get(*choice_idx) {
                                    println!("  [BOT] Event: {}", choice.text);
                                } else {
                                    println!("  [BOT] Event choice #{}", choice_idx);
                                }
                            } else if let Some(choice) = choices.get(*choice_idx) {
                                println!("  [BOT] Event: {}", choice.text);
                            } else {
                                println!("  [BOT] Event choice #{}", choice_idx);
                            }
                        } else {
                            println!("  [BOT] Event choice #{}", choice_idx);
                        }
                    }
                    ClientInput::SelectCard(_) | ClientInput::SubmitDiscoverChoice(_) => {
                        if let Some(description) =
                            describe_reward_screen_decision(&engine_state, &run_state, &decision)
                        {
                            println!("  [BOT] {description}");
                        } else if let ClientInput::SelectCard(idx) = &decision {
                            println!("  [BOT] Pick Card #{}", idx);
                        }
                    }
                    ClientInput::ClaimReward(_) => {
                        if let Some(description) =
                            describe_reward_screen_decision(&engine_state, &run_state, &decision)
                        {
                            println!("  [BOT] {description}");
                        } else if let ClientInput::ClaimReward(idx) = &decision {
                            println!("  [BOT] Reward #{}", idx);
                        }
                    }
                    ClientInput::SubmitRelicChoice(idx) => println!("  [BOT] Boss Relic #{}", idx),
                    ClientInput::SelectMapNode(_) => {
                        if let Some(description) =
                            describe_bot_decision(&decision, &run_state, &combat_state)
                        {
                            println!("  [BOT] {description}");
                        }
                    }
                    ClientInput::BuyCard(_)
                    | ClientInput::BuyRelic(_)
                    | ClientInput::BuyPotion(_)
                    | ClientInput::PurgeCard(_)
                        if matches!(engine_state, EngineState::Shop(_)) =>
                    {
                        if let Some(description) =
                            describe_shop_screen_decision(&engine_state, &run_state, &decision)
                        {
                            println!("  [BOT] {description}");
                        }
                    }
                    ClientInput::CampfireOption(choice) => match choice {
                        CampfireChoice::Smith(idx) => {
                            if let Some(card) = run_state.master_deck.get(*idx) {
                                let snapshot =
                                    sts_simulator::state::selection::DomainCardSnapshot {
                                        id: card.id,
                                        upgrades: card.upgrades,
                                        uuid: card.uuid,
                                    };
                                println!(
                                    "  [BOT] Camp Smith: {}",
                                    sts_simulator::cli::display::describe_card_snapshot(&snapshot)
                                );
                            } else {
                                println!("  [BOT] Camp: {:?}", choice);
                            }
                        }
                        CampfireChoice::Toke(idx) => {
                            if let Some(card) = run_state.master_deck.get(*idx) {
                                let snapshot =
                                    sts_simulator::state::selection::DomainCardSnapshot {
                                        id: card.id,
                                        upgrades: card.upgrades,
                                        uuid: card.uuid,
                                    };
                                println!(
                                    "  [BOT] Camp Toke: {}",
                                    sts_simulator::cli::display::describe_card_snapshot(&snapshot)
                                );
                            } else {
                                println!("  [BOT] Camp: {:?}", choice);
                            }
                        }
                        _ => println!("  [BOT] Camp: {:?}", choice),
                    },
                    ClientInput::SubmitSelection(selection) => println!(
                        "  [BOT] {:?} target: {}",
                        selection.scope,
                        describe_selection_resolution(selection, &run_state, &combat_state)
                    ),
                    _ => println!("  [BOT] {:?}", decision),
                }
            } else if app.mode == InputMode::SkipCombat {
                // In skip mode, print turn ends + significant actions (potions, key cards)
                match &decision {
                    ClientInput::EndTurn => {
                        if let Some(cs) = &combat_state {
                            println!(
                                "  Turn {} done — HP: {}/{}",
                                cs.turn.turn_count,
                                cs.entities.player.current_hp,
                                cs.entities.player.max_hp
                            );
                        }
                    }
                    ClientInput::UsePotion { potion_index, .. } => {
                        if let Some(cs) = &combat_state {
                            if let Some(Some(p)) = cs.entities.potions.get(*potion_index) {
                                let def =
                                    sts_simulator::content::potions::get_potion_definition(p.id);
                                println!("  [BOT] Used Potion: {}", def.name);
                            }
                        }
                    }
                    _ => {}
                }
            }
            // Track cards played for combat summary
            if matches!(&decision, ClientInput::PlayCard { .. }) {
                app.combat_cards_played += 1;
            }
            decision
        } else {
            // Manual: prompt and read
            print!("\n> ");
            stdout.flush().unwrap();

            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap() == 0 {
                break;
            }
            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            // Check meta commands first
            match handle_meta_command(&line, &mut app, &engine_state, &run_state, &combat_state) {
                MetaResult::Handled => continue,
                MetaResult::Quit => break 'outer,
                MetaResult::Step(input) => input,
                MetaResult::PassThrough => {
                    // Normal game input
                    if let Some(c) = sts_simulator::cli::input::parse_input(
                        &line,
                        &engine_state,
                        &run_state,
                        &combat_state,
                    ) {
                        c
                    } else {
                        println!("  Invalid input.");
                        continue;
                    }
                }
            }
        };

        if watchdog.feed(&engine_state, &run_state, &combat_state, &input) {
            break 'outer;
        }

        // ── Tick the engine ──
        let keep_running = tick_run(
            &mut engine_state,
            &mut run_state,
            &mut combat_state,
            Some(input),
        );

        // ── Process engine ticks until stable ──
        if matches!(engine_state, EngineState::CombatProcessing) {
            let mut iters = 0;
            loop {
                if !matches!(engine_state, EngineState::CombatProcessing) {
                    break;
                }
                let keep_running =
                    tick_run(&mut engine_state, &mut run_state, &mut combat_state, None);
                if !keep_running {
                    break;
                }
                iters += 1;
                if iters > 2000 {
                    println!("  WARNING: processing loop exceeded 2000 ticks");
                    break;
                }
            }
        }

        print_emitted_outputs(&mut run_state, &mut combat_state, app.is_debug());

        // ── Combat ended? ──
        if !matches!(
            engine_state,
            EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_)
                | EngineState::EventCombat(_)
        ) {
            if let Some(ref cs) = combat_state {
                let end_hp = cs.entities.player.current_hp;
                if app.current_display >= DisplayMode::Summary {
                    println!(
                        "  [COMBAT END] HP: {} → {} | Cards played: {}",
                        app.combat_start_hp, end_hp, app.combat_cards_played
                    );
                }
                combat_state = None;

                // If this was event combat, apply event-specific post-combat logic
                if let Some(ecs) = app.stashed_event_combat.take() {
                    if ecs.reward_allowed {
                        // Replace generated rewards with event-specific rewards
                        let mut rewards = ecs.rewards;
                        if !ecs.no_cards_in_rewards {
                            // Merge card rewards from normal generation
                            if let EngineState::RewardScreen(ref existing) = engine_state {
                                for item in &existing.items {
                                    if matches!(item, RewardItem::Card { .. }) {
                                        rewards.items.push(item.clone());
                                    }
                                }
                            }
                        }
                        engine_state = EngineState::RewardScreen(rewards);
                    } else {
                        // No rewards (e.g. Colosseum fight 1) → return to event/map
                        match ecs.post_combat_return {
                            PostCombatReturn::EventRoom => {
                                engine_state = EngineState::EventRoom;
                            }
                            PostCombatReturn::MapNavigation => {
                                engine_state = EngineState::MapNavigation;
                            }
                        }
                    }
                }

                if app.mode == InputMode::SkipCombat {
                    app.mode = InputMode::Manual;
                    app.dashboard.base_mode = DisplayMode::DeepDebug;
                    app.current_display = DisplayMode::DeepDebug;
                    println!("  [MODE] → Manual");
                }
            }
        }

        if !keep_running {
            println!("\n=== GAME OVER ===");
            match &engine_state {
                EngineState::GameOver(RunResult::Victory) => println!("  VICTORY!"),
                EngineState::GameOver(RunResult::Defeat) => println!("  DEFEAT."),
                _ => println!("  (unexpected end)"),
            }
            print_run_summary(&run_state);
            break;
        }
    }
}

fn print_run_summary(rs: &RunState) {
    let profile = sts_simulator::bot::evaluator::CardEvaluator::deck_profile(rs);
    println!("\n--- RUN SUMMARY ---");
    println!("  Floor: {} (Act {})", rs.floor_num, rs.act_num);
    println!("  HP: {} / {}", rs.current_hp, rs.max_hp);
    println!("  Gold: {}", rs.gold);
    println!(
        "  Archetype: {}",
        sts_simulator::bot::evaluator::CardEvaluator::archetype_summary(&profile)
    );

    let mut potion_count = 0;
    for p in &rs.potions {
        if p.is_some() {
            potion_count += 1;
        }
    }
    println!("  Potions: {} / {}", potion_count, rs.potions.len());

    println!("  Relics ({}):", rs.relics.len());
    for r in &rs.relics {
        println!("    - {:?} (counter: {})", r.id, r.counter);
    }

    println!("  Deck ({} cards):", rs.master_deck.len());
    let mut deck_counts = std::collections::HashMap::new();
    for c in &rs.master_deck {
        let def = sts_simulator::content::cards::get_card_definition(c.id);
        let name = if c.upgrades > 0 {
            format!("{} +{}", def.name, c.upgrades)
        } else {
            def.name.to_string()
        };
        *deck_counts.entry(name).or_insert(0) += 1;
    }
    let mut deck_list: Vec<_> = deck_counts.into_iter().collect();
    deck_list.sort_by_key(|&(ref name, _)| name.clone());
    for (name, count) in deck_list {
        if count > 1 {
            println!("    - {} x{}", name, count);
        } else {
            println!("    - {}", name);
        }
    }
    println!("-------------------\n");
}

// ─── Meta commands ──────────────────────────────────────────────────────────

enum MetaResult {
    Handled,
    Quit,
    PassThrough,
    Step(ClientInput),
}

fn handle_meta_command(
    line: &str,
    app: &mut AppState,
    es: &EngineState,
    rs: &RunState,
    cs: &Option<CombatState>,
) -> MetaResult {
    match line {
        "a" | "auto-step" | "step" => {
            let input = app.agent.decide(es, rs, cs, app.is_debug());
            println!("  [BOT] Decided: {:?}", input);
            MetaResult::Step(input)
        }
        "quit" | "q" => {
            println!("Goodbye!");
            MetaResult::Quit
        }
        "help" | "h" => {
            sts_simulator::cli::input::print_help();
            MetaResult::Handled
        }
        "state" | "s" => {
            sts_simulator::cli::display::print_detailed_state(es, rs, cs);
            MetaResult::Handled
        }
        "relics" => {
            println!("  Relics ({}):", rs.relics.len());
            for r in &rs.relics {
                println!("    {:?} (counter={})", r.id, r.counter);
            }
            MetaResult::Handled
        }
        "potions" => {
            println!("  Potions:");
            for (i, p) in rs.potions.iter().enumerate() {
                match p {
                    Some(pot) => println!("    [{}] {:?}", i, pot.id),
                    None => println!("    [{}] (empty)", i),
                }
            }
            MetaResult::Handled
        }
        "purge" => {
            if let EngineState::Shop(shop) = es {
                if shop.purge_available {
                    println!(
                        "  [PURGE] Choose a card to remove for {} gold:",
                        shop.purge_cost
                    );
                    for (i, card) in rs.master_deck.iter().enumerate() {
                        let def = sts_simulator::content::cards::get_card_definition(card.id);
                        println!("    [{}] {} (uuid={})", i, def.name, card.uuid);
                    }
                    println!("  → Type 'purge <deck_idx>' to confirm.");
                } else {
                    println!("  [PURGE] Sold Out.");
                }
            } else {
                println!("  [PURGE] Not in a shop.");
            }
            MetaResult::Handled
        }
        "draw" | "discard" | "exhaust" => {
            if let Some(combat) = cs {
                let pile = match line {
                    "draw" => &combat.zones.draw_pile,
                    "discard" => &combat.zones.discard_pile,
                    "exhaust" => &combat.zones.exhaust_pile,
                    _ => unreachable!(),
                };
                println!("  {} pile ({}):", line.to_uppercase(), pile.len());
                for (i, card) in pile.iter().enumerate() {
                    let def = sts_simulator::content::cards::get_card_definition(card.id);
                    println!("    [{}] {} (uuid={})", i, def.name, card.uuid);
                }
            } else {
                println!("  Not in combat.");
            }
            MetaResult::Handled
        }
        // ── Mode switching commands ──
        "auto" => {
            app.mode = InputMode::Auto;
            println!("  [MODE] Switched to AUTO. Type Ctrl+C to stop.");
            MetaResult::Handled
        }
        "auto run" => {
            app.mode = InputMode::Auto;
            app.dashboard.base_mode = DisplayMode::Summary;
            println!("  [MODE] Full auto + summary. Running to completion...");
            MetaResult::Handled
        }
        "silent run" | "sr" => {
            app.mode = InputMode::Auto;
            app.dashboard.base_mode = DisplayMode::TotalSilence;
            println!("  [MODE] Full auto + silent. Running to completion...");
            MetaResult::Handled
        }
        "manual" => {
            app.mode = InputMode::Manual;
            println!("  [MODE] Switched to MANUAL.");
            MetaResult::Handled
        }
        "skip" => {
            if cs.is_some() {
                app.mode = InputMode::SkipCombat;
                app.dashboard.base_mode = DisplayMode::Summary;
                println!("  [MODE] Skipping combat (bot plays, summary mode)...");
            } else {
                println!("  Not in combat. Use 'auto' for full auto.");
            }
            MetaResult::Handled
        }
        "fast" => {
            if app.dashboard.base_mode == DisplayMode::DeepDebug {
                app.dashboard.base_mode = DisplayMode::Summary;
            } else {
                app.dashboard.base_mode = DisplayMode::DeepDebug;
            }
            println!("  [DISPLAY] Base mode set to {:?}", app.dashboard.base_mode);
            MetaResult::Handled
        }
        _ => MetaResult::PassThrough,
    }
}

// ─── Init combat ────────────────────────────────────────────────────────────

fn init_combat(run_state: &mut RunState, current_display: DisplayMode) -> CombatState {
    use sts_simulator::content::monsters::factory;
    use sts_simulator::map::node::RoomType;

    let encounter_id = if let Some(room_type) = run_state.map.get_current_room_type() {
        match room_type {
            RoomType::MonsterRoomElite => run_state
                .next_elite()
                .unwrap_or(factory::EncounterId::JawWorm),
            RoomType::MonsterRoomBoss => run_state
                .next_boss()
                .unwrap_or(factory::EncounterId::Hexaghost),
            _ => run_state
                .next_encounter()
                .unwrap_or(factory::EncounterId::JawWorm),
        }
    } else {
        run_state
            .next_encounter()
            .unwrap_or(factory::EncounterId::JawWorm)
    };

    if current_display >= DisplayMode::Summary {
        println!("  [Encounter: {:?}]", encounter_id);
    }

    let player = run_state.build_combat_player(0);
    let monsters = factory::build_encounter(
        encounter_id,
        &mut run_state.rng_pool.misc_rng,
        &mut run_state.rng_pool.monster_hp_rng,
        run_state.ascension_level,
    );

    let mut cs = CombatState {
        meta: sts_simulator::combat::CombatMeta {
            ascension_level: run_state.ascension_level,
            player_class: run_state.player_class,
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: sts_simulator::combat::TurnRuntime {
            turn_count: 0,
            current_phase: sts_simulator::combat::CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: sts_simulator::combat::CardZones {
            draw_pile: run_state.master_deck.clone(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: std::collections::VecDeque::new(),
            card_uuid_counter: 9999,
        },
        entities: sts_simulator::combat::EntityState {
            player,
            monsters,
            potions: run_state.potions.clone(),
            power_db: std::collections::HashMap::new(),
        },
        engine: sts_simulator::combat::EngineRuntime {
            action_queue: std::collections::VecDeque::new(),
        },
        rng: sts_simulator::combat::CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    if let Some(room_type) = run_state.map.get_current_room_type() {
        cs.meta.is_boss_fight = room_type == RoomType::MonsterRoomBoss;
        cs.meta.is_elite_fight = room_type == RoomType::MonsterRoomElite;
    }

    // Initialize initial monster intents
    let monsters_clone = cs.entities.monsters.clone();
    for m in &mut cs.entities.monsters {
        let num = cs.rng.ai_rng.random(99);
        let (move_byte, intent) = sts_simulator::content::monsters::roll_monster_move(
            &mut cs.rng.ai_rng,
            m,
            cs.meta.ascension_level,
            num,
            &monsters_clone,
        );
        m.next_move_byte = move_byte;
        m.current_intent = intent;
        m.move_history.push_back(move_byte);
    }

    cs.turn.energy = cs.entities.player.energy_master;

    // Java: CardGroup.initializeDeck() calls shuffle(shuffleRng)
    sts_simulator::rng::shuffle_with_random_long(&mut cs.zones.draw_pile, &mut cs.rng.shuffle_rng);
    let mut innate_cards = Vec::new();
    let mut normal_cards = Vec::new();
    for card in std::mem::take(&mut cs.zones.draw_pile) {
        if sts_simulator::content::cards::is_innate_card(&card) {
            innate_cards.push(card);
        } else {
            normal_cards.push(card);
        }
    }
    innate_cards.extend(normal_cards);
    cs.zones.draw_pile = innate_cards;

    cs.engine
        .action_queue
        .push_back(sts_simulator::action::Action::PreBattleTrigger);

    cs
}

// ─── Event Combat helpers ───────────────────────────────────────────────────

fn encounter_key_to_id(
    key: &str,
) -> Option<sts_simulator::content::monsters::factory::EncounterId> {
    use sts_simulator::content::monsters::factory::EncounterId;
    match key {
        "Colosseum Slavers" => Some(EncounterId::ColosseumSlavers),
        "Colosseum Nobs" => Some(EncounterId::ColosseumNobs),
        "3 Bandits" => Some(EncounterId::MaskedBandits),
        "Dead Adventurer" => Some(EncounterId::LagavulinEvent),
        "3 Fungi Beasts" => Some(EncounterId::TheMushroomLair),
        "2 Orb Walkers" => Some(EncounterId::TwoOrbWalkers),
        "Mind Bloom Boss" => Some(EncounterId::AwakenedOne), // MindBloom spawns a random act 1 boss; fallback
        _ => None,
    }
}

fn init_event_combat(
    run_state: &mut RunState,
    encounter_id: sts_simulator::content::monsters::factory::EncounterId,
    current_display: DisplayMode,
) -> CombatState {
    use sts_simulator::content::monsters::factory;

    if current_display >= DisplayMode::Summary {
        println!("  [Encounter: {:?}]", encounter_id);
    }

    let player = run_state.build_combat_player(0);
    let monsters = factory::build_encounter(
        encounter_id,
        &mut run_state.rng_pool.misc_rng,
        &mut run_state.rng_pool.monster_hp_rng,
        run_state.ascension_level,
    );

    let mut cs = CombatState {
        meta: sts_simulator::combat::CombatMeta {
            ascension_level: run_state.ascension_level,
            player_class: run_state.player_class,
            is_boss_fight: false,
            is_elite_fight: false,
            meta_changes: Vec::new(),
        },
        turn: sts_simulator::combat::TurnRuntime {
            turn_count: 0,
            current_phase: sts_simulator::combat::CombatPhase::PlayerTurn,
            energy: 3,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        },
        zones: sts_simulator::combat::CardZones {
            draw_pile: run_state.master_deck.clone(),
            hand: Vec::new(),
            discard_pile: Vec::new(),
            exhaust_pile: Vec::new(),
            limbo: Vec::new(),
            queued_cards: std::collections::VecDeque::new(),
            card_uuid_counter: 9999,
        },
        entities: sts_simulator::combat::EntityState {
            player,
            monsters,
            potions: run_state.potions.clone(),
            power_db: std::collections::HashMap::new(),
        },
        engine: sts_simulator::combat::EngineRuntime {
            action_queue: std::collections::VecDeque::new(),
        },
        rng: sts_simulator::combat::CombatRng::new(run_state.rng_pool.clone()),
        runtime: Default::default(),
    };

    // Initialize initial monster intents
    let monsters_clone = cs.entities.monsters.clone();
    for m in &mut cs.entities.monsters {
        let num = cs.rng.ai_rng.random(99);
        let (move_byte, intent) = sts_simulator::content::monsters::roll_monster_move(
            &mut cs.rng.ai_rng,
            m,
            cs.meta.ascension_level,
            num,
            &monsters_clone,
        );
        m.next_move_byte = move_byte;
        m.current_intent = intent;
        m.move_history.push_back(move_byte);
    }

    cs.turn.energy = cs.entities.player.energy_master;

    // Java: CardGroup.initializeDeck() calls shuffle(shuffleRng)
    sts_simulator::rng::shuffle_with_random_long(&mut cs.zones.draw_pile, &mut cs.rng.shuffle_rng);
    let mut innate_cards = Vec::new();
    let mut normal_cards = Vec::new();
    for card in std::mem::take(&mut cs.zones.draw_pile) {
        if sts_simulator::content::cards::is_innate_card(&card) {
            innate_cards.push(card);
        } else {
            normal_cards.push(card);
        }
    }
    innate_cards.extend(normal_cards);
    cs.zones.draw_pile = innate_cards;

    cs.engine
        .action_queue
        .push_back(sts_simulator::action::Action::PreBattleTrigger);

    cs
}
