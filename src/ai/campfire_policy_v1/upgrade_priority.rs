use crate::content::cards::{get_card_definition, upgraded_base_cost_override, CardId, CardTag};
use crate::content::monsters::factory::EncounterId;
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

    priority += boss_mechanic_upgrade_delta(card.id, run_state);

    if is_starter_filler(card) {
        priority -= 80;
    }

    priority.max(0)
}

pub fn campfire_smith_upgrade_strategy_tag_v1(
    card: &CombatCard,
    run_state: &RunState,
) -> Option<&'static str> {
    match run_state.boss_key {
        Some(EncounterId::Automaton) if run_state.act_num == 2 => {
            automaton_upgrade_strategy_tag(card.id)
        }
        Some(EncounterId::TheChamp) if run_state.act_num == 2 => {
            champ_upgrade_strategy_tag(card.id, run_state)
        }
        _ => None,
    }
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
        CardId::HeavyBlade | CardId::LimitBreak => {
            let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
            startup.persistent_strength_source_count > 0
                || startup.convertible_strength_source_count > 0
        }
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

fn boss_mechanic_upgrade_delta(card: CardId, run_state: &RunState) -> i32 {
    match run_state.boss_key {
        Some(EncounterId::Automaton) if run_state.act_num == 2 => automaton_upgrade_delta(card),
        Some(EncounterId::TheChamp) if run_state.act_num == 2 => {
            champ_upgrade_delta(card, run_state)
        }
        _ => 0,
    }
}

fn automaton_upgrade_delta(card: CardId) -> i32 {
    match automaton_upgrade_strategy_tag_for_card(card) {
        Some(("stasis_proof", _)) => 700,
        Some(("hyperbeam_block", _)) => 620,
        Some(("block_engine", _)) => 500,
        Some(("artifact_or_weak", _)) => 360,
        Some(("access_recovery", _)) => 260,
        Some(("scaling_setup", _)) => 180,
        Some(_) | None => 0,
    }
}

fn automaton_upgrade_strategy_tag(card: CardId) -> Option<&'static str> {
    automaton_upgrade_strategy_tag_for_card(card).map(|(_, tag)| tag)
}

fn automaton_upgrade_strategy_tag_for_card(card: CardId) -> Option<(&'static str, &'static str)> {
    match card {
        CardId::Apparition => Some(("stasis_proof", "automaton:apparition_duration")),
        CardId::Impervious | CardId::PowerThrough | CardId::FlameBarrier => {
            Some(("hyperbeam_block", "automaton:hyperbeam_block"))
        }
        CardId::FeelNoPain | CardId::SecondWind | CardId::Barricade | CardId::Entrench => {
            Some(("block_engine", "automaton:block_engine"))
        }
        CardId::Shockwave | CardId::Disarm | CardId::Uppercut | CardId::Bash => {
            Some(("artifact_or_weak", "automaton:artifact_or_weak"))
        }
        CardId::Offering
        | CardId::BurningPact
        | CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Armaments => Some(("access_recovery", "automaton:access_recovery")),
        CardId::DemonForm | CardId::Corruption | CardId::LimitBreak => {
            Some(("scaling_setup", "automaton:scaling_setup"))
        }
        _ => None,
    }
}

fn champ_upgrade_delta(card: CardId, run_state: &RunState) -> i32 {
    match champ_upgrade_strategy_tag_for_card(card, run_state) {
        Some(("transition_burst", _)) => 620,
        Some(("execute_block", _)) => 560,
        Some(("access_recovery", _)) => 260,
        Some(("scaling_setup", _)) => 240,
        Some(_) | None => 0,
    }
}

fn champ_upgrade_strategy_tag(card: CardId, run_state: &RunState) -> Option<&'static str> {
    champ_upgrade_strategy_tag_for_card(card, run_state).map(|(_, tag)| tag)
}

