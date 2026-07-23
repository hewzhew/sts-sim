use std::path::PathBuf;

use sts_simulator::eval::run_control::OracleRunBoundaryV1;
use sts_simulator::runtime::branch::{
    load_oracle_analysis_workspace_v1, OracleAnalysisWorkspaceArtifactV1,
    OracleAnalysisWorkspaceV1, OracleRunBudget, OracleRunConfig,
};
use sts_simulator::state::core::ClientInput;

const SEED: u64 = 20_260_713_006;

fn workspace() -> OracleAnalysisWorkspaceV1 {
    OracleAnalysisWorkspaceV1::new(OracleRunConfig {
        seed: SEED,
        ascension: 0,
        budget: OracleRunBudget::default(),
    })
    .expect("analysis workspace")
}

#[test]
fn variations_are_created_without_mutating_the_parent_and_can_be_navigated() {
    let mut workspace = workspace();
    let root = workspace.view().expect("root view");
    let choice = root.choices.first().expect("root map choice").clone();
    let root_hp = root.current_hp;

    let child = workspace
        .try_choice(&choice.choice_ref)
        .expect("materialize child variation");
    assert_ne!(child.node_id, root.node_id);
    assert_eq!(workspace.session.cursor_node_id(), child.node_id);

    let parent_after = workspace
        .session
        .view_node(root.node_id)
        .expect("parent remains inspectable");
    assert_eq!(parent_after.current_hp, root_hp);
    assert!(parent_after
        .choices
        .iter()
        .any(|candidate| candidate.choice_ref == choice.choice_ref));
    let edge = parent_after
        .children
        .iter()
        .find(|edge| edge.child_node_id == child.node_id)
        .expect("variation edge")
        .edge_id;

    assert_eq!(workspace.session.back().expect("back"), root.node_id);
    workspace.session.follow_edge(edge).expect("follow child");
    assert_eq!(workspace.session.cursor_node_id(), child.node_id);
    workspace.session.promote_cursor();
    assert_eq!(workspace.session.mainline_node_id(), child.node_id);
}

#[test]
fn engine_owned_checkpoint_roundtrips_navigation_and_rejects_tampered_choices() {
    let mut workspace = workspace();
    let root = workspace.view().expect("root view");
    let choice = root.choices.first().expect("root map choice").clone();
    let child = workspace
        .try_choice(&choice.choice_ref)
        .expect("materialize child variation");
    workspace.session.promote_cursor();

    let mut tampered = choice.choice_ref.clone();
    tampered.push('0');
    assert!(workspace.try_choice(&tampered).is_err());

    let bytes =
        serde_json::to_vec(&workspace.artifact().expect("artifact")).expect("serialize artifact");
    let artifact = serde_json::from_slice::<OracleAnalysisWorkspaceArtifactV1>(&bytes)
        .expect("deserialize artifact");
    let mut restored = OracleAnalysisWorkspaceV1::restore(artifact).expect("restore workspace");
    let restored_view = restored.view().expect("restored view");
    assert_eq!(restored_view.node_id, child.node_id);
    assert_eq!(restored.session.mainline_node_id(), child.node_id);
    assert_eq!(
        restored.session.back().expect("restored back"),
        root.node_id
    );
}

#[test]
fn restored_combat_node_accepts_exact_actions_without_resident_search() {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let workspace_path =
        repository.join("fixtures/oracle_witnesses/seed20260713007_a0_full_run.workspace.json");
    let actions_path = repository.join(
        "fixtures/oracle_witnesses/seed20260713007_a0_awakened_one.layered-proof-cache.actions.json",
    );
    let actions = serde_json::from_slice::<Vec<ClientInput>>(
        &std::fs::read(actions_path).expect("read exact action fixture"),
    )
    .expect("parse exact action fixture");
    let mut workspace =
        load_oracle_analysis_workspace_v1(&workspace_path).expect("restore completed run");

    workspace
        .session
        .focus_node(141)
        .expect("focus exact boss combat parent");
    assert!(
        workspace.view().expect("combat parent").combat.is_none(),
        "restored workspaces intentionally do not serialize tactical search"
    );
    let child = workspace
        .accept_combat_actions(&actions)
        .expect("exact actions do not require a pre-existing search session");

    assert_eq!(child.boundary, OracleRunBoundaryV1::TerminalVictory);
    assert_eq!(child.current_hp, 16);
}
