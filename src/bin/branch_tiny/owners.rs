use sts_simulator::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CardBurden, CombatEvent, InstalledRule, Mechanic,
    PayoffRequirement, PlayEffect,
};
use sts_simulator::ai::strategy::boss_relic_admission::{
    assess_boss_relic_admission, boss_relic_admission_order_rank, skip_boss_relic_admission,
    BossRelicAdmission,
};
use sts_simulator::ai::strategy::deck_admission::{
    assess_deck_admission, DeckAdmission, DeckAdmissionContext,
};
use sts_simulator::ai::strategy::deck_construction_pressure::{
    assess_deck_construction_pressure, reward_construction_lane_adjustment,
    ConstructionLaneAdjustment, DeckConstructionContext, DeckConstructionPressure,
};
use sts_simulator::ai::strategy::reward_admission::{
    assess_reward_admission_from_master_deck, reward_admission_order_key_v1, skip_reward_admission,
    RewardAdmission, RewardAdmissionClass, RewardAdmissionOrderKeyV1, RewardAdmissionReason,
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
    survival_pressure: bool,
    deck_admission: DeckAdmissionContext,
    construction_pressure: DeckConstructionPressure,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShopTinyLane {
    CriticalCleanup,
    SurvivalBuy,
    SupportedEngineBuy,
    MainlineBuy,
    StarterCleanup,
    Leave,
    InspectOnly,
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
    let mut choices = executable_choices(surface)
        .into_iter()
        .filter(is_card_reward_choice)
        .map(|mut choice| {
            choice.annotation = reward_annotation_for_choice(session, &choice);
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
            card_reward_choice_rank(session, choice, has_mainline_take),
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
            choice.annotation = shop_tiny_annotation_for_choice(&choice);
            choice
        })
        .enumerate()
        .collect::<Vec<_>>();
    let mut auto_purge_targets = Vec::new();
    for (_, choice) in choices.iter_mut() {
        choice.expansion =
            shop_tiny_choice_expansion(context, deck, &choice.annotation, &mut auto_purge_targets);
    }
    choices.sort_by_key(|(index, choice)| {
        (
            if choice.auto_expand_allowed() { 0 } else { 1 },
            shop_tiny_choice_rank(context, choice, deck),
            *index,
        )
    });
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
    session: &RunControlSession,
    choice: &OwnerChoice,
) -> ChoiceAnnotation {
    match choice.key {
        Some(DecisionCandidateKey::CardRewardPick { card, upgrades, .. }) => {
            let deck = &session.run_state.master_deck;
            reward_annotation_with_deck_gate(
                assess_reward_admission_from_master_deck(deck, card, upgrades),
                deck_admission_context(session),
                deck,
            )
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

fn card_reward_choice_expansion(choice: &OwnerChoice) -> OwnerChoiceExpansion {
    if choice.annotation.reward_lane() == Some(RewardPlanLane::Reject) {
        OwnerChoiceExpansion::InspectOnly("rejected card reward candidate")
    } else {
        OwnerChoiceExpansion::AutoAllowed
    }
}

fn reward_annotation(admission: RewardAdmission) -> ChoiceAnnotation {
    let lane = reward_plan_lane(&admission);
    ChoiceAnnotation::Reward { admission, lane }
}

fn reward_annotation_with_deck_gate(
    admission: RewardAdmission,
    context: DeckAdmissionContext,
    deck: &[CombatCard],
) -> ChoiceAnnotation {
    let deck_admission = assess_deck_admission(deck, context, &admission);
    let pressure =
        assess_deck_construction_pressure(deck, DeckConstructionContext { act: context.act });
    let lane = reward_plan_lane_for_deck_and_pressure(&admission, deck_admission, pressure);
    ChoiceAnnotation::Reward { admission, lane }
}

fn reward_plan_lane(admission: &RewardAdmission) -> RewardPlanLane {
    match admission.class {
        RewardAdmissionClass::ClosesRequirement
        | RewardAdmissionClass::BuildsSupportedPackage
        | RewardAdmissionClass::ImmediateWork
        | RewardAdmissionClass::BurdenedImmediateWork => RewardPlanLane::Mainline,
        RewardAdmissionClass::EngineSeed if installs_combat_rule(admission) => {
            RewardPlanLane::Mainline
        }
        RewardAdmissionClass::EngineSeed | RewardAdmissionClass::OpensUnsupportedPayoff => {
            RewardPlanLane::Probe
        }
        RewardAdmissionClass::Skip => RewardPlanLane::Skip,
        RewardAdmissionClass::EmptyOrDeferred => RewardPlanLane::Reject,
    }
}

fn reward_plan_lane_for_deck(
    admission: &RewardAdmission,
    deck_admission: DeckAdmission,
) -> RewardPlanLane {
    match deck_admission {
        DeckAdmission::Welcome => reward_plan_lane(admission),
        DeckAdmission::Conditional => match reward_plan_lane(admission) {
            RewardPlanLane::Mainline => RewardPlanLane::Probe,
            lane => lane,
        },
        DeckAdmission::Discouraged => match reward_plan_lane(admission) {
            RewardPlanLane::Mainline | RewardPlanLane::Probe => RewardPlanLane::Reject,
            lane => lane,
        },
    }
}

fn reward_plan_lane_for_deck_and_pressure(
    admission: &RewardAdmission,
    deck_admission: DeckAdmission,
    pressure: DeckConstructionPressure,
) -> RewardPlanLane {
    let lane = reward_plan_lane_for_deck(admission, deck_admission);
    adjust_reward_lane_for_construction_pressure(admission, lane, pressure)
}

fn adjust_reward_lane_for_construction_pressure(
    admission: &RewardAdmission,
    lane: RewardPlanLane,
    pressure: DeckConstructionPressure,
) -> RewardPlanLane {
    match reward_construction_lane_adjustment(pressure, admission) {
        ConstructionLaneAdjustment::None => lane,
        ConstructionLaneAdjustment::PromoteToMainline => RewardPlanLane::Mainline,
        ConstructionLaneAdjustment::PromoteOneStep => match lane {
            RewardPlanLane::Probe => RewardPlanLane::Mainline,
            RewardPlanLane::Reject => RewardPlanLane::Probe,
            lane => lane,
        },
        ConstructionLaneAdjustment::SoftDemote => match lane {
            RewardPlanLane::Mainline => RewardPlanLane::Probe,
            RewardPlanLane::Probe => RewardPlanLane::Reject,
            lane => lane,
        },
        ConstructionLaneAdjustment::HardDemote => match lane {
            RewardPlanLane::Mainline | RewardPlanLane::Probe => RewardPlanLane::Reject,
            lane => lane,
        },
    }
}

fn deck_admission_context(session: &RunControlSession) -> DeckAdmissionContext {
    DeckAdmissionContext {
        act: session.run_state.act_num,
        current_hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
    }
}

fn installs_combat_rule(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .iter()
        .any(|reason| matches!(reason, RewardAdmissionReason::Installs(_)))
}

fn card_reward_choice_rank(
    session: &RunControlSession,
    choice: &OwnerChoice,
    has_mainline_take: bool,
) -> (u8, u8, u8, RewardAdmissionOrderKeyV1) {
    match &choice.key {
        Some(DecisionCandidateKey::CardRewardOpen { .. }) => {
            (0, 0, 0, RewardAdmissionOrderKeyV1::empty_or_deferred())
        }
        Some(DecisionCandidateKey::CardRewardPick { .. }) => (
            1,
            choice
                .annotation
                .reward_lane()
                .map(|lane| reward_plan_lane_rank(lane, has_mainline_take))
                .unwrap_or(3),
            survival_pressure_reward_rank(session, choice),
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::empty_or_deferred),
        ),
        Some(DecisionCandidateKey::CardRewardSingingBowl { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            9,
            RewardAdmissionOrderKeyV1::unscored_optional_reward(),
        ),
        Some(DecisionCandidateKey::CardRewardSkip { .. }) => (
            1,
            skip_reward_lane_rank(has_mainline_take),
            9,
            choice
                .annotation
                .reward()
                .map(reward_admission_order_key_v1)
                .unwrap_or_else(RewardAdmissionOrderKeyV1::static_skip_boundary),
        ),
        _ => (2, 3, 9, RewardAdmissionOrderKeyV1::empty_or_deferred()),
    }
}

fn survival_pressure_reward_rank(session: &RunControlSession, choice: &OwnerChoice) -> u8 {
    if !reward_survival_pressure(session) {
        return 5;
    }
    let Some(admission) = choice.annotation.reward() else {
        return 5;
    };
    let provides_block = admission_provides(admission, Mechanic::Block);
    let provides_draw = admission_provides(admission, Mechanic::CardDraw);
    if provides_block && provides_draw {
        return 0;
    }
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Provides(Mechanic::Weak | Mechanic::EnemyStrengthDown)
        )
    }) {
        return 1;
    }
    if provides_block {
        return 2;
    }
    if provides_draw {
        return 3;
    }
    if admission_provides(admission, Mechanic::Energy) {
        return 4;
    }
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::ThinSupport(Mechanic::Strength)
                | RewardAdmissionReason::DamageUses(Mechanic::Strength)
                | RewardAdmissionReason::Opens(PayoffRequirement::WantsMechanic(
                    Mechanic::Strength
                ))
        )
    }) {
        return 8;
    }
    5
}

