use crate::content::cards::CardId;
use crate::runtime::combat::QueuedCardSource;
use smallvec::SmallVec;
use std::hash::{Hash, Hasher};

/// Exact card-zone identity stored in one allocation.
///
/// The runtime already exposes each zone as an ordered card sequence. Keeping
/// five independent `Vec`s here paid five allocations for every transposition
/// key, even though search never mutates the projection. The boundary offsets
/// retain those five exact sequences in one backing allocation. Custom
/// `Debug` and `Hash` deliberately preserve the former field-by-field shape so
/// durable diagnostic identities and `HashMap` equality semantics do not
/// change with this ownership optimization.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CombatZonesKey {
    pub(crate) card_uuid_counter: u32,
    cards: Vec<CombatCardKey>,
    hand_end: usize,
    draw_end: usize,
    discard_end: usize,
    exhaust_end: usize,
    pub(crate) queued: Vec<CombatQueuedCardKey>,
}

impl CombatZonesKey {
    pub(crate) fn new(
        card_uuid_counter: u32,
        hand: impl ExactSizeIterator<Item = CombatCardKey>,
        draw: impl ExactSizeIterator<Item = CombatCardKey>,
        discard: impl ExactSizeIterator<Item = CombatCardKey>,
        exhaust: impl ExactSizeIterator<Item = CombatCardKey>,
        limbo: impl ExactSizeIterator<Item = CombatCardKey>,
        queued: Vec<CombatQueuedCardKey>,
    ) -> Self {
        let hand_len = hand.len();
        let draw_len = draw.len();
        let discard_len = discard.len();
        let exhaust_len = exhaust.len();
        let limbo_len = limbo.len();
        let total_len = hand_len + draw_len + discard_len + exhaust_len + limbo_len;
        let mut cards = Vec::with_capacity(total_len);
        cards.extend(hand);
        let hand_end = cards.len();
        cards.extend(draw);
        let draw_end = cards.len();
        cards.extend(discard);
        let discard_end = cards.len();
        cards.extend(exhaust);
        let exhaust_end = cards.len();
        cards.extend(limbo);
        debug_assert_eq!(cards.len(), total_len);

        Self {
            card_uuid_counter,
            cards,
            hand_end,
            draw_end,
            discard_end,
            exhaust_end,
            queued,
        }
    }

    fn hand(&self) -> &[CombatCardKey] {
        &self.cards[..self.hand_end]
    }

    fn draw(&self) -> &[CombatCardKey] {
        &self.cards[self.hand_end..self.draw_end]
    }

    fn discard(&self) -> &[CombatCardKey] {
        &self.cards[self.draw_end..self.discard_end]
    }

    fn exhaust(&self) -> &[CombatCardKey] {
        &self.cards[self.discard_end..self.exhaust_end]
    }

    fn limbo(&self) -> &[CombatCardKey] {
        &self.cards[self.exhaust_end..]
    }
}

impl std::fmt::Debug for CombatZonesKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CombatZonesKey")
            .field("card_uuid_counter", &self.card_uuid_counter)
            .field("hand", &self.hand())
            .field("draw", &self.draw())
            .field("discard", &self.discard())
            .field("exhaust", &self.exhaust())
            .field("limbo", &self.limbo())
            .field("queued", &self.queued)
            .finish()
    }
}

impl Hash for CombatZonesKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.card_uuid_counter.hash(state);
        self.hand().hash(state);
        self.draw().hash(state);
        self.discard().hash(state);
        self.exhaust().hash(state);
        self.limbo().hash(state);
        self.queued.hash(state);
    }
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
    pub(crate) multi_damage: SmallVec<[i32; 5]>,
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
