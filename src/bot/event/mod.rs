use crate::bot::deck_ops::{self, DeckOperationKind, DeckOpsAssessment};
use crate::bot::shared::{analyze_run_needs, score_reward_potion, RunNeedSnapshot};
use crate::engine::event_handler::{
    analyze_live_event_rebuild, get_event_options, LiveEventRebuildResult,
};
use crate::state::events::{
    EventActionKind, EventCardKind, EventEffect, EventOption, EventOptionConstraint,
    EventOptionTransition, EventSelectionKind, EventState,
};
use crate::state::run::RunState;
use serde_json::{json, Value};

#[derive(Clone, Debug, serde::Serialize)]
pub struct EventOptionAssessment {
    pub option_index: usize,
    pub command_index: usize,
    pub label: String,
    pub score: i32,
    pub benefit_score: i32,
    pub penalty_score: i32,
    pub transition_bonus: i32,
    pub situational_bonus: i32,
    pub rationale_key: &'static str,
    pub disabled: bool,
}

#[derive(Clone, Debug)]
pub struct EventDecisionDiagnostics {
    pub chosen_index: usize,
    pub fallback_used: bool,
    pub protocol_status: &'static str,
    pub options: Vec<EventOptionAssessment>,
    pub audit: Value,
}

#[derive(Clone, Debug)]
pub struct EventDecision {
    pub option_index: usize,
    pub command_index: usize,
    pub summary: String,
    pub detail: String,
    pub diagnostics: EventDecisionDiagnostics,
    pub deck_ops: Option<DeckOpsAssessment>,
}

#[derive(Clone, Copy)]
struct EventContext<'a> {
    run_state: &'a RunState,
    need: RunNeedSnapshot,
    remove_score: i32,
    upgrade_score: i32,
    duplicate_score: i32,
}

#[derive(Clone, Copy)]
struct EventEvaluation {
    benefit_score: i32,
    penalty_score: i32,
    transition_bonus: i32,
    situational_bonus: i32,
    rationale_key: &'static str,
}

pub fn decide_local(run_state: &RunState, event_state: &EventState) -> Option<EventDecision> {
    let options = get_event_options(run_state);
    if options.is_empty() {
        return None;
    }
    let audit = json!({
        "planner": "event_baseline",
        "mode": "local",
        "event_id": format!("{:?}", event_state.id),
        "screen": event_state.current_screen,
    });
    Some(score_options(run_state, options, "ready", false, audit))
}

pub fn decide_live(screen_state: &Value, run_state: &RunState) -> Option<EventDecision> {
    match analyze_live_event_rebuild(run_state, screen_state) {
        LiveEventRebuildResult::Ready {
            event_id,
            current_screen,
            options,
            ..
        } => Some(score_options(
            run_state,
            options,
            "ready",
            false,
            json!({
                "planner": "event_baseline",
                "mode": "live",
                "event_id": format!("{:?}", event_id),
                "screen": current_screen,
                "rebuild_status": "ready",
            }),
        )),
        other => score_live_fallback(run_state, screen_state, other),
    }
}

pub fn compact_choice_summary(decision: &EventDecision) -> String {
    decision.summary.clone()
}

pub fn describe_choice(decision: &EventDecision) -> String {
    decision.detail.clone()
}

pub fn audit_json(decision: &EventDecision) -> Value {
    decision.diagnostics.audit.clone()
}

