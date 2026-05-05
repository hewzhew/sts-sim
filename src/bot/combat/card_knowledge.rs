use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BranchFamily {
    Draw,
    EnergyPlusDraw,
    RandomCombatCard,
    RandomAttackCard,
    #[default]
    UnknownRandom,
}

impl BranchFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draw => "draw",
            Self::EnergyPlusDraw => "energy_plus_draw",
            Self::RandomCombatCard => "random_combat_card",
            Self::RandomAttackCard => "random_attack_card",
            Self::UnknownRandom => "unknown_random",
        }
    }
}

pub fn branch_family_for_card(card_id: CardId) -> Option<BranchFamily> {
    match card_id {
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath
        | CardId::FlashOfSteel
        | CardId::Finesse
        | CardId::GoodInstincts => Some(BranchFamily::Draw),
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => {
            Some(BranchFamily::EnergyPlusDraw)
        }
        CardId::Discovery | CardId::Magnetism | CardId::Mayhem => {
            Some(BranchFamily::RandomCombatCard)
        }
        CardId::InfernalBlade | CardId::SecretWeapon => Some(BranchFamily::RandomAttackCard),
        _ => None,
    }
}
