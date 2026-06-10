use crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1;
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::runtime::combat::CombatCard;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

pub fn run_choice_duplicate_priority_v1(card: &CombatCard, run_state: &RunState) -> i32 {
    let def = get_card_definition(card.id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return -10_000;
    }

    let profile = crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1(
        &RewardCard::new(card.id, card.upgrades),
    );
    let mut priority = 100;

    for role in profile.roles {
        priority += match role {
            CardRewardSemanticRoleV1::CardDraw => 180,
            CardRewardSemanticRoleV1::EnergySource => 170,
            CardRewardSemanticRoleV1::EnemyStrengthDown
            | CardRewardSemanticRoleV1::Weak
            | CardRewardSemanticRoleV1::Vulnerable => 160,
            CardRewardSemanticRoleV1::ScalingSource => 150,
            CardRewardSemanticRoleV1::Block
            | CardRewardSemanticRoleV1::BlockRetention
            | CardRewardSemanticRoleV1::BlockMultiplier => 130,
            CardRewardSemanticRoleV1::ExhaustGenerator => 120,
            CardRewardSemanticRoleV1::PackagePayoff
            | CardRewardSemanticRoleV1::ExhaustPayoff
            | CardRewardSemanticRoleV1::StatusPayoff
            | CardRewardSemanticRoleV1::BlockPayoff
            | CardRewardSemanticRoleV1::StrengthPayoff
            | CardRewardSemanticRoleV1::StrikePayoff
            | CardRewardSemanticRoleV1::UpgradePayoff
            | CardRewardSemanticRoleV1::SelfDamagePayoff => 90,
            CardRewardSemanticRoleV1::FrontloadDamage => 80,
            CardRewardSemanticRoleV1::AoeDamage => 60,
            CardRewardSemanticRoleV1::RandomOutput => -50,
            CardRewardSemanticRoleV1::ConditionalPlayability => -80,
            CardRewardSemanticRoleV1::UnsupportedMechanics => -120,
            CardRewardSemanticRoleV1::StatusGenerator => -40,
        };
    }

    priority += high_impact_duplicate_bonus(card.id);
    if card.upgrades > 0 {
        priority += 60;
    }
    if supports_existing_deck_package(card.id, run_state) {
        priority += 100;
    }
    if def.tags.contains(&CardTag::StarterStrike) || def.tags.contains(&CardTag::StarterDefend) {
        priority -= 500;
    }
    if def.rarity == crate::content::cards::CardRarity::Basic {
        priority -= 300;
    }

    priority
}

fn high_impact_duplicate_bonus(card: CardId) -> i32 {
    match card {
        CardId::Offering | CardId::Corruption => 520,
        CardId::Shockwave | CardId::Disarm => 480,
        CardId::Impervious | CardId::DemonForm | CardId::Feed | CardId::Reaper => 400,
        CardId::FiendFire | CardId::DarkEmbrace | CardId::FeelNoPain => 360,
        CardId::BattleTrance | CardId::BurningPact | CardId::PowerThrough => 300,
        CardId::FlameBarrier | CardId::Entrench | CardId::Barricade | CardId::BodySlam => 250,
        CardId::Uppercut | CardId::ShrugItOff | CardId::PommelStrike | CardId::TrueGrit => 220,
        CardId::Armaments | CardId::SpotWeakness | CardId::Inflame => 180,
        _ => 0,
    }
}

fn supports_existing_deck_package(card: CardId, run_state: &RunState) -> bool {
    match card {
        CardId::BodySlam | CardId::Entrench | CardId::Barricade => deck_has_any(
            run_state,
            &[CardId::BodySlam, CardId::Entrench, CardId::Barricade],
        ),
        CardId::HeavyBlade | CardId::LimitBreak => deck_has_any(
            run_state,
            &[
                CardId::Inflame,
                CardId::SpotWeakness,
                CardId::DemonForm,
                CardId::Flex,
            ],
        ),
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption => deck_has_any(
            run_state,
            &[
                CardId::FeelNoPain,
                CardId::DarkEmbrace,
                CardId::Corruption,
                CardId::SecondWind,
                CardId::FiendFire,
                CardId::TrueGrit,
            ],
        ),
        CardId::Evolve | CardId::FireBreathing => deck_has_any(
            run_state,
            &[
                CardId::Evolve,
                CardId::FireBreathing,
                CardId::PowerThrough,
                CardId::WildStrike,
                CardId::RecklessCharge,
            ],
        ),
        _ => false,
    }
}

fn deck_has_any(run_state: &RunState, cards: &[CardId]) -> bool {
    run_state
        .master_deck
        .iter()
        .any(|card| cards.contains(&card.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_priority_prefers_shockwave_over_starter_cards() {
        let run_state = RunState::new(1, 0, false, "Ironclad");

        assert!(
            run_choice_duplicate_priority_v1(&CombatCard::new(CardId::Shockwave, 1), &run_state)
                > run_choice_duplicate_priority_v1(&CombatCard::new(CardId::Strike, 2), &run_state)
        );
    }
}