fn score_options(
    run_state: &RunState,
    options: Vec<EventOption>,
    protocol_status: &'static str,
    fallback_used: bool,
    mut audit: Value,
) -> EventDecision {
    let context = build_context(run_state);
    let mut assessments = options
        .iter()
        .enumerate()
        .map(|(idx, option)| assess_option(&context, idx, option))
        .collect::<Vec<_>>();
    assessments.sort_by(|lhs, rhs| {
        rhs.score
            .cmp(&lhs.score)
            .then_with(|| lhs.option_index.cmp(&rhs.option_index))
    });
    let chosen = assessments
        .iter()
        .find(|option| !option.disabled)
        .cloned()
        .unwrap_or_else(|| assessments[0].clone());
    let deck_ops = deck_ops_for_option(run_state, &options[chosen.option_index]);

    if let Some(object) = audit.as_object_mut() {
        object.insert("options".to_string(), json!(assessments));
        if let Some(assessment) = deck_ops.as_ref() {
            object.insert(
                "deck_ops".to_string(),
                crate::bot::deck_ops::deck_ops_assessment_json(assessment),
            );
        }
    }

    EventDecision {
        option_index: chosen.option_index,
        command_index: chosen.command_index,
        summary: format!(
            "choose={} score={} rationale={}",
            chosen.label, chosen.score, chosen.rationale_key
        ),
        detail: format!(
            "option={} score={} benefit={} penalty={} transition={} situational={} disabled={} rationale={}",
            chosen.label,
            chosen.score,
            chosen.benefit_score,
            chosen.penalty_score,
            chosen.transition_bonus,
            chosen.situational_bonus,
            chosen.disabled,
            chosen.rationale_key
        ),
        diagnostics: EventDecisionDiagnostics {
            chosen_index: chosen.option_index,
            fallback_used,
            protocol_status,
            options: assessments,
            audit,
        },
        deck_ops,
    }
}

fn score_live_fallback(
    run_state: &RunState,
    screen_state: &Value,
    result: LiveEventRebuildResult,
) -> Option<EventDecision> {
    let options = screen_state.get("options")?.as_array()?;
    let assessments = options
        .iter()
        .enumerate()
        .map(|(idx, option)| fallback_option_assessment(run_state, idx, option))
        .collect::<Vec<_>>();
    let mut sorted = assessments.clone();
    sorted.sort_by(|lhs, rhs| {
        rhs.score
            .cmp(&lhs.score)
            .then_with(|| lhs.option_index.cmp(&rhs.option_index))
    });
    let chosen = sorted
        .iter()
        .find(|option| !option.disabled)
        .cloned()
        .unwrap_or_else(|| sorted[0].clone());

    Some(EventDecision {
        option_index: chosen.option_index,
        command_index: chosen.command_index,
        summary: format!("choose={} score={} fallback", chosen.label, chosen.score),
        detail: format!(
            "fallback option={} score={} benefit={} penalty={} rationale={}",
            chosen.label,
            chosen.score,
            chosen.benefit_score,
            chosen.penalty_score,
            chosen.rationale_key
        ),
        diagnostics: EventDecisionDiagnostics {
            chosen_index: chosen.option_index,
            fallback_used: true,
            protocol_status: protocol_status(&result),
            options: sorted,
            audit: json!({
                "planner": "event_baseline",
                "mode": "live_fallback",
                "rebuild_status": protocol_status(&result),
            }),
        },
        deck_ops: None,
    })
}

fn build_context(run_state: &RunState) -> EventContext<'_> {
    EventContext {
        run_state,
        need: analyze_run_needs(run_state),
        remove_score: deck_ops::assess(run_state, DeckOperationKind::Remove)
            .total_score
            .max(0),
        upgrade_score: deck_ops::assess(run_state, DeckOperationKind::Upgrade)
            .total_score
            .max(0),
        duplicate_score: deck_ops::assess(run_state, DeckOperationKind::Duplicate)
            .total_score
            .max(0),
    }
}

fn assess_option(
    context: &EventContext<'_>,
    idx: usize,
    option: &EventOption,
) -> EventOptionAssessment {
    let evaluation = evaluate_option(context, option);
    EventOptionAssessment {
        option_index: idx,
        command_index: idx,
        label: option.ui.text.clone(),
        score: if option.ui.disabled {
            i32::MIN / 4
        } else {
            evaluation.benefit_score + evaluation.transition_bonus + evaluation.situational_bonus
                - evaluation.penalty_score
        },
        benefit_score: evaluation.benefit_score,
        penalty_score: evaluation.penalty_score,
        transition_bonus: evaluation.transition_bonus,
        situational_bonus: evaluation.situational_bonus,
        rationale_key: evaluation.rationale_key,
        disabled: option.ui.disabled,
    }
}