fn reward_survival_pressure(session: &RunControlSession) -> bool {
    deck_admission_context(session).survival_pressure()
}

fn admission_provides(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(mechanic))
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

fn shop_tiny_context(session: &RunControlSession) -> ShopTinyContext {
    let hp = session.run_state.current_hp;
    let max_hp = session.run_state.max_hp.max(1);
    let deck_admission = deck_admission_context(session);
    ShopTinyContext {
        gold: session.run_state.gold,
        survival_pressure: session.run_state.act_num >= 2 && hp.saturating_mul(3) <= max_hp * 2,
        deck_admission,
        construction_pressure: assess_deck_construction_pressure(
            &session.run_state.master_deck,
            DeckConstructionContext {
                act: deck_admission.act,
            },
        ),
    }
}

fn shop_tiny_choice_expansion(
    context: ShopTinyContext,
    deck: &[CombatCard],
    annotation: &ChoiceAnnotation,
    auto_purge_targets: &mut Vec<ShopPurgeTargetKind>,
) -> OwnerChoiceExpansion {
    let ChoiceAnnotation::ShopTiny(annotation) = annotation else {
        return shop_tiny_inspect_only();
    };
    match annotation {
        ShopTinyAnnotation::Leave | ShopTinyAnnotation::OpenRewards => {
            OwnerChoiceExpansion::AutoAllowed
        }
        ShopTinyAnnotation::Purge { target, .. } if shop_tiny_auto_purge_target(*target) => {
            if auto_purge_targets.contains(target) {
                shop_tiny_inspect_only()
            } else {
                auto_purge_targets.push(*target);
                OwnerChoiceExpansion::AutoAllowed
            }
        }
        ShopTinyAnnotation::BuyCard {
            card,
            upgrades,
            price,
            ..
        } if *price <= context.gold && shop_tiny_auto_buy_card(context, deck, *card, *upgrades) => {
            OwnerChoiceExpansion::AutoAllowed
        }
        _ => shop_tiny_inspect_only(),
    }
}

