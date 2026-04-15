use crate::map::node::RoomType;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};

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
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventOptionScore {
    pub option_index: usize,
    pub total_score: f32,
    pub rationale_key: &'static str,
    pub viable: bool,
    pub safety_override_applied: bool,
    pub breakdown: Vec<(&'static str, f32)>,
}

pub fn choose_local_event_choice(
    rs: &RunState,
    event: &EventState,
    choices: &[EventChoiceMeta],
) -> Option<EventChoiceDecision> {
    let context = local_event_context(rs, event, choices);
    choose_event_option(rs, &context)
}

pub fn choose_live_event_choice(gs: &Value, rs: &RunState) -> Option<EventChoiceDecision> {
    let context = live_event_context(gs, rs)?;
    choose_event_option(rs, &context)
}

pub fn local_event_context(
    rs: &RunState,
    event: &EventState,
    choices: &[EventChoiceMeta],
) -> EventDecisionContext {
    let name = canonical_event_name(event.id);
    EventDecisionContext {
        event_id: name.to_string(),
        event_name: name.to_string(),
        current_screen: event.current_screen,
        current_screen_index: Some(event.current_screen),
        current_screen_key: None,
        screen_source: Some("rust_run_state".to_string()),
        options: choices
            .iter()
            .enumerate()
            .map(|(index, choice)| EventOptionView {
                index,
                text: choice.text.clone(),
                label: extract_bracket_label(&choice.text),
                disabled: choice.disabled,
                choice_index: Some(index),
                semantic_tags: classify_event_option_tags(
                    &choice.text,
                    extract_bracket_label(&choice.text).as_deref(),
                ),
                payload: parse_event_option_payload(
                    &choice.text,
                    extract_bracket_label(&choice.text).as_deref(),
                ),
            })
            .collect(),
        features: derive_event_features(rs),
    }
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

    compatibility_fallback_event_choice(rs, context).or_else(|| {
        let option_index = fallback_option_index(context)?;
        Some(EventChoiceDecision {
            option_index,
            command_index: option_command_index(context, option_index)?,
            family: EventPolicyFamily::GenericSafe,
            rationale_key: Some("generic_safe_first_enabled"),
            score: None,
            safety_override_applied: false,
            rationale: Some("generic_safe_first_enabled"),
        })
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
    })
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
    Some(EventChoiceDecision {
        option_index,
        command_index,
        family: EventPolicyFamily::CompatibilityFallback,
        rationale_key: Some("compatibility_fallback_adapter"),
        score: None,
        safety_override_applied: false,
        rationale: Some("compatibility_fallback_adapter"),
    })
}

fn score_press_your_luck_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    if is_named_event(context, "Scrap Ooze") {
        return context
            .options
            .iter()
            .map(|option| {
                let is_continue = contains_any(&option.text, &["reach in"]);
                let continue_score = scrap_ooze_continue_score(
                    rs,
                    option.payload.hp_cost,
                    nth_number(&option.text, 1),
                );
                if is_continue {
                    make_option_score(
                        rs,
                        option,
                        continue_score,
                        "press_your_luck_continue_positive_ev",
                    )
                } else {
                    make_option_score(
                        rs,
                        option,
                        safe_leave_score(rs, option),
                        "press_your_luck_stop",
                    )
                }
            })
            .collect();
    }
    if is_named_event(context, "World of Goop") {
        return context
            .options
            .iter()
            .map(|option| {
                if contains_any(&option.text, &["gather gold"]) {
                    make_option_score(
                        rs,
                        option,
                        world_of_goop_gather_score(
                            rs,
                            option.payload.gold_delta,
                            option.payload.hp_cost,
                        ),
                        "press_your_luck_continue_positive_ev",
                    )
                } else {
                    make_option_score(
                        rs,
                        option,
                        safe_leave_score(rs, option),
                        "press_your_luck_stop",
                    )
                }
            })
            .collect();
    }
    if is_named_event(context, "Dead Adventurer") {
        return context
            .options
            .iter()
            .map(|option| {
                if contains_any(&option.text, &["search"]) {
                    make_option_score(
                        rs,
                        option,
                        dead_adventurer_continue_score(rs, first_number(&option.text)),
                        "press_your_luck_continue_positive_ev",
                    )
                } else {
                    make_option_score(
                        rs,
                        option,
                        safe_leave_score(rs, option),
                        "press_your_luck_stop",
                    )
                }
            })
            .collect();
    }
    if is_named_event(context, "Knowing Skull") {
        return score_knowing_skull_options(rs, context);
    }
    if is_named_event(context, "Cursed Tome") {
        return context
            .options
            .iter()
            .map(|option| {
                if contains_any(&option.text, &["open", "read"]) {
                    make_option_score(
                        rs,
                        option,
                        820.0 - option.payload.hp_cost as f32 * hp_point_value(rs),
                        "press_your_luck_continue_positive_ev",
                    )
                } else {
                    make_option_score(
                        rs,
                        option,
                        safe_leave_score(rs, option),
                        "press_your_luck_stop",
                    )
                }
            })
            .collect();
    }
    score_generic_safe_options(rs, context)
}

