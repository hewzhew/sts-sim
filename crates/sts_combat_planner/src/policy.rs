use std::sync::Arc;
use std::time::Instant;

use sts_combat_strategy::{
    combat_plan_selection_member_timing_v1, combat_plan_state_guide_rank_v1,
    CombatPlanActionTimingV1,
};
use sts_core::sim::combat::CombatPosition;
use sts_core::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use sts_core::state::core::ClientInput;

use crate::types::TurnOptionAction;

/// One exact choice on a concrete simulator action surface.
///
/// Structured selections remain a family here: their (potentially enormous)
/// member language is scheduled lazily by the selection transaction.
#[derive(Clone, Copy)]
pub enum CombatPolicyChoice<'a> {
    Atomic(&'a ClientInput),
    StructuredSelection(&'a CombatSelectionActionFamilyV2),
}

/// Opaque, lexicographically ordered domain guidance for an exact combat
/// state. The planner does not assign meaning or units to the components; it
/// only uses the rank in an explicitly non-authoritative guide queue. Higher
/// components are preferred.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct CombatStateGuideRank(Arc<[i32]>);

impl CombatStateGuideRank {
    pub fn new(components: impl Into<Vec<i32>>) -> Self {
        Self(components.into().into())
    }

    /// Read-only diagnostic view of the policy-owned lexicographic rank.
    pub fn components(&self) -> &[i32] {
        self.0.as_ref()
    }
}

/// Opaque identity for one guide queue.
///
/// A policy may expose different guide sets at player-turn boundaries and
/// while constructing a turn.  Equal ids mean equal semantics, so a partial
/// expansion can safely publish its best retained promise back to the outer
/// search.  Different ids are never compared or joined merely because they
/// occupy the same vector position.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CombatGuideLaneId(u32);

impl CombatGuideLaneId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

/// One opaque rank in one explicitly identified guide queue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatStateGuide {
    pub lane: CombatGuideLaneId,
    pub rank: CombatStateGuideRank,
}

impl CombatStateGuide {
    pub fn new(lane: CombatGuideLaneId, components: impl Into<Vec<i32>>) -> Self {
        Self {
            lane,
            rank: CombatStateGuideRank::new(components),
        }
    }

    pub fn from_rank(lane: CombatGuideLaneId, rank: CombatStateGuideRank) -> Self {
        Self { lane, rank }
    }
}

/// A domain policy may cheaply propose a complete tactical suffix. The
/// planner never trusts the proposal as an outcome: every action and exact
/// successor hash is replayed from the original root before a witness exists.
#[derive(Clone, Debug)]
pub struct CombatPolicyWitnessProposal {
    pub actions: Vec<TurnOptionAction>,
    pub final_hp_hint: i32,
}

/// One untrusted complete winning suffix proposed relative to the exact state
/// passed to a lookahead evaluator.
///
/// The planner may join this suffix to its retained exact prefix, but it must
/// replay the complete line from the unchanged combat root before any witness
/// exists. A suffix never creates graph edges, prunes alternatives, or claims
/// terminal truth by itself.
#[derive(Clone, Debug)]
pub struct CombatLookaheadSuffixProposal {
    pub actions: Vec<ClientInput>,
    pub final_hp_hint: i32,
}

/// One non-authoritative, bounded lookahead observation for an exact combat
/// state. The planner may use its guide rank to order future exact work, but
/// the observation cannot create a successor or claim a terminal outcome.
#[derive(Clone, Debug)]
pub struct CombatLookaheadEvaluation {
    pub guide: CombatStateGuide,
    /// Optional replay-required evidence that the evaluated exact state has a
    /// complete winning continuation.
    pub winning_suffix: Option<CombatLookaheadSuffixProposal>,
    /// Deterministic evaluator work consumed by this observation. Implementors
    /// normally count simulated player inputs.
    pub work: usize,
}

