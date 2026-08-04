use super::*;

use crate::eval::run_control::{
    seed_oracle_run_explorer_from_session_v1, OracleRunCombatBudgetsV1, RunControlConfig,
    RunControlSearchCombatOptions, RunControlSession, RunProgressJournalV1,
};
use crate::runtime::combat::CombatCard;
use crate::sim::combat::CombatTerminal;
use crate::state::core::{
    ActiveCombat, ClientInput, CombatContext, EngineState, PendingChoice, RoomCombatContext,
};
use crate::state::map::node::RoomType;

fn scratch_analysis_at_engine(
    combat: crate::runtime::combat::CombatState,
    engine_state: EngineState,
) -> OracleAnalysisSessionV1 {
    let mut run = RunControlSession::new(RunControlConfig::default());
    run.engine_state = engine_state.clone();
    run.active_combat = Some(ActiveCombat::new(
        engine_state,
        combat,
        CombatContext::Room(RoomCombatContext {
            room_type: RoomType::MonsterRoom,
        }),
    ));
    let combat_budgets =
        OracleRunCombatBudgetsV1::uniform(RunControlSearchCombatOptions::default());
    let explorer = seed_oracle_run_explorer_from_session_v1(
        run,
        RunProgressJournalV1::default(),
        &combat_budgets,
        None,
    )
    .expect("seed scratch analysis");
    OracleAnalysisSessionV1::from_explorer(explorer, Some(0), combat_budgets, None, None)
        .expect("scratch analysis")
}

fn one_strike_scratch_analysis() -> OracleAnalysisSessionV1 {
    scratch_analysis_at_engine(one_strike_combat(), EngineState::CombatPlayerTurn)
}

fn one_strike_combat() -> crate::runtime::combat::CombatState {
    let mut combat = crate::test_support::blank_test_combat();
    let mut monster = crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
    let plan = crate::content::monsters::roll_monster_turn_plan(
        &mut combat.rng.ai_rng,
        &monster,
        combat.meta.ascension_level,
        99,
        std::slice::from_ref(&monster),
        &[],
    );
    monster.set_planned_move_id(plan.move_id);
    monster.set_planned_steps(plan.steps);
    monster.set_planned_visible_spec(plan.visible_spec);
    monster.current_hp = 6;
    monster.max_hp = 6;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![CombatCard::new(crate::content::cards::CardId::Strike, 1)];
    combat
}

fn scratch_action_ref(
    view: &OracleAnalysisCombatScratchViewV1,
    predicate: impl Fn(&ClientInput) -> bool,
) -> String {
    view.legal_actions
        .atomic_actions
        .iter()
        .find(|action| predicate(&action.input))
        .expect("matching scratch action")
        .action_ref
        .clone()
}

#[test]
fn combat_scratch_branches_with_deltas_without_mutating_the_run_tree() {
    let mut analysis = one_strike_scratch_analysis();
    let run_tree_before = serde_json::to_value(analysis.tree()).expect("run tree before scratch");
    let root = analysis
        .start_combat_scratch(None, 250, 0, 16)
        .expect("start combat scratch");
    let end_turn = scratch_action_ref(&root, |input| matches!(input, ClientInput::EndTurn));
    let after_end_turn = analysis
        .play_combat_scratch_action(&end_turn, 0, 16)
        .expect("play scratch end turn");
    assert_eq!(after_end_turn.scratch_node_count, 2);

    analysis
        .back_combat_scratch(0, 16)
        .expect("back to scratch root");
    let root = analysis
        .combat_scratch_view(0, 16)
        .expect("view scratch root");
    let strike = scratch_action_ref(&root, |input| matches!(input, ClientInput::PlayCard { .. }));
    let victory = analysis
        .play_combat_scratch_action(&strike, 0, 16)
        .expect("play scratch strike");
    assert_eq!(victory.position.terminal, CombatTerminal::Win);
    assert_eq!(victory.scratch_node_count, 3);

    let checkpoint = analysis.checkpoint().expect("scratch checkpoint");
    let scratch = checkpoint.combat_scratch.expect("persisted scratch");
    assert_eq!(scratch.nodes.len(), 3);
    assert!(scratch.nodes.iter().all(|node| {
        (node.scratch_node_id == 0 && node.input.is_none())
            || (node.parent_scratch_node_id.is_some() && node.input.is_some())
    }));
    let serialized = serde_json::to_value(&scratch).expect("serialize scratch checkpoint");
    assert!(serialized["nodes"]
        .as_array()
        .expect("scratch nodes")
        .iter()
        .all(|node| node.get("position").is_none()));
    assert_eq!(
        serde_json::to_value(analysis.tree()).expect("run tree after scratch"),
        run_tree_before,
        "scratch branching must not materialize run variations"
    );
}

