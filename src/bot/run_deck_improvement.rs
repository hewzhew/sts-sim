use crate::bot::card_structure::structure as card_structure;
use crate::bot::evaluator::{CardEvaluator, DeckProfile};
use crate::bot::run_rule_context::{self, RunRuleContext};
use crate::bot::upgrade_facts;
use crate::content::cards::{self, CardId, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeckOperationKind {
    Add(CardId),
    Remove,
    Upgrade,
    Duplicate,
    Transform {
        count: usize,
        upgraded_context: bool,
    },
    VampiresExchange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckOperationCandidate {
    pub target_index: Option<usize>,
    pub target_uuid: Option<u32>,
    pub score: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeckOperationAssessment {
    pub operation: DeckOperationKind,
    pub best_candidate: Option<DeckOperationCandidate>,
    pub total_prior_delta: i32,
    pub baseline_deck_delta: i32,
    pub need_aware_delta: i32,
    pub rule_context_delta: i32,
    pub shell_delta: i32,
    pub clutter_delta: i32,
    pub operation_specific_delta: i32,
    pub rationale_key: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct DeckImprovementContext {
    profile: DeckProfile,
    rule: RunRuleContext,
    need: Option<crate::bot::noncombat_families::NoncombatNeedSnapshot>,
}

#[derive(Clone, Copy, Debug, Default)]
struct CandidateComponents {
    baseline: i32,
    need: i32,
    rule: i32,
    shell: i32,
    clutter: i32,
    specific: i32,
}

#[derive(Clone, Debug)]
struct RankedCandidate {
    candidate: DeckOperationCandidate,
    components: CandidateComponents,
}

pub(crate) fn assess_deck_operation(
    rs: &RunState,
    operation: DeckOperationKind,
) -> DeckOperationAssessment {
    let ctx = build_deck_improvement_context(rs, None);
    assess_deck_operation_with_context(rs, &ctx, operation)
}

pub(crate) fn assess_deck_operation_with_need(
    rs: &RunState,
    need: &crate::bot::noncombat_families::NoncombatNeedSnapshot,
    operation: DeckOperationKind,
) -> DeckOperationAssessment {
    let ctx = build_deck_improvement_context(rs, Some(*need));
    assess_deck_operation_with_context(rs, &ctx, operation)
}

pub(crate) fn deck_operation_assessment_json(assessment: &DeckOperationAssessment) -> Value {
    json!({
        "operation": format!("{:?}", assessment.operation),
        "total_prior_delta": assessment.total_prior_delta,
        "baseline_deck_delta": assessment.baseline_deck_delta,
        "need_aware_delta": assessment.need_aware_delta,
        "rule_context_delta": assessment.rule_context_delta,
        "shell_delta": assessment.shell_delta,
        "clutter_delta": assessment.clutter_delta,
        "operation_specific_delta": assessment.operation_specific_delta,
        "rationale_key": assessment.rationale_key,
        "best_candidate": assessment.best_candidate.as_ref().map(|candidate| json!({
            "target_index": candidate.target_index,
            "target_uuid": candidate.target_uuid,
            "score": candidate.score,
            "rationale_key": candidate.rationale_key,
        })),
    })
}

pub(crate) fn deck_operation_component_breakdown(
    assessment: &DeckOperationAssessment,
) -> [(&'static str, i32); 6] {
    [
        ("baseline", assessment.baseline_deck_delta),
        ("need", assessment.need_aware_delta),
        ("rule", assessment.rule_context_delta),
        ("shell", assessment.shell_delta),
        ("clutter", assessment.clutter_delta),
        ("specific", assessment.operation_specific_delta),
    ]
}

pub(crate) fn deck_operation_focus_summary(assessment: &DeckOperationAssessment) -> String {
    let mut components = deck_operation_component_breakdown(assessment)
        .into_iter()
        .filter(|(_, value)| *value != 0)
        .collect::<Vec<_>>();
    components.sort_by_key(|(_, value)| -value.abs());
    let top = components
        .into_iter()
        .take(3)
        .map(|(name, value)| format!("{name}:{value:+}"))
        .collect::<Vec<_>>();
    let top_text = if top.is_empty() {
        "none".to_string()
    } else {
        top.join(",")
    };
    format!(
        "op={:?} total={} rationale={} comps={}",
        assessment.operation, assessment.total_prior_delta, assessment.rationale_key, top_text
    )
}

fn build_deck_improvement_context(
    rs: &RunState,
    need: Option<crate::bot::noncombat_families::NoncombatNeedSnapshot>,
) -> DeckImprovementContext {
    DeckImprovementContext {
        profile: CardEvaluator::deck_profile(rs),
        rule: run_rule_context::build_run_rule_context(rs),
        need,
    }
}

fn assess_deck_operation_with_context(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    operation: DeckOperationKind,
) -> DeckOperationAssessment {
    match operation {
        DeckOperationKind::Add(card_id) => {
            let candidate = rank_add_candidate(rs, ctx, card_id);
            assessment_from_ranked(operation, Some(candidate), "add_card_fit_and_need")
        }
        DeckOperationKind::Remove => {
            let ranked = (0..rs.master_deck.len())
                .map(|idx| rank_remove_candidate(rs, ctx, idx))
                .max_by_key(|entry| {
                    (
                        entry.candidate.score,
                        -(entry.candidate.target_index.unwrap_or(0) as i32),
                    )
                });
            assessment_from_ranked(operation, ranked, "remove_best_target")
        }
        DeckOperationKind::Upgrade => {
            let ranked = (0..rs.master_deck.len())
                .filter(|&idx| is_upgradable(&rs.master_deck[idx]))
                .map(|idx| rank_upgrade_candidate(rs, ctx, idx))
                .max_by_key(|entry| {
                    (
                        entry.candidate.score,
                        -(entry.candidate.target_index.unwrap_or(0) as i32),
                    )
                });
            assessment_from_ranked(operation, ranked, "upgrade_best_target")
        }
        DeckOperationKind::Duplicate => {
            let ranked = (0..rs.master_deck.len())
                .map(|idx| rank_duplicate_candidate(rs, ctx, idx))
                .max_by_key(|entry| {
                    (
                        entry.candidate.score,
                        -(entry.candidate.target_index.unwrap_or(0) as i32),
                    )
                });
            assessment_from_ranked(operation, ranked, "duplicate_best_target")
        }
        DeckOperationKind::Transform {
            count,
            upgraded_context,
        } => {
            let mut ranked = (0..rs.master_deck.len())
                .map(|idx| rank_transform_candidate(rs, ctx, idx, upgraded_context))
                .collect::<Vec<_>>();
            ranked.sort_by_key(|entry| {
                (
                    -entry.candidate.score,
                    entry.candidate.target_index.unwrap_or(0) as i32,
                )
            });
            let best = ranked.first().cloned();
            let mut assessment = assessment_from_ranked(operation, best, "transform_best_target");
            if count > 1 {
                let extra = ranked
                    .iter()
                    .skip(1)
                    .take(count.saturating_sub(1))
                    .map(|entry| entry.candidate.score.max(0))
                    .sum::<i32>();
                assessment.total_prior_delta += extra;
                assessment.operation_specific_delta += extra;
            }
            assessment
        }
        DeckOperationKind::VampiresExchange => assess_vampires_exchange(rs, ctx),
    }
}

fn assessment_from_ranked(
    operation: DeckOperationKind,
    ranked: Option<RankedCandidate>,
    fallback_key: &'static str,
) -> DeckOperationAssessment {
    let Some(ranked) = ranked else {
        return DeckOperationAssessment {
            operation,
            best_candidate: None,
            total_prior_delta: 0,
            baseline_deck_delta: 0,
            need_aware_delta: 0,
            rule_context_delta: 0,
            shell_delta: 0,
            clutter_delta: 0,
            operation_specific_delta: 0,
            rationale_key: fallback_key,
        };
    };
    DeckOperationAssessment {
        operation,
        total_prior_delta: ranked.candidate.score,
        baseline_deck_delta: ranked.components.baseline,
        need_aware_delta: ranked.components.need,
        rule_context_delta: ranked.components.rule,
        shell_delta: ranked.components.shell,
        clutter_delta: ranked.components.clutter,
        operation_specific_delta: ranked.components.specific,
        rationale_key: ranked.candidate.rationale_key,
        best_candidate: Some(ranked.candidate),
    }
}

fn rank_add_candidate(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    card_id: CardId,
) -> RankedCandidate {
    let baseline_before = baseline_run_deck_value(rs);
    let baseline_after = crate::state::run::with_suppressed_obtain_logs(|| {
        let mut after = rs.clone();
        after.add_card_to_deck(card_id);
        baseline_run_deck_value(&after)
    });
    let conditioned =
        run_rule_context::conditioned_card_addition_value_with_context(rs, card_id, &ctx.rule);
    let shell = card_fit_shell_bonus(card_id, &ctx.profile);
    let need = add_need_bonus(ctx.need, card_id);
    let clutter = add_clutter_penalty(card_id, ctx);
    let specific = add_operation_specific_bonus(card_id, ctx);
    let components = CandidateComponents {
        baseline: baseline_after - baseline_before,
        need,
        rule: conditioned.total,
        shell,
        clutter,
        specific,
    };
    let score = sum_components(components);
    RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: None,
            target_uuid: None,
            score,
            rationale_key: conditioned.rationale_key.unwrap_or("add_general_fit"),
        },
        components,
    }
}

fn rank_remove_candidate(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    idx: usize,
) -> RankedCandidate {
    let card = &rs.master_deck[idx];
    let baseline = removal_baseline_delta(rs, idx);
    let curse = curse_severity(card.id) * 32 + i32::from(is_dead_slot(card.id)) * 60;
    let starter = i32::from(cards::is_starter_basic(card.id)) * 80
        + i32::from(cards::is_starter_strike(card.id)) * 30;
    let draw_relief = i32::from(CardEvaluator::evaluate_owned_card(card.id, rs) <= 12) * 26;
    let shell = -card_fit_shell_bonus(card.id, &ctx.profile).min(40);
    let need = ctx.need.map(|need| need.purge_value / 10).unwrap_or(0);
    let rule = i32::from(
        ctx.rule.low_cost_filler_density >= 4 && cards::get_card_definition(card.id).cost <= 1,
    ) * 12;
    let components = CandidateComponents {
        baseline,
        need,
        rule,
        shell,
        clutter: curse + starter + draw_relief,
        specific: if matches!(
            cards::get_card_definition(card.id).card_type,
            CardType::Curse | CardType::Status
        ) {
            70
        } else {
            0
        },
    };
    let rationale_key = if curse_severity(card.id) >= 8
        || matches!(
            cards::get_card_definition(card.id).card_type,
            CardType::Curse | CardType::Status
        ) {
        "remove_curse_burden"
    } else if cards::is_starter_basic(card.id) {
        "remove_starter_density"
    } else {
        "remove_low_value_slot"
    };
    RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: Some(idx),
            target_uuid: Some(card.uuid),
            score: sum_components(components),
            rationale_key,
        },
        components,
    }
}

fn rank_upgrade_candidate(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    idx: usize,
) -> RankedCandidate {
    let card = &rs.master_deck[idx];
    let baseline = upgrade_baseline_delta(rs, idx);
    let (operation_bonus, rationale_key) =
        upgrade_operation_semantic_bonus(card.id, card.upgrades, ctx);
    let need = ctx
        .need
        .map(|need| need.best_upgrade_value / 14)
        .unwrap_or(0);
    let rule = conditioned_upgrade_bonus(rs, card.id, &ctx.rule);
    let shell =
        card_fit_shell_bonus(card.id, &ctx.profile) + upgrade_shell_bonus(card.id, &ctx.profile);
    let clutter = i32::from(
        ctx.need.is_some_and(|need| need.survival_pressure >= 180)
            && card_is_survival_patch(card.id),
    ) * 24;
    let components = CandidateComponents {
        baseline,
        need,
        rule,
        shell,
        clutter,
        specific: operation_bonus,
    };
    RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: Some(idx),
            target_uuid: Some(card.uuid),
            score: sum_components(components),
            rationale_key,
        },
        components,
    }
}

