use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::ai::decision_tags_v1::{
    combat_shape_change_tags_for_card_v1, TAG_COLLECTOR_ANSWER, TAG_DIGEST_CAPACITY_DRAW,
    TAG_DIGEST_CAPACITY_EXHAUST, TAG_DIGEST_CAPACITY_STATUS, TAG_DIGEST_CAPACITY_TOPDECK,
    TAG_ENGINE_CLOSURE, TAG_STARTUP_ACCESS,
};
use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

use super::types::ShopPurchaseTargetV1;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ShopPurchaseStrategyAnalysisV1 {
    pub evidence: Vec<String>,
}

pub(crate) fn shop_purchase_strategy_analysis_v1(
    target: ShopPurchaseTargetV1,
    run_state: &RunState,
    strategy: &RunStrategySnapshotV2,
) -> ShopPurchaseStrategyAnalysisV1 {
    let mut analysis = ShopPurchaseStrategyAnalysisV1::default();
    match target {
        ShopPurchaseTargetV1::Card { card, .. } => {
            analyze_shop_card(card, run_state, strategy, &mut analysis);
        }
        ShopPurchaseTargetV1::Relic { relic, .. } => {
            analyze_shop_relic(relic, run_state, &mut analysis);
        }
        ShopPurchaseTargetV1::Potion { potion, .. } => {
            analyze_shop_potion(potion, run_state, &mut analysis);
        }
    }
    analysis
}

fn analyze_shop_card(
    card: CardId,
    run_state: &RunState,
    strategy: &RunStrategySnapshotV2,
    analysis: &mut ShopPurchaseStrategyAnalysisV1,
) {
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, 0));

    if run_state.boss_key == Some(EncounterId::Collector) && collector_answer_card(card, &profile) {
        push_evidence(analysis, TAG_COLLECTOR_ANSWER);
    }

    if closes_or_supports_exhaust_engine(card, run_state, strategy) {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
    }

    if startup_access_card(card, &profile) {
        push_evidence(analysis, TAG_STARTUP_ACCESS);
    }

    let shape_tags = combat_shape_change_tags_for_card_v1(card);
    if !shape_tags.is_empty() {
        for tag in shape_tags {
            push_evidence(analysis, tag);
        }
        for tag in combat_shape_digest_capacity_tags(run_state) {
            push_evidence(analysis, tag);
        }
    }
}

fn analyze_shop_relic(
    relic: RelicId,
    run_state: &RunState,
    analysis: &mut ShopPurchaseStrategyAnalysisV1,
) {
    if relic == RelicId::MedicalKit
        && deck_contains_any(run_state, &[CardId::WildStrike, CardId::PowerThrough])
    {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
        push_evidence(analysis, TAG_DIGEST_CAPACITY_STATUS);
    }
    if relic == RelicId::OrangePellets
        && crate::ai::strength_profile_v1::strength_profile_v1(run_state).temporary_bursts > 0
    {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
    }
}

fn analyze_shop_potion(
    potion: PotionId,
    run_state: &RunState,
    analysis: &mut ShopPurchaseStrategyAnalysisV1,
) {
    if run_state.boss_key == Some(EncounterId::Collector) && collector_answer_potion(potion) {
        push_evidence(analysis, TAG_COLLECTOR_ANSWER);
    }
}

fn collector_answer_card(
    card: CardId,
    profile: &crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1,
) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
        || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        || matches!(
            card,
            CardId::FlameBarrier
                | CardId::Impervious
                | CardId::PowerThrough
                | CardId::Shockwave
                | CardId::Cleave
                | CardId::Whirlwind
                | CardId::Immolate
                | CardId::Disarm
                | CardId::Uppercut
        )
}

