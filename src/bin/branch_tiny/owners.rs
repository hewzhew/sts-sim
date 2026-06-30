use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, skip_boss_relic_admission, BossRelicAdmission,
};
use sts_simulator::ai::strategy::campfire_upgrade_quality::{
    rank_campfire_upgrades, should_rest_before_smith, CampfireUpgradeTier,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    boss_relic_order_key, evaluate_decision_candidate, CandidateEvaluation, CandidateOrderKey,
    CleanupTarget, DecisionCandidateKind, DecisionPipelineContext,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionOrderKeyV1,
};
use sts_simulator::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId, CardType,
};
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::{CampfireChoice, ClientInput, EngineState};

use super::Owner;

pub(super) type DecisionKey = DecisionCandidateKey;

pub(super) enum OwnerDecision {
    Candidates(Vec<OwnerChoice>),
    Routine(OwnerRoutine),
    Gap(String),
}

pub(super) enum OwnerRoutine {
    Command(RunControlCommand),
    RewardTinyAutomation,
    AdvanceEmptyCampfire,
}

#[derive(Clone)]
pub(super) struct OwnerChoice {
    pub(super) key: Option<DecisionKey>,
    pub(super) action: RunControlCommand,
    pub(super) label: String,
    pub(super) annotation: ChoiceAnnotation,
    pub(super) expansion: OwnerChoiceExpansion,
}

#[derive(Clone)]
pub(super) enum ChoiceAnnotation {
    None,
    Candidate(OwnerCandidateDecision),
    BossRelic(BossRelicAdmission),
}

#[derive(Clone)]
pub(super) struct OwnerCandidateDecision {
    pub(super) evaluation: CandidateEvaluation,
    pub(super) admission: Option<RewardAdmission>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerChoiceExpansion {
    AutoAllowed,
    InspectOnly(&'static str),
}

impl OwnerChoice {
    pub(super) fn auto_expand_allowed(&self) -> bool {
        matches!(self.expansion, OwnerChoiceExpansion::AutoAllowed)
    }

    pub(super) fn inspect_only_reason(&self) -> Option<&'static str> {
        match self.expansion {
            OwnerChoiceExpansion::InspectOnly(reason) => Some(reason),
            OwnerChoiceExpansion::AutoAllowed => None,
        }
    }
}

impl ChoiceAnnotation {
    fn admission(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Candidate(decision) => decision.admission.as_ref(),
            _ => None,
        }
    }

    fn evaluation(&self) -> Option<&CandidateEvaluation> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(&decision.evaluation),
            _ => None,
        }
    }

    fn candidate(&self) -> Option<&OwnerCandidateDecision> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(decision),
            _ => None,
        }
    }

    fn boss_relic(&self) -> Option<&BossRelicAdmission> {
        match self {
            ChoiceAnnotation::BossRelic(admission) => Some(admission),
            _ => None,
        }
    }
}

pub(super) fn owner_decision(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> OwnerDecision {
    match owner {
        Owner::NeowStart => OwnerDecision::Candidates(executable_choices(surface)),
        Owner::CardReward => OwnerDecision::Candidates(card_reward_owner_choices(session, surface)),
        Owner::BossRelic => OwnerDecision::Candidates(boss_relic_owner_choices(session, surface)),
        Owner::ShopTiny => OwnerDecision::Candidates(shop_tiny_owner_choices(session, surface)),
        Owner::Event(_) => event_owner_decision(session, surface),
        Owner::RewardTiny => reward_tiny_owner_decision(surface),
        Owner::Campfire => campfire_owner_decision(session, surface),
    }
}

pub(super) fn executable_choices(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, false)
}

pub(super) fn executable_choices_including_cancel(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    executable_choices_with_cancel(surface, true)
}