fn rank_duplicate_candidate(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    idx: usize,
) -> RankedCandidate {
    let card = &rs.master_deck[idx];
    let baseline = duplicate_baseline_delta(rs, idx);
    let shell = card_fit_shell_bonus(card.id, &ctx.profile)
        + i32::from(card_is_core_duplicate(card.id, &ctx.profile)) * 28;
    let need = ctx
        .need
        .map(|need| need.long_term_meta_value / 18 - need.survival_pressure / 24)
        .unwrap_or(0);
    let rule =
        i32::from(ctx.rule.summary.draw_rich && cards::get_card_definition(card.id).cost >= 2) * 10;
    let clog_penalty = i32::from(is_low_impact_filler(card.id) || cards::is_starter_basic(card.id))
        * 90
        + i32::from(ctx.rule.self_clog_replication_risk > 0 && card.id == CardId::Anger) * 120;
    let win_more_penalty = i32::from(
        ctx.need.is_some_and(|need| need.survival_pressure >= 180) && card_is_slow_setup(card.id),
    ) * 80;
    let components = CandidateComponents {
        baseline,
        need,
        rule,
        shell,
        clutter: -(clog_penalty + win_more_penalty),
        specific: 0,
    };
    RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: Some(idx),
            target_uuid: Some(card.uuid),
            score: sum_components(components),
            rationale_key: "duplicate_best_core",
        },
        components,
    }
}

