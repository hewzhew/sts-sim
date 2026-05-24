use super::action_effects::state_sustained_mitigation_score;
use super::card_pile_value::{card_pile_value_report, hand_value, next_draw_value};
use super::enemy_phase_value::enemy_phase_value;
use super::*;

pub(super) const COMBAT_SEARCH_FRONTIER_VALUE_POLICY: &str =
    "frontier_value_v1_visible_pressure_split_phase_enemy_progress_hand_next_draw_resources_no_terminal_claim";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchStateValueV1 {
    pub(super) fewer_living_enemies: i32,
    pub(super) phase_adjusted_enemy_progress: i32,
    pub(super) enemy_progress: i32,
    pub(super) split_debt_hp: i32,
    pub(super) survival_margin: i32,
    pub(super) sustained_mitigation: i32,
    pub(super) player_hp: i32,
    pub(super) player_block: i32,
    pub(super) hand_damage: i32,
    pub(super) hand_block: i32,
    pub(super) hand_playable_cards: i32,
    pub(super) hand_low_cost: i32,
    pub(super) next_draw_damage: i32,
    pub(super) next_draw_block: i32,
    pub(super) next_draw_playable_cards: i32,
    pub(super) next_draw_low_cost: i32,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchRolloutValueV1 {
    pub(super) evaluated: i32,
    pub(super) terminal_rank: i32,
    pub(super) final_hp: i32,
    pub(super) enemy_progress: i32,
    pub(super) survival_margin: i32,
    pub(super) potion_conservation: i32,
    pub(super) faster_turns: i32,
    pub(super) fewer_cards_played: i32,
}

impl Ord for CombatSearchStateValueV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fewer_living_enemies
            .cmp(&other.fewer_living_enemies)
            .then_with(|| {
                self.phase_adjusted_enemy_progress
                    .cmp(&other.phase_adjusted_enemy_progress)
            })
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| self.split_debt_hp.cmp(&other.split_debt_hp))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| self.sustained_mitigation.cmp(&other.sustained_mitigation))
            .then_with(|| self.player_hp.cmp(&other.player_hp))
            .then_with(|| self.player_block.cmp(&other.player_block))
            .then_with(|| self.hand_damage.cmp(&other.hand_damage))
            .then_with(|| self.hand_block.cmp(&other.hand_block))
            .then_with(|| self.hand_playable_cards.cmp(&other.hand_playable_cards))
            .then_with(|| self.hand_low_cost.cmp(&other.hand_low_cost))
            .then_with(|| self.next_draw_damage.cmp(&other.next_draw_damage))
            .then_with(|| self.next_draw_block.cmp(&other.next_draw_block))
            .then_with(|| {
                self.next_draw_playable_cards
                    .cmp(&other.next_draw_playable_cards)
            })
            .then_with(|| self.next_draw_low_cost.cmp(&other.next_draw_low_cost))
    }
}

impl PartialOrd for CombatSearchStateValueV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CombatSearchRolloutValueV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.evaluated
            .cmp(&other.evaluated)
            .then_with(|| self.terminal_rank.cmp(&other.terminal_rank))
            .then_with(|| self.final_hp.cmp(&other.final_hp))
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| self.potion_conservation.cmp(&other.potion_conservation))
            .then_with(|| self.faster_turns.cmp(&other.faster_turns))
            .then_with(|| self.fewer_cards_played.cmp(&other.fewer_cards_played))
    }
}