fn score_cost_tradeoff_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    if is_named_event(context, "Golden Idol") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["take", "obtain golden idol"]) {
                    if rs
                        .relics
                        .iter()
                        .any(|relic| relic.id == crate::content::relics::RelicId::Ectoplasm)
                    {
                        score -= 2_400.0;
                        "cost_tradeoff_avoid_ectoplasm_gold_relic"
                    } else {
                        score += 1_600.0;
                        "cost_tradeoff_take_relic"
                    }
                } else if contains_any(&option.text, &["lose max hp"]) {
                    score += 260.0;
                    "cost_tradeoff_preserve_current_hp"
                } else if contains_any(&option.text, &["fight", "damage"]) {
                    score += if context.features.hp_ratio >= 0.70 {
                        240.0
                    } else {
                        -180.0
                    };
                    "cost_tradeoff_pay_hp_keep_max_hp"
                } else if contains_any(&option.text, &["run", "injury", "curse"]) {
                    score -= 260.0 + context.features.curse_pressure as f32 * 12.0;
                    "cost_tradeoff_accept_curse_over_hp_loss"
                } else if contains_tag(option, EventOptionTag::Leave) {
                    if rs
                        .relics
                        .iter()
                        .any(|relic| relic.id == crate::content::relics::RelicId::Ectoplasm)
                    {
                        score += 420.0;
                        "cost_tradeoff_avoid_ectoplasm_gold_relic"
                    } else {
                        "cost_tradeoff_safe_leave"
                    }
                } else {
                    "cost_tradeoff_balanced"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Golden Wing") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_tag(option, EventOptionTag::Remove) {
                    score += generic_remove_value(rs) as f32;
                    "deck_surgery_remove_best"
                } else if contains_any(&option.text, &["attack"]) {
                    score += if context.features.hp_ratio >= 0.65 {
                        180.0
                    } else {
                        -120.0
                    };
                    "cost_tradeoff_fight_for_gold"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Golden Shrine") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["pray"]) {
                    score +=
                        option.payload.gold_delta.max(50) as f32 * gold_value_per_gold(rs) * 0.9;
                    "cost_tradeoff_gain_gold"
                } else if contains_any(&option.text, &["desecrate", "curse"]) {
                    score +=
                        option.payload.gold_delta.max(200) as f32 * gold_value_per_gold(rs) * 0.8;
                    score += curse_tractability_score(rs) as f32 * 320.0
                        + nearby_shop_conversion_bonus(rs) as f32;
                    score -= 1_650.0 + context.features.curse_pressure as f32 * 40.0;
                    "cost_tradeoff_accept_curse_for_gold"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Face Trader") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["trade", "face relic"]) {
                    score += relic_equity_value(rs) * 0.9;
                    "cost_tradeoff_trade_for_relic"
                } else if contains_any(&option.text, &["touch", "gold"]) {
                    score +=
                        option.payload.gold_delta.max(0) as f32 * gold_value_per_gold(rs) * 0.8;
                    if context.features.hp_ratio < 0.25 {
                        score -= 300.0;
                    }
                    "cost_tradeoff_gain_gold"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Beggar") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_tag(option, EventOptionTag::Remove) {
                    score += generic_remove_value(rs) as f32;
                    if rs.gold < option.payload.gold_delta.abs() + 40 {
                        score -= 420.0;
                    }
                    "cost_tradeoff_pay_gold_for_removal"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Moai Head") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["trade", "golden idol", "333 gold"])
                {
                    score += 333.0 * gold_value_per_gold(rs);
                    if context.features.has_golden_idol {
                        score += 320.0;
                    }
                    "cost_tradeoff_trade_idol_for_gold"
                } else if contains_any(&option.text, &["enter", "heal to full"]) {
                    let missing_hp = (rs.max_hp - rs.current_hp).max(0) as f32;
                    score += missing_hp * hp_point_value(rs) * 0.9;
                    "cost_tradeoff_pay_max_hp_for_full_heal"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Ghosts") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
                let route_support = profile.draw_sources * 2
                    + profile.exhaust_engines * 3
                    + profile.exhaust_outlets * 2
                    + profile.block_core
                    + profile.block_payoffs
                    + profile.power_scalers
                    + profile.self_damage_sources;
                let strong_support = route_support >= 5;
                let medium_support = route_support >= 3;
                let rationale = if contains_any(&option.text, &["accept", "apparitions"]) {
                    if rs.act_num <= 2
                        && ((strong_support && context.features.hp_ratio >= 0.45)
                            || (medium_support && context.features.hp_ratio >= 0.60))
                    {
                        score += 2_200.0;
                        "cost_tradeoff_accept_apparitions"
                    } else {
                        score -= 800.0;
                        "cost_tradeoff_refuse_apparitions"
                    }
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
    }
    if is_named_event(context, "Winding Halls") {
        return context
            .options
            .iter()
            .map(|option| {
                let mut score = base_tradeoff_score(rs, option);
                let rationale = if contains_any(&option.text, &["embrace", "madness"]) {
                    score += if context.features.hp_ratio >= 0.65 {
                        520.0
                    } else {
                        -180.0
                    };
                    "cost_tradeoff_take_madness"
                } else if contains_any(&option.text, &["retrace", "writhe"]) {
                    score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.7;
                    score -= 360.0 + context.features.curse_pressure as f32 * 22.0;
                    "cost_tradeoff_heal_with_curse"
                } else if contains_any(&option.text, &["accept", "max hp"]) {
                    score += 240.0;
                    "cost_tradeoff_take_max_hp"
                } else {
                    "cost_tradeoff_safe_leave"
                };
                make_option_score(rs, option, score, rationale)
            })
            .collect();
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
                    let potion_value = if context.features.potion_blocked {
                        -400.0
                    } else if context.features.empty_potion_slots > 0 {
                        640.0
                    } else {
                        180.0
                    };
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
                let has_strength_scaling = rs.master_deck.iter().any(|card| {
                    matches!(
                        card.id,
                        crate::content::cards::CardId::HeavyBlade
                            | crate::content::cards::CardId::SwordBoomerang
                            | crate::content::cards::CardId::TwinStrike
                            | crate::content::cards::CardId::Pummel
                            | crate::content::cards::CardId::Reaper
                    )
                });
                let rationale = if contains_tag(option, EventOptionTag::Transform) {
                    score += 1_800.0
                        + context.features.transform_targets as f32 * 260.0
                        + crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, 2, false)
                            .total as f32
                            * 9.0;
                    if context.features.transform_targets >= 2 && !has_strength_scaling {
                        score += 2_000.0;
                    }
                    "resource_shoplike_transform_best"
                } else if contains_tag(option, EventOptionTag::TakeRelic) {
                    score += if has_strength_scaling {
                        2_400.0
                    } else {
                        2_050.0
                    };
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
                        + if has_strength_scaling { 220.0 } else { 0.0 };
                    if context.features.transform_targets >= 2 && !has_strength_scaling {
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
    context
        .options
        .iter()
        .map(|option| {
            let score = deck_surgery_score(rs, option) + base_tradeoff_score(rs, option);
            let rationale = if contains_tag(option, EventOptionTag::Remove) {
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
            make_option_score(rs, option, score, rationale)
        })
        .collect()
}

fn score_generic_safe_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    context
        .options
        .iter()
        .map(|option| {
            let mut score = base_tradeoff_score(rs, option);
            let rationale = if contains_tag(option, EventOptionTag::Upgrade) {
                score += 1_600.0 + context.features.upgradable_cards as f32 * 180.0;
                "generic_safe_prefer_upgrade"
            } else if contains_tag(option, EventOptionTag::Remove) {
                score += generic_remove_value(rs) as f32;
                "generic_safe_prefer_removal"
            } else if contains_tag(option, EventOptionTag::Transform) {
                score += 1_250.0 + context.features.transform_targets as f32 * 120.0;
                "generic_safe_prefer_transform"
            } else if contains_tag(option, EventOptionTag::Heal) {
                score += option.payload.heal_amount.max(0) as f32 * hp_point_value(rs) * 0.6;
                "generic_safe_prefer_heal"
            } else if contains_tag(option, EventOptionTag::TakeRelic) {
                score += 1_400.0;
                "generic_safe_take_relic"
            } else if contains_tag(option, EventOptionTag::Leave) {
                "generic_safe_leave"
            } else {
                "generic_safe_default"
            };
            make_option_score(rs, option, score, rationale)
        })
        .collect()
}

fn score_knowing_skull_options(
    rs: &RunState,
    context: &EventDecisionContext,
) -> Vec<EventOptionScore> {
    context
        .options
        .iter()
        .map(|option| {
            let mut score = base_tradeoff_score(rs, option);
            let rationale = if contains_tag(option, EventOptionTag::ObtainColorlessCard) {
                score += 1_120.0;
                "press_your_luck_skull_take_card"
            } else if contains_any(&option.text, &["gain 90 gold", "[gold]"]) {
                score += 900.0 + 90.0 * gold_value_per_gold(rs);
                "press_your_luck_skull_take_gold"
            } else if contains_tag(option, EventOptionTag::ObtainPotion) {
                score += if context.features.potion_blocked {
                    -220.0
                } else if context.features.empty_potion_slots > 0 {
                    560.0
                } else {
                    120.0
                };
                "press_your_luck_skull_take_potion"
            } else {
                "press_your_luck_stop"
            };
            make_option_score(rs, option, score, rationale)
        })
        .collect()
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
    score += option.payload.gold_delta as f32 * gold_value_per_gold(rs);
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

fn deck_surgery_score(rs: &RunState, option: &EventOptionView) -> f32 {
    let lower = option.text.to_ascii_lowercase();
    let remove_targets = count_remove_targets(rs);
    let transform_targets = count_transform_targets(rs);
    let upgradable_cards = count_upgradable_cards(rs);
    let curse_pressure = curse_pressure_score(rs);
    let mut score = 0.0;
    if contains_tag(option, EventOptionTag::Remove) {
        score += 3_000.0 + remove_targets as f32 * 420.0 + curse_pressure as f32 * 55.0;
        score += crate::bot::deck_delta_eval::compare_purge_vs_keep(rs).total as f32 * 12.0;
    }
    if contains_tag(option, EventOptionTag::Transform) {
        let count = if contains_any(&lower, &["transform 2 cards"]) {
            2
        } else {
            1
        };
        score += 2_200.0 + transform_targets as f32 * if count == 2 { 450.0 } else { 320.0 };
        score += crate::bot::deck_delta_eval::compare_transform_vs_decline(rs, count, false).total
            as f32
            * 10.0;
    }
    if contains_tag(option, EventOptionTag::Upgrade) {
        let count = if contains_any(&lower, &["upgrade 2"]) {
            2
        } else {
            1
        };
        score += 2_000.0 + upgradable_cards as f32 * if count == 2 { 220.0 } else { 280.0 };
        score += crate::bot::deck_delta_eval::compare_upgrade_vs_decline(rs, count).total as f32
            * if count == 2 { 8.0 } else { 10.0 };
    }
    if contains_tag(option, EventOptionTag::Duplicate) {
        score += 2_000.0
            + crate::bot::deck_delta_eval::compare_duplicate_vs_decline(rs).total as f32 * 10.0;
    }
    score
}

fn context_curse_drag(rs: &RunState) -> f32 {
    40.0 + curse_pressure_score(rs) as f32 * 14.0
}

fn nearby_shop_conversion_bonus(rs: &RunState) -> i32 {
    match reachable_room_distance(rs, RoomType::ShopRoom, 3) {
        Some(1) => 380,
        Some(2) => 260,
        Some(3) => 120,
        _ => 0,
    }
}

fn curse_tractability_score(rs: &RunState) -> i32 {
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let purge_cost = 75 + rs.shop_purge_count.max(0) * 25;
    let mut score = 0;
    if let Some(distance) = reachable_room_distance(rs, RoomType::ShopRoom, 4) {
        if distance <= 2 && rs.gold >= purge_cost {
            score += 4;
        } else if distance <= 4 && rs.gold >= purge_cost {
            score += 2;
        } else if distance <= 2 {
            score += 1;
        }
    }
    if profile.exhaust_outlets >= 1 || profile.exhaust_engines >= 2 {
        score += 1;
    }
    score
}

fn best_we_meet_again_card_give_score(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Curse {
                2_500 + crate::bot::evaluator::curse_remove_severity(card.id) * 120
            } else if def
                .tags
                .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || def.rarity == crate::content::cards::CardRarity::Basic
            {
                1_900
            } else {
                let owned_value =
                    crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card.id, rs);
                (1_450 - owned_value * 18).max(-200)
            }
        })
        .max()
        .unwrap_or(-200)
}

fn potion_keep_value(potion_id: crate::content::potions::PotionId) -> i32 {
    use crate::content::potions::PotionId;
    match potion_id {
        PotionId::AncientPotion => 100,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::BlessingOfTheForge => 84,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 85,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 88,
        _ => 55,
    }
}

fn best_we_meet_again_potion_give_score(rs: &RunState) -> i32 {
    rs.potions
        .iter()
        .flatten()
        .map(|p| 1_850 - potion_keep_value(p.id) * 12)
        .max()
        .unwrap_or(-300)
}

fn scrap_ooze_continue_score(rs: &RunState, damage: i32, chance: i32) -> f32 {
    let success = chance as f32 / 100.0;
    let hp_cost = hp_point_value(rs);
    let relic_value = relic_equity_value(rs);
    let immediate_cost = damage.max(0) as f32 * hp_cost;
    let failure_drag = (1.0 - success) * (damage + 1).max(0) as f32 * hp_cost * 0.55;
    let safety_penalty = safety_gap_penalty(rs, damage);
    success * relic_value - immediate_cost - failure_drag - safety_penalty
}

fn world_of_goop_gather_score(rs: &RunState, gain: i32, damage: i32) -> f32 {
    gain.max(0) as f32 * gold_value_per_gold(rs)
        - damage.max(0) as f32 * hp_point_value(rs)
        - safety_gap_penalty(rs, damage)
}

fn dead_adventurer_continue_score(rs: &RunState, encounter_chance: i32) -> f32 {
    let fight_p = encounter_chance as f32 / 100.0;
    let safe_loot_value = 680.0 + relic_equity_value(rs) * 0.18;
    let combat_reward_value = 900.0 + gold_value_per_gold(rs) * 20.0;
    let combat_risk = hp_point_value(rs) * (7.0 + route_hostility_scalar(rs) * 0.6);
    let safety_penalty = safety_gap_penalty(rs, 9);
    (1.0 - fight_p) * safe_loot_value + fight_p * (combat_reward_value - combat_risk)
        - safety_penalty * 0.45
}

fn relic_equity_value(rs: &RunState) -> f32 {
    let mut value = 1_450.0;
    if rs.act_num == 1 {
        value += 150.0;
    }
    if rs.floor_num <= 5 {
        value += 120.0;
    }
    if matches!(
        reachable_room_distance(rs, RoomType::MonsterRoomElite, 3),
        Some(1 | 2)
    ) {
        value += 90.0;
    }
    value
}

fn gold_value_per_gold(rs: &RunState) -> f32 {
    let mut value = 11.5;
    match reachable_room_distance(rs, RoomType::ShopRoom, 3) {
        Some(1) => value += 2.5,
        Some(2) => value += 1.5,
        Some(3) => value += 0.5,
        _ => {}
    }
    if rs.gold < 75 {
        value += 1.0;
    }
    value
}

fn hp_point_value(rs: &RunState) -> f32 {
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
    let mut value = 42.0 + (1.0 - hp_ratio).max(0.0) * 78.0;
    if rs.current_hp <= 45 {
        value += 8.0;
    }
    if rs.current_hp <= 30 {
        value += 14.0;
    }
    if rs.current_hp <= 20 {
        value += 22.0;
    }
    match reachable_room_distance(rs, RoomType::MonsterRoomElite, 4) {
        Some(1) => value += 18.0,
        Some(2) => value += 10.0,
        _ => {}
    }
    match reachable_room_distance(rs, RoomType::RestRoom, 3) {
        Some(1) => value -= 12.0,
        Some(2) => value -= 6.0,
        _ => {}
    }
    value.max(18.0)
}

fn route_hostility_scalar(rs: &RunState) -> f32 {
    let mut scalar: f32 = 0.0;
    if matches!(
        reachable_room_distance(rs, RoomType::MonsterRoom, 2),
        Some(1)
    ) {
        scalar += 2.0;
    }
    match reachable_room_distance(rs, RoomType::MonsterRoomElite, 4) {
        Some(1) => scalar += 3.0,
        Some(2) => scalar += 2.0,
        Some(3) => scalar += 1.0,
        _ => {}
    }
    match reachable_room_distance(rs, RoomType::RestRoom, 3) {
        Some(1) => scalar -= 1.5,
        Some(2) => scalar -= 0.8,
        _ => {}
    }
    scalar.max(0.0)
}

fn safety_gap_penalty(rs: &RunState, immediate_damage: i32) -> f32 {
    let after_hp = (rs.current_hp - immediate_damage).max(0);
    let mut safety_floor = 14;
    if matches!(
        reachable_room_distance(rs, RoomType::MonsterRoomElite, 3),
        Some(1 | 2)
    ) {
        safety_floor += 10;
    }
    if matches!(
        reachable_room_distance(rs, RoomType::RestRoom, 3),
        None | Some(3)
    ) {
        safety_floor += 4;
    }
    let gap = (safety_floor - after_hp).max(0) as f32;
    gap * hp_point_value(rs) * 0.65
}

fn reachable_room_distance(rs: &RunState, target: RoomType, max_depth: i32) -> Option<i32> {
    if rs.map.current_y < 0 || rs.map.current_x < 0 {
        return None;
    }

    let start = (rs.map.current_x as usize, rs.map.current_y as usize);
    let mut q = VecDeque::from([(start, 0i32)]);
    let mut seen = HashSet::from([start]);

    while let Some(((x, y), depth)) = q.pop_front() {
        if depth > 0
            && rs
                .map
                .graph
                .get(y)
                .and_then(|row| row.get(x))
                .and_then(|node| node.class)
                == Some(target)
        {
            return Some(depth);
        }
        if depth >= max_depth {
            continue;
        }
        let Some(node) = rs.map.graph.get(y).and_then(|row| row.get(x)) else {
            continue;
        };
        for edge in &node.edges {
            if edge.dst_x < 0 || edge.dst_y < 0 {
                continue;
            }
            let next = (edge.dst_x as usize, edge.dst_y as usize);
            if seen.insert(next) {
                q.push_back((next, depth + 1));
            }
        }
    }

    None
}

fn first_number(text: &str) -> i32 {
    nth_number(text, 0)
}

fn nth_number(text: &str, index: usize) -> i32 {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .nth(index)
        .and_then(|segment| segment.parse::<i32>().ok())
        .unwrap_or(0)
}

fn extract_numbers(text: &str) -> Vec<i32> {
    text.split(|c: char| !c.is_ascii_digit())
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| segment.parse::<i32>().ok())
        .collect()
}

