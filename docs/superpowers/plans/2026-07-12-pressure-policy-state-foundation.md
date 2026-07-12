# Pressure And Challenger Policy-State Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build serializable pressure-contract, candidate-response, challenger-memory, and semantic-signature primitives, then expose evidence-only pressure responses in branch path artifacts without changing production decisions.

**Architecture:** New focused strategy modules own pressure reasoning and challenger state. Existing reward admissions provide factual candidate semantics; `branch_path` serializes the derived response for inspection, while owner selection, lane ordering, `heavy-burden`, and Probe behavior remain unchanged. The next implementation plan will persist these primitives inside continuously executing challenger lanes.

**Tech Stack:** Rust, serde/serde_json, Cargo unit tests, existing strategy and owner-audit artifact layers.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree.
- Never run `cargo clean`.
- Follow red-green TDD with focused tests and frequent local commits.
- Do not infer a damage or defense shortage directly from HP loss.
- Do not change production candidate scores, lane caps, owner ordering, or auto-expansion.
- Do not add card-ID-specific strategy rules; card IDs may appear in tests as semantic examples.
- Keep run-control free of strategy rules.
- Keep all new policy state serializable for later capsule persistence.
- Run the full library and `architecture_runtime_boundaries` suites only at the completion checkpoint.

---

### Task 1: Pressure Contract And Outcome Evidence

**Files:**
- Create: `src/ai/strategy/pressure_assessment.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Produces: `PressureAxis`, `PressureCoverage`, `EvidenceConfidence`, `SearchCoverage`, `PressureHypothesis`, `SurvivalPressureContract`, `OutcomePressureEvidence`, `PressureAssessment`, `assess_survival_pressure`, and `outcome_pressure_evidence`.
- Consumers: Tasks 2-4 use `PressureAxis`, `PressureHypothesis`, and coverage enums.
- Boundary: outcome evidence records unresolved pressure but never chooses a causal axis.

- [ ] **Step 1: Write the failing pressure tests in the new module**

Create `src/ai/strategy/pressure_assessment.rs` with imports and tests first:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hp_loss_opens_unresolved_pressure_without_assigning_an_axis() {
        let evidence = outcome_pressure_evidence(OutcomePressureEvidence {
            hp_loss: 12,
            died: false,
            search_coverage: SearchCoverage::Complete,
        })
        .expect("positive hp loss should be recorded");

        assert!(evidence.unresolved);
        assert_eq!(evidence.attributed_axis, None);
    }

    #[test]
    fn fast_resolution_covers_a_short_horizon_without_delay() {
        let assessment = assess_survival_pressure(SurvivalPressureContract {
            threat_turns: Some(3),
            resolution_turns: Some(2),
            finite_delay_turns: 0,
            repeatable_delay: false,
            deployability: PressureCoverage::Covered,
        });

        assert_eq!(assessment.overall, PressureCoverage::Covered);
        assert_eq!(assessment.effective_horizon_turns, Some(3));
    }

    #[test]
    fn finite_delay_cannot_cover_resolution_beyond_the_extended_horizon() {
        let assessment = assess_survival_pressure(SurvivalPressureContract {
            threat_turns: Some(2),
            resolution_turns: Some(5),
            finite_delay_turns: 2,
            repeatable_delay: false,
            deployability: PressureCoverage::Covered,
        });

        assert_eq!(assessment.overall, PressureCoverage::Open);
        assert_eq!(assessment.effective_horizon_turns, Some(4));
    }

    #[test]
    fn unknown_deployability_prevents_full_coverage() {
        let assessment = assess_survival_pressure(SurvivalPressureContract {
            threat_turns: Some(3),
            resolution_turns: Some(2),
            finite_delay_turns: 0,
            repeatable_delay: false,
            deployability: PressureCoverage::Unknown,
        });

        assert_eq!(assessment.overall, PressureCoverage::PartiallyCovered);
    }

    #[test]
    fn missing_horizon_or_resolution_stays_unknown() {
        let assessment = assess_survival_pressure(SurvivalPressureContract {
            threat_turns: None,
            resolution_turns: Some(2),
            finite_delay_turns: 1,
            repeatable_delay: false,
            deployability: PressureCoverage::Covered,
        });

        assert_eq!(assessment.overall, PressureCoverage::Unknown);
    }
}
```

