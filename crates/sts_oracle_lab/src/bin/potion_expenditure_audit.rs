//! Read-only, identity-preserving potion expenditure counterfactuals.
//!
//! Every lane starts from the same exact combat root. The planner filters
//! explicit use/discard inputs by slot instead of deleting inventory from the
//! state, so potion-sensitive simulator behavior and RNG remain unchanged.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

use clap::Args;
use serde::Serialize;
use sts_combat_planner::{
    CombatDecisionRoot, LocalTurnGraphWitnessInterruption, LocalTurnGraphWitnessStatus,
    OracleCombatWitnessSatisfaction, TurnOptionAction,
};
use sts_oracle_runtime::ai::card_semantics_v1::{
    potion_acquisition_traits_v1, PotionAcquisitionTraitV1,
};
use sts_oracle_runtime::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit, DeckStrategicDeficit,
};
use sts_oracle_runtime::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_oracle_runtime::content::cards::{get_card_definition, is_starter_basic, CardType};
use sts_oracle_runtime::content::potions::{Potion, PotionId};
use sts_oracle_runtime::content::relics::{energy_master_delta, RelicId};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::run_control::{
    existing_combat_knowledge_policy_v1, oracle_potion_rescue_tier_v1, OraclePotionRescueTierV1,
};
use sts_oracle_runtime::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, EngineCombatStepper,
};
use sts_oracle_runtime::state::core::ClientInput;

use super::combat_graph_search_spec::LocalGraphSearchSpec;

const SCHEMA_NAME: &str = "OracleCombatCasePotionExpenditureAuditV4";

