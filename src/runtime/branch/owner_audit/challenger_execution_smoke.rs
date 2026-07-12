use std::collections::VecDeque;
use std::time::Instant;

use sts_simulator::ai::strategy::challenger_decision_context::{
    open_inventory_pressure, ChallengerDecisionContext,
};
use sts_simulator::ai::strategy::decision_pipeline::{
    CandidateLane, CandidateLaneAdjudication, DecisionCandidateKind, ExpansionPlan,
};
use sts_simulator::ai::strategy::deck_plan::DeckPlanSnapshot;
use sts_simulator::ai::strategy::deck_strategic_deficit::{
    DeckStrategicDeficitSummary, StrategicBurdenLevel, StrategicDeficitLevel,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlConfig, RunControlSession,
};
use sts_simulator::state::core::EngineState;
use sts_simulator::state::rewards::{RewardCard, RewardItem, RewardState};

use super::branch_policy_lane::BranchPolicyLane;
use super::candidate_ir_adapter::card_reward_kind;
use super::frontier_checkpoint;
use super::owner_model::{ChoiceAnnotation, OwnerChoiceExpansion};
use super::policy_expansion_plan::plan_policy_expansions;
use super::run_contract::RunObjective;
use super::run_deadline::RunDeadline;
use super::{card_reward_owner, owner_choice_expander, Args, Branch, BranchStatus, Owner};

fn install_card_reward(session: &mut RunControlSession, cards: Vec<RewardCard>) {
    let mut reward = RewardState::new();
    reward.items = vec![RewardItem::Card {
        cards: cards.clone(),
    }];
    reward.pending_card_choice = Some(cards);
    reward.pending_card_reward_index = Some(0);
    session.engine_state = EngineState::RewardScreen(reward);
}

fn forced_probe_choices(
    session: &RunControlSession,
    probe_card: CardId,
) -> Vec<super::owner_model::OwnerChoice> {
    let surface = build_decision_surface(session);
    let mut choices = card_reward_owner::card_reward_owner_choices(session, &surface);
    for choice in &mut choices {
        let Some(DecisionCandidateKind::CardRewardPick { card, .. }) =
            card_reward_kind(&choice.key)
        else {
            continue;
        };
        if card != probe_card {
            continue;
        }
        let ChoiceAnnotation::Candidate(decision) = &mut choice.annotation else {
            continue;
        };
        decision.evaluation.lane = CandidateLane::Probe;
        decision.evaluation.adjudication =
            CandidateLaneAdjudication::uncapped(CandidateLane::Probe);
        decision.evaluation.expansion = ExpansionPlan::InspectOnly("challenger smoke probe");
        choice.expansion = OwnerChoiceExpansion::InspectOnly("challenger smoke probe");
    }
    choices.sort_by_key(|choice| match card_reward_kind(&choice.key) {
        Some(DecisionCandidateKind::CardRewardSkip) => 0,
        Some(DecisionCandidateKind::CardRewardPick { card, .. }) if card == probe_card => 1,
        _ => 2,
    });
    choices
}

fn growth_pressure_context(session: &RunControlSession) -> ChallengerDecisionContext {
    let facts = DeckStrategicDeficitSummary {
        frontload_damage: StrategicDeficitLevel::Adequate,
        aoe_or_minion_control: StrategicDeficitLevel::Adequate,
        block_or_mitigation: StrategicDeficitLevel::Adequate,
        boss_scaling_plan: StrategicDeficitLevel::Missing,
        deck_access: StrategicDeficitLevel::Adequate,
        energy_or_playability: StrategicDeficitLevel::Adequate,
        deck_burden: StrategicBurdenLevel::Clean,
        too_many_low_impact_attacks: false,
        opening_hand_pollution: false,
        severe_curse_burden: false,
    };
    ChallengerDecisionContext {
        deck_plan: DeckPlanSnapshot::from_run_state(&session.run_state),
        gold: session.run_state.gold,
        current_pressure: open_inventory_pressure(facts),
        automatic_commitments: Vec::new(),
    }
}

