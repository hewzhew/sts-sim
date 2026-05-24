use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionBoundaryId {
    StarterBasicDuplicatePlayCardByTarget,
    PendingChoiceIdenticalRuntimeCard,
    TurnSequenceOrderSensitive,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionBoundaryScope {
    LocalActionList,
    CombatSearchAnalysis,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionSoundnessLevel {
    ExactStructural,
    LocalActionEquivalent,
    HorizonExact,
    PublicObservationEquivalent,
    EstimateOnly,
    CandidateOnly,
    ReportOnly,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionConsumer {
    ProofPrune,
    LocalActionDedup,
    EstimateShare,
    CandidateOrdering,
    ReportOnly,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionRevealGate {
    NextDraw,
    NextShuffle,
    NextRandomCall,
    NextCardSelection,
    NextRelicCounterRead,
    NextLegalActionGeneration,
    CombatEnd,
    CurrentActionResolution,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateDivergenceKind {
    ImmediatePublicDelta,
    LegalActionDelta,
    TerminalDelta,
    DrawPileOrderDelta,
    DiscardOrderDelta,
    HandOrderDelta,
    ExhaustOrderDelta,
    RngStateDelta,
    RelicCounterDelta,
    CardUuidDelta,
    PendingQueueDelta,
    IdentityOnlyCandidate,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionBoundarySpec {
    pub id: StateAbstractionBoundaryId,
    pub name: &'static str,
    pub scope: StateAbstractionBoundaryScope,
    pub soundness: StateAbstractionSoundnessLevel,
    pub allowed_consumers: Vec<StateAbstractionConsumer>,
    pub ignored_fields: Vec<&'static str>,
    pub reveal_gates: Vec<StateAbstractionRevealGate>,
    pub audit_required: bool,
    pub notes: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionGateReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub policy: &'static str,
    pub registered_boundaries: Vec<StateAbstractionBoundarySpec>,
    pub case_count: usize,
    pub divergence_histogram: Vec<StateAbstractionHistogramEntry>,
    pub recommended_consumer_histogram: Vec<StateAbstractionHistogramEntry>,
    pub reveal_gate_histogram: Vec<StateAbstractionHistogramEntry>,
    pub cases: Vec<StateAbstractionCaseReport>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionHistogramEntry {
    pub key: &'static str,
    pub cases: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionCaseReport {
    pub case_id: String,
    pub boundary_id: StateAbstractionBoundaryId,
    pub soundness: StateAbstractionSoundnessLevel,
    pub allowed_consumers: Vec<StateAbstractionConsumer>,
    pub divergence_kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub public_observation_changed: Option<bool>,
    pub legal_actions_changed: Option<bool>,
    pub terminal_class_changed: Option<bool>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub recommended_consumer: StateAbstractionConsumer,
    pub pruning_allowed: bool,
    pub exact_branch_removal_allowed: bool,
    pub same_effect_turn_sequence_groups: usize,
    pub order_sensitive_turn_sequence_groups: usize,
    pub turn_sequence_divergence_histogram: Vec<StateAbstractionCaseDivergenceCount>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionCaseDivergenceCount {
    pub kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub groups: usize,
}

#[derive(Clone, Debug)]
pub struct StateAbstractionCaseInput<'a> {
    pub case_id: &'a str,
    pub same_effect_turn_sequence_groups: usize,
    pub order_sensitive_turn_sequence_groups: usize,
    pub turn_sequence_divergence_histogram: Vec<StateAbstractionDivergenceInput>,
}

#[derive(Clone, Debug)]
pub struct StateAbstractionDivergenceInput {
    pub kind: StateDivergenceKind,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub groups: usize,
}

pub fn boundary_spec(id: StateAbstractionBoundaryId) -> StateAbstractionBoundarySpec {
    match id {
        StateAbstractionBoundaryId::StarterBasicDuplicatePlayCardByTarget => {
            StateAbstractionBoundarySpec {
                id,
                name: "starter_basic_duplicate_play_card_by_target",
                scope: StateAbstractionBoundaryScope::LocalActionList,
                soundness: StateAbstractionSoundnessLevel::LocalActionEquivalent,
                allowed_consumers: vec![StateAbstractionConsumer::LocalActionDedup],
                ignored_fields: vec!["combat.card.uuid"],
                reveal_gates: vec![StateAbstractionRevealGate::CurrentActionResolution],
                audit_required: true,
                notes: "Deduplicates runtime-identical starter basic card plays to the same target inside one legal action list; it is not a global state merge.",
            }
        }
        StateAbstractionBoundaryId::PendingChoiceIdenticalRuntimeCard => {
            StateAbstractionBoundarySpec {
                id,
                name: "pending_choice_identical_runtime_card",
                scope: StateAbstractionBoundaryScope::LocalActionList,
                soundness: StateAbstractionSoundnessLevel::LocalActionEquivalent,
                allowed_consumers: vec![StateAbstractionConsumer::LocalActionDedup],
                ignored_fields: vec!["combat.card.uuid"],
                reveal_gates: vec![StateAbstractionRevealGate::CurrentActionResolution],
                audit_required: true,
                notes: "Deduplicates single-card pending grid/hand choices only when source scope and runtime card fields match; it is not a global state abstraction.",
            }
        }
        StateAbstractionBoundaryId::TurnSequenceOrderSensitive => {
            StateAbstractionBoundarySpec {
                id,
                name: "turn_sequence_order_sensitive",
                scope: StateAbstractionBoundaryScope::CombatSearchAnalysis,
                soundness: StateAbstractionSoundnessLevel::ReportOnly,
                allowed_consumers: vec![StateAbstractionConsumer::ReportOnly],
                ignored_fields: Vec::new(),
                reveal_gates: vec![StateAbstractionRevealGate::Unknown],
                audit_required: true,
                notes: "Observed turn-sequence variants are order-sensitive under the current exact/dominance key and must not prune exact branches.",
            }
        }
    }
}

pub fn registered_boundary_specs() -> Vec<StateAbstractionBoundarySpec> {
    vec![
        boundary_spec(StateAbstractionBoundaryId::StarterBasicDuplicatePlayCardByTarget),
        boundary_spec(StateAbstractionBoundaryId::PendingChoiceIdenticalRuntimeCard),
        boundary_spec(StateAbstractionBoundaryId::TurnSequenceOrderSensitive),
    ]
}

pub fn classify_state_abstraction_case(
    input: StateAbstractionCaseInput<'_>,
) -> Option<StateAbstractionCaseReport> {
    if input.same_effect_turn_sequence_groups > 0 {
        return Some(turn_sequence_case_report(
            input,
            StateDivergenceKind::IdentityOnlyCandidate,
            None,
            StateAbstractionRevealGate::Unknown,
            StateAbstractionConsumer::ReportOnly,
            vec![
                "same-effect ordered variants are candidates for later simulator-backed commutation probes",
                "v1 keeps this report-only because current diagnostics do not prove reveal gates",
            ],
        ));
    }
    if input.order_sensitive_turn_sequence_groups > 0 {
        let primary = primary_divergence(&input.turn_sequence_divergence_histogram);
        return Some(turn_sequence_case_report(
            input,
            primary.kind,
            primary.first_divergence_path,
            primary.guessed_reveal_gate,
            StateAbstractionConsumer::ReportOnly,
            vec![
                "current exact/dominance keys observe different future-relevant state for reordered prefixes",
                "turn-sequence divergence classification is diagnostic guidance, not a pruning proof",
            ],
        ));
    }
    None
}

pub fn build_state_abstraction_gate_report(
    cases: Vec<StateAbstractionCaseReport>,
) -> StateAbstractionGateReport {
    StateAbstractionGateReport {
        schema_name: "StateAbstractionGateReport",
        schema_version: 1,
        policy: "state abstractions are reported with explicit soundness and allowed consumers; report-only and estimate-only boundaries must not remove exact search branches",
        registered_boundaries: registered_boundary_specs(),
        case_count: cases.len(),
        divergence_histogram: histogram(cases.iter().map(|case| case.divergence_kind.label())),
        recommended_consumer_histogram: histogram(
            cases.iter().map(|case| case.recommended_consumer.label()),
        ),
        reveal_gate_histogram: histogram(cases.iter().map(|case| case.guessed_reveal_gate.label())),
        cases,
        notes: vec![
            "exact simulator state remains authoritative",
            "turn_sequence_order_sensitive is report_only in v1",
            "pending choice deduplication is local action-list equivalence, not global state equality",
        ],
    }
}

fn turn_sequence_case_report(
    input: StateAbstractionCaseInput<'_>,
    divergence_kind: StateDivergenceKind,
    first_divergence_path: Option<&'static str>,
    guessed_reveal_gate: StateAbstractionRevealGate,
    recommended_consumer: StateAbstractionConsumer,
    notes: Vec<&'static str>,
) -> StateAbstractionCaseReport {
    let spec = boundary_spec(StateAbstractionBoundaryId::TurnSequenceOrderSensitive);
    let public_observation_changed =
        matches!(divergence_kind, StateDivergenceKind::ImmediatePublicDelta);
    let legal_actions_changed = matches!(divergence_kind, StateDivergenceKind::LegalActionDelta);
    let terminal_class_changed = matches!(divergence_kind, StateDivergenceKind::TerminalDelta);
    let turn_sequence_divergence_histogram = input
        .turn_sequence_divergence_histogram
        .into_iter()
        .map(|entry| StateAbstractionCaseDivergenceCount {
            kind: entry.kind,
            first_divergence_path: entry.first_divergence_path,
            guessed_reveal_gate: entry.guessed_reveal_gate,
            groups: entry.groups,
        })
        .collect();
    StateAbstractionCaseReport {
        case_id: input.case_id.to_string(),
        boundary_id: spec.id,
        soundness: spec.soundness,
        allowed_consumers: spec.allowed_consumers,
        divergence_kind,
        first_divergence_path,
        public_observation_changed: Some(public_observation_changed),
        legal_actions_changed: Some(legal_actions_changed),
        terminal_class_changed: Some(terminal_class_changed),
        guessed_reveal_gate,
        recommended_consumer,
        pruning_allowed: false,
        exact_branch_removal_allowed: false,
        same_effect_turn_sequence_groups: input.same_effect_turn_sequence_groups,
        order_sensitive_turn_sequence_groups: input.order_sensitive_turn_sequence_groups,
        turn_sequence_divergence_histogram,
        notes,
    }
}

fn primary_divergence(
    histogram: &[StateAbstractionDivergenceInput],
) -> StateAbstractionDivergenceInput {
    histogram
        .iter()
        .max_by(|left, right| {
            left.groups
                .cmp(&right.groups)
                .then_with(|| divergence_rank(right.kind).cmp(&divergence_rank(left.kind)))
                .then_with(|| right.kind.cmp(&left.kind))
        })
        .cloned()
        .unwrap_or(StateAbstractionDivergenceInput {
            kind: StateDivergenceKind::Unknown,
            first_divergence_path: None,
            guessed_reveal_gate: StateAbstractionRevealGate::Unknown,
            groups: 0,
        })
}

fn divergence_rank(kind: StateDivergenceKind) -> u8 {
    match kind {
        StateDivergenceKind::TerminalDelta => 0,
        StateDivergenceKind::LegalActionDelta => 1,
        StateDivergenceKind::ImmediatePublicDelta => 2,
        StateDivergenceKind::HandOrderDelta => 3,
        StateDivergenceKind::DrawPileOrderDelta => 4,
        StateDivergenceKind::DiscardOrderDelta => 5,
        StateDivergenceKind::ExhaustOrderDelta => 6,
        StateDivergenceKind::RngStateDelta => 7,
        StateDivergenceKind::CardUuidDelta => 8,
        StateDivergenceKind::RelicCounterDelta => 9,
        StateDivergenceKind::PendingQueueDelta => 10,
        StateDivergenceKind::IdentityOnlyCandidate => 11,
        StateDivergenceKind::Unknown => 12,
    }
}

fn histogram(keys: impl Iterator<Item = &'static str>) -> Vec<StateAbstractionHistogramEntry> {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for key in keys {
        *counts.entry(key).or_default() += 1;
    }
    let mut entries = counts
        .into_iter()
        .map(|(key, cases)| StateAbstractionHistogramEntry { key, cases })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .cases
            .cmp(&left.cases)
            .then_with(|| left.key.cmp(right.key))
    });
    entries
}

impl StateAbstractionBoundaryId {
    pub fn label(self) -> &'static str {
        match self {
            StateAbstractionBoundaryId::StarterBasicDuplicatePlayCardByTarget => {
                "starter_basic_duplicate_play_card_by_target"
            }
            StateAbstractionBoundaryId::PendingChoiceIdenticalRuntimeCard => {
                "pending_choice_identical_runtime_card"
            }
            StateAbstractionBoundaryId::TurnSequenceOrderSensitive => {
                "turn_sequence_order_sensitive"
            }
        }
    }
}

impl StateAbstractionConsumer {
    pub fn label(self) -> &'static str {
        match self {
            StateAbstractionConsumer::ProofPrune => "proof_prune",
            StateAbstractionConsumer::LocalActionDedup => "local_action_dedup",
            StateAbstractionConsumer::EstimateShare => "estimate_share",
            StateAbstractionConsumer::CandidateOrdering => "candidate_ordering",
            StateAbstractionConsumer::ReportOnly => "report_only",
        }
    }
}

impl StateAbstractionRevealGate {
    pub fn label(self) -> &'static str {
        match self {
            StateAbstractionRevealGate::NextDraw => "next_draw",
            StateAbstractionRevealGate::NextShuffle => "next_shuffle",
            StateAbstractionRevealGate::NextRandomCall => "next_random_call",
            StateAbstractionRevealGate::NextCardSelection => "next_card_selection",
            StateAbstractionRevealGate::NextRelicCounterRead => "next_relic_counter_read",
            StateAbstractionRevealGate::NextLegalActionGeneration => "next_legal_action_generation",
            StateAbstractionRevealGate::CombatEnd => "combat_end",
            StateAbstractionRevealGate::CurrentActionResolution => "current_action_resolution",
            StateAbstractionRevealGate::Unknown => "unknown",
        }
    }
}

impl StateDivergenceKind {
    pub fn label(self) -> &'static str {
        match self {
            StateDivergenceKind::ImmediatePublicDelta => "immediate_public_delta",
            StateDivergenceKind::LegalActionDelta => "legal_action_delta",
            StateDivergenceKind::TerminalDelta => "terminal_delta",
            StateDivergenceKind::DrawPileOrderDelta => "draw_pile_order_delta",
            StateDivergenceKind::DiscardOrderDelta => "discard_order_delta",
            StateDivergenceKind::HandOrderDelta => "hand_order_delta",
            StateDivergenceKind::ExhaustOrderDelta => "exhaust_order_delta",
            StateDivergenceKind::RngStateDelta => "rng_state_delta",
            StateDivergenceKind::RelicCounterDelta => "relic_counter_delta",
            StateDivergenceKind::CardUuidDelta => "card_uuid_delta",
            StateDivergenceKind::PendingQueueDelta => "pending_queue_delta",
            StateDivergenceKind::IdentityOnlyCandidate => "identity_only_candidate",
            StateDivergenceKind::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_sequence_order_sensitive_boundary_is_report_only() {
        let spec = boundary_spec(StateAbstractionBoundaryId::TurnSequenceOrderSensitive);

        assert_eq!(spec.soundness, StateAbstractionSoundnessLevel::ReportOnly);
        assert_eq!(
            spec.allowed_consumers,
            vec![StateAbstractionConsumer::ReportOnly]
        );
    }

    #[test]
    fn pending_choice_boundary_is_local_action_equivalent_only() {
        let spec = boundary_spec(StateAbstractionBoundaryId::PendingChoiceIdenticalRuntimeCard);

        assert_eq!(
            spec.soundness,
            StateAbstractionSoundnessLevel::LocalActionEquivalent
        );
        assert_eq!(
            spec.allowed_consumers,
            vec![StateAbstractionConsumer::LocalActionDedup]
        );
    }

    #[test]
    fn classifier_marks_order_sensitive_turn_sequence_as_report_only() {
        let report = classify_state_abstraction_case(StateAbstractionCaseInput {
            case_id: "case",
            same_effect_turn_sequence_groups: 0,
            order_sensitive_turn_sequence_groups: 3,
            turn_sequence_divergence_histogram: Vec::new(),
        })
        .expect("order-sensitive sequence should classify");

        assert_eq!(
            report.boundary_id,
            StateAbstractionBoundaryId::TurnSequenceOrderSensitive
        );
        assert_eq!(report.divergence_kind, StateDivergenceKind::Unknown);
        assert_eq!(
            report.recommended_consumer,
            StateAbstractionConsumer::ReportOnly
        );
        assert!(!report.exact_branch_removal_allowed);
    }

    #[test]
    fn classifier_uses_turn_sequence_divergence_histogram() {
        let report = classify_state_abstraction_case(StateAbstractionCaseInput {
            case_id: "case",
            same_effect_turn_sequence_groups: 0,
            order_sensitive_turn_sequence_groups: 3,
            turn_sequence_divergence_histogram: vec![StateAbstractionDivergenceInput {
                kind: StateDivergenceKind::DrawPileOrderDelta,
                first_divergence_path: Some("combat.zones.draw_pile"),
                guessed_reveal_gate: StateAbstractionRevealGate::NextDraw,
                groups: 2,
            }],
        })
        .expect("order-sensitive sequence should classify");

        assert_eq!(
            report.divergence_kind,
            StateDivergenceKind::DrawPileOrderDelta
        );
        assert_eq!(report.first_divergence_path, Some("combat.zones.draw_pile"));
        assert_eq!(
            report.guessed_reveal_gate,
            StateAbstractionRevealGate::NextDraw
        );
        assert_eq!(report.turn_sequence_divergence_histogram.len(), 1);
        assert_eq!(report.public_observation_changed, Some(false));
    }
}
