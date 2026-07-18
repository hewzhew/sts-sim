use std::collections::VecDeque;

use crate::state::core::{ClientInput, EngineState};
use crate::state::events::EventId;
use crate::state::selection::{SelectionResolution, SelectionScope, SelectionTargetRef};

use super::{
    build_decision_surface, DecisionCandidateKey, RunControlSession, RunDecisionAction,
    RunProgressJournalV1,
};

const MAX_NEOW_MATERIALIZATIONS: usize = 4_096;

/// One exact, replayable decision made while closing a Neow root option.
#[derive(Clone, Debug, PartialEq)]
pub struct NeowOracleReplayStepV1 {
    pub candidate_id: String,
    pub label: String,
    pub action: RunDecisionAction,
}

/// A Neow root option after all of its immediate selections and reward screens
/// have been resolved through the authoritative run engine.
#[derive(Clone, Debug)]
pub struct CompletedNeowCandidateV1 {
    pub root_candidate_id: String,
    pub root_candidate_key: DecisionCandidateKey,
    pub root_label: String,
    pub replay: Vec<NeowOracleReplayStepV1>,
    pub journal: RunProgressJournalV1,
    pub session: RunControlSession,
}

/// A real successor that could not be closed to the map. It remains visible;
/// an execution error is not silently converted into a low strategic value.
#[derive(Clone, Debug)]
pub struct UnresolvedNeowCandidateV1 {
    pub root_candidate_id: String,
    pub root_label: String,
    pub replay: Vec<NeowOracleReplayStepV1>,
    pub boundary: String,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct NeowOracleExpansionV1 {
    pub completed: Vec<CompletedNeowCandidateV1>,
    pub unresolved: Vec<UnresolvedNeowCandidateV1>,
}

#[derive(Clone)]
struct NeowWork {
    root_candidate_id: String,
    root_candidate_key: DecisionCandidateKey,
    root_label: String,
    replay: Vec<NeowOracleReplayStepV1>,
    journal: RunProgressJournalV1,
    session: RunControlSession,
}

#[derive(Clone)]
struct ExecutableSuccessor {
    candidate_id: String,
    label: String,
    action: RunDecisionAction,
}

/// Exhaustively materialize the finite decision closure immediately produced
/// by the visible Neow choices. This function performs no strategic scoring:
/// every root option and every legal fixed-count deck selection is retained.
///
/// The input may be at Neow's intro or choice screen. Outcomes stop at the
/// first stable map boundary and carry the exact resulting session plus the
/// committed progress journal needed to replay the transition.
pub fn expand_oracle_neow_candidates_v1(
    start: &RunControlSession,
) -> Result<NeowOracleExpansionV1, String> {
    let mut common_session = start.clone();
    let mut common_replay = Vec::new();
    let mut common_journal = RunProgressJournalV1::default();
    advance_neow_intro(&mut common_session, &mut common_replay, &mut common_journal)?;

    let event = common_session
        .run_state
        .event_state
        .as_ref()
        .ok_or_else(|| "Neow oracle expansion requires event state".to_string())?;
    if event.id != EventId::Neow || event.current_screen != 1 {
        return Err(format!(
            "Neow oracle expansion requires the choice screen, got {:?} screen {}",
            event.id, event.current_screen
        ));
    }

    let roots = neow_root_successors(&common_session)?;
    if roots.is_empty() {
        return Err("Neow choice screen exposed no executable root options".to_string());
    }

    let mut queue = VecDeque::new();
    let mut expansion = NeowOracleExpansionV1::default();
    for root in roots {
        let root_key = root_candidate_key(&common_session, &root.candidate_id)?;
        let mut work = NeowWork {
            root_candidate_id: root.candidate_id.clone(),
            root_candidate_key: root_key,
            root_label: root.label.clone(),
            replay: common_replay.clone(),
            journal: common_journal.clone(),
            session: common_session.clone(),
        };
        match apply_successor(&mut work, root) {
            Ok(()) => queue.push_back(work),
            Err(reason) => expansion.unresolved.push(unresolved(&work, reason)),
        }
    }

    let mut materializations = 0usize;
    while let Some(work) = queue.pop_front() {
        if matches!(work.session.engine_state, EngineState::MapNavigation) {
            expansion.completed.push(CompletedNeowCandidateV1 {
                root_candidate_id: work.root_candidate_id,
                root_candidate_key: work.root_candidate_key,
                root_label: work.root_label,
                replay: work.replay,
                journal: work.journal,
                session: work.session,
            });
            continue;
        }

        let successors = match closure_successors(&work.session) {
            Ok(successors) if !successors.is_empty() => successors,
            Ok(_) => {
                expansion.unresolved.push(unresolved(
                    &work,
                    "Neow closure exposed no forward executable action".to_string(),
                ));
                continue;
            }
            Err(reason) => {
                expansion.unresolved.push(unresolved(&work, reason));
                continue;
            }
        };

        for successor in successors {
            materializations = materializations.saturating_add(1);
            if materializations > MAX_NEOW_MATERIALIZATIONS {
                return Err(format!(
                    "Neow closure exceeded {MAX_NEOW_MATERIALIZATIONS} authoritative transitions; likely a non-consuming engine boundary"
                ));
            }
            let mut child = work.clone();
            match apply_successor(&mut child, successor) {
                Ok(()) => queue.push_back(child),
                Err(reason) => expansion.unresolved.push(unresolved(&child, reason)),
            }
        }
    }

    Ok(expansion)
}

fn advance_neow_intro(
    session: &mut RunControlSession,
    replay: &mut Vec<NeowOracleReplayStepV1>,
    journal: &mut RunProgressJournalV1,
) -> Result<(), String> {
    let event = session
        .run_state
        .event_state
        .as_ref()
        .ok_or_else(|| "Neow oracle expansion requires event state".to_string())?;
    if event.id != EventId::Neow {
        return Err(format!(
            "Neow oracle expansion received {:?} event",
            event.id
        ));
    }
    if event.current_screen != 0 {
        return Ok(());
    }

    let surface = build_decision_surface(session);
    let candidates = surface
        .view
        .candidates
        .iter()
        .filter_map(executable_candidate)
        .collect::<Vec<_>>();
    let [intro] = candidates.as_slice() else {
        return Err(format!(
            "Neow intro expected one executable action, found {}",
            candidates.len()
        ));
    };
    apply_exact(session, replay, journal, intro.clone())
}

fn neow_root_successors(session: &RunControlSession) -> Result<Vec<ExecutableSuccessor>, String> {
    let surface = build_decision_surface(session);
    Ok(surface
        .view
        .candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::EventOption {
                    event_id: EventId::Neow,
                    screen: 1,
                    ..
                })
            )
        })
        .filter_map(executable_candidate)
        .collect())
}