fn shop_tiny_inspect_only() -> OwnerChoiceExpansion {
    OwnerChoiceExpansion::InspectOnly("shop tiny keeps this atomic shop action inspect-only")
}

fn shop_tiny_auto_purge_target(target: ShopPurgeTargetKind) -> bool {
    matches!(
        target,
        ShopPurgeTargetKind::Curse
            | ShopPurgeTargetKind::Status
            | ShopPurgeTargetKind::StarterStrike
    )
}

fn shop_tiny_auto_buy_card(
    context: ShopTinyContext,
    deck: &[CombatCard],
    card: CardId,
    upgrades: u8,
) -> bool {
    let admission = assess_reward_admission_from_master_deck(deck, card, upgrades);
    !matches!(
        shop_tiny_buy_lane(context, deck, card, upgrades, &admission),
        ShopTinyLane::InspectOnly
    )
}

fn shop_tiny_choice_rank(
    context: ShopTinyContext,
    choice: &OwnerChoice,
    deck: &[CombatCard],
) -> (u8, u8, RewardAdmissionOrderKeyV1) {
    match &choice.annotation {
        ChoiceAnnotation::ShopTiny(annotation) => match annotation {
            ShopTinyAnnotation::Purge { target, .. } => shop_tiny_purge_rank(context, *target),
            ShopTinyAnnotation::Leave => shop_tiny_rank(
                ShopTinyLane::Leave,
                0,
                RewardAdmissionOrderKeyV1::static_skip_boundary(),
            ),
            ShopTinyAnnotation::BuyCard { card, upgrades, .. } => {
                shop_tiny_card_rank(context, deck, *card, *upgrades)
            }
            ShopTinyAnnotation::OpenRewards => shop_tiny_rank(
                ShopTinyLane::CriticalCleanup,
                2,
                RewardAdmissionOrderKeyV1::empty_or_deferred(),
            ),
            ShopTinyAnnotation::BuyRelic { .. } => shop_tiny_rank(
                ShopTinyLane::InspectOnly,
                0,
                RewardAdmissionOrderKeyV1::empty_or_deferred(),
            ),
            ShopTinyAnnotation::BuyPotion { .. } => shop_tiny_rank(
                ShopTinyLane::InspectOnly,
                1,
                RewardAdmissionOrderKeyV1::empty_or_deferred(),
            ),
            ShopTinyAnnotation::Unsupported => shop_tiny_rank(
                ShopTinyLane::InspectOnly,
                2,
                RewardAdmissionOrderKeyV1::empty_or_deferred(),
            ),
        },
        _ => shop_tiny_rank(
            ShopTinyLane::InspectOnly,
            3,
            RewardAdmissionOrderKeyV1::empty_or_deferred(),
        ),
    }
}

