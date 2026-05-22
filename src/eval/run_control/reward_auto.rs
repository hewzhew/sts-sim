use crate::content::potions::get_potion_definition;
use crate::engine::run_loop::tick_run_active_with_observer;
use crate::state::core::{ClientInput, EngineState};
use crate::state::rewards::RewardItem;

use super::session::RunControlSession;

const MAX_AUTO_REWARD_CLAIMS: usize = 16;
const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

#[derive(Clone, Debug, PartialEq)]
pub struct RewardAutomationConfig {
    pub claim_gold: bool,
    pub claim_potion_with_empty_slot: bool,
}

impl Default for RewardAutomationConfig {
    fn default() -> Self {
        Self {
            claim_gold: true,
            claim_potion_with_empty_slot: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct RewardAutomationReport {
    pub(super) claims: Vec<RewardAutomationClaim>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct RewardAutomationClaim {
    pub(super) index: usize,
    pub(super) label: String,
}

impl RewardAutomationConfig {
    pub fn summary(&self) -> String {
        format!(
            "auto-reward: gold={} potion_if_empty_slot={}",
            on_off(self.claim_gold),
            on_off(self.claim_potion_with_empty_slot)
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
        let Some(claim) = next_auto_claim(session) else {
            return Ok(report);
        };
        apply_claim_reward_to_stable(session, claim.index)?;
        report.claims.push(claim);
    }
    Err(format!(
        "auto-reward exceeded {MAX_AUTO_REWARD_CLAIMS} claims without reaching a stable non-claim state"
    ))
}

fn next_auto_claim(session: &RunControlSession) -> Option<RewardAutomationClaim> {
    let EngineState::RewardScreen(reward) = &session.engine_state else {
        return None;
    };
    if reward.pending_card_choice.is_some() {
        return None;
    }

    reward
        .items
        .iter()
        .enumerate()
        .find_map(|(idx, item)| auto_claim_label(session, item).map(|label| (idx, label)))
        .map(|(index, label)| RewardAutomationClaim { index, label })
}

fn auto_claim_label(session: &RunControlSession, item: &RewardItem) -> Option<String> {
    match item {
        RewardItem::Gold { amount } if session.reward_automation.claim_gold => {
            Some(format!("{amount} gold"))
        }
        RewardItem::StolenGold { amount } if session.reward_automation.claim_gold => {
            Some(format!("{amount} stolen gold"))
        }
        RewardItem::Potion { potion_id } if can_auto_claim_potion(session) => {
            Some(format!("{} potion", get_potion_definition(*potion_id).name))
        }
        _ => None,
    }
}

fn can_auto_claim_potion(session: &RunControlSession) -> bool {
    session.reward_automation.claim_potion_with_empty_slot
        && session.run_state.find_empty_potion_slot().is_some()
        && !session
            .run_state
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Sozu)
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
        RewardAutomationTarget::All => {
            config.claim_gold = enabled;
            config.claim_potion_with_empty_slot = enabled;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RewardAutomationTarget {
    Gold,
    Potion,
    All,
}

pub fn parse_reward_automation_target(raw: &str) -> Result<RewardAutomationTarget, String> {
    match raw.to_ascii_lowercase().as_str() {
        "gold" | "money" => Ok(RewardAutomationTarget::Gold),
        "potion" | "potions" | "potion-if-empty" | "potion_if_empty_slot" => {
            Ok(RewardAutomationTarget::Potion)
        }
        "all" => Ok(RewardAutomationTarget::All),
        _ => Err(format!(
            "unknown auto-reward target '{raw}', expected gold|potion|all"
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

    fn reward_screen_session(items: Vec<RewardItem>) -> RunControlSession {
        let mut session = RunControlSession::new(super::super::RunControlConfig::default());
        let mut rewards = RewardState::new();
        rewards.items = items;
        session.engine_state = EngineState::RewardScreen(rewards);
        session
    }
}
