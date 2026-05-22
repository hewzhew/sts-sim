use std::mem::Discriminant;

use crate::runtime::action::Action;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedActionKey {
    pub(crate) discriminant: Discriminant<Action>,
    pub(crate) payload: String,
}
