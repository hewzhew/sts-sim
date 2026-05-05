use super::StableFrontierScope;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct TurnStateKey(pub(in crate::bot::combat) String);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StableOutcomeKey {
    scope: StableFrontierScope,
    engine: StableEngineKey,
    payload: StableOutcomePayload,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) enum StableEngineKey {
    CombatReady,
    CombatProcessing,
    PendingChoice(StablePendingChoiceKey),
    Reward(StableRewardKey),
    Campfire,
    Shop(StableShopKey),
    MapNavigation,
    EventRoom,
    RunPendingChoice(StableRunPendingChoiceKey),
    EventCombat(StableEventCombatKey),
    BossRelic(StableBossRelicKey),
    GameOver(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) enum StableOutcomePayload {
    Combat {
        turn: StableTurnKey,
        player: StableCombatPlayerKey,
        zones: StableZonesKey,
        monsters: Vec<String>,
        powers: Vec<String>,
        rng: String,
    },
    PostCombat {
        player: StablePostcombatPlayerKey,
        meta: StableMetaKey,
        runtime: StablePostcombatRuntimeKey,
        rng: String,
    },
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StableTurnKey {
    pub turn_count: u32,
    pub current_phase: String,
    pub energy: u8,
    pub turn_start_draw_modifier: i32,
    pub cards_played_this_turn: u8,
    pub attacks_played_this_turn: u8,
    pub times_damaged_this_combat: u8,
    pub victory_triggered: bool,
    pub discovery_cost_for_turn: Option<u8>,
    pub early_end_turn_pending: bool,
    pub player_escaping: bool,
    pub escape_pending_reward: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StableCombatPlayerKey {
    pub max_hp: i32,
    pub orbs: String,
    pub max_orbs: u8,
    pub stance: String,
    pub relics: String,
    pub relic_buses: String,
    pub energy_master: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StableZonesKey {
    pub draw: Vec<String>,
    pub hand: Vec<String>,
    pub discard: Vec<String>,
    pub exhaust: Vec<String>,
    pub limbo: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StablePostcombatPlayerKey {
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub relics: String,
    pub energy_master: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StableMetaKey {
    pub player_class: String,
    pub ascension_level: u8,
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
    pub meta_changes: Vec<StableMetaChangeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) enum StableMetaChangeKey {
    AddCardToMasterDeck(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) struct StablePostcombatRuntimeKey {
    pub pending_rewards: Vec<StableRewardItemKey>,
    pub combat_mugged: bool,
    pub combat_smoked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableRewardKey {
    pub screen_context: String,
    pub skippable: bool,
    pub items: Vec<StableRewardItemKey>,
    pub pending_card_choice: Vec<StableRewardCardKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) enum StableRewardItemKey {
    Gold(i32),
    StolenGold(i32),
    Card(Vec<StableRewardCardKey>),
    Relic(String),
    Potion(String),
    EmeraldKey,
    SapphireKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableRewardCardKey {
    pub id: String,
    pub upgrades: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableShopKey {
    pub purge_cost: i32,
    pub purge_available: bool,
    pub cards: Vec<StableShopRowKey>,
    pub relics: Vec<StableShopRowKey>,
    pub potions: Vec<StableShopRowKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableShopRowKey {
    pub id: String,
    pub price: i32,
    pub can_buy: bool,
    pub blocked_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableRunPendingChoiceKey {
    pub min_choices: usize,
    pub max_choices: usize,
    pub reason: String,
    pub return_state: StableRunPendingReturnKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) enum StableRunPendingReturnKey {
    Reward(StableRewardKey),
    Campfire,
    Shop(StableShopKey),
    MapNavigation,
    EventRoom,
    BossRelic(StableBossRelicKey),
    RunPendingChoice(Box<StableRunPendingChoiceKey>),
    GameOver(&'static str),
    Combat,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableEventCombatKey {
    pub encounter_key: String,
    pub reward_allowed: bool,
    pub no_cards_in_rewards: bool,
    pub post_combat_return: StablePostCombatReturnKey,
    pub rewards: StableRewardKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) enum StablePostCombatReturnKey {
    EventRoom,
    MapNavigation,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) struct StableBossRelicKey {
    pub relics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(in crate::bot::combat) enum StablePendingChoiceKey {
    GridSelect {
        source_pile: &'static str,
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: String,
        candidates: Vec<StableChoiceCandidateKey>,
    },
    HandSelect {
        min_cards: u8,
        max_cards: u8,
        can_cancel: bool,
        reason: String,
        candidates: Vec<StableChoiceCandidateKey>,
    },
    Discovery(Vec<String>),
    Scry(Vec<StableChoiceCandidateKey>),
    CardRewardSelect {
        destination: String,
        can_skip: bool,
        cards: Vec<String>,
    },
    StanceChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(in crate::bot::combat) enum StableChoiceCandidateKey {
    Card(String),
    Ref { prefix: &'static str, uuid: u32 },
}

impl StableOutcomeKey {
    pub(in crate::bot::combat::turn_state_key) fn new(
        scope: StableFrontierScope,
        engine: StableEngineKey,
        payload: StableOutcomePayload,
    ) -> Self {
        Self {
            scope,
            engine,
            payload,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(in crate::bot::combat::turn_state_key) fn diagnostic_string(&self) -> String {
        match &self.payload {
            StableOutcomePayload::Combat {
                turn,
                player,
                zones,
                monsters,
                powers,
                rng,
            } => format!(
                "scope={:?};engine={};turn={};player={};zones={};monsters=[{}];powers=[{}];rng={}",
                self.scope,
                self.engine.diagnostic_string(),
                turn.diagnostic_string(),
                player.diagnostic_string(),
                zones.diagnostic_string(),
                monsters.join("|"),
                powers.join("|"),
                rng,
            ),
            StableOutcomePayload::PostCombat {
                player,
                meta,
                runtime,
                rng,
            } => format!(
                "scope={:?};engine={};player={};meta={};runtime={};rng={}",
                self.scope,
                self.engine.diagnostic_string(),
                player.diagnostic_string(),
                meta.diagnostic_string(),
                runtime.diagnostic_string(),
                rng,
            ),
            StableOutcomePayload::GameOver => format!(
                "scope={:?};engine={}",
                self.scope,
                self.engine.diagnostic_string(),
            ),
        }
    }
}

impl StableEngineKey {
    #[cfg_attr(not(test), allow(dead_code))]
    fn diagnostic_string(&self) -> String {
        match self {
            StableEngineKey::CombatReady => "player_turn_ready".to_string(),
            StableEngineKey::CombatProcessing => "combat_processing".to_string(),
            StableEngineKey::PendingChoice(value) => {
                format!("pending_choice:{}", value.diagnostic_string())
            }
            StableEngineKey::Reward(value) => format!("reward:{}", value.diagnostic_string()),
            StableEngineKey::Campfire => "campfire".to_string(),
            StableEngineKey::Shop(value) => format!("shop:{}", value.diagnostic_string()),
            StableEngineKey::MapNavigation => "map_navigation".to_string(),
            StableEngineKey::EventRoom => "event_room".to_string(),
            StableEngineKey::RunPendingChoice(value) => {
                format!("run_choice:{}", value.diagnostic_string())
            }
            StableEngineKey::EventCombat(value) => {
                format!("event_combat:{}", value.diagnostic_string())
            }
            StableEngineKey::BossRelic(value) => {
                format!("boss_relic:{}", value.diagnostic_string())
            }
            StableEngineKey::GameOver(value) => format!("game_over:{value}"),
        }
    }
}

impl StableTurnKey {
    fn diagnostic_string(&self) -> String {
        format!(
            concat!(
                "count:{}:phase:{}:energy:{}:draw_mod:{}:",
                "cards:{}:attacks:{}:damaged:{}:victory:{}:discover:{:?}:",
                "early_end:{}:escaping:{}:escape_reward:{}"
            ),
            self.turn_count,
            self.current_phase,
            self.energy,
            self.turn_start_draw_modifier,
            self.cards_played_this_turn,
            self.attacks_played_this_turn,
            self.times_damaged_this_combat,
            self.victory_triggered,
            self.discovery_cost_for_turn,
            self.early_end_turn_pending,
            self.player_escaping,
            self.escape_pending_reward,
        )
    }
}

impl StableCombatPlayerKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "max_hp:{}:orbs:{}:max_orbs:{}:stance:{}:relics:{}:buses:{}:energy_master:{}",
            self.max_hp,
            self.orbs,
            self.max_orbs,
            self.stance,
            self.relics,
            self.relic_buses,
            self.energy_master,
        )
    }
}

impl StableZonesKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "draw:[{}];hand:[{}];disc:[{}];exhaust:[{}];limbo:[{}]",
            self.draw.join("|"),
            self.hand.join("|"),
            self.discard.join("|"),
            self.exhaust.join("|"),
            self.limbo.join("|"),
        )
    }
}

impl StablePostcombatPlayerKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "hp:{}:max_hp:{}:gold:{}:relics:{}:energy_master:{}",
            self.current_hp, self.max_hp, self.gold, self.relics, self.energy_master,
        )
    }
}

impl StableMetaKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "class:{}:asc:{}:boss:{}:elite:{}:changes:{}",
            self.player_class,
            self.ascension_level,
            self.is_boss_fight,
            self.is_elite_fight,
            join_diagnostic_strings(&self.meta_changes),
        )
    }
}

impl StableMetaChangeKey {
    fn diagnostic_string(&self) -> String {
        match self {
            StableMetaChangeKey::AddCardToMasterDeck(card) => format!("add_master:{card}"),
        }
    }
}

impl StablePostcombatRuntimeKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "pending_rewards:{}:mugged:{}:smoked:{}",
            join_diagnostic_strings(&self.pending_rewards),
            self.combat_mugged,
            self.combat_smoked,
        )
    }
}

impl StableRewardKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "ctx{}:skip{}:items{}:pending{}",
            self.screen_context,
            self.skippable,
            join_diagnostic_strings(&self.items),
            if self.pending_card_choice.is_empty() {
                "_".to_string()
            } else {
                join_diagnostic_strings(&self.pending_card_choice)
            },
        )
    }
}

impl StableRewardItemKey {
    fn diagnostic_string(&self) -> String {
        match self {
            StableRewardItemKey::Gold(amount) => format!("gold:{amount}"),
            StableRewardItemKey::StolenGold(amount) => format!("stolen_gold:{amount}"),
            StableRewardItemKey::Card(cards) => {
                format!("card:{}", join_diagnostic_strings(cards))
            }
            StableRewardItemKey::Relic(id) => format!("relic:{id}"),
            StableRewardItemKey::Potion(id) => format!("potion:{id}"),
            StableRewardItemKey::EmeraldKey => "emerald_key".to_string(),
            StableRewardItemKey::SapphireKey => "sapphire_key".to_string(),
        }
    }
}

impl StableRewardCardKey {
    fn diagnostic_string(&self) -> String {
        format!("{}:u{}", self.id, self.upgrades)
    }
}

impl StableShopKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "purge{}:{}:cards{}:relics{}:potions{}",
            self.purge_cost,
            self.purge_available,
            join_diagnostic_strings(&self.cards),
            join_diagnostic_strings(&self.relics),
            join_diagnostic_strings(&self.potions),
        )
    }
}