fn evaluate_option(context: &EventContext<'_>, option: &EventOption) -> EventEvaluation {
    let (mut benefit_score, mut penalty_score) = action_baseline(context, option.semantics.action);

    for effect in &option.semantics.effects {
        let (benefit, penalty) = effect_evaluation(context, effect);
        benefit_score += benefit;
        penalty_score += penalty;
    }
    for constraint in &option.semantics.constraints {
        penalty_score += constraint_penalty(context, constraint);
    }

    let transition_bonus = transition_bonus(&option.semantics.transition);
    let mut situational_bonus = situational_bonus(context, option);
    if option.semantics.repeatable {
        situational_bonus -= 4;
    }

    EventEvaluation {
        benefit_score,
        penalty_score,
        transition_bonus,
        situational_bonus,
        rationale_key: rationale_key(context, option),
    }
}

fn action_baseline(context: &EventContext<'_>, action: EventActionKind) -> (i32, i32) {
    match action {
        EventActionKind::Leave => (8, 0),
        EventActionKind::Continue => (12, 0),
        EventActionKind::Accept => (18, 0),
        EventActionKind::Decline => (10, 0),
        EventActionKind::Fight => {
            if context.need.hp_ratio >= 0.70 {
                (22, 0)
            } else {
                (0, 12)
            }
        }
        EventActionKind::Trade => (18, 0),
        EventActionKind::DeckOperation => (24, 0),
        EventActionKind::Gain => (24, 0),
        EventActionKind::Special => (16, 0),
        EventActionKind::Unknown => (8, 0),
    }
}

fn effect_evaluation(context: &EventContext<'_>, effect: &EventEffect) -> (i32, i32) {
    match effect {
        EventEffect::GainGold(amount) => ((*amount / 5).clamp(0, 40), 0),
        EventEffect::LoseGold(amount) => (0, (*amount / 5).clamp(0, 40)),
        EventEffect::LoseHp(amount) => (
            0,
            amount
                * if context.run_state.current_hp * 10 < context.run_state.max_hp * 6 {
                    4
                } else {
                    2
                },
        ),
        EventEffect::LoseMaxHp(amount) => (0, amount * 8),
        EventEffect::Heal(amount) => {
            let benefit = if context.run_state.current_hp < context.run_state.max_hp {
                amount * 3
            } else {
                *amount
            };
            (benefit, 0)
        }
        EventEffect::GainMaxHp(amount) => (amount * 8, 0),
        EventEffect::ObtainRelic { count, .. } => (34 * *count as i32, 0),
        EventEffect::ObtainPotion { count } => (18 * *count as i32, 0),
        EventEffect::ObtainCard { count, kind }
        | EventEffect::ObtainColorlessCard { count, kind } => {
            (card_kind_value(context.run_state, kind) * *count as i32, 0)
        }
        EventEffect::ObtainCurse { count, kind } => (0, curse_kind_penalty(*kind) * *count as i32),
        EventEffect::RemoveCard { count, .. } => (context.remove_score * *count as i32, 0),
        EventEffect::UpgradeCard { count } => (context.upgrade_score * *count as i32, 0),
        EventEffect::TransformCard { count } => (
            deck_ops::assess(
                context.run_state,
                DeckOperationKind::Transform {
                    count: *count,
                    upgraded_context: false,
                },
            )
            .total_score
            .max(0),
            0,
        ),
        EventEffect::DuplicateCard { count } => (context.duplicate_score * *count as i32, 0),
        EventEffect::LoseRelic { .. } | EventEffect::LoseStarterRelic { .. } => (0, 48),
        EventEffect::StartCombat => {
            if context.run_state.current_hp * 10 >= context.run_state.max_hp * 7 {
                (18, 0)
            } else {
                (0, 20)
            }
        }
    }
}

fn card_kind_value(run_state: &RunState, kind: &EventCardKind) -> i32 {
    match kind {
        EventCardKind::Specific(card_id) => {
            crate::bot::deck_scoring::score_card_offer(*card_id, run_state)
        }
        EventCardKind::RandomColorless => 22,
        EventCardKind::RandomClassCard => 26,
        EventCardKind::Unknown => 16,
    }
}

