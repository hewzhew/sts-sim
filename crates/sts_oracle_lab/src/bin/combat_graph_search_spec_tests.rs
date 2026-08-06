use sts_combat_planner::{
    CombatGuideLaneId, LocalTurnGraphGuideServiceBias, OracleCombatWitnessSatisfaction,
};

use super::{LocalGraphGuideServiceBiasSpec, LocalGraphSearchSpec};

#[test]
fn spec_serializes_every_effective_setting_and_allowance() {
    let spec = LocalGraphSearchSpec::from_controls(
        240,
        80,
        15,
        7,
        12_345,
        3,
        9,
        Some(0),
        false,
        Some(4),
        Some(96),
        Some(LocalGraphGuideServiceBiasSpec::new(2, 2)),
    );
    let value = serde_json::to_value(spec).expect("serialize search spec");

    assert_eq!(value["planner"]["max_engine_steps_per_transition"], 7);
    assert_eq!(value["planner"]["uniform_exploration_ppm"], 12_345);
    assert_eq!(value["planner"]["allow_potion_expenditure"], false);
    assert_eq!(value["planner"]["allow_potion_discard"], false);
    assert_eq!(value["planner"]["generation_quantum_work"], 3);
    assert_eq!(
        value["planner"]["backed_generation_quantum_work"],
        sts_combat_planner::DEFAULT_BACKED_GENERATION_QUANTUM_WORK
    );
    assert_eq!(value["planner"]["guide_service_bias"]["lane"], 2);
    assert_eq!(
        value["planner"]["guide_service_bias"]["extra_services_per_cycle"],
        2
    );
    assert!(value["planner"].get("lookahead_max_evaluations").is_none());
    assert_eq!(value["planner"]["root_initial_expansion_work"], 64);
    assert_eq!(value["planner"]["max_turn_depth"], 9);
    assert_eq!(value["planner"]["max_potions_used"], 0);
    assert_eq!(value["planner"]["allowed_potion_slots"], 4);
    assert_eq!(value["planner"]["initial_expansion_work"], 96);
    assert_eq!(value["allowance"]["max_selections"], 80);
    assert_eq!(value["allowance"]["max_generation_work"], 240);
    assert_eq!(value["allowance"]["max_engine_steps"], 1_680);
    assert_eq!(value["allowance"]["wall_ms"], 15);
}

#[test]
fn planner_config_and_quantum_are_built_from_the_reported_spec() {
    let spec = LocalGraphSearchSpec::from_controls(
        240,
        80,
        15,
        7,
        12_345,
        3,
        9,
        Some(2),
        true,
        Some(2),
        None,
        Some(LocalGraphGuideServiceBiasSpec::new(2, 1)),
    );
    let config = spec.planner_config(OracleCombatWitnessSatisfaction::HpLossAtMost(4));
    let quantum = spec.quantum();

    assert_eq!(config.generator.max_engine_steps_per_transition, 7);
    assert_eq!(config.generator.uniform_exploration_ppm, 12_345);
    assert!(config.generator.allow_potion_expenditure);
    assert!(config.generator.allow_potion_discard);
    assert_eq!(config.generator.allowed_potion_slots, Some(2));
    assert_eq!(config.generation_quantum_work, 3);
    assert_eq!(
        config.guide_service_bias,
        Some(LocalTurnGraphGuideServiceBias {
            lane: CombatGuideLaneId::new(2),
            extra_services_per_cycle: 1,
        })
    );
    assert_eq!(config.root_initial_expansion_work, 64);
    assert_eq!(config.max_turn_depth, 9);
    assert_eq!(config.max_potions_used, Some(2));
    assert_eq!(quantum.additional_selections, 80);
    assert_eq!(quantum.additional_generation_work, 240);
    assert_eq!(quantum.additional_engine_steps, 1_680);
    assert!(quantum.deadline.is_some());
}
