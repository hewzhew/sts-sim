use crate::bot::card_taxonomy::taxonomy;
use crate::bot::combat_heuristic;
use crate::bot::encounter_suite::{
    advance_suite_engine, rollout_entries_for_suite, start_suite_encounter, suite_for_run,
    weights_for_suite, EncounterSuiteId,
};
use crate::bot::noncombat_card_signals::signals as noncombat_card_signals;
use crate::content::cards::CardId;
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::run::RunState;

#[derive(Debug, Clone)]
pub struct DeltaScore {
    pub suite: EncounterSuiteId,
    pub prior_delta: i32,
    pub prior_rationale_key: Option<&'static str>,
    pub prior_assessment: crate::bot::run_deck_improvement::DeckOperationAssessment,
    pub suite_bias: i32,
    pub rollout_delta: i32,
    pub context_delta: i32,
    pub context_rationale_key: Option<&'static str>,
    pub rule_context_summary: Option<&'static str>,
    pub total: i32,
}

pub fn compare_pick_vs_skip(rs: &RunState, card_id: CardId) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::Add(card_id),
    );
    let suite_bias = suite_pick_bias(rs, suite, card_id);
    let rollout_delta = rollout_pick_delta(rs, suite, card_id);
    let conditioned = crate::bot::run_rule_context::conditioned_card_addition_value(rs, card_id);
    build_delta_score(
        suite,
        prior,
        suite_bias,
        rollout_delta,
        conditioned.total,
        conditioned.rationale_key,
        conditioned.rule_context_summary,
    )
}

pub(crate) fn compare_purge_vs_keep(rs: &RunState) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::Remove,
    );
    let suite_bias = suite_purge_bias(rs, suite);
    let rollout_delta = rollout_purge_delta(rs, suite);
    build_delta_score(suite, prior, suite_bias, rollout_delta, 0, None, None)
}

pub(crate) fn compare_transform_vs_decline(
    rs: &RunState,
    count: usize,
    upgraded_context: bool,
) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::Transform {
            count,
            upgraded_context,
        },
    );
    let suite_bias = suite_transform_bias(rs, suite, count, upgraded_context);
    let rollout_delta = rollout_transform_delta(rs, suite, count, upgraded_context);
    build_delta_score(suite, prior, suite_bias, rollout_delta, 0, None, None)
}

pub(crate) fn compare_upgrade_vs_decline(rs: &RunState, random_upgrades: usize) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::Upgrade,
    );
    let suite_bias = suite_upgrade_bias(rs, suite, random_upgrades);
    let rollout_delta = rollout_upgrade_delta(rs, suite, random_upgrades);
    build_delta_score(suite, prior, suite_bias, rollout_delta, 0, None, None)
}

pub(crate) fn compare_duplicate_vs_decline(rs: &RunState) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::Duplicate,
    );
    let suite_bias = suite_duplicate_bias(rs, suite);
    let rollout_delta = rollout_duplicate_delta(rs, suite);
    build_delta_score(suite, prior, suite_bias, rollout_delta, 0, None, None)
}

pub(crate) fn compare_vampires_vs_refuse(rs: &RunState) -> DeltaScore {
    let suite = suite_for_run(rs);
    let prior = crate::bot::run_deck_improvement::assess_deck_operation(
        rs,
        crate::bot::run_deck_improvement::DeckOperationKind::VampiresExchange,
    );
    let suite_bias = suite_vampires_bias(rs, suite);
    build_delta_score(suite, prior, suite_bias, 0, 0, None, None)
}

fn build_delta_score(
    suite: EncounterSuiteId,
    prior_assessment: crate::bot::run_deck_improvement::DeckOperationAssessment,
    suite_bias: i32,
    rollout_delta: i32,
    context_delta: i32,
    context_rationale_key: Option<&'static str>,
    rule_context_summary: Option<&'static str>,
) -> DeltaScore {
    let prior_delta = prior_assessment.total_prior_delta;
    let rollout_delta = stabilized_rollout_delta(prior_delta, suite_bias, rollout_delta);
    DeltaScore {
        suite,
        prior_delta,
        prior_rationale_key: Some(prior_assessment.rationale_key),
        prior_assessment,
        suite_bias,
        rollout_delta,
        context_delta,
        context_rationale_key,
        rule_context_summary,
        total: prior_delta + suite_bias + rollout_delta + context_delta,
    }
}

