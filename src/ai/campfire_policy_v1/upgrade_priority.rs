use crate::content::cards::{get_card_definition, upgraded_base_cost_override, CardId, CardTag};
use crate::runtime::combat::CombatCard;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

pub fn campfire_smith_upgrade_priority_v1(card: &CombatCard, run_state: &RunState) -> i32 {
    let def = get_card_definition(card.id);
    let upgraded_profile = crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1(
        &RewardCard::new(card.id, card.upgrades.saturating_add(1)),
    );
    let mut priority = 100;

    priority += upgrade_damage_delta(card.id, def.upgrade_damage) * 20;
    priority += def.upgrade_block.max(0) * 18;
    priority += def.upgrade_magic.max(0) * 20;
    priority += cost_reduction_delta(card, def.cost) * 180;

    if upgraded_profile.roles.iter().any(|role| {
        matches!(
            role,
            crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::Vulnerable
                | crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::Weak
                | crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::EnemyStrengthDown
        )
    }) {
        priority += def.upgrade_magic.max(1) * 80;
    }

    if upgraded_profile.roles.iter().any(|role| {
        matches!(
            role,
            crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::CardDraw
                | crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::EnergySource
                | crate::ai::card_reward_policy_v1::CardRewardSemanticRoleV1::ScalingSource
        )
    }) {
        priority += def.upgrade_magic.max(1) * 45;
    }

    if supports_visible_package(card.id, run_state) {
        priority += 90;
    }

    if is_starter_filler(card) {
        priority -= 80;
    }

    priority.max(0)
}

fn upgrade_damage_delta(card: CardId, single_hit_delta: i32) -> i32 {
    let def = get_card_definition(card);
    let hit_count = match card {
        CardId::TwinStrike => 2,
        CardId::SwordBoomerang => def.base_magic.max(1),
        CardId::RiddleWithHoles => 5,
        _ => 1,
    };
    single_hit_delta.max(0).saturating_mul(hit_count)
}

fn cost_reduction_delta(card: &CombatCard, base_cost: i8) -> i32 {
    if base_cost < 0 {
        return 0;
    }
    let mut upgraded = card.clone();
    upgraded.upgrades = upgraded.upgrades.saturating_add(1);
    upgraded_base_cost_override(&upgraded)
        .map(|new_cost| i32::from(base_cost.saturating_sub(new_cost)))
        .unwrap_or(0)
        .max(0)
}

fn supports_visible_package(card: CardId, run_state: &RunState) -> bool {
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

fn is_starter_filler(card: &CombatCard) -> bool {
    let def = get_card_definition(card.id);
    def.tags.contains(&CardTag::StarterStrike) || def.tags.contains(&CardTag::StarterDefend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campfire_upgrade_priority_prefers_bash_over_starter_strike() {
        let run_state = RunState::new(1, 0, false, "Ironclad");

        assert!(
            campfire_smith_upgrade_priority_v1(&CombatCard::new(CardId::Bash, 1), &run_state)
                > campfire_smith_upgrade_priority_v1(
                    &CombatCard::new(CardId::Strike, 2),
                    &run_state
                )
        );
    }
}