/// Optional expensive state guidance, scheduled lazily by the planner.
///
/// This is deliberately separate from `CombatActionPolicy`: cheap static
/// ranks are available when a node is admitted, while lookahead is paid for
/// only after the exact node receives evaluator service.
pub trait CombatLookaheadEvaluator: Send + Sync {
    /// The guide rank used before this exact state has been evaluated. Returning
    /// `None` means that the evaluator does not apply to this state.
    fn pending_guide(&self, position: &CombatPosition) -> Option<CombatStateGuide>;

    /// Whether one mid-turn exact state should pay for lookahead when it is
    /// naturally selected for expansion. This admission hook prevents an
    /// expensive evaluator from being run eagerly for every generated state.
    fn admit_atomic_state(
        &self,
        position: &CombatPosition,
        atomic_expansions_before: usize,
    ) -> bool;

    /// Evaluate one exact state within the caller-owned work and time bounds.
    /// Returning `None` leaves the state pending so a later quantum may retry.
    fn evaluate(
        &self,
        position: &CombatPosition,
        max_work: usize,
        deadline: Option<Instant>,
    ) -> Option<CombatLookaheadEvaluation>;
}

/// Supplies search guidance only. Returning a small weight never changes
/// legality, and invalid weights are treated as neutral rather than trusted.
pub trait CombatActionPolicy: Send + Sync {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64>;

    /// Optional ordering inside a finite structured family after its exact
    /// concrete inputs are known. The default remains uniform and therefore
    /// preserves canonical cursor order. A generator may call this only when
    /// materializing the family cannot cause combinatorial expansion.
    fn structured_selection_member_weights(
        &self,
        _position: &CombatPosition,
        _family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        vec![1.0; members.len()]
    }

    /// Optional state guidance for choosing between already materialized exact
    /// turn-boundary states. It never changes legality, duplicate ownership,
    /// or terminal claims. Search retains a policy-only anchor queue even when
    /// this rank is available.
    fn state_guide_rank(&self, _position: &CombatPosition) -> Option<CombatStateGuideRank> {
        None
    }

    /// Independent guide queues over one shared exact-state graph. Keeping
    /// ranks separate avoids inventing a calibration between unlike domain
    /// heuristics (for example, progress and survival).
    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.state_guide_rank(position)
            .map(|rank| CombatStateGuide::from_rank(CombatGuideLaneId::new(0), rank))
            .into_iter()
            .collect()
    }

    /// Guidance for partial states while constructing one complete player
    /// turn.  It is deliberately separate from turn-boundary guidance: a
    /// learned boundary value may describe the current state well while
    /// actively discouraging every action needed to reach the next boundary.
    /// Existing policies retain their behavior unless they opt into the
    /// distinction.
    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.state_guides(position)
    }
}

/// Adds one plan-owned turn-boundary guide without changing action generation.
///
/// This is intentionally not a turn-generation guide: the plan may describe
/// a desirable cross-turn state without knowing which mid-turn investment
/// actions reach it. The wrapped action weights and partial-state guides remain
/// byte-for-byte authoritative inside a single exact step.
pub struct CombatPlanStateGuidePolicyV1 {
    base: SharedCombatActionPolicy,
}

/// Orders concrete members of a structured selection using encounter-owned
/// plan timing without adding any state-guide lane.
///
/// This is separate from [`CombatPlanStateGuidePolicyV1`] so callers can
/// distinguish cross-turn state guidance from within-choice action ordering.
pub struct CombatPlanSelectionTimingPolicyV1 {
    base: SharedCombatActionPolicy,
}

impl CombatPlanStateGuidePolicyV1 {
    pub fn new(base: SharedCombatActionPolicy) -> Self {
        Self { base }
    }
}

impl CombatPlanSelectionTimingPolicyV1 {
    pub fn new(base: SharedCombatActionPolicy) -> Self {
        Self { base }
    }
}

impl CombatActionPolicy for CombatPlanStateGuidePolicyV1 {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        self.base
            .structured_selection_member_weights(position, family, members)
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        self.base.state_guide_rank(position)
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        let mut guides = self.base.state_guides(position);
        if let Some(rank) = combat_plan_state_guide_rank_v1(position) {
            guides.push(CombatStateGuide::new(
                COMBAT_PLAN_STATE_GUIDE_LANE_V1,
                rank.components().to_vec(),
            ));
        }
        guides
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.turn_generation_guides(position)
    }
}