fn args() -> Args {
    Args {
        seed: 77,
        ascension: 0,
        objective: RunObjective::FirstVictory,
        generations: 2,
        max_branches: 3,
        auto_ops: 1,
        search_nodes: 1,
        search_ms: 1,
        rescue_search_nodes: 1,
        rescue_search_ms: 1,
        boss_search_nodes: 1,
        boss_search_ms: 1,
        wall_ms: None,
        checkpoint_before_combat_portfolio: false,
        shop_boss_preview_bundle_limit: 0,
        shop_boss_preview_target_floor: None,
        wall_capped_search_budget: false,
        wall_capped_boss_budget: false,
    }
}

#[test]
fn challenger_diverges_twice_and_resumes_without_restarting_session() {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: 77,
        ..RunControlConfig::default()
    });
    let baseline_deck = session.run_state.master_deck.clone();
    install_card_reward(&mut session, vec![RewardCard::new(CardId::Corruption, 0)]);
    let root = Branch {
        id: 0,
        parent_id: None,
        path: Vec::new(),
        session,
        status: BranchStatus::Running {
            owner: Owner::CardReward,
            boundary: "first reward".to_string(),
        },
        policy_lane: BranchPolicyLane::default(),
        combat_portfolio: None,
        auto_steps: Vec::new(),
        combat_search: Vec::new(),
        combat_search_history: Vec::new(),
        accepted_high_loss_diagnostics: Vec::new(),
    };
    let first_choices = forced_probe_choices(&root.session, CardId::Corruption);
    let first_plans = plan_policy_expansions(
        &root.policy_lane,
        &growth_pressure_context(&root.session),
        &first_choices,
        3,
        "branch-0/step-0",
    );
    let first_challenger = first_plans
        .into_iter()
        .find(|plan| plan.child_lane.challenger_policy().is_some())
        .expect("first reward should seed a challenger");
    let mut next_branch_id = 1;
    let mut first_children = owner_choice_expander::expand_registered_owner(
        &root,
        args(),
        RunDeadline::new(Instant::now(), None),
        &first_choices,
        &[first_challenger],
        &mut next_branch_id,
    );
    let mut challenger = first_children
        .pop()
        .expect("challenger child should execute");
    assert!(challenger
        .session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == CardId::Corruption));
    assert_eq!(baseline_deck, root.session.run_state.master_deck);

    install_card_reward(
        &mut challenger.session,
        vec![RewardCard::new(CardId::DarkEmbrace, 0)],
    );
    challenger.status = BranchStatus::Running {
        owner: Owner::CardReward,
        boundary: "second reward".to_string(),
    };
    let second_choices = forced_probe_choices(&challenger.session, CardId::DarkEmbrace);
    let second_plans = plan_policy_expansions(
        &challenger.policy_lane,
        &growth_pressure_context(&challenger.session),
        &second_choices,
        3,
        "branch-1/step-1",
    );
    let mut second_children = owner_choice_expander::expand_registered_owner(
        &challenger,
        args(),
        RunDeadline::new(Instant::now(), None),
        &second_choices,
        &second_plans,
        &mut next_branch_id,
    );
    let mut completed = second_children
        .pop()
        .expect("second challenger choice should execute");
    completed.status = BranchStatus::AwaitingAuto {
        boundary: "smoke checkpoint".to_string(),
        reason: "test fixture".to_string(),
    };
    let path = std::env::temp_dir().join("branch_tiny_challenger_execution_smoke.json");
    let frontier = VecDeque::from([completed]);
    frontier_checkpoint::save(&path, args(), 2, next_branch_id, &frontier).unwrap();

    let (restored, _) = frontier_checkpoint::load(&path)
        .unwrap()
        .into_frontier()
        .unwrap();
    let restored_branch = restored.front().unwrap();
    let restored_policy = restored_branch
        .policy_lane
        .challenger_policy()
        .expect("restored branch should remain challenger");
    assert_eq!(restored_policy.divergence_count, 2);
    assert_eq!(restored_branch.path.len(), 2);
    assert_eq!(restored_branch.policy_lane.label(), "challenger-1");
    assert_eq!(restored_branch.session.run_state.seed, 77);
    assert_ne!(restored_branch.session.run_state.master_deck, baseline_deck);

    let _ = std::fs::remove_file(path);
}
