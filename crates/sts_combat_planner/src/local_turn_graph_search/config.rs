use crate::types::TurnOptionGeneratorConfig;
use crate::witness_search::OracleCombatWitnessSatisfaction;

/// Resumable search over a shared graph of exact player-turn boundaries.
///
/// Complete-turn generation remains lazy, but Widen and Deepen are decided at
/// the node that owns the alternatives. A deep path therefore does not have
/// to compete against every shallower generator in one global queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalTurnGraphWitnessConfig {
    pub generator: TurnOptionGeneratorConfig,
    /// One deterministic service unit for a selected node's resumable turn
    /// generator. This controls preemption granularity, not search quality.
    pub generation_quantum_work: usize,
    /// Coherent generator service after an exact boundary has earned backed
    /// exploitation. It remains preemptible at the graph level while avoiding
    /// repeated four-work drips on the selected expensive edge.
    pub backed_generation_quantum_work: usize,
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
            backed_generation_quantum_work: 256,
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