fn count_upgradable_cards(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            card.id == crate::content::cards::CardId::SearingBlow
                || (card.upgrades == 0
                    && def.card_type != crate::content::cards::CardType::Status
                    && def.card_type != crate::content::cards::CardType::Curse)
        })
        .count() as i32
}

fn count_remove_targets(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type == crate::content::cards::CardType::Curse
                || def.card_type == crate::content::cards::CardType::Status
                || def
                    .tags
                    .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || (def.rarity == crate::content::cards::CardRarity::Basic
                    && !def.tags.contains(&crate::content::cards::CardTag::Healing))
        })
        .count() as i32
}

fn count_transform_targets(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            def.card_type == crate::content::cards::CardType::Curse
                || def
                    .tags
                    .contains(&crate::content::cards::CardTag::StarterStrike)
                || def.name == "Defend"
                || def.rarity == crate::content::cards::CardRarity::Basic
                || (def.rarity == crate::content::cards::CardRarity::Common
                    && def.card_type != crate::content::cards::CardType::Power)
        })
        .count() as i32
}

fn curse_pressure_score(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .map(|card| {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.card_type == crate::content::cards::CardType::Curse {
                let severity = crate::bot::evaluator::curse_remove_severity(card.id);
                if severity > 0 {
                    severity
                } else {
                    3
                }
            } else {
                0
            }
        })
        .sum()
}

fn generic_remove_value(rs: &RunState) -> i32 {
    1_500
        + count_remove_targets(rs) * 420
        + curse_pressure_score(rs) * 90
        + crate::bot::deck_delta_eval::compare_purge_vs_keep(rs).total * 12
}

fn contains_tag(option: &EventOptionView, tag: EventOptionTag) -> bool {
    option.semantic_tags.contains(&tag)
}

fn is_named_event(context: &EventDecisionContext, name: &str) -> bool {
    context.event_id.eq_ignore_ascii_case(name) || context.event_name.eq_ignore_ascii_case(name)
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
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

