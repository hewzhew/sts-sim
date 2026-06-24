use crate::content::monsters::factory::EncounterId;
use crate::content::relics::RelicId;
use crate::rewards::state::RewardItem;
use crate::state::core::EngineState;
use crate::state::events::{EventId, EventState};
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
        possible_encounter: Option<EncounterId>,
        reward_already_obtained: bool,
        notes: Vec<EventRiskNoteV1>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventRiskNoteV1 {
    TreatSearchAsGuaranteedElite,
    LagavulinStartsAwake,
    LeaveAfterRelicReward,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOracleEvidenceV1 {
    pub event_id: EventId,
    pub observed_relic: Option<RelicId>,
    pub committed: bool,
    pub misc_rng_delta_if_committed: u32,
}

pub fn compile_event_plan_candidates_v1(
    run_state: &RunState,
    information_mode: EventInformationModeV1,
) -> Vec<EventPlanCandidateV1> {
    let Some(event_state) = &run_state.event_state else {
        return Vec::new();
    };
    match event_state.id {
        EventId::CursedTome => {
            compile_cursed_tome_plans_v1(run_state, event_state, information_mode)
        }
        EventId::ScrapOoze => compile_scrap_ooze_plans_v1(run_state, event_state),
        EventId::DeadAdventurer => compile_dead_adventurer_plans_v1(event_state),
        _ => Vec::new(),
    }
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
        _ => None,
    }
}

fn compile_cursed_tome_plans_v1(
    run_state: &RunState,
    event_state: &EventState,
    information_mode: EventInformationModeV1,
) -> Vec<EventPlanCandidateV1> {
    let mut plans = Vec::new();
    if event_state.current_screen == 0 {
        plans.push(EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::LeaveNow,
            event_id: EventId::CursedTome,
            steps: vec![EventPlanStepV1 {
                screen: 0,
                choice_index: 1,
            }],
            cost: project_hp_loss_cost_v1(run_state, &[]),
            reward: EventPlanRewardV1::None,
            risk_model: EventPlanRiskModelV1::None,
            oracle_evidence: None,
        });
    }

    if event_state.current_screen <= 4 {
        let prefix_steps = cursed_tome_continue_steps_from(event_state.current_screen);
        let prefix_losses = cursed_tome_continue_losses_from(event_state.current_screen);

        let mut stop_steps = prefix_steps.clone();
        stop_steps.push(EventPlanStepV1 {
            screen: 4,
            choice_index: 1,
        });
        let mut stop_losses = prefix_losses.clone();
        stop_losses.push(3);
        plans.push(EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::CursedTomeReadThenStop,
            event_id: EventId::CursedTome,
            steps: stop_steps,
            cost: project_hp_loss_cost_v1(run_state, &stop_losses),
            reward: EventPlanRewardV1::None,
            risk_model: EventPlanRiskModelV1::HpPayment,
            oracle_evidence: None,
        });

        let final_damage = cursed_tome_final_damage(run_state);
        let oracle_evidence = match information_mode {
            EventInformationModeV1::PublicOnly => None,
            EventInformationModeV1::CounterfactualOracle => peek_cursed_tome_book_v1(run_state),
        };
        let mut take_steps = prefix_steps;
        take_steps.push(EventPlanStepV1 {
            screen: 4,
            choice_index: 0,
        });
        let mut take_losses = prefix_losses;
        take_losses.push(final_damage);
        let observed = oracle_evidence
            .as_ref()
            .and_then(|evidence| evidence.observed_relic);
        plans.push(EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::CursedTomeReadAndTakeBook,
            event_id: EventId::CursedTome,
            steps: take_steps,
            cost: project_hp_loss_cost_v1(run_state, &take_losses),
            reward: EventPlanRewardV1::RandomBookRelic { observed },
            risk_model: EventPlanRiskModelV1::HpPayment,
            oracle_evidence,
        });
    }

    plans
}

