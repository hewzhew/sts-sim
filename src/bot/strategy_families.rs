use crate::content::cards::{CardId, CardType};

const SURVIVAL_HEAL_POINT_SCORE: i32 = 1_200;
const SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE: i32 = 220;
const SURVIVAL_KILL_SCORE: i32 = 1_600;
const SURVIVAL_LOW_HP_HEAL_BONUS: i32 = 4_000;
const SURVIVAL_LETHALISH_COVER_BONUS: i32 = 5_000;
const SURVIVAL_STABILIZE_BONUS: i32 = 2_500;
const SURVIVAL_EXACT_STABILIZE_BONUS: i32 = 6_500;
const SURVIVAL_LETHAL_WINDOW_BONUS: i32 = 10_000;
const SURVIVAL_MASSIVE_WINDOW_BONUS: i32 = 6_000;
const SURVIVAL_PARTIAL_LETHAL_SAVE_BONUS: i32 = 7_500;
const SURVIVAL_KILL_IN_LETHAL_WINDOW_BONUS: i32 = 2_500;

const PLAY_NOW_KEEP_PENALTY: i32 = -1_500;
const DELAY_TO_TOPDECK_BONUS: i32 = 1_500;

const APOTHEOSIS_BASE_SCORE: i32 = 8_500;
const APOTHEOSIS_UPGRADE_TARGET_SCORE: i32 = 1_400;
const APOTHEOSIS_NO_TARGETS_PENALTY: i32 = 6_000;
const APOTHEOSIS_MULTI_TARGET_BONUS: i32 = 4_000;
const APOTHEOSIS_UNDER_PRESSURE_PENALTY: i32 = 1_500;

const APPARITION_EXISTING_INTANGIBLE_PENALTY: i32 = 6_000;
const APPARITION_INTANGIBLE_STACK_PENALTY: i32 = 2_000;
const APPARITION_UNUPGRADED_OVERLAP_RELIEF: i32 = 2_000;
const APPARITION_PYRAMID_OVERLAP_PENALTY: i32 = 1_000;
const APPARITION_LOW_HP_OVERLAP_RELIEF: i32 = 1_500;
const APPARITION_HAND_FLOOD_RELIEF: i32 = 1_500;
const APPARITION_RESERVE_FLOOD_RELIEF: i32 = 800;
const APPARITION_PRESSURE_POINT_SCORE: i32 = 180;
const APPARITION_INCOMING_OVERLAP_RELIEF: i32 = 1_500;
const APPARITION_LETHAL_OVERLAP_RELIEF: i32 = 4_000;
const APPARITION_MASSIVE_OVERLAP_RELIEF: i32 = 3_000;
const APPARITION_UNUPGRADED_BASE_SCORE: i32 = 5_500;
const APPARITION_UNUPGRADED_THREAT_BONUS: i32 = 5_000;
const APPARITION_UNUPGRADED_LETHAL_BONUS: i32 = 8_000;
const APPARITION_UNUPGRADED_EXTRA_COPY_BONUS: i32 = 1_500;
const APPARITION_UNUPGRADED_FRONTLOAD_BONUS: i32 = 2_500;
const APPARITION_UNUPGRADED_FRONTLOAD_PRESSURE_SCORE: i32 = 120;
const APPARITION_UPGRADED_ACTIVE_BASE_SCORE: i32 = 8_500;
const APPARITION_UPGRADED_SAFE_PYRAMID_PENALTY: i32 = 3_000;
const APPARITION_UPGRADED_SAFE_DELAY_PENALTY: i32 = 1_500;
const APPARITION_UPGRADED_LETHAL_BONUS: i32 = 10_000;
const APPARITION_UPGRADED_MASSIVE_BONUS: i32 = 8_000;
const APPARITION_UPGRADED_PYRAMID_THREAT_BONUS: i32 = 2_500;
const APPARITION_UPGRADED_FRONTLOAD_BONUS: i32 = 2_000;
const APPARITION_UPGRADED_FRONTLOAD_PRESSURE_SCORE: i32 = 110;
const APPARITION_HAND_SHAPING_TOPDECK_BONUS: i32 = 2_500;
const APPARITION_HAND_SHAPING_EXTRA_COPY_SCORE: i32 = 500;
const APPARITION_HAND_SHAPING_TIMING_DIVISOR: i32 = 4;
const APPARITION_UPGRADED_HAND_SHAPING_TIMING_DIVISOR: i32 = 5;
const APPARITION_HAND_SHAPING_MIN_KEEP_PENALTY: i32 = 2_000;
const APPARITION_UPGRADED_HAND_SHAPING_MIN_KEEP_PENALTY: i32 = 1_500;
const APPARITION_UPGRADED_PYRAMID_HOLD_PENALTY: i32 = 2_400;
const APPARITION_UPGRADED_GENERIC_HOLD_PENALTY: i32 = 800;