fn root_candidate_key(
    session: &RunControlSession,
    candidate_id: &str,
) -> Result<DecisionCandidateKey, String> {
    build_decision_surface(session)
        .view
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == candidate_id)
        .and_then(|candidate| candidate.key)
        .ok_or_else(|| format!("Neow root candidate '{candidate_id}' lacks a typed key"))
}

fn closure_successors(session: &RunControlSession) -> Result<Vec<ExecutableSuccessor>, String> {
    match &session.engine_state {
        EngineState::EventRoom => completed_neow_event_successors(session),
        EngineState::RunPendingChoice(choice) => {
            let request = choice.selection_request(&session.run_state);
            let targets = request.targets;
            let mut selections = Vec::new();
            for count in choice.min_choices..=choice.max_choices.min(targets.len()) {
                combinations(&targets, count, 0, &mut Vec::new(), &mut selections);
            }
            if selections.is_empty() && choice.min_choices == 0 {
                selections.push(Vec::new());
            }
            selections
                .into_iter()
                .map(|selected| selection_successor(session, selected))
                .collect()
        }
        EngineState::RewardScreen(reward) => reward_successors(session, reward),
        EngineState::MapOverlay { .. } => Err(
            "Neow closure reached a map overlay while immediate rewards remained unclosed"
                .to_string(),
        ),
        other => Err(format!(
            "Neow closure reached unsupported boundary {}",
            engine_boundary_name(other)
        )),
    }
}