- [ ] **Step 2: Register the empty module and verify the red state**

Add to `src/ai/strategy/mod.rs`:

```rust
pub mod pressure_assessment;
```

Run: `cargo test --lib hp_loss_opens_unresolved_pressure_without_assigning_an_axis`

Expected: compilation fails because the pressure types and functions do not exist.

- [ ] **Step 3: Implement the pressure vocabulary and assessment**

Insert above the tests in `pressure_assessment.rs`:

```rust
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureAxis {
    ResolutionTempo,
    DelayCapacity,
    MultiTargetControl,
    GrowthHorizon,
    Deployability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureCoverage {
    Open,
    PartiallyCovered,
    Covered,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceConfidence {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCoverage {
    Complete,
    Limited,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PressureEvidenceSource {
    EncounterThreat,
    DeckCapability,
    ObservedOutcome,
    SearchCoverage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PressureEvidence {
    pub source: PressureEvidenceSource,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PressureHypothesis {
    pub axis: PressureAxis,
    pub coverage: PressureCoverage,
    pub confidence: EvidenceConfidence,
    pub supporting_evidence: Vec<PressureEvidence>,
    pub contradicting_evidence: Vec<PressureEvidence>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SurvivalPressureContract {
    pub threat_turns: Option<u8>,
    pub resolution_turns: Option<u8>,
    pub finite_delay_turns: u8,
    pub repeatable_delay: bool,
    pub deployability: PressureCoverage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutcomePressureEvidence {
    pub hp_loss: u16,
    pub died: bool,
    pub search_coverage: SearchCoverage,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UnresolvedOutcomePressure {
    pub unresolved: bool,
    pub attributed_axis: Option<PressureAxis>,
    pub search_coverage: SearchCoverage,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PressureAssessment {
    pub overall: PressureCoverage,
    pub effective_horizon_turns: Option<u8>,
    pub hypotheses: Vec<PressureHypothesis>,
}

pub fn outcome_pressure_evidence(
    evidence: OutcomePressureEvidence,
) -> Option<UnresolvedOutcomePressure> {
    (evidence.hp_loss > 0 || evidence.died).then_some(UnresolvedOutcomePressure {
        unresolved: true,
        attributed_axis: None,
        search_coverage: evidence.search_coverage,
    })
}

pub fn assess_survival_pressure(contract: SurvivalPressureContract) -> PressureAssessment {
    let (Some(threat_turns), Some(resolution_turns)) =
        (contract.threat_turns, contract.resolution_turns)
    else {
        return PressureAssessment {
            overall: PressureCoverage::Unknown,
            effective_horizon_turns: contract.threat_turns,
            hypotheses: Vec::new(),
        };
    };

    let effective_horizon_turns = if contract.repeatable_delay {
        u8::MAX
    } else {
        threat_turns.saturating_add(contract.finite_delay_turns)
    };
    let resolves_in_time = resolution_turns <= effective_horizon_turns;
    let overall = if !resolves_in_time {
        PressureCoverage::Open
    } else if contract.deployability == PressureCoverage::Covered {
        PressureCoverage::Covered
    } else {
        PressureCoverage::PartiallyCovered
    };

    PressureAssessment {
        overall,
        effective_horizon_turns: Some(effective_horizon_turns),
        hypotheses: Vec::new(),
    }
}
```

- [ ] **Step 4: Run focused pressure tests and verify green**

Run: `cargo test --lib pressure_assessment::tests`

Expected: five tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/pressure_assessment.rs
git commit -m "feat: add survival pressure contracts"
```

---

### Task 2: Candidate Pressure Responses From Existing Semantics

**Files:**
- Create: `src/ai/strategy/candidate_pressure_response.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Consumes: `PressureAxis`, card definitions, and `RewardAdmission` reasons.
- Produces: `StrategyCommitmentKind`, `CandidatePressureResponse`, and `assess_candidate_pressure_response(card, admission)`.
- Boundary: mappings use semantic effects and requirements, never a production card-ID allowlist.

- [ ] **Step 1: Write failing semantic response tests**

