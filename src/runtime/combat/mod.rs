use crate::content::cards::CardId;
use crate::content::relics::RelicState;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::monster_move::{AttackSpec, DamageKind, MonsterMoveSpec, MonsterTurnPlan};
use crate::state::selection::{DomainEvent, EngineDiagnostic};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};

pub use crate::content::powers::PowerId;
pub type MonsterId = usize;

mod card;
mod combat_methods;
mod entities;
mod monster_runtime;
mod orbs;
mod power;
mod state;

pub use card::*;
pub use entities::*;
pub use monster_runtime::*;
pub use orbs::*;
pub use power::*;
pub use state::*;

#[cfg(test)]
mod tests;
