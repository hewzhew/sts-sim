use sts_simulator::bot::combat::exact_turn_solver::{
    solve_exact_turn_with_config, ExactTurnConfig,
};
use sts_simulator::bot::combat::{
    diagnose_root_search_with_depth_and_runtime, SearchExactTurnMode, SearchExperimentFlags,
    SearchRuntimeBudget,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::engine::tick_until_stable_turn;
use sts_simulator::runtime::combat::{CombatCard, CombatState};
use sts_simulator::state::core::ClientInput;
use sts_simulator::state::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn starter_cards() -> Vec<CardId> {
    let mut cards = Vec::new();
    cards.extend([CardId::Strike; 5]);
    cards.extend([CardId::Defend; 4]);
    cards.push(CardId::Bash);
    cards
}

fn make_cultist_state(
    hand_ids: &[CardId],
    player_hp: i32,
    cultist_hp: i32,
    cultist_move: u8,
) -> CombatState {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.turn.turn_count = if cultist_move == 3 { 1 } else { 2 };
    combat.entities.player.current_hp = player_hp;
    let mut cultist = planned_monster(EnemyId::Cultist, cultist_move);
    cultist.current_hp = cultist_hp;
    cultist.max_hp = 50;
    combat.entities.monsters.push(cultist);

    let mut uuid = 1u32;
    for &id in hand_ids {
        combat.zones.hand.push(card(id, uuid));
        uuid += 1;
    }
    let mut remaining = starter_cards();
    for &id in hand_ids {
        let index = remaining
            .iter()
            .position(|candidate| *candidate == id)
            .unwrap_or_else(|| panic!("hand contains too many copies of {id:?}"));
        remaining.remove(index);
    }
    for id in remaining {
        combat.zones.draw_pile.push(card(id, uuid));
        uuid += 1;
    }
    combat
}

fn is_terminal(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(
        engine,
        EngineState::RewardScreen(_) | EngineState::GameOver(_)
    ) || combat.entities.player.current_hp <= 0
        || combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster.is_escaped || monster.current_hp <= 0)
}

fn card_label(card: CardId) -> &'static str {
    match card {
        CardId::Strike => "Strike",
        CardId::Defend => "Defend",
        CardId::Bash => "Bash",
        _ => "?",
    }
}

fn hand_label(hand: &[CardId]) -> String {
    hand.iter()
        .map(|card| card_label(*card))
        .collect::<Vec<_>>()
        .join(",")
}

fn input_label(input: &ClientInput, combat: &CombatState) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let name = combat
                .zones
                .hand
                .get(*card_index)
                .map(|card| format!("{:?}", card.id))
                .unwrap_or_else(|| "?".to_string());
            match target {
                Some(target) => format!("Play {name} @{target}"),
                None => format!("Play {name}"),
            }
        }
        ClientInput::EndTurn => "EndTurn".to_string(),
        other => format!("{other:?}"),
    }
}

fn optional_input_label(input: Option<&ClientInput>, combat: &CombatState) -> String {
    input
        .map(|input| input_label(input, combat))
        .unwrap_or_else(|| "<none>".to_string())
}

fn line_label(line: &[ClientInput], combat: &CombatState) -> String {
    if line.is_empty() {
        return "<empty>".to_string();
    }
    let mut engine = EngineState::CombatPlayerTurn;
    let mut replay_combat = combat.clone();
    let mut labels = Vec::new();
    for input in line {
        labels.push(input_label(input, &replay_combat));
        tick_until_stable_turn(&mut engine, &mut replay_combat, input.clone());
        if is_terminal(&engine, &replay_combat) {
            break;
        }
    }
    labels.join(" -> ")
}

fn first_played_card(line: &[ClientInput], combat: &CombatState) -> Option<CardId> {
    match line.first() {
        Some(ClientInput::PlayCard { card_index, .. }) => {
            combat.zones.hand.get(*card_index).map(|card| card.id)
        }
        _ => None,
    }
}

fn line_contains_card(line: &[ClientInput], combat: &CombatState, target_card: CardId) -> bool {
    let mut engine = EngineState::CombatPlayerTurn;
    let mut replay_combat = combat.clone();
    for input in line {
        if let ClientInput::PlayCard { card_index, .. } = input {
            if replay_combat
                .zones
                .hand
                .get(*card_index)
                .is_some_and(|card| card.id == target_card)
            {
                return true;
            }
        }
        tick_until_stable_turn(&mut engine, &mut replay_combat, input.clone());
        if is_terminal(&engine, &replay_combat) {
            break;
        }
    }
    false
}

fn controller_line(
    combat: &CombatState,
    max_steps: usize,
    experiment_flags: SearchExperimentFlags,
) -> Vec<ClientInput> {
    let mut engine = EngineState::CombatPlayerTurn;
    let mut combat = combat.clone();
    let runtime = SearchRuntimeBudget {
        exact_turn_mode: SearchExactTurnMode::Force,
        root_node_budget: 4_000,
        exact_turn_node_budget: 20_000,
        experiment_flags,
        ..SearchRuntimeBudget::default()
    };
    let mut line = Vec::new();
    for _ in 0..max_steps {
        if is_terminal(&engine, &combat) || !matches!(engine, EngineState::CombatPlayerTurn) {
            break;
        }
        let root = diagnose_root_search_with_depth_and_runtime(&engine, &combat, 4, 0, runtime);
        let input = root.chosen_move;
        let is_end_turn = matches!(input, ClientInput::EndTurn);
        line.push(input.clone());
        tick_until_stable_turn(&mut engine, &mut combat, input);
        if is_end_turn {
            break;
        }
    }
    line
}