impl PartialOrd for CombatSearchRolloutValueV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn terminal_rank(label: SearchTerminalLabel) -> i32 {
    match label {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

pub(super) fn total_living_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

pub(super) fn living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
}

pub(super) fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

pub(super) fn survival_margin(combat: &CombatState) -> i32 {
    combat.entities.player.current_hp + combat.entities.player.block
        - visible_incoming_damage(combat)
}

pub(super) fn combat_search_state_value(node: &SearchNode) -> CombatSearchStateValueV1 {
    let hand = hand_value(&node.combat);
    let next_draw = next_draw_value(&node.combat);
    let enemy_phase = enemy_phase_value(&node.combat);
    CombatSearchStateValueV1 {
        fewer_living_enemies: -(living_enemy_count(&node.combat) as i32),
        phase_adjusted_enemy_progress: -enemy_phase.phase_adjusted_living_enemy_hp,
        enemy_progress: -enemy_phase.raw_living_enemy_hp,
        split_debt_hp: -enemy_phase.split_debt_hp,
        survival_margin: survival_margin(&node.combat),
        sustained_mitigation: state_sustained_mitigation_score(&node.combat),
        player_hp: node.combat.entities.player.current_hp,
        player_block: node.combat.entities.player.block,
        hand_damage: hand.damage,
        hand_block: hand.block,
        hand_playable_cards: hand.playable_cards,
        hand_low_cost: hand.low_cost,
        next_draw_damage: next_draw.damage,
        next_draw_block: next_draw.block,
        next_draw_playable_cards: next_draw.playable_cards,
        next_draw_low_cost: next_draw.low_cost,
    }
}

pub(super) fn rollout_priority_value(estimate: RolloutNodeEstimate) -> CombatSearchRolloutValueV1 {
    CombatSearchRolloutValueV1 {
        evaluated: i32::from(estimate.evaluated),
        terminal_rank: estimate.priority_terminal_rank(),
        final_hp: estimate.final_hp,
        enemy_progress: estimate.enemy_progress(),
        survival_margin: estimate.survival_margin,
        potion_conservation: estimate.potion_conservation(),
        faster_turns: estimate.faster_turns(),
        fewer_cards_played: estimate.fewer_cards_played(),
    }
}

pub(super) fn combat_search_frontier_value_report(
    node: &SearchNode,
) -> CombatSearchV2FrontierValueReport {
    let hand = hand_value(&node.combat);
    let next_draw = next_draw_value(&node.combat);
    let enemy_phase = enemy_phase_value(&node.combat);
    CombatSearchV2FrontierValueReport {
        policy: COMBAT_SEARCH_FRONTIER_VALUE_POLICY,
        terminal: terminal_label(&node.engine, &node.combat),
        player_hp: node.combat.entities.player.current_hp,
        player_block: node.combat.entities.player.block,
        visible_incoming_damage: visible_incoming_damage(&node.combat),
        survival_margin: survival_margin(&node.combat),
        living_enemy_count: living_enemy_count(&node.combat),
        total_enemy_hp: enemy_phase.raw_living_enemy_hp,
        phase_adjusted_enemy_hp: enemy_phase.phase_adjusted_living_enemy_hp,
        split_pending_count: enemy_phase.split_pending_count,
        split_debt_hp: enemy_phase.split_debt_hp,
        sustained_mitigation: state_sustained_mitigation_score(&node.combat),
        hand: card_pile_value_report(hand),
        next_draw: card_pile_value_report(next_draw),
        potions_used: node.potions_used,
        potions_discarded: node.potions_discarded,
        cards_played: node.cards_played,
        actions_taken: node.actions.len(),
        estimated: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::test_support::blank_test_combat;

    #[test]
    fn state_value_prefers_survival_before_future_draw_quality() {
        let mut safe = test_node();
        safe.combat.entities.player.current_hp = 20;
        safe.combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 11)];

        let mut flashy = test_node();
        flashy.combat.entities.player.current_hp = 10;
        flashy.combat.zones.draw_pile = vec![CombatCard::new(CardId::Carnage, 12)];

        assert!(combat_search_state_value(&safe) > combat_search_state_value(&flashy));
    }

    #[test]
    fn state_value_accounts_for_pending_split_phase_debt() {
        let mut raw_progress = test_node();
        let mut raw_slime = crate::test_support::test_monster(EnemyId::AcidSlimeL);
        raw_slime.id = 12;
        raw_slime.current_hp = 32;
        raw_slime.max_hp = 65;
        raw_slime.set_planned_move_id(1);
        raw_progress.combat.entities.monsters = vec![raw_slime];

        let mut split_pending = test_node();
        let mut split_slime = crate::test_support::test_monster(EnemyId::AcidSlimeL);
        split_slime.id = 13;
        split_slime.current_hp = 31;
        split_slime.max_hp = 65;
        split_slime.set_planned_move_id(3);
        split_pending.combat.entities.monsters = vec![split_slime];
        split_pending.combat.entities.power_db.insert(
            13,
            vec![Power {
                power_type: PowerId::Split,
                instance_id: None,
                amount: -1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        assert!(
            combat_search_state_value(&raw_progress) > combat_search_state_value(&split_pending)
        );
    }

    fn test_node() -> SearchNode {
        SearchNode {
            engine: EngineState::CombatPlayerTurn,
            combat: blank_test_combat(),
            actions: Vec::new(),
            turn_prefix: TurnPrefixState::default(),
            initial_hp: 80,
            potions_used: 0,
            potions_discarded: 0,
            cards_played: 0,
            potion_tactical_priority: 0,
            last_turn_branch_priority: 0,
            rollout_estimate: RolloutNodeEstimate::unevaluated(),
        }
    }
}