fn rank_transform_candidate(
    rs: &RunState,
    ctx: &DeckImprovementContext,
    idx: usize,
    upgraded_context: bool,
) -> RankedCandidate {
    let remove_rank = rank_remove_candidate(rs, ctx, idx);
    let replacement =
        transform_replacement_expectation(ctx, &rs.master_deck[idx], upgraded_context);
    let mut components = remove_rank.components;
    components.specific += replacement;
    RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: remove_rank.candidate.target_index,
            target_uuid: remove_rank.candidate.target_uuid,
            score: sum_components(components),
            rationale_key: if upgraded_context {
                "transform_upgrade_window"
            } else {
                "transform_replace_burden"
            },
        },
        components,
    }
}

fn assess_vampires_exchange(
    rs: &RunState,
    ctx: &DeckImprovementContext,
) -> DeckOperationAssessment {
    let strike_count = rs
        .master_deck
        .iter()
        .filter(|card| cards::is_starter_strike(card.id))
        .count() as i32;
    if strike_count == 0 {
        return assessment_from_ranked(
            DeckOperationKind::VampiresExchange,
            None,
            "vampires_no_strikes",
        );
    }
    let baseline_before = baseline_run_deck_value(rs);
    let mut after = rs.clone();
    after
        .master_deck
        .retain(|card| !cards::is_starter_strike(card.id));
    for _ in 0..strike_count {
        after
            .master_deck
            .push(CombatCard::new(CardId::Bite, next_synthetic_uuid(&after)));
    }
    let components = CandidateComponents {
        baseline: baseline_run_deck_value(&after) - baseline_before,
        need: ctx
            .need
            .map(|need| need.survival_pressure / 10 - need.purge_value / 18)
            .unwrap_or(0),
        rule: i32::from(ctx.rule.summary.strength_scaling) * 8,
        shell: card_fit_shell_bonus(CardId::Bite, &ctx.profile) * strike_count.min(2),
        clutter: strike_count * 18,
        specific: i32::from(rs.current_hp * 10 < rs.max_hp * 7) * 30,
    };
    let ranked = RankedCandidate {
        candidate: DeckOperationCandidate {
            target_index: None,
            target_uuid: None,
            score: sum_components(components),
            rationale_key: "vampires_bite_package",
        },
        components,
    };
    assessment_from_ranked(
        DeckOperationKind::VampiresExchange,
        Some(ranked),
        "vampires_bite_package",
    )
}