fn stabilized_rollout_delta(prior_delta: i32, suite_bias: i32, rollout_delta: i32) -> i32 {
    if rollout_delta == 0 {
        return 0;
    }

    let prior_signal = prior_delta + (suite_bias / 2);
    let mut adjusted = rollout_delta.clamp(-36, 36);

    if prior_signal != 0 && adjusted.signum() != prior_signal.signum() {
        let divisor = if prior_signal.abs() >= 18 { 4 } else { 2 };
        adjusted /= divisor;
    }

    adjusted
}

fn rollout_pick_delta(rs: &RunState, suite: EncounterSuiteId, card_id: CardId) -> i32 {
    crate::state::run::with_suppressed_obtain_logs(|| {
        let mut picked = rs.clone();
        picked.add_card_to_deck(card_id);
        rollout_delta_between(rs, &picked, suite)
    })
}

fn rollout_purge_delta(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let Some(uuid) = crate::bot::run_deck_improvement::best_remove_uuid(rs) else {
        return 0;
    };

    crate::state::run::with_suppressed_obtain_logs(|| {
        let mut purged = rs.clone();
        purged.remove_card_from_deck(uuid);
        rollout_delta_between(rs, &purged, suite)
    })
}

fn rollout_upgrade_delta(rs: &RunState, suite: EncounterSuiteId, random_upgrades: usize) -> i32 {
    let Some(uuid) = crate::bot::run_deck_improvement::best_upgrade_uuid(rs) else {
        return 0;
    };

    let mut upgraded = rs.clone();
    let mut applied = 0usize;
    for card in &mut upgraded.master_deck {
        if card.uuid == uuid && crate::bot::run_deck_improvement::is_upgradable(card) {
            card.upgrades += 1;
            applied += 1;
            if applied >= random_upgrades.max(1) {
                break;
            }
        }
    }
    if applied == 0 {
        return 0;
    }
    rollout_delta_between(rs, &upgraded, suite)
}

fn rollout_duplicate_delta(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let Some(uuid) = crate::bot::run_deck_improvement::best_duplicate_uuid(rs) else {
        return 0;
    };
    let Some(card) = rs
        .master_deck
        .iter()
        .find(|card| card.uuid == uuid)
        .cloned()
    else {
        return 0;
    };

    crate::state::run::with_suppressed_obtain_logs(|| {
        let mut duplicated = rs.clone();
        duplicated.add_card_to_deck_with_upgrades(card.id, card.upgrades);
        rollout_delta_between(rs, &duplicated, suite)
    })
}

fn rollout_transform_delta(
    rs: &RunState,
    suite: EncounterSuiteId,
    count: usize,
    upgraded_context: bool,
) -> i32 {
    let targets =
        crate::bot::run_deck_improvement::best_transform_uuids(rs, count, upgraded_context);
    if targets.is_empty() {
        return 0;
    }

    crate::state::run::with_suppressed_obtain_logs(|| {
        let mut transformed = rs.clone();
        for uuid in targets {
            transformed.remove_card_from_deck(uuid);
            let replacement = representative_transform_replacement(suite);
            transformed.add_card_to_deck_with_upgrades(replacement, u8::from(upgraded_context));
        }
        rollout_delta_between(rs, &transformed, suite)
    })
}

fn rollout_delta_between(before: &RunState, after: &RunState, suite: EncounterSuiteId) -> i32 {
    let before_score = suite_rollout_score(before, suite);
    let after_score = suite_rollout_score(after, suite);
    after_score - before_score
}

fn suite_rollout_score(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let entries = rollout_entries_for_suite(suite);
    if entries.is_empty() {
        return 0;
    }

    let mut weighted_total = 0;
    let mut total_weight = 0;

    for &entry in entries {
        let outcome = run_conservative_rollout(rs, entry);
        weighted_total += outcome.score * entry.weight;
        total_weight += entry.weight;
    }

    if total_weight == 0 {
        0
    } else {
        weighted_total / total_weight
    }
}

fn representative_transform_replacement(suite: EncounterSuiteId) -> CardId {
    match suite {
        EncounterSuiteId::Act1Pathing => CardId::ShrugItOff,
        EncounterSuiteId::Act2Pathing => CardId::Uppercut,
        EncounterSuiteId::Act3Pathing => CardId::ShrugItOff,
    }
}

#[derive(Debug, Clone, Copy)]
struct RolloutOutcome {
    score: i32,
}

