use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, boss_relic_admission_order_rank, skip_boss_relic_admission,
    BossRelicAdmission,
};
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission, reward_admission_order_key_v1, skip_reward_admission, RewardAdmission,
    RewardAdmissionClass, RewardAdmissionOrderKeyV1,
};
use sts_simulator::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId, CardType,
};
use sts_simulator::content::potions::{get_potion_definition, PotionId};
use sts_simulator::content::relics::RelicId;
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, DecisionSurface, RunControlCommand, RunControlSession,
};
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
        lane: RewardPlanLane,
    },
    BossRelic(BossRelicAdmission),
    ShopTiny(ShopTinyAnnotation),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RewardPlanLane {
    Mainline,
    Probe,
    Skip,
    Reject,
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

#[derive(Clone, Copy)]
pub(super) enum ShopPurgeTargetKind {
    Curse,
    Status,
    StarterStrike,
    StarterDefend,
    OtherStarter,
    Other,
}

impl ChoiceAnnotation {
    fn reward(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Reward { admission, .. } => Some(admission),
            _ => None,
        }
    }

    fn reward_lane(&self) -> Option<RewardPlanLane> {
        match self {
            ChoiceAnnotation::Reward { lane, .. } => Some(*lane),
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
        Owner::ShopTiny => shop_tiny_owner_choices(surface),
        Owner::Event(_) | Owner::RewardTiny => Vec::new(),
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
    let deck = session
        .run_state
        .master_deck
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(is_card_reward_choice)
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(&deck, &choice);
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

fn shop_tiny_owner_choices(surface: &DecisionSurface) -> Vec<OwnerChoice> {
    let mut choices = executable_choices(surface)
        .into_iter()
        .map(|mut choice| {
            choice.annotation = shop_tiny_annotation_for_choice(&choice);
            choice.expansion = OwnerChoiceExpansion::InspectOnly(
                "shop tiny lists atomic shop actions but does not plan a full shop",
            );
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    choices.sort_by_key(|(index, choice)| (shop_tiny_choice_rank(choice), *index));
    choices.into_iter().map(|(_, choice)| choice).collect()
}

fn shop_tiny_annotation_for_choice(choice: &OwnerChoice) -> ChoiceAnnotation {
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
    ChoiceAnnotation::ShopTiny(annotation)
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
    deck: &[sts_simulator::content::cards::CardId],
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, .. }) => {
            reward_annotation(assess_reward_admission(deck, card))
        }
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => {
            reward_annotation(skip_reward_admission())
        }
        Some(DecisionCandidateKey::CardRewardOpen { .. })
        | Some(DecisionCandidateKey::CardRewardSingingBowl { .. })
        | None => ChoiceAnnotation::None,
        _ => ChoiceAnnotation::None,
    }
}

fn reward_annotation(admission: RewardAdmission) -> ChoiceAnnotation {
    let lane = reward_plan_lane(&admission);
    ChoiceAnnotation::Reward { admission, lane }
}

fn reward_plan_lane(admission: &RewardAdmission) -> RewardPlanLane {
    match admission.class {
        RewardAdmissionClass::ClosesRequirement
        | RewardAdmissionClass::BuildsSupportedPackage
        | RewardAdmissionClass::ImmediateWork
        | RewardAdmissionClass::BurdenedImmediateWork => RewardPlanLane::Mainline,
        RewardAdmissionClass::EngineSeed | RewardAdmissionClass::OpensUnsupportedPayoff => {
            RewardPlanLane::Probe
        }
        RewardAdmissionClass::Skip => RewardPlanLane::Skip,
        RewardAdmissionClass::EmptyOrDeferred => RewardPlanLane::Reject,
    }
}

fn card_reward_choice_rank(
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> (u8, u8, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => {
            (0, 0, RewardAdmissionOrderKeyV1::empty_or_deferred())
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .reward_lane()
                .map(|lane| reward_plan_lane_rank(lane, has_mainline_take))
                .unwrap_or(3),
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (2, 3, RewardAdmissionOrderKeyV1::empty_or_deferred()),
    }
}

fn is_mainline_card_reward_take(choice: &OwnerChoice) -> bool {
    matches!(
        choice.key,
        Some(DecisionCandidateKey::CardRewardPick { .. })
    ) && choice.annotation.reward_lane() == Some(RewardPlanLane::Mainline)
}

fn reward_plan_lane_rank(lane: RewardPlanLane, has_mainline_take: bool) -> u8 {
    match lane {
        RewardPlanLane::Mainline => 0,
        RewardPlanLane::Skip => skip_reward_lane_rank(has_mainline_take),
        RewardPlanLane::Probe => {
            if has_mainline_take {
                2
            } else {
                1
            }
        }
        RewardPlanLane::Reject => 3,
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

fn shop_tiny_choice_rank(choice: &OwnerChoice) -> (u8, u8) {
    match &choice.annotation {
        ChoiceAnnotation::ShopTiny(annotation) => match annotation {
            ShopTinyAnnotation::Purge { target, .. } => match target {
                ShopPurgeTargetKind::Curse => (0, 0),
                ShopPurgeTargetKind::Status => (0, 1),
                ShopPurgeTargetKind::StarterStrike => (1, 0),
                ShopPurgeTargetKind::StarterDefend => (1, 1),
                ShopPurgeTargetKind::OtherStarter => (1, 2),
                ShopPurgeTargetKind::Other => (3, 0),
            },
            ShopTinyAnnotation::BuyCard { .. } => (2, 0),
            ShopTinyAnnotation::BuyRelic { .. } => (2, 1),
            ShopTinyAnnotation::BuyPotion { .. } => (2, 2),
            ShopTinyAnnotation::OpenRewards => (3, 1),
            ShopTinyAnnotation::Leave => (4, 0),
            ShopTinyAnnotation::Unsupported => (5, 0),
        },
        _ => (5, 0),
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

pub(super) fn reward_plan_lane_label(lane: RewardPlanLane) -> &'static str {
    match lane {
        RewardPlanLane::Mainline => "mainline",
        RewardPlanLane::Probe => "probe",
        RewardPlanLane::Skip => "skip",
        RewardPlanLane::Reject => "reject",
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