fn baseline_run_deck_value(rs: &RunState) -> i32 {
    let profile = CardEvaluator::deck_profile(rs);
    let mut score = rs
        .master_deck
        .iter()
        .map(|card| per_card_baseline_value(rs, card))
        .sum::<i32>();
    score += profile.draw_sources * 10
        + profile.power_scalers * 6
        + profile.block_core.min(4) * 6
        + profile.attack_count.min(8) * 2;
    score += profile.strength_enablers.min(profile.strength_payoffs) * 26
        + profile.exhaust_engines.min(profile.exhaust_outlets) * 28;
    score -= rs
        .master_deck
        .iter()
        .map(|card| curse_severity(card.id) * 14 + i32::from(cards::is_starter_basic(card.id)) * 6)
        .sum::<i32>();
    score
}

fn per_card_baseline_value(rs: &RunState, card: &CombatCard) -> i32 {
    let mut score = CardEvaluator::evaluate_owned_card(card.id, rs);
    if cards::is_starter_strike(card.id) {
        score -= 14;
    } else if cards::is_starter_defend(card.id) {
        score -= 10;
    }
    if matches!(
        cards::get_card_definition(card.id).card_type,
        CardType::Curse | CardType::Status
    ) {
        score -= 140;
    }
    if card.upgrades > 0
        && !matches!(
            cards::get_card_definition(card.id).card_type,
            CardType::Curse | CardType::Status
        )
    {
        score += baseline_upgrade_semantic_bonus(card.id, card.upgrades);
    }
    score
}