fn collector_answer_potion(potion: PotionId) -> bool {
    matches!(
        potion,
        PotionId::FirePotion
            | PotionId::FearPotion
            | PotionId::WeakenPotion
            | PotionId::ExplosivePotion
            | PotionId::EssenceOfSteel
            | PotionId::BlockPotion
            | PotionId::EnergyPotion
            | PotionId::PowerPotion
            | PotionId::SkillPotion
            | PotionId::AttackPotion
            | PotionId::DuplicationPotion
    )
}

fn closes_or_supports_exhaust_engine(
    card: CardId,
    run_state: &RunState,
    strategy: &RunStrategySnapshotV2,
) -> bool {
    let has_corruption = deck_contains(run_state, CardId::Corruption);
    let has_dark_embrace = deck_contains(run_state, CardId::DarkEmbrace);
    let has_feel_no_pain = deck_contains(run_state, CardId::FeelNoPain);
    let has_exhaust_generator = deck_contains_any(
        run_state,
        &[
            CardId::TrueGrit,
            CardId::SecondWind,
            CardId::SeverSoul,
            CardId::BurningPact,
            CardId::FiendFire,
            CardId::Corruption,
        ],
    );
    let committed_exhaust_package = matches!(
        strategy.support(StrategyPackageIdV2::ExhaustEngine),
        StrategyPlanSupportV1::Plausible | StrategyPlanSupportV1::Strong
    );

    match card {
        CardId::Corruption => has_dark_embrace || has_feel_no_pain || has_exhaust_generator,
        CardId::DarkEmbrace => has_corruption || has_exhaust_generator || committed_exhaust_package,
        CardId::FeelNoPain => has_corruption || has_exhaust_generator || committed_exhaust_package,
        CardId::BurningPact | CardId::TrueGrit | CardId::SecondWind | CardId::SeverSoul => {
            has_dark_embrace || has_feel_no_pain || committed_exhaust_package
        }
        CardId::Sentinel => has_corruption && (has_dark_embrace || has_feel_no_pain),
        _ => false,
    }
}

fn startup_access_card(
    card: CardId,
    profile: &crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1,
) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
        || matches!(
            card,
            CardId::BattleTrance
                | CardId::Offering
                | CardId::BurningPact
                | CardId::PommelStrike
                | CardId::ShrugItOff
        )
}

fn combat_shape_digest_capacity_tags(run_state: &RunState) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if deck_contains_any(
        run_state,
        &[
            CardId::Evolve,
            CardId::FireBreathing,
            CardId::SecondWind,
            CardId::Corruption,
        ],
    ) || run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MedicalKit)
    {
        tags.push(TAG_DIGEST_CAPACITY_STATUS);
    }
    if deck_contains_any(
        run_state,
        &[
            CardId::BurningPact,
            CardId::Corruption,
            CardId::DarkEmbrace,
            CardId::FeelNoPain,
            CardId::FiendFire,
            CardId::SecondWind,
            CardId::TrueGrit,
        ],
    ) {
        tags.push(TAG_DIGEST_CAPACITY_EXHAUST);
    }
    if deck_contains_any(
        run_state,
        &[
            CardId::BattleTrance,
            CardId::BurningPact,
            CardId::Offering,
            CardId::PommelStrike,
            CardId::ShrugItOff,
        ],
    ) {
        tags.push(TAG_DIGEST_CAPACITY_DRAW);
    }
    if deck_contains_any(run_state, &[CardId::Headbutt, CardId::Warcry]) {
        tags.push(TAG_DIGEST_CAPACITY_TOPDECK);
    }
    tags
}

fn deck_contains(run_state: &RunState, card: CardId) -> bool {
    run_state
        .master_deck
        .iter()
        .any(|deck_card| deck_card.id == card)
}

fn deck_contains_any(run_state: &RunState, cards: &[CardId]) -> bool {
    cards.iter().any(|card| deck_contains(run_state, *card))
}

fn push_evidence(analysis: &mut ShopPurchaseStrategyAnalysisV1, tag: &'static str) {
    if !analysis.evidence.iter().any(|item| item == tag) {
        analysis.evidence.push(tag.to_string());
    }
}
