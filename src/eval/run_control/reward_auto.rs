use crate::ai::reward_policy_v1::{
    build_reward_decision_context_v1, plan_reward_decision_v1, RewardPolicyActionV1,
    RewardPolicyConfigV1,
};
use crate::engine::run_loop::tick_run_active_with_observer;
use crate::state::core::{ClientInput, EngineState};

use super::session::RunControlSession;
use super::trace_annotation::RunControlTraceAnnotationV1;

const MAX_AUTO_REWARD_CLAIMS: usize = 16;
const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

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

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RewardAutomationReport {
    pub(super) claims: Vec<RewardAutomationClaim>,
    pub(super) trace_annotations: Vec<RunControlTraceAnnotationV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RewardAutomationClaim {
    pub(super) index: usize,
    pub(super) label: String,
}

struct RewardAutomationPlan {
    claim: RewardAutomationClaim,
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

impl RewardAutomationReport {
    pub(super) fn is_empty(&self) -> bool {
        self.claims.is_empty()
    }

    pub(super) fn render(&self) -> String {
        let mut lines = vec!["Auto reward:".to_string()];
        for claim in &self.claims {
            lines.push(format!(
                "  claimed {} at reward index {}",
                claim.label, claim.index
            ));
        }
        lines.join("\n")
    }
}

pub(super) fn apply_reward_automation(
    session: &mut RunControlSession,
) -> Result<RewardAutomationReport, String> {
    let mut report = RewardAutomationReport::default();
    for _ in 0..MAX_AUTO_REWARD_CLAIMS {
        let Some(plan) = next_auto_claim(session)? else {
            return Ok(report);
        };
        apply_claim_reward_to_stable(session, plan.claim.index)?;
        report.claims.push(plan.claim);
        report.trace_annotations.push(plan.trace_annotation);
    }
    Err(format!(
        "auto-reward exceeded {MAX_AUTO_REWARD_CLAIMS} claims without reaching a stable non-claim state"
    ))
}

fn next_auto_claim(session: &RunControlSession) -> Result<Option<RewardAutomationPlan>, String> {
    let reward = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward,
        EngineState::RewardOverlay { reward_state, .. } => reward_state,
        _ => return Ok(None),
    };
    let context = build_reward_decision_context_v1(&session.run_state, reward);
    let decision = plan_reward_decision_v1(&context, &reward_policy_config(session));
    let RewardPolicyActionV1::Claim { index, label, .. } = &decision.action else {
        return Ok(None);
    };
    let record = decision.to_noncombat_decision_record_v1();
    Ok(Some(RewardAutomationPlan {
        claim: RewardAutomationClaim {
            index: *index,
            label: label.clone(),
        },
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

fn apply_claim_reward_to_stable(
    session: &mut RunControlSession,
    index: usize,
) -> Result<(), String> {
    let mut tick = tick_run_active_with_observer(
        &mut session.engine_state,
        &mut session.run_state,
        &mut session.active_combat,
        Some(ClientInput::ClaimReward(index)),
    );
    let mut advance_ticks = 0usize;
    while tick.keep_running && matches!(session.engine_state, EngineState::CombatProcessing) {
        if advance_ticks >= MAX_STABLE_ADVANCE_TICKS {
            return Err(format!(
                "auto-reward exceeded {MAX_STABLE_ADVANCE_TICKS} engine ticks while advancing to a stable boundary"
            ));
        }
        advance_ticks += 1;
        tick = tick_run_active_with_observer(
            &mut session.engine_state,
            &mut session.run_state,
            &mut session.active_combat,
            None,
        );
    }
    Ok(())
}

pub(super) fn set_reward_automation(
    config: &mut RewardAutomationConfig,
    target: RewardAutomationTarget,
    enabled: bool,
) {
    match target {
        RewardAutomationTarget::Gold => config.claim_gold = enabled,
        RewardAutomationTarget::Potion => config.claim_potion_with_empty_slot = enabled,
        RewardAutomationTarget::Relic => {
            config.claim_safe_relic_without_sapphire_key = enabled;
        }
        RewardAutomationTarget::All => {
            config.claim_gold = enabled;
            config.claim_potion_with_empty_slot = enabled;
            config.claim_safe_relic_without_sapphire_key = enabled;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RewardAutomationTarget {
    Gold,
    Potion,
    Relic,
    All,
}

pub fn parse_reward_automation_target(raw: &str) -> Result<RewardAutomationTarget, String> {
    match raw.to_ascii_lowercase().as_str() {
        "gold" | "money" => Ok(RewardAutomationTarget::Gold),
        "potion" | "potions" | "potion-if-empty" | "potion_if_empty_slot" => {
            Ok(RewardAutomationTarget::Potion)
        }
        "relic" | "relics" | "safe-relic" | "safe_relic" => Ok(RewardAutomationTarget::Relic),
        "all" => Ok(RewardAutomationTarget::All),
        _ => Err(format!(
            "unknown auto-reward target '{raw}', expected gold|potion|relic|all"
        )),
    }
}

pub fn parse_on_off(raw: &str) -> Result<bool, String> {
    match raw.to_ascii_lowercase().as_str() {
        "on" | "true" | "yes" | "1" => Ok(true),
        "off" | "false" | "no" | "0" => Ok(false),
        _ => Err(format!("expected on|off, got '{raw}'")),
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
    use crate::state::rewards::{RewardItem, RewardState};

    #[test]
    fn auto_reward_claims_gold_and_potion_when_slot_is_empty() {
        let mut session = reward_screen_session(vec![
            RewardItem::Gold { amount: 19 },
            RewardItem::Potion {
                potion_id: PotionId::EssenceOfSteel,
            },
            RewardItem::Card { cards: Vec::new() },
        ]);

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert_eq!(session.run_state.gold, 118);
        assert_eq!(
            session.run_state.potions[0]
                .as_ref()
                .map(|potion| potion.id),
            Some(PotionId::EssenceOfSteel)
        );
        assert_eq!(report.claims.len(), 2);
        assert_eq!(report.trace_annotations.len(), 2);
        for annotation in &report.trace_annotations {
            let super::super::trace_annotation::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } = annotation
            else {
                panic!("reward auto-claim should attach noncombat policy evidence");
            };
            crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
                .expect("reward auto-claim record should validate");
            assert_eq!(
                record.site,
                crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Reward
            );
            assert_eq!(
                record.data_role,
                crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
            );
        }
        let EngineState::RewardScreen(reward) = &session.engine_state else {
            panic!("card reward should keep reward screen open");
        };
        assert!(matches!(reward.items.as_slice(), [RewardItem::Card { .. }]));
    }

    #[test]
    fn auto_reward_leaves_potion_when_slots_are_full() {
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

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert!(report.is_empty());
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
    fn auto_reward_claims_stolen_gold() {
        let mut session = reward_screen_session(vec![RewardItem::StolenGold { amount: 40 }]);

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert_eq!(session.run_state.gold, 139);
        assert_eq!(
            report.claims,
            vec![RewardAutomationClaim {
                index: 0,
                label: "40 stolen gold".to_string(),
            }]
        );
    }

    #[test]
    fn auto_reward_claims_safe_relic_with_policy_annotation() {
        let mut session = reward_screen_session(vec![RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert!(session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
        assert_eq!(
            report.claims,
            vec![RewardAutomationClaim {
                index: 0,
                label: "Relic Anchor".to_string(),
            }]
        );
        assert_eq!(report.trace_annotations.len(), 1);
        let super::super::trace_annotation::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
            record,
            ..
        } = &report.trace_annotations[0]
        else {
            panic!("safe relic auto-claim should attach noncombat policy evidence");
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
    fn auto_reward_leaves_relic_when_sapphire_key_is_available() {
        let mut session = reward_screen_session(vec![
            RewardItem::Relic {
                relic_id: crate::content::relics::RelicId::Anchor,
            },
            RewardItem::SapphireKey,
        ]);

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert!(report.is_empty());
        assert!(report.trace_annotations.is_empty());
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
    fn auto_reward_leaves_relic_when_safe_relic_claiming_is_disabled() {
        let mut session = reward_screen_session(vec![RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);
        session
            .reward_automation
            .claim_safe_relic_without_sapphire_key = false;

        let report = apply_reward_automation(&mut session).expect("automation should run");

        assert!(report.is_empty());
        assert!(report.trace_annotations.is_empty());
        assert!(!session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
    }

    fn reward_screen_session(items: Vec<RewardItem>) -> RunControlSession {
        let mut session = RunControlSession::new(super::super::RunControlConfig::default());
        let mut rewards = RewardState::new();
        rewards.items = items;
        session.engine_state = EngineState::RewardScreen(rewards);
        session
    }
}