fn removal_baseline_delta(rs: &RunState, idx: usize) -> i32 {
    let before = baseline_run_deck_value(rs);
    let mut working = rs.clone();
    working.master_deck.remove(idx);
    baseline_run_deck_value(&working) - before
}

fn upgrade_baseline_delta(rs: &RunState, idx: usize) -> i32 {
    let before = baseline_run_deck_value(rs);
    let mut working = rs.clone();
    working.master_deck[idx].upgrades += 1;
    baseline_run_deck_value(&working) - before
}

fn duplicate_baseline_delta(rs: &RunState, idx: usize) -> i32 {
    let before = baseline_run_deck_value(rs);
    let mut working = rs.clone();
    let mut duplicated = working.master_deck[idx].clone();
    duplicated.uuid = next_synthetic_uuid(&working);
    working.master_deck.push(duplicated);
    baseline_run_deck_value(&working) - before
}

fn conditioned_upgrade_bonus(rs: &RunState, card_id: CardId, rule: &RunRuleContext) -> i32 {
    run_rule_context::conditioned_card_addition_value_with_context(rs, card_id, rule).total / 2
}

fn sum_components(components: CandidateComponents) -> i32 {
    components.baseline
        + components.need
        + components.rule
        + components.shell
        + components.clutter
        + components.specific
}

fn card_fit_shell_bonus(card_id: CardId, profile: &DeckProfile) -> i32 {
    let structure = card_structure(card_id);
    let mut bonus = 0;
    if profile.strength_enablers > 0 && structure.is_strength_payoff() {
        bonus += 18;
    }
    if profile.strength_payoffs > 0 && structure.is_strength_enabler() {
        bonus += 18;
    }
    if profile.exhaust_engines > 0 && structure.is_exhaust_outlet() {
        bonus += 18;
    }
    if profile.exhaust_outlets > 0 && structure.is_exhaust_engine() {
        bonus += 18;
    }
    if profile.block_core >= 2 && structure.is_block_payoff() {
        bonus += 22;
    }
    if profile.status_generators > 0 && structure.is_status_engine() {
        bonus += 16;
    }
    bonus
}

fn upgrade_shell_bonus(card_id: CardId, profile: &DeckProfile) -> i32 {
    card_fit_shell_bonus(card_id, profile) / 2
        + i32::from(profile.searing_blow_count > 0 && card_id == CardId::SearingBlow) * 30
}

