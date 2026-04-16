use crate::bot::event_policy_helpers::{
    context_curse_drag, cursed_tome_book_relic_value, cursed_tome_remaining_commitment_damage,
    dead_adventurer_continue_score, gold_value_per_gold, hp_point_value,
    knowing_skull_colorless_value, random_potion_offer_value, relic_equity_value,
    safety_gap_penalty, scrap_ooze_continue_score, world_of_goop_gather_score,
};
use crate::bot::noncombat_families::helpers::{
    best_we_meet_again_card_give_score, best_we_meet_again_potion_give_score, contains_any,
    count_remove_targets, count_transform_targets, count_upgradable_cards, curse_pressure_score,
    curse_tractability_score, first_number, generic_remove_value, nearby_shop_conversion_bonus,
    nth_number, reachable_room_distance,
};
use crate::map::node::RoomType;
use crate::state::events::{
    EventActionKind, EventCardKind, EventEffect, EventId, EventOption, EventOptionConstraint,
    EventOptionTransition, EventRelicKind, EventState,
};
use crate::state::run::RunState;
use serde_json::{json, Value};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventOptionTag {
    Continue,
    TakeRelic,
    ObtainRelic,
    ObtainPotion,
    ObtainCard,
    ObtainRandomCard,
    ObtainColorlessCard,
    Leave,
    Remove,
    Upgrade,
    Transform,
    Duplicate,
    GainMaxHp,
    GainHp,
    LoseHp,
    LoseMaxHp,
    ObtainCurse,
    GainGold,
    LoseGold,
    LoseStarterRelic,
    LoseRelic,
    Fight,
    Read,
    Heal,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventOptionPayload {
    pub hp_cost: i32,
    pub max_hp_cost: i32,
    pub gold_delta: i32,
    pub potion_count: i32,
    pub card_count: i32,
    pub colorless_card_count: i32,
    pub relic_count: i32,
    pub curse_count: i32,
    pub heal_amount: i32,
    pub max_hp_gain: i32,
    pub lose_starter_relic: bool,
    pub repeatable: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventOptionView {
    pub index: usize,
    pub text: String,
    pub label: Option<String>,
    pub disabled: bool,
    pub choice_index: Option<usize>,
    pub semantic_tags: Vec<EventOptionTag>,
    pub payload: EventOptionPayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventPolicyFamily {
    DeckSurgery,
    PressYourLuck,
    CostTradeoff,
    ResourceShoplike,
    GenericSafe,
    CompatibilityFallback,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventDecisionFeatures {
    pub current_hp: i32,
    pub max_hp: i32,
    pub hp_ratio: f32,
    pub gold: i32,
    pub empty_potion_slots: usize,
    pub potion_blocked: bool,
    pub rest_distance: Option<i32>,
    pub shop_distance: Option<i32>,
    pub elite_distance: Option<i32>,
    pub remove_targets: i32,
    pub transform_targets: i32,
    pub upgradable_cards: i32,
    pub curse_pressure: i32,
    pub has_golden_idol: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventDecisionContext {
    pub event_id: String,
    pub event_name: String,
    pub current_screen: usize,
    pub current_screen_index: Option<usize>,
    pub current_screen_key: Option<String>,
    pub screen_source: Option<String>,
    pub options: Vec<EventOptionView>,
    pub features: EventDecisionFeatures,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventChoiceDecision {
    pub option_index: usize,
    pub command_index: usize,
    pub family: EventPolicyFamily,
    pub rationale_key: Option<&'static str>,
    pub score: Option<i32>,
    pub safety_override_applied: bool,
    pub rationale: Option<&'static str>,
    pub deck_improvement_assessment:
        Option<crate::bot::run_deck_improvement::DeckOperationAssessment>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventOptionScore {
    pub option_index: usize,
    pub total_score: f32,
    pub rationale_key: &'static str,
    pub viable: bool,
    pub safety_override_applied: bool,
    pub breakdown: Vec<(&'static str, f32)>,
    pub deck_improvement_assessment:
        Option<crate::bot::run_deck_improvement::DeckOperationAssessment>,
}

#[derive(Clone, Debug, PartialEq)]
struct OptionScoreDraft {
    total_score: f32,
    rationale_key: &'static str,
    deck_improvement_assessment: Option<crate::bot::run_deck_improvement::DeckOperationAssessment>,
}

pub fn choose_local_event_choice(
    rs: &RunState,
    event: &EventState,
    options: &[EventOption],
) -> Option<EventChoiceDecision> {
    if let Some(decision) = choose_structured_local_event_choice(rs, event, options) {
        return Some(decision);
    }
    let context = local_event_context(rs, event, options);
    choose_event_option(rs, &context)
}

pub fn choose_live_event_choice(gs: &Value, rs: &RunState) -> Option<EventChoiceDecision> {
    if let Some(decision) = choose_structured_live_event_choice(gs, rs) {
        return Some(decision);
    }
    let context = live_event_context(gs, rs)?;
    choose_event_option(rs, &context)
}

pub fn local_event_context(
    rs: &RunState,
    event: &EventState,
    options: &[EventOption],
) -> EventDecisionContext {
    let name = canonical_event_name(event.id);
    EventDecisionContext {
        event_id: name.to_string(),
        event_name: name.to_string(),
        current_screen: event.current_screen,
        current_screen_index: Some(event.current_screen),
        current_screen_key: None,
        screen_source: Some("rust_run_state".to_string()),
        options: options
            .iter()
            .enumerate()
            .map(|(index, option)| local_event_option_view(index, option))
            .collect(),
        features: derive_event_features(rs),
    }
}

fn local_event_option_view(index: usize, option: &EventOption) -> EventOptionView {
    let text = option.ui.text.clone();
    let label = extract_bracket_label(&text);
    let (semantic_tags, payload) = if semantics_available(option) {
        (
            classify_structured_option_tags(option),
            parse_structured_option_payload(option),
        )
    } else {
        (
            classify_event_option_tags(&text, label.as_deref()),
            parse_event_option_payload(&text, label.as_deref()),
        )
    };
    EventOptionView {
        index,
        text,
        label,
        disabled: option.ui.disabled,
        choice_index: Some(index),
        semantic_tags,
        payload,
    }
}

fn semantics_available(option: &EventOption) -> bool {
    option.semantics.action != EventActionKind::Unknown
        || !option.semantics.effects.is_empty()
        || !option.semantics.constraints.is_empty()
        || option.semantics.transition != crate::state::events::EventOptionTransition::None
        || option.semantics.repeatable
        || option.semantics.terminal
}

fn choose_structured_live_event_choice(gs: &Value, rs: &RunState) -> Option<EventChoiceDecision> {
    let screen_state = gs.get("screen_state")?;
    let live_options = screen_state.get("options").and_then(|v| v.as_array())?;
    let (event_state, reconstructed) =
        match crate::engine::event_handler::analyze_live_event_rebuild(rs, screen_state) {
            crate::engine::event_handler::LiveEventRebuildResult::Ready {
                event_state,
                options,
                ..
            } => (event_state, options),
            _ => return None,
        };

    let mut decision = choose_structured_local_event_choice(rs, &event_state, &reconstructed)?;
    decision.command_index = live_option_command_index(live_options, decision.option_index)?;
    Some(decision)
}

fn live_option_command_index(live_options: &[Value], option_index: usize) -> Option<usize> {
    let option = live_options.get(option_index)?;
    if option
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return None;
    }
    if let Some(choice_index) = option.get("choice_index").and_then(|v| v.as_u64()) {
        return Some(choice_index as usize);
    }
    Some(
        live_options
            .iter()
            .take(option_index + 1)
            .filter(|option| {
                !option
                    .get("disabled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count()
            .saturating_sub(1),
    )
}

pub fn live_event_context(gs: &Value, rs: &RunState) -> Option<EventDecisionContext> {
    let screen_state = gs.get("screen_state")?;
    let options = screen_state.get("options").and_then(|v| v.as_array())?;
    Some(EventDecisionContext {
        event_id: screen_state
            .get("event_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        event_name: screen_state
            .get("event_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        current_screen: screen_state
            .get("current_screen_index")
            .or_else(|| screen_state.get("current_screen"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        current_screen_index: screen_state
            .get("current_screen_index")
            .or_else(|| screen_state.get("current_screen"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        current_screen_key: screen_state
            .get("current_screen_key")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        screen_source: screen_state
            .get("screen_source")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        options: options
            .iter()
            .enumerate()
            .map(|(index, option)| {
                let text = option
                    .get("text")
                    .and_then(|v| v.as_str())
                    .or_else(|| option.get("label").and_then(|v| v.as_str()))
                    .unwrap_or("")
                    .to_string();
                let label = option
                    .get("label")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| extract_bracket_label(&text));
                EventOptionView {
                    index,
                    text: text.clone(),
                    label: label.clone(),
                    disabled: option
                        .get("disabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    choice_index: option
                        .get("choice_index")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as usize),
                    semantic_tags: classify_event_option_tags(&text, label.as_deref()),
                    payload: parse_event_option_payload(&text, label.as_deref()),
                }
            })
            .collect(),
        features: derive_event_features(rs),
    })
}

pub fn choose_event_option(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Option<EventChoiceDecision> {
    let family = classify_event_family(context);
    let scores = score_event_options(rs, context, family);
    if let Some(decision) = choose_best_scored_option(context, family, &scores) {
        return Some(decision);
    }

    if family == EventPolicyFamily::CompatibilityFallback
        && supports_legacy_compatibility_fallback(context)
    {
        if let Some(decision) = compatibility_fallback_event_choice(rs, context) {
            return Some(decision);
        }
    }

    Some(EventChoiceDecision {
        option_index: fallback_option_index(context)?,
        command_index: option_command_index(context, fallback_option_index(context)?)?,
        family: EventPolicyFamily::GenericSafe,
        rationale_key: Some("generic_safe_first_enabled"),
        score: None,
        safety_override_applied: false,
        rationale: Some("generic_safe_first_enabled"),
        deck_improvement_assessment: None,
    })
}

fn choose_structured_local_event_choice(
    rs: &RunState,
    event: &EventState,
    options: &[EventOption],
) -> Option<EventChoiceDecision> {
    let features = derive_event_features(rs);
    let family = match event.id {
        EventId::GoldenIdol
        | EventId::Ghosts
        | EventId::Vampires
        | EventId::WindingHalls
        | EventId::ForgottenAltar
        | EventId::MindBloom => EventPolicyFamily::CostTradeoff,
        EventId::Falling => EventPolicyFamily::DeckSurgery,
        EventId::CursedTome => EventPolicyFamily::PressYourLuck,
        EventId::WeMeetAgain | EventId::Designer | EventId::WomanInBlue | EventId::Cleric => {
            EventPolicyFamily::ResourceShoplike
        }
        _ => return None,
    };

    let scored = match event.id {
        EventId::GoldenIdol => structured_golden_idol_scores(rs, &features, options),
        EventId::Ghosts => structured_ghosts_scores(rs, &features, options),
        EventId::Vampires => structured_vampires_scores(rs, options),
        EventId::WindingHalls => structured_winding_halls_scores(rs, &features, options),
        EventId::ForgottenAltar => structured_forgotten_altar_scores(rs, &features, options),
        EventId::MindBloom => structured_mind_bloom_scores(rs, &features, options),
        EventId::Falling => structured_falling_scores(rs, options),
        EventId::CursedTome => structured_cursed_tome_scores(rs, event, options),
        EventId::WeMeetAgain => structured_we_meet_again_scores(rs, options),
        EventId::Designer => structured_designer_scores(rs, &features, options),
        EventId::WomanInBlue => structured_woman_in_blue_scores(rs, &features, options),
        EventId::Cleric => structured_cleric_scores(rs, options),
        _ => Vec::new(),
    };

    choose_best_structured_option(rs, options, family, &scored)
}

#[derive(Clone, Copy, Debug)]
struct StructuredOptionScore {
    option_index: usize,
    total_score: f32,
    rationale_key: &'static str,
}

fn choose_best_structured_option(
    rs: &RunState,
    options: &[EventOption],
    family: EventPolicyFamily,
    scores: &[StructuredOptionScore],
) -> Option<EventChoiceDecision> {
    let best = scores
        .iter()
        .filter(|score| {
            options
                .get(score.option_index)
                .is_some_and(|option| structured_option_viable(rs, option))
        })
        .max_by(|left, right| left.total_score.total_cmp(&right.total_score))?;
    Some(EventChoiceDecision {
        option_index: best.option_index,
        command_index: best.option_index,
        family,
        rationale_key: Some(best.rationale_key),
        score: Some(best.total_score.round() as i32),
        safety_override_applied: false,
        rationale: Some(best.rationale_key),
        deck_improvement_assessment: None,
    })
}

fn structured_option_viable(rs: &RunState, option: &EventOption) -> bool {
    if option.ui.disabled {
        return false;
    }
    if structured_hp_cost(option) >= rs.current_hp && structured_hp_cost(option) > 0 {
        return false;
    }
    for constraint in &option.semantics.constraints {
        match constraint {
            EventOptionConstraint::RequiresGold(amount) if rs.gold < *amount => return false,
            EventOptionConstraint::RequiresRelic(relic_id)
                if !rs.relics.iter().any(|relic| relic.id == *relic_id) =>
            {
                return false;
            }
            EventOptionConstraint::RequiresPotion
                if !rs.potions.iter().any(|slot| slot.is_some()) =>
            {
                return false;
            }
            EventOptionConstraint::RequiresRemovableCard if count_remove_targets(rs) <= 0 => {
                return false;
            }
            EventOptionConstraint::RequiresUpgradeableCard if count_upgradable_cards(rs) <= 0 => {
                return false;
            }
            EventOptionConstraint::RequiresTransformableCard
                if count_transform_targets(rs) <= 0 =>
            {
                return false;
            }
            _ => {}
        }
    }
    true
}

fn structured_base_option_score(rs: &RunState, option: &EventOption) -> f32 {
    let mut score = 0.0;
    for effect in &option.semantics.effects {
        match effect {
            EventEffect::GainGold(amount) => {
                let effective = if gold_gain_blocked(rs) { 0 } else { *amount };
                score += effective as f32 * gold_value_per_gold(rs);
            }
            EventEffect::LoseGold(amount) => score -= *amount as f32 * gold_value_per_gold(rs),
            EventEffect::LoseHp(amount) => score -= *amount as f32 * hp_point_value(rs),
            EventEffect::LoseMaxHp(amount) => score -= *amount as f32 * 90.0,
            EventEffect::Heal(amount) => score += *amount as f32 * hp_point_value(rs) * 0.5,
            EventEffect::GainMaxHp(amount) => score += *amount as f32 * 110.0,
            EventEffect::ObtainCurse { count, .. } => {
                score -= *count as f32 * (640.0 + context_curse_drag(rs))
            }
            EventEffect::LoseStarterRelic { .. } => score -= 1_800.0,
            _ => {}
        }
    }
    score
}

fn structured_hp_cost(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::LoseHp(amount) => *amount,
            _ => 0,
        })
        .sum()
}

fn structured_max_hp_cost(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::LoseMaxHp(amount) => *amount,
            _ => 0,
        })
        .sum()
}

fn structured_heal_amount(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::Heal(amount) => *amount,
            _ => 0,
        })
        .sum()
}

fn structured_gold_loss(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::LoseGold(amount) => *amount,
            _ => 0,
        })
        .sum()
}

fn structured_gold_gain(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::GainGold(amount) => *amount,
            _ => 0,
        })
        .sum()
}

fn structured_potion_count(option: &EventOption) -> usize {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::ObtainPotion { count } => *count,
            _ => 0,
        })
        .sum()
}

fn structured_curse_count(option: &EventOption) -> i32 {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::ObtainCurse { count, .. } => *count as i32,
            _ => 0,
        })
        .sum()
}

fn structured_has_action(option: &EventOption, action: EventActionKind) -> bool {
    option.semantics.action == action
}

fn structured_has_transition(option: &EventOption, transition: EventOptionTransition) -> bool {
    option.semantics.transition == transition
}

fn structured_has_constraint(
    option: &EventOption,
    predicate: impl Fn(&EventOptionConstraint) -> bool,
) -> bool {
    option.semantics.constraints.iter().any(predicate)
}

fn structured_has_relic_kind(option: &EventOption, kind: EventRelicKind) -> bool {
    option.semantics.effects.iter().any(|effect| match effect {
        EventEffect::ObtainRelic {
            kind: effect_kind, ..
        } => *effect_kind == kind,
        _ => false,
    })
}

fn structured_has_specific_relic_loss(
    option: &EventOption,
    relic_id: crate::content::relics::RelicId,
) -> bool {
    option.semantics.effects.iter().any(|effect| match effect {
        EventEffect::LoseRelic { specific, .. } => *specific == Some(relic_id),
        EventEffect::LoseStarterRelic { specific } => *specific == Some(relic_id),
        _ => false,
    })
}

fn structured_specific_card_count(
    option: &EventOption,
    card_id: crate::content::cards::CardId,
) -> usize {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::ObtainCard {
                count,
                kind: EventCardKind::Specific(effect_card),
            }
            | EventEffect::ObtainColorlessCard {
                count,
                kind: EventCardKind::Specific(effect_card),
            }
            | EventEffect::ObtainCurse {
                count,
                kind: EventCardKind::Specific(effect_card),
            } if *effect_card == card_id => *count,
            _ => 0,
        })
        .sum()
}

