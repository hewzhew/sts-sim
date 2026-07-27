use super::CombatCard;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::slice;
use std::sync::Arc;

/// Draw-pile storage specialized for exact-search snapshots.
///
/// The active pile is the range `start..end`, in draw order (top first).
/// Drawing only clones the one returned card and advances `start`; cloned
/// combat states continue sharing the immutable backing order. Operations
/// that really change order materialize the active range first.
pub struct DrawPile {
    cards: Arc<Vec<CombatCard>>,
    start: usize,
    end: usize,
}

impl DrawPile {
    fn active(&self) -> &[CombatCard] {
        &self.cards[self.start..self.end]
    }

    fn materialize(&mut self) -> &mut Vec<CombatCard> {
        if self.start != 0 || self.end != self.cards.len() {
            self.cards = Arc::new(self.active().to_vec());
            self.start = 0;
            self.end = self.cards.len();
        }
        Arc::make_mut(&mut self.cards)
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [CombatCard] {
        self.materialize().as_mut_slice()
    }

    pub fn draw_top(&mut self) -> Option<CombatCard> {
        let card = self.active().first()?.clone();
        self.start += 1;
        Some(card)
    }

    pub fn push_top(&mut self, card: CombatCard) {
        if self.start > 0 {
            if let Some(cards) = Arc::get_mut(&mut self.cards) {
                self.start -= 1;
                cards[self.start] = card;
                return;
            }
        }
        self.materialize().insert(0, card);
        self.end += 1;
    }

    pub fn push(&mut self, card: CombatCard) {
        self.materialize().push(card);
        self.end += 1;
    }

    pub fn insert(&mut self, index: usize, card: CombatCard) {
        self.materialize().insert(index, card);
        self.end += 1;
    }

    pub fn remove(&mut self, index: usize) -> CombatCard {
        let card = self.materialize().remove(index);
        self.end -= 1;
        card
    }

    pub fn remove_by_uuid(&mut self, uuid: u32) -> Option<CombatCard> {
        let index = self.iter().position(|card| card.uuid == uuid)?;
        Some(self.remove(index))
    }

    pub fn pop(&mut self) -> Option<CombatCard> {
        let card = self.active().last()?.clone();
        self.end -= 1;
        Some(card)
    }

    pub fn clear(&mut self) {
        self.cards = Arc::new(Vec::new());
        self.start = 0;
        self.end = 0;
    }

    pub fn append(&mut self, other: &mut Vec<CombatCard>) {
        let added = other.len();
        self.materialize().append(other);
        self.end += added;
    }

    pub fn reverse(&mut self) {
        self.materialize().reverse();
    }

    pub fn swap(&mut self, left: usize, right: usize) {
        self.materialize().swap(left, right);
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, CombatCard> {
        self.materialize().iter_mut()
    }

    pub fn into_vec(self) -> Vec<CombatCard> {
        let mut cards = Arc::unwrap_or_clone(self.cards);
        if self.end < cards.len() {
            cards.truncate(self.end);
        }
        if self.start > 0 {
            cards.drain(..self.start);
        }
        cards
    }
}

impl Clone for DrawPile {
    fn clone(&self) -> Self {
        Self {
            cards: self.cards.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl Default for DrawPile {
    fn default() -> Self {
        Vec::new().into()
    }
}

impl fmt::Debug for DrawPile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq for DrawPile {
    fn eq(&self, other: &Self) -> bool {
        self.active() == other.active()
    }
}

impl Eq for DrawPile {}

impl Serialize for DrawPile {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.active().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DrawPile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<CombatCard>::deserialize(deserializer).map(Into::into)
    }
}

impl Deref for DrawPile {
    type Target = [CombatCard];

    fn deref(&self) -> &Self::Target {
        self.active()
    }
}

impl AsRef<[CombatCard]> for DrawPile {
    fn as_ref(&self) -> &[CombatCard] {
        self.active()
    }
}

impl From<Vec<CombatCard>> for DrawPile {
    fn from(cards: Vec<CombatCard>) -> Self {
        let end = cards.len();
        Self {
            cards: Arc::new(cards),
            start: 0,
            end,
        }
    }
}

impl FromIterator<CombatCard> for DrawPile {
    fn from_iter<T: IntoIterator<Item = CombatCard>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl IntoIterator for DrawPile {
    type Item = CombatCard;
    type IntoIter = std::vec::IntoIter<CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a DrawPile {
    type Item = &'a CombatCard;
    type IntoIter = slice::Iter<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut DrawPile {
    type Item = &'a mut CombatCard;
    type IntoIter = slice::IterMut<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl PartialEq<Vec<CombatCard>> for DrawPile {
    fn eq(&self, other: &Vec<CombatCard>) -> bool {
        self.active() == other.as_slice()
    }
}

/// A combat card pile with cheap snapshots and ordinary `Vec` ergonomics.
///
/// Exact search clones combat positions far more often than it mutates every
/// card zone.  Sharing the immutable backing allocation keeps those snapshots
/// cheap; the first mutable operation detaches through `Arc::make_mut`.
/// `serde(transparent)` deliberately preserves the existing JSON array wire
/// format so continuations, fingerprints, and external tools do not learn
/// about this ownership optimization.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct SharedCardPile(Arc<Vec<CombatCard>>);

impl SharedCardPile {
    pub fn into_vec(self) -> Vec<CombatCard> {
        Arc::unwrap_or_clone(self.0)
    }
}

impl Deref for SharedCardPile {
    type Target = Vec<CombatCard>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SharedCardPile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl From<Vec<CombatCard>> for SharedCardPile {
    fn from(cards: Vec<CombatCard>) -> Self {
        Self(Arc::new(cards))
    }
}

impl From<SharedCardPile> for Vec<CombatCard> {
    fn from(pile: SharedCardPile) -> Self {
        pile.into_vec()
    }
}

impl FromIterator<CombatCard> for SharedCardPile {
    fn from_iter<T: IntoIterator<Item = CombatCard>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl Extend<CombatCard> for SharedCardPile {
    fn extend<T: IntoIterator<Item = CombatCard>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl IntoIterator for SharedCardPile {
    type Item = CombatCard;
    type IntoIter = std::vec::IntoIter<CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a SharedCardPile {
    type Item = &'a CombatCard;
    type IntoIter = slice::Iter<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut SharedCardPile {
    type Item = &'a mut CombatCard;
    type IntoIter = slice::IterMut<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl PartialEq<Vec<CombatCard>> for SharedCardPile {
    fn eq(&self, other: &Vec<CombatCard>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;

    #[test]
    fn clone_shares_until_first_mutation() {
        let original: SharedCardPile = vec![CombatCard::new(CardId::Strike, 1)].into();
        let mut changed = original.clone();

        assert!(Arc::ptr_eq(&original.0, &changed.0));
        changed.push(CombatCard::new(CardId::Defend, 2));

        assert!(!Arc::ptr_eq(&original.0, &changed.0));
        assert_eq!(original.len(), 1);
        assert_eq!(changed.len(), 2);
    }

    #[test]
    fn serde_wire_format_remains_a_plain_array() {
        let cards = vec![CombatCard::new(CardId::Bash, 7)];
        let pile: SharedCardPile = cards.clone().into();

        assert_eq!(
            serde_json::to_value(&pile).expect("serialize card pile"),
            serde_json::to_value(&cards).expect("serialize card vec")
        );
        assert_eq!(
            serde_json::from_value::<SharedCardPile>(
                serde_json::to_value(&cards).expect("serialize source vec")
            )
            .expect("deserialize card pile"),
            cards
        );
    }

    #[test]
    fn draw_cursor_keeps_sibling_snapshot_and_serializes_only_active_cards() {
        let first = CombatCard::new(CardId::Strike, 1);
        let second = CombatCard::new(CardId::Defend, 2);
        let mut advanced: DrawPile = vec![first.clone(), second.clone()].into();
        let sibling = advanced.clone();

        assert_eq!(advanced.draw_top(), Some(first));
        assert!(Arc::ptr_eq(&advanced.cards, &sibling.cards));
        assert_eq!(advanced.as_ref(), [second.clone()]);
        assert_eq!(
            sibling.as_ref(),
            [CombatCard::new(CardId::Strike, 1), second.clone()]
        );
        assert_eq!(
            serde_json::to_value(&advanced).expect("serialize active draw pile"),
            serde_json::to_value(vec![second]).expect("serialize expected active cards")
        );
    }

    #[test]
    fn order_mutation_materializes_only_the_mutated_branch() {
        let mut branch: DrawPile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ]
        .into();
        let sibling = branch.clone();

        assert_eq!(branch.draw_top().map(|card| card.uuid), Some(1));
        branch.push_top(CombatCard::new(CardId::Bash, 3));

        assert!(!Arc::ptr_eq(&branch.cards, &sibling.cards));
        assert_eq!(
            branch.iter().map(|card| card.uuid).collect::<Vec<_>>(),
            vec![3, 2]
        );
        assert_eq!(
            sibling.iter().map(|card| card.uuid).collect::<Vec<_>>(),
            vec![1, 2]
        );
    }
}
