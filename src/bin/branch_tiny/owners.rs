use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, boss_relic_admission_order_rank, skip_boss_relic_admission,
    BossRelicAdmission,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    evaluate_decision_candidate, CandidateEvaluation, CandidateLane, CleanupTarget,
    DecisionCandidateKind, ExpansionPlan,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionOrderKeyV1,
};
use sts_simulator::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId, CardType,
};
use sts_simulator::content::potions::{get_potion_definition, PotionId};
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::{ClientInput, EngineState};

use super::Owner;

pub(super) type DecisionKey = DecisionCandidateKey;

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
    Reward {
        admission: RewardAdmission,
        evaluation: CandidateEvaluation,
    },
    BossRelic(BossRelicAdmission),
    ShopTiny {
        annotation: ShopTinyAnnotation,
        evaluation: CandidateEvaluation,
    },
}

#[derive(Clone)]
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

#[derive(Clone)]
pub(super) enum ShopTinyAnnotation {
    BuyCard {
        slot: usize,
        card: CardId,
        upgrades: u8,
        price: i32,
    },
    BuyRelic {
        slot: usize,
        relic: RelicId,
        price: i32,
    },
    BuyPotion {
        slot: usize,
        potion: PotionId,
        price: i32,
    },
    Purge {
        card: CardId,
        upgrades: u8,
        target: ShopPurgeTargetKind,
    },
    OpenRewards,
    Leave,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShopPurgeTargetKind {
    Curse,
    Status,
    StarterStrike,
    StarterDefend,
    OtherStarter,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ShopTinyContext {
    gold: i32,
    deck_plan: DeckPlanSnapshot,
}

impl ChoiceAnnotation {
    fn reward(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Reward { admission, .. } => Some(admission),
            _ => None,
        }
    }

    fn evaluation(&self) -> Option<&CandidateEvaluation> {
        match self {
            ChoiceAnnotation::Reward { evaluation, .. }
            | ChoiceAnnotation::ShopTiny { evaluation, .. } => Some(evaluation),
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

pub(super) fn owner_choices(
    session: &RunControlSession,
    owner: Owner,
    surface: &DecisionSurface,
) -> Vec<OwnerChoice> {
    match owner {
        Owner::NeowStart => executable_choices(surface),
        Owner::CardReward => card_reward_owner_choices(session, surface),
        Owner::BossRelic => boss_relic_owner_choices(session, surface),
        Owner::ShopTiny => shop_tiny_owner_choices(session, surface),
        Owner::Event(_) | Owner::RewardTiny | Owner::Campfire => Vec::new(),
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
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(is_card_reward_choice)
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice, deck_plan);
            choice.expansion = card_reward_choice_expansion(&choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let has_mainline_take = choices
        .iter()
        .any(|(_, choice)| is_mainline_card_reward_take(choice));
    choices.sort_by_key(|(index, choice)| {
        (
            card_reward_choice_rank(choice, has_mainline_take, deck_plan),
            *index,
        )
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
            choice.annotation = shop_tiny_annotation_for_choice(context, deck, &choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion =
            shop_tiny_choice_expansion(context, &choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| {
        (
            if choice.auto_expand_allowed() { 0 } else { 1 },
            shop_tiny_choice_rank(choice),
            *index,
        )
    });
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_annotation_for_choice(
    context: ShopTinyContext,
    deck: &[CombatCard],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    let annotation = match choice.key {
        Some(DecisionCandidateKey::ShopBuyCard {
            shop_slot,
            card,
            upgrades,
            price,
        }) => ShopTinyAnnotation::BuyCard {
            slot: shop_slot,
            card,
            upgrades,
            price,
        },
        Some(DecisionCandidateKey::ShopBuyRelic {
            shop_slot,
            relic,
            price,
        }) => ShopTinyAnnotation::BuyRelic {
            slot: shop_slot,
            relic,
            price,
        },
        Some(DecisionCandidateKey::ShopBuyPotion {
            shop_slot,
            potion,
            price,
        }) => ShopTinyAnnotation::BuyPotion {
            slot: shop_slot,
            potion,
            price,
        },
        Some(DecisionCandidateKey::ShopPurgeCard { card, upgrades, .. }) => {
            ShopTinyAnnotation::Purge {
                card,
                upgrades,
                target: classify_shop_purge_target(card),
            }
        }
        Some(DecisionCandidateKey::ShopOpenRewards) => ShopTinyAnnotation::OpenRewards,
        Some(DecisionCandidateKey::ShopLeave) => ShopTinyAnnotation::Leave,
        _ => ShopTinyAnnotation::Unsupported,
    };
    let evaluation = shop_tiny_evaluation_for_annotation(context, deck, &annotation);
    ChoiceAnnotation::ShopTiny {
        annotation,
        evaluation,
    }
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
    deck_plan: DeckPlanSnapshot,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            let deck = &session.run_state.master_deck;
            reward_annotation_with_deck_gate(
                DecisionCandidateKind::CardRewardPick { card, upgrades },
                assess_reward_admission_from_master_deck(deck, card, upgrades),
                deck_plan,
            )
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => reward_annotation_with_deck_gate(
            DecisionCandidateKind::CardRewardSkip,
            skip_reward_admission(),
            deck_plan,
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
    deck_plan: DeckPlanSnapshot,
) -> ChoiceAnnotation {
    let evaluation = evaluate_decision_candidate(deck_plan, kind, Some(&admission));
    ChoiceAnnotation::Reward {
        admission,
        evaluation,
    }
}

fn card_reward_choice_rank(
    choice: &OwnerChoice,
    has_mainline_take: bool,
    _deck_plan: DeckPlanSnapshot,
) -> (u8, u8, i32, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => {
            (0, 0, 0, RewardAdmissionOrderKeyV1::empty_or_deferred())
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .evaluation()
                .map(|evaluation| candidate_lane_rank(evaluation.lane, has_mainline_take))
                .unwrap_or(3),
            choice
                .annotation
                .evaluation()
                .map(|evaluation| -evaluation.total_score())
                .unwrap_or(0),
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            0,
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            0,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (2, 3, 0, RewardAdmissionOrderKeyV1::empty_or_deferred()),
    }
}

fn is_mainline_card_reward_take(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && choice
        .annotation
        .evaluation()
        .is_some_and(|evaluation| evaluation.lane == CandidateLane::Mainline)
}

fn candidate_lane_rank(lane: CandidateLane, has_mainline_take: bool) -> u8 {
    match lane {
        CandidateLane::Mainline => 0,
        CandidateLane::Skip => skip_reward_lane_rank(has_mainline_take),
        CandidateLane::Probe => {
            if has_mainline_take {
                2
            } else {
                1
            }
        }
        CandidateLane::Reject => 3,
    }
}

fn skip_reward_lane_rank(has_mainline_take: bool) -> u8 {
    if has_mainline_take {
        1
    } else {
        0
    }
}

fn boss_relic_choice_rank(choice: &OwnerChoice) -> (u8, u8) {
    let skip_order = boss_relic_admission_order_rank(&skip_boss_relic_admission());
    match choice.key {
        Some(DecisionCandidateKey::BossRelicPick { .. }) => (
            0,
            choice
                .annotation
                .boss_relic()
                .map(boss_relic_admission_order_rank)
                .unwrap_or(skip_order),
        ),
        Some(DecisionCandidateKey::BossRelicSkip) => (1, skip_order),
        _ => (2, skip_order),
    }
}

fn shop_tiny_context(session: &RunControlSession) -> ShopTinyContext {
    ShopTinyContext {
        gold: session.run_state.gold,
        deck_plan: DeckPlanSnapshot::from_run_state(&session.run_state),
    }
}

fn shop_tiny_choice_expansion(
    context: ShopTinyContext,
    annotation: &ChoiceAnnotation,
    auto_purge_targets: &mut Vec<ShopPurgeTargetKind>,
) -> OwnerChoiceExpansion {
    let ChoiceAnnotation::ShopTiny {
        annotation,
        evaluation,
    } = annotation
    else {
        return shop_tiny_inspect_only();
    };
    match annotation {
        ShopTinyAnnotation::Purge { target, .. }
            if matches!(evaluation.expansion, ExpansionPlan::Auto) =>
        {
            if auto_purge_targets.contains(target) {
                shop_tiny_inspect_only()
            } else {
                auto_purge_targets.push(*target);
                owner_expansion_from_evaluation(Some(evaluation))
            }
        }
        ShopTinyAnnotation::BuyCard { price, .. } if *price > context.gold => {
            OwnerChoiceExpansion::InspectOnly("shop card is unaffordable")
        }
        _ => owner_expansion_from_evaluation(Some(evaluation)),
    }
}

fn shop_tiny_inspect_only() -> OwnerChoiceExpansion {
    OwnerChoiceExpansion::InspectOnly("shop tiny keeps this atomic shop action inspect-only")
}

fn shop_tiny_evaluation_for_annotation(
    context: ShopTinyContext,
    deck: &[CombatCard],
    annotation: &ShopTinyAnnotation,
) -> CandidateEvaluation {
    match annotation {
        ShopTinyAnnotation::BuyCard {
            card,
            upgrades,
            price,
            ..
        } => {
            let admission = assess_reward_admission_from_master_deck(deck, *card, *upgrades);
            evaluate_decision_candidate(
                context.deck_plan,
                DecisionCandidateKind::ShopBuyCard {
                    card: *card,
                    upgrades: *upgrades,
                    price: *price,
                },
                Some(&admission),
            )
        }
        ShopTinyAnnotation::Purge { target, .. } => evaluate_decision_candidate(
            context.deck_plan,
            DecisionCandidateKind::ShopPurge {
                target: cleanup_target_from_shop_purge(*target),
            },
            None,
        ),
        ShopTinyAnnotation::OpenRewards => evaluate_decision_candidate(
            context.deck_plan,
            DecisionCandidateKind::ShopOpenRewards,
            None,
        ),
        ShopTinyAnnotation::Leave => {
            evaluate_decision_candidate(context.deck_plan, DecisionCandidateKind::ShopLeave, None)
        }
        ShopTinyAnnotation::BuyRelic { .. }
        | ShopTinyAnnotation::BuyPotion { .. }
        | ShopTinyAnnotation::Unsupported => {
            evaluate_decision_candidate(context.deck_plan, DecisionCandidateKind::Unsupported, None)
        }
    }
}

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, i32, u8) {
    match &choice.annotation {
        ChoiceAnnotation::ShopTiny {
            annotation,
            evaluation,
        } => (
            candidate_lane_rank(evaluation.lane, false),
            -evaluation.total_score(),
            shop_tiny_detail_rank(annotation),
        ),
        _ => (6, 0, 9),
    }
}

fn owner_expansion_from_evaluation(
    evaluation: Option<&CandidateEvaluation>,
) -> OwnerChoiceExpansion {
    match evaluation.map(|evaluation| evaluation.expansion) {
        Some(ExpansionPlan::Auto) => OwnerChoiceExpansion::AutoAllowed,
        Some(ExpansionPlan::InspectOnly(reason)) => OwnerChoiceExpansion::InspectOnly(reason),
        None => shop_tiny_inspect_only(),
    }
}

fn shop_tiny_detail_rank(annotation: &ShopTinyAnnotation) -> u8 {
    match annotation {
        ShopTinyAnnotation::Purge { target, .. } => match target {
            ShopPurgeTargetKind::Curse => 0,
            ShopPurgeTargetKind::Status => 1,
            ShopPurgeTargetKind::StarterStrike => 2,
            ShopPurgeTargetKind::StarterDefend => 3,
            ShopPurgeTargetKind::OtherStarter => 4,
            ShopPurgeTargetKind::Other => 5,
        },
        ShopTinyAnnotation::OpenRewards => 1,
        ShopTinyAnnotation::BuyCard { .. } => 2,
        ShopTinyAnnotation::BuyRelic { .. } => 3,
        ShopTinyAnnotation::BuyPotion { .. } => 4,
        ShopTinyAnnotation::Leave => 5,
        ShopTinyAnnotation::Unsupported => 9,
    }
}

fn cleanup_target_from_shop_purge(target: ShopPurgeTargetKind) -> CleanupTarget {
    match target {
        ShopPurgeTargetKind::Curse => CleanupTarget::Curse,
        ShopPurgeTargetKind::Status => CleanupTarget::Status,
        ShopPurgeTargetKind::StarterStrike => CleanupTarget::StarterStrike,
        ShopPurgeTargetKind::StarterDefend => CleanupTarget::StarterDefend,
        ShopPurgeTargetKind::OtherStarter => CleanupTarget::OtherStarter,
        ShopPurgeTargetKind::Other => CleanupTarget::Other,
    }
}

fn classify_shop_purge_target(card: CardId) -> ShopPurgeTargetKind {
    let definition = get_card_definition(card);
    match definition.card_type {
        CardType::Curse => ShopPurgeTargetKind::Curse,
        CardType::Status => ShopPurgeTargetKind::Status,
        _ if is_starter_strike(card) => ShopPurgeTargetKind::StarterStrike,
        _ if is_starter_defend(card) => ShopPurgeTargetKind::StarterDefend,
        _ if is_starter_basic(card) => ShopPurgeTargetKind::OtherStarter,
        _ => ShopPurgeTargetKind::Other,
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

pub(super) fn render_shop_tiny_annotation_compact(annotation: &ShopTinyAnnotation) -> String {
    match annotation {
        ShopTinyAnnotation::BuyCard {
            slot,
            card,
            upgrades,
            price,
        } => format!("BuyCard slot={slot} {card:?}+{upgrades} {price}g"),
        ShopTinyAnnotation::BuyRelic { slot, relic, price } => {
            format!("BuyRelic slot={slot} {relic:?} {price}g")
        }
        ShopTinyAnnotation::BuyPotion {
            slot,
            potion,
            price,
        } => {
            let name = get_potion_definition(*potion).name;
            format!("BuyPotion slot={slot} {name} {price}g")
        }
        ShopTinyAnnotation::Purge {
            card,
            upgrades,
            target,
        } => format!("Purge {card:?}+{upgrades} {}", purge_target_label(*target)),
        ShopTinyAnnotation::OpenRewards => "OpenRewards".to_string(),
        ShopTinyAnnotation::Leave => "Leave".to_string(),
        ShopTinyAnnotation::Unsupported => "Unsupported typed-gap".to_string(),
    }
}

fn purge_target_label(target: ShopPurgeTargetKind) -> &'static str {
    match target {
        ShopPurgeTargetKind::Curse => "curse",
        ShopPurgeTargetKind::Status => "status",
        ShopPurgeTargetKind::StarterStrike => "starter-attack",
        ShopPurgeTargetKind::StarterDefend => "starter-skill",
        ShopPurgeTargetKind::OtherStarter => "starter",
        ShopPurgeTargetKind::Other => "other",
    }
}