fn run_conservative_rollout(
    rs: &RunState,
    entry: crate::bot::encounter_suite::EncounterSuiteEntry,
) -> RolloutOutcome {
    crate::engine::core::with_suppressed_engine_warnings(|| {
        let (mut engine, mut combat) = start_suite_encounter(rs, entry);
        let potions_before = filled_potion_slots(&combat);
        let baseline_hp = combat.entities.player.current_hp;
        let mut steps = 0;

        loop {
            match engine {
                EngineState::GameOver(RunResult::Victory) => {
                    return RolloutOutcome {
                        score: rollout_terminal_score(
                            &combat,
                            true,
                            baseline_hp,
                            potions_before,
                            steps,
                        ),
                    };
                }
                EngineState::GameOver(_) => {
                    return RolloutOutcome {
                        score: rollout_terminal_score(
                            &combat,
                            false,
                            baseline_hp,
                            potions_before,
                            steps,
                        ),
                    };
                }
                EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
                    let legal_moves = crate::bot::search::get_legal_moves(&engine, &combat);
                    if legal_moves.is_empty() {
                        return RolloutOutcome {
                            score: rollout_terminal_score(
                                &combat,
                                false,
                                baseline_hp,
                                potions_before,
                                steps,
                            ),
                        };
                    }

                    let chosen = choose_conservative_action(&engine, &combat, &legal_moves);
                    if !crate::engine::core::tick_until_stable_turn(
                        &mut engine,
                        &mut combat,
                        chosen,
                    ) {
                        return RolloutOutcome {
                            score: rollout_terminal_score(
                                &combat,
                                false,
                                baseline_hp,
                                potions_before,
                                steps,
                            ),
                        };
                    }
                }
                _ => {
                    advance_suite_engine(&mut engine, &mut combat);
                }
            }

            steps += 1;
            if steps > 120 {
                return RolloutOutcome {
                    score: rollout_timeout_score(&combat, baseline_hp, potions_before, steps),
                };
            }
        }
    })
}

fn choose_conservative_action(
    engine: &EngineState,
    combat: &crate::runtime::combat::CombatState,
    legal_moves: &[ClientInput],
) -> ClientInput {
    if legal_moves.len() == 1 {
        return legal_moves[0].clone();
    }

    let mut best_move = legal_moves[0].clone();
    let mut best_score = f32::MIN;

    for candidate in legal_moves {
        let mut sim_engine = engine.clone();
        let mut sim_combat = combat.clone();
        let alive = crate::engine::core::tick_until_stable_turn(
            &mut sim_engine,
            &mut sim_combat,
            candidate.clone(),
        );
        let score = conservative_choice_score(
            &sim_engine,
            &sim_combat,
            candidate,
            legal_moves.len() > 1,
            alive,
        );
        if score > best_score {
            best_score = score;
            best_move = candidate.clone();
        }
    }

    best_move
}

fn conservative_choice_score(
    engine: &EngineState,
    combat: &crate::runtime::combat::CombatState,
    candidate: &ClientInput,
    has_alternative: bool,
    alive: bool,
) -> f32 {
    if !alive || matches!(engine, EngineState::GameOver(RunResult::Defeat)) {
        return -100_000.0;
    }

    if matches!(engine, EngineState::GameOver(RunResult::Victory)) || combat_cleared(combat) {
        return 100_000.0 + combat.entities.player.current_hp as f32 * 120.0;
    }

    let incoming = incoming_damage(combat);
    let unblocked = (incoming - combat.entities.player.block).max(0);
    let monster_hp = total_monster_hp(combat);
    let heuristic = combat_heuristic::evaluate_combat_state(combat) as f32 / 200.0;
    let mut score = combat.entities.player.current_hp as f32 * 550.0
        + combat.entities.player.block as f32 * 18.0
        - unblocked as f32 * 150.0
        - monster_hp as f32 * 8.0
        + heuristic;

    if matches!(candidate, ClientInput::EndTurn) && has_alternative {
        score -= 300.0;
    }
    if matches!(candidate, ClientInput::UsePotion { .. }) {
        score -= 110.0;
    }

    score
}

fn combat_cleared(combat: &crate::runtime::combat::CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}

fn total_monster_hp(combat: &crate::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn incoming_damage(combat: &crate::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| match monster.current_intent {
            crate::runtime::combat::Intent::Attack { hits, .. }
            | crate::runtime::combat::Intent::AttackBuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDefend { hits, .. } => {
                monster.intent_dmg * hits as i32
            }
            _ => 0,
        })
        .sum()
}

fn filled_potion_slots(combat: &crate::runtime::combat::CombatState) -> i32 {
    combat
        .entities
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count() as i32
}

fn rollout_terminal_score(
    combat: &crate::runtime::combat::CombatState,
    victory: bool,
    baseline_hp: i32,
    potions_before: i32,
    steps: i32,
) -> i32 {
    let hp_loss = (baseline_hp - combat.entities.player.current_hp).max(0);
    let potions_used = (potions_before - filled_potion_slots(combat)).max(0);
    if victory {
        4_000 + combat.entities.player.current_hp * 45
            - hp_loss * 20
            - potions_used * 90
            - steps * 6
    } else {
        -4_000 - total_monster_hp(combat) * 18 - hp_loss * 25 - potions_used * 90 - steps * 4
    }
}

