use crate::content::cards::CardType;

use crate::ai::combat_search_v2::turn_planner::types::core::TurnPlanV1;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) struct TurnPlanCoverageKeyV1 {
    pub(in crate::ai::combat_search_v2) damage: TurnPlanDamageBandV1,
    pub(in crate::ai::combat_search_v2) block: TurnPlanBlockBandV1,
    pub(in crate::ai::combat_search_v2) debuff: TurnPlanDebuffClassV1,
    pub(in crate::ai::combat_search_v2) setup: TurnPlanSetupClassV1,
    pub(in crate::ai::combat_search_v2) resource: TurnPlanResourceClassV1,
    pub(in crate::ai::combat_search_v2) risk: TurnPlanRiskBandV1,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanDamageBandV1 {
    None,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanBlockBandV1 {
    None,
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanDebuffClassV1 {
    None,
    Weak,
    Vulnerable,
    StrengthDown,
    Mixed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanSetupClassV1 {
    None,
    PlayerStrength,
    AccessGain,
    ExhaustOrQueueChange,
    Mixed,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanResourceClassV1 {
    Neutral,
    SpendsEnergy,
    UsesPotion,
    GainsAccess,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::ai::combat_search_v2) enum TurnPlanRiskBandV1 {
    NoHpLoss,
    LowHpLoss,
    HighHpLoss,
    ForcedTurnEndOrReactiveLoss,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct TurnPlanCoverageSignatureV1 {
    pub(in crate::ai::combat_search_v2) action_count: usize,
    pub(in crate::ai::combat_search_v2) cards_played: usize,
    pub(in crate::ai::combat_search_v2) attacks_played: usize,
    pub(in crate::ai::combat_search_v2) skills_played: usize,
    pub(in crate::ai::combat_search_v2) powers_played: usize,
    pub(in crate::ai::combat_search_v2) potions_used: usize,
    pub(in crate::ai::combat_search_v2) damage_done: i32,
    pub(in crate::ai::combat_search_v2) block_gained_proxy: i32,
    pub(in crate::ai::combat_search_v2) enemy_vulnerable_added: i32,
    pub(in crate::ai::combat_search_v2) enemy_weak_added: i32,
    pub(in crate::ai::combat_search_v2) enemy_strength_down_added: i32,
    pub(in crate::ai::combat_search_v2) player_strength_gain: i32,
    pub(in crate::ai::combat_search_v2) player_temporary_strength_gain: i32,
    pub(in crate::ai::combat_search_v2) energy_spent_proxy: i32,
    pub(in crate::ai::combat_search_v2) hand_delta: i32,
    pub(in crate::ai::combat_search_v2) draw_delta: i32,
    pub(in crate::ai::combat_search_v2) discard_delta: i32,
    pub(in crate::ai::combat_search_v2) exhaust_delta: i32,
    pub(in crate::ai::combat_search_v2) queued_cards_delta: i32,
    pub(in crate::ai::combat_search_v2) player_hp_lost: i32,
    pub(in crate::ai::combat_search_v2) reactive_player_hp_loss: i32,
    pub(in crate::ai::combat_search_v2) reactive_forced_turn_end_actions: usize,
    pub(in crate::ai::combat_search_v2) pending_choice_steps: usize,
}

impl TurnPlanDamageBandV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl TurnPlanBlockBandV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

impl TurnPlanDebuffClassV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Weak => "weak",
            Self::Vulnerable => "vulnerable",
            Self::StrengthDown => "strength_down",
            Self::Mixed => "mixed",
        }
    }
}

impl TurnPlanSetupClassV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::PlayerStrength => "player_strength",
            Self::AccessGain => "access_gain",
            Self::ExhaustOrQueueChange => "exhaust_or_queue_change",
            Self::Mixed => "mixed",
        }
    }
}

impl TurnPlanResourceClassV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::SpendsEnergy => "spends_energy",
            Self::UsesPotion => "uses_potion",
            Self::GainsAccess => "gains_access",
        }
    }
}

