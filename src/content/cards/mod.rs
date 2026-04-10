pub mod colorless;
pub mod curses;
pub mod hooks;
pub mod ironclad;
pub mod silent;
pub mod status;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum CardId {
    Strike,
    Defend,
    Bash,
    Cleave,
    IronWave,
    PerfectedStrike,
    TwinStrike,
    ThunderClap,
    ShrugItOff,
    Flex,
    TrueGrit,
    Inflame,
    DemonForm,
    Corruption,
    HeavyBlade,
    Whirlwind,
    Bloodletting,
    Offering,
    SwordBoomerang,
    Dropkick,
    PommelStrike,
    Headbutt,
    Bludgeon,
    DoubleTap,
    FeelNoPain,
    DarkEmbrace,
    Sentinel,
    FiendFire,
    SeverSoul,
    SecondWind,
    Exhume,
    BurningPact,
    Reaper,
    Feed,
    BloodForBlood,
    Rupture,
    Hemokinesis,
    Combust,
    Brutality,
    LimitBreak,
    SpotWeakness,
    Barricade,
    Entrench,
    Juggernaut,
    FlameBarrier,
    Metallicize,
    GhostlyArmor,
    Impervious,
    PowerThrough,
    Evolve,
    FireBreathing,
    Immolate,
    WildStrike,
    RecklessCharge,
    Havoc,
    Warcry,
    BattleTrance,
    Rampage,
    SearingBlow,
    Anger,
    Armaments,
    DualWield,
    InfernalBlade,
    SeeingRed,
    Rage,
    Berserk,
    Shockwave,
    Uppercut,
    Clothesline,
    Disarm,
    Intimidate,
    Carnage,
    Clash,
    BodySlam,
    Pummel,
    // Status
    Burn,
    Dazed,
    Slimed,
    Wound,
    Void,
    Parasite,
    Regret,
    AscendersBane,
    Clumsy,
    CurseOfTheBell,
    Decay,
    Doubt,
    Injury,
    Necronomicurse,
    Normality,
    Pain,
    Pride,
    Shame,
    Writhe,
    Miracle,
    Shiv,
    Bite,
    Apparition,
    Madness,
    RitualDagger,
    JAX,
    Finesse,
    // Colorless — Uncommon
    BandageUp,
    Blind,
    DarkShackles,
    DeepBreath,
    Discovery,
    DramaticEntrance,
    Enlightenment,
    FlashOfSteel,
    Forethought,
    GoodInstincts,
    Impatience,
    JackOfAllTrades,
    MindBlast,
    Panacea,
    PanicButton,
    Purity,
    SwiftStrike,
    Trip,
    // Colorless — Rare
    Apotheosis,
    Chrysalis,
    HandOfGreed,
    Magnetism,
    MasterOfStrategy,
    Mayhem,
    Metamorphosis,
    Panache,
    SadisticNature,
    SecretTechnique,
    SecretWeapon,
    TheBomb,
    ThinkingAhead,
    Transmutation,
    Violence,
    StrikeG,
    DefendG,
    Neutralize,
    Survivor,
    DeadlyPoison,
    BouncingFlask,
    Catalyst,
    NoxiousFumes,
    Footwork,
    BladeDance,
    CloakAndDagger,
    Backflip,
    Acrobatics,
    Prepared,
    DaggerThrow,
    PoisonedStab,
    DaggerSpray,
    Adrenaline,
    AfterImage,
    Burst,
    // Add more as we expand
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardType {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardRarity {
    Basic,
    Common,
    Uncommon,
    Rare,
    Special,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardTarget {
    Enemy,
    AllEnemy,
    SelfTarget,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardTag {
    Strike,
    StarterStrike,
    StarterDefend,
    Healing,
    Empty,
}

pub struct CardDefinition {
    pub id: CardId,
    pub name: &'static str,
    pub card_type: CardType,
    pub rarity: CardRarity,
    pub cost: i8, // -1 for X cost, -2 for unplayable
    pub base_damage: i32,
    pub base_block: i32,
    pub base_magic: i32,
    pub target: CardTarget,
    pub is_multi_damage: bool,
    pub exhaust: bool,
    pub ethereal: bool,
    pub innate: bool,
    pub tags: &'static [CardTag],
    pub upgrade_damage: i32,
    pub upgrade_block: i32,
    pub upgrade_magic: i32,
}

#[allow(dead_code)]
fn build_status(id: CardId, name: &'static str, cost: i8) -> CardDefinition {
    CardDefinition {
        id,
        name,
        card_type: CardType::Status,
        rarity: CardRarity::Special,
        cost,
        base_damage: 0,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::None,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

#[allow(dead_code)]
fn build_curse(id: CardId, name: &'static str, cost: i8) -> CardDefinition {
    let mut def = CardDefinition {
        id,
        name,
        card_type: CardType::Curse,
        rarity: CardRarity::Curse,
        cost,
        base_damage: 0,
        base_block: 0,
        base_magic: 0,
        target: CardTarget::None,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 0,
    };

    if id == CardId::AscendersBane
        || id == CardId::CurseOfTheBell
        || id == CardId::Necronomicurse
        || id == CardId::Pride
    {
        def.rarity = CardRarity::Special;
    }

    if id == CardId::AscendersBane || id == CardId::Clumsy {
        def.ethereal = true;
    }

    if id == CardId::Writhe || id == CardId::Pride {
        def.innate = true;
    }

    if id == CardId::Pride {
        def.exhaust = true;
    }

    def
}

/// Dispatches card-specific logic that natively occurs exactly when a card is exhausted.
/// Base Slay the Spire has only two cards that override `triggerOnExhaust()`:
/// 1. Sentinel (Gain Energy)
/// 2. Necronomicurse (Clone itself back to hand)
pub fn resolve_card_on_exhaust(
    card: &crate::combat::CombatCard,
    _state: &crate::combat::CombatState,
) -> Vec<crate::action::ActionInfo> {
    match card.id {
        CardId::Necronomicurse => vec![crate::action::ActionInfo {
            action: crate::action::Action::MakeTempCardInHand {
                card_id: CardId::Necronomicurse,
                amount: 1,
                upgraded: false,
            },
            insertion_mode: crate::action::AddTo::Bottom,
        }],
        CardId::Sentinel => vec![crate::action::ActionInfo {
            action: crate::action::Action::GainEnergy {
                amount: if card.upgrades > 0 { 3 } else { 2 },
            },
            insertion_mode: crate::action::AddTo::Bottom,
        }],
        _ => vec![],
    }
}

pub fn is_starter_strike(id: CardId) -> bool {
    matches!(id, CardId::Strike | CardId::StrikeG)
}

pub fn is_starter_defend(id: CardId) -> bool {
    matches!(id, CardId::Defend | CardId::DefendG)
}

pub fn is_starter_basic(id: CardId) -> bool {
    is_starter_strike(id) || is_starter_defend(id)
}

pub fn is_innate_card(card: &crate::combat::CombatCard) -> bool {
    get_card_definition(card.id).innate
        || matches!(card.id, CardId::AfterImage) && card.upgrades > 0
}

pub fn get_card_definition(id: CardId) -> CardDefinition {
    match id {
        CardId::AscendersBane => build_curse(CardId::AscendersBane, "Ascender's Bane", -2),
        CardId::Clumsy => build_curse(CardId::Clumsy, "Clumsy", -2),
        CardId::CurseOfTheBell => build_curse(CardId::CurseOfTheBell, "Curse of the Bell", -2),
        CardId::Decay => build_curse(CardId::Decay, "Decay", -2),
        CardId::Doubt => build_curse(CardId::Doubt, "Doubt", -2),
        CardId::Injury => build_curse(CardId::Injury, "Injury", -2),
        CardId::Necronomicurse => build_curse(CardId::Necronomicurse, "Necronomicurse", -2),
        CardId::Normality => build_curse(CardId::Normality, "Normality", -2),
        CardId::Pain => build_curse(CardId::Pain, "Pain", -2),
        CardId::Pride => build_curse(CardId::Pride, "Pride", 1),
        CardId::Shame => build_curse(CardId::Shame, "Shame", -2),
        CardId::Writhe => build_curse(CardId::Writhe, "Writhe", -2),
        CardId::Apparition => CardDefinition {
            id: CardId::Apparition,
            name: "Apparition",
            card_type: CardType::Skill,
            rarity: CardRarity::Special,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: true,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Madness => CardDefinition {
            id: CardId::Madness,
            name: "Madness",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::RitualDagger => CardDefinition {
            id: CardId::RitualDagger,
            name: "Ritual Dagger",
            card_type: CardType::Attack,
            rarity: CardRarity::Special,
            cost: 1,
            base_damage: 15,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::JAX => CardDefinition {
            id: CardId::JAX,
            name: "J.A.X.",
            card_type: CardType::Attack,
            rarity: CardRarity::Special,
            cost: 0,
            base_damage: 2,
            base_block: 0,
            base_magic: 3, // HP loss on play
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Finesse => CardDefinition {
            id: CardId::Finesse,
            name: "Finesse",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 2,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 2,
            upgrade_magic: 0,
        },
        // Colorless — Uncommon
        CardId::BandageUp => CardDefinition {
            id,
            name: "Bandage Up",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 4,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Blind => CardDefinition {
            id,
            name: "Blind",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::DarkShackles => CardDefinition {
            id,
            name: "Dark Shackles",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 9,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 6,
        },
        CardId::DeepBreath => CardDefinition {
            id,
            name: "Deep Breath",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Discovery => CardDefinition {
            id,
            name: "Discovery",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::DramaticEntrance => CardDefinition {
            id,
            name: "Dramatic Entrance",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 8,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: true,
            tags: &[],
            upgrade_damage: 4,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Enlightenment => CardDefinition {
            id,
            name: "Enlightenment",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::FlashOfSteel => CardDefinition {
            id,
            name: "Flash of Steel",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 3,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Forethought => CardDefinition {
            id,
            name: "Forethought",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::GoodInstincts => CardDefinition {
            id,
            name: "Good Instincts",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 6,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Impatience => CardDefinition {
            id,
            name: "Impatience",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::JackOfAllTrades => CardDefinition {
            id,
            name: "Jack of All Trades",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::MindBlast => CardDefinition {
            id,
            name: "Mind Blast",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: true,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Panacea => CardDefinition {
            id,
            name: "Panacea",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::PanicButton => CardDefinition {
            id,
            name: "Panic Button",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 30,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 10,
            upgrade_magic: 0,
        },
        CardId::Purity => CardDefinition {
            id,
            name: "Purity",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::SwiftStrike => CardDefinition {
            id,
            name: "Swift Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 7,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Trip => CardDefinition {
            id,
            name: "Trip",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        // Colorless — Rare
        CardId::Apotheosis => CardDefinition {
            id,
            name: "Apotheosis",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Chrysalis => CardDefinition {
            id,
            name: "Chrysalis",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::HandOfGreed => CardDefinition {
            id,
            name: "Hand of Greed",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 20,
            base_block: 0,
            base_magic: 20,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 5,
            upgrade_block: 0,
            upgrade_magic: 5,
        },
        CardId::Magnetism => CardDefinition {
            id,
            name: "Magnetism",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::MasterOfStrategy => CardDefinition {
            id,
            name: "Master of Strategy",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Mayhem => CardDefinition {
            id,
            name: "Mayhem",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Metamorphosis => CardDefinition {
            id,
            name: "Metamorphosis",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Panache => CardDefinition {
            id,
            name: "Panache",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 10,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 4,
        },
        CardId::SadisticNature => CardDefinition {
            id,
            name: "Sadistic Nature",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 5,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::SecretTechnique => CardDefinition {
            id,
            name: "Secret Technique",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::SecretWeapon => CardDefinition {
            id,
            name: "Secret Weapon",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::TheBomb => CardDefinition {
            id,
            name: "The Bomb",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 40,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 10,
        },
        CardId::ThinkingAhead => CardDefinition {
            id,
            name: "Thinking Ahead",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Transmutation => CardDefinition {
            id,
            name: "Transmutation",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: -1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Violence => CardDefinition {
            id,
            name: "Violence",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Parasite => CardDefinition {
            id: CardId::Parasite,
            name: "Parasite",
            card_type: CardType::Curse,
            rarity: CardRarity::Curse,
            cost: -2, // Unplayable
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Regret => CardDefinition {
            id: CardId::Regret,
            name: "Regret",
            card_type: CardType::Curse,
            rarity: CardRarity::Curse,
            cost: -2, // Unplayable
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Strike => CardDefinition {
            id: CardId::Strike,
            name: "Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Basic,
            cost: 1,
            base_damage: 6,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Strike, CardTag::StarterStrike],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Defend => CardDefinition {
            id: CardId::Defend,
            name: "Defend",
            card_type: CardType::Skill,
            rarity: CardRarity::Basic,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::StrikeG => CardDefinition {
            id: CardId::StrikeG,
            name: "Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Basic,
            cost: 1,
            base_damage: 6,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Strike, CardTag::StarterStrike],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::DefendG => CardDefinition {
            id: CardId::DefendG,
            name: "Defend",
            card_type: CardType::Skill,
            rarity: CardRarity::Basic,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::StarterDefend],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Bash => CardDefinition {
            id: CardId::Bash,
            name: "Bash",
            card_type: CardType::Attack,
            rarity: CardRarity::Basic,
            cost: 2,
            base_damage: 8,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Neutralize => CardDefinition {
            id: CardId::Neutralize,
            name: "Neutralize",
            card_type: CardType::Attack,
            rarity: CardRarity::Basic,
            cost: 0,
            base_damage: 3,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 1,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Survivor => CardDefinition {
            id: CardId::Survivor,
            name: "Survivor",
            card_type: CardType::Skill,
            rarity: CardRarity::Basic,
            cost: 1,
            base_damage: 0,
            base_block: 8,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Cleave => CardDefinition {
            id: CardId::Cleave,
            name: "Cleave",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 8,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::IronWave => CardDefinition {
            id: CardId::IronWave,
            name: "Iron Wave",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 5,
            base_block: 5,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 2,
            upgrade_magic: 0,
        },
        CardId::PerfectedStrike => CardDefinition {
            id: CardId::PerfectedStrike,
            name: "Perfected Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 2,
            base_damage: 6,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Strike],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::TwinStrike => CardDefinition {
            id: CardId::TwinStrike,
            name: "Twin Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 5,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Strike],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::ThunderClap => CardDefinition {
            id: CardId::ThunderClap,
            name: "Thunderclap",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 4,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::ShrugItOff => CardDefinition {
            id: CardId::ShrugItOff,
            name: "Shrug It Off",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 8,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Flex => CardDefinition {
            id: CardId::Flex,
            name: "Flex",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::TrueGrit => CardDefinition {
            id: CardId::TrueGrit,
            name: "True Grit",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 7,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 2,
            upgrade_magic: 0,
        },
        CardId::Inflame => CardDefinition {
            id: CardId::Inflame,
            name: "Inflame",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::DemonForm => CardDefinition {
            id: CardId::DemonForm,
            name: "Demon Form",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 3,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Corruption => CardDefinition {
            id: CardId::Corruption,
            name: "Corruption",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 3,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::HeavyBlade => CardDefinition {
            id: CardId::HeavyBlade,
            name: "Heavy Blade",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 2,
            base_damage: 14,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Whirlwind => CardDefinition {
            id: CardId::Whirlwind,
            name: "Whirlwind",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: -1,
            base_damage: 5,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Bloodletting => CardDefinition {
            id: CardId::Bloodletting,
            name: "Bloodletting",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Offering => CardDefinition {
            id: CardId::Offering,
            name: "Offering",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::SwordBoomerang => CardDefinition {
            id: CardId::SwordBoomerang,
            name: "Sword Boomerang",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 3,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::AllEnemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Dropkick => CardDefinition {
            id: CardId::Dropkick,
            name: "Dropkick",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 5,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::PommelStrike => CardDefinition {
            id: CardId::PommelStrike,
            name: "Pommel Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 9,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Strike],
            upgrade_damage: 1,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Headbutt => CardDefinition {
            id: CardId::Headbutt,
            name: "Headbutt",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 9,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Bludgeon => CardDefinition {
            id: CardId::Bludgeon,
            name: "Bludgeon",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 3,
            base_damage: 32,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 10,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::DoubleTap => CardDefinition {
            id: CardId::DoubleTap,
            name: "Double Tap",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::FeelNoPain => CardDefinition {
            id: CardId::FeelNoPain,
            name: "Feel No Pain",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::DarkEmbrace => CardDefinition {
            id: CardId::DarkEmbrace,
            name: "Dark Embrace",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Sentinel => CardDefinition {
            id: CardId::Sentinel,
            name: "Sentinel",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::FiendFire => CardDefinition {
            id: CardId::FiendFire,
            name: "Fiend Fire",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 7,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::SeverSoul => CardDefinition {
            id: CardId::SeverSoul,
            name: "Sever Soul",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 16,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 6,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::SecondWind => CardDefinition {
            id: CardId::SecondWind,
            name: "Second Wind",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 2,
            upgrade_magic: 0,
        },
        CardId::Exhume => CardDefinition {
            id: CardId::Exhume,
            name: "Exhume",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::BurningPact => CardDefinition {
            id: CardId::BurningPact,
            name: "Burning Pact",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Reaper => CardDefinition {
            id: CardId::Reaper,
            name: "Reaper",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 4,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Healing],
            upgrade_damage: 1,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Feed => CardDefinition {
            id: CardId::Feed,
            name: "Feed",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 10,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Healing],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::BloodForBlood => CardDefinition {
            id: CardId::BloodForBlood,
            name: "Blood for Blood",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 4,
            base_damage: 18,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 4,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Rupture => CardDefinition {
            id: CardId::Rupture,
            name: "Rupture",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Hemokinesis => CardDefinition {
            id: CardId::Hemokinesis,
            name: "Hemokinesis",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 15,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 5,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Combust => CardDefinition {
            id: CardId::Combust,
            name: "Combust",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 5,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Brutality => CardDefinition {
            id: CardId::Brutality,
            name: "Brutality",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::LimitBreak => CardDefinition {
            id: CardId::LimitBreak,
            name: "Limit Break",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[], // Note: Exhausts unless upgraded. We will handle upgrade later if possible,
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::SpotWeakness => CardDefinition {
            id: CardId::SpotWeakness,
            name: "Spot Weakness",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Barricade => CardDefinition {
            id: CardId::Barricade,
            name: "Barricade",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 3,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Entrench => CardDefinition {
            id: CardId::Entrench,
            name: "Entrench",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Juggernaut => CardDefinition {
            id: CardId::Juggernaut,
            name: "Juggernaut",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 5,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::FlameBarrier => CardDefinition {
            id: CardId::FlameBarrier,
            name: "Flame Barrier",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 12,
            base_magic: 4,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 4,
            upgrade_magic: 2,
        },
        CardId::Metallicize => CardDefinition {
            id: CardId::Metallicize,
            name: "Metallicize",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::GhostlyArmor => CardDefinition {
            id: CardId::GhostlyArmor,
            name: "Ghostly Armor",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 10,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: true,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Impervious => CardDefinition {
            id: CardId::Impervious,
            name: "Impervious",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 0,
            base_block: 30,
            base_magic: 0,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 10,
            upgrade_magic: 0,
        },
        CardId::PowerThrough => CardDefinition {
            id: CardId::PowerThrough,
            name: "Power Through",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 15,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 5,
            upgrade_magic: 0,
        },
        CardId::Evolve => CardDefinition {
            id: CardId::Evolve,
            name: "Evolve",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::FireBreathing => CardDefinition {
            id: CardId::FireBreathing,
            name: "Fire Breathing",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 6,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 4,
        },
        CardId::Immolate => CardDefinition {
            id: CardId::Immolate,
            name: "Immolate",
            card_type: CardType::Attack,
            rarity: CardRarity::Rare,
            cost: 2,
            base_damage: 21,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 7,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::WildStrike => CardDefinition {
            id: CardId::WildStrike,
            name: "Wild Strike",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 12,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 5,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::RecklessCharge => CardDefinition {
            id: CardId::RecklessCharge,
            name: "Reckless Charge",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 7,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Havoc => CardDefinition {
            id: CardId::Havoc,
            name: "Havoc",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Warcry => CardDefinition {
            id: CardId::Warcry,
            name: "Warcry",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::BattleTrance => CardDefinition {
            id: CardId::BattleTrance,
            name: "Battle Trance",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Rampage => CardDefinition {
            id: CardId::Rampage,
            name: "Rampage",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 8,
            base_block: 0,
            base_magic: 8,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 3,
        },
        CardId::SearingBlow => CardDefinition {
            id: CardId::SearingBlow,
            name: "Searing Blow",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 12,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Anger => CardDefinition {
            id: CardId::Anger,
            name: "Anger",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 0,
            base_damage: 6,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Armaments => CardDefinition {
            id: CardId::Armaments,
            name: "Armaments",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::DualWield => CardDefinition {
            id: CardId::DualWield,
            name: "Dual Wield",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::InfernalBlade => CardDefinition {
            id: CardId::InfernalBlade,
            name: "Infernal Blade",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::SeeingRed => CardDefinition {
            id: CardId::SeeingRed,
            name: "Seeing Red",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Rage => CardDefinition {
            id: CardId::Rage,
            name: "Rage",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Berserk => CardDefinition {
            id: CardId::Berserk,
            name: "Berserk",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: -1,
        },
        CardId::Shockwave => CardDefinition {
            id: CardId::Shockwave,
            name: "Shockwave",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::AllEnemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Uppercut => CardDefinition {
            id: CardId::Uppercut,
            name: "Uppercut",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 13,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Clothesline => CardDefinition {
            id: CardId::Clothesline,
            name: "Clothesline",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 2,
            base_damage: 12,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Disarm => CardDefinition {
            id: CardId::Disarm,
            name: "Disarm",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Intimidate => CardDefinition {
            id: CardId::Intimidate,
            name: "Intimidate",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::AllEnemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Carnage => CardDefinition {
            id: CardId::Carnage,
            name: "Carnage",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 20,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: true,
            innate: false,
            tags: &[],
            upgrade_damage: 8,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Clash => CardDefinition {
            id: CardId::Clash,
            name: "Clash",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 0,
            base_damage: 14,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 4,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::BodySlam => CardDefinition {
            id: CardId::BodySlam,
            name: "Body Slam",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Pummel => CardDefinition {
            id: CardId::Pummel,
            name: "Pummel",
            card_type: CardType::Attack,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 2,
            base_block: 0,
            base_magic: 4,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Wound => CardDefinition {
            id: CardId::Wound,
            name: "Wound",
            card_type: CardType::Status,
            rarity: CardRarity::Common,
            cost: -2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Burn => CardDefinition {
            id: CardId::Burn,
            name: "Burn",
            card_type: CardType::Status,
            rarity: CardRarity::Common,
            cost: -2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[], // In STS, Burn damages you at end of turn. We'll need a hook for this eventually,
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::Dazed => CardDefinition {
            id: CardId::Dazed,
            name: "Dazed",
            card_type: CardType::Status,
            rarity: CardRarity::Common,
            cost: -2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: true,
            innate: false,
            tags: &[], // Ethereal removes it,
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Slimed => CardDefinition {
            id: CardId::Slimed,
            name: "Slimed",
            card_type: CardType::Status,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Void => CardDefinition {
            id: CardId::Void,
            name: "Void",
            card_type: CardType::Status,
            rarity: CardRarity::Common,
            cost: -2,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: true,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Miracle => CardDefinition {
            id: CardId::Miracle,
            name: "Miracle",
            card_type: CardType::Skill,
            rarity: CardRarity::Special,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: true,
            ethereal: true,
            innate: false,
            tags: &[], // In game, Retain is actually hardcoded on Miracle,
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Shiv => CardDefinition {
            id: CardId::Shiv,
            name: "Shiv",
            card_type: CardType::Attack,
            rarity: CardRarity::Special,
            cost: 0,
            base_damage: 4,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Bite => CardDefinition {
            id: CardId::Bite,
            name: "Bite",
            card_type: CardType::Attack,
            rarity: CardRarity::Special,
            cost: 1,
            base_damage: 7,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[CardTag::Healing],
            upgrade_damage: 1,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::DeadlyPoison => CardDefinition {
            id: CardId::DeadlyPoison,
            name: "Deadly Poison",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 5,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 2,
        },
        CardId::BouncingFlask => CardDefinition {
            id: CardId::BouncingFlask,
            name: "Bouncing Flask",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 2,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::AllEnemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Catalyst => CardDefinition {
            id: CardId::Catalyst,
            name: "Catalyst",
            card_type: CardType::Skill,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::NoxiousFumes => CardDefinition {
            id: CardId::NoxiousFumes,
            name: "Noxious Fumes",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Footwork => CardDefinition {
            id: CardId::Footwork,
            name: "Footwork",
            card_type: CardType::Power,
            rarity: CardRarity::Uncommon,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::BladeDance => CardDefinition {
            id: CardId::BladeDance,
            name: "Blade Dance",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::CloakAndDagger => CardDefinition {
            id: CardId::CloakAndDagger,
            name: "Cloak And Dagger",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 6,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Backflip => CardDefinition {
            id: CardId::Backflip,
            name: "Backflip",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 5,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 3,
            upgrade_magic: 0,
        },
        CardId::Acrobatics => CardDefinition {
            id: CardId::Acrobatics,
            name: "Acrobatics",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::Prepared => CardDefinition {
            id: CardId::Prepared,
            name: "Prepared",
            card_type: CardType::Skill,
            rarity: CardRarity::Common,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::None,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::DaggerThrow => CardDefinition {
            id: CardId::DaggerThrow,
            name: "Dagger Throw",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 9,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 3,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::PoisonedStab => CardDefinition {
            id: CardId::PoisonedStab,
            name: "Poisoned Stab",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 6,
            base_block: 0,
            base_magic: 3,
            target: CardTarget::Enemy,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
        CardId::DaggerSpray => CardDefinition {
            id: CardId::DaggerSpray,
            name: "Dagger Spray",
            card_type: CardType::Attack,
            rarity: CardRarity::Common,
            cost: 1,
            base_damage: 4,
            base_block: 0,
            base_magic: 0,
            target: CardTarget::AllEnemy,
            is_multi_damage: true,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 2,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Adrenaline => CardDefinition {
            id: CardId::Adrenaline,
            name: "Adrenaline",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 0,
            base_damage: 0,
            base_block: 0,
            base_magic: 2,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: true,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::AfterImage => CardDefinition {
            id: CardId::AfterImage,
            name: "After Image",
            card_type: CardType::Power,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 0,
        },
        CardId::Burst => CardDefinition {
            id: CardId::Burst,
            name: "Burst",
            card_type: CardType::Skill,
            rarity: CardRarity::Rare,
            cost: 1,
            base_damage: 0,
            base_block: 0,
            base_magic: 1,
            target: CardTarget::SelfTarget,
            is_multi_damage: false,
            exhaust: false,
            ethereal: false,
            innate: false,
            tags: &[],
            upgrade_damage: 0,
            upgrade_block: 0,
            upgrade_magic: 1,
        },
    }
}

use crate::action::{Action, ActionInfo};
use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;
use crate::core::EntityId;
use smallvec::SmallVec;

/// Central dispatch table for resolving card play mechanics.
pub fn resolve_card_play(
    card_id: CardId,
    _state: &CombatState,
    _card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let t = target;
    match card_id {
        CardId::Strike | CardId::StrikeG => ironclad::strike::strike_play(_state, _card, t),
        CardId::Bash => ironclad::bash::bash_play(_state, _card, t),
        CardId::Cleave => ironclad::cleave::cleave_play(_state, _card),
        CardId::IronWave => ironclad::iron_wave::iron_wave_play(_state, _card, t),
        CardId::PerfectedStrike => {
            ironclad::perfected_strike::perfected_strike_play(_state, _card, t)
        }
        CardId::TwinStrike => ironclad::twin_strike::twin_strike_play(_state, _card, t),
        CardId::ThunderClap => ironclad::thunderclap::thunderclap_play(_state, _card),
        CardId::Defend | CardId::DefendG => ironclad::defend::defend_play(_state, _card),
        CardId::Neutralize => silent::neutralize::neutralize_play(_state, _card, t),
        CardId::Survivor => silent::survivor::survivor_play(_state, _card),
        CardId::ShrugItOff => ironclad::shrug_it_off::shrug_it_off_play(_state, _card),
        CardId::Flex => ironclad::flex::flex_play(_state, _card),
        CardId::TrueGrit => ironclad::true_grit::true_grit_play(_state, _card),
        CardId::Inflame => ironclad::inflame::inflame_play(_state, _card),
        CardId::DemonForm => ironclad::demon_form::demon_form_play(_state, _card),
        CardId::Corruption => ironclad::corruption::corruption_play(_state, _card),
        CardId::HeavyBlade => ironclad::heavy_blade::heavy_blade_play(_state, _card, t),
        CardId::Whirlwind => ironclad::whirlwind::whirlwind_play(_state, _card),
        CardId::Bloodletting => ironclad::bloodletting::bloodletting_play(_state, _card),
        CardId::Offering => ironclad::offering::offering_play(_state, _card),
        CardId::SwordBoomerang => ironclad::sword_boomerang::sword_boomerang_play(_state, _card),
        CardId::Dropkick => ironclad::dropkick::dropkick_play(_state, _card, t),
        CardId::PommelStrike => ironclad::pommel_strike::pommel_strike_play(_state, _card, t),
        CardId::Headbutt => ironclad::headbutt::headbutt_play(_state, _card, t),
        CardId::Bludgeon => ironclad::bludgeon::bludgeon_play(_state, _card, t),
        CardId::DoubleTap => ironclad::double_tap::double_tap_play(_state, _card, t),
        CardId::FeelNoPain => ironclad::feel_no_pain::feel_no_pain_play(_state, _card),
        CardId::DarkEmbrace => ironclad::dark_embrace::dark_embrace_play(_state, _card),
        CardId::Sentinel => ironclad::sentinel::sentinel_play(_state, _card),
        CardId::FiendFire => ironclad::fiend_fire::fiend_fire_play(_state, _card, t),
        CardId::SeverSoul => ironclad::sever_soul::sever_soul_play(_state, _card, t),
        CardId::SecondWind => ironclad::second_wind::second_wind_play(_state, _card),
        CardId::Exhume => ironclad::exhume::exhume_play(_state, _card),
        CardId::BurningPact => ironclad::burning_pact::burning_pact_play(_state, _card),
        CardId::Reaper => ironclad::reaper::reaper_play(_state, _card),
        CardId::Feed => ironclad::feed::feed_play(_state, _card, t),
        CardId::BloodForBlood => ironclad::blood_for_blood::blood_for_blood_play(_state, _card, t),
        CardId::Rupture => ironclad::rupture::rupture_play(_state, _card),
        CardId::Hemokinesis => ironclad::hemokinesis::hemokinesis_play(_state, _card, t),
        CardId::Combust => ironclad::combust::combust_play(_state, _card),
        CardId::Brutality => ironclad::brutality::brutality_play(_state, _card),
        CardId::LimitBreak => ironclad::limit_break::limit_break_play(_state, _card),
        CardId::SpotWeakness => ironclad::spot_weakness::spot_weakness_play(_state, _card, t),
        CardId::Barricade => ironclad::barricade::barricade_play(_state, _card),
        CardId::Entrench => ironclad::entrench::entrench_play(_state, _card),
        CardId::Juggernaut => ironclad::juggernaut::juggernaut_play(_state, _card),
        CardId::FlameBarrier => ironclad::flame_barrier::flame_barrier_play(_state, _card),
        CardId::Metallicize => ironclad::metallicize::metallicize_play(_state, _card),
        CardId::GhostlyArmor => ironclad::ghostly_armor::ghostly_armor_play(_state, _card),
        CardId::Impervious => ironclad::impervious::impervious_play(_state, _card),
        CardId::PowerThrough => ironclad::power_through::power_through_play(_state, _card),
        CardId::Evolve => ironclad::evolve::evolve_play(_state, _card),
        CardId::FireBreathing => ironclad::fire_breathing::fire_breathing_play(_state, _card),
        CardId::Immolate => ironclad::immolate::immolate_play(_state, _card),
        CardId::WildStrike => ironclad::wild_strike::wild_strike_play(_state, _card, t),
        CardId::RecklessCharge => ironclad::reckless_charge::reckless_charge_play(_state, _card, t),
        CardId::Havoc => ironclad::havoc::havoc_play(_state, _card, t),
        CardId::Warcry => ironclad::warcry::warcry_play(_state, _card),
        CardId::BattleTrance => ironclad::battle_trance::battle_trance_play(_state, _card),
        CardId::Rampage => ironclad::rampage::rampage_play(_state, _card, t),
        CardId::SearingBlow => ironclad::searing_blow::searing_blow_play(_state, _card, t),
        CardId::Anger => ironclad::anger::anger_play(_state, _card, t),
        CardId::Armaments => ironclad::armaments::armaments_play(_state, _card),
        CardId::DualWield => ironclad::dual_wield::dual_wield_play(_state, _card),
        CardId::InfernalBlade => ironclad::infernal_blade::infernal_blade_play(_state, _card),
        CardId::SeeingRed => ironclad::seeing_red::seeing_red_play(_state, _card),
        CardId::Rage => ironclad::rage::rage_play(_state, _card),
        CardId::Berserk => ironclad::berserk::berserk_play(_state, _card),
        CardId::Shockwave => ironclad::shockwave::shockwave_play(_state, _card),
        CardId::Uppercut => ironclad::uppercut::uppercut_play(_state, _card, t),
        CardId::Clothesline => ironclad::clothesline::clothesline_play(_state, _card, t),
        CardId::Disarm => ironclad::disarm::disarm_play(_state, _card, t),
        CardId::Intimidate => ironclad::intimidate::intimidate_play(_state, _card),
        CardId::Carnage => ironclad::carnage::carnage_play(_state, _card, t),
        CardId::Clash => ironclad::clash::clash_play(_state, _card, t),
        CardId::BodySlam => ironclad::body_slam::body_slam_play(_state, _card, t),
        CardId::Pummel => ironclad::pummel::pummel_play(_state, _card, t),
        CardId::Miracle => {
            smallvec::smallvec![ActionInfo {
                action: crate::action::Action::GainEnergy { amount: 1 },
                insertion_mode: crate::action::AddTo::Bottom,
            }]
        }
        CardId::Shiv => {
            let mut actions = smallvec::SmallVec::new();
            if let Some(t) = t {
                let def = get_card_definition(CardId::Shiv);
                actions.push(ActionInfo {
                    action: crate::action::Action::Damage(crate::action::DamageInfo {
                        source: _card.uuid as usize,
                        target: t,
                        base: def.base_damage,
                        output: _card.base_damage_mut,
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: _card.base_damage_mut != def.base_damage,
                    }),
                    insertion_mode: crate::action::AddTo::Bottom,
                });
            }
            actions
        }
        CardId::Bite => colorless::bite::bite_play(_state, _card, t),
        CardId::Apparition => smallvec::smallvec![ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::IntangiblePlayer,
                amount: _card.base_magic_num_mut.max(1),
            },
            insertion_mode: crate::action::AddTo::Bottom,
        }],
        CardId::DeadlyPoison => silent::deadly_poison::deadly_poison_play(_state, _card, t),
        CardId::BouncingFlask => silent::bouncing_flask::bouncing_flask_play(_state, _card),
        CardId::Catalyst => silent::catalyst::catalyst_play(_state, _card, t),
        CardId::NoxiousFumes => silent::noxious_fumes::noxious_fumes_play(_state, _card),
        CardId::Footwork => silent::footwork::footwork_play(_state, _card),
        CardId::BladeDance => silent::blade_dance::blade_dance_play(_state, _card),
        CardId::CloakAndDagger => silent::cloak_and_dagger::cloak_and_dagger_play(_state, _card),
        CardId::Backflip => silent::backflip::backflip_play(_state, _card),
        CardId::Acrobatics => silent::acrobatics::acrobatics_play(_state, _card),
        CardId::Prepared => silent::prepared::prepared_play(_state, _card),
        CardId::DaggerThrow => silent::dagger_throw::dagger_throw_play(_state, _card, t),
        CardId::PoisonedStab => silent::poisoned_stab::poisoned_stab_play(_state, _card, t),
        CardId::DaggerSpray => silent::dagger_spray::dagger_spray_play(_state, _card),
        CardId::Adrenaline => silent::adrenaline::adrenaline_play(_state, _card),
        CardId::AfterImage => silent::after_image::after_image_play(_state, _card),
        CardId::Burst => silent::burst::burst_play(_state, _card),
        CardId::Pride => smallvec::smallvec![], // Coast 1 but does nothing on play
        CardId::Finesse
        | CardId::BandageUp
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::DeepBreath
        | CardId::Discovery
        | CardId::DramaticEntrance
        | CardId::Enlightenment
        | CardId::FlashOfSteel
        | CardId::Forethought
        | CardId::GoodInstincts
        | CardId::Impatience
        | CardId::JackOfAllTrades
        | CardId::MindBlast
        | CardId::Panacea
        | CardId::PanicButton
        | CardId::Purity
        | CardId::SwiftStrike
        | CardId::Trip
        | CardId::Apotheosis
        | CardId::Chrysalis
        | CardId::HandOfGreed
        | CardId::Magnetism
        | CardId::MasterOfStrategy
        | CardId::Mayhem
        | CardId::Metamorphosis
        | CardId::Panache
        | CardId::SadisticNature
        | CardId::SecretTechnique
        | CardId::SecretWeapon
        | CardId::TheBomb
        | CardId::ThinkingAhead
        | CardId::Transmutation
        | CardId::Violence => {
            let _def = get_card_definition(_card.id);
            let dmg = _card.base_damage_mut;
            let blk = _card.base_block_mut;
            let mag = _card.base_magic_num_mut;
            let mut acts: SmallVec<[Action; 4]> = smallvec::smallvec![];
            match _card.id {
                // ── Colorless Uncommon ──
                CardId::BandageUp => {
                    // Heal magic HP
                    acts.push(Action::Heal {
                        target: 0,
                        amount: mag,
                    });
                }
                CardId::Blind => {
                    // Apply 2 Weak to target
                    let target_id = t.expect("Blind requires a target!");
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: target_id,
                        power_id: PowerId::Weak,
                        amount: mag,
                    });
                }
                CardId::DarkShackles => {
                    // Apply -9 Strength to target (temporary, lost at end of turn)
                    let target_id = t.expect("Dark Shackles requires a target!");
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: target_id,
                        power_id: PowerId::Strength,
                        amount: -mag,
                    });
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: target_id,
                        power_id: PowerId::LoseStrength,
                        amount: -mag,
                    });
                }
                CardId::DeepBreath => {
                    // Shuffle discard into draw pile, draw 1
                    acts.push(Action::EmptyDeckShuffle);
                    acts.push(Action::DrawCards(1));
                }
                CardId::Discovery => {
                    // Java: DiscoveryAction — present 3 random cards to choose from, chosen card costs 0
                    acts.push(Action::SuspendForDiscovery {
                        card_type: None,
                        cost_for_turn: Some(0),
                    });
                }
                CardId::DramaticEntrance => {
                    // Deal 8 damage to ALL enemies
                    acts.push(Action::DamageAllEnemies {
                        source: 0,
                        damages: crate::action::repeated_damage_matrix(
                            _state.entities.monsters.len(),
                            dmg,
                        ),
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: false,
                    });
                }
                CardId::Enlightenment => {
                    // Reduce cost of all cards in hand to 1 (this turn only)
                    acts.push(Action::ReduceAllHandCosts { amount: 1 });
                }
                CardId::Finesse => {
                    // Gain 2 Block, Draw 1
                    acts.push(Action::GainBlock {
                        target: 0,
                        amount: blk,
                    });
                    acts.push(Action::DrawCards(1));
                }
                CardId::FlashOfSteel => {
                    // Deal 3 damage, Draw 1
                    let target_id = t.expect("Flash of Steel requires a target!");
                    acts.push(Action::Damage(crate::action::DamageInfo {
                        source: 0,
                        target: target_id,
                        base: dmg,
                        output: dmg,
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: false,
                    }));
                    acts.push(Action::DrawCards(1));
                }
                CardId::Forethought => {
                    // Put card(s) from hand to bottom of draw pile (cost 0 next time)
                    // Base: choose 1 card. Upgraded: choose any number.
                    if !_state.zones.hand.is_empty() {
                        let upgraded = _card.upgrades > 0;
                        acts.push(Action::SuspendForHandSelect {
                            min: if upgraded { 0 } else { 1 },
                            max: if upgraded { 99 } else { 1 },
                            can_cancel: upgraded,
                            filter: crate::state::HandSelectFilter::Any,
                            reason: crate::state::HandSelectReason::PutToBottomOfDraw,
                        });
                    }
                }
                CardId::GoodInstincts => {
                    // Gain 6 Block
                    acts.push(Action::GainBlock {
                        target: 0,
                        amount: blk,
                    });
                }
                CardId::Impatience => {
                    // If no Attacks in hand, draw 2 cards
                    let has_attack = _state
                        .zones
                        .hand
                        .iter()
                        .any(|c| get_card_definition(c.id).card_type == CardType::Attack);
                    if !has_attack {
                        acts.push(Action::DrawCards(mag as u32));
                    }
                }
                CardId::JackOfAllTrades => {
                    // Add 1 random colorless card to hand
                    acts.push(Action::MakeRandomColorlessCardInHand {
                        rarity: CardRarity::Uncommon,
                        cost_for_turn: None,
                    });
                }
                CardId::MindBlast => {
                    // Deal damage equal to draw pile size
                    let draw_size = _state.zones.draw_pile.len() as i32;
                    let target_id = t.expect("Mind Blast requires a target!");
                    acts.push(Action::Damage(crate::action::DamageInfo {
                        source: 0,
                        target: target_id,
                        base: draw_size,
                        output: draw_size,
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: true,
                    }));
                }
                CardId::Panacea => {
                    // Gain 1 Artifact
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::Artifact,
                        amount: mag,
                    });
                }
                CardId::PanicButton => {
                    // Gain 30 Block
                    acts.push(Action::GainBlock {
                        target: 0,
                        amount: blk,
                    });
                }
                CardId::Purity => {
                    // Exhaust up to magic (3/5) cards from hand
                    if !_state.zones.hand.is_empty() {
                        acts.push(Action::SuspendForHandSelect {
                            min: 0,
                            max: mag as u8,
                            can_cancel: true,
                            filter: crate::state::HandSelectFilter::Any,
                            reason: crate::state::HandSelectReason::Exhaust,
                        });
                    }
                }
                CardId::SwiftStrike => {
                    // Deal 7 damage
                    let target_id = t.expect("Swift Strike requires a target!");
                    acts.push(Action::Damage(crate::action::DamageInfo {
                        source: 0,
                        target: target_id,
                        base: dmg,
                        output: dmg,
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: false,
                    }));
                }
                CardId::Trip => {
                    // Apply 2 Vulnerable to target
                    let target_id = t.expect("Trip requires a target!");
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: target_id,
                        power_id: PowerId::Vulnerable,
                        amount: mag,
                    });
                }
                // ── Colorless Rare ──
                CardId::Apotheosis => {
                    acts.push(Action::UpgradeAllInHand);
                }
                CardId::Chrysalis => {
                    for _ in 0..mag {
                        acts.push(Action::MakeRandomCardInHand {
                            card_type: Some(CardType::Skill),
                            cost_for_turn: Some(0),
                        });
                    }
                }
                CardId::HandOfGreed => {
                    let target_id = t.expect("Hand of Greed requires a target!");
                    acts.push(Action::Damage(crate::action::DamageInfo {
                        source: 0,
                        target: target_id,
                        base: dmg,
                        output: dmg,
                        damage_type: crate::action::DamageType::Normal,
                        is_modified: false,
                    }));
                }
                CardId::Magnetism => {
                    // Power: At start of each turn, add a random colorless card to hand
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::MagnetismPower,
                        amount: 1,
                    });
                }
                CardId::Mayhem => {
                    // Power: At start of each turn, play the top card of draw pile
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::MayhemPower,
                        amount: 1,
                    });
                }
                CardId::Panache => {
                    // Power: Every 5th card played, deal magic (10/14) damage to ALL enemies
                    // amount=5 (counter), extra_data=damage
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::PanachePower,
                        amount: 5,
                    });
                    // Store damage value in extra_data
                    acts.push(Action::UpdatePowerExtraData {
                        target: 0,
                        power_id: PowerId::PanachePower,
                        value: mag,
                    });
                }
                CardId::SadisticNature => {
                    // Power: When applying a debuff, deal magic (5/7) damage to that enemy
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::SadisticPower,
                        amount: mag,
                    });
                }
                CardId::TheBomb => {
                    // Power: At the end of 3 turns, deal magic (40/50) damage to ALL enemies
                    // amount=3 (countdown), extra_data=damage
                    acts.push(Action::ApplyPower {
                        source: 0,
                        target: 0,
                        power_id: PowerId::TheBombPower,
                        amount: 3,
                    });
                    acts.push(Action::UpdatePowerExtraData {
                        target: 0,
                        power_id: PowerId::TheBombPower,
                        value: mag,
                    });
                }
                CardId::MasterOfStrategy => {
                    acts.push(Action::DrawCards(mag as u32));
                }
                CardId::Metamorphosis => {
                    for _ in 0..mag {
                        acts.push(Action::MakeRandomCardInHand {
                            card_type: Some(CardType::Attack),
                            cost_for_turn: Some(0),
                        });
                    }
                }
                CardId::SecretTechnique => {
                    // Search draw pile for a Skill card and put it in hand
                    let skills: Vec<_> = _state
                        .zones
                        .draw_pile
                        .iter()
                        .filter(|c| get_card_definition(c.id).card_type == CardType::Skill)
                        .map(|c| c.uuid)
                        .collect();
                    if skills.len() == 1 {
                        acts.push(Action::MoveCard {
                            card_uuid: skills[0],
                            from: crate::state::PileType::Draw,
                            to: crate::state::PileType::Hand,
                        });
                    } else if !skills.is_empty() {
                        acts.push(Action::SuspendForGridSelect {
                            source_pile: crate::state::PileType::Draw,
                            min: 1,
                            max: 1,
                            can_cancel: false,
                            filter: crate::state::GridSelectFilter::Skill,
                            reason: crate::state::GridSelectReason::SkillFromDeckToHand,
                        });
                    }
                }
                CardId::SecretWeapon => {
                    // Search draw pile for an Attack card and put it in hand
                    let attacks: Vec<_> = _state
                        .zones
                        .draw_pile
                        .iter()
                        .filter(|c| get_card_definition(c.id).card_type == CardType::Attack)
                        .map(|c| c.uuid)
                        .collect();
                    if attacks.len() == 1 {
                        acts.push(Action::MoveCard {
                            card_uuid: attacks[0],
                            from: crate::state::PileType::Draw,
                            to: crate::state::PileType::Hand,
                        });
                    } else if !attacks.is_empty() {
                        acts.push(Action::SuspendForGridSelect {
                            source_pile: crate::state::PileType::Draw,
                            min: 1,
                            max: 1,
                            can_cancel: false,
                            filter: crate::state::GridSelectFilter::Attack,
                            reason: crate::state::GridSelectReason::AttackFromDeckToHand,
                        });
                    }
                }
                CardId::ThinkingAhead => {
                    acts.push(Action::DrawCards(2));
                }
                CardId::Transmutation => {
                    acts.push(Action::MakeRandomColorlessCardInHand {
                        rarity: CardRarity::Uncommon,
                        cost_for_turn: Some(0),
                    });
                }
                CardId::Violence => {
                    for _ in 0..mag {
                        acts.push(Action::DrawCards(1));
                    }
                }
                _ => {}
            }
            acts.into_iter()
                .map(|a| ActionInfo {
                    action: a,
                    insertion_mode: crate::action::AddTo::Bottom,
                })
                .collect()
        }
        // Unplayable stubs — curses, status, and special cards
        CardId::Wound
        | CardId::Burn
        | CardId::Dazed
        | CardId::Slimed
        | CardId::Parasite
        | CardId::Void
        | CardId::Regret
        | CardId::AscendersBane
        | CardId::Clumsy
        | CardId::CurseOfTheBell
        | CardId::Decay
        | CardId::Doubt
        | CardId::Injury
        | CardId::Necronomicurse
        | CardId::Normality
        | CardId::Pain
        | CardId::Shame
        | CardId::Writhe
        | CardId::Madness
        | CardId::RitualDagger
        | CardId::JAX => smallvec::smallvec![], // Unplayable / Stub
    }
}

/// Evaluates a card's damage, block, and magic number based on player powers, target powers, and specific card rules.
/// Maps directly to Java Spire's `applyPowers()` (when target is None) and `calculateCardDamage()` (when target is Some).
pub fn evaluate_card(card: &mut CombatCard, state: &CombatState, target: Option<EntityId>) {
    let def = get_card_definition(card.id);
    let u = if card.upgrades > 0 { 1 } else { 0 };
    let mut damage = (def.base_damage + u * def.upgrade_damage) as f32;
    let mut block = (def.base_block + u * def.upgrade_block) as f32;

    // 1. Card specific base overrides (Perfected Strike)
    if card.id == CardId::PerfectedStrike {
        let mut strike_count = 0;
        let is_strike = |id| get_card_definition(id).tags.contains(&CardTag::Strike);

        for c in &state.zones.hand {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        for c in &state.zones.draw_pile {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        for c in &state.zones.discard_pile {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }
        for c in &state.zones.limbo {
            if is_strike(c.id) && c.uuid != card.uuid {
                strike_count += 1;
            }
        }

        // Count the card itself definitively once
        if is_strike(card.id) {
            strike_count += 1;
        }

        damage += (card.base_magic_num_mut as f32) * (strike_count as f32);
    } else if card.id == CardId::BloodForBlood {
        // Dynamic Cost Reduction based on hits taken (unblocked or blocked depending on earlier engine implementation; Java increments when hp lost)
        card.cost_modifier = -(state.turn.counters.times_damaged_this_combat as i8);
    } else if card.id == CardId::BodySlam {
        damage = state.entities.player.block as f32;
    } else if card.id == CardId::Rampage {
        damage += card.misc_value as f32;
    } else if card.id == CardId::SearingBlow {
        let u = card.upgrades as f32;
        damage = 12.0 + u * (u + 7.0) / 2.0;
    }

    // 2. Relic atDamageModify hooks (Java: AbstractCard.applyPowers/calculateCardDamage)
    // run before player power atDamageGive hooks. This ordering matters for cases like
    // Strike Dummy under Weak: (base + 3) * 0.75, not base * 0.75 + 3.
    damage =
        crate::content::relics::hooks::modify_player_attack_damage_for_card(state, card, damage);

    // 3. Player Powers
    if let Some(powers) = crate::content::powers::store::powers_for(state, 0) {
        for power in powers {
            damage = crate::content::powers::resolve_power_on_calculate_damage_to_enemy(
                power.power_type,
                state,
                card,
                damage,
                power.amount,
            );
            block = crate::content::powers::resolve_power_on_calculate_block(
                power.power_type,
                state,
                card,
                block,
                power.amount,
            );
        }
    }

    // 4. Stance
    if def.card_type == crate::content::cards::CardType::Attack {
        match state.entities.player.stance {
            crate::combat::StanceId::Wrath => damage *= 2.0,
            crate::combat::StanceId::Divinity => damage *= 3.0,
            _ => {}
        }
    }
    // 5. Target Powers
    if def.is_multi_damage {
        card.multi_damage.clear();
        for m in &state.entities.monsters {
            let mut mdmg = damage;
            // Target specific powers (Vulnerable)
            if let Some(target_powers) = crate::content::powers::store::powers_for(state, m.id) {
                for power in target_powers {
                    mdmg = crate::content::powers::resolve_power_on_calculate_damage_from_player(
                        power.power_type,
                        state,
                        card,
                        m.id,
                        mdmg,
                        power.amount,
                    );
                }
            }
            if mdmg < 0.0 {
                mdmg = 0.0;
            }
            card.multi_damage.push(mdmg as i32);
        }
        if let Some(first) = card.multi_damage.first() {
            damage = *first as f32;
        }
    } else if let Some(target_id) = target {
        if let Some(target_powers) = crate::content::powers::store::powers_for(state, target_id) {
            for power in target_powers {
                damage = crate::content::powers::resolve_power_on_calculate_damage_from_player(
                    power.power_type,
                    state,
                    card,
                    target_id,
                    damage,
                    power.amount,
                );
            }
        }
    }

    if damage < 0.0 {
        damage = 0.0;
    }
    if block < 0.0 {
        block = 0.0;
    }

    card.base_damage_mut = damage as i32;
    card.base_block_mut = block as i32;
    card.base_magic_num_mut = def.base_magic + u * def.upgrade_magic;
}

/// Produces a freshly evaluated combat card for actual play execution.
///
/// This avoids relying on potentially stale cached mutation fields on the card
/// object when generating execution-time actions.
pub fn evaluate_card_for_play(
    card: &CombatCard,
    state: &CombatState,
    target: Option<EntityId>,
) -> CombatCard {
    let mut evaluated = card.clone();
    evaluate_card(&mut evaluated, state, target);
    evaluated
}

/// Returns the card's intrinsic exhaust-on-play behavior after applying
/// upgrade-sensitive card rules.
pub fn exhausts_when_played(card: &CombatCard) -> bool {
    match card.id {
        CardId::LimitBreak => card.upgrades == 0,
        CardId::Discovery => card.upgrades == 0,
        _ => get_card_definition(card.id).exhaust,
    }
}

/// Returns the card's effective ethereal status after upgrade-sensitive overrides.
pub fn is_ethereal(card: &CombatCard) -> bool {
    match card.id {
        CardId::Apparition => card.upgrades == 0,
        _ => get_card_definition(card.id).ethereal,
    }
}

/// Validates whether a card can be played based on energy, status locks, and curses like Normality.
pub fn can_play_card(card: &CombatCard, state: &CombatState) -> Result<(), &'static str> {
    // Curse: Normality Lock
    if state.zones.hand.iter().any(|c| c.id == CardId::Normality) {
        if state.turn.counters.cards_played_this_turn >= 3 {
            return Err("Normality: Cannot play more than 3 cards this turn.");
        }
    }

    let def = crate::content::cards::get_card_definition(card.id);
    let cost = card.get_cost();

    // In Slay the Spire, internally cost -2 means the card is Unplayable.
    if cost < -1 {
        if def.card_type == crate::content::cards::CardType::Curse
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::BlueCandle)
        {
            // Blue Candle override
        } else if def.card_type == crate::content::cards::CardType::Status
            && state
                .entities
                .player
                .has_relic(crate::content::relics::RelicId::MedicalKit)
        {
            // Medical Kit override
        } else {
            return Err("Card is unplayable.");
        }
    }

    // Java: hasEnoughEnergy() — Power.canPlayCard() hook
    // Iterates all player powers; if any returns false, card cannot be played.
    if let Some(player_powers) = crate::content::powers::store::powers_for(state, 0) {
        for ps in player_powers {
            if !crate::content::powers::resolve_power_can_play_card(ps.power_type, card) {
                return Err("A power prevents playing this card.");
            }
        }
    }

    // Java: hasEnoughEnergy() — Entangled hardcode (L857-860)
    // This is separate from the canPlayCard hook; Java checks it explicitly.
    if let Some(player_powers) = crate::content::powers::store::powers_for(state, 0) {
        if player_powers
            .iter()
            .any(|p| p.power_type == crate::content::powers::PowerId::Entangle)
            && def.card_type == crate::content::cards::CardType::Attack
        {
            return Err("Entangled: Cannot play Attacks this turn.");
        }
    }

    // Card-specific overrides (Java: card.canUse overrides)
    match card.id {
        CardId::Clash => {
            let has_non_attack = state.zones.hand.iter().any(|c| {
                let d = crate::content::cards::get_card_definition(c.id);
                d.card_type != crate::content::cards::CardType::Attack
            });
            if has_non_attack {
                return Err("Can only play Clash if every card in your hand is an Attack.");
            }
        }
        // Future cards like Grand Finale (if draw pile not empty) go here
        _ => {}
    }

    // Default cost validation
    if cost >= 0 && state.turn.energy < (cost as u8) {
        return Err("Not enough energy.");
    }

    Ok(())
}

/// A global hook called immediately after a card is played to aggregate passive triggers from the state (e.g. Curses).
pub fn on_play_card(played_card: &CombatCard, state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut passive_actions = smallvec::SmallVec::new();

    // Curse: Pain (Lose 1 HP for every card played)
    for card in &state.zones.hand {
        if card.id == CardId::Pain && card.uuid != played_card.uuid {
            passive_actions.push(crate::content::cards::curses::pain::on_other_card_played());
        }
    }

    // Other hooks (Time Eater, Velvet Choker, etc.) can be placed here later.

    passive_actions
}

/// Colorless pool — Uncommon tier (mirrors Java addColorlessCards: color=COLORLESS, rarity!=BASIC/SPECIAL)
pub const COLORLESS_UNCOMMON_POOL: &[CardId] = &[
    CardId::BandageUp,
    CardId::Blind,
    CardId::DarkShackles,
    CardId::DeepBreath,
    CardId::Discovery,
    CardId::DramaticEntrance,
    CardId::Enlightenment,
    CardId::Finesse,
    CardId::FlashOfSteel,
    CardId::Forethought,
    CardId::GoodInstincts,
    CardId::Impatience,
    CardId::JackOfAllTrades,
    CardId::Madness,
    CardId::MindBlast,
    CardId::Panacea,
    CardId::PanicButton,
    CardId::Purity,
    CardId::SwiftStrike,
    CardId::Trip,
];

/// Colorless pool — Rare tier
pub const COLORLESS_RARE_POOL: &[CardId] = &[
    CardId::Apotheosis,
    CardId::Chrysalis,
    CardId::HandOfGreed,
    CardId::Magnetism,
    CardId::MasterOfStrategy,
    CardId::Mayhem,
    CardId::Metamorphosis,
    CardId::Panache,
    CardId::SadisticNature,
    CardId::SecretTechnique,
    CardId::SecretWeapon,
    CardId::TheBomb,
    CardId::ThinkingAhead,
    CardId::Transmutation,
    CardId::Violence,
];

// ── Ironclad card pools (mirrors Java CardLibrary.addRedCards, excludes BASIC) ──

/// Ironclad Common pool (17 cards)
pub const IRONCLAD_COMMON_POOL: &[CardId] = &[
    CardId::Anger,
    CardId::Armaments,
    CardId::BodySlam,
    CardId::Clash,
    CardId::Cleave,
    CardId::Clothesline,
    CardId::Flex,
    CardId::Havoc,
    CardId::Headbutt,
    CardId::HeavyBlade,
    CardId::IronWave,
    CardId::PerfectedStrike,
    CardId::PommelStrike,
    CardId::ShrugItOff,
    CardId::SwordBoomerang,
    CardId::ThunderClap,
    CardId::TrueGrit,
    CardId::TwinStrike,
    CardId::Warcry,
    CardId::WildStrike,
];

/// Ironclad Uncommon pool (33 cards)
pub const IRONCLAD_UNCOMMON_POOL: &[CardId] = &[
    CardId::BattleTrance,
    CardId::BloodForBlood,
    CardId::Bloodletting,
    CardId::BurningPact,
    CardId::Carnage,
    CardId::Combust,
    CardId::DarkEmbrace,
    CardId::Disarm,
    CardId::Dropkick,
    CardId::DualWield,
    CardId::Entrench,
    CardId::Evolve,
    CardId::FeelNoPain,
    CardId::FireBreathing,
    CardId::FlameBarrier,
    CardId::GhostlyArmor,
    CardId::Hemokinesis,
    CardId::InfernalBlade,
    CardId::Inflame,
    CardId::Intimidate,
    CardId::Metallicize,
    CardId::PowerThrough,
    CardId::Pummel,
    CardId::Rage,
    CardId::Rampage,
    CardId::RecklessCharge,
    CardId::Rupture,
    CardId::SearingBlow,
    CardId::SecondWind,
    CardId::SeeingRed,
    CardId::Sentinel,
    CardId::SeverSoul,
    CardId::Shockwave,
    CardId::SpotWeakness,
    CardId::Uppercut,
    CardId::Whirlwind,
];

/// Ironclad Rare pool (16 cards)
pub const IRONCLAD_RARE_POOL: &[CardId] = &[
    CardId::Barricade,
    CardId::Berserk,
    CardId::Bludgeon,
    CardId::Brutality,
    CardId::Corruption,
    CardId::DemonForm,
    CardId::DoubleTap,
    CardId::Exhume,
    CardId::Feed,
    CardId::FiendFire,
    CardId::Immolate,
    CardId::Impervious,
    CardId::Juggernaut,
    CardId::LimitBreak,
    CardId::Offering,
    CardId::Reaper,
];

pub const SILENT_COMMON_POOL: &[CardId] = &[
    CardId::Acrobatics,
    CardId::Backflip,
    CardId::BladeDance,
    CardId::CloakAndDagger,
    CardId::DeadlyPoison,
    CardId::Prepared,
    CardId::DaggerThrow,
    CardId::PoisonedStab,
    CardId::DaggerSpray,
];

pub const SILENT_UNCOMMON_POOL: &[CardId] = &[
    CardId::BouncingFlask,
    CardId::Catalyst,
    CardId::Footwork,
    CardId::NoxiousFumes,
];

pub const SILENT_RARE_POOL: &[CardId] = &[CardId::Adrenaline, CardId::AfterImage, CardId::Burst];

/// Returns the pool for a given rarity (Ironclad).
/// Returns the pool of randomly obtainable curse cards.
/// Java: AbstractDungeon.returnRandomCurse() draws from this pool.
/// Excludes AscendersBane (Asc10 special) and CurseOfTheBell (event-only).
pub fn get_curse_pool() -> &'static [CardId] {
    &[
        CardId::Clumsy,
        CardId::Decay,
        CardId::Doubt,
        CardId::Injury,
        CardId::Necronomicurse,
        CardId::Normality,
        CardId::Pain,
        CardId::Parasite,
        CardId::Regret,
        CardId::Shame,
        CardId::Writhe,
    ]
}

pub fn ironclad_pool_for_rarity(rarity: CardRarity) -> &'static [CardId] {
    match rarity {
        CardRarity::Common => IRONCLAD_COMMON_POOL,
        CardRarity::Uncommon => IRONCLAD_UNCOMMON_POOL,
        CardRarity::Rare => IRONCLAD_RARE_POOL,
        _ => IRONCLAD_COMMON_POOL,
    }
}

/// Returns all Ironclad cards of the given CardType from the pool matching the given rarity.
pub fn ironclad_pool_for_type(card_type: CardType) -> Vec<CardId> {
    let mut result = Vec::new();
    for &pool in &[
        IRONCLAD_COMMON_POOL,
        IRONCLAD_UNCOMMON_POOL,
        IRONCLAD_RARE_POOL,
    ] {
        for &id in pool {
            if get_card_definition(id).card_type == card_type {
                result.push(id);
            }
        }
    }
    result
}

pub fn silent_pool_for_rarity(rarity: CardRarity) -> &'static [CardId] {
    match rarity {
        CardRarity::Common => SILENT_COMMON_POOL,
        CardRarity::Uncommon => SILENT_UNCOMMON_POOL,
        CardRarity::Rare => SILENT_RARE_POOL,
        _ => SILENT_COMMON_POOL,
    }
}

pub fn silent_pool_for_type(card_type: CardType) -> Vec<CardId> {
    let mut result = Vec::new();
    for &pool in &[SILENT_COMMON_POOL, SILENT_UNCOMMON_POOL, SILENT_RARE_POOL] {
        for &id in pool {
            if get_card_definition(id).card_type == card_type {
                result.push(id);
            }
        }
    }
    result
}

/// Returns the pool for a given rarity (Defect). Stub until Defect cards are implemented.
pub fn defect_pool_for_rarity(_rarity: CardRarity) -> &'static [CardId] {
    &[]
}

/// Returns the pool for a given rarity (Watcher). Stub until Watcher cards are implemented.
pub fn watcher_pool_for_rarity(_rarity: CardRarity) -> &'static [CardId] {
    &[]
}

/// Returns the colorless pool for a given rarity.
pub fn colorless_pool_for_rarity(rarity: CardRarity) -> &'static [CardId] {
    match rarity {
        CardRarity::Uncommon => COLORLESS_UNCOMMON_POOL,
        CardRarity::Rare => COLORLESS_RARE_POOL,
        _ => &[],
    }
}

// ============================================================================
// Java Card ID Mapping
// ============================================================================
// These strings are the exact `card.cardID` values from the Java game source.
// They are used by the verification pipeline to map between Java and Rust.
// Source: decompiled from com.megacrit.cardcrawl.cards.*.java

/// Returns the Java `card.cardID` string for a given Rust CardId.
pub fn java_id(id: CardId) -> &'static str {
    match id {
        // --- Ironclad Basic ---
        CardId::Strike => "Strike_R",
        CardId::Defend => "Defend_R",
        CardId::Bash => "Bash",
        CardId::StrikeG => "Strike_G",
        CardId::DefendG => "Defend_G",
        CardId::Neutralize => "Neutralize",
        CardId::Survivor => "Survivor",

        // --- Ironclad Common ---
        CardId::Anger => "Anger",
        CardId::Armaments => "Armaments",
        CardId::BodySlam => "Body Slam",
        CardId::Clash => "Clash",
        CardId::Cleave => "Cleave",
        CardId::Clothesline => "Clothesline",
        CardId::Flex => "Flex",
        CardId::Havoc => "Havoc",
        CardId::Headbutt => "Headbutt",
        CardId::HeavyBlade => "Heavy Blade",
        CardId::IronWave => "Iron Wave",
        CardId::PerfectedStrike => "Perfected Strike",
        CardId::PommelStrike => "Pommel Strike",
        CardId::ShrugItOff => "Shrug It Off",
        CardId::SwordBoomerang => "Sword Boomerang",
        CardId::ThunderClap => "Thunderclap",
        CardId::TrueGrit => "True Grit",
        CardId::TwinStrike => "Twin Strike",
        CardId::Warcry => "Warcry",
        CardId::WildStrike => "Wild Strike",

        // --- Ironclad Uncommon ---
        CardId::BattleTrance => "Battle Trance",
        CardId::BloodForBlood => "Blood for Blood",
        CardId::Bloodletting => "Bloodletting",
        CardId::BurningPact => "Burning Pact",
        CardId::Carnage => "Carnage",
        CardId::Combust => "Combust",
        CardId::Corruption => "Corruption",
        CardId::DarkEmbrace => "Dark Embrace",
        CardId::Disarm => "Disarm",
        CardId::DoubleTap => "Double Tap",
        CardId::Dropkick => "Dropkick",
        CardId::DualWield => "Dual Wield",
        CardId::Entrench => "Entrench",
        CardId::Evolve => "Evolve",
        CardId::FeelNoPain => "Feel No Pain",
        CardId::FireBreathing => "Fire Breathing",
        CardId::FlameBarrier => "Flame Barrier",
        CardId::GhostlyArmor => "Ghostly Armor",
        CardId::Hemokinesis => "Hemokinesis",
        CardId::InfernalBlade => "Infernal Blade",
        CardId::Inflame => "Inflame",
        CardId::Intimidate => "Intimidate",
        CardId::Metallicize => "Metallicize",
        CardId::PowerThrough => "Power Through",
        CardId::Pummel => "Pummel",
        CardId::Rage => "Rage",
        CardId::Rampage => "Rampage",
        CardId::RecklessCharge => "Reckless Charge",
        CardId::Rupture => "Rupture",
        CardId::SearingBlow => "Searing Blow",
        CardId::SecondWind => "Second Wind",
        CardId::SeeingRed => "Seeing Red",
        CardId::Sentinel => "Sentinel",
        CardId::SeverSoul => "Sever Soul",
        CardId::Shockwave => "Shockwave",
        CardId::SpotWeakness => "Spot Weakness",
        CardId::Uppercut => "Uppercut",
        CardId::Whirlwind => "Whirlwind",

        // --- Ironclad Rare ---
        CardId::Barricade => "Barricade",
        CardId::Berserk => "Berserk",
        CardId::Bludgeon => "Bludgeon",
        CardId::Brutality => "Brutality",
        CardId::DemonForm => "Demon Form",
        CardId::Exhume => "Exhume",
        CardId::Feed => "Feed",
        CardId::FiendFire => "Fiend Fire",
        CardId::Immolate => "Immolate",
        CardId::Impervious => "Impervious",
        CardId::Juggernaut => "Juggernaut",
        CardId::LimitBreak => "Limit Break",
        CardId::Offering => "Offering",
        CardId::Reaper => "Reaper",

        // --- Status ---
        CardId::Burn => "Burn",
        CardId::Dazed => "Dazed",
        CardId::Slimed => "Slimed",
        CardId::Wound => "Wound",
        CardId::Void => "Void",

        // --- Curses ---
        CardId::AscendersBane => "AscendersBane",
        CardId::Clumsy => "Clumsy",
        CardId::CurseOfTheBell => "CurseOfTheBell",
        CardId::Decay => "Decay",
        CardId::Doubt => "Doubt",
        CardId::Injury => "Injury",
        CardId::Necronomicurse => "Necronomicurse",
        CardId::Normality => "Normality",
        CardId::Pain => "Pain",
        CardId::Parasite => "Parasite",
        CardId::Pride => "Pride",
        CardId::Regret => "Regret",
        CardId::Shame => "Shame",
        CardId::Writhe => "Writhe",

        // --- Special / Temp ---
        CardId::Miracle => "Miracle",
        CardId::Shiv => "Shiv",
        CardId::Bite => "Bite",
        CardId::Apparition => "Ghostly",
        CardId::Madness => "Madness",
        CardId::RitualDagger => "RitualDagger",
        CardId::JAX => "J.A.X.",
        CardId::Finesse => "Finesse",

        // --- Colorless Uncommon ---
        CardId::BandageUp => "Bandage Up",
        CardId::Blind => "Blind",
        CardId::DarkShackles => "Dark Shackles",
        CardId::DeepBreath => "Deep Breath",
        CardId::Discovery => "Discovery",
        CardId::DramaticEntrance => "Dramatic Entrance",
        CardId::Enlightenment => "Enlightenment",
        CardId::FlashOfSteel => "Flash of Steel",
        CardId::Forethought => "Forethought",
        CardId::GoodInstincts => "Good Instincts",
        CardId::Impatience => "Impatience",
        CardId::JackOfAllTrades => "Jack Of All Trades",
        CardId::MindBlast => "Mind Blast",
        CardId::Panacea => "Panacea",
        CardId::PanicButton => "PanicButton",
        CardId::Purity => "Purity",
        CardId::SwiftStrike => "Swift Strike",
        CardId::Trip => "Trip",

        // --- Colorless Rare ---
        CardId::Apotheosis => "Apotheosis",
        CardId::Chrysalis => "Chrysalis",
        CardId::HandOfGreed => "HandOfGreed",
        CardId::Magnetism => "Magnetism",
        CardId::MasterOfStrategy => "Master of Strategy",
        CardId::Mayhem => "Mayhem",
        CardId::Metamorphosis => "Metamorphosis",
        CardId::Panache => "Panache",
        CardId::SadisticNature => "Sadistic Nature",
        CardId::SecretTechnique => "Secret Technique",
        CardId::SecretWeapon => "Secret Weapon",
        CardId::TheBomb => "The Bomb",
        CardId::ThinkingAhead => "Thinking Ahead",
        CardId::Transmutation => "Transmutation",
        CardId::Violence => "Violence",
        CardId::DeadlyPoison => "Deadly Poison",
        CardId::BouncingFlask => "Bouncing Flask",
        CardId::Catalyst => "Catalyst",
        CardId::NoxiousFumes => "Noxious Fumes",
        CardId::Footwork => "Footwork",
        CardId::BladeDance => "Blade Dance",
        CardId::CloakAndDagger => "Cloak And Dagger",
        CardId::Backflip => "Backflip",
        CardId::Acrobatics => "Acrobatics",
        CardId::Prepared => "Prepared",
        CardId::DaggerThrow => "Dagger Throw",
        CardId::PoisonedStab => "Poisoned Stab",
        CardId::DaggerSpray => "Dagger Spray",
        CardId::Adrenaline => "Adrenaline",
        CardId::AfterImage => "After Image",
        CardId::Burst => "Burst",
    }
}

/// Builds a reverse lookup: Java cardID string → Rust CardId.
/// Used by verification pipeline to map Java card data to Rust types.
pub fn build_java_id_map() -> std::collections::HashMap<&'static str, CardId> {
    use CardId::*;
    let all_ids = [
        Strike,
        Defend,
        Bash,
        StrikeG,
        DefendG,
        Neutralize,
        Survivor,
        Anger,
        Armaments,
        BodySlam,
        Clash,
        Cleave,
        Clothesline,
        Flex,
        Havoc,
        Headbutt,
        HeavyBlade,
        IronWave,
        PerfectedStrike,
        PommelStrike,
        ShrugItOff,
        SwordBoomerang,
        ThunderClap,
        TrueGrit,
        TwinStrike,
        Warcry,
        WildStrike,
        BattleTrance,
        BloodForBlood,
        Bloodletting,
        BurningPact,
        Carnage,
        Combust,
        Corruption,
        DarkEmbrace,
        Disarm,
        DoubleTap,
        Dropkick,
        DualWield,
        Entrench,
        Evolve,
        FeelNoPain,
        FireBreathing,
        FlameBarrier,
        GhostlyArmor,
        Hemokinesis,
        InfernalBlade,
        Inflame,
        Intimidate,
        Metallicize,
        PowerThrough,
        Pummel,
        Rage,
        Rampage,
        RecklessCharge,
        Rupture,
        SearingBlow,
        SecondWind,
        SeeingRed,
        Sentinel,
        SeverSoul,
        Shockwave,
        SpotWeakness,
        Uppercut,
        Whirlwind,
        Barricade,
        Berserk,
        Bludgeon,
        Brutality,
        DemonForm,
        Exhume,
        Feed,
        FiendFire,
        Immolate,
        Impervious,
        Juggernaut,
        LimitBreak,
        Offering,
        Reaper,
        Burn,
        Dazed,
        Slimed,
        Wound,
        Void,
        AscendersBane,
        Clumsy,
        CurseOfTheBell,
        Decay,
        Doubt,
        Injury,
        Necronomicurse,
        Normality,
        Pain,
        Parasite,
        Pride,
        Regret,
        Shame,
        Writhe,
        Miracle,
        Shiv,
        Bite,
        Apparition,
        Madness,
        RitualDagger,
        JAX,
        Finesse,
        BandageUp,
        Blind,
        DarkShackles,
        DeepBreath,
        Discovery,
        DramaticEntrance,
        Enlightenment,
        FlashOfSteel,
        Forethought,
        GoodInstincts,
        Impatience,
        JackOfAllTrades,
        MindBlast,
        Panacea,
        PanicButton,
        Purity,
        SwiftStrike,
        Trip,
        Apotheosis,
        Chrysalis,
        HandOfGreed,
        Magnetism,
        MasterOfStrategy,
        Mayhem,
        Metamorphosis,
        Panache,
        SadisticNature,
        SecretTechnique,
        SecretWeapon,
        TheBomb,
        ThinkingAhead,
        Transmutation,
        Violence,
        DeadlyPoison,
        BouncingFlask,
        Catalyst,
        NoxiousFumes,
        Footwork,
        BladeDance,
        CloakAndDagger,
        Backflip,
        Acrobatics,
        Prepared,
        DaggerThrow,
        PoisonedStab,
        DaggerSpray,
        Adrenaline,
        AfterImage,
        Burst,
    ];
    let mut map = std::collections::HashMap::with_capacity(all_ids.len());
    for id in all_ids {
        map.insert(java_id(id), id);
    }
    map
}
