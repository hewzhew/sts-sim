use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::{Potion, PotionId};
use crate::runtime::combat::CombatCard;
use crate::sim::combat_action::CombatActionChoice;
use crate::test_support::{blank_test_combat, test_monster};

mod defensive_resource;
mod offensive;

fn attacking_monster() -> MonsterEntity {
    let mut monster = test_monster(EnemyId::Cultist);
    monster.set_planned_move_id(1);
    monster
}