fn structured_upgrade_count(option: &EventOption) -> usize {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::UpgradeCard { count } => *count,
            _ => 0,
        })
        .sum()
}

fn structured_transform_count(option: &EventOption) -> usize {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::TransformCard { count } => *count,
            _ => 0,
        })
        .sum()
}

fn structured_remove_count(option: &EventOption) -> usize {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| match effect {
            EventEffect::RemoveCard { count, .. } => *count,
            _ => 0,
        })
        .sum()
}

fn structured_remove_target_uuid(option: &EventOption) -> Option<u32> {
    option
        .semantics
        .effects
        .iter()
        .find_map(|effect| match effect {
            EventEffect::RemoveCard { target_uuid, .. } => *target_uuid,
            _ => None,
        })
}

fn structured_has_random_book_relic(option: &EventOption) -> bool {
    structured_has_relic_kind(option, EventRelicKind::RandomBook)
}

fn structured_golden_idol_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_has_relic_kind(
                option,
                EventRelicKind::Specific(crate::content::relics::RelicId::GoldenIdol),
            ) {
                if gold_gain_blocked(rs) {
                    score -= 2_400.0;
                    "cost_tradeoff_avoid_ectoplasm_gold_relic"
                } else {
                    score += golden_idol_future_value(rs);
                    "cost_tradeoff_take_relic"
                }
            } else if structured_max_hp_cost(option) > 0 {
                score += 120.0 + (1.0 - features.hp_ratio).max(0.0) * 180.0;
                "cost_tradeoff_preserve_current_hp"
            } else if structured_curse_count(option) > 0 {
                score += curse_mitigation_bonus(rs, structured_curse_count(option).max(1));
                score -= 160.0 + features.curse_pressure as f32 * 16.0;
                "cost_tradeoff_accept_curse_over_hp_loss"
            } else if structured_hp_cost(option) > 0
                || structured_has_action(option, EventActionKind::Fight)
            {
                score += if features.hp_ratio >= 0.75 {
                    180.0
                } else if features.hp_ratio >= 0.60 {
                    40.0
                } else {
                    -260.0
                };
                "cost_tradeoff_pay_hp_keep_max_hp"
            } else {
                "cost_tradeoff_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_ghosts_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_specific_card_count(
                option,
                crate::content::cards::CardId::Apparition,
            ) > 0
            {
                let route_support = ghosts_route_support(rs);
                let late_act_without_support = rs.act_num >= 3
                    && (route_support < 6 || (route_support < 8 && features.hp_ratio < 0.55));
                if late_act_without_support {
                    score = score.min(-900.0);
                    "cost_tradeoff_refuse_apparitions"
                } else {
                    let count = structured_specific_card_count(
                        option,
                        crate::content::cards::CardId::Apparition,
                    );
                    let apparition_delta = crate::bot::deck_delta_eval::compare_pick_vs_skip(
                        rs,
                        crate::content::cards::CardId::Apparition,
                    );
                    let early_act_bonus = if rs.act_num <= 2 { 280.0 } else { -1_500.0 };
                    score += count as f32 * (80.0 + apparition_delta.total as f32 * 6.0)
                        + route_support as f32 * 60.0
                        + early_act_bonus;
                    if score > 0.0 {
                        "cost_tradeoff_accept_apparitions"
                    } else {
                        "cost_tradeoff_refuse_apparitions"
                    }
                }
            } else {
                "cost_tradeoff_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_cursed_tome_scores(
    rs: &RunState,
    event: &EventState,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    let book_value = cursed_tome_book_relic_value(rs);
    let continuation_drag = cursed_tome_remaining_commitment_damage(rs, event.current_screen)
        as f32
        * hp_point_value(rs)
        * 0.82;
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_has_random_book_relic(option)
                && structured_has_transition(option, EventOptionTransition::OpenReward)
            {
                score += book_value;
                "press_your_luck_take_book_relic"
            } else if structured_has_action(option, EventActionKind::Continue) {
                score += book_value - continuation_drag;
                "press_your_luck_continue_for_book"
            } else {
                "press_your_luck_stop"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_we_meet_again_scores(
    rs: &RunState,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_has_constraint(option, |constraint| {
                matches!(constraint, EventOptionConstraint::RequiresPotion)
            }) {
                score += best_we_meet_again_potion_give_score(rs) as f32;
                "resource_shoplike_trade_potion_for_relic"
            } else if structured_gold_loss(option) > 0 {
                score += 1_450.0
                    - structured_gold_loss(option) as f32 * 18.0
                    - nearby_shop_conversion_bonus(rs) as f32;
                "resource_shoplike_trade_gold_for_relic"
            } else if option
                .semantics
                .effects
                .iter()
                .any(|effect| matches!(effect, EventEffect::RemoveCard { .. }))
            {
                score += best_we_meet_again_card_give_score(rs) as f32;
                "resource_shoplike_trade_card_for_relic"
            } else if structured_has_action(option, EventActionKind::Decline) {
                "resource_shoplike_decline_trade"
            } else {
                "resource_shoplike_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_winding_halls_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key =
                if structured_specific_card_count(option, crate::content::cards::CardId::Madness)
                    >= 2
                {
                    score += madness_package_value(rs, 2);
                    "cost_tradeoff_take_madness"
                } else if structured_specific_card_count(
                    option,
                    crate::content::cards::CardId::Writhe,
                ) > 0
                {
                    score += structured_heal_amount(option) as f32 * hp_point_value(rs) * 0.35;
                    score += curse_mitigation_bonus(rs, structured_curse_count(option).max(1));
                    score -= 120.0 + features.curse_pressure as f32 * 10.0;
                    "cost_tradeoff_heal_with_curse"
                } else if structured_max_hp_cost(option) > 0 {
                    score += (rs.max_hp - rs.current_hp).max(0) as f32 * hp_point_value(rs) * 0.08;
                    "cost_tradeoff_take_max_hp"
                } else {
                    "cost_tradeoff_safe_leave"
                };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_vampires_scores(
    rs: &RunState,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_has_specific_relic_loss(
                option,
                crate::content::relics::RelicId::BloodVial,
            ) {
                score += vampires_exchange_value(rs) - blood_vial_value(rs);
                "cost_tradeoff_trade_relic_for_vampires"
            } else if structured_specific_card_count(option, crate::content::cards::CardId::Bite)
                >= 5
            {
                score += vampires_exchange_value(rs);
                "cost_tradeoff_accept_vampires"
            } else {
                "cost_tradeoff_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_forgotten_altar_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_has_relic_kind(
                option,
                EventRelicKind::Specific(crate::content::relics::RelicId::BloodyIdol),
            ) {
                let value = if gold_gain_blocked(rs) {
                    relic_equity_value(rs) * 0.35
                } else {
                    relic_equity_value(rs) * 0.85 + 240.0 + golden_idol_future_value(rs) * 0.25
                };
                score += value;
                "cost_tradeoff_take_relic"
            } else if option
                .semantics
                .effects
                .iter()
                .any(|effect| matches!(effect, EventEffect::GainMaxHp(5)))
            {
                score += 5.0 * hp_point_value(rs) * 0.30;
                if features.hp_ratio < 0.50 {
                    score -= 220.0;
                }
                "cost_tradeoff_take_max_hp"
            } else if structured_specific_card_count(option, crate::content::cards::CardId::Decay)
                > 0
            {
                score += curse_mitigation_bonus(rs, structured_curse_count(option).max(1));
                score -= 80.0;
                "cost_tradeoff_accept_curse_over_hp_loss"
            } else {
                "cost_tradeoff_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_mind_bloom_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let support_score = profile.block_core * 2
        + profile.block_payoffs
        + profile.draw_sources
        + profile.power_scalers
        + profile.attack_count.min(6);

    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key =
                if structured_has_transition(option, EventOptionTransition::StartCombat)
                    || structured_has_action(option, EventActionKind::Fight)
                {
                    let reward_value = relic_equity_value(rs) * 1.10
                        + if rs.ascension_level >= 13 {
                            25.0 * gold_value_per_gold(rs)
                        } else {
                            50.0 * gold_value_per_gold(rs)
                        };
                    let risk_value =
                        hp_point_value(rs) * (15.0 - support_score as f32 * 0.45).max(7.5);
                    score += reward_value - risk_value - safety_gap_penalty(rs, 20);
                    "cost_tradeoff_fight_for_relic"
                } else if structured_has_relic_kind(
                    option,
                    EventRelicKind::Specific(crate::content::relics::RelicId::MarkOfTheBloom),
                ) {
                    let upgrade_count = structured_upgrade_count(option);
                    let upgrade_value =
                        upgrade_count.min(features.upgradable_cards as usize) as f32 * 150.0;
                    let mark_penalty = 520.0
                        + (rs.max_hp - rs.current_hp).max(0) as f32 * hp_point_value(rs) * 0.65
                        + match features.rest_distance {
                            Some(1) => 220.0,
                            Some(2) => 140.0,
                            Some(3) => 80.0,
                            _ => 40.0,
                        };
                    score += upgrade_value - mark_penalty;
                    if score >= 0.0 {
                        "cost_tradeoff_take_mark_of_the_bloom"
                    } else {
                        "cost_tradeoff_avoid_mark_of_the_bloom"
                    }
                } else if structured_specific_card_count(
                    option,
                    crate::content::cards::CardId::Normality,
                ) >= 2
                {
                    let raw_gold = structured_gold_gain(option);
                    let spend_cap = match features.shop_distance {
                        Some(1) => 320,
                        Some(2) => 260,
                        Some(3) => 220,
                        _ => 160,
                    };
                    let excess_gold =
                        (raw_gold - spend_cap).max(0) as f32 * gold_value_per_gold(rs);
                    score -= excess_gold;
                    score -= 480.0;
                    "cost_tradeoff_accept_curse_for_gold"
                } else if structured_specific_card_count(
                    option,
                    crate::content::cards::CardId::Doubt,
                ) > 0
                {
                    score += (rs.max_hp - rs.current_hp).max(0) as f32 * hp_point_value(rs) * 0.45;
                    score -= 60.0;
                    "cost_tradeoff_heal_with_curse"
                } else {
                    "cost_tradeoff_safe_leave"
                };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_falling_scores(rs: &RunState, options: &[EventOption]) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if let Some(target_uuid) = structured_remove_target_uuid(option) {
                if let Some(candidate) =
                    crate::bot::run_deck_improvement::evaluate_remove_target(rs, target_uuid)
                {
                    score += candidate.score as f32;
                    candidate.rationale_key
                } else {
                    "deck_surgery_safe_leave"
                }
            } else if structured_has_action(option, EventActionKind::Decline) {
                "generic_safe_leave"
            } else {
                "deck_surgery_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_designer_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_remove_count(option) > 0
                && structured_upgrade_count(option) > 0
            {
                let remove_delta = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs);
                let upgrade_delta = crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, 1);
                score += 720.0
                    + features.remove_targets as f32 * 110.0
                    + remove_delta.total as f32 * 13.0
                    + 480.0
                    + features.upgradable_cards as f32 * 85.0
                    + upgrade_delta.total as f32 * 9.0;
                "resource_shoplike_full_service"
            } else if structured_remove_count(option) > 0 {
                let delta = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs);
                score += 720.0 + features.remove_targets as f32 * 120.0 + delta.total as f32 * 13.0;
                "resource_shoplike_buy_removal"
            } else if structured_transform_count(option) > 0 {
                let count = structured_transform_count(option);
                let delta =
                    crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, count, false);
                score += 700.0
                    + features.transform_targets as f32 * if count >= 2 { 120.0 } else { 90.0 }
                    + delta.total as f32 * 12.0;
                "resource_shoplike_buy_transform"
            } else if structured_upgrade_count(option) > 0 {
                let count = structured_upgrade_count(option);
                let delta = crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, count);
                score += 760.0
                    + features.upgradable_cards as f32 * if count >= 2 { 110.0 } else { 125.0 }
                    + delta.total as f32 * if count >= 2 { 11.0 } else { 13.0 };
                "resource_shoplike_buy_upgrade"
            } else if structured_hp_cost(option) > 0 {
                "resource_shoplike_decline_service"
            } else {
                "resource_shoplike_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_woman_in_blue_scores(
    rs: &RunState,
    features: &EventDecisionFeatures,
    options: &[EventOption],
) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_potion_count(option) > 0 {
                score += random_potion_offer_value(
                    rs,
                    features.empty_potion_slots,
                    features.potion_blocked,
                    structured_potion_count(option),
                    5.8,
                    1.9,
                    -240.0,
                );
                "resource_shoplike_buy_potion"
            } else if structured_has_action(option, EventActionKind::Leave) {
                "resource_shoplike_safe_leave"
            } else {
                "resource_shoplike_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn structured_cleric_scores(rs: &RunState, options: &[EventOption]) -> Vec<StructuredOptionScore> {
    options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let mut score = structured_base_option_score(rs, option);
            let rationale_key = if structured_remove_count(option) > 0 {
                let delta = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs);
                score +=
                    720.0 + count_remove_targets(rs) as f32 * 120.0 + delta.total as f32 * 13.0;
                "deck_surgery_remove_best"
            } else if structured_heal_amount(option) > 0 {
                score += structured_heal_amount(option) as f32 * hp_point_value(rs) * 0.6;
                "resource_shoplike_buy_heal"
            } else {
                "resource_shoplike_safe_leave"
            };
            StructuredOptionScore {
                option_index: index,
                total_score: score,
                rationale_key,
            }
        })
        .collect()
}

