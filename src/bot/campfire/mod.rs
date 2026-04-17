use crate::bot::deck_ops::{self, DeckOperationKind, DeckOpsAssessment};
use crate::bot::shared::{analyze_run_needs, RunNeedSnapshot};
use crate::content::relics::RelicId;
use crate::state::core::CampfireChoice;
use crate::state::run::RunState;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct CampfireOptionScore {
    pub action: String,
    pub score: i32,
    pub benefit_score: i32,
    pub penalty_score: i32,
    pub situational_bonus: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct CampfireDecisionDiagnostics {
    pub chosen_action: String,
    pub top_options: Vec<CampfireOptionScore>,
}

#[derive(Clone)]
struct CampfireContext {
    need: RunNeedSnapshot,
    upgrade: DeckOpsAssessment,
    purge: DeckOpsAssessment,
    heal_amount: i32,
    has_fusion_hammer: bool,
    has_shovel: bool,
    has_girya: bool,
    has_peace_pipe: bool,
    should_recall: bool,
}

#[derive(Clone, Copy)]
struct CampfireEvaluation {
    benefit_score: i32,
    penalty_score: i32,
    situational_bonus: i32,
    rationale_key: &'static str,
}

pub fn decide(run_state: &RunState) -> (CampfireChoice, CampfireDecisionDiagnostics) {
    let context = build_context(run_state);
    let mut options = build_options(&context);

    options.sort_by(|lhs, rhs| {
        rhs.score
            .cmp(&lhs.score)
            .then_with(|| lhs.action.cmp(&rhs.action))
    });
    let chosen = options.first().cloned().unwrap_or(CampfireOptionScore {
        action: "rest".to_string(),
        score: 0,
        benefit_score: 0,
        penalty_score: 0,
        situational_bonus: 0,
        rationale_key: "campfire_rest",
    });
    let choice = parse_choice(&chosen.action).unwrap_or(CampfireChoice::Rest);
    (
        choice,
        CampfireDecisionDiagnostics {
            chosen_action: chosen.action,
            top_options: options,
        },
    )
}

fn build_context(run_state: &RunState) -> CampfireContext {
    let has_relic = |relic_id| run_state.relics.iter().any(|relic| relic.id == relic_id);
    CampfireContext {
        need: analyze_run_needs(run_state),
        upgrade: deck_ops::assess(run_state, DeckOperationKind::Upgrade),
        purge: deck_ops::assess(run_state, DeckOperationKind::Remove),
        heal_amount: (run_state.max_hp / 3).max(12),
        has_fusion_hammer: has_relic(RelicId::FusionHammer),
        has_shovel: has_relic(RelicId::Shovel),
        has_girya: has_relic(RelicId::Girya),
        has_peace_pipe: has_relic(RelicId::PeacePipe),
        should_recall: run_state.is_final_act_available
            && !run_state.keys[0]
            && run_state.act_num >= 3,
    }
}

fn build_options(context: &CampfireContext) -> Vec<CampfireOptionScore> {
    let mut options = vec![build_option("rest".to_string(), rest_value(context))];

    if !context.has_fusion_hammer {
        if let Some(idx) = context
            .upgrade
            .best_candidate
            .as_ref()
            .and_then(|candidate| candidate.target_index)
        {
            options.push(build_option(format!("smith:{idx}"), smith_value(context)));
        }
    }

    if context.should_recall {
        options.push(build_option("recall".to_string(), recall_value(context)));
    }
    if context.has_shovel {
        options.push(build_option("dig".to_string(), dig_value(context)));
    }
    if context.has_girya {
        options.push(build_option("lift".to_string(), lift_value(context)));
    }
    if context.has_peace_pipe && context.purge.total_score > 0 {
        let idx = context
            .purge
            .best_candidate
            .as_ref()
            .and_then(|candidate| candidate.target_index)
            .unwrap_or(0);
        options.push(build_option(format!("toke:{idx}"), toke_value(context)));
    }

    options
}

fn build_option(action: String, evaluation: CampfireEvaluation) -> CampfireOptionScore {
    CampfireOptionScore {
        action,
        score: 24 + evaluation.benefit_score + evaluation.situational_bonus
            - evaluation.penalty_score,
        benefit_score: evaluation.benefit_score,
        penalty_score: evaluation.penalty_score,
        situational_bonus: evaluation.situational_bonus,
        rationale_key: evaluation.rationale_key,
    }
}

fn rest_value(context: &CampfireContext) -> CampfireEvaluation {
    let survival_bonus = context.need.survival_pressure / 3;
    let hp_floor_bonus = if context.need.hp_ratio < 0.45 {
        28
    } else if context.need.hp_ratio < 0.65 {
        16
    } else {
        4
    };
    let benefit_score = context.heal_amount / 2 + survival_bonus + hp_floor_bonus;
    let penalty_score = if context.need.hp_ratio >= 0.75 {
        18 + context.need.upgrade_pressure / 8
    } else if context.need.hp_ratio >= 0.60 {
        8
    } else {
        0
    };
    CampfireEvaluation {
        benefit_score,
        penalty_score,
        situational_bonus: 0,
        rationale_key: "campfire_rest",
    }
}

fn smith_value(context: &CampfireContext) -> CampfireEvaluation {
    let benefit_score = context.upgrade.total_score.max(0) + context.need.upgrade_pressure / 3;
    let penalty_score = if context.need.hp_ratio < 0.45 {
        18 + context.need.survival_pressure / 8
    } else if context.need.hp_ratio < 0.60 {
        8
    } else {
        0
    };
    CampfireEvaluation {
        benefit_score,
        penalty_score,
        situational_bonus: 6,
        rationale_key: "campfire_smith",
    }
}

fn recall_value(context: &CampfireContext) -> CampfireEvaluation {
    let urgency_bonus = if context.need.missing_keys > 0 { 20 } else { 0 };
    let penalty_score = if context.need.hp_ratio < 0.40 {
        20 + context.need.survival_pressure / 10
    } else if context.need.hp_ratio < 0.55 {
        10
    } else {
        0
    };
    CampfireEvaluation {
        benefit_score: 48 + urgency_bonus,
        penalty_score,
        situational_bonus: 10,
        rationale_key: "campfire_recall",
    }
}

fn dig_value(context: &CampfireContext) -> CampfireEvaluation {
    let benefit_score = 30;
    let penalty_score = if context.need.hp_ratio < 0.55 {
        24
    } else if context.need.hp_ratio < 0.70 {
        10
    } else {
        0
    };
    CampfireEvaluation {
        benefit_score,
        penalty_score,
        situational_bonus: 0,
        rationale_key: "campfire_dig",
    }
}

fn lift_value(context: &CampfireContext) -> CampfireEvaluation {
    let benefit_score = 22;
    let penalty_score = if context.need.hp_ratio < 0.60 {
        26
    } else if context.need.hp_ratio < 0.75 {
        10
    } else {
        0
    };
    CampfireEvaluation {
        benefit_score,
        penalty_score,
        situational_bonus: if context.need.damage_gap > 0 { 8 } else { 0 },
        rationale_key: "campfire_lift",
    }
}

fn toke_value(context: &CampfireContext) -> CampfireEvaluation {
    let benefit_score = context.purge.total_score.max(0) + context.need.purge_pressure / 5;
    let penalty_score = if context.need.hp_ratio < 0.50 { 14 } else { 0 };
    CampfireEvaluation {
        benefit_score,
        penalty_score,
        situational_bonus: 4,
        rationale_key: "campfire_toke",
    }
}

fn parse_choice(action: &str) -> Option<CampfireChoice> {
    if action == "rest" {
        return Some(CampfireChoice::Rest);
    }
    if action == "recall" {
        return Some(CampfireChoice::Recall);
    }
    if action == "dig" {
        return Some(CampfireChoice::Dig);
    }
    if action == "lift" {
        return Some(CampfireChoice::Lift);
    }
    if let Some(idx) = action
        .strip_prefix("smith:")
        .and_then(|rest| rest.parse::<usize>().ok())
    {
        return Some(CampfireChoice::Smith(idx));
    }
    action
        .strip_prefix("toke:")
        .and_then(|rest| rest.parse::<usize>().ok())
        .map(CampfireChoice::Toke)
}