#[derive(Debug, Args)]
pub(super) struct CombatCasePotionExpenditureAuditArgs {
    /// Exact combat root to audit. The file is loaded read-only.
    #[arg(long)]
    case: PathBuf,
    /// Largest initial-potion subset opened in one isolated lane.
    /// Zero runs only the no-potion lane.
    #[arg(long, default_value_t = 1)]
    max_combination_size: usize,
    /// Safety bound for combinatorial lane expansion, including no-potion.
    #[arg(long, default_value_t = 16)]
    max_lanes: usize,
    /// Optional strategic final-HP reserve reported for every exact witness.
    #[arg(long)]
    survival_reserve_hp: Option<i32>,
    /// Exact generation work granted independently to every lane.
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    /// Scheduler selections granted independently to every lane.
    #[arg(long, default_value_t = 1_000_000)]
    max_selections: usize,
    /// Wall allowance in milliseconds for each lane, not for the whole audit.
    #[arg(long, default_value_t = 10_000)]
    wall_ms_per_lane: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 50_000)]
    uniform_exploration_ppm: u32,
    #[arg(long, default_value_t = 4)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_depth: usize,
    /// Contract assertion for durable case-specific regression commands.
    #[arg(long)]
    expect_no_potion_min_final_hp: Option<i32>,
    /// Require the no-potion witness to Pareto-dominate every compliant
    /// witness that actually consumes a potion.
    #[arg(long)]
    expect_no_potion_dominates_consuming: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionVerifiedWinRescueTierV1 {
    BoundedQuality,
    FindAnyWin,
    Excluded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionSharedStrategyTraitV1 {
    CombatDamage,
    AoeDamage,
    CombatBlock,
    VulnerableSetup,
    WeakControl,
    EnergyBurst,
    StrengthGain,
    CardAccess,
    ActionAmplifier,
    DeathInsurance,
    DebuffControl,
    EscapeTool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionSharedStrategyCoverageV1 {
    Classified,
    Unclassified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionMechanicalRoleV1 {
    SingleTargetDamage,
    MultiTargetDamage,
    DamageOverTime,
    WeakControl,
    VulnerableControl,
    ImmediateBlock,
    ImmediateHealing,
    EnergyBurst,
    PersistentStrength,
    PersistentDexterity,
    TemporaryDexterity,
    TemporaryStrength,
    CardDraw,
    PersistentFocus,
    RandomAttackDiscovery,
    RandomSkillDiscovery,
    RandomPowerDiscovery,
    RandomColorlessDiscovery,
    MiracleGeneration,
    TemporaryUpgrade,
    Artifact,
    DelayedHealing,
    PlatedArmor,
    Thorns,
    RandomTopdeckPlay,
    NextCardDuplication,
    ShivGeneration,
    OrbCapacity,
    DiscardRecovery,
    HandRedraw,
    HandExhaust,
    StanceControl,
    DeathInsurance,
    Escape,
    MaxHpGain,
    PotionGeneration,
    CardDrawAndCostRandomization,
    Intangible,
    Metallicize,
    RitualScaling,
    Divinity,
    DarkOrbGeneration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionContinuationDependencyV1 {
    CurrentHpDeficit,
    FutureEncounterDamagePattern,
    FutureEnemyCountAndHealth,
    FutureFightLength,
    FutureHandAndDrawOrder,
    FutureDiscardState,
    DeckSynergy,
    RandomOutcomePool,
    HighValueCardTarget,
    DebuffTiming,
    LowHpInsuranceNeed,
    RouteEscapeValue,
    EmptyPotionSlotsAndAcquisitionRules,
    OrbPlan,
    StancePlan,
    OutOfCombatTiming,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PotionResourceV1 {
    slot: usize,
    id: String,
    uuid: u32,
    can_use: bool,
    can_discard: bool,
    verified_win_rescue_tier: PotionVerifiedWinRescueTierV1,
    shared_strategy_traits: Vec<PotionSharedStrategyTraitV1>,
    shared_strategy_coverage: PotionSharedStrategyCoverageV1,
    mechanical_role: PotionMechanicalRoleV1,
    continuation_dependencies: Vec<PotionContinuationDependencyV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionExpenditureModeV1 {
    Use,
    Discard,
    Passive,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PotionExpenditureEventV1 {
    action_index: usize,
    slot: usize,
    id: String,
    uuid: u32,
    mode: PotionExpenditureModeV1,
    verified_win_rescue_tier: PotionVerifiedWinRescueTierV1,
}

#[derive(Clone, Debug)]
struct PotionAuditLaneSpec {
    lane_id: String,
    allowed_slot_mask: u64,
    allowed_potions: Vec<PotionResourceV1>,
    max_explicit_expenditures: u32,
}

#[derive(Clone, Debug, Serialize)]
struct PotionAuditSearchSettingsV1 {
    max_combination_size: usize,
    max_lanes: usize,
    survival_reserve_hp: Option<i32>,
    max_nodes_per_lane: usize,
    max_selections_per_lane: usize,
    wall_ms_per_lane: u64,
    max_engine_steps_per_transition: usize,
    uniform_exploration_ppm: u32,
    generation_quantum_work: usize,
    max_turn_depth: usize,
    satisfaction: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct PotionAuditLaneCountersV1 {
    selections: usize,
    generation_work: usize,
    engine_steps: usize,
    exact_nodes: usize,
    terminal_win_options: usize,
    witness_replay_attempts: usize,
    witness_replay_improvements: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionMarginalAssessmentV1 {
    NoPotionBaseline,
    NoPotionFrontierExhaustedUnderContract,
    NoPotionWitnessNotFoundUnderAllowance,
    CrossesSurvivalReserve,
    ImprovesFinalHp,
    SameFinalHpWithExtraResource,
    WorseFinalHpWithExtraResource,
    NoAdditionalPotionConsumed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum VerifiedWinPotionDispositionV1 {
    NoPotionSpent,
    BoundedQualityOnly,
    ContainsReservedResource,
    ContainsExcludedResource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionPolicyReviewFlagV1 {
    ReservedResourceCrossesSurvivalReserve,
    ReservedResourceImprovesHpWithoutCrossingReserve,
    AdmittedResourceIsParetoDominated,
    AdmittedResourceHasNoHpBenefit,
    DelayedHealRequiresExtraTurns,
    ExcludedResourceConsumed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
enum PotionSpendAdjudicationV1 {
    NoPotionBaseline,
    NoAdditionalPotionConsumed,
    UnknownWithoutNoPotionWitness {
        baseline_frontier_exhausted: bool,
    },
    RejectDominated {
        dominated_by: Vec<String>,
    },
    RejectNonPositiveHpGain {
        final_hp_delta: i32,
    },
    SpendToCrossSurvivalReserve {
        final_hp_delta: i32,
    },
    CompareContinuationValue {
        immediate_hp_gain: i32,
        break_even_retained_value_hp: i32,
        final_turn_delta: i64,
        potion_expenditures: usize,
    },
    ExcludedFromVictorySpend,
}

#[derive(Clone, Debug, Serialize)]
struct PotionMarginalComparisonV1 {
    final_hp_delta: Option<i32>,
    final_turn_delta: Option<i64>,
    action_count_delta: Option<i64>,
    assessment: PotionMarginalAssessmentV1,
}

#[derive(Clone, Debug, Serialize)]
struct PotionAuditWitnessV1 {
    final_hp: i32,
    hp_loss: i32,
    final_player_turn: u32,
    turns_elapsed: u32,
    action_count: usize,
    explicit_potion_action_count: usize,
    potion_expenditures: Vec<PotionExpenditureEventV1>,
    verified_win_potion_disposition: VerifiedWinPotionDispositionV1,
    policy_review_flags: Vec<PotionPolicyReviewFlagV1>,
    lane_compliant: bool,
    meets_survival_reserve: Option<bool>,
    relative_to_no_potion: Option<PotionMarginalComparisonV1>,
    pareto_frontier: bool,
    dominated_by: Vec<String>,
    shadow_spend_adjudication: Option<PotionSpendAdjudicationV1>,
}

#[derive(Clone, Debug, Serialize)]
struct PotionAuditLaneResultV1 {
    lane_id: String,
    allowed_slot_mask: u64,
    allowed_potions: Vec<PotionResourceV1>,
    max_explicit_expenditures: u32,
    status: String,
    elapsed_ms: u64,
    counters: PotionAuditLaneCountersV1,
    witness: Option<PotionAuditWitnessV1>,
}

#[derive(Clone, Debug, Serialize)]
struct PotionAuditLimitationsV1 {
    lane_absence_is_budget_unknown_unless_frontier_exhausted: bool,
    continuation_value_not_in_combat_case: Vec<&'static str>,
    passive_consumption_handling: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionCurrentCombatStakeV1 {
    Normal,
    Elite,
    Boss,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PotionInventoryPressureV1 {
    slot_capacity: usize,
    occupied_slots: usize,
    empty_slots: usize,
    inventory_full: bool,
    new_potion_would_require_replacement_if_obtainable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct PotionRelicContextV1 {
    sacred_bark: bool,
    toy_ornithopter: bool,
    white_beast_statue: bool,
    sozu: bool,
    potion_belt: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PotionContinuationUnknownV1 {
    NextEncounterIdentity,
    RouteBeforeNextEliteOrBoss,
    FuturePotionDropRollAndIdentity,
    FuturePotionReplacementCandidate,
    FutureHandAndDrawOrder,
    FutureRestSiteAvailability,
}

#[derive(Clone, Debug, Serialize)]
struct PotionContinuationContextV1 {
    act: u8,
    floor: i32,
    current_combat_stake: PotionCurrentCombatStakeV1,
    current_hp: i32,
    max_hp: i32,
    deck_size: usize,
    inventory: PotionInventoryPressureV1,
    relics: PotionRelicContextV1,
    deck_strategic_deficit: DeckStrategicDeficit,
    unavailable_future_context: Vec<PotionContinuationUnknownV1>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct CombatCasePotionExpenditureAuditV4 {
    schema_name: &'static str,
    case: PathBuf,
    root_exact_state_hash: String,
    initial_hp: i32,
    initial_player_turn: u32,
    root_potions: Vec<PotionResourceV1>,
    continuation_context: PotionContinuationContextV1,
    settings: PotionAuditSearchSettingsV1,
    lanes: Vec<PotionAuditLaneResultV1>,
    pareto_lane_ids: Vec<String>,
    limitations: PotionAuditLimitationsV1,
}

pub(super) fn run(
    args: CombatCasePotionExpenditureAuditArgs,
) -> Result<CombatCasePotionExpenditureAuditV4, String> {
    let CombatCasePotionExpenditureAuditArgs {
        case,
        max_combination_size,
        max_lanes,
        survival_reserve_hp,
        max_nodes,
        max_selections,
        wall_ms_per_lane,
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        generation_quantum_work,
        max_turn_depth,
        expect_no_potion_min_final_hp,
        expect_no_potion_dominates_consuming,
    } = args;
    if max_lanes == 0 {
        return Err("potion audit max-lanes must be positive".to_owned());
    }
    if wall_ms_per_lane == 0 {
        return Err("potion audit wall-ms-per-lane must be positive".to_owned());
    }

    let loaded = load_combat_case(&case)?;
    let root = CombatDecisionRoot::new(loaded.position.clone())
        .map_err(|error| format!("invalid potion audit combat root: {error:?}"))?;
    let root_exact_state_hash = root.exact_state_hash().to_owned();
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let initial_player_turn = loaded.position.combat.turn.turn_count;
    let root_potions = root_potion_resources(&loaded.position)?;
    let continuation_context =
        potion_continuation_context(loaded.run.act, loaded.run.floor, &loaded.position);
    let lane_specs = build_lane_specs(&root_potions, max_combination_size, max_lanes)?;
    let base_policy = existing_combat_knowledge_policy_v1();
    let mut lanes = Vec::with_capacity(lane_specs.len());

    for lane in lane_specs {
        let search_spec = LocalGraphSearchSpec::from_controls(
            max_nodes,
            max_selections,
            wall_ms_per_lane,
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            generation_quantum_work,
            max_turn_depth,
            Some(lane.max_explicit_expenditures),
        );
        let mut config =
            search_spec.planner_config(OracleCombatWitnessSatisfaction::BudgetOrExhaustion);
        config.generator.allowed_potion_slots = Some(lane.allowed_slot_mask);
        let lane_root = CombatDecisionRoot::new(loaded.position.clone())
            .map_err(|error| format!("invalid potion audit lane root: {error:?}"))?;
        if lane_root.exact_state_hash() != root_exact_state_hash {
            return Err(format!(
                "potion audit lane '{}' did not preserve the exact root",
                lane.lane_id
            ));
        }
        let mut session = sts_combat_planner::LocalTurnGraphWitnessSession::with_policy(
            lane_root,
            config,
            base_policy.clone(),
        );
        let started = Instant::now();
        let report = session.advance(search_spec.quantum(), &EngineCombatStepper);
        let elapsed_ms = duration_millis_u64(started.elapsed());
        let witness = report
            .witness
            .as_ref()
            .map(|witness| {
                summarize_witness(
                    &loaded.position,
                    witness.actions.as_slice(),
                    &witness.final_position,
                    initial_hp,
                    initial_player_turn,
                    lane.allowed_slot_mask,
                    lane.max_explicit_expenditures,
                    survival_reserve_hp,
                    max_engine_steps_per_transition,
                )
            })
            .transpose()?;
        lanes.push(PotionAuditLaneResultV1 {
            lane_id: lane.lane_id,
            allowed_slot_mask: lane.allowed_slot_mask,
            allowed_potions: lane.allowed_potions,
            max_explicit_expenditures: lane.max_explicit_expenditures,
            status: status_label(&report.status),
            elapsed_ms,
            counters: PotionAuditLaneCountersV1 {
                selections: report.counters.selections,
                generation_work: report.counters.generation_work,
                engine_steps: report.counters.engine_steps,
                exact_nodes: report.counters.exact_nodes,
                terminal_win_options: report.counters.terminal_win_options,
                witness_replay_attempts: report.counters.witness_replay_attempts,
                witness_replay_improvements: report.counters.witness_replay_improvements,
            },
            witness,
        });
    }

    annotate_marginal_comparisons(&mut lanes, survival_reserve_hp);
    annotate_pareto_frontier(&mut lanes);
    annotate_shadow_spend_adjudications(&mut lanes);
    annotate_policy_review_flags(&mut lanes);
    validate_expectations(
        &lanes,
        expect_no_potion_min_final_hp,
        expect_no_potion_dominates_consuming,
    )?;
    let pareto_lane_ids = lanes
        .iter()
        .filter_map(|lane| {
            lane.witness
                .as_ref()
                .filter(|witness| witness.pareto_frontier)
                .map(|_| lane.lane_id.clone())
        })
        .collect();

    Ok(CombatCasePotionExpenditureAuditV4 {
        schema_name: SCHEMA_NAME,
        case,
        root_exact_state_hash,
        initial_hp,
        initial_player_turn,
        root_potions,
        continuation_context,
        settings: PotionAuditSearchSettingsV1 {
            max_combination_size,
            max_lanes,
            survival_reserve_hp,
            max_nodes_per_lane: max_nodes,
            max_selections_per_lane: max_selections,
            wall_ms_per_lane,
            max_engine_steps_per_transition,
            uniform_exploration_ppm,
            generation_quantum_work,
            max_turn_depth,
            satisfaction: "budget_or_exhaustion",
        },
        lanes,
        pareto_lane_ids,
        limitations: PotionAuditLimitationsV1 {
            lane_absence_is_budget_unknown_unless_frontier_exhausted: true,
            continuation_value_not_in_combat_case: vec![
                "forced_rest_avoidance",
                "planned_elite_or_boss",
                "future_potion_reward_identity",
                "future_encounter_specific_counterplay",
            ],
            passive_consumption_handling:
                "replay-detected; a disallowed passive expenditure makes the lane non-compliant",
        },
    })
}

fn potion_continuation_context(
    act: u8,
    floor: i32,
    position: &CombatPosition,
) -> PotionContinuationContextV1 {
    let combat = &position.combat;
    let player = &combat.entities.player;
    let deck = &combat.meta.master_deck_snapshot;
    let slot_capacity = combat.entities.potions.len();
    let occupied_slots = combat
        .entities
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count();
    let empty_slots = slot_capacity.saturating_sub(occupied_slots);
    let current_combat_stake = if combat.meta.is_boss_fight {
        PotionCurrentCombatStakeV1::Boss
    } else if combat.meta.is_elite_fight {
        PotionCurrentCombatStakeV1::Elite
    } else {
        PotionCurrentCombatStakeV1::Normal
    };
    let has_relic = |id| player.relics.iter().any(|relic| relic.id == id);
    let strategic_facts = RunStrategicFacts {
        entering_act: act,
        starter_basic_count: deck.iter().filter(|card| is_starter_basic(card.id)).count(),
        curse_count: deck
            .iter()
            .filter(|card| get_card_definition(card.id).card_type == CardType::Curse)
            .count(),
        has_energy_relic: player
            .relics
            .iter()
            .any(|relic| energy_master_delta(relic.id) > 0),
        has_runic_pyramid: has_relic(RelicId::RunicPyramid),
    };

    PotionContinuationContextV1 {
        act,
        floor,
        current_combat_stake,
        current_hp: player.current_hp,
        max_hp: player.max_hp,
        deck_size: deck.len(),
        inventory: PotionInventoryPressureV1 {
            slot_capacity,
            occupied_slots,
            empty_slots,
            inventory_full: empty_slots == 0,
            new_potion_would_require_replacement_if_obtainable: empty_slots == 0,
        },
        relics: PotionRelicContextV1 {
            sacred_bark: has_relic(RelicId::SacredBark),
            toy_ornithopter: has_relic(RelicId::ToyOrnithopter),
            white_beast_statue: has_relic(RelicId::WhiteBeastStatue),
            sozu: has_relic(RelicId::Sozu),
            potion_belt: has_relic(RelicId::PotionBelt),
        },
        deck_strategic_deficit: assess_deck_strategic_deficit(deck, strategic_facts),
        unavailable_future_context: vec![
            PotionContinuationUnknownV1::NextEncounterIdentity,
            PotionContinuationUnknownV1::RouteBeforeNextEliteOrBoss,
            PotionContinuationUnknownV1::FuturePotionDropRollAndIdentity,
            PotionContinuationUnknownV1::FuturePotionReplacementCandidate,
            PotionContinuationUnknownV1::FutureHandAndDrawOrder,
            PotionContinuationUnknownV1::FutureRestSiteAvailability,
        ],
    }
}

fn root_potion_resources(position: &CombatPosition) -> Result<Vec<PotionResourceV1>, String> {
    position
        .combat
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(slot, potion)| potion.as_ref().map(|potion| (slot, potion)))
        .map(|(slot, potion)| {
            if slot >= u64::BITS as usize {
                return Err(format!(
                    "potion slot {slot} exceeds the audit's 64-slot exact mask"
                ));
            }
            Ok(potion_resource(slot, potion))
        })
        .collect()
}

fn potion_resource(slot: usize, potion: &Potion) -> PotionResourceV1 {
    let shared_strategy_traits = potion_acquisition_traits_v1(potion.id)
        .into_iter()
        .map(shared_strategy_trait)
        .collect::<Vec<_>>();
    let shared_strategy_coverage = if shared_strategy_traits.is_empty() {
        PotionSharedStrategyCoverageV1::Unclassified
    } else {
        PotionSharedStrategyCoverageV1::Classified
    };
    PotionResourceV1 {
        slot,
        id: format!("{:?}", potion.id),
        uuid: potion.uuid,
        can_use: potion.can_use,
        can_discard: potion.can_discard,
        verified_win_rescue_tier: potion_rescue_tier(potion.id),
        shared_strategy_traits,
        shared_strategy_coverage,
        mechanical_role: potion_mechanical_role(potion.id),
        continuation_dependencies: potion_continuation_dependencies(potion.id),
    }
}

fn potion_mechanical_role(id: PotionId) -> PotionMechanicalRoleV1 {
    use PotionId as Id;
    use PotionMechanicalRoleV1 as Role;
    match id {
        Id::FirePotion => Role::SingleTargetDamage,
        Id::ExplosivePotion => Role::MultiTargetDamage,
        Id::PoisonPotion => Role::DamageOverTime,
        Id::WeakenPotion => Role::WeakControl,
        Id::FearPotion => Role::VulnerableControl,
        Id::BlockPotion => Role::ImmediateBlock,
        Id::BloodPotion => Role::ImmediateHealing,
        Id::EnergyPotion => Role::EnergyBurst,
        Id::StrengthPotion => Role::PersistentStrength,
        Id::DexterityPotion => Role::PersistentDexterity,
        Id::SpeedPotion => Role::TemporaryDexterity,
        Id::SteroidPotion => Role::TemporaryStrength,
        Id::SwiftPotion => Role::CardDraw,
        Id::FocusPotion => Role::PersistentFocus,
        Id::AttackPotion => Role::RandomAttackDiscovery,
        Id::SkillPotion => Role::RandomSkillDiscovery,
        Id::PowerPotion => Role::RandomPowerDiscovery,
        Id::ColorlessPotion => Role::RandomColorlessDiscovery,
        Id::BottledMiracle => Role::MiracleGeneration,
        Id::BlessingOfTheForge => Role::TemporaryUpgrade,
        Id::AncientPotion => Role::Artifact,
        Id::RegenPotion => Role::DelayedHealing,
        Id::EssenceOfSteel => Role::PlatedArmor,
        Id::LiquidBronze => Role::Thorns,
        Id::DistilledChaosPotion => Role::RandomTopdeckPlay,
        Id::DuplicationPotion => Role::NextCardDuplication,
        Id::CunningPotion => Role::ShivGeneration,
        Id::PotionOfCapacity => Role::OrbCapacity,
        Id::LiquidMemories => Role::DiscardRecovery,
        Id::GamblersBrew => Role::HandRedraw,
        Id::Elixir => Role::HandExhaust,
        Id::StancePotion => Role::StanceControl,
        Id::FairyPotion => Role::DeathInsurance,
        Id::SmokeBomb => Role::Escape,
        Id::FruitJuice => Role::MaxHpGain,
        Id::EntropicBrew => Role::PotionGeneration,
        Id::SneckoOil => Role::CardDrawAndCostRandomization,
        Id::GhostInAJar => Role::Intangible,
        Id::HeartOfIron => Role::Metallicize,
        Id::CultistPotion => Role::RitualScaling,
        Id::Ambrosia => Role::Divinity,
        Id::EssenceOfDarkness => Role::DarkOrbGeneration,
    }
}

fn potion_continuation_dependencies(id: PotionId) -> Vec<PotionContinuationDependencyV1> {
    use PotionContinuationDependencyV1 as Dependency;
    use PotionId as Id;
    match id {
        Id::FirePotion | Id::ExplosivePotion => {
            vec![Dependency::FutureEnemyCountAndHealth]
        }
        Id::PoisonPotion => vec![
            Dependency::FutureEnemyCountAndHealth,
            Dependency::FutureFightLength,
        ],
        Id::WeakenPotion => vec![
            Dependency::FutureEncounterDamagePattern,
            Dependency::FutureFightLength,
        ],
        Id::FearPotion => vec![
            Dependency::FutureEnemyCountAndHealth,
            Dependency::FutureFightLength,
        ],
        Id::BlockPotion | Id::GhostInAJar => {
            vec![Dependency::FutureEncounterDamagePattern]
        }
        Id::BloodPotion => vec![Dependency::CurrentHpDeficit, Dependency::OutOfCombatTiming],
        Id::EnergyPotion | Id::SwiftPotion | Id::BottledMiracle => {
            vec![Dependency::FutureHandAndDrawOrder]
        }
        Id::StrengthPotion | Id::DexterityPotion => {
            vec![Dependency::FutureFightLength, Dependency::DeckSynergy]
        }
        Id::SpeedPotion | Id::SteroidPotion => vec![
            Dependency::FutureHandAndDrawOrder,
            Dependency::DeckSynergy,
            Dependency::DebuffTiming,
        ],
        Id::FocusPotion => vec![
            Dependency::FutureFightLength,
            Dependency::DeckSynergy,
            Dependency::OrbPlan,
        ],
        Id::AttackPotion | Id::SkillPotion | Id::PowerPotion | Id::ColorlessPotion => vec![
            Dependency::FutureHandAndDrawOrder,
            Dependency::DeckSynergy,
            Dependency::RandomOutcomePool,
        ],
        Id::BlessingOfTheForge => vec![
            Dependency::FutureHandAndDrawOrder,
            Dependency::HighValueCardTarget,
        ],
        Id::AncientPotion => vec![Dependency::DebuffTiming, Dependency::DeckSynergy],
        Id::RegenPotion => vec![Dependency::CurrentHpDeficit, Dependency::FutureFightLength],
        Id::EssenceOfSteel | Id::LiquidBronze | Id::HeartOfIron => vec![
            Dependency::FutureEncounterDamagePattern,
            Dependency::FutureFightLength,
        ],
        Id::DistilledChaosPotion | Id::GamblersBrew | Id::SneckoOil => {
            vec![Dependency::FutureHandAndDrawOrder]
        }
        Id::DuplicationPotion => vec![
            Dependency::FutureHandAndDrawOrder,
            Dependency::DeckSynergy,
            Dependency::HighValueCardTarget,
        ],
        Id::CunningPotion => vec![Dependency::FutureHandAndDrawOrder, Dependency::DeckSynergy],
        Id::PotionOfCapacity | Id::EssenceOfDarkness => {
            vec![Dependency::DeckSynergy, Dependency::OrbPlan]
        }
        Id::LiquidMemories => vec![
            Dependency::FutureHandAndDrawOrder,
            Dependency::FutureDiscardState,
            Dependency::DeckSynergy,
            Dependency::HighValueCardTarget,
        ],
        Id::Elixir => vec![Dependency::FutureHandAndDrawOrder, Dependency::DeckSynergy],
        Id::StancePotion | Id::Ambrosia => {
            vec![Dependency::FutureHandAndDrawOrder, Dependency::StancePlan]
        }
        Id::FairyPotion => vec![Dependency::LowHpInsuranceNeed],
        Id::SmokeBomb => vec![Dependency::RouteEscapeValue],
        Id::FruitJuice => vec![Dependency::OutOfCombatTiming],
        Id::EntropicBrew => vec![Dependency::EmptyPotionSlotsAndAcquisitionRules],
        Id::CultistPotion => vec![Dependency::FutureFightLength],
    }
}

fn shared_strategy_trait(trait_: PotionAcquisitionTraitV1) -> PotionSharedStrategyTraitV1 {
    match trait_ {
        PotionAcquisitionTraitV1::CombatDamage => PotionSharedStrategyTraitV1::CombatDamage,
        PotionAcquisitionTraitV1::AoeDamage => PotionSharedStrategyTraitV1::AoeDamage,
        PotionAcquisitionTraitV1::CombatBlock => PotionSharedStrategyTraitV1::CombatBlock,
        PotionAcquisitionTraitV1::VulnerableSetup => PotionSharedStrategyTraitV1::VulnerableSetup,
        PotionAcquisitionTraitV1::WeakControl => PotionSharedStrategyTraitV1::WeakControl,
        PotionAcquisitionTraitV1::EnergyBurst => PotionSharedStrategyTraitV1::EnergyBurst,
        PotionAcquisitionTraitV1::StrengthGain => PotionSharedStrategyTraitV1::StrengthGain,
        PotionAcquisitionTraitV1::CardAccess => PotionSharedStrategyTraitV1::CardAccess,
        PotionAcquisitionTraitV1::ActionAmplifier => PotionSharedStrategyTraitV1::ActionAmplifier,
        PotionAcquisitionTraitV1::DeathInsurance => PotionSharedStrategyTraitV1::DeathInsurance,
        PotionAcquisitionTraitV1::DebuffControl => PotionSharedStrategyTraitV1::DebuffControl,
        PotionAcquisitionTraitV1::EscapeTool => PotionSharedStrategyTraitV1::EscapeTool,
    }
}

fn potion_rescue_tier(id: PotionId) -> PotionVerifiedWinRescueTierV1 {
    match oracle_potion_rescue_tier_v1(id) {
        OraclePotionRescueTierV1::BoundedQuality => PotionVerifiedWinRescueTierV1::BoundedQuality,
        OraclePotionRescueTierV1::FindAnyWin => PotionVerifiedWinRescueTierV1::FindAnyWin,
        OraclePotionRescueTierV1::Excluded => PotionVerifiedWinRescueTierV1::Excluded,
    }
}

fn build_lane_specs(
    resources: &[PotionResourceV1],
    max_combination_size: usize,
    max_lanes: usize,
) -> Result<Vec<PotionAuditLaneSpec>, String> {
    let mut lanes = vec![PotionAuditLaneSpec {
        lane_id: "no_potion".to_owned(),
        allowed_slot_mask: 0,
        allowed_potions: Vec::new(),
        max_explicit_expenditures: 0,
    }];
    let largest = max_combination_size.min(resources.len());
    for size in 1..=largest {
        let mut subsets = Vec::new();
        collect_resource_subsets(resources, size, 0, &mut Vec::new(), &mut subsets);
        for subset in subsets {
            let allowed_slot_mask = subset
                .iter()
                .fold(0_u64, |mask, resource| mask | (1_u64 << resource.slot));
            let slots = subset
                .iter()
                .map(|resource| resource.slot.to_string())
                .collect::<Vec<_>>()
                .join("_");
            let ids = subset
                .iter()
                .map(|resource| snake_case_debug_name(&resource.id))
                .collect::<Vec<_>>()
                .join("_");
            lanes.push(PotionAuditLaneSpec {
                lane_id: format!("slots_{slots}_{ids}"),
                allowed_slot_mask,
                max_explicit_expenditures: subset.len().try_into().unwrap_or(u32::MAX),
                allowed_potions: subset,
            });
        }
    }
    if lanes.len() > max_lanes {
        return Err(format!(
            "potion audit would create {} lanes, exceeding --max-lanes {max_lanes}",
            lanes.len()
        ));
    }
    Ok(lanes)
}

fn collect_resource_subsets(
    resources: &[PotionResourceV1],
    remaining: usize,
    start: usize,
    current: &mut Vec<PotionResourceV1>,
    output: &mut Vec<Vec<PotionResourceV1>>,
) {
    if remaining == 0 {
        output.push(current.clone());
        return;
    }
    let final_start = resources.len().saturating_sub(remaining);
    for index in start..=final_start {
        current.push(resources[index].clone());
        collect_resource_subsets(resources, remaining - 1, index + 1, current, output);
        current.pop();
    }
}

fn snake_case_debug_name(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 4);
    for (index, character) in value.chars().enumerate() {
        if character.is_ascii_uppercase() && index > 0 {
            output.push('_');
        }
        output.push(character.to_ascii_lowercase());
    }
    output
}

#[allow(clippy::too_many_arguments)]
fn summarize_witness(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    final_position: &CombatPosition,
    initial_hp: i32,
    initial_player_turn: u32,
    allowed_slot_mask: u64,
    max_explicit_expenditures: u32,
    survival_reserve_hp: Option<i32>,
    max_engine_steps_per_transition: usize,
) -> Result<PotionAuditWitnessV1, String> {
    let potion_expenditures =
        replay_potion_expenditures(root, actions, max_engine_steps_per_transition)?;
    let explicit_potion_action_count = potion_expenditures
        .iter()
        .filter(|event| event.mode != PotionExpenditureModeV1::Passive)
        .count();
    let verified_win_potion_disposition = verified_win_potion_disposition(&potion_expenditures);
    let all_slots_allowed = potion_expenditures.iter().all(|event| {
        event.slot < u64::BITS as usize && allowed_slot_mask & (1_u64 << event.slot) != 0
    });
    let lane_compliant =
        all_slots_allowed && potion_expenditures.len() <= max_explicit_expenditures as usize;
    let final_hp = final_position.combat.entities.player.current_hp;
    let final_player_turn = final_position.combat.turn.turn_count;
    Ok(PotionAuditWitnessV1 {
        final_hp,
        hp_loss: initial_hp.saturating_sub(final_hp),
        final_player_turn,
        turns_elapsed: final_player_turn.saturating_sub(initial_player_turn),
        action_count: actions.len(),
        explicit_potion_action_count,
        potion_expenditures,
        verified_win_potion_disposition,
        policy_review_flags: Vec::new(),
        lane_compliant,
        meets_survival_reserve: survival_reserve_hp.map(|reserve| final_hp >= reserve),
        relative_to_no_potion: None,
        pareto_frontier: false,
        dominated_by: Vec::new(),
        shadow_spend_adjudication: None,
    })
}

fn replay_potion_expenditures(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    max_engine_steps_per_transition: usize,
) -> Result<Vec<PotionExpenditureEventV1>, String> {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut events = Vec::new();
    for (action_index, action) in actions.iter().enumerate() {
        let before = position.combat.entities.potions.clone();
        let explicit_slot = match action.input {
            ClientInput::UsePotion { potion_index, .. } => {
                Some((potion_index, PotionExpenditureModeV1::Use))
            }
            ClientInput::DiscardPotion(slot) => Some((slot, PotionExpenditureModeV1::Discard)),
            _ => None,
        };
        let explicit_uuid = if let Some((slot, mode)) = explicit_slot {
            let potion = before.get(slot).and_then(Option::as_ref).ok_or_else(|| {
                format!("verified witness potion action {action_index} refers to empty slot {slot}")
            })?;
            events.push(PotionExpenditureEventV1 {
                action_index,
                slot,
                id: format!("{:?}", potion.id),
                uuid: potion.uuid,
                mode,
                verified_win_rescue_tier: potion_rescue_tier(potion.id),
            });
            Some(potion.uuid)
        } else {
            None
        };
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if result.truncated {
            return Err(format!(
                "potion audit replay exceeded the transition limit at action {action_index}"
            ));
        }
        for (slot, potion) in before
            .iter()
            .enumerate()
            .filter_map(|(slot, potion)| potion.as_ref().map(|potion| (slot, potion)))
        {
            let remains = result
                .position
                .combat
                .entities
                .potions
                .iter()
                .flatten()
                .any(|after| after.uuid == potion.uuid);
            if !remains && explicit_uuid != Some(potion.uuid) {
                events.push(PotionExpenditureEventV1 {
                    action_index,
                    slot,
                    id: format!("{:?}", potion.id),
                    uuid: potion.uuid,
                    mode: PotionExpenditureModeV1::Passive,
                    verified_win_rescue_tier: potion_rescue_tier(potion.id),
                });
            }
        }
        position = result.position;
    }
    Ok(events)
}

fn verified_win_potion_disposition(
    events: &[PotionExpenditureEventV1],
) -> VerifiedWinPotionDispositionV1 {
    if events
        .iter()
        .any(|event| event.verified_win_rescue_tier == PotionVerifiedWinRescueTierV1::Excluded)
    {
        VerifiedWinPotionDispositionV1::ContainsExcludedResource
    } else if events
        .iter()
        .any(|event| event.verified_win_rescue_tier == PotionVerifiedWinRescueTierV1::FindAnyWin)
    {
        VerifiedWinPotionDispositionV1::ContainsReservedResource
    } else if events.is_empty() {
        VerifiedWinPotionDispositionV1::NoPotionSpent
    } else {
        VerifiedWinPotionDispositionV1::BoundedQualityOnly
    }
}

fn annotate_marginal_comparisons(
    lanes: &mut [PotionAuditLaneResultV1],
    survival_reserve_hp: Option<i32>,
) {
    let baseline_lane = lanes.iter().find(|lane| lane.lane_id == "no_potion");
    let baseline_frontier_exhausted =
        baseline_lane.is_some_and(|lane| lane.status == "frontier_exhausted");
    let baseline = baseline_lane
        .and_then(|lane| lane.witness.as_ref())
        .map(|witness| {
            (
                witness.final_hp,
                witness.final_player_turn,
                witness.action_count,
                witness.meets_survival_reserve,
            )
        });
    for lane in lanes {
        let Some(witness) = lane.witness.as_mut() else {
            continue;
        };
        if lane.lane_id == "no_potion" {
            witness.relative_to_no_potion = Some(PotionMarginalComparisonV1 {
                final_hp_delta: Some(0),
                final_turn_delta: Some(0),
                action_count_delta: Some(0),
                assessment: PotionMarginalAssessmentV1::NoPotionBaseline,
            });
            continue;
        }
        let Some((base_hp, base_turn, base_actions, base_meets_reserve)) = baseline else {
            witness.relative_to_no_potion = Some(PotionMarginalComparisonV1 {
                final_hp_delta: None,
                final_turn_delta: None,
                action_count_delta: None,
                assessment: if baseline_frontier_exhausted {
                    PotionMarginalAssessmentV1::NoPotionFrontierExhaustedUnderContract
                } else {
                    PotionMarginalAssessmentV1::NoPotionWitnessNotFoundUnderAllowance
                },
            });
            continue;
        };
        let consumes_potion = !witness.potion_expenditures.is_empty();
        let assessment = if survival_reserve_hp.is_some()
            && base_meets_reserve == Some(false)
            && witness.meets_survival_reserve == Some(true)
        {
            PotionMarginalAssessmentV1::CrossesSurvivalReserve
        } else if !consumes_potion {
            PotionMarginalAssessmentV1::NoAdditionalPotionConsumed
        } else if witness.final_hp > base_hp {
            PotionMarginalAssessmentV1::ImprovesFinalHp
        } else if witness.final_hp == base_hp {
            PotionMarginalAssessmentV1::SameFinalHpWithExtraResource
        } else {
            PotionMarginalAssessmentV1::WorseFinalHpWithExtraResource
        };
        witness.relative_to_no_potion = Some(PotionMarginalComparisonV1 {
            final_hp_delta: Some(witness.final_hp.saturating_sub(base_hp)),
            final_turn_delta: Some(i64::from(witness.final_player_turn) - i64::from(base_turn)),
            action_count_delta: Some(witness.action_count as i64 - base_actions as i64),
            assessment,
        });
    }
}

fn annotate_pareto_frontier(lanes: &mut [PotionAuditLaneResultV1]) {
    let snapshots = lanes
        .iter()
        .filter_map(|lane| {
            lane.witness.as_ref().map(|witness| {
                (
                    lane.lane_id.clone(),
                    witness.final_hp,
                    witness.final_player_turn,
                    witness.action_count,
                    expenditure_identity_set(&witness.potion_expenditures),
                    witness.lane_compliant,
                )
            })
        })
        .collect::<Vec<_>>();
    for lane in lanes {
        let Some(witness) = lane.witness.as_mut() else {
            continue;
        };
        if !witness.lane_compliant {
            continue;
        }
        let target_resources = expenditure_identity_set(&witness.potion_expenditures);
        witness.dominated_by = snapshots
            .iter()
            .filter(|(other_id, ..)| other_id != &lane.lane_id)
            .filter(|(_, _, _, _, _, compliant)| *compliant)
            .filter(
                |(_, other_hp, other_turn, other_actions, other_resources, _)| {
                    dominates(
                        *other_hp,
                        *other_turn,
                        *other_actions,
                        other_resources,
                        witness.final_hp,
                        witness.final_player_turn,
                        witness.action_count,
                        &target_resources,
                    )
                },
            )
            .map(|(other_id, ..)| other_id.clone())
            .collect();
        witness.pareto_frontier = witness.dominated_by.is_empty();
    }
}

fn annotate_shadow_spend_adjudications(lanes: &mut [PotionAuditLaneResultV1]) {
    for lane in lanes {
        let Some(witness) = lane.witness.as_mut() else {
            continue;
        };
        let adjudication = if lane.lane_id == "no_potion" {
            PotionSpendAdjudicationV1::NoPotionBaseline
        } else if witness.potion_expenditures.is_empty() {
            PotionSpendAdjudicationV1::NoAdditionalPotionConsumed
        } else {
            let comparison = witness.relative_to_no_potion.as_ref();
            let baseline_frontier_exhausted = comparison.is_some_and(|comparison| {
                comparison.assessment
                    == PotionMarginalAssessmentV1::NoPotionFrontierExhaustedUnderContract
            });
            let Some((final_hp_delta, final_turn_delta)) = comparison
                .and_then(|comparison| comparison.final_hp_delta.zip(comparison.final_turn_delta))
            else {
                witness.shadow_spend_adjudication =
                    Some(PotionSpendAdjudicationV1::UnknownWithoutNoPotionWitness {
                        baseline_frontier_exhausted,
                    });
                continue;
            };
            if !witness.lane_compliant
                || witness.verified_win_potion_disposition
                    == VerifiedWinPotionDispositionV1::ContainsExcludedResource
            {
                PotionSpendAdjudicationV1::ExcludedFromVictorySpend
            } else if !witness.pareto_frontier {
                PotionSpendAdjudicationV1::RejectDominated {
                    dominated_by: witness.dominated_by.clone(),
                }
            } else if final_hp_delta <= 0 {
                PotionSpendAdjudicationV1::RejectNonPositiveHpGain { final_hp_delta }
            } else if comparison.is_some_and(|comparison| {
                comparison.assessment == PotionMarginalAssessmentV1::CrossesSurvivalReserve
            }) {
                PotionSpendAdjudicationV1::SpendToCrossSurvivalReserve { final_hp_delta }
            } else {
                PotionSpendAdjudicationV1::CompareContinuationValue {
                    immediate_hp_gain: final_hp_delta,
                    break_even_retained_value_hp: final_hp_delta,
                    final_turn_delta,
                    potion_expenditures: witness.potion_expenditures.len(),
                }
            }
        };
        witness.shadow_spend_adjudication = Some(adjudication);
    }
}

fn annotate_policy_review_flags(lanes: &mut [PotionAuditLaneResultV1]) {
    for lane in lanes {
        let Some(witness) = lane.witness.as_mut() else {
            continue;
        };
        let assessment = witness
            .relative_to_no_potion
            .as_ref()
            .map(|comparison| comparison.assessment);
        match witness.verified_win_potion_disposition {
            VerifiedWinPotionDispositionV1::NoPotionSpent => {}
            VerifiedWinPotionDispositionV1::BoundedQualityOnly => {
                if !witness.pareto_frontier {
                    witness
                        .policy_review_flags
                        .push(PotionPolicyReviewFlagV1::AdmittedResourceIsParetoDominated);
                }
                if matches!(
                    assessment,
                    Some(
                        PotionMarginalAssessmentV1::SameFinalHpWithExtraResource
                            | PotionMarginalAssessmentV1::WorseFinalHpWithExtraResource
                    )
                ) {
                    witness
                        .policy_review_flags
                        .push(PotionPolicyReviewFlagV1::AdmittedResourceHasNoHpBenefit);
                }
            }
            VerifiedWinPotionDispositionV1::ContainsReservedResource => {
                if assessment == Some(PotionMarginalAssessmentV1::CrossesSurvivalReserve) {
                    witness
                        .policy_review_flags
                        .push(PotionPolicyReviewFlagV1::ReservedResourceCrossesSurvivalReserve);
                } else if assessment == Some(PotionMarginalAssessmentV1::ImprovesFinalHp) {
                    witness.policy_review_flags.push(
                        PotionPolicyReviewFlagV1::ReservedResourceImprovesHpWithoutCrossingReserve,
                    );
                }
            }
            VerifiedWinPotionDispositionV1::ContainsExcludedResource => {
                witness
                    .policy_review_flags
                    .push(PotionPolicyReviewFlagV1::ExcludedResourceConsumed);
            }
        }
        let delayed_regen = witness
            .potion_expenditures
            .iter()
            .any(|event| event.id == "RegenPotion")
            && witness
                .relative_to_no_potion
                .as_ref()
                .and_then(|comparison| comparison.final_turn_delta)
                .is_some_and(|delta| delta > 0);
        if delayed_regen {
            witness
                .policy_review_flags
                .push(PotionPolicyReviewFlagV1::DelayedHealRequiresExtraTurns);
        }
    }
}

fn expenditure_identity_set(events: &[PotionExpenditureEventV1]) -> BTreeSet<u32> {
    events.iter().map(|event| event.uuid).collect()
}

#[allow(clippy::too_many_arguments)]
fn dominates(
    left_hp: i32,
    left_turn: u32,
    left_actions: usize,
    left_resources: &BTreeSet<u32>,
    right_hp: i32,
    right_turn: u32,
    right_actions: usize,
    right_resources: &BTreeSet<u32>,
) -> bool {
    let no_worse = left_hp >= right_hp
        && left_turn <= right_turn
        && left_actions <= right_actions
        && left_resources.is_subset(right_resources);
    let strictly_better = left_hp > right_hp
        || left_turn < right_turn
        || left_actions < right_actions
        || left_resources != right_resources;
    no_worse && strictly_better
}

fn validate_expectations(
    lanes: &[PotionAuditLaneResultV1],
    expect_no_potion_min_final_hp: Option<i32>,
    expect_no_potion_dominates_consuming: bool,
) -> Result<(), String> {
    let no_potion = lanes
        .iter()
        .find(|lane| lane.lane_id == "no_potion")
        .and_then(|lane| lane.witness.as_ref());
    if let Some(expected) = expect_no_potion_min_final_hp {
        let actual = no_potion
            .filter(|witness| witness.lane_compliant)
            .map(|witness| witness.final_hp)
            .ok_or_else(|| {
                "expected a compliant no-potion witness, but none was found".to_owned()
            })?;
        if actual < expected {
            return Err(format!(
                "no-potion final HP {actual} is below expected minimum {expected}"
            ));
        }
    }
    if expect_no_potion_dominates_consuming {
        let no_potion = no_potion
            .filter(|witness| witness.lane_compliant)
            .ok_or_else(|| {
                "cannot assert dominance without a compliant no-potion witness".to_owned()
            })?;
        let no_potion_resources = expenditure_identity_set(&no_potion.potion_expenditures);
        let consuming = lanes
            .iter()
            .filter_map(|lane| {
                lane.witness
                    .as_ref()
                    .filter(|witness| {
                        witness.lane_compliant && !witness.potion_expenditures.is_empty()
                    })
                    .map(|witness| (lane.lane_id.as_str(), witness))
            })
            .collect::<Vec<_>>();
        if consuming.is_empty() {
            return Err(
                "expected consuming witnesses to compare, but no compliant lane consumed a potion"
                    .to_owned(),
            );
        }
        let failures = consuming
            .into_iter()
            .filter_map(|(lane_id, witness)| {
                (!dominates(
                    no_potion.final_hp,
                    no_potion.final_player_turn,
                    no_potion.action_count,
                    &no_potion_resources,
                    witness.final_hp,
                    witness.final_player_turn,
                    witness.action_count,
                    &expenditure_identity_set(&witness.potion_expenditures),
                ))
                .then_some(lane_id)
            })
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            return Err(format!(
                "no-potion witness does not dominate consuming lanes: {}",
                failures.join(", ")
            ));
        }
    }
    Ok(())
}

fn status_label(status: &LocalTurnGraphWitnessStatus) -> String {
    match status {
        LocalTurnGraphWitnessStatus::WitnessFound => "witness_found".to_owned(),
        LocalTurnGraphWitnessStatus::FrontierExhausted => "frontier_exhausted".to_owned(),
        LocalTurnGraphWitnessStatus::MechanicsGap => "mechanics_gap".to_owned(),
        LocalTurnGraphWitnessStatus::ReplayMismatch(error) => {
            format!("replay_mismatch:{error:?}")
        }
        LocalTurnGraphWitnessStatus::Partial(interruption) => format!(
            "partial:{}",
            match interruption {
                LocalTurnGraphWitnessInterruption::SelectionBudget => "selection_budget",
                LocalTurnGraphWitnessInterruption::GenerationWorkBudget => {
                    "generation_work_budget"
                }
                LocalTurnGraphWitnessInterruption::EngineStepBudget => "engine_step_budget",
                LocalTurnGraphWitnessInterruption::Deadline => "deadline",
            }
        ),
    }
}

fn duration_millis_u64(duration: std::time::Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_combat_planner::{
        LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession,
    };
    use sts_oracle_runtime::ai::strategy::deck_strategic_deficit::StrategicPackageEvidence;
    use sts_oracle_runtime::content::cards::CardId;
    use sts_oracle_runtime::content::monsters::EnemyId;
    use sts_oracle_runtime::content::potions::{Potion, PotionId, ALL_POTIONS};
    use sts_oracle_runtime::content::relics::{RelicId, RelicState};
    use sts_oracle_runtime::runtime::combat::CombatCard;
    use sts_oracle_runtime::state::core::EngineState;

    fn resource(slot: usize, id: PotionId, uuid: u32) -> PotionResourceV1 {
        potion_resource(slot, &Potion::new(id, uuid))
    }

    fn expenditure(uuid: u32) -> PotionExpenditureEventV1 {
        PotionExpenditureEventV1 {
            action_index: 0,
            slot: uuid as usize,
            id: "TestPotion".to_owned(),
            uuid,
            mode: PotionExpenditureModeV1::Use,
            verified_win_rescue_tier: PotionVerifiedWinRescueTierV1::BoundedQuality,
        }
    }

    fn policy_lane(
        lane_id: &str,
        event: PotionExpenditureEventV1,
        disposition: VerifiedWinPotionDispositionV1,
        assessment: PotionMarginalAssessmentV1,
        final_turn_delta: i64,
        pareto_frontier: bool,
    ) -> PotionAuditLaneResultV1 {
        PotionAuditLaneResultV1 {
            lane_id: lane_id.to_owned(),
            allowed_slot_mask: 1,
            allowed_potions: Vec::new(),
            max_explicit_expenditures: 1,
            status: "partial:generation_work_budget".to_owned(),
            elapsed_ms: 0,
            counters: PotionAuditLaneCountersV1 {
                selections: 0,
                generation_work: 0,
                engine_steps: 0,
                exact_nodes: 0,
                terminal_win_options: 0,
                witness_replay_attempts: 0,
                witness_replay_improvements: 0,
            },
            witness: Some(PotionAuditWitnessV1 {
                final_hp: 30,
                hp_loss: 10,
                final_player_turn: 5,
                turns_elapsed: 5,
                action_count: 10,
                explicit_potion_action_count: 1,
                potion_expenditures: vec![event],
                verified_win_potion_disposition: disposition,
                policy_review_flags: Vec::new(),
                lane_compliant: true,
                meets_survival_reserve: Some(true),
                relative_to_no_potion: Some(PotionMarginalComparisonV1 {
                    final_hp_delta: Some(10),
                    final_turn_delta: Some(final_turn_delta),
                    action_count_delta: Some(1),
                    assessment,
                }),
                pareto_frontier,
                dominated_by: Vec::new(),
                shadow_spend_adjudication: None,
            }),
        }
    }

    fn shadow_adjudication(mut lane: PotionAuditLaneResultV1) -> PotionSpendAdjudicationV1 {
        annotate_shadow_spend_adjudications(std::slice::from_mut(&mut lane));
        lane.witness
            .unwrap()
            .shadow_spend_adjudication
            .expect("shadow spend adjudication")
    }

    #[test]
    fn lane_specs_keep_exact_slot_identity_and_bounded_combinations() {
        let resources = vec![
            resource(0, PotionId::BlockPotion, 10),
            resource(2, PotionId::SkillPotion, 20),
        ];
        let lanes = build_lane_specs(&resources, 2, 8).expect("lane specs");

        assert_eq!(lanes.len(), 4);
        assert_eq!(lanes[0].lane_id, "no_potion");
        assert_eq!(lanes[1].allowed_slot_mask, 1);
        assert_eq!(lanes[2].allowed_slot_mask, 1 << 2);
        assert_eq!(lanes[3].allowed_slot_mask, 1 | (1 << 2));
        assert_eq!(
            lanes[3]
                .allowed_potions
                .iter()
                .map(|resource| resource.uuid)
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
    }

    #[test]
    fn potion_resources_expose_shared_strategy_coverage_without_guessing_missing_traits() {
        let strength = resource(0, PotionId::StrengthPotion, 10);
        assert_eq!(
            strength.shared_strategy_traits,
            vec![PotionSharedStrategyTraitV1::StrengthGain]
        );
        assert_eq!(
            strength.shared_strategy_coverage,
            PotionSharedStrategyCoverageV1::Classified
        );

        let regen = resource(1, PotionId::RegenPotion, 20);
        assert!(regen.shared_strategy_traits.is_empty());
        assert_eq!(
            regen.shared_strategy_coverage,
            PotionSharedStrategyCoverageV1::Unclassified
        );
    }

    #[test]
    fn mechanical_roles_and_continuation_dependencies_cover_every_potion_identity() {
        for id in ALL_POTIONS {
            let resource = resource(0, *id, 10);
            assert!(
                !resource.continuation_dependencies.is_empty(),
                "{id:?} needs an explicit continuation dependency"
            );
        }

        let regen = resource(0, PotionId::RegenPotion, 20);
        assert_eq!(
            regen.mechanical_role,
            PotionMechanicalRoleV1::DelayedHealing
        );
        assert!(regen
            .continuation_dependencies
            .contains(&PotionContinuationDependencyV1::FutureFightLength));

        let duplication = resource(0, PotionId::DuplicationPotion, 30);
        assert_eq!(
            duplication.mechanical_role,
            PotionMechanicalRoleV1::NextCardDuplication
        );
        assert!(duplication
            .continuation_dependencies
            .contains(&PotionContinuationDependencyV1::HighValueCardTarget));
    }

    #[test]
    fn continuation_context_keeps_exact_inventory_relic_and_deck_pressure() {
        let mut combat = sts_oracle_runtime::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        combat.meta.master_deck_snapshot = vec![
            CombatCard::new(CardId::HeavyBlade, 10),
            CombatCard::new(CardId::Inflame, 20),
        ]
        .into();
        combat.entities.potions = vec![
            Some(Potion::new(PotionId::DuplicationPotion, 30)),
            Some(Potion::new(PotionId::SkillPotion, 40)),
            Some(Potion::new(PotionId::AttackPotion, 50)),
        ];
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::SacredBark));
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::WhiteBeastStatue));
        combat
            .entities
            .player
            .add_relic(RelicState::new(RelicId::Sozu));
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let context = potion_continuation_context(2, 32, &position);

        assert_eq!(
            context.current_combat_stake,
            PotionCurrentCombatStakeV1::Boss
        );
        assert_eq!(context.deck_size, 2);
        assert_eq!(context.inventory.slot_capacity, 3);
        assert_eq!(context.inventory.occupied_slots, 3);
        assert!(context.inventory.inventory_full);
        assert!(
            context
                .inventory
                .new_potion_would_require_replacement_if_obtainable
        );
        assert!(context.relics.sacred_bark);
        assert!(context.relics.white_beast_statue);
        assert!(context.relics.sozu);
        assert!(context
            .deck_strategic_deficit
            .package_evidence
            .contains(&StrategicPackageEvidence::StrengthScaling));
        assert!(context
            .unavailable_future_context
            .contains(&PotionContinuationUnknownV1::NextEncounterIdentity));
    }

    #[test]
    fn pareto_dominance_requires_a_resource_subset_and_no_worse_combat_axes() {
        let none = BTreeSet::new();
        let skill = BTreeSet::from([20]);
        let weak = BTreeSet::from([30]);

        assert!(dominates(93, 6, 23, &none, 93, 9, 49, &skill));
        assert!(!dominates(92, 6, 23, &none, 93, 9, 49, &skill));
        assert!(!dominates(93, 6, 23, &weak, 90, 9, 49, &skill));
    }

    #[test]
    fn expenditure_identity_uses_uuid_not_only_potion_kind_or_count() {
        let first = expenditure_identity_set(&[expenditure(10)]);
        let second = expenditure_identity_set(&[expenditure(20)]);

        assert!(!first.is_subset(&second));
        assert!(!second.is_subset(&first));
    }

    #[test]
    fn policy_review_flags_expose_reserved_upside_and_admitted_waste() {
        let mut regen = expenditure(10);
        regen.id = "RegenPotion".to_owned();
        regen.verified_win_rescue_tier = PotionVerifiedWinRescueTierV1::FindAnyWin;
        let mut lanes = vec![
            policy_lane(
                "regen",
                regen,
                VerifiedWinPotionDispositionV1::ContainsReservedResource,
                PotionMarginalAssessmentV1::CrossesSurvivalReserve,
                3,
                true,
            ),
            policy_lane(
                "fire",
                expenditure(20),
                VerifiedWinPotionDispositionV1::BoundedQualityOnly,
                PotionMarginalAssessmentV1::SameFinalHpWithExtraResource,
                1,
                false,
            ),
        ];

        annotate_policy_review_flags(&mut lanes);

        let regen_flags = &lanes[0].witness.as_ref().unwrap().policy_review_flags;
        assert!(
            regen_flags.contains(&PotionPolicyReviewFlagV1::ReservedResourceCrossesSurvivalReserve)
        );
        assert!(regen_flags.contains(&PotionPolicyReviewFlagV1::DelayedHealRequiresExtraTurns));
        let fire_flags = &lanes[1].witness.as_ref().unwrap().policy_review_flags;
        assert!(fire_flags.contains(&PotionPolicyReviewFlagV1::AdmittedResourceIsParetoDominated));
        assert!(fire_flags.contains(&PotionPolicyReviewFlagV1::AdmittedResourceHasNoHpBenefit));
    }

    #[test]
    fn shadow_spend_adjudication_preserves_baseline_and_budget_unknowns() {
        let baseline = policy_lane(
            "no_potion",
            expenditure(10),
            VerifiedWinPotionDispositionV1::BoundedQualityOnly,
            PotionMarginalAssessmentV1::ImprovesFinalHp,
            0,
            false,
        );
        assert_eq!(
            shadow_adjudication(baseline),
            PotionSpendAdjudicationV1::NoPotionBaseline
        );

        let mut no_spend = policy_lane(
            "power",
            expenditure(20),
            VerifiedWinPotionDispositionV1::NoPotionSpent,
            PotionMarginalAssessmentV1::NoPotionWitnessNotFoundUnderAllowance,
            0,
            false,
        );
        let no_spend_witness = no_spend.witness.as_mut().unwrap();
        no_spend_witness.potion_expenditures.clear();
        no_spend_witness
            .relative_to_no_potion
            .as_mut()
            .unwrap()
            .final_hp_delta = None;
        assert_eq!(
            shadow_adjudication(no_spend),
            PotionSpendAdjudicationV1::NoAdditionalPotionConsumed
        );

        let mut unknown = policy_lane(
            "fire",
            expenditure(30),
            VerifiedWinPotionDispositionV1::BoundedQualityOnly,
            PotionMarginalAssessmentV1::NoPotionFrontierExhaustedUnderContract,
            0,
            false,
        );
        let unknown_comparison = unknown
            .witness
            .as_mut()
            .unwrap()
            .relative_to_no_potion
            .as_mut()
            .unwrap();
        unknown_comparison.final_hp_delta = None;
        unknown_comparison.final_turn_delta = None;
        assert_eq!(
            shadow_adjudication(unknown),
            PotionSpendAdjudicationV1::UnknownWithoutNoPotionWitness {
                baseline_frontier_exhausted: true,
            }
        );
    }

    #[test]
    fn shadow_spend_adjudication_applies_safety_and_break_even_priority() {
        let mut excluded = policy_lane(
            "smoke",
            expenditure(10),
            VerifiedWinPotionDispositionV1::ContainsExcludedResource,
            PotionMarginalAssessmentV1::WorseFinalHpWithExtraResource,
            4,
            false,
        );
        excluded
            .witness
            .as_mut()
            .unwrap()
            .dominated_by
            .push("no_potion".to_owned());
        assert_eq!(
            shadow_adjudication(excluded),
            PotionSpendAdjudicationV1::ExcludedFromVictorySpend
        );

        let mut non_compliant = policy_lane(
            "passive",
            expenditure(15),
            VerifiedWinPotionDispositionV1::BoundedQualityOnly,
            PotionMarginalAssessmentV1::ImprovesFinalHp,
            0,
            false,
        );
        non_compliant.witness.as_mut().unwrap().lane_compliant = false;
        assert_eq!(
            shadow_adjudication(non_compliant),
            PotionSpendAdjudicationV1::ExcludedFromVictorySpend
        );

        let mut dominated = policy_lane(
            "block",
            expenditure(20),
            VerifiedWinPotionDispositionV1::BoundedQualityOnly,
            PotionMarginalAssessmentV1::WorseFinalHpWithExtraResource,
            1,
            false,
        );
        let dominated_witness = dominated.witness.as_mut().unwrap();
        dominated_witness
            .relative_to_no_potion
            .as_mut()
            .unwrap()
            .final_hp_delta = Some(-1);
        dominated_witness.dominated_by.push("no_potion".to_owned());
        assert_eq!(
            shadow_adjudication(dominated),
            PotionSpendAdjudicationV1::RejectDominated {
                dominated_by: vec!["no_potion".to_owned()],
            }
        );

        let mut no_gain = policy_lane(
            "strength",
            expenditure(30),
            VerifiedWinPotionDispositionV1::BoundedQualityOnly,
            PotionMarginalAssessmentV1::SameFinalHpWithExtraResource,
            -10,
            true,
        );
        no_gain
            .witness
            .as_mut()
            .unwrap()
            .relative_to_no_potion
            .as_mut()
            .unwrap()
            .final_hp_delta = Some(0);
        assert_eq!(
            shadow_adjudication(no_gain),
            PotionSpendAdjudicationV1::RejectNonPositiveHpGain { final_hp_delta: 0 }
        );

        let crosses_reserve = policy_lane(
            "duplication",
            expenditure(40),
            VerifiedWinPotionDispositionV1::ContainsReservedResource,
            PotionMarginalAssessmentV1::CrossesSurvivalReserve,
            2,
            true,
        );
        assert_eq!(
            shadow_adjudication(crosses_reserve),
            PotionSpendAdjudicationV1::SpendToCrossSurvivalReserve { final_hp_delta: 10 }
        );

        let continuation = policy_lane(
            "regen",
            expenditure(50),
            VerifiedWinPotionDispositionV1::ContainsReservedResource,
            PotionMarginalAssessmentV1::ImprovesFinalHp,
            3,
            true,
        );
        assert_eq!(
            shadow_adjudication(continuation),
            PotionSpendAdjudicationV1::CompareContinuationValue {
                immediate_hp_gain: 10,
                break_even_retained_value_hp: 10,
                final_turn_delta: 3,
                potion_expenditures: 1,
            }
        );
    }

    #[test]
    fn isolated_fire_potion_lane_can_rescue_a_proven_no_potion_loss() {
        let mut combat = sts_oracle_runtime::test_support::blank_test_combat();
        combat.entities.player.current_hp = 1;
        combat.entities.monsters = vec![sts_oracle_runtime::test_support::planned_monster(
            EnemyId::JawWorm,
            1,
        )];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 70))];
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let search = |allowed_potion_slots, max_potions_used| {
            let root = CombatDecisionRoot::new(position.clone()).expect("combat root");
            let mut config = LocalTurnGraphWitnessConfig {
                satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
                max_turn_depth: 2,
                max_potions_used: Some(max_potions_used),
                ..LocalTurnGraphWitnessConfig::default()
            };
            config.generator.max_engine_steps_per_transition = 256;
            config.generator.allowed_potion_slots = Some(allowed_potion_slots);
            let mut session = LocalTurnGraphWitnessSession::with_policy(
                root,
                config,
                existing_combat_knowledge_policy_v1(),
            );
            session.advance(
                LocalTurnGraphWitnessQuantum {
                    additional_selections: 10_000,
                    additional_generation_work: 10_000,
                    additional_engine_steps: 2_560_000,
                    deadline: None,
                },
                &EngineCombatStepper,
            )
        };

        let no_potion = search(0, 0);
        assert_eq!(
            no_potion.status,
            LocalTurnGraphWitnessStatus::FrontierExhausted
        );
        assert!(no_potion.witness.is_none());

        let fire = search(1, 1);
        let witness = fire.witness.expect("Fire Potion should rescue the combat");
        let summary = summarize_witness(
            &position,
            &witness.actions,
            &witness.final_position,
            1,
            1,
            1,
            1,
            Some(1),
            256,
        )
        .expect("potion summary");
        assert_eq!(summary.final_hp, 1);
        assert!(summary.lane_compliant);
        assert_eq!(
            summary
                .potion_expenditures
                .iter()
                .map(|event| (event.id.as_str(), event.mode))
                .collect::<Vec<_>>(),
            vec![("FirePotion", PotionExpenditureModeV1::Use)]
        );
    }

    #[test]
    fn replay_marks_disallowed_fairy_revive_as_passive_lane_expenditure() {
        let mut combat = sts_oracle_runtime::test_support::blank_test_combat();
        combat.entities.player.current_hp = 1;
        combat.entities.monsters = vec![sts_oracle_runtime::test_support::planned_monster(
            EnemyId::JawWorm,
            1,
        )];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FairyPotion, 71))];
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let actions = vec![TurnOptionAction {
            input: ClientInput::EndTurn,
            expected_successor_hash: "unused-by-audit-replay".into(),
            engine_steps: 0,
        }];

        let events = replay_potion_expenditures(&position, &actions, 256)
            .expect("Fairy Potion replay attribution");

        assert_eq!(
            events
                .iter()
                .map(|event| (event.id.as_str(), event.mode))
                .collect::<Vec<_>>(),
            vec![("FairyPotion", PotionExpenditureModeV1::Passive)]
        );
    }
}
