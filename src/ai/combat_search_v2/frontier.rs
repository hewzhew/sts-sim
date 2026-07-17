use super::*;
use crate::content::cards::CardId;

mod node;
mod priority;
mod queue;
pub(in crate::ai::combat_search_v2) mod resources;

pub(super) use node::{RootLineage, RootLineageId, SearchNode};
pub(super) use priority::QueueEntry;
pub(super) use queue::FrontierQueue;
pub(super) use resources::{is_resource_covered, ResourceVector};

pub(super) fn remember_best_complete(best: &mut Option<SearchNode>, candidate: SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(&candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate);
    }
}

pub(super) fn remember_win_candidate(
    candidates: &mut Vec<SearchNode>,
    candidate: &SearchNode,
) -> bool {
    let candidate_resources = WinCandidateResources::from_node(candidate);
    if candidates.iter().any(|existing| {
        WinCandidateResources::from_node(existing).strictly_dominates(candidate_resources)
    }) {
        return false;
    }

    if let Some(equal_index) = candidates
        .iter()
        .position(|existing| WinCandidateResources::from_node(existing) == candidate_resources)
    {
        if compare_nodes(candidate, &candidates[equal_index]) == Ordering::Greater {
            candidates[equal_index] = candidate.clone();
            return true;
        }
        return false;
    }

    candidates.retain(|existing| {
        !candidate_resources.strictly_dominates(WinCandidateResources::from_node(existing))
    });
    candidates.push(candidate.clone());
    true
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct WinCandidateResources {
    final_hp: i32,
    final_max_hp: i32,
    gold_delta_this_combat: i32,
    ritual_dagger_value: i32,
    genetic_algorithm_value: i32,
    external_burden_count: i32,
    potions_used: u32,
    potions_discarded: u32,
    action_count: usize,
}

impl WinCandidateResources {
    fn from_node(node: &SearchNode) -> Self {
        Self {
            final_hp: node.combat.entities.player.current_hp,
            final_max_hp: node.combat.entities.player.max_hp,
            gold_delta_this_combat: node.combat.entities.player.gold_delta_this_combat,
            ritual_dagger_value: super::external_payoff::persistent_card_value(
                &node.combat,
                CardId::RitualDagger,
            ),
            genetic_algorithm_value: super::external_payoff::persistent_card_value(
                &node.combat,
                CardId::GeneticAlgorithm,
            ),
            external_burden_count: super::outcome_score::external_burden_count(&node.combat),
            potions_used: node.potions_used,
            potions_discarded: node.potions_discarded,
            action_count: node.actions.len(),
        }
    }

    fn strictly_dominates(self, other: Self) -> bool {
        self.final_hp >= other.final_hp
            && self.final_max_hp >= other.final_max_hp
            && self.gold_delta_this_combat >= other.gold_delta_this_combat
            && self.ritual_dagger_value >= other.ritual_dagger_value
            && self.genetic_algorithm_value >= other.genetic_algorithm_value
            && self.external_burden_count <= other.external_burden_count
            && self.potions_used <= other.potions_used
            && self.potions_discarded <= other.potions_discarded
            && self.action_count <= other.action_count
            && self != other
    }
}

pub(super) fn remember_best_frontier(best: &mut Option<SearchNode>, candidate: &SearchNode) {
    let replace = best
        .as_ref()
        .map(|existing| compare_nodes(candidate, existing) == Ordering::Greater)
        .unwrap_or(true);
    if replace {
        *best = Some(candidate.clone());
    }
}

fn compare_nodes(left: &SearchNode, right: &SearchNode) -> Ordering {
    CombatOutcomeScore::from_node(left).cmp(&CombatOutcomeScore::from_node(right))
}

#[cfg(test)]
mod tests;
