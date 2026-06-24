use crate::content::monsters::factory::EncounterId;
use crate::content::relics::RelicId;
use crate::rewards::state::RewardItem;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventEffect, EventId, EventOptionTransition, EventState,
};
use crate::state::run::RunState;

use super::cost::{project_hp_loss_cost_v1, EventCostProjectionV1};
use super::types::EventPolicyConfigV1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventInformationModeV1 {
    PublicOnly,
    CounterfactualOracle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPlanIdV1 {
    LeaveNow,
    CursedTomeReadThenStop,
    CursedTomeReadAndTakeBook,
    ScrapOozeReachInOnce,
    DeadAdventurerLeaveNow,
    DeadAdventurerSearchAsOptionalElite,
    KnowingSkullLeaveNow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventPlanCandidateV1 {
    pub plan_id: EventPlanIdV1,
    pub event_id: EventId,
    pub steps: Vec<EventPlanStepV1>,
    pub cost: EventCostProjectionV1,
    pub reward: EventPlanRewardV1,
    pub risk_model: EventPlanRiskModelV1,
    pub oracle_evidence: Option<EventOracleEvidenceV1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventPlanStepV1 {
    pub screen: usize,
    pub choice_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventPlanRewardV1 {
    None,
    RandomBookRelic { observed: Option<RelicId> },
    RandomRelic,
    DeadAdventurerSearch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventPlanRiskModelV1 {
    None,
    HpPayment,
    RepeatedGamble {
        current_success_chance_percent: i32,
        current_effective_hp_loss: i32,
        next_effective_hp_loss: i32,
        treat_as_optional_elite: bool,
        worst_case_warning_hp_loss: i32,
    },
    OptionalEliteLike {
        fight_chance_percent: i32,
        encounter: Option<EventEncounterProjectionV1>,
        reward_already_obtained: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EventEncounterProjectionV1 {
    pub encounter_id: EncounterId,
    pub publicly_revealed: bool,
    pub starts_awake: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOracleEvidenceV1 {
    pub event_id: EventId,
    pub observed_relic: Option<RelicId>,
    pub outcome: EventOracleOutcomeV1,
    pub committed: bool,
    pub misc_rng_delta_if_committed: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventOracleOutcomeV1 {
    CursedTomeBook {
        observed_relic: Option<RelicId>,
    },
    ScrapOoze {
        attempts_until_success: Option<usize>,
        failed_attempts_before_stop: usize,
        effective_hp_loss_if_committed: i32,
        observed_relic: Option<RelicId>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EventPlanSpecV1 {
    plan_id: EventPlanIdV1,
    event_id: EventId,
    reward: EventPlanRewardV1,
    oracle_evidence: Option<EventOracleEvidenceV1>,
    kind: EventPlanSpecKindV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum EventPlanSpecKindV1 {
    NoCost {
        steps: Vec<EventPlanStepV1>,
        risk_model: EventPlanRiskModelV1,
    },
    HpPayment {
        steps: Vec<EventPlanStepV1>,
        hp_losses: Vec<i32>,
    },
    RepeatedPaidGamble {
        step: EventPlanStepV1,
        current_success_chance_percent: i32,
        current_hp_loss: i32,
        next_hp_loss: i32,
        treat_as_optional_elite: bool,
        worst_case_warning_hp_loss: i32,
    },
    OptionalEliteSearch {
        step: EventPlanStepV1,
        fight_chance_percent: i32,
        encounter: Option<EventEncounterProjectionV1>,
        reward_already_obtained: bool,
    },
}

impl EventPlanSpecV1 {
    fn leave(
        plan_id: EventPlanIdV1,
        event_id: EventId,
        screen: usize,
        choice_index: usize,
    ) -> Self {
        Self {
            plan_id,
            event_id,
            reward: EventPlanRewardV1::None,
            oracle_evidence: None,
            kind: EventPlanSpecKindV1::NoCost {
                steps: vec![event_plan_step(screen, choice_index)],
                risk_model: EventPlanRiskModelV1::None,
            },
        }
    }

    fn hp_payment(
        plan_id: EventPlanIdV1,
        event_id: EventId,
        steps: Vec<EventPlanStepV1>,
        hp_losses: Vec<i32>,
        reward: EventPlanRewardV1,
        oracle_evidence: Option<EventOracleEvidenceV1>,
    ) -> Self {
        Self {
            plan_id,
            event_id,
            reward,
            oracle_evidence,
            kind: EventPlanSpecKindV1::HpPayment { steps, hp_losses },
        }
    }

    fn repeated_paid_gamble(
        plan_id: EventPlanIdV1,
        event_id: EventId,
        step: EventPlanStepV1,
        current_success_chance_percent: i32,
        current_hp_loss: i32,
        next_hp_loss: i32,
        reward: EventPlanRewardV1,
        oracle_evidence: Option<EventOracleEvidenceV1>,
    ) -> Self {
        Self {
            plan_id,
            event_id,
            reward,
            oracle_evidence,
            kind: EventPlanSpecKindV1::RepeatedPaidGamble {
                step,
                current_success_chance_percent,
                current_hp_loss,
                next_hp_loss,
                treat_as_optional_elite: true,
                worst_case_warning_hp_loss: 10,
            },
        }
    }

    fn optional_elite_search(
        plan_id: EventPlanIdV1,
        event_id: EventId,
        step: EventPlanStepV1,
        fight_chance_percent: i32,
        encounter: Option<EventEncounterProjectionV1>,
        reward_already_obtained: bool,
    ) -> Self {
        Self {
            plan_id,
            event_id,
            reward: EventPlanRewardV1::DeadAdventurerSearch,
            oracle_evidence: None,
            kind: EventPlanSpecKindV1::OptionalEliteSearch {
                step,
                fight_chance_percent,
                encounter,
                reward_already_obtained,
            },
        }
    }
}

fn event_plan_step(screen: usize, choice_index: usize) -> EventPlanStepV1 {
    EventPlanStepV1 {
        screen,
        choice_index,
    }
}

pub fn compile_event_plan_candidates_v1(
    run_state: &RunState,
    information_mode: EventInformationModeV1,
) -> Vec<EventPlanCandidateV1> {
    let Some(event_state) = &run_state.event_state else {
        return Vec::new();
    };
    let specs = match event_state.id {
        EventId::CursedTome => {
            compile_cursed_tome_plan_specs_v1(run_state, event_state, information_mode)
        }
        EventId::ScrapOoze => {
            compile_scrap_ooze_plan_specs_v1(run_state, event_state, information_mode)
        }
        EventId::DeadAdventurer => compile_dead_adventurer_plan_specs_v1(event_state),
        EventId::KnowingSkull => compile_knowing_skull_plan_specs_v1(run_state, event_state),
        _ => Vec::new(),
    };
    materialize_event_plan_specs_v1(run_state, specs)
}

pub fn select_event_plan_candidate_v1(
    run_state: &RunState,
    information_mode: EventInformationModeV1,
    config: &EventPolicyConfigV1,
) -> Option<EventPlanCandidateV1> {
    let plans = compile_event_plan_candidates_v1(run_state, information_mode);
    let Some(event_state) = &run_state.event_state else {
        return None;
    };
    match event_state.id {
        EventId::CursedTome => select_cursed_tome_plan_v1(run_state, config, plans),
        EventId::ScrapOoze => select_scrap_ooze_plan_v1(run_state, config, plans),
        EventId::DeadAdventurer => select_dead_adventurer_plan_v1(run_state, config, plans),
        EventId::KnowingSkull => select_plan_by_id_v1(plans, EventPlanIdV1::KnowingSkullLeaveNow),
        _ => None,
    }
}

fn materialize_event_plan_specs_v1(
    run_state: &RunState,
    specs: Vec<EventPlanSpecV1>,
) -> Vec<EventPlanCandidateV1> {
    specs
        .into_iter()
        .map(|spec| materialize_event_plan_spec_v1(run_state, spec))
        .collect()
}

fn materialize_event_plan_spec_v1(
    run_state: &RunState,
    spec: EventPlanSpecV1,
) -> EventPlanCandidateV1 {
    let (steps, cost, risk_model) = match spec.kind {
        EventPlanSpecKindV1::NoCost { steps, risk_model } => {
            (steps, project_hp_loss_cost_v1(run_state, &[]), risk_model)
        }
        EventPlanSpecKindV1::HpPayment { steps, hp_losses } => (
            steps,
            project_hp_loss_cost_v1(run_state, &hp_losses),
            EventPlanRiskModelV1::HpPayment,
        ),
        EventPlanSpecKindV1::RepeatedPaidGamble {
            step,
            current_success_chance_percent,
            current_hp_loss,
            next_hp_loss,
            treat_as_optional_elite,
            worst_case_warning_hp_loss,
        } => {
            let current_cost = project_hp_loss_cost_v1(run_state, &[current_hp_loss]);
            let next_cost = project_hp_loss_cost_v1(run_state, &[next_hp_loss]);
            (
                vec![step],
                current_cost.clone(),
                EventPlanRiskModelV1::RepeatedGamble {
                    current_success_chance_percent,
                    current_effective_hp_loss: current_cost.effective_hp_loss,
                    next_effective_hp_loss: next_cost.effective_hp_loss,
                    treat_as_optional_elite,
                    worst_case_warning_hp_loss,
                },
            )
        }
        EventPlanSpecKindV1::OptionalEliteSearch {
            step,
            fight_chance_percent,
            encounter,
            reward_already_obtained,
        } => (
            vec![step],
            project_hp_loss_cost_v1(run_state, &[]),
            EventPlanRiskModelV1::OptionalEliteLike {
                fight_chance_percent,
                encounter,
                reward_already_obtained,
            },
        ),
    };

    EventPlanCandidateV1 {
        plan_id: spec.plan_id,
        event_id: spec.event_id,
        steps,
        cost,
        reward: spec.reward,
        risk_model,
        oracle_evidence: spec.oracle_evidence,
    }
}

fn compile_cursed_tome_plan_specs_v1(
    run_state: &RunState,
    event_state: &EventState,
    information_mode: EventInformationModeV1,
) -> Vec<EventPlanSpecV1> {
    let mut plans = Vec::new();
    if event_state.current_screen == 0 {
        plans.push(EventPlanSpecV1::leave(
            EventPlanIdV1::LeaveNow,
            EventId::CursedTome,
            0,
            1,
        ));
    }

    if event_state.current_screen <= 4 {
        let prefix_steps = cursed_tome_continue_steps_from(event_state.current_screen);
        let prefix_losses = cursed_tome_continue_losses_from(event_state.current_screen);

        let mut stop_steps = prefix_steps.clone();
        stop_steps.push(event_plan_step(4, 1));
        let mut stop_losses = prefix_losses.clone();
        stop_losses.push(3);
        plans.push(EventPlanSpecV1::hp_payment(
            EventPlanIdV1::CursedTomeReadThenStop,
            EventId::CursedTome,
            stop_steps,
            stop_losses,
            EventPlanRewardV1::None,
            None,
        ));

        let final_damage = cursed_tome_final_damage(run_state);
        let oracle_evidence = match information_mode {
            EventInformationModeV1::PublicOnly => None,
            EventInformationModeV1::CounterfactualOracle => peek_cursed_tome_book_v1(run_state),
        };
        let mut take_steps = prefix_steps;
        take_steps.push(event_plan_step(4, 0));
        let mut take_losses = prefix_losses;
        take_losses.push(final_damage);
        let observed = oracle_evidence
            .as_ref()
            .and_then(|evidence| evidence.observed_relic);
        plans.push(EventPlanSpecV1::hp_payment(
            EventPlanIdV1::CursedTomeReadAndTakeBook,
            EventId::CursedTome,
            take_steps,
            take_losses,
            EventPlanRewardV1::RandomBookRelic { observed },
            oracle_evidence,
        ));
    }

    plans
}

fn compile_scrap_ooze_plan_specs_v1(
    run_state: &RunState,
    event_state: &EventState,
    information_mode: EventInformationModeV1,
) -> Vec<EventPlanSpecV1> {
    if event_state.current_screen != 0 {
        return Vec::new();
    }

    let (chance, damage) = scrap_ooze_chance_and_damage(run_state, event_state);
    let oracle_evidence = match information_mode {
        EventInformationModeV1::PublicOnly => None,
        EventInformationModeV1::CounterfactualOracle => peek_scrap_ooze_v1(run_state),
    };

    vec![
        EventPlanSpecV1::leave(EventPlanIdV1::LeaveNow, EventId::ScrapOoze, 0, 1),
        EventPlanSpecV1::repeated_paid_gamble(
            EventPlanIdV1::ScrapOozeReachInOnce,
            EventId::ScrapOoze,
            event_plan_step(0, 0),
            chance,
            damage,
            damage + 1,
            EventPlanRewardV1::RandomRelic,
            oracle_evidence,
        ),
    ]
}

fn compile_dead_adventurer_plan_specs_v1(event_state: &EventState) -> Vec<EventPlanSpecV1> {
    if event_state.current_screen != 0 {
        return Vec::new();
    }

    let num_rewards = dead_adventurer_num_rewards(event_state.internal_state);
    let fight_chance = dead_adventurer_encounter_chance(event_state.internal_state);
    let reward_already_obtained = (0..num_rewards)
        .any(|idx| dead_adventurer_reward_type(event_state.internal_state, idx) == 2);
    let encounter = dead_adventurer_encounter_id(event_state.internal_state).map(|encounter_id| {
        EventEncounterProjectionV1 {
            encounter_id,
            publicly_revealed: true,
            starts_awake: encounter_id == EncounterId::LagavulinEvent,
        }
    });

    vec![
        EventPlanSpecV1::leave(
            EventPlanIdV1::DeadAdventurerLeaveNow,
            EventId::DeadAdventurer,
            0,
            1,
        ),
        EventPlanSpecV1::optional_elite_search(
            EventPlanIdV1::DeadAdventurerSearchAsOptionalElite,
            EventId::DeadAdventurer,
            event_plan_step(0, 0),
            fight_chance,
            encounter,
            reward_already_obtained,
        ),
    ]
}

fn compile_knowing_skull_plan_specs_v1(
    run_state: &RunState,
    event_state: &EventState,
) -> Vec<EventPlanSpecV1> {
    let Some((choice_index, hp_loss)) = knowing_skull_leave_choice_v1(run_state, event_state)
    else {
        return Vec::new();
    };
    vec![EventPlanSpecV1::hp_payment(
        EventPlanIdV1::KnowingSkullLeaveNow,
        EventId::KnowingSkull,
        vec![event_plan_step(event_state.current_screen, choice_index)],
        vec![hp_loss],
        EventPlanRewardV1::None,
        None,
    )]
}

fn select_cursed_tome_plan_v1(
    run_state: &RunState,
    config: &EventPolicyConfigV1,
    plans: Vec<EventPlanCandidateV1>,
) -> Option<EventPlanCandidateV1> {
    let hp_floor = hp_safety_floor(run_state, config);
    let take = plans
        .iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::CursedTomeReadAndTakeBook)
        .cloned();
    if let Some(take) = take {
        if run_state
            .current_hp
            .saturating_sub(take.cost.effective_hp_loss)
            >= hp_floor
        {
            return Some(take);
        }
    }

    plans
        .iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::LeaveNow)
        .cloned()
        .or_else(|| {
            plans
                .into_iter()
                .find(|plan| plan.plan_id == EventPlanIdV1::CursedTomeReadThenStop)
        })
}

fn select_scrap_ooze_plan_v1(
    run_state: &RunState,
    config: &EventPolicyConfigV1,
    plans: Vec<EventPlanCandidateV1>,
) -> Option<EventPlanCandidateV1> {
    let hp_floor = hp_safety_floor(run_state, config);
    let reach = plans
        .iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::ScrapOozeReachInOnce)
        .cloned();
    if let Some(reach) = reach {
        let warning_hp_loss = match &reach.risk_model {
            EventPlanRiskModelV1::RepeatedGamble {
                worst_case_warning_hp_loss,
                ..
            } => *worst_case_warning_hp_loss,
            _ => reach.cost.effective_hp_loss,
        };
        if run_state.current_hp.saturating_sub(warning_hp_loss) >= hp_floor {
            return Some(reach);
        }
    }

    plans
        .into_iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::LeaveNow)
}

fn select_dead_adventurer_plan_v1(
    run_state: &RunState,
    config: &EventPolicyConfigV1,
    plans: Vec<EventPlanCandidateV1>,
) -> Option<EventPlanCandidateV1> {
    let leave = plans
        .iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::DeadAdventurerLeaveNow)
        .cloned();
    let search = plans
        .iter()
        .find(|plan| plan.plan_id == EventPlanIdV1::DeadAdventurerSearchAsOptionalElite)
        .cloned();
    let Some(search) = search else {
        return leave;
    };

    if let EventPlanRiskModelV1::OptionalEliteLike {
        reward_already_obtained,
        ..
    } = &search.risk_model
    {
        if *reward_already_obtained {
            return leave.or(Some(search));
        }
    }

    let hp_floor = hp_safety_floor(run_state, config);
    if run_state.current_hp.saturating_sub(20) >= hp_floor {
        Some(search)
    } else {
        leave.or(Some(search))
    }
}

fn select_plan_by_id_v1(
    plans: Vec<EventPlanCandidateV1>,
    plan_id: EventPlanIdV1,
) -> Option<EventPlanCandidateV1> {
    plans.into_iter().find(|plan| plan.plan_id == plan_id)
}

fn hp_safety_floor(run_state: &RunState, config: &EventPolicyConfigV1) -> i32 {
    let ratio_floor =
        (run_state.max_hp.max(0) as f32 * config.min_hp_ratio_after_safe_hp_cost).ceil() as i32;
    config.min_hp_after_safe_hp_cost.max(ratio_floor)
}

fn cursed_tome_continue_steps_from(current_screen: usize) -> Vec<EventPlanStepV1> {
    (current_screen..=3)
        .map(|screen| event_plan_step(screen, 0))
        .collect()
}

fn cursed_tome_continue_losses_from(current_screen: usize) -> Vec<i32> {
    (current_screen.max(1)..=3)
        .map(|screen| screen as i32)
        .collect()
}

fn cursed_tome_final_damage(run_state: &RunState) -> i32 {
    if run_state.ascension_level >= 15 {
        15
    } else {
        10
    }
}

fn scrap_ooze_chance_and_damage(run_state: &RunState, event_state: &EventState) -> (i32, i32) {
    if event_state.internal_state == 0 {
        let base_damage = if run_state.ascension_level >= 15 {
            5
        } else {
            3
        };
        (25, base_damage)
    } else {
        let chance = event_state.internal_state & 0xFFFF;
        let damage = (event_state.internal_state >> 16) & 0xFFFF;
        (chance, damage)
    }
}

fn dead_adventurer_num_rewards(state: i32) -> i32 {
    state & 0xF
}

fn dead_adventurer_encounter_chance(state: i32) -> i32 {
    (state >> 4) & 0xFF
}

fn dead_adventurer_reward_type(state: i32, idx: i32) -> i32 {
    (state >> (12 + idx * 2)) & 0x3
}

fn dead_adventurer_encounter_id(state: i32) -> Option<EncounterId> {
    match (state >> 18) & 0x3 {
        0 => Some(EncounterId::ThreeSentries),
        1 => Some(EncounterId::GremlinNob),
        2 => Some(EncounterId::LagavulinEvent),
        _ => None,
    }
}

fn knowing_skull_leave_choice_v1(
    run_state: &RunState,
    event_state: &EventState,
) -> Option<(usize, i32)> {
    if event_state.id != EventId::KnowingSkull || event_state.current_screen != 1 {
        return None;
    }
    crate::content::events::knowing_skull::get_options(run_state, event_state)
        .into_iter()
        .enumerate()
        .find_map(|(index, option)| {
            if option.semantics.action != EventActionKind::Leave
                || option.semantics.transition != EventOptionTransition::AdvanceScreen
            {
                return None;
            }
            let hp_loss = option
                .semantics
                .effects
                .iter()
                .filter_map(|effect| match effect {
                    EventEffect::LoseHp(value) => Some(*value),
                    _ => None,
                })
                .sum();
            Some((index, hp_loss))
        })
}

fn peek_cursed_tome_book_v1(run_state: &RunState) -> Option<EventOracleEvidenceV1> {
    let Some(event_state) = &run_state.event_state else {
        return None;
    };
    if event_state.id != EventId::CursedTome || event_state.current_screen > 4 {
        return None;
    }

    let mut clone = run_state.clone();
    let misc_before = clone.rng_pool.misc_rng.counter;
    let mut engine_state = EngineState::EventRoom;

    while clone.event_state.as_ref()?.current_screen < 4 {
        crate::content::events::cursed_tome::handle_choice(&mut engine_state, &mut clone, 0);
    }
    crate::content::events::cursed_tome::handle_choice(&mut engine_state, &mut clone, 0);

    let observed_relic = match engine_state {
        EngineState::RewardScreen(rewards) => {
            rewards.items.into_iter().find_map(|item| match item {
                RewardItem::Relic { relic_id } => Some(relic_id),
                _ => None,
            })
        }
        _ => None,
    };
    let misc_after = clone.rng_pool.misc_rng.counter;

    Some(EventOracleEvidenceV1 {
        event_id: EventId::CursedTome,
        observed_relic,
        outcome: EventOracleOutcomeV1::CursedTomeBook { observed_relic },
        committed: false,
        misc_rng_delta_if_committed: misc_after.saturating_sub(misc_before),
    })
}

fn peek_scrap_ooze_v1(run_state: &RunState) -> Option<EventOracleEvidenceV1> {
    let Some(event_state) = &run_state.event_state else {
        return None;
    };
    if event_state.id != EventId::ScrapOoze || event_state.current_screen != 0 {
        return None;
    }

    let mut clone = run_state.clone();
    let misc_before = clone.rng_pool.misc_rng.counter;
    let hp_before = clone.current_hp;
    let relic_count_before = clone.relics.len();
    let mut engine_state = EngineState::EventRoom;
    let mut attempts = 0usize;

    while clone
        .event_state
        .as_ref()
        .is_some_and(|state| state.id == EventId::ScrapOoze && state.current_screen == 0)
        && clone.current_hp > 0
        && attempts < 32
    {
        crate::content::events::scrap_ooze::handle_choice(&mut engine_state, &mut clone, 0);
        attempts += 1;
    }

    let success = clone
        .event_state
        .as_ref()
        .is_some_and(|state| state.id == EventId::ScrapOoze && state.current_screen == 1)
        && clone.relics.len() > relic_count_before;
    let observed_relic = if success {
        clone.relics.get(relic_count_before).map(|relic| relic.id)
    } else {
        None
    };
    let misc_after = clone.rng_pool.misc_rng.counter;
    let effective_hp_loss_if_committed = hp_before.saturating_sub(clone.current_hp);

    Some(EventOracleEvidenceV1 {
        event_id: EventId::ScrapOoze,
        observed_relic,
        outcome: EventOracleOutcomeV1::ScrapOoze {
            attempts_until_success: success.then_some(attempts),
            failed_attempts_before_stop: if success {
                attempts.saturating_sub(1)
            } else {
                attempts
            },
            effective_hp_loss_if_committed,
            observed_relic,
        },
        committed: false,
        misc_rng_delta_if_committed: misc_after.saturating_sub(misc_before),
    })
}
