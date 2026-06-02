use serde::Serialize;

use crate::content::cards::{CardTarget, CardType};

use super::super::SearchTerminalLabel;

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ActionFacts {
    pub action_kind: &'static str,
    pub card: Option<CombatSearchV2ActionCardFacts>,
    pub target: Option<CombatSearchV2ActionTargetFacts>,
    pub immediate: CombatSearchV2ActionImmediateFacts,
    pub mechanics: CombatSearchV2ActionMechanicsFacts,
    pub exact_one_step_delta: CombatSearchV2ActionExactDeltaFacts,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ActionCardFacts {
    pub hand_index: usize,
    pub uuid: u32,
    pub card_id: String,
    pub name: &'static str,
    pub upgraded: bool,
    pub card_type: CardType,
    pub definition_target: CardTarget,
    pub effective_target: CardTarget,
    pub cost_for_turn: i32,
    pub base_cost: i8,
    pub evaluated_damage: i32,
    pub evaluated_block: i32,
    pub evaluated_magic: i32,
    pub exhaust: bool,
    pub ethereal: bool,
    pub innate: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ActionTargetFacts {
    pub target_slot: usize,
    pub entity_id: usize,
    pub enemy_id: String,
    pub hp: i32,
    pub block: i32,
    pub visible_incoming_damage: i32,
    pub vulnerable: i32,
    pub weak: i32,
    pub strength: i32,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2ActionImmediateFacts {
    pub damage_hint: i32,
    pub action_payload_damage_hint: i32,
    pub action_payload_damage_hit_count_hint: usize,
    pub block_hint: i32,
    pub target_progress_hint: i32,
    pub all_enemy_progress_hint: i32,
    pub exhausts_card: bool,
    pub creates_pending_choice_after_one_step: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2ActionMechanicsFacts {
    pub persistent_enemy_strength_down: i32,
    pub temporary_enemy_strength_down: i32,
    pub visible_attack_mitigation_hint: i32,
    pub enemy_weak: i32,
    pub enemy_vulnerable: i32,
    pub enemy_strength_gain: i32,
    pub visible_attack_pressure_hint: i32,
    pub player_strength_gain: i32,
    pub player_temporary_strength_gain: i32,
    pub reactive_player_hp_loss: i32,
    pub reactive_player_block: i32,
    pub reactive_enemy_damage: i32,
    pub reactive_bad_draw_cards: i32,
    pub reactive_forced_turn_end: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2ActionExactDeltaFacts {
    pub status: &'static str,
    pub terminal: SearchTerminalLabel,
    pub engine_steps: usize,
    pub player_hp_delta: i32,
    pub player_block_delta: i32,
    pub energy_delta: i32,
    pub hand_delta: i32,
    pub draw_delta: i32,
    pub discard_delta: i32,
    pub exhaust_delta: i32,
    pub limbo_delta: i32,
    pub queued_cards_delta: i32,
    pub total_enemy_hp_delta: i32,
    pub total_enemy_block_delta: i32,
    pub pending_choice_present: bool,
    pub pending_choice_estimated_action_fanout: usize,
}
