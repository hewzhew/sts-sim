use super::card_reward::{
    select_card_reward_branch_options_for_session, CardRewardBranchOption,
    CardRewardBranchOptionSource,
};
use crate::ai::deck_mutation_compiler_v1::{
    compile_direct_deck_mutation_plan_candidate_v1, deck_mutation_target_class_for_card_v1,
    DeckMutationPlanCandidateV1, DeckMutationTargetClassV1,
};
use crate::ai::event_policy_v1::{
    build_event_decision_context_v1, plan_event_decision_v1, EventCandidateTierV1,
    EventPolicyActionV1, EventPolicyClassV1, EventPolicyConfigV1,
};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::BranchExperimentChoiceDecisionSignalV1;
use crate::eval::run_control::{build_decision_surface, RunControlSession};
use crate::state::core::{ClientInput, EngineState, RunPendingChoiceReason};
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
    pub(crate) decision_signal: Option<BranchExperimentChoiceDecisionSignalV1>,
    deck_mutation_order_key: Option<(u8, i32)>,
    event_policy_order_key: Option<(u8, i32)>,
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
    let event_id = session.run_state.event_state.as_ref()?.id;
    let event_policy_context =
        build_event_decision_context_v1(&session.run_state, event_id, event_options.clone());
    let surface = build_decision_surface(session);
    let mut branch_options = Vec::new();
    let direct_remove_low_value_available =
        direct_event_remove_card_low_value_available(session, &event_options);

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
        let (card, upgrades) =
            event_option_specific_card_with_upgrades(session, index, event_option);
        let policy_candidate = event_policy_context
            .candidates
            .iter()
            .find(|candidate| candidate.index == index);
        let event_policy_order_key = policy_candidate.map(|candidate| {
            (
                event_candidate_tier_rank(candidate.evaluation.tier),
                -candidate.evaluation.score,
            )
        });
        let event_policy_note = policy_candidate
            .map(|candidate| {
                format!(
                    " | event_eval tier={:?} score={} reasons={}",
                    candidate.evaluation.tier,
                    candidate.evaluation.score,
                    candidate.evaluation.reasons.join("; ")
                )
            })
            .unwrap_or_default();
        let decision_signal = policy_candidate.map(|candidate| {
            super::event_policy_decision_signal_v1(
                candidate.evaluation.tier,
                candidate.evaluation.score,
            )
        });
        let mut branch_option = EventBranchOption {
            label: candidate.label.clone(),
            command: candidate.action.command_hint(),
            card,
            upgrades,
            effect_kind: semantics.effect_kind,
            effect_key: semantics.effect_key,
            effect_label: format!("{}{}", semantics.effect_label, event_policy_note),
            representative_count: 1,
            suppressed_count: 0,
            decision_signal,
            deck_mutation_order_key: None,
            event_policy_order_key,
        };
        if let Some(plan) = compile_direct_event_remove_card_plan(
            session,
            event_option,
            &branch_option.command,
            direct_remove_low_value_available,
        ) {
            apply_direct_event_deck_mutation_plan(&mut branch_option, plan);
        }
        branch_options.push(branch_option);
    }

    if branch_options.is_empty() {
        return None;
    }
    sort_all_direct_deck_mutation_options(&mut branch_options);
    sort_event_options_by_policy(&mut branch_options);
    if let Some(policy_options) =
        event_policy_safe_exit_branch_options(session, &event_options, &branch_options)
    {
        return Some(policy_options);
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

fn event_candidate_tier_rank(tier: EventCandidateTierV1) -> u8 {
    match tier {
        EventCandidateTierV1::Preferred => 0,
        EventCandidateTierV1::Viable => 1,
        EventCandidateTierV1::Risky => 2,
        EventCandidateTierV1::Avoid => 3,
        EventCandidateTierV1::Blocked => 4,
    }
}

fn sort_event_options_by_policy(options: &mut [EventBranchOption]) {
    if options.is_empty()
        || options
            .iter()
            .all(|option| option.deck_mutation_order_key.is_some())
        || !options
            .iter()
            .all(|option| option.event_policy_order_key.is_some())
    {
        return;
    }
    options.sort_by(|left, right| {
        left.event_policy_order_key
            .cmp(&right.event_policy_order_key)
            .then_with(|| left.command.cmp(&right.command))
    });
}

fn sort_all_direct_deck_mutation_options(options: &mut [EventBranchOption]) {
    if options.is_empty()
        || !options
            .iter()
            .all(|option| option.deck_mutation_order_key.is_some())
    {
        return;
    }
    options.sort_by(|left, right| {
        left.deck_mutation_order_key
            .cmp(&right.deck_mutation_order_key)
            .then_with(|| left.command.cmp(&right.command))
    });
}

fn direct_event_remove_card_low_value_available(
    session: &RunControlSession,
    event_options: &[EventOption],
) -> bool {
    event_options.iter().any(|option| {
        let Some(deck_index) = event_option_remove_card_target_deck_index(session, option) else {
            return false;
        };
        let Some(card) = session.run_state.master_deck.get(deck_index) else {
            return false;
        };
        direct_event_remove_target_is_low_value(deck_mutation_target_class_for_card_v1(
            RunPendingChoiceReason::Purge,
            card,
        ))
    })
}

fn direct_event_remove_target_is_low_value(class: DeckMutationTargetClassV1) -> bool {
    matches!(
        class,
        DeckMutationTargetClassV1::Curse
            | DeckMutationTargetClassV1::StarterStrike
            | DeckMutationTargetClassV1::StarterDefend
            | DeckMutationTargetClassV1::Basic
    )
}

fn compile_direct_event_remove_card_plan(
    session: &RunControlSession,
    option: &EventOption,
    command: &str,
    low_value_available: bool,
) -> Option<DeckMutationPlanCandidateV1> {
    let deck_index = event_option_remove_card_target_deck_index(session, option)?;
    let effect_key = format!(
        "event:direct_remove_card:{}",
        stable_event_option_key(option)
    );
    compile_direct_deck_mutation_plan_candidate_v1(
        &session.run_state,
        RunPendingChoiceReason::Purge,
        deck_index,
        command.to_string(),
        "remove_card".to_string(),
        effect_key,
        option.ui.text.clone(),
        low_value_available,
    )
}

fn event_option_remove_card_target_deck_index(
    session: &RunControlSession,
    option: &EventOption,
) -> Option<usize> {
    let target_uuid = option
        .semantics
        .effects
        .iter()
        .find_map(|effect| match effect {
            EventEffect::RemoveCard {
                target_uuid: Some(uuid),
                ..
            } => Some(*uuid),
            _ => None,
        })?;
    session
        .run_state
        .master_deck
        .iter()
        .position(|card| card.uuid == target_uuid)
}

fn apply_direct_event_deck_mutation_plan(
    option: &mut EventBranchOption,
    plan: DeckMutationPlanCandidateV1,
) {
    let decision_signal = Some(super::deck_mutation_decision_signal_v1(&plan));
    let card = plan.step.cards.first();
    let loss = card
        .map(|card| format!(" loss={:?}", card.target_loss.tier))
        .unwrap_or_default();
    option.card = card.map(|card| card.card);
    option.upgrades = card.map(|card| card.upgrades);
    option.effect_kind = plan.step.effect_kind;
    option.effect_key = plan.step.effect_key;
    option.effect_label = format!(
        "{} | deck mutation role={:?}{} confidence={:.2}",
        option.effect_label, plan.role, loss, plan.confidence
    );
    option.representative_count = plan.representative_count;
    option.suppressed_count = plan.suppressed_count;
    option.decision_signal = decision_signal;
    option.deck_mutation_order_key = Some((
        direct_event_deck_mutation_role_rank(plan.role),
        -plan.score_hint,
    ));
}

fn direct_event_deck_mutation_role_rank(
    role: crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1,
) -> u8 {
    match role {
        crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1::PolicyPreferred => 0,
        crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1::SafeAlternative => 1,
        crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1::RiskyExploration => 2,
        crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1::InspectOnly => 3,
        crate::ai::deck_mutation_compiler_v1::DeckMutationPlanRoleV1::Blocked => 4,
    }
}

fn event_policy_safe_exit_branch_options(
    session: &RunControlSession,
    event_options: &[EventOption],
    branch_options: &[EventBranchOption],
) -> Option<Vec<EventBranchOption>> {
    let event_id = session.run_state.event_state.as_ref()?.id;
    let context =
        build_event_decision_context_v1(&session.run_state, event_id, event_options.to_vec());
    let decision = plan_event_decision_v1(&context, &EventPolicyConfigV1::default());
    let EventPolicyActionV1::Pick {
        index,
        reason,
        confidence,
        ..
    } = decision.action
    else {
        return None;
    };
    let candidate = context
        .candidates
        .iter()
        .find(|candidate| candidate.index == index)?;
    if candidate.class != EventPolicyClassV1::SafeExit {
        return None;
    }
    let exits_optional_combat = context.candidates.iter().any(|candidate| {
        candidate.index != index
            && !candidate.disabled
            && candidate.class == EventPolicyClassV1::CombatStart
    });
    if !exits_optional_combat {
        return None;
    }

    let selected_command = format!("event {index}");
    let mut selected = branch_options
        .iter()
        .find(|option| option.command == selected_command)?
        .clone();
    selected.effect_label = format!(
        "{} | event policy safe exit confidence={confidence:.2}: {reason}",
        selected.effect_label
    );
    Some(vec![selected])
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
                decision_signal: None,
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

fn event_option_specific_card_with_upgrades(
    session: &RunControlSession,
    index: usize,
    option: &EventOption,
) -> (Option<CardId>, Option<u8>) {
    let card = event_option_specific_card(option);
    let upgrades =
        card.map(|card| event_option_specific_card_upgrades(session, index, card).unwrap_or(0));
    (card, upgrades)
}

fn event_option_specific_card_upgrades(
    session: &RunControlSession,
    index: usize,
    card: CardId,
) -> Option<u8> {
    let event_state = session.run_state.event_state.as_ref()?;
    match event_state.id {
        crate::state::events::EventId::TheLibrary if event_state.current_screen == 1 => {
            let (entry_card, upgrades) =
                crate::content::events::the_library::library_card_entry_at(
                    &session.run_state,
                    &event_state.extra_data,
                    index,
                )?;
            (entry_card == card).then_some(upgrades)
        }
        crate::state::events::EventId::NoteForYourself if event_state.current_screen == 1 => {
            (session.run_state.note_for_yourself_card == card)
                .then_some(session.run_state.note_for_yourself_upgrades)
        }
        _ => None,
    }
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