Create `candidate_pressure_response.rs` with these tests:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::reward_admission::assess_reward_admission;

    #[test]
    fn shockwave_exposes_tempo_and_delay_responses() {
        let admission = assess_reward_admission(&[], CardId::Shockwave);
        let response = assess_candidate_pressure_response(Some((CardId::Shockwave, 0)), &admission);

        assert!(response.axes.contains(&PressureAxis::ResolutionTempo));
        assert!(response.axes.contains(&PressureAxis::DelayCapacity));
    }

    #[test]
    fn corruption_opens_an_exhaust_commitment_without_card_id_rules() {
        let admission = assess_reward_admission(&[], CardId::Corruption);
        let response = assess_candidate_pressure_response(Some((CardId::Corruption, 0)), &admission);

        assert!(response.axes.contains(&PressureAxis::GrowthHorizon));
        assert!(response
            .opens_commitments
            .contains(&StrategyCommitmentKind::ExhaustEngine));
    }

    #[test]
    fn rupture_opens_self_damage_commitment_from_semantic_requirement() {
        let admission = assess_reward_admission(&[CardId::Offering], CardId::Rupture);
        let response = assess_candidate_pressure_response(Some((CardId::Rupture, 0)), &admission);

        assert!(response
            .opens_commitments
            .contains(&StrategyCommitmentKind::SelfDamageEngine));
    }

    #[test]
    fn offering_supports_self_damage_but_does_not_claim_repeatability() {
        let admission = assess_reward_admission(&[CardId::Rupture], CardId::Offering);
        let response = assess_candidate_pressure_response(Some((CardId::Offering, 0)), &admission);

        assert!(response
            .supports_commitments
            .contains(&StrategyCommitmentKind::SelfDamageEngine));
        assert!(!response.repeatable_self_damage_supply);
    }

    #[test]
    fn recurring_power_supports_repeatable_self_damage_from_handlers() {
        let admission = assess_reward_admission(&[CardId::Rupture], CardId::Brutality);
        let response = assess_candidate_pressure_response(Some((CardId::Brutality, 0)), &admission);

        assert!(response
            .supports_commitments
            .contains(&StrategyCommitmentKind::SelfDamageEngine));
        assert!(response.repeatable_self_damage_supply);
    }
}
```

- [ ] **Step 2: Register the module and verify red**

Add to `strategy/mod.rs`:

```rust
pub mod candidate_pressure_response;
```

Run: `cargo test --lib shockwave_exposes_tempo_and_delay_responses`

Expected: compilation fails because response types and the assessment function do not exist.

- [ ] **Step 3: Implement semantic response derivation**

Add imports and types:

```rust
use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CombatEvent, InstalledRule, Mechanic, PayoffRequirement,
    PlayEffect, TriggeredEffect,
};
use crate::ai::strategy::package_transition::PackageKind;
use crate::ai::strategy::pressure_assessment::PressureAxis;
use crate::ai::strategy::reward_admission::{RewardAdmission, RewardAdmissionReason};
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyCommitmentKind {
    ExhaustEngine,
    SelfDamageEngine,
    StrengthScaling,
    BlockEngine,
    UpgradeAccess,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CandidatePressureResponse {
    pub axes: Vec<PressureAxis>,
    pub opens_commitments: Vec<StrategyCommitmentKind>,
    pub supports_commitments: Vec<StrategyCommitmentKind>,
    pub repeatable_self_damage_supply: bool,
}
```

Implement helpers and assessment:

```rust
pub fn assess_candidate_pressure_response(
    card: Option<(CardId, u8)>,
    admission: &RewardAdmission,
) -> CandidatePressureResponse {
    let mut response = CandidatePressureResponse::default();
    for reason in &admission.reasons {
        match reason {
            RewardAdmissionReason::FrontloadDamage
            | RewardAdmissionReason::Provides(Mechanic::Vulnerable) => {
                push_unique(&mut response.axes, PressureAxis::ResolutionTempo)
            }
            RewardAdmissionReason::AreaDamage => {
                push_unique(&mut response.axes, PressureAxis::MultiTargetControl)
            }
            RewardAdmissionReason::Provides(
                Mechanic::Block | Mechanic::Weak | Mechanic::EnemyStrengthDown,
            ) => push_unique(&mut response.axes, PressureAxis::DelayCapacity),
            RewardAdmissionReason::Provides(Mechanic::CardDraw | Mechanic::Energy)
            | RewardAdmissionReason::CombatUpgrade => {
                push_unique(&mut response.axes, PressureAxis::Deployability)
            }
            RewardAdmissionReason::Provides(Mechanic::Strength | Mechanic::StrengthMultiplier)
            | RewardAdmissionReason::DamageScalesWith(_)
            | RewardAdmissionReason::Installs(_) => {
                push_unique(&mut response.axes, PressureAxis::GrowthHorizon)
            }
            RewardAdmissionReason::Supports(package) => {
                if let Some(kind) = commitment_for_package(*package) {
                    push_unique(&mut response.supports_commitments, kind);
                }
            }
            _ => {}
        }
    }

    if let Some((card, upgrades)) = card {
        let definition = card_definition_with_upgrades(card, upgrades);
        if definition.installed_rules.contains(&InstalledRule::SkillCardsCostZeroAndExhaust) {
            push_unique(&mut response.opens_commitments, StrategyCommitmentKind::ExhaustEngine);
            push_unique(&mut response.axes, PressureAxis::GrowthHorizon);
        }
        if definition.payoff_requirements.contains(&PayoffRequirement::WantsEventStream(
            CombatEvent::CardSelfDamage,
        )) {
            push_unique(
                &mut response.opens_commitments,
                StrategyCommitmentKind::SelfDamageEngine,
            );
            push_unique(&mut response.axes, PressureAxis::GrowthHorizon);
        }
        let emits_direct_self_damage = definition
            .play_effects
            .contains(&PlayEffect::EmitEvent(CombatEvent::CardSelfDamage));
        let emits_triggered_self_damage = definition
            .event_handlers
            .iter()
            .any(|handler| handler.effect == TriggeredEffect::LoseHpFromCard);
        if emits_direct_self_damage || emits_triggered_self_damage {
            push_unique(
                &mut response.supports_commitments,
                StrategyCommitmentKind::SelfDamageEngine,
            );
        }
        response.repeatable_self_damage_supply = emits_triggered_self_damage
            || (emits_direct_self_damage
                && !definition.play_effects.contains(&PlayEffect::ExhaustsSelf));
    }

    response.axes.sort();
    response.opens_commitments.sort();
    response.supports_commitments.sort();
    response
}

fn commitment_for_package(package: PackageKind) -> Option<StrategyCommitmentKind> {
    match package {
        PackageKind::Strength => Some(StrategyCommitmentKind::StrengthScaling),
        PackageKind::Exhaust => Some(StrategyCommitmentKind::ExhaustEngine),
        PackageKind::SelfDamage => Some(StrategyCommitmentKind::SelfDamageEngine),
        PackageKind::Block => Some(StrategyCommitmentKind::BlockEngine),
    }
}

fn push_unique<T: Copy + Eq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}
```

- [ ] **Step 4: Run focused response tests and verify green**

Run: `cargo test --lib candidate_pressure_response::tests`

Expected: five tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/candidate_pressure_response.rs
git commit -m "feat: derive candidate pressure responses"
```

