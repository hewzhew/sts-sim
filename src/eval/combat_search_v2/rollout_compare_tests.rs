use super::*;
use crate::state::core::ClientInput;

#[test]
fn first_action_diff_finds_first_key_change() {
    let left = vec![
        trace(0, "combat/end_turn"),
        trace(
            1,
            "combat/play_card/hand:0/card:Strike_R+0#1/target:monster_slot:0",
        ),
    ];
    let right = vec![
        trace(0, "combat/end_turn"),
        trace(
            1,
            "combat/play_card/hand:1/card:Bash+0#2/target:monster_slot:0",
        ),
    ];

    let diff = first_action_diff(Some(&left), Some(&right)).expect("diff should exist");

    assert_eq!(diff.action_index, 1);
    assert_eq!(diff.left_action_id, Some(1));
    assert_eq!(diff.right_action_id, Some(1));
    assert!(diff.left_action_key.unwrap().contains("Strike_R"));
    assert!(diff.right_action_key.unwrap().contains("Bash"));
}

#[test]
fn comparison_summary_counts_verdicts_and_hp_delta() {
    let mut summary = CombatSearchV2RolloutPolicyComparisonSummary::default();
    observe_case(
        &mut summary,
        &CombatSearchV2RolloutPolicyComparisonCase {
            id: "case".to_string(),
            verdict: CombatSearchV2RolloutPolicyComparisonVerdict::RightBetter,
            right_minus_left_final_hp: Some(3),
            right_minus_left_hp_loss: Some(-3),
            right_minus_left_turns: Some(0),
            right_minus_left_cards_played: Some(1),
            left: run_summary(10),
            right: run_summary(13),
            first_action_diff: Some(CombatSearchV2RolloutPolicyFirstActionDiff {
                action_index: 0,
                left_action_id: None,
                left_action_key: None,
                left_action_debug: None,
                right_action_id: None,
                right_action_key: None,
                right_action_debug: None,
            }),
        },
    );

    assert_eq!(summary.cases_compared, 1);
    assert_eq!(summary.right_better, 1);
    assert_eq!(summary.right_minus_left_final_hp_total, 3);
    assert_eq!(summary.first_action_diff_cases, 1);
}

fn trace(action_id: usize, action_key: &str) -> CombatSearchV2ActionTrace {
    CombatSearchV2ActionTrace {
        step_index: action_id,
        action_id,
        action_key: action_key.to_string(),
        action_debug: action_key.to_string(),
        input: ClientInput::EndTurn,
    }
}

fn run_summary(final_hp: i32) -> CombatSearchV2RolloutPolicyComparisonRun {
    CombatSearchV2RolloutPolicyComparisonRun {
        policy: "test",
        terminal: Some(SearchTerminalLabel::Win),
        proof_status: SearchProofStatus::DeadlineHit,
        complete_trajectory_found: true,
        final_hp: Some(final_hp),
        hp_loss: Some(0),
        turns: Some(1),
        potions_used: Some(0),
        cards_played: Some(1),
        action_count: Some(1),
        nodes_expanded: 1,
        nodes_generated: 1,
        nodes_to_first_win: Some(1),
        deadline_hit: true,
        node_budget_hit: false,
    }
}
