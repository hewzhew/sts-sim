use super::*;
use std::collections::BTreeSet;

pub(super) fn generator_needs_initial_grounding(
    generation_work: usize,
    generator_finished: bool,
) -> bool {
    generation_work == 0 && !generator_finished
}

pub(super) fn update_max_rank(
    current: &mut Option<CombatStateGuideRank>,
    candidate: &CombatStateGuideRank,
) -> bool {
    if current
        .as_ref()
        .is_some_and(|existing| existing >= candidate)
    {
        return false;
    }
    *current = Some(candidate.clone());
    true
}

pub(super) fn update_max_guide(
    current: &mut GuideRankMap,
    lane: CombatGuideLaneId,
    candidate: &CombatStateGuideRank,
) -> bool {
    current.update_max(lane, candidate)
}

pub(super) fn guide_choice_order(
    left_rank: &CombatStateGuideRank,
    left_anchor: f64,
    left_visits: usize,
    left_successor: usize,
    right_rank: &CombatStateGuideRank,
    right_anchor: f64,
    right_visits: usize,
    right_successor: usize,
) -> std::cmp::Ordering {
    // The policy-only anchor already owns completeness and fair service. An
    // auxiliary guide must remain exploitative; charging it service debt at
    // every tree level makes a good multi-turn corridor lose a fresh fraction
    // of its budget at every parent.
    right_rank
        .cmp(left_rank)
        .then_with(|| left_anchor.total_cmp(&right_anchor))
        .then_with(|| left_visits.cmp(&right_visits))
        .then_with(|| left_successor.cmp(&right_successor))
}

pub(super) fn local_path_base(atomic_depth: usize, negative_log_policy: f64) -> f64 {
    negative_log_policy + (atomic_depth.max(1) as f64).ln()
}

pub(super) fn guide_rank(
    node: &GraphNode,
    lane: CombatGuideLaneId,
) -> Option<&CombatStateGuideRank> {
    node.guides
        .iter()
        .find(|guide| guide.lane == lane)
        .map(|guide| &guide.rank)
}

pub(super) fn backed_guide_rank<'a>(
    edge: &'a GraphEdge,
    successor: &'a GraphNode,
    lane: CombatGuideLaneId,
) -> Option<&'a CombatStateGuideRank> {
    edge.backed_guides
        .get(&lane)
        .or_else(|| guide_rank(successor, lane))
}

pub(super) fn guides_with_pending_lookahead(
    policy: &dyn crate::policy::CombatActionPolicy,
    evaluator: Option<&dyn crate::policy::CombatLookaheadEvaluator>,
    position: &CombatPosition,
) -> (Vec<CombatStateGuide>, Option<CombatGuideLaneId>) {
    let mut guides = policy.state_guides(position);
    let pending_lane = evaluator
        .and_then(|evaluator| evaluator.pending_guide(position))
        .and_then(|pending| {
            if guides.iter().any(|guide| guide.lane == pending.lane) {
                None
            } else {
                let lane = pending.lane;
                guides.push(pending);
                Some(lane)
            }
        });
    (guides, pending_lane)
}

pub(super) fn guide_rank_map(guides: &[CombatStateGuide]) -> GuideRankMap {
    GuideRankMap::from_guides(guides)
}

pub(super) fn generation_service_views_from_lanes(
    lanes: impl IntoIterator<Item = CombatGuideLaneId>,
) -> Vec<LocalServiceView> {
    let lanes = lanes.into_iter().collect::<BTreeSet<_>>();
    std::iter::once(LocalServiceView::Anchor)
        .chain(lanes.into_iter().map(LocalServiceView::Guide))
        .collect()
}

pub(super) fn replay_witness(
    root: &CombatPosition,
    actions: &[TurnOptionAction],
    negative_log_policy: f64,
    discovery_source: OracleCombatWitnessDiscoverySource,
    stepper: &dyn CombatStepper,
) -> Result<OracleCombatWitness, OracleCombatWitnessReplayError> {
    let mut position = root.clone();
    let mut engine_steps = 0usize;
    for (action_index, action) in actions.iter().enumerate() {
        if stepper
            .choice_for_legal_input(&position, &action.input)
            .is_none()
        {
            return Err(OracleCombatWitnessReplayError::IllegalInput { action_index });
        }
        let result = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: action.engine_steps.max(1),
                deadline: None,
            },
        );
        engine_steps = engine_steps.saturating_add(result.engine_steps);
        if result.truncated || result.timed_out {
            return Err(OracleCombatWitnessReplayError::TransitionStepLimit { action_index });
        }
        if exact_hash(&result.position) != action.expected_successor_hash.as_str() {
            return Err(OracleCombatWitnessReplayError::SuccessorMismatch { action_index });
        }
        position = result.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win {
        return Err(OracleCombatWitnessReplayError::FinalStateIsNotWin);
    }
    Ok(OracleCombatWitness {
        actions: actions.to_vec(),
        final_position: position,
        negative_log_policy,
        replay_engine_steps: engine_steps,
        discovery_source,
    })
}

pub(super) fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

