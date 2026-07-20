use super::super::frontier::SearchNode;
use super::facts::combat_search_core_value_facts;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct CombatSearchStateValueV1 {
    pub(in crate::ai::combat_search_v2) fewer_living_enemies: i32,
    pub(in crate::ai::combat_search_v2) phase_adjusted_enemy_effort_progress: i32,
    pub(in crate::ai::combat_search_v2) enemy_effort_progress: i32,
    pub(in crate::ai::combat_search_v2) enemy_hp_progress: i32,
    pub(in crate::ai::combat_search_v2) split_debt_hp: i32,
    pub(in crate::ai::combat_search_v2) guardian_defensive_block: i32,
    pub(in crate::ai::combat_search_v2) guardian_mode_shift_pending: i32,
    pub(in crate::ai::combat_search_v2) lagavulin_waking_pressure: i32,
    pub(in crate::ai::combat_search_v2) gremlin_nob_enrage_pressure: i32,
    pub(in crate::ai::combat_search_v2) sentry_dazed_pressure: i32,
    pub(in crate::ai::combat_search_v2) hexaghost_opening_pressure: i32,
    pub(in crate::ai::combat_search_v2) high_fanout_pending_choice: i32,
    pub(in crate::ai::combat_search_v2) pending_choice_estimated_action_fanout: i32,
    pub(in crate::ai::combat_search_v2) survival_margin: i32,
    pub(in crate::ai::combat_search_v2) sustained_mitigation: i32,
    pub(in crate::ai::combat_search_v2) player_hp: i32,
    pub(in crate::ai::combat_search_v2) player_block: i32,
    pub(in crate::ai::combat_search_v2) hand_damage: i32,
    pub(in crate::ai::combat_search_v2) hand_block: i32,
    pub(in crate::ai::combat_search_v2) hand_playable_cards: i32,
    pub(in crate::ai::combat_search_v2) hand_low_cost: i32,
    pub(in crate::ai::combat_search_v2) next_draw_damage: i32,
    pub(in crate::ai::combat_search_v2) next_draw_block: i32,
    pub(in crate::ai::combat_search_v2) next_draw_playable_cards: i32,
    pub(in crate::ai::combat_search_v2) next_draw_low_cost: i32,
}

impl Ord for CombatSearchStateValueV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.fewer_living_enemies
            .cmp(&other.fewer_living_enemies)
            .then_with(|| {
                self.phase_adjusted_enemy_effort_progress
                    .cmp(&other.phase_adjusted_enemy_effort_progress)
            })
            .then_with(|| self.enemy_effort_progress.cmp(&other.enemy_effort_progress))
            .then_with(|| self.enemy_hp_progress.cmp(&other.enemy_hp_progress))
            .then_with(|| self.split_debt_hp.cmp(&other.split_debt_hp))
            .then_with(|| {
                self.guardian_defensive_block
                    .cmp(&other.guardian_defensive_block)
            })
            .then_with(|| {
                self.guardian_mode_shift_pending
                    .cmp(&other.guardian_mode_shift_pending)
            })
            .then_with(|| {
                self.lagavulin_waking_pressure
                    .cmp(&other.lagavulin_waking_pressure)
            })
            .then_with(|| {
                self.gremlin_nob_enrage_pressure
                    .cmp(&other.gremlin_nob_enrage_pressure)
            })
            .then_with(|| self.sentry_dazed_pressure.cmp(&other.sentry_dazed_pressure))
            .then_with(|| {
                self.hexaghost_opening_pressure
                    .cmp(&other.hexaghost_opening_pressure)
            })
            .then_with(|| {
                self.high_fanout_pending_choice
                    .cmp(&other.high_fanout_pending_choice)
            })
            .then_with(|| {
                self.pending_choice_estimated_action_fanout
                    .cmp(&other.pending_choice_estimated_action_fanout)
            })
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

pub(in crate::ai::combat_search_v2) fn combat_search_state_value(
    node: &SearchNode,
) -> CombatSearchStateValueV1 {
    let facts = combat_search_core_value_facts(&node.engine, &node.combat);
    CombatSearchStateValueV1 {
        fewer_living_enemies: -(facts
            .phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_count as i32),
        phase_adjusted_enemy_effort_progress: -facts
            .phase_profile
            .enemy_phase
            .phase_adjusted_living_enemy_effort,
        enemy_effort_progress: -facts.phase_profile.enemy_phase.raw_living_enemy_effort,
        enemy_hp_progress: -facts.phase_profile.enemy_phase.raw_living_enemy_hp,
        split_debt_hp: -facts.phase_profile.enemy_phase.split_debt_hp,
        guardian_defensive_block: -facts.phase_profile.enemy_phase.guardian_defensive_block,
        guardian_mode_shift_pending: -(facts
            .phase_profile
            .enemy_mechanics
            .guardian_mode_shift_pending_count as i32),
        lagavulin_waking_pressure: -(facts.phase_profile.enemy_mechanics.lagavulin_waking_count
            as i32),
        gremlin_nob_enrage_pressure: -facts
            .phase_profile
            .enemy_mechanics
            .gremlin_nob_anger_amount_total,
        sentry_dazed_pressure: -(facts
            .phase_profile
            .enemy_mechanics
            .sentry_dazed_pressure_count as i32),
        hexaghost_opening_pressure: -(facts
            .phase_profile
            .enemy_mechanics
            .hexaghost_opening_pressure_count as i32),
        high_fanout_pending_choice: -i32::from(facts.phase_profile.pending_choice.high_fanout),
        pending_choice_estimated_action_fanout: -(facts
            .phase_profile
            .pending_choice
            .estimated_action_fanout as i32),
        survival_margin: facts.phase_profile.pressure.survival_margin,
        sustained_mitigation: facts.sustained_mitigation,
        player_hp: node.combat.entities.player.current_hp,
        player_block: node.combat.entities.player.block,
        hand_damage: facts.hand.damage,
        hand_block: facts.hand.block,
        hand_playable_cards: facts.hand.playable_cards,
        hand_low_cost: facts.hand.low_cost,
        next_draw_damage: facts.next_draw.damage,
        next_draw_block: facts.next_draw.block,
        next_draw_playable_cards: facts.next_draw.playable_cards,
        next_draw_low_cost: facts.next_draw.low_cost,
    }
}
