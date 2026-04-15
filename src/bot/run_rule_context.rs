use crate::bot::card_taxonomy::taxonomy;
use crate::bot::evaluator::{CardEvaluator, DeckProfile};
use crate::combat::CombatCard;
use crate::content::cards::{self, CardId, CardType};
use crate::content::relics::{self, RelicId};
use crate::state::run::RunState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BaseCostBand {
    XCost,
    ZeroOne,
    Two,
    ThreePlus,
    Unplayable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RuleRegimeSummary {
    pub cost_randomized: bool,
    pub retain_heavy: bool,
    pub skills_free: bool,
    pub block_retention: bool,
    pub exhaust_positive: bool,
    pub draw_rich: bool,
    pub energy_rich: bool,
    pub status_positive: bool,
    pub strength_scaling: bool,
    pub multi_hit_scaling: bool,
}

impl RuleRegimeSummary {
    pub(crate) fn dominant_key(self) -> Option<&'static str> {
        if self.cost_randomized {
            Some("cost_randomized")
        } else if self.skills_free {
            Some("skills_free")
        } else if self.retain_heavy {
            Some("retain_heavy")
        } else if self.block_retention {
            Some("block_retention")
        } else if self.exhaust_positive {
            Some("exhaust_positive")
        } else if self.energy_rich {
            Some("energy_rich")
        } else if self.draw_rich {
            Some("draw_rich")
        } else if self.status_positive {
            Some("status_positive")
        } else if self.strength_scaling {
            Some("strength_scaling")
        } else if self.multi_hit_scaling {
            Some("multi_hit_scaling")
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) struct RunRuleContext {
    pub summary: RuleRegimeSummary,
    pub profile: DeckProfile,
    pub high_cost_goodstuff_density: i32,
    pub low_cost_filler_density: i32,
    pub draw_slot_waste: i32,
    pub randomized_cost_upside_density: i32,
    pub randomized_cost_downside_density: i32,
    pub expected_playable_value_under_randomized_cost: i32,
    pub self_clog_replication_risk: i32,
    pub deck_size: i32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ConditionedCardFeatures {
    pub base_cost_band: BaseCostBand,
    pub is_zero_one_cost: bool,
    pub is_high_cost: bool,
    pub is_draw: bool,
    pub is_energy_bridge: bool,
    pub is_setup: bool,
    pub is_payoff: bool,
    pub is_scaling: bool,
    pub is_x_cost: bool,
    pub is_self_replicating: bool,
    pub is_random_generation: bool,
    pub is_cost_manipulation_dependent: bool,
    pub draw_slot_pressure: i32,
    pub future_clog_risk: i32,
    pub snecko_roll_upside: i32,
    pub snecko_roll_downside: i32,
    pub shell_alignment: i32,
    pub shell_disruption: i32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConditionedDelta {
    pub total: i32,
    pub rationale_key: Option<&'static str>,
    pub rule_context_summary: Option<&'static str>,
}

pub(crate) fn build_run_rule_context(rs: &RunState) -> RunRuleContext {
    let profile = CardEvaluator::deck_profile(rs);
    let summary = RuleRegimeSummary {
        cost_randomized: has_relic(rs, RelicId::SneckoEye),
        retain_heavy: has_relic(rs, RelicId::RunicPyramid),
        skills_free: has_card(rs, CardId::Corruption),
        block_retention: has_card(rs, CardId::Barricade) || has_relic(rs, RelicId::Calipers),
        exhaust_positive: has_card(rs, CardId::Corruption)
            || has_relic(rs, RelicId::DeadBranch)
            || (profile.exhaust_engines >= 1 && profile.exhaust_outlets >= 1),
        draw_rich: profile.draw_sources >= 3 || has_relic(rs, RelicId::SneckoEye),
        energy_rich: has_relic(rs, RelicId::IceCream)
            || rs
                .relics
                .iter()
                .any(|relic| relics::energy_master_delta(relic.id) > 0),
        status_positive: profile.status_generators >= 1 && profile.status_payoffs >= 1,
        strength_scaling: profile.strength_enablers >= 1,
        multi_hit_scaling: count_multi_hit_cards(&rs.master_deck) >= 2
            && profile.strength_enablers >= 1,
    };

    let mut high_cost_goodstuff_density = 0;
    let mut low_cost_filler_density = 0;
    let mut draw_slot_waste = 0;
    let mut randomized_cost_upside_density = 0;
    let mut randomized_cost_downside_density = 0;
    let mut expected_playable_value_under_randomized_cost = 0;
    let mut self_clog_replication_risk = 0;

    for card in &rs.master_deck {
        let features = conditioned_card_features(rs, &summary, &profile, card.id);
        let owned_value = CardEvaluator::evaluate_owned_card(card.id, rs);
        if features.is_high_cost && owned_value >= 45 {
            high_cost_goodstuff_density += 1;
        }
        if features.is_zero_one_cost && owned_value <= 30 && !features.is_draw {
            low_cost_filler_density += 1;
        }
        draw_slot_waste += features.draw_slot_pressure;
        randomized_cost_upside_density += features.snecko_roll_upside.max(0) / 8;
        randomized_cost_downside_density += features.snecko_roll_downside.max(0) / 8;
        expected_playable_value_under_randomized_cost +=
            features.snecko_roll_upside - features.snecko_roll_downside / 2;
        self_clog_replication_risk += if features.is_self_replicating {
            features.future_clog_risk.max(1)
        } else {
            0
        };
    }

    RunRuleContext {
        summary,
        profile,
        high_cost_goodstuff_density,
        low_cost_filler_density,
        draw_slot_waste,
        randomized_cost_upside_density,
        randomized_cost_downside_density,
        expected_playable_value_under_randomized_cost,
        self_clog_replication_risk,
        deck_size: rs.master_deck.len() as i32,
    }
}

pub(crate) fn conditioned_card_addition_value(rs: &RunState, card_id: CardId) -> ConditionedDelta {
    let ctx = build_run_rule_context(rs);
    conditioned_card_addition_value_with_context(rs, card_id, &ctx)
}

pub(crate) fn conditioned_card_addition_value_with_context(
    rs: &RunState,
    card_id: CardId,
    ctx: &RunRuleContext,
) -> ConditionedDelta {
    let features = conditioned_card_features(rs, &ctx.summary, &ctx.profile, card_id);
    let mut total = 0;
    let mut dominant_key = None;
    let mut dominant_abs = 0;

    let components = [
        (
            "cost_randomized",
            ctx.summary.cost_randomized,
            apply_cost_randomized_adjustment(ctx, &features),
        ),
        (
            "retain_heavy",
            ctx.summary.retain_heavy,
            apply_retain_heavy_adjustment(ctx, &features),
        ),
        (
            "skills_free",
            ctx.summary.skills_free,
            apply_skills_free_adjustment(ctx, &features),
        ),
        (
            "block_retention",
            ctx.summary.block_retention,
            apply_block_retention_adjustment(ctx, &features),
        ),
        (
            "exhaust_positive",
            ctx.summary.exhaust_positive,
            apply_exhaust_positive_adjustment(ctx, &features),
        ),
        (
            "energy_rich",
            ctx.summary.energy_rich,
            apply_energy_rich_adjustment(ctx, &features),
        ),
        (
            "draw_rich",
            ctx.summary.draw_rich,
            apply_draw_rich_adjustment(ctx, &features),
        ),
    ];

    for (key, active, score) in components {
        if !active || score == 0 {
            continue;
        }
        total += score;
        let abs = score.abs();
        if abs > dominant_abs {
            dominant_abs = abs;
            dominant_key = Some(key);
        }
    }

    ConditionedDelta {
        total,
        rationale_key: dominant_key,
        rule_context_summary: ctx.summary.dominant_key(),
    }
}

pub(crate) fn conditioned_card_features(
    _rs: &RunState,
    summary: &RuleRegimeSummary,
    profile: &DeckProfile,
    card_id: CardId,
) -> ConditionedCardFeatures {
    let def = cards::get_card_definition(card_id);
    let tax = taxonomy(card_id);
    let base_cost_band = match def.cost {
        -1 => BaseCostBand::XCost,
        i8::MIN..=-2 => BaseCostBand::Unplayable,
        0 | 1 => BaseCostBand::ZeroOne,
        2 => BaseCostBand::Two,
        _ => BaseCostBand::ThreePlus,
    };
    let is_draw = tax.is_draw_core();
    let is_energy_bridge = tax.is_resource_conversion();
    let is_setup = tax.is_setup_power() || tax.is_engine_piece();
    let is_scaling = tax.is_scaling_power() || tax.is_strength_enabler();
    let is_payoff = tax.is_strength_payoff()
        || tax.is_multi_attack_payoff()
        || tax.is_block_payoff()
        || tax.is_combat_heal()
        || tax.is_vuln_payoff();
    let is_self_replicating = matches!(card_id, CardId::Anger);
    let is_random_generation = matches!(
        card_id,
        CardId::Discovery | CardId::InfernalBlade | CardId::Metamorphosis | CardId::Mayhem
    );
    let is_cost_manipulation_dependent = matches!(
        card_id,
        CardId::BloodForBlood | CardId::Dropkick | CardId::SeverSoul | CardId::SeeingRed
    ) || tax.is_conditional_free();

    let draw_slot_pressure = if is_self_replicating {
        4
    } else if is_random_generation {
        2
    } else if matches!(def.card_type, CardType::Attack | CardType::Skill)
        && !is_draw
        && !is_setup
        && !is_scaling
        && !is_payoff
        && def.cost <= 1
    {
        3
    } else if !is_draw && !is_setup && !is_scaling {
        1
    } else {
        0
    };

    let future_clog_risk = if is_self_replicating {
        5
    } else if summary.retain_heavy && !is_draw && !is_energy_bridge && !is_payoff {
        2
    } else if matches!(def.card_type, CardType::Power) && def.cost >= 2 {
        1
    } else {
        0
    };

    let snecko_roll_upside = snecko_roll_upside(def.cost, is_draw, is_setup, is_scaling, is_payoff);
    let snecko_roll_downside = snecko_roll_downside(
        def.cost,
        is_draw,
        is_self_replicating,
        is_cost_manipulation_dependent,
        is_random_generation,
        matches!(def.card_type, CardType::Attack | CardType::Skill),
    );

    let shell_alignment = shell_alignment_bonus(profile, card_id, is_draw, is_setup, is_payoff);
    let shell_disruption = shell_disruption_penalty(profile, card_id, future_clog_risk);

    ConditionedCardFeatures {
        base_cost_band,
        is_zero_one_cost: matches!(base_cost_band, BaseCostBand::ZeroOne),
        is_high_cost: matches!(base_cost_band, BaseCostBand::Two | BaseCostBand::ThreePlus),
        is_draw,
        is_energy_bridge,
        is_setup,
        is_payoff,
        is_scaling,
        is_x_cost: matches!(base_cost_band, BaseCostBand::XCost),
        is_self_replicating,
        is_random_generation,
        is_cost_manipulation_dependent,
        draw_slot_pressure,
        future_clog_risk,
        snecko_roll_upside,
        snecko_roll_downside,
        shell_alignment,
        shell_disruption,
    }
}

fn apply_cost_randomized_adjustment(
    ctx: &RunRuleContext,
    features: &ConditionedCardFeatures,
) -> i32 {
    let mut score = 0;

    if features.is_high_cost {
        score += 16 + features.snecko_roll_upside / 2;
        if features.is_setup || features.is_scaling || features.is_payoff {
            score += 8;
        }
    }
    if features.is_draw {
        score += 14 + ctx.high_cost_goodstuff_density.min(5) * 2;
    }
    if features.is_x_cost {
        score -= 26;
    }
    if features.is_zero_one_cost
        && !features.is_draw
        && !features.is_energy_bridge
        && !features.is_setup
    {
        score -= 18;
    }
    if features.is_self_replicating {
        score -= 30 + ctx.self_clog_replication_risk.min(5) * 2;
    }
    if features.is_cost_manipulation_dependent {
        score -= 12;
    }
    if features.draw_slot_pressure > 0 {
        score -= features.draw_slot_pressure * 4;
    }
    if features.future_clog_risk > 0 {
        score -= features.future_clog_risk * 3;
    }

    score += features.shell_alignment * 3;
    score -= features.shell_disruption * 3;

    if ctx.low_cost_filler_density >= 6 && features.is_zero_one_cost && !features.is_draw {
        score -= 8;
    }
    if ctx.high_cost_goodstuff_density >= 4 && features.is_draw {
        score += 6;
    }

    score
}

fn apply_retain_heavy_adjustment(_ctx: &RunRuleContext, features: &ConditionedCardFeatures) -> i32 {
    let mut score = 0;
    if features.future_clog_risk > 0 {
        score -= features.future_clog_risk * 4;
    }
    if features.is_draw {
        score -= 2;
    }
    if features.is_setup || features.is_payoff || features.is_scaling {
        score += 6;
    }
    if features.is_random_generation {
        score -= 4;
    }
    score
}

fn apply_skills_free_adjustment(_ctx: &RunRuleContext, features: &ConditionedCardFeatures) -> i32 {
    let mut score = 0;
    if matches!(
        features.base_cost_band,
        BaseCostBand::Two | BaseCostBand::ThreePlus
    ) && !features.is_payoff
    {
        score += 2;
    }
    if features.is_draw {
        score += 8;
    }
    if features.is_energy_bridge {
        score += 6;
    }
    if features.is_setup || features.is_scaling {
        score += 10;
    }
    if features.is_payoff && !features.is_draw {
        score -= 4;
    }
    score
}

fn apply_block_retention_adjustment(
    _ctx: &RunRuleContext,
    features: &ConditionedCardFeatures,
) -> i32 {
    let mut score = 0;
    if features.is_payoff || features.is_setup {
        score += 4;
    }
    if matches!(
        features.base_cost_band,
        BaseCostBand::Two | BaseCostBand::ThreePlus
    ) && features.is_draw
    {
        score += 2;
    }
    score
}

fn apply_exhaust_positive_adjustment(
    _ctx: &RunRuleContext,
    features: &ConditionedCardFeatures,
) -> i32 {
    let mut score = 0;
    if features.is_setup || features.is_scaling {
        score += 6;
    }
    if features.is_draw {
        score += 4;
    }
    if features.is_random_generation {
        score += 3;
    }
    score
}

fn apply_energy_rich_adjustment(_ctx: &RunRuleContext, features: &ConditionedCardFeatures) -> i32 {
    let mut score = 0;
    if features.is_high_cost {
        score += 6;
    }
    if features.is_x_cost {
        score += 8;
    }
    if features.is_zero_one_cost && !features.is_draw && !features.is_setup {
        score -= 4;
    }
    score
}

fn apply_draw_rich_adjustment(_ctx: &RunRuleContext, features: &ConditionedCardFeatures) -> i32 {
    let mut score = 0;
    if features.is_draw {
        score += 3;
    }
    if features.future_clog_risk > 0 {
        score -= 2 * features.future_clog_risk;
    }
    if features.is_high_cost && (features.is_setup || features.is_payoff) {
        score += 3;
    }
    score
}

fn snecko_roll_upside(
    cost: i8,
    is_draw: bool,
    is_setup: bool,
    is_scaling: bool,
    is_payoff: bool,
) -> i32 {
    let mut score = match cost {
        3..=i8::MAX => 24,
        2 => 16,
        1 => 4,
        0 => 0,
        _ => 0,
    };
    if is_draw {
        score += 10;
    }
    if is_setup || is_scaling || is_payoff {
        score += 6;
    }
    score
}

fn snecko_roll_downside(
    cost: i8,
    is_draw: bool,
    is_self_replicating: bool,
    is_cost_manipulation_dependent: bool,
    is_random_generation: bool,
    is_action_card: bool,
) -> i32 {
    let mut score = match cost {
        0 => 18,
        1 => 12,
        2 => 4,
        3..=i8::MAX => 0,
        -1 => 18,
        _ => 0,
    };
    if is_draw {
        score -= 6;
    }
    if is_self_replicating {
        score += 18;
    }
    if is_cost_manipulation_dependent {
        score += 12;
    }
    if is_random_generation {
        score += 8;
    }
    if is_action_card && cost <= 1 {
        score += 4;
    }
    score.max(0)
}

fn shell_alignment_bonus(
    profile: &DeckProfile,
    card_id: CardId,
    is_draw: bool,
    is_setup: bool,
    is_payoff: bool,
) -> i32 {
    let tax = taxonomy(card_id);
    let mut score = 0;
    if tax.is_strength_payoff() && profile.strength_enablers > 0 {
        score += 3;
    }
    if tax.is_strength_enabler() && profile.strength_payoffs > 0 {
        score += 2;
    }
    if (tax.is_exhaust_engine() || tax.is_exhaust_outlet())
        && profile.exhaust_outlets + profile.exhaust_engines > 0
    {
        score += 3;
    }
    if tax.is_block_core() && profile.block_payoffs > 0 {
        score += 2;
    }
    if tax.is_block_payoff() && profile.block_core >= 2 {
        score += 3;
    }
    if is_draw
        && (profile.exhaust_engines > 0 || profile.strength_payoffs > 0 || profile.block_core > 0)
    {
        score += 1;
    }
    if is_setup && profile.power_scalers > 0 {
        score += 1;
    }
    if is_payoff && profile.draw_sources >= 2 {
        score += 1;
    }
    score
}

fn shell_disruption_penalty(profile: &DeckProfile, card_id: CardId, future_clog_risk: i32) -> i32 {
    let def = cards::get_card_definition(card_id);
    let mut penalty = future_clog_risk;
    if cards::is_starter_basic(card_id) {
        penalty += 2;
    }
    if profile.draw_sources >= 2
        && matches!(def.card_type, CardType::Attack | CardType::Skill)
        && def.cost <= 1
        && !taxonomy(card_id).is_draw_core()
        && !taxonomy(card_id).is_strength_payoff()
        && !taxonomy(card_id).is_exhaust_outlet()
    {
        penalty += 2;
    }
    penalty
}

fn count_multi_hit_cards(cards: &[CombatCard]) -> i32 {
    cards
        .iter()
        .filter(|card| {
            taxonomy(card.id).is_multi_hit() || taxonomy(card.id).is_multi_attack_payoff()
        })
        .count() as i32
}

fn has_relic(rs: &RunState, relic_id: RelicId) -> bool {
    rs.relics.iter().any(|relic| relic.id == relic_id)
}

fn has_card(rs: &RunState, card_id: CardId) -> bool {
    rs.master_deck.iter().any(|card| card.id == card_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::CombatCard;
    use crate::content::relics::RelicState;

    fn run_with(cards: &[CardId]) -> RunState {
        let mut rs = RunState::new(99, 0, false, "Ironclad");
        rs.master_deck = cards
            .iter()
            .enumerate()
            .map(|(idx, &id)| CombatCard::new(id, idx as u32))
            .collect();
        rs
    }

    #[test]
    fn snecko_eye_sets_cost_randomized_context() {
        let mut rs = run_with(&[CardId::Strike, CardId::Defend, CardId::Bash]);
        rs.relics.push(RelicState::new(RelicId::SneckoEye));

        let ctx = build_run_rule_context(&rs);
        assert!(ctx.summary.cost_randomized);
        assert_eq!(ctx.summary.dominant_key(), Some("cost_randomized"));
    }

    #[test]
    fn runic_pyramid_sets_retain_heavy_context() {
        let mut rs = run_with(&[CardId::Strike, CardId::Defend, CardId::Bash]);
        rs.relics.push(RelicState::new(RelicId::RunicPyramid));

        let ctx = build_run_rule_context(&rs);
        assert!(ctx.summary.retain_heavy);
    }

    #[test]
    fn corruption_and_barricade_enable_skills_free_and_block_retention() {
        let rs = run_with(&[
            CardId::Corruption,
            CardId::Barricade,
            CardId::ShrugItOff,
            CardId::Strike,
        ]);

        let ctx = build_run_rule_context(&rs);
        assert!(ctx.summary.skills_free);
        assert!(ctx.summary.block_retention);
        assert!(ctx.summary.exhaust_positive);
    }

    #[test]
    fn snecko_penalizes_anger_and_rewards_expensive_scalers() {
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::PommelStrike,
            CardId::ShrugItOff,
        ]);
        rs.relics.push(RelicState::new(RelicId::SneckoEye));

        let anger = conditioned_card_addition_value(&rs, CardId::Anger);
        let demon = conditioned_card_addition_value(&rs, CardId::DemonForm);

        assert!(anger.total < 0);
        assert!(demon.total > anger.total);
    }

    #[test]
    fn snecko_prefers_draw_over_low_cost_filler() {
        let mut rs = run_with(&[
            CardId::Strike,
            CardId::Defend,
            CardId::ShrugItOff,
            CardId::BattleTrance,
        ]);
        rs.relics.push(RelicState::new(RelicId::SneckoEye));

        let draw = conditioned_card_addition_value(&rs, CardId::PommelStrike);
        let filler = conditioned_card_addition_value(&rs, CardId::IronWave);

        assert!(draw.total > filler.total);
    }
}
