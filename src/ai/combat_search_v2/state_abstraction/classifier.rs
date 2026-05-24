use std::collections::BTreeMap;

use super::registry::{boundary_spec, registered_boundary_specs};
use super::types::*;

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
    let identity_audit = identity_audit_report(&cases);
    StateAbstractionGateReport {
        schema_name: "StateAbstractionGateReport",
        schema_version: 2,
        policy: "state abstractions are reported with explicit soundness and allowed consumers; report-only and estimate-only boundaries must not remove exact search branches",
        registered_boundaries: registered_boundary_specs(),
        case_count: cases.len(),
        divergence_histogram: histogram(cases.iter().map(|case| case.divergence_kind.label())),
        divergence_group_histogram: group_histogram(cases.iter().flat_map(|case| {
            case.turn_sequence_divergence_histogram
                .iter()
                .map(|entry| (entry.kind.label(), entry.groups))
        })),
        divergence_path_histogram: histogram(
            cases
                .iter()
                .map(|case| case.first_divergence_path.unwrap_or("none")),
        ),
        latent_debt_histogram: histogram(cases.iter().map(|case| case.latent_debt_kind.label())),
        latent_debt_group_histogram: group_histogram(cases.iter().flat_map(|case| {
            case.turn_sequence_divergence_histogram.iter().map(|entry| {
                let debt = latent_debt_kind(entry.kind);
                (debt.label(), entry.groups)
            })
        })),
        candidate_level_histogram: histogram(cases.iter().map(|case| case.candidate_level.label())),
        candidate_level_group_histogram: group_histogram(cases.iter().flat_map(|case| {
            case.turn_sequence_divergence_histogram.iter().map(|entry| {
                let debt = latent_debt_kind(entry.kind);
                let level = candidate_level(
                    entry.kind,
                    debt,
                    entry.first_divergence_path,
                    entry.guessed_reveal_gate,
                );
                (level.label(), entry.groups)
            })
        })),
        recommended_consumer_histogram: histogram(
            cases.iter().map(|case| case.recommended_consumer.label()),
        ),
        reveal_gate_histogram: histogram(cases.iter().map(|case| case.guessed_reveal_gate.label())),
        reveal_gate_group_histogram: group_histogram(cases.iter().flat_map(|case| {
            case.turn_sequence_divergence_histogram
                .iter()
                .map(|entry| (entry.guessed_reveal_gate.label(), entry.groups))
        })),
        identity_audit,
        cases,
        notes: vec![
            "exact simulator state remains authoritative",
            "turn_sequence_order_sensitive is report_only in v1",
            "candidate_level identifies future audit targets and does not enable pruning",
            "pending choice deduplication is local action-list equivalence, not global state equality",
        ],
    }
}