fn shop_tiny_purge_rank(
    context: ShopTinyContext,
    target: ShopPurgeTargetKind,
) -> (u8, u8, RewardAdmissionOrderKeyV1) {
    let lane = match target {
        ShopPurgeTargetKind::Curse | ShopPurgeTargetKind::Status => ShopTinyLane::CriticalCleanup,
        ShopPurgeTargetKind::StarterStrike if context.survival_pressure => {
            ShopTinyLane::StarterCleanup
        }
        ShopPurgeTargetKind::StarterStrike => ShopTinyLane::CriticalCleanup,
        ShopPurgeTargetKind::StarterDefend | ShopPurgeTargetKind::OtherStarter => {
            ShopTinyLane::InspectOnly
        }
        ShopPurgeTargetKind::Other => ShopTinyLane::InspectOnly,
    };
    shop_tiny_rank(
        lane,
        purge_target_rank(target),
        RewardAdmissionOrderKeyV1::empty_or_deferred(),
    )
}

fn purge_target_rank(target: ShopPurgeTargetKind) -> u8 {
    match target {
        ShopPurgeTargetKind::Curse => 0,
        ShopPurgeTargetKind::Status => 1,
        ShopPurgeTargetKind::StarterStrike => 2,
        ShopPurgeTargetKind::StarterDefend => 3,
        ShopPurgeTargetKind::OtherStarter => 4,
        ShopPurgeTargetKind::Other => 5,
    }
}

