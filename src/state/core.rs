use crate::core::EntityId;
use crate::content::cards::CardId;

#[derive(Clone, Debug, PartialEq)]
pub enum EngineState {
    CombatPlayerTurn,
    CombatProcessing,
    RewardScreen(crate::state::reward::RewardState),
    Campfire,
    Shop(crate::shop::ShopState),
    MapNavigation,
    EventRoom,
    PendingChoice(PendingChoice),
    RunPendingChoice(RunPendingChoiceState), // Out of combat selection wrapper
    /// Event-triggered combat: carries pre-populated rewards and post-combat return info.
    /// Combat proceeds normally (CombatPlayerTurn), and when it ends, the engine
    /// checks this state to determine how to handle rewards and where to return.
    EventCombat(EventCombatState),
    BossRelicSelect(crate::state::reward::BossRelicChoiceState),
    GameOver(RunResult),
}

/// State for event-triggered combat.
#[derive(Clone, Debug, PartialEq)]
pub struct EventCombatState {
    /// Pre-populated rewards (gold, relics) added before combat starts.
    pub rewards: crate::state::reward::RewardState,
    /// If false, skip the reward screen entirely after combat (e.g., Colosseum fight 1).
    pub reward_allowed: bool,
    /// If true, suppress card rewards in the reward screen.
    pub no_cards_in_rewards: bool,
    /// Where to transition after combat + rewards are done.
    pub post_combat_return: PostCombatReturn,
    /// Monster encounter key (e.g., "2 Orb Walkers") for identification.
    pub encounter_key: &'static str,
}

/// Where to go after event combat finishes.
#[derive(Clone, Debug, PartialEq)]
pub enum PostCombatReturn {
    /// Return to the event dialog (e.g., Colosseum between fights).
    EventRoom,
    /// Standard: combat done → rewards → map navigation.
    MapNavigation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunPendingChoiceReason {
    Purge,
    Upgrade,
    Transform,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunPendingChoiceState {
    pub min_choices: usize,
    pub max_choices: usize,
    pub reason: RunPendingChoiceReason,
    pub return_state: Box<EngineState>, // State to revert to after selection
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunResult {
    Victory,
    Defeat,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PendingChoice {
    GridSelect {
        source_pile: PileType, 
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: GridSelectReason,
    },
    HandSelect {
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: HandSelectReason,
    },
    DiscoverySelect(Vec<CardId>),
    TargetSelect(TargetValidation),
    ScrySelect {
        cards: Vec<CardId>,
        card_uuids: Vec<u32>,
    },
    /// Unified card reward selection (NilrysCodex, Toolbox, etc.)
    /// Player picks 1 card from `cards`. Card goes to `destination`.
    /// If `can_skip`, player can Cancel to skip without picking.
    CardRewardSelect {
        cards: Vec<CardId>,
        destination: crate::action::CardDestination,
        can_skip: bool,
    },
    /// StancePotion: player chooses Wrath or Calm. Index 0 = Wrath, 1 = Calm.
    StanceChoice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PileType {
    Draw,
    Discard,
    Exhaust,
    Hand,
    Limbo,
    MasterDeck,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HandSelectReason {
    Exhaust,
    Discard,
    Retain,
    PutOnDrawPile,
    PutToBottomOfDraw,  // Forethought: moved cards become free_to_play_once
    Copy { amount: u8 },
    Upgrade,
    GamblingChip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridSelectReason {
    MoveToDrawPile,
    Exhume { upgrade: bool },
    SkillFromDeckToHand,    // SecretTechnique: pick Skill from draw → hand
    AttackFromDeckToHand,   // SecretWeapon: pick Attack from draw → hand
    DiscardToHand,          // LiquidMemories: pick from discard → hand (cost 0)
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TargetValidation {
    AnyEnemy,
    AnyMonster,
}

#[derive(Clone, Debug)]
pub enum ClientInput {
    PlayCard { card_index: usize, target: Option<EntityId> },
    UsePotion { potion_index: usize, target: Option<EntityId> },
    DiscardPotion(usize),
    EndTurn,
    SubmitCardChoice(Vec<usize>),
    SubmitDiscoverChoice(usize),
    SelectMapNode(usize),
    FlyToNode(usize, usize), // (x, y) — WingBoots flight to non-adjacent node
    SelectEventOption(usize),
    CampfireOption(CampfireChoice),
    EventChoice(usize),
    SubmitScryDiscard(Vec<usize>), // Array of indices (0 to N-1) to discard
    SubmitHandSelect(Vec<u32>),    // Array of card UUIDs selected
    SubmitGridSelect(Vec<u32>),    // Array of card UUIDs selected from grid (discard/draw)
    SubmitDeckSelect(Vec<usize>),  // Array of absolute master_deck indices selected
    ClaimReward(usize), // Index of the RewardItem to claim
    SelectCard(usize),  // Pick card at index from pending_card_choice
    BuyCard(usize),
    BuyRelic(usize),
    BuyPotion(usize),
    PurgeCard(usize), // Purge card at index in master deck
    SubmitRelicChoice(usize), // Pick boss relic at index from BossRelicSelect screen
    Proceed, // Used to skip screens (Reward, Campfire, BossRelicSelect, etc)
    Cancel,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CampfireChoice {
    Rest,
    Smith(usize),
    Dig,
    Lift,
    Toke(usize),
    Recall, // Ruby Key: skip rest to obtain the Ruby Key
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TopLevelState {
    InCombat,
    OnMap,
    AtCampfire,
    InShop,
    OnRewardScreen,
    OnEvent,
}