impl StableShopRowKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.id,
            self.price,
            self.can_buy,
            self.blocked_reason.as_deref().unwrap_or("_"),
        )
    }
}

impl StableRunPendingChoiceKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "min{}:max{}:reason{}:return{}",
            self.min_choices,
            self.max_choices,
            self.reason,
            self.return_state.diagnostic_string(),
        )
    }
}

impl StableRunPendingReturnKey {
    fn diagnostic_string(&self) -> String {
        match self {
            StableRunPendingReturnKey::Reward(value) => {
                format!("reward:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::Campfire => "campfire".to_string(),
            StableRunPendingReturnKey::Shop(value) => {
                format!("shop:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::MapNavigation => "map_navigation".to_string(),
            StableRunPendingReturnKey::EventRoom => "event_room".to_string(),
            StableRunPendingReturnKey::BossRelic(value) => {
                format!("boss_relic:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::RunPendingChoice(value) => {
                format!("run_pending_choice:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::GameOver(value) => format!("game_over:{value}"),
            StableRunPendingReturnKey::Combat => "combat".to_string(),
        }
    }
}

impl StableEventCombatKey {
    fn diagnostic_string(&self) -> String {
        format!(
            "encounter{}:reward_allowed{}:no_cards{}:return{}:rewards{}",
            self.encounter_key,
            self.reward_allowed,
            self.no_cards_in_rewards,
            self.post_combat_return.diagnostic_string(),
            self.rewards.diagnostic_string(),
        )
    }
}

impl StablePostCombatReturnKey {
    fn diagnostic_string(&self) -> &'static str {
        match self {
            StablePostCombatReturnKey::EventRoom => "event_room",
            StablePostCombatReturnKey::MapNavigation => "map_navigation",
        }
    }
}

impl StableBossRelicKey {
    fn diagnostic_string(&self) -> String {
        self.relics.join("|")
    }
}

impl StablePendingChoiceKey {
    fn diagnostic_string(&self) -> String {
        match self {
            StablePendingChoiceKey::GridSelect {
                source_pile,
                min_cards,
                max_cards,
                can_cancel,
                reason,
                candidates,
            } => format!(
                "grid:{source_pile}:{min_cards}:{max_cards}:{can_cancel}:{reason}:{}",
                join_diagnostic_strings(candidates)
            ),
            StablePendingChoiceKey::HandSelect {
                min_cards,
                max_cards,
                can_cancel,
                reason,
                candidates,
            } => format!(
                "hand:{min_cards}:{max_cards}:{can_cancel}:{reason}:{}",
                join_diagnostic_strings(candidates)
            ),
            StablePendingChoiceKey::Discovery(cards) => format!("discovery:{}", cards.join("|")),
            StablePendingChoiceKey::Scry(cards) => {
                format!("scry:{}", join_diagnostic_strings(cards))
            }
            StablePendingChoiceKey::CardRewardSelect {
                destination,
                can_skip,
                cards,
            } => format!("card_reward:{destination}:{can_skip}:{}", cards.join("|")),
            StablePendingChoiceKey::StanceChoice => "stance_choice".to_string(),
        }
    }
}

impl StableChoiceCandidateKey {
    fn diagnostic_string(&self) -> String {
        match self {
            StableChoiceCandidateKey::Card(value) => value.clone(),
            StableChoiceCandidateKey::Ref { prefix, uuid } => format!("{prefix}:{uuid}"),
        }
    }
}

trait DiagnosticKey {
    fn diagnostic_string(&self) -> String;
}

impl DiagnosticKey for StableMetaChangeKey {
    fn diagnostic_string(&self) -> String {
        StableMetaChangeKey::diagnostic_string(self)
    }
}

impl DiagnosticKey for StableRewardItemKey {
    fn diagnostic_string(&self) -> String {
        StableRewardItemKey::diagnostic_string(self)
    }
}

impl DiagnosticKey for StableRewardCardKey {
    fn diagnostic_string(&self) -> String {
        StableRewardCardKey::diagnostic_string(self)
    }
}

impl DiagnosticKey for StableShopRowKey {
    fn diagnostic_string(&self) -> String {
        StableShopRowKey::diagnostic_string(self)
    }
}

impl DiagnosticKey for StableChoiceCandidateKey {
    fn diagnostic_string(&self) -> String {
        StableChoiceCandidateKey::diagnostic_string(self)
    }
}

fn join_diagnostic_strings<T: DiagnosticKey>(values: &[T]) -> String {
    values
        .iter()
        .map(DiagnosticKey::diagnostic_string)
        .collect::<Vec<_>>()
        .join("|")
}