fn shop_tiny_card_rank(
    context: ShopTinyContext,
    deck: &[CombatCard],
    card: CardId,
    upgrades: u8,
) -> (u8, u8, RewardAdmissionOrderKeyV1) {
    let admission = assess_reward_admission_from_master_deck(deck, card, upgrades);
    shop_tiny_rank(
        shop_tiny_buy_lane(context, deck, card, upgrades, &admission),
        shop_tiny_survival_detail_rank(&admission),
        reward_admission_order_key_v1(&admission),
    )
}

fn shop_tiny_buy_lane(
    context: ShopTinyContext,
    deck: &[CombatCard],
    card: CardId,
    upgrades: u8,
    admission: &RewardAdmission,
) -> ShopTinyLane {
    let deck_admission = assess_deck_admission(deck, context.deck_admission, admission);
    let reward_lane = reward_plan_lane_for_deck_and_pressure(
        admission,
        deck_admission,
        context.construction_pressure,
    );
    if reward_lane != RewardPlanLane::Mainline {
        return ShopTinyLane::InspectOnly;
    }
    if shop_tiny_risky_card_buy(card, upgrades, admission) {
        return ShopTinyLane::InspectOnly;
    }
    if context.survival_pressure && shop_tiny_survival_buy(admission) {
        return ShopTinyLane::SurvivalBuy;
    }
    if card == CardId::FeelNoPain && deck_has_exhaust_stream(deck) {
        return ShopTinyLane::SupportedEngineBuy;
    }
    ShopTinyLane::MainlineBuy
}

fn shop_tiny_survival_buy(admission: &RewardAdmission) -> bool {
    admission_provides(admission, Mechanic::EnemyStrengthDown)
        || admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::Block)
}

fn shop_tiny_risky_card_buy(card: CardId, upgrades: u8, admission: &RewardAdmission) -> bool {
    let definition = card_definition_with_upgrades(card, upgrades);
    if definition.burdens.iter().any(|burden| {
        matches!(
            burden,
            CardBurden::RandomExhaust
                | CardBurden::AddsCombatDeckClutter
                | CardBurden::HpCost
                | CardBurden::DrawLockout
                | CardBurden::ExhaustsHand
        )
    }) {
        return true;
    }
    admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::DuplicateBurden(_) | RewardAdmissionReason::DuplicateConcern(_)
        )
    })
}

fn shop_tiny_survival_detail_rank(admission: &RewardAdmission) -> u8 {
    if admission_provides(admission, Mechanic::EnemyStrengthDown) {
        return 0;
    }
    if admission_provides(admission, Mechanic::Weak) {
        return 1;
    }
    if admission_provides(admission, Mechanic::Block)
        && admission_provides(admission, Mechanic::CardDraw)
    {
        return 2;
    }
    if admission_provides(admission, Mechanic::Block) {
        return 3;
    }
    9
}

fn deck_has_exhaust_stream(deck: &[CombatCard]) -> bool {
    deck.iter().any(|card| {
        let definition = card_definition_with_upgrades(card.id, card.upgrades);
        definition
            .play_effects
            .contains(&PlayEffect::EmitEvent(CombatEvent::CardExhausted))
            || definition
                .installed_rules
                .contains(&InstalledRule::SkillCardsCostZeroAndExhaust)
    })
}

fn shop_tiny_rank(
    lane: ShopTinyLane,
    detail: u8,
    order: RewardAdmissionOrderKeyV1,
) -> (u8, u8, RewardAdmissionOrderKeyV1) {
    (shop_tiny_lane_rank(lane), detail, order)
}

fn shop_tiny_lane_rank(lane: ShopTinyLane) -> u8 {
    match lane {
        ShopTinyLane::CriticalCleanup => 0,
        ShopTinyLane::SurvivalBuy => 1,
        ShopTinyLane::SupportedEngineBuy => 2,
        ShopTinyLane::MainlineBuy => 3,
        ShopTinyLane::StarterCleanup => 4,
        ShopTinyLane::Leave => 5,
        ShopTinyLane::InspectOnly => 6,
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
