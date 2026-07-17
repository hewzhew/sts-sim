use sts_core::runtime::combat::CombatState;

use super::types::{CombatDecisionRoot, CompleteTurnOption, CompleteTurnOptionBoundary};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactI32Change {
    pub before: i32,
    pub after: i32,
}

impl ExactI32Change {
    pub fn delta(self) -> i32 {
        self.after.saturating_sub(self.before)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactCountChange {
    pub before: usize,
    pub after: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExactCombatZoneCounts {
    pub hand: usize,
    pub draw: usize,
    pub discard: usize,
    pub exhaust: usize,
    pub limbo: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactProspectError {
    RootFingerprintMismatch,
}

/// Immediate, simulator-verifiable facts for one complete option.
///
/// These fields deliberately contain no score, preference, strategic reason,
/// or continuation estimate. A later comparator may consume them, but cannot
/// rewrite them into stronger evidence than exact one-turn consequences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactImmediateOptionProspect {
    pub root_exact_state_hash: String,
    pub successor_exact_state_hash: String,
    pub boundary: CompleteTurnOptionBoundary,
    pub player_hp: ExactI32Change,
    pub player_block: ExactI32Change,
    pub energy: ExactI32Change,
    pub gold: ExactI32Change,
    pub gold_delta_this_combat: ExactI32Change,
    pub living_enemies: ExactCountChange,
    pub total_enemy_hp: ExactI32Change,
    pub occupied_potion_slots: ExactCountChange,
    pub changed_potion_slots: usize,
    pub relic_count: ExactCountChange,
    pub relic_state_changed: bool,
    pub zones_before: ExactCombatZoneCounts,
    pub zones_after: ExactCombatZoneCounts,
    pub persistent_meta_changes: ExactCountChange,
}

impl ExactImmediateOptionProspect {
    pub fn from_option(
        root: &CombatDecisionRoot,
        option: &CompleteTurnOption,
    ) -> Result<Self, ExactProspectError> {
        if root.exact_state_hash() != option.root_exact_state_hash() {
            return Err(ExactProspectError::RootFingerprintMismatch);
        }
        let before = &root.position().combat;
        let after = &option.exact_successor().combat;
        Ok(Self {
            root_exact_state_hash: root.exact_state_hash().to_owned(),
            successor_exact_state_hash: option.exact_successor_hash().to_owned(),
            boundary: option.boundary(),
            player_hp: i32_change(
                before.entities.player.current_hp,
                after.entities.player.current_hp,
            ),
            player_block: i32_change(before.entities.player.block, after.entities.player.block),
            energy: i32_change(i32::from(before.turn.energy), i32::from(after.turn.energy)),
            gold: i32_change(before.entities.player.gold, after.entities.player.gold),
            gold_delta_this_combat: i32_change(
                before.entities.player.gold_delta_this_combat,
                after.entities.player.gold_delta_this_combat,
            ),
            living_enemies: count_change(living_enemies(before), living_enemies(after)),
            total_enemy_hp: i32_change(total_enemy_hp(before), total_enemy_hp(after)),
            occupied_potion_slots: count_change(
                before.entities.potions.iter().flatten().count(),
                after.entities.potions.iter().flatten().count(),
            ),
            changed_potion_slots: changed_potion_slots(before, after),
            relic_count: count_change(
                before.entities.player.relics.len(),
                after.entities.player.relics.len(),
            ),
            relic_state_changed: before.entities.player.relics != after.entities.player.relics,
            zones_before: zone_counts(before),
            zones_after: zone_counts(after),
            persistent_meta_changes: count_change(
                before.meta.meta_changes.len(),
                after.meta.meta_changes.len(),
            ),
        })
    }
}

fn i32_change(before: i32, after: i32) -> ExactI32Change {
    ExactI32Change { before, after }
}

fn count_change(before: usize, after: usize) -> ExactCountChange {
    ExactCountChange { before, after }
}

fn living_enemies(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

fn changed_potion_slots(before: &CombatState, after: &CombatState) -> usize {
    let slots = before
        .entities
        .potions
        .len()
        .max(after.entities.potions.len());
    (0..slots)
        .filter(|slot| before.entities.potions.get(*slot) != after.entities.potions.get(*slot))
        .count()
}

fn zone_counts(combat: &CombatState) -> ExactCombatZoneCounts {
    ExactCombatZoneCounts {
        hand: combat.zones.hand.len(),
        draw: combat.zones.draw_pile.len(),
        discard: combat.zones.discard_pile.len(),
        exhaust: combat.zones.exhaust_pile.len(),
        limbo: combat.zones.limbo.len(),
    }
}
