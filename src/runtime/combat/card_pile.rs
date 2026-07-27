use super::CombatCard;
use serde::{Deserialize, Serialize};
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use std::slice;
use std::sync::Arc;

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
pub struct CombatCardPile(Arc<Vec<CombatCard>>);

impl CombatCardPile {
    pub fn into_vec(self) -> Vec<CombatCard> {
        Arc::unwrap_or_clone(self.0)
    }
}

impl Deref for CombatCardPile {
    type Target = Vec<CombatCard>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for CombatCardPile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl From<Vec<CombatCard>> for CombatCardPile {
    fn from(cards: Vec<CombatCard>) -> Self {
        Self(Arc::new(cards))
    }
}

impl From<CombatCardPile> for Vec<CombatCard> {
    fn from(pile: CombatCardPile) -> Self {
        pile.into_vec()
    }
}

impl FromIterator<CombatCard> for CombatCardPile {
    fn from_iter<T: IntoIterator<Item = CombatCard>>(iter: T) -> Self {
        Vec::from_iter(iter).into()
    }
}

impl Extend<CombatCard> for CombatCardPile {
    fn extend<T: IntoIterator<Item = CombatCard>>(&mut self, iter: T) {
        Arc::make_mut(&mut self.0).extend(iter);
    }
}

impl IntoIterator for CombatCardPile {
    type Item = CombatCard;
    type IntoIter = std::vec::IntoIter<CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a CombatCardPile {
    type Item = &'a CombatCard;
    type IntoIter = slice::Iter<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut CombatCardPile {
    type Item = &'a mut CombatCard;
    type IntoIter = slice::IterMut<'a, CombatCard>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl PartialEq<Vec<CombatCard>> for CombatCardPile {
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
        let original: CombatCardPile = vec![CombatCard::new(CardId::Strike, 1)].into();
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
        let pile: CombatCardPile = cards.clone().into();

        assert_eq!(
            serde_json::to_value(&pile).expect("serialize card pile"),
            serde_json::to_value(&cards).expect("serialize card vec")
        );
        assert_eq!(
            serde_json::from_value::<CombatCardPile>(
                serde_json::to_value(&cards).expect("serialize source vec")
            )
            .expect("deserialize card pile"),
            cards
        );
    }
}
