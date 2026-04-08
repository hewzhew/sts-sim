use crate::core::EntityId;
use crate::state::{
    GridSelectFilter, GridSelectReason, HandSelectFilter, HandSelectReason, PileType,
};

#[derive(Clone, Debug, PartialEq)]
pub struct DamageInfo {
    pub source: EntityId,
    pub target: EntityId,
    pub base: i32,
    pub output: i32,
    pub damage_type: DamageType,
    pub is_modified: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Damage(DamageInfo),
    DamageAllEnemies {
        source: EntityId,
        damages: smallvec::SmallVec<[i32; 5]>,
        damage_type: DamageType,
        is_modified: bool,
    },
    GainBlock {
        target: EntityId,
        amount: i32,
    },
    GainBlockRandomMonster {
        source: EntityId,
        amount: i32,
    },
    LoseBlock {
        target: EntityId,
        amount: i32,
    },
    LoseHp {
        target: EntityId,
        amount: i32,
    },
    Heal {
        target: EntityId,
        amount: i32,
    },
    GainEnergy {
        amount: i32,
    },
    GainMaxHp {
        amount: i32,
    },
    LoseMaxHp {
        target: EntityId,
        amount: i32,
    },
    AttackDamageRandomEnemy {
        base_damage: i32,
        damage_type: DamageType,
        applies_target_modifiers: bool,
    },
    DropkickDamageAndEffect {
        target: EntityId,
        damage_info: DamageInfo,
    },
    FiendFire {
        target: EntityId,
        damage_info: DamageInfo,
    },
    ExhaustAllNonAttack,
    BlockPerNonAttack {
        block_per_card: i32,
    },
    Feed {
        target: EntityId,
        damage_info: DamageInfo,
        max_hp_amount: i32,
    },
    VampireDamage(DamageInfo),
    VampireDamageAllEnemies {
        source: EntityId,
        damages: smallvec::SmallVec<[i32; 5]>,
        damage_type: DamageType,
    },
    LimitBreak,
    DrawCards(u32),
    EmptyDeckShuffle,
    PlayCard {
        card_index: usize,
        target: Option<EntityId>,
    },
    PlayCardDirect {
        card: Box<crate::combat::CombatCard>,
        target: Option<EntityId>,
        purge: bool,
    },
    UsePotion {
        slot: usize,
        target: Option<EntityId>,
    },
    DiscardPotion {
        slot: usize,
    },
    UseCard {
        card_id: crate::content::cards::CardId,
        uuid: u32,
        exhaust: bool,
        rebound: bool,
        shuffle_back: bool,
        return_to_hand: bool,
        purge: bool,
    },
    ExhaustCard {
        card_uuid: u32,
        source_pile: PileType,
    },
    ExhaustRandomCard {
        amount: usize,
    },
    DiscardCard {
        card_uuid: u32,
    },
    MoveCard {
        card_uuid: u32,
        from: PileType,
        to: PileType,
    },
    SuspendForHandSelect {
        min: u8,
        max: u8,
        can_cancel: bool,
        filter: HandSelectFilter,
        reason: HandSelectReason,
    },
    SuspendForGridSelect {
        source_pile: PileType,
        min: u8,
        max: u8,
        can_cancel: bool,
        filter: GridSelectFilter,
        reason: GridSelectReason,
    },
    SuspendForDiscovery {
        card_type: Option<crate::content::cards::CardType>,
        cost_for_turn: Option<u8>,
    },
    /// StancePotion: Java ChooseOneAction(ChooseWrath, ChooseCalm)
    SuspendForStanceChoice,
    ApplyPower {
        source: EntityId,
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        amount: i32,
    },
    RemovePower {
        target: EntityId,
        power_id: crate::content::powers::PowerId,
    },
    RemoveAllDebuffs {
        target: EntityId,
    },
    AwakenedRebirthClear {
        target: EntityId,
    },
    UpdatePowerExtraData {
        target: EntityId,
        power_id: crate::combat::PowerId,
        value: i32,
    },
    MakeTempCardInHand {
        card_id: crate::content::cards::CardId,
        amount: u8,
        upgraded: bool,
    },
    MakeTempCardInDiscard {
        card_id: crate::content::cards::CardId,
        amount: u8,
        upgraded: bool,
    },
    MakeCopyInHand {
        original: Box<crate::combat::CombatCard>,
        amount: u8,
    },
    MakeCopyInDiscard {
        original: Box<crate::combat::CombatCard>,
        amount: u8,
    },
    MakeTempCardInDiscardAndDeck {
        card_id: crate::content::cards::CardId,
        amount: u8,
    },
    MakeRandomCardInHand {
        card_type: Option<crate::content::cards::CardType>,
        cost_for_turn: Option<u8>,
    },
    MakeRandomColorlessCardInHand {
        rarity: crate::content::cards::CardRarity,
        cost_for_turn: Option<u8>,
    },
    ReduceAllHandCosts {
        amount: u8,
    },
    RandomizeHandCosts,
    UpgradeAllInHand,
    /// Hexaghost's BurnIncreaseAction: upgrades all Burn cards in draw pile and discard pile.
    UpgradeAllBurns,
    MakeTempCardInDrawPile {
        card_id: crate::content::cards::CardId,
        amount: u8,
        random_spot: bool,
        upgraded: bool,
    },
    PlayTopCard {
        target: Option<EntityId>,
        exhaust: bool,
    },
    /// Java PlayTopCardAction with random target selection via cardRandomRng.
    /// Used by DistilledChaosPotion and similar.
    ModifyCardMisc {
        card_uuid: u32,
        amount: i32,
    },
    UpgradeCard {
        card_uuid: u32,
    },
    /// Java: UpgradeRandomCardAction — filters upgradeable non-STATUS cards, shuffles, upgrades first.
    UpgradeRandomCard,
    ExecuteMonsterTurn(EntityId),
    RollMonsterMove {
        monster_id: EntityId,
    },
    SetMonsterMove {
        monster_id: EntityId,
        next_move_byte: u8,
        intent: crate::combat::Intent,
    },
    AbortDeath {
        target: EntityId,
    },
    Suicide {
        target: EntityId,
    },
    IncreaseMaxOrb(u8),
    SpawnMonster {
        monster_id: crate::content::monsters::EnemyId,
        slot: u8,
        current_hp: i32,
        max_hp: i32,
        logical_position: i32,
    },
    SpawnMonsterSmart {
        monster_id: crate::content::monsters::EnemyId,
        logical_position: i32,
        current_hp: i32,
        max_hp: i32,
    },
    SpawnEncounter {
        encounter: crate::content::monsters::factory::EncounterId,
    },
    Escape {
        target: EntityId,
    },
    FleeCombat,
    /// Deferred card-to-pile placement (matches Java UseCardAction.update() ordering)
    /// Card is held in limbo until this action fires, then moved to discard/exhaust.
    UseCardDone {
        should_exhaust: bool,
    },
    StartTurnTrigger,
    PostDrawTrigger,
    EndTurnTrigger,
    PreBattleTrigger,
    BattleStartPreDrawTrigger,
    BattleStartTrigger,
    ClearCardQueue,
    AddCardToMasterDeck {
        card_id: crate::content::cards::CardId,
    },
    ApplyStasis {
        target_id: EntityId,
    },
    UpdateRelicCounter {
        relic_id: crate::content::relics::RelicId,
        counter: i32,
    },
    UpdateRelicAmount {
        relic_id: crate::content::relics::RelicId,
        amount: i32,
    },
    UpdateRelicUsedUp {
        relic_id: crate::content::relics::RelicId,
        used_up: bool,
    },
    ChannelOrb(crate::combat::OrbId),
    EvokeOrb,
    TriggerPassiveOrbs,
    Scry(usize),
    EnterStance(String), // Watcher stance: "Wrath", "Calm", "Divinity", "Neutral"
    MummifiedHandEffect,
    ObtainPotion,
    ObtainSpecificPotion(crate::content::potions::PotionId),
    /// Unified action for NilrysCodex (Codex) / Toolbox (ChooseOneColorless) / similar relic reward screens.
    /// Generates 3 unique random cards from `pool`, player picks 1.
    /// Card goes to `destination`. If `can_skip`, player can cancel.
    SuspendForCardReward {
        pool: CardRewardPool,
        destination: CardDestination,
        can_skip: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardRewardPool {
    /// All class cards (Common + Uncommon + Rare), any type
    /// Java: returnTrulyRandomCardInCombat()
    ClassAll,
    /// Colorless cards (Uncommon + Rare)
    /// Java: returnTrulyRandomColorlessCardInCombat()
    Colorless,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardDestination {
    /// Add to hand (overflow to discard if hand >= 10)
    Hand,
    /// Add to draw pile at a random position
    DrawPileRandom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DamageType {
    Normal,
    Thorns,
    HpLoss,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActionInfo {
    pub action: Action,
    pub insertion_mode: AddTo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AddTo {
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventPayload {
    CardPlayed {
        card_uuid: u32,
        card_id: crate::content::cards::CardId,
        target_id: Option<EntityId>,
    },
    CardDrawn {
        card_uuid: u32,
    },
    CardExhausted {
        card_uuid: u32,
        card_id: crate::content::cards::CardId,
    },
    CardDiscarded {
        card_uuid: u32,
    },
    HpLost {
        amount: i32,
    },
    DamageTaken {
        amount: i32,
        source: EntityId,
    },
    Attacked {
        target: EntityId,
        source: EntityId,
        amount: i32,
    },
    BlockGained {
        amount: i32,
        target: EntityId,
    },
    MonsterDied {
        target_id: EntityId,
    },
    TurnStarted,
    PostDraw,
    PlayerTurnEnded,
    MonsterTurnEnded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModifierContext {
    pub source: EntityId,
    pub target: EntityId,
    pub original_damage: i32,
    pub damage_type: DamageType,
}

pub type HookId = usize;
pub type DamageModifierId = usize;
pub type CardHookId = usize;

#[derive(Clone, Default)]
pub struct ModifierBus {
    pub on_attack_to_change_damage: Vec<DamageModifierId>, // Attacker changes damage
    pub on_attacked_to_change_damage: Vec<DamageModifierId>, // Defender changes damage
    pub on_attack_hooks: Vec<HookId>,                      // Execute after damage is calculated
    pub on_attacked_hooks: Vec<HookId>,                    // Execute after being attacked
    pub on_lose_hp_last: Vec<DamageModifierId>,            // Final modifier (Tungsten Rod)
    pub on_lose_hp: Vec<HookId>,
    pub was_hp_lost_hooks: Vec<HookId>,
    pub on_death_hooks: Vec<HookId>,
    pub on_monster_death_hooks: Vec<HookId>,

    // Card and Combat flow hooks
    pub on_calculate_damage: Vec<DamageModifierId>,
    pub on_play_card: Vec<CardHookId>,
    pub on_combat_start: Vec<HookId>,
    pub on_turn_start: Vec<HookId>,
}