fn exact_evidence_line(combat: &CombatState) -> Vec<ClientInput> {
    solve_exact_turn_with_config(
        &EngineState::CombatPlayerTurn,
        combat,
        ExactTurnConfig {
            max_nodes: 40_000,
            max_engine_steps: 240,
            deadline: None,
            root_inputs: None,
        },
    )
    .best_line
}

fn run_case(
    label: &str,
    hand: &[CardId],
    player_hp: i32,
    cultist_hp: i32,
    cultist_move: u8,
) -> String {
    let combat = make_cultist_state(hand, player_hp, cultist_hp, cultist_move);
    let exact = solve_exact_turn_with_config(
        &EngineState::CombatPlayerTurn,
        &combat,
        ExactTurnConfig {
            max_nodes: 40_000,
            max_engine_steps: 240,
            deadline: None,
            root_inputs: None,
        },
    );
    let current_line = controller_line(&combat, 8, SearchExperimentFlags::default());
    let exact_takeover_line = controller_line(
        &combat,
        8,
        SearchExperimentFlags {
            contested_strict_dominance_takeover: true,
            fragile_strict_dominance_takeover: true,
            advantage_strict_dominance_takeover: true,
            forbid_idle_end_turn_when_exact_prefers_play: true,
            forbid_potion_exact_takeover: true,
        },
    );
    let default_runtime = SearchRuntimeBudget {
        exact_turn_mode: SearchExactTurnMode::Force,
        root_node_budget: 4_000,
        exact_turn_node_budget: 20_000,
        ..SearchRuntimeBudget::default()
    };
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        &EngineState::CombatPlayerTurn,
        &combat,
        4,
        0,
        default_runtime,
    );
    let takeover_gate = diagnostics
        .decision_audit
        .get("takeover_gate")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let exact_verdict = diagnostics
        .decision_audit
        .get("exact_turn_verdict")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let top_moves = diagnostics
        .top_moves
        .iter()
        .take(5)
        .map(|stat| {
            format!(
                "{} score={:.1} hp={} block={} enemy={} unblocked={}",
                input_label(&stat.input, &combat),
                stat.avg_score,
                stat.projected_hp,
                stat.projected_block,
                stat.projected_enemy_total,
                stat.projected_unblocked,
            )
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let chosen_by = diagnostics
        .decision_audit
        .get("decision_trace")
        .and_then(|value| value.get("chosen_by"))
        .and_then(|value| value.as_str())
        .unwrap_or("?");
    format!(
        "{label} hand=[{}] player_hp={} cultist_hp={} move={} chosen_by={} current_line={} exact_takeover_line={} exact_first={} exact_line={} exact_truncated={} exact_nodes={} takeover_gate={} exact_verdict={} top_moves={}",
        hand_label(hand),
        player_hp,
        cultist_hp,
        cultist_move,
        chosen_by,
        line_label(&current_line, &combat),
        line_label(&exact_takeover_line, &combat),
        optional_input_label(exact.best_first_input.as_ref(), &combat),
        line_label(&exact.best_line, &combat),
        exact.truncated,
        exact.explored_nodes,
        takeover_gate,
        exact_verdict,
        top_moves,
    )
}

#[test]
fn cultist_starter_micro_world_diagnostic() {
    use CardId::{Bash, Defend, Strike};

    let cases: Vec<(&str, Vec<CardId>, i32, i32, u8)> = vec![
        (
            "ritual_all_basics",
            vec![Defend, Defend, Strike, Strike, Bash],
            80,
            50,
            3,
        ),
        (
            "ritual_attack_heavy",
            vec![Strike, Strike, Strike, Defend, Bash],
            80,
            50,
            3,
        ),
        (
            "attack_high_hp",
            vec![Defend, Defend, Strike, Strike, Bash],
            80,
            44,
            1,
        ),
        (
            "attack_near_kill",
            vec![Defend, Strike, Strike, Strike, Bash],
            80,
            21,
            1,
        ),
        (
            "attack_low_enemy_hp",
            vec![Defend, Defend, Strike, Strike, Bash],
            80,
            18,
            1,
        ),
        (
            "attack_low_player_hp",
            vec![Defend, Defend, Strike, Strike, Bash],
            8,
            44,
            1,
        ),
        (
            "attack_lethal_player_hp",
            vec![Defend, Defend, Strike, Strike, Bash],
            6,
            44,
            1,
        ),
    ];

    for (label, hand, player_hp, cultist_hp, cultist_move) in cases {
        let combat = make_cultist_state(&hand, player_hp, cultist_hp, cultist_move);
        let line = controller_line(&combat, 8, SearchExperimentFlags::default());
        let evidence_line = exact_evidence_line(&combat);
        let first_card = first_played_card(&evidence_line, &combat);
        if cultist_move == 3 {
            assert!(
                !line_contains_card(&evidence_line, &combat, Defend),
                "{label}: exact evidence line should not spend energy on Defend in ritual turn; evidence_line={} controller_line={}",
                line_label(&evidence_line, &combat),
                line_label(&line, &combat)
            );
        }
        if player_hp >= 80 && cultist_move == 1 {
            assert_ne!(
                first_card,
                Some(Defend),
                "{label}: exact evidence line should not open with Defend in high-hp attacking Cultist state; evidence_line={} controller_line={}",
                line_label(&evidence_line, &combat),
                line_label(&line, &combat)
            );
        }
        if label == "attack_lethal_player_hp" {
            assert_eq!(
                first_card,
                Some(Defend),
                "{label}: exact evidence line should open with Defend or equivalent survival play at lethal-risk hp; evidence_line={} controller_line={}",
                line_label(&evidence_line, &combat),
                line_label(&line, &combat)
            );
        }
        println!(
            "{}",
            run_case(label, &hand, player_hp, cultist_hp, cultist_move)
        );
    }
}
