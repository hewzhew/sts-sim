use crate::ai::boss_mechanics_v1::{
    boss_mechanic_pressure_profile_v1, relic_creates_enemy_strength_pressure_v1,
    BossMechanicRedFlagV1,
};
use crate::ai::card_reward_policy_v1::{
    card_facts, card_reward_semantic_profile_v1, CardRewardSemanticProfileV1,
    CardRewardSemanticRoleV1,
};
use crate::ai::card_semantics_v1::{potion_acquisition_traits_v1, PotionAcquisitionTraitV1};
use crate::ai::decision_tags_v1::{
    combat_shape_change_tags_for_card_v1, TAG_BOSS_PRESSURE_ENEMY_STRENGTH_MULTI_HIT_RISK,
    TAG_COLLECTOR_ANSWER, TAG_DIGEST_CAPACITY_DRAW, TAG_DIGEST_CAPACITY_EXHAUST,
    TAG_DIGEST_CAPACITY_STATUS, TAG_DIGEST_CAPACITY_TOPDECK, TAG_ENGINE_CLOSURE,
    TAG_STARTUP_ACCESS,
};
use crate::ai::deck_startup_profile_v1::{
    deck_startup_profile_v1, startup_energy_candidate_discounted_by_snecko_v1,
};
use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::ai::strength_profile_v1::StrengthProfileV1;
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
    pub risks: Vec<String>,
}

