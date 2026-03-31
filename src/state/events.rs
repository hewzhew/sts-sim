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
#[derive(Clone, Debug, PartialEq)]
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
