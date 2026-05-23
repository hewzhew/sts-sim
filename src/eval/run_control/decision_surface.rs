use crate::state::core::{ClientInput, EngineState};

use super::session::RunControlSession;
use super::view_model::{build_run_control_view_model, DecisionCandidate, RunControlViewModel};

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionSurface {
    pub view: RunControlViewModel,
    pub candidate_section_title: &'static str,
    pub inspectable_panels: &'static str,
    pub command_hint: String,
    pub visible_executable_inputs: Vec<ClientInput>,
}

pub fn build_decision_surface(session: &RunControlSession) -> DecisionSurface {
    let view = build_run_control_view_model(session);
    let visible_executable_inputs = view
        .candidates
        .iter()
        .filter_map(|candidate| candidate.action.executable_input())
        .collect::<Vec<_>>();
    DecisionSurface {
        candidate_section_title: candidate_section_title(session),
        inspectable_panels: inspectable_panels(session),
        command_hint: main_command_hint(session, &view),
        view,
        visible_executable_inputs,
    }
}

pub fn resolve_surface_candidate<'a>(
    surface: &'a DecisionSurface,
    engine_state: &EngineState,
    raw_id: &str,
) -> Option<&'a DecisionCandidate> {
    resolve_candidate_alias(&surface.view.candidates, engine_state, raw_id)
}

pub(super) fn resolve_candidate_alias<'a>(
    candidates: &'a [DecisionCandidate],
    engine_state: &EngineState,
    raw_id: &str,
) -> Option<&'a DecisionCandidate> {
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.id == raw_id) {
        return Some(candidate);
    }

    let id = raw_id.trim().to_ascii_lowercase();
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.id == id) {
        return Some(candidate);
    }
    if id.chars().all(|ch| ch.is_ascii_digit()) && !id.is_empty() {
        let structured = match engine_state {
            EngineState::Shop(_) => Some(format!("card-{id}")),
            EngineState::Campfire => Some(format!("smith-{id}")),
            _ => None,
        };
        if let Some(structured) = structured {
            if let Some(candidate) = candidates
                .iter()
                .find(|candidate| candidate.id == structured)
            {
                return Some(candidate);
            }
        }
    }

    match id.as_str() {
        "leave" | "skip" => candidates.iter().find(|candidate| {
            let label = candidate
                .label
                .trim_start()
                .to_ascii_lowercase()
                .trim_end_matches(['.', '!', '?'])
                .to_string();
            label.starts_with(&id)
        }),
        _ => None,
    }
}

pub fn surface_allows_visible_input(surface: &DecisionSurface, input: &ClientInput) -> bool {
    surface
        .visible_executable_inputs
        .iter()
        .any(|candidate_input| candidate_input == input)
}

#[cfg(test)]
pub(super) fn surface_legal_visibility_violations(session: &RunControlSession) -> Vec<String> {
    let surface = build_decision_surface(session);
    let mut violations = Vec::new();
    for candidate in &surface.view.candidates {
        if candidate.action.executable_input().is_none() {
            continue;
        }
        if candidate.id.trim().is_empty() {
            violations.push(format!(
                "visible candidate '{}' has empty id",
                candidate.label
            ));
        }
        if candidate.label.trim().is_empty() {
            violations.push(format!(
                "visible candidate '{}' has empty label",
                candidate.id
            ));
        }
    }

    if let Ok(position) = session.current_combat_position_for_actions() {
        if matches!(position.engine, EngineState::PendingChoice(_)) {
            let legal_moves = crate::sim::combat_legal_actions::get_legal_moves(
                &position.engine,
                &position.combat,
            );
            for legal in legal_moves {
                if !surface_allows_visible_input(&surface, &legal) {
                    violations.push(format!(
                        "pending choice legal input '{}' is not visible",
                        super::view_model::client_input_hint(&legal)
                    ));
                }
            }
        }
    }
    violations
}

fn candidate_section_title(session: &RunControlSession) -> &'static str {
    match &session.engine_state {
        EngineState::EventRoom => {
            if session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            }) {
                "Options:"
            } else {
                "Available action:"
            }
        }
        EngineState::PendingChoice(_) => "Selections:",
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing => "Actions:",
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => "Choices:",
        EngineState::MapNavigation => "Paths:",
        _ => "Available actions:",
    }
}

fn inspectable_panels(session: &RunControlSession) -> &'static str {
    match session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "deck | draw | discard | exhaust | relics | potions | inspect <id> | details | raw"
        }
        _ => "deck | map | relics | potions | inspect <id> | details | raw",
    }
}

fn main_command_hint(session: &RunControlSession, view: &RunControlViewModel) -> String {
    let first = view.candidates.first();
    let primary = match first {
        Some(candidate) if view.candidates.len() == 1 => {
            format!("Enter/{}: {}", candidate.id, candidate.label)
        }
        Some(_) => state_command_hint(session),
        None => "type a command".to_string(),
    };
    let views = match session.engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_) => {
            "draw | discard | exhaust | potions | relics | case | raw | help | q"
        }
        _ => "deck | map | relics | potions | case | raw | help | q",
    };
    let baseline = if session.last_completed_manual_combat_matches_capture_case() {
        " | baseline"
    } else {
        ""
    };
    format!("{primary} | {views}{baseline}")
}

