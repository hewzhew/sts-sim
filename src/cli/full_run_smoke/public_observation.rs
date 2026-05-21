use serde::Serialize;
use serde_json::Value;

use super::{
    ActionSemanticDescriptorV1, CandidatePlanDeltaV0, RunActionCandidate, RunCardFeatureV0,
    RunChoiceOptionV1, RunCombatHandCardObservationV0, RunCombatObservationV0, RunDecisionFrameV1,
    RunDeckCardObservationV0, RunDeckObservationV0, RunKeyObservationV0, RunMapEdgeObservationV0,
    RunMapNodeObservationV0, RunMapObservationV0, RunMapRouteContextV1, RunMonsterObservationV0,
    RunPendingChoiceObservationV0, RunPendingChoiceOptionObservationV0, RunPotionSlotObservationV0,
    RunPowerObservationV0, RunRecordingViewV1, RunRelicObservationV0,
    RunRewardCardChoiceObservationV0, RunRewardItemObservationV0, RunScreenObservationV0,
};

pub const FULL_RUN_PUBLIC_OBSERVATION_SCHEMA_VERSION: &str =
    "full_run_public_observation_v4_decision_frame";
pub const FULL_RUN_PUBLIC_ACTION_SCHEMA_VERSION: &str =
    "full_run_public_action_candidate_v6_choice_option";

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FullRunPublicObservationV1 {
    pub schema_version: String,
    pub source_schema_version: String,
    pub decision_type: String,
    pub engine_state: String,
    pub act: u8,
    pub floor: i32,
    pub current_room: Option<String>,
    pub current_hp: i32,
    pub max_hp: i32,
    pub hp_ratio_milli: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub potion_slots: usize,
    pub filled_potion_slots: usize,
    pub keys: PublicKeyObservationV1,
    pub deck: PublicDeckSummaryV1,
    pub deck_cards: Vec<PublicDeckCardV1>,
    pub relics: Vec<PublicRelicV1>,
    pub potions: Vec<PublicPotionSlotV1>,
    pub map: Option<PublicMapV1>,
    pub next_nodes: Vec<PublicMapNodeV1>,
    pub map_route_context: Option<RunMapRouteContextV1>,
    pub act_boss: Option<String>,
    pub reward_source: Option<String>,
    pub combat: Option<PublicCombatObservationV1>,
    pub screen: PublicScreenObservationV1,
    pub recording_view: PublicRecordingViewV1,
    pub decision_frame: RunDecisionFrameV1,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicRecordingViewV1 {
    pub schema_name: String,
    pub schema_version: u8,
    pub recording_source: String,
    pub state_lines: Vec<String>,
    pub context_lines: Vec<String>,
    pub warning_lines: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicKeyObservationV1 {
    pub ruby: bool,
    pub sapphire: bool,
    pub emerald: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicDeckSummaryV1 {
    pub attack_count: usize,
    pub skill_count: usize,
    pub power_count: usize,
    pub status_count: usize,
    pub curse_count: usize,
    pub starter_basic_count: usize,
    pub damage_card_count: usize,
    pub block_card_count: usize,
    pub draw_card_count: usize,
    pub scaling_card_count: usize,
    pub exhaust_card_count: usize,
    pub average_cost_milli: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicDeckCardV1 {
    pub deck_index: usize,
    pub uuid: u32,
    pub card: PublicCardFeatureV1,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicRelicV1 {
    pub relic_id: String,
    pub counter: i32,
    pub used_up: bool,
    pub amount: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicPotionSlotV1 {
    pub slot_index: usize,
    pub potion_id: Option<String>,
    pub uuid: Option<u32>,
    pub can_use: bool,
    pub can_discard: bool,
    pub requires_target: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicMapV1 {
    pub current_x: i32,
    pub current_y: i32,
    pub boss_node_available: bool,
    pub has_emerald_key: bool,
    pub nodes: Vec<PublicMapNodeV1>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicMapNodeV1 {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<String>,
    pub has_emerald_key: bool,
    pub reachable_now: bool,
    pub edges: Vec<PublicMapEdgeV1>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicMapEdgeV1 {
    pub dst_x: i32,
    pub dst_y: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct PublicCombatObservationV1 {
    pub player_hp: i32,
    pub player_block: i32,
    pub player_powers: Vec<PublicPowerV1>,
    pub energy: i32,
    pub combat_phase: String,
    pub turn_count: u32,
    pub hand_count: usize,
    pub hand_cards: Vec<PublicCombatHandCardV1>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub alive_monster_count: usize,
    pub dying_monster_count: usize,
    pub half_dead_monster_count: usize,
    pub zero_hp_monster_count: usize,
    pub pending_rebirth_monster_count: usize,
    pub total_monster_hp: i32,
    pub visible_incoming_damage: i32,
    pub pending_action_count: usize,
    pub queued_card_count: usize,
    pub limbo_count: usize,
    pub pending_choice_kind: Option<String>,
    pub pending_choice: Option<PublicPendingChoiceV1>,
    pub monsters: Vec<PublicMonsterV1>,
    pub encounter_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicMonsterV1 {
    pub entity_id: usize,
    pub slot: u8,
    pub monster_id: String,
    pub name: String,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub alive: bool,
    pub planned_move_id: u8,
    pub visible_intent: Option<String>,
    pub visible_intent_kind: String,
    pub visible_intent_damage_per_hit: Option<i32>,
    pub visible_intent_hits: u8,
    pub visible_intent_total_damage: Option<i32>,
    pub powers: Vec<PublicPowerV1>,
    pub mechanic_hints: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicPowerV1 {
    pub power_id: String,
    pub amount: i32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicPendingChoiceV1 {
    pub kind: String,
    pub min_select: u8,
    pub max_select: u8,
    pub can_cancel: bool,
    pub reason: Option<String>,
    pub source_pile: Option<String>,
    pub options: Vec<PublicPendingChoiceOptionV1>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicPendingChoiceOptionV1 {
    pub option_index: usize,
    pub label: String,
    pub card_id: Option<String>,
    pub card_uuid: Option<u32>,
    pub selection_uuids: Vec<u32>,
    pub source_pile: Option<String>,
    pub subject_ref: Option<String>,
    pub before_summary: Option<String>,
    pub after_summary: Option<String>,
    pub delta_summary: Option<String>,
    pub preview_status: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicCombatHandCardV1 {
    pub hand_index: usize,
    pub card_instance_id: u32,
    pub card_id: String,
    pub upgraded: bool,
    pub upgrades: u8,
    pub cost_for_turn: i8,
    pub playable: bool,
    pub base_semantics: Vec<String>,
    pub transient_tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicScreenObservationV1 {
    pub event_option_count: usize,
    pub event_options: Vec<PublicEventOptionV1>,
    pub reward_item_count: usize,
    pub reward_card_choice_count: usize,
    pub reward_phase: String,
    pub reward_items: Vec<PublicRewardItemV1>,
    pub reward_card_choices: Vec<PublicRewardCardChoiceV1>,
    pub reward_claimable_item_count: usize,
    pub reward_unclaimed_card_item_count: usize,
    pub shop_card_count: usize,
    pub shop_relic_count: usize,
    pub shop_potion_count: usize,
    pub boss_relic_choice_count: usize,
    pub selection_target_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicEventOptionV1 {
    pub option_index: usize,
    pub label: String,
    pub disabled: bool,
    pub disabled_reason: Option<String>,
    pub semantic_descriptor: ActionSemanticDescriptorV1,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicRewardItemV1 {
    pub item_index: usize,
    pub item_type: String,
    pub amount: i32,
    pub card_choice_count: usize,
    pub relic_id: Option<String>,
    pub potion_id: Option<String>,
    pub claimable: bool,
    pub opens_card_choice: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicRewardCardChoiceV1 {
    pub option_index: usize,
    pub card_id: String,
    pub card_name: String,
    pub upgrades: u8,
    pub card_type: String,
    pub rarity: String,
    pub cost: i8,
    pub base_semantics: Vec<String>,
    pub deck_copies: usize,
    pub card: PublicCardFeatureV1,
    pub plan_delta: CandidatePlanDeltaV0,
    pub semantic_descriptor: ActionSemanticDescriptorV1,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PublicCardFeatureV1 {
    pub card_id: String,
    pub card_id_hash: u32,
    pub card_type_id: u8,
    pub rarity_id: u8,
    pub cost: i8,
    pub upgrades: u8,
    pub base_damage: i32,
    pub base_block: i32,
    pub base_magic: i32,
    pub upgraded_damage: i32,
    pub upgraded_block: i32,
    pub upgraded_magic: i32,
    pub exhaust: bool,
    pub ethereal: bool,
    pub innate: bool,
    pub aoe: bool,
    pub multi_damage: bool,
    pub starter_basic: bool,
    pub draws_cards: bool,
    pub gains_energy: bool,
    pub applies_weak: bool,
    pub applies_vulnerable: bool,
    pub scaling_piece: bool,
    pub deck_copies: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct FullRunPublicActionCandidatePayloadV1 {
    pub schema_version: String,
    pub action_index: usize,
    pub action_id: u32,
    pub action_key: String,
    pub recording_label: String,
    pub recording_detail: Option<String>,
    pub recording_kind: String,
    pub action: Value,
    pub card: Option<PublicCardFeatureV1>,
    pub plan_delta: Option<Value>,
    pub reward_structure: Option<Value>,
    pub semantic_descriptor: Option<ActionSemanticDescriptorV1>,
    pub choice_option: RunChoiceOptionV1,
    pub dominated: bool,
    pub dominated_by_index: Option<usize>,
}

impl FullRunPublicObservationV1 {
    pub fn from_observation(value: &super::RunObservationV0, source_schema_version: &str) -> Self {
        Self {
            schema_version: FULL_RUN_PUBLIC_OBSERVATION_SCHEMA_VERSION.to_string(),
            source_schema_version: source_schema_version.to_string(),
            decision_type: value.decision_type.clone(),
            engine_state: value.engine_state.clone(),
            act: value.act,
            floor: value.floor,
            current_room: value.current_room.clone(),
            current_hp: value.current_hp,
            max_hp: value.max_hp,
            hp_ratio_milli: value.hp_ratio_milli,
            gold: value.gold,
            deck_size: value.deck_size,
            relic_count: value.relic_count,
            potion_slots: value.potion_slots,
            filled_potion_slots: value.filled_potion_slots,
            keys: PublicKeyObservationV1::from(&value.keys),
            deck: PublicDeckSummaryV1::from(&value.deck),
            deck_cards: value
                .deck_cards
                .iter()
                .map(PublicDeckCardV1::from)
                .collect(),
            relics: value.relics.iter().map(PublicRelicV1::from).collect(),
            potions: value.potions.iter().map(PublicPotionSlotV1::from).collect(),
            map: value.map.as_ref().map(PublicMapV1::from),
            next_nodes: value.next_nodes.iter().map(PublicMapNodeV1::from).collect(),
            map_route_context: value.map_route_context.clone(),
            act_boss: value.act_boss.clone(),
            reward_source: value.reward_source.clone(),
            combat: value.combat.as_ref().map(PublicCombatObservationV1::from),
            screen: PublicScreenObservationV1::from(&value.screen),
            recording_view: PublicRecordingViewV1::from(&value.recording_view),
            decision_frame: value.decision_frame.clone(),
        }
    }
}

impl FullRunPublicActionCandidatePayloadV1 {
    pub fn from_candidate(candidate: &RunActionCandidate) -> Result<Self, serde_json::Error> {
        Ok(Self {
            schema_version: FULL_RUN_PUBLIC_ACTION_SCHEMA_VERSION.to_string(),
            action_index: candidate.action_index,
            action_id: candidate.action_id,
            action_key: candidate.action_key.clone(),
            recording_label: candidate.recording_label.clone(),
            recording_detail: candidate.recording_detail.clone(),
            recording_kind: candidate.recording_kind.clone(),
            action: serde_json::to_value(&candidate.action)?,
            card: candidate.card.as_ref().map(PublicCardFeatureV1::from),
            plan_delta: candidate
                .plan_delta
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
            reward_structure: candidate
                .reward_structure
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?,
            semantic_descriptor: candidate.semantic_descriptor.clone(),
            choice_option: candidate.choice_option.clone(),
            dominated: candidate.dominated,
            dominated_by_index: candidate.dominated_by_index,
        })
    }
}

impl From<&RunRecordingViewV1> for PublicRecordingViewV1 {
    fn from(value: &RunRecordingViewV1) -> Self {
        Self {
            schema_name: value.schema_name.clone(),
            schema_version: value.schema_version,
            recording_source: value.recording_source.clone(),
            state_lines: value.state_lines.clone(),
            context_lines: value.context_lines.clone(),
            warning_lines: value.warning_lines.clone(),
        }
    }
}

impl From<&RunKeyObservationV0> for PublicKeyObservationV1 {
    fn from(value: &RunKeyObservationV0) -> Self {
        Self {
            ruby: value.ruby,
            sapphire: value.sapphire,
            emerald: value.emerald,
        }
    }
}

impl From<&RunDeckObservationV0> for PublicDeckSummaryV1 {
    fn from(value: &RunDeckObservationV0) -> Self {
        Self {
            attack_count: value.attack_count,
            skill_count: value.skill_count,
            power_count: value.power_count,
            status_count: value.status_count,
            curse_count: value.curse_count,
            starter_basic_count: value.starter_basic_count,
            damage_card_count: value.damage_card_count,
            block_card_count: value.block_card_count,
            draw_card_count: value.draw_card_count,
            scaling_card_count: value.scaling_card_count,
            exhaust_card_count: value.exhaust_card_count,
            average_cost_milli: value.average_cost_milli,
        }
    }
}

impl From<&RunDeckCardObservationV0> for PublicDeckCardV1 {
    fn from(value: &RunDeckCardObservationV0) -> Self {
        Self {
            deck_index: value.deck_index,
            uuid: value.uuid,
            card: PublicCardFeatureV1::from(&value.card),
        }
    }
}

impl From<&RunRelicObservationV0> for PublicRelicV1 {
    fn from(value: &RunRelicObservationV0) -> Self {
        Self {
            relic_id: value.relic_id.clone(),
            counter: value.counter,
            used_up: value.used_up,
            amount: value.amount,
        }
    }
}

impl From<&RunPotionSlotObservationV0> for PublicPotionSlotV1 {
    fn from(value: &RunPotionSlotObservationV0) -> Self {
        Self {
            slot_index: value.slot_index,
            potion_id: value.potion_id.clone(),
            uuid: value.uuid,
            can_use: value.can_use,
            can_discard: value.can_discard,
            requires_target: value.requires_target,
        }
    }
}

impl From<&RunMapObservationV0> for PublicMapV1 {
    fn from(value: &RunMapObservationV0) -> Self {
        Self {
            current_x: value.current_x,
            current_y: value.current_y,
            boss_node_available: value.boss_node_available,
            has_emerald_key: value.has_emerald_key,
            nodes: value.nodes.iter().map(PublicMapNodeV1::from).collect(),
        }
    }
}

impl From<&RunMapNodeObservationV0> for PublicMapNodeV1 {
    fn from(value: &RunMapNodeObservationV0) -> Self {
        Self {
            x: value.x,
            y: value.y,
            room_type: value.room_type.clone(),
            has_emerald_key: value.has_emerald_key,
            reachable_now: value.reachable_now,
            edges: value.edges.iter().map(PublicMapEdgeV1::from).collect(),
        }
    }
}

impl From<&RunMapEdgeObservationV0> for PublicMapEdgeV1 {
    fn from(value: &RunMapEdgeObservationV0) -> Self {
        Self {
            dst_x: value.dst_x,
            dst_y: value.dst_y,
        }
    }
}

impl From<&RunCombatObservationV0> for PublicCombatObservationV1 {
    fn from(value: &RunCombatObservationV0) -> Self {
        Self {
            player_hp: value.player_hp,
            player_block: value.player_block,
            player_powers: value
                .player_powers
                .iter()
                .map(PublicPowerV1::from)
                .collect(),
            energy: value.energy,
            combat_phase: value.combat_phase.clone(),
            turn_count: value.turn_count,
            hand_count: value.hand_count,
            hand_cards: value
                .hand_cards
                .iter()
                .map(PublicCombatHandCardV1::from)
                .collect(),
            draw_count: value.draw_count,
            discard_count: value.discard_count,
            exhaust_count: value.exhaust_count,
            alive_monster_count: value.alive_monster_count,
            dying_monster_count: value.dying_monster_count,
            half_dead_monster_count: value.half_dead_monster_count,
            zero_hp_monster_count: value.zero_hp_monster_count,
            pending_rebirth_monster_count: value.pending_rebirth_monster_count,
            total_monster_hp: value.total_monster_hp,
            visible_incoming_damage: value.visible_incoming_damage,
            pending_action_count: value.pending_action_count,
            queued_card_count: value.queued_card_count,
            limbo_count: value.limbo_count,
            pending_choice_kind: value.pending_choice_kind.clone(),
            pending_choice: value
                .pending_choice
                .as_ref()
                .map(PublicPendingChoiceV1::from),
            monsters: value.monsters.iter().map(PublicMonsterV1::from).collect(),
            encounter_hints: value.encounter_hints.clone(),
        }
    }
}

impl From<&RunMonsterObservationV0> for PublicMonsterV1 {
    fn from(value: &RunMonsterObservationV0) -> Self {
        Self {
            entity_id: value.entity_id,
            slot: value.slot,
            monster_id: value.monster_id.clone(),
            name: value.name.clone(),
            current_hp: value.current_hp,
            max_hp: value.max_hp,
            block: value.block,
            alive: value.alive,
            planned_move_id: value.planned_move_id,
            visible_intent: value.visible_intent.clone(),
            visible_intent_kind: value.visible_intent_kind.clone(),
            visible_intent_damage_per_hit: value.visible_intent_damage_per_hit,
            visible_intent_hits: value.visible_intent_hits,
            visible_intent_total_damage: value.visible_intent_total_damage,
            powers: value.powers.iter().map(PublicPowerV1::from).collect(),
            mechanic_hints: value.mechanic_hints.clone(),
        }
    }
}

impl From<&RunPowerObservationV0> for PublicPowerV1 {
    fn from(value: &RunPowerObservationV0) -> Self {
        Self {
            power_id: value.power_id.clone(),
            amount: value.amount,
        }
    }
}

impl From<&RunPendingChoiceObservationV0> for PublicPendingChoiceV1 {
    fn from(value: &RunPendingChoiceObservationV0) -> Self {
        Self {
            kind: value.kind.clone(),
            min_select: value.min_select,
            max_select: value.max_select,
            can_cancel: value.can_cancel,
            reason: value.reason.clone(),
            source_pile: value.source_pile.clone(),
            options: value
                .options
                .iter()
                .map(PublicPendingChoiceOptionV1::from)
                .collect(),
        }
    }
}

impl From<&RunPendingChoiceOptionObservationV0> for PublicPendingChoiceOptionV1 {
    fn from(value: &RunPendingChoiceOptionObservationV0) -> Self {
        Self {
            option_index: value.option_index,
            label: value.label.clone(),
            card_id: value.card_id.clone(),
            card_uuid: value.card_uuid,
            selection_uuids: value.selection_uuids.clone(),
            source_pile: value.source_pile.clone(),
            subject_ref: value.subject_ref.clone(),
            before_summary: value.before_summary.clone(),
            after_summary: value.after_summary.clone(),
            delta_summary: value.delta_summary.clone(),
            preview_status: value.preview_status.clone(),
        }
    }
}

impl From<&RunCombatHandCardObservationV0> for PublicCombatHandCardV1 {
    fn from(value: &RunCombatHandCardObservationV0) -> Self {
        Self {
            hand_index: value.hand_index,
            card_instance_id: value.card_instance_id,
            card_id: value.card_id.clone(),
            upgraded: value.upgraded,
            upgrades: value.upgrades,
            cost_for_turn: value.cost_for_turn,
            playable: value.playable,
            base_semantics: value.base_semantics.clone(),
            transient_tags: value.transient_tags.clone(),
        }
    }
}

impl From<&RunScreenObservationV0> for PublicScreenObservationV1 {
    fn from(value: &RunScreenObservationV0) -> Self {
        Self {
            event_option_count: value.event_option_count,
            event_options: value
                .event_options
                .iter()
                .map(|option| PublicEventOptionV1 {
                    option_index: option.option_index,
                    label: option.label.clone(),
                    disabled: option.disabled,
                    disabled_reason: option.disabled_reason.clone(),
                    semantic_descriptor: option.semantic_descriptor.clone(),
                })
                .collect(),
            reward_item_count: value.reward_item_count,
            reward_card_choice_count: value.reward_card_choice_count,
            reward_phase: value.reward_phase.clone(),
            reward_items: value
                .reward_items
                .iter()
                .map(PublicRewardItemV1::from)
                .collect(),
            reward_card_choices: value
                .reward_card_choices
                .iter()
                .map(PublicRewardCardChoiceV1::from)
                .collect(),
            reward_claimable_item_count: value.reward_claimable_item_count,
            reward_unclaimed_card_item_count: value.reward_unclaimed_card_item_count,
            shop_card_count: value.shop_card_count,
            shop_relic_count: value.shop_relic_count,
            shop_potion_count: value.shop_potion_count,
            boss_relic_choice_count: value.boss_relic_choice_count,
            selection_target_count: value.selection_target_count,
        }
    }
}

impl From<&RunRewardItemObservationV0> for PublicRewardItemV1 {
    fn from(value: &RunRewardItemObservationV0) -> Self {
        Self {
            item_index: value.item_index,
            item_type: value.item_type.clone(),
            amount: value.amount,
            card_choice_count: value.card_choice_count,
            relic_id: value.relic_id.clone(),
            potion_id: value.potion_id.clone(),
            claimable: value.claimable,
            opens_card_choice: value.opens_card_choice,
        }
    }
}

impl From<&RunRewardCardChoiceObservationV0> for PublicRewardCardChoiceV1 {
    fn from(value: &RunRewardCardChoiceObservationV0) -> Self {
        Self {
            option_index: value.option_index,
            card_id: value.card_id.clone(),
            card_name: value.card_name.clone(),
            upgrades: value.upgrades,
            card_type: value.card_type.clone(),
            rarity: value.rarity.clone(),
            cost: value.cost,
            base_semantics: value.base_semantics.clone(),
            deck_copies: value.deck_copies,
            card: PublicCardFeatureV1::from(&value.card),
            plan_delta: value.plan_delta.clone(),
            semantic_descriptor: value.semantic_descriptor.clone(),
        }
    }
}

impl From<&RunCardFeatureV0> for PublicCardFeatureV1 {
    fn from(value: &RunCardFeatureV0) -> Self {
        Self {
            card_id: value.card_id.clone(),
            card_id_hash: value.card_id_hash,
            card_type_id: value.card_type_id,
            rarity_id: value.rarity_id,
            cost: value.cost,
            upgrades: value.upgrades,
            base_damage: value.base_damage,
            base_block: value.base_block,
            base_magic: value.base_magic,
            upgraded_damage: value.upgraded_damage,
            upgraded_block: value.upgraded_block,
            upgraded_magic: value.upgraded_magic,
            exhaust: value.exhaust,
            ethereal: value.ethereal,
            innate: value.innate,
            aoe: value.aoe,
            multi_damage: value.multi_damage,
            starter_basic: value.starter_basic,
            draws_cards: value.draws_cards,
            gains_energy: value.gains_energy,
            applies_weak: value.applies_weak,
            applies_vulnerable: value.applies_vulnerable,
            scaling_piece: value.scaling_piece,
            deck_copies: value.deck_copies,
        }
    }
}
