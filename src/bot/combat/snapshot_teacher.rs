use crate::engine::core::tick_until_stable_turn;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, RunResult};
use crate::state::EngineState;
use serde::Serialize;
use std::time::{Duration, Instant};

use super::{
    diagnose_root_search_with_runtime, diagnose_root_search_with_runtime_and_root_inputs,
    SearchExactTurnMode, SearchRuntimeBudget,
};

#[derive(Clone, Debug)]
pub struct SnapshotTeacherConfig {
    pub horizon_decisions: usize,
    pub root_search_budget: u32,
    pub continuation_search_budget: u32,
    pub wall_time_ms: u64,
}

impl Default for SnapshotTeacherConfig {
    fn default() -> Self {
        Self {
            horizon_decisions: 8,
            root_search_budget: 120,
            continuation_search_budget: 80,
            wall_time_ms: 120,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotTeacherReport {
    pub schema_version: &'static str,
    pub mode: &'static str,
    pub horizon_decisions: usize,
    pub candidate_count: usize,
    pub reference_input: String,
    pub reference_outcome: SnapshotTeacherOutcome,
    pub candidates: Vec<SnapshotTeacherCandidateReport>,
    pub dominating_candidate_count: usize,
    pub best_dominating_index: Option<usize>,
    pub elapsed_ms: u128,
    pub timed_out: bool,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotTeacherCandidateReport {
    pub index: usize,
    pub input: String,
    pub is_reference: bool,
    pub outcome: SnapshotTeacherOutcome,
    pub dominates_reference: bool,
    pub dominance_reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SnapshotTeacherOutcome {
    pub stop_reason: String,
    pub decisions: usize,
    pub engine_state: String,
    pub terminal_tier: i32,
    pub combat_cleared: bool,
    pub player_dead: bool,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: i32,
    pub total_enemy_hp: i32,
    pub living_enemy_count: usize,
    pub hand_count: usize,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
}

pub fn evaluate_snapshot_teacher_shadow(
    engine: &EngineState,
    combat: &CombatState,
    root_inputs: Option<Vec<ClientInput>>,
    config: SnapshotTeacherConfig,
) -> SnapshotTeacherReport {
    let started = Instant::now();
    let deadline = started + Duration::from_millis(config.wall_time_ms.max(1));
    let root_inputs = root_inputs.unwrap_or_default();
    let notes = vec![
        "snapshot_teacher_shadow_v0 uses current Java truth import; it does not maintain a live FullRunEnv mirror".to_string(),
        "dominance is partial-order evidence only; it is not a scalar value label and does not control live play".to_string(),
    ];

    let runtime = runtime_budget(deadline);
    let reference_diag = diagnose_root_search_with_runtime_and_root_inputs(
        engine,
        combat,
        config.root_search_budget,
        runtime,
        if root_inputs.is_empty() {
            None
        } else {
            Some(root_inputs.clone())
        },
    );
    let reference_input = reference_diag.chosen_move.clone();
    let reference_outcome = rollout_from_root(
        engine,
        combat,
        &reference_input,
        config.horizon_decisions,
        config.continuation_search_budget,
        deadline,
    );

    let mut candidates = Vec::new();
    let mut best_dominating_index = None;
    let mut timed_out = false;
    for (index, input) in root_inputs.iter().enumerate() {
        if Instant::now() >= deadline {
            timed_out = true;
            break;
        }
        let outcome = rollout_from_root(
            engine,
            combat,
            input,
            config.horizon_decisions,
            config.continuation_search_budget,
            deadline,
        );
        let (dominates_reference, dominance_reasons) =
            dominance_reasons(&outcome, &reference_outcome);
        if dominates_reference && best_dominating_index.is_none() {
            best_dominating_index = Some(index);
        }
        candidates.push(SnapshotTeacherCandidateReport {
            index,
            input: format!("{input:?}"),
            is_reference: input == &reference_input,
            outcome,
            dominates_reference,
            dominance_reasons,
        });
    }
    let dominating_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.dominates_reference)
        .count();

    SnapshotTeacherReport {
        schema_version: "snapshot_teacher_shadow_v0",
        mode: "shadow_only",
        horizon_decisions: config.horizon_decisions,
        candidate_count: root_inputs.len(),
        reference_input: format!("{reference_input:?}"),
        reference_outcome,
        candidates,
        dominating_candidate_count,
        best_dominating_index,
        elapsed_ms: started.elapsed().as_millis(),
        timed_out,
        notes,
    }
}

fn rollout_from_root(
    engine: &EngineState,
    combat: &CombatState,
    root_input: &ClientInput,
    horizon_decisions: usize,
    continuation_search_budget: u32,
    deadline: Instant,
) -> SnapshotTeacherOutcome {
    let mut engine = engine.clone();
    let mut combat = combat.clone();
    let mut decisions = 0usize;
    let alive = tick_until_stable_turn(&mut engine, &mut combat, root_input.clone());
    if !alive {
        return outcome_from_state("terminal_after_root", decisions, &engine, &combat);
    }

    while decisions < horizon_decisions {
        if Instant::now() >= deadline {
            return outcome_from_state("wall_time_cap", decisions, &engine, &combat);
        }
        if !is_combat_decision_state(&engine) {
            return outcome_from_state("left_combat_decision_space", decisions, &engine, &combat);
        }
        if is_terminal_like(&engine, &combat) {
            return outcome_from_state("terminal", decisions, &engine, &combat);
        }
        let diag = diagnose_root_search_with_runtime(
            &engine,
            &combat,
            continuation_search_budget,
            runtime_budget(deadline),
        );
        let alive = tick_until_stable_turn(&mut engine, &mut combat, diag.chosen_move);
        decisions += 1;
        if !alive {
            return outcome_from_state("terminal", decisions, &engine, &combat);
        }
    }
    outcome_from_state("horizon_decision_cap", decisions, &engine, &combat)
}

fn runtime_budget(deadline: Instant) -> SearchRuntimeBudget {
    SearchRuntimeBudget {
        wall_clock_deadline: Some(deadline),
        root_node_budget: 48,
        engine_step_budget: 120,
        exact_turn_node_budget: 1_200,
        audit_budget: 4,
        exact_turn_mode: SearchExactTurnMode::Auto,
        experiment_flags: Default::default(),
    }
}

fn outcome_from_state(
    stop_reason: &str,
    decisions: usize,
    engine: &EngineState,
    combat: &CombatState,
) -> SnapshotTeacherOutcome {
    let combat_cleared = combat_cleared(combat)
        || matches!(engine, EngineState::GameOver(RunResult::Victory))
        || combat.turn.counters.victory_triggered;
    let player_dead = combat.entities.player.current_hp <= 0
        || matches!(engine, EngineState::GameOver(RunResult::Defeat));
    let terminal_tier = if combat_cleared {
        2
    } else if player_dead {
        -2
    } else if !is_combat_decision_state(engine) {
        1
    } else {
        0
    };
    SnapshotTeacherOutcome {
        stop_reason: stop_reason.to_string(),
        decisions,
        engine_state: format!("{engine:?}"),
        terminal_tier,
        combat_cleared,
        player_dead,
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: i32::from(combat.turn.energy),
        total_enemy_hp: total_enemy_hp(combat),
        living_enemy_count: living_enemy_count(combat),
        hand_count: combat.zones.hand.len(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
    }
}

fn dominance_reasons(
    candidate: &SnapshotTeacherOutcome,
    reference: &SnapshotTeacherOutcome,
) -> (bool, Vec<String>) {
    if candidate.player_dead {
        return (false, Vec::new());
    }
    if candidate.terminal_tier < reference.terminal_tier {
        return (false, Vec::new());
    }
    if candidate.player_hp < reference.player_hp {
        return (false, Vec::new());
    }
    if candidate.total_enemy_hp > reference.total_enemy_hp {
        return (false, Vec::new());
    }
    if candidate.living_enemy_count > reference.living_enemy_count {
        return (false, Vec::new());
    }

    let mut reasons = Vec::new();
    if candidate.terminal_tier > reference.terminal_tier {
        reasons.push("better_terminal_tier".to_string());
    }
    if candidate.combat_cleared && !reference.combat_cleared {
        reasons.push("clears_combat_when_reference_does_not".to_string());
    }
    if candidate.player_hp > reference.player_hp {
        reasons.push("higher_player_hp".to_string());
    }
    if candidate.total_enemy_hp < reference.total_enemy_hp {
        reasons.push("lower_total_enemy_hp".to_string());
    }
    if candidate.living_enemy_count < reference.living_enemy_count {
        reasons.push("fewer_living_enemies".to_string());
    }
    (!reasons.is_empty(), reasons)
}

fn is_combat_decision_state(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    )
}

fn is_terminal_like(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(engine, EngineState::GameOver(_))
        || combat.entities.player.current_hp <= 0
        || combat_cleared(combat)
}

fn combat_cleared(combat: &CombatState) -> bool {
    !combat.entities.monsters.is_empty()
        && combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::state_sync::build_combat_state_from_snapshots;
    use crate::protocol::java::{
        build_combat_affordance_snapshot, build_live_observation_snapshot,
        build_live_truth_snapshot,
    };
    use serde_json::{json, Value};
    use std::path::PathBuf;

    fn load_fixture_root() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("protocol_truth_samples")
            .join("sentry_livecomm")
            .join("frame.json");
        let text = std::fs::read_to_string(path).expect("fixture");
        serde_json::from_str(&text).expect("fixture json")
    }

    fn build_fixture_combat() -> CombatState {
        let root = load_fixture_root();
        let game_state = root.get("game_state").expect("game_state");
        let truth = build_live_truth_snapshot(game_state);
        let observation = build_live_observation_snapshot(game_state);
        let relics = game_state.get("relics").unwrap_or(&Value::Null);
        build_combat_state_from_snapshots(&truth, &observation, relics)
    }

    fn outcome_for_dominance(
        player_dead: bool,
        combat_cleared: bool,
        player_hp: i32,
        total_enemy_hp: i32,
        living_enemy_count: usize,
    ) -> SnapshotTeacherOutcome {
        SnapshotTeacherOutcome {
            stop_reason: "test".to_string(),
            decisions: 0,
            engine_state: "test".to_string(),
            terminal_tier: if combat_cleared {
                2
            } else if player_dead {
                -2
            } else {
                0
            },
            combat_cleared,
            player_dead,
            player_hp,
            player_block: 0,
            energy: 0,
            total_enemy_hp,
            living_enemy_count,
            hand_count: 0,
            draw_count: 0,
            discard_count: 0,
            exhaust_count: 0,
        }
    }

    #[test]
    fn dead_candidate_never_dominates_reference() {
        let candidate = outcome_for_dominance(true, false, 0, 5, 1);
        let reference = outcome_for_dominance(true, false, 0, 20, 2);

        let (dominates, reasons) = dominance_reasons(&candidate, &reference);

        assert!(!dominates);
        assert!(reasons.is_empty());
    }

    #[test]
    fn snapshot_teacher_shadow_consumes_protocol_root_actions() {
        let combat = build_fixture_combat();
        let action_space = json!({
            "combat_action_space": {
                "screen_type": "NONE",
                "actions": [
                    {
                        "action_id": "end_turn",
                        "kind": "end_turn",
                        "command": "END",
                        "target_required": false,
                        "target_options": []
                    },
                    {
                        "action_id": "strike-left",
                        "kind": "play_card",
                        "command": "PLAY 0 0",
                        "target_required": true,
                        "target_options": [0],
                        "target_index": 0,
                        "hand_index": 0,
                        "card_uuid": "e8d57bd7-e3d8-493c-839b-634afb7f6bf0",
                        "card_id": "Strike_R"
                    },
                    {
                        "action_id": "defend",
                        "kind": "play_card",
                        "command": "PLAY 3",
                        "target_required": false,
                        "target_options": [],
                        "hand_index": 3,
                        "card_uuid": "58844a0a-c106-423e-9214-99684d9a22d4",
                        "card_id": "Defend_R"
                    }
                ]
            }
        });
        let affordance = build_combat_affordance_snapshot(&action_space, &combat)
            .expect("affordance parse")
            .expect("combat action space");
        let protocol_inputs = affordance.protocol_root_inputs();

        let report = evaluate_snapshot_teacher_shadow(
            &EngineState::CombatPlayerTurn,
            &combat,
            Some(protocol_inputs),
            SnapshotTeacherConfig {
                horizon_decisions: 0,
                root_search_budget: 4,
                continuation_search_budget: 1,
                wall_time_ms: 10_000,
            },
        );

        assert_eq!(report.schema_version, "snapshot_teacher_shadow_v0");
        assert_eq!(report.mode, "shadow_only");
        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.candidates.len(), 3);
        assert!(
            report.reference_input.contains("EndTurn")
                || report.reference_input.contains("PlayCard"),
            "shadow teacher should choose a real reference action, got {}",
            report.reference_input
        );
        assert!(report
            .candidates
            .iter()
            .any(|candidate| candidate.input.contains("EndTurn")));
        assert!(report
            .candidates
            .iter()
            .any(|candidate| candidate.input.contains("PlayCard")));
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("does not maintain a live FullRunEnv mirror")),
            "report should document that this is snapshot-based shadow evidence"
        );
    }
}
