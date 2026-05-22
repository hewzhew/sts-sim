mod candidates;
mod context;
mod labels;

use crate::sim::combat::{combat_terminal, stable_boundary};
use crate::state::core::EngineState;

pub(super) use super::session::RunControlSession;
use candidates::decision_candidates;
use context::{decision_context, decision_warnings};
use labels::{boss_label, pending_choice_label};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunControlViewModel {
    pub header: RunControlHeader,
    pub decision: DecisionSummary,
    pub candidates: Vec<DecisionCandidate>,
    pub context: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunControlHeader {
    pub step: u64,
    pub title: String,
    pub location: String,
    pub config: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionSummary {
    pub label: String,
    pub status: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionCandidate {
    pub id: String,
    pub label: String,
    pub command: String,
    pub note: Option<String>,
}

pub fn build_run_control_view_model(session: &RunControlSession) -> RunControlViewModel {
    let title = decision_title(session);
    let header = RunControlHeader {
        step: session.decision_step,
        title: title.clone(),
        location: header_location(session),
        config: header_config(session),
    };

    RunControlViewModel {
        header,
        decision: decision_summary(session),
        candidates: decision_candidates(session),
        context: decision_context(session),
        warnings: decision_warnings(session),
    }
}

fn decision_title(session: &RunControlSession) -> String {
    match &session.engine_state {
        EngineState::EventRoom => {
            let Some(event) = session.run_state.event_state.as_ref() else {
                return "Event".to_string();
            };
            if event.id == crate::state::events::EventId::Neow {
                if event.current_screen == 0 {
                    "Neow intro".to_string()
                } else {
                    "Neow bonus".to_string()
                }
            } else {
                format!("{:?}", event.id)
            }
        }
        EngineState::MapNavigation => "Map choice".to_string(),
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "Card reward".to_string()
        }
        EngineState::RewardScreen(_) => "Reward screen".to_string(),
        EngineState::TreasureRoom(_) => "Treasure room".to_string(),
        EngineState::Campfire => "Campfire".to_string(),
        EngineState::Shop(_) => "Shop".to_string(),
        EngineState::CombatStart(request) => format!("Combat start {:?}", request.encounter_id),
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing => {
            "Combat decision".to_string()
        }
        EngineState::PendingChoice(choice) => {
            format!("Combat choice {}", pending_choice_label(choice))
        }
        EngineState::RunPendingChoice(choice) => format!("Run choice {:?}", choice.reason),
        EngineState::BossRelicSelect(_) => "Boss relic".to_string(),
        EngineState::GameOver(result) => format!("Game over {:?}", result),
    }
}

fn decision_summary(session: &RunControlSession) -> DecisionSummary {
    match &session.engine_state {
        EngineState::EventRoom => {
            let Some(event) = session.run_state.event_state.as_ref() else {
                return DecisionSummary {
                    label: "event room without event state".to_string(),
                    status: Some("invalid boundary".to_string()),
                };
            };
            let options = crate::engine::event_handler::get_event_options(&session.run_state);
            let status = if event.id == crate::state::events::EventId::Neow
                && event.current_screen == 0
                && options.len() == 1
            {
                Some("routine mechanical proceed".to_string())
            } else if options.iter().all(|option| option.ui.disabled) {
                Some("all visible options locked".to_string())
            } else {
                None
            };
            DecisionSummary {
                label: format!("{:?} event screen {}", event.id, event.current_screen),
                status,
            }
        }
        EngineState::MapNavigation => DecisionSummary {
            label: "choose next map node".to_string(),
            status: None,
        },
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            DecisionSummary {
                label: "choose one reward card or skip".to_string(),
                status: None,
            }
        }
        EngineState::RewardScreen(_) => DecisionSummary {
            label: "claim rewards or proceed".to_string(),
            status: None,
        },
        EngineState::TreasureRoom(_) => DecisionSummary {
            label: "open chest".to_string(),
            status: Some("routine room action".to_string()),
        },
        EngineState::Campfire => DecisionSummary {
            label: "choose campfire action".to_string(),
            status: None,
        },
        EngineState::Shop(_) => DecisionSummary {
            label: "buy, purge, or leave shop".to_string(),
            status: None,
        },
        EngineState::CombatStart(request) => DecisionSummary {
            label: format!("construct combat for {:?}", request.encounter_id),
            status: Some("transient engine boundary".to_string()),
        },
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => combat_decision_summary(session),
        EngineState::RunPendingChoice(choice) => DecisionSummary {
            label: format!(
                "choose {}-{} deck cards for {:?}",
                choice.min_choices, choice.max_choices, choice.reason
            ),
            status: None,
        },
        EngineState::BossRelicSelect(_) => DecisionSummary {
            label: "choose boss relic".to_string(),
            status: None,
        },
        EngineState::GameOver(result) => DecisionSummary {
            label: format!("{result:?}"),
            status: None,
        },
    }
}

fn combat_decision_summary(session: &RunControlSession) -> DecisionSummary {
    let Some(active) = session.active_combat.as_ref() else {
        return DecisionSummary {
            label: "combat state missing".to_string(),
            status: Some("invalid boundary".to_string()),
        };
    };
    let capture_state = session.current_active_combat_position().ok();
    let stable = capture_state
        .as_ref()
        .is_some_and(|position| stable_boundary(&position.engine, &position.combat));
    let terminal = capture_state
        .as_ref()
        .map(|position| combat_terminal(&position.engine, &position.combat));
    DecisionSummary {
        label: format!(
            "player turn {} | hp {}/{} | energy {}",
            active.combat_state.turn.turn_count,
            active.combat_state.entities.player.current_hp,
            active.combat_state.entities.player.max_hp,
            active.combat_state.turn.energy
        ),
        status: Some(format!("stable_capture={stable} terminal={terminal:?}")),
    }
}

fn header_location(session: &RunControlSession) -> String {
    let (player_hp, player_max_hp) = session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp));
    format!(
        "Act {} Floor {} | HP {}/{} | Gold {} | Boss {}",
        session.run_state.act_num,
        session.run_state.floor_num,
        player_hp,
        player_max_hp,
        session.run_state.gold,
        boss_label(&session.run_state)
    )
}

fn header_config(session: &RunControlSession) -> String {
    format!(
        "Seed={} | Ascension={} | Class={} | Deck={} | Relics={} | Potions={}",
        session.run_state.seed,
        session.run_state.ascension_level,
        session.run_state.player_class,
        session.run_state.master_deck.len(),
        session.run_state.relics.len(),
        session
            .run_state
            .potions
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    )
}
