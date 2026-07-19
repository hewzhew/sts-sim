use std::time::Duration;

use serde::Serialize;

use crate::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2Config, CombatSearchV2PotionPolicy,
    CombatSearchV2Satisfaction, SearchCoverageStatus, SearchTerminalLabel,
};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyCapabilityCoverageV1,
    StrategyCapabilityKindV1,
};
use crate::content::monsters::factory::EncounterId;
use crate::runtime::rng::RngPool;
use crate::sim::combat_start::build_natural_combat_start;
use crate::state::map::node::RoomType;
use crate::state::run::RunState;

pub const STRATEGIC_ENCOUNTER_PROBE_SCHEMA_NAME: &str = "StrategicEncounterProbeReport";
pub const STRATEGIC_ENCOUNTER_PROBE_SCHEMA_VERSION: u32 = 4;

/// A deliberately small diagnostic budget. The probe is evidence for a
/// shadow evaluator; it is not allowed to materialize a run successor or
/// prove that an encounter is unwinnable.
#[derive(Clone, Copy, Debug)]
pub struct StrategicEncounterProbeBudgetV1 {
    pub max_nodes_per_encounter: usize,
    pub wall_ms_per_encounter: u64,
    pub hp_basis: StrategicEncounterProbeHpBasisV1,
    /// Potion availability is an explicit experimental variable.  In
    /// particular, checkpoint potion counterfactuals must not silently run
    /// under `Never`, because that would make restoring potions a no-op.
    pub potion_use: StrategicEncounterProbePotionUseV1,
}

