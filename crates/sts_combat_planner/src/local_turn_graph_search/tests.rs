use super::{
    generator_needs_initial_grounding, selected_boundary_generation_work, update_max_guide,
    update_max_rank, GuideRankMap, LocalServiceView, LocalTurnGraphWitnessConfig,
};
use crate::policy::{CombatGuideLaneId, CombatStateGuideRank};

#[test]
fn backed_value_is_monotone_and_keeps_the_best_descendant() {
    let weak = CombatStateGuideRank::new(vec![1, 2]);
    let strong = CombatStateGuideRank::new(vec![1, 3]);
    let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
    let mut backed = None;

    assert!(update_max_rank(&mut backed, &weak));
    assert!(update_max_rank(&mut backed, &strong));
    assert!(!update_max_rank(&mut backed, &weaker_later));
    assert_eq!(backed, Some(strong));
}

#[test]
fn semantic_guide_backup_is_monotone_per_lane() {
    let lane = crate::policy::CombatGuideLaneId::new(4);
    let weak = CombatStateGuideRank::new(vec![1, 2]);
    let strong = CombatStateGuideRank::new(vec![1, 3]);
    let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
    let mut backed = GuideRankMap::default();

    assert!(update_max_guide(&mut backed, lane, &weak));
    assert!(update_max_guide(&mut backed, lane, &strong));
    assert!(!update_max_guide(&mut backed, lane, &weaker_later));
    assert_eq!(backed.get(&lane), Some(&strong));
}

#[test]
fn live_generator_receives_initial_grounding_even_if_an_external_edge_exists() {
    assert!(generator_needs_initial_grounding(0, false));
    assert!(!generator_needs_initial_grounding(1, false));
    assert!(!generator_needs_initial_grounding(0, true));
}

#[test]
fn fresh_guide_boundary_receives_the_coherent_guide_quantum() {
    let config = LocalTurnGraphWitnessConfig {
        backed_generation_quantum_work: 128,
        initial_expansion_work: 64,
        ..LocalTurnGraphWitnessConfig::default()
    };

    assert_eq!(
        selected_boundary_generation_work(
            &config,
            1,
            0,
            LocalServiceView::Guide(CombatGuideLaneId::new(7)),
        ),
        128
    );
    assert_eq!(
        selected_boundary_generation_work(&config, 1, 0, LocalServiceView::Anchor),
        64
    );
}