fn completed_neow_event_successors(
    session: &RunControlSession,
) -> Result<Vec<ExecutableSuccessor>, String> {
    let event = session
        .run_state
        .event_state
        .as_ref()
        .ok_or_else(|| "Neow closure returned to EventRoom without event state".to_string())?;
    if event.id != EventId::Neow || !event.completed {
        return Err(format!(
            "Neow closure returned to unfinished {:?} screen {}",
            event.id, event.current_screen
        ));
    }
    Ok(build_decision_surface(session)
        .view
        .candidates
        .iter()
        .filter_map(executable_candidate)
        .collect())
}

fn selection_successor(
    session: &RunControlSession,
    selected: Vec<SelectionTargetRef>,
) -> Result<ExecutableSuccessor, String> {
    let action = RunDecisionAction::Input(ClientInput::SubmitSelection(SelectionResolution {
        scope: SelectionScope::Deck,
        selected,
    }));
    let surface = build_decision_surface(session);
    let candidate = surface
        .view
        .candidates
        .iter()
        .find(|candidate| candidate.action.executable_action().as_ref() == Some(&action))
        .or_else(|| {
            surface.view.candidates.iter().find(|candidate| {
                matches!(
                    candidate.key,
                    Some(DecisionCandidateKey::SelectionSubmit { .. })
                )
            })
        })
        .ok_or_else(|| "run selection has no bindable decision-surface candidate".to_string())?;
    Ok(ExecutableSuccessor {
        candidate_id: candidate.id.clone(),
        label: format!(
            "{} [{} target(s)]",
            candidate.label,
            selection_count(&action)
        ),
        action,
    })
}

fn selection_count(action: &RunDecisionAction) -> usize {
    match action {
        RunDecisionAction::Input(ClientInput::SubmitSelection(resolution)) => {
            resolution.selected.len()
        }
        _ => 0,
    }
}

fn reward_successors(
    session: &RunControlSession,
    reward: &crate::state::rewards::RewardState,
) -> Result<Vec<ExecutableSuccessor>, String> {
    let surface = build_decision_surface(session);
    let candidates = surface
        .view
        .candidates
        .iter()
        .filter(|candidate| {
            let Some(action) = candidate.action.executable_action() else {
                return false;
            };
            if reward.pending_card_choice.is_some() {
                return !matches!(action, RunDecisionAction::Input(ClientInput::Cancel));
            }
            if reward.items.is_empty() {
                return matches!(action, RunDecisionAction::Input(ClientInput::Proceed));
            }
            matches!(
                action,
                RunDecisionAction::Input(ClientInput::ClaimReward(_))
                    | RunDecisionAction::SingingBowlCardReward { .. }
            )
        })
        .filter_map(executable_candidate)
        .collect::<Vec<_>>();
    Ok(candidates)
}

fn executable_candidate(
    candidate: &super::view_model::DecisionCandidate,
) -> Option<ExecutableSuccessor> {
    Some(ExecutableSuccessor {
        candidate_id: candidate.id.clone(),
        label: candidate.label.clone(),
        action: candidate.action.executable_action()?,
    })
}

fn apply_successor(work: &mut NeowWork, successor: ExecutableSuccessor) -> Result<(), String> {
    apply_exact(
        &mut work.session,
        &mut work.replay,
        &mut work.journal,
        successor,
    )
}

fn apply_exact(
    session: &mut RunControlSession,
    replay: &mut Vec<NeowOracleReplayStepV1>,
    journal: &mut RunProgressJournalV1,
    successor: ExecutableSuccessor,
) -> Result<(), String> {
    let outcome =
        session.apply_owner_candidate(&successor.candidate_id, successor.action.clone())?;
    if outcome.progress_steps.len() != 1 {
        return Err(format!(
            "candidate '{}' committed {} progress steps; expected exactly one",
            successor.candidate_id,
            outcome.progress_steps.len()
        ));
    }
    journal.append_committed_steps(outcome.progress_steps)?;
    replay.push(NeowOracleReplayStepV1 {
        candidate_id: successor.candidate_id,
        label: successor.label,
        action: successor.action,
    });
    Ok(())
}

