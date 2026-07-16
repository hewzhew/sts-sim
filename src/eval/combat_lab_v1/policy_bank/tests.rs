use crate::ai::combat_policy_v1::{CombatPublicActionV1, CombatScenarioActionPortfolioLimitsV1};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::eval::fingerprint::combat_state_fingerprint_v1;
use crate::runtime::combat::CombatCard;
use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec};
use crate::sim::combat::CombatPosition;
use crate::state::core::EngineState;

use super::summary::summarize_policy_bank;
use super::*;
use crate::eval::combat_lab_v1::CombatLabCompiledSampleV1;

#[test]
fn shared_public_information_set_is_decided_once_for_all_samples() {
    let first = lethal_sample(0, 10);
    let mut second = lethal_sample(1, 90_001);
    second.start.combat.rng.pool.shuffle_rng.counter += 1;
    second.start.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let mut policy = PlayNamedCardPolicy {
        card_id: "Bludgeon",
        calls: 0,
    };

    let report = execute_combat_lab_public_policy_bank_v1(&[first, second], &mut policy, limits())
        .expect("shared public policy bank");

    assert_eq!(policy.calls, 1);
    assert_eq!(
        report.information_scope,
        CombatLabPolicyInformationScopeV1::PublicHistoryScenarioPolicy
    );
    assert_eq!(report.information_set_decisions, 1);
    assert_eq!(report.summary.wins, 2);
    assert_eq!(report.summary.losses, 0);
    assert_eq!(report.summary.unresolved, 0);
    assert_eq!(report.policy_evaluation_engine_steps, 0);
    assert!(report.execution_engine_steps > 0);
    assert_eq!(report.summary.win_rate_all_scenarios, Some(1.0));
    assert_eq!(report.outcomes[0].public_action_history.len(), 1);
    assert_eq!(report.outcomes[1].public_action_history.len(), 1);

    let json = serde_json::to_string(&report).expect("public policy report serialization");
    assert!(!json.contains("90001"));
    assert!(!json.contains("uuid"));
    assert!(!json.contains("rng"));
}

#[test]
fn one_step_dominance_policy_resolves_clear_lethal_across_hidden_worlds() {
    let first = lethal_sample(0, 10);
    let mut second = lethal_sample(1, 90_001);
    second.start.combat.rng.pool.shuffle_rng.counter += 1;
    second.start.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let mut policy = CombatLabOneStepDominancePolicyV1::new(portfolio_limits());

    let report = execute_combat_lab_public_policy_bank_v1(&[first, second], &mut policy, limits())
        .expect("clear lethal should be strictly dominant");

    assert_eq!(report.summary.wins, 2);
    assert_eq!(report.summary.losses, 0);
    assert_eq!(report.summary.unresolved, 0);
    assert!(report.policy_evaluation_engine_steps > 0);
    assert_eq!(report.execution_engine_steps, 0);
    assert_eq!(
        report.engine_steps,
        report
            .policy_evaluation_engine_steps
            .saturating_add(report.execution_engine_steps)
    );
    assert!(report.outcomes.iter().all(|outcome| {
        matches!(
            outcome.public_action_history.as_slice(),
            [CombatPublicActionV1::PlayCard { card_id, .. }] if card_id == "Bludgeon"
        )
    }));
}

#[test]
fn one_step_dominance_policy_keeps_attack_defend_tradeoff_typed_and_unresolved() {
    let mut policy = CombatLabOneStepDominancePolicyV1::new(portfolio_limits());

    let report =
        execute_combat_lab_public_policy_bank_v1(&[tradeoff_sample(0)], &mut policy, limits())
            .expect("attack versus defend should remain a typed policy gap");

    assert_eq!(report.summary.wins, 0);
    assert_eq!(report.summary.losses, 0);
    assert_eq!(report.summary.unresolved, 1);
    assert_eq!(report.execution_engine_steps, 0);
    assert!(report.policy_evaluation_engine_steps > 0);
    assert_eq!(
        report.gaps[0].reason,
        CombatLabPolicyUnresolvedReasonV1::PolicyGap {
            gap: CombatLabPolicyDecisionGapV1::NoStrictDominance,
        }
    );
    assert!(report.outcomes[0].public_action_history.is_empty());
}

