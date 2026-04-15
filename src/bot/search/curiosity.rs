use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;

pub(crate) fn curiosity_archetype_move_bonus(
    combat: &CombatState,
    chosen_move: &ClientInput,
    curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
) -> f32 {
    let target = match curiosity_target {
        Some(crate::bot::coverage::CuriosityTarget::Archetype(target)) => normalize_name(target),
        _ => return 0.0,
    };
    let profile = crate::bot::evaluator::CardEvaluator::combat_profile(combat);

    match chosen_move {
        ClientInput::PlayCard { card_index, .. } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return 0.0;
            };
            let card_id = card.id;
            match target.as_str() {
                "strength" => match card_id {
                    crate::content::cards::CardId::LimitBreak => {
                        if profile.strength_enablers > 0 {
                            12_000.0
                        } else {
                            2_000.0
                        }
                    }
                    crate::content::cards::CardId::Inflame
                    | crate::content::cards::CardId::SpotWeakness
                    | crate::content::cards::CardId::DemonForm => 9_000.0,
                    crate::content::cards::CardId::HeavyBlade
                    | crate::content::cards::CardId::SwordBoomerang
                    | crate::content::cards::CardId::Pummel
                    | crate::content::cards::CardId::Whirlwind
                    | crate::content::cards::CardId::TwinStrike => {
                        if profile.strength_enablers > 0 {
                            6_500.0
                        } else {
                            2_500.0
                        }
                    }
                    crate::content::cards::CardId::Flex => 5_000.0,
                    crate::content::cards::CardId::Panacea => 3_000.0,
                    _ => 0.0,
                },
                "exhaust" => match card_id {
                    crate::content::cards::CardId::Corruption
                    | crate::content::cards::CardId::FeelNoPain
                    | crate::content::cards::CardId::DarkEmbrace => 10_000.0,
                    crate::content::cards::CardId::SecondWind
                    | crate::content::cards::CardId::BurningPact
                    | crate::content::cards::CardId::TrueGrit
                    | crate::content::cards::CardId::SeverSoul
                    | crate::content::cards::CardId::FiendFire => {
                        if profile.exhaust_engines > 0 {
                            7_500.0
                        } else {
                            3_000.0
                        }
                    }
                    _ => 0.0,
                },
                "block" => match card_id {
                    crate::content::cards::CardId::Barricade
                    | crate::content::cards::CardId::Entrench => 10_000.0,
                    crate::content::cards::CardId::BodySlam
                    | crate::content::cards::CardId::FlameBarrier
                    | crate::content::cards::CardId::Impervious
                    | crate::content::cards::CardId::ShrugItOff => {
                        if profile.block_core > 0 {
                            7_000.0
                        } else {
                            3_000.0
                        }
                    }
                    crate::content::cards::CardId::Juggernaut => 5_000.0,
                    _ => 0.0,
                },
                "selfdamage" => match card_id {
                    crate::content::cards::CardId::Rupture => 9_000.0,
                    crate::content::cards::CardId::Offering
                    | crate::content::cards::CardId::Bloodletting
                    | crate::content::cards::CardId::Hemokinesis
                    | crate::content::cards::CardId::Combust
                    | crate::content::cards::CardId::Brutality => 6_500.0,
                    _ => 0.0,
                },
                "drawcycle" => match card_id {
                    crate::content::cards::CardId::BattleTrance
                    | crate::content::cards::CardId::PommelStrike
                    | crate::content::cards::CardId::ShrugItOff
                    | crate::content::cards::CardId::Finesse
                    | crate::content::cards::CardId::FlashOfSteel
                    | crate::content::cards::CardId::MasterOfStrategy => 7_000.0,
                    crate::content::cards::CardId::BurningPact
                    | crate::content::cards::CardId::Offering
                    | crate::content::cards::CardId::Brutality => 5_000.0,
                    _ => 0.0,
                },
                "powerscaling" => match card_id {
                    crate::content::cards::CardId::DemonForm
                    | crate::content::cards::CardId::Corruption
                    | crate::content::cards::CardId::FeelNoPain
                    | crate::content::cards::CardId::DarkEmbrace
                    | crate::content::cards::CardId::Barricade
                    | crate::content::cards::CardId::Juggernaut
                    | crate::content::cards::CardId::Evolve
                    | crate::content::cards::CardId::FireBreathing
                    | crate::content::cards::CardId::Panache
                    | crate::content::cards::CardId::Mayhem
                    | crate::content::cards::CardId::Magnetism => 8_500.0,
                    _ => 0.0,
                },
                "status" => match card_id {
                    crate::content::cards::CardId::Evolve
                    | crate::content::cards::CardId::FireBreathing => 9_000.0,
                    crate::content::cards::CardId::WildStrike
                    | crate::content::cards::CardId::RecklessCharge
                    | crate::content::cards::CardId::PowerThrough => 4_500.0,
                    _ => 0.0,
                },
                _ => 0.0,
            }
        }
        ClientInput::UsePotion { potion_index, .. } => {
            let Some(Some(potion)) = combat.entities.potions.get(*potion_index) else {
                return 0.0;
            };
            match target.as_str() {
                "strength" => match potion.id {
                    crate::content::potions::PotionId::StrengthPotion
                    | crate::content::potions::PotionId::SteroidPotion
                    | crate::content::potions::PotionId::AncientPotion => 6_000.0,
                    _ => 0.0,
                },
                "exhaust" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion
                    | crate::content::potions::PotionId::DuplicationPotion => 4_500.0,
                    _ => 0.0,
                },
                "block" => match potion.id {
                    crate::content::potions::PotionId::BlockPotion
                    | crate::content::potions::PotionId::DexterityPotion
                    | crate::content::potions::PotionId::EssenceOfSteel
                    | crate::content::potions::PotionId::RegenPotion => 5_500.0,
                    _ => 0.0,
                },
                "selfdamage" => match potion.id {
                    crate::content::potions::PotionId::AncientPotion
                    | crate::content::potions::PotionId::RegenPotion => 4_500.0,
                    _ => 0.0,
                },
                "drawcycle" => match potion.id {
                    crate::content::potions::PotionId::SwiftPotion
                    | crate::content::potions::PotionId::EnergyPotion
                    | crate::content::potions::PotionId::GamblersBrew => 5_000.0,
                    _ => 0.0,
                },
                "powerscaling" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion
                    | crate::content::potions::PotionId::AncientPotion => 5_000.0,
                    _ => 0.0,
                },
                "status" => match potion.id {
                    crate::content::potions::PotionId::PowerPotion
                    | crate::content::potions::PotionId::ColorlessPotion => 4_000.0,
                    _ => 0.0,
                },
                _ => 0.0,
            }
        }
        _ => 0.0,
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}
