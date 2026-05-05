use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Debug, Clone, PartialEq)]
pub struct RewardCard {
    pub id: crate::content::cards::CardId,
    pub upgrades: u8,
}

impl RewardCard {
    pub fn new(id: crate::content::cards::CardId, upgrades: u8) -> Self {
        Self { id, upgrades }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RewardItem {
    Gold { amount: i32 },
    StolenGold { amount: i32 },
    Card { cards: Vec<RewardCard> },
    Relic { relic_id: RelicId },
    Potion { potion_id: PotionId },
    EmeraldKey,
    SapphireKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewardScreenContext {
    Standard,
    MuggedCombat,
    SmokedCombat,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RewardState {
    pub items: Vec<RewardItem>,
    pub skippable: bool,
    /// Mirrors the Java combat reward screen opening mode:
    /// `open()`, `openCombat(TEXT[0])`, or `openCombat(TEXT[1], true)`.
    pub screen_context: RewardScreenContext,
    /// When a Card reward is claimed, the offered cards are stored here
    /// until the player picks one (SelectCard) or skips.
    pub pending_card_choice: Option<Vec<RewardCard>>,
}

impl Default for RewardState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BossRelicChoiceState {
    pub relics: Vec<RelicId>,
}

impl BossRelicChoiceState {
    pub fn new(relics: Vec<RelicId>) -> Self {
        Self { relics }
    }
}

impl RewardState {
    pub fn new() -> Self {
        RewardState {
            items: Vec::new(),
            skippable: true,
            screen_context: RewardScreenContext::Standard,
            pending_card_choice: None,
        }
    }

    pub fn with_context(screen_context: RewardScreenContext) -> Self {
        RewardState {
            screen_context,
            ..Self::new()
        }
    }
}