impl TurnPlanRiskBandV1 {
    pub(in crate::ai::combat_search_v2) fn label(self) -> &'static str {
        match self {
            Self::NoHpLoss => "no_hp_loss",
            Self::LowHpLoss => "low_hp_loss",
            Self::HighHpLoss => "high_hp_loss",
            Self::ForcedTurnEndOrReactiveLoss => "forced_turn_end_or_reactive_loss",
        }
    }
}

impl TurnPlanCoverageSignatureV1 {
    pub(in crate::ai::combat_search_v2) fn from_plan(plan: &TurnPlanV1) -> Self {
        let mut signature = Self {
            action_count: plan.actions.len(),
            ..Self::default()
        };
        for action in &plan.actions {
            match action.input {
                crate::state::core::ClientInput::PlayCard { .. } => {
                    signature.cards_played = signature.cards_played.saturating_add(1);
                }
                crate::state::core::ClientInput::UsePotion { .. } => {
                    signature.potions_used = signature.potions_used.saturating_add(1);
                }
                _ => {}
            }
        }
        for facts in &plan.action_facts {
            if let Some(card) = facts.card.as_ref() {
                match card.card_type {
                    CardType::Attack => {
                        signature.attacks_played = signature.attacks_played.saturating_add(1);
                    }
                    CardType::Skill => {
                        signature.skills_played = signature.skills_played.saturating_add(1);
                    }
                    CardType::Power => {
                        signature.powers_played = signature.powers_played.saturating_add(1);
                    }
                    CardType::Status | CardType::Curse => {}
                }
            }
            let exact = &facts.exact_one_step_delta;
            signature.damage_done = signature
                .damage_done
                .saturating_add((-exact.total_enemy_hp_delta).max(0));
            signature.block_gained_proxy = signature
                .block_gained_proxy
                .saturating_add(exact.player_block_delta.max(0));
            signature.energy_spent_proxy = signature
                .energy_spent_proxy
                .saturating_add((-exact.energy_delta).max(0));
            signature.hand_delta = signature.hand_delta.saturating_add(exact.hand_delta);
            signature.draw_delta = signature.draw_delta.saturating_add(exact.draw_delta);
            signature.discard_delta = signature.discard_delta.saturating_add(exact.discard_delta);
            signature.exhaust_delta = signature.exhaust_delta.saturating_add(exact.exhaust_delta);
            signature.queued_cards_delta = signature
                .queued_cards_delta
                .saturating_add(exact.queued_cards_delta);
            signature.player_hp_lost = signature
                .player_hp_lost
                .saturating_add((-exact.player_hp_delta).max(0));
            if exact.pending_choice_present {
                signature.pending_choice_steps = signature.pending_choice_steps.saturating_add(1);
            }

            let mechanics = &facts.mechanics;
            signature.enemy_vulnerable_added = signature
                .enemy_vulnerable_added
                .saturating_add(mechanics.derived.enemy_vulnerable);
            signature.enemy_weak_added = signature
                .enemy_weak_added
                .saturating_add(mechanics.derived.enemy_weak);
            signature.enemy_strength_down_added = signature
                .enemy_strength_down_added
                .saturating_add(mechanics.direct.persistent_enemy_strength_down)
                .saturating_add(mechanics.direct.temporary_enemy_strength_down);
            signature.player_strength_gain = signature
                .player_strength_gain
                .saturating_add(mechanics.direct.player_strength_gain);
            signature.player_temporary_strength_gain = signature
                .player_temporary_strength_gain
                .saturating_add(mechanics.direct.player_temporary_strength_gain);
            signature.reactive_player_hp_loss = signature
                .reactive_player_hp_loss
                .saturating_add(mechanics.reactive.player_hp_loss);
            if mechanics.reactive.forced_turn_end {
                signature.reactive_forced_turn_end_actions =
                    signature.reactive_forced_turn_end_actions.saturating_add(1);
            }
        }
        signature
    }