#[test]
fn one_step_dominance_policy_stops_before_evaluation_when_candidate_cap_is_exceeded() {
    let mut policy =
        CombatLabOneStepDominancePolicyV1::new(CombatScenarioActionPortfolioLimitsV1 {
            max_candidates: 1,
            max_engine_steps_per_action: 100,
        });

    let report =
        execute_combat_lab_public_policy_bank_v1(&[lethal_sample(0, 10)], &mut policy, limits())
            .expect("candidate cap should become a typed policy gap");

    assert_eq!(report.policy_evaluation_engine_steps, 0);
    assert_eq!(report.execution_engine_steps, 0);
    assert_eq!(
        report.gaps[0].reason,
        CombatLabPolicyUnresolvedReasonV1::PolicyGap {
            gap: CombatLabPolicyDecisionGapV1::PortfolioTooLarge,
        }
    );
}

#[test]
fn public_action_portfolio_serialization_does_not_expose_exact_world_identity() {
    let first = lethal_sample(0, 10);
    let mut second = lethal_sample(1, 90_001);
    second.start.combat.rng.pool.shuffle_rng.counter += 1;
    second.start.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let mut policy = CapturePortfolioThenGap { json: None };

    execute_combat_lab_public_policy_bank_v1(&[first, second], &mut policy, limits())
        .expect("capture public action portfolio");

    let json = policy.json.expect("captured public portfolio JSON");
    assert!(!json.contains("90001"));
    assert!(!json.contains("combat_lab_sample"));
    assert!(!json.contains("uuid"));
    assert!(!json.contains("rng"));
}

#[test]
fn newly_revealed_draw_order_creates_two_later_policy_decisions() {
    let first = battle_trance_sample(
        0,
        [CardId::Bash, CardId::Defend, CardId::Strike, CardId::Anger],
    );
    let mut second = first.clone();
    second.sample_index = 1;
    second.shuffle_seed = 10_001;
    second.start.combat.zones.draw_pile.swap(0, 1);
    second.state_fingerprint = combat_state_fingerprint_v1(&second.start);
    let mut policy = RootBattleTranceThenGap { calls: 0 };

    let report = execute_combat_lab_public_policy_bank_v1(&[first, second], &mut policy, limits())
        .expect("public reveal should split later information sets");

    assert_eq!(policy.calls, 3);
    assert_eq!(report.information_set_decisions, 3);
    assert_eq!(report.max_frontier_information_sets, 2);
    assert_eq!(report.gaps.len(), 2);
    assert_eq!(report.summary.unresolved, 2);
    assert_eq!(report.summary.wins, 0);
    assert_eq!(report.summary.terminal_hp_loss.count, 0);
    assert!(report
        .outcomes
        .iter()
        .all(|outcome| outcome.public_action_history.len() == 1));
}

#[test]
fn policy_bank_summary_keeps_terminal_tail_separate_from_unresolved() {
    let outcomes = vec![
        outcome(0, CombatLabPolicyScenarioResolutionV1::Win, 1),
        outcome(1, CombatLabPolicyScenarioResolutionV1::Win, 10),
        outcome(2, CombatLabPolicyScenarioResolutionV1::Loss, 20),
        outcome(
            3,
            CombatLabPolicyScenarioResolutionV1::Unresolved {
                reason: CombatLabPolicyUnresolvedReasonV1::PolicyGap {
                    gap: CombatLabPolicyDecisionGapV1::NoAcceptableAction,
                },
            },
            50,
        ),
    ];

    let summary = summarize_policy_bank(&outcomes);

    assert_eq!(summary.scenario_count, 4);
    assert_eq!(summary.wins, 2);
    assert_eq!(summary.losses, 1);
    assert_eq!(summary.unresolved, 1);
    assert_eq!(summary.resolution_rate, Some(0.75));
    assert_eq!(summary.win_rate_all_scenarios, Some(0.5));
    assert_eq!(summary.win_rate_resolved, Some(2.0 / 3.0));
    assert_eq!(summary.terminal_hp_loss.count, 3);
    assert_eq!(summary.terminal_hp_loss.p90_nearest_rank, Some(20));
    assert_eq!(summary.terminal_hp_loss.max, Some(20));
    assert_eq!(summary.win_hp_loss.mean, Some(5.5));
    assert_eq!(summary.loss_hp_loss.mean, Some(20.0));
}

struct PlayNamedCardPolicy {
    card_id: &'static str,
    calls: usize,
}

impl CombatLabPublicPolicyV1 for PlayNamedCardPolicy {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1> {
        self.calls += 1;
        decision
            .information_set
            .candidates
            .iter()
            .find(|action| {
                matches!(
                    action,
                    CombatPublicActionV1::PlayCard { card_id, .. }
                        if card_id == self.card_id
                )
            })
            .cloned()
            .ok_or(CombatLabPolicyDecisionGapV1::NoAcceptableAction)
    }
}