fn supports_legacy_compatibility_fallback(context: &EventDecisionContext) -> bool {
    matches!(
        canonical_family_event_name(context).as_str(),
        "Neow" | "Note For Yourself" | "Bonfire Spirits" | "Bonfire Elementals"
    )
}

fn legacy_fallback_rationale_key(context: &EventDecisionContext) -> &'static str {
    match canonical_family_event_name(context).as_str() {
        "Neow" => "legacy_fallback_neow",
        "Note For Yourself" => "legacy_fallback_note_for_yourself",
        "Bonfire Spirits" | "Bonfire Elementals" => "legacy_fallback_bonfire",
        _ => "legacy_fallback_adapter",
    }
}

fn compatibility_fallback_event_choice(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Option<EventChoiceDecision> {
    let screen_state = json!({
        "event_id": context.event_id,
        "event_name": context.event_name,
        "current_screen": context.current_screen,
        "options": context.options.iter().map(|option| {
            json!({
                "text": option.text,
                "label": option.label,
                "disabled": option.disabled,
                "choice_index": option.choice_index,
            })
        }).collect::<Vec<_>>(),
    });
    let root = json!({ "screen_state": screen_state });
    let choice_names = derived_choice_names(context);
    let choice_refs = choice_names.iter().map(String::as_str).collect::<Vec<_>>();
    let command_index =
        crate::bot::noncombat_families::choose_event_choice(&root, rs, &choice_refs)
            .or_else(|| fallback_command_index(context))?;
    let option_index = command_index_to_option_index(context, command_index)
        .or_else(|| fallback_option_index(context))?;
    let rationale_key = legacy_fallback_rationale_key(context);
    Some(EventChoiceDecision {
        option_index,
        command_index,
        family: EventPolicyFamily::CompatibilityFallback,
        rationale_key: Some(rationale_key),
        score: None,
        safety_override_applied: false,
        rationale: Some(rationale_key),
        deck_improvement_assessment: None,
    })
}

pub fn describe_choice(context: &EventDecisionContext, decision: &EventChoiceDecision) -> String {
    let option = context.options.get(decision.option_index);
    let label = option
        .and_then(|opt| opt.label.as_deref())
        .or_else(|| option.map(|opt| opt.text.as_str()))
        .unwrap_or("unknown");
    let family = family_label(decision.family);
    let screen_label = screen_debug_label(context);
    match (decision.rationale_key, decision.score) {
        (Some(rationale), Some(score)) => format!(
            "{} [{}] -> {} (family={} rationale={} score={})",
            if context.event_name.is_empty() {
                "Event"
            } else {
                context.event_name.as_str()
            },
            screen_label,
            label,
            family,
            rationale,
            score
        ),
        (Some(rationale), None) => format!(
            "{} [{}] -> {} (family={} rationale={})",
            if context.event_name.is_empty() {
                "Event"
            } else {
                context.event_name.as_str()
            },
            screen_label,
            label,
            family,
            rationale
        ),
        (None, Some(score)) => format!(
            "{} [{}] -> {} (family={} score={})",
            if context.event_name.is_empty() {
                "Event"
            } else {
                context.event_name.as_str()
            },
            screen_label,
            label,
            family,
            score
        ),
        (None, None) => format!(
            "{} [{}] -> {} (family={})",
            if context.event_name.is_empty() {
                "Event"
            } else {
                context.event_name.as_str()
            },
            screen_label,
            label,
            family
        ),
    }
}

pub fn family_label(family: EventPolicyFamily) -> &'static str {
    match family {
        EventPolicyFamily::DeckSurgery => "deck_surgery",
        EventPolicyFamily::PressYourLuck => "press_your_luck",
        EventPolicyFamily::CostTradeoff => "cost_tradeoff",
        EventPolicyFamily::ResourceShoplike => "resource_shoplike",
        EventPolicyFamily::GenericSafe => "generic_safe",
        EventPolicyFamily::CompatibilityFallback => "compatibility_fallback",
    }
}

pub fn compact_choice_summary(
    context: &EventDecisionContext,
    decision: &EventChoiceDecision,
) -> String {
    let option = context.options.get(decision.option_index);
    let label = option
        .and_then(|opt| opt.label.as_deref())
        .or_else(|| option.map(|opt| opt.text.as_str()))
        .unwrap_or("unknown");
    let screen_label = screen_compact_label(context);
    match (decision.rationale_key, decision.score) {
        (Some(rationale), Some(score)) => format!(
            "{} {} -> {} | family={} rationale={} score={}",
            context.event_name,
            screen_label,
            label,
            family_label(decision.family),
            rationale,
            score
        ),
        (Some(rationale), None) => format!(
            "{} {} -> {} | family={} rationale={}",
            context.event_name,
            screen_label,
            label,
            family_label(decision.family),
            rationale
        ),
        _ => format!(
            "{} {} -> {} | family={}",
            context.event_name,
            screen_label,
            label,
            family_label(decision.family)
        ),
    }
}

pub fn decision_trace_json(
    context: &EventDecisionContext,
    decision: &EventChoiceDecision,
) -> Value {
    let option = context.options.get(decision.option_index);
    json!({
        "event_name": context.event_name,
        "event_id": context.event_id,
        "screen": context.current_screen,
        "screen_index": context.current_screen_index,
        "screen_key": context.current_screen_key,
        "screen_source": context.screen_source,
        "chosen_option_index": decision.option_index,
        "command_index": decision.command_index,
        "chosen_option_text": option.map(|opt| opt.text.clone()),
        "chosen_option_label": option.and_then(|opt| opt.label.clone()),
        "family": family_label(decision.family),
        "rationale_key": decision.rationale_key,
        "score": decision.score,
        "safety_override_applied": decision.safety_override_applied,
        "deck_improvement_assessment": decision
            .deck_improvement_assessment
            .as_ref()
            .map(crate::bot::run_deck_improvement::deck_operation_assessment_json),
    })
}

fn screen_compact_label(context: &EventDecisionContext) -> String {
    match (&context.current_screen_key, context.current_screen_index) {
        (Some(key), Some(index)) => format!("s{}[{}]", index, key),
        (Some(key), None) => format!("s[{}]", key),
        (None, Some(index)) => format!("s{}", index),
        (None, None) => "s?".to_string(),
    }
}

fn screen_debug_label(context: &EventDecisionContext) -> String {
    match (
        &context.current_screen_key,
        context.current_screen_index,
        context.screen_source.as_deref(),
    ) {
        (Some(key), Some(index), Some(source)) => {
            format!("screen {} [{}] source={}", index, key, source)
        }
        (Some(key), Some(index), None) => format!("screen {} [{}]", index, key),
        (Some(key), None, Some(source)) => format!("screen [{}] source={}", key, source),
        (Some(key), None, None) => format!("screen [{}]", key),
        (None, Some(index), Some(source)) => format!("screen {} source={}", index, source),
        (None, Some(index), None) => format!("screen {}", index),
        (None, None, Some(source)) => format!("screen ? source={}", source),
        (None, None, None) => "screen ?".to_string(),
    }
}

fn derived_choice_names(context: &EventDecisionContext) -> Vec<String> {
    let max_command = context
        .options
        .iter()
        .enumerate()
        .filter_map(|(index, _)| option_command_index(context, index))
        .max();
    let Some(max_command) = max_command else {
        return Vec::new();
    };
    let mut names = vec![String::new(); max_command + 1];
    for (index, option) in context.options.iter().enumerate() {
        if option.disabled {
            continue;
        }
        if let Some(command_index) = option_command_index(context, index) {
            names[command_index] = option
                .label
                .clone()
                .unwrap_or_else(|| option.text.clone())
                .to_ascii_lowercase();
        }
    }
    names
}

fn option_command_index(context: &EventDecisionContext, option_index: usize) -> Option<usize> {
    let option = context.options.get(option_index)?;
    if option.disabled {
        return None;
    }
    if let Some(choice_index) = option.choice_index {
        return Some(choice_index);
    }
    Some(
        context
            .options
            .iter()
            .take(option_index + 1)
            .filter(|option| !option.disabled)
            .count()
            .saturating_sub(1),
    )
}

fn command_index_to_option_index(
    context: &EventDecisionContext,
    command_index: usize,
) -> Option<usize> {
    context
        .options
        .iter()
        .enumerate()
        .find(|(index, _)| option_command_index(context, *index) == Some(command_index))
        .map(|(index, _)| index)
}

fn fallback_option_index(context: &EventDecisionContext) -> Option<usize> {
    context
        .options
        .iter()
        .position(|option| !option.disabled)
        .or_else(|| (!context.options.is_empty()).then_some(0))
}

fn fallback_command_index(context: &EventDecisionContext) -> Option<usize> {
    fallback_option_index(context).and_then(|index| option_command_index(context, index))
}

fn extract_bracket_label(text: &str) -> Option<String> {
    let start = text.find('[')?;
    let end = text[start + 1..].find(']')?;
    let label = text[start + 1..start + 1 + end].trim();
    (!label.is_empty()).then(|| label.to_string())
}

fn classify_structured_option_tags(option: &EventOption) -> Vec<EventOptionTag> {
    let mut tags = Vec::new();
    match option.semantics.action {
        EventActionKind::Leave | EventActionKind::Decline => tags.push(EventOptionTag::Leave),
        EventActionKind::Continue => tags.push(EventOptionTag::Continue),
        EventActionKind::Fight => tags.push(EventOptionTag::Fight),
        _ => {}
    }
    for effect in &option.semantics.effects {
        match effect {
            EventEffect::GainGold(_) => tags.push(EventOptionTag::GainGold),
            EventEffect::LoseGold(_) => tags.push(EventOptionTag::LoseGold),
            EventEffect::LoseHp(_) => tags.push(EventOptionTag::LoseHp),
            EventEffect::LoseMaxHp(_) => tags.push(EventOptionTag::LoseMaxHp),
            EventEffect::Heal(_) => {
                tags.push(EventOptionTag::GainHp);
                tags.push(EventOptionTag::Heal);
            }
            EventEffect::GainMaxHp(_) => tags.push(EventOptionTag::GainMaxHp),
            EventEffect::ObtainRelic { .. } => {
                tags.push(EventOptionTag::TakeRelic);
                tags.push(EventOptionTag::ObtainRelic);
            }
            EventEffect::ObtainPotion { .. } => tags.push(EventOptionTag::ObtainPotion),
            EventEffect::ObtainCard { .. } => {
                tags.push(EventOptionTag::ObtainCard);
                tags.push(EventOptionTag::ObtainRandomCard);
            }
            EventEffect::ObtainColorlessCard { .. } => {
                tags.push(EventOptionTag::ObtainColorlessCard);
                tags.push(EventOptionTag::ObtainCard);
            }
            EventEffect::ObtainCurse { .. } => tags.push(EventOptionTag::ObtainCurse),
            EventEffect::RemoveCard { .. } => tags.push(EventOptionTag::Remove),
            EventEffect::UpgradeCard { .. } => tags.push(EventOptionTag::Upgrade),
            EventEffect::TransformCard { .. } => tags.push(EventOptionTag::Transform),
            EventEffect::DuplicateCard { .. } => tags.push(EventOptionTag::Duplicate),
            EventEffect::LoseRelic { .. } => tags.push(EventOptionTag::LoseRelic),
            EventEffect::LoseStarterRelic { .. } => {
                tags.push(EventOptionTag::LoseStarterRelic);
                tags.push(EventOptionTag::LoseRelic);
            }
            EventEffect::StartCombat => tags.push(EventOptionTag::Fight),
        }
    }
    let mut unique = Vec::new();
    for tag in tags {
        if !unique.contains(&tag) {
            unique.push(tag);
        }
    }
    unique
}

