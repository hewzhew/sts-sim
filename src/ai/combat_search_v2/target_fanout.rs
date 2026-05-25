use super::*;
use crate::content::cards::{self, CardTarget};
use crate::content::potions::PotionId;
use std::collections::BTreeMap;

mod diagnostics;
pub(super) use diagnostics::TargetFanoutDiagnosticsCollector;

#[derive(Clone, Debug)]
pub(super) struct TargetFanoutSummary {
    targeted_actions: usize,
    groups: Vec<TargetFanoutGroupSummary>,
}

#[derive(Clone, Debug)]
struct TargetFanoutGroupSummary {
    kind: TargetFanoutKind,
    source_key: String,
    target_count: usize,
    lethal_targets: usize,
    min_target_hp_with_block: i32,
    max_target_hp_with_block: i32,
    min_damage_hint: i32,
    max_damage_hint: i32,
    first_action_key: String,
}

#[derive(Clone, Debug)]
struct TargetFanoutCandidate {
    kind: TargetFanoutKind,
    source_key: String,
    target_hp_with_block: i32,
    damage_hint: i32,
    lethal: bool,
    action_key: String,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum TargetFanoutKind {
    PlayCard,
    UsePotion,
}

pub(super) fn summarize_target_fanout(
    combat: &CombatState,
    choices: &[CombatActionChoice],
) -> TargetFanoutSummary {
    let mut grouped: BTreeMap<(TargetFanoutKind, String), Vec<TargetFanoutCandidate>> =
        BTreeMap::new();

    for choice in choices {
        if let Some(candidate) = target_fanout_candidate(combat, choice) {
            grouped
                .entry((candidate.kind, candidate.source_key.clone()))
                .or_default()
                .push(candidate);
        }
    }

    let targeted_actions = grouped.values().map(Vec::len).sum();
    let groups = grouped
        .into_values()
        .filter_map(summarize_group)
        .collect::<Vec<_>>();

    TargetFanoutSummary {
        targeted_actions,
        groups,
    }
}

fn target_fanout_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
) -> Option<TargetFanoutCandidate> {
    match &choice.input {
        ClientInput::PlayCard {
            card_index,
            target: Some(target),
        } => card_target_candidate(combat, choice, *card_index, *target),
        ClientInput::UsePotion {
            potion_index,
            target: Some(target),
        } => potion_target_candidate(combat, choice, *potion_index, *target),
        _ => None,
    }
}

fn card_target_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
    card_index: usize,
    target: usize,
) -> Option<TargetFanoutCandidate> {
    let card = combat.zones.hand.get(card_index)?;
    let target_hp_with_block = monster_hp_with_block(combat, target)?;
    let target_kind = cards::effective_target(card);
    if !matches!(target_kind, CardTarget::Enemy | CardTarget::SelfAndEnemy) {
        return None;
    }
    let evaluated = cards::evaluate_card_for_play(card, combat, Some(target));
    let damage_hint = evaluated.base_damage_mut.max(0);
    Some(TargetFanoutCandidate {
        kind: TargetFanoutKind::PlayCard,
        source_key: format!(
            "play_card/hand:{card_index}/card:{}+{}/uuid:{}/cost:{}",
            cards::java_id(card.id),
            card.upgrades,
            card.uuid,
            card.cost_for_turn_java()
        ),
        target_hp_with_block,
        damage_hint,
        lethal: damage_hint >= target_hp_with_block && damage_hint > 0,
        action_key: choice.action_key.clone(),
    })
}

fn potion_target_candidate(
    combat: &CombatState,
    choice: &CombatActionChoice,
    potion_index: usize,
    target: usize,
) -> Option<TargetFanoutCandidate> {
    let potion = combat.entities.potions.get(potion_index)?.as_ref()?;
    let target_hp_with_block = monster_hp_with_block(combat, target)?;
    let damage_hint = potion_single_target_damage_hint(combat, potion.id);
    Some(TargetFanoutCandidate {
        kind: TargetFanoutKind::UsePotion,
        source_key: format!(
            "use_potion/slot:{potion_index}/potion:{:?}/uuid:{}",
            potion.id, potion.uuid
        ),
        target_hp_with_block,
        damage_hint,
        lethal: damage_hint >= target_hp_with_block && damage_hint > 0,
        action_key: choice.action_key.clone(),
    })
}

fn potion_single_target_damage_hint(combat: &CombatState, id: PotionId) -> i32 {
    match id {
        PotionId::FirePotion => potion_potency(combat, id),
        _ => 0,
    }
}

fn potion_potency(combat: &CombatState, id: PotionId) -> i32 {
    let mut potency = crate::content::potions::get_potion_definition(id).base_potency;
    if combat
        .entities
        .player
        .has_relic(crate::content::relics::RelicId::SacredBark)
    {
        potency *= 2;
    }
    potency
}

fn summarize_group(candidates: Vec<TargetFanoutCandidate>) -> Option<TargetFanoutGroupSummary> {
    let first = candidates.first()?;
    let mut min_target_hp_with_block = i32::MAX;
    let mut max_target_hp_with_block = i32::MIN;
    let mut min_damage_hint = i32::MAX;
    let mut max_damage_hint = i32::MIN;
    let mut lethal_targets = 0usize;

    for candidate in &candidates {
        min_target_hp_with_block = min_target_hp_with_block.min(candidate.target_hp_with_block);
        max_target_hp_with_block = max_target_hp_with_block.max(candidate.target_hp_with_block);
        min_damage_hint = min_damage_hint.min(candidate.damage_hint);
        max_damage_hint = max_damage_hint.max(candidate.damage_hint);
        if candidate.lethal {
            lethal_targets = lethal_targets.saturating_add(1);
        }
    }

    Some(TargetFanoutGroupSummary {
        kind: first.kind,
        source_key: first.source_key.clone(),
        target_count: candidates.len(),
        lethal_targets,
        min_target_hp_with_block,
        max_target_hp_with_block,
        min_damage_hint,
        max_damage_hint,
        first_action_key: first.action_key.clone(),
    })
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

impl TargetFanoutGroupSummary {
    fn target_hp_span(&self) -> i32 {
        self.max_target_hp_with_block - self.min_target_hp_with_block
    }
}

#[cfg(test)]
mod tests;
