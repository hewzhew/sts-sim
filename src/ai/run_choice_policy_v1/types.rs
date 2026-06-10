use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::content::cards::CardId;
use crate::state::core::RunPendingChoiceReason;

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceDecisionContextV1 {
    pub reason: RunPendingChoiceReason,
    pub min_choices: usize,
    pub max_choices: usize,
    pub candidates: Vec<RunChoiceCandidateEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceCandidateEvidenceV1 {
    pub candidate_id: String,
    pub label: String,
    pub deck_index: usize,
    pub card: CardId,
    pub class: RunChoicePolicyClassV1,
    pub selectable: bool,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunChoicePolicyClassV1 {
    CursePurge,
    StarterStrikeMutation,
    StarterDefendMutation,
    BasicCardMutation,
    OtherDeckMutation,
    UnsupportedChoice,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoicePolicyConfigV1 {
    pub allow_curse_purge: bool,
    pub allow_low_value_purge: bool,
    pub allow_low_value_transform: bool,
}

impl Default for RunChoicePolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_low_value_purge: true,
            allow_low_value_transform: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceDecisionV1 {
    pub action: RunChoicePolicyActionV1,
    pub label_role: &'static str,
    pub context: RunChoiceDecisionContextV1,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunChoicePolicyActionV1 {
    SelectDeckIndices {
        indices: Vec<usize>,
        labels: Vec<String>,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

impl RunChoiceDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let selected_candidate_id = match &self.action {
            RunChoicePolicyActionV1::SelectDeckIndices { indices, .. } => {
                indices.first().copied().map(candidate_id)
            }
            RunChoicePolicyActionV1::Stop { .. } => None,
        };

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::RunChoice,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
                InformationClassV1::Belief,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "run_choice_policy_v1".to_string(),
                source_schema_name: "RunChoicePolicyConfigV1".to_string(),
                source_schema_version: 1,
            },
            candidates: self
                .context
                .candidates
                .iter()
                .map(candidate_descriptor)
                .collect(),
            evidence: EvidenceBundleV1 {
                items: evidence_items(&self.context),
                assumptions: vec![
                    "run choice automation only handles explicit deck mutation targets with visible low-value cards"
                        .to_string(),
                    "run choice automation is a behavior policy, not a teacher label".to_string(),
                ],
                warnings: Vec::new(),
            },
            values: Vec::new(),
            selection: match &self.action {
                RunChoicePolicyActionV1::SelectDeckIndices {
                    confidence, reason, ..
                } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_run_choice_certificate".to_string(),
                },
                RunChoicePolicyActionV1::Stop { reason } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Stopped,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: 0.0,
                    selection_mode: "human_required".to_string(),
                },
            },
        }
    }
}

fn candidate_descriptor(candidate: &RunChoiceCandidateEvidenceV1) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: candidate.candidate_id.clone(),
        site: DecisionSiteKindV1::RunChoice,
        label: candidate.label.clone(),
        action_plan: PublicActionPlanV1 {
            summary: format!("select deck index {}", candidate.deck_index),
            command: Some(format!("select {}", candidate.deck_index)),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: candidate.risks.clone(),
    }
}

fn evidence_items(context: &RunChoiceDecisionContextV1) -> Vec<EvidenceItemV1> {
    context
        .candidates
        .iter()
        .map(|candidate| EvidenceItemV1 {
            kind: EvidenceKindV1::CandidateFacts,
            candidate_id: Some(candidate.candidate_id.clone()),
            label: format!(
                "{}: {:?} selectable={}",
                candidate.label, candidate.class, candidate.selectable
            ),
            information_class: InformationClassV1::PublicObservation,
            components: Vec::new(),
        })
        .collect()
}

pub(crate) fn candidate_id(deck_index: usize) -> String {
    format!("run_choice:deck:{deck_index}")
}
