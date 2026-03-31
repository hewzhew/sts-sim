use crate::content::cards::CardId;
use crate::rng::RngPool;

pub type RelicId = usize;
pub type PotionId = usize;
pub type EventId = usize;

#[derive(Clone, Debug)]
pub struct GameState {
    pub rng: RngPool,

    pub player_class: PlayerClass,
    pub ascension_level: u8,
    pub floor_num: u32,
    pub act_num: u8,

    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,

    pub master_deck: MasterDeck,
    pub relics: RelicInventory,
    pub potions: PotionInventory,

    pub has_ruby_key: bool,
    pub has_emerald_key: bool,
    pub has_sapphire_key: bool,

    pub shop_purge_cost: i32,
    pub potions_bought_this_run: u32,
    pub boss_relics_taken: u8,
    pub seen_events: Vec<EventId>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerClass {
    Ironclad,
    Silent,
    Defect,
    Watcher,
}

#[derive(Clone, Debug, Default)]
pub struct MasterDeck {
    pub cards: Vec<DeckCard>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckCard {
    pub id: CardId,
    pub uuid: u32,
    pub upgrades: u8,
    pub misc_value: i32,
    pub is_bottled_lightning: bool,
    pub is_bottled_flame: bool,
    pub is_bottled_tornado: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RelicInventory {
    pub active_relics: Vec<RelicState>,
    pub max_energy_boost: u8, 
}

#[derive(Clone, Debug, PartialEq)]
pub enum RelicState {
    Stateless(RelicId),
    PenNib { attacks: u8 },       
    IncenseBurner { turns: u8 },  
    Sundial { shuffles: u8 },     
    HappyFlower { turns: u8 },    
    Abacus,
    Girya { lift_count: u8 },
    Torii,
    LizardTail { used: bool },
    FairyInABottle { used: bool },
    NeowsLament { charges: u8 },
    Omamori { charges: u8 },
    VoodooDoll,
}

#[derive(Clone, Debug, Default)]
pub struct PotionInventory {
    pub capacity: u8,
    pub slots: Vec<Option<PotionId>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PotionIdEnum { // Renamed from PotionId to avoid conflict with usize alias
    FirePotion,
    BlockPotion,
    EnergyPotion,
    ExplosivePotion,
    EntropicBrew,
}