fn add_need_bonus(
    need: Option<crate::bot::noncombat_families::NoncombatNeedSnapshot>,
    card_id: CardId,
) -> i32 {
    let Some(need) = need else {
        return 0;
    };
    let mut bonus = 0;
    if need.survival_pressure >= 180 && card_is_survival_patch(card_id) {
        bonus += 30;
    }
    if need.best_upgrade_value >= need.purge_value + 60 && card_id == CardId::Armaments {
        bonus += 16;
    }
    if need.purge_value >= need.best_upgrade_value + 60 && is_low_impact_filler(card_id) {
        bonus -= 22;
    }
    bonus
}

fn add_clutter_penalty(card_id: CardId, ctx: &DeckImprovementContext) -> i32 {
    -i32::from(is_low_impact_filler(card_id)) * 30
        - i32::from(
            ctx.rule.low_cost_filler_density >= 4 && cards::get_card_definition(card_id).cost <= 1,
        ) * 16
}

fn add_operation_specific_bonus(card_id: CardId, ctx: &DeckImprovementContext) -> i32 {
    let structure = card_structure(card_id);
    i32::from(ctx.rule.summary.energy_rich && cards::get_card_definition(card_id).cost >= 2) * 10
        + i32::from(ctx.rule.summary.draw_rich && structure.is_setup_piece()) * 8
}

fn upgrade_operation_semantic_bonus(
    card_id: CardId,
    upgrades: u8,
    ctx: &DeckImprovementContext,
) -> (i32, &'static str) {
    let facts = upgrade_facts::upgrade_facts(card_id);
    let structure = card_structure(card_id);
    let mut bonus = 0;

    if facts.repeatable_upgrade {
        bonus += 22 + upgrades as i32 * 8;
    }
    if facts.changes_cost {
        bonus += 12;
    }
    if facts.improves_draw_consistency {
        bonus += 10 + i32::from(ctx.rule.summary.draw_rich) * 4;
    }
    if facts.improves_target_control || facts.extends_debuff_duration {
        bonus += 12 + i32::from(ctx.need.is_some_and(|need| need.survival_pressure >= 140)) * 8;
    }
    if facts.improves_exhaust_control
        && (ctx.profile.exhaust_engines > 0
            || ctx.profile.exhaust_outlets > 0
            || ctx.rule.summary.exhaust_positive)
    {
        bonus += 14;
    }
    if facts.improves_scaling && (structure.is_strength_enabler() || structure.is_engine_piece()) {
        bonus += 12;
    }
    if facts.improves_block_efficiency && ctx.need.is_some_and(|need| need.survival_pressure >= 120)
    {
        bonus += 10;
    }
    if facts.improves_frontload && ctx.need.is_some_and(|need| need.survival_pressure >= 180) {
        bonus += 8;
    }

    (bonus, upgrade_facts::dominant_upgrade_semantic_key(card_id))
}

fn baseline_upgrade_semantic_bonus(card_id: CardId, upgrades: u8) -> i32 {
    let facts = upgrade_facts::upgrade_facts(card_id);
    let mut bonus = 0;
    if facts.repeatable_upgrade {
        bonus += 12 + upgrades as i32 * 4;
    }
    if facts.changes_cost {
        bonus += 6;
    }
    if facts.improves_draw_consistency {
        bonus += 5;
    }
    if facts.improves_target_control || facts.extends_debuff_duration {
        bonus += 6;
    }
    if facts.improves_exhaust_control {
        bonus += 6;
    }
    if facts.improves_scaling {
        bonus += 5;
    }
    if facts.improves_block_efficiency {
        bonus += 4;
    }
    if facts.improves_frontload {
        bonus += 4;
    }
    bonus
}

fn transform_replacement_expectation(
    ctx: &DeckImprovementContext,
    card: &CombatCard,
    upgraded_context: bool,
) -> i32 {
    let def = cards::get_card_definition(card.id);
    let mut score = if matches!(def.card_type, CardType::Curse | CardType::Status) {
        90 + curse_severity(card.id) * 10
    } else if cards::is_starter_basic(card.id) {
        64
    } else if def.rarity == cards::CardRarity::Common && !matches!(def.card_type, CardType::Power) {
        42
    } else {
        18
    };
    score += i32::from(upgraded_context) * 20;
    score += ctx
        .need
        .map(|need| need.best_upgrade_value / 24 - need.survival_pressure / 36)
        .unwrap_or(0);
    score +=
        i32::from(ctx.rule.summary.draw_rich) * 8 + i32::from(ctx.rule.summary.energy_rich) * 8;
    score
}

