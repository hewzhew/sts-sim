use super::*;

pub(super) const ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST: &str =
    "conservative_policy_selected_first_semantic_ordered_no_potion_action";

#[derive(Clone, Debug)]
pub(super) struct RolloutPolicySelection {
    pub(super) choice: IndexedActionChoice,
    pub(super) reason: &'static str,
}

pub(super) fn filtered_rollout_legal_actions(
    policy: CombatSearchV2RolloutPolicy,
    legal: Vec<CombatActionChoice>,
    combat: &CombatState,
) -> Vec<CombatActionChoice> {
    match policy {
        CombatSearchV2RolloutPolicy::Disabled => Vec::new(),
        CombatSearchV2RolloutPolicy::ConservativeNoPotion => {
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::Never, combat)
        }
    }
}

pub(super) fn choose_rollout_action(
    policy: CombatSearchV2RolloutPolicy,
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<CombatActionChoice>,
) -> Option<RolloutPolicySelection> {
    match policy {
        CombatSearchV2RolloutPolicy::Disabled => None,
        CombatSearchV2RolloutPolicy::ConservativeNoPotion => {
            choose_conservative_no_potion_action(engine, combat, legal)
        }
    }
}

fn choose_conservative_no_potion_action(
    engine: &EngineState,
    combat: &CombatState,
    legal: Vec<CombatActionChoice>,
) -> Option<RolloutPolicySelection> {
    let choices = legal
        .into_iter()
        .enumerate()
        .map(|(original_action_id, choice)| IndexedActionChoice {
            original_action_id,
            choice,
        })
        .collect();
    let ordered = order_indexed_action_choices(engine, combat, choices);
    ordered
        .choices
        .into_iter()
        .next()
        .map(|choice| RolloutPolicySelection {
            choice,
            reason: ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::blank_test_combat;

    #[test]
    fn conservative_rollout_policy_filters_potion_actions() {
        let combat = blank_test_combat();
        let legal = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: None,
                },
            ),
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
        ];

        let filtered = filtered_rollout_legal_actions(
            CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            legal,
            &combat,
        );

        assert_eq!(filtered.len(), 1);
        assert!(matches!(filtered[0].input, ClientInput::EndTurn));
    }

    #[test]
    fn conservative_rollout_policy_reports_selection_reason() {
        let combat = blank_test_combat();
        let legal = vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::EndTurn,
        )];

        let selection = choose_rollout_action(
            CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            &EngineState::CombatPlayerTurn,
            &combat,
            legal,
        )
        .expect("single legal action should be selected");

        assert_eq!(
            selection.reason,
            ROLLOUT_ACTION_REASON_CONSERVATIVE_ORDERING_FIRST
        );
        assert!(matches!(
            selection.choice.choice.input,
            ClientInput::EndTurn
        ));
    }
}
