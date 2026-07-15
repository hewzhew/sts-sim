use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::ai::strategy::candidate_pressure_response::assess_candidate_pressure_response;
use sts_simulator::ai::strategy::decision_pipeline::{candidate_lane_label, DecisionCandidateKind};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
use sts_simulator::ai::strategy::shop_boss_preview::{
    classify_shop_boss_preview_candidate, shop_boss_preview_bundles,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::run_control::ShopVisitContextV1;

use super::owner_model::{ChoiceAnnotation, DecisionKey, OwnerChoice};
use super::policy_expansion_plan::{PolicyExpansionClass, PolicyExpansionEvidence};
use super::{branch_status_view, decision_delta, render, trace, Branch};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct BranchPathStep {
    #[serde(default)]
    pub(super) policy_lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) policy_selection: Option<BranchPathPolicySelectionSnapshot>,
    pub(super) key: Option<DecisionKey>,
    pub(super) action_debug: String,
    pub(super) label: String,
    #[serde(default = "ChoiceAnnotationSnapshot::none")]
    pub(super) annotation: ChoiceAnnotationSnapshot,
    #[serde(default)]
    pub(super) state_before: Option<BranchPathState>,
    #[serde(default)]
    pub(super) decision_delta: Option<decision_delta::DecisionDeltaSnapshot>,
    #[serde(default)]
    pub(super) candidate_pool: Vec<BranchPathCandidateSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) shop_boss_preview_candidates: Vec<BranchPathShopBossPreviewSnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) shop_boss_preview_bundles: Vec<BranchPathShopBossPreviewBundleSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathPolicySelectionSnapshot {
    class: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    matched_pressure_axes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    matched_commitments: Vec<String>,
    original_lane: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_inspect_only: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    overrode_reject: bool,
    checkpoint_ref: String,
}

impl BranchPathPolicySelectionSnapshot {
    pub(super) fn from_evidence(evidence: &PolicyExpansionEvidence) -> Self {
        Self {
            class: policy_expansion_class_label(evidence.class).to_string(),
            matched_pressure_axes: evidence
                .matched_pressure_axes
                .iter()
                .map(serialized_enum_label)
                .collect(),
            matched_commitments: evidence
                .matched_commitments
                .iter()
                .map(serialized_enum_label)
                .collect(),
            original_lane: candidate_lane_label(evidence.original_lane).to_string(),
            original_inspect_only: evidence.original_inspect_only.clone(),
            overrode_reject: evidence.overrode_reject,
            checkpoint_ref: evidence.checkpoint_ref.clone(),
        }
    }
}

fn policy_expansion_class_label(class: PolicyExpansionClass) -> &'static str {
    match class {
        PolicyExpansionClass::Production => "production",
        PolicyExpansionClass::OrdinaryChallenger => "ordinary_challenger",
        PolicyExpansionClass::PressureRepair => "pressure_repair",
        PolicyExpansionClass::CommitmentRepair => "commitment_repair",
    }
}

