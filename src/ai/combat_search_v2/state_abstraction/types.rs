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
    TurnRuntimeDelta,
    TurnDrawModifierDelta,
    TurnActionCounterDelta,
    TurnPlayedCardHistoryDelta,
    TurnDiscardCounterDelta,
    TurnOrbHistoryDelta,
    TurnCombatFlagDelta,
    MonsterRuntimeDelta,
    CombatRuntimeHintDelta,
    PotionStateDelta,
    PlayerFutureDelta,
    ZoneRuntimeDelta,
    EngineRuntimeDelta,
    CombatMetaDelta,
    CardUuidDelta,
    PendingQueueDelta,
    IdentityOnlyCandidate,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionLatentDebtKind {
    DiscardOrder,
    CardIdentity,
    TurnPlayedCardHistory,
    ImmediatePublicState,
    TerminalClass,
    LegalActionSet,
    OtherRuntime,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAbstractionCandidateLevel {
    HorizonLimitedCandidate,
    IdentityAuditCandidate,
    ReportOnlyBlocked,
    ReportOnlyUnknown,
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
    pub divergence_group_histogram: Vec<StateAbstractionHistogramEntry>,
    pub divergence_path_histogram: Vec<StateAbstractionHistogramEntry>,
    pub latent_debt_histogram: Vec<StateAbstractionHistogramEntry>,
    pub latent_debt_group_histogram: Vec<StateAbstractionHistogramEntry>,
    pub candidate_level_histogram: Vec<StateAbstractionHistogramEntry>,
    pub candidate_level_group_histogram: Vec<StateAbstractionHistogramEntry>,
    pub recommended_consumer_histogram: Vec<StateAbstractionHistogramEntry>,
    pub reveal_gate_histogram: Vec<StateAbstractionHistogramEntry>,
    pub reveal_gate_group_histogram: Vec<StateAbstractionHistogramEntry>,
    pub identity_audit: StateAbstractionIdentityAuditReport,
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
    pub latent_debt_kind: StateAbstractionLatentDebtKind,
    pub candidate_level: StateAbstractionCandidateLevel,
    pub recommended_consumer: StateAbstractionConsumer,
    pub pruning_allowed: bool,
    pub exact_branch_removal_allowed: bool,
    pub same_effect_turn_sequence_groups: usize,
    pub order_sensitive_turn_sequence_groups: usize,
    pub turn_sequence_divergence_histogram: Vec<StateAbstractionCaseDivergenceCount>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionIdentityAuditReport {
    pub audit_policy: &'static str,
    pub behavioral_effect: &'static str,
    pub status: &'static str,
    pub candidate_cases: usize,
    pub candidate_groups: usize,
    pub proof_pruning_enabled: bool,
    pub exact_branch_removal_allowed: bool,
    pub blocked_reason: &'static str,
    pub required_checks: Vec<&'static str>,
    pub samples: Vec<StateAbstractionIdentityAuditSample>,
    pub notes: Vec<&'static str>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StateAbstractionIdentityAuditSample {
    pub case_id: String,
    pub groups: usize,
    pub first_divergence_path: Option<&'static str>,
    pub guessed_reveal_gate: StateAbstractionRevealGate,
    pub required_next_check: &'static str,
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

impl StateAbstractionLatentDebtKind {
    pub fn label(self) -> &'static str {
        match self {
            StateAbstractionLatentDebtKind::DiscardOrder => "discard_order",
            StateAbstractionLatentDebtKind::CardIdentity => "card_identity",
            StateAbstractionLatentDebtKind::TurnPlayedCardHistory => "turn_played_card_history",
            StateAbstractionLatentDebtKind::ImmediatePublicState => "immediate_public_state",
            StateAbstractionLatentDebtKind::TerminalClass => "terminal_class",
            StateAbstractionLatentDebtKind::LegalActionSet => "legal_action_set",
            StateAbstractionLatentDebtKind::OtherRuntime => "other_runtime",
            StateAbstractionLatentDebtKind::Unknown => "unknown",
        }
    }
}

impl StateAbstractionCandidateLevel {
    pub fn label(self) -> &'static str {
        match self {
            StateAbstractionCandidateLevel::HorizonLimitedCandidate => "horizon_limited_candidate",
            StateAbstractionCandidateLevel::IdentityAuditCandidate => "identity_audit_candidate",
            StateAbstractionCandidateLevel::ReportOnlyBlocked => "report_only_blocked",
            StateAbstractionCandidateLevel::ReportOnlyUnknown => "report_only_unknown",
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
            StateDivergenceKind::TurnRuntimeDelta => "turn_runtime_delta",
            StateDivergenceKind::TurnDrawModifierDelta => "turn_draw_modifier_delta",
            StateDivergenceKind::TurnActionCounterDelta => "turn_action_counter_delta",
            StateDivergenceKind::TurnPlayedCardHistoryDelta => "turn_played_card_history_delta",
            StateDivergenceKind::TurnDiscardCounterDelta => "turn_discard_counter_delta",
            StateDivergenceKind::TurnOrbHistoryDelta => "turn_orb_history_delta",
            StateDivergenceKind::TurnCombatFlagDelta => "turn_combat_flag_delta",
            StateDivergenceKind::MonsterRuntimeDelta => "monster_runtime_delta",
            StateDivergenceKind::CombatRuntimeHintDelta => "combat_runtime_hint_delta",
            StateDivergenceKind::PotionStateDelta => "potion_state_delta",
            StateDivergenceKind::PlayerFutureDelta => "player_future_delta",
            StateDivergenceKind::ZoneRuntimeDelta => "zone_runtime_delta",
            StateDivergenceKind::EngineRuntimeDelta => "engine_runtime_delta",
            StateDivergenceKind::CombatMetaDelta => "combat_meta_delta",
            StateDivergenceKind::CardUuidDelta => "card_uuid_delta",
            StateDivergenceKind::PendingQueueDelta => "pending_queue_delta",
            StateDivergenceKind::IdentityOnlyCandidate => "identity_only_candidate",
            StateDivergenceKind::Unknown => "unknown",
        }
    }
}
