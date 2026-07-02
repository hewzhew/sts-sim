use serde::{Deserialize, Serialize};
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    assess_deck_strategic_deficit, DeckStrategicDeficit, StrategicBurdenLevel,
    StrategicDeficitLevel,
};
use sts_simulator::ai::strategy::run_strategic_facts::RunStrategicFacts;
use sts_simulator::state::run::RunState;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct DecisionDeltaSnapshot {
    pub(super) deck_size_before: usize,
    pub(super) deck_size_after: usize,
    pub(super) gold_before: i32,
    pub(super) gold_after: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) changes: Vec<DecisionDeltaChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) improved_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) worsened_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) saturated_fields: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) adds_card_without_gap_improvement: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(super) burden_worsened: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct DecisionDeltaChange {
    pub(super) field: String,
    pub(super) before: String,
    pub(super) after: String,
}

pub(super) fn decision_delta(before: &RunState, after: &RunState) -> Option<DecisionDeltaSnapshot> {
    let before_deficit = assess_deck_strategic_deficit(
        &before.master_deck,
        RunStrategicFacts::from_run_state(before),
    );
    let after_deficit =
        assess_deck_strategic_deficit(&after.master_deck, RunStrategicFacts::from_run_state(after));
    let mut delta = DecisionDeltaSnapshot {
        deck_size_before: before.master_deck.len(),
        deck_size_after: after.master_deck.len(),
        gold_before: before.gold,
        gold_after: after.gold,
        changes: Vec::new(),
        improved_fields: Vec::new(),
        worsened_fields: Vec::new(),
        saturated_fields: Vec::new(),
        adds_card_without_gap_improvement: false,
        burden_worsened: false,
    };
    compare_deficit_fields(&mut delta, &before_deficit, &after_deficit);
    compare_burden(
        &mut delta,
        before_deficit.deck_burden,
        after_deficit.deck_burden,
    );
    delta.adds_card_without_gap_improvement =
        delta.deck_size_after > delta.deck_size_before && delta.improved_fields.is_empty();
    if delta.changes.is_empty()
        && delta.deck_size_before == delta.deck_size_after
        && delta.gold_before == delta.gold_after
    {
        None
    } else {
        Some(delta)
    }
}

fn compare_deficit_fields(
    delta: &mut DecisionDeltaSnapshot,
    before: &DeckStrategicDeficit,
    after: &DeckStrategicDeficit,
) {
    compare_deficit(
        delta,
        "frontload_damage",
        before.frontload_damage,
        after.frontload_damage,
        true,
    );
    compare_deficit(
        delta,
        "aoe_or_minion_control",
        before.aoe_or_minion_control,
        after.aoe_or_minion_control,
        true,
    );
    compare_deficit(
        delta,
        "block_or_mitigation",
        before.block_or_mitigation,
        after.block_or_mitigation,
        true,
    );
    compare_deficit(
        delta,
        "boss_scaling_plan",
        before.boss_scaling_plan,
        after.boss_scaling_plan,
        false,
    );
    compare_deficit(
        delta,
        "deck_access",
        before.deck_access,
        after.deck_access,
        true,
    );
    compare_deficit(
        delta,
        "energy_or_playability",
        before.energy_or_playability,
        after.energy_or_playability,
        false,
    );
}

fn compare_deficit(
    delta: &mut DecisionDeltaSnapshot,
    field: &'static str,
    before: StrategicDeficitLevel,
    after: StrategicDeficitLevel,
    track_saturation: bool,
) {
    if before == after {
        return;
    }
    delta.changes.push(DecisionDeltaChange {
        field: field.to_string(),
        before: deficit_label(before).to_string(),
        after: deficit_label(after).to_string(),
    });
    if deficit_rank(after) > deficit_rank(before)
        && matches!(
            before,
            StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
        )
    {
        delta.improved_fields.push(field.to_string());
    } else if deficit_rank(after) < deficit_rank(before) {
        delta.worsened_fields.push(field.to_string());
    }
    if track_saturation
        && before != StrategicDeficitLevel::Surplus
        && after == StrategicDeficitLevel::Surplus
    {
        delta.saturated_fields.push(field.to_string());
    }
}

fn compare_burden(
    delta: &mut DecisionDeltaSnapshot,
    before: StrategicBurdenLevel,
    after: StrategicBurdenLevel,
) {
    if before == after {
        return;
    }
    delta.changes.push(DecisionDeltaChange {
        field: "deck_burden".to_string(),
        before: burden_label(before).to_string(),
        after: burden_label(after).to_string(),
    });
    if burden_rank(after) > burden_rank(before) {
        delta.burden_worsened = true;
        delta.worsened_fields.push("deck_burden".to_string());
    } else {
        delta.improved_fields.push("deck_burden".to_string());
    }
}

fn deficit_rank(level: StrategicDeficitLevel) -> i32 {
    match level {
        StrategicDeficitLevel::Missing => 0,
        StrategicDeficitLevel::Thin => 1,
        StrategicDeficitLevel::Adequate => 2,
        StrategicDeficitLevel::Surplus => 3,
    }
}

fn burden_rank(level: StrategicBurdenLevel) -> i32 {
    match level {
        StrategicBurdenLevel::Clean => 0,
        StrategicBurdenLevel::Watch => 1,
        StrategicBurdenLevel::Heavy => 2,
    }
}

fn deficit_label(level: StrategicDeficitLevel) -> &'static str {
    match level {
        StrategicDeficitLevel::Missing => "missing",
        StrategicDeficitLevel::Thin => "thin",
        StrategicDeficitLevel::Adequate => "adequate",
        StrategicDeficitLevel::Surplus => "surplus",
    }
}

fn burden_label(level: StrategicBurdenLevel) -> &'static str {
    match level {
        StrategicBurdenLevel::Clean => "clean",
        StrategicBurdenLevel::Watch => "watch",
        StrategicBurdenLevel::Heavy => "heavy",
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}
