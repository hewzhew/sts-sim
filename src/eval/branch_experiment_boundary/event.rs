use super::card_reward::{
    select_card_reward_branch_options_for_session, CardRewardBranchOption,
    CardRewardBranchOptionSource,
};
use crate::content::cards::CardId;
use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::state::core::{ClientInput, EngineState};
use crate::state::events::{
    EventActionKind, EventCardKind, EventEffect, EventOption, EventOptionSemantics,
    EventOptionTransition, EventRelicKind, EventSelectionKind,
};

const MAX_EVENT_OPTIONS_PER_BRANCH: usize = 4;

#[derive(Clone, Debug)]
pub(crate) struct EventBranchOption {
    pub(crate) label: String,
    pub(crate) command: String,
    pub(crate) card: Option<CardId>,
    pub(crate) upgrades: Option<u8>,
    pub(crate) effect_kind: String,
    pub(crate) effect_key: String,
    pub(crate) effect_label: String,
    pub(crate) representative_count: usize,
    pub(crate) suppressed_count: usize,
}

#[derive(Clone, Debug)]
struct EventOptionBranchSemantics {
    effect_kind: String,
    effect_key: String,
    effect_label: String,
}

pub(crate) fn event_branch_options(
    session: &RunControlSession,
    max_card_offer_options: Option<usize>,
) -> Option<Vec<EventBranchOption>> {
    if !matches!(session.engine_state, EngineState::EventRoom) {
        return None;
    }
    let event_options = crate::engine::event_handler::get_event_options(&session.run_state);
    if event_options.len() == 1 && terminal_no_effect_leave(&event_options[0]) {
        return None;
    }
    let surface = build_decision_surface(session);
    let mut branch_options = Vec::new();

    for candidate in &surface.view.candidates {
        let Some(ClientInput::EventChoice(index)) = candidate.action.executable_input() else {
            continue;
        };
        let Some(event_option) = event_options.get(index) else {
            continue;
        };
        if event_option.ui.disabled {
            return None;
        }
        if nloth_trade_is_protected(session, event_option) {
            continue;
        }
        let semantics = branch_semantics_for_event_option(event_option);
        branch_options.push(EventBranchOption {
            label: candidate.label.clone(),
            command: candidate.action.command_hint(),
            card: event_option_specific_card(event_option),
            upgrades: event_option_upgrade_hint(event_option),
            effect_kind: semantics.effect_kind,
            effect_key: semantics.effect_key,
            effect_label: semantics.effect_label,
            representative_count: 1,
            suppressed_count: 0,
        });
    }

    if branch_options.is_empty() {
        return None;
    }
    if branch_options.len() > MAX_EVENT_OPTIONS_PER_BRANCH {
        if branch_options
            .iter()
            .all(|option| option.effect_kind == "event_card_reward")
        {
            let limit = max_card_offer_options
                .unwrap_or(MAX_EVENT_OPTIONS_PER_BRANCH)
                .min(branch_options.len());
            return select_event_card_reward_branch_options(session, branch_options, limit);
        }
        return None;
    }
    Some(branch_options)
}