fn card_is_survival_patch(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::ShrugItOff
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::Disarm
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::GhostlyArmor
    )
}

fn card_is_slow_setup(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Barricade | CardId::DemonForm | CardId::Juggernaut | CardId::LimitBreak
    )
}

fn is_low_impact_filler(card_id: CardId) -> bool {
    cards::is_starter_basic(card_id)
        || matches!(
            card_id,
            CardId::Clash
                | CardId::Warcry
                | CardId::WildStrike
                | CardId::IronWave
                | CardId::TwinStrike
                | CardId::Clothesline
                | CardId::PerfectedStrike
        )
}

fn card_is_core_duplicate(card_id: CardId, profile: &DeckProfile) -> bool {
    matches!(
        card_id,
        CardId::Offering
            | CardId::Shockwave
            | CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::Corruption
            | CardId::Impervious
    ) || (profile.strength_enablers > 0
        && matches!(card_id, CardId::LimitBreak | CardId::HeavyBlade))
}

fn is_dead_slot(card_id: CardId) -> bool {
    cards::is_starter_basic(card_id)
        || matches!(
            cards::get_card_definition(card_id).card_type,
            CardType::Curse | CardType::Status
        )
}

fn curse_severity(card_id: CardId) -> i32 {
    let severity = crate::bot::evaluator::curse_remove_severity(card_id);
    if severity > 0 {
        severity
    } else if matches!(
        cards::get_card_definition(card_id).card_type,
        CardType::Curse
    ) {
        3
    } else {
        0
    }
}