impl Default for StrategicEncounterProbeBudgetV1 {
    fn default() -> Self {
        Self {
            max_nodes_per_encounter: 25_000,
            wall_ms_per_encounter: 500,
            hp_basis: StrategicEncounterProbeHpBasisV1::Current,
            potion_use: StrategicEncounterProbePotionUseV1::Disabled,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicEncounterProbeHpBasisV1 {
    Current,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StrategicEncounterProbePotionUseV1 {
    Disabled,
    /// Uses the same semantic action filter for every paired variant while
    /// keeping the maximum number of consumed potions visible in the report.
    SemanticBudgeted {
        max_uses: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct StrategicEncounterProbeSpecV1 {
    pub probe_id: &'static str,
    pub encounter: EncounterId,
    pub room_type: RoomType,
    pub probe_seed: u64,
    pub capabilities_under_test: Vec<StrategyCapabilityKindV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicCapabilityPredictionV1 {
    pub capability: StrategyCapabilityKindV1,
    pub predicted_coverage: StrategyCapabilityCoverageV1,
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicEncounterWinObservationV1 {
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicEncounterFrontierObservationV1 {
    pub player_hp: i32,
    pub turn: u32,
    pub living_enemy_count: usize,
    pub total_enemy_hp: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicEncounterRolloutObservationV1 {
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub living_enemy_count: usize,
    pub total_enemy_hp: i32,
    pub survival_margin: i32,
    pub actions_simulated: usize,
    pub truncated: bool,
    pub stop_reason: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StrategicEncounterPrimaryEvidenceV1 {
    ExactWitness {
        witness: StrategicEncounterWinObservationV1,
    },
    ExhaustiveRefutation,
    BudgetUnknown,
    SetupError {
        message: String,
    },
}

/// Approximate evidence is deliberately stored beside, never inside, the
/// primary exact/budget outcome.  A heuristic estimate therefore cannot turn
/// `BudgetUnknown` into a loss or overwrite an exact witness/refutation.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum StrategicEncounterHeuristicEvidenceV1 {
    HeuristicEstimate {
        estimate: StrategicEncounterRolloutObservationV1,
    },
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicEncounterProbeObservationV1 {
    pub probe_id: &'static str,
    pub encounter: EncounterId,
    pub room_type: RoomType,
    pub probe_seed: u64,
    pub initial_hp: i32,
    pub initial_max_hp: i32,
    pub capabilities_under_test: Vec<StrategicCapabilityPredictionV1>,
    pub primary_evidence: StrategicEncounterPrimaryEvidenceV1,
    pub search_coverage_status: SearchCoverageStatus,
    /// Heuristic rollout evidence can order shadow work, but it is never an
    /// exact successor and never changes `primary_evidence`.
    pub heuristic_evidence: Option<StrategicEncounterHeuristicEvidenceV1>,
    pub best_frontier: Option<StrategicEncounterFrontierObservationV1>,
    pub rollout_evaluations: u64,
    pub rollout_elapsed_ms: u128,
    pub nodes_expanded: u64,
    pub nodes_generated: u64,
    pub elapsed_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct StrategicEncounterProbeReportV1 {
    pub schema_name: &'static str,
    pub schema_version: u32,
    /// The suite uses public act pools and fixed diagnostic RNG. It must not
    /// be interpreted as knowledge of the next encounter or its real RNG.
    pub information_boundary: &'static str,
    /// One fixed-RNG observation is a reproducible case, not an expectation
    /// over encounters, shuffles, or search noise.
    pub sample_semantics: &'static str,
    pub act: u8,
    pub floor: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub deck_size: usize,
    pub budget: StrategicEncounterProbeBudgetReportV1,
    pub observations: Vec<StrategicEncounterProbeObservationV1>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct StrategicEncounterProbeBudgetReportV1 {
    pub max_nodes_per_encounter: usize,
    pub wall_ms_per_encounter: u64,
    pub hp_basis: StrategicEncounterProbeHpBasisV1,
    pub potion_use: StrategicEncounterProbePotionUseV1,
}

/// Returns a small offline encounter battery for the current act. Distinct
/// elites remain distinct because they test different capabilities. The list
/// is not the actual future route and must be narrowed by route-visible pools
/// before any future production use.
pub fn strategic_encounter_probe_plan_v1(
    run_state: &RunState,
) -> Vec<StrategicEncounterProbeSpecV1> {
    let mut probes = match run_state.act_num {
        1 => vec![
            probe_spec(
                "act1_gremlin_nob_frontload",
                EncounterId::GremlinNob,
                RoomType::MonsterRoomElite,
                0xA1_001,
                &[
                    StrategyCapabilityKindV1::SingleTargetFrontload,
                    StrategyCapabilityKindV1::CardPlayEfficiency,
                ],
            ),
            probe_spec(
                "act1_three_sentries_control",
                EncounterId::ThreeSentries,
                RoomType::MonsterRoomElite,
                0xA1_002,
                &[
                    StrategyCapabilityKindV1::MultiTargetControl,
                    StrategyCapabilityKindV1::DrawEnergyConsistency,
                ],
            ),
            probe_spec(
                "act1_lagavulin_setup",
                EncounterId::Lagavulin,
                RoomType::MonsterRoomElite,
                0xA1_003,
                &[
                    StrategyCapabilityKindV1::LongFightScaling,
                    StrategyCapabilityKindV1::SustainedDefense,
                ],
            ),
        ],
        2 => vec![
            probe_spec(
                "act2_three_byrds_opening",
                EncounterId::ThreeByrds,
                RoomType::MonsterRoom,
                0xA2_001,
                &[
                    StrategyCapabilityKindV1::MultiTargetControl,
                    StrategyCapabilityKindV1::SingleTargetFrontload,
                ],
            ),
            probe_spec(
                "act2_slavers_frontload_control",
                EncounterId::Slavers,
                RoomType::MonsterRoomElite,
                0xA2_002,
                &[
                    StrategyCapabilityKindV1::SingleTargetFrontload,
                    StrategyCapabilityKindV1::MultiTargetControl,
                ],
            ),
            probe_spec(
                "act2_gremlin_leader_control",
                EncounterId::GremlinLeader,
                RoomType::MonsterRoomElite,
                0xA2_003,
                &[
                    StrategyCapabilityKindV1::MultiTargetControl,
                    StrategyCapabilityKindV1::DrawEnergyConsistency,
                ],
            ),
            probe_spec(
                "act2_book_sustained_defense",
                EncounterId::BookOfStabbing,
                RoomType::MonsterRoomElite,
                0xA2_004,
                &[
                    StrategyCapabilityKindV1::SustainedDefense,
                    StrategyCapabilityKindV1::LongFightScaling,
                ],
            ),
        ],
        3 => vec![
            probe_spec(
                "act3_transient_damage_race",
                EncounterId::Transient,
                RoomType::MonsterRoom,
                0xA3_001,
                &[
                    StrategyCapabilityKindV1::TimedDamageRace,
                    StrategyCapabilityKindV1::CardPlayEfficiency,
                ],
            ),
            probe_spec(
                "act3_shapes_retaliation",
                EncounterId::FourShapes,
                RoomType::MonsterRoom,
                0xA3_002,
                &[
                    StrategyCapabilityKindV1::RetaliationSafeDamage,
                    StrategyCapabilityKindV1::MultiTargetControl,
                ],
            ),
            probe_spec(
                "act3_giant_head_scaling",
                EncounterId::GiantHead,
                RoomType::MonsterRoomElite,
                0xA3_003,
                &[
                    StrategyCapabilityKindV1::LongFightScaling,
                    StrategyCapabilityKindV1::SustainedDefense,
                ],
            ),
            probe_spec(
                "act3_nemesis_consistency",
                EncounterId::TheNemesis,
                RoomType::MonsterRoomElite,
                0xA3_004,
                &[
                    StrategyCapabilityKindV1::DrawEnergyConsistency,
                    StrategyCapabilityKindV1::SingleTargetFrontload,
                ],
            ),
            probe_spec(
                "act3_reptomancer_target_control",
                EncounterId::Reptomancer,
                RoomType::MonsterRoomElite,
                0xA3_005,
                &[
                    StrategyCapabilityKindV1::MultiTargetControl,
                    StrategyCapabilityKindV1::SingleTargetFrontload,
                ],
            ),
        ],
        _ => Vec::new(),
    };

    if let Some(boss) = run_state.boss_key {
        probes.push(probe_spec(
            boss_probe_id(boss),
            boss,
            RoomType::MonsterRoomBoss,
            0xB0_000u64.saturating_add(boss as u64),
            boss_capabilities(boss),
        ));
    }
    probes
}

pub fn run_strategic_encounter_probe_suite_v1(
    run_state: &RunState,
    budget: StrategicEncounterProbeBudgetV1,
) -> StrategicEncounterProbeReportV1 {
    run_strategic_encounter_probes_v1(
        run_state,
        &strategic_encounter_probe_plan_v1(run_state),
        budget,
    )
}

pub fn run_strategic_encounter_probes_v1(
    run_state: &RunState,
    probes: &[StrategicEncounterProbeSpecV1],
    budget: StrategicEncounterProbeBudgetV1,
) -> StrategicEncounterProbeReportV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let observations = probes
        .iter()
        .map(|probe| {
            let predictions = probe
                .capabilities_under_test
                .iter()
                .map(|capability| StrategicCapabilityPredictionV1 {
                    capability: *capability,
                    predicted_coverage: strategy
                        .threat_coverage
                        .capability(*capability)
                        .map(|evidence| evidence.coverage)
                        .unwrap_or(StrategyCapabilityCoverageV1::Unknown),
                })
                .collect();
            run_one_probe(run_state, probe, predictions, budget)
        })
        .collect();

    StrategicEncounterProbeReportV1 {
        schema_name: STRATEGIC_ENCOUNTER_PROBE_SCHEMA_NAME,
        schema_version: STRATEGIC_ENCOUNTER_PROBE_SCHEMA_VERSION,
        information_boundary: "offline_shadow_counterfactual_fixed_act_pool_no_successor_authority",
        sample_semantics: "single_fixed_rng_case_not_an_expectation_or_candidate_comparison",
        act: run_state.act_num,
        floor: run_state.floor_num,
        current_hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        deck_size: run_state.master_deck.len(),
        budget: StrategicEncounterProbeBudgetReportV1 {
            max_nodes_per_encounter: budget.max_nodes_per_encounter,
            wall_ms_per_encounter: budget.wall_ms_per_encounter,
            hp_basis: budget.hp_basis,
            potion_use: budget.potion_use,
        },
        observations,
    }
}

fn run_one_probe(
    run_state: &RunState,
    probe: &StrategicEncounterProbeSpecV1,
    capabilities_under_test: Vec<StrategicCapabilityPredictionV1>,
    budget: StrategicEncounterProbeBudgetV1,
) -> StrategicEncounterProbeObservationV1 {
    let mut probe_run = run_state.clone();
    // Fixed counterfactual RNG prevents the probe from reading or consuming
    // the real run's hidden future streams.
    probe_run.rng_pool = RngPool::new(probe.probe_seed);
    if budget.hp_basis == StrategicEncounterProbeHpBasisV1::Full {
        probe_run.current_hp = probe_run.max_hp;
    }
    let initial_hp = probe_run.current_hp;
    let initial_max_hp = probe_run.max_hp;
    let (engine, combat) =
        match build_natural_combat_start(&mut probe_run, probe.encounter, probe.room_type) {
            Ok(position) => position,
            Err(error) => {
                return failed_observation(
                    probe,
                    capabilities_under_test,
                    initial_hp,
                    initial_max_hp,
                    error,
                )
            }
        };

    let (potion_policy, max_potions_used) = match budget.potion_use {
        StrategicEncounterProbePotionUseV1::Disabled => {
            (CombatSearchV2PotionPolicy::Never, Some(0))
        }
        StrategicEncounterProbePotionUseV1::SemanticBudgeted { max_uses } => {
            (CombatSearchV2PotionPolicy::SemanticBudgeted, Some(max_uses))
        }
    };
    let report = run_combat_search_v2(
        &engine,
        &combat,
        CombatSearchV2Config {
            max_nodes: budget.max_nodes_per_encounter,
            wall_time: Some(Duration::from_millis(budget.wall_ms_per_encounter)),
            satisfaction: CombatSearchV2Satisfaction::BudgetOrExhaustion,
            input_label: Some(format!("strategic_probe:{}", probe.probe_id)),
            potion_policy,
            max_potions_used,
            ..CombatSearchV2Config::default()
        },
    );
    let best_win =
        report
            .best_win_trajectory
            .as_ref()
            .map(|trajectory| StrategicEncounterWinObservationV1 {
                final_hp: trajectory.final_hp,
                hp_loss: trajectory.hp_loss,
                turns: trajectory.turns,
                potions_used: trajectory.potions_used,
            });
    let best_frontier = report.best_frontier_trajectory.as_ref().map(|trajectory| {
        StrategicEncounterFrontierObservationV1 {
            player_hp: trajectory.final_state.player_hp,
            turn: trajectory.final_state.turn_count,
            living_enemy_count: trajectory.final_state.living_enemy_count,
            total_enemy_hp: trajectory.final_state.total_enemy_hp,
        }
    });
    let heuristic_evidence = report
        .rollout
        .best_frontier_estimate
        .as_ref()
        .map(
            |estimate| StrategicEncounterHeuristicEvidenceV1::HeuristicEstimate {
                estimate: StrategicEncounterRolloutObservationV1 {
                    terminal: estimate.terminal,
                    final_hp: estimate.final_hp,
                    hp_loss: estimate.hp_loss,
                    turns: estimate.turns,
                    living_enemy_count: estimate.living_enemy_count,
                    total_enemy_hp: estimate.total_enemy_hp,
                    survival_margin: estimate.survival_margin,
                    actions_simulated: estimate.actions_simulated,
                    truncated: estimate.truncated,
                    stop_reason: estimate.stop_reason,
                },
            },
        );

    StrategicEncounterProbeObservationV1 {
        probe_id: probe.probe_id,
        encounter: probe.encounter,
        room_type: probe.room_type,
        probe_seed: probe.probe_seed,
        initial_hp,
        initial_max_hp,
        capabilities_under_test,
        primary_evidence: if let Some(witness) = best_win {
            StrategicEncounterPrimaryEvidenceV1::ExactWitness { witness }
        } else if report.outcome.exhaustive {
            StrategicEncounterPrimaryEvidenceV1::ExhaustiveRefutation
        } else {
            StrategicEncounterPrimaryEvidenceV1::BudgetUnknown
        },
        search_coverage_status: report.outcome.coverage_status,
        heuristic_evidence,
        best_frontier,
        rollout_evaluations: report.rollout.evaluations,
        rollout_elapsed_ms: report.performance.rollout_estimate_elapsed_us / 1_000,
        nodes_expanded: report.stats.nodes_expanded,
        nodes_generated: report.stats.nodes_generated,
        elapsed_ms: report.stats.elapsed_ms,
    }
}

fn failed_observation(
    probe: &StrategicEncounterProbeSpecV1,
    capabilities_under_test: Vec<StrategicCapabilityPredictionV1>,
    initial_hp: i32,
    initial_max_hp: i32,
    error: String,
) -> StrategicEncounterProbeObservationV1 {
    StrategicEncounterProbeObservationV1 {
        probe_id: probe.probe_id,
        encounter: probe.encounter,
        room_type: probe.room_type,
        probe_seed: probe.probe_seed,
        initial_hp,
        initial_max_hp,
        capabilities_under_test,
        primary_evidence: StrategicEncounterPrimaryEvidenceV1::SetupError { message: error },
        search_coverage_status: SearchCoverageStatus::FrontierOpen,
        heuristic_evidence: None,
        best_frontier: None,
        rollout_evaluations: 0,
        rollout_elapsed_ms: 0,
        nodes_expanded: 0,
        nodes_generated: 0,
        elapsed_ms: 0,
    }
}

fn probe_spec(
    probe_id: &'static str,
    encounter: EncounterId,
    room_type: RoomType,
    probe_seed: u64,
    capabilities: &[StrategyCapabilityKindV1],
) -> StrategicEncounterProbeSpecV1 {
    StrategicEncounterProbeSpecV1 {
        probe_id,
        encounter,
        room_type,
        probe_seed,
        capabilities_under_test: capabilities.to_vec(),
    }
}

fn boss_probe_id(encounter: EncounterId) -> &'static str {
    match encounter {
        EncounterId::TheGuardian => "act1_guardian_phase_control",
        EncounterId::Hexaghost => "act1_hexaghost_damage_race",
        EncounterId::SlimeBoss => "act1_slime_boss_split_control",
        EncounterId::Automaton => "act2_automaton_setup_defense",
        EncounterId::TheChamp => "act2_champ_phase_control",
        EncounterId::Collector => "act2_collector_target_control",
        EncounterId::AwakenedOne => "act3_awakened_one_long_fight",
        EncounterId::TimeEater => "act3_time_eater_card_efficiency",
        EncounterId::DonuAndDeca => "act3_donu_deca_scaling_control",
        _ => "visible_boss_counterfactual",
    }
}

fn boss_capabilities(encounter: EncounterId) -> &'static [StrategyCapabilityKindV1] {
    use StrategyCapabilityKindV1 as Capability;

    match encounter {
        EncounterId::TheGuardian | EncounterId::SlimeBoss | EncounterId::TheChamp => {
            &[Capability::PhaseControl, Capability::SustainedDefense]
        }
        EncounterId::Hexaghost => &[Capability::TimedDamageRace, Capability::SustainedDefense],
        EncounterId::Automaton => &[Capability::DebuffResilience, Capability::SustainedDefense],
        EncounterId::Collector | EncounterId::DonuAndDeca => {
            &[Capability::MultiTargetControl, Capability::LongFightScaling]
        }
        EncounterId::AwakenedOne => &[Capability::LongFightScaling, Capability::MultiTargetControl],
        EncounterId::TimeEater => &[Capability::CardPlayEfficiency, Capability::LongFightScaling],
        _ => &[
            Capability::SingleTargetFrontload,
            Capability::SustainedDefense,
        ],
    }
}