fn rollout_timeout_score(
    combat: &crate::runtime::combat::CombatState,
    baseline_hp: i32,
    potions_before: i32,
    steps: i32,
) -> i32 {
    let hp_loss = (baseline_hp - combat.entities.player.current_hp).max(0);
    let potions_used = (potions_before - filled_potion_slots(combat)).max(0);
    -1_500 - total_monster_hp(combat) * 10 - hp_loss * 15 - potions_used * 75 - steps * 5
}

fn suite_pick_bias(rs: &RunState, suite: EncounterSuiteId, card_id: CardId) -> i32 {
    let weights = weights_for_suite(suite);
    let tax = taxonomy(card_id);
    let signals = noncombat_card_signals(card_id);
    let mut score = 0;

    if tax.is_strength_payoff() || tax.is_multi_attack_payoff() || tax.is_attack_followup_priority()
    {
        score += weights.frontload;
    }
    if tax.is_block_core() || tax.is_vuln_enabler() || tax.is_weak_enabler() {
        score += weights.block / 2;
    }
    if tax.is_setup_power() || tax.is_scaling_power() || tax.is_engine_piece() {
        score += weights.scaling / 2;
    }
    if tax.is_draw_core() || tax.is_exhaust_outlet() || tax.is_exhaust_engine() {
        score += weights.deck_thinning / 3;
    }

    if rs.act_num == 1 && rs.floor_num <= 16 && signals.frontload_patch_strength >= 14 {
        score += weights.frontload;
    }
    if rs.act_num >= 2 && signals.scaling_signal >= 10 {
        score += weights.scaling;
    }

    score
}

fn suite_purge_bias(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let weights = weights_for_suite(suite);
    let starter_strikes = rs
        .master_deck
        .iter()
        .filter(|card| crate::content::cards::is_starter_strike(card.id))
        .count() as i32;
    let starter_basics = rs
        .master_deck
        .iter()
        .filter(|card| crate::content::cards::is_starter_basic(card.id))
        .count() as i32;

    starter_basics * (weights.deck_thinning / 3) + starter_strikes * (weights.frontload / 4)
}

fn suite_transform_bias(
    rs: &RunState,
    suite: EncounterSuiteId,
    count: usize,
    upgraded_context: bool,
) -> i32 {
    let weights = weights_for_suite(suite);
    let transform_targets = rs
        .master_deck
        .iter()
        .filter(|card| {
            crate::content::cards::is_starter_basic(card.id)
                || crate::bot::evaluator::curse_remove_severity(card.id) > 0
        })
        .count() as i32;

    transform_targets.min(count as i32) * (weights.deck_thinning / 2)
        + i32::from(upgraded_context) * (weights.scaling / 2)
}

fn suite_upgrade_bias(rs: &RunState, suite: EncounterSuiteId, random_upgrades: usize) -> i32 {
    let weights = weights_for_suite(suite);
    let upgradable = rs
        .master_deck
        .iter()
        .filter(|card| crate::bot::run_deck_improvement::is_upgradable(card))
        .count() as i32;

    upgradable.min(random_upgrades as i32).max(1) * (weights.scaling / 2)
}

fn suite_duplicate_bias(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let weights = weights_for_suite(suite);
    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let frontload_shell = profile.strength_enablers.min(profile.strength_payoffs);
    let block_shell = profile.block_core.min(profile.block_payoffs);

    frontload_shell * (weights.frontload / 2) + block_shell * (weights.scaling / 2)
}

fn suite_vampires_bias(rs: &RunState, suite: EncounterSuiteId) -> i32 {
    let weights = weights_for_suite(suite);
    let strike_count = rs
        .master_deck
        .iter()
        .filter(|card| crate::content::cards::is_starter_strike(card.id))
        .count() as i32;
    let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;

    strike_count * (weights.deck_thinning / 2) + i32::from(hp_ratio >= 0.60) * weights.block
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;

    #[test]
    fn delta_score_carries_typed_prior_assessment() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Shockwave,
            81_001,
        ));
        let delta = compare_pick_vs_skip(&rs, CardId::Armaments);
        assert_eq!(
            delta.prior_assessment.operation,
            crate::bot::run_deck_improvement::DeckOperationKind::Add(CardId::Armaments)
        );
        assert_eq!(
            delta.prior_rationale_key,
            Some(delta.prior_assessment.rationale_key)
        );
    }
}
