use crate::ai::reward_policy_v1::{
    build_reward_decision_context_v1, plan_reward_decision_v1, RewardPolicyActionV1,
    RewardPolicyClassV1, RewardPolicyConfigV1,
};
use crate::state::core::{ClientInput, EngineState};

use super::session::{RunControlSession, RunProgressOutcome};
use super::trace_annotation::RunControlTraceAnnotationV1;

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RewardAutomationConfig {
    pub claim_gold: bool,
    pub claim_potion_with_empty_slot: bool,
    pub claim_safe_relic_without_sapphire_key: bool,
}

impl Default for RewardAutomationConfig {
    fn default() -> Self {
        Self {
            claim_gold: true,
            claim_potion_with_empty_slot: true,
            claim_safe_relic_without_sapphire_key: true,
        }
    }
}

struct RewardPolicyPlan {
    reward_index: usize,
    trace_annotation: RunControlTraceAnnotationV1,
}

impl RewardAutomationConfig {
    pub fn summary(&self) -> String {
        format!(
            "auto-reward: gold={} potion_if_empty_slot={} safe_relic_without_sapphire_key={}",
            on_off(self.claim_gold),
            on_off(self.claim_potion_with_empty_slot),
            on_off(self.claim_safe_relic_without_sapphire_key)
        )
    }
}

pub fn apply_reward_policy_step(
    session: &mut RunControlSession,
) -> Result<Option<RunProgressOutcome>, String> {
    let Some(plan) = next_reward_policy_claim(session)? else {
        return Ok(None);
    };
    let action = super::RunDecisionAction::Input(ClientInput::ClaimReward(plan.reward_index));
    let surface = super::build_decision_surface(session);
    let matches = surface
        .view
        .candidates
        .iter()
        .filter(|candidate| candidate.action.executable_action().as_ref() == Some(&action))
        .collect::<Vec<_>>();
    let [candidate] = matches.as_slice() else {
        return Err(format!(
            "reward policy action {action:?} matched {} public candidates",
            matches.len()
        ));
    };
    let candidate_id = candidate.id.clone();
    let transaction =
        session.execute_reward_candidate_transaction(&candidate_id, plan.trace_annotation)?;
    Ok(Some(transaction.project_progress_outcome(session)))
}

pub fn reward_surface_has_only_unclaimable_potions(session: &RunControlSession) -> bool {
    let reward = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward,
        EngineState::RewardOverlay { reward_state, .. } => reward_state,
        _ => return false,
    };
    let context = build_reward_decision_context_v1(&session.run_state, reward);
    !context.candidates.is_empty()
        && context.candidates.iter().all(|candidate| {
            matches!(
                candidate.class,
                RewardPolicyClassV1::PotionNoEmptySlot | RewardPolicyClassV1::PotionBlockedBySozu
            )
        })
}

fn next_reward_policy_claim(
    session: &RunControlSession,
) -> Result<Option<RewardPolicyPlan>, String> {
    let reward = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward,
        EngineState::RewardOverlay { reward_state, .. } => reward_state,
        _ => return Ok(None),
    };
    let context = build_reward_decision_context_v1(&session.run_state, reward);
    let decision = plan_reward_decision_v1(&context, &reward_policy_config(session));
    let RewardPolicyActionV1::Claim { index, .. } = &decision.action else {
        return Ok(None);
    };
    let record = decision.to_noncombat_decision_record_v1();
    Ok(Some(RewardPolicyPlan {
        reward_index: *index,
        trace_annotation: super::noncombat_policy_annotation::noncombat_policy_annotation(
            "reward policy",
            record,
        )?,
    }))
}

fn reward_policy_config(session: &RunControlSession) -> RewardPolicyConfigV1 {
    RewardPolicyConfigV1 {
        claim_gold: session.reward_automation.claim_gold,
        claim_potion_with_empty_slot: session.reward_automation.claim_potion_with_empty_slot,
        claim_safe_relic_without_sapphire_key: session
            .reward_automation
            .claim_safe_relic_without_sapphire_key,
    }
}

