//! Hidden-state sampling conditioned on public agent information.

mod combat;

pub use combat::{
    sample_independent_combat_futures_v1, CombatBeliefParticleOriginV1, CombatBeliefParticleV1,
    CombatBeliefSamplingErrorV1,
};
