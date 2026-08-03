use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlSession, RunDecisionAction, RunProgressOutcome,
};
use sts_simulator::state::core::{ClientInput, EngineState};

use super::owner_model::OwnerRoutine;
use super::render;

pub(super) fn apply_owner_routine(
    session: &mut RunControlSession,
    routine: OwnerRoutine,
) -> Result<RunProgressOutcome, String> {
    match routine {
        OwnerRoutine::Candidate {
            candidate_id,
            action,
        } => session.apply_owner_candidate(&candidate_id, action),
        OwnerRoutine::RewardPolicyStep => apply_reward_policy_routine(session),
        OwnerRoutine::ForcedTransition(kind) => session.apply_forced_transition(kind),
    }
}

fn apply_reward_policy_routine(
    session: &mut RunControlSession,
) -> Result<RunProgressOutcome, String> {
    if let Some(outcome) = sts_simulator::eval::run_control::apply_reward_policy_step(session)? {
        return Ok(outcome);
    }
    if let Some(outcome) =
        sts_simulator::eval::run_control::apply_reward_potion_space_step(session)?
    {
        return Ok(outcome);
    }
    let action = RunDecisionAction::Input(reward_tiny_exit_input(session)?);
    let surface = build_decision_surface(session);
    let candidate_id = super::owner_commands::owner_candidate_id_for_action(&surface, &action)?;
    session.apply_owner_candidate(&candidate_id, action)
}

fn reward_tiny_exit_input(session: &RunControlSession) -> Result<ClientInput, String> {
    let (reward, exit) = match &session.engine_state {
        EngineState::RewardScreen(reward) => (reward, ClientInput::Proceed),
        EngineState::RewardOverlay { reward_state, .. } => (reward_state, ClientInput::Cancel),
        _ => return Err("RewardTiny owner requires reward surface".to_string()),
    };
    if reward.pending_card_choice.is_some() || reward.has_card_reward_item() {
        return Err("RewardTiny owner received card reward surface".to_string());
    }
    let only_unclaimable_potions =
        sts_simulator::eval::run_control::reward_surface_has_only_unclaimable_potions(session);
    if reward.items.is_empty() || only_unclaimable_potions {
        return require_visible_input(session, exit);
    }
    Err(format!(
        "RewardTiny owner has strategic residual reward items: {:?}",
        reward.items
    ))
}

