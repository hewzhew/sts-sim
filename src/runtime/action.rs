use crate::core::EntityId;
use crate::state::{
    GridSelectFilter, GridSelectReason, HandSelectFilter, HandSelectReason, PileType,
};

pub const NO_SOURCE: EntityId = EntityId::MAX;
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
pub enum MonsterRuntimePatch {
    Hexaghost {
        activated: Option<bool>,
        orb_active_count: Option<u8>,
        burn_upgraded: Option<bool>,
        divider_damage: Option<i32>,
        clear_divider_damage: bool,
    },
    Lagavulin {
        idle_count: Option<u8>,
        debuff_turn_count: Option<u8>,
        is_out: Option<bool>,
        is_out_triggered: Option<bool>,
    },
    Guardian {
        damage_threshold: Option<i32>,
        damage_taken: Option<i32>,
        is_open: Option<bool>,
        close_up_triggered: Option<bool>,
    },
    Byrd {
        first_move: Option<bool>,
        is_flying: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    Chosen {
        first_turn: Option<bool>,
        used_hex: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    Snecko {
        first_turn: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    ShelledParasite {
        first_move: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    BronzeAutomaton {
        first_turn: Option<bool>,
        num_turns: Option<u8>,
        protocol_seeded: Option<bool>,
    },
    BronzeOrb {
        used_stasis: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    BookOfStabbing {
        stab_count: Option<u8>,
        protocol_seeded: Option<bool>,
    },
    Collector {
        initial_spawn: Option<bool>,
        ult_used: Option<bool>,
        turns_taken: Option<u8>,
        protocol_seeded: Option<bool>,
    },
    Champ {
        first_turn: Option<bool>,
        num_turns: Option<u8>,
        forge_times: Option<u8>,
        threshold_reached: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    AwakenedOne {
        form1: Option<bool>,
        first_turn: Option<bool>,
        protocol_seeded: Option<bool>,
    },
    CorruptHeart {
        first_move: Option<bool>,
        move_count: Option<u8>,
        buff_count: Option<u8>,
        protocol_seeded: Option<bool>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Damage(DamageInfo),
    /// Java `PummelDamageAction`.
    ///
    /// Unlike ordinary `DamageAction`, execution re-checks that the target's
    /// current HP is still above zero before applying damage. Pummel queues
    /// light hits through this action and finishes with one ordinary
    /// `DamageAction`.
    PummelDamage(DamageInfo),
    /// Java `BaneAction`: execute a second attack only if the target is alive
    /// and still has Poison when this queued action resolves.
    BaneDamage(DamageInfo),
    /// Java `DamagePerAttackPlayedAction` used by Finisher.
    ///
    /// Execution reads the current attack count from the action manager state,
    /// subtracts the Finisher card itself, and queues that many ordinary
    /// `DamageAction`s to the top.
    DamagePerAttackPlayed(DamageInfo),
    /// Java `HeelHookAction`: queue damage, and if the target has Weak at
    /// execution time queue the energy/draw follow-up behind that damage.
    HeelHook(DamageInfo),
    /// Java `FlechetteAction`: count Skills in hand at execution time and
    /// queue one ordinary `DamageAction` per Skill.
    Flechettes(DamageInfo),
    /// Canonical monster attack action.
    ///
    /// Contract:
    /// - `base_damage` is the monster plan truth input.
    /// - Monster content code should emit this action instead of constructing `DamageInfo`.
    /// - For `Normal` monster attacks, engine execution re-resolves final damage from
    ///   `base_damage`; content code must not guess a modified `output` value.
    /// - Non-normal kinds such as `HpLoss` and `Thorns` use the provided numeric value
    ///   directly via the damage pipeline.
    MonsterAttack {
        source: EntityId,
        target: EntityId,
        base_damage: i32,
        damage_kind: crate::semantics::combat::DamageKind,
    },
    /// Canonical thief strike side effect.
    ///
    /// Contract:
    /// - `amount` is the locked per-hit steal cap from monster plan truth.
    /// - Execution steals `min(amount, player.gold)` immediately from the player.
    /// - Execution also records the actual stolen amount on the thief runtime state.
    StealPlayerGold {
        thief_id: EntityId,
        amount: i32,
    },
    DamageAllEnemies {
        source: EntityId,
        damages: smallvec::SmallVec<[i32; 5]>,
        damage_type: DamageType,
        is_modified: bool,
    },
    Whirlwind {
        damages: smallvec::SmallVec<[i32; 5]>,
        damage_type: DamageType,
        free_to_play_once: bool,
        energy_on_use: i32,
    },
    Skewer {
        target: EntityId,
        damage_info: DamageInfo,
        free_to_play_once: bool,
        energy_on_use: i32,
    },
    Doppelganger {
        upgraded: bool,
        free_to_play_once: bool,
        energy_on_use: i32,
    },
    Malaise {
        target: EntityId,
        upgraded: bool,
        free_to_play_once: bool,
        energy_on_use: i32,
    },
    GainBlock {
        target: EntityId,
        amount: i32,
    },
    DoubleBlock {
        target: EntityId,
    },
    GainBlockRandomMonster {
        source: EntityId,
        amount: i32,
    },
    LoseBlock {
        target: EntityId,
        amount: i32,
    },
    /// Rust migration shim for Java `LoseHPAction` provenance.
    ///
    /// In Java, whether HP loss triggers `RupturePower.wasHPLost(...)` is determined
    /// by the concrete action path and `DamageInfo.owner`, not by a standalone flag.
    /// Our unified action model loses that provenance, so `triggers_rupture` must be
    /// set only for player-authored self HP loss sources that Java routes through a
    /// `LoseHPAction(player, player, ...)`-equivalent path.
    LoseHp {
        target: EntityId,
        amount: i32,
        triggers_rupture: bool,
    },
    SetCurrentHp {
        target: EntityId,
        hp: i32,
    },
    Heal {
        target: EntityId,
        amount: i32,
    },
    GainGold {
        amount: i32,
    },
    AddCombatReward {
        item: crate::rewards::state::RewardItem,
    },
    GainEnergy {
        amount: i32,
    },
    /// Java `GainEnergyIfDiscardAction`: checked at action execution time.
    GainEnergyIfDiscardedThisTurn {
        amount: i32,
    },
    /// Java `ExpertiseAction`: draw until the current hand size reaches this
    /// amount, evaluated when the queued action resolves.
    ExpertiseDraw {
        target_hand_size: i32,
    },
    GainMaxHp {
        amount: i32,
    },
    LoseMaxHp {
        target: EntityId,
        amount: i32,
    },
    /// Java `DamageRandomEnemyAction`: select a random living monster at
    /// execution time, then queue/resolve a normal `DamageAction`.
    ///
    /// This is a DAMAGE action in Java and is therefore retained by
    /// `GameActionManager.clearPostCombatActions`.
    DamageRandomEnemy {
        source: EntityId,
        base_damage: i32,
        damage_type: DamageType,
    },
    /// Java `AttackDamageRandomEnemyAction`: select a random living monster at
    /// execution time, recalculate the referenced card against that target,
    /// then queue/resolve a normal `DamageAction`.
    ///
    /// This action does not set `actionType = DAMAGE` in Java; only the
    /// generated `DamageAction` is retained after post-combat cleanup.
    AttackDamageRandomEnemyCard {
        card: Box<crate::runtime::combat::CombatCard>,
    },
    BouncingFlask {
        target: Option<EntityId>,
        amount: i32,
        num_times: u8,
    },
    DropkickDamageAndEffect {
        target: EntityId,
        damage_info: DamageInfo,
    },
    SpotWeakness {
        target: EntityId,
        amount: i32,
    },
    /// Java Defect `ForTheEyesAction`: apply Weak to the target only if its
    /// current `getIntentBaseDmg()` is non-negative when the action resolves.
    ApplyWeakIfTargetAttacking {
        target: EntityId,
        amount: i32,
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
    HandOfGreed {
        target: EntityId,
        damage_info: DamageInfo,
        gold_amount: i32,
    },
    RitualDagger {
        target: EntityId,
        damage_info: DamageInfo,
        misc_amount: i32,
        card_uuid: u32,
    },
    VampireDamage(DamageInfo),
    VampireDamageAllEnemies {
        source: EntityId,
        damages: smallvec::SmallVec<[i32; 5]>,
        damage_type: DamageType,
    },
    /// Java `BarrageAction`: at action execution time, queue one damage action
    /// for each current non-empty orb slot.
    Barrage {
        damage: DamageInfo,
    },
    LimitBreak,
    DrawCards(u32),
    /// Java `CompileDriverAction`: at action execution time, draw
    /// `amount_per_orb_type` for each unique non-empty orb type.
    DrawForUniqueOrbTypes {
        amount_per_orb_type: u32,
    },
    /// Java `DrawCardAction` path that exposes its static `drawnCards`
    /// history to a queued follow-up action. Generic draw actions intentionally
    /// do not mutate this history; actions that need it must opt in.
    DrawCardsWithHistory {
        amount: u32,
        clear_history: bool,
    },
    /// Java `EscapePlanAction`: after the preceding DrawCardAction finishes,
    /// gain block if that DrawCardAction drew at least one Skill.
    EscapePlanBlockIfSkill {
        block: i32,
    },
    /// Java `CalculatedGambleAction`: at execution time, discard the current
    /// hand, then draw the same count, optionally plus one.
    CalculatedGamble {
        draw_extra: bool,
    },
    /// Java `BladeFuryAction` from Storm of Steel.
    BladeFury {
        upgraded: bool,
    },
    /// Java `ApplyBulletTimeAction`: set every current hand card free for the turn.
    ApplyBulletTime,
    /// Java `UnloadAction`: queues DiscardSpecificCardAction for every
    /// non-Attack in hand at execution time.
    UnloadNonAttack,
    EmptyDeckShuffle,
    ShuffleDiscardIntoDraw,
    PlayCard {
        card_index: usize,
        target: Option<EntityId>,
    },
    PlayCardDirect {
        card: Box<crate::runtime::combat::CombatCard>,
        target: Option<EntityId>,
        purge: bool,
    },
    EnqueueCardPlay {
        item: Box<crate::runtime::combat::QueuedCardPlay>,
        in_front: bool,
    },
    FlushNextQueuedCard,
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
    ExhaustFromHand {
        amount: usize,
        random: bool,
        any_number: bool,
        can_pick_zero: bool,
    },
    DiscardCard {
        card_uuid: u32,
    },
    DiscardFromHand {
        amount: i32,
        random: bool,
        end_turn: bool,
    },
    MoveCard {
        card_uuid: u32,
        from: PileType,
        to: PileType,
    },
    PutOnDeck {
        amount: usize,
        random: bool,
    },
    Forethought {
        upgraded: bool,
    },
    DiscardPileToTopOfDeck,
    ExhumeCard {
        card_uuid: u32,
        upgrade: bool,
    },
    RemoveCardFromPile {
        card_uuid: u32,
        from: PileType,
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
        colorless: bool,
        card_type: Option<crate::content::cards::CardType>,
        cost_for_turn: Option<u8>,
        can_skip: bool,
    },
    /// StancePotion: Java ChooseOneAction(ChooseWrath, ChooseCalm)
    SuspendForStanceChoice,
    ApplyPower {
        source: EntityId,
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        amount: i32,
    },
    ApplyPowerDetailed {
        source: EntityId,
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        amount: i32,
        instance_id: Option<u32>,
        extra_data: Option<i32>,
    },
    ApplyPowerWithPayload {
        source: EntityId,
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        amount: i32,
        instance_id: Option<u32>,
        extra_data: Option<i32>,
        payload: crate::runtime::combat::PowerPayload,
    },
    ReducePower {
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        amount: i32,
    },
    ReducePowerInstance {
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        instance_id: u32,
        amount: i32,
    },
    RemovePower {
        target: EntityId,
        power_id: crate::content::powers::PowerId,
    },
    RemovePowerInstance {
        target: EntityId,
        power_id: crate::content::powers::PowerId,
        instance_id: u32,
    },
    RemoveAllDebuffs {
        target: EntityId,
    },
    AwakenedRebirthClear {
        target: EntityId,
    },
    UpdatePowerExtraData {
        target: EntityId,
        power_id: crate::runtime::combat::PowerId,
        value: i32,
    },
    UpdatePowerExtraDataInstance {
        target: EntityId,
        power_id: crate::runtime::combat::PowerId,
        instance_id: u32,
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
        original: Box<crate::runtime::combat::CombatCard>,
        amount: u8,
    },
    MakeCopyInDrawPile {
        original: Box<crate::runtime::combat::CombatCard>,
        amount: u8,
        random_spot: bool,
        to_bottom: bool,
    },
    MakeCopyInDiscard {
        original: Box<crate::runtime::combat::CombatCard>,
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
    /// Java `NightmareAction`: at execution time choose/capture a hand card
    /// and apply one independent Nightmare power carrying a stat-equivalent
    /// card snapshot.
    Nightmare {
        amount: u8,
    },
    MakeRandomCardInDrawPile {
        card_type: Option<crate::content::cards::CardType>,
        cost_for_turn: Option<u8>,
        random_spot: bool,
    },
    DrawPileToHandByType {
        amount: u8,
        card_type: crate::content::cards::CardType,
    },
    MakeRandomColorlessCardInHand {
        cost_for_turn: Option<u8>,
        upgraded: bool,
    },
    Transmutation {
        upgraded: bool,
        free_to_play_once: bool,
        energy_on_use: i32,
    },
    ReduceAllHandCosts {
        amount: u8,
    },
    Enlightenment {
        permanent: bool,
    },
    Madness,
    RandomizeHandCosts,
    UpgradeAllInHand,
    UpgradeAllCardsInCombat,
    /// Hexaghost's BurnIncreaseAction: upgrades all Burn cards in draw pile and discard pile.
    UpgradeAllBurns,
    MakeTempCardInDrawPile {
        card_id: crate::content::cards::CardId,
        amount: u8,
        random_spot: bool,
        to_bottom: bool,
        upgraded: bool,
    },
    /// Java PlayTopCardAction with random target selection via cardRandomRng.
    /// Used by DistilledChaosPotion and similar.
    PlayTopCard {
        target: Option<EntityId>,
        exhaust: bool,
    },
    QueuePlayTopCardToBottom {
        target: Option<EntityId>,
        exhaust: bool,
    },
    ModifyCardMisc {
        card_uuid: u32,
        amount: i32,
    },
    ModifyCardDamage {
        card_uuid: u32,
        amount: i32,
    },
    /// Java Defect `GashAction`: increase the used Claw and all Claw cards in
    /// hand/draw/discard by the current magic amount.
    Gash {
        card_uuid: u32,
        amount: i32,
    },
    ModifyCardBlock {
        card_uuid: u32,
        amount: i32,
    },
    ReduceCardCostForCombat {
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
        planned_steps: crate::semantics::combat::MonsterTurnSteps,
        planned_visible_spec: Option<crate::semantics::combat::MonsterMoveSpec>,
    },
    UpdateMonsterRuntime {
        monster_id: EntityId,
        patch: MonsterRuntimePatch,
    },
    ReviveMonster {
        target: EntityId,
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
        protocol_draw_x: Option<i32>,
        is_minion: bool,
    },
    SpawnMonsterSmart {
        monster_id: crate::content::monsters::EnemyId,
        logical_position: i32,
        hp: crate::semantics::combat::SpawnHpSpec,
        protocol_draw_x: Option<i32>,
        is_minion: bool,
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
    /// Java `UseCardAction.update()` after-use power hook path for cards that
    /// do not enter the normal non-Power discard/exhaust cleanup.
    UseCardAfterUseHooks {
        card: Box<crate::runtime::combat::CombatCard>,
    },
    QueueEarlyEndTurn,
    TriggerTimeWarpEndTurn {
        owner: EntityId,
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
    ChannelOrb(crate::runtime::combat::OrbId),
    /// Java `ChannelAction(existingOrb, false)`: channel the concrete orb
    /// instance into an empty slot without auto-evoking.
    ChannelOrbEntity {
        orb: crate::runtime::combat::OrbEntity,
    },
    EvokeOrb,
    EvokeOrbWithoutRemoving,
    /// Java Defect `RedoAction`: snapshot the front orb instance, evoke it,
    /// then channel the same orb instance back into the empty slot.
    RedoOrb,
    TriggerStartOfTurnOrbs,
    TriggerEndOfTurnOrbs,
    TriggerImpulseOrbs,
    Scry(usize),
    EnterStance(String), // Watcher stance: "Wrath", "Calm", "Divinity", "Neutral"
    ObtainPotion,
    ObtainSpecificPotion(crate::content::potions::PotionId),
    /// Unified action for NilrysCodex (Codex) / Toolbox (ChooseOneColorless) / similar relic reward screens.
    /// Generates 3 unique random cards from `pool`, player picks 1.
    /// Card goes to `destination`. If `can_skip`, player can cancel.
    SuspendForCardReward {
        pool: CardRewardPool,
        destination: CardDestination,
        can_skip: bool,
        skip_if_monsters_basically_dead: bool,
    },
}

impl Action {
    pub fn retained_by_java_clear_post_combat_actions(&self) -> bool {
        matches!(
            self,
            Action::Damage(_)
                | Action::PummelDamage(_)
                | Action::MonsterAttack { .. }
                | Action::DamagePerAttackPlayed(_)
                | Action::DamageAllEnemies { .. }
                | Action::DamageRandomEnemy { .. }
                | Action::Feed { .. }
                | Action::HandOfGreed { .. }
                | Action::RitualDagger { .. }
                | Action::VampireDamage(_)
                | Action::VampireDamageAllEnemies { .. }
                | Action::LoseHp { .. }
                | Action::GainBlock { .. }
                | Action::Heal { .. }
                | Action::UseCardDone { .. }
                | Action::UseCardAfterUseHooks { .. }
        )
    }
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

pub fn repeated_damage_matrix(enemy_count: usize, amount: i32) -> smallvec::SmallVec<[i32; 5]> {
    std::iter::repeat_n(amount, enemy_count).collect()
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