fn champ_upgrade_strategy_tag_for_card(
    card: CardId,
    run_state: &RunState,
) -> Option<(&'static str, &'static str)> {
    match card {
        CardId::Carnage | CardId::Bludgeon | CardId::Immolate | CardId::Offering => {
            Some(("transition_burst", "champ:transition_burst"))
        }
        CardId::Whirlwind if has_extra_energy_access(run_state) => {
            Some(("transition_burst", "champ:transition_burst"))
        }
        CardId::HeavyBlade if has_champ_strength_burst_support(run_state) => {
            Some(("transition_burst", "champ:transition_burst"))
        }
        CardId::LimitBreak if has_champ_strength_conversion_support(run_state) => {
            Some(("transition_burst", "champ:transition_burst"))
        }
        CardId::Flex if has_champ_flex_upgrade_support(run_state) => {
            Some(("transition_burst", "champ:transition_burst"))
        }
        CardId::Impervious
        | CardId::PowerThrough
        | CardId::FlameBarrier
        | CardId::SecondWind
        | CardId::TrueGrit
        | CardId::Barricade
        | CardId::Entrench => Some(("execute_block", "champ:execute_block")),
        CardId::BurningPact | CardId::BattleTrance | CardId::PommelStrike | CardId::ShrugItOff => {
            Some(("access_recovery", "champ:access_recovery"))
        }
        CardId::DemonForm | CardId::Corruption => Some(("scaling_setup", "champ:scaling_setup")),
        _ => None,
    }
}

fn has_extra_energy_access(run_state: &RunState) -> bool {
    run_state.master_deck.iter().any(|card| {
        matches!(
            card.id,
            CardId::Offering | CardId::SeeingRed | CardId::Bloodletting
        )
    })
}

fn has_champ_strength_burst_support(run_state: &RunState) -> bool {
    let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
    startup.persistent_strength_source_count > 0
        || startup.temporary_strength_burst_count > 0
        || startup.convertible_strength_source_count > 0
}

fn has_champ_strength_conversion_support(run_state: &RunState) -> bool {
    let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
    startup.persistent_strength_source_count > 0 || startup.temporary_strength_burst_count > 0
}

fn has_champ_flex_upgrade_support(run_state: &RunState) -> bool {
    let startup = crate::ai::deck_startup_profile_v1::deck_startup_profile_v1(run_state);
    startup.strength_payoff_count > 0 || startup.strength_converter_count > 0
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

    #[test]
    fn campfire_upgrade_priority_marks_bash_as_clear_core_upgrade() {
        let run_state = RunState::new(1, 0, false, "Ironclad");

        assert!(
            campfire_smith_upgrade_priority_v1(&CombatCard::new(CardId::Bash, 1), &run_state)
                >= crate::ai::campfire_policy_v1::CampfirePolicyConfigV1::default()
                    .clear_core_smith_priority_threshold
        );
    }

    #[test]
    fn automaton_pressure_prioritizes_apparition_over_starter_smith() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.boss_key = Some(EncounterId::Automaton);

        let apparition = CombatCard::new(CardId::Apparition, 31);
        let strike = CombatCard::new(CardId::Strike, 32);

        assert!(
            campfire_smith_upgrade_priority_v1(&apparition, &run_state)
                > campfire_smith_upgrade_priority_v1(&strike, &run_state)
        );
        assert_eq!(
            campfire_smith_upgrade_strategy_tag_v1(&apparition, &run_state),
            Some("automaton:apparition_duration")
        );
    }

    #[test]
    fn champ_pressure_tags_transition_burst_and_execute_block_upgrades() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.boss_key = Some(EncounterId::TheChamp);

        let carnage = CombatCard::new(CardId::Carnage, 31);
        let true_grit = CombatCard::new(CardId::TrueGrit, 32);
        let strike = CombatCard::new(CardId::Strike, 33);

        assert_eq!(
            campfire_smith_upgrade_strategy_tag_v1(&carnage, &run_state),
            Some("champ:transition_burst")
        );
        assert_eq!(
            campfire_smith_upgrade_strategy_tag_v1(&true_grit, &run_state),
            Some("champ:execute_block")
        );
        assert!(
            campfire_smith_upgrade_priority_v1(&carnage, &run_state)
                > campfire_smith_upgrade_priority_v1(&strike, &run_state)
        );
        assert!(
            campfire_smith_upgrade_priority_v1(&true_grit, &run_state)
                > campfire_smith_upgrade_priority_v1(&strike, &run_state)
        );
    }

    #[test]
    fn champ_pressure_tags_flex_upgrade_as_burst_only_with_payoff() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.boss_key = Some(EncounterId::TheChamp);
        let flex = CombatCard::new(CardId::Flex, 31);

        assert_eq!(
            campfire_smith_upgrade_strategy_tag_v1(&flex, &run_state),
            None
        );

        run_state.add_card_to_deck(CardId::HeavyBlade);

        assert_eq!(
            campfire_smith_upgrade_strategy_tag_v1(&flex, &run_state),
            Some("champ:transition_burst")
        );
    }
}
