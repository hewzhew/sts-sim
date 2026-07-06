use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

use super::focus::{focus_witness_line, CombatReviewFocus};

#[path = "key_card_lifecycle/targets.rs"]
mod targets;
#[path = "key_card_lifecycle/tracking.rs"]
mod tracking;
#[path = "key_card_lifecycle/types.rs"]
mod types;
#[path = "key_card_lifecycle/zones.rs"]
mod zones;

pub(super) use targets::key_card_targets;
pub(super) use types::{KeyCardLifecycleReport, KeyCardReason};

use tracking::{
    finish_lifecycles, note_played_key_card, note_seen_zones, report_without_focus,
    tracked_key_cards,
};

pub(super) fn key_card_lifecycle(
    root: &CombatPosition,
    focus: Option<&CombatReviewFocus>,
) -> Option<KeyCardLifecycleReport> {
    let mut tracked_cards = tracked_key_cards(&root.combat);
    if tracked_cards.is_empty() {
        return None;
    }

    let Some(focus) = focus else {
        return Some(report_without_focus(tracked_cards));
    };
    let witness = focus_witness_line(focus);
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut replayed_actions = 0usize;
    let mut truncated = false;
    let mut timed_out = false;

    for (index, action) in witness.actions.iter().cloned().enumerate() {
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
        let step_index = index + 1;
        note_played_key_card(
            &mut tracked_cards,
            step_index,
            &action.action_key,
            &action.input,
        );
        let step = stepper.apply_to_stable(
            &position,
            action.input,
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        replayed_actions = replayed_actions.saturating_add(1);
        truncated |= step.truncated;
        timed_out |= step.timed_out;
        position = step.position;
        note_seen_zones(&mut tracked_cards, step_index, &position.combat);
        if truncated || timed_out || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }

    let truncated_by_preview = witness
        .action_count
        .is_some_and(|count| count > witness.actions.len());
    Some(KeyCardLifecycleReport {
        schema: "key_card_lifecycle_v0",
        contract: "exact_replay_key_card_visibility_and_play_timing_no_strategy_verdict",
        basis_line: focus.selected_review,
        witness_action_count: witness.action_count,
        replayed_actions,
        truncated_by_preview,
        truncated,
        timed_out,
        tracked_cards: finish_lifecycles(tracked_cards, replayed_actions, &position.combat),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::CombatSearchV2ActionPreview;
    use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::runtime::combat::CombatCard;
    use sts_simulator::sim::combat::CombatPosition;
    use sts_simulator::state::core::ClientInput;
    use sts_simulator::state::core::EngineState;
    use sts_simulator::test_support::{blank_test_combat, test_monster};

    use super::types::CardZoneLabel;

    fn focus_with_action(action_key: String, input: ClientInput) -> CombatReviewFocus {
        let full_action = CombatSearchV2ActionPreview {
            action_key: action_key.clone(),
            input: input.clone(),
        };
        CombatReviewFocus {
            selected_review: "test_review",
            reason: "test",
            progress: super::super::search_types::SearchDiagnosticProgressFacts {
                source: "best_complete",
                terminal: SearchTerminalLabel::Loss,
                estimated: false,
                final_hp: 0,
                hp_loss: 80,
                turns: 1,
                potions_used: 0,
                cards_played: 1,
                living_enemy_count: 1,
                total_enemy_hp: 10,
                half_dead_enemy_count: 0,
                visible_incoming_damage: None,
                action_count: Some(1),
                exact_prefix_action_count: Some(1),
                action_key_preview: vec![action_key],
                input_preview: vec![input],
                full_action_preview: vec![full_action],
            },
        }
    }

    #[test]
    fn records_played_triggered_scaling_card_from_focus_replay() {
        let mut combat = blank_test_combat();
        let demon_form = CombatCard::new(CardId::DemonForm, 42);
        combat.meta.master_deck_snapshot = vec![demon_form.clone()];
        combat.zones.hand = vec![demon_form];
        combat.entities.monsters = vec![test_monster(
            sts_simulator::content::monsters::EnemyId::Cultist,
        )];
        let input = ClientInput::PlayCard {
            card_index: 0,
            target: None,
        };
        let focus = focus_with_action(
            "combat/play_card/hand:0/card:Demon Form+0#42/target:none".to_string(),
            input,
        );
        let report = key_card_lifecycle(
            &CombatPosition::new(EngineState::CombatPlayerTurn, combat),
            Some(&focus),
        )
        .expect("Demon Form should be tracked");

        assert_eq!(report.tracked_cards.len(), 1);
        let lifecycle = &report.tracked_cards[0];
        assert_eq!(lifecycle.card, "Demon Form+0");
        assert_eq!(lifecycle.reason, KeyCardReason::StrengthScaling);
        assert_eq!(lifecycle.initial_zone, CardZoneLabel::Hand);
        assert!(lifecycle.played_in_replay);
        assert_eq!(
            lifecycle.first_play.as_ref().map(|play| play.step_index),
            Some(1)
        );
    }
}
