use crate::bot::combat_card_knowledge::{
    BranchFamily, ChanceProfile, OrderingConstraint, RiskProfile, TurnActionRole,
    TurnOrderingHint,
};
use crate::bot::combat_families::draw::DrawTimingContext;

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