pub(crate) fn shop_purchase_strategy_analysis_v1(
    target: ShopPurchaseTargetV1,
    run_state: &RunState,
    strategy: &RunStrategySnapshotV2,
    strength: &StrengthProfileV1,
) -> ShopPurchaseStrategyAnalysisV1 {
    let mut analysis = ShopPurchaseStrategyAnalysisV1::default();
    match target {
        ShopPurchaseTargetV1::Card { card, .. } => {
            analyze_shop_card(card, run_state, strategy, &mut analysis);
        }
        ShopPurchaseTargetV1::Relic { relic, .. } => {
            analyze_shop_relic(relic, run_state, strength, &mut analysis);
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
    let startup = deck_startup_profile_v1(run_state);

    if run_state.boss_key == Some(EncounterId::Collector) && collector_answer_card(card, &profile) {
        push_evidence(analysis, TAG_COLLECTOR_ANSWER);
    }

    if closes_or_supports_exhaust_engine(card, run_state, strategy) {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
    }

    if startup_access_card(card, &profile, &startup) {
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
    strength: &StrengthProfileV1,
    analysis: &mut ShopPurchaseStrategyAnalysisV1,
) {
    let mechanics = crate::ai::card_semantics_v1::relic_mechanics_profile_v1(relic);
    if mechanics.core_defense_or_survival {
        push_evidence(analysis, "shop_relic_core_defense_or_survival");
    }
    if mechanics.core_card_access {
        push_evidence(analysis, "shop_relic_core_card_access");
    }
    if relic == RelicId::MedicalKit
        && deck_has_role(run_state, CardRewardSemanticRoleV1::StatusGenerator)
    {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
        push_evidence(analysis, TAG_DIGEST_CAPACITY_STATUS);
    }
    if relic == RelicId::OrangePellets && strength.temporary_bursts > 0 {
        push_evidence(analysis, TAG_ENGINE_CLOSURE);
    }
    if relic_purchase_creates_boss_enemy_strength_risk(relic, run_state) {
        push_risk(analysis, TAG_BOSS_PRESSURE_ENEMY_STRENGTH_MULTI_HIT_RISK);
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

fn collector_answer_card(card: CardId, profile: &CardRewardSemanticProfileV1) -> bool {
    let facts = card_facts(&RewardCard::new(card, 0));
    has_role(profile, CardRewardSemanticRoleV1::AoeDamage)
        || has_role(profile, CardRewardSemanticRoleV1::Weak)
        || has_role(profile, CardRewardSemanticRoleV1::EnemyStrengthDown)
        || facts.block >= 12
}

fn collector_answer_potion(potion: PotionId) -> bool {
    let traits = potion_acquisition_traits_v1(potion);
    traits.iter().any(|trait_| {
        matches!(
            trait_,
            PotionAcquisitionTraitV1::CombatDamage
                | PotionAcquisitionTraitV1::CombatBlock
                | PotionAcquisitionTraitV1::DebuffSetup
                | PotionAcquisitionTraitV1::EnergyBurst
                | PotionAcquisitionTraitV1::CardAccess
                | PotionAcquisitionTraitV1::ActionAmplifier
        )
    })
}

fn closes_or_supports_exhaust_engine(
    card: CardId,
    run_state: &RunState,
    strategy: &RunStrategySnapshotV2,
) -> bool {
    let profile = card_reward_semantic_profile_v1(&RewardCard::new(card, 0));
    let deck_has_exhaust_generator =
        deck_has_role(run_state, CardRewardSemanticRoleV1::ExhaustGenerator);
    let deck_has_exhaust_payoff = deck_has_role(run_state, CardRewardSemanticRoleV1::ExhaustPayoff);
    let committed_exhaust_package = matches!(
        strategy.support(StrategyPackageIdV2::ExhaustEngine),
        StrategyPlanSupportV1::Plausible | StrategyPlanSupportV1::Strong
    );

    (has_role(&profile, CardRewardSemanticRoleV1::ExhaustPayoff)
        && (deck_has_exhaust_generator || committed_exhaust_package))
        || (has_role(&profile, CardRewardSemanticRoleV1::ExhaustGenerator)
            && (deck_has_exhaust_payoff || committed_exhaust_package))
}

fn startup_access_card(
    card: CardId,
    profile: &CardRewardSemanticProfileV1,
    startup: &crate::ai::deck_startup_profile_v1::DeckStartupProfileV1,
) -> bool {
    if startup.has_snecko_eye && startup_energy_candidate_discounted_by_snecko_v1(startup, card) {
        return false;
    }

    has_role(profile, CardRewardSemanticRoleV1::CardDraw)
        || has_role(profile, CardRewardSemanticRoleV1::CycleAccess)
        || has_role(profile, CardRewardSemanticRoleV1::EnergySource)
}

fn combat_shape_digest_capacity_tags(run_state: &RunState) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if deck_has_any_role(
        run_state,
        &[
            CardRewardSemanticRoleV1::StatusPayoff,
            CardRewardSemanticRoleV1::ExhaustGenerator,
        ],
    ) || run_state
        .relics
        .iter()
        .any(|relic| relic.id == RelicId::MedicalKit)
    {
        tags.push(TAG_DIGEST_CAPACITY_STATUS);
    }
    if deck_has_any_role(
        run_state,
        &[
            CardRewardSemanticRoleV1::ExhaustGenerator,
            CardRewardSemanticRoleV1::ExhaustPayoff,
        ],
    ) {
        tags.push(TAG_DIGEST_CAPACITY_EXHAUST);
    }
    if deck_has_any_role(
        run_state,
        &[
            CardRewardSemanticRoleV1::CardDraw,
            CardRewardSemanticRoleV1::CycleAccess,
            CardRewardSemanticRoleV1::EnergySource,
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

fn deck_has_role(run_state: &RunState, role: CardRewardSemanticRoleV1) -> bool {
    run_state.master_deck.iter().any(|deck_card| {
        let profile = card_reward_semantic_profile_v1(&RewardCard::new(deck_card.id, 0));
        has_role(&profile, role)
    })
}

fn deck_has_any_role(run_state: &RunState, roles: &[CardRewardSemanticRoleV1]) -> bool {
    roles.iter().any(|role| deck_has_role(run_state, *role))
}

fn has_role(profile: &CardRewardSemanticProfileV1, role: CardRewardSemanticRoleV1) -> bool {
    profile.roles.contains(&role)
}

fn push_evidence(analysis: &mut ShopPurchaseStrategyAnalysisV1, tag: &'static str) {
    if !analysis.evidence.iter().any(|item| item == tag) {
        analysis.evidence.push(tag.to_string());
    }
}

fn push_risk(analysis: &mut ShopPurchaseStrategyAnalysisV1, tag: &'static str) {
    if !analysis.risks.iter().any(|item| item == tag) {
        analysis.risks.push(tag.to_string());
    }
}

fn relic_purchase_creates_boss_enemy_strength_risk(relic: RelicId, run_state: &RunState) -> bool {
    if !relic_creates_enemy_strength_pressure_v1(relic) {
        return false;
    }
    let Some(boss) = run_state.boss_key else {
        return false;
    };
    let mut hypothetical = run_state.clone();
    if !hypothetical.relics.iter().any(|owned| owned.id == relic) {
        hypothetical
            .relics
            .push(crate::content::relics::RelicState::new(relic));
    }
    boss_mechanic_pressure_profile_v1(&hypothetical, boss)
        .has_red_flag(BossMechanicRedFlagV1::EnemyStrengthMultiHitRisk)
}
