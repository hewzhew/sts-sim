pub mod abacus;
pub mod akabeko;
pub mod anchor;
pub mod ancient_tea_set;
pub mod art_of_war;
pub mod bag_of_marbles;
pub mod bag_of_preparation;
pub mod bird_faced_urn;
pub mod blood_vial;
pub mod brimstone;
pub mod bronze_scales;
pub mod burning_blood;
pub mod captains_wheel;
pub mod centennial_puzzle;
pub mod champion_belt;
pub mod charons_ashes;
pub mod clockwork_souvenir;
pub mod dark_blood;
pub mod dodecahedron;
pub mod fossilized_helix;
pub mod girya;
pub mod gremlin_horn;
pub mod happy_flower;
pub mod hooks;
pub mod horn_cleat;
pub mod ice_cream;
pub mod incense_burner;
pub mod ink_bottle;
pub mod inserter;
pub mod kunai;
pub mod lantern;

// Missing A-H Batch A-B
pub mod astrolabe;
pub mod black_blood;
pub mod black_star;
pub mod bloody_idol;
pub mod blue_candle;
pub mod boot;
pub mod bottled_flame;
pub mod bottled_lightning;
pub mod bottled_tornado;
pub mod busted_crown;

// Missing A-H Batch C Part 1
pub mod calipers;
pub mod calling_bell;
pub mod cauldron;
pub mod ceramic_fish;
pub mod chemical_x;
pub mod circlet;

// Missing A-H Batch C Part 2
pub mod cloak_clasp;
pub mod coffee_dripper;
pub mod courier;
pub mod cracked_core;
pub mod cultist_mask;
pub mod cursed_key;