fn card_reward_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let deck_plan = DeckPlanSnapshot::from_run_state(&session.run_state);
    let context = DecisionPipelineContext::reward(deck_plan);
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(is_card_reward_choice)
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice, context);
            choice.expansion = card_reward_choice_expansion(&choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let has_mainline_take = choices
        .iter()
        .any(|(_, choice)| is_mainline_card_reward_take(choice));
    choices.sort_by_key(|(index, choice)| {
        (card_reward_choice_rank(choice, has_mainline_take), *index)
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn boss_relic_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let EngineState::BossRelicSelect(_) = &session.engine_state else {
        return Vec::new();
    };
    let mut choices = executable_choices_including_cancel(surface)
        .into_iter()
        .filter(is_boss_relic_choice)
        .map(|mut choice| {
            choice.annotation = boss_relic_annotation_for_choice(session, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (boss_relic_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_owner_choices(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    let context = shop_tiny_context(session);
    let deck = &session.run_state.master_deck;
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_candidate_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion = shop_tiny_choice_expansion(&choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| (shop_tiny_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn event_owner_decision(session: &RunControlSession, surface: &DecisionSurface) -> OwnerDecision {
    match sts_simulator::content::events::owner_policy::event_owner_policy_action(
        &session.engine_state,
        &session.run_state,
    ) {
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::ChooseOption(
            selector,
        )) => visible_event_option_decision(session, surface, &selector),
        Ok(sts_simulator::content::events::owner_policy::EventOwnerAction::SubmitSelection(
            resolution,
        )) => visible_input_decision(surface, ClientInput::SubmitSelection(resolution)),
        Err(err) => OwnerDecision::Gap(format!("{err:?}")),
    }
}

fn visible_event_option_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
    selector: &sts_simulator::content::events::owner_policy::EventOwnerOptionSelector,
) -> OwnerDecision {
    let Some(event) = session.run_state.event_state.as_ref() else {
        return OwnerDecision::Gap("event owner requires event_state".to_string());
    };
    let options = sts_simulator::engine::event_handler::get_event_options(&session.run_state);
    let matches = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let Some(DecisionCandidateKey::EventOption {
                event_id,
                screen,
                option_index,
                ..
            }) = candidate.key
            else {
                return None;
            };
            if event_id != event.id || screen != event.current_screen {
                return None;
            }
            let option = options.get(option_index)?;
            if option.ui.disabled || !selector.matches(&option.semantics) {
                return None;
            }
            candidate.action.executable_command()
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [command] => OwnerDecision::Routine(OwnerRoutine::Command(command.clone())),
        [] => OwnerDecision::Gap(format!("event selector {selector:?} has no visible option")),
        _ => OwnerDecision::Gap(format!(
            "event selector {selector:?} matched {} visible options",
            matches.len()
        )),
    }
}

fn reward_tiny_owner_decision(surface: &DecisionSurface) -> OwnerDecision {
    if let Some(command) = surface
        .view
        .candidates
        .iter()
        .find(|candidate| {
            matches!(
                candidate.key,
                Some(DecisionCandidateKey::CardRewardOpen { .. })
            )
        })
        .and_then(|candidate| candidate.action.executable_command())
    {
        return OwnerDecision::Routine(OwnerRoutine::Command(command));
    }
    OwnerDecision::Routine(OwnerRoutine::RewardTinyAutomation)
}

fn campfire_owner_decision(
    session: &RunControlSession,
    surface: &DecisionSurface,
) -> OwnerDecision {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return OwnerDecision::Gap("Campfire owner requires Campfire engine state".to_string());
    }
    let options =
        sts_simulator::engine::campfire_handler::get_available_options(&session.run_state);
    if options.is_empty() {
        return OwnerDecision::Routine(OwnerRoutine::AdvanceEmptyCampfire);
    }
    match choose_campfire_owner_action(session, surface, &options) {
        Ok(choice) => visible_input_decision(surface, ClientInput::CampfireOption(choice)),
        Err(err) => OwnerDecision::Gap(err),
    }
}

fn visible_input_decision(surface: &DecisionSurface, input: ClientInput) -> OwnerDecision {
    if surface
        .visible_executable_inputs
        .iter()
        .any(|visible| visible == &input)
    {
        OwnerDecision::Routine(OwnerRoutine::Command(RunControlCommand::Input(input)))
    } else {
        OwnerDecision::Gap(format!("routine input {input:?} is not visible"))
    }
}

fn choose_campfire_owner_action(
    session: &RunControlSession,
    surface: &DecisionSurface,
    options: &[CampfireChoice],
) -> Result<CampfireChoice, String> {
    let has_rest = options.contains(&CampfireChoice::Rest);
    let has_smith = options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Smith(_)));

    if has_rest
        && (!has_smith
            || should_rest_before_smith(session.run_state.current_hp, session.run_state.max_hp))
    {
        return Ok(CampfireChoice::Rest);
    }
    if let Some(choice) = best_campfire_toke(session, surface, options) {
        return Ok(choice);
    }
    if has_smith {
        let ranked = rank_campfire_upgrades(&session.run_state.master_deck);
        if let Some(best) = ranked
            .iter()
            .find(|target| target.tier >= CampfireUpgradeTier::Low)
            .or_else(|| ranked.first())
        {
            return Ok(CampfireChoice::Smith(best.deck_index));
        }
    }
    for fallback in [
        CampfireChoice::Dig,
        CampfireChoice::Lift,
        CampfireChoice::Recall,
        CampfireChoice::Rest,
    ] {
        if options.contains(&fallback) {
            return Ok(fallback);
        }
    }
    Err("Campfire owner found no policy action".to_string())
}

fn best_campfire_toke(
    session: &RunControlSession,
    surface: &DecisionSurface,
    options: &[CampfireChoice],
) -> Option<CampfireChoice> {
    if !options
        .iter()
        .any(|choice| matches!(choice, CampfireChoice::Toke(_)))
    {
        return None;
    }
    surface
        .visible_executable_inputs
        .iter()
        .filter_map(|input| {
            let ClientInput::CampfireOption(CampfireChoice::Toke(index)) = input else {
                return None;
            };
            session
                .run_state
                .master_deck
                .get(*index)
                .map(|card| (*index, card.id))
        })
        .min_by_key(|(_, card)| campfire_toke_rank(*card))
        .map(|(index, _)| CampfireChoice::Toke(index))
}

fn campfire_toke_rank(card: CardId) -> u8 {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => 0,
        CardType::Status => 1,
        _ if is_starter_basic(card) => 2,
        _ => 9,
    }
}

fn shop_tiny_candidate_for_choice(
    context: DecisionPipelineContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let kind = match choice.key {
        Some(DecisionCandidateKey::ShopBuyCard {
            card,
            upgrades,
            price,
            ..
        }) => DecisionCandidateKind::ShopBuyCard {
            card,
            upgrades,
            price,
        },
        Some(DecisionCandidateKey::ShopBuyRelic { relic, price, .. }) => {
            DecisionCandidateKind::ShopBuyRelic { relic, price }
        }
        Some(DecisionCandidateKey::ShopBuyPotion { potion, price, .. }) => {
            DecisionCandidateKind::ShopBuyPotion { potion, price }
        }
        Some(DecisionCandidateKey::ShopPurgeCard { card, .. }) => {
            DecisionCandidateKind::ShopPurge {
                target: classify_shop_purge_target(card),
            }
        }
        Some(DecisionCandidateKey::ShopOpenRewards) => DecisionCandidateKind::ShopOpenRewards,
        Some(DecisionCandidateKey::ShopLeave) => DecisionCandidateKind::ShopLeave,
        _ => DecisionCandidateKind::Unsupported,
    };
    candidate_annotation(context, kind, shop_card_admission(deck, kind))
}

fn boss_relic_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            ChoiceAnnotation::BossRelic(assess_boss_relic_admission(&session.run_state, relic))
        }
        Some(DecisionCandidateKey::BossRelicSkip) => {
            ChoiceAnnotation::BossRelic(skip_boss_relic_admission())
        }
        _ => ChoiceAnnotation::None,
    }
}

