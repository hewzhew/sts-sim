use crate::state::core::{ClientInput, EngineState};
use std::collections::VecDeque;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct HistoryFrame {
    pub state_summary: String,
    pub input: String,
}

static WATCHDOG_HISTORY: Mutex<VecDeque<HistoryFrame>> = Mutex::new(VecDeque::new());

/// Watchdog that monitors the EngineState between loops to detect stagnation (infinite loops)
/// and dumps history upon detection or when the thread panics.
pub struct SimulationWatchdog {
    capacity: usize,
    stagnation_limit: usize,
    stagnation_counter: usize,
    last_state_summary: Option<String>,
}

impl SimulationWatchdog {
    /// Creates a new Watchdog and automatically registers the Panic Hook.
    pub fn new(capacity: usize, stagnation_limit: usize) -> Self {
        Self::register_panic_hook();
        Self {
            capacity,
            stagnation_limit,
            stagnation_counter: 0,
            last_state_summary: None,
        }
    }

    fn register_panic_hook() {
        // Use std::sync::Once to ensure we only register the hook once.
        static INIT: std::sync::Once = std::sync::Once::new();
        INIT.call_once(|| {
            let default_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                Self::dump_trace(&format!("Panic: {}", panic_info));
                default_hook(panic_info);
            }));
        });
    }

    /// Feed the watchdog the current state and input.
    /// Returns true if stagnation is detected (e.g., identical state after N ticks).
    pub fn feed(
        &mut self,
        engine_state: &EngineState,
        run_state: &crate::state::run::RunState,
        combat_state: &Option<crate::runtime::combat::CombatState>,
        input: &ClientInput,
    ) -> bool {
        let state_summary = summarize_state(engine_state, run_state, combat_state);

        let frame = HistoryFrame {
            state_summary: state_summary.clone(),
            input: format!("{:?}", input),
        };

        {
            let mut history = WATCHDOG_HISTORY.lock().unwrap();
            if history.len() >= self.capacity {
                history.pop_front();
            }
            history.push_back(frame);
        }

        if let Some(last_summary) = &self.last_state_summary {
            if last_summary == &state_summary {
                self.stagnation_counter += 1;
            } else {
                self.stagnation_counter = 0;
            }
        } else {
            self.stagnation_counter = 0;
        }

        self.last_state_summary = Some(state_summary);

        if self.stagnation_counter >= self.stagnation_limit {
            Self::dump_trace(&format!(
                "Stagnation detected ({} consecutive identical states)",
                self.stagnation_limit
            ));
            return true;
        }

        false
    }

    /// Dumps the current history buffer to a crash trace file.
    pub fn dump_trace(reason: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("crash_trace_{}.log", timestamp);

        if let Ok(mut file) = File::create(&filename) {
            let _ = writeln!(file, "=== STS SIMULATOR CRASH TRACE ===");
            let _ = writeln!(file, "Reason: {}", reason);
            let _ = writeln!(file, "Timestamp: {}", timestamp);
            let _ = writeln!(file, "--- HISTORY BUFFER ---");

            let history = WATCHDOG_HISTORY.lock().unwrap();
            for (i, frame) in history.iter().enumerate() {
                let step_num = history.len() - i;
                let _ = writeln!(file, "Step -{}", step_num);
                let _ = writeln!(file, "Input: {}", frame.input);
                let _ = writeln!(file, "State: {}", frame.state_summary);
                let _ = writeln!(file, "----------------------");
            }
            eprintln!(
                "\n[Watchdog] 🚨 CRITICAL: Crash trace written to {} due to: {}",
                filename, reason
            );
        } else {
            eprintln!(
                "\n[Watchdog] 🚨 CRITICAL FAILED to write crash trace. Reason was: {}",
                reason
            );
        }
    }
}

fn summarize_state(
    engine_state: &EngineState,
    run_state: &crate::state::run::RunState,
    combat_state: &Option<crate::runtime::combat::CombatState>,
) -> String {
    if let Some(cs) = combat_state {
        let mon_hp: i32 = cs.entities.monsters.iter().map(|m| m.current_hp).sum();
        return format!(
            "{:?} | Turn {} | Hand {} | E {} | HP {} | MonHP {}",
            engine_state,
            cs.turn.turn_count,
            cs.zones.hand.len(),
            cs.turn.energy,
            cs.entities.player.current_hp,
            mon_hp
        );
    }

    match engine_state {
        EngineState::EventRoom => {
            if let Some(event) = &run_state.event_state {
                format!(
                    "EventRoom {:?} | Floor {} | Screen {} | Completed {} | Internal {} | ExtraHash {}",
                    event.id,
                    run_state.floor_num,
                    event.current_screen,
                    event.completed,
                    event.internal_state,
                    extra_data_hash(&event.extra_data),
                )
            } else {
                format!("EventRoom | Floor {} | NoEventState", run_state.floor_num)
            }
        }
        EngineState::MapNavigation => format!(
            "MapNavigation | Floor {} | Pos ({}, {})",
            run_state.floor_num, run_state.map.current_x, run_state.map.current_y
        ),
        EngineState::RewardScreen(reward) => format!(
            "RewardScreen | Floor {} | Items {} | PendingCards {}",
            run_state.floor_num,
            reward.items.len(),
            reward
                .pending_card_choice
                .as_ref()
                .map(|cards| cards.len())
                .unwrap_or(0)
        ),
        EngineState::BossRelicSelect(bs) => format!(
            "BossRelicSelect | Floor {} | Relics {}",
            run_state.floor_num,
            bs.relics.len()
        ),
        EngineState::RunPendingChoice(choice_state) => format!(
            "RunPendingChoice {:?} | Floor {} | Min {} | Max {}",
            choice_state.reason,
            run_state.floor_num,
            choice_state.min_choices,
            choice_state.max_choices
        ),
        _ => format!("{:?} | Floor {}", engine_state, run_state.floor_num),
    }
}

fn extra_data_hash(extra_data: &[i32]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    extra_data.hash(&mut hasher);
    hasher.finish()
}