const DRAW_BLOCKED_BY_NO_DRAW_PENALTY: i32 = 14_000;
const DRAW_BASE_SCORE: i32 = 2_800;
const DRAW_CARD_SCORE: i32 = 1_200;
const DRAW_HAND_OVERFLOW_PENALTY: i32 = 1_100;
const DRAW_NO_DRAW_SELF_PENALTY: i32 = 1_200;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct MassExhaustProfile {
    pub exhausted_count: i32,
    pub total_fuel: i32,
    pub remaining_cards_after: i32,
    pub remaining_low_value_fuel_after: i32,
    pub closeout_bonus: i32,
    pub junk_fuel_count: i32,
    pub protected_piece_count: i32,
    pub core_piece_count: i32,
    pub engine_support_level: i32,
    pub block_per_exhaust: i32,
    pub imminent_unblocked_damage: i32,
    pub playable_block_lost: i32,
    pub exact_stabilize: bool,
    pub low_pressure_high_hp: bool,
    pub dark_embrace_draw_count: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SurvivalTimingContext {
    pub current_hp: i32,
    pub imminent_unblocked_damage: i32,
    pub missing_hp: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ApparitionTimingContext {
    pub current_hp: i32,
    pub current_intangible: i32,
    pub imminent_unblocked_damage: i32,
    pub total_incoming_damage: i32,
    pub apparitions_in_hand: i32,
    pub remaining_apparitions_total: i32,
    pub upgraded: bool,
    pub has_runic_pyramid: bool,
    pub encounter_pressure: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DrawTimingContext {
    pub current_energy: i32,
    pub player_no_draw: bool,
    pub current_hand_size: i32,
    pub future_zero_cost_cards: i32,
    pub future_one_cost_cards: i32,
    pub future_two_plus_cost_cards: i32,
    pub future_key_delay_weight: i32,
    pub future_high_cost_key_delay_weight: i32,
    pub future_status_cards: i32,
    pub other_draw_sources_in_hand: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TurnActionRole {
    Setup,
    Payoff,
    Cycling,
    EnergyBridge,
    DefensiveBridge,
    Utility,
    Finisher,
    #[default]
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum TurnOrderingHint {
    PreferEarly,
    PreferLate,
    #[default]
    OrderFlexible,
    OrderConditional,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ChanceProfile {
    #[default]
    Deterministic,
    DrawBranch,
    RandomGeneration,
    TargetSensitive,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RiskProfile {
    #[default]
    Safe,
    WindowSensitive,
    DownsideSensitive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OrderingConstraint {
    SetupBeforePayoff,
    CyclingBeforeTerminalAttack,
    EnergyBridgeBeforeHighCostPayoff,
    DebuffBeforeMultiHitPayoff,
    FinisherAfterGrowthCheck,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BranchFamily {
    Draw,
    EnergyPlusDraw,
    RandomCombatCard,
    RandomAttackCard,
    #[default]
    UnknownRandom,
}

impl BranchFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draw => "draw",
            Self::EnergyPlusDraw => "energy_plus_draw",
            Self::RandomCombatCard => "random_combat_card",
            Self::RandomAttackCard => "random_attack_card",
            Self::UnknownRandom => "unknown_random",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BranchOpeningContext {
    pub draw: DrawTimingContext,
    pub risk: TurnRiskContext,
    pub draw_count: i32,
    pub applies_no_draw: bool,
    pub current_safe_line_exists: bool,
    pub current_defensive_floor: i32,
    pub energy_after_play: i32,
    pub hand_space_after_play: i32,
    pub immediate_action_value: i32,
    pub remaining_attack_followups: i32,
    pub remaining_defensive_followups: i32,
    pub branch_family: BranchFamily,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BranchOpeningEstimate {
    pub playable_hit_prob: f32,
    pub high_value_hit_prob: f32,
    pub dead_draw_prob: f32,
    pub followup_energy_feasible: f32,
    pub defensive_hit_prob: f32,
    pub stabilizing_hit_prob: f32,
    pub overdraw_risk: f32,
    pub energy_strand_prob: f32,
    pub bad_branch_lethal_risk: f32,
    pub good_branch_escape_value: i32,
    pub branch_family: BranchFamily,
    pub continuation_value: i32,
    pub downside_value: i32,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TurnRiskContext {
    pub current_hp: i32,
    pub unblocked_damage: i32,
    pub defense_gap: i32,
    pub lethal_pressure: bool,
    pub urgent_pressure: bool,
    pub current_energy: i32,
    pub remaining_actions: i32,
    pub has_safe_line: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TurnSequencingContext {
    pub role: TurnActionRole,
    pub ordering_hint: TurnOrderingHint,
    pub chance_profile: ChanceProfile,
    pub risk_profile: RiskProfile,
    pub ordering_constraint: Option<OrderingConstraint>,
    pub immediate_payoff: i32,
    pub followup_payoff: i32,
    pub growth_window: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SequencingAssessment {
    pub frontload_bonus: i32,
    pub defer_bonus: i32,
    pub branch_value: i32,
    pub downside_penalty: i32,
    pub rationale_key: &'static str,
    pub branch_rationale_key: &'static str,
    pub downside_rationale_key: &'static str,
}

impl SequencingAssessment {
    pub(crate) const fn total_delta(self) -> i32 {
        self.frontload_bonus + self.branch_value - self.defer_bonus - self.downside_penalty
    }
}

pub(crate) fn classify_turn_action(card_id: CardId, card_type: CardType) -> TurnActionRole {
    match card_id {
        CardId::Feed | CardId::Reaper => TurnActionRole::Finisher,
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => TurnActionRole::EnergyBridge,
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath
        | CardId::FlashOfSteel
        | CardId::Finesse
        | CardId::GoodInstincts => TurnActionRole::Cycling,
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::Disarm
        | CardId::Clothesline
        | CardId::Intimidate
        | CardId::SpotWeakness => TurnActionRole::Utility,
        CardId::Rage | CardId::Flex | CardId::Inflame | CardId::DemonForm => TurnActionRole::Setup,
        _ => match card_type {
            CardType::Power => TurnActionRole::Setup,
            CardType::Attack => TurnActionRole::Payoff,
            CardType::Skill => TurnActionRole::DefensiveBridge,
            _ => TurnActionRole::Other,
        },
    }
}

pub(crate) fn default_ordering_hint(card_id: CardId, role: TurnActionRole) -> TurnOrderingHint {
    match role {
        TurnActionRole::Setup | TurnActionRole::Cycling | TurnActionRole::EnergyBridge => {
            TurnOrderingHint::PreferEarly
        }
        TurnActionRole::DefensiveBridge => match card_id {
            CardId::Impervious
            | CardId::FlameBarrier
            | CardId::PowerThrough
            | CardId::PanicButton
            | CardId::GhostlyArmor => TurnOrderingHint::PreferEarly,
            _ => TurnOrderingHint::OrderConditional,
        },
        TurnActionRole::Payoff | TurnActionRole::Finisher => TurnOrderingHint::PreferLate,
        TurnActionRole::Utility => match card_id {
            CardId::Bash
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::ThunderClap
            | CardId::Trip
            | CardId::SpotWeakness => TurnOrderingHint::PreferEarly,
            _ => TurnOrderingHint::OrderConditional,
        },
        TurnActionRole::Other => TurnOrderingHint::OrderFlexible,
    }
}

pub(crate) fn default_chance_profile(card_id: CardId) -> ChanceProfile {
    match card_id {
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath
        | CardId::FlashOfSteel
        | CardId::Finesse
        | CardId::GoodInstincts
        | CardId::Offering => ChanceProfile::DrawBranch,
        CardId::InfernalBlade | CardId::Discovery | CardId::Magnetism | CardId::Mayhem => {
            ChanceProfile::RandomGeneration
        }
        CardId::Bash
        | CardId::Uppercut
        | CardId::Clothesline
        | CardId::Blind
        | CardId::DarkShackles
        | CardId::Disarm
        | CardId::Trip
        | CardId::Feed
        | CardId::HeavyBlade
        | CardId::SpotWeakness => ChanceProfile::TargetSensitive,
        _ => ChanceProfile::Deterministic,
    }
}

pub fn branch_family_for_card(card_id: CardId) -> Option<BranchFamily> {
    match card_id {
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath
        | CardId::FlashOfSteel
        | CardId::Finesse
        | CardId::GoodInstincts => Some(BranchFamily::Draw),
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => {
            Some(BranchFamily::EnergyPlusDraw)
        }
        CardId::Discovery | CardId::Magnetism | CardId::Mayhem => {
            Some(BranchFamily::RandomCombatCard)
        }
        CardId::InfernalBlade | CardId::SecretWeapon => Some(BranchFamily::RandomAttackCard),
        _ => None,
    }
}

pub(crate) fn default_risk_profile(card_id: CardId, role: TurnActionRole) -> RiskProfile {
    match role {
        TurnActionRole::Cycling | TurnActionRole::EnergyBridge => RiskProfile::DownsideSensitive,
        TurnActionRole::Utility => match card_id {
            CardId::Bash | CardId::Shockwave | CardId::Uppercut | CardId::Trip => {
                RiskProfile::WindowSensitive
            }
            _ => RiskProfile::Safe,
        },
        TurnActionRole::Payoff | TurnActionRole::Finisher => RiskProfile::WindowSensitive,
        _ => RiskProfile::Safe,
    }
}

pub(crate) fn default_ordering_constraint(card_id: CardId) -> Option<OrderingConstraint> {
    match card_id {
        CardId::Inflame
        | CardId::Flex
        | CardId::DemonForm
        | CardId::Rage
        | CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace => Some(OrderingConstraint::SetupBeforePayoff),
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Warcry
        | CardId::BattleTrance
        | CardId::BurningPact
        | CardId::MasterOfStrategy
        | CardId::DeepBreath => Some(OrderingConstraint::CyclingBeforeTerminalAttack),
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting => {
            Some(OrderingConstraint::EnergyBridgeBeforeHighCostPayoff)
        }
        CardId::Bash
        | CardId::Shockwave
        | CardId::Uppercut
        | CardId::ThunderClap
        | CardId::Trip => Some(OrderingConstraint::DebuffBeforeMultiHitPayoff),
        CardId::Feed | CardId::Reaper => Some(OrderingConstraint::FinisherAfterGrowthCheck),
        _ => None,
    }
}

pub(crate) fn assess_branch_opening(ctx: &BranchOpeningContext) -> BranchOpeningEstimate {
    let draw = &ctx.draw;
    let risk = &ctx.risk;
    let total_future = (draw.future_zero_cost_cards
        + draw.future_one_cost_cards
        + draw.future_two_plus_cost_cards
        + draw.future_status_cards)
        .max(1) as f32;
    let effective_draws = ctx.draw_count.max(0) as f32;
    let playable_mass = draw.future_zero_cost_cards as f32
        + draw.future_one_cost_cards as f32
        + if ctx.energy_after_play > 0 {
            (draw.future_two_plus_cost_cards as f32 * 0.5).max(0.0)
        } else {
            0.0
        };
    let high_value_mass = (draw.future_key_delay_weight + draw.future_high_cost_key_delay_weight)
        .max(0) as f32
        + (draw.future_two_plus_cost_cards.max(0) as f32 * 0.75);
    let status_mass = draw.future_status_cards.max(0) as f32;
    let defensive_mass = ctx.remaining_defensive_followups.max(0) as f32
        + (draw.future_zero_cost_cards.max(0) as f32 * 0.15)
        + (draw.future_one_cost_cards.max(0) as f32 * 0.20);
    let stabilizing_mass = defensive_mass + ctx.remaining_attack_followups.max(0) as f32 * 0.35;
    let hand_flood_tax = (draw.current_hand_size - 8).max(0) as f32 * 0.06;
    let player_no_draw_tax = if draw.player_no_draw || ctx.applies_no_draw {
        0.35
    } else {
        0.0
    };
    let playable_hit_prob =
        ((effective_draws * playable_mass / total_future / effective_draws.max(1.0))
            - hand_flood_tax)
            .clamp(0.0, 1.0);
    let high_value_hit_prob = ((effective_draws * high_value_mass / (total_future * 4.0))
        - hand_flood_tax * 0.5)
        .clamp(0.0, 1.0);
    let defensive_hit_prob =
        ((effective_draws * defensive_mass / total_future / effective_draws.max(1.0))
            - hand_flood_tax * 0.25)
            .clamp(0.0, 1.0);
    let stabilizing_hit_prob =
        ((effective_draws * stabilizing_mass / total_future / effective_draws.max(1.0))
            - hand_flood_tax * 0.15)
            .clamp(0.0, 1.0);
    let dead_draw_prob =
        ((effective_draws * status_mass / total_future / effective_draws.max(1.0))
            + player_no_draw_tax
            + if ctx.energy_after_play <= 0 && draw.future_two_plus_cost_cards > 0 {
                0.18
            } else {
                0.0
            })
        .clamp(0.0, 1.0);
    let followup_energy_feasible = (if ctx.energy_after_play > 0 {
        0.45
    } else {
        0.15
    } + draw.future_zero_cost_cards as f32 * 0.05
        + draw.future_one_cost_cards as f32 * 0.03
        - draw.future_two_plus_cost_cards as f32 * 0.04)
        .clamp(0.0, 1.0);
    let overdraw_risk = (((ctx.draw_count - ctx.hand_space_after_play).max(0) as f32)
        / ctx.draw_count.max(1) as f32
        + hand_flood_tax * 0.8)
        .clamp(0.0, 1.0);
    let energy_strand_prob = ((draw.future_two_plus_cost_cards.max(0) as f32 / total_future)
        * if ctx.energy_after_play <= 0 {
            1.0
        } else if ctx.energy_after_play == 1 {
            0.55
        } else {
            0.2
        })
    .clamp(0.0, 1.0);
    let defense_break_risk = if ctx.current_safe_line_exists
        && ctx.current_defensive_floor < risk.defense_gap
        && defensive_hit_prob < 0.45
    {
        0.35 + dead_draw_prob * 0.25
    } else {
        0.0
    };
    let defensive_floor_relief = if risk.defense_gap > 0 {
        (ctx.current_defensive_floor.min(risk.defense_gap).max(0) as f32
            / risk.defense_gap.max(1) as f32)
            * 0.30
    } else {
        0.0
    };
    let mut bad_branch_lethal_risk = (defense_break_risk
        + dead_draw_prob
            * if ctx.current_safe_line_exists {
                0.30
            } else {
                0.18
            }
        + energy_strand_prob * 0.22
        + overdraw_risk * 0.18
        + if risk.lethal_pressure { 0.22 } else { 0.0 }
        - defensive_floor_relief)
        .clamp(0.0, 1.0);
    if matches!(
        ctx.branch_family,
        BranchFamily::RandomCombatCard
            | BranchFamily::RandomAttackCard
            | BranchFamily::UnknownRandom
    ) {
        bad_branch_lethal_risk = (bad_branch_lethal_risk + 0.10).clamp(0.0, 1.0);
    }
    let good_branch_escape_value = if !ctx.current_safe_line_exists || risk.lethal_pressure {
        ((stabilizing_hit_prob * 6_000.0)
            + (defensive_hit_prob * 4_200.0)
            + (playable_hit_prob * 2_200.0)
            + (ctx.immediate_action_value.max(0) as f32 * 60.0)) as i32
    } else {
        0
    };

    let continuation_value = ((playable_hit_prob * 5_000.0)
        + (high_value_hit_prob * 8_000.0)
        + (defensive_hit_prob * 3_000.0)
        + (stabilizing_hit_prob * 4_200.0)
        + (followup_energy_feasible * 3_500.0)
        + good_branch_escape_value as f32
        + draw.future_key_delay_weight as f32 * 220.0
        + draw.future_high_cost_key_delay_weight as f32 * 160.0
        - draw.other_draw_sources_in_hand as f32 * 500.0
        + if matches!(ctx.branch_family, BranchFamily::EnergyPlusDraw) {
            1_800.0
        } else {
            0.0
        }) as i32;
    let downside_value = ((dead_draw_prob * 5_500.0)
        + if ctx.current_safe_line_exists {
            (risk.defense_gap - ctx.current_defensive_floor).max(0) as f32 * 260.0
        } else {
            0.0
        }
        + (overdraw_risk * 2_400.0)
        + (energy_strand_prob * 2_800.0)
        + (bad_branch_lethal_risk * 6_200.0)
        + if matches!(ctx.branch_family, BranchFamily::EnergyPlusDraw) && risk.current_hp <= 18 {
            1_800.0
        } else {
            0.0
        }
        + if risk.lethal_pressure { 4_500.0 } else { 0.0 }) as i32;

    BranchOpeningEstimate {
        playable_hit_prob,
        high_value_hit_prob,
        dead_draw_prob,
        followup_energy_feasible,
        defensive_hit_prob,
        stabilizing_hit_prob,
        overdraw_risk,
        energy_strand_prob,
        bad_branch_lethal_risk,
        good_branch_escape_value,
        branch_family: ctx.branch_family,
        continuation_value,
        downside_value,
    }
}

pub(crate) fn assess_turn_action(
    ctx: &TurnSequencingContext,
    risk: &TurnRiskContext,
    branch: Option<&BranchOpeningEstimate>,
) -> SequencingAssessment {
    let mut assessment = SequencingAssessment::default();

    match ctx.ordering_constraint {
        Some(OrderingConstraint::SetupBeforePayoff) if ctx.followup_payoff > 0 => {
            assessment.frontload_bonus += 8_500 + ctx.followup_payoff.min(40) * 200;
            assessment.rationale_key = "setup_before_payoff";
        }
        Some(OrderingConstraint::CyclingBeforeTerminalAttack) if branch.is_some() => {
            assessment.frontload_bonus += 2_400;
            assessment.rationale_key = "cycling_before_terminal_attack";
        }
        Some(OrderingConstraint::EnergyBridgeBeforeHighCostPayoff) if ctx.followup_payoff > 0 => {
            assessment.frontload_bonus += 5_500 + ctx.followup_payoff.min(35) * 130;
            assessment.rationale_key = "energy_bridge_before_high_cost_payoff";
        }
        Some(OrderingConstraint::DebuffBeforeMultiHitPayoff) if ctx.followup_payoff > 0 => {
            assessment.frontload_bonus += 3_800 + ctx.followup_payoff.min(30) * 110;
            assessment.rationale_key = "debuff_before_multi_hit_payoff";
        }
        Some(OrderingConstraint::FinisherAfterGrowthCheck) if ctx.growth_window => {
            assessment.frontload_bonus += 8_500 + ctx.immediate_payoff.min(20) * 160;
            assessment.rationale_key = "finisher_after_growth_check";
        }
        _ => {}
    }

    match ctx.ordering_hint {
        TurnOrderingHint::PreferEarly => {
            assessment.frontload_bonus += 1_000;
            if assessment.rationale_key.is_empty() {
                assessment.rationale_key = "prefer_early";
            }
        }
        TurnOrderingHint::PreferLate if ctx.followup_payoff > 0 => {
            assessment.defer_bonus += 3_200 + ctx.followup_payoff.min(25) * 110;
            if assessment.rationale_key.is_empty() {
                assessment.rationale_key = "prefer_late";
            }
        }
        TurnOrderingHint::OrderConditional if risk.urgent_pressure => {
            assessment.defer_bonus += 600;
            if assessment.rationale_key.is_empty() {
                assessment.rationale_key = "order_conditional";
            }
        }
        _ => {}
    }

    if let Some(branch) = branch {
        assessment.branch_value += branch.continuation_value;
        assessment.downside_penalty += branch.downside_value;
        assessment.branch_value += (branch.playable_hit_prob * 1_200.0) as i32;
        assessment.branch_value += (branch.followup_energy_feasible * 900.0) as i32;
        assessment.branch_value += (branch.defensive_hit_prob * 1_100.0) as i32;
        assessment.branch_value += (branch.stabilizing_hit_prob * 1_400.0) as i32;
        assessment.branch_value += (branch.good_branch_escape_value as f32 * 0.35) as i32;
        if matches!(
            branch.branch_family,
            BranchFamily::RandomCombatCard | BranchFamily::RandomAttackCard
        ) {
            assessment.downside_penalty += 400;
        }
        if branch.high_value_hit_prob >= 0.45 && branch.dead_draw_prob <= 0.35 {
            assessment.frontload_bonus += 1_400;
            assessment.branch_rationale_key = "branch_opening_positive_ev";
            if assessment.rationale_key.is_empty() {
                assessment.rationale_key = "branch_opening_positive_ev";
            }
        }
        if risk.has_safe_line && branch.bad_branch_lethal_risk >= 0.40 {
            assessment.downside_penalty += 3_200;
            assessment.downside_rationale_key = "defense_break_downside";
            assessment.rationale_key = "branch_opening_safety_guard";
        } else if branch.energy_strand_prob >= 0.40 {
            assessment.downside_penalty += 1_800;
            assessment.downside_rationale_key = "energy_strand_downside";
        } else if branch.overdraw_risk >= 0.35 {
            assessment.downside_penalty += 1_500;
            assessment.downside_rationale_key = "overdraw_downside";
        } else if risk.has_safe_line && branch.dead_draw_prob >= 0.45 {
            assessment.downside_penalty += 2_200;
            assessment.downside_rationale_key = "branch_opening_safety_guard";
            assessment.rationale_key = "branch_opening_safety_guard";
        }
    }

    if matches!(ctx.risk_profile, RiskProfile::DownsideSensitive) && risk.has_safe_line {
        assessment.downside_penalty += risk.defense_gap.max(0) * 140;
    }
    if risk.current_hp <= risk.unblocked_damage.max(0) + 8 {
        assessment.downside_penalty += 900;
    }
    if risk.remaining_actions <= 1
        && matches!(
            ctx.role,
            TurnActionRole::Cycling | TurnActionRole::EnergyBridge
        )
    {
        assessment.defer_bonus += 700;
    }
    if risk.current_energy <= 0
        && matches!(
            ctx.role,
            TurnActionRole::Cycling | TurnActionRole::EnergyBridge
        )
    {
        assessment.downside_penalty += 350;
    }
    if matches!(ctx.risk_profile, RiskProfile::WindowSensitive) && !risk.urgent_pressure {
        assessment.defer_bonus += 400;
    }
    if matches!(ctx.chance_profile, ChanceProfile::RandomGeneration) && risk.has_safe_line {
        assessment.downside_penalty += 1_100;
    }
    if risk.lethal_pressure && matches!(ctx.role, TurnActionRole::Payoff) && ctx.followup_payoff > 0
    {
        assessment.defer_bonus += 2_000;
        if assessment.rationale_key.is_empty() {
            assessment.rationale_key = "premature_payoff_penalty";
        }
    }
    if !risk.urgent_pressure
        && matches!(ctx.role, TurnActionRole::Payoff)
        && ctx.followup_payoff >= ctx.immediate_payoff
    {
        assessment.defer_bonus += 1_600;
        if assessment.rationale_key.is_empty() {
            assessment.rationale_key = "payoff_after_setup_window";
        }
    }

    assessment
}

pub(crate) fn survival_swing_score(
    ctx: &SurvivalTimingContext,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    let effective_heal = hp_gain.min(ctx.missing_hp.max(0));
    let lethalish = ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_sub(4);
    let covered = effective_heal + prevented_damage;
    let exact_stabilize =
        ctx.imminent_unblocked_damage > 0 && covered >= ctx.imminent_unblocked_damage;
    let lethal_window = ctx.imminent_unblocked_damage >= ctx.current_hp;
    let massive_window = ctx.imminent_unblocked_damage >= ctx.current_hp + 10
        || ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_mul(2);
    let remaining_gap = (ctx.imminent_unblocked_damage - covered).max(0);

    let mut value = effective_heal * SURVIVAL_HEAL_POINT_SCORE;
    value += prevented_damage * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE;
    value += kills * SURVIVAL_KILL_SCORE;

    if ctx.current_hp <= 30 && effective_heal > 0 {
        value += SURVIVAL_LOW_HP_HEAL_BONUS;
    }
    if lethalish && covered > 0 {
        value += SURVIVAL_LETHALISH_COVER_BONUS + covered * 180;
    } else if covered >= ctx.imminent_unblocked_damage.max(0) && covered > 0 {
        value += SURVIVAL_STABILIZE_BONUS;
    }
    if exact_stabilize {
        value += SURVIVAL_EXACT_STABILIZE_BONUS
            + ctx.imminent_unblocked_damage.max(0) * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE;
        if lethal_window {
            value += SURVIVAL_LETHAL_WINDOW_BONUS;
        }
        if massive_window {
            value += SURVIVAL_MASSIVE_WINDOW_BONUS;
        }
    } else if lethal_window && covered > 0 {
        value += SURVIVAL_PARTIAL_LETHAL_SAVE_BONUS
            + covered * SURVIVAL_PREVENTED_DAMAGE_POINT_SCORE
            - remaining_gap * 260;
    }
    if lethal_window && kills > 0 {
        value += SURVIVAL_KILL_IN_LETHAL_WINDOW_BONUS;
    }

    value
}

pub(crate) fn reaper_timing_score(
    ctx: &SurvivalTimingContext,
    hp_gain: i32,
    prevented_damage: i32,
    kills: i32,
) -> i32 {
    survival_swing_score(ctx, hp_gain, prevented_damage, kills)
}

pub(crate) fn hand_shaping_play_now_score(can_play_now: bool) -> i32 {
    if can_play_now {
        PLAY_NOW_KEEP_PENALTY
    } else {
        DELAY_TO_TOPDECK_BONUS
    }
}

pub(crate) fn apotheosis_timing_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    let mut value =
        APOTHEOSIS_BASE_SCORE + upgradable_targets.max(0) * APOTHEOSIS_UPGRADE_TARGET_SCORE;
    if upgradable_targets <= 0 {
        value -= APOTHEOSIS_NO_TARGETS_PENALTY;
    } else if upgradable_targets >= 3 {
        value += APOTHEOSIS_MULTI_TARGET_BONUS;
    }
    if imminent_unblocked_damage > 8 {
        value -= APOTHEOSIS_UNDER_PRESSURE_PENALTY;
    }
    value
}

pub(crate) fn apotheosis_hand_shaping_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    -(apotheosis_timing_score(upgradable_targets, imminent_unblocked_damage) / 2)
}

pub(crate) fn apparition_timing_score(ctx: &ApparitionTimingContext) -> i32 {
    let prevented_damage = if ctx.imminent_unblocked_damage > 0 {
        ctx.imminent_unblocked_damage
    } else {
        ctx.total_incoming_damage
    };
    let swing = survival_swing_score(
        &SurvivalTimingContext {
            current_hp: ctx.current_hp,
            imminent_unblocked_damage: ctx.imminent_unblocked_damage,
            missing_hp: 0,
        },
        0,
        prevented_damage,
        0,
    );
    let lethal_window = ctx.imminent_unblocked_damage >= ctx.current_hp;
    let massive_window = ctx.imminent_unblocked_damage >= ctx.current_hp + 10
        || ctx.imminent_unblocked_damage >= ctx.current_hp.saturating_mul(2);
    let hand_pressure = ctx.apparitions_in_hand.saturating_sub(1);
    let reserve_pressure = ctx.remaining_apparitions_total.saturating_sub(1);

    if ctx.current_intangible > 0 {
        let mut value = -APPARITION_EXISTING_INTANGIBLE_PENALTY
            - ctx.current_intangible * APPARITION_INTANGIBLE_STACK_PENALTY;
        if !ctx.upgraded {
            value += APPARITION_UNUPGRADED_OVERLAP_RELIEF;
        } else if ctx.has_runic_pyramid {
            value -= APPARITION_PYRAMID_OVERLAP_PENALTY;
        }
        if ctx.current_hp <= 25 {
            value += APPARITION_LOW_HP_OVERLAP_RELIEF;
        }
        if hand_pressure >= 2 {
            value += hand_pressure * APPARITION_HAND_FLOOD_RELIEF;
        }
        if reserve_pressure >= 2 {
            value += reserve_pressure.min(4) * APPARITION_RESERVE_FLOOD_RELIEF;
        }
        value += ctx.encounter_pressure.max(0) * APPARITION_PRESSURE_POINT_SCORE;
        if ctx.total_incoming_damage >= 12 {
            value += APPARITION_INCOMING_OVERLAP_RELIEF;
        }
        if lethal_window {
            value += APPARITION_LETHAL_OVERLAP_RELIEF;
        }
        if massive_window {
            value += APPARITION_MASSIVE_OVERLAP_RELIEF;
        }
        return value;
    }

    if !ctx.upgraded {
        let mut value = APPARITION_UNUPGRADED_BASE_SCORE + swing;
        if ctx.imminent_unblocked_damage > 0 || ctx.current_hp <= 35 {
            value += APPARITION_UNUPGRADED_THREAT_BONUS;
        }
        if lethal_window {
            value += APPARITION_UNUPGRADED_LETHAL_BONUS;
        }
        if ctx.apparitions_in_hand >= 2 {
            value += APPARITION_UNUPGRADED_EXTRA_COPY_BONUS;
        }
        if ctx.imminent_unblocked_damage == 0
            && ctx.total_incoming_damage == 0
            && ctx.current_hp <= 35
            && reserve_pressure >= 2
            && ctx.encounter_pressure >= 10
        {
            value += APPARITION_UNUPGRADED_FRONTLOAD_BONUS
                + ctx.encounter_pressure * APPARITION_UNUPGRADED_FRONTLOAD_PRESSURE_SCORE;
        }
        value
    } else {
        let mut value = if ctx.imminent_unblocked_damage > 0 || ctx.current_hp <= 22 {
            APPARITION_UPGRADED_ACTIVE_BASE_SCORE + swing
        } else if ctx.has_runic_pyramid {
            -APPARITION_UPGRADED_SAFE_PYRAMID_PENALTY
        } else {
            -APPARITION_UPGRADED_SAFE_DELAY_PENALTY
        };
        if lethal_window {
            value += APPARITION_UPGRADED_LETHAL_BONUS;
        }
        if massive_window {
            value += APPARITION_UPGRADED_MASSIVE_BONUS;
        }
        if ctx.has_runic_pyramid && (lethal_window || massive_window) {
            value += APPARITION_UPGRADED_PYRAMID_THREAT_BONUS;
        }
        if ctx.imminent_unblocked_damage == 0
            && ctx.total_incoming_damage == 0
            && ctx.current_hp <= 28
            && reserve_pressure >= 2
            && ctx.encounter_pressure >= 12
        {
            value += APPARITION_UPGRADED_FRONTLOAD_BONUS
                + ctx.encounter_pressure * APPARITION_UPGRADED_FRONTLOAD_PRESSURE_SCORE;
        }
        value
    }
}

pub(crate) fn apparition_hand_shaping_score(ctx: &ApparitionTimingContext) -> i32 {
    let timing = apparition_timing_score(ctx);

    if !ctx.upgraded {
        if timing >= 8_000 {
            -(timing / APPARITION_HAND_SHAPING_TIMING_DIVISOR)
                .max(APPARITION_HAND_SHAPING_MIN_KEEP_PENALTY)
        } else {
            APPARITION_HAND_SHAPING_TOPDECK_BONUS
                + ctx.apparitions_in_hand.saturating_sub(1)
                    * APPARITION_HAND_SHAPING_EXTRA_COPY_SCORE
        }
    } else if timing > 0 {
        -(timing / APPARITION_UPGRADED_HAND_SHAPING_TIMING_DIVISOR)
            .max(APPARITION_UPGRADED_HAND_SHAPING_MIN_KEEP_PENALTY)
    } else if ctx.has_runic_pyramid {
        -APPARITION_UPGRADED_PYRAMID_HOLD_PENALTY
    } else {
        -APPARITION_UPGRADED_GENERIC_HOLD_PENALTY
    }
}

pub(crate) fn reaper_hand_shaping_score(ctx: &SurvivalTimingContext) -> i32 {
    let assumed_heal = ctx.missing_hp.min(12).max(0);
    let assumed_prevented = ctx.imminent_unblocked_damage.min(assumed_heal).max(0);
    -(reaper_timing_score(ctx, assumed_heal, assumed_prevented, 0) / 3)
}

pub(crate) fn hand_shaping_next_draw_window_score(
    draws_next_turn: i32,
    guaranteed_topdeck: bool,
) -> i32 {
    if !guaranteed_topdeck || draws_next_turn <= 0 {
        0
    } else {
        -600 - draws_next_turn.min(5) * 120
    }
}

pub(crate) fn hand_shaping_delay_quality_score(
    card_id: CardId,
    card_type: CardType,
    cost: i32,
    current_energy: i32,
    safe_block_turn: bool,
) -> i32 {
    let mut score = match card_type {
        CardType::Curse | CardType::Status => -20_000,
        _ => 0,
    };

    if card_type == CardType::Skill && safe_block_turn {
        score += 900;
    }
    if matches!(card_id, CardId::Defend | CardId::DefendG) && safe_block_turn {
        score += 1_200;
    }
    if matches!(card_id, CardId::Warcry | CardId::ThinkingAhead) {
        score += 800;
    }
    if card_type == CardType::Attack && cost > current_energy {
        score += 1_000;
    }
    if cost == 0 {
        score -= 700;
    }

    score
}

pub(crate) fn body_slam_delay_score(
    current_damage: i32,
    additional_block_before_slam: i32,
    can_kill_now: bool,
    imminent_unblocked_damage: i32,
) -> i32 {
    if can_kill_now {
        return 4_500 + current_damage.max(0) * 80;
    }
    if additional_block_before_slam <= 0 {
        return 0;
    }

    let mut score = -(2_500 + additional_block_before_slam * 280);
    if current_damage <= 0 {
        score -= 2_200;
    }
    if imminent_unblocked_damage > 0 {
        score -= additional_block_before_slam.min(imminent_unblocked_damage.max(0)) * 120;
    }
    score
}

pub(crate) fn forced_mass_exhaust_selectivity_score(
    junk_fuel_count: i32,
    protected_piece_count: i32,
    core_piece_count: i32,
    exact_stabilize: bool,
    imminent_unblocked_damage: i32,
    engine_support_level: i32,
) -> i32 {
    let mut score = 0;

    if junk_fuel_count > 0 {
        score += 2_500 + junk_fuel_count * 2_000;
        if imminent_unblocked_damage > 0 {
            score += junk_fuel_count * 900;
        }
    }

    score -= protected_piece_count * 7_500;
    score -= core_piece_count * 14_000;

    let junk_shortage = (protected_piece_count - junk_fuel_count).max(0);
    if junk_shortage > 0 {
        score -= junk_shortage * 8_500;
    }
    if junk_fuel_count == 0 && protected_piece_count > 0 {
        score -= 8_000;
    }
    if engine_support_level > 0 && junk_fuel_count > 0 {
        score += junk_fuel_count * engine_support_level * 2_000;
    }
    if exact_stabilize {
        score += 5_000 + imminent_unblocked_damage.min(25).max(0) * 220;
    }

    score
}

pub(crate) fn exhaust_fuel_value_score(
    card_id: CardId,
    card_type: CardType,
    cost: i32,
    current_energy: i32,
    safe_block_turn: bool,
    can_play_now: bool,
    timing_hold_score: i32,
    feel_no_pain_amount: i32,
    has_dark_embrace: bool,
) -> i32 {
    let mut score =
        hand_shaping_delay_quality_score(card_id, card_type, cost, current_energy, safe_block_turn);
    score += hand_shaping_play_now_score(can_play_now);
    score += timing_hold_score;

    if matches!(card_type, CardType::Curse | CardType::Status) {
        score +=
            status_exhaust_value_score(card_id, card_type, feel_no_pain_amount, has_dark_embrace);
    }

    if card_type == CardType::Power {
        score -= 4_500;
    }

    score -= match card_id {
        CardId::FiendFire
        | CardId::SecondWind
        | CardId::TrueGrit
        | CardId::BurningPact
        | CardId::SeverSoul
        | CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace
        | CardId::LimitBreak
        | CardId::DemonForm => 2_800,
        CardId::Inflame | CardId::Shockwave => 4_200,
        CardId::Offering => 1_400,
        _ => 0,
    };

    score
}

fn status_exhaust_value_score(
    card_id: CardId,
    card_type: CardType,
    feel_no_pain_amount: i32,
    has_dark_embrace: bool,
) -> i32 {
    match card_type {
        CardType::Curse => match card_id {
            CardId::Parasite => 49_000,
            CardId::Pain | CardId::Normality => 47_000,
            CardId::Regret | CardId::Writhe | CardId::Decay => 44_000,
            _ => 40_000,
        },
        CardType::Status => match card_id {
            CardId::Dazed => {
                let mut score = 10_000;
                if feel_no_pain_amount > 0 {
                    score += 7_000 + feel_no_pain_amount * 450;
                }
                if has_dark_embrace {
                    score += 9_000;
                }
                score
            }
            CardId::Slimed => 44_000,
            CardId::Burn => 47_000,
            _ => 40_000,
        },
        _ => 0,
    }
}

pub(crate) fn exhaust_deck_floor_penalty(remaining_cards_after: i32) -> i32 {
    match remaining_cards_after {
        i32::MIN..=4 => 12_000,
        5 => 7_500,
        6 => 4_500,
        7 => 2_000,
        8 => 800,
        _ => 0,
    }
}

pub(crate) fn exhaust_fuel_reserve_penalty(
    remaining_low_value_fuel_after: i32,
    exhaust_count: i32,
) -> i32 {
    if exhaust_count <= 1 {
        return 0;
    }

    match remaining_low_value_fuel_after {
        i32::MIN..=0 => 3_500,
        1 => 1_800,
        _ => 0,
    }
}

pub(crate) fn exhaust_future_fuel_reserve_score(
    remaining_low_value_fuel_after: i32,
    future_exhaust_demand: i32,
) -> i32 {
    if future_exhaust_demand <= 0 {
        return 0;
    }

    let shortage = (future_exhaust_demand - remaining_low_value_fuel_after).max(0);
    -(shortage * 25_000)
}

pub(crate) fn exhaust_mass_play_score(
    total_fuel_value: i32,
    exhaust_count: i32,
    remaining_cards_after: i32,
    remaining_low_value_fuel_after: i32,
    closeout_bonus: i32,
) -> i32 {
    total_fuel_value + exhaust_count.max(0) * 220 + closeout_bonus
        - exhaust_deck_floor_penalty(remaining_cards_after)
        - exhaust_fuel_reserve_penalty(remaining_low_value_fuel_after, exhaust_count)
}

pub(crate) fn mass_exhaust_base_score(profile: &MassExhaustProfile, total_cycle_cards: i32) -> i32 {
    exhaust_mass_play_score(
        profile.total_fuel,
        profile.exhausted_count,
        profile.remaining_cards_after,
        profile.remaining_low_value_fuel_after,
        profile.closeout_bonus,
    ) + exhaust_engine_immediate_payoff_score(
        profile.exhausted_count,
        profile.block_per_exhaust,
        profile.dark_embrace_draw_count,
    ) + deck_cycle_thinning_score(
        total_cycle_cards,
        profile.remaining_cards_after,
        profile.dark_embrace_draw_count,
        0,
        0,
        0,
    )
}

pub(crate) fn mass_exhaust_second_wind_selectivity_score(profile: &MassExhaustProfile) -> i32 {
    let mut score = forced_mass_exhaust_selectivity_score(
        profile.junk_fuel_count,
        profile.protected_piece_count,
        profile.core_piece_count,
        profile.exact_stabilize,
        profile.imminent_unblocked_damage,
        profile.engine_support_level,
    );

    if profile.playable_block_lost > 0 {
        let emergency_relief = if profile.exact_stabilize {
            0
        } else {
            profile.imminent_unblocked_damage.min(18) * 180
        };
        let preserve_penalty = (profile.playable_block_lost * 6_200)
            - profile.junk_fuel_count * 650
            - emergency_relief;
        score -= preserve_penalty.max(0);
        if profile.junk_fuel_count >= 1
            && profile.playable_block_lost >= 2
            && !profile.exact_stabilize
        {
            score -= 12_000 + profile.playable_block_lost * 2_500;
        }
    }

    if profile.engine_support_level == 0
        && profile.junk_fuel_count == 0
        && profile.protected_piece_count >= 2
        && profile.low_pressure_high_hp
    {
        let base_penalty = if profile.exact_stabilize {
            9_000
        } else {
            14_000
        };
        score -=
            base_penalty + profile.protected_piece_count * 1_800 + profile.core_piece_count * 2_500;
    }

    score
}

pub(crate) fn mass_exhaust_keeper_penalty(
    profile: &MassExhaustProfile,
    protected_piece_weight: i32,
    core_piece_weight: i32,
) -> i32 {
    profile.protected_piece_count * protected_piece_weight
        + profile.core_piece_count * core_piece_weight
}

pub(crate) fn exhaust_random_play_score(
    low_value_fuel_count: i32,
    protected_piece_count: i32,
    remaining_cards_after: i32,
) -> i32 {
    let mut score = low_value_fuel_count * 1_600 - protected_piece_count * 1_400;
    score -= exhaust_deck_floor_penalty(remaining_cards_after);

    if low_value_fuel_count <= 0 {
        score -= 4_500;
    } else if low_value_fuel_count == 1 {
        score -= 1_200;
    }

    score
}

pub(crate) fn exhaust_random_core_risk_score(
    low_value_fuel_count: i32,
    core_piece_count: i32,
    near_core_piece_count: i32,
) -> i32 {
    let mut score = 0;

    if low_value_fuel_count <= 0 {
        score -= core_piece_count * 3_500 + near_core_piece_count * 1_800;
    } else if low_value_fuel_count == 1 {
        score -= core_piece_count * 1_800 + near_core_piece_count * 900;
    }

    score
}

#[allow(dead_code)]
pub(crate) fn exhaust_engine_payoff_score(
    exhaust_count: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
    status_fuel_count: i32,
    future_status_cards: i32,
) -> i32 {
    exhaust_engine_immediate_payoff_score(exhaust_count, block_per_exhaust, draw_per_exhaust)
        + exhaust_engine_future_payoff_score(status_fuel_count, future_status_cards, 0)
}

pub(crate) fn exhaust_engine_immediate_payoff_score(
    exhaust_count: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
) -> i32 {
    let exhaust_count = exhaust_count.max(0);
    let mut score = 0;
    score += exhaust_count * block_per_exhaust.max(0) * 220;
    score += exhaust_count * draw_per_exhaust.max(0) * 1_000;
    score
}

pub(crate) fn exhaust_engine_future_payoff_score(
    status_fuel_count: i32,
    future_status_cards: i32,
    sentry_count: i32,
) -> i32 {
    let mut score = 0;
    score += status_fuel_count.max(0) * 450;
    score += future_status_cards.max(0) * 160;
    score += sentry_count.max(0) * 2_500;
    score
}

pub(crate) fn exhaust_engine_setup_score(
    already_active: bool,
    immediate_exhaust_count: i32,
    future_exhaust_sources: i32,
    block_per_exhaust: i32,
    draw_per_exhaust: i32,
    status_fuel_count: i32,
    future_status_cards: i32,
    sentry_count: i32,
    extra_synergy: i32,
) -> i32 {
    if already_active {
        return -8_000;
    }

    2_500
        + exhaust_engine_immediate_payoff_score(
            immediate_exhaust_count,
            block_per_exhaust,
            draw_per_exhaust,
        )
        + future_exhaust_sources.max(0) * 900
        + exhaust_engine_future_payoff_score(status_fuel_count, future_status_cards, sentry_count)
        + extra_synergy
}

pub(crate) fn draw_continuity_score(
    remaining_cards_after: i32,
    immediate_draws: i32,
    future_draws: i32,
    shuffle_recovery_cards: i32,
) -> i32 {
    let remaining_cards_after = remaining_cards_after.max(0);
    let accessible_cycle_cards =
        remaining_cards_after + immediate_draws.max(0) + shuffle_recovery_cards.max(0);
    let mut score = immediate_draws.max(0) * 900 + future_draws.max(0) * 240;

    score += match accessible_cycle_cards {
        i32::MIN..=3 => -10_000,
        4 => -6_500,
        5 => -3_500,
        6 => -1_400,
        7 => -400,
        8..=10 => 300,
        _ => 0,
    };

    if remaining_cards_after >= 12 {
        score += 500;
    }

    score
}

pub(crate) fn battle_trance_timing_score(ctx: &DrawTimingContext, draw_count: i32) -> i32 {
    draw_action_timing_score(ctx, true, draw_count)
}

pub(crate) fn draw_action_timing_score(
    ctx: &DrawTimingContext,
    applies_no_draw: bool,
    draw_count: i32,
) -> i32 {
    if ctx.player_no_draw {
        return -DRAW_BLOCKED_BY_NO_DRAW_PENALTY;
    }

    let mut score = DRAW_BASE_SCORE + draw_count.max(0) * DRAW_CARD_SCORE;
    let hand_after_draw = ctx.current_hand_size + draw_count;
    score -= (hand_after_draw - 9).max(0) * DRAW_HAND_OVERFLOW_PENALTY;
    if applies_no_draw {
        score -= DRAW_NO_DRAW_SELF_PENALTY;
    }

    match ctx.current_energy {
        i32::MIN..=0 => {
            score -= 4_800;
            score += ctx.future_zero_cost_cards * 700;
            score += ctx.future_one_cost_cards * 120;
            score -= ctx.future_two_plus_cost_cards * 900;
            score -= ctx.future_key_delay_weight * 260;
            score -= ctx.future_high_cost_key_delay_weight * 420;
            score -= ctx.future_status_cards * 850;
            score -= ctx.other_draw_sources_in_hand * 1_200;
            if applies_no_draw {
                score -= 1_400 + ctx.other_draw_sources_in_hand * 600;
            }
            if ctx.future_zero_cost_cards == 0 {
                score -= 2_000;
            }
        }
        1 => {
            score += ctx.future_zero_cost_cards * 600;
            score += ctx.future_one_cost_cards * 450;
            score -= ctx.future_two_plus_cost_cards * 450;
            score -= ctx.future_key_delay_weight * 140;
            score -= ctx.future_high_cost_key_delay_weight * 220;
            score -= ctx.future_status_cards * 600;
            score -= ctx.other_draw_sources_in_hand * 900;
            if applies_no_draw {
                score -= 1_000 + ctx.other_draw_sources_in_hand * 450;
            }
        }
        _ => {
            score += ctx.future_zero_cost_cards * 280;
            score += ctx.future_one_cost_cards * 420;
            score += ctx.future_two_plus_cost_cards * 260;
            score -= ctx.future_key_delay_weight * 40;
            score -= ctx.future_high_cost_key_delay_weight * 60;
            score -= ctx.future_status_cards * 350;
            score -= ctx.other_draw_sources_in_hand * 650;
            if applies_no_draw {
                score -= 500 + ctx.other_draw_sources_in_hand * 250;
            }
        }
    }

    score
}

pub(crate) fn deck_cycle_thinning_score(
    card_pool_size_before: i32,
    remaining_cards_after: i32,
    immediate_draws: i32,
    future_draws: i32,
    shuffle_recovery_cards: i32,
    extra_loop_value: i32,
) -> i32 {
    let removed_cards = (card_pool_size_before - remaining_cards_after).max(0);
    let mut score = removed_cards * 260;

    if card_pool_size_before <= 8 {
        score -= removed_cards * 700;
    } else if card_pool_size_before <= 10 {
        score -= removed_cards * 300;
    }

    score
        + draw_continuity_score(
            remaining_cards_after,
            immediate_draws,
            future_draws,
            shuffle_recovery_cards,
        )
        + extra_loop_value
}

pub(crate) fn status_loop_cycle_score(
    draw_per_status: i32,
    status_in_draw: i32,
    status_in_discard: i32,
    shuffle_discard_into_draw: bool,
    extra_cycle_draws: i32,
    sentry_count: i32,
) -> i32 {
    let draw_per_status = draw_per_status.max(0);
    let draw_status_value = status_in_draw.max(0) * draw_per_status * 850;
    let discard_status_value = if shuffle_discard_into_draw {
        status_in_discard.max(0) * draw_per_status * 1_050
    } else {
        status_in_discard.max(0) * draw_per_status * 240
    };

    draw_status_value
        + discard_status_value
        + extra_cycle_draws.max(0) * 240
        + sentry_count.max(0) * draw_per_status * 1_800
}

pub(crate) fn exhaust_finish_window_score(
    exact_lethal: bool,
    kills: i32,
    prevented_damage: i32,
    remaining_alive_after: i32,
) -> i32 {
    let mut score = prevented_damage.max(0) * 180 + kills.max(0) * 1_600;

    if exact_lethal {
        score += 8_000;
    } else if remaining_alive_after <= 1 && kills > 0 {
        score += 3_500;
    }

    score
}

pub(crate) fn flight_break_progress_score(hits: i32, flight: i32, prevented_damage: i32) -> f32 {
    if hits <= 0 || flight <= 0 {
        return 0.0;
    }

    if hits >= flight {
        4_500.0 + prevented_damage as f32 * 220.0
    } else {
        900.0 * hits as f32
    }
}

pub(crate) fn persistent_block_progress_score(block_damage: i32) -> f32 {
    block_damage.max(0) as f32 * 90.0
}