fn curse_kind_penalty(kind: EventCardKind) -> i32 {
    match kind {
        EventCardKind::Specific(card_id) => {
            18 + crate::bot::deck_scoring::curse_remove_severity(card_id) * 18
        }
        EventCardKind::RandomColorless | EventCardKind::RandomClassCard => 36,
        EventCardKind::Unknown => 30,
    }
}

fn constraint_penalty(context: &EventContext<'_>, constraint: &EventOptionConstraint) -> i32 {
    match constraint {
        EventOptionConstraint::RequiresGold(amount) => {
            if context.run_state.gold >= *amount {
                0
            } else {
                200
            }
        }
        EventOptionConstraint::RequiresRelic(relic_id) => {
            if context
                .run_state
                .relics
                .iter()
                .any(|relic| relic.id == *relic_id)
            {
                0
            } else {
                200
            }
        }
        EventOptionConstraint::RequiresRemovableCard => {
            if context.run_state.master_deck.is_empty() {
                200
            } else {
                0
            }
        }
        EventOptionConstraint::RequiresUpgradeableCard => {
            if deck_ops::best_upgrade_index(context.run_state).is_some() {
                0
            } else {
                200
            }
        }
        EventOptionConstraint::RequiresTransformableCard => {
            if context.run_state.master_deck.is_empty() {
                200
            } else {
                0
            }
        }
        EventOptionConstraint::RequiresPotion => {
            if context.run_state.potions.iter().any(|slot| slot.is_some()) {
                0
            } else {
                200
            }
        }
        EventOptionConstraint::RequiresPotionSlotValue => {
            if context.run_state.potions.iter().any(|slot| slot.is_none()) {
                0
            } else {
                24
            }
        }
    }
}

fn transition_bonus(transition: &EventOptionTransition) -> i32 {
    match transition {
        EventOptionTransition::AdvanceScreen => 4,
        EventOptionTransition::Complete => 6,
        EventOptionTransition::OpenSelection(kind) => match kind {
            EventSelectionKind::RemoveCard
            | EventSelectionKind::UpgradeCard
            | EventSelectionKind::TransformCard
            | EventSelectionKind::DuplicateCard => 8,
            EventSelectionKind::OfferCard => 4,
            EventSelectionKind::None => 0,
        },
        EventOptionTransition::OpenReward => 8,
        EventOptionTransition::StartCombat => 0,
        EventOptionTransition::None => 0,
    }
}

fn situational_bonus(context: &EventContext<'_>, option: &EventOption) -> i32 {
    let has_deck_ops = option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::RemoveCard { .. }
                | EventEffect::UpgradeCard { .. }
                | EventEffect::TransformCard { .. }
                | EventEffect::DuplicateCard { .. }
        )
    });
    let has_recovery = option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::Heal(_) | EventEffect::GainMaxHp(_) | EventEffect::ObtainPotion { .. }
        )
    });
    let has_big_gain = option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic { .. } | EventEffect::GainGold(_)
        )
    });

    let mut bonus = 0;
    if has_deck_ops {
        bonus += 4;
    }
    if has_recovery && context.need.survival_pressure >= 120 {
        bonus += 6;
    }
    if has_big_gain && context.need.missing_keys > 0 {
        bonus += 2;
    }
    bonus
}

