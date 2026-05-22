use std::mem::Discriminant;

use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::powers::PowerId;
use crate::content::relics::RelicId;
use crate::runtime::action::Action;
use crate::runtime::combat::{
    AwakenedOneRuntimeState, BookOfStabbingRuntimeState, BronzeAutomatonRuntimeState,
    BronzeOrbRuntimeState, ByrdRuntimeState, ChampRuntimeState, ChosenRuntimeState,
    CorruptHeartRuntimeState, CultistRuntimeState, DarklingRuntimeState, DecaRuntimeState,
    DonuRuntimeState, ExploderRuntimeState, GiantHeadRuntimeState, GremlinLeaderRuntimeState,
    GremlinNobRuntimeState, GremlinWizardRuntimeState, GuardianRuntimeState, HexaghostRuntimeState,
    JawWormRuntimeState, LagavulinRuntimeState, LargeSlimeRuntimeState, LouseRuntimeState,
    MawRuntimeState, MonsterMoveState, NemesisRuntimeState, OrbId, QueuedCardSource,
    ReptomancerRuntimeState, SentryRuntimeState, ShelledParasiteRuntimeState,
    SlaverRedRuntimeState, SlimeBossRuntimeState, SnakeDaggerRuntimeState, SneckoRuntimeState,
    SphericGuardianRuntimeState, SpikerRuntimeState, SpireShieldRuntimeState,
    SpireSpearRuntimeState, StanceId, ThiefRuntimeState, TimeEaterRuntimeState,
    TransientRuntimeState, WrithingMassRuntimeState,
};
use crate::runtime::monster_move::MonsterTurnPlan;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatExactStateKey {
    pub(crate) common: CombatRuntimeKey,
    pub(crate) player: CombatExactPlayerKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDominanceKey {
    pub(crate) common: CombatRuntimeKey,
    pub(crate) player: CombatDominancePlayerKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRuntimeKey {
    pub(crate) engine: CombatEngineKey,
    pub(crate) turn: CombatTurnKey,
    pub(crate) meta: CombatMetaKey,
    pub(crate) zones: CombatZonesKey,
    pub(crate) monsters: Vec<CombatMonsterKey>,
    pub(crate) powers: Vec<CombatEntityPowersKey>,
    pub(crate) potions: Vec<CombatPotionSlotKey>,
    pub(crate) queue: Vec<CombatQueuedActionKey>,
    pub(crate) runtime: CombatRuntimeHintsKey,
    pub(crate) rng: CombatRngPoolKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatExactPlayerKey {
    pub(crate) current_hp: i32,
    pub(crate) block: i32,
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDominancePlayerKey {
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatEngineKey {
    CombatPlayerTurn,
    CombatProcessing,
    PendingChoice(CombatPendingChoiceKey),
    RewardScreen(String),
    TreasureRoom(String),
    Campfire,
    Shop(String),
    MapNavigation,
    EventRoom,
    RunPendingChoice(String),
    EventCombat(String),
    BossRelicSelect(String),
    GameOver(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatPendingChoiceKey {
    GridSelect {
        source_pile: CombatPileTypeKey,
        candidate_uuids: Vec<u32>,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: CombatGridSelectReasonKey,
    },
    HandSelect {
        candidate_uuids: Vec<u32>,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: CombatHandSelectReasonKey,
    },
    DiscoverySelect {
        cards: Vec<CardId>,
        colorless: bool,
        card_type: Option<CombatCardTypeKey>,
        amount: u8,
        can_skip: bool,
    },
    ScrySelect {
        cards: Vec<CardId>,
        card_uuids: Vec<u32>,
    },
    CardRewardSelect {
        cards: Vec<CardId>,
        destination: CombatCardDestinationKey,
        can_skip: bool,
    },
    ForeignInfluenceSelect {
        cards: Vec<CardId>,
        upgraded: bool,
    },
    ChooseOneSelect {
        choices: Vec<CombatChooseOneCardKey>,
    },
    StanceChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatChooseOneCardKey {
    pub(crate) card_id: CardId,
    pub(crate) upgrades: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatPileTypeKey {
    Draw,
    Discard,
    Exhaust,
    Hand,
    Limbo,
    MasterDeck,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatHandSelectReasonKey {
    Exhaust,
    Discard,
    Retain,
    PutOnDrawPile,
    PutToBottomOfDraw,
    Setup,
    Copy { amount: u8 },
    Nightmare { amount: u8 },
    Upgrade,
    GamblingChip,
    Recycle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatGridSelectReasonKey {
    MoveToDrawPile,
    Exhume { upgrade: bool },
    DrawPileToHand,
    SkillFromDeckToHand,
    AttackFromDeckToHand,
    DiscardToHand,
    DiscardToHandNoCostChange,
    DiscardToHandRetain,
    Omniscience { play_amount: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatCardTypeKey {
    Attack,
    Skill,
    Power,
    Status,
    Curse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatCardDestinationKey {
    Hand,
    DrawPileRandom,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatTurnKey {
    pub(crate) turn_count: u32,
    pub(crate) phase: CombatPhaseKey,
    pub(crate) energy: u8,
    pub(crate) turn_start_draw_modifier: i32,
    pub(crate) counters: CombatTurnCountersKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatPhaseKey {
    PlayerTurn,
    MonsterTurn,
    TurnTransition,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatTurnCountersKey {
    pub(crate) cards_played_this_turn: u8,
    pub(crate) attacks_played_this_turn: u8,
    pub(crate) cards_discarded_this_turn: u16,
    pub(crate) card_ids_played_this_turn: Vec<CardId>,
    pub(crate) card_ids_played_this_combat: Vec<CardId>,
    pub(crate) orbs_channeled_this_turn: Vec<OrbId>,
    pub(crate) orbs_channeled_this_combat: Vec<OrbId>,
    pub(crate) mantra_gained_this_combat: i32,
    pub(crate) times_damaged_this_combat: u8,
    pub(crate) victory_triggered: bool,
    pub(crate) discovery_cost_for_turn: Option<u8>,
    pub(crate) early_end_turn_pending: bool,
    pub(crate) skip_monster_turn_pending: bool,
    pub(crate) player_escaping: bool,
    pub(crate) escape_pending_reward: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMetaKey {
    pub(crate) ascension_level: u8,
    pub(crate) player_class: &'static str,
    pub(crate) is_boss_fight: bool,
    pub(crate) is_elite_fight: bool,
    pub(crate) master_deck_snapshot: Vec<CombatCardKey>,
    pub(crate) meta_changes: Vec<CombatMetaChangeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatMetaChangeKey {
    AddCardToMasterDeck(CardId),
    ModifyCardMisc { card_uuid: u32, amount: i32 },
    UpgradeMasterDeckCard { card_uuid: u32 },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatPlayerFutureKey {
    pub(crate) entity_id: usize,
    pub(crate) max_hp: i32,
    pub(crate) facing_left: bool,
    pub(crate) gold_delta_this_combat: i32,
    pub(crate) gold: i32,
    pub(crate) max_orbs: u8,
    pub(crate) orbs: Vec<CombatOrbKey>,
    pub(crate) stance: StanceId,
    pub(crate) relics: Vec<CombatRelicKey>,
    pub(crate) relic_buses: String,
    pub(crate) energy_master: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatOrbKey {
    pub(crate) id: OrbId,
    pub(crate) base_passive_amount: i32,
    pub(crate) base_evoke_amount: i32,
    pub(crate) passive_amount: i32,
    pub(crate) evoke_amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRelicKey {
    pub(crate) id: RelicId,
    pub(crate) counter: i32,
    pub(crate) used_up: bool,
    pub(crate) amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatZonesKey {
    pub(crate) card_uuid_counter: u32,
    pub(crate) hand: Vec<CombatCardKey>,
    pub(crate) draw: Vec<CombatCardKey>,
    pub(crate) discard: Vec<CombatCardKey>,
    pub(crate) exhaust: Vec<CombatCardKey>,
    pub(crate) limbo: Vec<CombatCardKey>,
    pub(crate) queued: Vec<CombatQueuedCardKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatCardKey {
    pub(crate) id: CardId,
    pub(crate) uuid: u32,
    pub(crate) upgrades: u8,
    pub(crate) misc_value: i32,
    pub(crate) base_damage_override: Option<i32>,
    pub(crate) base_block_override: Option<i32>,
    pub(crate) cost_modifier: i8,
    pub(crate) cost_for_turn: Option<u8>,
    pub(crate) base_damage_mut: i32,
    pub(crate) base_block_mut: i32,
    pub(crate) base_magic_num_mut: i32,
    pub(crate) multi_damage: Vec<i32>,
    pub(crate) exhaust_override: Option<bool>,
    pub(crate) retain_override: Option<bool>,
    pub(crate) free_to_play_once: bool,
    pub(crate) energy_on_use: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedCardKey {
    pub(crate) card: CombatCardKey,
    pub(crate) target: CombatTargetKey,
    pub(crate) energy_on_use: i32,
    pub(crate) ignore_energy_total: bool,
    pub(crate) autoplay: bool,
    pub(crate) random_target: bool,
    pub(crate) is_end_turn_autoplay: bool,
    pub(crate) purge_on_use: bool,
    pub(crate) source: QueuedCardSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatTargetKey {
    None,
    MonsterSlot(usize),
    Entity(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterKey {
    pub(crate) entity_id: usize,
    pub(crate) monster_type: usize,
    pub(crate) current_hp: i32,
    pub(crate) max_hp: i32,
    pub(crate) block: i32,
    pub(crate) slot: u8,
    pub(crate) logical_position: i32,
    pub(crate) is_dying: bool,
    pub(crate) is_escaped: bool,
    pub(crate) half_dead: bool,
    pub(crate) move_state: MonsterMoveState,
    pub(crate) turn_plan: MonsterTurnPlan,
    pub(crate) runtime: CombatMonsterRuntimeKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterRuntimeKey {
    pub(crate) hexaghost: HexaghostRuntimeState,
    pub(crate) louse: LouseRuntimeState,
    pub(crate) jaw_worm: JawWormRuntimeState,
    pub(crate) thief: ThiefRuntimeState,
    pub(crate) byrd: ByrdRuntimeState,
    pub(crate) chosen: ChosenRuntimeState,
    pub(crate) snecko: SneckoRuntimeState,
    pub(crate) shelled_parasite: ShelledParasiteRuntimeState,
    pub(crate) bronze_automaton: BronzeAutomatonRuntimeState,
    pub(crate) bronze_orb: BronzeOrbRuntimeState,
    pub(crate) book_of_stabbing: BookOfStabbingRuntimeState,
    pub(crate) collector: crate::runtime::combat::CollectorRuntimeState,
    pub(crate) champ: ChampRuntimeState,
    pub(crate) awakened_one: AwakenedOneRuntimeState,
    pub(crate) corrupt_heart: CorruptHeartRuntimeState,
    pub(crate) writhing_mass: WrithingMassRuntimeState,
    pub(crate) spiker: SpikerRuntimeState,
    pub(crate) spire_shield: SpireShieldRuntimeState,
    pub(crate) spire_spear: SpireSpearRuntimeState,
    pub(crate) slaver_red: SlaverRedRuntimeState,
    pub(crate) gremlin_leader: GremlinLeaderRuntimeState,
    pub(crate) gremlin_nob: GremlinNobRuntimeState,
    pub(crate) gremlin_wizard: GremlinWizardRuntimeState,
    pub(crate) cultist: CultistRuntimeState,
    pub(crate) sentry: SentryRuntimeState,
    pub(crate) slime_boss: SlimeBossRuntimeState,
    pub(crate) large_slime: LargeSlimeRuntimeState,
    pub(crate) spheric_guardian: SphericGuardianRuntimeState,
    pub(crate) reptomancer: ReptomancerRuntimeState,
    pub(crate) darkling: DarklingRuntimeState,
    pub(crate) nemesis: NemesisRuntimeState,
    pub(crate) giant_head: GiantHeadRuntimeState,
    pub(crate) time_eater: TimeEaterRuntimeState,
    pub(crate) donu: DonuRuntimeState,
    pub(crate) deca: DecaRuntimeState,
    pub(crate) transient: TransientRuntimeState,
    pub(crate) exploder: ExploderRuntimeState,
    pub(crate) maw: MawRuntimeState,
    pub(crate) snake_dagger: SnakeDaggerRuntimeState,
    pub(crate) lagavulin: LagavulinRuntimeState,
    pub(crate) guardian: GuardianRuntimeState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatEntityPowersKey {
    pub(crate) entity_id: usize,
    pub(crate) powers: Vec<CombatPowerKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatPowerKey {
    pub(crate) power_type: PowerId,
    pub(crate) instance_id: Option<u32>,
    pub(crate) amount: i32,
    pub(crate) extra_data: i32,
    pub(crate) payload: CombatPowerPayloadKey,
    pub(crate) just_applied: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatPowerPayloadKey {
    None,
    Card(CombatCardKey),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatPotionSlotKey {
    pub(crate) slot: usize,
    pub(crate) potion: Option<CombatPotionKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatPotionKey {
    pub(crate) id: PotionId,
    pub(crate) uuid: u32,
    pub(crate) can_use: bool,
    pub(crate) can_discard: bool,
    pub(crate) requires_target: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedActionKey {
    pub(crate) discriminant: Discriminant<Action>,
    pub(crate) payload: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRuntimeHintsKey {
    pub(crate) using_card: bool,
    pub(crate) card_queue: Vec<CombatQueuedCardHintKey>,
    pub(crate) colorless_combat_pool: Vec<CardId>,
    pub(crate) emitted_events: Vec<String>,
    pub(crate) engine_diagnostics: Vec<String>,
    pub(crate) pending_rewards: Vec<String>,
    pub(crate) power_instance_counter: u32,
    pub(crate) last_drawn_cards: Vec<CombatDrawnCardKey>,
    pub(crate) monster_protocol: Vec<CombatMonsterProtocolKey>,
    pub(crate) combat_mugged: bool,
    pub(crate) combat_smoked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedCardHintKey {
    pub(crate) card_uuid: u32,
    pub(crate) card_id: CardId,
    pub(crate) target_monster_index: Option<usize>,
    pub(crate) energy_on_use: i32,
    pub(crate) ignore_energy_total: bool,
    pub(crate) autoplay: bool,
    pub(crate) random_target: bool,
    pub(crate) is_end_turn_autoplay: bool,
    pub(crate) purge_on_use: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDrawnCardKey {
    pub(crate) card_uuid: u32,
    pub(crate) card_id: CardId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterProtocolKey {
    pub(crate) entity_id: usize,
    pub(crate) payload: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRngPoolKey {
    pub(crate) monster_rng: CombatStsRngKey,
    pub(crate) event_rng: CombatStsRngKey,
    pub(crate) merchant_rng: CombatStsRngKey,
    pub(crate) card_rng: CombatStsRngKey,
    pub(crate) treasure_rng: CombatStsRngKey,
    pub(crate) relic_rng: CombatStsRngKey,
    pub(crate) potion_rng: CombatStsRngKey,
    pub(crate) monster_hp_rng: CombatStsRngKey,
    pub(crate) ai_rng: CombatStsRngKey,
    pub(crate) shuffle_rng: CombatStsRngKey,
    pub(crate) card_random_rng: CombatStsRngKey,
    pub(crate) misc_rng: CombatStsRngKey,
    pub(crate) math_rng: CombatStsRngKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatStsRngKey {
    pub(crate) seed0: u64,
    pub(crate) seed1: u64,
    pub(crate) counter: u32,
}