struct RootBattleTranceThenGap {
    calls: usize,
}

impl CombatLabPublicPolicyV1 for RootBattleTranceThenGap {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1> {
        self.calls += 1;
        if decision.depth > 0 {
            return Err(CombatLabPolicyDecisionGapV1::NoAcceptableAction);
        }
        decision
            .information_set
            .candidates
            .iter()
            .find(|action| {
                matches!(
                    action,
                    CombatPublicActionV1::PlayCard { card_id, .. }
                        if card_id == "Battle Trance"
                )
            })
            .cloned()
            .ok_or(CombatLabPolicyDecisionGapV1::NoAcceptableAction)
    }
}

struct CapturePortfolioThenGap {
    json: Option<String>,
}

impl CombatLabPublicPolicyV1 for CapturePortfolioThenGap {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1> {
        let portfolio = decision
            .action_portfolio
            .evaluate(portfolio_limits())
            .map_err(|_| CombatLabPolicyDecisionGapV1::PortfolioEvaluationFailed)?;
        self.json =
            Some(serde_json::to_string(&portfolio).expect("serialize public action portfolio"));
        Err(CombatLabPolicyDecisionGapV1::NoAcceptableAction)
    }
}

fn lethal_sample(sample_index: u64, card_uuid: u32) -> CombatLabCompiledSampleV1 {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::Bludgeon, card_uuid)];
    combat.entities.monsters = vec![deterministic_jaw_worm()];
    compiled_sample(
        sample_index,
        CombatPosition::new(EngineState::CombatPlayerTurn, combat),
    )
}

fn tradeoff_sample(sample_index: u64) -> CombatLabCompiledSampleV1 {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
    ];
    combat.entities.monsters = vec![deterministic_jaw_worm()];
    compiled_sample(
        sample_index,
        CombatPosition::new(EngineState::CombatPlayerTurn, combat),
    )
}

fn deterministic_jaw_worm() -> crate::runtime::combat::MonsterEntity {
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    let attack = MonsterMoveSpec::Attack(AttackSpec {
        base_damage: 1,
        hits: 1,
        damage_kind: DamageKind::Normal,
    });
    monster.set_planned_move_id(1);
    monster.set_planned_steps(attack.to_steps());
    monster.set_planned_visible_spec(Some(attack));
    monster
}

fn battle_trance_sample(sample_index: u64, draw_order: [CardId; 4]) -> CombatLabCompiledSampleV1 {
    let mut combat = crate::test_support::blank_test_combat();
    combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 10)];
    combat.zones.draw_pile = draw_order
        .into_iter()
        .enumerate()
        .map(|(index, card_id)| CombatCard::new(card_id, 20 + index as u32))
        .collect();
    combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    compiled_sample(
        sample_index,
        CombatPosition::new(EngineState::CombatPlayerTurn, combat),
    )
}

fn compiled_sample(sample_index: u64, start: CombatPosition) -> CombatLabCompiledSampleV1 {
    CombatLabCompiledSampleV1 {
        sample_index,
        shuffle_seed: 10_000 + sample_index,
        state_fingerprint: combat_state_fingerprint_v1(&start),
        start,
        non_shuffle_rng_hash: "non-shuffle".to_string(),
        shuffle_rng_hash: format!("shuffle-{sample_index}"),
        monster_snapshot_hash: "monsters".to_string(),
    }
}

fn limits() -> CombatLabPolicyBankLimitsV1 {
    CombatLabPolicyBankLimitsV1 {
        max_information_set_decisions: 20,
        max_actions_per_scenario: 10,
        max_engine_steps_per_action: 100,
    }
}

fn portfolio_limits() -> CombatScenarioActionPortfolioLimitsV1 {
    CombatScenarioActionPortfolioLimitsV1 {
        max_candidates: 32,
        max_engine_steps_per_action: 100,
    }
}

fn outcome(
    sample_index: u64,
    resolution: CombatLabPolicyScenarioResolutionV1,
    hp_loss: i32,
) -> CombatLabPolicyScenarioOutcomeV1 {
    CombatLabPolicyScenarioOutcomeV1 {
        sample_index,
        shuffle_seed: sample_index,
        resolution,
        start_hp: 80,
        final_observed_hp: 80 - hp_loss,
        observed_hp_loss: hp_loss,
        turn_count: 1,
        actions: 1,
        cards_played: 1,
        potions_used: 0,
        public_action_history: Vec::new(),
    }
}