impl CombatActionPolicy for CombatPlanSelectionTimingPolicyV1 {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        self.base.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        let mut weights = self
            .base
            .structured_selection_member_weights(position, family, members);
        if weights.len() != members.len() || members.is_empty() {
            return weights;
        }
        let timings = members
            .iter()
            .map(|member| combat_plan_selection_member_timing_v1(position, family, member))
            .collect::<Vec<_>>();
        let has_compatible_member = timings
            .iter()
            .any(|timing| !matches!(timing, CombatPlanActionTimingV1::Defer(_)));
        if !has_compatible_member {
            return weights;
        }
        let compatible_floor = weights
            .iter()
            .zip(&timings)
            .filter_map(|(weight, timing)| {
                (!matches!(timing, CombatPlanActionTimingV1::Defer(_))
                    && weight.is_finite()
                    && *weight > 0.0)
                    .then_some(*weight)
            })
            .min_by(f64::total_cmp)
            .unwrap_or(1.0);
        for (weight, timing) in weights.iter_mut().zip(timings) {
            if matches!(timing, CombatPlanActionTimingV1::Defer(_)) {
                // Categorical ordering only: every compatible member remains
                // ahead of a member which destroys a plan-owned resource.
                // The deferred member keeps positive mass and is never
                // removed from exact search.
                *weight = compatible_floor * 0.5;
            }
        }
        weights
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        self.base.state_guide_rank(position)
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.state_guides(position)
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.turn_generation_guides(position)
    }
}

pub fn combat_plan_state_guide_policy_v1(
    base: SharedCombatActionPolicy,
) -> SharedCombatActionPolicy {
    Arc::new(CombatPlanStateGuidePolicyV1::new(base))
}

pub fn combat_plan_selection_timing_policy_v1(
    base: SharedCombatActionPolicy,
) -> SharedCombatActionPolicy {
    Arc::new(CombatPlanSelectionTimingPolicyV1::new(base))
}

pub const COMBAT_PLAN_STATE_GUIDE_LANE_V1: CombatGuideLaneId = CombatGuideLaneId::new(0x4350_0001);

#[derive(Clone, Copy, Debug, Default)]
pub struct UniformCombatActionPolicy;

impl CombatActionPolicy for UniformCombatActionPolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        vec![1.0; choices.len()]
    }
}

pub type SharedCombatActionPolicy = Arc<dyn CombatActionPolicy>;
pub type SharedCombatLookaheadEvaluator = Arc<dyn CombatLookaheadEvaluator>;

pub(crate) fn uniform_policy() -> SharedCombatActionPolicy {
    Arc::new(UniformCombatActionPolicy)
}