fn identity_audit_report(
    cases: &[StateAbstractionCaseReport],
) -> StateAbstractionIdentityAuditReport {
    let mut candidate_cases = BTreeMap::<&str, usize>::new();
    let mut candidate_groups = 0usize;
    let mut samples = Vec::new();

    for case in cases {
        for entry in &case.turn_sequence_divergence_histogram {
            let debt = latent_debt_kind(entry.kind);
            let level = candidate_level(
                entry.kind,
                debt,
                entry.first_divergence_path,
                entry.guessed_reveal_gate,
            );
            if level != StateAbstractionCandidateLevel::IdentityAuditCandidate {
                continue;
            }

            candidate_groups = candidate_groups.saturating_add(entry.groups);
            *candidate_cases.entry(&case.case_id).or_default() += entry.groups;
            if samples.len() < 8 {
                samples.push(StateAbstractionIdentityAuditSample {
                    case_id: case.case_id.clone(),
                    groups: entry.groups,
                    first_divergence_path: entry.first_divergence_path,
                    guessed_reveal_gate: entry.guessed_reveal_gate,
                    required_next_check: "prove_uuid_not_referenced_by_pending_queue_legal_actions_or_discard_selection_before_shuffle",
                });
            }
        }
    }

    StateAbstractionIdentityAuditReport {
        audit_policy: "card_uuid_identity_candidates_are_blocked_until_reference_audit",
        behavioral_effect: "report_only_no_prune_no_state_merge",
        status: if candidate_groups == 0 {
            "no_identity_audit_candidates_observed"
        } else {
            "blocked_until_card_identity_reference_audit"
        },
        candidate_cases: candidate_cases.len(),
        candidate_groups,
        proof_pruning_enabled: false,
        exact_branch_removal_allowed: false,
        blocked_reason: "card uuid differences can be referenced by pending queues, legal action descriptors, selection payloads, or future card identity lookups",
        required_checks: vec![
            "pending queue and queued card payloads do not reference the differing uuid",
            "current legal action descriptors and selectable payloads are unchanged by the uuid order",
            "no discard-pile card selection can read the differing uuid before the reveal gate",
            "shuffle or other reveal-gate handling must split back to exact representatives before using the abstraction",
        ],
        samples,
        notes: vec![
            "identity_audit_candidate is deliberately blocked from proof pruning in v1",
            "discard pile uuid-order differences may look similar to discard-order debt but are a separate identity-reference problem",
            "this report is an audit target list, not a search optimization",
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
    let latent_debt_kind = latent_debt_kind(divergence_kind);
    let candidate_level = candidate_level(
        divergence_kind,
        latent_debt_kind,
        first_divergence_path,
        guessed_reveal_gate,
    );
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
        latent_debt_kind,
        candidate_level,
        recommended_consumer,
        pruning_allowed: false,
        exact_branch_removal_allowed: false,
        same_effect_turn_sequence_groups: input.same_effect_turn_sequence_groups,
        order_sensitive_turn_sequence_groups: input.order_sensitive_turn_sequence_groups,
        turn_sequence_divergence_histogram,
        notes,
    }
}

fn latent_debt_kind(divergence_kind: StateDivergenceKind) -> StateAbstractionLatentDebtKind {
    match divergence_kind {
        StateDivergenceKind::DiscardOrderDelta => StateAbstractionLatentDebtKind::DiscardOrder,
        StateDivergenceKind::CardUuidDelta => StateAbstractionLatentDebtKind::CardIdentity,
        StateDivergenceKind::TurnPlayedCardHistoryDelta => {
            StateAbstractionLatentDebtKind::TurnPlayedCardHistory
        }
        StateDivergenceKind::ImmediatePublicDelta => {
            StateAbstractionLatentDebtKind::ImmediatePublicState
        }
        StateDivergenceKind::TerminalDelta => StateAbstractionLatentDebtKind::TerminalClass,
        StateDivergenceKind::LegalActionDelta => StateAbstractionLatentDebtKind::LegalActionSet,
        StateDivergenceKind::Unknown => StateAbstractionLatentDebtKind::Unknown,
        _ => StateAbstractionLatentDebtKind::OtherRuntime,
    }
}

fn candidate_level(
    divergence_kind: StateDivergenceKind,
    latent_debt_kind: StateAbstractionLatentDebtKind,
    first_divergence_path: Option<&'static str>,
    guessed_reveal_gate: StateAbstractionRevealGate,
) -> StateAbstractionCandidateLevel {
    match (
        divergence_kind,
        latent_debt_kind,
        first_divergence_path,
        guessed_reveal_gate,
    ) {
        (
            StateDivergenceKind::DiscardOrderDelta,
            StateAbstractionLatentDebtKind::DiscardOrder,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ) => StateAbstractionCandidateLevel::HorizonLimitedCandidate,
        (
            StateDivergenceKind::CardUuidDelta,
            StateAbstractionLatentDebtKind::CardIdentity,
            Some("combat.zones.discard_pile.uuid_order"),
            StateAbstractionRevealGate::NextShuffle,
        ) => StateAbstractionCandidateLevel::IdentityAuditCandidate,
        (_, StateAbstractionLatentDebtKind::Unknown, _, _) => {
            StateAbstractionCandidateLevel::ReportOnlyUnknown
        }
        _ => StateAbstractionCandidateLevel::ReportOnlyBlocked,
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
        StateDivergenceKind::TurnRuntimeDelta => 9,
        StateDivergenceKind::TurnDrawModifierDelta => 10,
        StateDivergenceKind::TurnActionCounterDelta => 11,
        StateDivergenceKind::TurnPlayedCardHistoryDelta => 12,
        StateDivergenceKind::TurnDiscardCounterDelta => 13,
        StateDivergenceKind::TurnOrbHistoryDelta => 14,
        StateDivergenceKind::TurnCombatFlagDelta => 15,
        StateDivergenceKind::MonsterRuntimeDelta => 16,
        StateDivergenceKind::CombatRuntimeHintDelta => 17,
        StateDivergenceKind::PotionStateDelta => 18,
        StateDivergenceKind::RelicCounterDelta => 19,
        StateDivergenceKind::PlayerFutureDelta => 20,
        StateDivergenceKind::ZoneRuntimeDelta => 21,
        StateDivergenceKind::EngineRuntimeDelta => 22,
        StateDivergenceKind::CombatMetaDelta => 23,
        StateDivergenceKind::PendingQueueDelta => 24,
        StateDivergenceKind::IdentityOnlyCandidate => 25,
        StateDivergenceKind::Unknown => 26,
    }
}

fn histogram(keys: impl Iterator<Item = &'static str>) -> Vec<StateAbstractionHistogramEntry> {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for key in keys {
        *counts.entry(key).or_default() += 1;
    }
    histogram_entries(counts)
}

fn group_histogram(
    entries: impl Iterator<Item = (&'static str, usize)>,
) -> Vec<StateAbstractionHistogramEntry> {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for (key, groups) in entries {
        *counts.entry(key).or_default() += groups;
    }
    histogram_entries(counts)
}

fn histogram_entries(counts: BTreeMap<&'static str, usize>) -> Vec<StateAbstractionHistogramEntry> {
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
