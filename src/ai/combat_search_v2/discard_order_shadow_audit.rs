use super::state_abstraction::{StateAbstractionRevealGate, StateDivergenceKind};
use super::types::{
    CombatSearchV2DiagnosticsDiscardOrderShadowAudit,
    CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample,
};

const DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT: usize = 8;

#[derive(Clone, Debug)]
pub(super) struct DiscardOrderShadowAuditObservation {
    pub origin_key: String,
    pub unordered_key_preview: String,
    pub states: u64,
    pub max_prefix_length: usize,
    pub ordered_variants: usize,
    pub effect_variants: usize,
    pub max_legal_actions: usize,
    pub first_divergence_path: Option<&'static str>,
    pub reveal_gate: StateAbstractionRevealGate,
}

pub(super) fn is_static_discard_order_candidate(
    kind: StateDivergenceKind,
    first_divergence_path: Option<&'static str>,
    reveal_gate: StateAbstractionRevealGate,
) -> bool {
    matches!(kind, StateDivergenceKind::DiscardOrderDelta)
        && first_divergence_path == Some("combat.zones.discard_pile")
        && matches!(reveal_gate, StateAbstractionRevealGate::NextShuffle)
}

pub(super) fn summarize_discard_order_shadow_audit(
    mut observations: Vec<DiscardOrderShadowAuditObservation>,
) -> CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
    observations.sort_by(|left, right| {
        right
            .states
            .cmp(&left.states)
            .then_with(|| right.ordered_variants.cmp(&left.ordered_variants))
            .then_with(|| right.effect_variants.cmp(&left.effect_variants))
            .then_with(|| left.origin_key.cmp(&right.origin_key))
            .then_with(|| left.unordered_key_preview.cmp(&right.unordered_key_preview))
    });

    let candidate_groups = observations.len();
    let candidate_states = observations.iter().map(|item| item.states).sum();
    let samples = observations
        .into_iter()
        .take(DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT)
        .map(
            |item| CombatSearchV2DiagnosticsDiscardOrderShadowAuditSample {
                origin_key: item.origin_key,
                unordered_key_preview: item.unordered_key_preview,
                states: item.states,
                max_prefix_length: item.max_prefix_length,
                ordered_variants: item.ordered_variants,
                effect_variants: item.effect_variants,
                max_legal_actions: item.max_legal_actions,
                first_divergence_path: item.first_divergence_path,
                reveal_gate: item.reveal_gate,
            },
        )
        .collect();

    CombatSearchV2DiagnosticsDiscardOrderShadowAudit {
        audit_policy: "static_discard_order_candidate_until_next_shuffle",
        behavioral_effect: "diagnostic_only_no_rollout_no_prune_no_state_merge",
        candidate_groups,
        candidate_states,
        static_immediate_safe_groups: candidate_groups,
        static_immediate_safe_states: candidate_states,
        exact_rollout_verified_groups: 0,
        proof_pruning_enabled: false,
        reveal_gate: StateAbstractionRevealGate::NextShuffle,
        sample_limit: DISCARD_ORDER_SHADOW_AUDIT_SAMPLE_LIMIT,
        samples,
        notes: vec![
            "static audit only; no child branch was removed",
            "groups are candidates for future exact shadow rollout until next shuffle",
            "the classifier has already excluded immediate public, legal-action, and terminal deltas for these groups",
            "exact_rollout_verified_groups stays zero until a simulator-backed shadow audit is implemented",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_candidate_requires_discard_order_until_next_shuffle() {
        assert!(is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.draw_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::DiscardOrderDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextDraw,
        ));
        assert!(!is_static_discard_order_candidate(
            StateDivergenceKind::ImmediatePublicDelta,
            Some("combat.zones.discard_pile"),
            StateAbstractionRevealGate::NextShuffle,
        ));
    }

    #[test]
    fn summary_reports_static_candidates_without_pruning() {
        let report = summarize_discard_order_shadow_audit(vec![
            DiscardOrderShadowAuditObservation {
                origin_key: "origin_b".to_string(),
                unordered_key_preview: "B>A".to_string(),
                states: 2,
                max_prefix_length: 2,
                ordered_variants: 2,
                effect_variants: 2,
                max_legal_actions: 4,
                first_divergence_path: Some("combat.zones.discard_pile"),
                reveal_gate: StateAbstractionRevealGate::NextShuffle,
            },
            DiscardOrderShadowAuditObservation {
                origin_key: "origin_a".to_string(),
                unordered_key_preview: "A>B".to_string(),
                states: 3,
                max_prefix_length: 3,
                ordered_variants: 2,
                effect_variants: 2,
                max_legal_actions: 5,
                first_divergence_path: Some("combat.zones.discard_pile"),
                reveal_gate: StateAbstractionRevealGate::NextShuffle,
            },
        ]);

        assert_eq!(report.candidate_groups, 2);
        assert_eq!(report.candidate_states, 5);
        assert_eq!(report.static_immediate_safe_groups, 2);
        assert_eq!(report.static_immediate_safe_states, 5);
        assert_eq!(report.exact_rollout_verified_groups, 0);
        assert!(!report.proof_pruning_enabled);
        assert_eq!(report.samples[0].origin_key, "origin_a");
    }
}
