use crate::ai::noncombat_decision_v1::{
    render_noncombat_decision_record_validation_errors, validate_noncombat_decision_record_v1,
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::content::potions::get_potion_definition;
use crate::engine::run_loop::tick_run_active_with_observer;
use crate::state::core::{ClientInput, EngineState};
use crate::state::rewards::{RewardItem, RewardState};

use super::session::RunControlSession;
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::view_model::reward_item_label;

const MAX_AUTO_REWARD_CLAIMS: usize = 16;
const MAX_STABLE_ADVANCE_TICKS: usize = 2_000;

#[derive(Clone, Debug, PartialEq)]
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
    trace_annotation: Option<RunControlTraceAnnotationV1>,
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
        if let Some(annotation) = plan.trace_annotation {
            report.trace_annotations.push(annotation);
        }
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
    if reward.pending_card_choice.is_some() {
        return Ok(None);
    }

    for (idx, item) in reward.items.iter().enumerate() {
        if let Some(plan) = auto_claim_plan(session, reward, idx, item)? {
            return Ok(Some(plan));
        }
    }
    Ok(None)
}

fn auto_claim_plan(
    session: &RunControlSession,
    reward: &RewardState,
    index: usize,
    item: &RewardItem,
) -> Result<Option<RewardAutomationPlan>, String> {
    match item {
        RewardItem::Gold { amount } if session.reward_automation.claim_gold => Ok(Some(
            reward_automation_plan(index, format!("{amount} gold"), None),
        )),
        RewardItem::StolenGold { amount } if session.reward_automation.claim_gold => Ok(Some(
            reward_automation_plan(index, format!("{amount} stolen gold"), None),
        )),
        RewardItem::Potion { potion_id } if can_auto_claim_potion(session) => {
            Ok(Some(reward_automation_plan(
                index,
                format!("{} potion", get_potion_definition(*potion_id).name),
                None,
            )))
        }
        RewardItem::Relic { relic_id } if can_auto_claim_relic_reward(session, reward) => {
            let annotation = safe_relic_reward_policy_annotation(reward, index)?;
            Ok(Some(reward_automation_plan(
                index,
                format!("Relic {relic_id:?}"),
                Some(annotation),
            )))
        }
        _ => Ok(None),
    }
}

fn reward_automation_plan(
    index: usize,
    label: String,
    trace_annotation: Option<RunControlTraceAnnotationV1>,
) -> RewardAutomationPlan {
    RewardAutomationPlan {
        claim: RewardAutomationClaim { index, label },
        trace_annotation,
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

fn can_auto_claim_relic_reward(session: &RunControlSession, reward: &RewardState) -> bool {
    session
        .reward_automation
        .claim_safe_relic_without_sapphire_key
        && !reward
            .items
            .iter()
            .any(|item| matches!(item, RewardItem::SapphireKey))
}

fn safe_relic_reward_policy_annotation(
    reward: &RewardState,
    selected_index: usize,
) -> Result<RunControlTraceAnnotationV1, String> {
    let record = safe_relic_reward_policy_record(reward, selected_index);
    validate_noncombat_decision_record_v1(&record).map_err(|errors| {
        format!(
            "safe relic reward policy produced invalid NonCombatDecisionRecordV1: {}",
            render_noncombat_decision_record_validation_errors(&errors)
        )
    })?;
    Ok(RunControlTraceAnnotationV1::NonCombatPolicyDecision { record })
}

fn safe_relic_reward_policy_record(
    reward: &RewardState,
    selected_index: usize,
) -> NonCombatDecisionRecordV1 {
    let candidates = reward
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| reward_candidate_descriptor(idx, item))
        .collect::<Vec<_>>();
    let evidence_items = reward
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| reward_candidate_evidence_item(idx, item))
        .collect::<Vec<_>>();
    let selected_candidate_id = reward
        .items
        .get(selected_index)
        .map(|item| reward_candidate_id(selected_index, item));

    NonCombatDecisionRecordV1 {
        schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
        schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
        site: DecisionSiteKindV1::Reward,
        data_role: DataRoleV1::BehaviorPolicyNotTeacher,
        information_boundary: InformationBoundaryV1::hidden_free(vec![
            InformationClassV1::PublicObservation,
        ]),
        provenance: PolicyProvenanceV1 {
            source_policy: "reward_auto_safe_relic_v1".to_string(),
            source_schema_name: "RewardAutomationConfig".to_string(),
            source_schema_version: 1,
        },
        candidates,
        evidence: EvidenceBundleV1 {
            items: evidence_items,
            assumptions: vec![
                "ordinary relic rewards are auto-claimed only when no Sapphire Key is present on the same reward screen"
                    .to_string(),
                "this is a behavior-policy convenience action, not a teacher label".to_string(),
            ],
            warnings: Vec::new(),
        },
        values: Vec::new(),
        selection: PolicySelectionV1 {
            status: PolicySelectionStatusV1::Selected,
            selected_candidate_id,
            reason:
                "auto-claimed safe relic reward because no Sapphire Key choice is present"
                    .to_string(),
            confidence: 0.85,
            selection_mode: "safe_relic_reward_gate".to_string(),
        },
    }
}

fn reward_candidate_descriptor(index: usize, item: &RewardItem) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: reward_candidate_id(index, item),
        site: DecisionSiteKindV1::Reward,
        label: reward_item_label(item),
        action_plan: PublicActionPlanV1 {
            summary: format!("claim reward index {index}"),
            command: Some(format!("claim {index}")),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: reward_candidate_uncertainty_notes(item),
    }
}

fn reward_candidate_evidence_item(index: usize, item: &RewardItem) -> EvidenceItemV1 {
    EvidenceItemV1 {
        kind: EvidenceKindV1::CandidateFacts,
        candidate_id: Some(reward_candidate_id(index, item)),
        label: reward_item_label(item),
        information_class: InformationClassV1::PublicObservation,
        components: Vec::new(),
    }
}

fn reward_candidate_id(index: usize, item: &RewardItem) -> String {
    match item {
        RewardItem::Gold { .. } => format!("reward:gold:{index}"),
        RewardItem::StolenGold { .. } => format!("reward:stolen_gold:{index}"),
        RewardItem::Card { .. } => format!("reward:card:{index}"),
        RewardItem::Relic { relic_id } => format!("reward:relic:{index}:{relic_id:?}"),
        RewardItem::Potion { potion_id } => format!("reward:potion:{index}:{potion_id:?}"),
        RewardItem::EmeraldKey => format!("reward:emerald_key:{index}"),
        RewardItem::SapphireKey => format!("reward:sapphire_key:{index}"),
    }
}

fn reward_candidate_uncertainty_notes(item: &RewardItem) -> Vec<String> {
    match item {
        RewardItem::SapphireKey => vec![
            "Sapphire Key competes with the visible relic reward and blocks safe relic auto-claim"
                .to_string(),
        ],
        RewardItem::Card { .. } => {
            vec!["card reward selection remains a separate policy boundary".to_string()]
        }
        _ => Vec::new(),
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