fn on_off(enabled: bool) -> &'static str {
    if enabled {
        "on"
    } else {
        "off"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::potions::PotionId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::eval::run_control::RunDecisionSelectionSourceV1;
    use crate::state::rewards::{RewardItem, RewardState};

    #[test]
    fn reward_policy_claims_exactly_one_public_candidate_per_step() {
        let mut session = reward_screen_session(vec![
            RewardItem::Gold { amount: 19 },
            RewardItem::Potion {
                potion_id: PotionId::EssenceOfSteel,
            },
            RewardItem::Card { cards: Vec::new() },
        ]);

        let gold = apply_reward_policy_step(&mut session)
            .expect("gold policy step should run")
            .expect("gold should be selected");

        assert_eq!(session.run_state.gold, 118);
        assert!(session.run_state.potions[0].is_none());
        assert_reward_policy_transaction(&gold, 0, 1);

        let potion = apply_reward_policy_step(&mut session)
            .expect("potion policy step should run")
            .expect("potion should be selected on the next boundary");

        assert_eq!(
            session.run_state.potions[0]
                .as_ref()
                .map(|potion| potion.id),
            Some(PotionId::EssenceOfSteel)
        );
        assert_reward_policy_transaction(&potion, 1, 2);
        assert!(apply_reward_policy_step(&mut session)
            .expect("card boundary should be inspected")
            .is_none());
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("card reward should keep reward screen open");
        };
        assert!(matches!(reward.items.as_slice(), [RewardItem::Card { .. }]));
    }

    #[test]
    fn reward_policy_leaves_potion_when_slots_are_full() {
        let mut session = reward_screen_session(vec![RewardItem::Potion {
            potion_id: PotionId::EssenceOfSteel,
        }]);
        session.run_state.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::FirePotion,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::DexterityPotion,
                2,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::StrengthPotion,
                3,
            )),
        ];

        let outcome = apply_reward_policy_step(&mut session).expect("policy should inspect reward");

        assert!(outcome.is_none());
        assert_eq!(session.decision_step, 0);
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("full potion slots should leave reward screen open");
        };
        assert!(matches!(
            reward.items.as_slice(),
            [RewardItem::Potion {
                potion_id: PotionId::EssenceOfSteel
            }]
        ));
    }

    #[test]
    fn reward_policy_leaves_sozu_blocked_potion_for_explicit_exit() {
        let mut session = reward_screen_session(vec![RewardItem::Potion {
            potion_id: PotionId::EnergyPotion,
        }]);
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::Sozu));
        assert!(session.run_state.find_empty_potion_slot().is_some());

        let outcome = apply_reward_policy_step(&mut session)
            .expect("reward policy should inspect blocked potion");

        assert!(outcome.is_none());
        assert!(session.run_state.potions.iter().all(Option::is_none));
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("blocked potion should remain on the reward screen until exit");
        };
        assert_eq!(reward.items.len(), 1);
        assert_eq!(session.decision_step, 0);
    }

    #[test]
    fn reward_policy_claims_stolen_gold_as_one_transaction() {
        let mut session = reward_screen_session(vec![RewardItem::StolenGold { amount: 40 }]);

        let outcome = apply_reward_policy_step(&mut session)
            .expect("policy should run")
            .expect("stolen gold should be selected");

        assert_eq!(session.run_state.gold, 139);
        assert_reward_policy_transaction(&outcome, 0, 1);
    }

    #[test]
    fn reward_policy_claims_safe_relic_with_policy_annotation() {
        let mut session = reward_screen_session(vec![RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);

        let outcome = apply_reward_policy_step(&mut session)
            .expect("policy should run")
            .expect("safe relic should be selected");

        assert!(session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
        assert_reward_policy_transaction(&outcome, 0, 1);
        let super::super::trace_annotation::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            ..
        } = &outcome.trace_annotations[0]
        else {
            panic!("safe relic policy claim should attach noncombat policy evidence");
        };
        crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
            .expect("safe relic reward record should validate");
        assert_eq!(
            record.site,
            crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Reward
        );
        assert_eq!(
            record.selection.status,
            crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
        );
    }

    #[test]
    fn reward_policy_leaves_relic_when_sapphire_key_is_available() {
        let mut session = reward_screen_session(vec![
            RewardItem::Relic {
                relic_id: crate::content::relics::RelicId::Anchor,
            },
            RewardItem::SapphireKey,
        ]);

        let outcome = apply_reward_policy_step(&mut session).expect("policy should inspect reward");

        assert!(outcome.is_none());
        assert!(!session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("sapphire/relic choice should remain on reward screen");
        };
        assert!(matches!(
            reward.items.as_slice(),
            [
                RewardItem::Relic {
                    relic_id: crate::content::relics::RelicId::Anchor
                },
                RewardItem::SapphireKey
            ]
        ));
    }

    #[test]
    fn reward_policy_leaves_relic_when_safe_relic_claiming_is_disabled() {
        let mut session = reward_screen_session(vec![RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);
        session
            .reward_automation
            .claim_safe_relic_without_sapphire_key = false;

        let outcome = apply_reward_policy_step(&mut session).expect("policy should inspect reward");

        assert!(outcome.is_none());
        assert!(!session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
    }

    fn assert_reward_policy_transaction(
        outcome: &RunProgressOutcome,
        before_step: u64,
        after_step: u64,
    ) {
        let Some(transaction) = outcome.single_decision_transaction() else {
            panic!("one reward policy step should preserve exactly one transaction");
        };
        assert_eq!(
            transaction.selection.source,
            RunDecisionSelectionSourceV1::RewardPolicy
        );
        assert_eq!(transaction.before.decision_step, before_step);
        assert_eq!(transaction.after.decision_step, after_step);
        assert_eq!(outcome.trace_annotations.len(), 1);
        let super::super::trace_annotation::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            ..
        } = &outcome.trace_annotations[0]
        else {
            panic!("reward policy transaction should attach noncombat policy evidence");
        };
        crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
            .expect("reward policy record should validate");
        assert_eq!(
            record.site,
            crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Reward
        );
        assert_eq!(
            record.data_role,
            crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
        );
    }

    fn reward_screen_session(items: Vec<RewardItem>) -> RunControlSession {
        let mut session = RunControlSession::new(super::super::RunControlConfig::default());
        let mut rewards = RewardState::new();
        rewards.items = items;
        session.engine_state = EngineState::RewardScreen(rewards);
        session
    }
}