fn combinations(
    targets: &[SelectionTargetRef],
    count: usize,
    start: usize,
    current: &mut Vec<SelectionTargetRef>,
    out: &mut Vec<Vec<SelectionTargetRef>>,
) {
    if current.len() == count {
        out.push(current.clone());
        return;
    }
    let remaining = count.saturating_sub(current.len());
    if targets.len().saturating_sub(start) < remaining {
        return;
    }
    for index in start..targets.len() {
        current.push(targets[index]);
        combinations(targets, count, index + 1, current, out);
        current.pop();
    }
}

fn unresolved(work: &NeowWork, reason: String) -> UnresolvedNeowCandidateV1 {
    UnresolvedNeowCandidateV1 {
        root_candidate_id: work.root_candidate_id.clone(),
        root_label: work.root_label.clone(),
        replay: work.replay.clone(),
        boundary: build_decision_surface(&work.session).view.header.title,
        reason,
    }
}

fn engine_boundary_name(state: &EngineState) -> &'static str {
    match state {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::CombatProcessing => "combat_processing",
        EngineState::RewardScreen(_) => "reward_screen",
        EngineState::RewardOverlay { .. } => "reward_overlay",
        EngineState::TreasureRoom(_) => "treasure_room",
        EngineState::Campfire => "campfire",
        EngineState::Shop(_) => "shop",
        EngineState::MapNavigation => "map_navigation",
        EngineState::MapOverlay { .. } => "map_overlay",
        EngineState::EventRoom => "event_room",
        EngineState::CombatStart(_) => "combat_start",
        EngineState::PendingChoice(_) => "combat_pending_choice",
        EngineState::RunPendingChoice(_) => "run_pending_choice",
        EngineState::BossRelicSelect(_) => "boss_relic_select",
        EngineState::GameOver(_) => "game_over",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::eval::run_control::RunControlConfig;

    #[test]
    fn seed006_materializes_every_neow_root_to_a_replayable_map_state() {
        let session = RunControlSession::new(RunControlConfig {
            seed: 6,
            ascension_level: 0,
            final_act: false,
            ..RunControlConfig::default()
        });

        let expansion = expand_oracle_neow_candidates_v1(&session)
            .expect("seed006 Neow closure should execute through authoritative run actions");
        let roots = expansion
            .completed
            .iter()
            .map(|candidate| candidate.root_candidate_id.as_str())
            .collect::<BTreeSet<_>>();
        let root_counts = roots
            .iter()
            .map(|root| {
                (
                    *root,
                    expansion
                        .completed
                        .iter()
                        .filter(|candidate| candidate.root_candidate_id == **root)
                        .count(),
                )
            })
            .collect::<Vec<_>>();
        println!(
            "seed006 completed={} root_counts={root_counts:?}",
            expansion.completed.len()
        );

        assert!(
            expansion.unresolved.is_empty(),
            "all seed006 Neow outcomes should close: {:?}",
            expansion
                .unresolved
                .iter()
                .map(|candidate| (&candidate.root_label, &candidate.reason))
                .collect::<Vec<_>>()
        );
        assert_eq!(roots, BTreeSet::from(["0", "1", "2", "3"]));
        assert!(expansion.completed.len() > roots.len());
        for candidate in expansion.completed {
            println!(
                "root={} label={} hp={}/{} gold={} deck={:?} relics={:?}",
                candidate.root_candidate_id,
                candidate.root_label,
                candidate.session.run_state.current_hp,
                candidate.session.run_state.max_hp,
                candidate.session.run_state.gold,
                candidate
                    .session
                    .run_state
                    .master_deck
                    .iter()
                    .map(|card| (card.id, card.upgrades))
                    .collect::<Vec<_>>(),
                candidate
                    .session
                    .run_state
                    .relics
                    .iter()
                    .map(|relic| relic.id)
                    .collect::<Vec<_>>()
            );
            assert!(matches!(
                candidate.session.engine_state,
                EngineState::MapNavigation
            ));
            assert_eq!(candidate.journal.len(), candidate.replay.len());
            assert!(candidate.replay.len() >= 2);
        }
    }
}