fn compile_scrap_ooze_plans_v1(
    run_state: &RunState,
    event_state: &EventState,
) -> Vec<EventPlanCandidateV1> {
    if event_state.current_screen != 0 {
        return Vec::new();
    }

    let (chance, damage) = scrap_ooze_chance_and_damage(run_state, event_state);
    let current_cost = project_hp_loss_cost_v1(run_state, &[damage]);
    let next_cost = project_hp_loss_cost_v1(run_state, &[damage + 1]);

    vec![
        EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::LeaveNow,
            event_id: EventId::ScrapOoze,
            steps: vec![EventPlanStepV1 {
                screen: 0,
                choice_index: 1,
            }],
            cost: project_hp_loss_cost_v1(run_state, &[]),
            reward: EventPlanRewardV1::None,
            risk_model: EventPlanRiskModelV1::None,
            oracle_evidence: None,
        },
        EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::ScrapOozeReachInOnce,
            event_id: EventId::ScrapOoze,
            steps: vec![EventPlanStepV1 {
                screen: 0,
                choice_index: 0,
            }],
            cost: current_cost.clone(),
            reward: EventPlanRewardV1::RandomRelic,
            risk_model: EventPlanRiskModelV1::RepeatedGamble {
                current_success_chance_percent: chance,
                current_effective_hp_loss: current_cost.effective_hp_loss,
                next_effective_hp_loss: next_cost.effective_hp_loss,
                treat_as_optional_elite: true,
                worst_case_warning_hp_loss: 10,
            },
            oracle_evidence: None,
        },
    ]
}

fn compile_dead_adventurer_plans_v1(event_state: &EventState) -> Vec<EventPlanCandidateV1> {
    if event_state.current_screen != 0 {
        return Vec::new();
    }

    let num_rewards = dead_adventurer_num_rewards(event_state.internal_state);
    let fight_chance = dead_adventurer_encounter_chance(event_state.internal_state);
    let reward_already_obtained = (0..num_rewards)
        .any(|idx| dead_adventurer_reward_type(event_state.internal_state, idx) == 2);
    let possible_encounter = dead_adventurer_encounter_id(event_state.internal_state);
    let mut notes = vec![EventRiskNoteV1::TreatSearchAsGuaranteedElite];
    if reward_already_obtained {
        notes.push(EventRiskNoteV1::LeaveAfterRelicReward);
    }
    if possible_encounter == Some(EncounterId::LagavulinEvent) {
        notes.push(EventRiskNoteV1::LagavulinStartsAwake);
    }

    vec![
        EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::DeadAdventurerLeaveNow,
            event_id: EventId::DeadAdventurer,
            steps: vec![EventPlanStepV1 {
                screen: 0,
                choice_index: 1,
            }],
            cost: EventCostProjectionV1 {
                nominal_hp_loss: 0,
                effective_hp_loss: 0,
                modifiers: Vec::new(),
            },
            reward: EventPlanRewardV1::None,
            risk_model: EventPlanRiskModelV1::None,
            oracle_evidence: None,
        },
        EventPlanCandidateV1 {
            plan_id: EventPlanIdV1::DeadAdventurerSearchAsOptionalElite,
            event_id: EventId::DeadAdventurer,
            steps: vec![EventPlanStepV1 {
                screen: 0,
                choice_index: 0,
            }],
            cost: EventCostProjectionV1 {
                nominal_hp_loss: 0,
                effective_hp_loss: 0,
                modifiers: Vec::new(),
            },
            reward: EventPlanRewardV1::DeadAdventurerSearch,
            risk_model: EventPlanRiskModelV1::OptionalEliteLike {
                fight_chance_percent: fight_chance,
                possible_encounter,
                reward_already_obtained,
                notes,
            },
            oracle_evidence: None,
        },
    ]
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

fn hp_safety_floor(run_state: &RunState, config: &EventPolicyConfigV1) -> i32 {
    let ratio_floor =
        (run_state.max_hp.max(0) as f32 * config.min_hp_ratio_after_safe_hp_cost).ceil() as i32;
    config.min_hp_after_safe_hp_cost.max(ratio_floor)
}

fn cursed_tome_continue_steps_from(current_screen: usize) -> Vec<EventPlanStepV1> {
    (current_screen..=3)
        .map(|screen| EventPlanStepV1 {
            screen,
            choice_index: 0,
        })
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
        committed: false,
        misc_rng_delta_if_committed: misc_after.saturating_sub(misc_before),
    })
}
