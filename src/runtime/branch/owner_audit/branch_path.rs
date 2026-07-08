use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::ai::strategy::decision_pipeline::{candidate_lane_label, DecisionCandidateKind};
use sts_simulator::ai::strategy::shop_boss_preview::{
    classify_shop_boss_preview_candidate, shop_boss_preview_bundles,
};

use super::owner_model::{ChoiceAnnotation, DecisionKey, OwnerChoice};
use super::{branch_status_view, decision_delta, render, trace, Branch};

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct BranchPathStep {
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
pub(super) struct BranchPathState {
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
    deck_size: usize,
    boundary: String,
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
            .filter(|choice| choice.auto_expand_allowed())
            .filter_map(|choice| {
                choice
                    .annotation
                    .candidate()
                    .map(|decision| decision.evaluation.candidate.kind)
            })
            .collect::<Vec<_>>();
        shop_boss_preview_bundles(kinds, current_gold, 12)
            .into_iter()
            .enumerate()
            .map(|(index, bundle)| {
                let items = bundle
                    .items
                    .iter()
                    .map(|kind| BranchPathShopBossPreviewBundleItemSnapshot {
                        label: preview_item_label(choices, *kind),
                        candidate: trace::candidate_kind_value(*kind),
                    })
                    .collect::<Vec<_>>();
                Self {
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
                    total_cost: bundle.total_cost,
                    gold_after: bundle.gold_after,
                    reason: format!("{:?}", bundle.reason),
                    items,
                }
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
        Self {
            act: run.act_num,
            floor: run.floor_num,
            hp: run.current_hp,
            max_hp: run.max_hp,
            gold: run.gold,
            deck_size: run.master_deck.len(),
            boundary: branch_status_view::status_boundary_label(&branch.status),
        }
    }
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::strategy::decision_pipeline::{
        CandidateEvaluation, CandidateLane, CandidateLaneAdjudication, CandidateLaneCap,
        CandidateLaneCapSource, DecisionCandidateIr, DecisionCandidateKind, ExpansionPlan,
    };
    use sts_simulator::ai::strategy::role_saturation::LaneCap;
    use sts_simulator::content::cards::CardId;
    use sts_simulator::eval::run_control::RunControlCommand;

    use super::super::owner_model::{OwnerCandidateDecision, OwnerChoice, OwnerChoiceExpansion};
    use super::*;

    #[test]
    fn candidate_snapshot_keeps_selected_and_inspect_reason() {
        let choices = vec![
            OwnerChoice {
                key: None,
                action: RunControlCommand::Noop,
                label: "take".to_string(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::AutoAllowed,
            },
            OwnerChoice {
                key: None,
                action: RunControlCommand::Noop,
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
                action: RunControlCommand::Noop,
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
    fn shop_boss_preview_bundle_summary_excludes_inspect_only_choices() {
        fn choice(kind: DecisionCandidateKind, expansion: OwnerChoiceExpansion) -> OwnerChoice {
            OwnerChoice {
                key: None,
                action: RunControlCommand::Noop,
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

        assert_eq!(bundles.len(), 1);
        assert!(bundles[0].items.is_empty());
    }
}
