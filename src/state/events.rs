use crate::content::cards::CardId;
use crate::content::relics::RelicId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventId {
    // Act 1
    BigFish,
    Cleric,
    DeadAdventurer,
    GoldenIdol,
    LivingWall,
    Mushrooms,
    ScrapOoze,
    ShiningLight,
    Ssssserpent,
    WorldOfGoop,
    GoldenWing,
    MatchAndKeep,
    GoldenShrine,

    // Act 2
    Addict,
    BackTotheBasics,
    Beggar,
    Colosseum,
    CursedTome,
    DrugDealer,
    ForgottenAltar,
    Ghosts,
    KnowingSkull,
    MaskedBandits,
    Mausoleum,
    Nest,
    Nloth,
    TheJoust,
    TheLibrary,
    Vampires,

    // Act 3
    Falling,
    MindBloom,
    MoaiHead,
    MysteriousSphere,
    SensoryStone,
    TombRedMask,
    WindingHalls,

    // Any Act Base Shrines
    AccursedBlacksmith,
    BonfireElementals,
    BonfireSpirits,
    Designer,
    Duplicator,
    FaceTrader,
    FountainOfCurseCleansing,
    GremlinWheelGame,
    Lab,
    NoteForYourself,
    Purifier,
    Transmorgrifier,
    UpgradeShrine,
    WeMeetAgain,
    WomanInBlue,

    // Special
    Neow,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventState {
    pub id: EventId,
    pub current_screen: usize,
    pub internal_state: i32,
    pub completed: bool,
    /// When true, the engine will return to EventRoom after combat+rewards
    /// instead of transitioning to MapNavigation.
    pub combat_pending: bool,
    /// Extra data for complex events (e.g. GremlinMatchGame board).
    /// Format is event-specific.
    pub extra_data: Vec<i32>,
}

impl EventState {
    pub fn new(id: EventId) -> Self {
        EventState {
            id,
            current_screen: 0,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        }
    }
}

/// Represents a single UI button option in an event. E.g "Take 12 Damage, Obtain Golden Idol"
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventChoiceMeta {
    pub text: String,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
}

impl EventChoiceMeta {
    pub fn new(text: impl Into<String>) -> Self {
        EventChoiceMeta {
            text: text.into(),
            disabled: false,
            disabled_reason: None,
        }
    }

    pub fn disabled(text: impl Into<String>, reason: impl Into<String>) -> Self {
        EventChoiceMeta {
            text: text.into(),
            disabled: true,
            disabled_reason: Some(reason.into()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EventOptionSemantics {
    pub action: EventActionKind,
    pub effects: Vec<EventEffect>,
    pub constraints: Vec<EventOptionConstraint>,
    pub transition: EventOptionTransition,
    pub repeatable: bool,
    pub terminal: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventOption {
    pub ui: EventChoiceMeta,
    pub semantics: EventOptionSemantics,
}

impl EventOption {
    pub fn unknown(ui: EventChoiceMeta) -> Self {
        EventOption {
            ui,
            semantics: EventOptionSemantics::default(),
        }
    }

    pub fn new(ui: EventChoiceMeta, semantics: EventOptionSemantics) -> Self {
        EventOption { ui, semantics }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventActionKind {
    #[default]
    Unknown,
    Leave,
    Continue,
    Accept,
    Decline,
    Fight,
    Trade,
    DeckOperation,
    Gain,
    Special,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventEffect {
    GainGold(i32),
    LoseGold(i32),
    LoseHp(i32),
    LoseMaxHp(i32),
    Heal(i32),
    GainMaxHp(i32),
    ObtainRelic {
        count: usize,
        kind: EventRelicKind,
    },
    ObtainPotion {
        count: usize,
    },
    ObtainCard {
        count: usize,
        kind: EventCardKind,
    },
    ObtainColorlessCard {
        count: usize,
        kind: EventCardKind,
    },
    ObtainCurse {
        count: usize,
        kind: EventCardKind,
    },
    RemoveCard {
        count: usize,
        target_uuid: Option<u32>,
        kind: EventCardKind,
    },
    UpgradeCard {
        count: usize,
    },
    TransformCard {
        count: usize,
    },
    DuplicateCard {
        count: usize,
    },
    LoseRelic {
        specific: Option<RelicId>,
        starter_only: bool,
    },
    LoseStarterRelic {
        specific: Option<RelicId>,
    },
    StartCombat,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventOptionTransition {
    #[default]
    None,
    AdvanceScreen,
    Complete,
    OpenSelection(EventSelectionKind),
    OpenReward,
    StartCombat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventOptionConstraint {
    RequiresGold(i32),
    RequiresRelic(RelicId),
    RequiresRemovableCard,
    RequiresUpgradeableCard,
    RequiresTransformableCard,
    RequiresPotion,
    RequiresPotionSlotValue,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventSelectionKind {
    #[default]
    None,
    RemoveCard,
    UpgradeCard,
    TransformCard,
    DuplicateCard,
    OfferCard,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventCardKind {
    #[default]
    Unknown,
    Specific(CardId),
    RandomColorless,
    RandomClassCard,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EventRelicKind {
    #[default]
    Unknown,
    Specific(RelicId),
    RandomRelic,
    RandomBook,
    RandomFace,
}