pub(crate) fn normalized_probabilities(
    weights: impl IntoIterator<Item = f64>,
    uniform_exploration_ppm: u32,
) -> Vec<f64> {
    let weights = weights
        .into_iter()
        .map(|weight| {
            if weight.is_finite() && weight > 0.0 {
                weight
            } else {
                1.0
            }
        })
        .collect::<Vec<_>>();
    if weights.is_empty() {
        return Vec::new();
    }
    let total = weights.iter().sum::<f64>();
    let uniform = 1.0 / weights.len() as f64;
    let epsilon = (uniform_exploration_ppm.min(1_000_000) as f64) / 1_000_000.0;
    weights
        .into_iter()
        .map(|weight| {
            ((1.0 - epsilon) * (weight / total) + epsilon * uniform).max(f64::MIN_POSITIVE)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_core::content::cards::CardId;
    use sts_core::content::monsters::EnemyId;
    use sts_core::runtime::combat::CombatCard;
    use sts_core::state::core::{EngineState, HandSelectReason, PendingChoice};
    use sts_core::state::selection::{SelectionResolution, SelectionScope};
    use sts_core::test_support::{blank_test_combat, test_monster};

    #[test]
    fn mixed_distribution_is_positive_and_normalized() {
        let distribution = normalized_probabilities([100.0, 1.0, 0.0, f64::NAN], 50_000);
        assert!(distribution.iter().all(|probability| *probability > 0.0));
        assert!((distribution.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert!(distribution[0] > distribution[1]);
    }

    #[test]
    fn full_uniform_mix_ignores_expert_weight() {
        let distribution = normalized_probabilities([100.0, 1.0], 1_000_000);
        assert_eq!(distribution, vec![0.5, 0.5]);
    }

    #[test]
    fn plan_state_guide_policy_does_not_reorder_mid_turn_actions() {
        let mut combat = blank_test_combat();
        let mut awakened = test_monster(EnemyId::AwakenedOne);
        awakened.slot = 2;
        combat.entities.monsters.push(awakened);
        combat.zones.hand = vec![
            CombatCard::new(CardId::Corruption, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let corruption = ClientInput::PlayCard {
            card_index: 0,
            target: None,
        };
        let defend = ClientInput::PlayCard {
            card_index: 1,
            target: None,
        };
        let policy = combat_plan_state_guide_policy_v1(Arc::new(UniformCombatActionPolicy));

        let weights = policy.weights(
            &position,
            &[
                CombatPolicyChoice::Atomic(&corruption),
                CombatPolicyChoice::Atomic(&defend),
            ],
        );

        assert_eq!(weights, vec![1.0, 1.0]);
        assert!(policy.turn_generation_guides(&position).is_empty());
        let guides = policy.state_guides(&position);
        assert_eq!(guides.len(), 1);
        assert_eq!(guides[0].lane, COMBAT_PLAN_STATE_GUIDE_LANE_V1);
    }

    #[test]
    fn plan_state_guide_policy_preserves_structured_selection_ordering() {
        let mut combat = blank_test_combat();
        let mut awakened = test_monster(EnemyId::AwakenedOne);
        awakened.id = 10;
        awakened.slot = 2;
        combat.entities.monsters.push(awakened);
        combat.zones.hand = vec![
            CombatCard::new(CardId::DemonForm, 31),
            CombatCard::new(CardId::Strike, 32),
        ];
        let position = CombatPosition::new(
            EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: vec![31, 32],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: HandSelectReason::Exhaust,
            }),
            combat,
        );
        let surface = sts_core::sim::combat_action_surface::combat_legal_action_surface_v2(
            &position.engine,
            &position.combat,
        );
        let family = surface
            .selection_families
            .first()
            .expect("forced exhaust family");
        let members = vec![
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                [31],
            )),
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                [32],
            )),
        ];
        let policy = combat_plan_state_guide_policy_v1(Arc::new(UniformCombatActionPolicy));

        let weights = policy.structured_selection_member_weights(&position, family, &members);

        assert_eq!(weights, vec![1.0, 1.0]);
    }

    #[test]
    fn plan_selection_timing_orders_compatible_exhaust_without_adding_guides() {
        let mut combat = blank_test_combat();
        let mut awakened = test_monster(EnemyId::AwakenedOne);
        awakened.id = 10;
        awakened.slot = 2;
        combat.entities.monsters.push(awakened);
        combat.zones.hand = vec![
            CombatCard::new(CardId::DemonForm, 31),
            CombatCard::new(CardId::Strike, 32),
        ];
        let position = CombatPosition::new(
            EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: vec![31, 32],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: HandSelectReason::Exhaust,
            }),
            combat,
        );
        let surface = sts_core::sim::combat_action_surface::combat_legal_action_surface_v2(
            &position.engine,
            &position.combat,
        );
        let family = surface
            .selection_families
            .first()
            .expect("forced exhaust family");
        let members = vec![
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                [31],
            )),
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                [32],
            )),
        ];
        let policy = combat_plan_selection_timing_policy_v1(Arc::new(UniformCombatActionPolicy));

        let weights = policy.structured_selection_member_weights(&position, family, &members);

        assert!(weights[1] > weights[0]);
        assert!(weights[0] > 0.0);
        assert!(policy.state_guides(&position).is_empty());
        assert!(policy.turn_generation_guides(&position).is_empty());
    }
}