fn next_synthetic_uuid(rs: &RunState) -> u32 {
    rs.master_deck
        .iter()
        .map(|card| card.uuid)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

#[allow(dead_code)]
pub(crate) fn card_add_improvement_delta(rs: &RunState, card_id: CardId) -> i32 {
    assess_deck_operation(rs, DeckOperationKind::Add(card_id)).total_prior_delta
}

#[allow(dead_code)]
pub(crate) fn best_remove_improvement(rs: &RunState) -> i32 {
    assess_deck_operation(rs, DeckOperationKind::Remove).total_prior_delta
}

pub(crate) fn best_remove_uuid(rs: &RunState) -> Option<u32> {
    assess_deck_operation(rs, DeckOperationKind::Remove)
        .best_candidate
        .and_then(|candidate| candidate.target_uuid)
}

#[allow(dead_code)]
pub(crate) fn best_upgrade_improvement(rs: &RunState) -> i32 {
    assess_deck_operation(rs, DeckOperationKind::Upgrade).total_prior_delta
}

pub(crate) fn best_upgrade_uuid(rs: &RunState) -> Option<u32> {
    assess_deck_operation(rs, DeckOperationKind::Upgrade)
        .best_candidate
        .and_then(|candidate| candidate.target_uuid)
}

#[allow(dead_code)]
pub(crate) fn best_duplicate_improvement(rs: &RunState) -> i32 {
    assess_deck_operation(rs, DeckOperationKind::Duplicate).total_prior_delta
}

pub(crate) fn best_duplicate_uuid(rs: &RunState) -> Option<u32> {
    assess_deck_operation(rs, DeckOperationKind::Duplicate)
        .best_candidate
        .and_then(|candidate| candidate.target_uuid)
}

#[allow(dead_code)]
pub(crate) fn best_transform_improvement(
    rs: &RunState,
    count: usize,
    upgraded_context: bool,
) -> i32 {
    assess_deck_operation(
        rs,
        DeckOperationKind::Transform {
            count,
            upgraded_context,
        },
    )
    .total_prior_delta
}

pub(crate) fn best_transform_uuids(
    rs: &RunState,
    count: usize,
    upgraded_context: bool,
) -> Vec<u32> {
    let assessment = assess_deck_operation(
        rs,
        DeckOperationKind::Transform {
            count,
            upgraded_context,
        },
    );
    let mut uuids = assessment
        .best_candidate
        .and_then(|candidate| candidate.target_uuid)
        .into_iter()
        .collect::<Vec<_>>();
    if count <= 1 {
        return uuids;
    }
    let ctx = build_deck_improvement_context(rs, None);
    let mut ranked = (0..rs.master_deck.len())
        .map(|idx| rank_transform_candidate(rs, &ctx, idx, upgraded_context))
        .collect::<Vec<_>>();
    ranked.sort_by_key(|entry| {
        (
            -entry.candidate.score,
            entry.candidate.target_index.unwrap_or(0) as i32,
        )
    });
    for entry in ranked {
        if let Some(uuid) = entry.candidate.target_uuid {
            if !uuids.contains(&uuid) {
                uuids.push(uuid);
            }
            if uuids.len() >= count {
                break;
            }
        }
    }
    uuids
}

#[allow(dead_code)]
pub(crate) fn vampires_bite_exchange_value(rs: &RunState) -> i32 {
    assess_deck_operation(rs, DeckOperationKind::VampiresExchange).total_prior_delta
}

pub(crate) fn is_upgradable(card: &CombatCard) -> bool {
    let def = cards::get_card_definition(card.id);
    card.id == CardId::SearingBlow
        || (card.upgrades == 0 && !matches!(def.card_type, CardType::Status | CardType::Curse))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remove_prefers_curse_over_shell_piece() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck
            .push(CombatCard::new(CardId::Parasite, 7_001));
        rs.master_deck
            .push(CombatCard::new(CardId::Shockwave, 7_002));
        let assessment = assess_deck_operation(&rs, DeckOperationKind::Remove);
        assert_eq!(
            assessment
                .best_candidate
                .and_then(|candidate| candidate.target_uuid),
            Some(7_001)
        );
    }

    #[test]
    fn upgrade_prefers_high_impact_upgrade() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck
            .push(CombatCard::new(CardId::Shockwave, 7_011));
        rs.master_deck.push(CombatCard::new(CardId::Strike, 7_012));
        let assessment = assess_deck_operation(&rs, DeckOperationKind::Upgrade);
        assert_eq!(
            assessment
                .best_candidate
                .and_then(|candidate| candidate.target_uuid),
            Some(7_011)
        );
    }

    #[test]
    fn duplicate_avoids_low_value_basics() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck.push(CombatCard::new(CardId::Strike, 7_021));
        rs.master_deck
            .push(CombatCard::new(CardId::Offering, 7_022));
        let assessment = assess_deck_operation(&rs, DeckOperationKind::Duplicate);
        assert_eq!(
            assessment
                .best_candidate
                .and_then(|candidate| candidate.target_uuid),
            Some(7_022)
        );
    }

    #[test]
    fn transform_prefers_burden_and_upgrade_window_helps() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck
            .push(CombatCard::new(CardId::Parasite, 7_031));
        let plain = assess_deck_operation(
            &rs,
            DeckOperationKind::Transform {
                count: 1,
                upgraded_context: false,
            },
        );
        let upgraded = assess_deck_operation(
            &rs,
            DeckOperationKind::Transform {
                count: 1,
                upgraded_context: true,
            },
        );
        assert_eq!(
            plain
                .best_candidate
                .and_then(|candidate| candidate.target_uuid),
            Some(7_031)
        );
        assert!(upgraded.total_prior_delta > plain.total_prior_delta);
    }

    #[test]
    fn vampires_exchange_scales_with_strike_burden() {
        let base = RunState::new(1, 0, true, "Ironclad");
        let mut with_extra_strikes = base.clone();
        with_extra_strikes
            .master_deck
            .push(CombatCard::new(CardId::Strike, 7_041));
        with_extra_strikes
            .master_deck
            .push(CombatCard::new(CardId::Strike, 7_042));
        assert!(
            vampires_bite_exchange_value(&with_extra_strikes) > vampires_bite_exchange_value(&base)
        );
    }
}