fn reward_annotation_for_choice(
    session: &RunControlSession,
    choice: &OwnerChoice,
    context: DecisionPipelineContext,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            let deck = &session.run_state.master_deck;
            reward_annotation_with_deck_gate(
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                assess_reward_admission_from_master_deck(deck, card, upgrades),
                context,
            )
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => reward_annotation_with_deck_gate(
            DecisionCandidateKind::CardRewardSkip,
            skip_reward_admission(),
            context,
        ),
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | None => ChoiceAnnotation::None,
        _ => ChoiceAnnotation::None,
    }
}

fn card_reward_choice_expansion(choice: &OwnerChoice) -> OwnerChoiceExpansion {
    owner_expansion_from_evaluation(choice.annotation.evaluation())
}

fn reward_annotation_with_deck_gate(
    kind: DecisionCandidateKind,
    admission: RewardAdmission,
    context: DecisionPipelineContext,
) -> ChoiceAnnotation {
    candidate_annotation(context, kind, Some(admission))
}

fn candidate_annotation(
    context: DecisionPipelineContext,
    kind: DecisionCandidateKind,
    admission: Option<RewardAdmission>,
) -> ChoiceAnnotation {
    let evaluation = evaluate_decision_candidate(context, kind, admission.as_ref());
    ChoiceAnnotation::Candidate(OwnerCandidateDecision {
        admission,
        evaluation,
    })
}

fn card_reward_choice_rank(
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> (u8, CandidateOrderKey, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => (
            0,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(CandidateOrderKey::fallback),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            CandidateOrderKey::optional_skip(has_mainline_take),
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| evaluation.order_key(has_mainline_take))
                .unwrap_or_else(|| CandidateOrderKey::optional_skip(has_mainline_take)),
            choice
                .annotation
                .admission()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (
            2,
            CandidateOrderKey::fallback(),
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
    }
}

fn is_mainline_card_reward_take(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && choice
        .annotation
        .evaluation()
        .is_some_and(|evaluation| evaluation.is_mainline())
}

fn boss_relic_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    let kind = boss_relic_candidate_kind(choice);
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { .. }) => (
            0,
            boss_relic_order_key(kind, choice.annotation.boss_relic()),
        ),
        Some(DecisionCandidateKey::BossRelicSkip) => (
            1,
            boss_relic_order_key(kind, choice.annotation.boss_relic()),
        ),
        _ => (2, CandidateOrderKey::fallback()),
    }
}