    pub(in crate::ai::combat_search_v2) fn coverage_key(self) -> TurnPlanCoverageKeyV1 {
        TurnPlanCoverageKeyV1 {
            damage: damage_band(self.damage_done),
            block: block_band(self.block_gained_proxy),
            debuff: debuff_class(self),
            setup: setup_class(self),
            resource: resource_class(self),
            risk: risk_band(self),
        }
    }
}

fn damage_band(value: i32) -> TurnPlanDamageBandV1 {
    match value {
        i32::MIN..=0 => TurnPlanDamageBandV1::None,
        1..=7 => TurnPlanDamageBandV1::Low,
        8..=17 => TurnPlanDamageBandV1::Medium,
        _ => TurnPlanDamageBandV1::High,
    }
}

fn block_band(value: i32) -> TurnPlanBlockBandV1 {
    match value {
        i32::MIN..=0 => TurnPlanBlockBandV1::None,
        1..=5 => TurnPlanBlockBandV1::Low,
        6..=12 => TurnPlanBlockBandV1::Medium,
        _ => TurnPlanBlockBandV1::High,
    }
}

fn debuff_class(signature: TurnPlanCoverageSignatureV1) -> TurnPlanDebuffClassV1 {
    let weak = signature.enemy_weak_added > 0;
    let vulnerable = signature.enemy_vulnerable_added > 0;
    let strength_down = signature.enemy_strength_down_added > 0;
    match (weak, vulnerable, strength_down) {
        (false, false, false) => TurnPlanDebuffClassV1::None,
        (true, false, false) => TurnPlanDebuffClassV1::Weak,
        (false, true, false) => TurnPlanDebuffClassV1::Vulnerable,
        (false, false, true) => TurnPlanDebuffClassV1::StrengthDown,
        _ => TurnPlanDebuffClassV1::Mixed,
    }
}

fn setup_class(signature: TurnPlanCoverageSignatureV1) -> TurnPlanSetupClassV1 {
    let strength =
        signature.player_strength_gain > 0 || signature.player_temporary_strength_gain > 0;
    let access = signature.hand_delta > 0 || signature.draw_delta < 0;
    let shape = signature.exhaust_delta != 0 || signature.queued_cards_delta != 0;
    match (strength, access, shape) {
        (false, false, false) => TurnPlanSetupClassV1::None,
        (true, false, false) => TurnPlanSetupClassV1::PlayerStrength,
        (false, true, false) => TurnPlanSetupClassV1::AccessGain,
        (false, false, true) => TurnPlanSetupClassV1::ExhaustOrQueueChange,
        _ => TurnPlanSetupClassV1::Mixed,
    }
}

fn resource_class(signature: TurnPlanCoverageSignatureV1) -> TurnPlanResourceClassV1 {
    if signature.potions_used > 0 {
        return TurnPlanResourceClassV1::UsesPotion;
    }
    if signature.hand_delta > 0 || signature.draw_delta < 0 {
        return TurnPlanResourceClassV1::GainsAccess;
    }
    if signature.energy_spent_proxy > 0 {
        return TurnPlanResourceClassV1::SpendsEnergy;
    }
    TurnPlanResourceClassV1::Neutral
}

fn risk_band(signature: TurnPlanCoverageSignatureV1) -> TurnPlanRiskBandV1 {
    if signature.reactive_forced_turn_end_actions > 0 || signature.reactive_player_hp_loss > 0 {
        return TurnPlanRiskBandV1::ForcedTurnEndOrReactiveLoss;
    }
    match signature.player_hp_lost {
        i32::MIN..=0 => TurnPlanRiskBandV1::NoHpLoss,
        1..=6 => TurnPlanRiskBandV1::LowHpLoss,
        _ => TurnPlanRiskBandV1::HighHpLoss,
    }
}