fn fallback_option_assessment(
    run_state: &RunState,
    idx: usize,
    option: &Value,
) -> EventOptionAssessment {
    let label = option
        .get("label")
        .or_else(|| option.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("option")
        .to_string();
    let label_lower = label.to_ascii_lowercase();

    let mut benefit_score = if label_lower.contains("leave")
        || label_lower.contains("skip")
        || label_lower.contains("ignore")
        || label_lower.contains("continue")
    {
        8
    } else {
        18
    };
    let mut penalty_score = 0;

    if label_lower.contains("lose")
        && (label_lower.contains("max hp") || label_lower.contains("hp"))
    {
        penalty_score += if run_state.current_hp * 10 < run_state.max_hp * 6 {
            40
        } else {
            20
        };
    }
    if label_lower.contains("curse") {
        penalty_score += 36;
    }
    if label_lower.contains("fight") || label_lower.contains("combat") {
        if run_state.current_hp * 10 >= run_state.max_hp * 7 {
            benefit_score += 12;
        } else {
            penalty_score += 20;
        }
    }
    if label_lower.contains("gold") {
        benefit_score += 12;
    }
    if label_lower.contains("relic") {
        benefit_score += 22;
    }
    if label_lower.contains("remove") || label_lower.contains("purge") {
        benefit_score += deck_ops::assess(run_state, DeckOperationKind::Remove)
            .total_score
            .max(0);
    }
    if label_lower.contains("upgrade") {
        benefit_score += deck_ops::assess(run_state, DeckOperationKind::Upgrade)
            .total_score
            .max(0);
    }
    if label_lower.contains("transform") {
        benefit_score += deck_ops::assess(
            run_state,
            DeckOperationKind::Transform {
                count: 1,
                upgraded_context: false,
            },
        )
        .total_score
        .max(0);
    }
    if label_lower.contains("duplicate") {
        benefit_score += deck_ops::assess(run_state, DeckOperationKind::Duplicate)
            .total_score
            .max(0);
    }
    if label_lower.contains("potion") {
        benefit_score +=
            score_reward_potion(run_state, crate::content::potions::PotionId::PowerPotion) / 4;
    }

    EventOptionAssessment {
        option_index: idx,
        command_index: idx,
        label,
        score: benefit_score - penalty_score,
        benefit_score,
        penalty_score,
        transition_bonus: 0,
        situational_bonus: 0,
        rationale_key: "event_fallback_conservative",
        disabled: option
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn protocol_status(result: &LiveEventRebuildResult) -> &'static str {
    match result {
        LiveEventRebuildResult::Ready { .. } => "ready",
        LiveEventRebuildResult::UnknownEventName { .. } => "unknown_event_name",
        LiveEventRebuildResult::MissingSemanticsState { .. } => "missing_semantics_state",
        LiveEventRebuildResult::StateDecodeFailed { .. } => "state_decode_failed",
        LiveEventRebuildResult::UnsupportedEvent { .. } => "unsupported_event",
        LiveEventRebuildResult::OptionCountMismatch { .. } => "option_count_mismatch",
        LiveEventRebuildResult::DisabledMismatch { .. } => "disabled_mismatch",
    }
}

fn rationale_key(context: &EventContext<'_>, option: &EventOption) -> &'static str {
    if option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::RemoveCard { .. }
                | EventEffect::UpgradeCard { .. }
                | EventEffect::TransformCard { .. }
                | EventEffect::DuplicateCard { .. }
        )
    }) {
        "event_deck_improvement"
    } else if option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::LoseHp(_)
                | EventEffect::LoseMaxHp(_)
                | EventEffect::ObtainCurse { .. }
                | EventEffect::LoseGold(_)
                | EventEffect::LoseRelic { .. }
                | EventEffect::LoseStarterRelic { .. }
        )
    }) {
        "event_cost_tradeoff"
    } else if option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic { .. }
                | EventEffect::GainGold(_)
                | EventEffect::Heal(_)
                | EventEffect::GainMaxHp(_)
                | EventEffect::ObtainPotion { .. }
        )
    }) {
        "event_gain_value"
    } else if matches!(option.semantics.action, EventActionKind::Fight) {
        if context.need.hp_ratio >= 0.70 {
            "event_fight_ok"
        } else {
            "event_fight_risk"
        }
    } else {
        "event_baseline"
    }
}

fn deck_ops_for_option(run_state: &RunState, option: &EventOption) -> Option<DeckOpsAssessment> {
    for effect in &option.semantics.effects {
        let assessment = match effect {
            EventEffect::RemoveCard { .. } => {
                Some(deck_ops::assess(run_state, DeckOperationKind::Remove))
            }
            EventEffect::UpgradeCard { .. } => {
                Some(deck_ops::assess(run_state, DeckOperationKind::Upgrade))
            }
            EventEffect::TransformCard { count } => Some(deck_ops::assess(
                run_state,
                DeckOperationKind::Transform {
                    count: *count,
                    upgraded_context: false,
                },
            )),
            EventEffect::DuplicateCard { .. } => {
                Some(deck_ops::assess(run_state, DeckOperationKind::Duplicate))
            }
            _ => None,
        };
        if assessment.is_some() {
            return assessment;
        }
    }
    None
}