fn boss_relic_candidate_kind(choice: &OwnerChoice) -> DecisionCandidateKind {
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { relic, .. }) => {
            DecisionCandidateKind::BossRelicPick { relic }
        }
        Some(DecisionCandidateKey::BossRelicSkip) => DecisionCandidateKind::BossRelicSkip,
        _ => DecisionCandidateKind::Unsupported,
    }
}

fn shop_tiny_context(session: &RunControlSession) -> DecisionPipelineContext {
    DecisionPipelineContext::shop(
        DeckPlanSnapshot::from_run_state(&session.run_state),
        session.run_state.gold,
    )
}

fn shop_tiny_choice_expansion(
    annotation: &ChoiceAnnotation,
    auto_purge_targets: &mut Vec<CleanupTarget>,
) -> OwnerChoiceExpansion {
    let Some(decision) = annotation.candidate() else {
        return shop_tiny_inspect_only();
    };
    match decision.evaluation.candidate.kind {
        DecisionCandidateKind::ShopPurge { target } if decision.evaluation.auto_expands() => {
            if auto_purge_targets.contains(&target) {
                shop_tiny_inspect_only()
            } else {
                auto_purge_targets.push(target);
                owner_expansion_from_evaluation(Some(&decision.evaluation))
            }
        }
        _ => owner_expansion_from_evaluation(Some(&decision.evaluation)),
    }
}

fn shop_tiny_inspect_only() -> OwnerChoiceExpansion {
    OwnerChoiceExpansion::InspectOnly("shop tiny keeps this atomic shop action inspect-only")
}

fn shop_card_admission(
    deck: &[CombatCard],
    kind: DecisionCandidateKind,
) -> Option<RewardAdmission> {
    if let DecisionCandidateKind::ShopBuyCard { card, upgrades, .. } = kind {
        Some(assess_reward_admission_from_master_deck(
            deck, card, upgrades,
        ))
    } else {
        None
    }
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, CandidateOrderKey) {
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => decision.evaluation.auto_order_key(false),
        _ => (u8::MAX, CandidateOrderKey::fallback()),
    }
}

fn owner_expansion_from_evaluation(
    evaluation: Option<&CandidateEvaluation>,
) -> OwnerChoiceExpansion {
    match evaluation {
        Some(evaluation) => match evaluation.inspect_only_reason() {
            None => OwnerChoiceExpansion::AutoAllowed,
            Some(reason) => OwnerChoiceExpansion::InspectOnly(reason),
        },
        None => shop_tiny_inspect_only(),
    }
}

fn classify_shop_purge_target(card: CardId) -> CleanupTarget {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => CleanupTarget::Curse,
        CardType::Status => CleanupTarget::Status,
        _ if is_starter_strike(card) => CleanupTarget::StarterStrike,
        _ if is_starter_defend(card) => CleanupTarget::StarterDefend,
        _ if is_starter_basic(card) => CleanupTarget::OtherStarter,
        _ => CleanupTarget::Other,
    }
}

fn executable_choices_with_cancel(
    surface: &DecisionSurface,
    include_cancel: bool,
) -> Vec<OwnerChoice> {
    surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            let action = candidate.action.executable_command()?;
            if !include_owner_choice_command(&action, include_cancel) {
                return None;
            }
            Some(OwnerChoice {
                key: candidate.key.clone(),
                action,
                label: candidate.label.clone(),
                annotation: ChoiceAnnotation::None,
                expansion: OwnerChoiceExpansion::AutoAllowed,
            })
        })
        .collect()
}

fn include_owner_choice_command(command: &RunControlCommand, include_cancel: bool) -> bool {
    include_cancel || !is_navigation_only_command(command)
}

fn is_navigation_only_command(command: &RunControlCommand) -> bool {
    matches!(command, RunControlCommand::Input(ClientInput::Cancel))
}

fn is_card_reward_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(
            DecisionCandidateKey::CardRewardOpen { .. }
                | DecisionCandidateKey::CardRewardPick { .. }
                | DecisionCandidateKey::CardRewardSingingBowl { .. }
                | DecisionCandidateKey::CardRewardSkip { .. }
        )
    )
}

fn is_boss_relic_choice(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::BossRelicPick { .. } | DecisionCandidateKey::BossRelicSkip)
    )
}

pub(super) fn cleanup_target_label(target: CleanupTarget) -> &'static str {
    match target {
        CleanupTarget::Curse => "curse",
        CleanupTarget::Status => "status",
        CleanupTarget::StarterStrike => "starter-attack",
        CleanupTarget::StarterDefend => "starter-skill",
        CleanupTarget::OtherStarter => "starter",
        CleanupTarget::Other => "other",
    }
}