pub mod damaru;
pub mod darkstone_periapt;
pub mod data_disk;
pub mod dead_branch;
pub mod discerning_monocle;
pub mod dollys_mirror;
pub mod dream_catcher;
pub mod du_vu_doll;
pub mod duality;
pub mod ectoplasm;
pub mod emotion_chip;
pub mod empty_cage;
pub mod enchiridion;
pub mod eternal_feather;
pub mod face_of_cleric;
pub mod frozen_core;
pub mod frozen_egg;
pub mod frozen_eye;
pub mod gambling_chip;
pub mod ginger;
pub mod gold_plated_cables;
pub mod golden_eye;
pub mod golden_idol;
pub mod gremlin_mask;
pub mod hand_drill;
pub mod holy_water;
pub mod hovering_kite;
pub mod lizard_tail;
pub mod magic_flower;
pub mod mango;
pub mod meat_on_the_bone;
pub mod medical_kit;
pub mod melange;
pub mod mercury_hourglass;
pub mod mummified_hand;
pub mod ninja_scroll;
pub mod nloths_gift;
pub mod nuclear_battery;
pub mod nunchaku;
pub mod odd_mushroom;
pub mod oddly_smooth_stone;
pub mod omamori;
pub mod orichalcum;
pub mod ornamental_fan;
pub mod pantograph;
pub mod peace_pipe;
pub mod pear;
pub mod pen_nib;
pub mod potion_belt;
pub mod preserved_insect;
pub mod question_card;
pub mod red_mask;
pub mod sacred_bark;
pub mod self_forming_clay;
pub mod shovel;
pub mod singing_bowl;
pub mod smiling_mask;
pub mod snake_ring;
pub mod snecko_eye;
pub mod snecko_skull;
pub mod torii;
pub mod toxic_egg;
pub mod toy_ornithopter;
pub mod unceasing_top;
pub mod vajra;
pub mod white_beast_statue;
// P1 High-Impact Relics
pub mod letter_opener;
pub mod mark_of_pain;
pub mod mark_of_the_bloom;
pub mod mutagenic_strength;
pub mod neows_lament;
pub mod philosopher_stone;
pub mod pocketwatch;
pub mod shuriken;
pub mod stone_calendar;
pub mod sundial;
pub mod thread_and_needle;
pub mod tingsha;
pub mod tough_bandages;
pub mod tungsten_rod;
pub mod twisted_funnel;
pub mod warped_tongs;
// Remaining P1 Relics
pub mod matryoshka;
pub mod necronomicon;
pub mod orange_pellets;
pub mod pandoras_box;
pub mod paper_crane;
pub mod paper_frog;
pub mod red_skull;
pub mod runic_cube;
pub mod slavers_collar;
pub mod sling;
pub mod the_specimen;
pub mod toolbox;
pub mod velvet_choker;
pub mod violet_lotus;
pub mod wrist_blade;
// ---------- END RELIC MODULES ----------
pub mod nilrys_codex;
pub mod old_coin;
pub mod orrery;
pub mod pure_water;
pub mod runic_capacitor;
pub mod strawberry;
pub mod symbiotic_virus;
pub mod teardrop_locket;
pub mod tiny_house;
pub mod turnip;
pub mod waffle;
pub mod war_paint;
pub mod whetstone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelicTier {
    Starter,
    Common,
    Uncommon,
    Rare,
    Boss,
    Shop,
    Event,
    Special,
    Deprecated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RelicId {
    Abacus,
    Akabeko,
    Anchor,
    AncientTeaSet,
    ArtOfWar,
    Astrolabe,
    BagOfMarbles,
    BagOfPreparation,
    BirdFacedUrn,
    BlackBlood,
    BlackStar,
    BloodVial,
    BloodyIdol,
    BlueCandle,
    Boot,
    BottledFlame,
    BottledLightning,
    BottledTornado,
    Brimstone,
    BronzeScales,
    BurningBlood,
    BustedCrown,
    Calipers,
    CallingBell,
    CaptainsWheel,
    Cauldron,
    CentennialPuzzle,
    CeramicFish,
    ChampionBelt,
    CharonsAshes,
    ChemicalX,
    Circlet,
    CloakClasp,
    ClockworkSouvenir,
    CoffeeDripper,
    Courier,
    CrackedCore,
    CultistMask,
    CursedKey,
    Damaru,
    DarkBlood,
    DarkstonePeriapt,
    DataDisk,
    DeadBranch,
    DiscerningMonocle,
    Dodecahedron,
    DollysMirror,
    DreamCatcher,
    DuVuDoll,
    Duality,
    Ectoplasm,
    EmotionChip,
    EmptyCage,
    Enchiridion,
    EternalFeather,
    FaceOfCleric,
    FossilizedHelix,
    FrozenCore,
    FrozenEgg,
    FrozenEye,
    FusionHammer,
    GamblingChip,
    Ginger,
    Girya,
    GoldPlatedCables,
    GoldenEye,
    GoldenIdol,
    GremlinHorn,
    GremlinMask,
    HandDrill,
    HappyFlower,
    HolyWater,
    HornCleat,
    HoveringKite,
    IceCream,
    IncenseBurner,
    InkBottle,
    Inserter,
    JuzuBracelet,
    Kunai,
    Lantern,
    LetterOpener,
    LizardTail,
    MagicFlower,
    Mango,
    MarkOfPain,
    MarkOfTheBloom,
    Matryoshka,
    MawBank,
    MealTicket,
    MeatOnTheBone,
    MedicalKit,
    Melange,
    MembershipCard,
    MercuryHourglass,
    MoltenEgg,
    MummifiedHand,
    MutagenicStrength,
    Necronomicon,
    NeowsLament,
    NilrysCodex,
    NinjaScroll,
    NlothsGift,
    NlothsMask,
    NuclearBattery,
    Nunchaku,
    OddMushroom,
    OddlySmoothStone,
    OldCoin,
    Omamori,
    OrangePellets,
    Orichalcum,
    OrnamentalFan,
    Orrery,
    PandorasBox,
    Pantograph,
    PaperCrane,
    PaperFrog,
    PeacePipe,
    Pear,
    PenNib,
    PhilosopherStone,
    Pocketwatch,
    PotionBelt,
    PrayerWheel,
    PreservedInsect,
    PrismaticShard,
    PureWater,
    QuestionCard,
    RedMask,
    RedSkull,
    RegalPillow,
    RingOfTheSerpent,
    RunicCapacitor,
    RunicCube,
    RunicDome,
    RunicPyramid,
    SacredBark,
    SelfFormingClay,
    Shovel,
    Shuriken,
    SingingBowl,
    SlaversCollar,
    Sling,
    SmilingMask,
    SnakeRing,
    SneckoEye,
    SneckoSkull,
    Sozu,
    SpiritPoop,
    SsserpentHead,
    StoneCalendar,
    StrangeSpoon,
    Strawberry,
    StrikeDummy,
    Sundial,
    SymbioticVirus,
    TeardropLocket,
    TheSpecimen,
    ThreadAndNeedle,
    Tingsha,
    TinyChest,
    TinyHouse,
    Toolbox,
    Torii,
    ToughBandages,
    ToxicEgg,
    ToyOrnithopter,
    TungstenRod,
    Turnip,
    TwistedFunnel,
    UnceasingTop,
    Vajra,
    VelvetChoker,
    VioletLotus,
    Waffle,
    WarPaint,
    WarpedTongs,
    Whetstone,
    WhiteBeastStatue,
    WingBoots,
    WristBlade,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelicState {
    pub id: RelicId,
    pub counter: i32,
    pub used_up: bool,
    pub amount: i32,
}

impl RelicState {
    pub fn new(id: RelicId) -> Self {
        let mut counter = -1;
        match id {
            RelicId::Omamori => counter = 2,
            RelicId::NeowsLament | RelicId::WingBoots => counter = 3,
            RelicId::PenNib
            | RelicId::Nunchaku
            | RelicId::InkBottle
            | RelicId::IncenseBurner
            | RelicId::HappyFlower
            | RelicId::Sundial
            | RelicId::OrnamentalFan
            | RelicId::StoneCalendar
            | RelicId::TinyChest => counter = 0,
            _ => {}
        }

        RelicState {
            id,
            counter,
            used_up: false,
            amount: 0,
        }
    }
}

/// Returns the canonical tier for a relic, matching Java's AbstractRelic.tier field.
pub fn get_relic_tier(id: RelicId) -> RelicTier {
    use RelicId::*;
    match id {
        // Starter
        BurningBlood | CrackedCore | PureWater | SnakeRing | DarkBlood => RelicTier::Starter,
        // Common (shared)
        Akabeko | Anchor | AncientTeaSet | ArtOfWar | BagOfMarbles | BagOfPreparation
        | BloodVial | Boot | BronzeScales | CentennialPuzzle | CeramicFish | DreamCatcher
        | HappyFlower | JuzuBracelet | Lantern | MawBank | MealTicket | Nunchaku
        | OddlySmoothStone | Omamori | Orichalcum | PenNib | PotionBelt | PreservedInsect
        | RegalPillow | SmilingMask | Strawberry | TinyChest | ToyOrnithopter | Vajra
        | WarPaint | Whetstone => RelicTier::Common,
        // Common (class-specific)
        Damaru | DataDisk | RedSkull | SneckoSkull => RelicTier::Common,
        // Uncommon (shared)
        BlueCandle | BottledFlame | BottledLightning | BottledTornado | Courier
        | DarkstonePeriapt | EternalFeather | FrozenEgg | GremlinHorn | HornCleat | InkBottle
        | Kunai | LetterOpener | Matryoshka | MeatOnTheBone | MercuryHourglass | MoltenEgg
        | MummifiedHand | OrnamentalFan | Pantograph | Pear | QuestionCard | Shuriken
        | SingingBowl | StrikeDummy | Sundial | ToxicEgg | WhiteBeastStatue => RelicTier::Uncommon,
        // Uncommon (class-specific)
        Duality | GoldPlatedCables | NinjaScroll | PaperCrane | PaperFrog | SelfFormingClay
        | SymbioticVirus | TeardropLocket => RelicTier::Uncommon,
        // Rare (shared)
        BirdFacedUrn | Calipers | CaptainsWheel | DeadBranch | DuVuDoll | FossilizedHelix
        | GamblingChip | Ginger | Girya | IceCream | IncenseBurner | LizardTail | Mango
        | OldCoin | PeacePipe | Pocketwatch | PrayerWheel | Shovel | StoneCalendar
        | ThreadAndNeedle | Torii | TungstenRod | Turnip | UnceasingTop | WingBoots => {
            RelicTier::Rare
        }
        // Rare (class-specific)
        ChampionBelt | CharonsAshes | CloakClasp | EmotionChip | GoldenEye | MagicFlower
        | TheSpecimen | Tingsha | ToughBandages => RelicTier::Rare,
        // Boss (shared)
        Astrolabe | BlackStar | BustedCrown | CallingBell | CoffeeDripper | CursedKey
        | Ectoplasm | EmptyCage | FusionHammer | PandorasBox | PhilosopherStone | RunicDome
        | RunicPyramid | SacredBark | SlaversCollar | SneckoEye | Sozu | TinyHouse
        | VelvetChoker => RelicTier::Boss,
        // Boss (class-specific)
        BlackBlood | FrozenCore | HolyWater | HoveringKite | Inserter | MarkOfPain
        | NuclearBattery | RingOfTheSerpent | RunicCube | VioletLotus | WristBlade => {
            RelicTier::Boss
        }
        // Shop (shared)
        Abacus | Cauldron | ChemicalX | ClockworkSouvenir | DiscerningMonocle | DollysMirror
        | FrozenEye | HandDrill | MedicalKit | MembershipCard | OrangePellets | Orrery
        | PrismaticShard | Sling | StrangeSpoon | Toolbox | Waffle => RelicTier::Shop,
        // Shop (class-specific)
        Brimstone | Melange | RunicCapacitor | TwistedFunnel => RelicTier::Shop,
        // Special / Event (never in relic pools)
        BloodyIdol | Circlet | CultistMask | Enchiridion | FaceOfCleric | GoldenIdol
        | GremlinMask | MarkOfTheBloom | MutagenicStrength | Necronomicon | NeowsLament
        | NilrysCodex | NlothsGift | NlothsMask | OddMushroom | RedMask | SpiritPoop
        | SsserpentHead | WarpedTongs => RelicTier::Special,
        // Fallback
        _ => RelicTier::Special,
    }
}

/// Indicates which player class a relic belongs to.
/// Shared relics return None. Class-specific relics return Some(class_name).
fn relic_class(id: RelicId) -> Option<&'static str> {
    use RelicId::*;
    match id {
        // Red (Ironclad)
        BlackBlood | Brimstone | BurningBlood | ChampionBelt | CharonsAshes | DarkBlood
        | MagicFlower | MarkOfPain | PaperFrog | RedSkull | RunicCube | SelfFormingClay => {
            Some("Ironclad")
        }
        // Green (Silent)
        HoveringKite | NinjaScroll | PaperCrane | RingOfTheSerpent | SnakeRing | SneckoSkull
        | TheSpecimen | Tingsha | ToughBandages | TwistedFunnel | WristBlade => Some("Silent"),
        // Blue (Defect)
        CrackedCore | DataDisk | EmotionChip | FrozenCore | GoldPlatedCables | Inserter
        | NuclearBattery | RunicCapacitor | SymbioticVirus => Some("Defect"),
        // Purple (Watcher)
        CloakClasp | Damaru | Duality | GoldenEye | HolyWater | Melange | PureWater
        | TeardropLocket | VioletLotus => Some("Watcher"),
        _ => None,
    }
}

/// Builds a relic pool for a given tier and player class.
/// Includes all shared relics of that tier + class-specific relics of that tier.
/// Mirrors Java's `RelicLibrary.populateRelicPool(pool, tier, playerClass)`.
pub fn build_relic_pool(tier: RelicTier, player_class: &str) -> Vec<RelicId> {
    use RelicId::*;
    // All known RelicIds — iterate and filter
    const ALL_RELICS: &[RelicId] = &[
        Abacus,
        Akabeko,
        Anchor,
        AncientTeaSet,
        ArtOfWar,
        Astrolabe,
        BagOfMarbles,
        BagOfPreparation,
        BirdFacedUrn,
        BlackBlood,
        BlackStar,
        BloodVial,
        BloodyIdol,
        BlueCandle,
        Boot,
        BottledFlame,
        BottledLightning,
        BottledTornado,
        Brimstone,
        BronzeScales,
        BurningBlood,
        BustedCrown,
        Calipers,
        CallingBell,
        CaptainsWheel,
        Cauldron,
        CentennialPuzzle,
        CeramicFish,
        ChampionBelt,
        CharonsAshes,
        ChemicalX,
        Circlet,
        CloakClasp,
        ClockworkSouvenir,
        CoffeeDripper,
        Courier,
        CrackedCore,
        CultistMask,
        CursedKey,
        Damaru,
        DarkBlood,
        DarkstonePeriapt,
        DataDisk,
        DeadBranch,
        Dodecahedron,
        DiscerningMonocle,
        DollysMirror,
        DreamCatcher,
        DuVuDoll,
        Duality,
        Ectoplasm,
        EmotionChip,
        EmptyCage,
        Enchiridion,
        EternalFeather,
        FaceOfCleric,
        FossilizedHelix,
        FrozenCore,
        FrozenEgg,
        FrozenEye,
        FusionHammer,
        GamblingChip,
        Ginger,
        Girya,
        GoldPlatedCables,
        GoldenEye,
        GoldenIdol,
        GremlinHorn,
        GremlinMask,
        HandDrill,
        HappyFlower,
        HolyWater,
        HornCleat,
        HoveringKite,
        IceCream,
        IncenseBurner,
        InkBottle,
        Inserter,
        JuzuBracelet,
        Kunai,
        Lantern,
        LetterOpener,
        LizardTail,
        MagicFlower,
        Mango,
        MarkOfPain,
        MarkOfTheBloom,
        Matryoshka,
        MawBank,
        MealTicket,
        MeatOnTheBone,
        MedicalKit,
        Melange,
        MembershipCard,
        MercuryHourglass,
        MoltenEgg,
        MummifiedHand,
        MutagenicStrength,
        Necronomicon,
        NeowsLament,
        NilrysCodex,
        NinjaScroll,
        NlothsGift,
        NlothsMask,
        NuclearBattery,
        Nunchaku,
        OddMushroom,
        OddlySmoothStone,
        OldCoin,
        Omamori,
        OrangePellets,
        Orichalcum,
        OrnamentalFan,
        Orrery,
        PandorasBox,
        Pantograph,
        PaperCrane,
        PaperFrog,
        PeacePipe,
        Pear,
        PenNib,
        PhilosopherStone,
        Pocketwatch,
        PotionBelt,
        PrayerWheel,
        PreservedInsect,
        PrismaticShard,
        PureWater,
        QuestionCard,
        RedMask,
        RedSkull,
        RegalPillow,
        RingOfTheSerpent,
        RunicCapacitor,
        RunicCube,
        RunicDome,
        RunicPyramid,
        SacredBark,
        SelfFormingClay,
        Shovel,
        Shuriken,
        SingingBowl,
        SlaversCollar,
        Sling,
        SmilingMask,
        SnakeRing,
        SneckoEye,
        SneckoSkull,
        Sozu,
        SpiritPoop,
        SsserpentHead,
        StoneCalendar,
        StrangeSpoon,
        Strawberry,
        StrikeDummy,
        Sundial,
        SymbioticVirus,
        TeardropLocket,
        TheSpecimen,
        ThreadAndNeedle,
        Tingsha,
        TinyChest,
        TinyHouse,
        Toolbox,
        Torii,
        ToughBandages,
        ToxicEgg,
        ToyOrnithopter,
        TungstenRod,
        Turnip,
        TwistedFunnel,
        UnceasingTop,
        Vajra,
        VelvetChoker,
        VioletLotus,
        Waffle,
        WarPaint,
        WarpedTongs,
        Whetstone,
        WhiteBeastStatue,
        WingBoots,
        WristBlade,
    ];

    let mut pool = Vec::new();
    for &relic in ALL_RELICS {
        if get_relic_tier(relic) != tier {
            continue;
        }
        match relic_class(relic) {
            None => pool.push(relic),                         // Shared: always include
            Some(c) if c == player_class => pool.push(relic), // Matching class
            _ => {}                                           // Different class: skip
        }
    }
    pool
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RelicSubscriptions {
    pub at_pre_battle: bool,
    pub at_battle_start_pre_draw: bool,
    pub at_battle_start: bool,
    pub at_turn_start: bool,
    pub on_use_card: bool,
    pub on_shuffle: bool,
    pub on_exhaust: bool,
    pub on_lose_hp: bool,
    pub on_victory: bool,
    pub on_apply_power: bool,
    pub on_monster_death: bool,
    pub on_spawn_monster: bool,
    pub at_end_of_turn: bool,
    pub on_discard: bool,
    pub on_use_potion: bool,
    pub on_change_stance: bool,
    pub on_attacked_to_change_damage: bool,
    pub on_lose_hp_last: bool,

    // Core Engine Value Modifiers
    pub on_calculate_heal: bool,
    pub on_calculate_x_cost: bool,
    pub on_calculate_block_retained: bool,
    pub on_calculate_energy_retained: bool,
    pub on_scry: bool,
    pub on_receive_power_modify: bool,
    pub on_calculate_vulnerable_multiplier: bool,

    // Macro/Out-of-Combat Routing Hooks
    pub on_enter_rest_room: bool,
    pub on_rest: bool,
    pub on_enter_shop: bool,
    pub on_purchase: bool,
    pub on_reward_generation: bool,
}

pub fn get_relic_subscriptions(id: RelicId) -> RelicSubscriptions {
    let mut sub = RelicSubscriptions::default();
    match id {
        RelicId::Abacus => sub.on_shuffle = true,
        RelicId::Akabeko => sub.at_battle_start = true,
        RelicId::Anchor => sub.at_battle_start = true,
        RelicId::AncientTeaSet => {
            sub.at_pre_battle = true;
            sub.at_turn_start = true;
            sub.on_enter_rest_room = true;
        }
        RelicId::ArtOfWar => {
            sub.at_pre_battle = true;
            sub.at_turn_start = true;
            sub.on_use_card = true;
        }
        RelicId::BagOfMarbles => sub.at_battle_start = true,
        RelicId::BagOfPreparation => sub.at_battle_start = true,
        RelicId::BirdFacedUrn => sub.on_use_card = true,
        RelicId::BloodVial => sub.at_battle_start = true,
        RelicId::Brimstone => sub.at_turn_start = true,
        RelicId::BronzeScales => sub.at_battle_start = true,
        RelicId::BurningBlood => sub.on_victory = true,
        RelicId::BlackBlood => sub.on_victory = true,
        RelicId::BlackStar => sub.on_victory = true,
        RelicId::BloodyIdol => {} // Requires an `on_gain_gold` hook! (Out of bounds for pure headless combat loop usually, or tracked specially)
        RelicId::BlueCandle => sub.on_use_card = true,
        RelicId::Boot => {} // Engine native query hook for on_attack_to_change_damage
        RelicId::Calipers => sub.on_calculate_block_retained = true,
        RelicId::CallingBell => {} // Out-of-Combat
        RelicId::Cauldron => {}    // Out-of-Combat
        RelicId::CeramicFish => {} // Out-of-Combat
        RelicId::ChemicalX => sub.on_calculate_x_cost = true,
        RelicId::Circlet => {} // Pure state tracker
        RelicId::CloakClasp => sub.at_end_of_turn = true,
        RelicId::CoffeeDripper => {}
        RelicId::Courier => {}
        RelicId::CrackedCore => sub.at_pre_battle = true,
        RelicId::CultistMask => sub.at_battle_start = true,
        RelicId::CursedKey => {}

        RelicId::CaptainsWheel => sub.at_turn_start = true,
        RelicId::CentennialPuzzle => {
            sub.at_pre_battle = true;
            sub.on_lose_hp = true;
        }
        RelicId::CharonsAshes => sub.on_exhaust = true,
        RelicId::ChampionBelt => sub.on_apply_power = true,
        RelicId::ClockworkSouvenir => sub.at_battle_start = true,
        RelicId::Damaru => {
            sub.at_turn_start = true;
        }
        RelicId::DarkstonePeriapt => {} // OOC
        RelicId::DataDisk => {
            sub.at_battle_start = true;
        }
        RelicId::DeadBranch => {
            sub.on_exhaust = true;
        }
        RelicId::DiscerningMonocle => {} // OOC — shop price modifier in generate_shop()
        RelicId::DollysMirror => {}      // OOC
        RelicId::DreamCatcher => {}      // OOC
        RelicId::DuVuDoll => {
            sub.at_battle_start = true;
        }
        RelicId::Duality => {
            sub.on_use_card = true;
        }
        RelicId::Ectoplasm => {}
        RelicId::EmotionChip => {
            sub.at_turn_start = true;
            sub.on_lose_hp = true;
        }
        RelicId::EmptyCage => {}
        RelicId::Enchiridion => {
            sub.at_pre_battle = true;
        }
        RelicId::EternalFeather => {}
        RelicId::FaceOfCleric => {
            sub.on_victory = true;
        }
        RelicId::FusionHammer => {
            // Out of combat / UI only
        }
        RelicId::GamblingChip => {
            sub.at_battle_start_pre_draw = true;
        }
        RelicId::Ginger => sub.on_receive_power_modify = true,
        RelicId::Turnip => sub.on_receive_power_modify = true,
        RelicId::GoldPlatedCables => {
            sub.at_end_of_turn = true;
        }
        RelicId::GoldenEye => sub.on_scry = true,
        RelicId::GoldenIdol => {
            // Gold multiplier passive
        }
        RelicId::GremlinMask => {
            sub.at_battle_start = true;
        }
        RelicId::HandDrill => {
            // Evaluated implicitly from Damage routine hooks over BlockBreak
        }
        RelicId::HolyWater => {
            sub.at_battle_start_pre_draw = true;
        }
        RelicId::HoveringKite => {
            sub.at_turn_start = true;
            sub.on_discard = true;
        }
        RelicId::IceCream => sub.on_calculate_energy_retained = true,
        RelicId::IncenseBurner => {
            sub.at_turn_start = true;
        }
        RelicId::InkBottle => {
            sub.on_use_card = true;
        }
        RelicId::FrozenCore => {
            sub.at_end_of_turn = true; // Java: onPlayerEndTurn (not atTurnStart)
        }
        RelicId::FrozenEgg => {}
        RelicId::FrozenEye => {}
        RelicId::DarkBlood => sub.on_victory = true,
        RelicId::Dodecahedron => sub.at_battle_start = true,
        RelicId::FossilizedHelix => sub.at_battle_start = true,
        RelicId::Girya => sub.at_battle_start = true,
        RelicId::GremlinHorn => sub.on_monster_death = true,
        RelicId::HappyFlower => sub.at_turn_start = true,
        RelicId::HornCleat => {
            sub.at_battle_start = true;
            sub.at_turn_start = true;
        }
        RelicId::Inserter => sub.at_turn_start = true,
        RelicId::Kunai => sub.on_use_card = true,
        RelicId::Lantern => {
            sub.at_pre_battle = true;
            sub.at_turn_start = true;
        }
        RelicId::LizardTail => sub.on_lose_hp = true,
        RelicId::MagicFlower => sub.on_calculate_heal = true,
        RelicId::MarkOfTheBloom => sub.on_calculate_heal = true,
        RelicId::Mango => {} // OOC: onEquip increaseMaxHp(14) only
        RelicId::MeatOnTheBone => sub.on_victory = true,
        RelicId::Melange => sub.on_shuffle = true,
        RelicId::MedicalKit => {
            sub.on_use_card = true; // Java: onUseCard → if Status, exhaust
        }
        RelicId::MercuryHourglass => sub.at_turn_start = true,
        RelicId::MummifiedHand => sub.on_use_card = true,
        RelicId::NinjaScroll => sub.at_battle_start_pre_draw = true,
        RelicId::NlothsGift => {} // Evaluated passively during card rewards
        RelicId::NuclearBattery => sub.at_pre_battle = true,
        RelicId::Nunchaku => sub.on_use_card = true,
        RelicId::OddMushroom | RelicId::PaperFrog => {
            sub.on_calculate_vulnerable_multiplier = true
        }
        RelicId::OddlySmoothStone => sub.at_battle_start = true,
        RelicId::Omamori => {} // Passive evaluated out of combat
        RelicId::Orichalcum => sub.at_end_of_turn = true,
        RelicId::OrnamentalFan => {
            sub.on_use_card = true;
            sub.at_turn_start = true; // resets counter
        }
        RelicId::Pantograph => sub.at_battle_start = true, // checks boss combat
        RelicId::PeacePipe => {} // Passive evaluated out of combat at rest sites
        RelicId::Pear => {}      // Passive (+10 Max HP) evaluated on acquire
        RelicId::PenNib => {
            sub.on_use_card = true;
            sub.at_battle_start = true;
        }
        RelicId::PotionBelt => {} // Passive (+2 potion slots)
        RelicId::PreservedInsect => sub.at_battle_start = true, // checks elite combat
        RelicId::QuestionCard => {} // Passive evaluated out of combat on rewards
        RelicId::SelfFormingClay => sub.on_lose_hp = true,
        RelicId::Shovel => {}      // Passive out of combat
        RelicId::SingingBowl => {} // Passive out of combat
        RelicId::SmilingMask => {} // Passive out of combat
        RelicId::SnakeRing => sub.at_battle_start = true,
        RelicId::SneckoEye => sub.at_pre_battle = true,
        RelicId::SneckoSkull => sub.on_apply_power = true,
        RelicId::Torii => sub.on_attacked_to_change_damage = true,
        RelicId::ToxicEgg => {} // Passive out of combat
        RelicId::ToyOrnithopter => sub.on_use_potion = true,
        RelicId::SacredBark => {} // Passive out of combat
        RelicId::Sozu => {}       // Passive — blocks potion obtaining
        RelicId::RunicCube => sub.on_lose_hp = true,
        RelicId::PureWater => sub.at_battle_start_pre_draw = true,
        RelicId::SymbioticVirus => sub.at_pre_battle = true, // Java: atPreBattle → channelOrb(Dark)
        RelicId::TeardropLocket => sub.at_battle_start = true, // start combat in calm
        RelicId::VioletLotus => sub.on_change_stance = true, // obtaining
        RelicId::UnceasingTop => {}                          // Engine loop evaluated natively
        RelicId::Vajra => sub.at_battle_start = true,
        RelicId::WhiteBeastStatue => {} // Passive out of combat
        RelicId::RedMask => sub.at_battle_start = true,
        // P1 High-Impact Relics
        RelicId::PhilosopherStone => {
            sub.at_battle_start = true;
            sub.on_spawn_monster = true;
        }
        RelicId::MarkOfPain => sub.at_battle_start = true,
        RelicId::ThreadAndNeedle => sub.at_battle_start = true,
        RelicId::MutagenicStrength => sub.at_battle_start = true,
        RelicId::NeowsLament => sub.at_battle_start = true,
        RelicId::TwistedFunnel => sub.at_battle_start = true,
        RelicId::Shuriken => sub.on_use_card = true,
        RelicId::LetterOpener => sub.on_use_card = true,
        RelicId::ToughBandages => sub.on_discard = true,
        RelicId::Tingsha => sub.on_discard = true,
        RelicId::StoneCalendar => {
            sub.at_turn_start = true;
            sub.at_end_of_turn = true;
        }
        RelicId::Pocketwatch => {
            sub.at_end_of_turn = true;
            sub.at_turn_start = true;
        }
        RelicId::Sundial => sub.on_shuffle = true,
        RelicId::WarpedTongs => sub.at_turn_start = true,
        RelicId::TungstenRod => {
            sub.on_lose_hp = true;
            sub.on_lose_hp_last = true;
        }
        // Remaining P1 Relics
        RelicId::Necronomicon => sub.on_use_card = true,
        RelicId::VelvetChoker => {} // Passive — engine checks can_play_card
        RelicId::OrangePellets => sub.on_use_card = true,
        RelicId::Sling => sub.at_battle_start = true,
        RelicId::WristBlade => {} // Passive — damage calc checks
        RelicId::PaperCrane => {} // Passive — damage calc constant
        RelicId::RedSkull => sub.at_battle_start = true,
        RelicId::TheSpecimen => sub.on_monster_death = true,
        RelicId::Matryoshka => {} // Passive — treasure room check
        RelicId::SlaversCollar => {
            sub.at_battle_start = true;
        } // Java: beforeEnergyPrep
        RelicId::RunicCapacitor => sub.at_pre_battle = true, // Java: atBattleStart → IncreaseMaxOrb(3)
        RelicId::NilrysCodex => sub.at_end_of_turn = true,   // Java: onPlayerEndTurn → CodexAction
        RelicId::Toolbox => sub.at_battle_start_pre_draw = true, // Java: atBattleStartPreDraw → ChooseOneColorless
        _ => {}
    }
    sub
}