fn classify_event_option_tags(text: &str, label: Option<&str>) -> Vec<EventOptionTag> {
    let merged = format!("{} {}", text, label.unwrap_or("")).to_ascii_lowercase();
    let mut tags = Vec::new();
    if merged.contains("continue") {
        tags.push(EventOptionTag::Continue);
    }
    if merged.contains("relic") || merged.contains("idol") {
        tags.push(EventOptionTag::TakeRelic);
        tags.push(EventOptionTag::ObtainRelic);
    }
    if merged.contains("potion") {
        tags.push(EventOptionTag::ObtainPotion);
    }
    if merged.contains("colorless card") {
        tags.push(EventOptionTag::ObtainColorlessCard);
        tags.push(EventOptionTag::ObtainCard);
    } else if merged.contains("card") && !merged.contains("remove") && !merged.contains("upgrade") {
        tags.push(EventOptionTag::ObtainCard);
        if contains_any(&merged, &["obtain", "gain"]) {
            tags.push(EventOptionTag::ObtainRandomCard);
        }
    }
    if merged.contains("leave")
        || merged.contains("ignore")
        || merged.contains("refuse")
        || merged.contains("proceed")
    {
        tags.push(EventOptionTag::Leave);
    }
    if merged.contains("remove a card") || merged.contains("purge") || merged.contains("forget") {
        tags.push(EventOptionTag::Remove);
    }
    if merged.contains("upgrade") || merged.contains("grow") || merged.contains("smith") {
        tags.push(EventOptionTag::Upgrade);
    }
    if merged.contains("transform") || merged.contains("change") {
        tags.push(EventOptionTag::Transform);
    }
    if merged.contains("duplicate") || merged.contains("copy a card") {
        tags.push(EventOptionTag::Duplicate);
    }
    if merged.contains("gain max hp")
        || merged.contains("max hp +")
        || merged.contains("+") && merged.contains("max hp")
    {
        tags.push(EventOptionTag::GainMaxHp);
    }
    if merged.contains("lose max hp") || merged.contains("max hp") && merged.contains("lose") {
        tags.push(EventOptionTag::LoseMaxHp);
    }
    if merged.contains("heal") || merged.contains("heal to full") || merged.contains("banana") {
        tags.push(EventOptionTag::GainHp);
    }
    if merged.contains("damage") || merged.contains("lose hp") || merged.contains("take ") {
        tags.push(EventOptionTag::LoseHp);
    }
    if merged.contains("curse")
        || merged.contains("injury")
        || merged.contains("regret")
        || merged.contains("writhe")
        || merged.contains("parasite")
    {
        tags.push(EventOptionTag::ObtainCurse);
    }
    if merged.contains("gold") {
        if contains_any(&merged, &["lose gold", "give gold", "donate"]) {
            tags.push(EventOptionTag::LoseGold);
        } else {
            tags.push(EventOptionTag::GainGold);
        }
    }
    if merged.contains("lose your starter relic") || merged.contains("lose your starting relic") {
        tags.push(EventOptionTag::LoseStarterRelic);
        tags.push(EventOptionTag::LoseRelic);
    }
    if contains_any(
        &merged,
        &["give golden idol", "trade golden idol", "lose relic"],
    ) {
        tags.push(EventOptionTag::LoseRelic);
    }
    if merged.contains("fight") || merged.contains("attack") || merged.contains("stomp") {
        tags.push(EventOptionTag::Fight);
    }
    if merged.contains("read") {
        tags.push(EventOptionTag::Read);
    }
    if merged.contains("heal") || merged.contains("sleep") || merged.contains("banana") {
        tags.push(EventOptionTag::Heal);
    }
    tags
}

fn parse_structured_option_payload(option: &EventOption) -> EventOptionPayload {
    let mut payload = EventOptionPayload {
        repeatable: option.semantics.repeatable,
        ..EventOptionPayload::default()
    };
    for effect in &option.semantics.effects {
        match effect {
            EventEffect::GainGold(amount) => payload.gold_delta += *amount,
            EventEffect::LoseGold(amount) => payload.gold_delta -= *amount,
            EventEffect::LoseHp(amount) => payload.hp_cost += *amount,
            EventEffect::LoseMaxHp(amount) => payload.max_hp_cost += *amount,
            EventEffect::Heal(amount) => payload.heal_amount += *amount,
            EventEffect::GainMaxHp(amount) => payload.max_hp_gain += *amount,
            EventEffect::ObtainRelic { count, .. } => payload.relic_count += *count as i32,
            EventEffect::ObtainPotion { count } => payload.potion_count += *count as i32,
            EventEffect::ObtainCard { count, .. } => payload.card_count += *count as i32,
            EventEffect::ObtainColorlessCard { count, .. } => {
                payload.card_count += *count as i32;
                payload.colorless_card_count += *count as i32;
            }
            EventEffect::ObtainCurse { count, .. } => payload.curse_count += *count as i32,
            EventEffect::LoseStarterRelic { .. } => payload.lose_starter_relic = true,
            EventEffect::RemoveCard { .. }
            | EventEffect::UpgradeCard { .. }
            | EventEffect::TransformCard { .. }
            | EventEffect::DuplicateCard { .. }
            | EventEffect::LoseRelic { .. }
            | EventEffect::StartCombat => {}
        }
    }
    payload
}

fn parse_event_option_payload(text: &str, label: Option<&str>) -> EventOptionPayload {
    let merged = format!("{} {}", text, label.unwrap_or("")).to_ascii_lowercase();
    let numbers = extract_numbers(text);
    let first = numbers.first().copied().unwrap_or(0);
    let last = numbers.last().copied().unwrap_or(first);

    let mut payload = EventOptionPayload::default();
    payload.repeatable = contains_any(&merged, &["anything else", "reach in", "search"]);
    if merged.contains("gain") && merged.contains("gold") {
        payload.gold_delta = first;
    } else if contains_any(&merged, &["lose gold"]) {
        payload.gold_delta = -first.max(last);
    }
    if contains_any(&merged, &["lose max hp", "max hp"]) && merged.contains("lose") {
        payload.max_hp_cost = last.max(first);
    } else if contains_any(&merged, &["gain max hp", "max hp +", "+"]) && merged.contains("max hp")
    {
        payload.max_hp_gain = first.max(last);
    }
    let mentions_hp_loss = (merged.contains("lose") && merged.contains("hp"))
        || merged.contains("damage")
        || merged.contains("take ");
    if mentions_hp_loss && !contains_any(&merged, &["heal"]) {
        payload.hp_cost =
            if merged.contains("chance") && contains_any(&merged, &["damage", "lose hp"]) {
                first
            } else if merged.contains("gain") && merged.contains("gold") {
                last
            } else {
                if merged.contains("lose") && merged.contains("hp") && !merged.contains("gold") {
                    last
                } else {
                    first.max(last)
                }
            };
    }
    if contains_any(&merged, &["heal"]) {
        payload.heal_amount = first.max(last);
    }
    if contains_any(&merged, &["potion"]) {
        payload.potion_count = 1;
    }
    if contains_any(&merged, &["colorless card"]) {
        payload.card_count = 1;
        payload.colorless_card_count = 1;
    } else if contains_any(&merged, &["card"]) && !contains_any(&merged, &["remove", "upgrade"]) {
        payload.card_count = 1;
    }
    if contains_any(&merged, &["relic", "golden idol", "bloody idol"]) {
        payload.relic_count = 1;
    }
    if contains_any(
        &merged,
        &[
            "obtain a curse",
            "become cursed",
            "injury",
            "regret",
            "writhe",
            "parasite",
        ],
    ) {
        payload.curse_count = 1;
    }
    if contains_any(
        &merged,
        &["lose your starter relic", "lose your starting relic"],
    ) {
        payload.lose_starter_relic = true;
    }
    payload
}

fn derive_event_features(rs: &RunState) -> EventDecisionFeatures {
    EventDecisionFeatures {
        current_hp: rs.current_hp,
        max_hp: rs.max_hp,
        hp_ratio: rs.current_hp as f32 / rs.max_hp.max(1) as f32,
        gold: rs.gold,
        empty_potion_slots: rs.potions.iter().filter(|slot| slot.is_none()).count(),
        potion_blocked: rs
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::Sozu),
        rest_distance: reachable_room_distance(rs, RoomType::RestRoom, 3),
        shop_distance: reachable_room_distance(rs, RoomType::ShopRoom, 3),
        elite_distance: reachable_room_distance(rs, RoomType::MonsterRoomElite, 4),
        remove_targets: count_remove_targets(rs),
        transform_targets: count_transform_targets(rs),
        upgradable_cards: count_upgradable_cards(rs),
        curse_pressure: curse_pressure_score(rs),
        has_golden_idol: rs
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::GoldenIdol),
    }
}

fn classify_event_family(context: &EventDecisionContext) -> EventPolicyFamily {
    if matches!(
        canonical_family_event_name(context).as_str(),
        "Living Wall"
            | "Purification Shrine"
            | "Upgrade Shrine"
            | "Transmogrifier"
            | "Duplicator"
            | "Back to Basics"
            | "Falling"
    ) {
        EventPolicyFamily::DeckSurgery
    } else if matches!(
        canonical_family_event_name(context).as_str(),
        "Scrap Ooze" | "World of Goop" | "Dead Adventurer" | "Knowing Skull" | "Cursed Tome"
    ) {
        EventPolicyFamily::PressYourLuck
    } else if matches!(
        canonical_family_event_name(context).as_str(),
        "Golden Idol"
            | "Golden Wing"
            | "Big Fish"
            | "Forgotten Altar"
            | "Vampires"
            | "Golden Shrine"
            | "Face Trader"
            | "Beggar"
            | "Moai Head"
            | "Ghosts"
            | "Winding Halls"
            | "Mind Bloom"
    ) {
        EventPolicyFamily::CostTradeoff
    } else if matches!(
        canonical_family_event_name(context).as_str(),
        "Woman in Blue" | "Cleric" | "Designer" | "We Meet Again" | "Drug Dealer"
    ) {
        EventPolicyFamily::ResourceShoplike
    } else if context.event_name.is_empty() && context.event_id.is_empty() {
        EventPolicyFamily::GenericSafe
    } else {
        EventPolicyFamily::CompatibilityFallback
    }
}

fn canonical_family_event_name(context: &EventDecisionContext) -> String {
    if !context.event_name.is_empty() {
        context.event_name.clone()
    } else {
        context.event_id.clone()
    }
}

fn score_event_options(
    rs: &RunState,
    context: &EventDecisionContext,
    family: EventPolicyFamily,
) -> Vec<EventOptionScore> {
    match family {
        EventPolicyFamily::DeckSurgery => score_deck_surgery_options(rs, context),
        EventPolicyFamily::PressYourLuck => score_press_your_luck_options(rs, context),
        EventPolicyFamily::CostTradeoff => score_cost_tradeoff_options(rs, context),
        EventPolicyFamily::ResourceShoplike => score_resource_shoplike_options(rs, context),
        EventPolicyFamily::GenericSafe => score_generic_safe_options(rs, context),
        EventPolicyFamily::CompatibilityFallback => Vec::new(),
    }
}

fn choose_best_scored_option(
    context: &EventDecisionContext,
    family: EventPolicyFamily,
    scores: &[EventOptionScore],
) -> Option<EventChoiceDecision> {
    let best = scores
        .iter()
        .filter(|score| {
            context
                .options
                .get(score.option_index)
                .is_some_and(|option| !option.disabled)
        })
        .max_by(|left, right| left.total_score.total_cmp(&right.total_score))?;
    Some(EventChoiceDecision {
        option_index: best.option_index,
        command_index: option_command_index(context, best.option_index)?,
        family,
        rationale_key: Some(best.rationale_key),
        score: Some(best.total_score.round() as i32),
        safety_override_applied: best.safety_override_applied,
        rationale: Some(best.rationale_key),
        deck_improvement_assessment: best.deck_improvement_assessment.clone(),
    })
}

