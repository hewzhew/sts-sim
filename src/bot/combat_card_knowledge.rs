use crate::content::cards::{CardId, CardType};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TurnActionRole {
    Setup,
    Payoff,
    Cycling,
    EnergyBridge,
    DefensiveBridge,
    Utility,
    Finisher,
    #[default]
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TurnOrderingHint {
    PreferEarly,
    PreferLate,
    #[default]
    OrderFlexible,
    OrderConditional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChanceProfile {
    #[default]
    Deterministic,
    DrawBranch,
    RandomGeneration,
    TargetSensitive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RiskProfile {
    #[default]
    Safe,
    WindowSensitive,
    DownsideSensitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OrderingConstraint {
    SetupBeforePayoff,
    CyclingBeforeTerminalAttack,
    EnergyBridgeBeforeHighCostPayoff,
    DebuffBeforeMultiHitPayoff,
    FinisherAfterGrowthCheck,
}

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

pub(crate) fn classify_turn_action(card_id: CardId, card_type: CardType) -> TurnActionRole {
    match card_id {
        CardId::Feed | CardId::Reaper => TurnActionRole::Finisher,
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => {
            TurnActionRole::EnergyBridge
        }
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath
        | CardId::FlashOfSteel
        | CardId::Finesse
        | CardId::GoodInstincts => TurnActionRole::Cycling,
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::Disarm
        | CardId::Clothesline
        | CardId::Intimidate
        | CardId::SpotWeakness => TurnActionRole::Utility,
        CardId::Rage | CardId::Flex | CardId::Inflame | CardId::DemonForm => {
            TurnActionRole::Setup
        }
        _ => match card_type {
            CardType::Power => TurnActionRole::Setup,
            CardType::Attack => TurnActionRole::Payoff,
            CardType::Skill => TurnActionRole::DefensiveBridge,
            _ => TurnActionRole::Other,
        },
    }
}

pub(crate) fn default_ordering_hint(card_id: CardId, role: TurnActionRole) -> TurnOrderingHint {
    match role {
        TurnActionRole::Setup | TurnActionRole::Cycling | TurnActionRole::EnergyBridge => {
            TurnOrderingHint::PreferEarly
        }
        TurnActionRole::DefensiveBridge => match card_id {
            CardId::Impervious
            | CardId::FlameBarrier
            | CardId::PowerThrough
            | CardId::PanicButton
            | CardId::GhostlyArmor => TurnOrderingHint::PreferEarly,
            _ => TurnOrderingHint::OrderConditional,
        },
        TurnActionRole::Payoff | TurnActionRole::Finisher => TurnOrderingHint::PreferLate,
        TurnActionRole::Utility => match card_id {
            CardId::Bash
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::ThunderClap
            | CardId::Trip
            | CardId::SpotWeakness => TurnOrderingHint::PreferEarly,
            _ => TurnOrderingHint::OrderConditional,
        },
        TurnActionRole::Other => TurnOrderingHint::OrderFlexible,
    }
}

pub(crate) fn default_chance_profile(card_id: CardId) -> ChanceProfile {
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
        | CardId::GoodInstincts
        | CardId::Offering => ChanceProfile::DrawBranch,
        CardId::InfernalBlade | CardId::Discovery | CardId::Magnetism | CardId::Mayhem => {
            ChanceProfile::RandomGeneration
        }
        CardId::Bash
        | CardId::Uppercut
        | CardId::Clothesline
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::Disarm
        | CardId::Trip
        | CardId::Feed
        | CardId::HeavyBlade
        | CardId::SpotWeakness => ChanceProfile::TargetSensitive,
        _ => ChanceProfile::Deterministic,
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

pub(crate) fn default_risk_profile(card_id: CardId, role: TurnActionRole) -> RiskProfile {
    match role {
        TurnActionRole::Cycling | TurnActionRole::EnergyBridge => RiskProfile::DownsideSensitive,
        TurnActionRole::Utility => match card_id {
            CardId::Bash | CardId::Shockwave | CardId::Uppercut | CardId::Trip => {
                RiskProfile::WindowSensitive
            }
            _ => RiskProfile::Safe,
        },
        TurnActionRole::Payoff | TurnActionRole::Finisher => RiskProfile::WindowSensitive,
        _ => RiskProfile::Safe,
    }
}

pub(crate) fn default_ordering_constraint(card_id: CardId) -> Option<OrderingConstraint> {
    match card_id {
        CardId::Inflame
        | CardId::Flex
        | CardId::DemonForm
        | CardId::Rage
        | CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace => Some(OrderingConstraint::SetupBeforePayoff),
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath => Some(OrderingConstraint::CyclingBeforeTerminalAttack),
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => {
            Some(OrderingConstraint::EnergyBridgeBeforeHighCostPayoff)
        }
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip => Some(OrderingConstraint::DebuffBeforeMultiHitPayoff),
        CardId::Feed | CardId::Reaper => Some(OrderingConstraint::FinisherAfterGrowthCheck),
        _ => None,
    }
}