fn require_visible_input(
    session: &RunControlSession,
    input: ClientInput,
) -> Result<ClientInput, String> {
    let surface = build_decision_surface(session);
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible_input| visible_input == &input)
    {
        return Ok(input);
    }
    Err(format!(
        "input {:?} is not visible at {} among [{}]",
        input,
        surface.view.header.title,
        super::owner_commands::executable_choices_including_cancel(&surface)
            .iter()
            .map(render::render_timeline_choice)
            .collect::<Vec<_>>()
            .join(" | ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::content::potions::PotionId;
    use sts_simulator::content::relics::{RelicId, RelicState};
    use sts_simulator::eval::run_control::{RunControlConfig, RunDecisionSelectionSourceV1};
    use sts_simulator::state::rewards::{RewardItem, RewardState};

    #[test]
    fn reward_policy_routine_claims_only_one_public_candidate() {
        let mut session = reward_session(vec![
            RewardItem::Gold { amount: 19 },
            RewardItem::Potion {
                potion_id: PotionId::EssenceOfSteel,
            },
        ]);

        let outcome = apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
            .expect("owner reward policy step should claim gold");

        assert_eq!(session.run_state.gold, 118);
        assert!(session.run_state.potions[0].is_none());
        let Some(transaction) = outcome.single_decision_transaction() else {
            panic!("owner reward policy should preserve one transaction");
        };
        assert_eq!(
            transaction.selection.source,
            RunDecisionSelectionSourceV1::RewardPolicy
        );
        assert_eq!(session.decision_step, 1);
    }

    #[test]
    fn reward_policy_routine_realizes_fruit_juice_before_full_belt_exit() {
        let mut session = reward_session(vec![RewardItem::Potion {
            potion_id: PotionId::EssenceOfSteel,
        }]);
        session.run_state.potions = vec![
            Some(sts_simulator::content::potions::Potion::new(
                PotionId::FruitJuice,
                1,
            )),
            Some(sts_simulator::content::potions::Potion::new(
                PotionId::FirePotion,
                2,
            )),
            Some(sts_simulator::content::potions::Potion::new(
                PotionId::DexterityPotion,
                3,
            )),
        ];
        let before_max_hp = session.run_state.max_hp;

        apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
            .expect("owner should realize Fruit Juice before leaving the reward");

        assert_eq!(session.run_state.max_hp, before_max_hp + 5);
        assert!(session.run_state.potions[0].is_none());
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("Fruit Juice realization should preserve the reward screen");
        };
        assert!(matches!(
            reward.items.as_slice(),
            [RewardItem::Potion {
                potion_id: PotionId::EssenceOfSteel
            }]
        ));

        apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
            .expect("owner should claim the potion on its next atomic step");
        assert_eq!(
            session.run_state.potions[0]
                .as_ref()
                .map(|potion| potion.id),
            Some(PotionId::EssenceOfSteel)
        );
    }

    #[test]
    fn reward_policy_routine_preserves_all_lab_potions_by_claiming_fruit_juice_first() {
        let mut session = reward_session(vec![
            RewardItem::Potion {
                potion_id: PotionId::FearPotion,
            },
            RewardItem::Potion {
                potion_id: PotionId::FirePotion,
            },
            RewardItem::Potion {
                potion_id: PotionId::FruitJuice,
            },
        ]);
        session.run_state.potions = vec![
            Some(sts_simulator::content::potions::Potion::new(
                PotionId::FearPotion,
                1,
            )),
            None,
            None,
        ];
        let before_max_hp = session.run_state.max_hp;

        let actions = (0..4)
            .map(|_| {
                apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
                    .expect("the Lab reward transaction should remain owner-resolvable")
                    .single_decision_transaction()
                    .expect("each owner step should commit exactly one decision")
                    .action
                    .clone()
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![
                RunDecisionAction::Input(ClientInput::ClaimReward(2)),
                RunDecisionAction::Input(ClientInput::ClaimReward(0)),
                RunDecisionAction::Input(ClientInput::UsePotion {
                    potion_index: 1,
                    target: None,
                }),
                RunDecisionAction::Input(ClientInput::ClaimReward(0)),
            ]
        );
        assert_eq!(session.run_state.max_hp, before_max_hp + 5);
        assert_eq!(session.run_state.current_hp, before_max_hp + 5);
        assert_eq!(
            session
                .run_state
                .potions
                .iter()
                .map(|potion| potion.as_ref().map(|potion| potion.id))
                .collect::<Vec<_>>(),
            vec![
                Some(PotionId::FearPotion),
                Some(PotionId::FirePotion),
                Some(PotionId::FearPotion),
            ]
        );
        assert!(matches!(session.engine_state, EngineState::MapNavigation));
    }

    #[test]
    fn sozu_blocked_potion_uses_public_exit_without_deleting_reward() {
        let mut session = reward_session(vec![RewardItem::Potion {
            potion_id: PotionId::EnergyPotion,
        }]);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::Sozu));

        let outcome = apply_owner_routine(&mut session, OwnerRoutine::RewardPolicyStep)
            .expect("blocked potion should hand off to the public reward exit");

        let Some(transaction) = outcome.single_decision_transaction() else {
            panic!("reward exit should preserve one owner transaction");
        };
        assert_eq!(
            transaction.selection.source,
            RunDecisionSelectionSourceV1::OwnerPolicy
        );
        assert!(matches!(
            transaction.action,
            RunDecisionAction::Input(ClientInput::Proceed)
        ));
        let EngineState::MapOverlay { return_state } = &session.engine_state else {
            panic!("leaving unclaimed rewards should open the map overlay");
        };
        let EngineState::RewardScreen(reward) = return_state.as_ref() else {
            panic!("map overlay should retain the unclaimed reward screen");
        };
        assert_eq!(reward.items.len(), 1);
    }

    fn reward_session(items: Vec<RewardItem>) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.items = items;
        session.engine_state = EngineState::RewardScreen(reward);
        session
    }
}