fn score_press_your_luck_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    if is_named_event(context, "Scrap Ooze") {
        return score_option_drafts(rs, context, |option| scrap_ooze_option_draft(rs, option));
    }
    if is_named_event(context, "World of Goop") {
        return score_option_drafts(rs, context, |option| world_of_goop_option_draft(rs, option));
    }
    if is_named_event(context, "Dead Adventurer") {
        return score_option_drafts(rs, context, |option| {
            dead_adventurer_option_draft(rs, option)
        });
    }
    if is_named_event(context, "Knowing Skull") {
        return score_option_drafts(rs, context, |option| {
            knowing_skull_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Cursed Tome") {
        return score_option_drafts(rs, context, |option| {
            cursed_tome_option_draft(rs, context, option)
        });
    }
    score_generic_safe_options(rs, context)
}

fn score_cost_tradeoff_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    if is_named_event(context, "Golden Idol") {
        return score_option_drafts(rs, context, |option| {
            golden_idol_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Big Fish") {
        return score_option_drafts(rs, context, |option| big_fish_option_draft(rs, option));
    }
    if is_named_event(context, "Golden Wing") {
        return score_option_drafts(rs, context, |option| {
            golden_wing_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Golden Shrine") {
        return score_option_drafts(rs, context, |option| {
            golden_shrine_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Face Trader") {
        return score_option_drafts(rs, context, |option| {
            face_trader_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Beggar") {
        return score_option_drafts(rs, context, |option| beggar_option_draft(rs, option));
    }
    if is_named_event(context, "Forgotten Altar") {
        return score_option_drafts(rs, context, |option| {
            forgotten_altar_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Moai Head") {
        return score_option_drafts(rs, context, |option| {
            moai_head_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Ghosts") {
        return score_option_drafts(rs, context, |option| {
            ghosts_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Winding Halls") {
        return score_option_drafts(rs, context, |option| {
            winding_halls_option_draft(rs, context, option)
        });
    }
    if is_named_event(context, "Vampires") {
        return score_option_drafts(rs, context, |option| vampires_option_draft(rs, option));
    }
    score_generic_safe_options(rs, context)
}

fn score_resource_shoplike_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    if is_named_event(context, "Woman in Blue") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_tag(option, EventOptionTag::ObtainPotion) {
                    let potion_count = first_number(&option.text).max(1) as usize;
                    let potion_value = random_potion_offer_value(
                        rs,
                        context.features.empty_potion_slots,
                        context.features.potion_blocked,
                        potion_count,
                        5.8,
                        1.9,
                        -240.0,
                    );
                    score += potion_value;
                    "resource_shoplike_buy_potion"
                } else {
                    "resource_shoplike_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Cleric") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_tag(option, EventOptionTag::Remove) {
                    score += generic_remove_value(rs) as f32;
                    "deck_surgery_remove_best"
                } else if contains_tag(option, EventOptionTag::Heal) {
                    score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.6;
                    "resource_shoplike_buy_heal"
                } else {
                    "resource_shoplike_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "We Meet Again") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["give potion"]) {
                    score += best_we_meet_again_potion_give_score(rs) as f32;
                    "resource_shoplike_trade_potion_for_relic"
                } else if contains_any(&option.text, &["give gold"]) {
                    score += 1_450.0
                        - option.payload.gold_delta.abs() as f32 * 18.0
                        - nearby_shop_conversion_bonus(rs) as f32;
                    "resource_shoplike_trade_gold_for_relic"
                } else if contains_any(&option.text, &["give card"]) {
                    score += best_we_meet_again_card_give_score(rs) as f32;
                    "resource_shoplike_trade_card_for_relic"
                } else if contains_any(&option.text, &["attack"]) {
                    "resource_shoplike_decline_trade"
                } else {
                    "resource_shoplike_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Drug Dealer") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
                let strength_shell_score = profile.strength_payoffs * 2
                    + profile.self_damage_sources
                    + profile.strength_enablers;
                let supports_mutagenic_strength = strength_shell_score >= 2;
                let rationale = if contains_tag(option, EventOptionTag::Transform) {
                    let transform_value =
                        crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, 2, false);
                    score += 1_800.0
                        + context.features.transform_targets as f32 * 260.0
                        + transform_value.total as f32 * 9.0;
                    if context.features.transform_targets >= 2 && !supports_mutagenic_strength {
                        score += 2_000.0;
                    }
                    "resource_shoplike_transform_best"
                } else if contains_tag(option, EventOptionTag::TakeRelic) {
                    score += 2_050.0 + strength_shell_score as f32 * 120.0;
                    "resource_shoplike_take_relic"
                } else if contains_any(&option.text, &["j.a.x", "jax", "ingest mutagens"]) {
                    let jax_delta = crate::bot::deck_delta_eval::compare_pick_vs_skip(
                        rs,
                        crate::content::cards::CardId::JAX,
                    );
                    score += 1_150.0
                        + jax_delta.prior_delta as f32 * 10.0
                        + jax_delta.rollout_delta as f32 * 6.0
                        + jax_delta.suite_bias as f32 * 3.0
                        + profile.strength_payoffs as f32 * 110.0
                        + profile.self_damage_sources as f32 * 80.0;
                    if context.features.transform_targets >= 2 && !supports_mutagenic_strength {
                        score -= 700.0;
                    }
                    "resource_shoplike_take_jax"
                } else {
                    "resource_shoplike_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    score_generic_safe_options(rs, context)
}

fn score_deck_surgery_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    let need = crate::bot::noncombat_families::build_noncombat_need_snapshot_for_run(rs);
    score_option_drafts(rs, context, |option| {
        deck_surgery_option_draft(rs, option, &need)
    })
}

fn score_generic_safe_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    score_option_drafts(rs, context, |option| {
        generic_safe_option_draft(rs, context, option)
    })
}

fn scrap_ooze_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    if contains_any(&option.text, &["reach in"]) {
        OptionScoreDraft {
            total_score: scrap_ooze_continue_score(
                rs,
                option.payload.hp_cost,
                nth_number(&option.text, 1),
            ),
            rationale_key: "press_your_luck_continue_positive_ev",
            deck_improvement_assessment: None,
        }
    } else {
        press_your_luck_stop_draft(rs, option)
    }
}

fn world_of_goop_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    if contains_any(&option.text, &["gather gold"]) {
        OptionScoreDraft {
            total_score: world_of_goop_gather_score(
                rs,
                option.payload.gold_delta,
                option.payload.hp_cost,
            ),
            rationale_key: "press_your_luck_continue_positive_ev",
            deck_improvement_assessment: None,
        }
    } else {
        press_your_luck_stop_draft(rs, option)
    }
}

fn dead_adventurer_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    if contains_any(&option.text, &["search"]) {
        OptionScoreDraft {
            total_score: dead_adventurer_continue_score(rs, first_number(&option.text)),
            rationale_key: "press_your_luck_continue_positive_ev",
            deck_improvement_assessment: None,
        }
    } else {
        press_your_luck_stop_draft(rs, option)
    }
}

fn knowing_skull_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let colorless_value = knowing_skull_colorless_value(rs);
    let potion_value = random_potion_offer_value(
        rs,
        context.features.empty_potion_slots,
        context.features.potion_blocked,
        1,
        6.0,
        2.0,
        -260.0,
    );
    let mut score = base_tradeoff_score(rs, option);
    let rationale_key = if contains_tag(option, EventOptionTag::ObtainColorlessCard) {
        score += colorless_value;
        "press_your_luck_skull_take_card"
    } else if contains_any(&option.text, &["gain 90 gold", "[gold]"]) {
        score += nearby_shop_conversion_bonus(rs) as f32 * 0.25;
        "press_your_luck_skull_take_gold"
    } else if contains_tag(option, EventOptionTag::ObtainPotion) {
        score += potion_value;
        "press_your_luck_skull_take_potion"
    } else {
        return press_your_luck_stop_draft(rs, option);
    };
    OptionScoreDraft {
        total_score: score,
        rationale_key,
        deck_improvement_assessment: None,
    }
}

fn cursed_tome_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let book_value = cursed_tome_book_relic_value(rs);
    let continuation_drag = cursed_tome_remaining_commitment_damage(rs, context.current_screen)
        as f32
        * hp_point_value(rs)
        * 0.82;
    if contains_any(&option.text, &["take the book", "book relic"]) {
        OptionScoreDraft {
            total_score: base_tradeoff_score(rs, option) + book_value,
            rationale_key: "press_your_luck_take_book_relic",
            deck_improvement_assessment: None,
        }
    } else if contains_any(&option.text, &["stop reading", "leave"]) {
        press_your_luck_stop_draft(rs, option)
    } else if contains_any(&option.text, &["continue", "read"]) {
        OptionScoreDraft {
            total_score: base_tradeoff_score(rs, option) + book_value - continuation_drag,
            rationale_key: "press_your_luck_continue_for_book",
            deck_improvement_assessment: None,
        }
    } else {
        press_your_luck_stop_draft(rs, option)
    }
}

fn press_your_luck_stop_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    OptionScoreDraft {
        total_score: safe_leave_score(rs, option),
        rationale_key: "press_your_luck_stop",
        deck_improvement_assessment: None,
    }
}

fn golden_idol_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let ectoplasm_blocks_gold = gold_gain_blocked(rs);
    let mut draft = cost_tradeoff_draft(rs, option, "cost_tradeoff_balanced");
    if contains_tag(option, EventOptionTag::TakeRelic) {
        if ectoplasm_blocks_gold {
            draft.total_score -= 2_400.0;
            draft.rationale_key = "cost_tradeoff_avoid_ectoplasm_gold_relic";
        } else {
            draft.total_score += golden_idol_future_value(rs);
            draft.rationale_key = "cost_tradeoff_take_relic";
        }
    } else if contains_tag(option, EventOptionTag::LoseMaxHp) {
        draft.total_score += 120.0 + (1.0 - context.features.hp_ratio).max(0.0) * 180.0;
        draft.rationale_key = "cost_tradeoff_preserve_current_hp";
    } else if contains_tag(option, EventOptionTag::Fight)
        || contains_tag(option, EventOptionTag::LoseHp)
    {
        draft.total_score += if context.features.hp_ratio >= 0.75 {
            180.0
        } else if context.features.hp_ratio >= 0.60 {
            40.0
        } else {
            -260.0
        };
        draft.rationale_key = "cost_tradeoff_pay_hp_keep_max_hp";
    } else if contains_tag(option, EventOptionTag::ObtainCurse) {
        draft.total_score += curse_mitigation_bonus(rs, 1);
        draft.total_score -= 160.0 + context.features.curse_pressure as f32 * 16.0;
        draft.rationale_key = "cost_tradeoff_accept_curse_over_hp_loss";
    } else if contains_tag(option, EventOptionTag::Leave) {
        draft.rationale_key = "cost_tradeoff_safe_leave";
        if ectoplasm_blocks_gold {
            draft.total_score += 420.0;
            draft.rationale_key = "cost_tradeoff_avoid_ectoplasm_gold_relic";
        }
    }
    draft
}

fn big_fish_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["banana", "heal"]) {
        let missing_hp = (rs.max_hp - rs.current_hp).max(0) as f32;
        draft.total_score +=
            missing_hp.min(option.payload.heal_amount.max(0) as f32) * hp_point_value(rs) * 0.35;
        draft.rationale_key = "cost_tradeoff_take_heal";
    } else if contains_any(&option.text, &["donut", "max hp"]) {
        draft.total_score += 5.0 * hp_point_value(rs) * 0.45;
        draft.rationale_key = "cost_tradeoff_take_max_hp";
    } else if contains_any(&option.text, &["box", "relic"]) {
        draft.total_score += relic_equity_value(rs) * 0.95;
        draft.total_score += curse_mitigation_bonus(rs, option.payload.curse_count.max(1));
        draft.rationale_key = "cost_tradeoff_take_relic";
    }
    draft
}

fn golden_wing_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_tag(option, EventOptionTag::Remove) {
        draft.total_score += generic_remove_value(rs) as f32;
        draft.rationale_key = "deck_surgery_remove_best";
    } else if contains_any(&option.text, &["attack"]) {
        draft.total_score += if context.features.hp_ratio >= 0.65 {
            180.0
        } else {
            -120.0
        };
        draft.rationale_key = "cost_tradeoff_fight_for_gold";
    }
    draft
}

fn golden_shrine_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    if gold_gain_blocked(rs) {
        let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
        if !contains_tag(option, EventOptionTag::Leave) {
            draft.total_score -= 220.0;
        }
        return draft;
    }

    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["pray"]) {
        draft.total_score +=
            option.payload.gold_delta.max(50) as f32 * gold_value_per_gold(rs) * 0.4;
        draft.rationale_key = "cost_tradeoff_gain_gold";
    } else if contains_any(&option.text, &["desecrate", "curse"]) {
        draft.total_score +=
            option.payload.gold_delta.max(275) as f32 * gold_value_per_gold(rs) * 0.22;
        draft.total_score += curse_mitigation_bonus(rs, option.payload.curse_count.max(1));
        draft.total_score -= 120.0 + context.features.curse_pressure as f32 * 18.0;
        draft.rationale_key = "cost_tradeoff_accept_curse_for_gold";
    }
    draft
}

fn face_trader_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["trade", "face relic"]) {
        draft.total_score += relic_equity_value(rs) * 0.9;
        draft.rationale_key = "cost_tradeoff_trade_for_relic";
    } else if contains_any(&option.text, &["touch", "gold"]) {
        draft.total_score +=
            option.payload.gold_delta.max(0) as f32 * gold_value_per_gold(rs) * 0.8;
        if context.features.hp_ratio < 0.25 {
            draft.total_score -= 300.0;
        }
        draft.rationale_key = "cost_tradeoff_gain_gold";
    }
    draft
}

fn beggar_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_tag(option, EventOptionTag::Remove) {
        draft.total_score += generic_remove_value(rs) as f32;
        if rs.gold < option.payload.gold_delta.abs() + 40 {
            draft.total_score -= 420.0;
        }
        draft.rationale_key = "cost_tradeoff_pay_gold_for_removal";
    }
    draft
}

fn forgotten_altar_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["offer", "bloody idol"]) {
        let value = if gold_gain_blocked(rs) {
            relic_equity_value(rs) * 0.35
        } else {
            relic_equity_value(rs) * 0.85 + 240.0 + golden_idol_future_value(rs) * 0.25
        };
        draft.total_score += value;
        draft.rationale_key = "cost_tradeoff_take_relic";
    } else if contains_any(&option.text, &["pray", "gain 5 max hp"]) {
        draft.total_score += 5.0 * hp_point_value(rs) * 0.30;
        if context.features.hp_ratio < 0.50 {
            draft.total_score -= 220.0;
        }
        draft.rationale_key = "cost_tradeoff_take_max_hp";
    } else if contains_any(&option.text, &["desecrate", "decay", "curse"]) {
        draft.total_score += curse_mitigation_bonus(rs, option.payload.curse_count.max(1));
        draft.total_score -= 80.0;
        draft.rationale_key = "cost_tradeoff_accept_curse_over_hp_loss";
    }
    draft
}

fn moai_head_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["trade", "golden idol", "333 gold"]) {
        if gold_gain_blocked(rs) {
            draft.total_score -= 240.0;
        } else {
            draft.total_score += 333.0 * gold_value_per_gold(rs);
        }
        if context.features.has_golden_idol {
            draft.total_score += 320.0;
        }
        draft.rationale_key = "cost_tradeoff_trade_idol_for_gold";
    } else if contains_any(&option.text, &["enter", "heal to full"]) {
        let missing_hp = (rs.max_hp - rs.current_hp).max(0) as f32;
        draft.total_score += missing_hp * hp_point_value(rs) * 0.9;
        draft.rationale_key = "cost_tradeoff_pay_max_hp_for_full_heal";
    }
    draft
}

fn ghosts_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_tag(option, EventOptionTag::ObtainCard) && option.payload.max_hp_cost > 0 {
        let route_support = ghosts_route_support(rs);
        let hp_ratio = context.features.hp_ratio;
        let late_act_without_support =
            rs.act_num >= 3 && (route_support < 6 || (route_support < 8 && hp_ratio < 0.55));
        if late_act_without_support {
            draft.total_score = draft.total_score.min(-900.0);
            draft.rationale_key = "cost_tradeoff_refuse_apparitions";
            return draft;
        }
        draft.total_score += apparition_package_value(rs, option);
        draft.rationale_key = if draft.total_score > 0.0 {
            "cost_tradeoff_accept_apparitions"
        } else {
            "cost_tradeoff_refuse_apparitions"
        };
    }
    draft
}

fn winding_halls_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["embrace", "madness"]) {
        draft.total_score += madness_package_value(rs, 2);
        draft.rationale_key = "cost_tradeoff_take_madness";
    } else if contains_any(&option.text, &["retrace", "writhe"]) {
        draft.total_score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.35;
        draft.total_score += curse_mitigation_bonus(rs, option.payload.curse_count.max(1));
        draft.total_score -= 120.0 + context.features.curse_pressure as f32 * 10.0;
        draft.rationale_key = "cost_tradeoff_heal_with_curse";
    } else if contains_any(&option.text, &["accept", "max hp"]) {
        draft.total_score += (rs.max_hp - rs.current_hp).max(0) as f32 * hp_point_value(rs) * 0.08;
        draft.rationale_key = "cost_tradeoff_take_max_hp";
    }
    draft
}

fn vampires_option_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    let mut draft = cost_tradeoff_safe_leave_draft(rs, option);
    if contains_any(&option.text, &["give vial", "blood vial"]) {
        draft.total_score += vampires_exchange_value(rs) - blood_vial_value(rs);
        draft.rationale_key = "cost_tradeoff_trade_relic_for_vampires";
    } else if contains_any(
        &option.text,
        &["accept", "replace all strikes with 5 bites"],
    ) {
        draft.total_score += vampires_exchange_value(rs);
        draft.rationale_key = "cost_tradeoff_accept_vampires";
    }
    draft
}