fn state_command_hint(session: &RunControlSession) -> String {
    match session.engine_state {
        EngineState::Shop(_) => {
            "card-2 or card 2 | relic-1 or relic 1 | potion-0 or potion 0 | leave".to_string()
        }
        EngineState::Campfire => "rest | smith-<deck_idx> or smith <deck_idx> | recall".to_string(),
        EngineState::MapNavigation => "type a path id, e.g. 0 or 5".to_string(),
        EngineState::RewardScreen(_) => "type visible id, pick <idx>, or skip".to_string(),
        EngineState::PendingChoice(_) => "type visible selection id".to_string(),
        EngineState::CombatPlayerTurn | EngineState::CombatProcessing => {
            "cap <case_id> | n | visible action id | end".to_string()
        }
        _ => "type visible id".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::runtime::action::CardDestination;
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, ChooseOneCardChoice, CombatContext, DiscoveryChoiceState, GridSelectReason,
        PendingChoice, RoomCombatContext,
    };
    use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
    use crate::state::map::state::MapState;

    #[test]
    fn decision_surface_visible_candidates_are_accepted_by_input_gate() {
        for session in contract_sessions() {
            let surface = build_decision_surface(&session);
            for candidate in surface.view.candidates {
                let Some(input) = candidate.action.executable_input() else {
                    continue;
                };
                session
                    .validate_input_for_current_state(&input)
                    .unwrap_or_else(|err| {
                        panic!(
                            "visible candidate '{}' in '{}' should be accepted: {err}",
                            candidate.id,
                            build_decision_surface(&session).view.header.title
                        )
                    });
            }
        }
    }

    #[test]
    fn decision_surface_pending_choices_expose_all_legal_inputs() {
        for session in pending_choice_contract_sessions() {
            let violations = surface_legal_visibility_violations(&session);
            assert!(
                violations.is_empty(),
                "decision surface contract violations: {violations:?}"
            );
        }
    }

    #[test]
    fn decision_surface_pending_choice_hints_do_not_offer_end_turn() {
        let surface = build_decision_surface(&pending_grid_session());

        assert_eq!(surface.candidate_section_title, "Selections:");
        assert!(surface
            .command_hint
            .starts_with("type visible selection id"));
        assert!(
            !surface.command_hint.contains("end"),
            "pending choices must not advertise end turn: {}",
            surface.command_hint
        );
    }

    fn contract_sessions() -> Vec<RunControlSession> {
        let mut sessions = vec![
            RunControlSession::new(Default::default()),
            test_session_after_neow_at_map(),
            test_session_with_first_monster_room(),
            pending_grid_session(),
        ];
        let mut combat = test_session_with_first_monster_room();
        combat
            .apply_command(super::super::RunControlCommand::Input(
                ClientInput::SelectMapNode(0),
            ))
            .expect("map input should enter combat");
        sessions.push(combat);
        sessions
    }

    fn pending_choice_contract_sessions() -> Vec<RunControlSession> {
        vec![
            pending_grid_session(),
            pending_hand_session(),
            pending_discovery_session(),
            pending_card_reward_session(),
            pending_foreign_influence_session(),
            pending_choose_one_session(),
            pending_stance_session(),
        ]
    }

    fn pending_grid_session() -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.discard_pile = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 20),
        ];
        let choice = PendingChoice::GridSelect {
            source_pile: crate::state::core::PileType::Discard,
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };
        pending_session(choice, combat)
    }

    fn pending_hand_session() -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 20),
        ];
        let choice = PendingChoice::HandSelect {
            candidate_uuids: vec![10, 20],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::HandSelectReason::Discard,
        };
        pending_session(choice, combat)
    }

    fn pending_discovery_session() -> RunControlSession {
        let choice = PendingChoice::DiscoverySelect(DiscoveryChoiceState {
            cards: vec![CardId::Strike, CardId::Defend],
            colorless: false,
            card_type: None,
            amount: 1,
            can_skip: true,
        });
        pending_session(choice, crate::test_support::blank_test_combat())
    }

    fn pending_card_reward_session() -> RunControlSession {
        let choice = PendingChoice::CardRewardSelect {
            cards: vec![CardId::BattleTrance, CardId::ShrugItOff],
            destination: CardDestination::Hand,
            can_skip: true,
        };
        pending_session(choice, crate::test_support::blank_test_combat())
    }

    fn pending_foreign_influence_session() -> RunControlSession {
        let choice = PendingChoice::ForeignInfluenceSelect {
            cards: vec![CardId::Strike, CardId::Headbutt],
            upgraded: true,
        };
        pending_session(choice, crate::test_support::blank_test_combat())
    }

    fn pending_choose_one_session() -> RunControlSession {
        let choice = PendingChoice::ChooseOneSelect {
            choices: vec![
                ChooseOneCardChoice {
                    card_id: CardId::InfernalBlade,
                    upgrades: 0,
                },
                ChooseOneCardChoice {
                    card_id: CardId::Warcry,
                    upgrades: 1,
                },
            ],
        };
        pending_session(choice, crate::test_support::blank_test_combat())
    }

    fn pending_stance_session() -> RunControlSession {
        pending_session(
            PendingChoice::StanceChoice,
            crate::test_support::blank_test_combat(),
        )
    }

    fn pending_session(
        choice: PendingChoice,
        combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut session = RunControlSession::new(Default::default());
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    fn test_session_with_first_monster_room() -> RunControlSession {
        let mut session = test_session_after_neow_at_map();
        let mut first = MapRoomNode::new(0, 0);
        first.class = Some(RoomType::MonsterRoom);
        first.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut second = MapRoomNode::new(0, 1);
        second.class = Some(RoomType::MonsterRoom);
        session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
        session.run_state.monster_list = vec![EncounterId::JawWorm, EncounterId::Cultist];
        session
    }

    fn test_session_after_neow_at_map() -> RunControlSession {
        let mut session = RunControlSession::new(Default::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;
        session
    }
}
