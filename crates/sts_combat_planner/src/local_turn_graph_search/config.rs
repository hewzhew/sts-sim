use crate::types::TurnOptionGeneratorConfig;
use crate::witness::OracleCombatWitnessSatisfaction;
use crate::CombatGuideLaneId;

/// Coherent work granted by one guide-selected boundary service.
///
/// Runtime and laboratory hosts share this value so a report's scheduler
/// identity matches production.  It is large enough to make one selected
/// boundary productive without letting a few one-shot guide entries consume
/// the entire bounded allowance before later high-ranked states are visited.
pub const DEFAULT_BACKED_GENERATION_QUANTUM_WORK: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTurnGraphGuideServiceBias {
    pub lane: CombatGuideLaneId,
    pub extra_services_per_cycle: usize,
}

/// Resumable search over a shared graph of exact player-turn boundaries.
///
/// Complete-turn generation remains lazy. Independent global views select one
/// shared boundary node, while the selected node owns its local generation
/// lane. No guide recursively owns the subtree below the state it selected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    /// One deterministic service unit for a selected node's resumable turn
    /// generator. This controls preemption granularity, not search quality.
    pub generation_quantum_work: usize,
    /// Coherent generator service granted when one non-anchor guide selects an
    /// exact boundary. A guide services a shared state once; repeated and
    /// exhaustive coverage remains owned by the anchor queue.
    pub backed_generation_quantum_work: usize,
    /// Optional typed scheduler control that gives one existing boundary guide
    /// additional service turns per round-robin cycle. The lane still owns
    /// one-shot entries and every selected boundary receives the ordinary
    /// guide quantum. The default is disabled; an explicit typed caller may
    /// opt one encounter into a measured service concentration.
    pub guide_service_bias: Option<LocalTurnGraphGuideServiceBias>,
    /// Deterministic work reserved for the first expansion of a selected exact
    /// turn-boundary node. Later resumptions return to the small quantum.
    pub initial_expansion_work: usize,
    /// Root-only discovery batch. Root proposals gate every deeper path, so
    /// they receive a wider but still bounded first expansion.
    pub root_initial_expansion_work: usize,
    /// Maximum number of exact states that may receive an optional expensive
    /// lookahead evaluation during this session.
    pub lookahead_max_evaluations: usize,
    /// Maximum deterministic evaluator work charged to one exact state.
    pub lookahead_work_per_evaluation: usize,
    pub max_turn_depth: usize,
    pub satisfaction: OracleCombatWitnessSatisfaction,
    /// Maximum number of potion resources expended by an accepted witness.
    ///
    /// This is a run-resource contract, not an action prior. Legal prefixes up
    /// to the limit remain searchable, while uses or discards beyond the
    /// remaining allowance are never generated. The spent allowance is also
    /// part of constrained exact-state identity.
    pub max_potions_used: Option<u32>,
}

impl Default for LocalTurnGraphWitnessConfig {
    fn default() -> Self {
        Self {
            generator: TurnOptionGeneratorConfig::default(),
            generation_quantum_work: 4,
            backed_generation_quantum_work: DEFAULT_BACKED_GENERATION_QUANTUM_WORK,
            guide_service_bias: None,
            initial_expansion_work: 64,
            root_initial_expansion_work: 2_048,
            lookahead_max_evaluations: 384,
            lookahead_work_per_evaluation: 24,
            max_turn_depth: 32,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
            max_potions_used: None,
        }
    }
}