fn score_option_drafts(
    rs: &RunState,
    context: &EventDecisionContext,
    mut draft_for_option: impl FnMut(&EventOptionView) -> OptionScoreDraft,
) -> Vec<EventOptionScore> {
    context
        .options
        .iter()
        .map(|option| finalize_option_score(rs, option, draft_for_option(option)))
        .collect()
}

fn finalize_option_score(
    rs: &RunState,
    option: &EventOptionView,
    draft: OptionScoreDraft,
) -> EventOptionScore {
    let mut option_score = make_option_score(rs, option, draft.total_score, draft.rationale_key);
    option_score.deck_improvement_assessment = draft.deck_improvement_assessment;
    option_score
}

fn cost_tradeoff_draft(
    rs: &RunState,
    option: &EventOptionView,
    rationale_key: &'static str,
) -> OptionScoreDraft {
    OptionScoreDraft {
        total_score: base_tradeoff_score(rs, option),
        rationale_key,
        deck_improvement_assessment: None,
    }
}

fn cost_tradeoff_safe_leave_draft(rs: &RunState, option: &EventOptionView) -> OptionScoreDraft {
    cost_tradeoff_draft(rs, option, "cost_tradeoff_safe_leave")
}

fn generic_safe_option_draft(
    rs: &RunState,
    context: &EventDecisionContext,
    option: &EventOptionView,
) -> OptionScoreDraft {
    let mut score = base_tradeoff_score(rs, option);
    let mut assessment = None;
    let rationale_key = if contains_tag(option, EventOptionTag::Upgrade) {
        let upgrade_count = option_upgrade_count(option);
        let delta = crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, upgrade_count);
        assessment = Some(delta.prior_assessment.clone());
        score += 760.0
            + context.features.upgradable_cards as f32
                * if upgrade_count == 2 { 110.0 } else { 125.0 }
            + delta.total as f32 * if upgrade_count == 2 { 11.0 } else { 13.0 };
        "generic_safe_prefer_upgrade"
    } else if contains_tag(option, EventOptionTag::Remove) {
        let delta = crate::bot::deck_delta_eval::compare_purge_vs_keep(rs);
        assessment = Some(delta.prior_assessment.clone());
        score += 720.0 + context.features.remove_targets as f32 * 120.0 + delta.total as f32 * 13.0;
        "generic_safe_prefer_removal"
    } else if contains_tag(option, EventOptionTag::Transform) {
        let transform_count = option_transform_count(option);
        let delta =
            crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, transform_count, false);
        assessment = Some(delta.prior_assessment.clone());
        score += 700.0
            + context.features.transform_targets as f32
                * if transform_count == 2 { 120.0 } else { 90.0 }
            + delta.total as f32 * 12.0;
        "generic_safe_prefer_transform"
    } else if contains_tag(option, EventOptionTag::Heal) {
        score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.6;
        "generic_safe_prefer_heal"
    } else if contains_tag(option, EventOptionTag::TakeRelic) {
        score += relic_equity_value(rs) * 0.95;
        "generic_safe_take_relic"
    } else if contains_tag(option, EventOptionTag::Leave) {
        "generic_safe_leave"
    } else {
        "generic_safe_default"
    };

    OptionScoreDraft {
        total_score: score,
        rationale_key,
        deck_improvement_assessment: assessment,
    }
}

fn deck_surgery_option_draft(
    rs: &RunState,
    option: &EventOptionView,
    need: &crate::bot::noncombat_families::NoncombatNeedSnapshot,
) -> OptionScoreDraft {
    let assessment = crate::bot::noncombat_families::deck_surgery_option_assessment(
        rs,
        need,
        &option.text.to_ascii_lowercase(),
    );
    let rationale_key = if contains_tag(option, EventOptionTag::Remove) {
        "deck_surgery_remove_best"
    } else if contains_tag(option, EventOptionTag::Upgrade) {
        "deck_surgery_upgrade_best"
    } else if contains_tag(option, EventOptionTag::Transform) {
        "deck_surgery_transform_best"
    } else if contains_tag(option, EventOptionTag::Duplicate) {
        "deck_surgery_duplicate_best"
    } else {
        "deck_surgery_safe_leave"
    };

    OptionScoreDraft {
        total_score: deck_surgery_score(rs, option, assessment.as_ref())
            + base_tradeoff_score(rs, option),
        rationale_key,
        deck_improvement_assessment: assessment,
    }
}

fn option_upgrade_count(option: &EventOptionView) -> usize {
    if contains_any(&option.text, &["upgrade 2"]) {
        2
    } else {
        1
    }
}

fn option_transform_count(option: &EventOptionView) -> usize {
    if contains_any(&option.text, &["transform 2 cards"]) {
        2
    } else {
        1
    }
}

fn make_option_score(
    rs: &RunState,
    option: &EventOptionView,
    mut score: f32,
    rationale_key: &'static str,
) -> EventOptionScore {
    let viability = option_viable(rs, option);
    let safety_override_applied = !viability;
    if !viability {
        if contains_tag(option, EventOptionTag::Leave) {
            score += 240.0;
        } else {
            score = -10_000_000.0 - option.payload.hp_cost.max(0) as f32 * hp_point_value(rs);
        }
    }
    EventOptionScore {
        option_index: option.index,
        total_score: score,
        rationale_key: if !viability && !contains_tag(option, EventOptionTag::Leave) {
            "press_your_luck_stop_safety_guard"
        } else {
            rationale_key
        },
        viable: viability,
        safety_override_applied,
        breakdown: Vec::new(),
        deck_improvement_assessment: None,
    }
}

fn option_viable(rs: &RunState, option: &EventOptionView) -> bool {
    if option.disabled {
        return false;
    }
    if option.payload.hp_cost > 0 && option.payload.hp_cost >= rs.current_hp {
        return false;
    }
    true
}

fn base_tradeoff_score(rs: &RunState, option: &EventOptionView) -> f32 {
    let mut score = 0.0;
    score -= option.payload.hp_cost.max(0) as f32 * hp_point_value(rs);
    score -= option.payload.max_hp_cost.max(0) as f32 * 90.0;
    let effective_gold_delta = if option.payload.gold_delta > 0 && gold_gain_blocked(rs) {
        0
    } else {
        option.payload.gold_delta
    };
    score += effective_gold_delta as f32 * gold_value_per_gold(rs);
    score += option.payload.max_hp_gain.max(0) as f32 * 110.0;
    score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.5;
    score -= option.payload.curse_count.max(0) as f32 * (640.0 + context_curse_drag(rs));
    if option.payload.lose_starter_relic {
        score -= 1_800.0;
    }
    score
}

fn safe_leave_score(rs: &RunState, option: &EventOptionView) -> f32 {
    -option.payload.hp_cost.max(0) as f32 * hp_point_value(rs) * 0.6
}

fn deck_surgery_score(
    rs: &RunState,
    option: &EventOptionView,
    assessment: Option<&crate::bot::run_deck_improvement::DeckOperationAssessment>,
) -> f32 {
    let remove_targets = count_remove_targets(rs);
    let transform_targets = count_transform_targets(rs);
    let upgradable_cards = count_upgradable_cards(rs);
    let curse_pressure = curse_pressure_score(rs);
    let mut score = 0.0;
    if contains_tag(option, EventOptionTag::Remove) {
        let delta = assessment
            .map(|value| value.total_prior_delta)
            .unwrap_or_else(|| crate::bot::deck_delta_eval::compare_purge_vs_keep(rs).total);
        score += 1_050.0 + remove_targets as f32 * 120.0 + curse_pressure as f32 * 28.0;
        score += delta as f32 * 14.0;
    }
    if contains_tag(option, EventOptionTag::Transform) {
        let count = option_transform_count(option);
        let delta = assessment
            .map(|value| value.total_prior_delta)
            .unwrap_or_else(|| {
                crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, count, false).total
            });
        score += 900.0 + transform_targets as f32 * if count == 2 { 150.0 } else { 100.0 };
        score += delta as f32 * 14.0;
    }
    if contains_tag(option, EventOptionTag::Upgrade) {
        let count = option_upgrade_count(option);
        let delta = assessment
            .map(|value| value.total_prior_delta)
            .unwrap_or_else(|| {
                crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, count).total
            });
        score += 850.0 + upgradable_cards as f32 * if count == 2 { 120.0 } else { 140.0 };
        score += delta as f32 * if count == 2 { 12.0 } else { 14.0 };
    }
    if contains_tag(option, EventOptionTag::Duplicate) {
        let delta = assessment
            .map(|value| value.total_prior_delta)
            .unwrap_or_else(|| crate::bot::deck_delta_eval::compare_duplicate_vs_decline(rs).total);
        score += 780.0 + delta as f32 * 14.0;
    }
    score
}

fn ghosts_route_support(rs: &RunState) -> i32 {
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    profile.draw_sources * 2
        + profile.exhaust_engines * 3
        + profile.exhaust_outlets * 2
        + profile.block_core
        + profile.block_payoffs
        + profile.power_scalers
        + profile.self_damage_sources
}

fn apparition_package_value(rs: &RunState, option: &EventOptionView) -> f32 {
    let count = first_number(&option.text).max(if rs.ascension_level >= 15 { 3 } else { 5 });
    let apparition_delta = crate::bot::deck_delta_eval::compare_pick_vs_skip(
        rs,
        crate::content::cards::CardId::Apparition,
    );
    let early_act_bonus = if rs.act_num <= 2 { 280.0 } else { -1_500.0 };
    count as f32 * (80.0 + apparition_delta.total as f32 * 6.0)
        + ghosts_route_support(rs) as f32 * 60.0
        + early_act_bonus
}

fn madness_package_value(rs: &RunState, count: usize) -> f32 {
    let madness_delta = crate::bot::deck_delta_eval::compare_pick_vs_skip(
        rs,
        crate::content::cards::CardId::Madness,
    );
    count as f32 * (90.0 + madness_delta.total as f32 * 9.0)
}

fn vampires_exchange_value(rs: &RunState) -> f32 {
    let bite_exchange = crate::bot::deck_delta_eval::compare_vampires_vs_refuse(rs);
    bite_exchange.total as f32 * 12.0
        + event_count_starter_strikes(rs) as f32 * 85.0
        + if rs.act_num <= 2 { 220.0 } else { 0.0 }
        - nearby_shop_conversion_bonus(rs) as f32
}

fn blood_vial_value(rs: &RunState) -> f32 {
    220.0 + (1.0 - rs.current_hp as f32 / rs.max_hp.max(1) as f32).max(0.0) * 120.0
}

fn golden_idol_future_value(rs: &RunState) -> f32 {
    if gold_gain_blocked(rs) {
        return -400.0;
    }
    let mut value = relic_equity_value(rs) * 0.72;
    if rs.act_num == 1 {
        value += 260.0;
    }
    if rs.floor_num <= 20 {
        value += 140.0;
    }
    value
}

fn curse_mitigation_bonus(rs: &RunState, curse_count: i32) -> f32 {
    let count = curse_count.max(0) as f32;
    if count == 0.0 {
        return 0.0;
    }
    let mut bonus = curse_tractability_score(rs) as f32 * 220.0 * count;
    if has_live_omamori(rs) {
        bonus += 1_450.0 * count.min(omamori_charges(rs) as f32);
    }
    if has_relic(rs, crate::content::relics::RelicId::BlueCandle) {
        bonus += 520.0 * count;
    }
    bonus
}

fn gold_gain_blocked(rs: &RunState) -> bool {
    has_relic(rs, crate::content::relics::RelicId::Ectoplasm)
}

fn has_live_omamori(rs: &RunState) -> bool {
    omamori_charges(rs) > 0
}

fn omamori_charges(rs: &RunState) -> i32 {
    rs.relics
        .iter()
        .find(|relic| relic.id == crate::content::relics::RelicId::Omamori)
        .map(|relic| relic.counter.max(0))
        .unwrap_or(0)
}

fn event_count_starter_strikes(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            crate::content::cards::get_card_definition(card.id)
                .tags
                .contains(&crate::content::cards::CardTag::StarterStrike)
        })
        .count() as i32
}

fn has_relic(rs: &RunState, relic_id: crate::content::relics::RelicId) -> bool {
    rs.relics.iter().any(|relic| relic.id == relic_id)
}

fn extract_numbers(text: &str) -> Vec<i32> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| segment.parse::<i32>().ok())
        .collect()
}

fn contains_tag(option: &EventOptionView, tag: EventOptionTag) -> bool {
    option.semantic_tags.contains(&tag)
}

fn is_named_event(context: &EventDecisionContext, name: &str) -> bool {
    context.event_id.eq_ignore_ascii_case(name) || context.event_name.eq_ignore_ascii_case(name)
}