#[test]
fn combat_scratch_restore_replays_exact_hashes_and_rejects_stale_refs_atomically() {
    let mut analysis = one_strike_scratch_analysis();
    let budgets = analysis.combat_budgets.clone();
    let root = analysis
        .start_combat_scratch(None, 250, 0, 16)
        .expect("start combat scratch");
    let stale_strike =
        scratch_action_ref(&root, |input| matches!(input, ClientInput::PlayCard { .. }));
    let end_turn = scratch_action_ref(&root, |input| matches!(input, ClientInput::EndTurn));
    let descendant = analysis
        .play_combat_scratch_action(&end_turn, 0, 16)
        .expect("play scratch end turn");
    let expected_hash = descendant.position.fingerprint.exact_state_hash.clone();
    let before_failure = serde_json::to_value(analysis.checkpoint().expect("before stale ref"))
        .expect("serialize before stale ref");
    let error = analysis
        .play_combat_scratch_action(&stale_strike, 0, 16)
        .expect_err("root action ref must be stale at descendant");
    assert!(
        error.contains("stale"),
        "unexpected stale-ref error: {error}"
    );
    assert_eq!(
        serde_json::to_value(analysis.checkpoint().expect("after stale ref"))
            .expect("serialize after stale ref"),
        before_failure,
        "a stale action ref must not mutate scratch or run state"
    );

    let checkpoint = analysis
        .checkpoint()
        .expect("checkpoint scratch descendant");
    let restored = OracleAnalysisSessionV1::restore(checkpoint, budgets, None, None)
        .expect("restore scratch descendant");
    assert_eq!(
        restored
            .combat_scratch_view(0, 16)
            .expect("restored scratch view")
            .position
            .fingerprint
            .exact_state_hash,
        expected_hash
    );
}

#[test]
fn combat_scratch_commit_requires_victory_and_then_materializes_one_atomic_witness() {
    let mut analysis = one_strike_scratch_analysis();
    analysis
        .start_combat_scratch(None, 250, 0, 16)
        .expect("start combat scratch");
    let before_rejection =
        serde_json::to_value(analysis.checkpoint().expect("before nonterminal commit"))
            .expect("serialize before nonterminal commit");
    let error = analysis
        .commit_combat_scratch()
        .expect_err("nonterminal scratch must not commit");
    assert!(error.contains("not a terminal victory"), "{error}");
    assert_eq!(
        serde_json::to_value(analysis.checkpoint().expect("after nonterminal commit"))
            .expect("serialize after nonterminal commit"),
        before_rejection
    );

    let root = analysis
        .combat_scratch_view(0, 16)
        .expect("view scratch root");
    let strike = scratch_action_ref(&root, |input| matches!(input, ClientInput::PlayCard { .. }));
    analysis
        .play_combat_scratch_action(&strike, 0, 16)
        .expect("play winning strike");
    let child = analysis
        .commit_combat_scratch()
        .expect("commit terminal scratch witness");

    assert_eq!(analysis.cursor_node_id(), child);
    assert_eq!(
        analysis.view_cursor().expect("committed child").boundary,
        OracleRunBoundaryV1::Reward
    );
    assert!(analysis
        .checkpoint()
        .expect("post-commit checkpoint")
        .combat_scratch
        .is_none());
    assert_eq!(analysis.tree().nodes.len(), 2);
}

#[test]
fn combat_scratch_bounded_search_appends_a_verified_suffix_without_touching_run_history() {
    let mut analysis = one_strike_scratch_analysis();
    analysis
        .start_combat_scratch(None, 250, 0, 16)
        .expect("start combat scratch");
    let run_tree_before = serde_json::to_value(analysis.tree()).expect("run tree before search");
    let (report, view) = analysis
        .search_combat_scratch(
            OracleAnalysisCombatScratchSearchRequestV1 {
                max_quanta: 4,
                quantum_nodes: 64,
                quantum_ms: 100,
                wall_ms: 500,
            },
            0,
            16,
        )
        .expect("bounded scratch descendant search");

    assert_eq!(
        report.exit,
        OracleAnalysisCombatScratchSearchExitV1::WitnessAdded
    );
    assert!(report.appended_action_count > 0);
    assert_eq!(report.additional_potions_allowed, 0);
    assert_eq!(view.position.terminal, CombatTerminal::Win);
    assert_eq!(
        serde_json::to_value(analysis.tree()).expect("run tree after search"),
        run_tree_before,
        "search may append scratch deltas but cannot commit run history"
    );
}

#[test]
fn combat_scratch_pages_structured_selection_inputs_without_eager_storage() {
    let mut combat = one_strike_combat();
    combat.zones.draw_pile = vec![
        CombatCard::new(crate::content::cards::CardId::Strike, 11),
        CombatCard::new(crate::content::cards::CardId::Defend, 22),
        CombatCard::new(crate::content::cards::CardId::Bash, 33),
    ]
    .into();
    let engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![
            crate::content::cards::CardId::Strike,
            crate::content::cards::CardId::Defend,
            crate::content::cards::CardId::Bash,
        ],
        card_uuids: vec![11, 22, 33],
    });
    let mut analysis = scratch_analysis_at_engine(combat, engine);
    let first_page = analysis
        .start_combat_scratch(None, 250, 0, 2)
        .expect("start structured combat scratch");
    let family = &first_page.legal_actions.selection_families[0];
    assert_eq!(family.total_input_count, 16);
    assert_eq!(family.actions.len(), 2);
    assert_eq!(family.next_page_offset, Some(2));
    assert!(analysis
        .checkpoint()
        .expect("structured scratch checkpoint")
        .combat_scratch
        .expect("structured scratch")
        .nodes
        .iter()
        .all(|node| node.input.is_none()));

    let second_page = analysis
        .combat_scratch_view(2, 2)
        .expect("second structured page");
    assert_eq!(
        second_page.legal_actions.selection_families[0].page_offset,
        2
    );
    assert_ne!(
        first_page.legal_actions.selection_families[0].actions[0].action_ref,
        second_page.legal_actions.selection_families[0].actions[0].action_ref
    );
}