fn select_event_card_reward_branch_options(
    session: &RunControlSession,
    options: Vec<EventBranchOption>,
    limit: usize,
) -> Option<Vec<EventBranchOption>> {
    if limit == 0 {
        return None;
    }
    let original_count = options.len();
    let card_options = options
        .iter()
        .map(|option| {
            Some(CardRewardBranchOption {
                label: option.label.clone(),
                command: option.command.clone(),
                card: Some(option.card?),
                upgrades: Some(option.upgrades.unwrap_or_default()),
                source: CardRewardBranchOptionSource::PermanentReward,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let selected =
        select_card_reward_branch_options_for_session(session, card_options, Some(limit), None)
            .options;
    let selected_commands = selected
        .iter()
        .map(|option| option.command.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut selected_options = options
        .into_iter()
        .filter(|option| selected_commands.contains(&option.command))
        .collect::<Vec<_>>();
    let suppressed = original_count.saturating_sub(selected_options.len());
    if suppressed > 0 {
        if let Some(first) = selected_options.first_mut() {
            first.suppressed_count = suppressed;
            first.effect_label = format!(
                "{} | event card portfolio cap suppressed {suppressed} card offer(s)",
                first.effect_label
            );
        }
    }
    Some(selected_options)
}

fn terminal_no_effect_leave(option: &EventOption) -> bool {
    matches!(option.semantics.action, EventActionKind::Leave)
        && option.semantics.effects.is_empty()
        && option.semantics.constraints.is_empty()
        && option.semantics.terminal
        && matches!(option.semantics.transition, EventOptionTransition::Complete)
}

fn event_option_specific_card(option: &EventOption) -> Option<CardId> {
    option
        .semantics
        .effects
        .iter()
        .find_map(|effect| match effect {
            EventEffect::ObtainCard {
                kind: EventCardKind::Specific(card),
                ..
            }
            | EventEffect::ObtainColorlessCard {
                kind: EventCardKind::Specific(card),
                ..
            } => Some(*card),
            _ => None,
        })
}

fn event_option_upgrade_hint(option: &EventOption) -> Option<u8> {
    option.ui.text.contains('+').then_some(1)
}

fn nloth_trade_is_protected(session: &RunControlSession, option: &EventOption) -> bool {
    let trades_for_gift = option.semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::ObtainRelic {
                kind: EventRelicKind::Specific(crate::content::relics::RelicId::NlothsGift),
                ..
            }
        )
    });
    if !trades_for_gift {
        return false;
    }
    let Some(lost_relic_id) = option
        .semantics
        .effects
        .iter()
        .find_map(|effect| match effect {
            EventEffect::LoseRelic {
                specific: Some(relic),
                ..
            } => Some(*relic),
            _ => None,
        })
    else {
        return false;
    };
    let Some(relic) = session
        .run_state
        .relics
        .iter()
        .find(|relic| relic.id == lost_relic_id)
    else {
        return true;
    };

    crate::ai::relic_trade_policy_v1::nloth_trade_judgment_v1(relic, &session.run_state).protects()
}

fn branch_semantics_for_event_option(option: &EventOption) -> EventOptionBranchSemantics {
    let effect_kind = event_option_effect_kind(&option.semantics).to_string();

    EventOptionBranchSemantics {
        effect_key: format!("event:{effect_kind}:{}", stable_event_option_key(option)),
        effect_kind,
        effect_label: option.ui.text.clone(),
    }
}

fn event_option_effect_kind(semantics: &EventOptionSemantics) -> &'static str {
    if matches!(semantics.action, EventActionKind::Leave)
        || (semantics.terminal
            && semantics.effects.is_empty()
            && matches!(semantics.transition, EventOptionTransition::Complete))
    {
        return "event_leave";
    }
    if matches!(semantics.transition, EventOptionTransition::StartCombat)
        || semantics
            .effects
            .iter()
            .any(|effect| matches!(effect, EventEffect::StartCombat))
        || matches!(semantics.action, EventActionKind::Fight)
    {
        return "event_start_combat";
    }
    match semantics.transition {
        EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard) => {
            return "event_remove_card";
        }
        EventOptionTransition::OpenSelection(EventSelectionKind::UpgradeCard) => {
            return "event_upgrade_card";
        }
        EventOptionTransition::OpenSelection(EventSelectionKind::TransformCard) => {
            return "event_transform_card";
        }
        EventOptionTransition::OpenSelection(EventSelectionKind::DuplicateCard) => {
            return "event_duplicate_card";
        }
        EventOptionTransition::OpenSelection(EventSelectionKind::OfferCard) => {
            return "event_card_reward";
        }
        _ => {}
    }

    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::RemoveCard { .. }))
    {
        return "event_remove_card";
    }
    if semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::UpgradeCard { .. } | EventEffect::UpgradeAllCards
        )
    }) {
        return "event_upgrade_card";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::TransformCard { .. }))
    {
        return "event_transform_card";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::DuplicateCard { .. }))
    {
        return "event_duplicate_card";
    }
    if semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::OfferCards { .. }
                | EventEffect::ObtainCard { .. }
                | EventEffect::ObtainColorlessCard { .. }
        )
    }) {
        return "event_card_reward";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::ObtainRelic { .. }))
    {
        return "event_gain_relic";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::ObtainCurse { .. }))
    {
        return "event_gain_curse";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::ObtainPotion { .. }))
    {
        return "event_gain_potion";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::GainMaxHp(_)))
    {
        return "event_gain_max_hp";
    }
    if semantics
        .effects
        .iter()
        .any(|effect| matches!(effect, EventEffect::Heal(_)))
    {
        return "event_heal";
    }
    if semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::GainGold(_) | EventEffect::GainGoldRange { .. }
        )
    }) {
        return "event_gain_gold";
    }
    if semantics.effects.iter().any(|effect| {
        matches!(
            effect,
            EventEffect::LoseHp(_) | EventEffect::LoseGold(_) | EventEffect::LoseMaxHp(_)
        )
    }) {
        return "event_pay_resource";
    }

    match semantics.action {
        EventActionKind::Continue => "event_continue",
        EventActionKind::Accept => "event_accept",
        EventActionKind::Decline => "event_decline",
        EventActionKind::DeckOperation => "event_deck_operation",
        EventActionKind::Gain => "event_gain",
        EventActionKind::Trade => "event_trade",
        EventActionKind::Special => "event_special",
        EventActionKind::Unknown | EventActionKind::Leave | EventActionKind::Fight => {
            "event_choice"
        }
    }
}

fn stable_event_option_key(option: &EventOption) -> String {
    option
        .semantics
        .effects
        .iter()
        .map(|effect| format!("{effect:?}"))
        .chain(std::iter::once(format!(
            "transition:{:?}",
            option.semantics.transition
        )))
        .collect::<Vec<_>>()
        .join("|")
}
