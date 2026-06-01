use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct StableRunPendingChoiceKey {
    pub min_choices: usize,
    pub max_choices: usize,
    pub reason: String,
    pub return_state: StableRunPendingReturnKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum StableRunPendingReturnKey {
    Reward(StableRewardKey),
    TreasureRoom(StableTreasureChestKey),
    Campfire,
    Shop(StableShopKey),
    MapNavigation,
    MapOverlay(Box<StableRunPendingReturnKey>),
    EventRoom,
    BossRelic(StableBossRelicKey),
    RunPendingChoice(Box<StableRunPendingChoiceKey>),
    GameOver(&'static str),
    Combat,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct StableTreasureChestKey {
    pub size: String,
    pub base_relic_tier: String,
    pub gold_reward: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct StableBossRelicKey {
    pub relics: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StablePendingChoiceKey {
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
    ForeignInfluence {
        upgraded: bool,
        cards: Vec<String>,
    },
    ChooseOne(Vec<String>),
    StanceChoice,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum StableChoiceCandidateKey {
    Card(String),
    Ref { prefix: &'static str, uuid: u32 },
}

impl StableRunPendingChoiceKey {
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
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
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
        match self {
            StableRunPendingReturnKey::Reward(value) => {
                format!("reward:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::TreasureRoom(value) => {
                format!("treasure:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::Campfire => "campfire".to_string(),
            StableRunPendingReturnKey::Shop(value) => {
                format!("shop:{}", value.diagnostic_string())
            }
            StableRunPendingReturnKey::MapNavigation => "map_navigation".to_string(),
            StableRunPendingReturnKey::MapOverlay(value) => {
                format!("map_overlay:return{}", value.diagnostic_string())
            }
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

impl StableBossRelicKey {
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
        self.relics.join("|")
    }
}

impl StablePendingChoiceKey {
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
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
            StablePendingChoiceKey::ForeignInfluence { upgraded, cards } => {
                format!("foreign_influence:{upgraded}:{}", cards.join("|"))
            }
            StablePendingChoiceKey::ChooseOne(cards) => format!("choose_one:{}", cards.join("|")),
            StablePendingChoiceKey::StanceChoice => "stance_choice".to_string(),
        }
    }
}

impl StableTreasureChestKey {
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
        format!(
            "size{}:tier{}:gold{}",
            self.size,
            self.base_relic_tier,
            self.gold_reward
                .map(|amount| amount.to_string())
                .unwrap_or_else(|| "_".to_string())
        )
    }
}

impl StableChoiceCandidateKey {
    pub(in crate::ai::combat_state_key) fn diagnostic_string(&self) -> String {
        match self {
            StableChoiceCandidateKey::Card(value) => value.clone(),
            StableChoiceCandidateKey::Ref { prefix, uuid } => format!("{prefix}:{uuid}"),
        }
    }
}