pub(super) fn elapsed_nanos_u64(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

pub(super) fn witness_potion_expenditures(
    root: &CombatPosition,
    witness: &OracleCombatWitness,
) -> u32 {
    crate::witness::trajectory_potion_contract_usage(
        root,
        &witness.actions,
        &witness.final_position,
    )
    .expenditures
}

pub(super) fn witness_within_potion_contract(
    root: &CombatPosition,
    witness: &OracleCombatWitness,
    max_potions_used: Option<u32>,
    allowed_potion_slots: Option<u64>,
) -> bool {
    crate::witness::trajectory_within_potion_contract(
        root,
        &witness.actions,
        &witness.final_position,
        max_potions_used,
        allowed_potion_slots,
    )
}

pub(super) fn witness_better_with_potion_budget(
    root: &CombatPosition,
    left: &OracleCombatWitness,
    right: &OracleCombatWitness,
    max_potions_used: Option<u32>,
) -> bool {
    let left_potions = witness_potion_expenditures(root, left);
    let right_potions = witness_potion_expenditures(root, right);
    observable_witness_quality_order(
        ObservableWitnessQuality {
            within_budget: max_potions_used.is_none_or(|limit| left_potions <= limit),
            final_hp: left.final_position.combat.entities.player.current_hp,
            action_count: left.actions.len(),
            negative_log_policy: left.negative_log_policy,
            potion_expenditures: left_potions,
        },
        ObservableWitnessQuality {
            within_budget: max_potions_used.is_none_or(|limit| right_potions <= limit),
            final_hp: right.final_position.combat.entities.player.current_hp,
            action_count: right.actions.len(),
            negative_log_policy: right.negative_log_policy,
            potion_expenditures: right_potions,
        },
    ) == std::cmp::Ordering::Greater
}

pub(super) fn terminal_candidate_could_improve_witness(
    root: &CombatPosition,
    current: &OracleCombatWitness,
    candidate_final_position: &CombatPosition,
    candidate_actions: &[TurnOptionAction],
    candidate_negative_log_policy: f64,
    max_potions_used: Option<u32>,
) -> bool {
    let candidate_potions = crate::witness::trajectory_potion_contract_usage(
        root,
        candidate_actions,
        candidate_final_position,
    )
    .expenditures;
    let current_potions = witness_potion_expenditures(root, current);
    observable_witness_quality_order(
        ObservableWitnessQuality {
            within_budget: max_potions_used.is_none_or(|limit| candidate_potions <= limit),
            final_hp: candidate_final_position.combat.entities.player.current_hp,
            action_count: candidate_actions.len(),
            negative_log_policy: candidate_negative_log_policy,
            potion_expenditures: candidate_potions,
        },
        ObservableWitnessQuality {
            within_budget: max_potions_used.is_none_or(|limit| current_potions <= limit),
            final_hp: current.final_position.combat.entities.player.current_hp,
            action_count: current.actions.len(),
            negative_log_policy: current.negative_log_policy,
            potion_expenditures: current_potions,
        },
    ) == std::cmp::Ordering::Greater
}

#[derive(Clone, Copy)]
struct ObservableWitnessQuality {
    within_budget: bool,
    final_hp: i32,
    action_count: usize,
    negative_log_policy: f64,
    potion_expenditures: u32,
}

fn observable_witness_quality_order(
    left: ObservableWitnessQuality,
    right: ObservableWitnessQuality,
) -> std::cmp::Ordering {
    left.within_budget
        .cmp(&right.within_budget)
        .then_with(|| left.final_hp.cmp(&right.final_hp))
        .then_with(|| right.action_count.cmp(&left.action_count))
        .then_with(|| {
            right
                .negative_log_policy
                .total_cmp(&left.negative_log_policy)
        })
        .then_with(|| right.potion_expenditures.cmp(&left.potion_expenditures))
}

pub(super) fn local_deep_state_snapshot(
    node: &GraphNode,
    path_atomic_depth: usize,
) -> OracleCombatDeepStateSnapshot {
    let combat = &node.generator.root().position().combat;
    let alive_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .collect::<Vec<_>>();
    OracleCombatDeepStateSnapshot {
        player_turn: combat.turn.turn_count,
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        alive_enemy_count: alive_monsters.len(),
        enemy_total_hp: alive_monsters
            .into_iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        hand_size: combat.zones.hand.len(),
        draw_pile_size: combat.zones.draw_pile.len(),
        discard_pile_size: combat.zones.discard_pile.len(),
        exhaust_pile_size: combat.zones.exhaust_pile.len(),
        path_atomic_depth,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_core::content::potions::{Potion, PotionId};
    use sts_core::state::core::EngineState;

    fn witness(final_hp: i32, uses_potion: bool) -> OracleCombatWitness {
        let mut combat = sts_core::test_support::blank_test_combat();
        combat.entities.player.current_hp = final_hp;
        OracleCombatWitness {
            actions: uses_potion
                .then(|| TurnOptionAction {
                    input: ClientInput::UsePotion {
                        potion_index: 0,
                        target: None,
                    },
                    expected_successor_hash: "test".into(),
                    engine_steps: 1,
                })
                .into_iter()
                .collect(),
            final_position: CombatPosition::new(EngineState::CombatPlayerTurn, combat),
            negative_log_policy: 1.0,
            replay_engine_steps: 1,
            discovery_source: OracleCombatWitnessDiscoverySource::PlannerSearch,
        }
    }

    #[test]
    fn potion_budget_is_a_constraint_without_globally_overriding_hp_quality() {
        let root = CombatPosition::new(
            EngineState::CombatPlayerTurn,
            sts_core::test_support::blank_test_combat(),
        );
        let high_hp_potion = witness(50, true);
        let low_hp_clean = witness(20, false);

        assert!(witness_better_with_potion_budget(
            &root,
            &high_hp_potion,
            &low_hp_clean,
            None
        ));
        assert!(witness_better_with_potion_budget(
            &root,
            &low_hp_clean,
            &high_hp_potion,
            Some(0)
        ));
    }

    #[test]
    fn implicit_starting_potion_consumption_obeys_identity_contract() {
        let mut start_combat = sts_core::test_support::blank_test_combat();
        start_combat.entities.potions = vec![
            Some(Potion::new(PotionId::ColorlessPotion, 7)),
            Some(Potion::new(PotionId::FairyPotion, 8)),
        ];
        let root = CombatPosition::new(EngineState::CombatPlayerTurn, start_combat);
        let mut implicit_fairy = witness(10, false);
        implicit_fairy.final_position.combat.entities.potions =
            vec![Some(Potion::new(PotionId::ColorlessPotion, 7)), None];

        assert!(!witness_within_potion_contract(
            &root,
            &implicit_fairy,
            Some(1),
            Some(1 << 0),
        ));
        assert!(witness_within_potion_contract(
            &root,
            &implicit_fairy,
            Some(1),
            Some(1 << 1),
        ));
        assert!(!witness_within_potion_contract(
            &root,
            &implicit_fairy,
            Some(0),
            Some(1 << 1),
        ));

        let mut explicit_colorless_and_implicit_fairy = witness(10, true);
        explicit_colorless_and_implicit_fairy
            .final_position
            .combat
            .entities
            .potions = vec![None, None];
        assert!(!witness_within_potion_contract(
            &root,
            &explicit_colorless_and_implicit_fairy,
            Some(1),
            Some((1 << 0) | (1 << 1)),
        ));
        assert!(witness_within_potion_contract(
            &root,
            &explicit_colorless_and_implicit_fairy,
            Some(2),
            Some((1 << 0) | (1 << 1)),
        ));
    }

    #[test]
    fn terminal_candidate_filter_uses_the_same_complete_witness_order() {
        let root = CombatPosition::new(
            EngineState::CombatPlayerTurn,
            sts_core::test_support::blank_test_combat(),
        );
        let current = witness(50, false);

        let candidate = witness(49, false);
        assert!(!terminal_candidate_could_improve_witness(
            &root,
            &current,
            &candidate.final_position,
            &candidate.actions,
            0.5,
            None
        ));
        let candidate = witness(51, true);
        assert!(terminal_candidate_could_improve_witness(
            &root,
            &current,
            &candidate.final_position,
            &candidate.actions,
            20.0,
            None
        ));
        let candidate = witness(50, false);
        assert!(terminal_candidate_could_improve_witness(
            &root,
            &current,
            &candidate.final_position,
            &candidate.actions,
            0.5,
            None
        ));
        assert!(!terminal_candidate_could_improve_witness(
            &root,
            &current,
            &candidate.final_position,
            &candidate.actions,
            1.0,
            None
        ));
        let candidate = witness(51, true);
        assert!(!terminal_candidate_could_improve_witness(
            &root,
            &current,
            &candidate.final_position,
            &candidate.actions,
            0.5,
            Some(0),
        ));
    }

    #[test]
    fn implicit_potion_consumption_participates_in_witness_ordering() {
        let mut start_combat = sts_core::test_support::blank_test_combat();
        start_combat.entities.potions = vec![Some(Potion::new(PotionId::FairyPotion, 8))];
        let root = CombatPosition::new(EngineState::CombatPlayerTurn, start_combat);
        let mut clean = witness(50, false);
        clean.final_position.combat.entities.potions =
            vec![Some(Potion::new(PotionId::FairyPotion, 8))];
        let mut implicit_fairy = witness(50, false);
        implicit_fairy.final_position.combat.entities.potions = vec![None];

        assert!(witness_better_with_potion_budget(
            &root,
            &clean,
            &implicit_fairy,
            Some(1),
        ));
        assert!(!terminal_candidate_could_improve_witness(
            &root,
            &clean,
            &implicit_fairy.final_position,
            &implicit_fairy.actions,
            implicit_fairy.negative_log_policy,
            Some(1),
        ));
        assert!(terminal_candidate_could_improve_witness(
            &root,
            &implicit_fairy,
            &clean.final_position,
            &clean.actions,
            clean.negative_log_policy,
            Some(1),
        ));
    }
}