---

### Task 3: Serializable Challenger Policy Memory

**Files:**
- Create: `src/ai/strategy/challenger_policy_state.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Consumes: pressure hypotheses and candidate pressure responses.
- Produces: commitment requirements, horizons, progress events, `StrategyCommitment`, `ChallengerPolicyState`, and methods for sequential divergence, support, completion, and expiry.
- Boundary: state records policy hypotheses; it does not execute owner actions.

- [ ] **Step 1: Write failing memory tests**

Create `challenger_policy_state.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::candidate_pressure_response::CandidatePressureResponse;

    #[test]
    fn challenger_remembers_multiple_sequential_divergences() {
        let mut state = ChallengerPolicyState::new(1);
        state.record_divergence("a2f19", &CandidatePressureResponse::default());
        state.record_divergence("a2f23", &CandidatePressureResponse::default());

        assert_eq!(state.divergence_count, 2);
        assert_eq!(state.last_checkpoint_ref.as_deref(), Some("a2f23"));
    }

    #[test]
    fn active_commitment_recognizes_later_support() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::DecisionBoundaries(3),
            burden_units: 1,
        });
        let response = CandidatePressureResponse {
            supports_commitments: vec![StrategyCommitmentKind::ExhaustEngine],
            ..CandidatePressureResponse::default()
        };

        assert!(state.candidate_supports_active_commitment(&response));
    }

    #[test]
    fn unsupported_commitment_expires_at_its_decision_horizon() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::SelfDamageEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::RepeatableSupply],
            horizon: CommitmentHorizon::DecisionBoundaries(1),
            burden_units: 1,
        });

        state.advance(PolicyProgress::DecisionBoundary);

        assert_eq!(state.commitments[0].status, CommitmentStatus::Expired);
    }

    #[test]
    fn satisfying_last_requirement_completes_commitment() {
        let mut state = ChallengerPolicyState::new(1);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::SelfDamageEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::RepeatableSupply],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });

        state.observe_requirement(
            StrategyCommitmentKind::SelfDamageEngine,
            CommitmentRequirement::RepeatableSupply,
        );

        assert_eq!(state.commitments[0].status, CommitmentStatus::Completed);
    }

    #[test]
    fn challenger_policy_state_round_trips_through_json() {
        let mut state = ChallengerPolicyState::new(2);
        state.open_commitment(StrategyCommitment {
            kind: StrategyCommitmentKind::ExhaustEngine,
            status: CommitmentStatus::Active,
            requirements: vec![CommitmentRequirement::Payoff],
            horizon: CommitmentHorizon::CurrentActBoss,
            burden_units: 1,
        });

        let json = serde_json::to_string(&state).expect("policy state should serialize");
        let restored: ChallengerPolicyState =
            serde_json::from_str(&json).expect("policy state should deserialize");

        assert_eq!(restored, state);
    }
}
```

- [ ] **Step 2: Register the module and verify red**

Add to `strategy/mod.rs`:

```rust
pub mod challenger_policy_state;
```

Run: `cargo test --lib challenger_remembers_multiple_sequential_divergences`

Expected: compilation fails because challenger policy-state types do not exist.

- [ ] **Step 3: Implement policy memory**

Add imports and types above the tests:

```rust
use crate::ai::strategy::candidate_pressure_response::{
    CandidatePressureResponse, StrategyCommitmentKind,
};
use crate::ai::strategy::pressure_assessment::PressureHypothesis;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentRequirement {
    RepeatableSupply,
    Source,
    Payoff,
    Deployability,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentStatus {
    Active,
    Completed,
    Abandoned,
    Expired,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentHorizon {
    DecisionBoundaries(u8),
    NextEliteOrBoss,
    CurrentActBoss,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProgress {
    DecisionBoundary,
    EliteReached,
    BossReached,
    ActAdvanced,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StrategyCommitment {
    pub kind: StrategyCommitmentKind,
    pub status: CommitmentStatus,
    pub requirements: Vec<CommitmentRequirement>,
    pub horizon: CommitmentHorizon,
    pub burden_units: u8,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChallengerPolicyState {
    pub lane_id: u8,
    pub active_pressure: Vec<PressureHypothesis>,
    pub commitments: Vec<StrategyCommitment>,
    pub divergence_count: u16,
    pub last_checkpoint_ref: Option<String>,
}
```

Implement methods:

```rust
impl ChallengerPolicyState {
    pub fn new(lane_id: u8) -> Self {
        Self {
            lane_id,
            active_pressure: Vec::new(),
            commitments: Vec::new(),
            divergence_count: 0,
            last_checkpoint_ref: None,
        }
    }

    pub fn record_divergence(
        &mut self,
        checkpoint_ref: impl Into<String>,
        response: &CandidatePressureResponse,
    ) {
        self.divergence_count = self.divergence_count.saturating_add(1);
        self.last_checkpoint_ref = Some(checkpoint_ref.into());
        for kind in &response.opens_commitments {
            if !self.commitments.iter().any(|commitment| {
                commitment.kind == *kind && commitment.status == CommitmentStatus::Active
            }) {
                self.open_commitment(StrategyCommitment {
                    kind: *kind,
                    status: CommitmentStatus::Active,
                    requirements: default_requirements(*kind),
                    horizon: CommitmentHorizon::CurrentActBoss,
                    burden_units: 0,
                });
            }
        }
    }

    pub fn open_commitment(&mut self, commitment: StrategyCommitment) {
        self.commitments.push(commitment);
    }

    pub fn candidate_supports_active_commitment(
        &self,
        response: &CandidatePressureResponse,
    ) -> bool {
        self.commitments.iter().any(|commitment| {
            commitment.status == CommitmentStatus::Active
                && response.supports_commitments.contains(&commitment.kind)
        })
    }

    pub fn observe_requirement(
        &mut self,
        kind: StrategyCommitmentKind,
        requirement: CommitmentRequirement,
    ) {
        if let Some(commitment) = self.commitments.iter_mut().find(|commitment| {
            commitment.kind == kind && commitment.status == CommitmentStatus::Active
        }) {
            commitment.requirements.retain(|item| *item != requirement);
            if commitment.requirements.is_empty() {
                commitment.status = CommitmentStatus::Completed;
            }
        }
    }

    pub fn advance(&mut self, progress: PolicyProgress) {
        for commitment in &mut self.commitments {
            if commitment.status != CommitmentStatus::Active {
                continue;
            }
            let expires = match (&mut commitment.horizon, progress) {
                (CommitmentHorizon::DecisionBoundaries(remaining), PolicyProgress::DecisionBoundary) => {
                    *remaining = remaining.saturating_sub(1);
                    *remaining == 0
                }
                (CommitmentHorizon::NextEliteOrBoss, PolicyProgress::EliteReached | PolicyProgress::BossReached) => true,
                (CommitmentHorizon::CurrentActBoss, PolicyProgress::BossReached | PolicyProgress::ActAdvanced) => true,
                _ => false,
            };
            if expires {
                commitment.status = CommitmentStatus::Expired;
            }
        }
    }
}

fn default_requirements(kind: StrategyCommitmentKind) -> Vec<CommitmentRequirement> {
    match kind {
        StrategyCommitmentKind::ExhaustEngine => vec![CommitmentRequirement::Payoff],
        StrategyCommitmentKind::SelfDamageEngine => {
            vec![CommitmentRequirement::RepeatableSupply]
        }
        StrategyCommitmentKind::StrengthScaling => vec![CommitmentRequirement::Source],
        StrategyCommitmentKind::BlockEngine => vec![CommitmentRequirement::Source],
        StrategyCommitmentKind::UpgradeAccess => vec![CommitmentRequirement::Deployability],
    }
}
```

- [ ] **Step 4: Run focused memory tests and verify green**

Run: `cargo test --lib challenger_policy_state::tests`

Expected: five tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/challenger_policy_state.rs
git commit -m "feat: add challenger policy memory"
```

---

### Task 4: Challenger Signatures And Semantic Deduplication

**Files:**
- Create: `src/ai/strategy/challenger_signature.rs`
- Modify: `src/ai/strategy/mod.rs`

**Interfaces:**
- Consumes: `ChallengerPolicyState`.
- Produces: `DeckBurdenBand`, `DeployabilityBand`, `ChallengerSignature`, `ChallengerLaneSnapshot`, and `retain_distinct_challengers`.
- Contract: retain at most two semantically distinct challengers and prefer stronger evidence only within equal signatures.

- [ ] **Step 1: Write failing signature tests**

Create `challenger_signature.rs` with:

```rust
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::strategy::pressure_assessment::{
        EvidenceConfidence, PressureCoverage, PressureHypothesis,
    };

    fn lane(lane_id: u8, axis: PressureAxis, evidence_rank: u16) -> ChallengerLaneSnapshot {
        let mut policy = ChallengerPolicyState::new(lane_id);
        policy.active_pressure.push(PressureHypothesis {
            axis,
            coverage: PressureCoverage::Open,
            confidence: EvidenceConfidence::High,
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
        });
        ChallengerLaneSnapshot {
            policy,
            burden: DeckBurdenBand::Heavy,
            deployability: DeployabilityBand::Thin,
            evidence_rank,
        }
    }

    #[test]
    fn equivalent_challengers_keep_the_stronger_evidence() {
        let retained = retain_distinct_challengers(vec![
            lane(1, PressureAxis::ResolutionTempo, 4),
            lane(2, PressureAxis::ResolutionTempo, 9),
        ]);

        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].policy.lane_id, 2);
    }

    #[test]
    fn distinct_pressure_hypotheses_survive_up_to_two_lanes() {
        let retained = retain_distinct_challengers(vec![
            lane(1, PressureAxis::ResolutionTempo, 9),
            lane(2, PressureAxis::DelayCapacity, 8),
            lane(3, PressureAxis::Deployability, 1),
        ]);

        assert_eq!(retained.len(), 2);
        assert!(retained.iter().any(|lane| lane.policy.lane_id == 1));
        assert!(retained.iter().any(|lane| lane.policy.lane_id == 2));
    }
}
```

- [ ] **Step 2: Register the module and verify red**

Add to `strategy/mod.rs`:

```rust
pub mod challenger_signature;
```

Run: `cargo test --lib equivalent_challengers_keep_the_stronger_evidence`

Expected: compilation fails because signature types do not exist.

- [ ] **Step 3: Implement signatures and bounded deduplication**

Add imports and types:

```rust
use std::collections::BTreeMap;

use crate::ai::strategy::candidate_pressure_response::StrategyCommitmentKind;
use crate::ai::strategy::challenger_policy_state::{ChallengerPolicyState, CommitmentStatus};
use crate::ai::strategy::pressure_assessment::PressureAxis;

const MAX_CHALLENGERS: usize = 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeckBurdenBand {
    Clean,
    Watch,
    Heavy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeployabilityBand {
    Thin,
    Adequate,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ChallengerSignature {
    pub pressure_axes: Vec<PressureAxis>,
    pub active_commitments: Vec<StrategyCommitmentKind>,
    pub burden: DeckBurdenBand,
    pub deployability: DeployabilityBand,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChallengerLaneSnapshot {
    pub policy: ChallengerPolicyState,
    pub burden: DeckBurdenBand,
    pub deployability: DeployabilityBand,
    pub evidence_rank: u16,
}
```

Implement signature and retention:

```rust
impl ChallengerLaneSnapshot {
    pub fn signature(&self) -> ChallengerSignature {
        let mut pressure_axes = self
            .policy
            .active_pressure
            .iter()
            .map(|hypothesis| hypothesis.axis)
            .collect::<Vec<_>>();
        pressure_axes.sort();
        pressure_axes.dedup();

        let mut active_commitments = self
            .policy
            .commitments
            .iter()
            .filter(|commitment| commitment.status == CommitmentStatus::Active)
            .map(|commitment| commitment.kind)
            .collect::<Vec<_>>();
        active_commitments.sort();
        active_commitments.dedup();

        ChallengerSignature {
            pressure_axes,
            active_commitments,
            burden: self.burden,
            deployability: self.deployability,
        }
    }
}

pub fn retain_distinct_challengers(
    lanes: Vec<ChallengerLaneSnapshot>,
) -> Vec<ChallengerLaneSnapshot> {
    let mut by_signature = BTreeMap::<ChallengerSignature, ChallengerLaneSnapshot>::new();
    for lane in lanes {
        let signature = lane.signature();
        let replace = match by_signature.get(&signature) {
            None => true,
            Some(existing) => lane.evidence_rank > existing.evidence_rank,
        };
        if replace {
            by_signature.insert(signature, lane);
        }
    }
    let mut retained = by_signature.into_values().collect::<Vec<_>>();
    retained.sort_by(|left, right| right.evidence_rank.cmp(&left.evidence_rank));
    retained.truncate(MAX_CHALLENGERS);
    retained
}
```

- [ ] **Step 4: Run focused signature tests and verify green**

Run: `cargo test --lib challenger_signature::tests`

Expected: two tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/challenger_signature.rs
git commit -m "feat: deduplicate challenger policy signatures"
```

---

### Task 5: Evidence-Only Pressure Responses In Path Artifacts

**Files:**
- Modify: `src/runtime/branch/owner_audit/branch_path.rs:1-380`

**Interfaces:**
- Consumes: `assess_candidate_pressure_response` and existing candidate/admission annotations.
- Produces: optional `pressure_response` JSON on candidate snapshots.
- Contract: serialization changes only; candidate score, lane, expansion, and selected choice remain unchanged.

- [ ] **Step 1: Write a failing snapshot test**

Add this test to the existing `branch_path.rs` test module:

```rust
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
```

- [ ] **Step 2: Run the test and verify red**

Run: `cargo test --bin branch_tiny branch_path`

Expected: compilation fails because the `Candidate` snapshot variant has no `pressure_response` field.

- [ ] **Step 3: Add pressure response serialization**

Add imports:

```rust
use sts_simulator::ai::strategy::candidate_pressure_response::assess_candidate_pressure_response;
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
```

Add the field to `ChoiceAnnotationSnapshot::Candidate`:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pressure_response: Option<Value>,
```

Add this helper near the snapshot conversion:

```rust
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
    let admission = admission?;
    Some(json!(assess_candidate_pressure_response(
        candidate_card_identity(kind),
        admission,
    )))
}
```

Populate the new field in `from_annotation`:

```rust
pressure_response: pressure_response_value(
    decision.evaluation.candidate.kind,
    decision.admission.as_ref(),
),
```

- [ ] **Step 4: Run branch path and library foundation tests**

Run:

```powershell
cargo test --bin branch_tiny branch_path
cargo test --lib pressure_assessment::tests
cargo test --lib candidate_pressure_response::tests
cargo test --lib challenger_policy_state::tests
cargo test --lib challenger_signature::tests
```

Expected: all matching tests pass; existing lane and score assertions remain unchanged.

- [ ] **Step 5: Commit**

```powershell
git add src/runtime/branch/owner_audit/branch_path.rs
git commit -m "feat: expose candidate pressure responses"
```

---

### Task 6: Completion Verification

**Files:**
- Verify: all files changed in Tasks 1-5

**Interfaces:**
- Consumes: the completed foundation.
- Produces: evidence that the new serializable strategy primitives and artifact annotation preserve existing runtime behavior.

- [ ] **Step 1: Format and inspect**

```powershell
cargo fmt --all
cargo fmt --all -- --check
git diff --check
git status --short
```

Expected: format and diff checks pass; status contains only intentional formatter changes, if any.

- [ ] **Step 2: Run the full library suite once**

Run: `cargo test --lib`

Expected: all library tests pass.

- [ ] **Step 3: Run branch-tiny tests once**

Run: `cargo test --bin branch_tiny`

Expected: all branch-tiny tests pass.

- [ ] **Step 4: Run architecture boundaries once**

Run: `cargo test --test architecture_runtime_boundaries`

Expected: all seven architecture boundary tests pass.

- [ ] **Step 5: Verify serde round trips and no behavior wiring**

Run:

```powershell
rg -n "pressure_assessment|candidate_pressure_response|challenger_policy_state|challenger_signature" src/ai/strategy src/runtime/branch/owner_audit/branch_path.rs
$behaviorMatches = rg -n "assess_candidate_pressure_response" src/ai/strategy/decision_pipeline.rs src/runtime/branch/owner_audit/owner_choice_expander.rs
if ($LASTEXITCODE -eq 0) { $behaviorMatches; throw "pressure response is wired into production behavior" }
if ($LASTEXITCODE -ne 1) { throw "rg failed while checking behavior wiring" }
```

Expected: new modules are registered and used for artifact explanation; the second command returns no matches, proving the response does not affect decision scoring or owner expansion.

- [ ] **Step 6: Commit formatter changes only when present**

```powershell
git add src/ai/strategy/mod.rs src/ai/strategy/pressure_assessment.rs src/ai/strategy/candidate_pressure_response.rs src/ai/strategy/challenger_policy_state.rs src/ai/strategy/challenger_signature.rs src/runtime/branch/owner_audit/branch_path.rs
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) { git commit -m "style: format challenger policy foundation" }
```

- [ ] **Step 7: Record final state**

```powershell
git status --short --branch
git log -7 --oneline
```

Expected: worktree clean; history contains the design, plan, and five focused implementation commits.