fn serialized_enum_label<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathState {
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
    deck_size: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    boss: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    boss_list: Vec<String>,
    #[serde(default)]
    deck: Vec<BranchPathCardState>,
    boundary: String,
    #[serde(default)]
    relics: Vec<BranchPathRelicState>,
    maw_bank: BranchPathMawBankState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    shop_visit_context: Option<BranchPathShopVisitContext>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathCardState {
    id: CardId,
    uuid: u32,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    upgrades: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathRelicState {
    id: RelicId,
    #[serde(default, skip_serializing_if = "is_false")]
    used_up: bool,
    #[serde(
        default = "default_relic_counter",
        skip_serializing_if = "is_default_counter"
    )]
    counter: i32,
    #[serde(default, skip_serializing_if = "is_zero")]
    amount: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathMawBankState {
    owned: bool,
    active: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    used_up: bool,
    #[serde(
        default = "default_relic_counter",
        skip_serializing_if = "is_default_counter"
    )]
    counter: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathShopVisitContext {
    entry_act: u8,
    entry_floor: i32,
    entry_gold: i32,
    maw_bank_live_at_entry: bool,
    spent_gold_in_visit: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ScoreComponentSnapshot {
    by: String,
    value: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct LaneCapSnapshot {
    source: String,
    cap: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathCandidateSnapshot {
    rank: usize,
    selected: bool,
    auto_expand: bool,
    inspect_only: Option<String>,
    key: Option<DecisionKey>,
    label: String,
    annotation: ChoiceAnnotationSnapshot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathShopBossPreviewSnapshot {
    pub(super) rank: usize,
    pub(super) label: String,
    pub(super) candidate: Value,
    pub(super) class: String,
    pub(super) reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathShopBossPreviewBundleSnapshot {
    pub(super) rank: usize,
    pub(super) label: String,
    pub(super) total_cost: i32,
    pub(super) gold_after: i32,
    pub(super) reason: String,
    pub(super) items: Vec<BranchPathShopBossPreviewBundleItemSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct BranchPathShopBossPreviewBundleItemSnapshot {
    pub(super) label: String,
    pub(super) candidate: Value,
    pub(super) cost: i32,
    #[serde(default)]
    pub(super) auto_expand: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) blocked_reason: Option<String>,
}

impl BranchPathCandidateSnapshot {
    pub(super) fn from_choices(choices: &[OwnerChoice], selected_index: usize) -> Vec<Self> {
        choices
            .iter()
            .enumerate()
            .map(|(index, choice)| Self {
                rank: index + 1,
                selected: index == selected_index,
                auto_expand: choice.auto_expand_allowed(),
                inspect_only: choice.inspect_only_reason().map(str::to_string),
                key: choice.key.clone(),
                label: choice.label.clone(),
                annotation: ChoiceAnnotationSnapshot::from_annotation(&choice.annotation),
            })
            .collect()
    }
}

impl BranchPathShopBossPreviewSnapshot {
    pub(super) fn from_choices(choices: &[OwnerChoice]) -> Vec<Self> {
        let mut preview_candidates = Vec::new();
        let mut seen = Vec::new();
        for (index, choice) in choices.iter().enumerate() {
            let Some(decision) = choice.annotation.candidate() else {
                continue;
            };
            let kind = decision.evaluation.candidate.kind;
            let preview = classify_shop_boss_preview_candidate(kind);
            if !preview.include_in_v0 || seen.contains(&kind) {
                continue;
            }
            seen.push(kind);
            preview_candidates.push(Self {
                rank: index + 1,
                label: choice.label.clone(),
                candidate: trace::candidate_kind_value(kind),
                class: format!("{:?}", preview.class),
                reason: preview.reason.to_string(),
            });
        }
        preview_candidates
    }
}

impl BranchPathShopBossPreviewBundleSnapshot {
    pub(super) fn from_choices(choices: &[OwnerChoice], current_gold: i32) -> Vec<Self> {
        let kinds = choices
            .iter()
            .filter_map(|choice| {
                choice
                    .annotation
                    .candidate()
                    .map(|decision| decision.evaluation.candidate.kind)
            })
            .collect::<Vec<_>>();
        shop_boss_preview_bundles(kinds, current_gold, 12)
            .into_iter()
            .filter_map(|bundle| {
                let items = bundle
                    .items
                    .iter()
                    .map(|kind| preview_item_snapshot(choices, *kind))
                    .collect::<Vec<_>>();
                let total_cost = items.iter().map(|item| item.cost).sum::<i32>();
                if total_cost > current_gold {
                    return None;
                }
                Some((bundle, items, total_cost))
            })
            .enumerate()
            .map(|(index, (bundle, items, total_cost))| Self {
                rank: index + 1,
                label: if items.is_empty() {
                    "Leave".to_string()
                } else {
                    items
                        .iter()
                        .map(|item| item.label.as_str())
                        .collect::<Vec<_>>()
                        .join(" + ")
                },
                total_cost,
                gold_after: current_gold - total_cost,
                reason: format!("{:?}", bundle.reason),
                items,
            })
            .collect()
    }
}

fn preview_item_label(choices: &[OwnerChoice], kind: DecisionCandidateKind) -> String {
    choices
        .iter()
        .find_map(|choice| {
            let decision = choice.annotation.candidate()?;
            if decision.evaluation.candidate.kind == kind {
                Some(choice.label.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| format!("{kind:?}"))
}

fn preview_item_snapshot(
    choices: &[OwnerChoice],
    kind: DecisionCandidateKind,
) -> BranchPathShopBossPreviewBundleItemSnapshot {
    choices
        .iter()
        .find_map(|choice| {
            let decision = choice.annotation.candidate()?;
            if decision.evaluation.candidate.kind == kind {
                Some(BranchPathShopBossPreviewBundleItemSnapshot {
                    label: choice.label.clone(),
                    candidate: trace::candidate_kind_value(kind),
                    cost: choice_cost(choice, kind),
                    auto_expand: choice.auto_expand_allowed(),
                    blocked_reason: choice.inspect_only_reason().map(str::to_string),
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| BranchPathShopBossPreviewBundleItemSnapshot {
            label: format!("{kind:?}"),
            candidate: trace::candidate_kind_value(kind),
            cost: candidate_kind_cost_hint(kind),
            auto_expand: false,
            blocked_reason: Some("missing source choice".to_string()),
        })
}

fn choice_cost(choice: &OwnerChoice, kind: DecisionCandidateKind) -> i32 {
    match kind {
        DecisionCandidateKind::ShopPurge { .. } => {
            parse_gold_cost_from_label(&choice.label).unwrap_or(75)
        }
        _ => candidate_kind_cost_hint(kind),
    }
}

fn candidate_kind_cost_hint(kind: DecisionCandidateKind) -> i32 {
    match kind {
        DecisionCandidateKind::ShopBuyCard { price, .. }
        | DecisionCandidateKind::ShopBuyRelic { price, .. }
        | DecisionCandidateKind::ShopBuyPotion { price, .. } => price,
        DecisionCandidateKind::ShopPurge { .. } => 75,
        _ => 0,
    }
}

fn parse_gold_cost_from_label(label: &str) -> Option<i32> {
    let (_, suffix) = label.rsplit_once('|')?;
    suffix.trim().strip_suffix(" gold")?.trim().parse().ok()
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ChoiceAnnotationSnapshot {
    None,
    Candidate {
        lane: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        raw_lane: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        lane_caps: Vec<LaneCapSnapshot>,
        score: i32,
        #[serde(default)]
        scores: Vec<ScoreComponentSnapshot>,
        candidate: Value,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        shop_boss_preview: Option<Value>,
        admission: Option<Value>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pressure_response: Option<Value>,
        detail: String,
    },
    BossRelic {
        relic: Value,
        lane: String,
        class: String,
        detail: String,
    },
}

impl ChoiceAnnotationSnapshot {
    pub(super) fn none() -> Self {
        Self::None
    }

    pub(super) fn from_annotation(annotation: &ChoiceAnnotation) -> Self {
        match annotation {
            ChoiceAnnotation::None => Self::None,
            ChoiceAnnotation::Candidate(decision) => Self::Candidate {
                lane: candidate_lane_label(decision.evaluation.lane).to_string(),
                raw_lane: (decision.evaluation.adjudication.raw_lane != decision.evaluation.lane)
                    .then(|| {
                        candidate_lane_label(decision.evaluation.adjudication.raw_lane).to_string()
                    }),
                lane_caps: decision
                    .evaluation
                    .adjudication
                    .caps
                    .iter()
                    .map(|cap| LaneCapSnapshot {
                        source: format!("{:?}", cap.source),
                        cap: format!("{:?}", cap.cap),
                    })
                    .collect(),
                score: decision.evaluation.total_score(),
                scores: decision
                    .evaluation
                    .scores
                    .iter()
                    .map(|score| ScoreComponentSnapshot {
                        by: score.by.to_string(),
                        value: score.value,
                    })
                    .collect(),
                candidate: trace::candidate_kind_value(decision.evaluation.candidate.kind),
                shop_boss_preview: shop_boss_preview_value(decision.evaluation.candidate.kind),
                admission: decision.admission.as_ref().map(|admission| {
                    json!({
                        "card": admission.card,
                        "class": format!("{:?}", admission.class),
                    })
                }),
                pressure_response: pressure_response_value(
                    decision.evaluation.candidate.kind,
                    decision.admission.as_ref(),
                ),
                detail: render::render_candidate_decision_compact(decision),
            },
            ChoiceAnnotation::BossRelic(admission) => Self::BossRelic {
                relic: json!(admission.relic),
                lane: format!("{:?}", admission.lane),
                class: format!("{:?}", admission.class),
                detail: sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact(admission),
            },
        }
    }

    pub(super) fn detail(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Candidate { detail, .. } | Self::BossRelic { detail, .. } => Some(detail),
        }
    }
}

fn candidate_card_identity(kind: DecisionCandidateKind) -> Option<(CardId, u8)> {
    match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades }
        | DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } => Some((card, upgrades)),
        _ => None,
    }
}

fn pressure_response_value(
    kind: DecisionCandidateKind,
    admission: Option<&RewardAdmission>,
) -> Option<Value> {
    let card = candidate_card_identity(kind)?;
    let admission = admission?;
    Some(json!(assess_candidate_pressure_response(
        Some(card),
        admission,
    )))
}

fn shop_boss_preview_value(kind: DecisionCandidateKind) -> Option<Value> {
    match kind {
        DecisionCandidateKind::ShopBuyCard { .. }
        | DecisionCandidateKind::ShopBuyRelic { .. }
        | DecisionCandidateKind::ShopBuyPotion { .. }
        | DecisionCandidateKind::ShopPurge { .. }
        | DecisionCandidateKind::ShopLeave => {
            let preview = classify_shop_boss_preview_candidate(kind);
            Some(json!({
                "class": format!("{:?}", preview.class),
                "include_in_v0": preview.include_in_v0,
                "reason": preview.reason,
            }))
        }
        _ => None,
    }
}

impl BranchPathState {
    pub(super) fn from_branch(branch: &Branch) -> Self {
        let run = &branch.session.run_state;
        let maw_bank = run.relics.iter().find(|relic| relic.id == RelicId::MawBank);
        Self {
            act: run.act_num,
            floor: run.floor_num,
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            deck_size: run.master_deck.len(),
            boss: run.boss_key.as_ref().map(|boss| format!("{boss:?}")),
            boss_list: run
                .boss_list
                .iter()
                .map(|boss| format!("{boss:?}"))
                .collect(),
            deck: run
                .master_deck
                .iter()
                .map(|card| BranchPathCardState {
                    id: card.id,
                    uuid: card.uuid,
                    upgrades: card.upgrades,
                })
                .collect(),
            boundary: branch_status_view::status_boundary_label(&branch.status),
            relics: run
                .relics
                .iter()
                .map(|relic| BranchPathRelicState {
                    id: relic.id,
                    used_up: relic.used_up,
                    counter: relic.counter,
                    amount: relic.amount,
                })
                .collect(),
            maw_bank: BranchPathMawBankState {
                owned: maw_bank.is_some(),
                active: maw_bank.is_some_and(|relic| !relic.used_up),
                used_up: maw_bank.is_some_and(|relic| relic.used_up),
                counter: maw_bank.map_or(-1, |relic| relic.counter),
            },
            shop_visit_context: branch
                .session
                .shop_visit_context()
                .map(BranchPathShopVisitContext::from),
        }
    }
}

impl From<ShopVisitContextV1> for BranchPathShopVisitContext {
    fn from(context: ShopVisitContextV1) -> Self {
        Self {
            entry_act: context.entry_act,
            entry_floor: context.entry_floor,
            entry_gold: context.entry_gold,
            maw_bank_live_at_entry: context.maw_bank_live_at_entry,
            spent_gold_in_visit: context.spent_gold_in_visit,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero(value: &i32) -> bool {
    *value == 0
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

fn is_default_counter(value: &i32) -> bool {
    *value == -1
}

fn default_relic_counter() -> i32 {
    -1
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, CandidateLaneCap,
        CandidateLaneCapSource, DecisionCandidateIr, DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::ai::strategy::pressure_assessment::PressureAxis;
    use sts_simulator::ai::strategy::role_saturation::LaneCap;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::RunDecisionAction;

    use super::super::owner_model::{OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion};
    use super::*;

    #[test]
    fn omitted_relic_counters_restore_the_minus_one_sentinel() {
        let relic: BranchPathRelicState = serde_json::from_value(serde_json::json!({
            "id": "BurningBlood"
        }))
        .expect("relic snapshot");
        let maw_bank: BranchPathMawBankState = serde_json::from_value(serde_json::json!({
            "owned": false,
            "active": false
        }))
        .expect("maw bank snapshot");

        assert_eq!(relic.counter, -1);
        assert_eq!(maw_bank.counter, -1);
        assert!(serde_json::to_value(relic)
            .expect("serialize relic")
            .get("counter")
            .is_none());
        assert!(serde_json::to_value(maw_bank)
            .expect("serialize maw bank")
            .get("counter")
            .is_none());
    }

    #[test]
    fn candidate_snapshot_keeps_selected_and_inspect_reason() {
        let choices = vec![
            OwnerChoice {
                key: None,
                action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
                label: "take".to_string(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::AutoAllowed,
            },
            OwnerChoice {
                key: None,
                action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
                label: "skip".to_string(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::InspectOnly("blocked"),
            },
        ];

        let snapshot = BranchPathCandidateSnapshot::from_choices(&choices, 1);

        assert_eq!(snapshot.len(), 2);
        assert!(!snapshot[0].selected);
        assert!(snapshot[1].selected);
        assert_eq!(snapshot[1].inspect_only.as_deref(), Some("blocked"));
    }

    #[test]
    fn candidate_snapshot_includes_shop_boss_preview_classification() {
        let annotation = ChoiceAnnotation::Candidate(OwnerCandidateDecision {
            evaluation: CandidateEvaluation {
                candidate: DecisionCandidateIr {
                    kind: DecisionCandidateKind::ShopBuyCard {
                        card: CardId::FiendFire,
                        upgrades: 0,
                        price: 170,
                    },
                },
                lane: CandidateLane::Mainline,
                adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                expansion: ExpansionPlan::Auto,
                scores: Vec::new(),
            },
            admission: None,
        });

        let snapshot = ChoiceAnnotationSnapshot::from_annotation(&annotation);

        let ChoiceAnnotationSnapshot::Candidate {
            shop_boss_preview, ..
        } = snapshot
        else {
            panic!("expected candidate annotation snapshot");
        };
        let shop_boss_preview = shop_boss_preview.expect("Fiend Fire should be previewable");
        assert_eq!(shop_boss_preview["class"], "DeterministicBossRepair");
        assert_eq!(shop_boss_preview["include_in_v0"], true);
    }

    #[test]
    fn card_candidate_snapshot_exposes_pressure_response_without_changing_lane() {
        use sts_simulator::ai::strategy::reward_admission::assess_reward_admission;

        let admission = assess_reward_admission(&[], CardId::Shockwave);
        let annotation = ChoiceAnnotation::Candidate(OwnerCandidateDecision {
            evaluation: CandidateEvaluation {
                candidate: DecisionCandidateIr {
                    kind: DecisionCandidateKind::CardRewardPick {
                        card: CardId::Shockwave,
                        upgrades: 0,
                    },
                },
                lane: CandidateLane::Mainline,
                adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                expansion: ExpansionPlan::Auto,
                scores: Vec::new(),
            },
            admission: Some(admission),
        });

        let snapshot = ChoiceAnnotationSnapshot::from_annotation(&annotation);
        let ChoiceAnnotationSnapshot::Candidate {
            lane,
            score,
            pressure_response,
            ..
        } = snapshot
        else {
            panic!("expected candidate annotation snapshot");
        };

        assert_eq!(lane, "mainline");
        assert_eq!(score, 0);
        let response = pressure_response.expect("card candidate should expose pressure response");
        assert!(response["axes"]
            .as_array()
            .is_some_and(|axes| !axes.is_empty()));
    }

    #[test]
    fn candidate_snapshot_exposes_lane_adjudication_caps() {
        let annotation = ChoiceAnnotation::Candidate(OwnerCandidateDecision {
            evaluation: CandidateEvaluation {
                candidate: DecisionCandidateIr {
                    kind: DecisionCandidateKind::CardRewardPick {
                        card: CardId::IronWave,
                        upgrades: 0,
                    },
                },
                lane: CandidateLane::Probe,
                adjudication: CandidateLaneAdjudication {
                    raw_lane: CandidateLane::Mainline,
                    final_lane: CandidateLane::Probe,
                    caps: vec![CandidateLaneCap {
                        source: CandidateLaneCapSource::Acquisition,
                        cap: LaneCap::ProbeOnly,
                    }],
                },
                expansion: ExpansionPlan::Auto,
                scores: Vec::new(),
            },
            admission: None,
        });

        let snapshot = ChoiceAnnotationSnapshot::from_annotation(&annotation);

        let ChoiceAnnotationSnapshot::Candidate {
            lane,
            raw_lane,
            lane_caps,
            ..
        } = snapshot
        else {
            panic!("expected candidate annotation snapshot");
        };
        assert_eq!(lane, "probe");
        assert_eq!(raw_lane.as_deref(), Some("mainline"));
        assert_eq!(lane_caps.len(), 1);
        assert_eq!(lane_caps[0].source, "Acquisition");
        assert_eq!(lane_caps[0].cap, "ProbeOnly");
    }

    #[test]
    fn shop_boss_preview_step_summary_deduplicates_cleanup_targets() {
        use sts_simulator::ai::strategy::decision_pipeline::CleanupTarget;

        fn purge_choice(target: CleanupTarget) -> OwnerChoice {
            OwnerChoice {
                key: None,
                action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
                label: format!("Remove {target:?}"),
                annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                    evaluation: CandidateEvaluation {
                        candidate: DecisionCandidateIr {
                            kind: DecisionCandidateKind::ShopPurge { target },
                        },
                        lane: CandidateLane::Mainline,
                        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                        expansion: ExpansionPlan::Auto,
                        scores: Vec::new(),
                    },
                    admission: None,
                }),
                expansion: OwnerChoiceExpansion::AutoAllowed,
            }
        }

        let choices = vec![
            purge_choice(CleanupTarget::StarterStrike),
            purge_choice(CleanupTarget::StarterStrike),
            purge_choice(CleanupTarget::StarterDefend),
            purge_choice(CleanupTarget::StarterDefend),
        ];

        let preview = BranchPathShopBossPreviewSnapshot::from_choices(&choices);

        assert_eq!(preview.len(), 2);
        assert_eq!(preview[0].class, "DeterministicCleanup");
        assert_eq!(preview[1].class, "DeterministicCleanup");
    }

    #[test]
    fn shop_boss_preview_bundle_summary_includes_blocked_choices_for_review() {
        fn choice(kind: DecisionCandidateKind, expansion: OwnerChoiceExpansion) -> OwnerChoice {
            OwnerChoice {
                key: None,
                action: RunDecisionAction::Input(sts_simulator::state::core::ClientInput::Proceed),
                label: format!("{kind:?}"),
                annotation: ChoiceAnnotation::Candidate(OwnerCandidateDecision {
                    evaluation: CandidateEvaluation {
                        candidate: DecisionCandidateIr { kind },
                        lane: CandidateLane::Mainline,
                        adjudication: CandidateLaneAdjudication::uncapped(CandidateLane::Mainline),
                        expansion: ExpansionPlan::Auto,
                        scores: Vec::new(),
                    },
                    admission: None,
                }),
                expansion,
            }
        }

        let choices = vec![
            choice(
                DecisionCandidateKind::ShopLeave,
                OwnerChoiceExpansion::AutoAllowed,
            ),
            choice(
                DecisionCandidateKind::ShopBuyCard {
                    card: CardId::FiendFire,
                    upgrades: 0,
                    price: 152,
                },
                OwnerChoiceExpansion::InspectOnly("blocked"),
            ),
        ];

        let bundles = BranchPathShopBossPreviewBundleSnapshot::from_choices(&choices, 200);

        assert_eq!(bundles.len(), 2);
        assert!(bundles[0].items.is_empty());
        assert_eq!(
            bundles[1].label,
            "ShopBuyCard { card: FiendFire, upgrades: 0, price: 152 }"
        );
        assert_eq!(bundles[1].total_cost, 152);
        assert_eq!(bundles[1].gold_after, 48);
        assert_eq!(
            bundles[1].items[0].blocked_reason.as_deref(),
            Some("blocked")
        );
        assert!(!bundles[1].items[0].auto_expand);
    }

    #[test]
    fn path_state_exposes_relic_status_for_timeline_review() {
        use sts_simulator::content::relics::{RelicId, RelicState};
        use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};

        use super::super::branch_model::{BranchStatus, Owner};

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::BurningBlood));
        let mut maw_bank = RelicState::new(RelicId::MawBank);
        maw_bank.used_up = true;
        maw_bank.counter = -2;
        session.run_state.relics.push(maw_bank);

        let branch = Branch {
            id: 1,
            parent_id: None,
            path: Vec::new(),
            session,
            status: BranchStatus::Running {
                owner: Owner::ShopTiny,
                boundary: "Shop".to_string(),
            },
            policy_lane: super::super::branch_policy_lane::BranchPolicyLane::default(),
            combat_portfolio: None,
            auto_steps: Vec::new(),
            combat_search: Vec::new(),
            combat_search_history: Vec::new(),
            comparison_search_start: None,
            accepted_high_loss_diagnostics: Vec::new(),
        };

        let state = serde_json::to_value(BranchPathState::from_branch(&branch)).unwrap();

        assert_eq!(state["deck"].as_array().unwrap().len(), 10);
        assert_eq!(state["deck"][0]["id"], "Strike");
        assert!(state["deck"][0]["uuid"].is_number());
        assert_eq!(state["relics"][0]["id"], "BurningBlood");
        assert_eq!(state["relics"][1]["id"], "MawBank");
        assert_eq!(state["relics"][1]["used_up"], true);
        assert_eq!(state["maw_bank"]["owned"], true);
        assert_eq!(state["maw_bank"]["active"], false);
    }

    #[test]
    fn policy_selection_snapshot_keeps_repair_authorization() {
        let evidence = PolicyExpansionEvidence {
            class: PolicyExpansionClass::CommitmentRepair,
            matched_pressure_axes: vec![PressureAxis::GrowthHorizon],
            matched_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
            original_lane: CandidateLane::Reject,
            original_inspect_only: Some("candidate score rejected".to_string()),
            overrode_reject: true,
            checkpoint_ref: "branch-0/step-0".to_string(),
        };

        let value =
            serde_json::to_value(BranchPathPolicySelectionSnapshot::from_evidence(&evidence))
                .expect("policy selection should serialize");

        assert_eq!(value["class"], "commitment_repair");
        assert_eq!(value["matched_pressure_axes"][0], "growth_horizon");
        assert_eq!(value["matched_commitments"][0], "exhaust_engine");
        assert_eq!(value["original_lane"], "reject");
        assert_eq!(value["overrode_reject"], true);
        assert_eq!(value["checkpoint_ref"], "branch-0/step-0");
    }

    #[test]
    fn legacy_path_step_without_policy_selection_remains_readable() {
        let step: BranchPathStep = serde_json::from_value(json!({
            "key": null,
            "action_debug": "Noop",
            "label": "legacy"
        }))
        .expect("legacy branch path should remain readable");

        assert!(step.policy_selection.is_none());
    }
}