fn canonical_event_name(event_id: EventId) -> &'static str {
    match event_id {
        EventId::BigFish => "Big Fish",
        EventId::Cleric => "Cleric",
        EventId::DeadAdventurer => "Dead Adventurer",
        EventId::GoldenIdol => "Golden Idol",
        EventId::LivingWall => "Living Wall",
        EventId::Mushrooms => "Mushrooms",
        EventId::ScrapOoze => "Scrap Ooze",
        EventId::ShiningLight => "Shining Light",
        EventId::Ssssserpent => "Ssssserpent",
        EventId::WorldOfGoop => "World of Goop",
        EventId::GoldenWing => "Golden Wing",
        EventId::MatchAndKeep => "Match and Keep",
        EventId::GoldenShrine => "Golden Shrine",
        EventId::Addict => "Addict",
        EventId::BackTotheBasics => "Back to Basics",
        EventId::Beggar => "Beggar",
        EventId::Colosseum => "Colosseum",
        EventId::CursedTome => "Cursed Tome",
        EventId::DrugDealer => "Drug Dealer",
        EventId::ForgottenAltar => "Forgotten Altar",
        EventId::Ghosts => "Ghosts",
        EventId::KnowingSkull => "Knowing Skull",
        EventId::MaskedBandits => "Masked Bandits",
        EventId::Mausoleum => "Mausoleum",
        EventId::Nest => "Nest",
        EventId::Nloth => "Nloth",
        EventId::TheJoust => "The Joust",
        EventId::TheLibrary => "The Library",
        EventId::Vampires => "Vampires",
        EventId::Falling => "Falling",
        EventId::MindBloom => "Mind Bloom",
        EventId::MoaiHead => "Moai Head",
        EventId::MysteriousSphere => "Mysterious Sphere",
        EventId::SensoryStone => "Sensory Stone",
        EventId::TombRedMask => "Tomb Red Mask",
        EventId::WindingHalls => "Winding Halls",
        EventId::AccursedBlacksmith => "Accursed Blacksmith",
        EventId::BonfireElementals => "Bonfire Elementals",
        EventId::BonfireSpirits => "Bonfire Spirits",
        EventId::Designer => "Designer",
        EventId::Duplicator => "Duplicator",
        EventId::FaceTrader => "Face Trader",
        EventId::FountainOfCurseCleansing => "Fountain of Curse Cleansing",
        EventId::GremlinWheelGame => "Gremlin Wheel Game",
        EventId::Lab => "Lab",
        EventId::NoteForYourself => "Note For Yourself",
        EventId::Purifier => "Purification Shrine",
        EventId::Transmorgrifier => "Transmogrifier",
        EventId::UpgradeShrine => "Upgrade Shrine",
        EventId::WeMeetAgain => "We Meet Again",
        EventId::WomanInBlue => "Woman in Blue",
        EventId::Neow => "Neow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::state::events::EventChoiceMeta;

    #[test]
    fn unknown_event_falls_back_to_generic_safe_instead_of_legacy_adapter() {
        let rs = RunState::new(1, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: "Totally Unknown".to_string(),
            event_name: "Totally Unknown".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![EventOptionView {
                index: 0,
                text: "Leave".to_string(),
                label: Some("Leave".to_string()),
                disabled: false,
                choice_index: Some(0),
                semantic_tags: vec![EventOptionTag::Leave],
                payload: EventOptionPayload::default(),
            }],
            features: EventDecisionFeatures::default(),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::GenericSafe);
        assert_eq!(decision.rationale_key, Some("generic_safe_first_enabled"));
    }

    #[test]
    fn legacy_neow_keeps_compatibility_fallback_path() {
        let rs = RunState::new(1, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: "Neow".to_string(),
            event_name: "Neow".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![EventOptionView {
                index: 0,
                text: "Proceed".to_string(),
                label: Some("Proceed".to_string()),
                disabled: false,
                choice_index: Some(0),
                semantic_tags: vec![EventOptionTag::Leave],
                payload: EventOptionPayload::default(),
            }],
            features: EventDecisionFeatures::default(),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CompatibilityFallback);
        assert_eq!(decision.rationale_key, Some("legacy_fallback_neow"));
    }

    #[test]
    fn drug_dealer_prefers_transform_when_strength_shell_is_absent() {
        let rs = RunState::new(1, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: "Drug Dealer".to_string(),
            event_name: "Drug Dealer".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Test Subject] Transform 2 cards".to_string(),
                    label: Some("Test Subject".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::Transform],
                    payload: EventOptionPayload::default(),
                },
                EventOptionView {
                    index: 1,
                    text: "[Mutagenic Strength] Take relic".to_string(),
                    label: Some("Mutagenic Strength".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::TakeRelic, EventOptionTag::ObtainRelic],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::ResourceShoplike);
        assert_eq!(
            decision.rationale_key,
            Some("resource_shoplike_transform_best")
        );
    }

    #[test]
    fn deck_surgery_event_prefers_remove_and_carries_assessment() {
        let mut rs = RunState::new(2, 0, true, "Ironclad");
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            91_001,
        ));
        let context = EventDecisionContext {
            event_id: "Purification Shrine".to_string(),
            event_name: "Purification Shrine".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Purify] Remove a card".to_string(),
                    label: Some("Purify".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::Remove],
                    payload: EventOptionPayload::default(),
                },
                EventOptionView {
                    index: 1,
                    text: "[Leave] Leave".to_string(),
                    label: Some("Leave".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::DeckSurgery);
        assert_eq!(decision.rationale_key, Some("deck_surgery_remove_best"));
        assert!(decision.deck_improvement_assessment.is_some());
    }

    #[test]
    fn knowing_skull_uses_contextual_card_value_when_potions_are_blocked() {
        let mut rs = RunState::new(3, 0, true, "Ironclad");
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Sozu,
        ));
        let context = EventDecisionContext {
            event_id: "Knowing Skull".to_string(),
            event_name: "Knowing Skull".to_string(),
            current_screen: 1,
            current_screen_index: Some(1),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Potion] Lose 6 HP. Obtain a random Potion.".to_string(),
                    label: Some("Potion".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::ObtainPotion],
                    payload: EventOptionPayload {
                        hp_cost: 6,
                        potion_count: 1,
                        repeatable: true,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Card] Lose 6 HP. Obtain a colorless card.".to_string(),
                    label: Some("Card".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::ObtainColorlessCard],
                    payload: EventOptionPayload {
                        hp_cost: 6,
                        colorless_card_count: 1,
                        repeatable: true,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 2,
                    text: "[Leave] Lose 6 HP.".to_string(),
                    label: Some("Leave".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload {
                        hp_cost: 6,
                        ..EventOptionPayload::default()
                    },
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::PressYourLuck);
        assert_eq!(
            decision.rationale_key,
            Some("press_your_luck_skull_take_card")
        );
    }

    #[test]
    fn generic_safe_upgrade_uses_typed_assessment() {
        let rs = RunState::new(4, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: String::new(),
            event_name: String::new(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Upgrade] Upgrade a card".to_string(),
                    label: Some("Upgrade".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::Upgrade],
                    payload: EventOptionPayload::default(),
                },
                EventOptionView {
                    index: 1,
                    text: "[Leave] Leave".to_string(),
                    label: Some("Leave".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::GenericSafe);
        assert_eq!(decision.rationale_key, Some("generic_safe_prefer_upgrade"));
        assert!(decision.deck_improvement_assessment.is_some());
    }

    #[test]
    fn woman_in_blue_prefers_more_potions_when_slots_are_open() {
        let rs = RunState::new(5, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: "Woman in Blue".to_string(),
            event_name: "Woman in Blue".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[1 Potion] Lose 20 Gold.".to_string(),
                    label: Some("1 Potion".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::ObtainPotion],
                    payload: EventOptionPayload {
                        gold_delta: -20,
                        potion_count: 1,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[2 Potions] Lose 30 Gold.".to_string(),
                    label: Some("2 Potions".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::ObtainPotion],
                    payload: EventOptionPayload {
                        gold_delta: -30,
                        potion_count: 2,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 2,
                    text: "[3 Potions] Lose 40 Gold.".to_string(),
                    label: Some("3 Potions".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::ObtainPotion],
                    payload: EventOptionPayload {
                        gold_delta: -40,
                        potion_count: 3,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 3,
                    text: "[Leave] Leave".to_string(),
                    label: Some("Leave".to_string()),
                    disabled: false,
                    choice_index: Some(3),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::ResourceShoplike);
        assert_eq!(decision.option_index, 2);
        assert_eq!(decision.rationale_key, Some("resource_shoplike_buy_potion"));
    }

    #[test]
    fn cursed_tome_takes_book_when_hp_can_support_it() {
        let rs = RunState::new(6, 0, true, "Ironclad");
        let context = EventDecisionContext {
            event_id: "Cursed Tome".to_string(),
            event_name: "Cursed Tome".to_string(),
            current_screen: 4,
            current_screen_index: Some(4),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Take the Book] Take 10 damage. Obtain a Book relic.".to_string(),
                    label: Some("Take the Book".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::TakeRelic, EventOptionTag::ObtainRelic],
                    payload: EventOptionPayload {
                        hp_cost: 10,
                        relic_count: 1,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Stop Reading] Take 3 damage.".to_string(),
                    label: Some("Stop Reading".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload {
                        hp_cost: 3,
                        ..EventOptionPayload::default()
                    },
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::PressYourLuck);
        assert_eq!(
            decision.rationale_key,
            Some("press_your_luck_take_book_relic")
        );
    }

    #[test]
    fn golden_shrine_with_ectoplasm_prefers_leave() {
        let mut rs = RunState::new(7, 0, true, "Ironclad");
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Ectoplasm,
        ));
        let context = EventDecisionContext {
            event_id: "Golden Shrine".to_string(),
            event_name: "Golden Shrine".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Pray] Gain 100 Gold.".to_string(),
                    label: Some("Pray".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::GainGold],
                    payload: EventOptionPayload {
                        gold_delta: 100,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Desecrate] Gain 275 Gold. Become Cursed - Regret.".to_string(),
                    label: Some("Desecrate".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::GainGold, EventOptionTag::ObtainCurse],
                    payload: EventOptionPayload {
                        gold_delta: 275,
                        curse_count: 1,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 2,
                    text: "[Leave]".to_string(),
                    label: Some("Leave".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CostTradeoff);
        assert_eq!(decision.option_index, 2);
    }

    #[test]
    fn big_fish_with_omamori_prefers_box() {
        let mut rs = RunState::new(8, 0, true, "Ironclad");
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::Omamori,
        ));
        let context = EventDecisionContext {
            event_id: "Big Fish".to_string(),
            event_name: "Big Fish".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Banana] Heal 26 HP.".to_string(),
                    label: Some("Banana".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::Heal],
                    payload: EventOptionPayload {
                        heal_amount: 26,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Donut] Gain 5 Max HP.".to_string(),
                    label: Some("Donut".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::GainMaxHp],
                    payload: EventOptionPayload {
                        max_hp_gain: 5,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 2,
                    text: "[Box] Obtain a random Relic. Become Cursed - Regret.".to_string(),
                    label: Some("Box".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::TakeRelic, EventOptionTag::ObtainCurse],
                    payload: EventOptionPayload {
                        relic_count: 1,
                        curse_count: 1,
                        ..EventOptionPayload::default()
                    },
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CostTradeoff);
        assert_eq!(decision.option_index, 2);
        assert_eq!(decision.rationale_key, Some("cost_tradeoff_take_relic"));
    }

    #[test]
    fn vampires_with_blood_vial_prefers_vial_exchange() {
        let mut rs = RunState::new(9, 0, true, "Ironclad");
        rs.relics.push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::BloodVial,
        ));
        let context = EventDecisionContext {
            event_id: "Vampires".to_string(),
            event_name: "Vampires".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Accept] Lose 24 Max HP. Replace all Strikes with 5 Bites.".to_string(),
                    label: Some("Accept".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::LoseMaxHp],
                    payload: EventOptionPayload {
                        max_hp_cost: 24,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Give Vial] Lose Blood Vial. Replace all Strikes with 5 Bites."
                        .to_string(),
                    label: Some("Give Vial".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::LoseRelic],
                    payload: EventOptionPayload::default(),
                },
                EventOptionView {
                    index: 2,
                    text: "[Refuse] Leave.".to_string(),
                    label: Some("Refuse".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CostTradeoff);
        assert_eq!(decision.option_index, 1);
        assert_eq!(
            decision.rationale_key,
            Some("cost_tradeoff_trade_relic_for_vampires")
        );
    }

    #[test]
    fn ghosts_late_weak_deck_refuses_apparitions() {
        let mut rs = RunState::new(10, 0, true, "Ironclad");
        rs.act_num = 3;
        let context = EventDecisionContext {
            event_id: "Ghosts".to_string(),
            event_name: "Ghosts".to_string(),
            current_screen: 0,
            current_screen_index: Some(0),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Accept] Lose 40 Max HP. Obtain 5 Apparitions.".to_string(),
                    label: Some("Accept".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::LoseMaxHp, EventOptionTag::ObtainCard],
                    payload: EventOptionPayload {
                        max_hp_cost: 40,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Refuse]".to_string(),
                    label: Some("Refuse".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::Leave],
                    payload: EventOptionPayload::default(),
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CostTradeoff);
        assert_eq!(decision.option_index, 1);
    }

    #[test]
    fn winding_halls_low_hp_prefers_retrace_over_madness() {
        let mut rs = RunState::new(11, 0, true, "Ironclad");
        rs.current_hp = 18;
        let context = EventDecisionContext {
            event_id: "Winding Halls".to_string(),
            event_name: "Winding Halls".to_string(),
            current_screen: 1,
            current_screen_index: Some(1),
            current_screen_key: None,
            screen_source: Some("test".to_string()),
            options: vec![
                EventOptionView {
                    index: 0,
                    text: "[Embrace] Lose 10 HP. Obtain 2 Madness.".to_string(),
                    label: Some("Embrace".to_string()),
                    disabled: false,
                    choice_index: Some(0),
                    semantic_tags: vec![EventOptionTag::LoseHp, EventOptionTag::ObtainCard],
                    payload: EventOptionPayload {
                        hp_cost: 10,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 1,
                    text: "[Retrace] Heal 20 HP. Become Cursed - Writhe.".to_string(),
                    label: Some("Retrace".to_string()),
                    disabled: false,
                    choice_index: Some(1),
                    semantic_tags: vec![EventOptionTag::Heal, EventOptionTag::ObtainCurse],
                    payload: EventOptionPayload {
                        heal_amount: 20,
                        curse_count: 1,
                        ..EventOptionPayload::default()
                    },
                },
                EventOptionView {
                    index: 2,
                    text: "[Accept] Lose 4 Max HP.".to_string(),
                    label: Some("Accept".to_string()),
                    disabled: false,
                    choice_index: Some(2),
                    semantic_tags: vec![EventOptionTag::LoseMaxHp],
                    payload: EventOptionPayload {
                        max_hp_cost: 4,
                        ..EventOptionPayload::default()
                    },
                },
            ],
            features: derive_event_features(&rs),
        };

        let decision = choose_event_option(&rs, &context).unwrap();
        assert_eq!(decision.family, EventPolicyFamily::CostTradeoff);
        assert_eq!(decision.option_index, 1);
        assert_eq!(
            decision.rationale_key,
            Some("cost_tradeoff_heal_with_curse")
        );
    }

    #[test]
    fn local_event_context_uses_structured_golden_idol_semantics() {
        let mut rs = RunState::new(12, 0, true, "Ironclad");
        rs.event_state = Some(EventState {
            id: EventId::GoldenIdol,
            current_screen: 1,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        let event = rs.event_state.clone().unwrap();
        let options = crate::engine::event_handler::get_event_options(&rs);
        let context = local_event_context(&rs, &event, &options);

        let fight = &context.options[1];
        assert!(contains_tag(fight, EventOptionTag::Fight));
        assert!(contains_tag(fight, EventOptionTag::LoseHp));
        assert!(fight.payload.hp_cost > 0);
    }

    #[test]
    fn local_event_context_uses_structured_ghosts_semantics() {
        let mut rs = RunState::new(13, 0, true, "Ironclad");
        rs.event_state = Some(EventState {
            id: EventId::Ghosts,
            current_screen: 0,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        let event = rs.event_state.clone().unwrap();
        let options = crate::engine::event_handler::get_event_options(&rs);
        let context = local_event_context(&rs, &event, &options);

        let accept = &context.options[0];
        assert!(contains_tag(accept, EventOptionTag::ObtainCard));
        assert!(contains_tag(accept, EventOptionTag::LoseMaxHp));
        assert!(accept.payload.card_count >= 3);
        assert!(accept.payload.max_hp_cost > 0);
    }

    #[test]
    fn structured_local_ghosts_decision_ignores_option_text() {
        let mut rs = RunState::new(14, 0, true, "Ironclad");
        rs.act_num = 3;
        let event = EventState::new(EventId::Ghosts);
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("nonsense alpha"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Accept,
                    effects: vec![
                        EventEffect::LoseMaxHp(40),
                        EventEffect::ObtainCard {
                            count: 5,
                            kind: EventCardKind::Specific(CardId::Apparition),
                        },
                    ],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("nonsense beta"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Decline,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 1);
    }

    #[test]
    fn structured_local_cursed_tome_decision_ignores_option_text() {
        let rs = RunState::new(15, 0, true, "Ironclad");
        let event = EventState {
            id: EventId::CursedTome,
            current_screen: 4,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("garbage one"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Accept,
                    effects: vec![
                        EventEffect::LoseHp(10),
                        EventEffect::ObtainRelic {
                            count: 1,
                            kind: EventRelicKind::RandomBook,
                        },
                    ],
                    constraints: vec![],
                    transition: EventOptionTransition::OpenReward,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("garbage two"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Decline,
                    effects: vec![EventEffect::LoseHp(3)],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 0);
        assert_eq!(
            decision.rationale_key,
            Some("press_your_luck_take_book_relic")
        );
    }

    #[test]
    fn structured_local_we_meet_again_decision_ignores_option_text() {
        let mut rs = RunState::new(16, 0, true, "Ironclad");
        rs.potions[0] = Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::StrengthPotion,
            99_001,
        ));
        let event = EventState::new(EventId::WeMeetAgain);
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("x"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Trade,
                    effects: vec![EventEffect::ObtainRelic {
                        count: 1,
                        kind: EventRelicKind::RandomRelic,
                    }],
                    constraints: vec![EventOptionConstraint::RequiresPotion],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("y"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Decline,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 0);
        assert_eq!(
            decision.rationale_key,
            Some("resource_shoplike_trade_potion_for_relic")
        );
    }

    #[test]
    fn structured_local_mind_bloom_decision_ignores_option_text() {
        let mut rs = RunState::new(17, 0, true, "Ironclad");
        rs.floor_num = 45;
        rs.max_hp = 80;
        rs.current_hp = 12;
        let event = EventState::new(EventId::MindBloom);
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("alpha"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Fight,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::StartCombat,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("beta"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Accept,
                    effects: vec![
                        EventEffect::UpgradeCard { count: usize::MAX },
                        EventEffect::ObtainRelic {
                            count: 1,
                            kind: EventRelicKind::Specific(
                                crate::content::relics::RelicId::MarkOfTheBloom,
                            ),
                        },
                    ],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("gamma"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Accept,
                    effects: vec![
                        EventEffect::Heal(68),
                        EventEffect::ObtainCurse {
                            count: 1,
                            kind: EventCardKind::Specific(CardId::Doubt),
                        },
                    ],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 2);
        assert_eq!(
            decision.rationale_key,
            Some("cost_tradeoff_heal_with_curse")
        );
    }

    #[test]
    fn structured_local_falling_decision_ignores_option_text() {
        let mut rs = RunState::new(18, 0, true, "Ironclad");
        rs.add_card_to_deck(CardId::DemonForm);

        let defend_uuid = rs
            .master_deck
            .iter()
            .find(|card| card.id == CardId::Defend)
            .map(|card| card.uuid)
            .unwrap();
        let bash_uuid = rs
            .master_deck
            .iter()
            .find(|card| card.id == CardId::Bash)
            .map(|card| card.uuid)
            .unwrap();
        let demon_form_uuid = rs
            .master_deck
            .iter()
            .find(|card| card.id == CardId::DemonForm)
            .map(|card| card.uuid)
            .unwrap();

        let event = EventState {
            id: EventId::Falling,
            current_screen: 1,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("skill path"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![EventEffect::RemoveCard {
                        count: 1,
                        target_uuid: Some(defend_uuid),
                        kind: EventCardKind::Specific(CardId::Defend),
                    }],
                    constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("power path"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![EventEffect::RemoveCard {
                        count: 1,
                        target_uuid: Some(demon_form_uuid),
                        kind: EventCardKind::Specific(CardId::DemonForm),
                    }],
                    constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("attack path"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![EventEffect::RemoveCard {
                        count: 1,
                        target_uuid: Some(bash_uuid),
                        kind: EventCardKind::Specific(CardId::Bash),
                    }],
                    constraints: vec![EventOptionConstraint::RequiresRemovableCard],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 0);
    }

    #[test]
    fn structured_local_designer_decision_ignores_option_text() {
        let mut rs = RunState::new(19, 0, true, "Ironclad");
        rs.gold = 200;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            91_777,
        ));
        let event = EventState {
            id: EventId::Designer,
            current_screen: 1,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("adjust"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![
                        EventEffect::LoseGold(40),
                        EventEffect::UpgradeCard { count: 1 },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(40),
                        EventOptionConstraint::RequiresUpgradeableCard,
                    ],
                    transition: EventOptionTransition::OpenSelection(
                        crate::state::events::EventSelectionKind::UpgradeCard,
                    ),
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("cleanup"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![
                        EventEffect::LoseGold(60),
                        EventEffect::TransformCard { count: 2 },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(60),
                        EventOptionConstraint::RequiresTransformableCard,
                    ],
                    transition: EventOptionTransition::OpenSelection(
                        crate::state::events::EventSelectionKind::TransformCard,
                    ),
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("full"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![
                        EventEffect::LoseGold(90),
                        EventEffect::RemoveCard {
                            count: 1,
                            target_uuid: None,
                            kind: EventCardKind::Unknown,
                        },
                        EventEffect::UpgradeCard { count: 1 },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(90),
                        EventOptionConstraint::RequiresRemovableCard,
                    ],
                    transition: EventOptionTransition::OpenSelection(
                        crate::state::events::EventSelectionKind::RemoveCard,
                    ),
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("punch"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Decline,
                    effects: vec![EventEffect::LoseHp(3)],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(
            decision.rationale_key,
            Some("resource_shoplike_buy_transform")
        );
    }

    #[test]
    fn structured_local_woman_in_blue_decision_ignores_option_text() {
        let rs = RunState::new(20, 0, true, "Ironclad");
        let event = EventState::new(EventId::WomanInBlue);
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("one"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Trade,
                    effects: vec![
                        EventEffect::LoseGold(20),
                        EventEffect::ObtainPotion { count: 1 },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(20),
                        EventOptionConstraint::RequiresPotionSlotValue,
                    ],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("three"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Trade,
                    effects: vec![
                        EventEffect::LoseGold(40),
                        EventEffect::ObtainPotion { count: 3 },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(40),
                        EventOptionConstraint::RequiresPotionSlotValue,
                    ],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("leave"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Leave,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(decision.rationale_key, Some("resource_shoplike_buy_potion"));
    }

    #[test]
    fn structured_local_cleric_decision_ignores_option_text() {
        let mut rs = RunState::new(21, 0, true, "Ironclad");
        rs.current_hp = 20;
        rs.max_hp = 80;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            92_001,
        ));
        let event = EventState::new(EventId::Cleric);
        let options = vec![
            EventOption::new(
                EventChoiceMeta::new("heal"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Trade,
                    effects: vec![EventEffect::LoseGold(35), EventEffect::Heal(20)],
                    constraints: vec![EventOptionConstraint::RequiresGold(35)],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("purify"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::DeckOperation,
                    effects: vec![
                        EventEffect::LoseGold(50),
                        EventEffect::RemoveCard {
                            count: 1,
                            target_uuid: None,
                            kind: EventCardKind::Unknown,
                        },
                    ],
                    constraints: vec![
                        EventOptionConstraint::RequiresGold(50),
                        EventOptionConstraint::RequiresRemovableCard,
                    ],
                    transition: EventOptionTransition::OpenSelection(
                        crate::state::events::EventSelectionKind::RemoveCard,
                    ),
                    repeatable: false,
                    terminal: false,
                },
            ),
            EventOption::new(
                EventChoiceMeta::new("leave"),
                crate::state::events::EventOptionSemantics {
                    action: EventActionKind::Leave,
                    effects: vec![],
                    constraints: vec![],
                    transition: EventOptionTransition::AdvanceScreen,
                    repeatable: false,
                    terminal: false,
                },
            ),
        ];

        let decision = choose_local_event_choice(&rs, &event, &options).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(decision.rationale_key, Some("deck_surgery_remove_best"));
    }

    #[test]
    fn structured_live_woman_in_blue_decision_ignores_option_text() {
        let rs = RunState::new(22, 0, true, "Ironclad");
        let gs = json!({
            "screen_state": {
                "event_name": "Woman in Blue",
                "event_id": "Woman in Blue",
                "current_screen": 0,
                "options": [
                    { "text": "aaa", "disabled": false, "choice_index": 4 },
                    { "text": "bbb", "disabled": false, "choice_index": 5 },
                    { "text": "ccc", "disabled": false, "choice_index": 6 },
                    { "text": "ddd", "disabled": false, "choice_index": 7 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 2);
        assert_eq!(decision.command_index, 6);
        assert_eq!(decision.rationale_key, Some("resource_shoplike_buy_potion"));
    }

    #[test]
    fn structured_live_ghosts_decision_ignores_option_text() {
        let mut rs = RunState::new(23, 0, true, "Ironclad");
        rs.act_num = 3;
        let gs = json!({
            "screen_state": {
                "event_name": "Ghosts",
                "event_id": "Ghosts",
                "current_screen": 0,
                "options": [
                    { "text": "junk one", "disabled": false, "choice_index": 0 },
                    { "text": "junk two", "disabled": false, "choice_index": 1 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(decision.command_index, 1);
    }

    #[test]
    fn structured_live_cleric_decision_ignores_option_text() {
        let mut rs = RunState::new(24, 0, true, "Ironclad");
        rs.current_hp = 20;
        rs.max_hp = 80;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            92_101,
        ));
        let gs = json!({
            "screen_state": {
                "event_name": "Cleric",
                "event_id": "Cleric",
                "current_screen": 0,
                "options": [
                    { "text": "h1", "disabled": false, "choice_index": 0 },
                    { "text": "h2", "disabled": false, "choice_index": 1 },
                    { "text": "h3", "disabled": false, "choice_index": 2 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(decision.command_index, 1);
        assert_eq!(decision.rationale_key, Some("deck_surgery_remove_best"));
    }

    #[test]
    fn structured_live_designer_decision_uses_event_semantics_state() {
        let mut rs = RunState::new(25, 0, true, "Ironclad");
        rs.gold = 70;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            92_201,
        ));
        let gs = json!({
            "screen_state": {
                "event_name": "Designer",
                "event_id": "Designer",
                "current_screen": 1,
                "event_semantics_state": {
                    "adjust_upgrades_one": true,
                    "clean_up_removes_cards": true
                },
                "options": [
                    { "text": "d1", "disabled": false, "choice_index": 10 },
                    { "text": "d2", "disabled": false, "choice_index": 11 },
                    { "text": "d3", "disabled": true, "choice_index": 12 },
                    { "text": "d4", "disabled": false, "choice_index": 13 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 1);
        assert_eq!(decision.command_index, 11);
        assert_eq!(
            decision.rationale_key,
            Some("resource_shoplike_buy_removal")
        );
    }

    #[test]
    fn structured_live_we_meet_again_decision_uses_event_semantics_state() {
        let mut rs = RunState::new(26, 0, true, "Ironclad");
        rs.potions[0] = Some(crate::content::potions::Potion::new(
            crate::content::potions::PotionId::StrengthPotion,
            99_101,
        ));
        let gs = json!({
            "screen_state": {
                "event_name": "We Meet Again",
                "event_id": "We Meet Again",
                "current_screen": 0,
                "event_semantics_state": {
                    "potion_slot": 0,
                    "gold_amount": 75,
                    "card_uuid": null
                },
                "options": [
                    { "text": "w1", "disabled": false, "choice_index": 20 },
                    { "text": "w2", "disabled": false, "choice_index": 21 },
                    { "text": "w3", "disabled": true, "choice_index": 22 },
                    { "text": "w4", "disabled": false, "choice_index": 23 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 0);
        assert_eq!(decision.command_index, 20);
        assert_eq!(
            decision.rationale_key,
            Some("resource_shoplike_trade_potion_for_relic")
        );
    }

    #[test]
    fn structured_live_falling_decision_uses_event_semantics_state() {
        let mut rs = RunState::new(27, 0, true, "Ironclad");
        rs.add_card_to_deck(CardId::ShrugItOff);
        let skill_uuid = rs.master_deck.last().unwrap().uuid;
        rs.add_card_to_deck(CardId::Inflame);
        let power_uuid = rs.master_deck.last().unwrap().uuid;
        let attack_uuid = rs
            .master_deck
            .iter()
            .find(|card| card.id == CardId::Strike)
            .map(|card| card.uuid)
            .unwrap();
        let gs = json!({
            "screen_state": {
                "event_name": "Falling",
                "event_id": "Falling",
                "current_screen": 1,
                "event_semantics_state": {
                    "skill_uuid": skill_uuid,
                    "power_uuid": power_uuid,
                    "attack_uuid": attack_uuid
                },
                "options": [
                    { "text": "f1", "disabled": false, "choice_index": 30 },
                    { "text": "f2", "disabled": false, "choice_index": 31 },
                    { "text": "f3", "disabled": false, "choice_index": 32 }
                ]
            }
        });

        let decision = choose_live_event_choice(&gs, &rs).unwrap();
        assert_eq!(decision.option_index, 2);
        assert_eq!(decision.command_index, 32);
    }
}
