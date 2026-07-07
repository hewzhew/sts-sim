use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_simulator::ai::strategy::decision_pipeline::candidate_lane_label;

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
pub(super) struct BranchPathCandidateSnapshot {
    rank: usize,
    selected: bool,
    auto_expand: bool,
    inspect_only: Option<String>,
    key: Option<DecisionKey>,
    label: String,
    annotation: ChoiceAnnotationSnapshot,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ChoiceAnnotationSnapshot {
    None,
    Candidate {
        lane: String,
        score: i32,
        #[serde(default)]
        scores: Vec<ScoreComponentSnapshot>,
        candidate: Value,
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
    use sts_simulator::eval::run_control::RunControlCommand;

    use super::super::owner_model::{OwnerChoice, OwnerChoiceExpansion};
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
}
